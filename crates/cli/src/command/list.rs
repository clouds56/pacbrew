use anyhow::Result;
use core_lib::db::{self, InstalledVersionStatus};
use core_lib::io::read::read_formulas;
use core_lib::package::package::InstallReason;
use core_lib::package::package::PackageVersion;

use crate::config::Config;

#[derive(Debug, Clone, clap::Args)]
pub struct ListArgs {
  #[arg(long)]
  pub all: bool,

  #[arg(long)]
  pub outdated: bool,
}

pub fn run(config: &Config, args: ListArgs) -> Result<()> {
  let installed = db::list_installed(&config.base.db)?;
  let latest_versions = if args.outdated {
    let formulas = read_formulas(config.base.formula_json())?;
    formulas.into_iter().map(|formula| {
      let package = PackageVersion::from(formula);
      let latest = package.version_full();
      (package.name, latest)
    }).collect()
  } else {
    std::collections::HashMap::new()
  };

  for pkg in installed {
    if !args.all && pkg.reason != InstallReason::Explicit {
      continue;
    }
    if args.outdated {
      let Some(latest) = latest_versions.get(&pkg.name) else {
        continue;
      };
      if db::version_status(Some(&pkg.version), latest) != InstalledVersionStatus::Outdated {
        continue;
      }
      println!("{} {} -> {}", pkg.name, pkg.version, latest);
      continue;
    }

    println!("{} {}", pkg.name, pkg.version);
  }
  Ok(())
}
