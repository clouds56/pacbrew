use indicatif::ProgressStyle;

pub mod update;
pub mod download;

#[derive(Debug, Clone, clap::Args)]
pub struct QueryArgs {
  pub names: Vec<String>,
}

pub enum PbStyle {
  Items,
  Bytes,
}

impl PbStyle {
  pub fn style(&self) -> ProgressStyle {
    match self {
      Self::Items => ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
        .expect("style with_template"),
      Self::Bytes => ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} {msg}")
        .expect("style with_template"),
    }.progress_chars("#>-")

    // ProgressStyle::with_template("{spinner:.green} {prefix} {wide_msg} {human_pos}/{human_len} {per_sec} {eta_precise} [{bar:40.cyan/blue}] {percent:>3}%")
    // .expect("style with_template")
    // .with_key("per_sec", |state: &ProgressState, w: &mut dyn std::fmt::Write| { write!(w, "{:5}/s", state.per_sec() as usize).ok(); })
    // .with_key("eta_precise", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
    //   write!(w, "{}", if state.is_finished() { FormattedDuration(state.elapsed()) } else { FormattedDuration(state.eta()) }).ok();
    // })
    // .progress_chars("#>-")
  }
}
