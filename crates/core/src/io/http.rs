
use std::path::PathBuf;
use crate::{error::{Error, ErrorExt, Result}, ui::EventListener};

use futures::StreamExt as _;
use reqwest::{IntoUrl, Url};
use tokio::io::AsyncWriteExt as _;

use super::{fetch::FetchState, read::tmp_path};

pub trait ErrorDownloadExt<T> {
  fn when_download(self, task: &DownloadTask) -> Result<T>;
}

impl<T> ErrorDownloadExt<T> for Result<T, reqwest::Error> {
  fn when_download(self, ctx: &DownloadTask) -> Result<T> {
    self.map_err(|error| Error::HttpDownloadFailed { task: ctx.clone(), error })
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
  pub async fn run(&self, tracker: impl EventListener<FetchState>) -> Result<FetchState> {
    if !self.force && self.filename.exists() {
      let length = self.filename.metadata().when(("metadata", &self.filename))?.len();
      return Ok(FetchState { current: length, max: length })
    }
    let client = self.client.clone().unwrap_or_else(|| reqwest::Client::new());
    let resp = client.get(self.url.clone()).send().await.when_download(&self)?;
    if !resp.status().is_success() {
      info!(url=%self.url, filename=%self.filename.display(), status_code=?resp.status(), "request failed");
      return Err(std::io::Error::other(format!("download from {} failed with status {}", self.url, resp.status()))).when(("dowanlod", &self.filename))?;
    }
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
      tracker.on_event(FetchState { current: partial_len as u64, max: length });
    }
    file.sync_all().await.when(("sync", &tmp_filename))?;
    debug!(message="rename", from=%tmp_filename.display(), to=%self.filename.display());
    tokio::fs::rename(&tmp_filename, &self.filename).await.when(("rename", &self.filename))?;
    Ok(FetchState { current: partial_len, max: length })
  }
}

fn into_url(url: impl IntoUrl) -> Result<Url> {
  let url_string = url.as_str().to_string();
  url.into_url().map_err(|_| Error::MalformedUrl(url_string))
}
