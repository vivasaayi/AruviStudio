use crate::ClientError;

/// Bound both advertised and chunked responses before parsing their content.
pub(crate) async fn read_bounded(mut response: reqwest::Response, limit: usize, label: &str) -> Result<Vec<u8>, ClientError> {
    if response.content_length().is_some_and(|length| length > limit as u64) {
        return Err(ClientError::Provider(format!("{label} exceeds size limit")));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| ClientError::Provider(format!("{label} read failed: {error}")))? {
        if chunk.len() > limit - bytes.len() {
            return Err(ClientError::Provider(format!("{label} exceeds size limit")));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
