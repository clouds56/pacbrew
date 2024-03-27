use std::path::Path;

use crate::{error::Result, package::package::PackageCache, ui::{event::DetailEvent, EventListener}};

pub async fn exec<'a, P1: AsRef<Path>, P2: AsRef<Path>, I: IntoIterator<Item = &'a PackageCache>>(
  prefix: P1,
  cellar: P2,
  pkgs: I,
  tracker: impl EventListener<DetailEvent<usize, u64>>
) -> Result<()> {
  use DetailEvent::*;
  use crate::ui::event::Event::*;
  for (i, pkg) in pkgs.into_iter().enumerate() {
    info!(cache_pkg=%pkg.cache_pkg.display(), "");

    tracker.on_event(Overall(Progress { current: i, max: None }));
  }
  tracker.on_event(Overall(Finish));
  Ok(())
}

#[tokio::test]
async fn test_verify() {
  use crate::tests::*;
  let active_pb = init_logger(None);

  let packages = get_formulas().into_iter()
    .map(crate::package::package::PackageVersion::from).collect::<Vec<_>>();
  let pkgs = crate::stage::verify::get_pkgs(&packages, CACHE_PATH);

  crate::ui::with_progess_multibar(active_pb, None, |tracker| exec(
    "root",
    "root/opt",
    pkgs.iter().map(|i| &i.2),
    tracker
  ), ()).await.unwrap();
}
