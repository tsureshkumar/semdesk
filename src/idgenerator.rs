// Copyright: (c) 2023 Sureshkumar T
// License: Apache-2.0

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

use crate::catalog::Catalog;

pub struct IdGenerator {
    id: Arc<Catalog>,
}

impl IdGenerator {
    pub fn new(catalog: Arc<Catalog>) -> Self {
        IdGenerator {
            id: catalog,
        }
    }

    pub fn next(&self) -> u64 {
        let res = self.id.gen_id();
        res
    }
}
