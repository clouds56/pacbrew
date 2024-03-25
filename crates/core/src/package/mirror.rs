use super::package::{ArchUrl, PackageOffline};

pub enum MirrorType {
  Ghcr, Oci, Bottle,
}

pub struct MirrorServer {
  pub server_type: MirrorType,
  pub base_url: String,
}

impl MirrorServer {
  pub fn api_formula(&self) -> Option<String> {
    match self.server_type {
      MirrorType::Ghcr => Some("https://formulae.brew.sh/api/formula.json".to_string()),
      MirrorType::Oci => None,
      MirrorType::Bottle => Some(format!("{}/api/formula.json", self.base_url)),
    }
  }

  pub fn package_url(&self, info: &PackageOffline, arch: &ArchUrl) -> String {
    match self.server_type {
      MirrorType::Oci | MirrorType::Ghcr => format!("{}/{}/blobs/sha256:{}", self.base_url, info.name.replace("@", "/").replace("+", "x"), arch.sha256),
      MirrorType::Bottle => format!("{}/{}", self.base_url, arch.filename),
    }
  }
}

#[test]
fn test_mirror() {
  crate::tests::init_logger();

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
