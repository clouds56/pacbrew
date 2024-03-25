use anyhow::Result;
use core_lib::{io::{fetch::MirrorLists, read::read_formulas}, stage::{self, download, probe, resolve}, ui::{with_progess_bar, with_progess_multibar}};

use crate::{command::PbStyle, config::Config, ACTIVE_PB};

use super::QueryArgs;

#[tracing::instrument(level = "info", skip_all, fields(formula = %config.base.formula_json().display(), query = ?query.names, arch = %config.base.arch))]
pub async fn run(config: &Config, mirrors: &MirrorLists, query: QueryArgs) -> Result<()> {
  let formulas = read_formulas(config.base.formula_json())?;

  info!(message="resolve", ?query.names);
  let resolved = with_progess_bar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Items.style()),
    Some(stage::Event::new(query.names.len())),
    |tracker| resolve::exec(&formulas, query.names.iter(), tracker),
    ()
  ).await.unwrap();

  info!(message="probe", ?resolved.names, resolved.len=resolved.packages.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(","));
  let urls = with_progess_bar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Items.style()),
    Some(stage::Event::new(resolved.packages.len())),
    |tracker| probe::exec(&config.base.cache, mirrors, &config.base.arch, &resolved.packages, tracker),
    (),
  ).await.unwrap();

  info!(message="probe", urls.len=urls.len(), pkgs=urls.iter().map(|i| i.pkg.filename.as_str()).collect::<Vec<_>>().join(","));
  with_progess_multibar(
    ACTIVE_PB.clone(),
    Some(PbStyle::Bytes.style()),
    |tracker| download::exec(mirrors, urls.iter().map(|i| (&i.pkg, &i.url)), &config.base.cache, tracker),
    (),
  ).await.unwrap();
  // TODO: verify
  Ok(())
}
