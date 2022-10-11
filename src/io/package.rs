use std::{fs::File, path::Path, sync::{Arc, Mutex}};

use flate2::read::GzDecoder;
use tar::Archive;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("read package: io {0:?}")]
  Io(#[from] std::io::Error),
  #[error("read package: utf8")]
  Utf8,
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

pub struct PackageArchive {
  inner: Arc<Mutex<Archive<GzDecoder<File>>>>,
}

impl PackageArchive {
  pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
    let file = File::open(path)?;
    let archive = Archive::new(GzDecoder::new(file));
    Ok(Self { inner: Arc::new(Mutex::new(archive)) })
  }

  pub fn entries(&self) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for entry in self.inner.lock().expect("lock").entries()? {
      result.push(entry?.path()?.to_str().ok_or_else(|| Error::Utf8)?.to_string())
    }
    Ok(result)
  }
}
