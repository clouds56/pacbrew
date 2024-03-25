use std::path::{Path, PathBuf};

use crate::{error::Result, io::fetch::{self, DownloadState}, package::{mirror::MirrorServer, package::{ArchUrl, PackageUrl}}, stage::Event, ui::EventListener};

pub async fn step<P: AsRef<Path>>(mirror: &MirrorServer, pkg: &ArchUrl, cache_path: P, tracker: impl EventListener<DownloadState>) -> Result<PathBuf> {
  let target = cache_path.as_ref().join(&pkg.filename);
  let task = fetch::DownloadTask::new(&pkg.url, &target, Some(pkg.sha256.clone()))?;
  task.run(tracker).await?;
  Ok(target)
}

#[tracing::instrument(level = "debug", skip_all, fields(cache_path = %cache_path.as_ref().display(), mirror = mirror.base_url))]
pub async fn exec<'a, P: AsRef<Path>, I: IntoIterator<Item = (&'a ArchUrl, &'a PackageUrl)>>(mirror: &MirrorServer, pkgs: I, cache_path: P, tracker: impl EventListener<Event>) -> Result<Vec<PathBuf>> {
  let mut result = Vec::new();
  for (i, (pkg, url)) in pkgs.into_iter().enumerate() {
    tracker.on_event(Event { name: pkg.filename.clone(), current: i as _, max: None });
    let value = step(mirror, pkg, &cache_path, ()).await?;
    result.push(value)
  }
  Ok(result)
}

#[tokio::test]
pub async fn test_download() {
  let cache_dir = "cache";
  std::fs::create_dir_all(cache_dir).ok();
  let active_pb = crate::tests::init_logger();
  let mirror = MirrorServer::ghcr();
  let arch = "arm64_sonoma".to_string();
  let query = ["wget"];
  let formulas = crate::io::read::read_formulas(crate::tests::FORMULA_FILE).unwrap();
  let resolved = super::resolve::exec(&formulas, query, ()).await.unwrap().resolved;
  let urls = super::probe::exec(&mirror, &resolved, arch, ()).await.unwrap();
  let result = crate::ui::with_progess_bar(active_pb, Event::new(resolved.len()), |tracker| async {
    let tmp = urls.iter().map(|i| (&i.pkg, &i.url)).collect::<Vec<_>>();
    exec(&mirror, tmp, cache_dir, tracker).await
  }, ()).await.unwrap();
  info!(len=result.len());
  assert_eq!(result.len(), resolved.len());
}
