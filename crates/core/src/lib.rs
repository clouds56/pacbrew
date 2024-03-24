#[macro_use]
extern crate tracing;

pub mod error;
pub mod progress;

pub mod io {
  pub mod fetch;
}

#[cfg(test)]
mod tests {
  use std::str::FromStr;

  pub fn init_logger() {
    let _ = tracing_subscriber::fmt()
      .with_env_filter(tracing_subscriber::EnvFilter::from_str("info,pacbrew_core=debug").unwrap())
      .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
      .init();
  }
}
