use super::formula::Formula;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Package {
  #[serde(flatten)]
  pub offline: PackageOffline,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ArchUrl {
  pub arch: String,
  pub url: String,
  pub sha256: String,
}

impl std::fmt::Debug for ArchUrl {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ArchUrl").field("arch", &self.arch).field("sha256", &self.sha256).finish()
  }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PackageOffline {
  pub name: String,
  pub version: String,
  pub revision: u32,
  pub desc: String,
  pub license: Option<String>,
  pub deps: Vec<String>,
  pub tar: Vec<ArchUrl>,
  pub link_overwrite: Vec<String>,
}

impl From<Formula> for PackageOffline {
  fn from(f: Formula) -> Self {
    let tar = f.bottle.get("stable").iter().flat_map(|i| &i.files)
      .map(|(arch, bottle)| ArchUrl { arch: arch.to_string(), url: bottle.url.clone(), sha256: bottle.sha256.clone() })
      .collect::<Vec<_>>();
    Self {
      name: f.name,
      version: f.versions.stable,
      revision: f.revision,
      desc: f.desc,
      license: f.license,
      deps: f.dependencies,
      tar,
      link_overwrite: f.link_overwrite,
    }
  }
}
