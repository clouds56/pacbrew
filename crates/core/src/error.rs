use crate::io::fetch::DownloadTask;
use std::{path::{Path, PathBuf}, result::Result as StdResult};

pub type Result<T, E=Error> = StdResult<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("download from {} to {} failed, caused by: {error}", .task.url, .task.filename.to_string_lossy())]
  DownloadFailed {
    task: DownloadTask,
    #[source]
    error: reqwest::Error,
  },
  #[error("io failed when {} file {}, caused by: {error}", .action, .filename.to_string_lossy())]
  IoFailed {
    action: &'static str,
    filename: PathBuf,
    #[source]
    error: std::io::Error,
  },
  #[error("serde_json: expect {expect_type}, caused by: {error}")]
  SerdeJson {
    expect_type: String,
    #[source]
    error: serde_json::Error,
  },
  #[error("malformed url {}", .0)]
  MalformedUrl(String),
}

pub trait ErrorExt<'a, T, E> {
  type Ctx: 'a;
  fn when(self, ctx: Self::Ctx) -> Result<T, Error>;
}

impl<'a, T> ErrorExt<'a, T, reqwest::Error> for StdResult<T, reqwest::Error> {
  type Ctx = &'a DownloadTask;
  fn when(self, ctx: Self::Ctx) -> Result<T> {
    self.map_err(|error| Error::DownloadFailed { task: ctx.clone(), error })
  }
}

impl<'a, T> ErrorExt<'a, T, std::io::Error> for StdResult<T, std::io::Error> {
  type Ctx = (&'static str, &'a Path);
  fn when(self, (action, filename): Self::Ctx) -> Result<T> {
    self.map_err(|error| Error::IoFailed { action, filename: filename.to_owned(), error })
  }
}

impl<'a, T> ErrorExt<'a, T, serde_json::Error> for StdResult<T, serde_json::Error> {
  type Ctx = &'a str;
  fn when(self, expect_type: Self::Ctx) -> Result<T, Error> {
    self.map_err(|error| Error::SerdeJson { expect_type: expect_type.to_string(), error })
  }
}
