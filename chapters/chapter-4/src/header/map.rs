//! A simplified version of `HeaderMap` implemented in the `http` crate
//! A multi-map of header-names (`Bytes`) to one or more header-values (`Bytes`).
//!
//! <https://github.com/hyperium/http/blob/master/src/header/map.rs>
//!
//! HTTP 1.1 spec allow the same header to have multiple values in the same request
//! or response, and the values can be either in the same line separated by commas or in multiple
//! lines with the same header name.
//! e.g.
//! ```http
//! Field1: value1
//! Field1: value2
//! Field2: value3, value4
//! Field2: value5
//! ```
//! In this server implementation, comma separated values will be treated as a single value, and
//! multiple lines with the same header name will be treated as multiple values.
//!
//! Unlike the `http` crate, this implementation does not check for illegal bytes/characters in the header names or values upon insertion,
//! and assumes that the input is always valid.
//!
//! Additionally, this implementation does not support case-insensitive header names, and treats header names as case-sensitive.
//! So they will be stored and looked up as-is, without any normalization or conversion to lowercase.

use std::mem;

use bytes::Bytes;
use rapidhash::v3::rapidhash_v3;
use smallvec::{SmallVec, smallvec};
use thiserror::Error;

type HashSize = u32;
type IndexSize = u16;

/// The load-factor threshold is effectively the maximum percentage of the entries the map can be filled
/// before it needs to grow the capacity and rehash the entries.
const LOAD_FACTOR_THRESHOLD: f32 = 0.2;

/// The sever allows up to 32KB of headers.
/// The shortest possible header-line that needs to be accounted for is 5 bytes.
/// i.e `"A:B:\r\n"`
///
/// This means that the maximum headers-lines with both a key and value is 32KB / 5 = 6553.6.
/// Rounded up to 6554.
/// Since the load factor threshold is 0.2, the maximum capacity of the header map is
/// 6553.6 / 0.2 = 32768
const MAX_CAPACITY: usize = 32 << 10;

/// The initial capacity of the header map indices vector list.
const INITIAL_CAPACITY: usize = 16;

/// Placeholder hash value to indicate an empty slot in the indices vector list.
/// Probably not the best way to go about doing this, but it works 🤷
const POS_HASH_EMPTY: HashSize = 0;

#[derive(Debug, Clone)]
struct Pos {
    /// The hash of the header name, used for quick comparisons during lookups and insertions.
    hash: HashSize,

    /// The index of the corresponding bucket in the entries vector list, where the header value is
    /// stored
    entry_idx: IndexSize,

    /// The ideal index of pos within the indices
    ideal_pos: IndexSize,
}

impl Pos {
    #[inline]
    pub fn new(hash: HashSize, entry_idx: IndexSize, ideal_pos: IndexSize) -> Self {
        Self {
            hash,
            entry_idx,
            ideal_pos,
        }
    }
    #[inline]
    pub fn empty() -> Self {
        Self {
            hash: POS_HASH_EMPTY,
            entry_idx: 0,
            ideal_pos: 0,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.hash == POS_HASH_EMPTY
    }
}

/// A Node of of a vector backed linked list used to store the extra values of a header
#[derive(Debug)]
struct ExtraValue {
    value: Bytes,
    next: Option<usize>,
    prev: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct Link {
    head: usize,
    tail: usize,
}

/// A bucket in the header map, which stores a key-value pair and an optional link to with indices
/// to the overflow/additional values of the same header-name
#[derive(Debug)]
struct Bucket {
    key: Bytes,
    value: Bytes,
    overflow: Option<Link>,
    overflow_count: usize,
}

#[derive(Debug, Error)]
pub enum HeaderMapErrors {
    #[error("HeaderMap has reached max capacity")]
    MaxCapacityReached,
}

/// An iterator over the values of a header
#[derive(Debug)]
pub struct ValueIter<'a> {
    map: &'a HeaderMap,
    head: Option<usize>,
    next: Option<usize>,
    first: Option<&'a [u8]>,
    first_complete: bool,
}

impl<'a> ValueIter<'a> {
    pub fn new(map: &'a HeaderMap, first: Option<&'a [u8]>, head: Option<usize>) -> Self {
        Self {
            map,
            head,
            next: None,
            first,
            first_complete: false,
        }
    }
}

impl<'a> Iterator for ValueIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.first?;

        if !self.first_complete {
            self.first_complete = true;
            self.next = self.head;
            return self.first;
        }

        self.next?;

        let curr_idx = self.next.unwrap();
        let curr = &self.map.extra_values[curr_idx];
        self.next = curr.next;

        Some(&curr.value)
    }
}

/// An iterator over all header name and values
#[derive(Debug)]
pub struct MapIter<'a> {
    map: &'a HeaderMap,
    index: usize,
    indices_left: usize,
    value_iter: Option<ValueIter<'a>>,
}

impl<'a> MapIter<'a> {
    pub fn new(map: &'a HeaderMap) -> Self {
        Self {
            map,
            index: 0,
            indices_left: map.size,
            value_iter: None,
        }
    }
}

impl<'a> Iterator for MapIter<'a> {
    type Item = (&'a [u8], &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.indices_left == 0 {
            return None;
        }

        match self.value_iter.as_mut().and_then(|iter| iter.next()) {
            Some(value) => {
                let pos = &self.map.indices[self.index];
                let bucket = &self.map.entries[pos.entry_idx as usize];
                return Some((bucket.key.as_ref(), value));
            }
            None => self.index += 1,
        }

        while self.map.indices[self.index].is_empty() {
            self.index += 1;
        }

        let pos = &self.map.indices[self.index];
        let bucket = &self.map.entries[pos.entry_idx as usize];

        self.value_iter = Some(ValueIter::new(
            self.map,
            Some(&bucket.value),
            bucket.overflow.as_ref().map(|link| link.head),
        ));
        self.indices_left -= 1;

        self.value_iter
            .as_mut()
            .unwrap()
            .next()
            .map(|value| (bucket.key.as_ref(), value))
    }
}

/// A simplified version of `HeaderMap` implemented in the `http` crate
/// A multi-map of header-names (`Bytes`) to one or more header-values (`Bytes`).
///
/// Example usage:
/// ```rust
/// use bytes::Bytes;
/// let mut map = HeaderMap::new();
///
/// map.insert(Bytes::from_static(b"content-type"), Bytes::from_static(b"text/html")).unwrap();
/// assert_eq!(map.get(b"content-type"), Some(b"text/html".as_ref()));
///
/// map.append(Bytes::from_static(b"content-type"), Bytes::from_static(b"text/plain")).unwrap();
/// assert_eq!(map.get_all(b"content-type").collect::<Vec<_>>(), vec![b"text/html".as_ref(), b"text/plain".as_ref()]);
/// ```
#[derive(Debug, Default)]
pub struct HeaderMap {
    mask: u16,
    size: usize,
    capactiy: usize,
    values_count: usize,
    indices: Box<[Pos]>,
    entries: Vec<Bucket>,
    extra_values: Vec<ExtraValue>,
    vacant_entries: SmallVec<[usize; 4]>,
}

impl HeaderMap {
    pub const MAX_SIZE: usize = (MAX_CAPACITY as f32 * LOAD_FACTOR_THRESHOLD) as usize + 1;

    #[inline]
    pub fn new() -> Self {
        Self {
            mask: (INITIAL_CAPACITY - 1) as u16,
            size: 0,
            capactiy: INITIAL_CAPACITY,
            values_count: 0,
            indices: vec![Pos::empty(); INITIAL_CAPACITY].into_boxed_slice(),
            entries: vec![],
            extra_values: vec![],
            vacant_entries: smallvec![],
        }
    }

    #[inline]
    fn load_factor(&self) -> f32 {
        self.size as f32 / self.capactiy as f32
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capactiy
    }

    #[inline]
    pub fn values_count(&self) -> usize {
        self.values_count
    }

    /// Inserts a key-value pair into the map.
    /// Replaces any existing value for the same key if present.
    pub fn insert(&mut self, key: Bytes, value: Bytes) -> Result<Option<Bytes>, HeaderMapErrors> {
        self.ensure_capacity()?;

        let hash = hash_key(&key);
        let ideal_pos = ideal_pos(hash, self.mask);
        let mut probe = ideal_pos;

        while !self.indices[probe].is_empty() {
            if self.indices[probe].hash == hash {
                break;
            }
            probe = (probe + 1) & self.mask as usize;
        }

        if self.indices[probe].is_empty() {
            let idx = match self.vacant_entries.pop() {
                Some(idx) => {
                    self.entries[idx] = Bucket {
                        key,
                        value,
                        overflow: None,
                        overflow_count: 0,
                    };
                    idx
                }
                None => {
                    self.entries.push(Bucket {
                        key,
                        value,
                        overflow: None,
                        overflow_count: 0,
                    });
                    self.entries.len() - 1
                }
            };
            self.indices[probe] = Pos::new(hash, idx as IndexSize, ideal_pos as IndexSize);
            self.size += 1;
            self.values_count += 1;
            return Ok(None);
        }

        let pos = &self.indices[probe];
        let mut new_bucket = Bucket {
            key,
            value,
            overflow: None,
            overflow_count: 0,
        };
        let old_bucket = &mut self.entries[pos.entry_idx as usize];
        self.values_count -= old_bucket.overflow_count;
        mem::swap(old_bucket, &mut new_bucket);

        Ok(Some(new_bucket.value))
    }

    /// Gets the first value associated with the given key, if it exists.
    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        if self.size == 0 {
            return None;
        }

        let hash = hash_key(key);
        let mut probe = ideal_pos(hash, self.mask);

        while !self.indices[probe].is_empty() {
            if self.indices[probe].hash == hash {
                return Some(&self.entries[self.indices[probe].entry_idx as usize].value);
            }
            probe = (probe + 1) & self.mask as usize;
        }
        None
    }

    /// Gets a mutable reference to the first value associated with the given key, if it exists.
    pub fn get_mut(&mut self, key: &[u8]) -> Option<&mut Bytes> {
        if self.size == 0 {
            return None;
        }

        let hash = hash_key(key);
        let mut probe = ideal_pos(hash, self.mask);

        while !self.indices[probe].is_empty() {
            if self.indices[probe].hash == hash {
                return Some(&mut self.entries[self.indices[probe].entry_idx as usize].value);
            }
            probe = (probe + 1) & self.mask as usize;
        }
        None
    }

    /// Appends a value to the list of values associated with the given key.
    /// If the key does not exist, it will be inserted with the given value.
    pub fn append(&mut self, key: Bytes, value: Bytes) -> Result<(), HeaderMapErrors> {
        self.ensure_capacity()?;

        let hash = hash_key(&key);
        let ideal_pos = ideal_pos(hash, self.mask);
        let mut probe = ideal_pos;

        while !self.indices[probe].is_empty() {
            if self.indices[probe].hash == hash {
                break;
            }
            probe = (probe + 1) & self.mask as usize;
        }

        if self.indices[probe].is_empty() {
            let idx = match self.vacant_entries.pop() {
                Some(idx) => {
                    self.entries[idx] = Bucket {
                        key,
                        value,
                        overflow: None,
                        overflow_count: 0,
                    };
                    idx
                }
                None => {
                    self.entries.push(Bucket {
                        key,
                        value,
                        overflow: None,
                        overflow_count: 0,
                    });
                    self.entries.len() - 1
                }
            };

            self.indices[probe] = Pos::new(hash, idx as IndexSize, ideal_pos as IndexSize);

            self.size += 1;
            self.values_count += 1;
            return Ok(());
        }

        let pos = &self.indices[probe];
        let bucket = &mut self.entries[pos.entry_idx as usize];
        let mut new_extra = ExtraValue {
            value,
            next: None,
            prev: None,
        };

        let new_link = match bucket.overflow {
            Some(link) => {
                let next = self.extra_values.len();
                let current_tail = &mut self.extra_values[link.tail];

                new_extra.prev = Some(link.tail);
                current_tail.next = Some(next);

                Link {
                    head: link.head,
                    tail: next,
                }
            }
            None => {
                let next = self.extra_values.len();
                Link {
                    head: next,
                    tail: next,
                }
            }
        };

        self.extra_values.push(new_extra);
        self.values_count += 1;
        bucket.overflow_count += 1;
        bucket.overflow = Some(new_link);

        Ok(())
    }

    /// Removes the values associated with the given key and returns the first value if it exists.
    pub fn remove(&mut self, key: &[u8]) -> Option<&[u8]> {
        let hash = hash_key(key);
        let ideal_pos = ideal_pos(hash, self.mask);
        let mut probe = ideal_pos;

        if self.indices[probe].is_empty() {
            return None;
        }

        while !self.indices[probe].is_empty() {
            if self.indices[probe].hash == hash {
                break;
            }
            probe = (probe + 1) & self.mask as usize;
        }

        let pos = &self.indices[probe];
        let bucket = &self.entries[pos.entry_idx as usize];

        self.size -= 1;
        self.values_count -= 1 + bucket.overflow_count;
        self.vacant_entries.push(pos.entry_idx as usize);
        self.indices[probe] = Pos::empty();

        probe = (probe + 1) & self.mask as usize;

        while !self.indices[probe].is_empty() {
            // if this value's ideal_pos is the same as probe, it doesn't need to be shifted back.
            if self.indices[probe].ideal_pos as usize == probe {
                break;
            }

            let prev = (probe - 1) & self.mask as usize;
            self.indices.swap(prev, probe);
            probe = (probe + 1) & self.mask as usize;
        }

        Some(&bucket.value)
    }

    /// Checks if the map contains the given key.
    pub fn contains_key(&self, key: &[u8]) -> bool {
        self.get(key).is_some()
    }

    /// Clears the map, removing all key-value pairs.
    pub fn clear(&mut self) {
        // no need to clear the indices since `ensure_capacity` will create a new boxed slice to
        // replace as capacity is reset to 0
        self.entries.clear();
        self.extra_values.clear();
        self.size = 0;
        self.values_count = 0;
        self.capactiy = 0;
        self.mask = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Checks if the map contains multiple values for the given key.
    pub fn has_multiple_values(&self, key: &[u8]) -> Option<bool> {
        if self.size == 0 {
            return None;
        }

        let hash = hash_key(key);
        let mut probe = ideal_pos(hash, self.mask);

        while !self.indices[probe].is_empty() {
            if self.indices[probe].hash == hash {
                return Some(
                    self.entries[self.indices[probe].entry_idx as usize].overflow_count > 0,
                );
            }
            probe = (probe + 1) & self.mask as usize;
        }
        Some(false)
    }

    /// Gets an iterator over all values associated with the given key.
    pub fn get_all(&self, key: &[u8]) -> ValueIter<'_> {
        if self.size == 0 {
            return ValueIter::new(self, None, None);
        }

        let hash = hash_key(key);
        let mut probe = ideal_pos(hash, self.mask);

        while !self.indices[probe].is_empty() {
            if self.indices[probe].hash == hash {
                break;
            }
            probe = (probe + 1) & self.mask as usize;
        }

        let pos = &self.indices[probe];
        if pos.is_empty() {
            return ValueIter::new(self, None, None);
        }

        let bucket = &self.entries[pos.entry_idx as usize];
        ValueIter::new(
            self,
            Some(&bucket.value),
            bucket.overflow.as_ref().map(|link| link.head),
        )
    }

    /// Gets an iterator over all key-value pairs in the map.
    pub fn iter(&self) -> MapIter<'_> {
        MapIter::new(self)
    }

    #[inline]
    fn ensure_capacity(&mut self) -> Result<(), HeaderMapErrors> {
        // apply the initial values and capacity to the vectors
        if self.capactiy == 0 {
            self.capactiy = INITIAL_CAPACITY;
            self.indices = vec![Pos::empty(); INITIAL_CAPACITY].into_boxed_slice();
            self.mask = (INITIAL_CAPACITY - 1) as u16;
            return Ok(());
        }

        // return early if the load factor is within the threshold
        if self.load_factor() < LOAD_FACTOR_THRESHOLD {
            return Ok(());
        }

        // if not try to grow the vec capacity and rehash the entries
        self.try_grow(self.capactiy << 1)?;
        Ok(())
    }

    #[inline]
    fn try_grow(&mut self, new_cap: usize) -> Result<(), HeaderMapErrors> {
        if new_cap > MAX_CAPACITY {
            return Err(HeaderMapErrors::MaxCapacityReached);
        }

        self.mask = (new_cap - 1) as u16;
        let mut new_indices = vec![Pos::empty(); new_cap].into_boxed_slice();

        let mut idx = 0;

        loop {
            if idx >= self.indices.len() {
                break;
            }

            let pos = &self.indices[idx];
            if pos.is_empty() {
                idx += 1;
                continue;
            }

            let ideal_pos = ideal_pos(pos.hash, self.mask);
            let mut probe = ideal_pos;

            while !new_indices[probe].is_empty() {
                probe = (probe + 1) & self.mask as usize;
            }

            mem::swap(&mut new_indices[probe], &mut self.indices[idx]);
            new_indices[probe].ideal_pos = ideal_pos as IndexSize;

            idx += 1;
        }

        self.indices = new_indices;
        self.capactiy = new_cap;

        Ok(())
    }
}

#[inline]
fn hash_key(key: &[u8]) -> HashSize {
    // let mut hasher = GxHasher::default();
    // hasher.write(key);
    // hasher.finish()
    rapidhash_v3(key) as HashSize
}

#[inline]
fn ideal_pos(hash: HashSize, mask: u16) -> usize {
    (hash & mask as HashSize) as usize
}

#[cfg(test)]
mod tests {
    use rand::prelude::*;

    use super::super::HEADER_CHARS;
    use super::*;

    #[test]
    fn insert() -> Result<(), HeaderMapErrors> {
        let mut map = HeaderMap::new();

        if map
            .insert(
                Bytes::from_static(b"content-type"),
                Bytes::from_static(b"text/html"),
            )?
            .is_some()
        {
            panic!("Expected None, got Some")
        };
        match map.insert(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/plain"),
        )? {
            Some(old_value) => assert_eq!(old_value, Bytes::from_static(b"text/html")),
            None => panic!("Expected Some, got None"),
        }
        match map.insert(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/css"),
        )? {
            Some(old_value) => assert_eq!(old_value, Bytes::from_static(b"text/plain")),
            None => panic!("Expected Some, got None"),
        }

        assert_eq!(map.len(), 1);
        assert_eq!(map.values_count(), 1);
        assert_eq!(map.capacity(), 16);

        if map
            .insert(
                Bytes::from_static(b"content-length"),
                Bytes::from_static(b"123"),
            )?
            .is_some()
        {
            panic!("Expected None, got Some")
        }

        match map.insert(
            Bytes::from_static(b"content-length"),
            Bytes::from_static(b"456"),
        )? {
            Some(old_value) => assert_eq!(old_value, Bytes::from_static(b"123")),
            None => panic!("Expected Some, got None"),
        }

        match map.insert(
            Bytes::from_static(b"content-length"),
            Bytes::from_static(b"789"),
        )? {
            Some(old_value) => assert_eq!(old_value, Bytes::from_static(b"456")),
            None => panic!("Expected Some, got None"),
        }

        match map.insert(
            Bytes::from_static(b"content-length"),
            Bytes::from_static(b"100"),
        )? {
            Some(old_value) => assert_eq!(old_value, Bytes::from_static(b"789")),
            None => panic!("Expected Some, got None"),
        }

        assert_eq!(map.len(), 2);
        assert_eq!(map.values_count(), 2);
        assert_eq!(map.capacity(), 16);
        Ok(())
    }

    #[test]
    fn insert_many() -> Result<(), HeaderMapErrors> {
        let mut headers = vec![];
        for i in 0..100 {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            headers.push((key, value));
        }

        let mut map = HeaderMap::new();
        for (key, value) in headers.iter() {
            if map
                .insert(
                    Bytes::copy_from_slice(key.as_bytes()),
                    Bytes::copy_from_slice(value.as_bytes()),
                )?
                .is_some()
            {
                panic!("Expected None, got Some")
            }
        }

        assert_eq!(map.len(), 100);
        assert_eq!(map.values_count(), 100);
        assert_eq!(map.capacity(), (100 * 5_usize).next_power_of_two());
        Ok(())
    }

    #[test]
    fn insert_until_capacity() -> Result<(), HeaderMapErrors> {
        let mut headers = vec![];
        for i in 0..(HeaderMap::MAX_SIZE) {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            headers.push((key, value));
        }

        let mut map = HeaderMap::new();
        for (key, value) in headers.iter() {
            let ins = map.insert(
                Bytes::copy_from_slice(key.as_bytes()),
                Bytes::copy_from_slice(value.as_bytes()),
            )?;
            if ins.is_some() {
                dbg!(&map.indices);
                dbg!(&map.entries);
                dbg!(&key, &value, ins);
                panic!("Expected None, got Some")
            }
        }

        assert_eq!(map.len(), HeaderMap::MAX_SIZE);
        assert_eq!(map.values_count(), HeaderMap::MAX_SIZE);
        assert_eq!(map.capacity(), MAX_CAPACITY);
        Ok(())
    }

    #[test]
    fn insert_too_many() -> Result<(), HeaderMapErrors> {
        let mut headers = vec![];
        for i in 0..(HeaderMap::MAX_SIZE + 1) {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            headers.push((key, value));
        }

        let mut map = HeaderMap::new();
        for (i, (key, value)) in headers.iter().enumerate() {
            if i == HeaderMap::MAX_SIZE {
                match map.insert(
                    Bytes::copy_from_slice(key.as_bytes()),
                    Bytes::copy_from_slice(value.as_bytes()),
                ) {
                    Ok(_) => panic!("Expected Err, got Ok"),
                    Err(e) => assert_eq!(e.to_string(), "HeaderMap has reached max capacity"),
                }
            } else {
                if map
                    .insert(
                        Bytes::copy_from_slice(key.as_bytes()),
                        Bytes::copy_from_slice(value.as_bytes()),
                    )?
                    .is_some()
                {
                    panic!("Expected None, got Some")
                }
            }
        }
        assert_eq!(map.len(), HeaderMap::MAX_SIZE);
        assert_eq!(map.values_count(), HeaderMap::MAX_SIZE);
        assert_eq!(map.capacity(), MAX_CAPACITY);
        Ok(())
    }

    #[test]
    fn get() -> Result<(), HeaderMapErrors> {
        let mut map = HeaderMap::new();

        assert_eq!(map.get(b"content-type"), None);

        map.insert(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/html"),
        )?;
        assert_eq!(map.get(b"content-type"), Some(b"text/html".as_ref()));

        map.insert(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/plain"),
        )?;
        assert_eq!(map.get(b"content-type"), Some(b"text/plain".as_ref()));

        map.insert(
            Bytes::from_static(b"content-length"),
            Bytes::from_static(b"123"),
        )?;
        assert_eq!(map.get(b"content-length"), Some(b"123".as_ref()));

        Ok(())
    }

    #[test]
    fn get_mut() -> Result<(), HeaderMapErrors> {
        let mut map = HeaderMap::new();

        assert_eq!(map.get_mut(b"content-type"), None);

        map.insert(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/html"),
        )?;
        assert_eq!(
            map.get_mut(b"content-type"),
            Some(&mut Bytes::from_static(b"text/html"))
        );

        map.insert(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/plain"),
        )?;
        assert_eq!(
            map.get_mut(b"content-type"),
            Some(&mut Bytes::from_static(b"text/plain"))
        );

        map.insert(
            Bytes::from_static(b"content-length"),
            Bytes::from_static(b"123"),
        )?;
        assert_eq!(
            map.get_mut(b"content-length"),
            Some(&mut Bytes::from_static(b"123"))
        );

        let content_type_mut = map.get_mut(b"content-type").unwrap();
        *content_type_mut = Bytes::from_static(b"text/css");

        assert_eq!(
            map.get_mut(b"content-type"),
            Some(&mut Bytes::from_static(b"text/css"))
        );

        Ok(())
    }

    #[test]
    fn append() -> Result<(), HeaderMapErrors> {
        let mut map = HeaderMap::new();

        map.append(
            Bytes::from_static(b"set-cookie"),
            Bytes::from_static(b"cookie1=value1"),
        )?;
        map.append(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/html"),
        )?;

        map.append(
            Bytes::from_static(b"set-cookie"),
            Bytes::from_static(b"cookie2=value2"),
        )?;
        map.append(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/plain"),
        )?;

        map.append(
            Bytes::from_static(b"set-cookie"),
            Bytes::from_static(b"cookie3=value3"),
        )?;
        map.append(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/css"),
        )?;

        map.append(
            Bytes::from_static(b"set-cookie"),
            Bytes::from_static(b"cookie4=value4"),
        )?;
        map.append(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/xml"),
        )?;

        assert_eq!(map.get(b"set-cookie"), Some(b"cookie1=value1".as_ref()));
        assert_eq!(map.get(b"content-type"), Some(b"text/html".as_ref()));
        assert_eq!(map.values_count(), 8);
        assert_eq!(map.len(), 2);
        Ok(())
    }

    #[test]
    fn remove() -> Result<(), HeaderMapErrors> {
        let mut map = HeaderMap::new();

        map.append(
            Bytes::from_static(b"host"),
            Bytes::from_static(b"example.com"),
        )?;
        map.append(
            Bytes::from_static(b"host"),
            Bytes::from_static(b"example.org"),
        )?;
        map.append(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/html"),
        )?;
        map.append(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/plain"),
        )?;
        map.append(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/css"),
        )?;
        map.insert(
            Bytes::from_static(b"content-length"),
            Bytes::from_static(b"123"),
        )?;

        assert_eq!(map.remove(b"content-type"), Some(b"text/html".as_ref()));
        assert_eq!(map.get(b"content-type"), None);
        assert_eq!(map.len(), 2);
        assert_eq!(map.values_count(), 3);

        assert_eq!(map.remove(b"host"), Some(b"example.com".as_ref()));
        assert_eq!(map.get(b"host"), None);
        assert_eq!(map.len(), 1);
        assert_eq!(map.values_count(), 1);

        assert_eq!(map.remove(b"content-length"), Some(b"123".as_ref()));
        assert_eq!(map.get(b"content-length"), None);
        assert_eq!(map.len(), 0);
        assert_eq!(map.values_count(), 0);

        Ok(())
    }

    #[test]
    fn remove_clustered() -> Result<(), HeaderMapErrors> {
        let mut map = HeaderMap::new();

        // The ideal position for both keys will be 2 when the map capacity is 16
        map.insert(
            Bytes::from_static(b"key--1"),
            Bytes::from_static(b"value-1"),
        )?;
        map.insert(
            Bytes::from_static(b"key--2"),
            Bytes::from_static(b"value-2"),
        )?;

        assert_eq!(map.remove(b"key--1"), Some(b"value-1".as_ref()));
        assert_eq!(map.get(b"key--1"), None);

        assert_eq!(map.remove(b"key--2"), Some(b"value-2".as_ref()));
        assert_eq!(map.get(b"key--2"), None);

        assert_eq!(map.len(), 0);
        assert_eq!(map.values_count(), 0);
        Ok(())
    }

    #[test]
    fn get_all() -> Result<(), HeaderMapErrors> {
        let mut map = HeaderMap::new();
        map.append(
            Bytes::from_static(b"set-cookie"),
            Bytes::from_static(b"cookie1=value1"),
        )?;
        map.append(
            Bytes::from_static(b"set-cookie"),
            Bytes::from_static(b"cookie2=value2"),
        )?;
        map.append(
            Bytes::from_static(b"set-cookie"),
            Bytes::from_static(b"cookie3=value3"),
        )?;

        let mut values = map.get_all(b"set-cookie");
        assert_eq!(values.next(), Some(b"cookie1=value1".as_ref()));
        assert_eq!(values.next(), Some(b"cookie2=value2".as_ref()));
        assert_eq!(values.next(), Some(b"cookie3=value3".as_ref()));
        assert_eq!(values.next(), None);
        Ok(())
    }

    #[test]
    fn iter() -> Result<(), HeaderMapErrors> {
        let mut map = HeaderMap::new();
        map.append(
            Bytes::from_static(b"set-cookie"),
            Bytes::from_static(b"cookie1=value1"),
        )?;
        map.append(
            Bytes::from_static(b"set-cookie"),
            Bytes::from_static(b"cookie2=value2"),
        )?;
        map.append(
            Bytes::from_static(b"set-cookie"),
            Bytes::from_static(b"cookie3=value3"),
        )?;
        map.append(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/html"),
        )?;
        map.append(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/plain"),
        )?;
        map.append(
            Bytes::from_static(b"content-type"),
            Bytes::from_static(b"text/css"),
        )?;
        map.insert(
            Bytes::from_static(b"content-length"),
            Bytes::from_static(b"123"),
        )?;

        let mut items = vec![];

        for (key, value) in map.iter() {
            items.push((
                String::from_utf8_lossy(key).to_string(),
                String::from_utf8_lossy(value).to_string(),
            ));
        }

        assert_eq!(map.len(), 3);
        assert_eq!(map.values_count(), 7);
        assert_eq!(items.len(), 7);
        assert!(items.contains(&("set-cookie".into(), "cookie1=value1".into())));
        assert!(items.contains(&("set-cookie".into(), "cookie2=value2".into())));
        assert!(items.contains(&("set-cookie".into(), "cookie3=value3".into())));
        assert!(items.contains(&("content-type".into(), "text/html".into())));
        assert!(items.contains(&("content-type".into(), "text/plain".into())));
        assert!(items.contains(&("content-type".into(), "text/css".into())));
        assert!(items.contains(&("content-length".into(), "123".into())));
        Ok(())
    }

    #[test]
    #[ignore = "This test is non-deterministic and is meant to be run multiple times to catch edge cases in the implementation"]
    fn insert_random() -> Result<(), HeaderMapErrors> {
        let mut rng = rand::rng();

        for _ in 0..20 {
            let mut map = HeaderMap::new();
            let mut headers = vec![];

            for _ in 0..HeaderMap::MAX_SIZE {
                let key_len = rng.random_range(1..=20);
                let val_len = rng.random_range(1..=100);

                let mut key = vec![];
                let mut val = vec![];

                while key.len() < key_len {
                    let idx = rng.random_range(0..HEADER_CHARS.len());
                    let ch = HEADER_CHARS[idx];
                    if ch != 0 {
                        key.push(ch);
                    }
                }

                while val.len() < val_len {
                    let idx = rng.random_range(0..HEADER_CHARS.len());
                    let ch = HEADER_CHARS[idx];
                    if ch != 0 {
                        val.push(ch);
                    }
                }

                headers.push((key, val));
            }
            for (key, val) in headers.iter() {
                let count = headers.iter().filter(|&(k, _)| k == key).count();
                let contains = map.contains_key(key.as_ref());

                let key = Bytes::copy_from_slice(key.as_ref());
                let val = Bytes::copy_from_slice(val.as_ref());
                let ins = map.insert(key.clone(), val.clone())?;

                if count > 0 && contains {
                    if ins.is_none() {
                        dbg!(&map.indices);
                        dbg!(&map.entries);
                        dbg!(&key, &val, ins);
                        panic!("Expected Some, got None")
                    }
                    // assert!(ins.is_some());
                } else {
                    if ins.is_some() {
                        dbg!(&map.indices);
                        dbg!(&map.entries);
                        dbg!(&key, &val, ins);
                        panic!("Expected None, got Some")
                    }
                    // assert!(ins.is_none());
                }
            }
        }
        Ok(())
    }
}
