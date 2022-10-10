use std::{collections::BTreeMap, path::Path};
use serde::{Deserialize, Serialize};

use crate::formula::Formula;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "os_name", rename_all="snake_case")]
pub enum Os {
  Macos { #[serde(rename="os_version")] version: String },
  Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum Arch {
  x86_64, arm64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum Channel {
  stable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
  #[serde(flatten)]
  pub os: Os,
  pub arch: Arch,
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub target: String,
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub cache_dir: String,
}

impl Config {
  pub fn build_target(os: &Os, arch: Arch) -> String {
    let target = match (os, arch) {
      (Os::Macos { version }, Arch::arm64) => format!("arm64_{}", version),
      (Os::Macos { version }, Arch::x86_64) => format!("{}", version),
      (Os::Linux, arch) => format!("{:?}_linux", arch),
    };
    target
  }

  pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    let s = std::fs::read_to_string(path)?;
    let mut result = toml::from_str::<Self>(&s)?;
    // TODO: figure out most thing with default value
    result.target = Config::build_target(&result.os, result.arch);
    if result.cache_dir.is_empty() {
      result.cache_dir = "/opt/homebrew/cache/pactree/packages".to_string();
    }
    Ok(result)
  }

  pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
    let s = toml::to_string_pretty(&self)?;
    std::fs::write(path, s)?;
    Ok(())
  }
}

pub struct PacTree {
  pub packages: BTreeMap<String, Formula>,
  pub aliases: BTreeMap<String, String>,
  pub config: Config,
}

impl PacTree {
  pub fn get_package(&self, name: &str) -> Option<&Formula> {
    if let Some(package) = self.packages.get(name) {
      return Some(package)
    }
    if let Some(package_name) = self.aliases.get(name) {
      if let Some(package) = self.packages.get(package_name) {
        return Some(package)
      }
    }
    None
  }
}

#[test]
fn test_config() {
  let config = Config {
    os: Os::Macos { version: "monterey".to_string() },
    arch: Arch::arm64,
    target: String::new(),
    cache_dir: String::new(),
  };

  config.save("cache/pactree.conf.old").expect("save");
  let config = Config::load("cache/pactree.conf").expect("load");
  assert_eq!(config.target, "arm64_monterey");
  config.save("cache/pactree.conf.new").expect("save");
}
