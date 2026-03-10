use anyhow::Context;
use bytes::BytesMut;
use thiserror::Error;

use super::line::{HttpVersion, RequestLine, RequestMethod};

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
        let mut spaces = memchr::memchr_iter(b' ', line);
        let sp_one = spaces.next().context(RequestParserError::MalformedRequestLine)?;
        let sp_two = spaces.next().context(RequestParserError::MalformedRequestLine)?;

        if spaces.next().is_some() {
            return Err(RequestParserError::MalformedRequestLine.into());
        }

        let method = &line[..sp_one];
        let request_target = &line[sp_one + 1..sp_two];
        let http_version = &line[sp_two + 1..];

        let method = match method.len() {
            3 => match method {
                b"GET" => RequestMethod::GET,
                b"PUT" => RequestMethod::PUT,
                _ => return Err(RequestParserError::InvalidRequestMethod)?,
            },
            4 => match method {
                b"POST" => RequestMethod::POST,
                b"HEAD" => RequestMethod::HEAD,
                _ => return Err(RequestParserError::InvalidRequestMethod)?,
            },
            5 => match method {
                b"PATCH" => RequestMethod::PATCH,
                _ => return Err(RequestParserError::InvalidRequestMethod)?,
            },
            6 => match method {
                b"DELETE" => RequestMethod::DELETE,
                _ => return Err(RequestParserError::InvalidRequestMethod)?,
            },
            7 => match method {
                b"OPTIONS" => RequestMethod::OPTIONS,
                _ => return Err(RequestParserError::InvalidRequestMethod)?,
            },
            _ => return Err(RequestParserError::InvalidRequestMethod)?,
        };

        let request_target = BytesMut::from(request_target);

        let http_version = match http_version {
            b"HTTP/1.1" => HttpVersion::HTTP1_1,
            _ => return Err(RequestParserError::UnsupportedHttpVersion)?,
        };

        Ok(RequestLine {
            method,
            request_target,
            http_version,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_request_line_good() {
        let data = b"GET / HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";

        let mut req_parser = RequestParser::new(1024);
        let _ = match req_parser.parse(data) {
            Ok(n) => n,
            Err(e) => panic!("Error parsing request: {e}"),
        };

        let request_line = match req_parser.request_line {
            Some(ref rl) => rl,
            None => panic!("Request line should be parsed!"),
        };

        assert!(matches!(request_line.method, RequestMethod::GET));
        assert_eq!(&request_line.request_target[..], b"/");
        assert!(matches!(request_line.http_version, HttpVersion::HTTP1_1));
    }

    #[test]
    fn parse_request_line_good_with_path() {
        let data = b"POST /path/to/resource HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/8.16.0\r\nAccept: */*\r\n\r\n";
        let mut req_parser = RequestParser::new(1024);
        let _ = match req_parser.parse(data) {
            Ok(n) => n,
            Err(e) => panic!("Error parsing request: {e}"),
        };
        let request_line = match req_parser.request_line {
            Some(ref rl) => rl,
            None => panic!("Request line should be parsed!"),
        };
        assert!(matches!(request_line.method, RequestMethod::POST));
        assert_eq!(&request_line.request_target[..], b"/path/to/resource");
        assert!(matches!(request_line.http_version, HttpVersion::HTTP1_1));
    }

    #[test]
    fn parse_request_line_invalid_method() {
        let data = b"FOO / HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
        let mut req_parser = RequestParser::new(1024);
        let r = req_parser.parse(data);
        assert!(r.is_err());
        assert!(matches!(
            r.err().unwrap().downcast_ref::<RequestParserError>(),
            Some(RequestParserError::InvalidRequestMethod)
        ));
    }

    #[test]
    fn parse_request_line_invalid_version() {
        let data = b"GET / HTTP/2.0\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
        let mut req_parser = RequestParser::new(1024);
        let r = req_parser.parse(data);
        assert!(r.is_err());
        assert!(matches!(
            r.err().unwrap().downcast_ref::<RequestParserError>(),
            Some(RequestParserError::UnsupportedHttpVersion)
        ));
    }

    #[test]
    fn parse_request_line_invalid_number_of_parts() {
        let data = b"GET / HTTP/1.1 extra\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
        let mut req_parser = RequestParser::new(1024);
        let r = req_parser.parse(data);
        assert!(r.is_err());
        assert!(matches!(
            r.err().unwrap().downcast_ref::<RequestParserError>(),
            Some(RequestParserError::MalformedRequestLine)
        ));
    }
    
    #[test]
    fn parse_request_line_invalid_target() {
        let data = b"GET --hello<'-'>bye-- HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
        let mut req_parser = RequestParser::new(1024);
        let r = req_parser.parse(data);
        dbg!(&r);
        assert!(r.is_err());
        assert!(matches!(
            r.err().unwrap().downcast_ref::<RequestParserError>(),
            Some(RequestParserError::MalformedRequestLine)
        ));
    }

    #[test]
    fn parse_request_line_too_long() {
        let long_request_target = vec![b'a'; MAX_REQUEST_LINE_SIZE + 1];
        let data = [
            &b"GET / "[..],
            &long_request_target[..],
            b" HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n",
        ]
        .concat();
        let mut req_parser = RequestParser::new(1024);
        let r = req_parser.parse(&data);
        assert!(r.is_err());
        assert!(matches!(
            r.err().unwrap().downcast_ref::<RequestParserError>(),
            Some(RequestParserError::RequestLineTooLong)
        ));
    }
}
