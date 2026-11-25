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

//! Reference-release policies selected by reproducible execution and source probes.

/// Policy projections of the pinned reference-build observations.
/// These describe language rules, never authority in a live interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleasePolicyFacts {
    /// Numeric literal grammar selected by the reference probes.
    pub numbers: crate::NumberSyntax,
    /// Backslash widths and Unicode escape decoding.
    pub escapes: crate::EscapeSyntax,
    /// Unit used by string length on a supplementary character.
    pub characters: crate::StringCharacterModel,
    /// Default float spelling and mutable precision behaviour.
    pub double_string: crate::DoubleStringPolicy,
    /// Whether a hash starts an expression comment.
    pub expr_comments: crate::ExprCommentStyle,
    /// Whether namespace-scope lookup falls back to the global variable.
    pub namespace_var_global_fallback: bool,
    /// Whether the lowercase end-of-string regex anchor is accepted.
    pub regex_z_anchor: bool,
    /// Selection of an optional upvar level word.
    pub upvar_level_presence: crate::FrameLevelPresence,
    /// Selection of an optional uplevel level word.
    pub uplevel_level_presence: crate::FrameLevelPresence,
    /// Source-defined local-name preparation and frame-table access.
    pub compiled_locals: CompiledLocalPolicy,
}

/// The C compiler's local-name preparation and frame-table capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledLocalPolicy {
    /// Whether compilation can reuse an existing frame local cache.
    pub borrow_frame_slots: bool,
    /// Whether scalar preparation receives counted token bytes.
    pub counted_scalar_tokens: bool,
    /// Whether preparation can reserve an array base before declining.
    pub reserves_array_base: bool,
}

include!("generated_release_policy.rs");

impl crate::TclVersion {
    /// Source- and oracle-checked policies for this reference release.
    #[must_use]
    pub const fn release_policy(self) -> &'static ReleasePolicyFacts {
        &RELEASE_POLICIES[self as usize]
    }
}
