use crate::{io::http::DownloadTask, package::package::PackageOffline};
use std::{path::{Path, PathBuf}, result::Result as StdResult};

pub type Result<T, E=Error> = StdResult<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("request {} failed to {}", .action, .url)]
  RequestFailed {
    action: &'static str,
    url: String,
    #[source]
    error: Option<reqwest::Error>,
  },
  #[error("parse response of {} {} failed when {} due to {inner}", .action, .url, .reason)]
  ResponseMalformed {
    action: &'static str,
    url: String,
    reason: String,
    #[source]
    inner: anyhow::Error,
  },
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
  #[error("serde_json: cannot {action} {expect_type} context: {context:?}, caused by: {error}")]
  SerdeJson {
    action: &'static str,
    expect_type: String,
    context: Option<String>,
    #[source]
    error: serde_json::Error,
  },
  #[error("serde_toml: cannot ser {expect_type}, caused by: {error}")]
  SerdeTomlSer {
    expect_type: String,
    #[source]
    error: toml::ser::Error,
  },
  #[error("serde_toml: cannot de {expect_type}, caused by: {error}")]
  SerdeTomlDe {
    expect_type: String,
    #[source]
    error: toml::de::Error,
  },
  #[error("malformed url {}", .0)]
  MalformedUrl(String),
  #[error("package not found: {} with {:?} in [{}]", .name, .arch, .avaliable.join(","))]
  PackageNotFound {
    name: String,
    arch: Option<String>,
    avaliable: Vec<String>,
  },
}

impl Error {
  pub fn package_not_found(package: &str) -> Self {
    Self::PackageNotFound { name: package.to_string(), arch: None, avaliable: vec![] }
  }
  pub fn package_arch_not_found(package: &PackageOffline, arch: &str) -> Self {
    Self::PackageNotFound { name: package.name.clone(), arch: Some(arch.to_string()), avaliable: package.prebuilds.iter().map(|i| i.arch.clone()).collect() }
  }
  pub fn parse_response<'a, E: Into<anyhow::Error>>(action: &'static str, url: &'a str, reason: &'a str) -> impl FnOnce(E) -> Self + 'a {
    move |e: E| Self::ResponseMalformed { action, url: url.to_string(), reason: reason.to_string(), inner: e.into() }
  }
  pub fn parse_response_error<'a>(action: &'static str, url: &'a str, reason: &'a str) -> Self {
    Self::ResponseMalformed { action, url: url.to_string(), reason: reason.to_string(), inner: anyhow::Error::msg("option") }
  }
}

pub trait ErrorExt<'a, T, E> {
  type Ctx: 'a;
  fn when(self, ctx: Self::Ctx) -> Result<T, Error>;
}

impl<'a, T> ErrorExt<'a, T, reqwest::Error> for StdResult<T, reqwest::Error> {
  type Ctx = (&'static str, &'a str);
  fn when(self, (action, url): Self::Ctx) -> Result<T> {
    self.map_err(|error| Error::RequestFailed { action, url: url.to_string(), error: Some(error) })
  }
}

impl<'a, T> ErrorExt<'a, T, std::io::Error> for StdResult<T, std::io::Error> {
  type Ctx = (&'static str, &'a Path);
  fn when(self, (action, filename): Self::Ctx) -> Result<T> {
    self.map_err(|error| Error::IoFailed { action, filename: filename.to_owned(), error })
  }
}

fn get_context(source: &str, line: usize, col: usize) -> Option<String> {
  let s = source.lines().skip(line.saturating_sub(1)).next()?;
  if s.len() > 100 {
    if col >= s.len() { return None }
    let start = col.saturating_sub(50);
    let end = col.saturating_add(50).min(s.len());
    Some(format!("{} ^^^here^^^ {}", &s[start..col], &s[col..end]))
  } else {
    Some(s.to_string())
  }
}

impl<'a, T> ErrorExt<'a, T, serde_json::Error> for StdResult<T, serde_json::Error> {
  type Ctx = (&'static str, &'a str, Option<&'a str>);
  fn when(self, (action, expect_type, source): Self::Ctx) -> Result<T, Error> {
    self.map_err(|error| {
      let context = source.and_then(|source| get_context(source, error.line(), error.column())).map(|i| i.to_string());
      Error::SerdeJson { action, expect_type: expect_type.to_string(), error, context }
    })
  }
}

impl<'a, T> ErrorExt<'a, T, toml::ser::Error> for StdResult<T, toml::ser::Error> {
  type Ctx = (&'static str, &'a str);
  fn when(self, (action, expect_type): Self::Ctx) -> Result<T, Error> {
    if action != "ser" { warn!(message="should be ser", action) }
    self.map_err(|error| Error::SerdeTomlSer { expect_type: expect_type.to_string(), error })
  }
}

impl<'a, T> ErrorExt<'a, T, toml::de::Error> for StdResult<T, toml::de::Error> {
  type Ctx = (&'static str, &'a str, Option<&'a str>);
  fn when(self, (action, expect_type, _): Self::Ctx) -> Result<T, Error> {
    if action != "de" { warn!(message="should be de", action) }
    self.map_err(|error| Error::SerdeTomlDe { expect_type: expect_type.to_string(), error })
  }
}
