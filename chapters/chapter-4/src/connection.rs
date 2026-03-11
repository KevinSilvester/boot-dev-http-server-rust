use std::net::SocketAddr;

use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol::net::TcpListener;

use crate::config::ServerConfig;
use crate::request::{Request, RequestParser};

#[derive(Debug)]
pub struct Connection {
    addr: SocketAddr,
    request: Request,
    //  todo:
    // response: Response,
}

pub async fn handle_connections(listener: TcpListener, config: ServerConfig) -> anyhow::Result<()> {
    loop {
        let (mut stream, _peer_addr) = listener.accept().await?;

        // start a request timer
        let request_timer = std::time::Instant::now();

        // let's just construct a simple request first before we move onto the complicated stuff
        // like the entire connection, the response, middleware, request extensions
        let mut req_parser = RequestParser::new(config.max_body_size);
        let mut stream_buffer = [0u8; 8 << 10];
        let mut buf_len = 0;

        while !req_parser.done() {
            let n = match stream.read(&mut stream_buffer).await {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(_) => panic!("Error handled!!! 💀"),
            };

            buf_len += n;
            let read = req_parser.parse(&stream_buffer[..buf_len])?;

            // Shift the remaining data to the beginning of the buffer
            // todo: we can optimize this by using a circular buffer or something like that, but for simplicity, we'll just shift the data
            stream_buffer.copy_within(read..buf_len, 0);
            stream_buffer[buf_len - read..].fill(0);
            buf_len -= read;
        }

        let time = request_timer.elapsed();
        println!("Request parsed: {:#?}", req_parser.request_line);
        println!("Request parsing took: {} us", time.as_micros());
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 29\r\nContent-Type: text/plain\r\n\r\nActix ain't got shit on me!!!")
            .await?;

        let time = request_timer.elapsed();
        println!("Response sent in    : {} us", time.as_micros());
    }
}
