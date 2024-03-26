use std::path::Path;

use reqwest::header;

use crate::{error::{Error, ErrorExt, Result}, io::fetch::{FetchReq, MirrorLists}, package::package::{PackageUrl, PackageVersion, PkgBuild}, ui::{event::ItemEvent, EventListener}};

#[tracing::instrument(level = "trace", skip_all, fields(mirrors.len = mirrors.len(), package = %pkg.name, arch = %pkg.arch))]
pub async fn step(mirrors: &MirrorLists, pkg: &PkgBuild) -> Result<PackageUrl> {
  trace!(?pkg);
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
  pub cached: bool,
}

pub struct Args<'a> {
  pub arch: &'a str,
  pub mirrors: &'a MirrorLists,
  pub cache_dir: Option<&'a Path>,
  pub filter_cached: bool,
}
impl<'a> Args<'a> {
  pub fn new(arch: &'a str, mirrors: &'a MirrorLists) -> Self {
    Self { arch, mirrors, cache_dir: None, filter_cached: false }
  }
  pub fn cache<P: AsRef<Path> + 'a>(mut self, cache_dir: &'a P, filter_cached: bool) -> Self {
    self.cache_dir = self.cache_dir.or(Some(cache_dir.as_ref()));
    self.filter_cached = filter_cached;
    self
  }
}

#[tracing::instrument(level = "debug", skip_all, fields(arch = %args.arch))]
pub async fn exec<'a, I>(
  args: Args<'_>,
  packages: I,
  tracker: impl EventListener<ItemEvent>
) -> Result<Vec<Value>>
where
  I: IntoIterator<Item = &'a PackageVersion> + Clone,
{
  let mut result = Vec::new();
  let urls = packages.clone().into_iter().map(|package| {
    package.find_arch(args.arch).ok_or_else(|| Error::package_arch_not_found(package, args.arch))
  }).collect::<Result<Vec<_>, _>>()?;
  for (i, (info, pkg)) in packages.into_iter().zip(urls).enumerate() {
    tracker.on_event(ItemEvent::Progress { current: i, max: None });
    tracker.on_event(ItemEvent::Message { name: format!("probing {}", info.name) });
    // TODO: check part?
    let (url, cached) = match args.cache_dir.map(|i| i.join(&pkg.filename)) {
      Some(target) if target.exists() => {
        if args.filter_cached { continue }
        (PackageUrl {
          name: info.name.clone(),
          pkg_url: target.to_string_lossy().to_string(),
          pkg_size: target.metadata().when(("metadata", &target))?.len(),
        }, true)
      },
      _ => (step(args.mirrors, pkg).await?, false),
    };
    result.push(Value {
      pkg: pkg.clone(),
      url,
      cached,
    })
  }
  tracker.on_event(ItemEvent::Message { name: format!("probe finished") });
  tracker.on_event(ItemEvent::Finish);
  Ok(result)
}

#[tokio::test]
async fn test_probe() {
  use crate::tests::*;
  let arch = ARCH;
  let cache_dir = CACHE_PATH;
  let active_pb = init_logger(None);
  let mirrors = get_mirrors();
  let query = ["llvm"];
  let formulas = crate::io::read::read_formulas(crate::tests::FORMULA_FILE).unwrap();
  let resolved = super::resolve::exec(&formulas, query, ()).await.unwrap().packages;
  let result = crate::ui::with_progess_bar(active_pb, None, Some(ItemEvent::Init { max: resolved.len() }), |tracker| async {
    exec(Args::new(arch, &mirrors).cache(&cache_dir, false), &resolved, tracker).await
  }, ()).await.unwrap();
  info!(len=result.len(), sum=result.iter().map(|i| i.url.pkg_size).sum::<u64>());
  assert_eq!(result.len(), resolved.len());
  assert_eq!(result.iter().map(|i| &i.url.name).collect::<Vec<_>>(), resolved.iter().map(|i| &i.name).collect::<Vec<_>>());
}
