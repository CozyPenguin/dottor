use std::io::{stdin, stdout, Write};

use path_abs::{FileRead, FileWrite, ListDir, PathAbs, PathDir, PathFile, PathInfo};

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
    match write.write_all(contents) {
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

    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();
    if input.trim().len() == 0 {
        default
    } else {
        input.to_lowercase().trim() == "y"
    }
}
