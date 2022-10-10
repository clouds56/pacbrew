use std::{path::{Path, PathBuf}, fs::File, io::Write};
use sha2::{Sha256, Digest};
use futures::StreamExt;

pub struct Task {
  // TODO: mirrors, fallback
  pub url: String,
  pub filename: PathBuf,
  pub temp: PathBuf,
  // TODO: verify function
  pub sha256: String,
}

impl Task {
  fn new<S: ToString, S2: AsRef<Path>, S3: AsRef<Path>>(url: S, filename: S2, temp: Option<S3>, sha256: String) -> Self {
    let url = url.to_string();
    let sha256 = sha256.to_ascii_lowercase();
    let mut filename = filename.as_ref().to_path_buf();
    if filename.is_dir() {
      let stem = url.split("/").last().unwrap_or_default();
      let stem = if stem.is_empty() { stem } else { &sha256 };
      filename = filename.join(stem);
    }
    let temp = match temp {
      Some(temp) => temp.as_ref().to_path_buf(),
      None => {
        let stem = filename.file_name().unwrap_or_else(|| Path::new(&sha256).as_os_str());
        filename.with_file_name(stem)
      }
    };
    Self { url, filename, temp, sha256 }
  }

  fn check_result(&self, filename: &Path) -> anyhow::Result<()> {
    if !self.filename.exists() {
      anyhow::bail!("file {} not exists", filename.to_string_lossy())
    }

    // https://stackoverflow.com/questions/69787906/how-to-hash-a-binary-file-in-rust
    let mut hasher = Sha256::new();
    let mut file = File::open(&filename)?;

    let bytes_written = std::io::copy(&mut file, &mut hasher)?;
    let hash_bytes = hasher.finalize();
    if format!("{:x}", hash_bytes) != self.sha256.to_ascii_lowercase() {
      anyhow::bail!("hash of file {} (len:{}) not match {:x} != {}", filename.to_string_lossy(), bytes_written, hash_bytes, self.sha256)
    }
    Ok(())
  }

  pub fn is_finished(&self) -> bool {
    self.check_result(&self.filename).is_ok()
  }

  pub fn partial_len(&self) -> Option<u64> {
    if self.temp.exists() {
      std::fs::metadata(&self.temp).ok().map(|i| i.len())
    } else {
      None
    }
  }

  pub async fn download(&self) -> anyhow::Result<u64> {
    // https://gist.github.com/giuliano-oliveira/4d11d6b3bb003dba3a1b53f43d81b30d
    let client = reqwest::Client::new();
    // TODO: share client?
    let mut downloaded = 0;
    let resp = client.get(&self.url).bearer_auth("QQ==").send().await?;
    // let total_size = resp.content_length().unwrap_or(1);
    let mut file = File::create(&self.temp)?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
      let chunk = chunk?;
      file.write_all(&chunk)?;
      downloaded += chunk.len() as u64;
    }
    Ok(downloaded)
  }

  pub async fn run(&mut self) -> anyhow::Result<()> {
    if self.is_finished() {
      return Ok(())
    }
    // TODO: partial download
    self.partial_len();
    self.download().await?;
    if self.check_result(&self.temp).is_ok() {
      if self.temp != self.filename {
        std::fs::rename(&self.temp, &self.filename)?;
      }
    } else {
      anyhow::bail!("file {} broken", self.temp.to_string_lossy())
    }
    Ok(())
  }
}
