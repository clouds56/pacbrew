#[macro_use] extern crate tracing;

use std::sync::{Arc, RwLock};

use clap::Parser;
use core_lib::{io::{fetch::MirrorLists, read::read_toml}, package::mirror::MirrorServer, ui::bar::{ActiveSuspendable, PbWriter}};
use tracing_subscriber::fmt::format::FmtSpan;

pub mod config;
pub mod command;

#[derive(Debug, Clone, clap::Parser)]
pub struct Args {
  #[command(subcommand)]
  pub command: Command,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Command {
  Update,
  Download(command::QueryArgs),
}

lazy_static::lazy_static! {
  static ref ACTIVE_PB: ActiveSuspendable = Arc::new(RwLock::new(None));
}

#[tokio::main]
async fn main() {
  dotenvy::dotenv().ok();
  let _ = tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
    .compact()
    .with_writer(move || PbWriter::new(ACTIVE_PB.read().unwrap().clone(), std::io::stderr()))
    .init();
  let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
  std::env::set_current_dir(&root).ok();
  info!(cwd=root);
  let config: config::Config = read_toml("pacbrew.toml").unwrap();
  let args = Args::parse();
  info!(?config, ?args);
  let mirrors = MirrorLists {
    lists: config.mirror_list.iter().map(|i| MirrorServer::new(i.r#type, &i.url, i.api_url.as_deref())).collect()
  };
  match args.command {
    Command::Update => command::update::run(&config, &mirrors).await.unwrap(),
    Command::Download(query) => command::download::run(&config, &mirrors, query).await.unwrap(),
  }
}
