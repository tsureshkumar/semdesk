// Copyright: (c) 2023 Sureshkumar T
// License: Apache-2.0

mod settings;
mod crawler;
mod indexer;
mod idgenerator;
mod query_processor;
mod retriever;
mod catalog;
mod parsers;
mod error;



use rust_bert::pipelines::question_answering::{QuestionAnsweringModel, QaInput};
use pdf_extract;
use std::thread;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use ndarray::prelude::*;
use ndarray::{Array, Array1, Axis};
use env_logger;
use tracing;
use tracing::{info, Level};
use tracing_subscriber;
use which::which;

use clap;
use clap::{Arg, Parser, Subcommand, ArgMatches};


use indexer::{Indexer, IndexerImpl};
use settings::get_config;
use crawler::{Crawler, CrawlerImpl};
use query_processor::{QueryProcessor, QueryProcessorImpl};
use retriever::{Retriever, RetrieverImpl};
use catalog::Catalog;

fn warmup() {
    // remove unix socket file
    let _ = std::fs::remove_file(settings::get_socket_path());
    let _ = std::fs::create_dir_all(settings::get_config_dir());
    // check for binary runtime dependencies
    let binaries = vec!["pdf2ps", "ps2ascii"];
    for bin in binaries {
        let bin_path = which::which(bin);
        match bin_path {
            Ok(_) => {
                tracing::info!("{} found", bin);
            },
            Err(_) => {
                tracing::warn!("{} not found", bin);
            }
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "semdesk")]
struct Tool {
    #[arg(short, long)]
    local: bool,

    #[arg(short, long)]
    dir: Option<String>,

    #[arg(short, long)]
    verbose: bool
}

fn main() {
    let cli = Tool::parse();
    tracing_subscriber::fmt().with_thread_ids(true)
        .with_max_level(match cli.verbose {
            true => Level::TRACE,
            _ => Level::INFO,
        })
        .init();


    match cli.local {
        true =>  {
            info!("Running in local mode");
            settings::get_config(Some(settings::LocalModeSettings::new(cli.dir.unwrap(), cli.local, cli.verbose, 2)));
        }
        _ =>
        {
            info!("Running in server mode");
            settings::get_config(None);
        }
    }

    warmup();

    let catalog = Catalog::new();
    let arc_catalog = Arc::new(catalog);

    let mut indexer = IndexerImpl::new(idgenerator::IdGenerator::new(arc_catalog.clone()));
    let idx_adder_ch = indexer.get_adder();
    let idx_query_ch = indexer.get_retriever();
    let mut file_crawler = CrawlerImpl::new(arc_catalog.clone(), idx_adder_ch);

    let thr1 = thread::spawn(move || {
        file_crawler.run();
    });

    let thr2 = thread::spawn(move || {
        indexer.run();
    });

    let mut retriever_obj: RetrieverImpl = Retriever::new(arc_catalog.clone(), idx_query_ch);
    let retriever_ch = retriever_obj.get_sender();

    let thr4 = thread::spawn(move || {
        retriever_obj.run();
    });

    let mut query_processor: QueryProcessorImpl = QueryProcessor::new(retriever_ch);
    let query_ch = query_processor.get_query_channel();
    let thr3 = thread::spawn(move || {
        query_processor.run();
    });

    thr1.join().unwrap();
    thr2.join().unwrap();
    thr3.join().unwrap();
    thr4.join().unwrap();
}
