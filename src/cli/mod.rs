use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

pub mod add;

pub type PackageInfos = BTreeMap<String, PackageInfo>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

pub struct PackageMeta {
  pub keg: String,
  pub depend: Vec<String>,
  pub required: Vec<String>,
  pub files: Vec<String>, // TODO mod?
  pub links: Vec<String>,
}
