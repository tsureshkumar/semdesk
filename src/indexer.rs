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
#[derive(Clone)]
pub struct DocResult {
    id: String,
    pub loc: String,
    pub text: String,
    pub score: f32,
}

impl DocResult {
    fn new(id: String, loc: String, text: String, score: f32) -> Self {
        DocResult {
            id,
            loc,
            text,
            score,
        }
    }
}

impl std::fmt::Display for DocResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "id: {}, loc: {}, text: {}, score: {}", self.id, self.loc, self.text, self.score)
    }
}


pub trait Indexer {
    fn add_document(&mut self, document: &str, loc: &str);
    fn retrieve_document(&mut self, query: &str) -> Vec<DocResult>;
    fn persist_index(&self);
}

pub struct IndexerImpl {
    index: IndexImpl,
    map: HashMap<String, DocResult>,
    last_id: u64,
    model: SentenceEmbeddingsModel,
    token_size: usize,
}

impl IndexerImpl {
    pub fn new() -> Self {
        //let index = index_factory(self.token_size, "Flat", MetricType::L2).unwrap();
        let token_size = 384;
        let index = index_factory(token_size, "IDMap,Flat", MetricType::InnerProduct).unwrap();
        let model = SentenceEmbeddingsBuilder::remote(
             SentenceEmbeddingsModelType::AllMiniLmL6V2,
         ).create_model().unwrap();
        IndexerImpl {
            index,
            model,
            map: HashMap::new(),
            last_id: 0,
            token_size: token_size as usize,
        }
    }
}

impl Indexer for IndexerImpl {
    fn add_document(&mut self, document: &str, loc: &str) {
        let id = sha256::digest(document.as_bytes());
        // chunk the document into self.token_size byte chunks with padding if less
        let chunks = document.as_bytes().chunks(self.token_size).map(|c| String::from_utf8(c.to_vec()).unwrap()).collect::<Vec<String>>();
        let tokens = self.model.encode(&chunks).unwrap();
        for i in 0..tokens.len() {
            let mut token = tokens[i].clone();
            if token.len() < self.token_size {
                token.resize(self.token_size, 0.0);
            }
            renorm_L2(self.token_size, 1, token.as_mut_ptr());
            //self.index.add(&token).unwrap();
            self.map.insert(self.last_id.to_string(), DocResult::new(id.to_string(), loc.to_string(), chunks[i].clone(), 0.0)); 
            let idx = Idx::new(self.last_id);
            self.index.add_with_ids(&token, &[idx]).unwrap();
            self.last_id += 1;
        }
    }
    fn retrieve_document(&mut self, query: &str) -> Vec<DocResult>{
        let tokens = self.model.encode(&[String::from(query)]).unwrap();
        let mut token = tokens[0].clone();
        if tokens.len() < self.token_size {
                token.resize(self.token_size, 0.0);
        }
        renorm_L2(self.token_size, 1, token.as_mut_ptr());
        let res = self.index.search(&token, 6).unwrap();
        let mut docs = Vec::new();
        for (dist, label) in res.distances.iter().zip(res.labels.iter()) {
            if dist < &0.10 {
                continue;
            }
            let id = label;
            if id.is_none()  || id.get() >= Some(self.index.ntotal()) {
                continue;
            }
            let mut ret: DocResult =  self.map.get(&id.get().unwrap().to_string()).unwrap().clone();
            ret.score = *dist;
            docs.push(ret.clone());
        }
        docs
    }
    fn persist_index(&self) {
    }
}
