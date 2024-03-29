///! mach_o_files:
///!   `pn.dylib? || pn.mach_o_bundle? || pn.mach_o_executable?`
///! fix_dynamic_linkage:
///!   fix absolute link starts with HOMEBREW_CELLAR (/opt/homebrew) and HOMEBREW_PREFIX to relative link
///! relocate_dynamic_linkage:
///!   for LC_ID_DYLIB change_dylib_id, (only for dylib)
///!   for LC_LOAD_DYLIB, LC_LOAD_WEAK_DYLIB, LC_REEXPORT_DYLIB, LC_LOAD_UPWARD_DYLIB,
///!     LC_LAZY_LOAD_DYLIB, LC_PREBOUND_DYLIB change_install_name,
///!   and for LC_RPATH change rpath
///! replace_text_in_files
///! when HOMEBREW_RELOCATE_BUILD_PREFIX is set, `relocate_build_prefix` would be additionally called.
///!
///! otool -l cache/a.out | grep -B3 "@@"
///! install_name_tool -id <new_id> -change <old_lib> <new_lib> -rpath <old_path> <new_path> cache/a.out
///! codesign --sign - --force --preserve-metadata=entitlements,requirements,flags,runtime cache/a.out
///!
///! see also:
///!   https://github.com/Homebrew/brew/blob/master/Library/Homebrew/keg_relocate.rb
///!   https://github.com/Homebrew/brew/blob/master/Library/Homebrew/extend/os/mac/keg_relocate.rb
///!   https://opensource.apple.com/source/cctools/cctools-795/misc/install_name_tool.c.auto.html

use std::{borrow::Cow, collections::BTreeMap, path::{Path, PathBuf}};

use goblin::mach::MachO;
use memmap2::MmapOptions;

use crate::error::{ErrorExt, Result};

/// We found these replacement in homebrew
/// TODO: link here
/// @@HOMEBREW_PREFIX@@ => ${prefix}/
/// @@HOMEBREW_CELLAR@@ => ${prefix}/Cellar
/// @@HOMEBREW_PERL@@ => /usr/bin/perl
/// @@HOMEBREW_JAVA@@ => ${prefix}/opt/openjdk/libexec
/// and would like to read prefix and cellar folder from config
pub struct RelocationPattern {
  pub install_name: BTreeMap<String, String>,
  pub extra_name: BTreeMap<String, String>,
}

impl RelocationPattern {
  pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(prefix: P1, cellar: P2) -> Self {
    // TODO: unwrap 1. current dir exists, 2. utf8 system
    let prefix = try_abs_path(&prefix).unwrap().to_str().unwrap().to_string();
    let cellar = try_abs_path(&cellar).unwrap().to_str().unwrap().to_string();
    let mut install_name = BTreeMap::new();
    install_name.insert("@@HOMEBREW_PREFIX@@".to_string(), prefix.clone());
    install_name.insert("@@HOMEBREW_CELLAR@@".to_string(), cellar.clone());
    let mut extra_name = BTreeMap::new();
    extra_name.insert("@@HOMEBREW_PERL@@".to_string(), "/usr/bin/perl".to_string());
    extra_name.insert("@@HOMEBREW_JAVA@@".to_string(), format!("{}/opt/openjdk/libexec", prefix));
    Self {
      install_name, extra_name,
    }
  }

  pub fn replace_dylib<'a>(&self, name: &'a str) -> Cow<'a, str> {
    for (i, v) in &self.install_name {
      if name.starts_with(i) {
        return Cow::Owned(name.replacen(i, v, 1));
      }
    }
    Cow::Borrowed(name)
  }

  pub fn replace_text<'a>(&self, s: &'a str) -> Cow<'a, str> {
    let mut t = Cow::Borrowed(s);
    // TODO: scan str only once
    for (i, v) in self.install_name.iter().chain(&self.extra_name) {
      t = Cow::Owned(t.replace(i, v));
    }
    if t == s {
      Cow::Borrowed(s)
    } else {
      t
    }
  }
}


pub fn try_abs_path<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
  let path = path.as_ref();
  let path = match path.canonicalize() {
    Ok(path) => path,
    Err(_) => if path.is_absolute() {
      path.to_path_buf()
    } else {
      match Path::new(".").canonicalize() {
        Ok(cur) => cur.join(path),
        Err(_) => unreachable!("current dir does not exist"),
      }
    }
  };
  Some(path_clean::clean(path))
}

pub fn with_permission<P: AsRef<Path>, F: FnOnce() -> R, R>(filename: P, f: F) -> std::io::Result<R> {
  let permission = std::fs::metadata(filename.as_ref())?.permissions();
  let mut new_permission = permission.clone();
  new_permission.set_readonly(false);
  let result;
  if permission.readonly() {
    std::fs::set_permissions(filename.as_ref(), new_permission)?;
    result = f();
    std::fs::set_permissions(filename.as_ref(), permission)?;
  } else {
    result = f();
  }
  Ok(result)
}

#[derive(Default, Clone, Debug)]
pub struct Relocations {
  pub id: (String, String),
  pub links: BTreeMap<String, String>,
  pub rpaths: BTreeMap<String, String>,
}

impl Relocations {
  pub fn from_macho(file: &MachO, pattern: &RelocationPattern) -> Result<Self> {
    let mut result = Self::default();
    if let Some(name) = file.name {
      if let Cow::Owned(new_name) = pattern.replace_dylib(name) {
        // TODO: check size
        // let new_len = new_name.len() as u32 / 8 + 1;
        // new_sizeofcmds = new_sizeofcmds + new_len - name.len() as u32;
        result.id = (name.to_string(), new_name);
      }
    }
    for &name in &file.libs {
      // TODO check if these command all processed
      // LoadCommand::LoadDyLib(dylib) | LoadCommand::LoadWeakDyLib(dylib) |
      // LoadCommand::ReexportDyLib(dylib) | LoadCommand::LoadUpwardDylib(dylib) |
      // LoadCommand::LazyLoadDylib(dylib) => {
      if let Cow::Owned(new_name) = pattern.replace_dylib(name) {
        result.links.insert(name.to_string(), new_name);
      }
    }
    for &name in &file.rpaths {
      if let Cow::Owned(new_name) = pattern.replace_dylib(name) {
        result.rpaths.insert(name.to_string(), new_name);
      }
    }
    // TODO: check if processed OFile::FatFile { files, .. } | OFile::ArFile { files }
    Ok(result)
  }

  pub fn is_empty(&self) -> bool {
    return self.id.0.is_empty() && self.links.is_empty() && self.rpaths.is_empty()
  }

  pub fn apply_file<P: AsRef<Path>>(&self, filename: P) -> Result<(), std::io::Error> {
    if self.is_empty() {
      return Ok(())
    }
    trace!(filename=%filename.as_ref().display(), "patch macho file");

    let mut cmd = &mut std::process::Command::new("install_name_tool");
    if self.id.0 != "" {
      cmd = cmd.args(["-id", &self.id.1]);
    }
    for (i, v) in &self.links {
      cmd = cmd.args(["-change", i, v])
    }
    for (i, v) in &self.rpaths {
      cmd = cmd.args(["-rpath", i, v])
    }
    cmd = cmd.arg(filename.as_ref());
    info!(?cmd, "install_name_tool");
    let result = cmd.output()?;
    for i in String::from_utf8_lossy(&result.stderr).lines() {
      if i.is_empty() || i.contains("invalidate the code signature") {
        continue;
      }
      warn!("stderr: {}", i);
    }
    if !result.status.success() {
      return Err(std::io::Error::other("install_name_tool"));
    }

    let result = &mut std::process::Command::new("codesign")
        .args(["--sign", "-", "--force", "--preserve-metadata=entitlements,requirements,flags,runtime"])
        .arg(filename.as_ref()).output()?;
    if !result.status.success() {
      return Err(std::io::Error::other("codesign"));
    }
    for i in String::from_utf8_lossy(&result.stderr).lines() {
      if i.is_empty() || i.contains("replacing existing signature") {
        continue;
      }
      warn!("stderr: {}", i);
    }
    Ok(())
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RelocateType {
  MachO, Text, None
}

pub fn relocate<P: AsRef<Path>>(filename: P, pattern: &RelocationPattern) -> Result<RelocateType> {
  let filename = filename.as_ref();
  if !filename.exists() || filename.is_symlink() {
    return Ok(RelocateType::None);
  }
  let file = std::fs::File::open(filename).when(("open", filename))?;
  let mmap = unsafe { MmapOptions::new().map(&file) }.when(("memmap", filename))?;
  if let Ok(macho) = MachO::parse(&mmap, 0) {
    let reloc = Relocations::from_macho(&macho, &pattern)?;
    if !reloc.is_empty() {
      debug!(filename=%filename.display(), "reloc macho");
      reloc.apply_file(filename).when(("otool", filename))?;
      return Ok(RelocateType::MachO);
    }
  } else if let Ok(text) = std::fs::read_to_string(&filename) {
    if let Cow::Owned(text) = pattern.replace_text(&text) {
      debug!(filename=%filename.display(), "reloc text");
      with_permission(&filename, ||
        std::fs::write(filename, text)
      ).when(("write", &filename))?.when(("permission", &filename))?;
      return Ok(RelocateType::Text)
    }
  }
  return Ok(RelocateType::None)
}

#[test]
fn test_relocate() {
  use crate::tests::*;
  init_logger(None);
  let tmp_dir = "cache/reloc/";
  let pattern = RelocationPattern::new("cache", "cache/Cellar");
  std::fs::create_dir_all(tmp_dir).ok();
  for file in walkdir::WalkDir::new("cache/root/opt") {
    let file = file.unwrap();
    if file.file_type().is_file() {
      let filename = Path::new(tmp_dir).join(file.file_name());
      std::fs::copy(file.path(), &filename).ok();
      let result = relocate(&filename, &pattern).unwrap();
      if result != RelocateType::None {
        info!(?result, filename=%file.path().display());
      }
      std::fs::remove_file(&filename).ok();
    }
  }
}
