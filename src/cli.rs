// Copyright: (c) 2023 Sureshkumar T
// License: Apache-2.0

mod settings;
mod error;
mod indexer;
mod idgenerator;
mod catalog;

use clap;
use clap::{Arg, Parser, Subcommand, ArgMatches};

use std::error::Error;
use std::path::PathBuf;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

use tracing;
use tracing_subscriber::prelude::*;
use tracing_subscriber;
use tracing::Level as LogLevel;

use indexer::Indexer;

#[derive(Parser, Debug)]
#[command(name = "semdesk-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long)]
    verbose: bool,
}

#[derive(Parser, Debug)]
enum Commands {
    #[command(name = "query")]
    Query {
        #[arg(required = true)]
        query: String,
    },

    #[command(name = "add")]
    AddDocument {
        #[arg(required = true)]
        location: String,
    },
}

#[derive(Debug)]
struct QueryResult {
    id: String,
    loc: String,
    text: String,
    score: f32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Cli::parse();

    if matches.verbose {
        tracing::subscriber::set_global_default(tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish())
            .expect("setting default subscriber failed");
        tracing::debug!("Verbose mode enabled");
    } else {
        tracing_subscriber::fmt()
            .with_max_level(LogLevel::WARN)
            .init();
    }

    match matches.command {
        Commands::Query { query } => {
            tracing::debug!("Query command");

            let socket_path = settings::get_socket_path();
            let mut socket = UnixStream::connect(socket_path)?;

            socket.write_all(query.as_bytes())?;
            // get the results from the unix domain socket
            let mut results = String::new();
            socket.read_to_string(&mut results)?;

            // parse the results into a result struct. The string is of the format split by |
            // id|loc|text|score
            let mut results: Vec<QueryResult> = results.split("\n")
                .filter(|r| !r.trim().is_empty())
                .map(|r| {
                    tracing::debug!("Parsing result: {}", r);
                    let mut r = r.split("|");
                    QueryResult {
                        id: r.next().unwrap().to_string(),
                        loc: r.next().unwrap().to_string(),
                        text: r.next().unwrap().to_string(),
                        score: r.next().unwrap().parse::<f32>().unwrap(),
                    }
                })
                .collect();
            
            // sort the results by score descending
            results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
            println!("Results:");
            for r in results {
                println!("File: {}", r.loc);
                println!("Match Probability: {:.2}%", r.score*100.0);
                println!("{}\n", r.text);
            }

        },
        Commands::AddDocument { location } => {
            tracing::warn!("this command is only for testing");
            let catalog = catalog::Catalog::new();
            let arc_catalog = Arc::new(catalog);
            let mut ind: indexer::IndexerImpl = indexer::IndexerImpl::new(idgenerator::IdGenerator::new(arc_catalog.clone()));
            let mut file = File::open(location.clone())?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            ind.add_document(contents, 0, location);
            // wait for user input to exit
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
        }
    }
    Ok(())
}
    
