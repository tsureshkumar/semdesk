// Copyright: (c) 2023 Sureshkumar T
// License: Apache-2.0

use std::error::Error;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::process::Stdio;
use mime_guess;

use pdf_extract;

use crate::error::{FileNotFoundError, UnsupportedFileTypeError};

pub struct Parser {
}

impl Parser {
    pub fn new() -> Self {
        Parser {}
    }

    pub fn parse(&self, filename: &str) -> Result<String, Box<dyn Error>> {
        if !Path::new(filename).exists() {
            return Err(Box::new(FileNotFoundError::new(filename)));
        }
        let mime = mime_guess::from_path(filename).first();
        if mime.is_none() {
            return Err(Box::new(UnsupportedFileTypeError::new(filename)));
        }
        if let Some(mime) = mime {
            if "application/pdf" == mime {
                return self.parse_pdf(filename);
            } else if "text/plain" == mime {
                return self.parse_text(filename);
            }
        }
        return Err(Box::new(UnsupportedFileTypeError::new(filename)));
    }

    pub fn parse_pdf(&self, filename: &str) -> Result<String, Box<dyn Error>> {
        log::debug!("parsing pdf file: {}", filename);
        let mut pdf2ps = Command::new("pdf2ps")
            .arg(filename)
            .arg("-")
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to execute process");
        let pdf2ps_out = pdf2ps.stdout.expect("failed to get stdout");
        let mut ps2ascii = Command::new("ps2ascii")
            .stdin(Stdio::from(pdf2ps_out))
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to execute process");
        let output = ps2ascii.wait_with_output().expect("failed to wait on child");
        let output = String::from_utf8(output.stdout)?;
        log::debug!("output: {}", output.len());
        Ok(output)
    }

    pub fn parse_text(&self, filename: &str) -> Result<String, Box<dyn Error>> {
        let mut file = File::open(filename)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }
}
