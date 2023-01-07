use core::fmt;
use std::{
    fs::File,
    io::{BufReader, BufWriter},
};

use bincode::{deserialize_from, serialize_into};

use crate::parsing::ScpObject;

const CACHE_O_PATH: &str = "cache_o.data";

pub fn cache_objects(objects: Vec<ScpObject>) {
    let path = std::env::current_dir()
        .unwrap()
        .as_path()
        .join(CACHE_O_PATH);
    let mut f = BufWriter::new(File::create(path).unwrap());
    serialize_into(&mut f, &objects).unwrap();
}
#[derive(Debug, PartialEq)]
pub enum CacheError {
    FileCacheNotExists,
}

impl std::error::Error for CacheError {}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CacheError::FileCacheNotExists => write!(f, "Cache not exists"),
        }
    }
}

pub fn decache_objects() -> Result<Vec<ScpObject>, CacheError> {
    let path = std::env::current_dir()
        .unwrap()
        .as_path()
        .join(CACHE_O_PATH);
    let o = File::open(path);

    match o {
        Ok(o) => {
            let f = BufReader::new(o);
            let objects: Vec<ScpObject> = deserialize_from(f).unwrap();
            Ok(objects)
        }
        Err(_) => Err(CacheError::FileCacheNotExists),
    }
}
