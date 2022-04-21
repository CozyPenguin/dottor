#![feature(iter_collect_into)]

use std::env;
use std::fs;
use std::path::PathBuf;

use clap::arg;
use clap::ArgMatches;
use clap::{command, Command};
use config::read_configuration;
use config::RootConfiguration;
use err::Error;
use git2::Repository;
use glob::glob;
use io_util::check_dir_null_or_empty;
use io_util::check_empty;
use io_util::check_root_present;
use io_util::create_dir_all;
use io_util::current_dir;
use io_util::list_root;
use io_util::set_current_dir;
use io_util::write;
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
                .arg(arg!([NAME] "The name of the configuration")),
        )
        .subcommand(
            Command::new(subcommands::DEPLOY).about("Deploy your configurations to the system"),
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
        Some((subcommands::DEPLOY, _)) => deploy(),
        _ => Ok(()),
    } {
        eprintln!("{} Aborting!", error);
    }
}

fn init() -> err::Result<()> {
    // check that we don't accidentally populate an existing directory
    check_empty("")?;
    // create the default root configuration
    write(
        config::ROOT_PATH,
        toml::to_string_pretty(&RootConfiguration::default())
            .map_err(|_| Error::new("could not create root configuration"))?,
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
    config::create_config(name.as_ref())
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

fn deploy() -> err::Result<()> {
    let dirs = list_root()?;

    for dir in dirs {
        let dir = dir.map_err(|_| Error::new("Error while reading config directory."))?;
        let mut path = dir.path();
        path.push(config::CONFIG_PATH);

        if path.exists() && path.is_file() {
            let local_config = read_configuration(&path)?;
            path.pop();
            match env::consts::OS {
                "windows" => deploy_to(
                    path.file_name().unwrap().to_str().unwrap(),
                    local_config.deploy.windows.target.as_str(),
                )?,
                _ => (),
            }
        } else {
            ()
        }
    }

    Ok(())
}

fn deploy_to(name: &str, path: &str) -> err::Result<()> {
    // checks if the directory already has files in it
    check_dir_null_or_empty(path)?;
    create_dir_all(path)?;

    // switches the directory to the configuration
    let dir = current_dir()?;
    set_current_dir(name)?;

    let pattern = "./**/*";

    for entry in
        glob(&pattern[..]).expect(format!("Failed to read glob pattern '{}'", pattern).as_str())
    {
        if let Ok(inner) = entry {
            if !inner.to_str().unwrap().eq(config::CONFIG_PATH) && inner.is_file() {
                let mut to = PathBuf::from(path);
                to.push(inner.clone());
                let to = to.as_path();
                let inner = inner.as_path();
                create_dir_all(to.parent().unwrap())?;
                fs::copy(inner, to).unwrap();
            }
        } else {
            println!("{:?}", entry);
        }
    }

    set_current_dir(dir)?;

    Ok(())
}
