//! The VM's internal error type.

use crate::value::Value;

/// A Tcl runtime error — a message plus (eventually) the options dict.
///
/// Internal `Result<_, TclError>` is converted to a `Completion { code: Error,
/// result: <message> }` at the dispatch boundary.
#[derive(Debug, Clone)]
pub struct TclError {
    /// The error message (becomes the completion result).
    pub message: String,
}

impl TclError {
    /// Build an error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The error message as a [`Value`].
    #[must_use]
    pub fn into_value(self) -> Value {
        Value::string(self.message.as_str())
    }
}
