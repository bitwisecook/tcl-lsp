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

//! Registry-owned worked examples for closed compiler vocabularies.
//!
//! These are deliberately data, not web-UI copy. A trait, taint colour, or
//! side-effect target owns the Tcl program that explains its end-to-end
//! consequence. Registry browsers serialise the same value, and exhaustive
//! matches make a newly-added vocabulary item fail to compile until it has a
//! worked example.

/// One source-aligned explanation in a [`DocumentationExample`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentationAnnotation {
    /// Zero-based source line.
    pub line: usize,
    /// Exact text on `line` that the presentation should point at.
    pub needle: &'static str,
    /// What happens at this point in the end-to-end flow.
    pub label: &'static str,
}

impl DocumentationAnnotation {
    /// Construct one source annotation.
    #[must_use]
    pub const fn new(line: usize, needle: &'static str, label: &'static str) -> Self {
        Self {
            line,
            needle,
            label,
        }
    }
}

/// A complete Tcl example plus the source spans that explain its dataflow or
/// observable effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentationExample {
    /// Tcl source, normally showing input/state, the annotated operation, and
    /// the resulting sink or observable value.
    pub code: &'static str,
    /// Ordered source annotations rendered as arrows by registry browsers.
    pub annotations: &'static [DocumentationAnnotation],
}

impl DocumentationExample {
    /// Construct a required worked example.
    #[must_use]
    pub const fn new(code: &'static str, annotations: &'static [DocumentationAnnotation]) -> Self {
        Self { code, annotations }
    }
}
