use std::path::Path;

use serde::de::DeserializeOwned;

use crate::error::{ErrorExt, Result};

pub fn read_json<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T> {
  let path = path.as_ref();
  let s = std::fs::read_to_string(path).when(("read", path))?;
  Ok(serde_json::from_str(&s).when(std::any::type_name::<T>())?)
}

pub fn read_formulas<P: AsRef<Path>>(path: P) -> Result<Vec<crate::package::formula::Formula>> {
  read_json(path)
}
