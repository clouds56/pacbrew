#[macro_use]
extern crate tracing;

pub mod error;
pub mod progress;
pub mod pb;

pub mod package;
pub mod io {
  pub mod fetch;
}

#[cfg(test)]
mod tests {
  use std::str::FromStr;
  use crate::pb::{PbWriter, Suspendable};

  pub static ACTIVE_PB: std::sync::RwLock<Option<Suspendable>> = std::sync::RwLock::new(None);

  pub fn init_logger() {
    let _ = tracing_subscriber::fmt()
      .with_env_filter(tracing_subscriber::EnvFilter::from_str("info,pacbrew_core=debug").unwrap())
      .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
      .with_writer(|| PbWriter::new(ACTIVE_PB.read().unwrap().clone(), std::io::stderr()))
      .init();
  }
}
