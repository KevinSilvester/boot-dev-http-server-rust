use std::hash::{DefaultHasher, Hasher};
use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use gxhash::GxHasher;
use rapidhash::v3::rapidhash_v3;

/// defualt hasher used for the `http` crate's `HeaderMap`
struct FnvHasher(u64);

impl FnvHasher {
    #[inline]
    fn new() -> Self {
        FnvHasher(0xcbf29ce484222325)
    }
}

impl std::hash::Hasher for FnvHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = self.0;
        for &b in bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.0 = hash;
    }
}

fn hash_with_fnv(data: &[u8]) -> u64 {
    let mut hasher = FnvHasher::new();
    hasher.write(data);
    hasher.finish()
}

fn hash_with_gxhash(data: &[u8]) -> u64 {
    let mut hasher = GxHasher::default();
    hasher.write(data);
    hasher.finish()
}

fn hash_with_default(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::default();
    hasher.write(data);
    hasher.finish()
}

fn hash_compare(c: &mut Criterion) {
    // let data = *b"accept-encoding";
    let data = *b"accept-encoding";

    // c.bench_function("fnv_hash", |b| b.iter(|| hash_with_fnv(black_box(&data))));
    c.bench_function("rapid_hash", |b| b.iter(|| rapidhash_v3(&data)));
    c.bench_function("gxhash_hash", |b| b.iter(|| hash_with_gxhash(&data)));
    // c.bench_function("default_hash", |b| {
    //     b.iter(|| hash_with_default(black_box(&data)))
    // });
}

const WARMUP: Duration = Duration::from_millis(1000);
const MTIME: Duration = Duration::from_millis(5000);
const SAMPLES: usize = 1000;
criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(SAMPLES).warm_up_time(WARMUP).measurement_time(MTIME);
    targets = hash_compare
}
criterion_main!(benches);
