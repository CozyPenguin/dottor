#![feature(let_chains)]

use std::env;
use std::path::PathBuf;

use clap::arg;
use clap::ArgMatches;
use clap::{command, Command};
use config::Configuration;
use config::RootConfiguration;
use err::Error;
use git2::Repository;
use glob::glob;
use glob::GlobError;
use io_util::check_dir_null_or_empty;
use io_util::check_not_empty;
use io_util::check_root_present;
use io_util::set_current_dir;
use io_util::write;
use path_abs::PathAbs;
use path_abs::PathDir;
use path_abs::PathOps;
use path_abs::PathType;
use structure::Structure;

mod config;
mod err;
mod io_util;
mod structure;

mod subcommands {
    pub const CONFIG: &str = "config";
    pub const DEPLOY: &str = "deploy";
    pub const INIT: &str = "init";
    pub const UPGRADE: &str = "upgrade";
    pub mod config {
        pub const CREATE: &str = "create";
        pub const DELETE: &str = "delete";
    }
}

fn main() {
    let structure = structure::Structure::resolve().unwrap();

    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new(subcommands::CONFIG)
                .arg_required_else_help(true)
                .about("Manage your individual dotfile configurations")
                .subcommand(
                    Command::new(subcommands::config::CREATE)
                        .about("Create a new configuration")
                        .arg_required_else_help(true)
                        .arg(arg!(<NAME> "The name of the configuration")),
                )
                .subcommand(
                    Command::new(subcommands::config::DELETE)
                        .about("Delete a configuration")
                        .arg_required_else_help(true)
                        .arg(arg!(<NAME> "The name of the configuration")),
                )
                .arg(arg!(<NAME> "The name of the configuration")),
        )
        .subcommand(
            Command::new(subcommands::DEPLOY)
                .about("Deploy your configurations to the system")
                .arg(arg!([name] "The name of the configuration")),
        )
        .subcommand(
            Command::new(subcommands::INIT)
                .about("Initialize a new dotfiles repo in the current directory"),
        )
        .subcommand(
            Command::new(subcommands::UPGRADE)
                .about("Migrate your dotfiles repo after upgrading dottor"),
        )
        .get_matches();

    if let Err(error) = match matches.subcommand() {
        Some((subcommands::INIT, _)) => init(),
        Some((subcommands::CONFIG, sub_matches)) => config(sub_matches, structure),
        Some((subcommands::DEPLOY, sub_matches)) => deploy(sub_matches, structure),
        _ => Ok(()),
    } {
        eprintln!("{} Aborting!", error);
    }
}

fn init() -> err::Result<()> {
    // check that we don't accidentally populate an existing directory
    check_not_empty(&PathDir::current_dir()?)?;
    // create the default root configuration
    write(
        &PathAbs::new(config::ROOT_PATH)?,
        toml::to_string_pretty(&RootConfiguration::default())
            .map_err(|_| Error::new("could not create root configuration."))?
            .as_bytes(),
    )?;

    // initialize a new git repository
    match Repository::init("./") {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::new("Could not initialize git repository.")),
    }
}

/// verifies that the structure of the dotfiles folder is correct
/// It does not however verify the configs inside of the folder
fn verify_structure(structure: Option<Structure>) -> err::Result<Structure> {
    match structure {
        Some(value) => Ok(value),
        None => Err(Error::new(
            "Structure of the dotfiles repository is invalid.",
        )),
    }
}

/// runs the config command
fn config(matches: &ArgMatches, structure: Option<Structure>) -> err::Result<()> {
    check_root_present()?;
    let structure = verify_structure(structure)?;

    match matches.subcommand() {
        Some((subcommands::config::CREATE, sub_matches)) => config_create(sub_matches, structure),
        Some((subcommands::config::DELETE, sub_matches)) => config_delete(sub_matches, structure),
        None => config_create(matches, structure), // if no subcommand was provided, create is implied (like e.g. git branch)
        _ => Ok(()),
    }
}

/// creates a new config
fn config_create(matches: &ArgMatches, structure: Structure) -> err::Result<()> {
    let name = matches.value_of("NAME").expect("name not provided");
    if structure.configs.contains_key(name) {
        return Err(Error::from_string(format!(
            "There already exists a config with the name '{}'",
            name
        )));
    }
    config::create_config(name)
}

/// deletes a config
fn config_delete(matches: &ArgMatches, structure: Structure) -> err::Result<()> {
    let name = matches.value_of("NAME").expect("name not provided");
    if structure.configs.contains_key(name) {
        return Err(Error::from_string(format!(
            "There is no config with the name '{}'",
            name
        )));
    }
    config::delete_config(name)
}

fn deploy(matches: &ArgMatches, structure: Option<Structure>) -> err::Result<()> {
    check_root_present()?;
    let mut structure = verify_structure(structure)?;

    let name = matches.value_of("name");

    if let Some(name) = name {
        let config = structure.configs.remove(name);
        match config {
            Some(config) => deploy_to(&String::from(name), config),
            None => Err(Error::from_string(format!(
                "Config '{name}' does not exist."
            ))),
        }
    } else {
        for (name, config) in structure.configs {
            match deploy_to(&name, config) {
                Ok(_) => {}
                Err(error) => println!("Could not deploy config '{}': {}", name, error),
            }
        }
        Ok(())
    }
}

fn deploy_to(name: &String, config: Configuration) -> err::Result<()> {
    let mut target = match env::consts::OS {
        "windows" => config.deploy.windows,
        "linux" => config.deploy.linux,
        value => {
            return Err(Error::from_string(format!(
                "Operating system '{value}' is not supported."
            )))
        }
    };

    let target_path = PathAbs::new(&shellexpand::tilde(&target.target).into_owned())?;

    // checks if the target directory already has files in it
    match &target.target_require_empty {
        Some(value) => {
            if *value {
                check_dir_null_or_empty(&target_path)?;
            }
        }
        None => {
            if config.deploy.target_require_empty {
                check_dir_null_or_empty(&target_path)?;
            }
        }
    }
    // create target
    PathDir::create_all(&target_path)?;

    // switches the directory to the configuration
    let dir = PathDir::current_dir()?;
    set_current_dir(&name)?;

    let pattern = "./**/*";

    let mut excluded_files = config.deploy.exclude;
    excluded_files.append(&mut target.exclude);
    let mut excluded_files = excluded_files
        .iter()
        .map(|pattern| glob(pattern))
        .flatten()
        .flatten()
        .collect::<Result<Vec<PathBuf>, GlobError>>()
        .map_err(|_| Error::new("Failed to read exclude patterns"))?
        .iter()
        .map(PathType::new)
        .collect::<Result<Vec<PathType>, path_abs::Error>>()?;

    excluded_files.push(PathType::new(config::CONFIG_PATH)?);
    // copy files to target
    for entry in glob(&pattern[..])
        .map_err(|_| Error::from_string(format!("Failed to read glob pattern '{}'", pattern)))?
    {
        if let Ok(inner) = entry {
            let from = PathType::new(&inner)?;
            let to = target_path.concat(inner)?;

            if !excluded_files.contains(&from) {
                match from {
                    PathType::File(file) => {
                        file.copy(to)?;
                    }
                    PathType::Dir(dir) => {
                        PathDir::create_all(dir)?;
                    }
                }
            }
        } else {
            println!("{:?}", entry);
        }
    }

    set_current_dir(dir)?;

    Ok(())
}
