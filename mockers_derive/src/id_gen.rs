///! Thread-safe unique ID generator.

use std::sync::atomic::{AtomicUsize, Ordering};

pub struct IdGen {
    next: AtomicUsize,
}

impl IdGen {
    pub fn new() -> Self { IdGen { next: AtomicUsize::new(0) } }
    pub fn next_id(&self) -> usize { self.next.fetch_add(1, Ordering::Relaxed) }
}
