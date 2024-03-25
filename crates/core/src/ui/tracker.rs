use std::sync::{Arc, Mutex};

use futures::FutureExt;
use tokio::sync::broadcast::{error::RecvError, Receiver, Sender};

use super::EventListener;

pub struct Tracker<T> {
  init: Arc<Mutex<T>>,
  tx: Sender<T>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Tracker<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ProgressTracker").field("init", &self.init).field("tx", &"Sender").finish()
  }
}

impl<T: Clone> Tracker<T> {
  pub fn new(init: T) -> Self {
    let (tx, _) = tokio::sync::broadcast::channel(1024);
    Self { init: Arc::new(Mutex::new(init)), tx }
  }
}

impl<T: Clone + 'static> EventListener<T> for Tracker<T> {
  fn on_event(&self, event: T) {
    self.send(event);
  }
}

impl<T: Clone + 'static> Tracker<T> {
  pub fn send(&self, event: T) -> bool {
    *self.init.lock().unwrap() = event.clone();
    self.tx.send(event).is_ok()
  }

  pub fn progress(&self) -> Events<T> {
    let stream = self.tx.subscribe();
    Events { init: self.init.clone(), stream, lagged: false }
  }
}

pub struct Events<T> {
  pub init: Arc<Mutex<T>>,
  stream: Receiver<T>,
  lagged: bool,
}

impl<T: Clone> Events<T> {
  pub async fn recv(&mut self) -> Option<T> {
    let item = loop {
      if self.lagged {
        self.stream = self.stream.resubscribe();
      }
      let item = match self.stream.recv().await {
        Ok(item) => item,
        Err(RecvError::Closed) => return None,
        Err(RecvError::Lagged(_)) => {
          self.lagged = true;
          match self.stream.recv().now_or_never() {
            Some(Ok(item)) => item,
            _ => continue,
          }
        }
      };
      break item;
    };
    Some(item)
  }
}
