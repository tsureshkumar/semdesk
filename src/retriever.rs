// Copyright: (c) 2023 Sureshkumar T
// License: Apache-2.0

use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::collections::HashSet;
use std::error::Error as StdError;
use std::sync::Arc;

use crate::parsers::Parser;
use crate::error::Error;
use crate::catalog::Catalog;
use crate::indexer;
use crate::catalog;



#[derive(Debug)]
pub enum Message {
    RetrieveByQuery(String, Sender<Reply>),
    RetrieveByPath(String, Sender<Vec<String>>),
    RetrieveById(u64, Sender<Vec<String>>),
}

pub enum Reply {
    Docs(Vec<(String, String)>),
}


pub trait Retriever {
    fn new(catalog: Arc<Catalog>, indexer_channel: Sender<indexer::Message>) -> Self;
    fn retrieve(&self, query: String) -> Vec<(String,String)>;
    fn retrieve_by_path(&self, path: String) -> Vec<String>;
    fn retrieve_by_id(&self, id: u64) -> Vec<String>;
    fn run(&mut self);
}

pub struct RetrieverImpl {
    retriever_channel: (Sender<Message>, Receiver<Message>),
    indexer_channel: Sender<indexer::Message>,
    catalog: Arc<Catalog>,
}

impl RetrieverImpl {
    pub fn new(catalog: Arc<Catalog>, indexer_channel: Sender<indexer::Message>) -> Self {
        RetrieverImpl {
            retriever_channel: channel(),
            indexer_channel,
            catalog,
        }
    }

    pub fn get_sender(&self) -> Sender<Message> {
        self.retriever_channel.0.clone()
    }

}

impl Retriever for RetrieverImpl {
    fn new(catalog: Arc<Catalog>, indexer_channel: Sender<indexer::Message>) -> Self {
        RetrieverImpl {
            retriever_channel: channel(),
            indexer_channel,
            catalog,
        }
    }

    fn retrieve(&self, query: String) -> Vec<(String, String)> {
        let mut results: Vec<(String, String)> = Vec::new();

        let mut files_set = HashSet::new();
        let ch = channel();
        self.indexer_channel.send(indexer::Message::RetrieveDocument(query, ch.0)).unwrap();
        if let Ok(rep) = ch.1.recv() {
            match rep {
                indexer::Reply::Docs(ids_scores) => {
                    for (id,score) in ids_scores {
                        log::debug!("Retriever: id: {}, score: {}", id, score);
                        let rep = self.catalog.get_by_index(id);
                        log::debug!("Retriever: rep: {:?}", rep);
                        if let Ok(catalog::Entry { filename, .. }) = rep {
                            files_set.insert(filename);
                        }
                    }
                }
                _ => {}
            }
        }

        for file in files_set {
            let catalog = self.catalog.clone();
            log::debug!("Retriever: file: {:?}", file);
            if !catalog.is_file_in_catalog(file.to_string()) {
                continue;
            }
            let content: Result<String, Box<dyn StdError>> = Parser::new().parse(&file);
            if let Ok(content) = content {
                results.push((file, content));
            } else {
                log::debug!("Retriever: error: {:?}", content);
            }
        }
        results
    }

    fn run(&mut self) {
        loop {
            let query = self.retriever_channel.1.recv().unwrap();
            log::debug!("Retriever received query: {:?}", query);
            match query {
                Message::RetrieveByQuery(query, sender) => {
                    let results = self.retrieve(query);
                    log::debug!("Retriever retrieved results: {}", results.len());
                    sender.send(Reply::Docs(results)).unwrap();
                }
                Message::RetrieveByPath(path, sender) => {
                    let results = self.retrieve_by_path(path);
                    sender.send(results).unwrap();
                }
                Message::RetrieveById(id, sender) => {
                    let results = self.retrieve_by_id(id);
                    sender.send(results).unwrap();
                }
            }
        }
    }

    fn retrieve_by_path(&self, path: String) -> Vec<String> {
        let mut results = Vec::new();
        let content = Parser::new().parse(&path);
        if content.is_ok() {
            results.push(content.unwrap());
        }
        results
    }

    fn retrieve_by_id(&self, id: u64) -> Vec<String> {
        let mut results = Vec::new();
        let rep = self.catalog.get_by_index(id);
        if let Ok(catalog::Entry { filename, .. }) = rep {
            let content = Parser::new().parse(&filename);
            if content.is_ok() {
                results.push(content.unwrap());
            }
        }
        results
    }
}
