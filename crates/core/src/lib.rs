#[macro_use]
extern crate tracing;

pub mod error;
pub mod ui;

pub mod package;
pub mod stage;
pub mod io {
  pub mod fetch;
  pub mod read;
}

#[cfg(test)]
mod tests {
  use std::str::FromStr;
  use crate::ui::bar::{PbWriter, Suspendable};

  pub static FORMULA_FILE: &str = "formula.json";
  pub static ACTIVE_PB: std::sync::RwLock<Option<Suspendable>> = std::sync::RwLock::new(None);

  pub fn init_logger() {
    use tracing_subscriber::fmt::format::FmtSpan;
    let _ = tracing_subscriber::fmt()
      .with_env_filter(tracing_subscriber::EnvFilter::from_str("info,pacbrew_core=debug").unwrap())
      .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
      .with_writer(|| PbWriter::new(ACTIVE_PB.read().unwrap().clone(), std::io::stderr()))
      .init();
  }
}
