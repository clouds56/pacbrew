use std::sync::{Arc, Mutex};

use futures::FutureExt;
use tokio::sync::broadcast::{error::RecvError, Receiver, Sender};

pub struct Progress<T> {
  init: Arc<Mutex<T>>,
  tx: Sender<T>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Progress<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ProgressTracker").field("init", &self.init).field("tx", &"Sender").finish()
  }
}

impl<T: Clone> Progress<T> {
  pub fn new(init: T) -> Self {
    let (tx, _) = tokio::sync::broadcast::channel(1024);
    Self { init: Arc::new(Mutex::new(init)), tx }
  }
}

impl<T: Clone + 'static> ProgressTrack<T> for Progress<T> {
  fn send(&self, event: T) -> bool {
    *self.init.lock().unwrap() = event.clone();
    self.tx.send(event).is_ok()
  }

  fn progress(&self) -> Events<T> {
    let stream = self.tx.subscribe();
    Events { init: self.init.clone(), stream, lagged: false }
  }
}

pub trait ProgressTrack<T> {
  fn send(&self, event: T) -> bool;
  fn progress(&self) -> Events<T>;
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
