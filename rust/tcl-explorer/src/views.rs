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

//! Explorer view metadata: the tab table and severity vocabulary.
//!
//! The front-ends key off the `id`
//! strings, so this table is part of the de-facto explorer contract.

/// One explorer view tab: `(id, label, group)`.
///
/// `group` is informational (`compiler` / `optimiser` / `codegen`); the
/// CLI's `--show` grouping uses the separate `VIEW_GROUPS` mapping.
pub type ViewMeta = (&'static str, &'static str, &'static str);

/// The ordered view-tab table. The parser produces a single red-green CST
/// (no separate standalone "green tree"), so there is no `greentree` tab and
/// `cst` is the sole parse-tree view.
pub const VIEW_META: &[ViewMeta] = &[
    ("cst", "CST", "compiler"),
    ("segments", "Segments", "compiler"),
    ("structuralIndex", "Structural Index", "compiler"),
    ("sourceMap", "Source Map", "compiler"),
    ("ir", "IR", "compiler"),
    ("cfg", "CFG (pre-SSA)", "compiler"),
    ("ssa", "CFG (post-SSA)", "compiler"),
    ("loops", "Loops", "compiler"),
    ("types", "Types", "compiler"),
    ("intervals", "Intervals", "compiler"),
    ("bounds", "Bounds", "compiler"),
    ("dataflow", "Data Flow", "compiler"),
    ("interproc", "Interprocedural", "compiler"),
    ("unitScope", "Unit Scope", "compiler"),
    ("rendered", "Rendered Props", "compiler"),
    ("opt", "Optimisations", "optimiser"),
    ("optimiserPasses", "Pass Pipeline", "optimiser"),
    ("gvn", "GVN", "optimiser"),
    ("shimmer", "Shimmer", "optimiser"),
    ("taint", "Taint", "optimiser"),
    ("irules", "iRules Flow", "optimiser"),
    ("eventOrder", "Event Order", "optimiser"),
    ("callouts", "Source Callouts", "optimiser"),
    ("asm", "Tcl ASM", "codegen"),
    ("asmOpt", "Tcl ASM (opt)", "codegen"),
    ("wasm", "WASM", "codegen"),
    ("wasmOpt", "WASM (opt)", "codegen"),
];

/// Renderer-agnostic severity classification.
///
/// The string value is what each renderer (CLI ANSI, GUI CSS class) keys
/// off.; order matches the severity-enum
/// declaration so `meta.severities` lists `[error, warning, info]`.
///
/// This is the explorer's three-level *view* vocabulary, distinct from the
/// diagnostic severity in `tcl-core-types`: it carries only the levels a
/// renderer paints (no `Hint`/`Suggestion`) and its declaration order is the
/// rendering order the `meta.severities` contract pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// An error-level annotation.
    Error,
    /// A warning-level annotation.
    Warning,
    /// An informational annotation.
    Info,
}

impl Severity {
    /// All severities in declaration order.
    pub const ALL: [Severity; 3] = [Severity::Error, Severity::Warning, Severity::Info];

    /// The contract string the renderers key off.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}
