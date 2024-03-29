use std::path::{Path, PathBuf};

use crate::{error::{Error, ErrorExt, Result}, package::{mirror::MirrorServer, package::PkgBuild}, ui::{bar::FeedBar, EventListener}};

use super::http::DownloadTask;

pub struct MirrorLists {
  pub lists: Vec<MirrorServer>,
}

impl MirrorLists {
  pub fn url_iter<'a>(&'a self, req: FetchReq) -> Box<dyn Iterator<Item = (reqwest::Client, String)> + Send + 'a> {
    match req {
      FetchReq::Api(api) => {
        let iter = self.lists.iter().filter_map(move |i| i.api_url(&api).map(|u| (i.client(), u)));
        return Box::new(iter)
      },
      FetchReq::Package(pkg) => {
        let iter = self.lists.iter().map(move |i| (i.client(), i.package_url(&pkg)));
        return Box::new(iter)
      },
    }
  }
  pub fn len(&self) -> usize {
    self.lists.len()
  }
}

#[derive(Debug, Clone)]
pub enum FetchReq {
  Api(String),
  Package(PkgBuild),
}

impl FetchReq {
  pub fn target<P: AsRef<Path>>(&self, base_dir: P) -> PathBuf {
    match self {
      FetchReq::Api(api) => base_dir.as_ref().join(api),
      FetchReq::Package(build) => base_dir.as_ref().join(&build.filename),
    }
  }
}

impl std::fmt::Display for FetchReq {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      FetchReq::Api(api) => write!(f, "api: {}", api),
      FetchReq::Package(build) => write!(f, "package: {}", build.filename),
    }
  }
}


#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FetchState {
  pub current: u64,
  pub max: u64,
}

impl FeedBar for FetchState {
  fn message(&self) -> Option<String> { None }
  fn position(&self) -> Option<u64> { Some(self.current as _) }
  fn length(&self) -> Option<u64> { Some(self.max as _) }
}

/// download json api from https://formulae.brew.sh/api/formula.json
#[tracing::instrument(level = "debug", skip_all, fields(mirrors.len=mirrors.lists.len(), req = %req, path = %path.as_ref().to_string_lossy()))]
pub async fn fetch_remote<P: AsRef<Path>>(mirrors: &MirrorLists, req: FetchReq, path: P, tracker: impl EventListener<FetchState>) -> Result<()> {
  let filename = path.as_ref();
  if let Some(i) = path.as_ref().parent() {
    std::fs::create_dir_all(i).when(("create_dir_all", i))?;
  }
  let mut retrying = false;
  for (client, url) in mirrors.url_iter(req.clone()) {
    debug!(message="try mirror", url);
    if retrying {
      info!(url, "download failed, retrying");
    }
    let mut task = DownloadTask::new(url, filename, None)?;
    // TODO: keep partial download
    match task.client(Some(client)).force(true).run(|e| tracker.on_event(e)).await {
      Ok(state) => {
        tracker.on_event(state.clone());
        return Ok(())
      },
      Err(e) => {
        warn!(error=%e, message="download failed");
        retrying = true;
      }
    }
  }
  return Err(Error::MirrorFailed(req));
}

#[tokio::test]
async fn test_download_db() {
  use crate::tests::*;
  let active_pb = init_logger(None);

  // let url = "https://formulae.brew.sh/api/formula.json".to_string();
  let req = FetchReq::Api("formula.json".to_string());
  let target = req.target("cache");
  info!(%req, target=%target.display());
  let mirrors = get_mirrors();

  crate::ui::with_progess_bar(active_pb, None, None, |tracker| async {
    fetch_remote(&mirrors, FetchReq::Api("formula.json".to_string()), &target, tracker).await
  }, ()).await.unwrap();
  assert!(target.exists());
  info!(len=%std::fs::metadata(&target).unwrap().len());
  // std::fs::remove_file(target).unwrap();
}
