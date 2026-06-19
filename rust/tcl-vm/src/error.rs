//! The VM's internal error type.

use tcl_runtime_api::Code;

use crate::value::Value;

/// A Tcl runtime error — a message plus (eventually) the options dict.
///
/// Internal `Result<_, TclError>` is converted to a `Completion { code: Error,
/// result: <message> }` at the dispatch boundary.
#[derive(Debug, Clone)]
pub struct TclError {
    /// The error message (becomes the completion result).
    pub message: String,
    /// A non-`Error` completion code to propagate instead — set when a command
    /// substitution (`[break]`, `[eval continue]`, `[return]`) carries a
    /// `break`/`continue`/`return` out of the expression/word evaluation. `None`
    /// means an ordinary error.
    pub code: Option<Code>,
}

impl TclError {
    /// Build an error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
        }
    }

    /// Build a propagating non-`Error` completion (a `break`/`continue`/`return`
    /// escaping a command substitution).
    pub fn with_code(message: impl Into<String>, code: Code) -> Self {
        Self {
            message: message.into(),
            code: Some(code),
        }
    }

    /// The error message as a [`Value`].
    #[must_use]
    pub fn into_value(self) -> Value {
        Value::string(self.message.as_str())
    }
}
