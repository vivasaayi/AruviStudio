use crate::ClientError;

const MAX_STREAM_BYTES: usize = 2 * 1024 * 1024;

#[derive(Default)]
pub(crate) struct EventParser {
    line: Vec<u8>,
    data: Vec<u8>,
    has_data: bool,
    after_cr: bool,
    first_line: bool,
    total: usize,
    max_bytes: usize,
}

impl EventParser {
    pub(crate) fn new(max_bytes: usize) -> Self {
        Self {
            first_line: true,
            max_bytes,
            ..Self::default()
        }
    }

    pub(crate) fn push<F>(&mut self, chunk: &[u8], on_event: &mut F) -> Result<bool, ClientError>
    where
        F: FnMut(&[u8]) -> Result<bool, ClientError>,
    {
        self.total = self
            .total
            .checked_add(chunk.len())
            .filter(|size| *size <= self.max_bytes)
            .ok_or_else(|| ClientError::Provider("Model stream exceeds size limit".into()))?;
        for &byte in chunk {
            if self.after_cr {
                self.after_cr = false;
                if byte == b'\n' {
                    continue;
                }
            }
            match byte {
                b'\r' | b'\n' => {
                    self.after_cr = byte == b'\r';
                    if self.finish_line(on_event)? {
                        return Ok(true);
                    }
                }
                _ => self.line.push(byte),
            }
        }
        Ok(false)
    }

    fn finish_line<F>(&mut self, on_event: &mut F) -> Result<bool, ClientError>
    where
        F: FnMut(&[u8]) -> Result<bool, ClientError>,
    {
        let mut line = self.line.as_slice();
        if self.first_line {
            self.first_line = false;
            line = line.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(line);
        }
        if line.is_empty() {
            if self.has_data {
                let done = on_event(&self.data)?;
                self.data.clear();
                self.has_data = false;
                self.line.clear();
                return Ok(done);
            }
        } else {
            let value = line
                .strip_prefix(b"data:")
                .map(|v| v.strip_prefix(b" ").unwrap_or(v))
                .or_else(|| (line == b"data").then_some(&[][..]));
            if let Some(value) = value {
                if self.has_data {
                    self.data.push(b'\n');
                }
                self.data.extend_from_slice(value);
                self.has_data = true;
            }
        }
        self.line.clear();
        Ok(false)
    }
}

/// Read bounded SSE records without assuming HTTP chunks align with UTF-8 or lines.
/// The callback returns true when the protocol's terminal event has arrived.
pub(crate) async fn read_sse<F>(
    mut response: reqwest::Response,
    provider: &str,
    mut on_event: F,
) -> Result<(), ClientError>
where
    F: FnMut(&[u8]) -> Result<bool, ClientError> + Send,
{
    let mut parser = EventParser::new(MAX_STREAM_BYTES);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| crate::http::transport_error(&format!("{provider} stream read failed"), error))?
    {
        if parser.push(&chunk, &mut on_event)? {
            return Ok(());
        }
    }
    Err(ClientError::Provider(format!(
        "{provider} stream ended before its terminal event"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fragmented_line_endings_bom_and_empty_data_fields() {
        let mut parser = EventParser::new(MAX_STREAM_BYTES);
        let mut events = Vec::new();
        let bytes = b"\xef\xbb\xbf:comment\rdata: alpha\rdata\rdata: beta\r\n\r\ndata: [DONE]\n\n";
        for byte in bytes {
            if parser
                .push(&[*byte], &mut |event| {
                    events.push(event.to_vec());
                    Ok(event == b"[DONE]")
                })
                .unwrap()
            {
                break;
            }
        }
        assert_eq!(events, vec![b"alpha\n\nbeta".to_vec(), b"[DONE]".to_vec()]);
    }

    #[test]
    fn rejects_oversized_fragmented_stream() {
        let mut parser = EventParser::new(MAX_STREAM_BYTES);
        let mut ignore = |_: &[u8]| Ok(false);
        for _ in 0..MAX_STREAM_BYTES / 1024 {
            parser.push(&[b'x'; 1024], &mut ignore).unwrap();
        }
        assert!(parser.push(b"x", &mut ignore).is_err());
    }
}
