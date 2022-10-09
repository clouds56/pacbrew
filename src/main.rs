#[macro_use] extern crate log;
use std::collections::BTreeMap;

use clap::Parser;

mod cli;
mod config;
mod formula;

pub use formula::Formula;

#[derive(Parser)]
pub enum Subcommand {
  Add(cli::add::Opts)
}

fn build_aliases(formula: &[Formula]) -> BTreeMap<String, String> {
  let mut aliases = BTreeMap::new();
  for f in formula {
    let name = f.full_name.to_string();
    if f.name != name {
      aliases.insert(f.name.to_string(), name.to_string());
    }
    if let Some(old_name) = &f.old_name {
      aliases.insert(old_name.to_string(), name.to_string());
    }
    for alias_name in &f.aliases {
      aliases.insert(alias_name.to_string(), name.to_string());
    }
  }
  aliases
}

fn main() -> anyhow::Result<()> {
  flexi_logger::Logger::try_with_str("info, pacbrew=debug")?.start()?;
  let sub = Subcommand::parse();
  let formula_str = include_str!("../cache/formula.json");
  let formula = serde_json::from_str::<Vec<Formula>>(formula_str)?;
  let env = config::PacTree {
    aliases: build_aliases(&formula),
    packages: formula.into_iter().map(|i| (i.full_name.clone(), i)).collect(),
    config: config::Config::new(config::Os::Macos("monterey".to_string()), config::Arch::arm64),
  };
  match sub {
    Subcommand::Add(opts) => cli::add::run(opts, &env)?,
  }
  Ok(())
}
