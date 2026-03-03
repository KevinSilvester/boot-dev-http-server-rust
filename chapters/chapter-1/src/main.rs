mod readers;

const MESSAGES: &str = "messages.txt";

fn main() -> anyhow::Result<()> {
    readers::read_8_bytes(MESSAGES);
    readers::read_lines(MESSAGES);

    smol::block_on(async move {
        let file = smol::fs::File::open(MESSAGES).await?;
        let rx = readers::read_lines_stream(file)?;

        while let Ok(line) = rx.recv().await {
            println!("read w: {}", String::from_utf8_lossy(&line));
        }
        Ok(())
    })
}
