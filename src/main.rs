use std::collections::BTreeMap;

use clap::Parser;

#[macro_use]
mod io;
mod cli;
pub mod config;
pub mod formula;
pub mod relocation;

pub use formula::Formula;

#[derive(Parser)]
pub enum Subcommand {
  Add(cli::add::Opts),
  Update(cli::update::Opts),
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
  flexi_logger::Logger::try_with_env_or_str("info, pacbrew=debug")?.start()?;
  let sub = Subcommand::parse();
  let config = config::Config::load("cache/pactree.conf")?;
  debug!("config: {:?}", config);
  let formula_str = std::fs::read_to_string("cache/formula.json")?;
  let formula = serde_json::from_str::<Vec<Formula>>(&formula_str)?;
  let env = config::PacTree {
    aliases: build_aliases(&formula),
    packages: formula.into_iter().map(|i| (i.full_name.clone(), i)).collect(),
    config,
  };
  match sub {
    Subcommand::Add(opts) => cli::add::run(opts, &env)?,
    Subcommand::Update(opts) => cli::update::run(opts, &env)?,
  }
  Ok(())
}
