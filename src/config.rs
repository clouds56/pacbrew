use std::collections::BTreeMap;

use crate::formula::Formula;

pub struct PacTree {
  pub packages: BTreeMap<String, Formula>,
  pub aliases: BTreeMap<String, String>,
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
