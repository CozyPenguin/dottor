use anyhow::{anyhow, Context, Result};
use std::{
    env::current_dir,
    error,
    fmt::Display,
    fs::{self, File, ReadDir},
    io::{self, stdin, stdout, Read, Write},
    path::{Path, PathBuf},
};

use relative_path::{RelativePath, RelativePathBuf};
use walkdir::WalkDir;

use crate::config;

#[derive(Debug, Clone)]
pub enum ExpectedType {
    File,
    Directory,
}

impl ExpectedType {
    fn invert(&self) -> Self {
        match self {
            ExpectedType::File => ExpectedType::Directory,
            ExpectedType::Directory => ExpectedType::File,
        }
    }
}

impl Display for ExpectedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ExpectedType::File => "file",
                ExpectedType::Directory => "directory",
            }
        )
    }
}

#[derive(Debug)]
pub enum IOError {
    /// A path has a different type than excepted.
    IncorrectType(ExpectedType),
    MissingRoot,
    DirectoryNotEmpty(PathBuf),
    IO(io::Error),
}

impl error::Error for IOError {}

impl Display for IOError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IOError::IncorrectType(expected) => {
                write!(f, "Path doesn't lead to a {}", expected)
            }
            IOError::MissingRoot => write!(
                f,
                "Directory doesn't contain root configuration '{}'",
                config::ROOT_PATH
            ),
            IOError::IO(err) => write!(f, "{}", err),
            IOError::DirectoryNotEmpty(name) => {
                write!(f, "Directory '{}' is not empty.", name.display())
            }
        }
    }
}

impl From<io::Error> for IOError {
    fn from(value: io::Error) -> Self {
        IOError::IO(value)
    }
}

/// Checks if the root configuration is present in the current directory.
pub fn is_root_present() -> bool {
    RelativePathBuf::from(config::ROOT_PATH)
        .to_path(".")
        .is_file()
}

/// Asserts that the root configuration is present in the current directory.
/// # Errors
/// Returns an error if no root configuration was found in the current working directory.
pub fn assert_root_present() -> Result<()> {
    if is_root_present() {
        Ok(())
    } else {
        Err(IOError::MissingRoot.into())
    }
}

/// Returns an iterator over all items in the root directory
pub fn list_root() -> Result<ReadDir> {
    Ok(current_dir()?.read_dir()?)
}

/// Ensures that the passed directory is empty.
pub fn assert_empty(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        Err(IOError::IncorrectType(ExpectedType::Directory).into())
    } else if dir.read_dir()?.next().is_none() {
        Ok(())
    } else {
        Err(IOError::DirectoryNotEmpty(dir.into()).into())
    }
}

/// Ensures that the passed directory doesn't exist or is empty
pub fn check_dir_null_or_empty(dir: &Path) -> Result<()> {
    if dir.is_dir() {
        assert_empty(&dir)?;
    }
    Ok(())
}

/// Ensures that the passed path is a valid directory
pub fn check_valid_dir(dir: &Path) -> Result<()> {
    if dir.is_dir() {
        Ok(())
    } else {
        Err(anyhow!("'{}' is not a valid directory", dir.display()))
    }
}

pub fn write(path: &Path, contents: &[u8]) -> Result<()> {
    let mut write = File::create(path)?;
    match write.write_all(contents) {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!(format!(
            "Could not write to file '{}'",
            path.display()
        ))),
    }
}

pub fn copy_dir(from: &Path, to: &Path) -> Result<()> {
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

pub fn read_to_string(file: &Path) -> Result<String> {
    let mut read =
        File::open(file).with_context(|| format!("Could not read open file {}", file.display()))?;
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
