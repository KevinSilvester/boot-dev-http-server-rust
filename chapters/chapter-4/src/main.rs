mod config;
mod connection;
mod header;
mod request;
mod simd;
mod utils;

use gxhash::GxHasher;
use smol::io::AsyncWriteExt;
use smol::net::TcpListener;
use rapidhash::v3::rapidhash_v3;

use std::hash::{DefaultHasher, Hasher};

use crate::config::ServerConfigBuilder;
use crate::connection::handle_connections;
use crate::request::Request;

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

const MAX_CAP_MASK: u64 = 8192 - 1;
const CUR_CAP_MASK: u16 = 16 - 1;

fn hash_with_gxhash(data: &[u8]) -> (u64, usize) {
    let mut hasher = GxHasher::default();
    hasher.write(data);
    let hash = hasher.finish();
    let index = ((hash & MAX_CAP_MASK) & 15) as usize;
    (hash, index)
}

fn hash_with_fnv(data: &[u8]) -> (u64, usize) {
    let mut hasher = FnvHasher::new();
    hasher.write(data);
    let hash = hasher.finish();
    let index = ((hash & MAX_CAP_MASK) & MAX_CAP_MASK) as usize;
    (hash, index)
}

fn hash_with_default(data: &[u8]) -> (u64, usize) {
    let mut hasher = DefaultHasher::default();
    hasher.write(data);
    let hash = hasher.finish();
    let index = ((hash & MAX_CAP_MASK) & 15) as usize;
    (hash, index)
}

fn hash_with_rapid(data: &[u8]) -> (u64, usize) {
    let hash = rapidhash_v3(data);
    let index = (hash & 15) as usize;
    (hash, index)
}

fn main() -> anyhow::Result<()> {
    let config = ServerConfigBuilder::new().build();

    let start = std::time::Instant::now();
    let gxhashes = [
        hash_with_gxhash(b"accept-encoding"),
        hash_with_gxhash(b"content-length"),
        hash_with_gxhash(b"content-type"),
        hash_with_gxhash(b"origin"),
        hash_with_gxhash(b"set-cookie"),
        hash_with_gxhash(b"cookie"),
        hash_with_gxhash(b"authorization"),
        hash_with_gxhash(b"accept-language"),
        hash_with_gxhash(b"user-agent"),
        hash_with_gxhash(b"accept"),
    ];
    let end = std::time::Instant::now();
    println!("{}", (end - start).as_nanos());
    

    let fnvhashes = [
        hash_with_fnv(b"accept-encoding"),
        hash_with_fnv(b"content-length"),
        hash_with_fnv(b"content-type"),
        hash_with_fnv(b"origin"),
        hash_with_fnv(b"set-cookie"),
        hash_with_fnv(b"cookie"),
        hash_with_fnv(b"authorization"),
        hash_with_fnv(b"accept-language"),
        hash_with_fnv(b"user-agent"),
        hash_with_fnv(b"accept"),
    ];

    let default_hashes = [
        hash_with_default(b"accept-encoding"),
        hash_with_default(b"content-length"),
        hash_with_default(b"content-type"),
        hash_with_default(b"origin"),
        hash_with_default(b"set-cookie"),
        hash_with_default(b"cookie"),
        hash_with_default(b"authorization"),
        hash_with_default(b"accept-language"),
        hash_with_default(b"user-agent"),
        hash_with_default(b"accept"),
    ];
    
    let start = std::time::Instant::now();
    let rapid_hashes = [
        hash_with_rapid(b"accept-encoding"),
        hash_with_rapid(b"content-length"),
        hash_with_rapid(b"content-type"),
        hash_with_rapid(b"origin"),
        hash_with_rapid(b"set-cookie"),
        hash_with_rapid(b"cookie"),
        hash_with_rapid(b"authorization"),
        hash_with_rapid(b"accept-language"),
        hash_with_rapid(b"user-agent"),
        hash_with_rapid(b"accept"),
    ];
    let end = std::time::Instant::now();
    println!("{}", (end - start).as_nanos());

    // dbg!(rapid_hashes);

    smol::block_on(async move {
        let listener = TcpListener::bind(("127.0.0.1", 42069)).await?;

        println!("Listening on {}", listener.local_addr()?);
        println!("Now start a TCP client.");

        handle_connections(listener, config).await?;

        Ok(())
    })
}
