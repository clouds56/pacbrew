use super::formula::Formula;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Package {
  #[serde(flatten)]
  pub offline: PackageOffline,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ArchUrl {
  pub arch: String,
  pub rebuild: u32,
  pub filename: String,
  pub url: String,
  pub sha256: String,
}

impl std::fmt::Debug for ArchUrl {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ArchUrl")
      .field("arch", &self.arch)
      .field("rebuild", &self.rebuild)
      .field("filename", &self.filename)
      // .field("url", &self.url)
      .field("sha256", &self.sha256)
      .finish()
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
    let version_full = Self::version_full_(&f.versions.stable, f.revision);
    let tar = f.bottle.get("stable").iter().flat_map(|i| i.files.iter().map(|(s, t)| (s, *i, t)))
      .map(|(arch, meta, bottle)|
        ArchUrl {
          arch: arch.to_string(),
          rebuild: meta.rebuild,
          filename: if meta.rebuild == 0 {
            format!("{}-{}.{}.bottle.tar.gz", f.name, version_full, arch)
          } else {
            format!("{}-{}.{}.bottle.{}.tar.gz", f.name, version_full, arch, meta.rebuild)
          },
          url: bottle.url.clone(),
          sha256: bottle.sha256.clone()
        })
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

impl PackageOffline {
  pub fn version_full(&self) -> String {
    Self::version_full_(&self.version, self.revision)
  }

  pub fn version_full_(version: &str, revision: u32) -> String {
    if revision == 0 {
      version.to_string()
    } else {
      format!("{}_{}", version, revision)
    }
  }

  pub fn find_arch(&self, arch: &str) -> Option<&ArchUrl> {
    // TODO: arch to enum, and fallback
    self.tar.iter().find(|i| i.arch == arch)
  }
}
