use super::{bar::{FeedBar, FeedMulti}, EventListener};

pub trait AsU64: Copy {
  fn as_u64(self) -> u64;
}
impl AsU64 for usize {
  fn as_u64(self) -> u64 { self as _ }
}
impl AsU64 for u64 {
  fn as_u64(self) -> u64 { self }
}

pub type ItemEvent = Event<usize>;
pub type BytesEvent = Event<u64>;

#[derive(Debug, Clone)]
pub enum Event<T> {
  Init { max: T },
  Progress { current: T, max: Option<T> },
  Message { name: String },
  Finish,
}

impl<T: AsU64> FeedBar for Event<T> {
  fn message(&self) -> Option<String> {
    match self {
      Self::Message { name } => Some(name.clone()),
      _ => None,
    }
  }

  fn position(&self) -> Option<u64> {
    match self {
      Self::Progress { current, .. } => Some(current.as_u64()),
      _ => None,
    }
  }

  fn length(&self) -> Option<u64> {
    match self {
      Self::Init { max } => Some(max.as_u64()),
      Self::Progress { max, .. } => max.map(|i| i.as_u64()),
      _ => None,
    }
  }
}

#[derive(Debug, Clone)]
pub enum DetailEvent<S, T> {
  Overall(Event<S>),
  Item(usize, Event<T>),
}

impl<S: AsU64, T: AsU64> FeedBar for DetailEvent<S, T> {
  fn message(&self) -> Option<String> {
    match self {
      Self::Overall(e) => e.message(),
      Self::Item(_, e) => e.message(),
    }
  }

  fn position(&self) -> Option<u64> {
    match self {
      Self::Overall(e) => e.position(),
      Self::Item(_, e) => e.position(),
    }
  }

  fn length(&self) -> Option<u64> {
    match self {
      Self::Overall(e) => e.length(),
      Self::Item(_, e) => e.length(),
    }
  }
}

impl<S: AsU64, T: AsU64> FeedMulti<usize> for DetailEvent<S, T> {
  fn graduate(&self) -> bool {
    match self {
      Self::Overall(e) => matches!(e, Event::Finish),
      Self::Item(_, e) => matches!(e, Event::Finish),
    }
  }

  fn tag(&self) -> Option<usize> {
    match self {
      Self::Overall(_) => None,
      Self::Item(i, _) => Some(*i),
    }
  }
}

pub fn simplify_tracker<S, T>(tracker: impl EventListener<Event<S>>) -> impl EventListener<DetailEvent<S, T>> {
  move |event| match event {
    DetailEvent::Overall(e) => tracker.on_event(e),
    DetailEvent::Item(_, _) => return,
  }
}
