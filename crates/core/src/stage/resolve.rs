use std::collections::{HashMap, HashSet, VecDeque};

///! query would find in Vec<Formula> to get correspond Package
///! with there dependences.

use crate::{error::Result, package::{formula::Formula, package::PackageOffline}};

pub struct Value {
  pub names: Vec<String>,
  pub resolved: Vec<PackageOffline>,
}

#[tracing::instrument(level = "debug", skip_all, fields(formulas.len=formulas.len()))]
pub fn exec<'a, I: IntoIterator<Item = &'a str>>(formulas: &[Formula], query: I) -> Result<Value> {
  let mut queue = VecDeque::from_iter(query);
  let mut direct_names = queue.iter().map(|&i| (i, i)).collect::<HashMap<_,_>>();
  let mut visited = HashSet::<&str>::new();
  let mut formula_index = formulas.iter().map(|f| (f.name.as_str(), f)).collect::<HashMap<_, _>>();
  formula_index.extend(formulas.iter().flat_map(|f| f.oldname.iter().map(move |name| (name.as_str(), f))));
  formula_index.extend(formulas.iter().flat_map(|f| f.oldnames.iter().map(move |name| (name.as_str(), f))));
  formula_index.extend(formulas.iter().flat_map(|f| f.aliases.iter().map(move |name| (name.as_str(), f))));
  formula_index.extend(formulas.iter().map(|f| (f.full_name.as_str(), f)));
  let mut collected = Vec::new();
  while let Some(item) = queue.pop_front() {
    let formula = *formula_index.get(item).ok_or_else(|| crate::error::Error::package_not_found(item))?;
    if direct_names.contains_key(item) {
      direct_names.insert(item, &formula.full_name);
    }
    if visited.contains(formula.name.as_str()) {
      continue;
    }
    visited.insert(&formula.name);
    // TODO: warn about cyclic dep here;
    let deps = formula.dependencies.iter().filter(|i| !visited.contains(i.as_str())).map(|d| d.as_str()).collect::<Vec<_>>();
    if !deps.is_empty() {
      debug!(deps.from=formula.name, deps.to=deps.join(","));
    }
    queue.extend(deps);
    collected.push(formula.clone());
  }
  let mut direct_names = direct_names.values().map(|i| i.to_string()).collect::<Vec<_>>();
  direct_names.sort();
  // TODO: convert formula to package
  Ok(Value {
    names: direct_names,
    resolved: collected.into_iter().map(|f| f.into()).collect(),
  })
}

#[test]
fn test_exec() {
  crate::tests::init_logger();
  let query = ["wget", "llvm", "python", "ffmpeg"];
  let formulas = crate::io::read::read_formulas("formula.json").unwrap();
  let result = exec(&formulas, query).unwrap();

  info!(names=result.names.join(","));
  info!(resolved=result.resolved.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(","));
  result.resolved.iter().for_each(|package| trace!(?package));
  assert_eq!(result.names.len(), query.len());
  assert_eq!(result.names.iter().map(|i| i.split('@').next().unwrap()).collect::<HashSet<_>>(), query.iter().cloned().collect());
  assert_eq!(result.resolved.len(), result.resolved.iter().map(|f| &f.name).collect::<HashSet<_>>().len())
}
