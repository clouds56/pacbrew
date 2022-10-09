use clap::Parser;
use crate::config::PacTree;

#[derive(Parser)]
pub struct Opts {
  names: Vec<String>,
}

/// stage1: collect dependencies
pub fn resolve(names: &[String]) -> Vec<String> {

}

pub fn run(opts: Opts, env: &PacTree) {
  info!("adding {:?}", opts.names);
  for i in &opts.names {
    match env.get_package(i) {
      Some(t) => debug!("found {:?}", t),
      None => error!("cannot found {}", i),
    }
  }
}
