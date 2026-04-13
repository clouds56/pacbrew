use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use core_lib::io::read::read_formulas;
use core_lib::package::formula::Formula;

use crate::config::Config;

#[derive(Debug, Clone, clap::Args)]
pub struct TreeArgs {
  #[arg(long)]
  pub rev: bool,

  pub name: String,
}

pub fn run(config: &Config, args: TreeArgs) -> Result<()> {
  let formulas = read_formulas(config.base.formula_json())?;
  let index = build_formula_index(&formulas);

  let root = resolve_formula_name(&index, &args.name)?;
  println!("{root}");

  let graph = if args.rev {
    build_reverse_graph(&formulas, &index)
  } else {
    build_dependency_graph(&formulas, &index)
  };

  let mut stack = HashSet::new();
  stack.insert(root.to_string());
  print_children(&root, "", &graph, &mut stack);
  Ok(())
}

fn build_formula_index<'a>(formulas: &'a [Formula]) -> HashMap<&'a str, &'a Formula> {
  let mut index = formulas
    .iter()
    .map(|formula| (formula.name.as_str(), formula))
    .collect::<HashMap<_, _>>();

  index.extend(
    formulas
      .iter()
      .flat_map(|formula| formula.oldname.iter().map(move |name| (name.as_str(), formula))),
  );
  index.extend(
    formulas
      .iter()
      .flat_map(|formula| formula.oldnames.iter().map(move |name| (name.as_str(), formula))),
  );
  index.extend(
    formulas
      .iter()
      .flat_map(|formula| formula.aliases.iter().map(move |name| (name.as_str(), formula))),
  );
  index.extend(
    formulas
      .iter()
      .map(|formula| (formula.full_name.as_str(), formula)),
  );

  index
}

fn resolve_formula_name(index: &HashMap<&str, &Formula>, name: &str) -> Result<String> {
  index
    .get(name)
    .map(|formula| formula.name.clone())
    .ok_or_else(|| anyhow!("package not found: {name}"))
}

fn normalize_name(index: &HashMap<&str, &Formula>, name: &str) -> Option<String> {
  index.get(name).map(|formula| formula.name.clone())
}

fn build_dependency_graph(
  formulas: &[Formula],
  index: &HashMap<&str, &Formula>,
) -> HashMap<String, Vec<String>> {
  let mut graph = formulas
    .iter()
    .map(|formula| (formula.name.clone(), Vec::<String>::new()))
    .collect::<HashMap<_, _>>();

  for formula in formulas {
    let mut deps = formula
      .dependencies
      .iter()
      .filter_map(|dependency| normalize_name(index, dependency))
      .collect::<Vec<_>>();
    deps.sort();
    deps.dedup();
    if let Some(children) = graph.get_mut(&formula.name) {
      *children = deps;
    }
  }

  graph
}

fn build_reverse_graph(
  formulas: &[Formula],
  index: &HashMap<&str, &Formula>,
) -> HashMap<String, Vec<String>> {
  let mut graph = formulas
    .iter()
    .map(|formula| (formula.name.clone(), Vec::<String>::new()))
    .collect::<HashMap<_, _>>();

  for formula in formulas {
    for dependency in &formula.dependencies {
      let Some(dependency) = normalize_name(index, dependency) else {
        continue;
      };
      if let Some(dependents) = graph.get_mut(&dependency) {
        dependents.push(formula.name.clone());
      }
    }
  }

  for dependents in graph.values_mut() {
    dependents.sort();
    dependents.dedup();
  }

  graph
}

fn print_children(
  node: &str,
  prefix: &str,
  graph: &HashMap<String, Vec<String>>,
  visited: &mut HashSet<String>,
) {
  let children = graph
    .get(node)
    .cloned()
    .unwrap_or_default();

  for (i, child) in children.iter().enumerate() {
    let is_last = i + 1 == children.len();
    let branch = if is_last { "`-- " } else { "|-- " };
    let next_prefix = if is_last {
      format!("{prefix}    ")
    } else {
      format!("{prefix}|   ")
    };

    if visited.contains(child) {
      println!("{prefix}{branch}{child}");
      println!("{next_prefix}`-- ...");
      continue;
    }

    println!("{prefix}{branch}{child}");
    visited.insert(child.clone());
    print_children(child, &next_prefix, graph, visited);
  }
}
