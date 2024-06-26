use std::path::Path;

use crate::{error::{Error, ErrorExt as _, Result}, io::{fetch::{fetch_remote, FetchReq, MirrorLists}, read::{read_formulas, tmp_path}, FetchState}, ui::{event::BytesEvent, EventListener}};

#[tracing::instrument(level = "debug", skip_all, fields(mirrors.len = mirrors.len()))]
pub async fn exec<P: AsRef<Path>>(mirrors: &MirrorLists, dest_dir: P, tracker: impl EventListener<BytesEvent>) -> Result<()> {
  // TODO: support formula.json.gz
  let req = FetchReq::Api("formula.json".to_string());
  let target = req.target(dest_dir);
  let tmp_file = tmp_path(&target, ".new");
  fetch_remote(mirrors, req, &tmp_file, |state: FetchState| tracker.on_event(BytesEvent::Progress { current: state.current, max: Some(state.max) })).await?;
  if !tmp_file.exists() {
    return Err(Error::parse_response_error("fetch", &tmp_file.display().to_string(), "not exists"));
  }
  let formulas = read_formulas(&tmp_file)?;
  if formulas.is_empty() {
    return Err(Error::parse_response_error("fetch", &tmp_file.display().to_string(), "empty"));
  }
  std::fs::rename(&tmp_file, &target).when(("rename", &target))?;
  Ok(())
}

#[tokio::test]
async fn test_update_formula() {
  use crate::tests::*;
  let active_pb = init_logger(None);

  let mirrors = get_mirrors();
  let target = Path::new(FORMULA_FILE);
  crate::ui::with_progess_bar(
    active_pb,
    None,
    None,
    |tracker| exec(&mirrors, CACHE_PATH, tracker),
    (),
  ).await.unwrap();
  assert!(target.exists());
  info!(len=%std::fs::metadata(&target).unwrap().len());
  // std::fs::remove_file(target).unwrap();
}
