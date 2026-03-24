use std::hash::Hasher;

use bytes::Bytes;
use rapidhash::v3::rapidhash_v3;
use http::HeaderMap;
use thiserror::Error;

///
const POS_RESET: u64 = !0;
const MAX_SIZE: usize = 6553;
const MAX_CAPACITY: usize = 8192;
const INITIAL_CAPACITY: usize = 16;
const LOAD_FACTOR_THRESHOLD: f32 = 0.2;

#[derive(Debug)]
pub struct ExtraValues<'a> {
    value: &'a [u8],
    link: Link,
}

#[derive(Debug)]
pub struct Link {
    prev: usize,
    next: usize,
}

#[derive(Debug)]
pub struct Bucket<'a> {
    hash: u16,
    key: &'a [u8],
    value: &'a [u8],
    overflow: Option<Link>,
}

#[derive(Debug, Error)]
pub enum HeaderMapErrors {
    #[error("HeaderMap has reached max capacity")]
    MaxCapacityReached
}

#[derive(Debug, Default)]
pub struct HeaderMap2<'a> {
    mask: u16,
    size: usize,
    capactiy: usize,
    pos: &'a [u64],
    entries: Vec<Bucket<'a>>,
}

impl<'a> HeaderMap2<'a> {
    pub fn new() -> Self {
        Self {
            mask: 0,
            size: 0,
            capactiy: 0,
            pos: &[0; 0],
            entries: Vec::new(),
        }
    }

    fn load_factor(&self) -> f32 {
        self.size as f32 / self.capactiy as f32
    }

    pub fn insert(&mut self, key: &[u8], value: &[u8]) {}

    fn ensure_capacity(&mut self) -> Result<(), HeaderMapErrors> {
        // apply the initial values and capacity to the slice and vector
        if self.capactiy == 0 {
            self.capactiy = INITIAL_CAPACITY;
            self.pos = &[POS_RESET; INITIAL_CAPACITY];
            self.entries.reserve_exact(INITIAL_CAPACITY);
            return Ok(());
        }

        // return early if the load factor is within the threshold
        if self.load_factor() < LOAD_FACTOR_THRESHOLD {
            return Ok(())
        }

        self.try_grow(self.capactiy * 2)?;
        Ok(())
    }

    fn try_grow(&mut self, new_cap: usize) -> Result<(), HeaderMapErrors> {
        Ok(())
    }
}
