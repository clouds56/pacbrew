use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_with::{serde_as, TryFromInto};

// {
//   "name": "postgresql@14",
//   "full_name": "postgresql@14",
//   "tap": "homebrew/core",
//   "oldname": "postgresql",
//   "aliases": [],
//   "versioned_formulae": ["postgresql@13", "postgresql@12", "postgresql@11", "postgresql@10", "postgresql@9.6", "postgresql@9.5", "postgresql@9.4"],
//   "desc": "Object-relational database system",
//   "license": "PostgreSQL",
//   "homepage": "https://www.postgresql.org/",
//   "versions": {
//     "stable": "14.5",
//     "head": null,
//     "bottle": true
//   },
//   "urls": {
//     "stable": {
//       "url": "https://ftp.postgresql.org/pub/source/v14.5/postgresql-14.5.tar.bz2",
//       "tag": null,
//       "revision": null
//     }
//   },
//   "revision": 5,
//   "version_scheme": 0,
//   "bottle": {
//     "stable": {
//       "rebuild": 0,
//       "root_url": "https://ghcr.io/v2/homebrew/core",
//       "files": {
//         "arm64_monterey": {
//           "cellar": "/opt/homebrew/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/14/blobs/sha256:5cb8cbf8fee5ba9a32a223deff80f6c885deb430d1bd38bc113e971d239d1534",
//           "sha256": "5cb8cbf8fee5ba9a32a223deff80f6c885deb430d1bd38bc113e971d239d1534"
//         },
//         "arm64_big_sur": {
//           "cellar": "/opt/homebrew/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/14/blobs/sha256:52eabb4213030febe2435da260983eabbe7c6d29c2758829c53a43a8eb05d85c",
//           "sha256": "52eabb4213030febe2435da260983eabbe7c6d29c2758829c53a43a8eb05d85c"
//         },
//         "monterey": {
//           "cellar": "/usr/local/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/14/blobs/sha256:f45e2b403e4dc0303f0c61cb429a1bd58c3c1cbe365adf5dbfada569cd2ba094",
//           "sha256": "f45e2b403e4dc0303f0c61cb429a1bd58c3c1cbe365adf5dbfada569cd2ba094"
//         },
//         "big_sur": {
//           "cellar": "/usr/local/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/14/blobs/sha256:49a0e979e4c58f1e6e6c63b3ac118a4c9216428ae1bd3b1114ed24602b4dadda",
//           "sha256": "49a0e979e4c58f1e6e6c63b3ac118a4c9216428ae1bd3b1114ed24602b4dadda"
//         },
//         "catalina": {
//           "cellar": "/usr/local/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/14/blobs/sha256:1d800b98a3a1715ec852beeb8fc6b28520d1f54f91ee521714309cfbfe139e2b",
//           "sha256": "1d800b98a3a1715ec852beeb8fc6b28520d1f54f91ee521714309cfbfe139e2b"
//         },
//         "x86_64_linux": {
//           "cellar": "/home/linuxbrew/.linuxbrew/Cellar",
//           "url": "https://ghcr.io/v2/homebrew/core/postgresql/14/blobs/sha256:a4bf3d8e17f8a922d560c9d4f2151bc61f799d9dd15e29b1e9fc72fa9c6c4b12",
//           "sha256": "a4bf3d8e17f8a922d560c9d4f2151bc61f799d9dd15e29b1e9fc72fa9c6c4b12"
//         }
//       }
//     }
//   },
//   "keg_only": false,
//   "keg_only_reason": null,
//   "options": [],
//   "build_dependencies": ["pkg-config"],
//   "dependencies": ["icu4c", "krb5", "lz4", "openssl@1.1", "readline"],
//   "test_dependencies": [],
//   "recommended_dependencies": [],
//   "optional_dependencies": [],
//   "uses_from_macos": ["libxml2", "libxslt", "openldap", "perl"],
//   "requirements": [],
//   "conflicts_with": [],
//   "caveats": "This formula has created a default database cluster with:\n  initdb --locale=C -E UTF-8 $(brew --prefix)/var/postgresql@14\nFor more details, read:\n  https://www.postgresql.org/docs/14/app-initdb.html\n",
//   "installed": [{
//     "version": "14.5_4",
//     "used_options": [],
//     "built_as_bottle": true,
//     "poured_from_bottle": true,
//     "time": 1664768139,
//     "runtime_dependencies": [{
//       "full_name": "icu4c",
//       "version": "71.1",
//       "declared_directly": true
//     }, {
//       "full_name": "ca-certificates",
//       "version": "2022-07-19",
//       "declared_directly": false
//     }, {
//       "full_name": "openssl@1.1",
//       "version": "1.1.1q",
//       "declared_directly": true
//     }, {
//       "full_name": "krb5",
//       "version": "1.20",
//       "declared_directly": true
//     }, {
//       "full_name": "lz4",
//       "version": "1.9.4",
//       "declared_directly": true
//     }, {
//       "full_name": "readline",
//       "version": "8.1.2",
//       "declared_directly": true
//     }],
//     "installed_as_dependency": false,
//     "installed_on_request": true
//   }],
//   "linked_keg": "14.5_4",
//   "pinned": false,
//   "outdated": true,
//   "deprecated": false,
//   "deprecation_date": "2026-11-12",
//   "deprecation_reason": null,
//   "disabled": false,
//   "disable_date": null,
//   "disable_reason": null,
//   "variations": {
//     "x86_64_linux": {
//       "dependencies": ["icu4c", "krb5", "lz4", "openssl@1.1", "readline", "libxml2", "libxslt", "openldap", "perl", "linux-pam", "util-linux"]
//     }
//   }
// },

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
  pub rebuild: usize,
  pub root_url: String,
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
  #[serde(alias = "maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_macos")]
  #[serde(alias = "maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_maximum_macos")]
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
  pub old_name: Option<String>,
  pub aliases: Vec<String>,
  pub versioned_formulae: Vec<String>,
  pub desc: String,
  pub license: Option<String>,
  pub homepage: String,
  pub versions: Versions,
  pub urls: HashMap<String, UrlInfo>,
  pub revision: usize,
  pub version_scheme: usize,
  pub bottle: HashMap<String, Bottles>,
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
  /// mostly arch
  pub requirements: Vec<Requirement>,
  pub conflicts_with: Dependencies,
  pub caveats: Option<String>,
}
