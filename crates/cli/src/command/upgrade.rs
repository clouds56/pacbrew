use std::collections::HashMap;

use anyhow::Result;
use core_lib::{db::{self, InstalledVersionStatus}, io::{fetch::MirrorLists, read::read_formulas}, package::package::PackageVersion};

use crate::config::Config;

use super::QueryArgs;

#[tracing::instrument(level = "debug", skip_all, fields(arch = %config.base.arch))]
pub async fn run(config: &Config, mirrors: &MirrorLists) -> Result<()> {
  let installed = db::list_installed(&config.base.db)?;
  let formulas = read_formulas(config.base.formula_json())?;
  let latest_versions: HashMap<_, _> = formulas.into_iter().map(|formula| {
    let package = PackageVersion::from(formula);
    let latest = package.version_full();
    (package.name, latest)
  }).collect();

  let outdated = installed.into_iter()
    .filter_map(|pkg| match latest_versions.get(&pkg.name) {
      Some(latest) if db::version_status(Some(&pkg.version), latest) == InstalledVersionStatus::Outdated => Some(pkg.name),
      _ => None,
    })
    .collect::<Vec<_>>();

  if outdated.is_empty() {
    eprintln!("no outdated packages");
    return Ok(());
  }

  eprintln!("upgrading {} package(s): {}", outdated.len(), outdated.join(", "));
  super::install::run(config, mirrors, QueryArgs { names: outdated }).await?;
  Ok(())
}
