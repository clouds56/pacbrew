use anyhow::Result;
use core_lib::{io::{fetch::MirrorLists, read::{read_formulas, tmp_path}}, stage::{download, probe, resolve, verify}, ui::{event::{simplify_tracker, ItemEvent}, with_progess_bar, with_progess_multibar}};

use crate::{command::PbStyle, config::Config, ACTIVE_PB};

use super::QueryArgs;

#[tracing::instrument(level = "debug", skip_all, fields(query = ?query.names, arch = %config.base.arch))]
pub async fn run(config: &Config, mirrors: &MirrorLists, query: QueryArgs) -> Result<()> {
  let formulas = read_formulas(config.base.formula_json())?;

  info!(message="resolve", ?query.names);
  let resolved = with_progess_bar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Items.style()),
    Some(ItemEvent::Init { max: query.names.len() }),
    |tracker| resolve::exec(
      &formulas,
      query.names.iter(),
      tracker
    ),
    ()
  ).await.unwrap();

  info!(message="probe", ?resolved.names, resolved=resolved.packages.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(","));
  let urls = with_progess_bar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Items.style()),
    Some(ItemEvent::Init { max: resolved.packages.len() }),
    |tracker| probe::exec(
      probe::Args::new(&config.base.arch, mirrors)
        .cache(&config.base.cache, false),
      &resolved.packages,
      tracker
    ),
    (),
  ).await.unwrap();

  info!(message="download", urls.len=urls.len(), pkgs=urls.iter().map(|i| i.pkg.filename.as_str()).collect::<Vec<_>>().join(","));
  let cached = with_progess_multibar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Bytes.style()),
    |tracker| download::exec(
      mirrors,
      &config.base.cache,
      urls.iter().filter(|v| !v.cached).map(|i| (&i.pkg, &i.url)),
      tracker
    ),
    (),
  ).await.unwrap();
  cached.iter().for_each(|i| info!(message="download", name=%i.name, size=%i.cache_size, path=%i.cache_pkg.display()));

  info!(message="verify", cached.len=cached.len(), urls.len=urls.len());
  let failed = with_progess_bar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Items.style()),
    Some(ItemEvent::Init { max: cached.len() }),
    |tracker| verify::exec(
      &config.base.cache,
      urls.iter().map(|a| (&a.pkg, &a.url, None)),
      simplify_tracker(tracker)
    ),
    ()
  ).await.unwrap();

  failed.iter().for_each(|i| {
    warn!(message="failed", name=%i.name, reason=%i.reason);
    eprintln!("file {} may be broken for package {}", i.file.display(), i.name);
    std::fs::rename(&i.file, tmp_path(&i.file, "broken")).ok();
  });
  assert!(failed.is_empty());
  Ok(())
}
