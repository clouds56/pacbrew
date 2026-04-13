use anyhow::Result;
use core_lib::stage::link;

use crate::config::Config;

pub fn run(config: &Config) -> Result<()> {
  let installed = link::list_installed(&config.base.local_opt())?;
  for pkg in installed {
    println!("{} {}", pkg.name, pkg.version);
  }
  Ok(())
}
