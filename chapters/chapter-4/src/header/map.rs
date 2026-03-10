//! A simplified version of `HeaderMap` implemented in `actix-http`
//! A multi-map of `HeaderName` to one or more `HeaderValue`s.
//! This is a trimmed down and altered version of the `HeaderMap` found the in the `actix-http`
//! crate.
//!
//! <https://github.com/actix/actix-web/blob/main/actix-http/src/header/map.rs>
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

use std::collections::hash_map::Entry;

use arrayvec::ArrayVec;
use gxhash::HashMap;

use super::*;

/// There isn't a clear limit to the now many value should be allowed, but I'm limiting it to
/// 4 because that's the limit in the `actix-http` crate
const MAP_VALUES_PER_HEADER: usize = 4;

/// A helper trait for converting various types into `HeaderName` and `HeaderValue`
pub trait HeaderThing<T> {
    type Error;

    fn try_to_thing(self) -> Result<T, Self::Error>;
}

impl HeaderThing<HeaderName> for &[u8] {
    type Error = InvalidHeaderName;

    #[inline]
    fn try_to_thing(self) -> Result<HeaderName, Self::Error> {
        HeaderName::from_bytes(self)
    }
}

impl HeaderThing<HeaderName> for &str {
    type Error = InvalidHeaderName;

    #[inline]
    fn try_to_thing(self) -> Result<HeaderName, Self::Error> {
        HeaderName::from_bytes(self.as_bytes())
    }
}

impl HeaderThing<HeaderName> for HeaderName {
    type Error = ();

    #[inline]
    fn try_to_thing(self) -> Result<HeaderName, Self::Error> {
        Ok(self)
    }
}

impl HeaderThing<HeaderValue> for &[u8] {
    type Error = InvalidHeaderValue;

    #[inline]
    fn try_to_thing(self) -> Result<HeaderValue, Self::Error> {
        HeaderValue::from_bytes(self)
    }
}

impl HeaderThing<HeaderValue> for &str {
    type Error = InvalidHeaderValue;

    #[inline]
    fn try_to_thing(self) -> Result<HeaderValue, Self::Error> {
        HeaderValue::from_bytes(self.as_bytes())
    }
}

impl HeaderThing<HeaderValue> for HeaderValue {
    type Error = ();

    #[inline]
    fn try_to_thing(self) -> Result<HeaderValue, Self::Error> {
        Ok(self)
    }
}

macro_rules! try_to_thing {
    ($thing:expr, $error_msg:literal) => {{
        match $thing.try_to_thing() {
            Ok(t) => t,
            Err(_) => anyhow::bail!($error_msg),
        }
    }};
}

/// A simplified version of `HeaderMap` implemented in `actix-http`
/// ref: https://github.com/actix/actix-web/blob/main/actix-http/src/header/map.rs
#[derive(Debug, Clone, Default)]
pub struct HeaderMap {
    inner: HashMap<HeaderName, Value>,
}

#[derive(Debug, Clone, Default)]
pub struct Value {
    inner: ArrayVec<HeaderValue, MAP_VALUES_PER_HEADER>,
}

impl Value {
    pub fn one(val: HeaderValue) -> Self {
        let mut inner = ArrayVec::new();
        inner.push(val);
        Self { inner }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn append(&mut self, value: HeaderValue) {
        self.inner.push(value);
    }

    pub fn nth(&self, index: usize) -> Option<&HeaderValue> {
        if self.inner.is_empty() || index > self.len() {
            return None;
        }
        Some(&self.inner[index])
    }

    pub fn remove(&mut self, index: usize) -> Option<HeaderValue> {
        if self.inner.is_empty() || index > self.len() {
            return None;
        }
        Some(self.inner.remove(index))
    }
}

impl FromIterator<HeaderValue> for Value {
    fn from_iter<T: IntoIterator<Item = HeaderValue>>(iter: T) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl HeaderMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.inner.values().map(|vals| vals.len()).sum()
    }

    pub fn len_keys(&self) -> usize {
        self.inner.len()
    }

    pub fn len_vals(&self, name: impl HeaderThing<HeaderName>) -> anyhow::Result<usize> {
        let name = match name.try_to_thing() {
            Ok(n) => n,
            Err(_) => anyhow::bail!("Invalid HeaderName"),
        };
        match self.inner.get(&name) {
            Some(vals) => Ok(vals.len()),
            None => anyhow::bail!("Unknown Header"),
        }
    }

    pub fn contains_key(&self, name: impl HeaderThing<HeaderName>) -> bool {
        match name.try_to_thing() {
            Ok(n) => self.inner.contains_key(&n),
            Err(_) => false,
        }
    }

    pub fn insert(
        &mut self,
        name: impl HeaderThing<HeaderName>,
        value: impl HeaderThing<HeaderValue>,
    ) -> anyhow::Result<Option<Value>> {
        let name = try_to_thing!(name, "Invalid HeaderName");
        let value = try_to_thing!(value, "Invalid HeaderValue");

        let inserted = self.inner.insert(name, Value::one(value));
        Ok(inserted)
    }

    pub fn append(
        &mut self,
        name: impl HeaderThing<HeaderName>,
        value: impl HeaderThing<HeaderValue>,
    ) -> anyhow::Result<()> {
        let name = try_to_thing!(name, "Invalid HeaderName");
        let value = try_to_thing!(value, "Invalid HeaderValue");

        match self.inner.entry(name) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().append(value);
            }
            Entry::Vacant(entry) => {
                entry.insert(Value::one(value));
            }
        };

        Ok(())
    }

    pub fn retain(&mut self, mut retain_fn: impl FnMut(&HeaderName, &mut HeaderValue) -> bool) {
        self.inner.retain(|name, vals| {
            vals.inner.retain(|val| retain_fn(name, val));
            !vals.inner.is_empty()
        });
    }

    pub fn get(
        &self,
        name: impl HeaderThing<HeaderName>,
        index: usize,
    ) -> anyhow::Result<Option<&HeaderValue>> {
        let name = try_to_thing!(name, "Invalid HeaderName");

        match self.inner.get(&name) {
            Some(vals) => Ok(vals.nth(index)),
            None => Ok(None),
        }
    }

    pub fn get_all(
        &self,
        name: impl HeaderThing<HeaderName>,
    ) -> anyhow::Result<std::slice::Iter<'_, HeaderValue>> {
        let name = try_to_thing!(name, "Invalid HeaderName");

        match self.inner.get(&name) {
            Some(values) => Ok(values.inner.iter()),
            None => Ok([].iter()),
        }
    }

    pub fn remove(&mut self, name: impl HeaderThing<HeaderName>) -> anyhow::Result<Option<Value>> {
        let name = try_to_thing!(name, "Invalid HeaderName");

        Ok(self.inner.remove(&name))
    }

    pub fn remove_val(
        &mut self,
        name: impl HeaderThing<HeaderName>,
        index: usize,
    ) -> anyhow::Result<Option<HeaderValue>> {
        let name = try_to_thing!(name, "Invalid HeaderName");

        match self.inner.get_mut(&name) {
            Some(vals) => Ok(vals.remove(index)),
            None => Ok(None),
        }
    }
}

/// Trimmed down and modified unit tests from the original `HeaderMap` tests in the `actix-http`
/// crate
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create() {
        let map = HeaderMap::new();
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn insert() {
        let mut map = HeaderMap::new();

        map.insert(common_headers::LOCATION, "/test").unwrap();
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn contains() {
        let mut map = HeaderMap::new();
        assert!(!map.contains_key(common_headers::LOCATION));

        map.insert(common_headers::LOCATION, "/test").unwrap();
        assert!(map.contains_key(common_headers::LOCATION));
        assert!(map.contains_key(&b"Location"[..]));
        assert!(map.contains_key("Location"));
    }

    #[test]
    fn retain() {
        let mut map = HeaderMap::new();

        map.append(common_headers::LOCATION, "/test").unwrap();
        map.append(common_headers::HOST, "duck.com").unwrap();
        map.append(common_headers::COOKIE, "one=1").unwrap();
        map.append(common_headers::COOKIE, "two=2").unwrap();

        assert_eq!(map.len(), 4);

        // by value
        map.retain(|_, val| !val.as_bytes().contains(&b'/'));
        assert_eq!(map.len(), 3);

        // by name
        map.retain(|name, _| name.as_str() != "cookie");
        assert_eq!(map.len(), 1);

        // keep but mutate value
        map.retain(|_, val| {
            *val = HeaderValue::from_static("replaced");
            true
        });
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("host", 0).unwrap().unwrap(), "replaced");
    }

    #[test]
    fn retain_removes_empty_value_lists() {
        let mut map = HeaderMap::new();

        map.append(common_headers::HOST, "duck.com").unwrap();
        map.append(common_headers::HOST, "duck.com").unwrap();

        assert_eq!(map.len(), 2);
        assert_eq!(map.len_keys(), 1);
        assert_eq!(map.inner.len(), 1);

        // remove everything
        map.retain(|_n, _v| false);

        assert_eq!(map.len(), 0);
        assert_eq!(map.len_keys(), 0);
        assert_eq!(map.inner.len(), 0);
    }

    #[test]
    fn get_all_iteration_order_matches_insertion_order() {
        let mut map = HeaderMap::new();

        let mut vals = map.get_all(common_headers::COOKIE).unwrap();
        assert!(vals.next().is_none());

        map.append(common_headers::COOKIE, "1").unwrap();
        let mut vals = map.get_all(common_headers::COOKIE).unwrap();
        assert_eq!(vals.next().unwrap().as_bytes(), b"1");
        assert!(vals.next().is_none());

        map.append(common_headers::COOKIE, "2").unwrap();
        let mut vals = map.get_all(common_headers::COOKIE).unwrap();
        assert_eq!(vals.next().unwrap().as_bytes(), b"1");
        assert_eq!(vals.next().unwrap().as_bytes(), b"2");
        assert!(vals.next().is_none());

        map.append(common_headers::COOKIE, "3").unwrap();
        map.append(common_headers::COOKIE, "4").unwrap();
        map.append(common_headers::COOKIE, "5").unwrap();
        let mut vals = map.get_all(common_headers::COOKIE).unwrap();
        assert_eq!(vals.next().unwrap().as_bytes(), b"1");
        assert_eq!(vals.next().unwrap().as_bytes(), b"2");
        assert_eq!(vals.next().unwrap().as_bytes(), b"3");
        assert_eq!(vals.next().unwrap().as_bytes(), b"4");
        assert_eq!(vals.next().unwrap().as_bytes(), b"5");
        assert!(vals.next().is_none());

        let _ = map.insert(common_headers::COOKIE, "6");
        let mut vals = map.get_all(common_headers::COOKIE).unwrap();
        assert_eq!(vals.next().unwrap().as_bytes(), b"6");
        assert!(vals.next().is_none());

        let _ = map.insert(common_headers::COOKIE, "7");
        let _ = map.insert(common_headers::COOKIE, "8");
        let mut vals = map.get_all(common_headers::COOKIE).unwrap();
        assert_eq!(vals.next().unwrap().as_bytes(), b"8");
        assert!(vals.next().is_none());

        map.append(common_headers::COOKIE, "9").unwrap();
        let mut vals = map.get_all(common_headers::COOKIE).unwrap();
        assert_eq!(vals.next().unwrap().as_bytes(), b"8");
        assert_eq!(vals.next().unwrap().as_bytes(), b"9");
        assert!(vals.next().is_none());

        // check for fused-ness
        assert!(vals.next().is_none());
    }

    #[test]
    fn remove() {
        let mut map = HeaderMap::new();

        map.append(common_headers::LOCATION, "/test").unwrap();
        map.append(common_headers::HOST, "duck.com").unwrap();
        map.append(common_headers::HOST, "duck.com").unwrap();
        map.append(common_headers::COOKIE, "one=1").unwrap();
        map.append(common_headers::COOKIE, "two=2").unwrap();
        map.append(common_headers::COOKIE, "two=3").unwrap();

        assert_eq!(map.len(), 6);
        assert_eq!(map.len_keys(), 3);
        assert_eq!(map.inner.len(), 3);

        let removed = map.remove(common_headers::HOST).unwrap().unwrap();

        assert_eq!(removed.len(), 2);
        assert_eq!(map.len(), 4);
        assert_eq!(map.len_keys(), 2);
        assert_eq!(map.inner.len(), 2);

        let removed_val = map.remove_val(common_headers::COOKIE, 1).unwrap().unwrap();

        assert_eq!(removed_val.as_bytes(), b"two=2");
        assert_eq!(map.len(), 3);
        assert_eq!(map.len_keys(), 2);
        assert_eq!(map.inner.len(), 2);
    }
}
