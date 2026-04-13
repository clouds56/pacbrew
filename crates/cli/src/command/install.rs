use anyhow::Result;
use std::io::{BufRead, Write};
use std::collections::{HashMap, HashSet};

use core_lib::{db::{self, InstalledVersionStatus}, io::{fetch::MirrorLists, read::{read_formulas, tmp_path}}, package::{formula::Formula, package::{InstallReason, InstalledPackage, InstalledPackageRecord, PackageCache, PackageVersion}}, stage::{download, link, probe, resolve, unpack, verify}, ui::{event::ItemEvent, with_progess_bar, with_progess_multibar}};

use crate::{command::PbStyle, config::Config, ACTIVE_PB};

use super::QueryArgs;

#[derive(Debug, Default)]
struct InstallPlan {
  packages: Vec<PlannedPackage>,
  skipped_dependencies: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlanAction {
  Install,
  Upgrade,
  Reinstall,
}

#[derive(Debug, Clone)]
struct PlannedPackage {
  package: PackageVersion,
  action: PlanAction,
  requested: bool,
  installed_version: Option<String>,
}

fn requested_package_names(formulas: &[Formula], query: &[String]) -> Result<HashSet<String>> {
  let mut formula_index = formulas.iter().map(|formula| (formula.name.as_str(), formula)).collect::<HashMap<_, _>>();
  formula_index.extend(formulas.iter().flat_map(|formula| formula.oldname.iter().map(move |name| (name.as_str(), formula))));
  formula_index.extend(formulas.iter().flat_map(|formula| formula.oldnames.iter().map(move |name| (name.as_str(), formula))));
  formula_index.extend(formulas.iter().flat_map(|formula| formula.aliases.iter().map(move |name| (name.as_str(), formula))));
  formula_index.extend(formulas.iter().map(|formula| (formula.full_name.as_str(), formula)));

  query.iter()
    .map(|name| {
      formula_index
        .get(name.as_str())
        .map(|formula| formula.name.clone())
        .ok_or_else(|| anyhow::anyhow!("package not found: {name}"))
    })
    .collect()
}

fn plan_packages(
  resolved: &[PackageVersion],
  requested_names: &HashSet<String>,
  installed: &HashMap<String, InstalledPackageRecord>,
) -> InstallPlan {
  let mut plan = InstallPlan::default();
  let mut seen = HashSet::new();

  for package in resolved {
    if !seen.insert(package.name.clone()) {
      continue;
    }

    let is_requested = requested_names.contains(&package.name);
    let version = package.version_full();
    let installed_version = installed.get(&package.name).map(|record| record.version.clone());
    let status = db::version_status(installed_version.as_deref(), &version);

    if !is_requested && status == InstalledVersionStatus::Satisfied {
      plan.skipped_dependencies.push(package.name.clone());
      continue;
    }

    let action = match status {
      InstalledVersionStatus::Missing => PlanAction::Install,
      InstalledVersionStatus::Satisfied => PlanAction::Reinstall,
      InstalledVersionStatus::Outdated => PlanAction::Upgrade,
    };

    plan.packages.push(PlannedPackage {
      package: package.clone(),
      action,
      requested: is_requested,
      installed_version,
    });
  }

  plan
}

fn review_plan<W: Write>(writer: &mut W, plan: &InstallPlan) -> std::io::Result<()> {
  writeln!(writer, "install plan:")?;
  for item in &plan.packages {
    let scope = if item.requested { "root" } else { "dep" };
    match item.action {
      PlanAction::Install => writeln!(writer, "  install   {scope} {} {}", item.package.name, item.package.version_full())?,
      PlanAction::Upgrade => writeln!(writer, "  upgrade   {scope} {} {} -> {}", item.package.name, item.installed_version.as_deref().unwrap_or("?"), item.package.version_full())?,
      PlanAction::Reinstall => writeln!(writer, "  reinstall {scope} {} {}", item.package.name, item.package.version_full())?,
    }
  }
  // if !plan.skipped_dependencies.is_empty() {
  //   writeln!(writer, "skip satisfied deps:")?;
  //   for name in &plan.skipped_dependencies {
  //     writeln!(writer, "  {name}")?;
  //   }
  // }
  Ok(())
}

fn prompt_yes_no<R: BufRead, W: Write>(reader: &mut R, writer: &mut W, prompt: &str) -> std::io::Result<bool> {
  loop {
    write!(writer, "{prompt}")?;
    writer.flush()?;

    let mut input = String::new();
    let bytes = reader.read_line(&mut input)?;
    if bytes == 0 {
      return Ok(false);
    }

    match input.trim().to_ascii_lowercase().as_str() {
      "" | "y" | "yes" => return Ok(true),
      "n" | "no" => return Ok(false),
      _ => writeln!(writer, "please answer y or n")?,
    }
  }
}

#[tracing::instrument(level = "debug", skip_all, fields(query = ?query.names, arch = %config.base.arch))]
pub async fn run(config: &Config, mirrors: &MirrorLists, query: QueryArgs) -> Result<bool> {
  let formulas = read_formulas(config.base.formula_json())?;
  let requested_names = requested_package_names(&formulas, &query.names)?;
  let installed = db::installed_index(&config.base.db)?;

  info!(message="resolve", ?query.names);
  let resolved = resolve::exec(
    &formulas,
    query.names.iter(),
    (),
  ).await.unwrap();

  let plan = plan_packages(&resolved.packages, &requested_names, &installed);
  if !plan.skipped_dependencies.is_empty() {
    info!(message="skip satisfied dependencies", skipped=plan.skipped_dependencies.join(","));
  }
  review_plan(&mut std::io::stderr(), &plan)?;
  if !prompt_yes_no(&mut std::io::BufReader::new(std::io::stdin()), &mut std::io::stderr(), "Proceed with download? [Y/n] ")? {
    eprintln!("aborted");
    return Ok(false);
  }

  let cached_pkg = config.base.cache_pkg();
  info!(message="probe", ?resolved.names, planned=plan.packages.iter().map(|i| i.package.name.as_str()).collect::<Vec<_>>().join(","));
  let urls = with_progess_bar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Items.style()),
    Some(ItemEvent::Init { max: plan.packages.len() }),
    |tracker| probe::exec(
      probe::Args::new(&config.base.arch, mirrors)
        .cache(&cached_pkg, false),
      plan.packages.iter().map(|item| &item.package).collect::<Vec<_>>(),
      tracker,
    ),
    (),
  ).await.unwrap();

  with_progess_multibar(
    ACTIVE_PB.clone(),
    PbStyle::Bytes.style().into(),
    |tracker| download::exec(
      mirrors,
      &cached_pkg,
      urls.iter().filter(|value| !value.cached).map(|value| (&value.pkg, &value.url)),
      tracker,
    ),
    (),
  ).await.unwrap();

  let mut cached = Vec::new();
  for i in &urls {
    let cache_pkg = cached_pkg.join(&i.pkg.filename);
    let cache_size = std::fs::metadata(&cache_pkg).map(|a| a.len()).unwrap();
    cached.push(PackageCache {
      name: i.pkg.name.clone(),
      cache_pkg,
      cache_size,
    });
  }

  info!(message="verify", cached.len=cached.len(), urls.len=urls.len());
  let failed = verify::exec(
    &cached_pkg,
    urls.iter().map(|a| (&a.pkg, &a.url, None)),
    (),
  ).await.unwrap();

  failed.iter().for_each(|i| {
    warn!(message="failed", name=%i.name, reason=%i.reason);
    eprintln!("file {} may be broken for package {}", i.file.display(), i.name);
    std::fs::rename(&i.file, tmp_path(&i.file, "broken")).ok();
  });
  assert!(failed.is_empty());

  let local_opt_dir = config.base.local_opt();
  let unpacked = with_progess_multibar(
    ACTIVE_PB.clone(),
    PbStyle::Bytes.style().into(),
    |tracker| unpack::exec(
      // TODO: force in args
      unpack::Args::new(&config.base.prefix, &local_opt_dir).force(true),
      &cached,
      tracker
    ),
    (),
  ).await.unwrap();
  unpacked.iter().for_each(|i| info!(message="unpacked", name=%i.name, dest=%i.dest.display()));

  let linked = with_progess_bar(
    ACTIVE_PB.clone(),
    PbStyle::Items.style().into(),
    ItemEvent::Init { max: unpacked.len() }.into(),
    |tracker| link::exec(
      &config.base.prefix,
      &unpacked,
      tracker,
    ),
    (),
  ).await.unwrap();
  linked.iter().for_each(|i| info!(message="linked", name=%i.name, version=%i.version));

  let package_index = resolved.packages.iter().map(|pkg| (pkg.name.as_str(), pkg)).collect::<HashMap<_, _>>();
  let unpacked_index = unpacked.iter().map(|pkg| (pkg.name.as_str(), pkg)).collect::<HashMap<_, _>>();
  for pkg in &linked {
    let meta = package_index.get(pkg.name.as_str()).unwrap();
    let reloc = unpacked_index.get(pkg.name.as_str())
      .map(|pkg| pkg.reloc.clone())
      .unwrap_or_default();
    let reason = installed.get(&pkg.name)
      .map(|installed| installed.reason)
      .unwrap_or_else(|| {
        if requested_names.contains(&pkg.name) {
          InstallReason::Explicit
        } else {
          InstallReason::Dependency
        }
      });
    db::write_installed(&config.base.db, &InstalledPackage {
      record: InstalledPackageRecord {
        name: pkg.name.clone(),
        version: pkg.version.clone(),
        desc: meta.desc.clone(),
        license: meta.license.clone(),
        deps: meta.deps.clone(),
        reason,
        install_date: db::now_unix(),
        dest: pkg.dest.clone(),
      },
      files: pkg.files.clone(),
      reloc,
    })?;
  }
  Ok(true)
}

#[cfg(test)]
mod tests {
  use std::{collections::{HashMap, HashSet}, path::PathBuf};

  use core_lib::package::package::{InstallReason, InstalledPackageRecord, PackageVersion};

  use std::io::Cursor;

  use super::{plan_packages, prompt_yes_no, review_plan, PlanAction};

  fn package(name: &str, version: &str, deps: &[&str]) -> PackageVersion {
    PackageVersion {
      name: name.to_string(),
      version: version.to_string(),
      revision: 0,
      desc: format!("{name} desc"),
      license: None,
      deps: deps.iter().map(|value| value.to_string()).collect(),
      prebuilds: Vec::new(),
      link_overwrite: Vec::new(),
    }
  }

  fn installed(name: &str, version: &str) -> InstalledPackageRecord {
    InstalledPackageRecord {
      name: name.to_string(),
      version: version.to_string(),
      desc: format!("{name} installed"),
      license: None,
      deps: Vec::new(),
      reason: InstallReason::Dependency,
      install_date: 0,
      dest: PathBuf::from(format!("/tmp/{name}")),
    }
  }

  #[test]
  fn skips_satisfied_dependencies_but_keeps_requested_roots() {
    let resolved = vec![
      package("foo", "1.0.0", &["bar"]),
      package("bar", "2.0.0", &[]),
    ];
    let requested = HashSet::from(["foo".to_string()]);
    let installed = HashMap::from([
      ("foo".to_string(), installed("foo", "1.0.0")),
      ("bar".to_string(), installed("bar", "2.0.0")),
    ]);

    let plan = plan_packages(&resolved, &requested, &installed);

    assert_eq!(plan.packages.iter().map(|pkg| pkg.package.name.as_str()).collect::<Vec<_>>(), vec!["foo"]);
    assert_eq!(plan.packages[0].action, PlanAction::Reinstall);
    assert_eq!(plan.skipped_dependencies, vec!["bar"]);
  }

  #[test]
  fn upgrades_outdated_dependencies_in_same_plan() {
    let resolved = vec![
      package("foo", "1.0.0", &["bar"]),
      package("bar", "2.0.0", &[]),
    ];
    let requested = HashSet::from(["foo".to_string()]);
    let installed = HashMap::from([
      ("bar".to_string(), installed("bar", "1.5.0")),
    ]);

    let plan = plan_packages(&resolved, &requested, &installed);

    assert_eq!(plan.packages.iter().map(|pkg| pkg.package.name.as_str()).collect::<Vec<_>>(), vec!["foo", "bar"]);
    assert_eq!(plan.packages[1].action, PlanAction::Upgrade);
    assert!(plan.skipped_dependencies.is_empty());
  }

  #[test]
  fn deduplicates_shared_dependencies_by_exact_name() {
    let resolved = vec![
      package("foo", "1.0.0", &["shared"]),
      package("bar", "1.0.0", &["shared"]),
      package("shared", "3.0.0", &[]),
      package("shared", "3.0.0", &[]),
    ];
    let requested = HashSet::from(["foo".to_string(), "bar".to_string()]);

    let plan = plan_packages(&resolved, &requested, &HashMap::new());

    assert_eq!(plan.packages.iter().map(|pkg| pkg.package.name.as_str()).collect::<Vec<_>>(), vec!["foo", "bar", "shared"]);
  }

  #[test]
  fn prompt_yes_by_default() {
    let mut input = Cursor::new("\n");
    let mut output = Vec::new();

    let confirmed = prompt_yes_no(&mut input, &mut output, "Proceed? [Y/n] ").unwrap();

    assert!(confirmed);
  }

  #[test]
  fn prompt_retries_until_valid_answer() {
    let mut input = Cursor::new("maybe\nn\n");
    let mut output = Vec::new();

    let confirmed = prompt_yes_no(&mut input, &mut output, "Proceed? [Y/n] ").unwrap();

    assert!(!confirmed);
    assert!(String::from_utf8(output).unwrap().contains("please answer y or n"));
  }

  #[test]
  fn review_plan_lists_actions() {
    let resolved = vec![
      package("foo", "1.0.0", &["bar"]),
      package("bar", "2.0.0", &[]),
    ];
    let requested = HashSet::from(["foo".to_string()]);
    let installed = HashMap::from([
      ("foo".to_string(), installed("foo", "1.0.0")),
    ]);
    let plan = plan_packages(&resolved, &requested, &installed);
    let mut output = Vec::new();

    review_plan(&mut output, &plan).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("reinstall root foo 1.0.0"));
    assert!(output.contains("install   dep bar 2.0.0"));
  }
}
