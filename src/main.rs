mod formula;

pub use formula::Formula;

fn main() -> anyhow::Result<()> {
  let formula_str = include_str!("../cache/formula.json");
  let formula = serde_json::from_str::<Vec<Formula>>(formula_str)?;
  println!("{:?}", &formula[..10]);
  Ok(())
}
