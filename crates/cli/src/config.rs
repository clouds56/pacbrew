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
  #[serde(default, skip_serializing_if = "LogConfig::is_empty")]
  pub log: LogConfig,
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
  pub fn cache_pkg(&self) -> PathBuf { self.cache.join("pkg") }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LogConfig {
  pub rust_log: Option<String>,
  pub file: Option<PathBuf>,
}

impl LogConfig {
  pub fn is_empty(&self) -> bool { self.rust_log.is_none() && self.file.is_none() }
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
