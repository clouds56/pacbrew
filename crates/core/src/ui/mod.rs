pub mod bar;
pub mod tracker;

pub trait EventListener<T> {
  fn on_event(&self, event: T);
}

impl<T> EventListener<T> for () {
  fn on_event(&self, _: T) {}
}
