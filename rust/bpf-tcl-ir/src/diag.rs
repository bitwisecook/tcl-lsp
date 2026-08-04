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

//! Diagnostics for the BPF-Tcl front-end. Spans are byte offsets into the
//! handler body (the CLI adds [`crate::ir::BpfProgramDecl::source_base`] to map
//! back to the original file), matching the rest of the toolchain's span model.

use tcl_lexer::Span;

/// A diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfDiag {
    /// A construct outside the typed DSL subset.
    OutOfSubset,
    /// An unknown DSL command.
    UnknownCommand,
    /// Wrong number of arguments.
    BadArity,
    /// A type mismatch (e.g. a pointer used in arithmetic).
    TypeMismatch,
    /// Read of a variable that was never assigned.
    UndefinedVar,
    /// A malformed integer literal.
    BadInt,
    /// An unbounded loop (no literal bound) — reserved for the loops milestone.
    UnboundedLoop,
    /// An unknown event name in a `when` block.
    BadEvent,
    /// An unknown profile, or a malformed `profile`/`field` declaration.
    BadProfile,
    /// An unknown template, or a malformed `template`/`use` declaration.
    BadTemplate,
    /// A command forbidden by the profile's `allow`/`deny` capability policy,
    /// or a malformed `allow`/`deny` declaration.
    CapabilityDenied,
    /// A malformed `attach`, or an attach that matches no program's type.
    BadAttach,
    /// A top-level statement that is neither a `when` block nor a recognised
    /// declaration when a `when` block is present — it would otherwise be
    /// silently dropped from the emitted program.
    StrayTopLevel,
    /// Too many values for the 512-byte eBPF stack.
    StackOverflow,
    /// A handler path can fall off the end without an explicit verdict.
    MissingVerdict,
    /// A malformed `map` declaration (bad kind, size, capacity, or
    /// concurrency word), or a map access incompatible with the declaration.
    BadMap,
    /// An internal invariant failure (a compiler bug, not user error).
    Internal,
}

impl BpfDiag {
    /// A short, stable code string (used by `--emit` and tests).
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            BpfDiag::OutOfSubset => "BPF001",
            BpfDiag::UnknownCommand => "BPF002",
            BpfDiag::BadArity => "BPF003",
            BpfDiag::TypeMismatch => "BPF004",
            BpfDiag::UndefinedVar => "BPF005",
            BpfDiag::BadInt => "BPF006",
            BpfDiag::UnboundedLoop => "BPF007",
            BpfDiag::BadEvent => "BPF008",
            BpfDiag::StackOverflow => "BPF009",
            BpfDiag::BadProfile => "BPF010",
            BpfDiag::BadTemplate => "BPF011",
            BpfDiag::CapabilityDenied => "BPF012",
            BpfDiag::BadAttach => "BPF013",
            BpfDiag::StrayTopLevel => "BPF014",
            BpfDiag::MissingVerdict => "BPF015",
            BpfDiag::BadMap => "BPF016",
            BpfDiag::Internal => "BPF999",
        }
    }
}

/// A front-end error with a source span.
#[derive(Debug, Clone)]
pub struct BpfError {
    /// Source span (body-relative).
    pub span: Span,
    /// Diagnostic code.
    pub code: BpfDiag,
    /// Human-readable message.
    pub msg: String,
}

impl BpfError {
    /// Construct a new error.
    #[must_use]
    pub fn new(code: BpfDiag, span: Span, msg: impl Into<String>) -> Self {
        Self {
            span,
            code,
            msg: msg.into(),
        }
    }

    /// Shift this error's span by `base` bytes, mapping a body-relative span to
    /// an absolute file offset.
    #[must_use]
    pub fn offset(mut self, base: u32) -> Self {
        let start = self.span.start().saturating_add(base);
        let end = self.span.end().saturating_add(base);
        self.span = Span::new(start, end);
        self
    }
}

impl std::fmt::Display for BpfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code.code(), self.msg)
    }
}

impl std::error::Error for BpfError {}
