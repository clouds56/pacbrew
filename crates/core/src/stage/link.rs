use std::path::Path;

use crate::{error::{ErrorExt, Result}, package::package::{PackageInstalled, PackageLinked}, ui::{event::ItemEvent, EventListener}};

pub fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q, force: bool) -> std::io::Result<()> {
  let link = link.as_ref();
  let Some(link_parent) = link.parent() else {
    return Err(std::io::Error::other("link parent none"))
  };
  if !original.as_ref().exists() {
    return Err(std::io::Error::other("link original not present"))
  }
  // force only override link, not intend to remove regular file
  // TODO: remove empty dir?
  if force && link.read_link().is_ok() {
    symlink::remove_symlink_dir(link)?;
    assert!(!link.exists());
    assert!(link.read_link().is_err());
  }
  // let src = original.as_ref();
  let src = pathdiff::diff_paths(original.as_ref(), link_parent).unwrap();
  debug!(origin=%src.display(), link=%link.display(), "ln -s");
  symlink::symlink_dir(src, link)
}

pub fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q, force: bool) -> std::io::Result<()> {
  let link = link.as_ref();
  let Some(link_parent) = link.parent() else {
    return Err(std::io::Error::other("link parent none"))
  };
  if !original.as_ref().exists() {
    return Err(std::io::Error::other("link original not present"))
  }
  // force only override link, not intend to remove regular file
  if force && link.read_link().is_ok() {
    symlink::remove_symlink_file(link)?;
  }
  let src = pathdiff::diff_paths(original.as_ref(), link_parent).unwrap();
  debug!(origin=%src.display(), link=%link.display(), "ln -s");
  symlink::symlink_file(src, link)
}

pub async fn exec<'a, P: AsRef<Path>, I: IntoIterator<Item = &'a PackageInstalled>>(
  prefix: P,
  pkgs: I,
  tracker: impl EventListener<ItemEvent>,
) -> Result<Vec<PackageLinked>> {
  let prefix = prefix.as_ref();
  let opt_dir = prefix.join("opt");
  std::fs::create_dir_all(&opt_dir).when(("create_dir_all", &opt_dir))?;
  let mut result = Vec::new();
  for (i, pkg) in pkgs.into_iter().enumerate() {
    tracker.on_event(ItemEvent::Message { name: format!("linking {}", pkg.name) });
    symlink_dir(&pkg.dest, opt_dir.join(&pkg.name), true).ok();
    tracker.on_event(ItemEvent::Progress { current: i, max: None });
    result.push(PackageLinked {
      name: pkg.name.clone(),
      dest: pkg.dest.clone(),
      version: pkg.version.clone(),
    })
  }
  tracker.on_event(ItemEvent::Message { name: "link finished".to_string() });
  tracker.on_event(ItemEvent::Finish);
  Ok(result)
}

#[cfg(test)]
fn guess_installed(path: &Path) -> Option<PackageInstalled> {
  let mut versions = std::fs::read_dir(path).ok()?
    .filter_map(|v| v.ok())
    .filter_map(|v| Some((v.metadata().ok()?.created().ok()?, v)))
    .collect::<Vec<_>>();
  versions.sort_by_key(|i| i.0);
  let version = versions.last()?.1.file_name().into_string().ok()?;
  Some(PackageInstalled {
    name: path.file_name()?.to_string_lossy().to_string(),
    dest: path.join(&version),
    version,
    reloc: Default::default(),
  })
}

#[tokio::test]
async fn test_link() {
  use crate::tests::*;
  init_logger(None);

  let mut pkgs = Vec::new();
  let cellar_dir = Path::new(CELLAR_PATH);
  for i in std::fs::read_dir(cellar_dir).unwrap() {
    let i = i.unwrap();
    let name = i.file_name().to_string_lossy().to_string();
    if let Some(pkg) = guess_installed(&cellar_dir.join(name)) {
      // debug!(?pkg);
      pkgs.push(pkg)
    }
  }

  let result = exec(PREFIX_PATH, &pkgs, ()).await.unwrap();
  assert_eq!(result.len(), pkgs.len())
}
