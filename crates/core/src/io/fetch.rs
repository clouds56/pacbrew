
use std::path::{Path, PathBuf};
use crate::{error::{Error, ErrorExt, Result}, progress::{Events, Progress, ProgressTrack}};

use futures::StreamExt as _;
use reqwest::{IntoUrl, Url};
use tokio::{io::AsyncWriteExt as _, task::JoinHandle};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadState {
  pub current: u64,
  pub max: u64,
}

/// The download task would download url to filename, and verify sha256.
/// it would first download to filename.tmp, then rename to filename.
#[derive(Debug)]
pub struct DownloadTask {
  pub url: Url,
  pub filename: PathBuf,
  pub sha256: Option<String>,
  pub force: bool,
  pub tracker: Option<Progress<DownloadState>>,
}

impl Clone for DownloadTask {
  fn clone(&self) -> Self {
    Self {
      url: self.url.clone(),
      filename: self.filename.clone(),
      sha256: self.sha256.clone(),
      force: self.force.clone(),
      tracker: Some(Progress::new(Default::default())),
    }
  }
}

impl DownloadTask {
  pub fn new<U: IntoUrl, P: Into<PathBuf>>(url: U, filename: P, sha256: Option<String>) -> Result<Self> {
    let url = into_url(url)?;
    let filename = filename.into();
    Ok(Self { url, filename, sha256, force: false, tracker: Some(Progress::new(Default::default())) })
  }

  pub fn force(&mut self, force: bool) -> &mut Self {
    self.force = force;
    self
  }

  /// append .tmp suffix to self.filename
  pub fn tmp_file(&self) -> PathBuf {
    let mut tmp = self.filename.clone();
    let mut stem = tmp.file_name().unwrap_or_default().to_os_string();
    stem.push(".tmp");
    tmp.set_file_name(stem);
    tmp
  }

  #[tracing::instrument(level = "debug", skip_all, fields(url = %self.url.as_str(), path = %self.filename.to_string_lossy()))]
  pub async fn run(&self) -> Result<DownloadState> {
    if !self.force && self.filename.exists() {
      let length = self.filename.metadata().when(("metadata", &self.filename))?.len();
      return Ok(DownloadState { current: length, max: length })
    }
    let client = reqwest::Client::new();
    let resp = client.get(self.url.clone()).send().await.when(&self)?;
    let length = resp.content_length().unwrap_or(0);
    let mut partial_len = 0;
    let tmp_filename = self.tmp_file();
    let mut file = tokio::fs::File::create(&tmp_filename).await.when(("create", &tmp_filename))?;
    let mut stream = resp.bytes_stream();
    while let Some(bytes) = stream.next().await {
      let bytes = bytes.when(&self)?;
      partial_len += bytes.len() as u64;
      file.write_all(&bytes).await.when(("write", &tmp_filename))?;
      debug!(tracker=self.tracker.is_some(), partial_len);
      if let Some(tracker) = &self.tracker {
        tracker.send(DownloadState { current: partial_len as u64, max: length });
      }
    }
    file.sync_all().await.when(("sync", &tmp_filename))?;
    tokio::fs::rename(&tmp_filename, &self.filename).await.when(("rename", &self.filename))?;
    Ok(DownloadState { current: partial_len, max: length })
  }
}

fn into_url(url: impl IntoUrl) -> Result<Url> {
  let url_string = url.as_str().to_string();
  url.into_url().map_err(|_| Error::MalformedUrl(url_string))
}

/// download json api from https://formulae.brew.sh/api/formula.json
#[tracing::instrument(level = "debug", skip(url, path), fields(url = %url.as_str(), path = %path.as_ref().to_string_lossy()))]
pub async fn download_db<U: IntoUrl, P: AsRef<Path>>(url: U, path: P) -> Result<(JoinHandle<()>, Events<DownloadState>)> {
  let url = url.as_str();
  let filename = path.as_ref();
  let mut task = DownloadTask::new(url, filename, None)?;
  let events = task.tracker.as_ref().unwrap().progress();
  let handle = tokio::spawn(async move {
    let _ = task.force(true).run().await;
  });
  Ok((handle, events))
}

#[tokio::test]
async fn test_download_db() {
  crate::tests::init_logger();

  let target = "example.html";

  // let (handle, mut events) = download_db("https://formulae.brew.sh/api/formula.json", "formula.json").await.unwrap();
  let (handle, mut events) = download_db("https://example.com", target).await.unwrap();
  while let Some(event) = events.recv().await {
    info!(?event);
  }
  handle.await.unwrap();
  assert!(Path::new(target).exists());
  info!(len=%std::fs::metadata(target).unwrap().len());
  std::fs::remove_file(target).unwrap();
}
