use std::{path::Path, sync::{Arc, Mutex}, io::{Seek, SeekFrom, Read}, ops::DerefMut, fs::File};

use flate2::read::GzDecoder;
use indicatif::ProgressBar;
use tar::{Archive, Entry};

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("read package: io {0:?}")]
  Io(#[from] std::io::Error),
  #[error("read package: utf8")]
  Utf8,
  #[error("read package: prefix")]
  Prefix,
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

pub struct PackageArchive<R> {
  inner: Arc<Mutex<R>>,
}

impl PackageArchive<()> {
  pub fn open<P: AsRef<Path>>(path: P) -> Result<PackageArchive<File>> {
    let file = File::open(path)?;
    PackageArchive::new(file)
  }
}

impl<R: Read + Seek> PackageArchive<R> {
  pub fn new(file: R) -> Result<Self> {
    Ok(Self { inner: Arc::new(Mutex::new(file)) })
  }

  pub fn for_each<F: FnMut(Entry<GzDecoder<&mut R>>) -> Result<()>>(&self, mut f: F, skip_if_error: bool) -> Result<()> {
    let mut file = self.inner.lock().expect("lock");
    file.seek(SeekFrom::Start(0))?;
    let mut archive = Archive::new(GzDecoder::new(file.deref_mut()));
    for entry in archive.entries()? {
      if skip_if_error {
        if let Ok(entry) = entry {
          f(entry).ok();
        }
      } else {
        f(entry?)?;
      }
    }
    Ok(())
  }

  pub fn size(&self) -> Result<u64> {
    let mut size = 0;
    self.for_each(|entry| {
      size += entry.size(); Ok(())
    }, true)?;
    Ok(size)
  }

  pub fn entries(&self) -> Result<Vec<String>> {
    let mut result = Vec::new();
    self.for_each(|entry| {
      result.push(entry.path()?.to_str().ok_or_else(|| Error::Utf8)?.to_string());
      Ok(())
    }, false)?;
    Ok(result)
  }

  pub fn unpack_with_pb<P: AsRef<Path>>(&self, pb: &ProgressBar, prefix: &str, dst: P) -> Result<()> {
    let dst = dst.as_ref();
    self.for_each(|mut entry| {
      let path = entry.path()?.strip_prefix(prefix).map_err(|_| Error::Prefix)?.to_path_buf();
      entry.unpack(dst.join(path))?;
      pb.inc(entry.size());
      Ok(())
    }, false)?;
    Ok(())
  }
}
