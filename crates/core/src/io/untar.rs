use std::path::Path;

use async_compression::tokio::bufread::GzipDecoder;
use futures::StreamExt;

use crate::error::{ErrorExt, Result};

// TODO: futures::io::AsyncRead?
pub struct Archive<Reader: tokio::io::AsyncRead + Unpin> {
  inner: tokio_tar::Archive<Reader>
}

impl<Reader: tokio::io::AsyncRead + Unpin> Archive<Reader> {
  pub fn new(reader: Reader) -> Self {
    Self {
      inner: tokio_tar::Archive::new(reader)
    }
  }

  pub async fn open<P: AsRef<Path>, Decoder>(decoder: Decoder, path: P) -> Result<Self>
  where
    Decoder: FnOnce(tokio::io::BufReader<tokio::fs::File>) -> Reader,
  {
    let file = tokio::fs::File::open(path.as_ref()).await.when(("open", path.as_ref()))?;
    Ok(Self::new(decoder(tokio::io::BufReader::new(file))))
  }
}

pub async fn untar<P1: AsRef<Path>, P2: AsRef<Path>, Decoder, Reader>(decoder: Decoder, tar: P1, dest: P2) -> Result<()>
where
  Reader: tokio::io::AsyncRead + Unpin,
  Decoder: FnOnce(tokio::io::BufReader<tokio::fs::File>) -> Reader,
{
  tokio::fs::create_dir_all(dest.as_ref()).await.when(("create_dir_all", dest.as_ref()))?;
  let tar = tar.as_ref();
  let dest = dest.as_ref();
  let mut archive = Archive::open(decoder, tar).await?;
  let mut entries = archive.inner.entries().when(("untar.read_entries", tar))?;
  while let Some(entry) = entries.next().await {
    let mut entry = entry.when(("untar.get_entry", tar))?;
    let entry_path = entry.path().when(("untar.get_entry_path", tar))?.into_owned();
    let path = dest.join(&entry_path);
    if entry_path.as_os_str().as_encoded_bytes().ends_with(b"/") {
      tokio::fs::create_dir_all(&path).await.when(("untar.create_dir_for_entry", &path))?;
      continue;
    }
    if let Some(dir) = path.parent() {
      tokio::fs::create_dir_all(dir).await.when(("untar.create_dir_all_for_entry", dir))?;
    }
    entry.unpack_in(dest).await.when(("untar.unpack_entry", &entry_path))?;
  }
  Ok(())
}

pub async fn untar_gz<P1: AsRef<Path>, P2: AsRef<Path>>(tar: P1, dest: P2) -> Result<()> {
  untar(GzipDecoder::new, tar, dest).await
}

#[tokio::test]
async fn test_untar() {
  use crate::tests::*;
  init_logger(None);
  let tar = Path::new("cache/wget-1.24.5.arm64_sonoma.bottle.tar.gz");
  let dest = Path::new("cache/wget-1.24.5");
  untar_gz(tar, dest).await.unwrap();
  assert!(dest.exists());
}
