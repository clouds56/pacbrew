use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};

use crate::error::{ErrorExt, Result};

/// append `suffix` to `path`
pub fn tmp_path(path: &Path, suffix: &'static str) -> PathBuf {
  let mut tmp = path.to_owned();
  let mut stem = tmp.file_name().unwrap_or_default().to_os_string();
  stem.push(suffix);
  tmp.set_file_name(stem);
  tmp
}

pub fn read_json<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T> {
  let path = path.as_ref();
  let s = std::fs::read_to_string(path).when(("read", path))?;
  Ok(serde_json::from_str(&s).when(("de", std::any::type_name::<T>(), Some(&s)))?)
}

pub fn read_toml<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T> {
  let path = path.as_ref();
  let s = std::fs::read_to_string(path).when(("read", path))?;
  Ok(toml::from_str(&s).when(("de", std::any::type_name::<T>(), Some(&s)))?)
}


pub fn read_formulas<P: AsRef<Path>>(path: P) -> Result<Vec<crate::package::formula::Formula>> {
  read_json(path)
}

pub fn write_to_file<P: AsRef<Path>>(path: P, content: &[u8], force: bool) -> Result<u64> {
  let path = path.as_ref();
  if path.exists() && !force {
    return Err(crate::error::Error::IoFailed { action: "write", filename: path.to_owned(), error: std::io::Error::from(std::io::ErrorKind::AlreadyExists) });
  }
  let tmp_file = tmp_path(path, ".tmp");
  if tmp_file.exists() {
    warn!(message="tmp file already exists, would overwrite", tmp_file=%tmp_file.display());
  }
  std::fs::write(&tmp_file, content).when(("write", &tmp_file))?;
  std::fs::rename(&tmp_file, path).when(("rename", path))?;
  Ok(content.len() as _)
}

pub fn write_toml<T: Serialize, P: AsRef<Path>>(path: P, content: &T, force: bool) -> Result<u64> {
  let content = toml::to_string(content).when(("ser", std::any::type_name::<T>()))?;
  write_to_file(path, content.as_bytes(), force)
}
