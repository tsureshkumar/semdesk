// Copyright: (c) 2023 Sureshkumar T
// License: Apache-2.0

use sled;
use serde::{Serialize, Deserialize};
use std::sync::Mutex;


use crate::error::Error;
use crate::settings;

pub struct Catalog {
    db: Option<sled::Db>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    pub filename: String,
    indexes: Vec<u64>,
}

impl Entry {
    pub fn new(filename: String, indexes: Vec<u64>) -> Self {
        Entry { filename, indexes }
    }
}

impl Catalog {
    pub fn new() -> Self {
        let db_dir = settings::get_db_dir();
        log::info!("Opening catalog at {}", db_dir);
        let db = sled::open(db_dir).unwrap();
        Catalog { db: Some(db) }
    }

    pub fn add(&self, entry: Entry) {
        log::debug!("Adding entry: {:?}", entry);
        let db = self.db.as_ref().unwrap();
        let mut entry = entry;
        let mut filename2 = entry.filename.clone();
        if filename2.starts_with("/") {
            filename2 = filename2[1..].to_string();
        }
        let key = std::fmt::format(format_args!("/byfile/{}", filename2));
        let value: String = serde_json::to_string(&entry).unwrap();
        log::debug!("key: {}, value: {}", key, value);
        db.insert(key, value.as_bytes()).unwrap();

        for index in entry.indexes {
            let key = std::fmt::format(format_args!("/byindex/{}", index));
            db.insert(key, value.as_bytes()).unwrap();
        }
    }

    pub fn get_by_file(&self, filename: String) -> Result<Entry, Error> {
        let db = self.db.as_ref().unwrap();
        let mut filename2 = filename.clone();
        if filename2.starts_with("/") {
            filename2 = filename2[1..].to_string();
        }
        let key = std::fmt::format(format_args!("/byfile/{}", filename2));
        let value = db.get(key).unwrap();
        let entry = serde_json::from_str(std::str::from_utf8(value.unwrap().as_ref()).unwrap()).unwrap();
        Ok(entry)
    }

    pub fn get_by_index(&self, index: u64) -> Result<Entry, Error> {
        let db = self.db.as_ref().unwrap();
        let key = std::fmt::format(format_args!("/byindex/{}", index));
        if !db.contains_key(key.clone()).unwrap() {
            return Err(Error::new("Index not found"));
        }
        let value = db.get(key).unwrap();
        let entry = serde_json::from_str(std::str::from_utf8(value.unwrap().as_ref()).unwrap()).unwrap();
        Ok(entry)
    }

    pub fn delete(&self, filename: String) {
        let db = self.db.as_ref().unwrap();
        let mut filename2 = filename.clone();
        if filename2.starts_with("/") {
           filename2 = filename[1..].to_string();
        }
        let key = std::fmt::format(format_args!("/byfile/{}", filename2));
        let entry = self.get_by_file(filename).unwrap();
        db.remove(key).unwrap();
        for index in entry.indexes {
            let key = std::fmt::format(format_args!("/byindex/{}", index));
            db.remove(key).unwrap();
        }
    }

    pub fn is_file_in_catalog(&self, filename: String) -> bool {
        let db = self.db.as_ref().unwrap();
        let mut filename2 = filename.clone();
        if filename2.starts_with("/") {
           filename2 = filename2[1..].to_string();
        }
        let key = std::fmt::format(format_args!("/byfile/{}", filename2));
        let res = db.contains_key(key.clone()).unwrap();
        res

    }

    pub fn gen_id(&self) -> u64 {
        // generate unique id from sled
        let db = self.db.as_ref().unwrap();
        let idg = db.generate_id().unwrap();
        idg
    }
}
