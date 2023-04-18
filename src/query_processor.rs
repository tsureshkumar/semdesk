// Copyright: (c) 2023 Sureshkumar T
// License: Apache-2.0


use std::thread;
use std::sync::mpsc::{Sender, Receiver, channel};
use rust_bert::pipelines::question_answering::{QuestionAnsweringModel, QaInput};
use std::os::unix::net::{UnixStream,UnixListener};
use std::io::{Read, Write};

use crate::retriever;
use crate::settings;

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


pub trait QueryProcessor {
    fn new(retriever: Sender<retriever::Message>) -> Self;
    fn process(&mut self, query: &str, context: String) -> Vec<DocResult>;
    fn run(&mut self);
}

pub struct QueryProcessorImpl {
    qa_model: QuestionAnsweringModel,
    retriever: Sender<retriever::Message>,
    comm: (Sender<(String, Sender<(String, Vec<DocResult>)>)>, Receiver<(String, Sender<(String, Vec<DocResult>)>)>),
    listener: UnixListener,
}


impl QueryProcessorImpl {
    fn new(retriever: Sender<retriever::Message>) -> Self {
        let sock_path = settings::get_socket_path();
        let listener = UnixListener::bind(sock_path).unwrap();
        listener.set_nonblocking(true).unwrap();

        QueryProcessorImpl {
            qa_model: QuestionAnsweringModel::new(Default::default()).unwrap(),
            retriever,
            comm: channel(),
            listener: listener,
        }
    }
    pub fn get_query_channel(&self) -> Sender<(String, Sender<(String, Vec<DocResult>)>)> {
        self.comm.0.clone()
    }
}

impl QueryProcessor for QueryProcessorImpl {
    fn new(retriever: Sender<retriever::Message>) -> Self {
        QueryProcessorImpl::new(retriever)
    }

    fn process(&mut self, query: &str, context: String) -> Vec<DocResult> {
        let mut results = Vec::new();
        let answers = self.qa_model.predict(&[QaInput { question: String::from(query), context: String::from(context) }], 3, 32);
        log::debug!("{:?}", answers);
        if answers.len() == 0 {
            return results;
        }
        for answer in answers[0].iter() {
            let result = DocResult::new(String::from("id"), String::from("loc"), answer.answer.clone(), answer.score as f32);
            results.push(result);
        }
        results
    }

    fn run(&mut self) {
        loop {
            if let Ok((msg, ch)) = self.comm.1.try_recv() {
                log::debug!("QueryProcessor received: {}", msg);
                let (sender, receiver) = channel();
                self.retriever.send(retriever::Message::RetrieveByQuery(msg.clone(), sender)).unwrap();
                if let Ok(docs) = receiver.recv() {
                    match docs {
                        retriever::Reply::Docs(docs) => {
                            for (filename,doc) in docs.iter() {
                                log::debug!("doc: {}", doc.len());
                                let mut results: Vec<DocResult> = self.process(&msg, doc.to_string());
                                let sent = ch.send((filename.clone(), results));
                                if sent.is_err() {
                                    log::error!("Error sending results: {}", sent.err().unwrap());
                                    break;
                                }
                            }
                        },
                        _ => {
                            log::debug!("No docs found");
                        }
                    }
                }

            }

            let s = self.listener.accept();
            if let Ok((mut stream, _)) = s {
                    let mut buf = [0; 1024];
                    let chan = stream.read(&mut buf);
                    if chan.is_err() {
                        log::error!("Error reading from socket: {}", chan.err().unwrap());
                        return;
                    }
                    let n = chan.unwrap();
                    if n > 0 {
                        let msg = String::from_utf8_lossy(&buf[..n]);
                        log::debug!("QueryProcessor received: {}", msg);
                        let (sender, receiver) = channel();
                        self.retriever.send(retriever::Message::RetrieveByQuery(msg.clone().to_string(), sender)).unwrap();
                        if let Ok(docs) = receiver.recv() {
                            match docs {
                                retriever::Reply::Docs(docs) => {
                                    for (filename,doc) in docs.iter() {
                                        log::debug!("doc2: {}", doc.len());
                                        let mut results: Vec<DocResult> = self.process(&msg, doc.to_string());
                                        let mut res = String::new();
                                        for result in results.iter() {
                                            // remove unprintable characters
                                            let mut text = result.text.replace("\n", " ");
                                            text = text.replace("\r", " ");
                                            text = text.replace("|", " ");
                                            res.push_str(&format!("{}|{}|{}|{}\n", result.id, filename, text, result.score));
                                        }
                                        let res = stream.write(res.as_bytes());
                                        if res.is_err() {
                                            log::error!("Error writing to socket: {}", res.err().unwrap());
                                            return;
                                        }
                                    }
                                },
                                _ => {
                                    log::debug!("No docs found");
                                }
                            }
                        }
                        stream.flush().unwrap();
                        if let Err(e) = stream.shutdown(std::net::Shutdown::Both) {
                            log::error!("Error shutting down socket: {}", e);
                        }
                    }
            } else if let Err(e) = s {
                if e.kind() != std::io::ErrorKind::WouldBlock {
                    log::error!("Error accepting socket: {}", e);
                }
            }

            thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
