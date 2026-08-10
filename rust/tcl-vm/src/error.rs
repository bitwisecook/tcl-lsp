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
    /// An explicit Tcl `-errorcode` for an ordinary error completion.
    ///
    /// This is distinct from [`Self::code`]: the latter propagates a non-error
    /// completion such as `break`; this field preserves the structured code of
    /// an actual `TCL_ERROR` as it crosses expression helpers.
    pub error_code: Option<String>,
}

impl TclError {
    /// Build an error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
            error_code: None,
        }
    }

    /// Build a propagating non-`Error` completion (a `break`/`continue`/`return`
    /// escaping a command substitution).
    pub fn with_code(message: impl Into<String>, code: Code) -> Self {
        Self {
            message: message.into(),
            code: Some(code),
            error_code: None,
        }
    }

    /// Build an ordinary error carrying Tcl's structured `-errorcode`.
    pub fn with_error_code(message: impl Into<String>, error_code: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
            error_code: Some(error_code.into()),
        }
    }

    /// The error message as a [`Value`].
    #[must_use]
    pub fn into_value(self) -> Value {
        Value::string(self.message.as_str())
    }
}
