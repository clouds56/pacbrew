use anyhow::Result;
use core_lib::db;

use crate::config::Config;

pub fn run(config: &Config) -> Result<()> {
  let installed = db::list_installed(&config.base.db)?;
  for pkg in installed {
    println!("{} {}", pkg.name, pkg.version);
  }
  Ok(())
}
