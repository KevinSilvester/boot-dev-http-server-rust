use async_channel::{Receiver, bounded};
use smol::io::AsyncReadExt;
use std::fs::File;
use std::io::Read;

pub fn read_8_bytes(file: &str) {
    let mut buf: [u8; 8] = [0; 8];
    let mut file = File::open(file).unwrap();

    loop {
        let n = match file.read(&mut buf) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("{e:?}");
                break;
            }
        };

        if n == 0 {
            break;
        }

        println!("read: {}", str::from_utf8(&buf[..n]).unwrap());
    }
}

pub fn read_lines(file: &str) {
    let mut in_buf: [u8; 8] = [0; 8];
    let mut line_buf: Vec<u8> = vec![];
    let mut file = File::open(file).unwrap();

    loop {
        let n = match file.read(&mut in_buf) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("{e:?}");
                break;
            }
        };

        if n == 0 {
            break;
        }

        match in_buf.iter().position(|x| *x == b'\n') {
            Some(idx) => {
                line_buf.append(&mut in_buf[..idx].into());
                println!("read: {}", str::from_utf8(&line_buf).unwrap());
                line_buf = in_buf[(idx + 1)..].into();
            }
            None => line_buf.append(&mut in_buf[..].into()),
        }
    }
}

pub fn read_lines_stream<T: AsyncReadExt + Send + Unpin + 'static>(
    mut reader: T,
) -> anyhow::Result<Receiver<Vec<u8>>> {
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
