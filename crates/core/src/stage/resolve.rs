use std::{borrow::Borrow, collections::{HashMap, HashSet, VecDeque}, time::Duration};

///! query would find in Vec<Formula> to get correspond Package
///! with there dependences.

use crate::{error::Result, package::{formula::Formula, package::PackageVersion}, ui::{event::ItemEvent, EventListener}};

pub struct Value {
  pub names: Vec<String>,
  pub packages: Vec<PackageVersion>,
}

#[tracing::instrument(level = "debug", skip_all, fields(formulas.len=formulas.len()))]
pub async fn exec<'a, S, I>(
  formulas: &[Formula],
  query: I,
  tracker: impl EventListener<ItemEvent>
) -> Result<Value>
where
  S: Borrow<str> + ?Sized + 'a,
  I: IntoIterator<Item = &'a S>,
{
  let mut queue = VecDeque::from_iter(query.into_iter().map(|i| i.borrow()));
  let mut direct_names = queue.iter().map(|&i| (i, i)).collect::<HashMap<_,_>>();
  let mut visited = HashSet::<&str>::new();
  let mut formula_index = formulas.iter().map(|f| (f.name.as_str(), f)).collect::<HashMap<_, _>>();
  formula_index.extend(formulas.iter().flat_map(|f| f.oldname.iter().map(move |name| (name.as_str(), f))));
  formula_index.extend(formulas.iter().flat_map(|f| f.oldnames.iter().map(move |name| (name.as_str(), f))));
  formula_index.extend(formulas.iter().flat_map(|f| f.aliases.iter().map(move |name| (name.as_str(), f))));
  formula_index.extend(formulas.iter().map(|f| (f.full_name.as_str(), f)));
  let mut collected = Vec::new();

  let mut i = 0;
  while let Some(item) = queue.pop_front() {
    i += 1;
    tracker.on_event(ItemEvent::Progress { current: i, max: Some(i + queue.len()) });
    tracker.on_event(ItemEvent::Message { name: format!("resolving {}", item) });
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
    // TODO: better parking method
    tokio::time::sleep(Duration::from_millis(0)).await;
  }
  tracker.on_event(ItemEvent::Message { name: format!("resolve finished") });
  tracker.on_event(ItemEvent::Finish);
  let mut direct_names = direct_names.values().map(|i| i.to_string()).collect::<Vec<_>>();
  direct_names.sort();
  // TODO: convert formula to package
  Ok(Value {
    names: direct_names,
    packages: collected.into_iter().map(|f| f.into()).collect(),
  })
}

#[tokio::test]
async fn test_resolve() {
  use crate::tests::*;
  let active_pb = init_logger(None);
  let query = ["wget", "llvm", "python", "ffmpeg"];
  let formulas = get_formulas();

  let init = ItemEvent::Init { max: query.len() };
  let result = crate::ui::with_progess_bar(active_pb.clone(), None, Some(init), |tracker| async move {
    exec(&formulas, query, tracker).await
  }, ()).await.unwrap();

  info!(names=result.names.join(","));
  info!(resolved=result.packages.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(","));
  result.packages.iter().for_each(|package| trace!(?package));
  assert_eq!(result.names.len(), query.len());
  assert_eq!(result.names.iter().map(|i| i.split('@').next().unwrap()).collect::<HashSet<_>>(), query.iter().cloned().collect());
  assert_eq!(result.packages.len(), result.packages.iter().map(|f| &f.name).collect::<HashSet<_>>().len())
}
