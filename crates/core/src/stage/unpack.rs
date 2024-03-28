use std::path::{Path, PathBuf};

use crate::{error::{ErrorExt, IoErrorExt, Result}, io::untar::{untar_gz, UnpackEvent}, package::package::PackageCache, ui::{event::DetailEvent, EventListener}};

pub async fn step() {

}

pub struct Value {
  pub name: String,
  pub dest: PathBuf,
}

pub async fn exec<'a, P: AsRef<Path>, I: IntoIterator<Item = &'a PackageCache> + Clone>(
  cellar: P,
  pkgs: I,
  tracker: impl EventListener<DetailEvent<usize, u64>>
) -> Result<Vec<Value>> {
  use DetailEvent::*;
  use crate::ui::event::Event::*;
  let mut result = Vec::new();
  tracker.on_event(Overall(Init { max: pkgs.clone().into_iter().count() }));
  for (i, pkg) in pkgs.into_iter().enumerate() {
    info!(cache_pkg=%pkg.cache_pkg.display(), "");
    let tmp_target = cellar.as_ref().join(&pkg.name).join("tmp");
    std::fs::remove_dir_all(&tmp_target).ok_not_found().when(("remove_dir_all", &tmp_target))?;
    std::fs::create_dir_all(&tmp_target).when(("create_dir_all", &tmp_target))?;

    tracker.on_event(Item(i, Message { name: format!("{}", pkg.name) }));
    tracker.on_event(Item(i, Message { name: format!("unpacking {}", pkg.name) }));
    untar_gz(&pkg.cache_pkg, &tmp_target, |e: UnpackEvent| {
      tracker.on_event(Item(i, Progress { current: e.pos, max: Some(e.total_size) }));
    }).await?;
    tracker.on_event(Item(i, Finish));
    tracker.on_event(Overall(Progress { current: i, max: None }));

    result.push(Value {
      name: pkg.name.clone(),
      dest: tmp_target,
    });
  }
  tracker.on_event(Overall(Finish));
  Ok(result)
}

#[tokio::test]
async fn test_verify() {
  use crate::tests::*;
  let active_pb = init_logger(None);

  let packages = get_formulas().into_iter()
    .map(crate::package::package::PackageVersion::from).collect::<Vec<_>>();
  let pkgs = crate::stage::verify::get_pkgs(&packages, CACHE_PATH);

  let result = crate::ui::with_progess_multibar(active_pb, None, |tracker| exec(
    "cache/root/opt",
    pkgs.iter().map(|i| &i.2),
    tracker
  ), ()).await.unwrap();

  assert_eq!(result.len(), pkgs.len());
  for i in result {
    assert!(i.dest.exists());
  }
}
