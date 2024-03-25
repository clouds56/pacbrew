#[macro_use] extern crate tracing;

use core_lib::io::read::read_toml;

pub mod config;

fn main() {
  dotenvy::dotenv().ok();
  tracing_subscriber::fmt::init();
  let config: config::Config = read_toml("pacbrew.toml").unwrap();
  info!(?config);
  println!("Hello, world!");
}
