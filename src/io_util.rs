use std::{
    env,
    fs::{self, ReadDir},
    io::{stdin, stdout, Read, Write},
    path::{Path, PathBuf},
    thread::current,
};

use crate::{
    config,
    err::{self, Error},
};

pub fn is_root_present() -> err::Result<bool> {
    file_exists(config::ROOT_PATH)
}

pub fn check_root_present() -> err::Result<()> {
    if is_root_present()? {
        Ok(())
    } else {
        Err(Error::from_string(format!(
            "Directory doesn't contain root config '{}'.",
            config::ROOT_PATH
        )))
    }
}

pub fn list_root() -> err::Result<ReadDir> {
    current_dir()?
        .read_dir()
        .map_err(|_| Error::new("Could not read contents of root directory."))
}

pub fn current_dir() -> err::Result<PathBuf> {
    env::current_dir().map_err(|_| Error::new("Failed to resolve current directory. "))
}

pub fn set_current_dir<P: AsRef<Path>>(path: P) -> err::Result<()> {
    env::set_current_dir(path).map_err(|_| Error::new("Could not change directory."))
}

pub fn read_dir(dir: &PathBuf) -> err::Result<ReadDir> {
    dir.read_dir()
        .map_err(|_| Error::new("Failed to read contents of current directory. "))
}

/// ensures that the passed directory is empty
pub fn check_empty<P: AsRef<Path>>(dir: P) -> err::Result<()> {
    let mut path = current_dir()?;
    path.push(dir);
    if read_dir(&path)?.next().is_none() {
        Ok(())
    } else {
        Err(Error::from_string(format!("Directory isn't empty.")))
    }
}

/// ensures that the passed directory doesn't exist or is empty
pub fn check_dir_null_or_empty<P: AsRef<Path>>(dir: P) -> err::Result<()> {
    if is_dir(dir.as_ref())? {
        check_empty(dir)?;
    }
    Ok(())
}

/// ensures that the passed path is a valid directory
pub fn check_valid_dir<P: AsRef<Path>>(dir: P) -> err::Result<()> {
    match is_dir(dir) {
        Ok(value) => {
            if value {
                Ok(())
            } else {
                Err(Error::new("Directory not valid."))
            }
        }
        Err(error) => Err(error),
    }
}

pub fn is_dir<P: AsRef<Path>>(dir: P) -> err::Result<bool> {
    let mut path = current_dir()?;
    path.push(dir);
    return Ok(path.exists() && path.is_dir());
}

pub fn file_exists<P: AsRef<Path>>(file: P) -> err::Result<bool> {
    let mut path = current_dir()?;
    path.push(file);
    return Ok(path.exists() && path.is_file());
}

pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> err::Result<()> {
    fs::write(path, contents).map_err(|_| Error::from_string(format!("Could not write to path")))
}

pub fn read_to_string<P: AsRef<Path>>(file: P) -> err::Result<String> {
    fs::read_to_string(file).map_err(|_| Error::new("Could not read file."))
}

pub fn create_dir_all<P: AsRef<Path>>(dir: P) -> err::Result<()> {
    fs::create_dir_all(dir).map_err(|_| Error::from_string(format!("Could not create directory")))
}

pub fn remove_dir_all(dir: &Path) -> err::Result<()> {
    fs::remove_dir_all(dir)
        .map_err(|_| Error::from_string(format!("Could not remove directory '{}'", dir.display())))
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
