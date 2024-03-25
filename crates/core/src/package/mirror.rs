use super::package::PkgBuild;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MirrorType {
  Ghcr, Oci, Bottle,
}

pub struct MirrorServer {
  pub server_type: MirrorType,
  pub base_url: String,
  pub api_base_url: Option<String>,
}

impl MirrorServer {
  pub fn new(server_type: MirrorType, base_url: &str, api_base_url: Option<&str>) -> Self {
    if server_type == MirrorType::Ghcr {
      warn!("should not use ghcr with custom base_url, please use MirrorServer::ghcr() instead");
    }
    Self { server_type, base_url: base_url.to_string(), api_base_url: api_base_url.map(|s| s.to_string()) }
  }
  pub fn ghcr() -> Self {
    Self {
      server_type: MirrorType::Ghcr,
      api_base_url: Some("https://formulae.brew.sh/api/".to_string()),
      base_url: "https://ghcr.io/v2/homebrew/core/".to_string(),
    }
  }

  pub fn api_url(&self, target: &str) -> Option<String> {
    match (self.server_type, &self.api_base_url) {
      (_, Some(api_base_url)) => Some(format!("{}/{}", api_base_url.trim_end_matches('/'), target)),
      (MirrorType::Bottle, _) => Some(format!("{}/api/{}", self.base_url.trim_end_matches('/'), target)),
      _ => None
    }
  }

  pub fn package_url(&self, build: &PkgBuild) -> String {
    match self.server_type {
      MirrorType::Oci | MirrorType::Ghcr => format!("{}/{}/blobs/sha256:{}", self.base_url, build.name.replace("@", "/").replace("+", "x"), build.sha256),
      MirrorType::Bottle => format!("{}/{}", self.base_url, build.filename),
    }
  }

  pub fn client(&self) -> reqwest::Client {
    let builder = reqwest::Client::builder();
    let builder = match self.server_type {
      MirrorType::Ghcr => {
        use reqwest::header;
        let mut headers = header::HeaderMap::new();
        let mut auth_value = header::HeaderValue::from_static("Bearer QQ==");
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);
        builder
          .user_agent("pacbrew/0.1")
          .default_headers(headers)
        },
      MirrorType::Oci | MirrorType::Bottle => {
        builder
          .user_agent("Wget/1.21.3")
      },
    };
    builder.build().expect("build client")
  }
}

#[test]
fn test_mirror() {
  crate::tests::init_logger(None);

  let mirror = MirrorServer::ghcr();
  let packages = crate::io::read::read_formulas(crate::tests::FORMULA_FILE).unwrap()
    .into_iter().map(crate::package::package::PackageOffline::from).collect::<Vec<_>>();
  for package in &packages {
    for arch in &package.prebuilds {
      let url = mirror.package_url(&arch);
      assert_eq!(url, arch.url);
    }
  }
}
