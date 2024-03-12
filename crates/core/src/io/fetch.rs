
use std::path::PathBuf;
use crate::error::{Error, ErrorExt, Result};

use futures::StreamExt as _;
use reqwest::{IntoUrl, Url};
use tokio::io::AsyncWriteExt as _;

/// The download task would download url to filename, and verify sha256.
/// it would first download to filename.tmp, then rename to filename.
#[derive(Debug, Clone)]
pub struct DownloadTask {
  pub url: Url,
  pub filename: PathBuf,
  pub sha256: Option<String>,
  pub force: bool,
}

impl DownloadTask {
  pub fn new<U: IntoUrl, P: Into<PathBuf>>(url: U, filename: P, sha256: Option<String>) -> Result<Self> {
    let url = into_url(url)?;
    let filename = filename.into();
    Ok(Self { url, filename, sha256, force: false })
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

  pub async fn run(&self) -> Result<()> {
    if !self.force && self.filename.exists() {
      return Ok(())
    }
    let client = reqwest::Client::new();
    let resp = client.get(self.url.clone()).send().await.when(&self)?;
    let tmp_filename = self.tmp_file();
    let mut file = tokio::fs::File::create(&tmp_filename).await.when(("create", &tmp_filename))?;
    let mut stream = resp.bytes_stream();
    while let Some(bytes) = stream.next().await {
      let bytes = bytes.when(&self)?;
      file.write_all(&bytes).await.when(("write", &tmp_filename))?;
    }
    file.sync_all().await.when(("sync", &tmp_filename))?;
    tokio::fs::rename(&tmp_filename, &self.filename).await.when(("rename", &self.filename))?;
    Ok(())
  }
}

fn into_url(url: impl IntoUrl) -> Result<Url> {
  let url_string = url.as_str().to_string();
  url.into_url().map_err(|_| Error::MalformedUrl(url_string))
}

/// download json api from https://formulae.brew.sh/api/formula.json
pub async fn download_db<U: IntoUrl, P: Into<PathBuf>>(url: U, path: P) -> Result<()> {
  let url = url.as_str();
  let filename = path.into();
  info!("update formula db from {}", url);
  let mut task = DownloadTask::new(url, filename, None)?;
  task.force(true).run().await?;
  Ok(())
}

#[tokio::test]
async fn test_download_db() {
  download_db("https://formulae.brew.sh/api/formula.json", "formula.json").await.unwrap();
  assert!(std::path::Path::new("formula.json").exists());
  // std::fs::remove_file("formula.json").unwrap();
}
