use std::{collections::{VecDeque, BTreeMap}, sync::Arc};

use clap::Parser;
use crate::config::PacTree;

#[derive(Parser)]
pub struct Opts {
  names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageName {
  pub name: String,
  pub version: String,
  pub reason: Arc<Vec<String>>,
}

impl From<String> for PackageName {
  fn from(v: String) -> Self {
    Self { name: v.clone(), version: String::new(), reason: Arc::new(vec![]) }
  }
}

impl PackageName {
  pub fn dependency(&self, names: &[String]) -> Vec<PackageName> {
    let mut reason = self.reason.to_vec();
    reason.push(self.name.to_string());
    let reason = Arc::new(reason);
    names.iter().map(|i| Self { name: i.to_string(), version: String::new(), reason: reason.clone() }).collect()
  }

  pub fn replace(&self, name: String, version: String) -> Self {
    Self { name, version, reason: self.reason.clone() }
  }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
  #[error("resolve: package {0:?} not found")]
  Resolve(PackageName), // TODO: dependency path
  #[error("prebuilt")]
  Prebuilt(PackageName),
  #[error("resolve-net")]
  ResolveNet(PackageName, #[source] Arc<reqwest::Error>),
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
    // TODO: channel
    let version = package.versions.stable.clone();
    // TODO: check requirements
    debug!("resolving {}:{} => {:?}", package.name, version, package.dependencies);
    let p = p.replace(package.full_name.to_string(), version);
    names.extend(p.dependency(&package.dependencies));
    result.insert(p.name.to_string(), p);
  }
  Ok(result)
}

pub fn resolve_url(names: &[PackageName], env: &PacTree) -> Result<BTreeMap<String, String>> {
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
    debug!("url of {} ({}) => {}", p.name, bottle.sha256, bottle.url);
    // TODO: mirrors
    result.insert(p.name.clone(), bottle.url.clone());
  }
  Ok(result)
}

#[tokio::main]
pub async fn resolve_size(names: &BTreeMap<String, String>, env: &PacTree) -> Result<BTreeMap<String, u64>> {
  let mut result = BTreeMap::new();
  let client = reqwest::Client::new();
  // TODO: true concurrent
  for (name, url) in names {
    // TODO: mirrors
    let resp = client.head(url).bearer_auth("QQ==").send().await.map_err(|e| Error::ResolveNet(PackageName::from(name.to_string()), Arc::new(e)))?;
    if resp.status().is_success() {
      let headers = resp.headers();
      // TODO: handle error
      let size = headers.get("content-length")
          .and_then(|i| i.to_str().ok())
          .and_then(|i| i.parse::<u64>().ok())
          .unwrap_or_default();
      result.insert(name.to_string(), size);
      debug!("head {} => {}", url, size);
    } else {
      warn!("{} => {} {:?}", url, resp.status(), resp.headers());
    }
  }
  Ok(result)
}

pub fn run(opts: Opts, env: &PacTree) -> Result<()> {
  info!("adding {:?}", opts.names);
  let all_names = resolve(&opts.names, env)?;
  info!("resolved {:?}", all_names.keys());
  let all_names = all_names.values().cloned().collect::<Vec<_>>();
  let urls = resolve_url(&all_names, env)?;
  let size = resolve_size(&urls, env)?;
  // TODO: confirm and human readable
  info!("total download {}", size.values().sum::<u64>());
  Ok(())
}
