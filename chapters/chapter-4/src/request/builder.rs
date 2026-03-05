use anyhow::Context;
use bytes::BytesMut;
use thiserror::Error;

use super::{HttpVersion, RequestLine, RequestMethod};

/// ref: https://community.cloudflare.com/t/maximum-on-http-header-values/424067/3
const MAX_REQUEST_LINE_SIZE: usize = 8 << 10;
const MIN_REQUEST_LINE_SIZE: usize = 14; // e.g. "GET / HTTP/1.1\r\n"

const MAX_HEADER_SIZE: usize = 32 << 10;
const MAX_HEADER_LINE_SIZE: usize = 8 << 10;

#[derive(Debug, Clone)]
pub enum RequestParserState {
    RequestLine,
    Headers,
    Body,
    Done,
}

#[derive(Debug, Error)]
pub enum RequestParserError {
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

    #[error("Request line too long")]
    RequestLineTooLong,

    #[error("Header too large")]
    HeaderTooLarge,

    #[error("Header line too long")]
    HeaderLineTooLong,

    #[error("Body too large")]
    BodyTooLarge,
}

#[derive(Debug)]
pub struct RequestParser {
    state: RequestParserState,
    max_body_size: usize,
    pub request_line: Option<RequestLine>,
}

impl RequestParser {
    pub fn new(max_body_size: usize) -> Self {
        Self {
            state: RequestParserState::RequestLine,
            max_body_size,
            request_line: None,
        }
    }

    pub fn done(&self) -> bool {
        matches!(self.state, RequestParserState::Done)
    }

    pub fn parse_request_line(line: &[u8]) -> anyhow::Result<RequestLine> {
        let mut parts = line.split(|b| *b == b' ');

        let method = parts
            .next()
            .context(RequestParserError::MalformedRequestLine)?;
        let request_target = parts
            .next()
            .context(RequestParserError::MalformedRequestLine)?;
        let version = parts
            .next()
            .context(RequestParserError::MalformedRequestLine)?;

        if parts.next().is_some() {
            Err(RequestParserError::MalformedRequestLine)?;
        }

        let method = match method {
            b"GET" => RequestMethod::GET,
            b"POST" => RequestMethod::POST,
            b"PUT" => RequestMethod::PUT,
            b"DELETE" => RequestMethod::DELETE,
            b"HEAD" => RequestMethod::HEAD,
            b"OPTIONS" => RequestMethod::OPTIONS,
            b"PATCH" => RequestMethod::PATCH,
            _ => return Err(RequestParserError::InvalidRequestMethod)?,
        };

        let request_target = BytesMut::from(request_target);

        let version = match version {
            b"HTTP/1.1" => HttpVersion::Http11,
            _ => return Err(RequestParserError::UnsupportedHttpVersion)?,
        };

        Ok(RequestLine {
            method,
            request_target,
            http_version: version,
        })
    }

    fn line_end_pos(&self, buf: &[u8], limit: usize) -> anyhow::Result<usize> {
        let lf = match memchr::memchr(b'\n', buf) {
            Some(nl) => nl,
            None => return Ok(0),
        };
        let cr = lf - 1;

        if lf > limit {
            match self.state {
                RequestParserState::RequestLine => Err(RequestParserError::RequestLineTooLong)?,
                RequestParserState::Headers => Err(RequestParserError::HeaderLineTooLong)?,
                _ => Err(RequestParserError::BodyTooLarge)?,
            }
        }

        if lf < 1 || buf[cr] != b'\r' {
            return Err(RequestParserError::InvalidLineEnding.into());
        }

        Ok(cr)
    }

    pub fn parse(&mut self, buf: &[u8]) -> anyhow::Result<usize> {
        let mut read = 0;

        loop {
            match self.state {
                RequestParserState::RequestLine => {
                    let line_end = self.line_end_pos(buf, MAX_REQUEST_LINE_SIZE)?;
                    if line_end < MIN_REQUEST_LINE_SIZE {
                        break;
                    }
                    read += line_end + 2; // +2 for \r\n
                    self.request_line = Some(Self::parse_request_line(&buf[..line_end])?);
                    self.state = RequestParserState::Headers;
                }
                RequestParserState::Headers => {
                    self.state = RequestParserState::Body;
                }
                RequestParserState::Body => {
                    self.state = RequestParserState::Done;
                }
                RequestParserState::Done => break,
            }
        }

        Ok(read)
    }
}
