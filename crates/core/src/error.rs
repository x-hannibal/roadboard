use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{entity_type} not found: {id}")]
    NotFound { entity_type: &'static str, id: String },
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("storage error: {0}")]
    Storage(String),
}

pub type Result<T> = std::result::Result<T, Error>;
