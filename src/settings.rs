// Copyright: (c) 2023 Sureshkumar T
// License: Apache-2.0

use std::error::Error;
use dirs;
use lazy_static::lazy_static;
use std::path::PathBuf;
use config::{Config, File, Environment};


pub struct Settings {
    pub files: Vec<String>,
    pub max_scan_depth: u32,
    pub db_dir: String,
    pub scan_status_file: &'static str,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            files: vec![],
            max_scan_depth: 2,
            db_dir: String::from("~/.local/share/semdesk/db"),
            scan_status_file: "~/.local/share/semdesk/scan_status.txt",
        }
    }
}

pub fn get_config_dir() -> String {
    let home = dirs::home_dir().unwrap();
    let config_dir = home.join(".config/semdesk");
    config_dir.to_str().unwrap().to_string()
}

pub fn get_db_dir() -> String {
    let dir = CONFIG.db_dir.clone();
    if dir.starts_with("~") {
        let home = dirs::home_dir().unwrap();
        let db_dir = home.join(&dir[2..]);
        log::debug!("DB dir: {}", db_dir.to_str().unwrap());
        db_dir.to_str().unwrap().to_string()
    } else {
        dir
    }
}

impl Settings {
    fn new() -> Result<Settings, Box<dyn Error>> {
        let config_dir = PathBuf::from(get_config_dir());
        let config_file = config_dir.join("config.toml");
        let mut config = config::Config::default();
        if config_file.exists() {
            config.merge(config::File::with_name(config_file.to_str().unwrap()))?;
            let files: Vec<String> = config.get("crawler.files")?;
            let max_scan_depth: u32 = config.get("crawler.max_scan_depth").unwrap_or(2);
            let db_dir: String = config.get("db.dir").unwrap_or(String::from("~/.local/share/semdesk/db"));
            let scan_status_file = config.get("crawler.scan_status_file").unwrap_or("~/.local/share/semdesk/scan_status.txt");
            Ok(Settings { files, max_scan_depth, db_dir, scan_status_file })
        } else {
            Ok(Settings::default())
        }
    }
}

lazy_static! {
    pub static ref CONFIG: Settings = Settings::new().unwrap();
}

pub fn get_config() -> &'static Settings {
    &CONFIG
}

pub fn get_scan_status_file() -> String {
    let file = CONFIG.scan_status_file.clone();
    if file.starts_with("~") {
        let home = dirs::home_dir().unwrap();
        let scan_status_file = home.join(&file[2..]);
        log::debug!("Scan status file: {}", scan_status_file.to_str().unwrap());
        scan_status_file.to_str().unwrap().to_string()
    } else {
        file.to_string()
    }
}

pub fn get_socket_path() -> String {
    let userdir = dirs::home_dir().unwrap();
    let sock_path = userdir.join(".local/share/semdesk.sock");
    sock_path.to_str().unwrap().to_string()
}
