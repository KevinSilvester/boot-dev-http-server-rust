mod body;
mod header;
mod line;
mod builder;

use std::net::SocketAddr;
use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use smol::io::AsyncReadExt;
use smol::lock::Mutex;
use smol::net::TcpStream;

pub use self::line::{Line /* Method, Version */};
pub use self::builder::{RequestParserState, RequestParserError, RequestParser};

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
    Http11,
}

#[derive(Debug)]
pub struct RequestLine {
    pub method: RequestMethod,
    pub request_target: BytesMut,
    pub http_version: HttpVersion,
}

#[derive(Debug)]
pub struct Request {
    bytes: BytesMut,
    pub peer_address: SocketAddr,
    pub request_line: Line,
}

// TODO: add request timeouts
impl Request {
    // #[allow(clippy::new_ret_no_self)]
    // pub async fn new(
    //     mut stream: TcpStream,
    //     _peer_address: SocketAddr,
    //     max_size: usize,
    // ) -> anyhow::Result<()> {
    //     let buffer = Arc::new(Mutex::new(BytesMut::with_capacity(max_size)));
    //     let buffer_clone = Arc::clone(&buffer);
    //     let (tx, rx) = async_channel::bounded::<(usize, usize)>(32);

    //     smol::spawn(async move {
    //         let allowed_size = max_size + MAX_REQUEST_LINE_SIZE + MAX_HEADER_SIZE;

    //         let mut stream_buffer = [0u8; 1024];
    //         let mut total_bytes_read = 0;
    //         let mut line_start = 0;
    //         let mut line_end = 0;

    //         loop {
    //             match stream.read(&mut stream_buffer).await {
    //                 Ok(0) => break, // EOF
    //                 Ok(n) => {
    //                     if total_bytes_read + n > allowed_size {
    //                         anyhow::bail!("request too large");
    //                     }
    //                     let mut buffer = buffer_clone.lock().await;
    //                     let new_lines = memchr::memchr_iter(b'\n', &stream_buffer[..n]);

    //                     buffer.extend_from_slice(&stream_buffer[..n]);

    //                     for pos in new_lines {
    //                         if total_bytes_read + pos == 0
    //                             || buffer[total_bytes_read + pos - 1] != b'\r'
    //                         {
    //                             anyhow::bail!("request malformed")
    //                         }

    //                         if line_end != 0 {
    //                             line_start = line_end + 2;
    //                         }
    //                         line_end = total_bytes_read + pos - 1;

    //                         tx.send((line_start, line_end)).await?;
    //                     }
    //                     total_bytes_read += n;
    //                 }
    //                 Err(_) => anyhow::bail!("error reading from stream"),
    //             }
    //         }

    //         tx.close();
    //         Ok::<_, anyhow::Error>(())
    //     })
    //     .detach();

    //     let bytes = Arc::clone(&buffer);

    //     while let Ok((line_start, line_end)) = rx.recv().await {
    //         let line = &bytes.lock().await[line_start..line_end];
    //         println!("{:?}", String::from_utf8_lossy(line));
    //     }
    //     Ok(())
    // }
}

// #![allow(unused_imports)]

// use std::io::Bytes;

// use anyhow::Context;
// use async_channel::{Receiver, Sender, bounded};
// use bytes::BytesMut;
// use smol::io::{AsyncRead, AsyncReadExt};
// use thiserror::Error;

// pub trait TReader: AsyncRead + Unpin + Send + 'static {}
// impl<T> TReader for T where T: AsyncRead + Unpin + Send + 'static {}

// #[derive(Debug)]
// pub struct Request<'r> {
//     request_bytes: BytesMut,
//     request_line: RequestLine<'r>,
// }

// #[derive(Debug)]
// pub struct RequestLine<'r> {
//     method: &'r [u8],
//     request_target: &'r [u8],
//     http_version: &'r [u8],
// }

// #[derive(Debug)]
// pub struct RequestHeader<'r> {
//     name: &'r [u8],
//     value: &'r [u8],
// }

// #[derive(Debug, Error)]
// pub enum RequestError {
//     #[error("Invalid line ending")]
//     InvalidLineEnding,

//     #[error("Malformed request line")]
//     MalformedRequestLine,

//     #[error("Invalid request method")]
//     InvalidRequestMethod,

//     #[error("Unsupported HTTP version")]
//     UnsupportedHttpVersion,

//     #[error("Request parsing error")]
//     RequestParsingError,
// }

// #[derive(Debug, Clone)]
// pub enum RequestParserState {
//     RequestLine,
//     Headers,
//     Body,
//     Done,
// }

// // METHOD
// fn parse_request_line(line: &[u8]) -> anyhow::Result<RequestLine<'_>> {
//     let mut parts = line.split(|b| *b == b' ');

//     let method = parts.next().context(RequestError::MalformedRequestLine)?;
//     let request_target = parts.next().context(RequestError::MalformedRequestLine)?;
//     let version = parts.next().context(RequestError::MalformedRequestLine)?;

//     if parts.next().is_some() {
//         Err(RequestError::MalformedRequestLine)?;
//     }

//     match method {
//         b"GET" | b"POST" | b"PUT" | b"DELETE" | b"HEAD" | b"OPTIONS" | b"PATCH" => {}
//         _ => return Err(RequestError::InvalidRequestMethod)?,
//     }

//     match version {
//         b"HTTP/1.1" => {}
//         _ => return Err(RequestError::UnsupportedHttpVersion)?,
//     }

//     Ok(RequestLine {
//         method,
//         request_target,
//         http_version: &version[5..],
//     })
// }

// pub async fn request_from_reader(reader: impl TReader) -> anyhow::Result<Request<'static>> {
//     todo!()
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use smol::io::Cursor;

//     #[test]
//     fn test_good_get_request_line() {
//         smol::block_on(async move {
//             let data = b"GET / HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
//             let cursor = Cursor::new(data);

//             let r = request_from_reader(cursor).await;
//             assert!(r.is_ok());

//             let r = r.unwrap();
//             assert_eq!(r.request_line.method, b"GET");
//             assert_eq!(r.request_line.request_target, b"/");
//             assert_eq!(r.request_line.http_version, b"1.1");
//         })
//     }

//     #[test]
//     fn test_good_get_request_line_with_path() {
//         smol::block_on(async move {
//             let data = b"GET /coffee HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
//             let cursor = Cursor::new(data);

//             let r = request_from_reader(cursor).await;
//             assert!(r.is_ok());

//             let r = r.unwrap();
//             assert_eq!(r.request_line.method, b"GET");
//             assert_eq!(r.request_line.request_target, b"/path/to/resource");
//             assert_eq!(r.request_line.http_version, b"1.0");
//         })
//     }

//     #[test]
//     fn test_invalid_number_of_parts() {
//         smol::block_on(async move {
//             let data = b"/coffee HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
//             let cursor = Cursor::new(data);

//             let r = request_from_reader(cursor).await;
//             assert!(r.is_err());
//         })
//     }
// }
