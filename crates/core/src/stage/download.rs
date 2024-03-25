use std::{path::{Path, PathBuf}, time::Duration};

use indicatif::ProgressStyle;

use crate::{error::Result, io::http::{self, DownloadState}, package::{mirror::MirrorServer, package::{PkgBuild, PackageUrl}}, ui::{bar::{FeedBar, FeedMulti}, EventListener}};

#[derive(Clone, Debug)]
pub struct Event {
  pub name: String,
  pub current: Option<u64>,
  pub max: Option<u64>,
  pub finish: bool,
  pub overall: bool,
}

impl Event {
  pub fn new(max: usize) -> Self {
    Self { name: String::new(), current: Some(0), max: Some(max as _), finish: false, overall: true }
  }
  pub fn task(name: &str, current: u64, max: Option<u64>) -> Self {
    Self { name: name.to_string(), current: Some(current), max, finish: false, overall: false }
  }
  pub fn overall(name: &str, current: usize, max: Option<usize>) -> Self {
    Self { name: name.to_string(), current: Some(current as _), max: max.map(|i| i as _), finish: false, overall: true }
  }
  pub fn finish(name: Option<&str>) -> Self {
    Self { name: name.unwrap_or("").to_string(), current: None, max: None, finish: true, overall: name.is_none() }
  }
}

impl FeedBar for Event {
  fn style() -> Option<indicatif::ProgressStyle> {
    Some(ProgressStyle::default_bar().template("{msg} [{bar:40.cyan/blue}] {pos}/{len}").unwrap())
  }
  fn message(&self) -> Option<String> { Some(self.name.clone()) }
  fn position(&self) -> Option<u64> { self.current }
  fn length(&self) -> Option<u64> { self.max }
}

impl FeedMulti<String> for Event {
  fn graduate(&self) -> bool { self.finish }
  fn tag(&self) -> Option<String> {
    if self.overall {
      None
    } else {
      Some(self.name.clone())
    }
  }
}

pub async fn step<P: AsRef<Path>>(client: reqwest::Client, pkg: &PkgBuild, cache_path: P, tracker: impl EventListener<DownloadState>) -> Result<PathBuf> {
  let target = cache_path.as_ref().join(&pkg.filename);
  let mut task = http::DownloadTask::new(&pkg.url, &target, Some(pkg.sha256.clone()))?;
  task.client(Some(client)).run(tracker).await?;
  tokio::time::sleep(Duration::from_millis(1000)).await;
  Ok(target)
}

#[tracing::instrument(level = "debug", skip_all, fields(cache_path = %cache_path.as_ref().display(), mirror = mirror.base_url))]
pub async fn exec<'a, P: AsRef<Path>, I: IntoIterator<Item = (&'a PkgBuild, &'a PackageUrl)>>(mirror: &MirrorServer, pkgs: I, cache_path: P, tracker: impl EventListener<Event>) -> Result<Vec<PathBuf>> {
  let mut result = Vec::new();
  let client = mirror.client();
  for (i, (pkg, url)) in pkgs.into_iter().enumerate() {
    tracker.on_event(Event::overall(&format!("now {}", url.name), i as _, None));
    tracker.on_event(Event::task(&pkg.filename, 0, Some(url.pkg_size)));
    let value = step(client.clone(), pkg, &cache_path, |e: DownloadState| tracker.on_event(Event::task(&pkg.filename, e.current, Some(e.max)))).await?;
    tracker.on_event(Event::finish(Some(&pkg.filename)));
    result.push(value)
  }
  Ok(result)
}

#[tokio::test]
pub async fn test_download() {
  use crate::tests::*;
  let cache_path = CACHE_PATH;
  let arch = ARCH;
  std::fs::create_dir_all(cache_path).ok();
  let active_pb = crate::tests::init_logger(Some("warn"));
  let mirror = MirrorServer::new(MIRROR.0, MIRROR.1);
  let query = ["wget"];
  let formulas = crate::io::read::read_formulas(crate::tests::FORMULA_FILE).unwrap();
  let resolved = super::resolve::exec(&formulas, query, ()).await.unwrap().resolved;
  let urls = super::probe::exec(cache_path, &mirror, arch, &resolved, ()).await.unwrap();
  warn!("start downloading");
  let result = crate::ui::with_progess_multibar(active_pb, Event::new(resolved.len()), |tracker| async {
    let tmp = urls.iter().map(|i| (&i.pkg, &i.url)).collect::<Vec<_>>();
    exec(&mirror, tmp, cache_path, tracker).await
  }, ()).await.unwrap();
  info!(len=result.len());
  assert_eq!(result.len(), resolved.len());
}
