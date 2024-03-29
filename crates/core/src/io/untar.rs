use std::{collections::BTreeMap, path::{Path, PathBuf}, sync::atomic::{AtomicU64, AtomicUsize}};

use async_compression::tokio::bufread::GzipDecoder;
use futures::StreamExt;

use crate::{error::{ErrorExt, Result}, ui::EventListener};

#[derive(Debug, Clone, Default)]
pub struct UnpackEvent {
  pub current_entry: Option<PathBuf>,
  pub pos: u64,
  pub total_size: u64,
  pub item_current: usize,
  pub item_count: usize,
}

pub trait Transformer<R> {
  type Reader: tokio::io::AsyncRead + Unpin;
  fn transform(&mut self, reader: R) -> Self::Reader;
}

impl<'a, R, DR, F> Transformer<R> for F
where
  R: tokio::io::AsyncBufRead + Unpin + 'a,
  DR: tokio::io::AsyncRead + Unpin + 'a,
  F: FnMut(R) -> DR,
{
  type Reader = DR;
  fn transform(&mut self, reader: R) -> Self::Reader {
    self(reader)
  }
}

pub struct GzipTransformer;
impl<R: tokio::io::AsyncBufRead + Unpin> Transformer<R> for GzipTransformer {
  type Reader = GzipDecoder<R>;

  fn transform(&mut self, reader: R) -> Self::Reader {
    GzipDecoder::new(reader)
  }
}

// TODO: futures::io::AsyncRead?
pub struct Archive<Reader, Decoder> {
  inner: Reader,
  decoder: Decoder,
}

impl<Reader, Decoder> Archive<Reader, Decoder>
where
  Reader: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin,
  Decoder: for<'a>Transformer<tokio::io::BufReader<&'a mut Reader>>,
{
  pub fn new(reader: Reader, decoder: Decoder) -> Self {
    Self { inner: reader, decoder }
  }

  pub async fn reset(&mut self) -> Result<()> {
    use tokio::io::AsyncSeekExt;
    self.inner.seek(std::io::SeekFrom::Start(0)).await.when(("Archive.reset", Path::new("")))?;
    Ok(())
  }

  pub async fn get<'a>(&'a mut self) -> Result<tokio_tar::Archive<<Decoder as Transformer<tokio::io::BufReader<&'a mut Reader>>>::Reader>> {
    self.reset().await?;
    let decoder = &mut self.decoder;
    let archive = tokio_tar::Archive::new(decoder.transform(tokio::io::BufReader::new(&mut self.inner)));
    Ok(archive)
  }
}

pub(crate) trait ArchiveExt {
  fn name(&self) -> &Path { Path::new("") }
  async fn uncompressed_size(self) -> Result<BTreeMap<PathBuf, u64>>;
  async fn unpack<P: AsRef<Path>>(self, dest: P, tracker: impl EventListener<(PathBuf, u64)>) -> Result<()>;
}

impl<R: tokio::io::AsyncRead + Unpin> ArchiveExt for tokio_tar::Archive<R> {
  async fn uncompressed_size(mut self) -> Result<BTreeMap<PathBuf, u64>> {
    let tar = self.name().to_path_buf();
    let mut entries = self.entries().when(("total_size.read_entries", &tar))?;
    let mut result = BTreeMap::new();
    while let Some(entry) = entries.next().await {
      let entry = entry.when(("total_size.get_entry", &tar))?;
      let entry_path = entry.path().when(("total_size.get_entry_path", &tar))?;
      if entry_path.as_os_str().as_encoded_bytes().ends_with(b"/") {
        continue;
      }
      result.insert(
        entry_path.into_owned(),
        entry.header().size().when(("total_size.get_entry_size", &tar))?,
      );
    }
    Ok(result)
  }

  async fn unpack<P: AsRef<Path>>(mut self, dest: P, tracker: impl EventListener<(PathBuf, u64)>) -> Result<()> {
    let tar = self.name().to_path_buf();
    tokio::fs::create_dir_all(dest.as_ref()).await.when(("create_dir_all", &tar))?;
    let dest = dest.as_ref();
    let mut entries = self.entries().when(("untar.read_entries", &tar))?;
    while let Some(entry) = entries.next().await {
      let mut entry = entry.when(("untar.get_entry", &tar))?;
      let entry_path = entry.path().when(("untar.get_entry_path", &tar))?.into_owned();
      let entry_size = entry.header().size().when(("total_size.get_entry_size", &tar))?;
      let path = dest.join(&entry_path);
      if entry_path.as_os_str().as_encoded_bytes().ends_with(b"/") {
        tokio::fs::create_dir_all(&path).await.when(("untar.create_dir_for_entry", &path))?;
        continue;
      }
      if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir).await.when(("untar.create_dir_all_for_entry", dir))?;
      }
      entry.unpack_in(dest).await.when(("untar.unpack_entry", &tar.join("!").join(&entry_path)))?;
      tracker.on_event((entry_path, entry_size))
    }
    Ok(())
  }
}


pub async fn untar_gz<P1: AsRef<Path>, P2: AsRef<Path>>(tar: P1, dest: P2, tracker: impl EventListener<UnpackEvent>) -> Result<(usize, u64)> {
  let file = tokio::fs::File::open(tar.as_ref()).await.when(("untar.open", tar.as_ref()))?;
  let mut archive = Archive::new(file, GzipTransformer);
  let entries_size = archive.get().await?.uncompressed_size().await?;
  let total_size = entries_size.values().sum();
  let item_count = entries_size.len();
  tracker.on_event(UnpackEvent { current_entry: None, pos: 0, total_size, item_current: 0, item_count });
  debug!(total_size, item_count, "start unpack");
  let pos = AtomicU64::new(0);
  let idx = AtomicUsize::new(0);
  archive.get().await?.unpack(dest, |(entry, size)| {
    let pos = pos.fetch_add(size, std::sync::atomic::Ordering::AcqRel);
    let idx = idx.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    tracker.on_event(UnpackEvent { current_entry: Some(entry), pos: pos+size, total_size, item_current: idx+1, item_count })
  }).await?;
  Ok((idx.load(std::sync::atomic::Ordering::Acquire), pos.load(std::sync::atomic::Ordering::Acquire)))
}

#[tokio::test]
async fn test_untar() {
  use crate::tests::*;
  init_logger(None);
  let tar = Path::new("cache/wget-1.24.5.arm64_sonoma.bottle.tar.gz");
  let dest = Path::new("cache/wget-1.24.5");
  untar_gz(tar, dest, |e| debug!(?e)).await.unwrap();
  assert!(dest.exists());
}
