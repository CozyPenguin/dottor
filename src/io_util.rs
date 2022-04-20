use std::{
    env,
    fs::{ReadDir, self},
    io::{self, stdin, stdout, Read, Write}, path::Path,
};

use crate::config::{Configuration};

pub fn list_root() -> io::Result<ReadDir> {
    env::current_dir()?.read_dir()
}

pub fn check_empty(dir: &str) -> io::Result<()> {
    let mut path = env::current_dir()?;
    path.push(dir);
    if path.read_dir()?.next().is_none() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Directory isn't empty.",
        ))
    }
}

pub fn check_dir_null_or_empty(dir: &str) -> io::Result<()> {
    if dir_exists(dir)? {
        check_empty(dir)?;
    }
    Ok(())
}

pub fn check_dir_exists(dir: &str) -> io::Result<()> {
    match dir_exists(dir) {
        Ok(value) => {
            if value {
                Ok(())
            } else { Err(io::Error::new(io::ErrorKind::NotFound, "Directory doesn't exist."))
            }
        }
        Err(error) => Err(error),
    }
}

pub fn dir_exists(dir: &str) -> io::Result<bool> {
    let mut path = env::current_dir()?;
    path.push(dir);
    return Ok(path.exists() && path.is_dir());
}

pub fn file_exists(file: &str) -> io::Result<bool> {
    let mut path = env::current_dir()?;
    path.push(file);
    return Ok(path.exists() && path.is_file());
}

pub fn parse_config(file: &Path) -> io::Result<Configuration> {
    let source = fs::read_to_string(file)?;
    let config = toml::from_str(&source[..])?;
    return Ok(config);
}

pub fn prompt_bool(message: &str, default: bool) -> bool {
    if default {
        print!("{message} Proceed? [Y/n]: ");
    } else {
        print!("{message} Proceed? [y/N]: ");
    }

    stdout().flush().ok();

    let input = stdin().bytes().next().and_then(|result| result.ok());
    if let Some(char) = input {
        return char == b'y' || char == b'Y';
    }
    return false;
}
