use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

use crate::{error::{ErrorExt, Result}, package::package::{PackageCache, PackageUrl, PkgBuild}, ui::{event::{BytesEvent, DetailEvent, ItemEvent}, EventListener}};

pub struct Failed {
  pub name: String,
  pub reason: String,
}

#[tracing::instrument(level = "trace", skip_all, fields(cache_path = %cache_path.as_ref().display()))]
pub async fn step<P: AsRef<Path>>(cache_path: P, tracker: impl EventListener<u64>) -> Result<String> {
  let mut file = tokio::fs::File::open(cache_path.as_ref()).await.when(("open", cache_path.as_ref()))?;
  let mut hasher = Sha256::new();
  let mut buf = vec![0; 8*1024*1024];
  let mut total_size = 0;
  loop {
    let size = file.read(&mut buf).await.when(("read", cache_path.as_ref()))?;
    if size == 0 {
      break;
    }
    hasher.update(&buf[..size]);
    total_size += size as u64;
    tracker.on_event(total_size);
  }
  let hash = format!("{:x}", hasher.finalize());
  debug!(hash, cache_path=%cache_path.as_ref().display(), "hash");
  Ok(hash)
}

#[tracing::instrument(level = "debug", skip_all)]
pub async fn exec<'a, I: IntoIterator<Item = (&'a PkgBuild, &'a PackageUrl, &'a PackageCache)>>(pkg: I, tracker: impl EventListener<DetailEvent<usize, u64>>) -> Result<Vec<Failed>> {
  let mut result = Vec::new();
  for (i, (pkg, url, cached)) in pkg.into_iter().enumerate() {
    let mut reason = None;
    if pkg.name != url.name || pkg.name != cached.name {
      reason = Some("name not match");
      warn!(pkg.name, url.name, cached.name, "name not match");
    } else if cached.cache_size != url.pkg_size {
      reason = Some("size not match");
      warn!(pkg.name, cached.cache_size, url.pkg_size, "size not match");
    } else if !cached.cache_pkg.exists() || !cached.cache_pkg.is_file() {
      reason = Some("cache_pkg not exists");
      warn!(pkg.name, cached.cache_pkg=%cached.cache_pkg.display(), "not exists");
    }

    tracker.on_event(DetailEvent::Item(i, BytesEvent::Init { max: cached.cache_size }));
    let hash = step(&cached.cache_pkg, |pos| tracker.on_event(DetailEvent::Item(i, BytesEvent::Progress { current: pos, max: None }))).await?;
    tracker.on_event(DetailEvent::Item(i, BytesEvent::Finish));

    if hash != pkg.sha256 {
      reason = Some("hash not match");
      warn!(pkg.name, hash, pkg.sha256, "hash not match");
    }

    if let Some(reason) = reason {
      result.push(Failed {
        name: pkg.name.clone(),
        reason: reason.to_string(),
      });
    }
    tracker.on_event(DetailEvent::Overall(ItemEvent::Progress { current: i, max: None }));
    tracker.on_event(DetailEvent::Overall(ItemEvent::Message { name: format!("verfying {}", pkg.filename) }));
  }
  tracker.on_event(DetailEvent::Overall(ItemEvent::Message { name: "verify finished".to_string() }));
  tracker.on_event(DetailEvent::Overall(ItemEvent::Finish));

  Ok(vec![])
}

#[tokio::test]
async fn test_verify() {
  use crate::tests::*;
  let _active_pb = init_logger(None);

  let packages = get_formulas().into_iter()
    .map(crate::package::package::PackageVersion::from).collect::<Vec<_>>();
  let mut pkgs = Vec::new();
  for entry in std::fs::read_dir(CACHE_PATH).unwrap() {
    let entry = entry.unwrap();
    if !entry.file_type().unwrap().is_file() {
      continue;
    }
    let file_name = entry.file_name().to_string_lossy().to_string();
    if !file_name.ends_with(".bottle.tar.gz") {
      continue;
    }
    debug!(file_name, "reverse check");
    let pkg = packages.iter().filter(|p| file_name.starts_with(&p.name))
      .filter_map(|package| package.prebuilds.iter().find(|p| file_name == p.filename))
      .find(|_| true).unwrap();
    debug!(?pkg);
    let url = PackageUrl {
      name: pkg.name.clone(),
      pkg_url: file_name.clone(),
      pkg_size: entry.metadata().unwrap().len(),
    };
    let cache = PackageCache {
      name: pkg.name.clone(),
      cache_pkg: entry.path(),
      cache_size: entry.metadata().unwrap().len(),
    };
    pkgs.push((pkg, url, cache));
  }
  let iter = pkgs.iter().map(|i| (i.0, &i.1, &i.2));
  let result = exec(iter, ()).await.unwrap();
  assert!(result.is_empty());
}
