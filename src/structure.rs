use std::collections::{HashMap, HashSet};

use crate::{
    config::{self, read_configuration, read_root_configuration, Configuration, RootConfiguration},
    err,
    io::{is_root_present, list_root},
};

#[derive(Debug)]
pub struct Structure {
    pub root: RootConfiguration,
    pub configs: HashMap<String, Configuration>,
}

impl Structure {
    pub fn resolve() -> err::Result<Option<Self>> {
        if is_root_present() {
            let root = read_root_configuration().unwrap();

            let mut exclude = HashSet::new();
            root.exclude.iter().for_each(|p| {
                let mut p = p.clone();
                if p.ends_with('/') {
                    p.remove(p.len() - 1);
                }
                exclude.insert(p);
            });

            let mut configs = HashMap::new();

            for path in list_root().unwrap() {
                let path = path.unwrap().path();
                let key = path.file_name().unwrap().to_str().unwrap().to_string();

                if path.is_dir() && !exclude.contains(&key) {
                    let config = read_configuration(&path.join(config::CONFIG_PATH)).unwrap();
                    configs.insert(key, config);
                }
            }

            return Ok(Some(Structure { root, configs }));
        }
        Ok(None)
    }
}
