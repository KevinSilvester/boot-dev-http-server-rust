use anyhow::Context;
use bytes::BytesMut;
use thiserror::Error;

use super::line::{HttpVersion, RequestLine, RequestMethod};

/// The request line is composed of 3 sections (excluding the \r\n new-line delimiter).
/// 1. The method (e.g. GET, POST, etc.)
/// 2. The request target (e.g. /, /path/to/resource, etc
/// 3. The HTTP version (e.g. HTTP/1.1, HTTP/2.0, etc.)
///
/// The maximum combined bytes lengths of the method (OPTIONS), HTTP version (HTTP/1.1) and the
/// 2 space characters in between the sections is 17B.
/// This leaves the rest of the 2KiB for the request target, which should be plenty.
/// Ideally the this length should be configurable and would default to the longest possible
/// request-endpoint path of the server.
const MAX_REQUEST_LINE_SIZE: usize = 2 << 10;
const MIN_REQUEST_LINE_SIZE: usize = 14; // "GET / HTTP/1.1\r\n"

/// ref: https://community.cloudflare.com/t/maximum-on-http-header-values/424067/3
const MAX_HEADER_SIZE: usize = 32 << 10;
const MAX_HEADER_LINE_SIZE: usize = 8 << 10;
const MIN_HEADER_LINE_SIZE: usize = 3; // e.g. "A: \r\n"
//
const GET_: u32 = u32::from_ne_bytes(*b"GET ");
const PUT_: u32 = u32::from_ne_bytes(*b"PUT ");
const POST: u32 = u32::from_ne_bytes(*b"POST");
const HEAD: u32 = u32::from_ne_bytes(*b"HEAD");
const PATC: u32 = u32::from_ne_bytes(*b"PATC");
const DELE: u32 = u32::from_ne_bytes(*b"DELE");
const OPTI: u32 = u32::from_ne_bytes(*b"OPTI");

const HTTP_1_1: u64 = u64::from_ne_bytes(*b"HTTP/1.1");

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

    #[error("Header line malformed")]
    MalformedHeaderLine,

    #[error("Body too large")]
    BodyTooLarge,

    #[error("Body missing bytes: {0}")]
    BodyMissingBytes(usize),
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

    fn parse_request_line(line: &[u8]) -> anyhow::Result<RequestLine> {
        let mut spaces = memchr::memchr_iter(b' ', line);
        let sp_1 = spaces
            .next()
            .context(RequestParserError::MalformedRequestLine)?;
        let sp_2 = spaces
            .next()
            .context(RequestParserError::MalformedRequestLine)?;

        if spaces.next().is_some() {
            return Err(RequestParserError::MalformedRequestLine.into());
        }

        // - The first space must be at least at the 3th index
        // - The request target must be at least 1 byte (/).
        // - The HTTP version must be at least 8 bytes (HTTP/1.1).
        if sp_1 < 3 || (sp_2 - sp_1) < 2 || (line.len() - sp_2) != 9 {
            return Err(RequestParserError::MalformedRequestLine.into());
        }

        let method = u32::from_ne_bytes([line[0], line[1], line[2], line[3]]);
        let request_target = &line[sp_1 + 1..sp_2];
        let http_version = u64::from_ne_bytes([
            line[sp_2 + 1],
            line[sp_2 + 2],
            line[sp_2 + 3],
            line[sp_2 + 4],
            line[sp_2 + 5],
            line[sp_2 + 6],
            line[sp_2 + 7],
            line[sp_2 + 8],
        ]);

        let method = match method {
            GET_ => RequestMethod::GET,
            PUT_ => RequestMethod::PUT,
            POST => RequestMethod::POST,
            HEAD => RequestMethod::HEAD,
            PATC if line[4] == b'H' => RequestMethod::PATCH,
            DELE if line[4] == b'T' && line[5] == b'E' => RequestMethod::DELETE,
            OPTI if line[4] == b'O' && line[5] == b'N' && line[6] == b'S' => RequestMethod::OPTIONS,
            _ => return Err(RequestParserError::InvalidRequestMethod)?,
        };

        // TODO: validate request target (e.g. no spaces, no control characters, etc.)
        let request_target = BytesMut::from(request_target);

        let http_version = match http_version {
            HTTP_1_1 => HttpVersion::HTTP_1_1,
            _ => return Err(RequestParserError::UnsupportedHttpVersion)?,
        };

        Ok(RequestLine {
            method,
            request_target,
            http_version,
        })
    }

    fn line_end_pos(&self, buf: &[u8], max_size: usize, min_size: usize) -> anyhow::Result<usize> {
        let lf = match memchr::memchr(b'\n', buf) {
            Some(nl) => nl,
            None => return Ok(0),
        };

        if lf > max_size - 1 {
            Err(match self.state {
                RequestParserState::RequestLine => RequestParserError::RequestLineTooLong,
                RequestParserState::Headers => RequestParserError::HeaderLineTooLong,
                _ => unreachable!(),
            })?
        }

        if lf < min_size - 1 {
            Err(match self.state {
                RequestParserState::RequestLine => RequestParserError::MalformedRequestLine,
                RequestParserState::Headers => RequestParserError::MalformedHeaderLine,
                _ => unreachable!(),
            })?
        }

        if lf == 0 || buf[lf - 1] != b'\r' {
            return Err(RequestParserError::InvalidLineEnding)?;
        }

        Ok(lf)
    }

    pub fn parse(&mut self, buf: &[u8]) -> anyhow::Result<usize> {
        let mut read = 0;

        loop {
            match self.state {
                RequestParserState::RequestLine => {
                    let line_end =
                        self.line_end_pos(buf, MAX_REQUEST_LINE_SIZE, MIN_REQUEST_LINE_SIZE)?;
                    if line_end == 0 {
                        break;
                    }
                    read += line_end;
                    self.request_line = Some(Self::parse_request_line(&buf[..line_end - 1])?);
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

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_REQUEST_LINE_GET: &[u8] = b"GET / HTTP/1.1";
    const GOOD_REQUEST_LINE_POST: &[u8] = b"POST / HTTP/1.1";
    const GOOD_REQUEST_LINE_HEAD: &[u8] = b"HEAD / HTTP/1.1";
    const GOOD_REQUEST_LINE_PATCH: &[u8] = b"PATCH / HTTP/1.1";
    const GOOD_REQUEST_LINE_DELETE: &[u8] = b"DELETE / HTTP/1.1";
    const GOOD_REQUEST_LINE_OPTIONS: &[u8] = b"OPTIONS / HTTP/1.1";
    const GOOD_REQUEST_LINE_WITH_PATH: &[u8] = b"POST /path/to/resource HTTP/1.1";

    const BAD_REQUEST_LINE_INVALID_METHOD: &[u8] = b"FOO / HTTP/1.1";
    const BAD_REQUEST_LINE_UNSUPPORTED_HTTP_VERSION: &[u8] = b"GET / HTTP/2.0";
    const BAD_REQUEST_LINE_TOO_MANY_PARTS: &[u8] = b"GET / HTTP/1.1 extra";
    const BAD_REQUEST_LINE_INVALID_TARGET: &[u8] = b"GET --hello<'-'>bye-- HTTP/1.1";
    const BAD_REQUEST_LINE_TOO_LONG: [u8; MAX_REQUEST_LINE_SIZE + 1] = build_bad_request_line();

    const fn build_bad_request_line() -> [u8; MAX_REQUEST_LINE_SIZE + 1] {
        let mut line = [b'a'; MAX_REQUEST_LINE_SIZE + 1];

        line[0] = b'G';
        line[1] = b'E';
        line[2] = b'T';
        line[3] = b' ';
        line[4] = b'/';

        let i = MAX_REQUEST_LINE_SIZE + 1 - 11;

        line[i] = b' ';
        line[i + 1] = b'H';
        line[i + 2] = b'T';
        line[i + 3] = b'T';
        line[i + 4] = b'P';
        line[i + 5] = b'/';
        line[i + 6] = b'1';
        line[i + 7] = b'.';
        line[i + 8] = b'1';
        line[i + 9] = b'\r';
        line[i + 10] = b'\n';

        line
    }

    #[test]
    fn parse_good_request_line_get() {
        let request_line = RequestParser::parse_request_line(GOOD_REQUEST_LINE_GET).unwrap();
        assert!(matches!(request_line.method, RequestMethod::GET));
        assert_eq!(&request_line.request_target[..], b"/");
        assert!(matches!(request_line.http_version, HttpVersion::HTTP_1_1));
    }

    #[test]
    fn parse_good_request_line_post() {
        let request_line = RequestParser::parse_request_line(GOOD_REQUEST_LINE_POST).unwrap();
        assert!(matches!(request_line.method, RequestMethod::POST));
        assert_eq!(&request_line.request_target[..], b"/");
        assert!(matches!(request_line.http_version, HttpVersion::HTTP_1_1));
    }

    #[test]
    fn parse_good_request_line_head() {
        let request_line = RequestParser::parse_request_line(GOOD_REQUEST_LINE_HEAD).unwrap();
        assert!(matches!(request_line.method, RequestMethod::HEAD));
        assert_eq!(&request_line.request_target[..], b"/");
        assert!(matches!(request_line.http_version, HttpVersion::HTTP_1_1));
    }

    #[test]
    fn parse_good_request_line_patch() {
        let request_line = RequestParser::parse_request_line(GOOD_REQUEST_LINE_PATCH).unwrap();
        assert!(matches!(request_line.method, RequestMethod::PATCH));
        assert_eq!(&request_line.request_target[..], b"/");
        assert!(matches!(request_line.http_version, HttpVersion::HTTP_1_1));
    }

    #[test]
    fn parse_good_request_line_delete() {
        let request_line = RequestParser::parse_request_line(GOOD_REQUEST_LINE_DELETE).unwrap();
        assert!(matches!(request_line.method, RequestMethod::DELETE));
        assert_eq!(&request_line.request_target[..], b"/");
        assert!(matches!(request_line.http_version, HttpVersion::HTTP_1_1));
    }

    #[test]
    fn parse_good_request_line_options() {
        let request_line = RequestParser::parse_request_line(GOOD_REQUEST_LINE_OPTIONS).unwrap();
        assert!(matches!(request_line.method, RequestMethod::OPTIONS));
        assert_eq!(&request_line.request_target[..], b"/");
        assert!(matches!(request_line.http_version, HttpVersion::HTTP_1_1));
    }

    #[test]
    fn parse_request_line_good_with_path() {
        let request_line = RequestParser::parse_request_line(GOOD_REQUEST_LINE_WITH_PATH).unwrap();
        assert!(matches!(request_line.method, RequestMethod::POST));
        assert_eq!(&request_line.request_target[..], b"/path/to/resource");
        assert!(matches!(request_line.http_version, HttpVersion::HTTP_1_1));
    }

    #[test]
    fn parse_request_line_invalid_method() {
        let r = RequestParser::parse_request_line(BAD_REQUEST_LINE_INVALID_METHOD);
        assert!(r.is_err());
        assert!(matches!(
            r.err().unwrap().downcast_ref::<RequestParserError>(),
            Some(RequestParserError::InvalidRequestMethod)
        ));
    }

    #[test]
    fn parse_request_line_invalid_version() {
        let r = RequestParser::parse_request_line(BAD_REQUEST_LINE_UNSUPPORTED_HTTP_VERSION);
        assert!(r.is_err());
        assert!(matches!(
            r.err().unwrap().downcast_ref::<RequestParserError>(),
            Some(RequestParserError::UnsupportedHttpVersion)
        ));
    }

    #[test]
    fn parse_request_line_invalid_number_of_parts() {
        let r = RequestParser::parse_request_line(BAD_REQUEST_LINE_TOO_MANY_PARTS);
        assert!(r.is_err());
        assert!(matches!(
            r.err().unwrap().downcast_ref::<RequestParserError>(),
            Some(RequestParserError::MalformedRequestLine)
        ));
    }

    #[test]
    fn parse_request_line_invalid_target() {
        let r = RequestParser::parse_request_line(BAD_REQUEST_LINE_INVALID_TARGET);
        assert!(r.is_err());
        assert!(matches!(
            r.err().unwrap().downcast_ref::<RequestParserError>(),
            Some(RequestParserError::MalformedRequestLine)
        ));
    }

    #[test]
    fn parse_request_line_too_long() {
        let mut req_parser = RequestParser::new(1024);
        let r = req_parser.parse(&BAD_REQUEST_LINE_TOO_LONG);
        assert!(r.is_err());
        assert!(matches!(
            r.err().unwrap().downcast_ref::<RequestParserError>(),
            Some(RequestParserError::RequestLineTooLong)
        ));
    }
}
