use std::{path::{Path, PathBuf}, fs::File, io::Write};
use reqwest::Client;
use sha2::{Sha256, Digest};
use futures::StreamExt;

// TODO some kind of suspend
pub trait Progress {
  fn set_position(&mut self, size: u64);
  fn set_length(&mut self, size: u64);
}

impl Progress for indicatif::ProgressBar {
  fn set_position(&mut self, size: u64) {
    indicatif::ProgressBar::set_position(self, size)
  }
  fn set_length(&mut self, size: u64) {
    indicatif::ProgressBar::set_length(self, size)
  }
}

pub fn basic_client() -> reqwest::Client {
  reqwest::Client::builder()
    .user_agent("Wget/1.21.3")
    .build().expect("build client")
}

pub fn github_client() -> reqwest::Client {
  use reqwest::header;
  let mut headers = header::HeaderMap::new();
  let mut auth_value = header::HeaderValue::from_static("Bearer QQ==");
  auth_value.set_sensitive(true);
  headers.insert(header::AUTHORIZATION, auth_value);
  let client = reqwest::Client::builder()
    .user_agent("pacbrew/0.1")
    .default_headers(headers).build().expect("build client");
  client
}

pub struct Task {
  pub client: Client,
  // TODO: mirrors, fallback
  pub url: String,
  pub filename: PathBuf,
  pub temp: PathBuf,
  // TODO: verify function
  pub sha256: String,
  pub progress: Option<Box<dyn Progress>>,
}

pub fn check_sha256(filename: &Path, sha256: &str) -> anyhow::Result<()> {
  if !filename.exists() {
    anyhow::bail!("file {} not exists", filename.to_string_lossy())
  }
  if sha256.is_empty() {
    return Ok(())
  }

  // https://stackoverflow.com/questions/69787906/how-to-hash-a-binary-file-in-rust
  let mut hasher = Sha256::new();
  let mut file = File::open(&filename)?;

  let bytes_written = std::io::copy(&mut file, &mut hasher)?;
  let hash_bytes = hasher.finalize();
  if format!("{:x}", hash_bytes) != sha256.to_ascii_lowercase() {
    anyhow::bail!("hash of file {} (len:{}) not match {:x} != {}", filename.to_string_lossy(), bytes_written, hash_bytes, sha256)
  }
  Ok(())
}

impl Task {
  pub fn new<S: ToString, S2: AsRef<Path>>(client: Client, url: S, filename: S2, temp: Option<S2>, sha256: String) -> Self {
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
        let mut stem = filename.file_name().unwrap_or_else(|| Path::new(&sha256).as_os_str()).to_os_string();
        stem.push(".tmp");
        filename.with_file_name(stem)
      }
    };
    Self { client, url, filename, temp, sha256, progress: None }
  }

  pub fn set_progress<P: Progress + 'static>(&mut self, progress: P) -> &mut Self {
    self.progress = Some(Box::new(progress));
    self
  }

  pub fn is_finished(&self) -> bool {
    check_sha256(&self.filename, &self.sha256).is_ok()
  }

  pub fn partial_len(&self) -> Option<u64> {
    if self.temp.exists() {
      std::fs::metadata(&self.temp).ok().map(|i| i.len())
    } else {
      None
    }
  }

  pub async fn download(&mut self) -> anyhow::Result<u64> {
    // https://gist.github.com/giuliano-oliveira/4d11d6b3bb003dba3a1b53f43d81b30d
    let mut downloaded = 0;
    trace!("downloading {} to {}", self.url, self.temp.to_string_lossy());
    let request = self.client.get(&self.url);
    trace!("req: {:?}", request);
    let resp = request.send().await?;
    trace!("resp: {:?}", resp);
    if !resp.status().is_success() {
      anyhow::bail!("request to {} failed with {}", self.url, resp.status())
    }
    let total_size = resp.content_length().unwrap_or(1);
    if let Some(progress) = self.progress.as_mut() {
      progress.set_length(total_size);
    }
    let mut file = File::create(&self.temp)?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
      let chunk = chunk?;
      file.write_all(&chunk)?;
      downloaded += chunk.len() as u64;
      if let Some(progress) = self.progress.as_mut() {
        progress.set_position(downloaded);
      }
    }
    Ok(downloaded)
  }

  pub async fn run(&mut self) -> anyhow::Result<()> {
    if self.is_finished() {
      trace!("{} exists", self.filename.to_string_lossy());
      return Ok(())
    }
    // TODO: partial download
    if let Some(len) = self.partial_len() {
      warn!("fixme: partial file exists ({}), overwrite", len);
    }
    self.download().await?;
    if let Err(e) = check_sha256(&self.temp, &self.sha256) {
      anyhow::bail!("file {} broken {:?}", self.temp.to_string_lossy(), e)
    } else if self.temp != self.filename {
      std::fs::rename(&self.temp, &self.filename)?;
    }
    Ok(())
  }

  #[tokio::main]
  #[allow(dead_code)]
  pub async fn run_sync(&mut self) -> anyhow::Result<()> {
    self.run().await
  }
}
