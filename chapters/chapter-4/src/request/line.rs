use bytes::BytesMut;

#[derive(Debug)]
pub enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    PATCH,
}

#[derive(Debug)]
pub enum HttpVersion {
    HTTP1_1,
}

#[derive(Debug)]
pub struct RequestLine {
    pub method: RequestMethod,
    pub request_target: BytesMut,
    pub http_version: HttpVersion,
}
