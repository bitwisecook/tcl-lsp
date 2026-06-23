//! [`CmdError`] — the command-level error the portable helpers return.
//!
//! A command helper computes `Result<V, CmdError>`; the per-runtime adapter maps
//! `CmdError` onto that runtime's protocol (`Completion<Value>` for the VM,
//! `interp.set_result(...) + Code` for the WASM runtime). `CmdError` is the home
//! of the **canonical Tcl error-message catalogue** — built once here rather
//! than hand-assembled per runtime — and the closed coercion / host errors
//! ([`tcl_syntax::value::ValueError`], [`tcl_platform::HostError`]) lift into it
//! with a plain `From`.

use tcl_platform::HostError;
use tcl_syntax::value::ValueError;

/// A failed Tcl command: the error message that becomes the interpreter result.
///
/// Carries the rendered message only for now; `-errorcode`/`-errorinfo`
/// threading is added with the catch/return work (Family-B), where the per-frame
/// `CmdFrame` accumulation lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmdError {
    message: String,
}

impl CmdError {
    /// A command error with the given Tcl result message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The Tcl error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Consume the error, yielding its message (what an adapter sets as the
    /// interpreter result).
    #[must_use]
    pub fn into_message(self) -> String {
        self.message
    }

    /// `wrong # args: should be "…"` — the canonical Tcl arity error.
    #[must_use]
    pub fn wrong_args(usage: &str) -> Self {
        Self::new(format!("wrong # args: should be \"{usage}\""))
    }

    /// `bad <what> "<got>": must be <choices>` — the canonical Tcl
    /// bad-option/subcommand error.
    #[must_use]
    pub fn bad_choice(what: &str, got: &str, choices: &str) -> Self {
        Self::new(format!("bad {what} \"{got}\": must be {choices}"))
    }
}

impl core::fmt::Display for CmdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CmdError {}

impl From<ValueError> for CmdError {
    fn from(e: ValueError) -> Self {
        Self::new(e.message())
    }
}

impl From<HostError> for CmdError {
    fn from(e: HostError) -> Self {
        // The bare reason; contextual helpers (e.g. `open`) build the richer
        // `couldn't open "<path>": <reason>` form themselves.
        Self::new(e.reason())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_tcl_error_message_formats() {
        // The canonical Tcl error texts (`Tcl_WrongNumArgs` /
        // bad-option) — cmd-core error.rs had no unit coverage (TEST-MIGRATE).
        assert_eq!(CmdError::new("boom").message(), "boom");
        assert_eq!(
            CmdError::wrong_args("string length string").message(),
            r#"wrong # args: should be "string length string""#
        );
        assert_eq!(
            CmdError::bad_choice("option", "foo", "a, b, or c").message(),
            r#"bad option "foo": must be a, b, or c"#
        );
        // `Display` mirrors the message, and `into_message` consumes it.
        assert_eq!(
            format!("{}", CmdError::wrong_args("x")),
            r#"wrong # args: should be "x""#
        );
        assert_eq!(CmdError::new("z").into_message(), "z");
    }
}
