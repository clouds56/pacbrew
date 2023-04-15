use std::{collections::BTreeMap, path::Path};
use serde::{Deserialize, Serialize};
use specs::{World, Entity, Component, DenseVecStorage, WorldExt, world, Builder};

use crate::{formula::Formula, meta::{PackageInfo, self}};

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

fn is_false(b: &bool) -> bool { !*b }
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Mirror {
  pub url: String,
  /// if oci { "{url}/{p.name.replace("@", "/")}/blobs/sha256:{p.sha256}" }
  /// else   { "{url}/{p.name}-{p.version}.{p.arch}.bottle.tar.gz" }
  #[serde(default, skip_serializing_if = "is_false")]
  pub oci: bool,
}

impl Mirror {
  pub fn new(url: String) -> Self {
    Self { url, oci: false }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
  #[serde(flatten)]
  pub os: Os,
  pub os_fallback: Vec<String>,
  pub arch: Arch,
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub target: String,
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub scripts_dir: String,
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub root_dir: String,
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub meta_dir: String,
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub cache_dir: String,
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub cellar_dir: String,
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub formula_url: String,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub mirror_list: Vec<Mirror>,
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

  pub fn normalize(&mut self) {
    self.target = Config::build_target(&self.os, self.arch);
    if self.os_fallback.is_empty() {
      if let Os::Macos { version } = &self.os {
        // TODO
      }
    }
    if self.root_dir.is_empty() {
      self.root_dir = "/opt/homebrew/".to_string();
    }
    if self.scripts_dir.is_empty() {
      self.scripts_dir = Path::new("scripts").canonicalize().expect("scripts").to_string_lossy().to_string();
    }
    self.root_dir = self.root_dir.replace("\\", "/");
    if !self.root_dir.ends_with("/") {
      self.root_dir += "/"
    }
    if self.meta_dir.is_empty() {
      self.meta_dir = self.root_dir.clone() + "var/lib/pactree";
    }
    if self.cache_dir.is_empty() {
      self.cache_dir = self.root_dir.clone() + "var/cache/pactree";
    }
    if self.cellar_dir.is_empty() {
      self.cellar_dir = self.root_dir.clone() + "Cellar";
    }
    if self.formula_url.is_empty() {
      self.formula_url = "https://formulae.brew.sh/api/formula.json".to_string();
    }
    if self.mirror_list.is_empty() {
      let mut default_mirror = Mirror::new("https://ghcr.io/v2/homebrew/core".to_string());
      default_mirror.oci = true;
      self.mirror_list.push(default_mirror);
    }
  }

  pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    let s = std::fs::read_to_string(path)?;
    let mut result = toml::from_str::<Self>(&s)?;
    // TODO: figure out most thing with default value
    result.normalize();
    Ok(result)
  }

  pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
    let s = toml::to_string_pretty(&self)?;
    std::fs::write(path, s)?;
    Ok(())
  }
}

#[derive(Debug, Clone, Component)]
pub struct PackageName(pub String);

#[derive(Debug, Default)]
pub struct PackageMap(pub BTreeMap<String, Entity>);

pub struct PacTree {
  pub world: World,
  // pub info: Storage<PackageInfo>,
  // pub meta: Storage<PackageMeta>,
  // pub formula: Storage<PackageFormula>,

  // pub map: BTreeMap<String, Entity>,
  // pub config: Config,
}

impl PacTree {
  pub fn new(config: Config) -> Self {
    let mut world = World::new();
    world.insert(Some(config));
    world.insert(PackageMap(Default::default()));
    world.register::<PackageName>();
    world.register::<Formula>();
    world.register::<PackageInfo>();
    world.register::<crate::cli::add::Stage>();
    Self { world }
  }

  pub fn insert(&mut self, name: String, formula: &Formula) -> Entity {
    let mut aliases = formula.aliases.clone();
    aliases.extend(formula.old_name.clone());
    let entity = self.world.create_entity().with(PackageName(name.to_string())).with(formula.clone()).build();
    let mut map = self.world.write_resource::<PackageMap>();
    for name in aliases {
      map.0.insert(name, entity);
    }
    map.0.insert(name, entity);
    map.0.insert(formula.name.clone(), entity);
    map.0.insert(formula.full_name.clone(), entity);
    entity
  }
}

#[test]
fn test_config() {
  let config = Config {
    os: Os::Macos { version: "monterey".to_string() },
    os_fallback: Vec::new(),
    arch: Arch::arm64,
    target: String::new(),
    scripts_dir: String::new(),
    root_dir: String::new(),
    meta_dir: String::new(),
    cache_dir: String::new(),
    cellar_dir: String::new(),
    formula_url: String::new(),
    mirror_list: Vec::new(),
  };

  config.save("cache/pactree.conf.old").expect("save");
  let config = Config::load("cache/pactree.conf").expect("load");
  assert_eq!(config.target, "arm64_monterey");
  config.save("cache/pactree.conf.new").expect("save");
}

#[test]
fn test_formula() {
  // TODO: enable brotli?
  use crate::io::{fetch::Task, progress::create_pbb};
  let formula_url = "https://formulae.brew.sh/api/formula.json";
  let mut task = Task::new(reqwest::Client::new(), formula_url, "cache/formula.json.tmp", None, String::new());
  task.set_progress(create_pbb("formula.json", 0)).run_sync().expect("download");
  let formula_str = std::fs::read_to_string("cache/formula.json.tmp").expect("read");
  let formula = serde_json::from_str::<Vec<Formula>>(&formula_str).expect("parse");
  assert_ne!(formula.len(), 0);
  std::fs::rename("cache/formula.json.tmp", "cache/formula.json").expect("rename");
}
