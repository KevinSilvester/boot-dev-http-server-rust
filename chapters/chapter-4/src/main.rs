mod config;
mod connection;
mod header;
mod readers;
mod request;

use smol::io::AsyncWriteExt;
use smol::net::TcpListener;

use crate::config::ServerConfigBuilder;
use crate::connection::handle_connections;
use crate::request::Request;

fn main() -> anyhow::Result<()> {
    let config = ServerConfigBuilder::new().build();

    smol::block_on(async move {
        let listener = TcpListener::bind(("127.0.0.1", 42069)).await?;

        println!("Listening on {}", listener.local_addr()?);
        println!("Now start a TCP client.");

        handle_connections(listener, config).await?;

        Ok(())
    })
}
