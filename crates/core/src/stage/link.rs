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

/// https://github.com/Homebrew/brew/blob/master/Library/Homebrew/keg.rb#L406
/// ```ruby
/// KEG_LINK_DIRECTORIES = %w[
///   bin etc include lib sbin share var
/// ].freeze
///
/// # Locale-specific directories have the form `language[_territory][.codeset][@modifier]`
/// LOCALEDIR_RX = %r{(locale|man)/([a-z]{2}|C|POSIX)(_[A-Z]{2})?(\.[a-zA-Z\-0-9]+(@.+)?)?}
/// INFOFILE_RX = %r{info/([^.].*?\.info|dir)$}
/// # These paths relative to the keg's share directory should always be real
/// # directories in the prefix, never symlinks.
/// SHARE_PATHS = %w[
///   aclocal doc info java locale man
///   man/man1 man/man2 man/man3 man/man4
///   man/man5 man/man6 man/man7 man/man8
///   man/cat1 man/cat2 man/cat3 man/cat4
///   man/cat5 man/cat6 man/cat7 man/cat8
///   applications gnome gnome/help icons
///   mime-info pixmaps sounds postgresql
/// ].freeze
/// ```
///
/// strategies
/// - `:skip_dir`: direct children with type of file should be symlink
/// - `:mkpath`: every file recursively should be symlink
/// - `:link`: direct files and dirs should be symlink, this is the default value
///
pub fn shared_dir() -> Vec<String> {
  vec![
    "bin", // :skip_dir
    "sbin", // :skip_dir
    "etc", // :mkpath
    "include", // :link
    // include:mkpath => %r{^postgresql@\d+/}
    "lib/pkgconfig", "lib/cmake", "lib/dtrace", // :mkpath
    "lib", // :link
    // lib:skip_file => "charset.alias"
    // lib:mkpath => /^gdk-pixbuf/, "ghc", /^gio/, /^lua/, /^mecab/, /^node/, /^ocaml/, /^perl5/, "php", %r{^postgresql@\d+/}, /^python[23]\.\d+/, /^R/, /^ruby/
    "share",
    // share:info => INFOFILE_RX
    // share:skip_file => "locale/locale.alias", %r{^icons/.*/icon-theme\.cache$}
    // share:mkpath => LOCALEDIR_RX, %r{^icons/}, /^zsh/, /^fish/, %r{^lua/}, %r{^guile/}, %r{^postgresql@\d+/}, *SHARE_PATHS
    "Frameworks", // :link
    // Frameworks:mkpath => %r{[^/]*\.framework(/Versions)?$}
  ].iter().map(|i| i.to_string()).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
  File, // :skip_dir
  Recursively, // :mkpath
  Child, // :link
}

pub fn list_dir<P: AsRef<Path>, Q: AsRef<str>>(base: P, path: Q, stretegy: Strategy) -> Result<Vec<String>> {
  let base = base.as_ref();
  let path = path.as_ref();
  debug!(path=%path, "list_dir");
  if !base.join(path).exists() { return Ok(vec![]) }
  let mut result = Vec::new();
  for i in std::fs::read_dir(base.join(path)).when(("list_dir.read_dir", &Path::new(path)))? {
    let i = i.when(("list_dir.read_dir_entry", &Path::new(path)))?;
    let file_type = i.file_type().when(("list_dir.file_type", &Path::new(path)))?;
    let name = i.file_name().to_string_lossy().to_string();
    let relative_name = format!("{}/{}", path, name);
    if file_type.is_dir() {
      if stretegy == Strategy::Recursively {
        result.extend(list_dir(base, relative_name, stretegy)?);
      } else if stretegy == Strategy::Child {
        result.push(relative_name)
      }
    } else if stretegy != Strategy::Recursively {
      result.push(relative_name)
    }
  }
  Ok(result)
}

fn collect_link_targets(opt_path: &Path) -> Result<Vec<String>> {
  let mut to_link = Vec::new();
  to_link.extend(list_dir(opt_path, "bin", Strategy::File)?);
  to_link.extend(list_dir(opt_path, "lib/pkgconfig", Strategy::Recursively)?);
  to_link.extend(list_dir(opt_path, "lib/cmake", Strategy::Recursively)?);
  to_link.extend(list_dir(opt_path, "lib/dtrace", Strategy::Recursively)?);
  to_link.extend(list_dir(opt_path, "lib", Strategy::Child)?);
  to_link = to_link.into_iter().filter(|i| !(i == "lib/pkgconfig" || i == "lib/cmake" || i == "lib/dtrace")).collect();
  to_link.extend(list_dir(opt_path, "include", Strategy::Child)?);
  Ok(to_link)
}

#[tracing::instrument(level = "debug", skip(prefix, opt_path, tracker), fields(prefix = %prefix.as_ref().display(), opt_path = %opt_path.as_ref().display()))]
pub async fn step<P: AsRef<Path>, Q: AsRef<Path>>(prefix: P, opt_path: Q, tracker: impl EventListener<ItemEvent>) -> Result<()> {
  // TODO: generate list of link and check conflict before unpack
  let opt_path = opt_path.as_ref();
  let prefix = prefix.as_ref().canonicalize().when(("step.prefix", &prefix.as_ref()))?;
  let to_link = collect_link_targets(opt_path)?;
  tracker.on_event(ItemEvent::Init { max: to_link.len() });
  for (i, file) in to_link.into_iter().enumerate() {
    info!(file, "linking");
    let src = opt_path.join(&file).canonicalize().when(("step.src", &opt_path.join(&file)))?;
    let dest = prefix.join(&file);
    if let Some(parent) = dest.parent() {
      if !parent.exists() {
        std::fs::create_dir_all(parent).when(("step.create_dir_all", &parent))?;
      }
    }
    if src.is_dir() {
      symlink_dir(&src, dest, true).when(("step.symlink_dir", &src))?
    } else {
      symlink_file(&src, dest, true).when(("step.symlink_file", &src))?
    }
    tracker.on_event(ItemEvent::Progress { current: i, max: None });
  }
  Ok(())
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
    let mut files = vec![format!("opt/{}", pkg.name)];
    files.extend(collect_link_targets(&pkg.dest)?);
    symlink_dir(&pkg.dest, opt_dir.join(&pkg.name), true).ok();
    tracker.on_event(ItemEvent::Progress { current: i, max: None });
    step(prefix, &pkg.dest, ()).await?;
    result.push(PackageLinked {
      name: pkg.name.clone(),
      dest: pkg.dest.clone(),
      version: pkg.version.clone(),
      files,
    })
  }
  tracker.on_event(ItemEvent::Message { name: "link finished".to_string() });
  tracker.on_event(ItemEvent::Finish);
  Ok(result)
}

pub fn guess_installed(path: &Path) -> Option<PackageInstalled> {
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

pub fn list_installed(cellar_dir: &Path) -> Result<Vec<PackageInstalled>> {
  if !cellar_dir.exists() {
    return Ok(Vec::new());
  }

  let mut pkgs = Vec::new();
  for entry in std::fs::read_dir(cellar_dir).when(("list_installed.read_dir", cellar_dir))? {
    let entry = entry.when(("list_installed.read_dir_entry", cellar_dir))?;
    let path = entry.path();
    if !entry.file_type().when(("list_installed.file_type", &path))?.is_dir() {
      continue;
    }
    if let Some(pkg) = guess_installed(&path) {
      pkgs.push(pkg);
    }
  }

  pkgs.sort_by(|left, right| left.name.cmp(&right.name));
  Ok(pkgs)
}

#[tokio::test]
async fn test_link() {
  use crate::tests::*;
  init_logger(None);

  let cellar_dir = Path::new(CELLAR_PATH);
  let pkgs = list_installed(cellar_dir).unwrap();

  let result = exec(PREFIX_PATH, &pkgs, ()).await.unwrap();
  assert_eq!(result.len(), pkgs.len())
}
