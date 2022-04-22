use std::{
    env,
    io::{stdin, stdout, Read, Write},
    path::Path,
};

use path_abs::{FileWrite, PathAbs, PathDir, PathFile, PathInfo, FileRead, ListDir};

use crate::{
    config,
    err::{self, Error},
};

pub fn is_root_present() -> err::Result<bool> {
    Ok(PathAbs::new(config::ROOT_PATH)?.exists())
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

pub fn list_root() -> err::Result<ListDir> {
    Ok(PathDir::current_dir()?.list()?)
}

pub fn set_current_dir<P: AsRef<Path>>(path: P) -> err::Result<()> {
    env::set_current_dir(path).map_err(|_| Error::new("Could not change directory."))
}

/// ensures that the passed directory is empty
pub fn check_not_empty(dir: &PathDir) -> err::Result<()> {
    if dir.list()?.next().is_none() {
        Ok(())
    } else {
        Err(Error::from_string(format!(
            "Directory '{}' isn't empty.",
            dir.display()
        )))
    }
}

/// ensures that the passed directory doesn't exist or is empty
pub fn check_dir_null_or_empty(dir: &PathAbs) -> err::Result<()> {
    if dir.is_dir() {
        check_not_empty(&PathDir::new(dir)?)?;
    }
    Ok(())
}

/// ensures that the passed path is a valid directory
pub fn check_valid_dir(dir: &PathAbs) -> err::Result<()> {
    if dir.is_dir() {
        Ok(())
    } else {
        Err(Error::from_string(format!(
            "'{}' is not a valid directory",
            dir.display()
        )))
    }
}

pub fn write(path: &PathAbs, contents: &[u8]) -> err::Result<()> {
    let mut write = FileWrite::create(path)?;
    match write.write(contents) {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::from_string(format!(
            "Could not write to file '{}'",
            path.display()
        ))),
    }
}

pub fn read_to_string(file: &PathFile) -> err::Result<String> {
    let mut read = FileRead::open(file)?;
    Ok(read.read_string()?)
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
