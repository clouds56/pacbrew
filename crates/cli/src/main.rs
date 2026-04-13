#[macro_use] extern crate tracing;

use std::{path::Path, sync::{Arc, RwLock}};

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
  Import(command::QueryArgs),
  Install(command::QueryArgs),
  Remove(command::RemoveArgs),
  List(command::list::ListArgs),
  Upgrade,
}

lazy_static::lazy_static! {
  static ref ACTIVE_PB: ActiveSuspendable = Arc::new(RwLock::new(None));
}

type TracingReloadHandle = tracing_subscriber::reload::Handle<tracing_subscriber::EnvFilter, tracing_subscriber::Registry>;
fn init_logger() -> TracingReloadHandle {
  use tracing_subscriber::{reload, EnvFilter, prelude::*};
  let fmt = tracing_subscriber::fmt::layer()
    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
    .compact()
    .with_writer(move || PbWriter::new(ACTIVE_PB.read().unwrap().clone(), std::io::stderr()))
    // .finish();
    ;
  let (filter, reload_handle) = reload::Layer::new(EnvFilter::new("warn"));
  let _ = tracing_subscriber::registry()
    .with(filter)
    .with(fmt)
    .init();
  reload_handle
}

#[tokio::main]
async fn main() {
  let mut default_config: &'static str = ".config/pacbrew/config.toml";
  std::env::vars().for_each(|(k, v)| trace!(%k, %v));
  if let Some(root) = std::env::var("CARGO_MANIFEST_DIR").ok() {
    std::env::set_current_dir(&root).ok();
    default_config = "pacbrew.toml"
  } else if let Some(home) = std::env::var("HOME").ok() {
    std::env::set_current_dir(&home).ok();
  }
  dotenvy::dotenv().ok();
  let reload_handle = init_logger();
  info!(cwd=%std::env::current_dir().unwrap().display());
  info!(default_config, exists=Path::new(default_config).exists());
  let config: config::Config = read_toml(default_config).unwrap();
  if let Some(log) = config.log.rust_log.as_deref() {
    reload_handle.reload(tracing_subscriber::EnvFilter::new(log)).ok();
  }
  let args = Args::parse();
  info!(?config, ?args);
  let mirrors = MirrorLists {
    lists: config.mirror_list.iter().map(|i| MirrorServer::new(i.r#type, &i.url, i.api_url.as_deref())).collect()
  };
  match args.command {
    Command::Update => command::update::run(&config, &mirrors).await.unwrap(),
    Command::Download(query) => command::download::run(&config, &mirrors, query).await.unwrap(),
    Command::Import(query) => command::import::run(&config, query).unwrap(),
    Command::Install(query) => {
      let installed = command::install::run(&config, &mirrors, query.clone()).await.unwrap();
      if installed {
        if let Some(log_file) = config.log.file.as_ref() {
        use std::io::Write;
        std::fs::create_dir_all(log_file.parent().unwrap()).ok();
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(log_file).unwrap();
        write!(file, "install {}\n", query.names.join(",")).unwrap();
        }
      }
    },
    Command::Remove(args) => command::remove::run(&config, args).unwrap(),
    Command::List(args) => command::list::run(&config, args).unwrap(),
    Command::Upgrade => command::upgrade::run(&config, &mirrors).await.unwrap(),
  }
}
