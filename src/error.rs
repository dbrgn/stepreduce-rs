use std::{fmt, io};

/// Errors that can occur during STEP file reduction.
#[derive(Debug)]
pub enum ReduceError {
    /// An I/O error (file not found, permission denied, etc.).
    Io(io::Error),
    /// A parse error in the STEP file content.
    Parse(String),
}

impl fmt::Display for ReduceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReduceError::Io(e) => write!(f, "I/O error: {e}"),
            ReduceError::Parse(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for ReduceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReduceError::Io(e) => Some(e),
            ReduceError::Parse(_) => None,
        }
    }
}

impl From<io::Error> for ReduceError {
    fn from(e: io::Error) -> Self {
        ReduceError::Io(e)
    }
}
