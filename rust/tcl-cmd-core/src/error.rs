// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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

/// A failed Tcl command: its result message and optional structured error code.
///
/// The shared command core owns the semantic error identity; each runtime
/// adapter publishes it through its native completion/error state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmdError {
    message: String,
    error_code: Option<String>,
}

impl CmdError {
    /// A command error with the given Tcl result message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            error_code: None,
        }
    }

    /// A command error carrying Tcl's structured `-errorcode` list.
    pub fn with_error_code(message: impl Into<String>, error_code: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            error_code: Some(error_code.into()),
        }
    }

    /// The Tcl error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Tcl's structured `-errorcode` list, when the command supplied one.
    #[must_use]
    pub fn error_code(&self) -> Option<&str> {
        self.error_code.as_deref()
    }

    /// Consume the error, yielding its message (what an adapter sets as the
    /// interpreter result).
    #[must_use]
    pub fn into_message(self) -> String {
        self.message
    }

    /// Consume the error, yielding its message and optional error code.
    #[must_use]
    pub fn into_parts(self) -> (String, Option<String>) {
        (self.message, self.error_code)
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

    /// A Tcl list syntax failure, preserving the list owner's message and
    /// structured error code.
    #[must_use]
    pub fn list(error: tcl_syntax::list::ListError, source: &str) -> Self {
        Self::with_error_code(error.full_message(source), error.error_code())
    }

    /// A failed `Tcl_GetIndexFromObj`-style lookup.
    #[must_use]
    pub fn lookup_index(message: impl Into<String>, what: &str, word: &str) -> Self {
        let code = tcl_syntax::list::join_list(["TCL", "LOOKUP", "INDEX", what, word]);
        Self::with_error_code(message, code)
    }

    /// A command argument whose shape is invalid after Tcl list parsing.
    #[must_use]
    pub fn argument_format(message: impl Into<String>) -> Self {
        Self::with_error_code(message, "TCL ARGUMENT FORMAT")
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
        // The canonical Tcl error texts (`Tcl_WrongNumArgs` / bad-option).
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
        let coded = CmdError::with_error_code("constant", "TCL UNSET CONST");
        assert_eq!(coded.error_code(), Some("TCL UNSET CONST"));
        assert_eq!(
            coded.into_parts(),
            ("constant".to_string(), Some("TCL UNSET CONST".to_string()))
        );
        assert_eq!(
            CmdError::list(tcl_syntax::list::ListError::UnmatchedBrace, "{bad").into_parts(),
            (
                "unmatched open brace in list".to_string(),
                Some("TCL VALUE LIST BRACE".to_string())
            )
        );
        assert_eq!(
            CmdError::lookup_index("bad option", "option", "two words").into_parts(),
            (
                "bad option".to_string(),
                Some("TCL LOOKUP INDEX option {two words}".to_string())
            )
        );
        assert_eq!(
            CmdError::argument_format("bad shape").error_code(),
            Some("TCL ARGUMENT FORMAT")
        );
    }
}
