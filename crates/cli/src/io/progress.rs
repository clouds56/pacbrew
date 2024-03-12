use indicatif::{ProgressBar, ProgressState, ProgressStyle, FormattedDuration};

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

pub fn create_pb(prefix: &str, total_len: usize) -> ProgressBar {
  let style = ProgressStyle::with_template("{spinner:.green} {prefix} {wide_msg} {human_pos}/{human_len} {per_sec} {eta_precise} [{bar:40.cyan/blue}] {percent:>3}%")
    .expect("style with_template")
    .with_key("per_sec", |state: &ProgressState, w: &mut dyn std::fmt::Write| { write!(w, "{:5}/s", state.per_sec() as usize).ok(); })
    .with_key("eta_precise", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
      write!(w, "{}", if state.is_finished() { FormattedDuration(state.elapsed()) } else { FormattedDuration(state.eta()) }).ok();
    })
    .progress_chars("#>-");
  let pb = ProgressBar::new(total_len as _);
  pb.set_style(style);
  pb.set_prefix(prefix.to_string());
  pb
}

pub fn create_pbb(prefix: &str, total_size: u64) -> ProgressBar {
  let style = ProgressStyle::with_template("{spinner:.green} {prefix} {wide_msg} {bytes:>10}/{total_bytes:10} {binary_bytes_per_sec:>15} {eta_precise} [{bar:40.cyan/blue}] {percent:>3}%")
    .expect("style with_template")
    // .with_key("per_sec", |state: &ProgressState, w: &mut dyn std::fmt::Write| { write!(w, "{:5}/s", state.per_sec() as usize).ok(); })
    .with_key("eta_precise", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
      write!(w, "{}", if state.is_finished() { FormattedDuration(state.elapsed()) } else { FormattedDuration(state.eta()) }).ok();
    })
    .progress_chars("#>-");
  let pb = ProgressBar::new(total_size);
  pb.set_style(style);
  pb.set_prefix(prefix.to_string());
  pb
}
