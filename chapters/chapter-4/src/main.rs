mod readers;
mod request;

use smol::io::AsyncWriteExt;
use smol::net::TcpListener;

fn main() -> anyhow::Result<()> {
    smol::block_on(async move {
        let listener = TcpListener::bind(("127.0.0.1", 42069)).await?;

        println!("Listening on {}", listener.local_addr()?);
        println!("Now start a TCP client.");

        loop {
            let (mut stream, _peer_addr) = listener.accept().await?;
            let rx = readers::read_lines_stream(stream.clone())?;

            while let Ok(line) = rx.recv().await {
                stream.write(&line).await?;
                println!("{}", String::from_utf8_lossy(&line));
            }
        }
    })
}
