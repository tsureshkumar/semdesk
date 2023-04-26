// Copyright: (c) 2023 Sureshkumar T
// License: Apache-2.0


use faiss::{ index_factory, MetricType};
use faiss::index::IndexImpl;
use faiss::Index;
use faiss;
use rust_bert::pipelines::sentence_embeddings::SentenceEmbeddingsBuilder;
use rust_bert::pipelines::sentence_embeddings::SentenceEmbeddingsModelType;
use rust_bert::pipelines::sentence_embeddings::SentenceEmbeddingsModel;
use sha256;
use faiss::Idx;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::sync::Arc;
use ndarray::prelude::*;
use ndarray::{Array, Array1, Array2, ArrayView1, ArrayView2, Axis};
use std::time::Duration;
use std::thread;

use crate::idgenerator::IdGenerator;
use crate::catalog::Catalog;


#[link(name = "faiss")]
extern "C" {
    #[doc = " L2-renormalize a set of vector. Nothing done if the vector is 0-normed"]
    pub fn faiss_fvec_renorm_L2(d: usize, nx: usize, x: *mut f32);
}

pub fn renorm_L2(d: usize, nx: usize, x: *mut f32) {
    unsafe {
        faiss_fvec_renorm_L2(d, nx, x);
    }
}

#[derive(Debug)]
pub enum Message {
    AddDocument(String, u64, String, Sender<Reply>),
    RetrieveDocument(String, Sender<Reply>),
}

pub enum Reply {
    Done(String, Vec<u64>),
    Docs(Vec<(u64, f32)>),
}

pub trait Indexer {
    fn add_document(&mut self, document: String, docid: u64, loc: String) -> Vec<u64>;
    fn retrieve_document(&mut self, query: &str) -> Vec<(u64, f32)>;
    fn run(&mut self);
}

fn get_index_location() -> String {
    let user_dir = dirs::home_dir().unwrap();
    let index_dir = user_dir.join(".cache/semdesk/.indexer");
    index_dir.to_str().unwrap().to_string()
}

pub struct IndexerImpl {
    index: IndexImpl,
    model: SentenceEmbeddingsModel,
    token_size: usize,
    adder_channel: (Sender<Message>, Receiver<Message>),
    retriever_channel: (Sender<Message>, Receiver<Message>),
    id_gen: IdGenerator,
    muted: bool,
}

impl IndexerImpl {
    pub fn new(id_gen: IdGenerator) -> Self {
        let token_size = 384;
        let mut index = index_factory(token_size, "IDMap,Flat", MetricType::InnerProduct).unwrap();
        let model = SentenceEmbeddingsBuilder::remote(
             SentenceEmbeddingsModelType::AllMiniLmL6V2,
         ).create_model().unwrap();
        let index_location = get_index_location();
        if std::path::Path::new(&index_location).exists() {
            log::debug!("Loading index from {}", index_location);
            index = faiss::read_index(&index_location).unwrap();
            log::debug!("Loaded index {}", index.ntotal());
        }
        IndexerImpl {
            index,
            model,
            token_size: token_size as usize,
            adder_channel: channel(),
            retriever_channel: channel(),
            id_gen,
            muted: false,
        }
    }

    pub fn get_adder(&self) -> Sender<Message> {
        self.adder_channel.0.clone()
    }

    pub fn get_retriever(&self) -> Sender<Message> {
        self.retriever_channel.0.clone()
    }

}

impl Indexer for IndexerImpl {
    fn add_document(&mut self, document: String, docid1:u64, loc: String) -> Vec<u64> {
        // chunk the document into self.token_size byte chunks with padding if less
        let chunks = document.as_bytes().chunks(self.token_size).map(|c| 
                                         String::from_utf8(c.to_vec())
                                         .unwrap_or(String::from(""))
                                         ).collect::<Vec<String>>();
        let mut input = Vec::new();
        for chunk in chunks.iter() {
            let chunk = chunk.trim();
            if chunk.len() > 0 {
                input.push(chunk);
            }
        }
        if chunks.len() == 0 || input.len() == 0 {
            log::debug!("No input for document {}", loc);
            return Vec::new();
        }
        println!("Input size {} Chunk size {}", input.len(), chunks.len());
        // chunks of length greater than  ~50 takes a hell lot of memory if we give all of them at
        // one go.  We can throttle with few sentences each time. But that increases cpu time.
        // Hence, let's truncate a large file at loss of information
        if input.len() > 50 {
            input.truncate(50);
        }
        let tokens = self.model.encode(&input).unwrap();
        let mut ids = Vec::new();
        for i in 0..tokens.len() {
            let mut token = tokens[i].clone();
            log::debug!("Token size {}/{}", token.len(), self.token_size);
            if token.len() < self.token_size {
                log::debug!("Resizing");
                token.resize(self.token_size, 0.0);
            }
            renorm_L2(self.token_size, 1, token.as_mut_ptr());
            let docid = self.id_gen.next();
            ids.push(docid);
            log::debug!("Adding document {} ", docid);
            let idx = Idx::new(docid);
            self.index.add_with_ids(&token, &[idx]).unwrap();
            self.muted = true;
        }
        log::debug!("Done indexing document {} ", loc);
        ids
    }
    fn retrieve_document(&mut self, query: &str) -> Vec<(u64, f32)> {
        log::debug!("Query {} total_indexes: {}", query, self.index.ntotal());
        let tokens = self.model.encode(&[String::from(query)]).unwrap();
        let mut token = tokens[0].clone();
        if tokens.len() < self.token_size {
                token.resize(self.token_size, 0.0);
        }
        renorm_L2(self.token_size, 1, token.as_mut_ptr());
        let res = self.index.search(&token, 6).unwrap();
        let mut docs: Vec<(u64, f32)> = Vec::new();
        for (dist, label) in res.distances.iter().zip(res.labels.iter()) {
            log::debug!("Dist {} Label {}", dist, label);
            if dist < &0.10 {
                continue;
            }
            let id = label;
            if id.is_none() {
                continue;
            }
            docs.push((id.get().unwrap(), dist.clone()));
        }
        docs
    }
    fn run(&mut self) {
        let mut counter = 0;
        loop {
            while let Ok(msg) = self.retriever_channel.1.try_recv() {
                match msg {
                    Message::RetrieveDocument(query, tx) => {
                        log::debug!("Retrieving document {} ", query);
                        let docs = self.retrieve_document(&query);
                        tx.send(Reply::Docs(docs)).unwrap();
                    }
                    _ => { break; }
                }
            }

            if let Ok(msg) = self.adder_channel.1.try_recv() {
                match msg {
                    Message::AddDocument(doc, id, loc, tx) => {
                        log::debug!("Received Indexing document {} ", loc);
                        let ids = self.add_document(doc, id, loc.clone());
                        tx.send(Reply::Done(loc.clone(), ids)).unwrap();
                    }
                    _ => {}
                }
            }

            if counter % 2000 == 0  && self.muted {
                let index_location = get_index_location();
                log::debug!("Writing index to {}", index_location);
                
                // backup the current index file before overwriting.  Always keep only two copies.
                if std::path::Path::new(&index_location).exists() {
                    let backup_location = format!("{}.bak", index_location);
                    if std::path::Path::new(&backup_location).exists() {
                        std::fs::remove_file(&backup_location).unwrap();
                    }
                    std::fs::rename(&index_location, &backup_location).unwrap();
                }

                faiss::write_index(&self.index, index_location).unwrap();

                counter = 0;
                self.muted = false;
                log::debug!("Done writing index");
            }
            counter += 100;

            thread::sleep(Duration::from_millis(100));
        }
    }
}
