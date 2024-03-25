pub mod resolve;
pub mod probe;
pub mod download;

#[derive(Debug, Clone)]
pub struct Event {
  pub name: String,
  pub current: usize,
  pub max: Option<usize>,
}

impl Event {
  pub fn new(max: usize) -> Self {
    Self { name: String::new(), current: 0, max: Some(max) }
  }
}

impl crate::ui::bar::FeedBar for Event {
  fn message(&self) -> Option<String> { Some(self.name.clone()) }
  fn position(&self) -> Option<u64> { Some(self.current as _) }
  fn length(&self) -> Option<u64> { self.max.map(|i| i as _) }
}
