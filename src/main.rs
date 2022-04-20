#![feature(io_error_more)]

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

use clap::arg;
use clap::ArgMatches;
use clap::{command, Command};
use config::check_root_present;
use git2::Repository;
use glob::glob;
use io_util::check_dir_null_or_empty;
use io_util::check_empty;
use io_util::list_root;
use io_util::parse_config;

mod config;
mod io_util;

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
        Some((subcommands::CONFIG, sub_matches)) => config(sub_matches),
        Some((subcommands::DEPLOY, _)) => deploy(),
        _ => Ok(()),
    } {
        eprintln!("{} Aborting!", error);
    }
}

fn init() -> io::Result<()> {
    // check that we don't accidentally populate an existing directory
    check_empty("")?;
    // create the default root configuration
    fs::write(config::ROOT_PATH, config::ROOT)?;

    // initialize a new git repository 
    let repo = match Repository::init("./") {
        Ok(value) => value,
        Err(_) => return Err(io::Error::new(io::ErrorKind::Other, "could not initialise git repository")),
    };

    Ok(())
}

/// runs the config command
fn config(matches: &ArgMatches) -> io::Result<()> {
    check_root_present()?;

    match matches.subcommand() {
        Some((subcommands::config::CREATE, sub_matches)) => config_create(sub_matches),
        Some((subcommands::config::DELETE, sub_matches)) => config_delete(sub_matches),
        None => config_create(matches),
        _ => Ok(()),
    }
}

/// creates a new config
fn config_create(matches: &ArgMatches) -> io::Result<()> {
    let name = matches.value_of("NAME").expect("name not provided");
    config::create_config(name)
}

/// deletes a config
fn config_delete(matches: &ArgMatches) -> io::Result<()> {
    let name = matches.value_of("NAME").expect("name not provided");
    config::delete_config(name)
}

fn deploy() -> io::Result<()> {
    let dirs = list_root()?;

    for dir in dirs {
        let dir = dir?;
        let mut path = dir.path();
        path.push(config::CONFIG_PATH);

        if path.exists() && path.is_file() {
            let local_config = parse_config(&path)?;
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

fn deploy_to(name: &str, path: &str) -> io::Result<()> {
    // checks if the directory already has files in it
    check_dir_null_or_empty(path)?;
    fs::create_dir_all(path)?;

    // switches the directory to the configuration
    let dir = env::current_dir()?;
    env::set_current_dir(name)?;

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
                fs::create_dir_all(to.parent().unwrap())?;
                fs::copy(inner, to).unwrap();
            }
        } else {
            println!("{:?}", entry);
        }
    }
    
    env::set_current_dir(dir)?;

    Ok(())
}
