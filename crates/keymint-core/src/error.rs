use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

/// Error type shared by the core. Serializes to a plain string so it can cross
/// the IPC boundary and be shown directly in the UI.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Config(String),
    #[error("Unknown command: {0}")]
    UnknownCommand(String),
    #[error("{0}")]
    InvalidInput(String),
    #[error("No API key set for {0}. Add one in Settings → Providers.")]
    MissingApiKey(String),
    #[error("{0}")]
    Provider(String),
    #[error("Secret storage error: {0}")]
    Secret(String),
    #[error("Command failed: {0}")]
    Process(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("Network error: {0}")]
    Http(#[from] reqwest::Error),
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
