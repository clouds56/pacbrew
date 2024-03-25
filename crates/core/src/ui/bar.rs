use std::{collections::HashMap, fmt::Debug, hash::Hash, pin::Pin, sync::{Arc, RwLock}};

use futures::Future;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};

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

pub async fn with_progess_bar<'a, T, R, F, Fut>(
  active: ActiveSuspendable,
  style: Option<ProgressStyle>,
  init: Option<T>,
  f: F,
  tracker: impl EventListener<T>,
) -> R
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
  if let Some(style) = style.or_else(|| T::style()) {
    pb.set_style(style);
  }
  let old = active.write().unwrap().replace(Suspendable::ProgressBar(pb.clone()));
  if let Some(init) = &init {
    pb.on_event(init.clone());
  }
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

pub trait FeedMulti<S: Clone>: FeedBar {
  fn graduate(&self) -> bool;
  // None means overall
  fn tag(&self) -> Option<S>;
}

pub struct MultiBar<S> {
  handle: MultiProgress,
  style: Option<ProgressStyle>,
  state: std::sync::Mutex<HashMap<S, ProgressBar>>,
  overall: std::sync::Mutex<Option<ProgressBar>>,
  history: std::sync::Mutex<Vec<ProgressBar>>,
}
impl<S: Hash + Eq + Clone + Debug> MultiBar<S> {
  fn new(handle: MultiProgress, style: Option<ProgressStyle>) -> Self {
    Self {
      handle, style,
      state: std::sync::Mutex::new(HashMap::new()),
      overall: std::sync::Mutex::new(None),
      history: std::sync::Mutex::new(Vec::new()),
    }
  }
  fn get_overall(&self) -> ProgressBar {
    let mut overall = self.overall.lock().unwrap();
    match overall.as_ref() {
      Some(pb) => pb.clone(),
      _ => {
        debug!("create overall");
        let pb = self.handle.add(ProgressBar::new(0));
        if let Some(style) = &self.style {
          pb.set_style(style.clone());
        }
        overall.replace(pb.clone());
        pb
      }
    }
  }
  fn remove_overall(&self) {
    let mut overall = self.overall.lock().unwrap();
    match overall.take() {
      Some(pb) => self.handle.remove(&pb),
      _ => { warn!("multibar: remove overall not present") }
    }
  }
  fn get_pb(&self, tag: &Option<S>) -> ProgressBar {
    let mut state = self.state.lock().unwrap();
    let Some(tag) = tag.as_ref() else {
      return self.get_overall()
    };
    if let Some(pb) = state.get(tag) {
      pb.clone()
    } else {
      let index = if self.overall.lock().unwrap().is_some() { 1 } else { 0 };
      let pb = self.handle.insert_from_back(index, ProgressBar::new(0));
      state.insert(tag.clone(), pb.clone());
      if let Some(style) = &self.style {
        pb.set_style(style.clone());
      }
      pb
    }
  }
  fn remove_pb(&self, tag: &Option<S>) {
    let Some(tag) = tag.as_ref() else {
      return self.remove_overall()
    };
    match self.state.lock().unwrap().remove(tag) {
      Some(pb) => {
        self.handle.remove(&pb);
        pb.set_draw_target(ProgressDrawTarget::stderr());
        pb.finish();
        trace!(current=?pb.position(), max=?pb.length(), message=?pb.message(), "finish pb");
        self.history.lock().unwrap().push(pb);
      },
      _ => { warn!("multibar: remove some tag not present") }
    }
  }

}
impl<S: Hash + Eq + Clone + Debug, T: FeedMulti<S> + Clone> EventListener<T> for MultiBar<S> {
  fn on_event(&self, event: T) {
    let tag = event.tag();
    let pb = self.get_pb(&tag);
    pb.on_event(event.clone());
    if event.graduate() {
      trace!(message="finish pb", ?tag);
      pb.finish();
      self.remove_pb(&tag);
    }
  }
}
pub async fn with_progess_multibar<'a, S, T, R, F, Fut>(
  active: ActiveSuspendable,
  style: Option<ProgressStyle>,
  f: F,
  tracker: impl EventListener<T>
) -> R
where
  S: Hash + Eq + Clone + Debug,
  T: Clone + 'static + FeedMulti<S> + Debug,
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
  let style = style.or_else(|| T::style());
  let pb_multi = MultiProgress::new();
  let pb_multi_bar = MultiBar::new(pb_multi.clone(), style);
  let old = active.write().unwrap().replace(Suspendable::MultiProgress(pb_multi));
  let pb_tracker = Tracker::new(None);
  let mut events = pb_tracker.progress();
  let fut = unsafe { make_static(f(pb_tracker)) };
  let handle = tokio::spawn(fut);
  while let Some(event) = events.recv().await {
    debug!(?event);
    pb_multi_bar.on_event(event.clone());
    tracker.on_event(event);
  }
  pb_multi_bar.handle.clear().ok();
  drop(pb_multi_bar);
  *active.write().unwrap() = old;
  // since we make static, make sure the future is finished before returning
  handle.await.unwrap()
}

#[test]
fn test_multibar() {
  let active_pb = crate::tests::init_logger(None);
  #[derive(Clone, PartialEq, Eq, Debug)]
  pub struct Event {
    pub name: Option<i32>,
    pub position: u64,
    pub length: u64,
    pub finish: bool,
  }
  impl FeedBar for Event {
    fn style() -> Option<ProgressStyle> {
      Some(ProgressStyle::default_bar().template("[{bar:40.cyan/blue}] {pos}/{len} {msg}").unwrap())
    }
    fn message(&self) -> Option<String> { format!("{:?}", self.name).into() }
    fn position(&self) -> Option<u64> { self.position.into() }
    fn length(&self) -> Option<u64> { self.length.into() }
  }
  impl FeedMulti<i32> for Event {
    fn graduate(&self) -> bool { self.finish }
    fn tag(&self) -> Option<i32> { self.name }
  }
  let handle = MultiProgress::new();
  active_pb.write().unwrap().replace(Suspendable::MultiProgress(handle.clone()));
  let multi_bar = MultiBar::new(handle, None);
  let events = vec![
    Event { name: Some(1), position: 1, length: 10, finish: false },
    Event { name: Some(1), position: 5, length: 10, finish: false },
    Event { name: Some(1), position: 7, length: 10, finish: false },

    Event { name: Some(2), position: 1, length: 10, finish: false },
    Event { name: Some(2), position: 5, length: 10, finish: false },
    Event { name: Some(2), position: 7, length: 10, finish: false },
    Event { name: Some(2), position: 7, length: 10, finish: true },

    Event { name: Some(1), position: 7, length: 10, finish: true },

    Event { name: Some(3), position: 1, length: 10, finish: false },
    Event { name: Some(3), position: 5, length: 10, finish: false },
    Event { name: Some(3), position: 7, length: 10, finish: false },
    Event { name: Some(3), position: 7, length: 10, finish: true },

    Event { name: Some(4), position: 1, length: 10, finish: false },
    Event { name: Some(4), position: 5, length: 10, finish: false },
    Event { name: Some(4), position: 7, length: 10, finish: false },
    Event { name: Some(4), position: 7, length: 10, finish: true },
  ];
  for e in events {
    multi_bar.on_event(e);
    std::thread::sleep(std::time::Duration::from_millis(200))
  }

}
