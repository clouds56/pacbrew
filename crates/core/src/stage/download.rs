use std::path::{Path, PathBuf};

use indicatif::ProgressStyle;

use crate::{error::{ErrorExt as _, Result}, io::{fetch::{fetch_remote, FetchReq, MirrorLists}, FetchState}, package::package::{PackageCache, PackageUrl, PkgBuild}, ui::{bar::{FeedBar, FeedMulti}, EventListener}};

#[derive(Clone, Debug)]
pub struct Event {
  pub name: Option<String>,
  pub current: Option<u64>,
  pub max: Option<u64>,
  pub finish: bool,
  pub overall: bool,
}

impl Event {
  pub fn new(max: usize) -> Self {
    Self { name: None, current: Some(0), max: Some(max as _), finish: false, overall: true }
  }
  pub fn task(name: &str, current: u64, max: Option<u64>) -> Self {
    Self { name: Some(name.to_string()), current: Some(current), max, finish: false, overall: false }
  }
  pub fn overall_setup(name: &str, max: u64) -> Self {
    Self { name: Some(name.to_string()), current:  None, max: Some(max as _), finish: false, overall: true }
  }
  pub fn overall_progress(current: u64, max: Option<u64>) -> Self {
    Self { name: None, current: Some(current), max, finish: false, overall: true }
  }
  pub fn finish(name: Option<&str>) -> Self {
    Self { name: name.map(|i| i.to_string()), current: None, max: None, finish: true, overall: name.is_none() }
  }
}

impl FeedBar for Event {
  fn style() -> Option<indicatif::ProgressStyle> {
    Some(ProgressStyle::default_bar().template("{msg} [{bar:40.cyan/blue}] {pos}/{len}").unwrap())
  }
  fn message(&self) -> Option<String> { self.name.clone() }
  fn position(&self) -> Option<u64> { self.current }
  fn length(&self) -> Option<u64> { self.max }
}

impl FeedMulti<String> for Event {
  fn graduate(&self) -> bool { self.finish }
  fn tag(&self) -> Option<String> {
    if self.overall {
      None
    } else {
      self.name.clone()
    }
  }
}

pub async fn step<P: AsRef<Path>>(mirrors: &MirrorLists, pkg: &PkgBuild, cache_path: P, tracker: impl EventListener<FetchState>) -> Result<PathBuf> {
  let req = FetchReq::Package(pkg.clone());
  let target = req.target(cache_path.as_ref());
  let _ = fetch_remote(mirrors, req, &target, tracker).await?;
  Ok(target)
}

#[tracing::instrument(level = "debug", skip_all, fields(cache_path = %cache_path.as_ref().display(), mirrors.len = mirrors.lists.len()))]
pub async fn exec<'a, P: AsRef<Path>, I: IntoIterator<Item = (&'a PkgBuild, &'a PackageUrl)>>(mirrors: &MirrorLists, pkgs: I, cache_path: P, tracker: impl EventListener<Event>) -> Result<Vec<PackageCache>> {
  let mut result = Vec::new();
  let pkgs = pkgs.into_iter().collect::<Vec<_>>();
  let mut total_size = 0;
  let mut downloaded_size = 0;
  for &(_, url) in &pkgs {
    total_size += url.pkg_size;
  }
  tracker.on_event(Event::overall_setup("now", total_size));
  for (i, (pkg, url)) in pkgs.into_iter().enumerate() {
    tracker.on_event(Event::overall_setup(&format!("now [{}] {}", i, pkg.name), total_size));
    tracker.on_event(Event::task(&pkg.filename, 0, Some(url.pkg_size)));
    let value = step(mirrors, pkg, &cache_path, |e: FetchState| {
      tracker.on_event(Event::task(&pkg.filename, e.current, Some(e.max)));
      tracker.on_event(Event::overall_progress(downloaded_size + e.current, None));
    }).await?;
    downloaded_size += url.pkg_size;
    tracker.on_event(Event::finish(Some(&pkg.filename)));
    let cache_size = std::fs::metadata(&value).when(("metadata", &value))?.len();
    if cache_size != url.pkg_size {
      warn!(cache_size, url.pkg_size, "size not match");
    }
    result.push(PackageCache {
      name: pkg.name.clone(),
      cache_pkg: value,
      cache_size,
    })
  }
  tracker.on_event(Event::finish(None));
  Ok(result)
}

#[tokio::test]
pub async fn test_download() {
  use crate::tests::*;
  let cache_path = CACHE_PATH;
  let arch = ARCH;
  std::fs::create_dir_all(cache_path).ok();
  let active_pb = crate::tests::init_logger(Some("warn"));
  let mirrors = get_mirrors();
  let query = ["wget"];
  let formulas = crate::io::read::read_formulas(crate::tests::FORMULA_FILE).unwrap();
  let resolved = super::resolve::exec(&formulas, query, ()).await.unwrap().packages;
  let urls = super::probe::exec(cache_path, &mirrors, arch, &resolved, ()).await.unwrap();
  warn!("start downloading");
  let result = crate::ui::with_progess_multibar(active_pb, None, |tracker| async {
    let tmp = urls.iter().map(|i| (&i.pkg, &i.url)).collect::<Vec<_>>();
    exec(&mirrors, tmp, cache_path, tracker).await
  }, ()).await.unwrap();
  info!(len=result.len());
  assert_eq!(result.len(), resolved.len());
}
