use crate::ClientError;

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
    const MAX_STREAM_BYTES: usize = 2 * 1024 * 1024;
    let mut pending = Vec::new();
    let mut data = Vec::new();
    let mut total = 0usize;
    while let Some(chunk) = response.chunk().await
        .map_err(|error| ClientError::Provider(format!("{provider} stream read failed: {error}")))? {
        total = total.checked_add(chunk.len()).filter(|size| *size <= MAX_STREAM_BYTES)
            .ok_or_else(|| ClientError::Provider(format!("{provider} stream exceeds size limit")))?;
        pending.extend_from_slice(&chunk);
        while let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<u8> = pending.drain(..=end).collect();
            line.pop();
            if line.last() == Some(&b'\r') { line.pop(); }
            if line.is_empty() {
                if !data.is_empty() {
                    if on_event(&data)? { return Ok(()); }
                    data.clear();
                }
            } else if let Some(value) = line.strip_prefix(b"data:") {
                if !data.is_empty() { data.push(b'\n'); }
                data.extend_from_slice(value.strip_prefix(b" ").unwrap_or(value));
            }
        }
    }
    Err(ClientError::Provider(format!("{provider} stream ended before its terminal event")))
}
