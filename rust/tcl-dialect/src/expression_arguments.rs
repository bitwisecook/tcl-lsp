// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native expression command argv composition, separate from expression syntax.

/// How a native expression command receives its source object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExpressionArguments {
    /// C Tcl concatenates one or more source arguments, separated by spaces.
    Concatenate,
    /// Current Jim accepts exactly one original source value object.
    Single,
}

impl ExpressionArguments {
    /// Whether native argv validation reaches the expression source conversion.
    #[must_use]
    pub const fn accepts_len(self, count: usize) -> bool {
        match self {
            Self::Concatenate => count >= 1,
            Self::Single => count == 1,
        }
    }

    /// Native usage text used after argument validation fails.
    #[must_use]
    pub const fn synopsis(self) -> &'static str {
        match self {
            Self::Concatenate => "expr arg ?arg ...?",
            Self::Single => "expr expression",
        }
    }
}
