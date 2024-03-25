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
  pub db: PathBuf,
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
