use anyhow::Result;
use core_lib::{io::{fetch::{fetch_remote, FetchReq, MirrorLists}, FetchState}, ui::with_progess_bar};

use crate::{command::PbStyle, config::Config, ACTIVE_PB};


#[tracing::instrument(level = "info", skip_all)]
pub async fn run(config: &Config, mirrors: &MirrorLists) -> Result<()> {
  let req = FetchReq::Api("formula.json".to_string());
  let target = req.target(&config.base.cache);
  with_progess_bar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Bytes.style()),
    FetchState::default(),
    |tracker| fetch_remote(mirrors, req, &target, tracker),
    (),
  ).await.unwrap();
  info!("update formula.json success");
  Ok(())
}
