/// mach_o_files:
///   `pn.dylib? || pn.mach_o_bundle? || pn.mach_o_executable?`
/// fix_dynamic_linkage:
///   fix absolute link starts with HOMEBREW_CELLAR (/opt/homebrew) and HOMEBREW_PREFIX to relative link
/// relocate_dynamic_linkage:
///   for LC_ID_DYLIB change_dylib_id, (only for dylib)
///   for LC_LOAD_DYLIB, LC_LOAD_WEAK_DYLIB, LC_REEXPORT_DYLIB, LC_LOAD_UPWARD_DYLIB,
///     LC_LAZY_LOAD_DYLIB, LC_PREBOUND_DYLIB change_install_name,
///   and for LC_RPATH change rpath
/// replace_text_in_files
/// when HOMEBREW_RELOCATE_BUILD_PREFIX is set, `relocate_build_prefix` would be additionally called.
///
/// otool -l cache/a.out | grep -B3 "@@"
/// install_name_tool -id <new_id> -change <old_lib> <new_lib> -rpath <old_path> <new_path> cache/a.out
/// codesign --sign - --force --preserve-metadata=entitlements,requirements,flags,runtime cache/a.out
///
/// see also:
///   https://github.com/Homebrew/brew/blob/master/Library/Homebrew/keg_relocate.rb
///   https://github.com/Homebrew/brew/blob/master/Library/Homebrew/extend/os/mac/keg_relocate.rb
///   https://opensource.apple.com/source/cctools/cctools-795/misc/install_name_tool.c.auto.html

use std::{collections::BTreeMap, io::Cursor, path::Path, borrow::Cow};

use mach_object::{OFile, MachCommand, LoadCommand};

use crate::config::Config;

pub struct RelocationPattern {
  pub install_name: BTreeMap<String, String>,
  pub extra_name: BTreeMap<String, String>,
}

impl RelocationPattern {
  pub fn abs_path(path: &str) -> anyhow::Result<String> {
    match std::path::Path::new(path).canonicalize()?.to_str() {
      Some(s) => Ok(s.to_string()),
      None => anyhow::bail!("utf8 error: {:?}", path),
    }
  }
  pub fn new(config: &Config) -> anyhow::Result<Self> {
    let mut install_name = BTreeMap::new();
    install_name.insert("@@HOMEBREW_PREFIX@@".to_string(), Self::abs_path(&config.root_dir)?);
    install_name.insert("@@HOMEBREW_CELLAR@@".to_string(), Self::abs_path(&config.cellar_dir)?);
    let mut extra_name = BTreeMap::new();
    extra_name.insert("@@HOMEBREW_PERL@@".to_string(), "/usr/bin/perl".to_string());
    extra_name.insert("@@HOMEBREW_JAVA@@".to_string(), format!("{}/opt/openjdk/libexec", Self::abs_path(&config.root_dir)?));
    Ok(Self {
      install_name, extra_name,
    })
  }

  pub fn replace_dylib(&self, name: &str) -> Option<String> {
    for (i, v) in &self.install_name {
      if name.starts_with(i) {
        return Some(name.replacen(i, v, 1));
      }
    }
    None
  }

  pub fn replace_text(&self, s: &str) -> Option<String> {
    let mut t = Cow::Borrowed(s);
    for (i, v) in self.install_name.iter().chain(&self.extra_name) {
      t = Cow::Owned(t.replace(i, v));
    }
    if t == s {
      None
    } else {
      Some(t.into_owned())
    }
  }
}

#[derive(Default, Clone, Debug)]
pub struct Relocations {
  pub id: (String, String),
  pub links: BTreeMap<String, String>,
  pub rpaths: BTreeMap<String, String>,
}

pub fn try_open_ofile<P: AsRef<Path>>(filename: P) -> anyhow::Result<OFile> {
  let file = std::fs::File::open(filename)?;
  let mmap = unsafe { memmap::MmapOptions::new().map(&file)? };
  let ofile = OFile::parse(&mut Cursor::new(mmap))?;
  Ok(ofile)
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

impl Relocations {
  pub fn parse_file<P: AsRef<Path>>(filename: P, pattern: &RelocationPattern) -> anyhow::Result<Self> {
    let ofile = try_open_ofile(filename)?;
    Self::from_ofile(&ofile, pattern)
  }

  pub fn from_ofile(file: &OFile, pattern: &RelocationPattern) -> anyhow::Result<Self> {
    match file {
      OFile::MachFile { header, commands } => {
        let mut result = Self::default();
        let mut low_fileoff = u32::MAX;
        let mut new_sizeofcmds = header.sizeofcmds;
        for MachCommand(ref cmd, cmdsize) in commands {
          match cmd {
            LoadCommand::Segment64 { segname, sections, .. } |
            LoadCommand::Segment { segname, sections, .. } => {
              trace!("segment: {} ({})", segname, cmdsize);

              for sect in sections {
                trace!("  section: {}", sect.sectname);
                low_fileoff = std::cmp::min(sect.offset, low_fileoff);
              }
            }
            // TODO: prebound_dylib not supported

            LoadCommand::IdDyLib(dylib) => {
              let name = dylib.name.as_str();
              if let Some(new_name) = pattern.replace_dylib(name) {
                let new_len = new_name.len() as u32 / 8 + 1;
                new_sizeofcmds = new_sizeofcmds + new_len - name.len() as u32;
                result.id = (name.to_string(), new_name);
              }
            }

            LoadCommand::LoadDyLib(dylib) | LoadCommand::LoadWeakDyLib(dylib) |
            LoadCommand::ReexportDyLib(dylib) | LoadCommand::LoadUpwardDylib(dylib) |
            LoadCommand::LazyLoadDylib(dylib) => {
              let name = dylib.name.as_str();
              if let Some(new_name) = pattern.replace_dylib(name) {
                let new_len = new_name.len() as u32 / 8 + 1;
                new_sizeofcmds = new_sizeofcmds + new_len - name.len() as u32;
                result.links.insert(name.to_string(), new_name);
              }
            }

            LoadCommand::Rpath(name) => {
              if let Some(new_name) = pattern.replace_dylib(name) {
                let new_len = new_name.len() as u32 / 8 + 1;
                new_sizeofcmds = new_sizeofcmds + new_len - name.len() as u32;
                result.rpaths.insert(name.to_string(), new_name);
              }
            }
            _ => {
              trace!("cmd: {:?} ({})", cmd, cmdsize);
            }
          }
        }
        Ok(result)
      },
      OFile::FatFile { files, .. } => {
        let mut result = Self::default();
        for (_, o) in files {
          let r = Self::from_ofile(o, pattern)?;
          if result.id.0 == "" {
            result.id = r.id;
          } else if r.id.0 != "" {
            anyhow::ensure!(result.id == r.id);
          }
          result.links.extend(r.links);
          result.rpaths.extend(r.rpaths);
        }
        Ok(result)
      },
      OFile::ArFile { files } => {
        let mut result = Self::default();
        for (_, o) in files {
          let r = Self::from_ofile(o, pattern)?;
          if result.id.0 == "" {
            result.id = r.id;
          } else if r.id.0 != "" {
            anyhow::ensure!(result.id == r.id);
          }
          result.links.extend(r.links);
          result.rpaths.extend(r.rpaths);
        }
        Ok(result)
      },
      OFile::SymDef { .. } => Ok(Self::default()),
    }
  }

  pub fn is_empty(&self) -> bool {
    return self.id.0.is_empty() && self.links.is_empty() && self.rpaths.is_empty()
  }

  pub fn apply_file<P: AsRef<Path>>(&self, filename: P) -> anyhow::Result<()> {
    if self.is_empty() {
      return Ok(())
    }
    trace!("patch macho file {}", filename.as_ref().to_string_lossy());

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
    let result = cmd.arg(filename.as_ref()).output()?;
    for i in String::from_utf8_lossy(&result.stderr).lines() {
      if i.is_empty() || i.contains("invalidate the code signature") {
        continue;
      }
      warn!("stderr: {}", i);
    }
    anyhow::ensure!(result.status.code() == Some(0));

    let result = &mut std::process::Command::new("codesign")
        .args(["--sign", "-", "--force", "--preserve-metadata=entitlements,requirements,flags,runtime"])
        .arg(filename.as_ref()).output()?;
    anyhow::ensure!(result.status.code() == Some(0));
    for i in String::from_utf8_lossy(&result.stderr).lines() {
      if i.is_empty() || i.contains("replacing existing signature") {
        continue;
      }
      warn!("stderr: {}", i);
    }
    Ok(())
  }
}

#[test]
fn test_load() {
  // let filename = "target/debug/pacbrew";
  let filename = "cache/a.out";
  std::fs::remove_file(filename).ok();
  std::fs::copy("cache/root/bin/ffmpeg", filename).expect("copy");
  let mut install_name = BTreeMap::new();
  install_name.insert("@@HOMEBREW_PREFIX@@".to_string(), "/opt/homebrew".to_string());
  install_name.insert("@@HOMEBREW_CELLAR@@".to_string(), "/opt/homebrew/Cellar".to_string());
  let pattern = RelocationPattern { install_name, extra_name: Default::default() };
  let relocations = Relocations::parse_file(filename, &pattern).expect("parse file");
  println!("install_names: {:?}", relocations);
  relocations.apply_file(filename).expect("apply");
  let relocations = Relocations::parse_file(filename, &pattern).expect("parse file");
  assert!(relocations.id.0 == "");
  assert!(relocations.links.is_empty());
  assert!(relocations.rpaths.is_empty());
}
