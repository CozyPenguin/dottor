use std::env;
use std::env::current_dir;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use clap::arg;
use clap::ArgMatches;
use clap::{command, Command};
use config::Configuration;
use config::RootConfiguration;
use config::ROOT_PATH;
use err::Error;
use git2::Repository;
use globset::Glob;
use globset::GlobMatcher;
use globset::GlobSet;
use globset::GlobSetBuilder;
use io::assert_empty;
use io::check_dir_null_or_empty;
use io::check_root_present;
use io::prompt_bool;
use io::write;
use relative_path::RelativePathBuf;
use similar::ChangeTag;
use similar::TextDiff;
use structure::Structure;
use walkdir::WalkDir;

mod config;
mod err;
mod io;
mod structure;

mod subcommands {
    pub const CONFIG: &str = "config";
    pub const INIT: &str = "init";
    pub const NEW: &str = "new";
    pub mod config {
        pub const CREATE: &str = "create";
        pub const DELETE: &str = "delete";
        pub const DEPLOY: &str = "deploy";
        pub const PULL: &str = "pull";
    }
}

fn init_logger() {}

fn main() {
    let structure = structure::Structure::resolve().unwrap();

    init_logger();

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
                .subcommand(
                    Command::new(subcommands::config::DEPLOY)
                        .about("Deploy your configurations to the system")
                        .arg_required_else_help(true)
                        .arg(arg!([name] "The name of the configuration"))
                        .arg(arg!(-a --all "Deploy all configurations")),
                )
                .subcommand(
                    Command::new(subcommands::config::PULL)
                        .about(
                            "Pull changes from the deployed configuration into the dotfiles repo",
                        )
                        .arg_required_else_help(true)
                        .arg(arg!([name] "The name of the configuration"))
                        .arg(arg!(-a --all "Pull in changes from all configurations"))
                        .arg(arg!(-f --force "Don't ask for confirmation when pulling in changes")),
                )
                .arg(arg!([NAME] "The name of the configuration")),
        )
        .subcommand(
            Command::new(subcommands::INIT)
                .about("Initialize a new dotfiles repository in the current directory"),
        )
        .subcommand(
            Command::new(subcommands::NEW)
                .about("Initialize a new dotfiles repository in a subdirectory")
                .arg(arg!(<FOLDER> "The folder where the dotfiles repository will be created")),
        )
        .get_matches();

    if let Err(error) = match matches.subcommand() {
        Some((subcommands::INIT, _)) => init(&current_dir().unwrap()),
        Some((subcommands::NEW, sub_matches)) => {
            new(sub_matches.get_one("NAME").unwrap() as &String)
        }
        Some((subcommands::CONFIG, sub_matches)) => config(sub_matches, structure),
        _ => Ok(()),
    } {
        eprintln!("{} Aborting!", error);
    }
}

/// Initialize a new dottor repository in the current directory
fn init(path: &Path) -> err::Result<()> {
    // check that we don't accidentally populate an existing directory
    assert_empty(path)?;

    // create the default root configuration
    write(
        &RelativePathBuf::from(ROOT_PATH).to_path("."),
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

fn new(name: &str) -> err::Result<()> {
    let path = RelativePathBuf::from(name).to_path(".");
    init(&path)
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
        Some((subcommands::config::DEPLOY, sub_matches)) => config_deploy(sub_matches, structure),
        Some((subcommands::config::PULL, sub_matches)) => config_pull(sub_matches, structure),
        _ => Err(err::Error::new("Invalid subcommand")),
    }
}

/// creates a new config
fn config_create(matches: &ArgMatches, structure: Structure) -> err::Result<()> {
    let name: &String = matches.get_one("NAME").expect("name not provided");
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
    let name: &String = matches.get_one("NAME").expect("name not provided");
    if structure.configs.contains_key(name) {
        return Err(Error::from_string(format!(
            "There is no config with the name '{}'",
            name
        )));
    }

    if prompt_bool("Do you want to delete this configuration? ", false) {
        config::delete_config(name)
    } else {
        Ok(())
    }
}

fn config_pull(matches: &ArgMatches, mut structure: Structure) -> err::Result<()> {
    let name: Option<&String> = matches.get_one("name");
    let all = matches.get_flag("all");
    let force = matches.get_flag("force");

    if let Some(name) = name {
        if all {
            return Err(Error::new("You cannot use the all flag in combination with a specific configuration. Try removing \"--all\" or the configuration name."));
        }
        let config = structure.configs.remove(name);
        match config {
            Some(config) => pull_single(&String::from(name), config, force),
            None => Err(Error::from_string(format!(
                "Config '{name}' does not exist."
            ))),
        }
    } else if all {
        for (name, config) in structure.configs {
            if !force {
                println!("Pulling config '{}'", name);
            }
            match pull_single(&name, config, force) {
                Ok(_) => {}
                Err(error) => println!("Could not pull config '{}': {}", name, error),
            }
        }
        Ok(())
    } else {
        Err(Error::new("No configurations matched the query."))
    }
}

/// pull local changes from a config into the repository
fn pull_single(name: &String, config: Configuration, force: bool) -> err::Result<()> {
    fn print_file_name(
        name: &Path,
        modifier_symbol: &'static str,
        separator_pos: usize,
        total_width: usize,
        continue_table: bool,
    ) {
        println!(
            "{char:\u{2550}^width_left$}\u{2564}{char:\u{2550}^width_right$}",
            char = "\u{2550}",
            width_left = separator_pos - 1,
            width_right = total_width - separator_pos
        );
        println!(
            "{: ^width_left$}{} \u{2502} {}",
            " ",
            modifier_symbol,
            name.display(),
            width_left = separator_pos - 3
        );

        if continue_table {
            print_separator_line(separator_pos, total_width);
        } else {
            print_end_line(separator_pos, total_width);
        }
    }

    fn print_separator_line(separator_pos: usize, total_width: usize) {
        println!(
            "{char:\u{2500}^ln_width$}\u{253C}{char:\u{2500}^total_width$}",
            char = "\u{2500}",
            ln_width = separator_pos - 1,
            total_width = total_width - separator_pos
        );
    }

    fn print_end_line(separator_pos: usize, total_width: usize) {
        println!(
            "{char:\u{2500}^ln_width$}\u{2534}{char:\u{2500}^total_width$}",
            char = "\u{2500}",
            ln_width = separator_pos - 1,
            total_width = total_width - separator_pos
        );
    }

    // get correct deploy and pull configuration
    let target = match env::consts::OS {
        "windows" => config.target.windows,
        "linux" => config.target.linux,
        value => {
            return Err(Error::from_string(format!(
                "Operating system '{value}' is not supported."
            )))
        }
    };

    let to_dir = RelativePathBuf::from(name).to_path(".");
    let dotconfig = to_dir.clone().join(config::CONFIG_PATH);

    // resolve exclude glob patterns
    let mut exclude_patterns = GlobSetBuilder::new();
    config.target.exclude.iter().for_each(|pattern| {
        exclude_patterns.add(Glob::new(pattern.as_str()).unwrap());
    });
    target.exclude.iter().for_each(|pattern| {
        exclude_patterns.add(Glob::new(pattern.as_str()).unwrap());
    });
    let exclude_patterns = exclude_patterns.build().unwrap();

    // check 'file' and 'directory'
    if target.directory.is_some() && target.file.is_some() {
        Err(Error::new(
            "Cannot use both 'directory' and 'file' targets.",
        ))
    } else if let Some(from) = target.file {
        let from_file = PathBuf::from(shellexpand::tilde(&from).into_owned());

        pull_file(
            from_file.parent().unwrap(),
            &from_file,
            name,
            &exclude_patterns,
            force,
        )?;
        Ok(())
    } else if let Some(from) = target.directory {
        let from_dir = PathBuf::from(shellexpand::tilde(&from).into_owned());

        let from_paths = get_paths_in(&from_dir, "**/*")?;
        let to_paths = get_paths_in(&to_dir, "**/*")?;

        // pull file from deployed configuration
        // there are four cases for this:
        //  1) from exists, to exists && unchanged -> do nothing
        //  2) from exists, to exists && modified -> display diff
        //  3) from exists, to doesn't exist -> display addition
        //  4) from doesn't exist, to exists -> display removal
        for from_abs in from_paths {
            pull_file(&from_dir, &from_abs, name, &exclude_patterns, force)?;
        }

        // check for case 4) file was deleted
        for to_abs in to_paths {
            // resolve relative path
            let path_rel = to_abs
                .strip_prefix(&to_dir)
                .map_err(|_| Error::new("could not resolve relative path"))?;
            // get source
            let from_abs = from_dir.join(path_rel);

            if !exclude_patterns.is_match(path_rel) && to_abs.clone() != dotconfig {
                // check if file was deleted
                if !from_abs.exists() {
                    if force {
                        fs::remove_file(to_abs)?;
                        continue;
                    }
                    print_file_name(path_rel, "\x1b[31m-\x1b[0m", 5, 80, false);
                    if prompt_bool("Do you want to continue? ", true) {
                        fs::remove_file(to_abs)?;
                    }
                }
            }
        }

        Ok(())
    } else {
        Err(Error::new("'file' or 'directory' target must be set"))
    }
}

fn pull_file(
    from_dir: &Path,
    from: &Path,
    to: &String,
    exclude: &GlobSet,
    force: bool,
) -> err::Result<()> {
    // pull file from deployed configuration
    // there are four cases for this:
    //  1) from exists, to exists && unchanged -> do nothing
    //  2) from exists, to exists && modified -> display diff
    //  3) from exists, to doesn't exist -> display addition
    //  4) from doesn't exist, to exists -> display removal

    let to_dir = PathBuf::from(to);
    let dotconfig = to_dir.join(config::CONFIG_PATH);
    // resolve relative path
    let path_rel = from
        .strip_prefix(from_dir)
        .map_err(|_| Error::new("could not resolve relative path"))?;
    let from_abs = from;
    // get destination
    let to_abs = to_dir.join(path_rel);
    println!(
        "to: {}, from: {}, rel: {}",
        to_abs.display(),
        from_abs.display(),
        path_rel.display()
    );

    if !exclude.is_match(path_rel) {
        // ensure that we aren't accidentally overwriting the dotconfig
        if to_abs == dotconfig {
            return Err(Error::new("Trying to overwrite dotconfig.toml configuration file. Please add 'dotconfig.toml' to your excludes in the target configuration."));
        }

        // if the file exists, we check if any changes were made to it
        if to_abs.exists() {
            let mut from = File::open(from)?;
            let mut to = File::open(&to_abs)?;

            let mut buf = Vec::new();
            from.read_to_end(&mut buf).unwrap();
            let from_contents = String::from_utf8(buf);
            let mut buf = Vec::new();
            to.read_to_end(&mut buf)?;
            let to_contents = String::from_utf8(buf);

            if let (Ok(from_contents), Ok(to_contents)) = (from_contents, to_contents) {
                // check for case 1) files are the same
                if from_contents == to_contents {
                    return Ok(());
                }

                if force {
                    fs::create_dir_all(to_abs.parent().unwrap())?;
                    fs::copy(from_abs, to_abs)?;
                    return Ok(());
                }

                // case 2) compute diff
                let diff = TextDiff::from_lines(&to_contents, &from_contents);

                // compute the width of the line numbers
                let ln_width = f32::ceil(f32::log10(usize::max(
                    from_contents.lines().count(),
                    to_contents.lines().count(),
                ) as f32)) as usize;
                let separator_pos = ln_width * 2 + 4;
                let total_width = 80;

                // print the file name
                print_file_name(
                    path_rel,
                    "\x1b[36m~\x1b[0m",
                    separator_pos,
                    total_width,
                    true,
                );

                // adapted from https://github.com/mitsuhiko/similar/blob/main/examples/terminal-inline.rs
                for (idx, group) in diff.grouped_ops(2).iter().enumerate() {
                    // print separating line between changes
                    if idx > 0 {
                        print_separator_line(separator_pos, total_width);
                    }

                    // iterate over changes
                    for op in group {
                        for change in diff.iter_inline_changes(op) {
                            let (bright_style, style, sign) = match change.tag() {
                                ChangeTag::Delete => ("\x1b[91m", "\x1b[31m", '-'),
                                ChangeTag::Insert => ("\x1b[92m", "\x1b[32m", '+'),
                                ChangeTag::Equal => ("\x1b[2m", "\x1b[2m", ' '),
                            };

                            // print line numbers
                            print!(
                                    "\x1b[2m{:ln_width$} {:ln_width$} \x1b[0m{style}{}\x1b[0m\u{2502}{style} ",
                                    change
                                        .old_index()
                                        .map_or(String::new(), |idx| idx.to_string()),
                                    change
                                        .new_index()
                                        .map_or(String::new(), |idx| idx.to_string()),
                                        sign,
                                    style=style,
                                    ln_width = ln_width
                                );

                            // print actual changes
                            for (emphasized, value) in change.iter_strings_lossy() {
                                if emphasized {
                                    print!("\x1b[0;3m{}{}", bright_style, &value);
                                } else {
                                    print!("\x1b[0m{}{}", style, &value);
                                }
                            }

                            // reset the style
                            print!("\x1b[0m");

                            // print a final newline if missing
                            if change.missing_newline() {
                                println!();
                            }
                        }
                    }
                }

                // print closing line
                print_end_line(separator_pos, total_width);
            } else {
                // print modification if file could not be read
                print_file_name(path_rel, "\x1b[36m~\x1b[0m", 5, 80, false);
            }
        }
        // case 3) file doesn't exist yet
        else {
            // print addition
            print_file_name(path_rel, "\x1b[32m+\x1b[0m", 5, 80, false);
        }

        // copy the file
        if prompt_bool("Do you want to continue? ", true) {
            fs::create_dir_all(to_abs.parent().unwrap())?;
            fs::copy(from_abs, to_abs)?;
        }
    }
    Ok(())
}

fn print_file_name(
    name: &Path,
    modifier_symbol: &'static str,
    separator_pos: usize,
    total_width: usize,
    continue_table: bool,
) {
    println!(
        "{char:\u{2550}^width_left$}\u{2564}{char:\u{2550}^width_right$}",
        char = "\u{2550}",
        width_left = separator_pos - 1,
        width_right = total_width - separator_pos
    );
    println!(
        "{: ^width_left$}{} \u{2502} {}",
        " ",
        modifier_symbol,
        name.display(),
        width_left = separator_pos - 3
    );

    if continue_table {
        print_separator_line(separator_pos, total_width);
    } else {
        print_end_line(separator_pos, total_width);
    }
}

fn print_separator_line(separator_pos: usize, total_width: usize) {
    println!(
        "{char:\u{2500}^ln_width$}\u{253C}{char:\u{2500}^total_width$}",
        char = "\u{2500}",
        ln_width = separator_pos - 1,
        total_width = total_width - separator_pos
    );
}

fn print_end_line(separator_pos: usize, total_width: usize) {
    println!(
        "{char:\u{2500}^ln_width$}\u{2534}{char:\u{2500}^total_width$}",
        char = "\u{2500}",
        ln_width = separator_pos - 1,
        total_width = total_width - separator_pos
    );
}

/// deploy one or all configs to the local system
fn config_deploy(matches: &ArgMatches, mut structure: Structure) -> err::Result<()> {
    let name: Option<&String> = matches.get_one("name");
    let all = matches.get_flag("all");

    if let Some(name) = name {
        if all {
            return Err(Error::new("You cannot use the all flag in combination with a specific configuration. Try removing \"--all\" or the configuration name."));
        }
        let config = structure.configs.remove(name);
        match config {
            Some(config) => deploy_single(&String::from(name), config),
            None => Err(Error::from_string(format!(
                "Config '{name}' does not exist."
            ))),
        }
    } else if all {
        for (name, config) in structure.configs {
            match deploy_single(&name, config) {
                Ok(_) => {}
                Err(error) => println!("Could not deploy config '{}': {}", name, error),
            }
        }
        Ok(())
    } else {
        Err(Error::new("No configurations matched the query."))
    }
}

fn deploy_single(name: &String, config: Configuration) -> err::Result<()> {
    let target = match env::consts::OS {
        "windows" => config.target.windows,
        "linux" => config.target.linux,
        value => {
            return Err(Error::from_string(format!(
                "Operating system '{value}' is not supported."
            )))
        }
    };

    let target_path = PathBuf::from(shellexpand::tilde(&target.directory.unwrap()).into_owned());

    // checks if the target directory already has files in it
    match &target.require_empty {
        Some(value) => {
            if *value {
                check_dir_null_or_empty(&target_path)?;
            }
        }
        None => {
            if config.target.require_empty {
                check_dir_null_or_empty(&target_path)?;
            }
        }
    }
    // create target
    fs::create_dir_all(&target_path)?;

    // the source directoy
    let config_dir = RelativePathBuf::from(name).to_path(".");
    let dotconfig = config_dir.join(config::CONFIG_PATH);

    let mut exclude_patterns = GlobSetBuilder::new();
    config.target.exclude.iter().for_each(|pattern| {
        exclude_patterns.add(Glob::new(pattern.as_str()).unwrap());
    });
    target.exclude.iter().for_each(|pattern| {
        exclude_patterns.add(Glob::new(pattern.as_str()).unwrap());
    });
    let exclude_patterns = exclude_patterns.build().unwrap();

    // copy files to target
    for from in get_paths_in(&config_dir, "**/*")? {
        let to = target_path.join(
            from.strip_prefix(&config_dir)
                .map_err(|_| Error::new("could not resolve relative path"))?,
        );

        if !(exclude_patterns.is_match(&from) || dotconfig == from) {
            fs::create_dir_all(to.parent().unwrap())?;
            fs::copy(from, to);
        }
    }

    Ok(())
}

fn get_paths_in(dir: &Path, pattern: &str) -> err::Result<Vec<PathBuf>> {
    let glob = Glob::new(dir.join(pattern).to_str().unwrap())
        .unwrap()
        .compile_matcher();

    return list_dir(&glob, dir);

    fn list_dir(glob: &GlobMatcher, dir: &Path) -> err::Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        for value in WalkDir::new(dir) {
            match value {
                Ok(value) => {
                    let path = value.path();

                    if path.is_file() && glob.is_match(path) {
                        paths.push(path.into());
                    }
                }
                Err(error) => return Err(Error::new("walkdir error")),
            }
        }

        Ok(paths)
    }
}
