use std::{collections::HashMap, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

use crate::{error::{ErrorExt, Result}, io::read::{read_toml, write_to_file, write_toml}, package::package::{InstalledPackage, InstalledPackageRecord}};

const LOCAL_DIR: &str = "local";
const RECORD_FILE: &str = "desc.toml";
const FILES_FILE: &str = "files.txt";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstalledVersionStatus {
  Missing,
  Satisfied,
  Outdated,
}

fn local_dir(root: &Path) -> PathBuf {
  root.join(LOCAL_DIR)
}

fn package_dir(root: &Path, name: &str, version: &str) -> PathBuf {
  local_dir(root).join(format!("{}-{}", name, version))
}

fn files_path(root: &Path, name: &str, version: &str) -> PathBuf {
  package_dir(root, name, version).join(FILES_FILE)
}

fn record_path(root: &Path, name: &str, version: &str) -> PathBuf {
  package_dir(root, name, version).join(RECORD_FILE)
}

pub fn now_unix() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

pub fn write_installed(root: &Path, package: &InstalledPackage) -> Result<()> {
  let local_root = local_dir(root);
  std::fs::create_dir_all(&local_root).when(("create_dir_all", &local_root))?;

  let prefix = format!("{}-", package.record.name);
  for entry in std::fs::read_dir(&local_root).when(("read_dir", &local_root))? {
    let entry = entry.when(("read_dir", &local_root))?;
    let path = entry.path();
    if !entry.file_type().when(("file_type", &path))?.is_dir() {
      continue;
    }
    let dir_name = entry.file_name().to_string_lossy().to_string();
    if dir_name.starts_with(&prefix) && dir_name != format!("{}{}", prefix, package.record.version) {
      std::fs::remove_dir_all(&path).when(("remove_dir_all", &path))?;
    }
  }

  let pkg_dir = package_dir(root, &package.record.name, &package.record.version);
  std::fs::create_dir_all(&pkg_dir).when(("create_dir_all", &pkg_dir))?;
  write_toml(record_path(root, &package.record.name, &package.record.version), &package.record, true)?;
  let files = if package.files.is_empty() {
    String::new()
  } else {
    format!("{}\n", package.files.join("\n"))
  };
  write_to_file(files_path(root, &package.record.name, &package.record.version), files.as_bytes(), true)?;
  Ok(())
}

pub fn list_installed(root: &Path) -> Result<Vec<InstalledPackageRecord>> {
  let local_root = local_dir(root);
  if !local_root.exists() {
    return Ok(Vec::new());
  }

  let mut result: Vec<InstalledPackageRecord> = Vec::new();
  for entry in std::fs::read_dir(&local_root).when(("read_dir", &local_root))? {
    let entry = entry.when(("read_dir", &local_root))?;
    let path = entry.path();
    if !entry.file_type().when(("file_type", &path))?.is_dir() {
      continue;
    }
    let record_file = path.join(RECORD_FILE);
    if !record_file.exists() {
      continue;
    }
    result.push(read_toml(&record_file)?);
  }
  result.sort_by(|left, right| left.name.cmp(&right.name));
  Ok(result)
}

pub fn installed_index(root: &Path) -> Result<HashMap<String, InstalledPackageRecord>> {
  Ok(list_installed(root)?
    .into_iter()
    .map(|record| (record.name.clone(), record))
    .collect())
}

pub fn version_status(installed_version: Option<&str>, candidate_version: &str) -> InstalledVersionStatus {
  match installed_version {
    None => InstalledVersionStatus::Missing,
    Some(version) if version == candidate_version => InstalledVersionStatus::Satisfied,
    Some(_) => InstalledVersionStatus::Outdated,
  }
}

pub fn read_installed(root: &Path, name: &str) -> Result<Option<InstalledPackage>> {
  let local_root = local_dir(root);
  if !local_root.exists() {
    return Ok(None);
  }

  for entry in std::fs::read_dir(&local_root).when(("read_dir", &local_root))? {
    let entry = entry.when(("read_dir", &local_root))?;
    let path = entry.path();
    if !entry.file_type().when(("file_type", &path))?.is_dir() {
      continue;
    }

    let Some(dir_name) = path.file_name().map(|value| value.to_string_lossy().to_string()) else {
      continue;
    };
    if !dir_name.starts_with(&format!("{}-", name)) {
      continue;
    }

    let record: InstalledPackageRecord = read_toml(path.join(RECORD_FILE))?;
    let files = std::fs::read_to_string(path.join(FILES_FILE))
      .map(|content| content.lines().map(|line| line.to_string()).collect())
      .unwrap_or_default();
    return Ok(Some(InstalledPackage { record, files }));
  }

  Ok(None)
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;
  use std::time::{SystemTime, UNIX_EPOCH};

  use crate::package::package::{InstallReason, InstalledPackage, InstalledPackageRecord};

  fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!(
      "pacbrew-db-test-{}-{}",
      std::process::id(),
      SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    ))
  }

  use super::{installed_index, list_installed, read_installed, version_status, write_installed, InstalledVersionStatus};

  #[test]
  fn test_db_roundtrip() {
    let root = temp_root();
    let package = InstalledPackage {
      record: InstalledPackageRecord {
        name: "wget".to_string(),
        version: "1.0.0".to_string(),
        desc: "desc".to_string(),
        license: Some("MIT".to_string()),
        deps: vec!["openssl@3".to_string()],
        reason: InstallReason::Explicit,
        install_date: 123,
        dest: PathBuf::from("/tmp/wget"),
      },
      files: vec!["bin/wget".to_string(), "opt/wget".to_string()],
    };

    write_installed(&root, &package).unwrap();
    let listed = list_installed(&root).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "wget");

    let loaded = read_installed(&root, "wget").unwrap().unwrap();
    assert_eq!(loaded.record.version, "1.0.0");
    assert_eq!(loaded.files, package.files);

    let index = installed_index(&root).unwrap();
    assert_eq!(index.get("wget").unwrap().version, "1.0.0");

    assert_eq!(version_status(None, "1.0.0"), InstalledVersionStatus::Missing);
    assert_eq!(version_status(Some("1.0.0"), "1.0.0"), InstalledVersionStatus::Satisfied);
    assert_eq!(version_status(Some("0.9.0"), "1.0.0"), InstalledVersionStatus::Outdated);

    std::fs::remove_dir_all(&root).ok();
  }
}
