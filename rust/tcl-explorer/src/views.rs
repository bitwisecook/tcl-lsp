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

/// How the shared Explorer renderers present a view.
///
/// Keeping this with the public tab metadata avoids a second, manually kept
/// list of views in the text/TUI renderer. Front ends that do not have a
/// renderer for a view can still expose its descriptor and degrade locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewRenderKind {
    /// An analysis tree rendered by [`crate::view_tree`].
    Tree,
    /// A specialised flat text view such as disassembly.
    FlatText,
    /// A front-end-specific visualisation with no shared tree renderer yet.
    Frontend,
}

impl ViewRenderKind {
    /// Stable serialised renderer-family name for front ends.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tree => "tree",
            Self::FlatText => "flat-text",
            Self::Frontend => "frontend",
        }
    }
}

/// One Explorer view descriptor.
///
/// `group` is informational (`compiler` / `optimiser` / `codegen`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewDescriptor {
    /// Stable contract key used by front ends.
    pub id: &'static str,
    /// Human-readable tab label.
    pub label: &'static str,
    /// Top-level Explorer JSON field the view renders.
    pub payload: &'static str,
    /// Informational stage group.
    pub group: &'static str,
    /// Shared renderer family, if any.
    pub render_kind: ViewRenderKind,
}

/// The ordered view-tab table. The parser produces a single red-green CST
/// (no separate standalone "green tree"), so there is no `greentree` tab and
/// `cst` is the sole parse-tree view.
pub const VIEW_META: &[ViewDescriptor] = &[
    ViewDescriptor {
        id: "cst",
        label: "CST",
        payload: "cst",
        group: "compiler",
        render_kind: ViewRenderKind::Frontend,
    },
    ViewDescriptor {
        id: "segments",
        label: "Segments",
        payload: "segments",
        group: "compiler",
        render_kind: ViewRenderKind::Frontend,
    },
    ViewDescriptor {
        id: "structuralIndex",
        label: "Structural Index",
        payload: "structuralIndex",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "sourceMap",
        label: "Source Map",
        payload: "sourceMap",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "ir",
        label: "IR",
        payload: "ir",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "cfg",
        label: "CFG (pre-SSA)",
        payload: "cfgPreSsa",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "ssa",
        label: "CFG (post-SSA)",
        payload: "cfgPostSsa",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "dominators",
        label: "Dominators",
        payload: "dominators",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "sccp",
        label: "SCCP",
        payload: "sccp",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "liveness",
        label: "Liveness",
        payload: "liveness",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "semantic",
        label: "Executable Semantics",
        payload: "semantic",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "worldSsa",
        label: "World SSA",
        payload: "worldSsa",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "loops",
        label: "Loops",
        payload: "loops",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "types",
        label: "Types",
        payload: "types",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "intervals",
        label: "Intervals",
        payload: "intervals",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "bounds",
        label: "Bounds",
        payload: "bounds",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "dataflow",
        label: "Data Flow",
        payload: "dataflow",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "interproc",
        label: "Interprocedural",
        payload: "interprocedural",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "unitScope",
        label: "Unit Scope",
        payload: "unitScope",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "rendered",
        label: "Rendered Props",
        payload: "renderedProperties",
        group: "compiler",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "opt",
        label: "Optimisations",
        payload: "optimisations",
        group: "optimiser",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "optimiserPasses",
        label: "Pass Pipeline",
        payload: "optimiserPasses",
        group: "optimiser",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "gvn",
        label: "GVN",
        payload: "gvn",
        group: "optimiser",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "shimmer",
        label: "Shimmer",
        payload: "shimmer",
        group: "optimiser",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "taint",
        label: "Taint",
        payload: "taintWarnings",
        group: "optimiser",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "taintFacts",
        label: "Taint Lattice",
        payload: "taintFacts",
        group: "optimiser",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "irules",
        label: "iRules Flow",
        payload: "irulesFlow",
        group: "optimiser",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "connectionScope",
        label: "Connection Scope",
        payload: "connectionScope",
        group: "optimiser",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "eventOrder",
        label: "Event Order",
        payload: "eventOrder",
        group: "optimiser",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "callouts",
        label: "Source Callouts",
        payload: "annotations",
        group: "optimiser",
        render_kind: ViewRenderKind::Tree,
    },
    ViewDescriptor {
        id: "asm",
        label: "Tcl ASM",
        payload: "asm",
        group: "codegen",
        render_kind: ViewRenderKind::FlatText,
    },
    ViewDescriptor {
        id: "asmOpt",
        label: "Tcl ASM (opt)",
        payload: "asmOptimised",
        group: "codegen",
        render_kind: ViewRenderKind::Frontend,
    },
    ViewDescriptor {
        id: "wasm",
        label: "WASM",
        payload: "wasm",
        group: "codegen",
        render_kind: ViewRenderKind::FlatText,
    },
    ViewDescriptor {
        id: "wasmOpt",
        label: "WASM (opt)",
        payload: "wasmOptimised",
        group: "codegen",
        render_kind: ViewRenderKind::Frontend,
    },
    // The toggle surface for the two views above: one row per semantic/AOT
    // optimisation pass with the state the shown module was built with. A
    // front end renders its checkboxes from this payload rather than from a
    // copy of the pass list — which is the point of shipping it as a view.
    ViewDescriptor {
        id: "semanticOptimisations",
        label: "Optimisation Passes",
        payload: "semanticOptimisations",
        group: "codegen",
        render_kind: ViewRenderKind::Frontend,
    },
];

/// IDs handled by the shared Explorer tree model, in tab-table order.
pub fn tree_view_ids() -> impl Iterator<Item = &'static str> {
    VIEW_META
        .iter()
        .filter(|view| view.render_kind == ViewRenderKind::Tree)
        .map(|view| view.id)
}

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
