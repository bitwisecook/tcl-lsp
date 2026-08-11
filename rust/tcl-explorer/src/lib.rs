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

//! Compiler-explorer pipeline over the Tcl compiler crates.
//!
//! This crate is the single source-in / result-out entry point that the
//! CLI (`tcl explore`), the
//! TUI, and the Rust → WASM web GUI all share. It is a *thin aggregator*:
//! the heavy lifting (lexing, lowering, CFG/SSA, the analyses) lives in
//! `tcl-compiler`; this crate only assembles those artefacts into the
//! shape the explorer front-ends render, and (in later phases) serialises
//! them to the `wasm-explorer-view.md` JSON contract.
//!
//! No `pyo3` dependency, no filesystem, no I/O — `run_pipeline` is pure
//! compute over a source string so the same code compiles to
//! `wasm32-unknown-unknown` for the in-browser GUI.

#![forbid(unsafe_code)]

pub mod asm;
pub mod coverage;
pub mod cst;
pub mod formatters;
pub mod render;
pub mod serialise;
pub mod view_tree;
pub mod views;
pub mod wasm_explorer;

pub use serialise::{serialise_meta, serialise_result};
pub use view_tree::{ViewNode, build_view};

use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_dialect::DialectProfile;
use tcl_registry::registry_for_dialect;

/// Per-function compilation artefacts surfaced by the explorer.
///
/// The underlying
/// [`FunctionUnit`] already carries the CFG, SSA, def-use, SCCP, type, and
/// taint results, so the snapshot is just a named, ordered view onto the
/// [`CompilationUnit`]'s function table (top-level first, then procedures
/// in qualified-name order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionSnapshotKind {
    /// The compilation unit's top-level script.
    TopLevel,
    /// A named Tcl procedure.
    Procedure,
    /// A `TclOO` method.
    Method,
    /// A synthetic `apply`/`namespace eval` body.
    BodyUnit,
}

impl FunctionSnapshotKind {
    /// Stable JSON/display label for the function owner.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TopLevel => "top-level",
            Self::Procedure => "procedure",
            Self::Method => "method",
            Self::BodyUnit => "body-unit",
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::TopLevel => 0,
            Self::Procedure => 1,
            Self::Method => 2,
            Self::BodyUnit => 3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FunctionSnapshot<'a> {
    /// Qualified function name (`::top` for the top-level script).
    pub name: &'a str,
    /// Stable owner kind, retained even when names collide across tables.
    pub kind: FunctionSnapshotKind,
    /// The full per-function unit (CFG, SSA, analyses).
    pub unit: &'a FunctionUnit,
}

impl FunctionSnapshot<'_> {
    /// Number of basic blocks in this function's CFG.
    #[must_use]
    pub fn block_count(&self) -> usize {
        self.unit.cfg.blocks.len()
    }
}

/// Complete result of running the explorer pipeline.
///
/// Owns the [`CompilationUnit`]; the serialiser and renderers borrow from
/// it.
#[derive(Debug)]
pub struct ExplorerResult {
    /// The source that was compiled.
    pub source: String,
    /// The dialect the pipeline was configured for.
    pub dialect: String,
    /// The shared compilation unit (IR module, CFG module, per-function units).
    pub unit: CompilationUnit,
}

impl ExplorerResult {
    /// Per-function snapshots: top-level first, then procedures in
    /// qualified-name order.
    #[must_use]
    pub fn snapshots(&self) -> Vec<FunctionSnapshot<'_>> {
        let mut out = vec![FunctionSnapshot {
            name: "::top",
            kind: FunctionSnapshotKind::TopLevel,
            unit: &self.unit.top_level,
        }];
        let mut qnames: Vec<&String> = self.unit.procedures.keys().collect();
        qnames.sort();
        for qname in qnames {
            out.push(FunctionSnapshot {
                name: qname,
                kind: FunctionSnapshotKind::Procedure,
                unit: &self.unit.procedures[qname],
            });
        }
        out
    }

    /// Every durable function unit, including `TclOO` methods and synthetic
    /// `apply`/`namespace eval` bodies, in deterministic qualified-name order.
    ///
    /// Coverage-complete views use this iterator so a newly retained body unit
    /// cannot disappear from the explorer merely because it lives outside
    /// `CompilationUnit::procedures`.
    #[must_use]
    pub fn all_snapshots(&self) -> Vec<FunctionSnapshot<'_>> {
        let mut out = self.snapshots();
        let mut extra: Vec<FunctionSnapshot<'_>> = self
            .unit
            .methods
            .iter()
            .map(|(name, unit)| FunctionSnapshot {
                name,
                kind: FunctionSnapshotKind::Method,
                unit,
            })
            .chain(
                self.unit
                    .body_units
                    .iter()
                    .map(|(name, unit)| FunctionSnapshot {
                        name,
                        kind: FunctionSnapshotKind::BodyUnit,
                        unit,
                    }),
            )
            .collect();
        extra.sort_by(|a, b| a.name.cmp(b.name).then(a.kind.rank().cmp(&b.kind.rank())));
        out.extend(extra);
        out
    }

    /// Total basic-block count across every function.
    #[must_use]
    pub fn total_blocks(&self) -> usize {
        self.all_snapshots()
            .iter()
            .map(FunctionSnapshot::block_count)
            .sum()
    }
}

/// Run the full explorer pipeline on `source` for `dialect`.
///
/// Builds the [`CompilationUnit`], including interprocedural analysis, as
/// `run_pipeline` does. No per-pass graceful degradation is needed because
/// the Rust passes are infallible — they are run as part of building the unit.
#[must_use]
pub fn run_pipeline(source: &str, dialect: &str) -> ExplorerResult {
    let registry = registry_for_dialect(dialect);
    // Build for the requested dialect so every dialect-sensitive layer is
    // honoured: Tcl 8.4 / iRules disable `{*}` expansion, iRules enable the
    // `}{` brace-separator, and the iRules word operators (`contains`, …) are
    // real expression operators. `build_for` would otherwise use the default
    // Tcl-8.5+ config and plain-Tcl grammar for both.
    // Memory-SSA is built so the `dataflow` view can surface alias sets
    // (upvar / global / variable / namespace upvar). Without it the
    // `aliases` list degrades to empty.
    let unit = CompilationUnit::build_for_dialect(source, registry, false, dialect)
        .with_interprocedural(registry, Some(dialect))
        .with_memory_ssa(registry, DialectProfile::by_name(dialect).availability_mask)
        // The ordinary compiler path builds world SSA only when interactive
        // GVN can consume it. Explorer is an explicit inspection surface, so
        // it asks for the complete source-faithful sidecar and displays typed
        // declines rather than silently presenting an empty graph.
        .with_deep_semantic_analysis(registry, DialectProfile::by_name(dialect).availability_mask);

    ExplorerResult {
        source: source.to_owned(),
        dialect: dialect.to_owned(),
        unit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_pipeline_top_level_only() {
        let result = run_pipeline("set x 1\nset y 2", "tcl8.6");
        let snapshots = result.snapshots();
        // Only the top-level function exists for a script with no procs.
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].name, "::top");
        assert!(result.total_blocks() >= 1);
    }

    #[test]
    fn run_pipeline_collects_procedures_in_order() {
        let src = "proc beta {} { return 2 }\nproc alpha {} { return 1 }\nalpha";
        let result = run_pipeline(src, "tcl8.6");
        let names: Vec<&str> = result.snapshots().iter().map(|s| s.name).collect();
        // Top-level first, then procedures sorted by qualified name.
        assert_eq!(names[0], "::top");
        let procs = &names[1..];
        assert!(procs.contains(&"::alpha"));
        assert!(procs.contains(&"::beta"));
        let mut sorted = procs.to_vec();
        sorted.sort_unstable();
        assert_eq!(procs, sorted.as_slice());
    }

    #[test]
    fn unparseable_dialect_does_not_panic() {
        let result = run_pipeline("set x 1", "not-a-real-dialect");
        assert_eq!(result.dialect, "not-a-real-dialect");
        assert!(!result.snapshots().is_empty());
    }
}
