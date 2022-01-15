use thiserror::Error;

/// Helper to quit with a status code and no message.
#[derive(Debug, Error)]
#[error("exit with status code {}", _0)]
pub struct QuietExit(pub i32);
