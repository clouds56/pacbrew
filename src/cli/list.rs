use clap::Parser;

use crate::config::PacTree;

#[derive(Parser)]
pub struct Opts {
  #[arg(long)]
  all: bool,
  #[arg(long, short)] // TODO: count
  verbose: bool,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {

}
pub type Result<T, E=Error> = std::result::Result<T, E>;

pub fn run(opts: Opts, env: &PacTree) -> Result<()> {
  // info!("listing {}", env.config.meta_dir);
  Ok(())
}
