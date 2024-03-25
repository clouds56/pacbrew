use std::path::Path;

use crate::{error::Result, package::{mirror::MirrorServer, package::PkgBuild}, ui::{bar::FeedBar, EventListener}};

use super::http::DownloadTask;

pub struct MirrorLists {
  pub lists: Vec<MirrorServer>,
}

#[derive(Debug, Clone)]
pub enum FetchReq {
  Api(String),
  Package(PkgBuild),
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
#[tracing::instrument(level = "debug", skip_all, fields(req = ?req, path = %path.as_ref().to_string_lossy()))]
pub async fn download_db<P: AsRef<Path>>(mirrors: &MirrorLists, req: FetchReq, path: P, tracker: impl EventListener<FetchState>) -> Result<FetchState> {
  let filename = path.as_ref();
  let url = "";
  let mut task = DownloadTask::new(url, filename, None)?;
  task.force(true).run(tracker).await
}

#[tokio::test]
async fn test_download_db() {
  let active_pb = crate::tests::init_logger(None);

  let url = std::env::var("TEST_DOWNLOAD_URL").unwrap_or("https://example.com".to_string());
  // let url = "https://formulae.brew.sh/api/formula.json".to_string();
  let target = url.rsplit('/').next().unwrap();

  let mirrors = MirrorLists {
    lists: vec![]
  };

  crate::ui::with_progess_bar(active_pb, FetchState::default(), |tracker| async {
    download_db(&mirrors, FetchReq::Api("formula.json".to_string()), target, tracker).await
  }, ()).await.unwrap();
  assert!(Path::new(target).exists());
  info!(len=%std::fs::metadata(target).unwrap().len());
  std::fs::remove_file(target).unwrap();
}
