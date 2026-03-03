#![allow(unused_imports)]

use anyhow::Context;
use async_channel::{Receiver, Sender, bounded};
use smol::io::{AsyncRead, AsyncReadExt};
use thiserror::Error;

pub trait TReader: AsyncRead + Unpin + Send + 'static {}
impl<T> TReader for T where T: AsyncRead + Unpin + Send + 'static {}

#[derive(Debug)]
pub struct Request<'r> {
    request_line: RequestLine<'r>,
}

#[derive(Debug)]
pub struct RequestLine<'r> {
    method: &'r [u8],
    request_target: &'r [u8],
    http_version: &'r [u8],
}

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("Invalid line ending")]
    InvalidLineEnding,

    #[error("Malformed request line")]
    MalformedRequestLine,

    #[error("Invalid request method")]
    InvalidRequestMethod,

    #[error("Unsupported HTTP version")]
    UnsupportedHttpVersion,

    #[error("Request parsing error")]
    RequestParsingError,
}

#[derive(Debug, Clone)]
pub enum RequestParserState {
    RequestLine,
    Headers,
    Body,
    Done,
}

#[derive(Debug, Clone)]
pub struct RequestParser {
    state: RequestParserState,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
}

impl RequestParser {
    pub fn new() -> Self {
        let (tx, rx) = bounded::<Vec<u8>>(32);
        Self {
            state: RequestParserState::RequestLine,
            tx,
            rx,
        }
    }

    pub fn read_lines_stream(self, mut reader: impl TReader) -> anyhow::Result<()> {
        smol::spawn(async move {
            let mut buffer = [0u8; 8];
            let mut line = vec![];

            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) if line.is_empty() => break,
                    Ok(0) => {
                        self.tx.send(line).await?;
                        break;
                    }
                    Ok(n) => {
                        for &byte in &buffer[..n] {
                            if byte == b'\n' {
                                self.tx.send(std::mem::take(&mut line)).await?;
                            } else {
                                line.push(byte);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            Ok::<_, anyhow::Error>(())
        })
        .detach();

        Ok(())
    }

    pub async fn parse<'r>(&mut self, mut reader: impl TReader) -> anyhow::Result<Request<'r>> {
        let mut request_line: Option<RequestLine> = None;

        while let Ok(line) = self.rx.recv().await {
            if line.is_empty() || line[line.len() - 1] != b'\r' {
                return Err(RequestError::InvalidLineEnding)?;
            }

            let line = &line[..line.len() - 1];

            match self.state {
                RequestParserState::RequestLine => {
                    request_line = Some(parse_request_line(&line)?);
                    self.state = RequestParserState::Headers;
                }
                RequestParserState::Headers => {
                    self.state = RequestParserState::Body;
                }
                RequestParserState::Body => {
                    self.state = RequestParserState::Done;
                }
                RequestParserState::Done => {
                    break;
                }
            };
        }

        Ok(Request {
            request_line: request_line.context(RequestError::RequestParsingError)?,
        })
    }
}

// METHOD
fn parse_request_line(line: &[u8]) -> anyhow::Result<RequestLine<'_>> {
    let mut parts = line.split(|b| *b == b' ');

    let method = parts.next().context(RequestError::MalformedRequestLine)?;
    let request_target = parts.next().context(RequestError::MalformedRequestLine)?;
    let version = parts.next().context(RequestError::MalformedRequestLine)?;

    if parts.next().is_some() {
        Err(RequestError::MalformedRequestLine)?;
    }

    match method {
        b"GET" | b"POST" | b"PUT" | b"DELETE" | b"HEAD" | b"OPTIONS" | b"PATCH" => {}
        _ => return Err(RequestError::InvalidRequestMethod)?,
    }

    match version {
        b"HTTP/1.1" => {}
        _ => return Err(RequestError::UnsupportedHttpVersion)?,
    }

    Ok(RequestLine {
        method,
        request_target,
        http_version: &version[5..],
    })
}

pub async fn request_from_reader(reader: impl TReader) -> anyhow::Result<Request<'static>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol::io::Cursor;

    #[test]
    fn test_good_get_request_line() {
        smol::block_on(async move {
            let data = b"GET / HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
            let cursor = Cursor::new(data);

            let r = request_from_reader(cursor).await;
            assert!(r.is_ok());

            let r = r.unwrap();
            assert_eq!(r.request_line.method, b"GET");
            assert_eq!(r.request_line.request_target, b"/");
            assert_eq!(r.request_line.http_version, b"1.1");
        })
    }

    #[test]
    fn test_good_get_request_line_with_path() {
        smol::block_on(async move {
            let data = b"GET /coffee HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
            let cursor = Cursor::new(data);

            let r = request_from_reader(cursor).await;
            assert!(r.is_ok());

            let r = r.unwrap();
            assert_eq!(r.request_line.method, b"GET");
            assert_eq!(r.request_line.request_target, b"/path/to/resource");
            assert_eq!(r.request_line.http_version, b"1.0");
        })
    }

    #[test]
    fn test_invalid_number_of_parts() {
        smol::block_on(async move {
            let data = b"/coffee HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
            let cursor = Cursor::new(data);

            let r = request_from_reader(cursor).await;
            assert!(r.is_err());
        })
    }
}
