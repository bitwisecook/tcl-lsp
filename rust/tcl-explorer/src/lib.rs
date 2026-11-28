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
//! shape the explorer front-ends render, and serialises them to the
//! `wasm-explorer-view.md` JSON contract.
//!
//! No `pyo3` dependency, no filesystem, no I/O — `run_pipeline` is pure
//! compute over a source string so the same code compiles to
//! `wasm32-unknown-unknown` for the in-browser GUI.

#![forbid(unsafe_code)]

pub mod asm;
pub mod coverage;
pub mod cst;
mod environment;
pub mod formatters;
#[cfg(feature = "live-entry")]
pub mod live_entry;
pub mod render;
pub mod serialise;
pub mod view_tree;
pub mod views;
pub mod wasm_explorer;

pub use serialise::{
    serialise_meta, serialise_result, serialise_result_with_optimisations,
    serialise_semantic_optimisations,
};
/// The semantic/AOT optimisation configuration the `wasm` views are built
/// with, re-exported so a front end names passes through the explorer
/// contract rather than reaching past it into `tcl-compiler`.
pub use tcl_compiler::semantic_optimisation::{
    PASS_GROUPS, SemanticOptimisationConfig, SemanticOptimisationPassId,
};
pub use view_tree::{ViewNode, build_view};

use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
// Bundled SpecTcl loadables share the registry used by source diagnostics.
use tcl_spectcl::bundled::active_registry_for_dialect as registry_for_dialect;

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
        let mut procedures: Vec<_> = self.unit.procedures.iter().collect();
        procedures.sort_by(|(left_id, left), (right_id, right)| {
            left.name.cmp(&right.name).then(left_id.cmp(right_id))
        });
        for (_, unit) in procedures {
            out.push(FunctionSnapshot {
                name: &unit.name,
                kind: FunctionSnapshotKind::Procedure,
                unit,
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
            .values()
            .map(|unit| FunctionSnapshot {
                name: &unit.name,
                kind: FunctionSnapshotKind::Method,
                unit,
            })
            .chain(
                self.unit
                    .body_lowering_functions()
                    .map(|(_, unit)| FunctionSnapshot {
                        name: &unit.name,
                        kind: FunctionSnapshotKind::BodyUnit,
                        unit,
                    }),
            )
            .collect();
        extra.sort_by(|a, b| {
            a.name
                .cmp(b.name)
                .then(a.kind.rank().cmp(&b.kind.rank()))
                .then(a.unit.body_id.cmp(&b.unit.body_id))
        });
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
    let registry_held = registry_for_dialect(dialect);
    let profile = environment::profile_for_dialect(dialect);
    run_pipeline_with_inputs(
        source,
        dialect,
        &registry_held,
        profile,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        None,
    )
}

/// Run the explorer using the exact immutable source-analysis inputs selected
/// for an open editor document. The caller supplies the retained document
/// input, so this path does not resolve a registry again from its display
/// dialect label.
#[must_use]
pub fn run_pipeline_with_analysis_input(
    source: &str,
    dialect: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> ExplorerResult {
    let context = input.context_registry();
    run_pipeline_with_inputs(
        source,
        dialect,
        context.commands(),
        input.unit_profile(),
        input.lexer_config(),
        Some(input),
    )
}

fn run_pipeline_with_inputs(
    source: &str,
    dialect: &str,
    registry: &tcl_registry::CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
    lexer_config: tcl_lexer::LexerConfig,
    source_input: Option<&tcl_compiler::analyser::ResolvedAnalysisInput>,
) -> ExplorerResult {
    // Build for the requested dialect so every dialect-sensitive layer is
    // honoured: Tcl 8.4 / iRules disable `{*}` expansion, iRules enable the
    // `}{` brace-separator, and the iRules word operators (`contains`, …) are
    // real expression operators. `build_for` would otherwise use the default
    // Tcl-8.5+ config and plain-Tcl grammar for both.
    // Memory-SSA is built so the `dataflow` view can surface alias sets
    // (upvar / global / variable / namespace upvar). Without it the
    // `aliases` list degrades to empty.
    // The document's own `# tcl-lsp: stub` declarations, so the views show the
    // IR the diagnostics path actually built: a stub-declared `script:body`
    // word lowers to a barrier carrying a script, a `var` word to a def. The
    // explorer holds source text, not a path, so the sidecar half of the
    // surface is out of reach here.
    let declared =
        tcl_compiler::analyser::utils::document_declared_surface(source, None, profile.name);
    let options = tcl_compiler::compilation_unit::UnitBuildOptions {
        registry,
        defer_top_level: false,
        config: lexer_config,
        dialect: Some(profile),
        external_call_sites: None,
        declared_commands: Some(&declared),
    };
    let unit = match source_input {
        Some(input) => CompilationUnit::build_with_analysis_input(source, options, None, input),
        None => CompilationUnit::build_with_options(source, options),
    }
    .with_interprocedural(registry, Some(profile))
    .with_retained_memory_ssa(registry)
    // The ordinary compiler path builds world SSA only when interactive
    // GVN can consume it. Explorer is an explicit inspection surface, so
    // it asks for the complete source-faithful sidecar and displays typed
    // declines rather than silently presenting an empty graph.
    .with_retained_deep_semantic_analysis(registry);

    ExplorerResult {
        source: source.to_owned(),
        dialect: dialect.to_owned(),
        unit,
    }
}
