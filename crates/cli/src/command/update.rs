use anyhow::Result;
use core_lib::{io::{fetch::MirrorLists, FetchState}, stage::update_db, ui::with_progess_bar};

use crate::{command::PbStyle, config::Config, ACTIVE_PB};


#[tracing::instrument(level = "info", skip_all)]
pub async fn run(config: &Config, mirrors: &MirrorLists) -> Result<()> {
  with_progess_bar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Bytes.style()),
    FetchState::default(),
    |tracker| update_db::exec(mirrors, &config.base.cache, tracker),
    (),
  ).await.unwrap();
  info!("update formula.json success");
  Ok(())
}
