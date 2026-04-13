use std::{collections::{HashMap, HashSet}, path::Path};

use anyhow::Result;
use core_lib::{db, io::read::read_formulas, package::{formula::Formula, package::{InstallReason, InstalledPackage, InstalledPackageRecord, PackageVersion}}, stage::link};

use crate::config::Config;

use super::QueryArgs;

fn dependency_names(installed: &HashSet<String>, packages: &HashMap<String, PackageVersion>) -> HashSet<String> {
  let mut required = HashSet::new();
  for name in installed {
    let Some(package) = packages.get(name) else {
      continue;
    };
    for dep in &package.deps {
      if installed.contains(dep) {
        required.insert(dep.clone());
      }
    }
  }
  required
}

fn formula_index(formulas: Vec<Formula>) -> HashMap<String, PackageVersion> {
  let mut index = HashMap::new();
  for formula in formulas {
    let aliases = formula.aliases.clone();
    let oldnames = formula.oldnames.clone();
    let oldname = formula.oldname.clone();
    let full_name = formula.full_name.clone();
    let package = PackageVersion::from(formula);

    index.insert(package.name.clone(), package.clone());
    index.insert(full_name, package.clone());
    if let Some(oldname) = oldname {
      index.insert(oldname, package.clone());
    }
    for alias in aliases {
      index.insert(alias, package.clone());
    }
    for oldname in oldnames {
      index.insert(oldname, package.clone());
    }
  }
  index
}

fn install_date(path: &Path) -> u64 {
  path.metadata()
    .ok()
    .and_then(|meta| meta.created().ok().or_else(|| meta.modified().ok()))
    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
    .map(|duration| duration.as_secs())
    .unwrap_or_else(db::now_unix)
}

pub fn run(config: &Config, query: QueryArgs) -> Result<()> {
  let formula_path = config.base.formula_json();
  let packages = if formula_path.exists() {
    formula_index(read_formulas(&formula_path)?)
  } else {
    HashMap::new()
  };

  let installed = link::list_installed(&config.base.local_opt())?;
  let installed_names = installed.iter().map(|pkg| pkg.name.clone()).collect::<HashSet<_>>();
  let dependency_pkgs = dependency_names(&installed_names, &packages);
  let selected = query.names.into_iter().collect::<HashSet<_>>();
  let mut imported = 0usize;
  for pkg in installed {
    if !selected.is_empty() && !selected.contains(&pkg.name) {
      continue;
    }
    if db::read_installed(&config.base.db, &pkg.name)?.is_some() {
      continue;
    }

    let meta = packages.get(&pkg.name);
    db::write_installed(&config.base.db, &InstalledPackage {
      record: InstalledPackageRecord {
        name: pkg.name.clone(),
        version: pkg.version.clone(),
        desc: meta.map(|pkg| pkg.desc.clone()).unwrap_or_default(),
        license: meta.and_then(|pkg| pkg.license.clone()),
        deps: meta.map(|pkg| pkg.deps.clone()).unwrap_or_default(),
        reason: if dependency_pkgs.contains(&pkg.name) {
          InstallReason::Dependency
        } else {
          InstallReason::Explicit
        },
        install_date: install_date(&pkg.dest),
        dest: pkg.dest.clone(),
      },
      files: link::owned_files(&pkg.name, &pkg.dest).unwrap_or_else(|_| vec![format!("opt/{}", pkg.name)]),
    })?;
    imported += 1;
  }

  eprintln!("imported {} installed package(s) into {}", imported, config.base.db.display());
  Ok(())
}

#[cfg(test)]
mod tests {
  use std::collections::{HashMap, HashSet};

  use core_lib::package::package::PackageVersion;

  use super::dependency_names;

  #[test]
  fn dependency_reason_uses_installed_reverse_edges() {
    let installed = ["wget", "openssl@3", "sqlite"]
      .into_iter()
      .map(str::to_string)
      .collect::<HashSet<_>>();
    let packages = HashMap::from([
      (
        "wget".to_string(),
        PackageVersion {
          name: "wget".to_string(),
          version: "1.0.0".to_string(),
          revision: 0,
          desc: String::new(),
          license: None,
          deps: vec!["openssl@3".to_string()],
          prebuilds: vec![],
          link_overwrite: vec![],
        },
      ),
      (
        "openssl@3".to_string(),
        PackageVersion {
          name: "openssl@3".to_string(),
          version: "1.0.0".to_string(),
          revision: 0,
          desc: String::new(),
          license: None,
          deps: vec![],
          prebuilds: vec![],
          link_overwrite: vec![],
        },
      ),
      (
        "sqlite".to_string(),
        PackageVersion {
          name: "sqlite".to_string(),
          version: "1.0.0".to_string(),
          revision: 0,
          desc: String::new(),
          license: None,
          deps: vec![],
          prebuilds: vec![],
          link_overwrite: vec![],
        },
      ),
    ]);

    let required = dependency_names(&installed, &packages);
    assert!(required.contains("openssl@3"));
    assert!(!required.contains("wget"));
    assert!(!required.contains("sqlite"));
  }
}
