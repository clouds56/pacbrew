use std::collections::{VecDeque, BTreeMap};

use clap::Parser;
use crate::config::PacTree;

#[derive(Parser)]
pub struct Opts {
  names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageName {
  pub name: String,
  pub reason: Box<Vec<String>>,
}

impl From<String> for PackageName {
  fn from(v: String) -> Self {
    Self { name: v.clone(), reason: Box::new(vec![]) }
  }
}

impl PackageName {
  pub fn dependency(&self, names: &[String]) -> Vec<PackageName> {
    let mut reason = self.reason.to_vec();
    reason.push(self.name.to_string());
    let reason = Box::new(reason);
    names.iter().map(|i| Self { name: i.to_string(), reason: reason.clone() }).collect()
  }

  pub fn replace(&self, name: String) -> Self {
    Self { name, reason: self.reason.clone() }
  }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
  #[error("resolve: package {0:?} not found")]
  Resolve(PackageName), // TODO: dependency path
  #[error("prebuilt")]
  Prebuilt(PackageName),
  #[error("resolve-net")]
  ResolveNet(),
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

/// stage1: collect dependencies
/// TODO: sort in topological order
pub fn resolve(names: &[String], env: &PacTree) -> Result<BTreeMap<String, PackageName>> {
  let mut result = BTreeMap::new();
  let mut names = names.iter().map(|i| PackageName::from(i.to_string())).collect::<VecDeque<_>>();
  while let Some(p) = names.pop_front() {
    if result.contains_key(&p.name) {
      continue
    }

    let package = match env.get_package(&p.name) {
      Some(t) => t,
      None => {
        error!("cannot found {}", &p.name);
        return Err(Error::Resolve(p))
      }
    };
    // TODO: check requirements
    debug!("resolving {} => {:?}", package.name, package.dependencies);
    let p = p.replace(package.full_name.to_string());
    names.extend(p.dependency(&package.dependencies));
    result.insert(p.name.to_string(), p);
  }
  Ok(result)
}

pub fn resolve_size(names: &[PackageName], env: &PacTree) -> Result<BTreeMap<String, u64>> {
  let mut result = BTreeMap::new();
  for p in names {
    let package = match env.get_package(&p.name) {
      Some(t) => t,
      None => {
        error!("cannot found {}", &p.name);
        return Err(Error::Resolve(p.clone()))
      }
    };
    let bottles = match package.bottle.get("stable") {
      Some(bottles) => bottles,
      None => {
        error!("channel stable not exists {}", &p.name);
        return Err(Error::Prebuilt(p.clone()));
      }
    };
    let bottle = match bottles.files.get(&env.config.target).or_else(|| bottles.files.get("all")) {
      Some(bottle) => bottle,
      None => {
        error!("target {} not found in {:?} for {}", env.config.target, bottles.files.keys(), p.name);
        return Err(Error::Prebuilt(p.clone()));
      }
    };
    debug!("head of {} ({}) => {}", p.name, bottle.sha256, bottle.url);
    result.insert(p.name.clone(), 0);
  }
  Ok(result)
}

pub fn run(opts: Opts, env: &PacTree) -> Result<()> {
  info!("adding {:?}", opts.names);
  let all_names = resolve(&opts.names, env)?;
  info!("resolved {:?}", all_names.keys());
  let all_names = all_names.values().cloned().collect::<Vec<_>>();
  let size = resolve_size(&all_names, env)?;
  Ok(())
}
