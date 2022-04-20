use std::{env, fs, io};

use regex::Regex;
use serde::{
    de::{self, Visitor},
    Deserialize,
};

use crate::io_util::{check_dir_exists, check_dir_null_or_empty, file_exists, prompt_bool};

pub const ROOT: &str = r#"exclude = [] # an array of directories which aren't indexed by dottor

[synchronisation]
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
#[derive(Deserialize, Debug)]
pub struct Configuration {
    pub config: Config,
    pub deploy: Deploy,
    pub dependencies: Dependencies,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Config {
    pub name: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Deploy {
    pub exclude: Vec<String>,
    pub windows: DeployTarget,
    pub linux: DeployTarget,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct DeployTarget {
    pub target: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Dependencies {
    pub simple: SimpleDependencies,
    pub local: Vec<LocalDependency>,
    pub system: Vec<SystemDependency>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct SimpleDependencies {
    pub local: Vec<String>,
    pub system: Vec<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct LocalDependency {
    name: String,
    required: bool,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct SystemDependency {
    name: String,
    required: bool,
    version_args: String,
    #[serde(deserialize_with = "de_from_str")]
    version: Version,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Version {
        Version {
            major: major,
            minor: minor,
            patch: patch,
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
                    static ref RE: Regex = Regex::new(r"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)(?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+(?P<buildmetadata>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$").unwrap();
                }

                let version_match = RE.captures(&v).unwrap();
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

                Ok(Version::new(major, minor, patch))
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
