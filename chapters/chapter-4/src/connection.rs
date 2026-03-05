use std::net::SocketAddr;

use smol::io::AsyncReadExt;
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
        let (mut stream, peer_addr) = listener.accept().await?;

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

            let read_n = req_parser.parse(&stream_buffer[..buf_len + n])?;
            dbg!(&req_parser.request_line);
        }
    }
    Ok(())
}
