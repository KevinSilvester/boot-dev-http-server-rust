use async_channel::{Receiver, bounded};
use smol::io::AsyncReadExt;

pub trait ReaderT: smol::io::AsyncRead + Unpin + Send + 'static {}
impl<T> ReaderT for T where T: smol::io::AsyncRead + Unpin + Send + 'static {}

pub fn read_lines_stream(mut reader: impl ReaderT) -> anyhow::Result<Receiver<Vec<u8>>> {
    let (tx, rx) = bounded::<Vec<u8>>(32);

    smol::spawn(async move {
        let mut buffer = [0u8; 8];
        let mut line = vec![];

        loop {
            match reader.read(&mut buffer).await {
                Ok(0) if line.is_empty() => break,
                Ok(0) => {
                    tx.send(line).await?;
                    break;
                }
                Ok(n) => {
                    for &byte in &buffer[..n] {
                        if byte == b'\n' {
                            tx.send(std::mem::take(&mut line)).await?;
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

    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol::io::Cursor;

    #[test]
    fn test_read_lines_stream() -> anyhow::Result<()> {
        let data = b"Hello, world!\nThis is a test.\nAnother line.";
        let cursor = Cursor::new(data);
        let rx = read_lines_stream(cursor)?;

        smol::block_on(async {
            let mut lines = Vec::new();
            while let Ok(line) = rx.recv().await {
                lines.push(line);
            }
            assert_eq!(lines.len(), 3);
            assert_eq!(lines[0], b"Hello, world!");
            assert_eq!(lines[1], b"This is a test.");
            assert_eq!(lines[2], b"Another line.");
        });
        Ok(())
    }
}
