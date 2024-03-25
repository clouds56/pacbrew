use reqwest::header;

use crate::{error::{Error, ErrorExt, Result}, package::{mirror::MirrorServer, package::PackageOffline}};

#[tracing::instrument(level = "debug", skip_all, fields(mirror = mirror.base_url, package = %package.name, arch = %arch))]
pub async fn step(mirror: &MirrorServer, package: &PackageOffline, arch: String) -> Result<u64> {
  let pkg = package.find_arch(&arch).ok_or_else(|| Error::package_arch_not_found(package, &arch))?;
  info!(?pkg);
  let url = mirror.package_url(package, pkg);
  let resp = mirror.client().head(&url).send().await.when(("head", &url))?;
  let size = resp.headers()
    .get(header::CONTENT_LENGTH).ok_or_else(|| Error::parse_response_error("head", &url, "CONTENT_LENGTH"))?
    .to_str().map_err(Error::parse_response("head", &url, "CONTENT_LENGTH.to_str"))?
    .parse::<u64>().map_err(Error::parse_response("head", &url, "CONTENT_LENGTH.parse"))?;
  Ok(size)
}

#[tokio::test]
async fn test_probe() {
  crate::tests::init_logger();
  let mirror = MirrorServer::ghcr();
  let formulas = crate::io::read::read_formulas(crate::tests::FORMULA_FILE).unwrap();
  let resolved = super::resolve::exec(&formulas, ["llvm"], ()).await.unwrap().resolved;
  let size = step(&mirror, resolved.get(0).unwrap(), "arm64_sonoma".to_string()).await.unwrap();
  info!(size)
}
