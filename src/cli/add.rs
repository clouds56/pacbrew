use std::{collections::{VecDeque, BTreeMap}, sync::Arc, path::{PathBuf, Path}, cell};

use clap::Parser;
use crate::{io::{progress::{create_pb, create_pbb}, fetch::{github_client, basic_client, check_sha256}, package::{PackageArchive, self}}, relocation::{try_open_ofile, Relocations, RelocationPattern, with_permission}};
use crate::config::PacTree;
use super::{PackageInfo, PackageInfos, PackageMeta, save_package_info, RelocateMode};

#[derive(Parser)]
pub struct Opts {
  #[clap(short, long)]
  skip_unpack: bool,
  names: Vec<String>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
  #[error("resolve: package {0:?} not found")]
  Resolve(PackageInfo), // TODO: dependency path
  #[error("prebuilt")]
  Prebuilt(PackageInfo),
  #[error("resolve-net")]
  ResolveNet(PackageInfo, #[source] Arc<reqwest::Error>),
  #[error("download: package {0:?} failed")]
  Download(PackageInfo, #[source] Arc<anyhow::Error>),
  #[error("io: {0:?}")]
  Io(PathBuf, #[source] Arc<std::io::Error>),
  #[error("broken package {0:?}")]
  Package(PackageInfo, #[source] Arc<package::Error>),
  #[error("package info {0:?}")]
  PackageInfo(PackageInfo, #[source] Arc<anyhow::Error>),
  #[error("package ruby {0:?}")]
  PackageRuby(PackageInfo, #[source] Arc<anyhow::Error>),
  #[error("package relocation: package {0:?}, file: {1}")]
  PackageRelocation(PackageInfo, String, #[source] Arc<anyhow::Error>),
  #[error("post install: package {0:?}")]
  PostInstall(PackageInfo, #[source] Arc<std::io::Error>),
  #[error("unimplemented: package {0:?} not implement {1}")]
  Unimplemented(PackageInfo, String, #[source] Arc<anyhow::Error>),
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

/// stage1: collect dependencies
/// TODO: sort in topological order
pub fn resolve(names: &[String], env: &PacTree) -> Result<PackageInfos> {
  let mut result = PackageInfos::new();
  let mut names = names.iter().map(|i| PackageInfo::new(i.to_string())).collect::<VecDeque<_>>();
  while let Some(p) = names.pop_front() {
    if result.contains_key(&p.name) {
      continue
    }

    let package = match env.get_package(&p.name) {
      Some(t) => t,
      None => {
        error!("cannot found {}", &p.name);
        return Err(Error::Resolve(p))
      }
    };
    // TODO: channel
    let version = package.versions.stable.clone();
    // TODO: check requirements
    debug!("resolving {}:{} => {:?}", package.name, version, package.dependencies);
    let p = p.with_name(package.full_name.to_string(), version, package.revision);
    names.extend(p.with_dependencies(&package.dependencies));
    result.insert(p.name.to_string(), p);
  }
  Ok(result)
}

pub fn resolve_url(infos: &mut PackageInfos, env: &PacTree) -> Result<BTreeMap<String, String>> {
  let pb = create_pb("Resolve url", infos.len());
  let mut result = BTreeMap::new();
  for p in infos.values_mut() {
    pb.set_message(format!("for {}", p.name));
    let package = match env.get_package(&p.name) {
      Some(t) => t,
      None => {
        error!(@pb => "cannot found {}", &p.name);
        return Err(Error::Resolve(p.clone()))
      }
    };
    let bottles = match package.bottle.get("stable") {
      Some(bottles) => bottles,
      None => {
        error!(@pb => "channel stable not exists {}", &p.name);
        return Err(Error::Prebuilt(p.clone()));
      }
    };
    let mut bottle = None;
    for arch in vec![env.config.target.as_str(), "all"].into_iter().chain(env.config.os_fallback.iter().map(|i| i.as_str())) {
      if let Some(b) = bottles.files.get(arch) {
        p.arch = arch.to_string();
        bottle = Some(b);
        break;
      }
    }
    let bottle = match bottle {
      Some(bottle) => bottle,
      None => {
        error!(@pb => "target {} not found in {:?} for {}", env.config.target, bottles.files.keys(), p.name);
        return Err(Error::Prebuilt(p.clone()));
      }
    };
    // TODO: mirrors
    p.rebuild = bottles.rebuild;
    p.relocate = bottle.cellar.to_string().try_into().map_err(|e: String| Error::Unimplemented(p.clone(), "relocate".to_string(), Arc::new(anyhow::anyhow!("{}", e))))?;
    p.sha256 = bottle.sha256.clone();
    if let Some(mirror) = env.config.mirror_list.first() {
      if mirror.oci {
        p.url = format!("{}/{}/blobs/sha256:{}", mirror.url, p.name.replace("@", "/"), p.sha256)
      } else {
        let rebuild = if p.rebuild != 0 { format!(".{}", p.rebuild)} else { "".to_string() };
        p.url = format!("{}/{}-{}.{}.bottle{}.tar.gz", mirror.url, p.name, p.version_full, p.arch, rebuild)
      }
    } else {
      p.url = bottle.url.clone();
    }
    debug!(@pb => "url of {} ({:?}, {}) => {}", p.name, p.relocate, p.sha256, p.url);
    result.insert(p.name.clone(), p.url.clone());
    pb.inc(1);
  }
  pb.finish_with_message("");
  Ok(result)
}

#[tokio::main]
pub async fn resolve_size(infos: &mut PackageInfos, env: &PacTree) -> Result<BTreeMap<String, u64>> {
  let pb = create_pb("Resolve size", infos.len());
  let mut result = BTreeMap::new();
  let cache_dir = Path::new(&env.config.cache_dir).join("pkg");
  // TODO: true concurrent
  for p in infos.values_mut() {
    pb.set_message(format!("for {}", p.name));
    // TODO: mirrors
    p.package_name = format!("{}-{}.{}.bottle.tar.gz", p.name, p.version_full, p.arch);
    if cache_dir.join(&p.package_name).exists() {
      pb.set_length(pb.length().expect("length") - 1);
      // TODO load package size
      continue
    }
    let client = if p.url.contains("//ghcr.io/") { github_client() } else { basic_client() };
    let resp = client.head(&p.url).send().await.map_err(|e| Error::ResolveNet(p.clone(), Arc::new(e)))?;
    if resp.status().is_success() {
      // TODO: handle error
      // let size = resp.content_length().unwrap_or_default(); <-- this is broken, always return 0
      let size = resp.headers().get("content-length")
          .and_then(|i| i.to_str().ok())
          .and_then(|i| i.parse::<u64>().ok())
          .unwrap_or_default();
      result.insert(p.name.to_string(), size);
      p.size = size;
      // TODO check partial
      p.download_size = size;
      debug!(@pb => "head {} => {}", &p.url, size);
    } else {
      warn!(@pb => "{} => {} {:?}", &p.url, resp.status(), resp.headers());
    }
    pb.inc(1);
  }
  pb.finish_with_message("");
  Ok(result)
}

#[tokio::main]
pub async fn download_packages(infos: &mut PackageInfos, env: &PacTree) -> Result<BTreeMap<String, PathBuf>> {
  use crate::io::fetch::Task;
  let mut result = BTreeMap::new();
  let cache_dir = Path::new(&env.config.cache_dir).join("pkg");
  std::fs::create_dir_all(&cache_dir).map_err(|e| Error::Io(cache_dir.to_path_buf(), Arc::new(e)))?;
  // TODO show global progress bar
  for p in infos.values_mut() {
    let package_path = cache_dir.join(&p.package_name);
    // TODO: reuse client
    let client = if p.url.contains("//ghcr.io/") { github_client() } else { basic_client() };
    let mut task = Task::new(client, &p.url, &package_path, None, p.sha256.clone());
    if !package_path.exists() {
      let pb = create_pbb("Download", 0);
      pb.set_message(p.name.clone());
      if let Err(e) = task.set_progress(pb.clone()).run().await {
        warn!(@pb => "download {} from {} failed: {:?}", p.name, p.url, e);
      }
      pb.finish();
    }
    p.pacakge_path = package_path.clone();
    result.insert(p.name.clone(), package_path);
  }
  Ok(result)
}

pub fn check_packages(infos: &PackageInfos, _env: &PacTree) -> Result<BTreeMap<String, PackageMeta>> {
  let mut result = BTreeMap::new();
  let pb = create_pb("Check package", infos.len());
  // TODO: true concurrent
  for p in infos.values() {
    pb.set_message(format!("{}", p.name));

    check_sha256(&p.pacakge_path, &p.sha256).map_err(|e| Error::Download(p.clone(), Arc::new(e)))?;

    // check all files in subfolder "{p.name}/{p.version_full}"
    // https://rust-lang-nursery.github.io/rust-cookbook/compression/tar.html
    let mut meta = PackageMeta::new(format!("{}/{}", p.name, p.version_full));
    let archive = PackageArchive::open(&p.pacakge_path).map_err(|e| Error::Package(p.clone(), Arc::new(e)))?;
    let entries = archive.entries().map_err(|e| Error::Package(p.clone(), Arc::new(e)))?;
    let mut found_brew_rb = false;
    let brew_rb_file = p.brew_rb_file();
    for entry in &entries {
      if !entry.starts_with(&meta.keg) {
        error!(@pb => "package {} contains file", entry);
      }
      if entry == &brew_rb_file {
        found_brew_rb = true;
      }
    }
    if !found_brew_rb {
      warn!(@pb => "package {} doesn't contains brew {} file", p.name, brew_rb_file)
    }
    meta.files = entries;
    if p.reason.is_empty() {
      meta.explicit = true;
    } else {
      meta.required.push(p.reason.last().cloned().expect("last"));
    }
    result.insert(p.name.clone(), meta);
    pb.inc(1);
  }
  pb.finish_with_message("");
  Ok(result)
}

pub fn unpack_packages(infos: &PackageInfos, meta: &BTreeMap<String, PackageMeta>, env: &PacTree) -> Result<()> {
  let meta_local_dir = Path::new(&env.config.meta_dir).join("local");
  std::fs::create_dir_all(&meta_local_dir).map_err(|e| Error::Io(meta_local_dir.to_path_buf(), Arc::new(e)))?;
  for p in infos.values() {
    let m = meta.get(&p.name).expect("meta not present");
    let dst = Path::new(&env.config.cellar_dir).join(&m.keg);
    std::fs::create_dir_all(&dst).map_err(|e| Error::Io(dst.to_path_buf(), Arc::new(e)))?;
    let archive = PackageArchive::open(&p.pacakge_path).map_err(|e| Error::Package(p.clone(), Arc::new(e)))?;
    let pb = create_pbb(&format!("Install {}", p.name), archive.size().unwrap_or_default());
    archive.unpack_with_pb(&pb, &m.keg, &env.config.cellar_dir).map_err(|e| Error::Package(p.clone(), Arc::new(e)))?;
    let meta_path = meta_local_dir.join(&p.name).join("current");
    save_package_info(meta_path, p, m).map_err(|e| Error::PackageInfo(p.clone(), Arc::new(e)))?;
    pb.finish();
  }
  Ok(())
}

pub fn relocate_packages(infos: &PackageInfos, meta: &mut BTreeMap<String, PackageMeta>, env: &PacTree) -> Result<()> {
  let mut len = infos.len();
  let meta_local_dir = Path::new(&env.config.meta_dir).join("local");
  std::fs::create_dir_all(&meta_local_dir).map_err(|e| Error::Io(meta_local_dir.to_path_buf(), Arc::new(e)))?;
  let relocation_pattern = RelocationPattern::new(&env.config).expect("path cannot resolved");
  let pb = create_pb("Relocate package", infos.len());
  for p in infos.values() {
    if p.relocate == RelocateMode::Skip {
      len -= 1;
      pb.set_length(len as u64);
      continue;
    }
    let m = meta.get_mut(&p.name).expect("meta not present");
    let mut patched_binaries = Vec::new();
    let mut patched_text = Vec::new();
    let dst = Path::new(&env.config.cellar_dir);
    for f in &m.files {
      let filename =  dst.join(f);
      if !filename.exists() {
        warn!(@pb => "reloc cannot open file {}", filename.to_string_lossy());
      } else if filename.is_symlink() {
        continue;
      } if let Ok(ofile) = try_open_ofile(&filename) {
        let reloc = Relocations::from_ofile(&ofile, &relocation_pattern).map_err(|e| Error::PackageRelocation(p.clone(), f.clone(), Arc::new(e)))?;
        if !reloc.is_empty() {
          debug!(@pb => "reloc bin {}", filename.to_string_lossy());
          reloc.apply_file(filename).map_err(|e| Error::PackageRelocation(p.clone(), f.clone(), Arc::new(e)))?;
          patched_binaries.push(f.clone());
        }
      } else if let Ok(text) = std::fs::read_to_string(&filename) {
        if let Some(text) = relocation_pattern.replace_text(&text) {
          debug!(@pb => "reloc text {}", filename.to_string_lossy());
          with_permission(filename.as_path(), ||
            std::fs::write(filename.as_path(), text)
          ).map_err(|e| Error::Io(filename.to_path_buf(), Arc::new(e)))?
          .map_err(|e| Error::PackageRelocation(p.clone(), f.clone(), Arc::new(e.into())))?;
          patched_text.push(f.clone());
        }
      }
    }
    m.patched_binaries = patched_binaries;
    m.patched_text = patched_text;
    let meta_path = meta_local_dir.join(&p.name).join("current");
    save_package_info(meta_path, p, m).map_err(|e| Error::PackageInfo(p.clone(), Arc::new(e)))?;
    pb.inc(1);
  }
  Ok(())
}

fn list_dir<P: AsRef<Path>>(base: P, folder: &str) -> Result<Vec<String>> {
  let path = base.as_ref().join(folder);
  let mut result = Vec::new();
  if path.exists() {
    for f in std::fs::read_dir(&path).map_err(|e| Error::Io(path, Arc::new(e)))? {
      if let Ok(f) = f {
        result.push(format!("{}/{}", folder, f.file_name().to_string_lossy()));
      }
    }
  }
  Ok(result)
}

pub fn link_packages(infos: &PackageInfos, meta: &mut BTreeMap<String, PackageMeta>, env: &PacTree) -> Result<()> {
  let meta_local_dir = Path::new(&env.config.meta_dir).join("local");
  std::fs::create_dir_all(&meta_local_dir).map_err(|e| Error::Io(meta_local_dir.to_path_buf(), Arc::new(e)))?;
  std::fs::create_dir_all(Path::new(&env.config.root_dir).join("opt")).map_err(|e| Error::Io(Path::new(&env.config.root_dir).join("opt"), Arc::new(e)))?;
  let pb = create_pb("Link package", infos.len());
  // TODO: true concurrent
  for p in infos.values() {
    pb.set_message(format!("{}", p.name));
    let m = meta.get_mut(&p.name).expect("meta not present");
    let cellar_path = Path::new(&env.config.cellar_dir).join(&m.keg);
    let cellar_abs_path = cellar_path.canonicalize().map_err(|e| Error::Io(cellar_path.to_path_buf(), Arc::new(e)))?;
    let brew_rb_path = Path::new(&env.config.cellar_dir).join(p.brew_rb_file());
    let brew_rb_file = std::fs::read_to_string(&brew_rb_path).map_err(|e| Error::Io(brew_rb_path.to_path_buf(), Arc::new(e)))?;
    let mut link_overwrite = Vec::new();
    let bin_name = p.name.split("@").next().expect("first");
    for folder in ["share", "libexec"] {
      if cellar_path.join(folder).join(&bin_name).exists() {
        link_overwrite.push(format!("{}/{}", folder, bin_name));
      }
    }
    link_overwrite.extend(list_dir(&cellar_path, "bin")?);
    link_overwrite.extend(list_dir(&cellar_path, "lib")?.into_iter().filter(|i| i != "lib/pkgconfig" && i != "lib/cmake"));
    link_overwrite.extend(list_dir(&cellar_path, "lib/pkgconfig")?);
    link_overwrite.extend(list_dir(&cellar_path, "lib/cmake")?);
    link_overwrite.extend(list_dir(&cellar_path, "include")?);
    let mut link_param_str = "[".to_string();
    for line in brew_rb_file.lines() {
      if link_param_str != "[" {
        link_param_str += line.trim();
      } else if line.trim().starts_with("link_overwrite ") {
        link_param_str += line.trim().trim_start_matches("link_overwrite").trim();
      }
      if link_param_str.ends_with(",") {
        continue;
      }
      if link_param_str != "[" {
        let s = link_param_str + "]";
        // debug!("parsing {:?}", s);
        let s = serde_json::from_str::<Vec<String>>(&s).map_err(|e| Error::PackageRuby(p.clone(), Arc::new(e.into())))?;
        link_overwrite.extend(s);
        link_param_str = "[".to_string();
      }
    }
    let mut links = Vec::new();
    for link in &link_overwrite {
      let src = cellar_abs_path.join(link);
      let dst = Path::new(&env.config.root_dir).join(link);
      debug!(@pb => "link package {}: {}", p.name, link);
      if !src.exists() {
        error!(@pb => "file {} not exists", cellar_path.join(link).to_string_lossy());
        // TODO: link blob (like include/hwy/* in highway)
        continue;
      }
      if dst.exists() || std::fs::symlink_metadata(&dst).is_ok() {
        // TODO: force?
        // TODO: remove parent dir
        symlink::remove_symlink_auto(&dst).ok();
      }
      if dst.exists() || std::fs::symlink_metadata(&dst).is_ok() {
        error!(@pb => "file {} already exists", dst.to_string_lossy());
      }
      std::fs::create_dir_all(dst.parent().expect("parent")).map_err(|e| Error::Io(dst.to_path_buf(), Arc::new(e)))?;
      if src.is_dir() {
        std::os::unix::fs::symlink(&src, &dst).map_err(|e| Error::Io(dst.to_path_buf(), Arc::new(e)))?;
        links.push(link.trim_end_matches('/').to_string() + "/");
      } else {
        std::os::unix::fs::symlink(&src, &dst).map_err(|e| Error::Io(dst.to_path_buf(), Arc::new(e)))?;
        links.push(link.to_string());
      }
    }

    let opt_path = Path::new(&env.config.root_dir).join("opt").join(&p.name);
    if opt_path.exists() || std::fs::symlink_metadata(&opt_path).is_ok() {
      symlink::remove_symlink_auto(&opt_path).ok();
    }
    std::os::unix::fs::symlink(&cellar_abs_path, &opt_path).map_err(|e| Error::Io(opt_path.to_path_buf(), Arc::new(e)))?;
    m.links = links;
    let meta_path = meta_local_dir.join(&p.name).join("current");
    save_package_info(meta_path, p, m).map_err(|e| Error::PackageInfo(p.clone(), Arc::new(e)))?;
    pb.inc(1);
  }
  pb.finish_with_message("");
  Ok(())
}

pub fn post_install(infos: &PackageInfos, meta: &BTreeMap<String, PackageMeta>, env: &PacTree) -> Result<()> {
  let mut post_install_packages = Vec::new();
  for p in infos.values() {
    if Path::new(&env.config.scripts_dir).join(format!("{}.sh", p.name)).exists() {
      debug!("found {}.sh post_install", p.name);
      post_install_packages.push(p.name.clone());
    }
  }
  if post_install_packages.len() == 0 {
    return Ok(())
  }
  let pb = create_pb("Post install", post_install_packages.len());
  let root_dir = Path::new(&env.config.root_dir).canonicalize().map_err(|e| Error::Io(Path::new(&env.config.root_dir).to_path_buf(), Arc::new(e)))?.to_string_lossy().to_string();
  let cellar_dir = Path::new(&env.config.cellar_dir).canonicalize().map_err(|e| Error::Io(Path::new(&env.config.cellar_dir).to_path_buf(), Arc::new(e)))?;
  for name in post_install_packages {
    let p = infos.get(&name).expect("info not present");
    let m = meta.get(&p.name).expect("meta not present");
    let output = std::process::Command::new("bash").arg("-c")
      .arg(format!(r#"export PREFIX='{}';export CELLAR='{}';export PKG_NAME={};source '{}' && post_install"#,
        root_dir,
        cellar_dir.join(&m.keg).to_string_lossy(),
        p.name,
        Path::new(&env.config.scripts_dir).join(format!("{}.sh", p.name)).to_string_lossy()))
      .output().map_err(|e| Error::PostInstall(p.clone(), Arc::new(e)))?;
    if !output.stdout.is_empty() {
      println!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
      println!("{}", String::from_utf8_lossy(&output.stderr));
    }
    pb.inc(1);
  }
  pb.finish();
  Ok(())
}


pub fn run(opts: Opts, env: &PacTree) -> Result<()> {
  info!("adding {:?}", opts.names);
  let mut all_packages = resolve(&opts.names, env)?;
  info!("resolved {:?}", all_packages.keys());
  // TODO: fallback url?
  resolve_url(&mut all_packages, env)?;
  resolve_size(&mut all_packages, env)?;
  // TODO: confirm and human readable
  info!("total download {}", all_packages.values().map(|i| i.size).sum::<u64>());
  std::fs::create_dir_all(&env.config.cache_dir).map_err(|e| Error::Io(Path::new(&env.config.cache_dir).to_owned(), Arc::new(e)))?;
  download_packages(&mut all_packages, env)?;
  let mut package_meta = check_packages(&all_packages, env)?;
  if !opts.skip_unpack {
    unpack_packages(&all_packages, &package_meta, env)?;
    relocate_packages(&all_packages, &mut package_meta, env)?;
  }
  link_packages(&all_packages, &mut package_meta, env)?;
  post_install(&all_packages, &mut package_meta, env)?;
  // TODO: post install scripts
  Ok(())
}
