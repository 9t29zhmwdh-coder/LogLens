use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum LlError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Anyhow: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("{0}")]
    Other(String),
}

impl Serialize for LlError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, LlError>;
