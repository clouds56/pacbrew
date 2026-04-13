use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use core_lib::db;
use core_lib::error::{ErrorExt, IoErrorExt};
use core_lib::package::package::{InstallReason, InstalledPackageRecord};

use crate::command::RemoveArgs;
use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoveReason {
  Requested,
  ForcedDependent,
  AutoPruned,
}

#[derive(Debug)]
struct RemovePlan {
  order: Vec<String>,
  reasons: HashMap<String, RemoveReason>,
}

pub fn run(config: &Config, args: RemoveArgs) -> Result<()> {
  if args.names.is_empty() {
    return Err(anyhow!("no package specified"));
  }

  let installed = db::installed_index(&config.base.db)?;
  if installed.is_empty() {
    return Err(anyhow!("no packages installed"));
  }

  let requested: HashSet<String> = args.names.into_iter().collect();
  let mut missing = requested
    .iter()
    .filter(|name| !installed.contains_key(*name))
    .cloned()
    .collect::<Vec<_>>();
  if !missing.is_empty() {
    missing.sort();
    return Err(anyhow!("package not installed: {}", missing.join(", ")));
  }

  let reverse = build_reverse_dependencies(&installed);
  let plan = plan_removals(&installed, &requested, &reverse, args.force)?;

  eprintln!("remove plan:");
  for name in &plan.order {
    let reason = match plan.reasons.get(name).copied().unwrap_or(RemoveReason::Requested) {
      RemoveReason::Requested => "requested",
      RemoveReason::ForcedDependent => "forced-dependent",
      RemoveReason::AutoPruned => "auto-pruned",
    };
    let version = installed.get(name).map(|pkg| pkg.version.as_str()).unwrap_or("?");
    eprintln!("  remove {:16} {} {}", reason, name, version);
  }

  for name in &plan.order {
    uninstall_installed_by_name(&config.base.db, &config.base.prefix, name)?;
  }

  Ok(())
}

fn plan_removals(
  installed: &HashMap<String, InstalledPackageRecord>,
  requested: &HashSet<String>,
  reverse: &HashMap<String, Vec<String>>,
  force: bool,
) -> Result<RemovePlan> {
  let mut reasons = requested
    .iter()
    .map(|name| (name.clone(), RemoveReason::Requested))
    .collect::<HashMap<_, _>>();
  let mut removal_set = requested.clone();

  if !force {
    let mut blockers = requested
      .iter()
      .flat_map(|target| {
        reverse
          .get(target)
          .into_iter()
          .flatten()
          .filter(|dependent| !requested.contains(*dependent))
          .map(move |dependent| format!("{target} required by {dependent}"))
      })
      .collect::<Vec<_>>();
    if !blockers.is_empty() {
      blockers.sort();
      return Err(anyhow!(
        "cannot remove due to reverse dependencies (use --force):\n{}",
        blockers.join("\n")
      ));
    }
  } else {
    let mut queue = requested.iter().cloned().collect::<VecDeque<_>>();
    while let Some(name) = queue.pop_front() {
      for dependent in reverse.get(&name).into_iter().flatten() {
        if removal_set.insert(dependent.clone()) {
          reasons.insert(dependent.clone(), RemoveReason::ForcedDependent);
          queue.push_back(dependent.clone());
        }
      }
    }
  }

  loop {
    let mut changed = false;
    for (name, record) in installed {
      if removal_set.contains(name) {
        continue;
      }
      if record.reason != InstallReason::Dependency {
        continue;
      }
      if has_remaining_dependents(name, reverse, &removal_set) {
        continue;
      }
      removal_set.insert(name.clone());
      reasons.entry(name.clone()).or_insert(RemoveReason::AutoPruned);
      changed = true;
    }
    if !changed {
      break;
    }
  }

  let order = removal_order(installed, &removal_set);
  Ok(RemovePlan { order, reasons })
}

pub(crate) fn build_reverse_dependencies(
  installed: &HashMap<String, InstalledPackageRecord>,
) -> HashMap<String, Vec<String>> {
  let mut reverse = installed
    .keys()
    .map(|name| (name.clone(), Vec::<String>::new()))
    .collect::<HashMap<_, _>>();

  for (name, record) in installed {
    for dependency in &record.deps {
      if let Some(dependents) = reverse.get_mut(dependency) {
        dependents.push(name.clone());
      }
    }
  }

  reverse
}

pub(crate) fn has_remaining_dependents(
  target: &str,
  reverse: &HashMap<String, Vec<String>>,
  removal_set: &HashSet<String>,
) -> bool {
  reverse
    .get(target)
    .into_iter()
    .flatten()
    .any(|dependent| !removal_set.contains(dependent))
}

pub(crate) fn removal_order(
  installed: &HashMap<String, InstalledPackageRecord>,
  removal_set: &HashSet<String>,
) -> Vec<String> {
  fn visit(
    name: &str,
    installed: &HashMap<String, InstalledPackageRecord>,
    removal_set: &HashSet<String>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    ordered: &mut Vec<String>,
  ) {
    if visited.contains(name) {
      return;
    }
    if !visiting.insert(name.to_string()) {
      return;
    }

    if let Some(record) = installed.get(name) {
      for dependency in &record.deps {
        if removal_set.contains(dependency) {
          visit(dependency, installed, removal_set, visiting, visited, ordered);
        }
      }
    }

    visiting.remove(name);
    if visited.insert(name.to_string()) {
      ordered.push(name.to_string());
    }
  }

  let mut ordered = Vec::new();
  let mut visiting = HashSet::new();
  let mut visited = HashSet::new();
  let mut sorted = removal_set.iter().cloned().collect::<Vec<_>>();
  sorted.sort();

  for name in sorted {
    visit(
      &name,
      installed,
      removal_set,
      &mut visiting,
      &mut visited,
      &mut ordered,
    );
  }

  ordered.reverse();
  ordered
}

pub(crate) fn unlink_owned_files(prefix: &Path, files: &[String]) -> core_lib::error::Result<()> {
  for rel in files {
    let path = prefix.join(rel);
    let metadata = match std::fs::symlink_metadata(&path).ok_not_found() {
      Ok(Some(metadata)) => metadata,
      Ok(None) => continue,
      Err(error) => {
        return Err(core_lib::error::Error::IoFailed {
          action: "symlink_metadata",
          filename: path,
          error,
        })
      }
    };

    if metadata.file_type().is_symlink() {
      std::fs::remove_file(&path)
        .ok_not_found_none()
        .when(("remove_file", &path))?;
      cleanup_empty_parents(prefix, &path)?;
    } else {
      warn!(path = %path.display(), "skip unlink non-symlink path");
    }
  }
  Ok(())
}

pub(crate) fn uninstall_installed_by_name(db_path: &Path, prefix: &Path, name: &str) -> Result<bool> {
  let Some(pkg) = db::remove_installed(db_path, name)? else {
    return Ok(false);
  };
  unlink_owned_files(prefix, &pkg.files)?;
  std::fs::remove_dir_all(&pkg.record.dest)
    .ok_not_found_none()
    .when(("remove_dir_all", &pkg.record.dest))?;
  Ok(true)
}

fn cleanup_empty_parents(prefix: &Path, path: &Path) -> core_lib::error::Result<()> {
  let mut current = path.parent();
  while let Some(dir) = current {
    if dir == prefix {
      break;
    }
    match std::fs::remove_dir(dir) {
      Ok(()) => {
        current = dir.parent();
      }
      Err(error)
        if matches!(
          error.kind(),
          std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
        ) =>
      {
        break;
      }
      Err(error) => {
        return Err(core_lib::error::Error::IoFailed {
          action: "remove_dir",
          filename: dir.to_path_buf(),
          error,
        })
      }
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::{build_reverse_dependencies, plan_removals, RemoveReason};
  use core_lib::package::package::{InstallReason, InstalledPackageRecord};

  fn installed(
    name: &str,
    deps: &[&str],
    reason: InstallReason,
  ) -> (String, InstalledPackageRecord) {
    (
      name.to_string(),
      InstalledPackageRecord {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        desc: String::new(),
        license: None,
        deps: deps.iter().map(|value| value.to_string()).collect(),
        reason,
        install_date: 0,
        dest: PathBuf::from(format!("/tmp/{name}")),
      },
    )
  }

  #[test]
  fn blocks_reverse_dependencies_without_force() {
    let installed = std::collections::HashMap::from([
      installed("a", &[], InstallReason::Explicit),
      installed("b", &["a"], InstallReason::Explicit),
    ]);
    let reverse = build_reverse_dependencies(&installed);
    let requested = std::collections::HashSet::from(["a".to_string()]);

    let err = plan_removals(&installed, &requested, &reverse, false)
      .expect_err("remove should fail without force");
    assert!(err.to_string().contains("required by b"));
  }

  #[test]
  fn force_recursively_removes_dependents() {
    let installed = std::collections::HashMap::from([
      installed("a", &[], InstallReason::Explicit),
      installed("b", &["a"], InstallReason::Explicit),
      installed("c", &["b"], InstallReason::Explicit),
    ]);
    let reverse = build_reverse_dependencies(&installed);
    let requested = std::collections::HashSet::from(["a".to_string()]);

    let plan = plan_removals(&installed, &requested, &reverse, true).unwrap();
    assert_eq!(plan.order.len(), 3);
    assert_eq!(plan.reasons.get("a"), Some(&RemoveReason::Requested));
    assert_eq!(plan.reasons.get("b"), Some(&RemoveReason::ForcedDependent));
    assert_eq!(plan.reasons.get("c"), Some(&RemoveReason::ForcedDependent));
  }

  #[test]
  fn auto_prune_dependency_orphans_only() {
    let installed = std::collections::HashMap::from([
      installed("app", &["dep", "leaf"], InstallReason::Explicit),
      installed("dep", &[], InstallReason::Dependency),
      installed("leaf", &[], InstallReason::Explicit),
    ]);
    let reverse = build_reverse_dependencies(&installed);
    let requested = std::collections::HashSet::from(["app".to_string()]);

    let plan = plan_removals(&installed, &requested, &reverse, true).unwrap();
    assert!(plan.reasons.contains_key("app"));
    assert_eq!(plan.reasons.get("dep"), Some(&RemoveReason::AutoPruned));
    assert!(!plan.reasons.contains_key("leaf"));
  }
}
