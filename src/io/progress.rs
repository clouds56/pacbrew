use indicatif::{ProgressBar, ProgressState, ProgressStyle};

macro_rules! trace {
  (@$pb:expr => $($t:tt)*) => {
    $pb.suspend(|| ::log::trace!($($t)*))
  };
  ($($t:tt)*) => {
    ::log::trace!($($t)*)
  };
}

macro_rules! debug {
  (@$pb:expr => $($t:tt)*) => {
    $pb.suspend(|| ::log::debug!($($t)*))
  };
  ($($t:tt)*) => {
    ::log::debug!($($t)*)
  };
}

macro_rules! info {
  (@$pb:expr => $($t:tt)*) => {
    $pb.suspend(|| ::log::info!($($t)*))
  };
  ($($t:tt)*) => {
    ::log::info!($($t)*)
  };
}

macro_rules! warn {
  (@$pb:expr => $($t:tt)*) => {
    $pb.suspend(|| ::log::warn!($($t)*))
  };
  ($($t:tt)*) => {
    ::log::warn!($($t)*)
  };
}

macro_rules! error {
  (@$pb:expr => $($t:tt)*) => {
    $pb.suspend(|| ::log::error!($($t)*))
  };
  ($($t:tt)*) => {
    ::log::error!($($t)*)
  };
}

pub fn create_pb(total_len: usize) -> ProgressBar {
  let pb = ProgressBar::new(total_len as _);
  pb.set_style(ProgressStyle::with_template("{spinner:.green} {wide_msg} {human_pos}/{human_len} {per_sec} {eta_precise} [{bar:40.cyan/blue}] {percent:>3}%")
    .expect("style with_template")
    .with_key("per_sec", |state: &ProgressState, w: &mut dyn std::fmt::Write| { write!(w, "{:5}/s", state.per_sec() as usize).ok(); })
    .progress_chars("#>-"));
  pb
}

pub fn create_pbb(total_size: u64) -> ProgressBar {
  let pb = ProgressBar::new(total_size);
  pb.set_style(ProgressStyle::with_template("{spinner:.green} {wide_msg} {bytes}/{total_bytes} {binary_bytes_per_sec:>10} {eta_precise} [{bar:40.cyan/blue}] {percent:>3}%")
    .expect("style with_template")
    // .with_key("per_sec", |state: &ProgressState, w: &mut dyn std::fmt::Write| { write!(w, "{:5}/s", state.per_sec() as usize).ok(); })
    .progress_chars("#>-"));
  pb
}
