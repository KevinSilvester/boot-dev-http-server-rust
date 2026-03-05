use bytes::Bytes;

#[derive(Debug)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    PATCH,
}

#[derive(Debug)]
pub enum Version {
    HTTP1_1,
}

#[derive(Debug)]
pub struct Line {
    method: Method,
    version: Version,
    target: Bytes,
}

