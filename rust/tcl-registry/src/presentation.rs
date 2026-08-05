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

//! Presentation roles — how a *formatter* should lay an argument out.
//!
//! Distinct from [`crate::ArgRole`], which says what an argument **is**
//! semantically. Two arguments can share a semantic role and still want
//! different presentation: `for start test next body` has three script
//! arguments ([`crate::ArgRole::Body`] at indices 0, 2, and 3), but only the
//! trailing `body` is conventionally expanded onto its own indented lines —
//! `start` and `next` stay on the `for` line:
//!
//! ```tcl
//! for {set i 0} {$i < 3} {incr i} {
//!     puts $i
//! }
//! ```
//!
//! Erasing the semantic distinction (calling `start` / `next` something other
//! than `Body`) would break every analysis consumer that walks scripts, and
//! keeping a `name == "for"` branch in the formatter is the command-specific
//! knowledge the registry contract forbids. So the layout preference is its
//! own registry fact, declared beside the roles it refines (issue #1186).
//!
//! Only *overrides* are declared. Every `Body` argument is a
//! [`ArgPresentation::BlockScript`] by default, so a spec that has nothing
//! unusual to say declares nothing.

/// How a formatter should lay out one argument, refining its
/// [`crate::ArgRole`].
///
/// `#[non_exhaustive]` on purpose: this is the extension point for future
/// presentation facts (a keyword that must start a line, an argument list
/// that wraps, …), and a consumer must keep working when one arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ArgPresentation {
    /// A script expanded onto its own indented lines, with the opening brace
    /// left on the command's line (K&R). The default for every
    /// [`crate::ArgRole::Body`] argument.
    #[default]
    BlockScript,
    /// A script kept **inline** on the command's own line, however long it
    /// is. `for`'s `start` and `next` scripts: expanding them would turn a
    /// one-line loop header into four lines and is not how any Tcl in the
    /// wild is written.
    InlineScript,
}

impl ArgPresentation {
    /// Whether an argument in this presentation is expanded onto its own
    /// lines. The one place that answers the question, so the formatter's
    /// block decision cannot drift from the declaration.
    #[must_use]
    pub const fn is_block(self) -> bool {
        matches!(self, Self::BlockScript)
    }
}
