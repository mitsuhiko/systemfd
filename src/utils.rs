use std::fmt;

/// Helper to quit with a status code and no message.
#[derive(Debug)]
pub struct QuietExit(pub i32);

impl fmt::Display for QuietExit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exit with status code {}", self.0)
    }
}

impl std::error::Error for QuietExit {}
