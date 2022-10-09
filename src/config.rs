use std::collections::BTreeMap;

use crate::formula::Formula;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Os {
  Macos(String),
  Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Arch {
  x86_64, arm64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
  pub os: Os,
  pub arch: Arch,
  pub target: String,
}

impl Config {
  pub fn new(os: Os, arch: Arch) -> Self {
    let target = match (&os, arch) {
      (Os::Macos(s), Arch::arm64) => format!("arm64_{}", s),
      (Os::Macos(s), Arch::x86_64) => format!("{}", s),
      (Os::Linux, arch) => format!("{:?}_linux", arch),
    };
    Self { os, arch, target }
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
