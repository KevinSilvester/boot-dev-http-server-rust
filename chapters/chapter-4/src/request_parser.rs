
#[derive(Debug, Clone)]
pub struct RequestParser {
    state: RequestParserState,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
}

impl RequestParser {
    pub fn new() -> Self {
        let (tx, rx) = bounded::<Vec<u8>>(32);
        Self {
            state: RequestParserState::RequestLine,
            tx,
            rx,
        }
    }

    pub fn read_lines_stream(self, mut reader: impl TReader) -> anyhow::Result<()> {
        smol::spawn(async move {
            let mut buffer = [0u8; 8];
            let mut line = vec![];

            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) if line.is_empty() => break,
                    Ok(0) => {
                        self.tx.send(line).await?;
                        break;
                    }
                    Ok(n) => {
                        for &byte in &buffer[..n] {
                            if byte == b'\n' {
                                self.tx.send(std::mem::take(&mut line)).await?;
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

        Ok(())
    }

    pub async fn parse<'r>(&mut self, mut reader: impl TReader) -> anyhow::Result<Request<'r>> {
        let mut request_line: Option<RequestLine> = None;

        while let Ok(mut line) = self.rx.recv().await {
            if line.is_empty() || line[line.len() - 1] != b'\r' {
                return Err(RequestError::InvalidLineEnding)?;
            }

            let line = std::mem::take(&mut line);

            match self.state {
                RequestParserState::RequestLine => {
                    request_line = Some(parse_request_line(&line)?);
                    self.state = RequestParserState::Headers;
                }
                RequestParserState::Headers => {
                    self.state = RequestParserState::Body;
                }
                RequestParserState::Body => {
                    self.state = RequestParserState::Done;
                }
                RequestParserState::Done => {
                    break;
                }
            };
        }
        todo!()

        // Ok(Request {
        //     request_line: request_line.context(RequestError::RequestParsingError)?,
        // })
    }
}
