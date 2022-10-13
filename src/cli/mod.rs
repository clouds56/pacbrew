use std::{collections::BTreeMap, path::{PathBuf, Path}, sync::Arc};

use serde::{Deserialize, Serialize};

pub mod add;

pub type PackageInfos = BTreeMap<String, PackageInfo>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PackageInfo {
  // stage resolve
  pub name: String,
  pub version: String,
  pub version_full: String,
  // stage resolve_url
  pub arch: String,
  pub sha256: String,
  pub url: String,
  pub download_size: u64,
  pub size: u64,
  // stage download
  pub package_name: String,
  pub pacakge_path: PathBuf,
  #[serde(skip)]
  pub reason: Arc<Vec<String>>,
}

impl PackageInfo {
  fn new(v: String) -> Self {
    Self {
      name: v.clone(), version: String::new(), version_full: String::new(),
      arch: String::new(), sha256: String::new(), url: String::new(), download_size: 0, size: 0,
      package_name: String::new(), pacakge_path: PathBuf::new(),
      reason: Arc::new(vec![])
    }
  }
}

impl PackageInfo {
  pub fn with_dependencies(&self, names: &[String]) -> Vec<Self> {
    let mut reason = self.reason.to_vec();
    reason.push(self.name.to_string());
    let reason = Arc::new(reason);
    names.iter().map(|i| Self {
      reason: reason.clone(),
      ..Self::new(i.to_string())
    }).collect()
  }

  pub fn with_name(&self, name: String, version: String, revision: usize) -> Self {
    let version_full = if revision == 0 {
      version.clone()
    } else {
      format!("{}_{}", version, revision)
    };
    Self { name, version, version_full, ..self.clone() }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMeta {
  pub keg: String,
  pub explicit: bool,
  pub size: u64,
  pub unpacked_size: u64,
  pub depend: Vec<String>,
  pub required: Vec<String>,
  pub files: Vec<String>, // TODO mod?
  pub links: Vec<String>,
}

impl PackageMeta {
  pub fn new(keg: String) -> Self {
    Self { keg, explicit: false, size: 0, unpacked_size: 0, depend: Vec::new(), required: Vec::new(), files: Vec::new(), links: Vec::new() }
  }
}

pub fn load_package_info<P: AsRef<Path>>(path: P) -> anyhow::Result<(PackageInfo, PackageMeta)> {
  let path = path.as_ref();
  let s = std::fs::read_to_string(path)?;
  let info = toml::from_str::<PackageInfo>(&s)?;
  let s = std::fs::read_to_string(path.parent().expect("parent").join(&info.version_full).join("meta"))?;
  let meta = toml::from_str::<PackageMeta>(&s)?;
  Ok((info, meta))
}

pub fn save_package_info<P: AsRef<Path>>(path: P, info: &PackageInfo, meta: &PackageMeta) -> anyhow::Result<()> {
  let path = path.as_ref();
  let meta_version_path = path.parent().expect("parent").join(&info.version_full).join("meta");
  std::fs::create_dir_all(meta_version_path.parent().expect("parent"))?;
  let s = toml::to_string_pretty(&info)?;
  std::fs::write(path, s)?;
  let s = toml::to_string_pretty(&meta)?;
  std::fs::write(meta_version_path, s)?;
  Ok(())
}
