use super::package::{PkgBuild, PackageOffline};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorType {
  Ghcr, Oci, Bottle,
}

pub struct MirrorServer {
  pub server_type: MirrorType,
  pub base_url: String,
}

impl MirrorServer {
  pub fn new(server_type: MirrorType, base_url: &str) -> Self {
    if server_type == MirrorType::Ghcr {
      warn!("should not use ghcr with custom base_url, please use MirrorServer::ghcr() instead");
    }
    Self { server_type, base_url: base_url.to_string() }
  }
  pub fn ghcr() -> Self {
    Self {
      server_type: MirrorType::Ghcr,
      base_url: "https://ghcr.io/v2/homebrew/core".to_string(),
    }
  }

  pub fn api_formula(&self) -> Option<String> {
    match self.server_type {
      MirrorType::Ghcr => Some("https://formulae.brew.sh/api/formula.json".to_string()),
      MirrorType::Oci => None,
      MirrorType::Bottle => Some(format!("{}/api/formula.json", self.base_url)),
    }
  }

  pub fn package_url(&self, info: &PackageOffline, arch: &PkgBuild) -> String {
    match self.server_type {
      MirrorType::Oci | MirrorType::Ghcr => format!("{}/{}/blobs/sha256:{}", self.base_url, info.name.replace("@", "/").replace("+", "x"), arch.sha256),
      MirrorType::Bottle => format!("{}/{}", self.base_url, arch.filename),
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

  let mirror = MirrorServer {
    server_type: MirrorType::Ghcr,
    base_url: "https://ghcr.io/v2/homebrew/core".to_string(),
  };
  let packages = crate::io::read::read_formulas(crate::tests::FORMULA_FILE).unwrap().into_iter().map(PackageOffline::from).collect::<Vec<_>>();
  for package in &packages {
    for arch in &package.tar {
      let url = mirror.package_url(&package, &arch);
      assert_eq!(url, arch.url);
    }
  }
}
