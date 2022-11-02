use std::path::Path;

use clap::Parser;

use crate::{config::PacTree, io::{fetch::Task, progress::create_pbb}, Formula};

#[derive(Parser)]
pub struct Opts {
  #[arg(long, short)]
  force: bool,
}

#[tokio::main]
pub async fn check_update(client: &reqwest::Client, formula_url: &str) -> anyhow::Result<bool> {
  Ok(true)
}

// sha256
// curl https://api.github.com/repos/testacc01/testrepo01/contents/test.txt

pub fn run(opts: Opts, env: &PacTree) -> anyhow::Result<()> {
  debug!("downloading from {}", env.config.formula_url);
  // let formula_url = "https://httpbin.org/get";
  let formula_url = env.config.formula_url.clone();
  let filename = Path::new(&env.config.cache_dir).join("formula.json");
  let new_filename = Path::new(&env.config.cache_dir).join("formula.json.new");
  if new_filename.exists() {
    std::fs::remove_file(new_filename.as_path()).ok();
  }
  let client = reqwest::Client::builder().http1_title_case_headers().http1_only().gzip(true).deflate(true).brotli(false).user_agent("Wget/1.21.3").build()?;
  let mut task = Task::new(client, formula_url, new_filename.as_path(), None, String::new());
  let pb = create_pbb("formula.json", 0);
  task.set_progress(pb.clone()).run_sync()?;
  pb.finish();
  let formula_str = std::fs::read_to_string(new_filename.as_path())?;
  let formula = serde_json::from_str::<Vec<Formula>>(&formula_str)?;
  debug!("formula count: {}", formula.len());
  if formula.len() != 0 {
    std::fs::rename(new_filename, filename)?;
  }
  Ok(())
}
