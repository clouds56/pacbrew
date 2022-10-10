use std::{collections::{VecDeque, BTreeMap}, sync::Arc, path::{PathBuf, Path}};

use clap::Parser;
use crate::io::{progress::{create_pb, create_pbb}, fetch::{github_client, basic_client}};
use crate::config::PacTree;

#[derive(Parser)]
pub struct Opts {
  names: Vec<String>,
}

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
      arch: String::new(), sha256: String::new(), url: String::new(), size: 0,
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

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
  #[error("resolve: package {0:?} not found")]
  Resolve(PackageInfo), // TODO: dependency path
  #[error("prebuilt")]
  Prebuilt(PackageInfo),
  #[error("resolve-net")]
  ResolveNet(PackageInfo, #[source] Arc<reqwest::Error>),
  #[error("download: package {0:?} failed")]
  Download(PackageInfo, #[source] Arc<anyhow::Error>),
  #[error("io {0:?}")]
  Io(#[from] Arc<std::io::Error>),
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

/// stage1: collect dependencies
/// TODO: sort in topological order
pub fn resolve(names: &[String], env: &PacTree) -> Result<PackageInfos> {
  let mut result = PackageInfos::new();
  let mut names = names.iter().map(|i| PackageInfo::new(i.to_string())).collect::<VecDeque<_>>();
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
    let p = p.with_name(package.full_name.to_string(), version, package.revision);
    names.extend(p.with_dependencies(&package.dependencies));
    result.insert(p.name.to_string(), p);
  }
  Ok(result)
}

pub fn resolve_url(infos: &mut PackageInfos, env: &PacTree) -> Result<BTreeMap<String, String>> {
  let pb = create_pb(infos.len());
  pb.set_message("Resolve url");
  let mut result = BTreeMap::new();
  for p in infos.values_mut() {
    pb.set_message(format!("Resolve url for {}", p.name));
    let package = match env.get_package(&p.name) {
      Some(t) => t,
      None => {
        error!(@pb => "cannot found {}", &p.name);
        return Err(Error::Resolve(p.clone()))
      }
    };
    let bottles = match package.bottle.get("stable") {
      Some(bottles) => bottles,
      None => {
        error!(@pb => "channel stable not exists {}", &p.name);
        return Err(Error::Prebuilt(p.clone()));
      }
    };
    let bottle = match bottles.files.get(&env.config.target).or_else(|| bottles.files.get("all")) {
      Some(bottle) => bottle,
      None => {
        error!(@pb => "target {} not found in {:?} for {}", env.config.target, bottles.files.keys(), p.name);
        return Err(Error::Prebuilt(p.clone()));
      }
    };
    // TODO: mirrors
    p.arch = if bottles.files.contains_key(&env.config.target) {
      env.config.target.clone()
    } else { "all".to_string() };
    p.sha256 = bottle.sha256.clone();
    if let Some(mirror) = env.config.mirror_list.first() {
      if mirror.oci {
        p.url = format!("{}/{}/blobs/sha256:{}", mirror.url, p.name, p.sha256)
      } else {
        p.url = format!("{}/{}-{}.{}.bottle.tar.gz", mirror.url, p.name, p.version_full, p.arch)
      }
    } else {
      p.url = bottle.url.clone();
    }
    debug!(@pb => "url of {} ({}) => {}", p.name, p.sha256, p.url);
    result.insert(p.name.clone(), p.url.clone());
    pb.inc(1);
  }
  pb.finish_with_message("Resolve url");
  Ok(result)
}

#[tokio::main]
pub async fn resolve_size(infos: &mut PackageInfos, env: &PacTree) -> Result<BTreeMap<String, u64>> {
  let pb = create_pb(infos.len());
  pb.set_message("Resolve size");
  let mut result = BTreeMap::new();
  // TODO: true concurrent
  for p in infos.values_mut() {
    pb.set_message(format!("Resolve size for {}", p.name));
    // TODO: mirrors
    let client = if p.url.contains("//ghcr.io/") { github_client() } else { basic_client() };
    let resp = client.head(&p.url).send().await.map_err(|e| Error::ResolveNet(p.clone(), Arc::new(e)))?;
    if resp.status().is_success() {
      // TODO: handle error
      // let size = resp.content_length().unwrap_or_default(); <-- this is broken, always return 0
      let size = resp.headers().get("content-length")
          .and_then(|i| i.to_str().ok())
          .and_then(|i| i.parse::<u64>().ok())
          .unwrap_or_default();
      result.insert(p.name.to_string(), size);
      p.size = size;
      debug!(@pb => "head {} => {}", &p.url, size);
    } else {
      warn!(@pb => "{} => {} {:?}", &p.url, resp.status(), resp.headers());
    }
    pb.inc(1);
  }
  pb.finish_with_message("Resolve size");
  Ok(result)
}

#[tokio::main]
pub async fn download_packages(infos: &mut PackageInfos, env: &PacTree) -> Result<BTreeMap<String, PathBuf>> {
  use crate::io::fetch::Task;
  let mut result = BTreeMap::new();
  let cache_dir = Path::new(&env.config.cache_dir);
  for p in infos.values_mut() {
    p.package_name = format!("{}-{}.{}.bottle.tar.gz", p.name, p.version_full, p.arch);
    let package_path = cache_dir.join(&p.package_name);
    let client = if p.url.contains("//ghcr.io/") { github_client() } else { basic_client() };
    let mut task = Task::new(client, &p.url, &package_path, None, p.sha256.clone());
    if task.is_finished() {
      continue
    }
    let pb = create_pbb(0);
    pb.set_message(p.name.clone());
    task.set_progress(pb.clone()).run().await.map_err(|e| Error::Download(p.clone(), Arc::new(e)))?;
    p.pacakge_path = package_path.clone();
    result.insert(p.name.clone(), package_path);
    pb.finish(); // TODO show elapsed time
  }
  Ok(result)
}

pub fn run(opts: Opts, env: &PacTree) -> Result<()> {
  info!("adding {:?}", opts.names);
  let mut all_packages = resolve(&opts.names, env)?;
  info!("resolved {:?}", all_packages.keys());
  let urls = resolve_url(&mut all_packages, env)?;
  let size = resolve_size(&mut all_packages, env)?;
  // TODO: confirm and human readable
  info!("total download {}", all_packages.values().map(|i| i.size).sum::<u64>());
  std::fs::create_dir_all(&env.config.cache_dir).map_err(|e| Error::Io(Arc::new(e)))?;
  download_packages(&mut all_packages, env)?;
  Ok(())
}
