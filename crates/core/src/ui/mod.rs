pub mod bar;
pub mod tracker;
pub mod event;

pub use bar::{with_progess_bar, with_progess_multibar};

pub trait EventListener<T> {
  fn on_event(&self, event: T);
}

impl<T> EventListener<T> for () {
  fn on_event(&self, _: T) {}
}

impl<T, F: Fn(T)> EventListener<T> for F {
  fn on_event(&self, event: T) {
    self(event)
  }
}
