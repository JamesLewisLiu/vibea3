use thiserror::Error;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid packet header: {0}")]
    Header(&'static str),
    #[error("invalid schema: {0}")]
    Schema(String),
    #[error("invalid value for {field}: {reason}")]
    Value { field: String, reason: String },
    #[error("missing required field {0}")]
    Missing(&'static str),
    #[error("duplicate field {0}")]
    Duplicate(&'static str),
    #[error("unsupported wire feature: {0}")]
    Unsupported(&'static str),
    #[error("character encoding failed for {0}")]
    Encoding(&'static str),
    #[error("xml error: {0}")]
    Xml(String),
    #[error("lz77 error: {0}")]
    Lz77(&'static str),
    #[error("transport error: {0}")]
    Transport(String),
    #[error("request exceeds configured limit")]
    Limit,
    #[error("handler failed: {0}")]
    Handler(String),
}
