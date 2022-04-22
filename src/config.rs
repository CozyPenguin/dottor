use std::cmp::Ordering;

use path_abs::{PathAbs, PathDir, PathFile, PathOps};
use regex::Regex;
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};

use crate::{
    err::{self, Error},
    io_util::{
        check_dir_null_or_empty, check_root_present, check_valid_dir, prompt_bool, read_to_string,
        write,
    },
};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct RootConfiguration {
    pub exclude: Vec<String>,
    pub synchronization: RootSynchronization,
}

impl Default for RootConfiguration {
    fn default() -> Self {
        Self {
            exclude: vec![".git/".to_string()],
            synchronization: Default::default(),
        }
    }
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct RootSynchronization {
    pub repository: String,
    pub remote: String,
    pub branch: String,
}

impl Default for RootSynchronization {
    fn default() -> Self {
        Self {
            repository: Default::default(),
            remote: String::from("origin"),
            branch: {
                // tries to set the default branch name to the one specified in the git configuration, with "main" as a fallback
                match git2::Config::open_default() {
                    Ok(config) => config
                        .get_string("init.defaultBranch")
                        .unwrap_or(String::from("main")),
                    Err(_) => String::from("main"),
                }
            },
        }
    }
}

pub const ROOT_PATH: &str = "dottor.toml";

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Configuration {
    pub config: Config,
    pub deploy: Deploy,
    pub dependencies: Dependencies,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub name: Option<String>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Deploy {
    pub exclude: Vec<String>,
    #[serde(default)]
    pub target_require_empty: bool,
    pub windows: DeployTarget,
    pub linux: DeployTarget,
}

impl Default for Deploy {
    fn default() -> Self {
        Self {
            exclude: Default::default(),
            target_require_empty: true,
            windows: Default::default(),
            linux: Default::default(),
        }
    }
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DeployTarget {
    pub target: String,
    pub target_require_empty: Option<bool>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Dependencies {
    #[serde(default)]
    pub simple: SimpleDependencies,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub local: Vec<LocalDependency>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub system: Vec<SystemDependency>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SimpleDependencies {
    #[serde(default)]
    pub local: Vec<String>,
    #[serde(default)]
    pub system: Vec<String>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct LocalDependency {
    name: String,
    #[serde(default)]
    required: bool,
}

impl Default for LocalDependency {
    fn default() -> Self {
        Self {
            name: Default::default(),
            required: true,
        }
    }
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct SystemDependency {
    name: String,
    #[serde(default)]
    required: bool,
    #[serde(
        deserialize_with = "Version::deserialize",
        serialize_with = "Version::serialize"
    )]
    version: Version,
    #[serde(default)]
    version_args: String,
}

impl Default for SystemDependency {
    fn default() -> Self {
        Self {
            name: Default::default(),
            required: true,
            version: Default::default(),
            version_args: String::from("--version"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum VersionSpecifier {
    Any,
    None,
    Equals,
    GreaterEquals,
    GreaterThan,
    LessEquals,
    LessThan,
    MatchMinor,
    MatchMajor,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Version {
    pub specifier: VersionSpecifier,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(specifier: VersionSpecifier, major: u32, minor: u32, patch: u32) -> Version {
        Version {
            specifier: specifier,
            major: major,
            minor: minor,
            patch: patch,
        }
    }

    pub fn any() -> Version {
        Version {
            specifier: VersionSpecifier::Any,
            major: 0,
            minor: 0,
            patch: 0,
        }
    }

    pub fn compatible(&self, version: &Self) -> bool {
        match self.specifier {
            VersionSpecifier::Any => true,
            VersionSpecifier::None | VersionSpecifier::MatchMajor => self.major == version.major,
            VersionSpecifier::Equals => self == version,
            VersionSpecifier::GreaterEquals => self >= version,
            VersionSpecifier::GreaterThan => self > version,
            VersionSpecifier::LessEquals => self <= version,
            VersionSpecifier::LessThan => self < version,
            VersionSpecifier::MatchMinor => {
                self.major == version.major && self.minor == version.minor
            }
        }
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major && self.minor == other.minor && self.patch == other.patch
    }

    fn ne(&self, other: &Self) -> bool {
        self.major != other.major || self.minor != other.minor || self.patch != other.patch
    }
}

impl PartialOrd for Version {
    fn ge(&self, other: &Self) -> bool {
        self.major >= other.major
            || self.major == other.major && self.minor >= other.minor
            || self.major == other.major && self.minor == other.minor && self.patch >= other.patch
    }

    fn gt(&self, other: &Self) -> bool {
        self.major > other.major
            || self.major == other.major && self.minor > other.minor
            || self.major == other.major && self.minor == other.minor && self.patch > other.patch
    }
    fn le(&self, other: &Self) -> bool {
        self.major <= other.major
            || self.major == other.major && self.minor <= other.minor
            || self.major == other.major && self.minor == other.minor && self.patch <= other.patch
    }
    fn lt(&self, other: &Self) -> bool {
        self.major < other.major
            || self.major == other.major && self.minor < other.minor
            || self.major == other.major && self.minor == other.minor && self.patch < other.patch
    }

    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let major = self.major.partial_cmp(&other.major)?;
        if major != Ordering::Equal {
            return Some(major);
        }
        let minor = self.minor.partial_cmp(&other.minor)?;
        if minor != Ordering::Equal {
            return Some(minor);
        }
        let patch = self.minor.partial_cmp(&other.patch)?;
        if patch != Ordering::Equal {
            return Some(patch);
        }
        Some(Ordering::Equal)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self {
            specifier: VersionSpecifier::None,
            major: 1,
            minor: 0,
            patch: 0,
        }
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.specifier == VersionSpecifier::Any {
            serializer.serialize_str("*")
        } else {
            serializer.serialize_str(
                format!(
                    "{}{}.{}.{}",
                    match self.specifier {
                        VersionSpecifier::Any => "*",
                        VersionSpecifier::None => "",
                        VersionSpecifier::Equals => "=",
                        VersionSpecifier::GreaterEquals => ">=",
                        VersionSpecifier::GreaterThan => ">",
                        VersionSpecifier::LessEquals => "<=",
                        VersionSpecifier::LessThan => "<",
                        VersionSpecifier::MatchMinor => "~",
                        VersionSpecifier::MatchMajor => "^",
                    },
                    self.major,
                    self.minor,
                    self.patch
                )
                .as_str(),
            )
        }
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct VersionVisitor;

        impl<'de> Visitor<'de> for VersionVisitor {
            type Value = Version;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Version")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                lazy_static::lazy_static! {
                    static ref RE: Regex = Regex::new(r"^(?P<asterisk>\*)$|^(?P<specifier>=|>=|>|<=|<|~|\^)?(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)(?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+(?P<buildmetadata>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$").unwrap();
                }

                if !RE.is_match(&v) {
                    return Err(de::Error::custom("could not parse version"));
                }

                let version_match = RE.captures(&v).unwrap();

                // matches the single asterisk
                if let Some(_) = version_match.name("asterisk") {
                    return Ok(Version::any());
                }

                // checks for specifier
                let specifier = match version_match.name("specifier") {
                    Some(value) => match value.as_str() {
                        "=" => VersionSpecifier::Equals,
                        ">=" => VersionSpecifier::GreaterEquals,
                        ">" => VersionSpecifier::GreaterThan,
                        "<=" => VersionSpecifier::LessEquals,
                        "<" => VersionSpecifier::LessThan,
                        "~" => VersionSpecifier::MatchMinor,
                        "^" => VersionSpecifier::MatchMajor,
                        _ => return Err(de::Error::custom("invalid version specifier")),
                    },
                    None => VersionSpecifier::None,
                };

                // matches actual version
                let major = version_match
                    .name("major")
                    .ok_or_else(|| de::Error::custom("no major version found"))?
                    .as_str()
                    .parse::<u32>()
                    .unwrap();
                let minor = version_match
                    .name("minor")
                    .ok_or_else(|| de::Error::custom("no minor version found"))?
                    .as_str()
                    .parse::<u32>()
                    .unwrap();
                let patch = version_match
                    .name("patch")
                    .ok_or_else(|| de::Error::custom("no patch version found"))?
                    .as_str()
                    .parse::<u32>()
                    .unwrap();

                Ok(Version::new(specifier, major, minor, patch))
            }
        }

        deserializer.deserialize_str(VersionVisitor)
    }
}

pub const CONFIG_PATH: &str = "dotconfig.toml";

pub fn create_config(name: &str) -> err::Result<()> {
    let path = PathDir::current_dir()?.concat(name)?;
    check_dir_null_or_empty(&path)?;
    PathDir::create_all(&path)?;
    let path = path.concat(CONFIG_PATH)?;
    write(
        &path,
        toml::to_string_pretty(&Configuration::default())
            .map_err(|_| Error::new("Could not create configuration file in config."))?
            .as_bytes(),
    )
}

pub fn delete_config(name: &str) -> err::Result<()> {
    let dir = PathDir::new(name)?;
    check_valid_dir(&PathAbs::new(&dir)?)?;
    if prompt_bool(
        "Proceeding will cause the config and all files in the directory to be deleted.",
        false,
    ) {
        Ok(PathDir::remove_all(dir)?)
    } else {
        Ok(())
    }
}

pub fn read_configuration(file: &PathFile) -> err::Result<Configuration> {
    let source = read_to_string(file)?;
    let config = toml::from_str(&source[..]).map_err(|_| {
        Error::from_string(format!(
            "Could not parse configuration file '{}'.",
            file.as_path().display()
        ))
    })?;
    Ok(config)
}

pub fn read_root_configuration() -> err::Result<RootConfiguration> {
    check_root_present()?;
    let source = read_to_string(&PathFile::new(ROOT_PATH)?)?;
    let config = toml::from_str(&source[..])
        .map_err(|_| Error::new("Could not parse root configuration."))?;
    Ok(config)
}
