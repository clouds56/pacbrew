///! query would find in Vec<Formula> to get correspond Package
///! with there dependences.

use crate::{error::Result, package::formula::Formula};

#[tracing::instrument(level = "debug", skip_all, fields(formulas.len=formulas.len()))]
pub fn exec<'a, I: IntoIterator<Item = &'a str>>(formulas: &[Formula], query: I) -> Result<(Vec<String>, Vec<String>)> {
  todo!()
}

#[test]
fn test_exec() {
  crate::tests::init_logger();
  let formulas = crate::io::read::read_formulas("formula.json").unwrap();
  let result = exec(&formulas, ["wget", "gcc"]).unwrap();
}
