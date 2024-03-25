use std::path::PathBuf;

use super::formula::Formula;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Package {
  pub base: PackageVersion,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub prebuild: Option<PkgBuild>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub url: Option<PackageUrl>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub cache: Option<PackageCache>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PkgBuild {
  pub name: String,
  pub arch: String,
  pub rebuild: u32,
  pub filename: String,
  pub url: String,
  pub sha256: String,
}

impl std::fmt::Debug for PkgBuild {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PkgBuild")
      .field("name", &self.name)
      .field("arch", &self.arch)
      .field("rebuild", &self.rebuild)
      .field("filename", &self.filename)
      // .field("url", &self.url)
      .field("sha256", &self.sha256)
      .finish()
  }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PackageVersion {
  pub name: String,
  pub version: String,
  pub revision: u32,
  pub desc: String,
  pub license: Option<String>,
  pub deps: Vec<String>,
  pub prebuilds: Vec<PkgBuild>,
  pub link_overwrite: Vec<String>,
}

impl From<Formula> for PackageVersion {
  fn from(f: Formula) -> Self {
    let version_full = Self::version_full_(&f.versions.stable, f.revision);
    let tar = f.bottle.get("stable").iter().flat_map(|i| i.files.iter().map(|(s, t)| (s, *i, t)))
      .map(|(arch, meta, bottle)|
        PkgBuild {
          name: f.name.clone(),
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
      prebuilds: tar,
      link_overwrite: f.link_overwrite,
    }
  }
}

impl PackageVersion {
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

  pub fn find_arch(&self, arch: &str) -> Option<&PkgBuild> {
    // TODO: arch to enum, and fallback
    self.prebuilds.iter().find(|i| i.arch == arch)
      .or_else(|| self.prebuilds.iter().find(|i| i.arch == "all"))
  }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PackageUrl {
  pub name: String,
  pub pkg_url: String,
  pub pkg_size: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PackageCache {
  pub name: String,
  pub cache_pkg: PathBuf,
  pub cache_size: u64,
}
