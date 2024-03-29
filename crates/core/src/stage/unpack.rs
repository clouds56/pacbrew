use std::{ffi::OsString, path::{Path, PathBuf}, sync::{Arc, Mutex}};

use crate::{error::{ErrorExt, IoErrorExt, Result}, io::{relocate::{relocate, RelocateType, RelocationPattern}, untar::{untar_gz, UnpackEvent}}, package::package::{PackageCache, PackageInstalled}, ui::{event::{BytesEvent, DetailEvent}, EventListener}};

pub async fn step<P: AsRef<Path>, Q: AsRef<Path>>(pattern: &RelocationPattern, cache_pkg: P, dest_dir: Q, tracker: impl EventListener<BytesEvent>) -> Result<Vec<(PathBuf, RelocateType)>> {
  use crate::ui::event::Event::*;
  let dest_dir = dest_dir.as_ref();
  let relocates = Arc::new(Mutex::new(Vec::new()));
  untar_gz(&cache_pkg, dest_dir, |e: UnpackEvent| {
    if let Some(name) = e.current_entry {
      match relocate(dest_dir.join(&name), pattern) {
        Ok(RelocateType::None) => {},
        Ok(ty) => relocates.lock().unwrap().push((name, ty)),
        Err(e) => {
          error!(error=?e, "relocate failed");
          relocates.lock().unwrap().push((name, RelocateType::None));
        },
      }
    }
    tracker.on_event(Progress { current: e.pos, max: Some(e.total_size) });
  }).await?;
  let relocates: Vec<_> = std::mem::take(relocates.lock().unwrap().as_mut());
  for (name, ty) in &relocates {
    if ty == &RelocateType::None {
      warn!(name=%name.display(), "relocate failed");
      return Err(std::io::Error::other(format!("relocate {} failed", name.display()))).when(("unpack", cache_pkg.as_ref()))?;
    }
  }
  Ok(relocates)
}

pub struct Args<'a> {
  pub prefix: &'a Path,
  pub cellar: &'a Path,
  pub force: bool,
}
impl<'a> Args<'a> {
  pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(prefix: &'a P1, cellar: &'a P2) -> Self {
    Self { prefix: prefix.as_ref(), cellar: cellar.as_ref(), force: false }
  }
  pub fn force(self, f: bool) -> Self {
    Self { force: f, ..self }
  }
}

pub async fn exec<'a, I: IntoIterator<Item = &'a PackageCache> + Clone>(
  args: Args<'a>,
  pkgs: I,
  tracker: impl EventListener<DetailEvent<usize, u64>>
) -> Result<Vec<PackageInstalled>> {
  use DetailEvent::*;
  use crate::ui::event::Event::*;
  let mut result = Vec::new();
  let pattern = RelocationPattern::new(args.prefix, args.cellar);
  tracker.on_event(Overall(Init { max: pkgs.clone().into_iter().count() }));
  for (i, pkg) in pkgs.into_iter().enumerate() {
    info!(cache_pkg=%pkg.cache_pkg.display(), "");
    let tmp_target = Path::new(args.cellar).join(&pkg.name).join("tmp");
    std::fs::remove_dir_all(&tmp_target).ok_not_found().when(("remove_dir_all", &tmp_target))?;
    std::fs::create_dir_all(&tmp_target).when(("create_dir_all", &tmp_target))?;

    tracker.on_event(Item(i, Message { name: format!("{}", pkg.name) }));
    tracker.on_event(Item(i, Message { name: format!("unpacking {}", pkg.name) }));
    let reloc = step(&pattern, &pkg.cache_pkg, &tmp_target, |e: BytesEvent| tracker.on_event(Item(i, e))).await?;
    tracker.on_event(Item(i, Finish));
    tracker.on_event(Overall(Progress { current: i, max: None }));
    let version = guess_version(tmp_target.join(&pkg.name)).when(("unpack guess version", &tmp_target))?;
    let tmp_target_versioned = tmp_target.join(&pkg.name).join(&version);
    let target_versioned = Path::new(args.cellar).join(&pkg.name).join(&version);
    debug!(tmp_target_versioned=%tmp_target_versioned.display(), target_versioned=%target_versioned.display(), "rename");
    if args.force {
      std::fs::remove_dir_all(&target_versioned).ok();
    }
    std::fs::rename(&tmp_target_versioned, &target_versioned).when(("unpack.rename", &tmp_target_versioned))?;
    std::fs::remove_dir_all(&tmp_target).ok();

    result.push(PackageInstalled {
      name: pkg.name.clone(),
      dest: target_versioned,
      version: version.to_string_lossy().to_string(),
      reloc: reloc.into_iter().collect(),
    });
  }
  tracker.on_event(Overall(Finish));
  Ok(result)
}

fn guess_version(path: PathBuf) -> std::io::Result<OsString> {
  let mut children = path.read_dir()?;
  let child = children.next().ok_or_else(|| std::io::Error::other("no children"))??;
  if children.next().is_some() {
    return Err(std::io::Error::other("multiple child found"));
  }
  Ok(child.file_name())
}

#[tokio::test]
async fn test_verify() {
  use crate::tests::*;
  let active_pb = init_logger(None);

  let packages = get_formulas().into_iter()
    .map(crate::package::package::PackageVersion::from).collect::<Vec<_>>();
  let pkgs = crate::stage::verify::get_pkgs(&packages, CACHE_PATH);

  let result = crate::ui::with_progess_multibar(active_pb, None, |tracker| exec(
    Args::new(&PREFIX_PATH, &CELLAR_PATH).force(true),
    pkgs.iter().map(|i| &i.2),
    tracker
  ), ()).await.unwrap();

  assert_eq!(result.len(), pkgs.len());
  for i in result {
    assert!(i.dest.exists());
  }
}
