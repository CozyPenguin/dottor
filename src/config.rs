use std::{cmp::Ordering, env, fs, io};

use regex::Regex;
use serde::{
    de::{self, Visitor},
    Deserialize,
};

use crate::io_util::{check_dir_exists, check_dir_null_or_empty, file_exists, prompt_bool};

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct RootConfiguration {
    pub exclude: Vec<String>,
    pub synchronization: RootSynchronization,
}

impl Default for RootConfiguration {
    fn default() -> Self {
        Self {
            exclude: vec![".git/**".to_string()],
            synchronization: Default::default(),
        }
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
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
            branch: String::from("main"),
        }
    }
}

pub const ROOT: &str = r#"exclude = [".git/**"] # an array of globs which aren't indexed by dottor

[synchronization]
## The repository field can be set either to a "user/repository" string, which will automatically search for that repository on github
## or alternatively be set to a complete url, e.g. "https://my-fabulous-git-server.com/omega-dotfiles/"
repository = ""
# remote = "origin"
# branch = "main"
"#;

pub const ROOT_PATH: &str = "dottor.toml";

pub fn check_root_present() -> io::Result<()> {
    let result = file_exists(ROOT_PATH);

    match result {
        Ok(exists) => {
            if exists {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Directory doesn't contain root config '{ROOT_PATH}'."),
                ))
            }
        }
        Err(err) => Err(err),
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Default)]
pub struct Configuration {
    pub config: Config,
    pub deploy: Deploy,
    pub dependencies: Dependencies,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Default)]
pub struct Config {
    pub name: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Default)]
pub struct Deploy {
    pub exclude: Vec<String>,
    pub windows: DeployTarget,
    pub linux: DeployTarget,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Default)]
pub struct DeployTarget {
    pub target: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Default)]
pub struct Dependencies {
    pub simple: SimpleDependencies,
    pub local: Vec<LocalDependency>,
    pub system: Vec<SystemDependency>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Default)]
pub struct SimpleDependencies {
    pub local: Vec<String>,
    pub system: Vec<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
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
#[derive(Deserialize, Debug)]
pub struct SystemDependency {
    name: String,
    #[serde(default)]
    required: bool,
    #[serde(deserialize_with = "de_from_str")]
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

#[derive(Debug)]
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

fn de_from_str<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Version::deserialize(deserializer)
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

pub const CONFIG: &str = r#"[config]
# name = '' # defaults to directory name

[deploy]
exclude = [] # an array of globs which aren't exported

    [deploy.windows]
    target = ''

    [deploy.linux]
    target = ''

## Specify dependencies on other configurations or programs that are required for this configuration
[dependencies]
    [dependencies.simple]
    ## The easiest way to declare a local dependency is to add a string to the 'local' array.
    ## For example, the string "theming" will make the configuration depend on the config with the name "theming"
    local = [] 
    ## System dependencies are used to check if programs or files are present on the system path
    system = []

    # ## You can also declare more complex dependencies using [dependencies.local.name] or [dependencies.system.name]
    # [[dependencies.local]]
    # name = 'example' # the name of the configuration
    # required = true # if false, config deployment will not fail if the dependency isn't found or failed itself
 
    # [[dependencies.system]]
    # name = 'example' # the program/file this config depends on
    # required = true # if false, config deployment will not fail if the dependency isn't found or the versions don't match
    # ## Dottor can check if a dependency meets specific version requirements
    # ## To do so, the dependency is executed with an argument from which the semantic version, according to semver 2.0.0, is parsed using a regex
    # ## version checking makes use of version ranges (there's a great overview available at https://github.com/QuiltMC/rfcs/blob/master/specification/0002-quilt.mod.json.md#version-specifier):
    # ## WARNING! using this will execute the program, so don't use this if you don't trust the program or the system you're working on
    # version = '0.1.0' # the version requirement
    # version_args = '--version' # the arguments that should be passed to the program, common options include "--version" or "-v"
"#;

pub const CONFIG_PATH: &str = "dotconfig.toml";

pub fn create_config(name: &str) -> io::Result<()> {
    let mut path = env::current_dir()?;
    path.push(name);
    check_dir_null_or_empty(name)?;

    fs::create_dir_all(path.clone())?;
    path.push(CONFIG_PATH);
    fs::write(path, CONFIG)
}

pub fn delete_config(name: &str) -> io::Result<()> {
    check_dir_exists(name)?;
    if prompt_bool(
        "Proceeding will cause the config and all files in the directory to be deleted.",
        false,
    ) {
        fs::remove_dir_all(name)
    } else {
        Ok(())
    }
}
