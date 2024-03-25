use std::{pin::Pin, sync::{Arc, RwLock}};

use futures::Future;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::{tracker::Tracker, EventListener};

pub type ActiveSuspendable = Arc<RwLock<Option<Suspendable>>>;

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

pub trait FeedBar {
  fn style() -> Option<ProgressStyle> {
    None
  }
  fn message(&self) -> Option<String>;
  fn position(&self) -> Option<u64>;
  fn length(&self) -> Option<u64>;
}

impl<T: FeedBar> EventListener<T> for ProgressBar {
  fn on_event(&self, event: T) {
    if let Some(msg) = event.message() {
      self.set_message(msg);
    }
    if let Some(len) = event.length() {
      self.set_length(len);
    }
    if let Some(pos) = event.position() {
      self.set_position(pos);
    }
  }
}

pub async fn with_progess_bar<'a, T, R, F, Fut>(active: ActiveSuspendable, init: T, f: F, tracker: impl EventListener<T>) -> R
where
  T: Clone + 'static + FeedBar,
  F: FnOnce(Tracker<T>) -> Fut + 'a,
  Fut: Future<Output = R> + Send + 'a,
  R: Send + 'static,
{
  unsafe fn make_static<R>(f: impl Future<Output=R> + Send) -> Pin<Box<dyn Future<Output = R> + Send + 'static>> {
    std::mem::transmute::<
      Pin<Box<dyn Future<Output=R> + Send>>,
      Pin<Box<dyn Future<Output=R> + Send + 'static>>
    >(Box::pin(f))
  }
  let pb = ProgressBar::new(0);
  if let Some(style) = T::style() {
    pb.set_style(style);
  }
  let old = active.write().unwrap().replace(Suspendable::ProgressBar(pb.clone()));
  pb.on_event(init.clone());
  let pb_tracker = Tracker::new(init);
  let mut events = pb_tracker.progress();
  let fut = unsafe { make_static(f(pb_tracker)) };
  let handle = tokio::spawn(fut);
  while let Some(event) = events.recv().await {
    pb.on_event(event.clone());
    tracker.on_event(event);
  }
  pb.finish();
  drop(pb);
  *active.write().unwrap() = old;
  // since we make static, make sure the future is finished before returning
  handle.await.unwrap()
}
