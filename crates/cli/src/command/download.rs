use anyhow::Result;
use core_lib::{io::{fetch::MirrorLists, read::read_formulas}, stage::{download, probe, resolve, verify}, ui::{event::{simplify_tracker, ItemEvent}, with_progess_bar, with_progess_multibar}};

use crate::{command::PbStyle, config::Config, ACTIVE_PB};

use super::QueryArgs;

#[tracing::instrument(level = "info", skip_all, fields(formula = %config.base.formula_json().display(), query = ?query.names, arch = %config.base.arch))]
pub async fn run(config: &Config, mirrors: &MirrorLists, query: QueryArgs) -> Result<()> {
  let formulas = read_formulas(config.base.formula_json())?;

  info!(message="resolve", ?query.names);
  let resolved = with_progess_bar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Items.style()),
    Some(ItemEvent::Init { max: query.names.len() }),
    |tracker| resolve::exec(&formulas, query.names.iter(), tracker),
    ()
  ).await.unwrap();

  info!(message="probe", ?resolved.names, resolved.len=resolved.packages.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(","));
  let urls = with_progess_bar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Items.style()),
    Some(ItemEvent::Init { max: resolved.packages.len() }),
    |tracker| probe::exec(&config.base.cache, mirrors, &config.base.arch, &resolved.packages, tracker),
    (),
  ).await.unwrap();

  info!(message="probe", urls.len=urls.len(), pkgs=urls.iter().map(|i| i.pkg.filename.as_str()).collect::<Vec<_>>().join(","));
  let cached = with_progess_multibar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Bytes.style()),
    |tracker| download::exec(mirrors, urls.iter().map(|i| (&i.pkg, &i.url)), &config.base.cache, tracker),
    (),
  ).await.unwrap();
  cached.iter().for_each(|i| info!(message="download", name=%i.name, size=%i.cache_size, path=%i.cache_pkg.display()));

  info!(message="verify", cached.len=cached.len(), urls.len=urls.len());
  let failed = with_progess_bar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Items.style()),
    Some(ItemEvent::Init { max: cached.len() }),
    |tracker| verify::exec(cached.iter().zip(&urls).map(|(a, b)| (&b.pkg, &b.url, a)), simplify_tracker(tracker)),
    ()
  ).await.unwrap();

  failed.iter().for_each(|i| warn!(message="failed", name=%i.name, reason=%i.reason));
  assert!(failed.is_empty());
  Ok(())
}
