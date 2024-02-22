use std::{
    collections::HashMap,
};

use path_abs::{PathAbs, PathFile, PathInfo, PathOps};

use crate::{
    config::{self, read_configuration, read_root_configuration, Configuration, RootConfiguration},
    err,
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

                let exclude = root
                    .exclude
                    .iter()
                    .map(PathAbs::new)
                    .into_iter()
                    .collect::<Result<Vec<PathAbs>, path_abs::Error>>()?;

                let mut configs = HashMap::new();

                for path in list_root()? {
                    let path = path?;
                    let key = path.file_name().unwrap().to_str().unwrap().to_string();

                    if path.is_dir() && !exclude.contains(&path.to_owned().into()) {
                        let config =
                            read_configuration(&PathFile::new(path.concat(config::CONFIG_PATH)?)?)?;
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
