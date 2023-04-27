// Copyright: (c) 2023 Sureshkumar T
// License: Apache-2.0

use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use dirs;
use std::path::PathBuf;
use std::sync::Arc;
use std::error::Error as StdError;
use log;
use chrono;

use crate::settings;
use crate::catalog::Catalog;
use crate::catalog;
use crate::parsers::Parser;
use crate::indexer;

pub trait Crawler {
    fn new(catalog: Arc<Catalog>, indexer_ch: Sender<indexer::Message>) -> Self;
    fn scan(&self);
    fn run(&mut self);
}

pub struct CrawlerImpl {
    catalog: Arc<Catalog>,
    indexer_channel: Sender<indexer::Message>,
}


impl CrawlerImpl {
    fn is_file_indexed(&self, filename: String) -> bool {
        return self.catalog.is_file_in_catalog(filename);
    }

    fn scan_file(&self, filename: String, depth: u32) {
        if depth > settings::get_config(None).max_scan_depth {
            return;
        }
        if self.is_file_indexed(filename.clone()) {
            return;
        }
        if PathBuf::from(&filename).is_file() {
            // don't index if size is greater than 10 MB
            let metadata = std::fs::metadata(&filename).unwrap();
            if metadata.len() > 10 * 1024 * 1024 {
                return;
            }

            // don't index if file is hidden
            // take the filename part from the path
            let filenameonly = PathBuf::from(&filename).file_name().unwrap().to_str().unwrap().to_string();
            if filenameonly.starts_with(".") {
                return;
            }
            
            log::debug!("Indexing: {}", filename);
            let content: Result<String, Box<dyn StdError>> = Parser::new().parse(&filename);
            if let Err(e) = content {
                log::debug!("Error: {}", e);
                return;
            }
            let content = content.unwrap();
            let ch = channel();
            self.indexer_channel.send(indexer::Message::AddDocument(content, 0, filename, ch.0)).unwrap();
            if let Ok(rep) = ch.1.recv() {
                match rep {
                    indexer::Reply::Done(filename, ids) => {
                        log::debug!("File indexed: {}", filename);
                        self.catalog.add(catalog::Entry::new(filename, ids) );
                    }
                    _ => {}
                }
            }
        } else if PathBuf::from(&filename).is_dir() {
            for entry in std::fs::read_dir(filename).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                let fname = path.to_str().unwrap().to_string();
                self.scan_file(fname, depth + 1);
            }
        }
    }
}

impl Crawler for CrawlerImpl {
    fn new(catalog: Arc<Catalog>, indexer_channel: Sender<indexer::Message>) -> Self {
        let mut obj = CrawlerImpl {
            catalog,
            indexer_channel,
        };
        obj
    }

    fn scan(&self) {
        let files = settings::get_config(None).files.clone();
        for filename in files {
            let mut fname = filename.clone();
            if fname.starts_with("~") {
                let home = dirs::home_dir().unwrap();
                fname = home.to_str().unwrap().to_string() + &filename[1..];
            }
            log::debug!("Scanning: {}", fname);
            if PathBuf::from(&fname).is_dir() {
                for entry in std::fs::read_dir(fname.clone()).unwrap() {
                    log::debug!("Dir listing: {:?} {:?}", fname, entry);
                    let entry = entry.unwrap();
                    let path = entry.path();
                    let fname = path.to_str().unwrap().to_string();
                    self.scan_file(fname, 0);
                }
            } else {
                self.scan_file(fname, 0);
            }
        }
    }

    fn run(&mut self) {
        // run scan at midnight between 2 to 4 am
        let now = chrono::Local::now();
        let midnight = now.date().and_hms(0, 0, 0);
        let mut next_scan = midnight + chrono::Duration::days(1);
        if now > midnight + chrono::Duration::hours(2) && now < midnight + chrono::Duration::hours(4) {
            self.scan();
            next_scan = now + chrono::Duration::days(1);
        }


        loop {
            let now = chrono::Local::now();

            let last_scan_file: String = settings::get_scan_status_file();
            if !PathBuf::from(&last_scan_file).exists() {
                std::fs::write(last_scan_file.clone(), (now - chrono::Duration::days(2)).to_rfc3339()).unwrap();
            }
            let last_scan_time_str = std::fs::read_to_string(last_scan_file.clone());
            let last_scan_time: chrono::DateTime<chrono::FixedOffset> = match last_scan_time_str {
                Ok(s) => chrono::DateTime::parse_from_rfc3339(&s).unwrap(),
                Err(_) =>  {
                    let a_day_b4 = chrono::Local::now() - chrono::Duration::days(2);
                    a_day_b4.with_timezone(a_day_b4.offset())
                }
            };

            log::info!("Last scan: {}", last_scan_time.to_rfc3339());
            if now > next_scan  || now > last_scan_time + chrono::Duration::days(1){
                log::info!("Scanning...");
                self.scan();
                std::fs::write(last_scan_file, now.to_rfc3339()).unwrap();
                next_scan = now + chrono::Duration::days(1);
            }
            thread::sleep(Duration::from_secs(60*5));
        }
    }

}
