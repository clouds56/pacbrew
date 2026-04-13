use anyhow::Result;
use std::collections::{HashMap, HashSet};

use core_lib::{db, io::{fetch::MirrorLists, read::{read_formulas, tmp_path}}, package::package::{InstallReason, InstalledPackage, InstalledPackageRecord, PackageCache}, stage::{link, probe, resolve, unpack, verify}, ui::{event::ItemEvent, with_progess_bar, with_progess_multibar}};

use crate::{command::PbStyle, config::Config, ACTIVE_PB};

use super::QueryArgs;

#[tracing::instrument(level = "debug", skip_all, fields(query = ?query.names, arch = %config.base.arch))]
pub async fn run(config: &Config, mirrors: &MirrorLists, query: QueryArgs) -> Result<()> {
  let formulas = read_formulas(config.base.formula_json())?;

  super::download::run(config, mirrors, query.clone()).await.ok();

  info!(message="resolve", ?query.names);
  let resolved = resolve::exec(
    &formulas,
    query.names.iter(),
    (),
  ).await.unwrap();

  let cached_pkg = config.base.cache_pkg();
  info!(message="probe", ?resolved.names, resolved=resolved.packages.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(","));
  let urls = probe::exec(
    probe::Args::new(&config.base.arch, mirrors)
      .cache(&cached_pkg, false),
    &resolved.packages,
    (),
  ).await.unwrap();

  let mut cached = Vec::new();
  for i in &urls {
    let cache_pkg = cached_pkg.join(&i.pkg.filename);
    let cache_size = std::fs::metadata(&cache_pkg).map(|a| a.len()).unwrap();
    cached.push(PackageCache {
      name: i.pkg.name.clone(),
      cache_pkg,
      cache_size,
    });
  }

  info!(message="verify", cached.len=cached.len(), urls.len=urls.len());
  let failed = verify::exec(
    &cached_pkg,
    urls.iter().map(|a| (&a.pkg, &a.url, None)),
    (),
  ).await.unwrap();

  failed.iter().for_each(|i| {
    warn!(message="failed", name=%i.name, reason=%i.reason);
    eprintln!("file {} may be broken for package {}", i.file.display(), i.name);
    std::fs::rename(&i.file, tmp_path(&i.file, "broken")).ok();
  });
  assert!(failed.is_empty());

  let local_opt_dir = config.base.local_opt();
  let unpacked = with_progess_multibar(
    ACTIVE_PB.clone(),
    PbStyle::Bytes.style().into(),
    |tracker| unpack::exec(
      // TODO: force in args
      unpack::Args::new(&config.base.prefix, &local_opt_dir).force(true),
      &cached,
      tracker
    ),
    (),
  ).await.unwrap();
  unpacked.iter().for_each(|i| info!(message="unpacked", name=%i.name, dest=%i.dest.display()));

  let linked = with_progess_bar(
    ACTIVE_PB.clone(),
    PbStyle::Items.style().into(),
    ItemEvent::Init { max: unpacked.len() }.into(),
    |tracker| link::exec(
      &config.base.prefix,
      &unpacked,
      tracker,
    ),
    (),
  ).await.unwrap();
  linked.iter().for_each(|i| info!(message="linked", name=%i.name, version=%i.version));

  let direct_names = resolved.names.iter().cloned().collect::<HashSet<_>>();
  let package_index = resolved.packages.iter().map(|pkg| (pkg.name.as_str(), pkg)).collect::<HashMap<_, _>>();
  for pkg in &linked {
    let meta = package_index.get(pkg.name.as_str()).unwrap();
    let reason = db::read_installed(&config.base.db, &pkg.name)?
      .map(|installed| installed.record.reason)
      .unwrap_or_else(|| {
        if direct_names.contains(&pkg.name) {
          InstallReason::Explicit
        } else {
          InstallReason::Dependency
        }
      });
    db::write_installed(&config.base.db, &InstalledPackage {
      record: InstalledPackageRecord {
        name: pkg.name.clone(),
        version: pkg.version.clone(),
        desc: meta.desc.clone(),
        license: meta.license.clone(),
        deps: meta.deps.clone(),
        reason,
        install_date: db::now_unix(),
        dest: pkg.dest.clone(),
      },
      files: pkg.files.clone(),
    })?;
  }
  Ok(())
}
