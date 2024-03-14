use std::{
    env::current_dir,
    fmt::format,
    fs::{self, File, ReadDir},
    io::{stdin, stdout, Read, Write},
    path::Path,
};

use relative_path::{RelativePath, RelativePathBuf};
use walkdir::WalkDir;

use crate::{
    config,
    err::{self, Error},
};

pub fn is_root_present() -> bool {
    RelativePathBuf::from(config::ROOT_PATH)
        .to_path(".")
        .is_file()
}

pub fn check_root_present() -> err::Result<()> {
    if is_root_present() {
        Ok(())
    } else {
        Err(Error::from_string(format!(
            "Directory doesn't contain root config '{}'.",
            config::ROOT_PATH
        )))
    }
}

pub fn list_root() -> err::Result<ReadDir> {
    Ok(current_dir()?.read_dir()?)
}

/// ensures that the passed directory is empty
pub fn assert_empty(dir: &Path) -> err::Result<()> {
    if !dir.is_dir() {
        return Err(Error::from_string("Directory doesn't exist".into()));
    }
    if dir.read_dir().unwrap().next().is_none() {
        Ok(())
    } else {
        Err(Error::from_string(format!(
            "Directory '{}' isn't empty.",
            dir.display()
        )))
    }
}

/// ensures that the passed directory doesn't exist or is empty
pub fn check_dir_null_or_empty(dir: &Path) -> err::Result<()> {
    if dir.is_dir() {
        assert_empty(&dir)?;
    }
    Ok(())
}

/// ensures that the passed path is a valid directory
pub fn check_valid_dir(dir: &Path) -> err::Result<()> {
    if dir.is_dir() {
        Ok(())
    } else {
        Err(Error::from_string(format!(
            "'{}' is not a valid directory",
            dir.display()
        )))
    }
}

pub fn write(path: &Path, contents: &[u8]) -> err::Result<()> {
    let mut write = File::create(path)?;
    match write.write_all(contents) {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::from_string(format!(
            "Could not write to file '{}'",
            path.display()
        ))),
    }
}

pub fn copy_dir(from: &Path, to: &Path) -> err::Result<()> {
    for entry in WalkDir::new(from) {
        let entry = entry.unwrap();
        let path = entry.path();
        let relative_path = RelativePath::from_path(path.strip_prefix(&from).unwrap()).unwrap();

        if path.is_file() {
            fs::copy(path, relative_path.to_path(to));
        } else if path.is_dir() {
            fs::create_dir_all(relative_path.to_path(to));
        }
    }

    Ok(())
}

pub fn read_to_string(file: &Path) -> err::Result<String> {
    let mut read = File::open(file)
        .map_err(|_| Error::from_string(format!("Could not read open file {}", file.display())))?;
    let mut str = String::new();
    read.read_to_string(&mut str)?;

    Ok(str)
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
