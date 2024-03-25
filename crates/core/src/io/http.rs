
use std::path::{Path, PathBuf};
use crate::{error::{Error, ErrorExt, Result}, ui::{bar::FeedBar, EventListener}};

use futures::StreamExt as _;
use reqwest::{IntoUrl, Url};
use tokio::io::AsyncWriteExt as _;

use super::read::tmp_path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadState {
  pub current: u64,
  pub max: u64,
}

impl FeedBar for DownloadState {
  fn message(&self) -> Option<String> { None }
  fn position(&self) -> Option<u64> { Some(self.current as _) }
  fn length(&self) -> Option<u64> { Some(self.max as _) }
}

pub trait ErrorDownloadExt<T> {
  fn when_download(self, task: &DownloadTask) -> Result<T>;
}

impl<T> ErrorDownloadExt<T> for Result<T, reqwest::Error> {
  fn when_download(self, ctx: &DownloadTask) -> Result<T> {
    self.map_err(|error| Error::DownloadFailed { task: ctx.clone(), error })
  }
}

/// The download task would download url to filename, and verify sha256.
/// it would first download to filename.tmp, then rename to filename.
#[derive(Debug)]
pub struct DownloadTask {
  pub client: Option<reqwest::Client>,
  pub url: Url,
  pub filename: PathBuf,
  pub sha256: Option<String>,
  pub force: bool,
}

impl Clone for DownloadTask {
  fn clone(&self) -> Self {
    Self {
      client: self.client.clone(),
      url: self.url.clone(),
      filename: self.filename.clone(),
      sha256: self.sha256.clone(),
      force: self.force.clone(),
    }
  }
}

impl DownloadTask {
  pub fn new<U: IntoUrl, P: Into<PathBuf>>(url: U, filename: P, sha256: Option<String>) -> Result<Self> {
    let url = into_url(url)?;
    let filename = filename.into();
    Ok(Self { client: None, url, filename, sha256, force: false })
  }

  pub fn client(&mut self, client: Option<reqwest::Client>) -> &mut Self {
    self.client = client;
    self
  }

  pub fn force(&mut self, force: bool) -> &mut Self {
    self.force = force;
    self
  }

  #[tracing::instrument(level = "trace", skip_all, fields(url = %self.url.as_str(), path = %self.filename.to_string_lossy()))]
  pub async fn run(&self, tracker: impl EventListener<DownloadState>) -> Result<DownloadState> {
    if !self.force && self.filename.exists() {
      let length = self.filename.metadata().when(("metadata", &self.filename))?.len();
      return Ok(DownloadState { current: length, max: length })
    }
    let client = self.client.clone().unwrap_or_else(|| reqwest::Client::new());
    let resp = client.get(self.url.clone()).send().await.when_download(&self)?;
    let length = resp.content_length().unwrap_or(0);
    let mut partial_len = 0;
    let tmp_filename = tmp_path(&self.filename, ".part");
    debug!(message="download_to", tmp_filename=%tmp_filename.display());
    let mut file = tokio::fs::File::create(&tmp_filename).await.when(("create", &tmp_filename))?;
    let mut stream = resp.bytes_stream();
    while let Some(bytes) = stream.next().await {
      let bytes = bytes.when_download(&self)?;
      partial_len += bytes.len() as u64;
      file.write_all(&bytes).await.when(("write", &tmp_filename))?;
      // debug!(tracker=self.tracker.is_some(), partial_len);
      tracker.on_event(DownloadState { current: partial_len as u64, max: length });
    }
    file.sync_all().await.when(("sync", &tmp_filename))?;
    debug!(message="rename", from=%tmp_filename.display(), to=%self.filename.display());
    tokio::fs::rename(&tmp_filename, &self.filename).await.when(("rename", &self.filename))?;
    Ok(DownloadState { current: partial_len, max: length })
  }
}

fn into_url(url: impl IntoUrl) -> Result<Url> {
  let url_string = url.as_str().to_string();
  url.into_url().map_err(|_| Error::MalformedUrl(url_string))
}

/// download json api from https://formulae.brew.sh/api/formula.json
#[tracing::instrument(level = "debug", skip_all, fields(url = %url.as_str(), path = %path.as_ref().to_string_lossy()))]
pub async fn download_db<U: IntoUrl, P: AsRef<Path>>(url: U, path: P, tracker: impl EventListener<DownloadState>) -> Result<DownloadState> {
  let url = url.as_str();
  let filename = path.as_ref();
  let mut task = DownloadTask::new(url, filename, None)?;
  task.force(true).run(tracker).await
}

#[tokio::test]
async fn test_download_db() {
  let active_pb = crate::tests::init_logger(None);

  let url = std::env::var("TEST_DOWNLOAD_URL").unwrap_or("https://example.com".to_string());
  // let url = "https://formulae.brew.sh/api/formula.json".to_string();
  let target = url.rsplit('/').next().unwrap();

  crate::ui::with_progess_bar(active_pb, DownloadState::default(), |tracker| async {
    download_db(&url, target, tracker).await
  }, ()).await.unwrap();
  assert!(Path::new(target).exists());
  info!(len=%std::fs::metadata(target).unwrap().len());
  std::fs::remove_file(target).unwrap();
}
