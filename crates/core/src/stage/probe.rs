use reqwest::header;

use crate::{error::{Error, ErrorExt, Result}, package::{mirror::MirrorServer, package::{ArchUrl, PackageOffline, PackageUrl}}, ui::EventListener};

use super::Event;

#[tracing::instrument(level = "debug", skip_all, fields(mirror = mirror.base_url, package = %info.name, arch = %pkg.arch))]
pub async fn step(mirror: &MirrorServer, info: &PackageOffline, pkg: &ArchUrl) -> Result<PackageUrl> {
  info!(?pkg);
  let url = mirror.package_url(info, pkg);
  let resp = mirror.client().head(&url).send().await.when(("head", &url))?;
  let size = resp.headers()
    .get(header::CONTENT_LENGTH).ok_or_else(|| Error::parse_response_error("head", &url, "CONTENT_LENGTH"))?
    .to_str().map_err(Error::parse_response("head", &url, "CONTENT_LENGTH.to_str"))?
    .parse::<u64>().map_err(Error::parse_response("head", &url, "CONTENT_LENGTH.parse"))?;
  Ok(PackageUrl {
    name: info.name.clone(),
    pkg_url: url.to_string(),
    pkg_size: size,
  })
}

pub struct Value {
  pub pkg: ArchUrl,
  pub url: PackageUrl,
}

pub async fn exec<'a, I: IntoIterator<Item = &'a PackageOffline> + Clone>(mirror: &MirrorServer, packages: I, arch: String, tracker: impl EventListener<Event>) -> Result<Vec<Value>> {
  let mut result = Vec::new();
  let urls = packages.clone().into_iter().map(|package| {
    package.find_arch(&arch).ok_or_else(|| Error::package_arch_not_found(package, &arch))
  }).collect::<Result<Vec<_>, _>>()?;
  for (i, (info, pkg)) in packages.into_iter().zip(urls).enumerate() {
    tracker.on_event(Event { name: info.name.clone(), current: i, max: None });
    let url = step(mirror, info, pkg).await?;
    result.push(Value {
      pkg: pkg.clone(),
      url,
    })
  }
  Ok(result)
}

#[tokio::test]
async fn test_probe() {
  let active_pb = crate::tests::init_logger(None);
  let mirror = MirrorServer::ghcr();
  let arch = "arm64_sonoma".to_string();
  let query = ["llvm"];
  let formulas = crate::io::read::read_formulas(crate::tests::FORMULA_FILE).unwrap();
  let resolved = super::resolve::exec(&formulas, query, ()).await.unwrap().resolved;
  let result = crate::ui::with_progess_bar(active_pb, Event::new(resolved.len()), |tracker| async {
    exec(&mirror, &resolved, arch, tracker).await
  }, ()).await.unwrap();
  info!(len=result.len(), sum=result.iter().map(|i| i.url.pkg_size).sum::<u64>());
  assert_eq!(result.len(), resolved.len());
  assert_eq!(result.iter().map(|i| &i.url.name).collect::<Vec<_>>(), resolved.iter().map(|i| &i.name).collect::<Vec<_>>());
}
