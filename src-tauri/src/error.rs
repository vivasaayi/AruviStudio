use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Provider error: {message}")]
    ProviderHttp { status: u16, message: String },

    #[error("Provider error: {message}")]
    ProviderTransport { kind: aruvi_model_client::TransportErrorKind, message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<aruvi_model_client::ClientError> for AppError {
    fn from(error: aruvi_model_client::ClientError) -> Self {
        match error {
            aruvi_model_client::ClientError::Http { status, message } => Self::ProviderHttp { status, message },
            aruvi_model_client::ClientError::Transport { kind, message } => Self::ProviderTransport { kind, message },
            other => Self::Provider(other.to_string()),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
