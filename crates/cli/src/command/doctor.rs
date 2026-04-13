use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use crate::config::Config;

fn ensure_symlink_dir(root: &Path, link_rel: &str, target_rel: &str) -> Result<()> {
  let link = root.join(link_rel);
  let expected_target = PathBuf::from(target_rel);

  if let Some(parent) = link.parent() {
    std::fs::create_dir_all(parent)?;
  }

  if let Ok(meta) = std::fs::symlink_metadata(&link) {
    if meta.file_type().is_symlink() {
      let current = std::fs::read_link(&link)?;
      if current == expected_target {
        return Ok(());
      }
      symlink::remove_symlink_dir(&link)?;
    } else {
      return Err(anyhow!(
        "doctor expects symlink at {}, found non-symlink",
        link.display()
      ));
    }
  }

  symlink::symlink_dir(&expected_target, &link)?;
  Ok(())
}

pub fn run(config: &Config) -> Result<()> {
  let prefix = &config.base.prefix;

  ensure_symlink_dir(prefix, "Cellar", "local/opt")?;
  ensure_symlink_dir(prefix, "local/bin", "../bin")?;
  ensure_symlink_dir(prefix, "local/lib", "../lib")?;
  ensure_symlink_dir(prefix, "local/include", "../include")?;
  ensure_symlink_dir(prefix, "local/share", "../share")?;

  eprintln!("doctor repaired local symlink layout under {}", prefix.display());
  Ok(())
}
