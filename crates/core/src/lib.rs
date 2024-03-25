#[macro_use]
extern crate tracing;

pub mod error;
pub mod ui;

pub mod package;
pub mod stage;
pub mod io;

#[cfg(test)]
mod tests {
  use std::{str::FromStr, sync::{Arc, RwLock}};
  use crate::{io::fetch::MirrorLists, package::{formula::Formula, mirror::{MirrorServer, MirrorType}}, ui::bar::{PbWriter, Suspendable}};

  pub static FORMULA_FILE: &str = "cache/formula.json";
  pub static CACHE_PATH: &str = "cache";
  pub static ARCH: &str = "arm64_sonoma";
  pub static MIRROR: (MirrorType, &str) = (MirrorType::Bottle, "https://mirrors.ustc.edu.cn/homebrew-bottles");

  pub fn get_mirrors() -> MirrorLists {
    MirrorLists {
      lists: vec![MirrorServer::new(MIRROR.0, MIRROR.1, None)]
    }
  }

  pub fn get_formulas() -> Vec<Formula> {
    crate::io::read::read_formulas(FORMULA_FILE).unwrap()
  }

  pub fn init_logger(env_filter: Option<&str>) -> Arc<RwLock<Option<Suspendable>>> {
    use tracing_subscriber::fmt::format::FmtSpan;
    let active_pb = Arc::new(RwLock::new(None));
    let result = active_pb.clone();
    let _ = tracing_subscriber::fmt()
      .with_env_filter(tracing_subscriber::EnvFilter::from_str(env_filter.unwrap_or("info,pacbrew_core=debug")).unwrap())
      .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
      .with_writer(move || PbWriter::new(active_pb.read().unwrap().clone(), std::io::stderr()))
      .init();
    result
  }
}
