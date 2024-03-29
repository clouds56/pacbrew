use std::path::PathBuf;

use core_lib::package::mirror::MirrorType;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Mirror {
  pub url: String,
  pub api_url: Option<String>,
  pub r#type: MirrorType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
  pub mirror_list: Vec<Mirror>,
  pub base: BaseConfig,
  #[serde(default)]
  pub network: NetworkConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BaseConfig {
  pub cache: PathBuf,
  pub prefix: PathBuf,
  #[serde(alias="cellar")]
  pub local_opt: Option<PathBuf>,
  pub db: PathBuf,
  pub arch: String,
}

impl BaseConfig {
  pub fn formula_json(&self) -> PathBuf { self.cache.join("formula.json") }
  pub fn local_opt(&self) -> PathBuf { self.local_opt.clone().unwrap_or_else(|| self.prefix.join("local").join("opt")) }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkConfig {
  #[serde(default = "retry_default")]
  pub retry: usize,
}

impl Default for NetworkConfig {
  fn default() -> Self {
    Self {
      retry: retry_default(),
    }
  }
}

const fn retry_default() -> usize { 5 }
