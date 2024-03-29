use anyhow::Result;
use core_lib::{io::{fetch::MirrorLists, read::read_formulas}, package::package::PackageCache, stage::{probe, resolve, unpack, verify}, ui::with_progess_multibar};

use crate::{command::PbStyle, config::Config, ACTIVE_PB};

use super::QueryArgs;

#[tracing::instrument(level = "debug", skip_all, fields(query = ?query.names, arch = %config.base.arch))]
pub async fn run(config: &Config, mirrors: &MirrorLists, query: QueryArgs) -> Result<()> {
  let formulas = read_formulas(config.base.formula_json())?;

  info!(message="resolve", ?query.names);
  let resolved = resolve::exec(
    &formulas,
    query.names.iter(),
    (),
  ).await.unwrap();

  info!(message="probe", ?resolved.names, resolved.len=resolved.packages.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(","));
  let urls = probe::exec(
    probe::Args::new(&config.base.arch, mirrors)
      .cache(&config.base.cache, false),
    &resolved.packages,
    (),
  ).await.unwrap();

  let mut cached = Vec::new();
  for i in &urls {
    let cache_pkg = config.base.cache.join(&i.pkg.filename);
    let cache_size = std::fs::metadata(&cache_pkg).map(|a| a.len()).unwrap();
    cached.push(PackageCache {
      name: i.pkg.name.clone(),
      cache_pkg,
      cache_size,
    });
  }

  info!(message="verify", cached.len=cached.len(), urls.len=urls.len());
  let failed = verify::exec(
    &config.base.cache,
    urls.iter().map(|a| (&a.pkg, &a.url, None)),
    (),
  ).await.unwrap();

  failed.iter().for_each(|i| warn!(message="failed", name=%i.name, reason=%i.reason));
  assert!(failed.is_empty());

  let cellar_dir = config.base.prefix.join("Cellar");
  let unpacked = with_progess_multibar(
    ACTIVE_PB.clone(),
    PbStyle::Bytes.style().into(),
    |tracker| unpack::exec(
      // TODO: force in args
      unpack::Args::new(&config.base.prefix, &cellar_dir).force(true),
      &cached,
      tracker
    ),
    (),
  ).await.unwrap();

  unpacked.iter().for_each(|i| info!(message="unpacked", name=%i.name, dest=%i.dest.display()));

  Ok(())
}
