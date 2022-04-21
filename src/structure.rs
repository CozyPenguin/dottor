use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    config::{self, read_configuration, read_root_configuration, Configuration, RootConfiguration},
    err::{self, Error},
    io_util::{is_root_present, list_root},
};

#[derive(Debug)]
pub struct Structure {
    pub root: RootConfiguration,
    pub configs: HashMap<String, Configuration>,
}

impl Structure {
    pub fn resolve() -> err::Result<Option<Self>> {
        match is_root_present() {
            Ok(true) => {
                let root = read_root_configuration().unwrap();

                let mut configs = HashMap::new();
                for path in list_root()? {
                    let path = path.map_err(|_| Error::new("Invalid path encountered"))?;
                    if Path::is_dir(&path.path())
                        && !root.exclude.iter().any(|p| {
                            fs::canonicalize(PathBuf::from(p)).unwrap()
                                == fs::canonicalize(path.path()).unwrap()
                        })
                    {
                        let mut path = path.path();
                        let key = path.file_name().unwrap().to_str().unwrap().to_string();
                        path.push(config::CONFIG_PATH);
                        let config = read_configuration(&path)?;
                        configs.insert(key, config);
                    }
                }

                Ok(Some(Structure {
                    root: root,
                    configs: configs,
                }))
            }
            _ => Ok(None),
        }
    }
}
