// Copyright: (c) 2023 Sureshkumar T
// License: Apache-2.0

use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
#[derive(Clone)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: &str) -> Self {
        Error {
            message: message.to_string(),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        &self.message
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SemDeskError: {}", self.message)
    }
}

#[derive(Debug)]
pub struct FileNotFoundError {
    filename: String,
}
impl Display for FileNotFoundError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "FileNotFoundError: {}", self.filename)
    }
}
impl FileNotFoundError {
    pub fn new(filename: &str) -> Self {
        FileNotFoundError {
            filename: filename.to_string(),
        }
    }
}
impl StdError for FileNotFoundError {
    fn description(&self) -> &str {
        &self.filename
    }
}


#[derive(Debug)]
pub struct UnsupportedFileTypeError {
    filename: String,
}
impl UnsupportedFileTypeError {
    pub fn new(filename: &str) -> Self {
        UnsupportedFileTypeError {
            filename: filename.to_string(),
        }
    }
}
impl Display for UnsupportedFileTypeError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "UnsupportedFileTypeError: {}", self.filename)
    }
}
impl StdError for UnsupportedFileTypeError {
    fn description(&self) -> &str {
        &self.filename
    }
}
