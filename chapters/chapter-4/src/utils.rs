#![allow(dead_code)]

pub fn to_u32(bytes: &[u8]) -> u32 {
    let mut result = 0;

    for &byte in bytes {
        result <<= 8;
        result |= byte as u32;
    }
    result
}

pub fn to_u64(bytes: &[u8]) -> u64 {
    let mut result = 0;
    for &byte in bytes {
        result <<= 8;
        result |= byte as u64;
    }
    result
}
