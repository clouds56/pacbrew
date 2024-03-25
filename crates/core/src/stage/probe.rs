use std::path::Path;

use reqwest::header;

use crate::{error::{Error, ErrorExt, Result}, io::fetch::{FetchReq, MirrorLists}, package::package::{PackageOffline, PackageUrl, PkgBuild}, ui::EventListener};

use super::Event;

#[tracing::instrument(level = "trace", skip_all, fields(mirrors.len = mirrors.len(), package = %pkg.name, arch = %pkg.arch))]
pub async fn step(mirrors: &MirrorLists, pkg: &PkgBuild) -> Result<PackageUrl> {
  info!(?pkg);
  let req = FetchReq::Package(pkg.clone());
  for (client, url) in mirrors.url_iter(req.clone()) {
    let result: Result<_> = async move {
      let resp = client.head(&url).send().await.when(("head", &url))?;
      let size = resp.headers()
        .get(header::CONTENT_LENGTH).ok_or_else(|| Error::parse_response_error("head", &url, "CONTENT_LENGTH"))?
        .to_str().map_err(Error::parse_response("head", &url, "CONTENT_LENGTH.to_str"))?
        .parse::<u64>().map_err(Error::parse_response("head", &url, "CONTENT_LENGTH.parse"))?;
      Ok(PackageUrl {
        name: pkg.name.clone(),
        pkg_url: url.to_string(),
        pkg_size: size,
      })
    }.await;
    match result {
      Ok(url) => return Ok(url),
      _ => {
        warn!(?result, "failed to head");
      }
    }
  }
  Err(Error::MirrorFailed(req))
}

pub struct Value {
  pub pkg: PkgBuild,
  pub url: PackageUrl,
}

#[tracing::instrument(level = "debug", skip_all, fields(cache_dir = %cache_dir.as_ref().display(), arch = %arch))]
pub async fn exec<'a, P: AsRef<Path>, I: IntoIterator<Item = &'a PackageOffline> + Clone>(cache_dir: P, mirrors: &MirrorLists, arch: &str, packages: I, tracker: impl EventListener<Event>) -> Result<Vec<Value>> {
  let mut result = Vec::new();
  let urls = packages.clone().into_iter().map(|package| {
    package.find_arch(arch).ok_or_else(|| Error::package_arch_not_found(package, &arch))
  }).collect::<Result<Vec<_>, _>>()?;
  for (i, (info, pkg)) in packages.into_iter().zip(urls).enumerate() {
    tracker.on_event(Event { name: info.name.clone(), current: i, max: None });
    // TODO: check part?
    let target = cache_dir.as_ref().join(&pkg.filename);
    let url = if target.exists() {
      PackageUrl {
        name: info.name.clone(),
        pkg_url: target.to_string_lossy().to_string(),
        pkg_size: target.metadata().when(("metadata", &target))?.len(),
      }
    } else {
      step(mirrors, pkg).await?
    };
    result.push(Value {
      pkg: pkg.clone(),
      url,
    })
  }
  Ok(result)
}

#[tokio::test]
async fn test_probe() {
  use crate::tests::*;
  let arch = ARCH;
  let cache_path = CACHE_PATH;
  let active_pb = init_logger(None);
  let mirrors = get_mirrors();
  let query = ["llvm"];
  let formulas = crate::io::read::read_formulas(crate::tests::FORMULA_FILE).unwrap();
  let resolved = super::resolve::exec(&formulas, query, ()).await.unwrap().resolved;
  let result = crate::ui::with_progess_bar(active_pb, None, Event::new(resolved.len()), |tracker| async {
    exec(cache_path, &mirrors, arch, &resolved, tracker).await
  }, ()).await.unwrap();
  info!(len=result.len(), sum=result.iter().map(|i| i.url.pkg_size).sum::<u64>());
  assert_eq!(result.len(), resolved.len());
  assert_eq!(result.iter().map(|i| &i.url.name).collect::<Vec<_>>(), resolved.iter().map(|i| &i.name).collect::<Vec<_>>());
}
