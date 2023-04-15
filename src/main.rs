use std::collections::BTreeMap;

use clap::Parser;

#[macro_use]
mod io;
mod cli;
pub mod config;
pub mod formula;
pub mod meta;
pub mod relocation;

pub use formula::Formula;

#[derive(Parser)]
pub enum Subcommand {
  Add(cli::add::Opts),
  // Update(cli::update::Opts),
  List(cli::list::Opts),
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
  let mut env = config::PacTree::new(config);
  for f in formula {
    env.insert(f.full_name.clone(), &f);
  }
  match sub {
    Subcommand::Add(opts) => cli::add::run(opts, &env)?,
    // Subcommand::Update(opts) => cli::update::run(opts, &env)?,
    Subcommand::List(opts) => cli::list::run(opts, &env)?,
  }
  Ok(())
}
