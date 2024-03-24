use indicatif::{MultiProgress, ProgressBar};

pub struct PbWriter<W> {
  pb: Option<Suspendable>,
  writer: W,
  buffer: Vec<u8>,
}

impl<W> PbWriter<W> {
  pub fn new(pb: Option<Suspendable>, writer: W) -> Self {
    Self {
      pb,
      writer,
      buffer: Vec::new(),
    }
  }
  pub fn set_pb(&mut self, pb: &Suspendable) {
    self.pb = Some(pb.clone());
  }
}

impl<W: std::io::Write> std::io::Write for PbWriter<W> {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    self.buffer.extend_from_slice(buf);
    if buf.ends_with("\n".as_bytes()) {
      self.flush()?;
    }
    Ok(buf.len())
  }

  fn flush(&mut self) -> std::io::Result<()> {
    let buf = std::mem::take(&mut self.buffer);
    if let Some(ref pb) = self.pb {
      pb.suspend(|| {
        self.writer.write(&buf)?;
        self.writer.flush()
      })
    } else {
      self.writer.write(&buf)?;
      self.writer.flush()
    }
  }
}

#[derive(Clone)]
pub enum Suspendable {
  ProgressBar(ProgressBar),
  MultiProgress(MultiProgress),
}

impl Suspendable {
  fn suspend<R, F: FnOnce() -> R>(&self, f: F) -> R {
    match self {
      Suspendable::ProgressBar(pb) => pb.suspend(f),
      Suspendable::MultiProgress(pb) => pb.suspend(f),
    }
  }
}
