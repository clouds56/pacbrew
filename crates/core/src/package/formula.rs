use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_with::{serde_as, TryFromInto};

// {
//   "name": "postgresql@16",
//   "full_name": "postgresql@16",
//   "tap": "homebrew/core",
//   "oldname": null,
//   "oldnames": [],
//   "aliases": [],
//   "versioned_formulae": [
//     "postgresql@15",
//     "postgresql@14",
//     "postgresql@13",
//     "postgresql@12",
//     "postgresql@11",
//     "postgresql@10"
//   ],
//   "desc": "Object-relational database system",
//   "license": "PostgreSQL",
//   "homepage": "https://www.postgresql.org/",
//   "versions": {
//     "stable": "16.2",
//     "head": null,
//     "bottle": true
//   },
//   "urls": {
//     "stable": {
//       "url": "https://ftp.postgresql.org/pub/source/v16.2/postgresql-16.2.tar.bz2",
//       "tag": null,
//       "revision": null,
//       "using": null,
//       "checksum": "446e88294dbc2c9085ab4b7061a646fa604b4bec03521d5ea671c2e5ad9b2952"
//     }
//   },
//   "revision": 1,
//   "version_scheme": 0,
//   "bottle": {
//     "stable": {
//       "rebuild": 0,
//       "root_url": "https://ghcr.io/v2/homebrew/core",
//       "files": {
//         "arm64_sonoma": {
//           "cellar": "/opt/homebrew/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/16/blobs/sha256:40d2efe6bbcf70078f6fbe80ca8e72eafc16d6003d3903d475856f53514624a1",
//           "sha256": "40d2efe6bbcf70078f6fbe80ca8e72eafc16d6003d3903d475856f53514624a1"
//         },
//         "arm64_ventura": {
//           "cellar": "/opt/homebrew/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/16/blobs/sha256:656a32a5ed4e7f8505f39d1e9e3b2b25ae2f153d7f25d774a87eb2b5fe8371d0",
//           "sha256": "656a32a5ed4e7f8505f39d1e9e3b2b25ae2f153d7f25d774a87eb2b5fe8371d0"
//         },
//         "arm64_monterey": {
//           "cellar": "/opt/homebrew/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/16/blobs/sha256:c9d66495302339354a77a85caae0ab1cd579e4c218f3697c3dde2d438fe2edf4",
//           "sha256": "c9d66495302339354a77a85caae0ab1cd579e4c218f3697c3dde2d438fe2edf4"
//         },
//         "sonoma": {
//           "cellar": "/usr/local/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/16/blobs/sha256:82f8aa9bb1711ab710664f8b90d681c954a4b5b255203fab54db51a65fcd3715",
//           "sha256": "82f8aa9bb1711ab710664f8b90d681c954a4b5b255203fab54db51a65fcd3715"
//         },
//         "ventura": {
//           "cellar": "/usr/local/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/16/blobs/sha256:d9a631c87687f289b61e6066808eb97a38dde76e61d97b51ed9f99fdae9d4538",
//           "sha256": "d9a631c87687f289b61e6066808eb97a38dde76e61d97b51ed9f99fdae9d4538"
//         },
//         "monterey": {
//           "cellar": "/usr/local/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/16/blobs/sha256:d57943164297ec488d94b5cacdbca72dc3f82048734185a127b7f609245f231a",
//           "sha256": "d57943164297ec488d94b5cacdbca72dc3f82048734185a127b7f609245f231a"
//         },
//         "x86_64_linux": {
//           "cellar": "/home/linuxbrew/.linuxbrew/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/16/blobs/sha256:4655a82d8c2e9503f55bb453d71b28e313f81e78908670b19b8e741060ed02f4",
//           "sha256": "4655a82d8c2e9503f55bb453d71b28e313f81e78908670b19b8e741060ed02f4"
//         }
//       }
//     }
//   },
//   "pour_bottle_only_if": null,
//   "keg_only": true,
//   "keg_only_reason": {
//     "reason": ":versioned_formula",
//     "explanation": ""
//   },
//   "options": [],
//   "build_dependencies": [
//     "pkg-config"
//   ],
//   "dependencies": [
//     "gettext",
//     "icu4c",
//     "krb5",
//     "lz4",
//     "openssl@3",
//     "readline",
//     "zstd"
//   ],
//   "test_dependencies": [],
//   "recommended_dependencies": [],
//   "optional_dependencies": [],
//   "uses_from_macos": [
//     "libxml2",
//     "libxslt",
//     "openldap",
//     "perl"
//   ],
//   "uses_from_macos_bounds": [
//     {},
//     {},
//     {},
//     {}
//   ],
//   "requirements": [],
//   "conflicts_with": [],
//   "conflicts_with_reasons": [],
//   "link_overwrite": [],
//   "caveats": "This formula has created a default database cluster with:\n  initdb --locale=C -E UTF-8 $HOMEBREW_PREFIX/var/postgresql@16\nFor more details, read:\n  https://www.postgresql.org/docs/16/app-initdb.html\n",
//   "installed": [],
//   "linked_keg": null,
//   "pinned": false,
//   "outdated": false,
//   "deprecated": false,
//   "deprecation_date": "2028-11-09",
//   "deprecation_reason": null,
//   "disabled": false,
//   "disable_date": null,
//   "disable_reason": null,
//   "post_install_defined": true,
//   "service": {
//     "run": [
//       "$HOMEBREW_PREFIX/opt/postgresql@16/bin/postgres",
//       "-D",
//       "$HOMEBREW_PREFIX/var/postgresql@16"
//     ],
//     "run_type": "immediate",
//     "keep_alive": {
//       "always": true
//     },
//     "environment_variables": {
//       "LC_ALL": "C"
//     },
//     "working_dir": "$HOMEBREW_PREFIX",
//     "log_path": "$HOMEBREW_PREFIX/var/log/postgresql@16.log",
//     "error_log_path": "$HOMEBREW_PREFIX/var/log/postgresql@16.log"
//   },
//   "tap_git_head": "0bbac30cc6051545a4b0d96ddd0289e6a23c4141",
//   "ruby_source_path": "Formula/p/postgresql@16.rb",
//   "ruby_source_checksum": {
//     "sha256": "af43f12f1a9dc74b209c874db27a715bcbbeac4f6f30aa157b7a8b98e0926474"
//   },
//   "variations": {
//     "x86_64_linux": {
//       "dependencies": [
//         "gettext",
//         "icu4c",
//         "krb5",
//         "lz4",
//         "openssl@3",
//         "readline",
//         "zstd",
//         "linux-pam",
//         "util-linux"
//       ]
//     }
//   }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Versions {
  pub stable: String, // TODO: version schema
  pub head: Option<String>,
  pub bottle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlInfo {
  pub url: String,
  /// git tag
  pub tag: Option<String>,
  /// git revision
  pub revision: Option<String>,
  pub using: Option<String>,
  /// no checksum for git
  pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottle {
  /// path or :any_skip_relocation, or one of:
  ///   /home/linuxbrew/.linuxbrew/Cellar
  ///   /opt/homebrew/Cellar
  ///   /usr/local/Cellar
  pub cellar: String,
  pub url: String,
  pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottles {
  pub rebuild: u32,
  pub root_url: String,
  /// possible keys:
  /// arm64_sonoma, arm64_ventura, arm64_monterey,
  /// sonoma, ventura, monterey,
  /// x86_64_linux,
  pub files: HashMap<String, Bottle>,
}

pub type Dependencies = Vec<String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
#[serde(tag = "name", rename_all = "snake_case")]
/// {"name":"arch","cask":null,"download":null,"version":"x86_64","contexts":[]}
pub enum RequirementName {
  Arch { version: String },
  Linux, Macos,
  MaximumMacos { version: String },
  Xcode,
  // this two only for glibc on linux
  brewedglibcnotolder, linuxkernel,
  gawk, make, sed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
  #[serde(flatten)]
  pub inner: RequirementName,
  pub contexts: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
  Build, Test,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Stages {
  One(Stage),
  Multi(Vec<Stage>)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FromMacOS {
  Name(String),
  Object(HashMap<String, Stages>),
}

/// {"reason":":versioned_formula","explanation":""}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KegCode {
  VersionedFormula,
  #[serde(alias = "provided by macOS")]
  ProvidedByMacos,
  ShadowedByMacos,
  #[serde(alias = "it shadows the host toolchain")]
  ShadowsXcode,
  #[serde(alias = "this installs several executables which shadow macOS system commands")]
  #[serde(alias = "it can shadow system glibc if linked")]
  ShadowsMacos,
  ConflictWith,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reason<T> {
  pub reason: T,
  pub explanation: String,
}

impl TryFrom<Reason<KegCode>> for Reason<String> {
  type Error = serde_json::Error;
  fn try_from(value: Reason<KegCode>) -> Result<Self, Self::Error> {
    let result = match value.reason {
      KegCode::ConflictWith => Self { reason: value.explanation, explanation: String::new() },
      reason => {
        let reason = serde_json::to_value(reason)?;
        Self { reason: format!(":{}", reason.as_str().unwrap()), explanation: value.explanation }
      }
    };
    Ok(result)
  }
}

impl TryFrom<Reason<String>> for Reason<KegCode> {
  type Error = serde_json::Error;
  fn try_from(t: Reason<String>) -> Result<Self, Self::Error> {
    let result = if t.reason.starts_with(":") {
      let reason = serde_json::from_value::<KegCode>(serde_json::Value::String(t.reason[1..].to_string()))?;
      Self {
        reason, explanation: t.explanation,
      }
    } else {
      let reason = serde_json::from_value::<KegCode>(serde_json::Value::String(t.reason.to_string())).unwrap_or(KegCode::ConflictWith);
      Self {
        reason, explanation: if t.explanation.is_empty() { t.reason } else { t.explanation }
      }
    };
    Ok(result)
  }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula {
  pub name: String,
  pub full_name: String,
  pub tap: String,
  pub oldname: Option<String>,
  pub oldnames: Vec<String>,
  pub aliases: Vec<String>,
  pub versioned_formulae: Vec<String>,
  pub desc: String,
  pub license: Option<String>,
  pub homepage: String,
  pub versions: Versions,
  /// possible keys: stable
  pub urls: HashMap<String, UrlInfo>,
  pub revision: u32,
  pub version_scheme: usize,
  /// keys match urls
  pub bottle: HashMap<String, Bottles>,
  /// possible keys: clt_installed
  pub pour_bottle_only_if: Option<String>, // TODO enum?
  pub keg_only: bool,
  #[serde_as(as = "Option<TryFromInto<Reason<String>>>")]
  pub keg_only_reason: Option<Reason<KegCode>>,
  /// unknown, always empty
  pub options: Vec<String>,
  pub build_dependencies: Dependencies,
  pub dependencies: Dependencies,
  pub test_dependencies: Dependencies,
  pub recommended_dependencies: Dependencies,
  pub optional_dependencies: Dependencies,
  pub uses_from_macos: Vec<FromMacOS>, // TODO: add prefix: macos_
  /// possible keys: since
  pub uses_from_macos_bounds: Vec<HashMap<String, String>>,
  /// mostly arch
  pub requirements: Vec<Requirement>,
  pub conflicts_with: Dependencies,
  // pub conflicts_with_reasons: Vec<??>,
  pub link_overwrite: Vec<String>,
  pub caveats: Option<String>,
  pub deprecated: bool,
  pub deprecation_date: Option<String>,
  pub deprecation_reason: Option<String>,
  pub disabled: bool,
  pub disable_date: Option<String>,
  pub disable_reason: Option<String>,
  pub post_install_defined: bool,
  // possible keys: run
  // pub service: Option<Services>,
}

#[test]
fn test_formula() {
  crate::tests::init_logger();
  // TODO: enable brotli?
  let filename = "formula.json";
  let formulas = crate::io::read::read_formulas(filename).unwrap();
  info!(message="parsed", formula.len=formulas.len());
  assert_ne!(formulas.len(), 0);
}
