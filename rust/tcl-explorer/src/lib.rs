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
            .iter()
            .map(|(_, unit)| FunctionSnapshot {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// With the shipped packs loaded, the registry a `jim` unit is built with
    /// resolves Jim's own commands to the rows a `jim` document's context
    /// resolves them to, and the pipeline built on it reports the same unknown
    /// calls as the one built on the static context's store.
    #[test]
    fn jim_own_commands_resolve_with_packs_loaded() {
        assert!(tcl_spectcl::bundled::active().is_none());
        assert_ne!(
            tcl_spectcl::bundled::packs().key,
            0,
            "the shipped packs are installed"
        );
        let context =
            tcl_registry::model::ingress::resolve_environment("jim").default_context_registry();
        let explorer = registry_for_dialect("jim");
        let analysed = tcl_registry::model::ingress::resolve_environment("jim")
            .context_registry(
                &tcl_registry::model::KeyedVersions::default(),
                tcl_spectcl::bundled::packs().key,
            )
            .expect("a jim analysis finds the shipped packs");
        assert!(
            std::sync::Arc::ptr_eq(analysed.commands(), &explorer),
            "the explorer and a jim analysis read one store"
        );
        for head in [
            "local",
            "upcall",
            "stackdump",
            "getref",
            "setref",
            "defer",
            "xtrace",
        ] {
            let expected = context
                .resolve_command(head)
                .unwrap_or_else(|| panic!("`{head}` resolves for a jim document"));
            let held = explorer
                .get(head)
                .unwrap_or_else(|| panic!("`{head}` is unknown to the explorer's registry"));
            assert!(std::ptr::eq(held, expected), "`{head}`");
        }

        let source = "proc ::p {} {\n    stackdump {}\n    getref r\n}\n\
                      proc ::q {} {\n    nosuchcmd\n}\n";
        let unknown_calls = |result: &ExplorerResult| {
            let mut rows: Vec<(String, bool)> = result
                .unit
                .interproc
                .as_ref()
                .expect("interprocedural analysis")
                .procedures
                .values()
                .map(|summary| (summary.qualified_name.clone(), summary.has_unknown_calls))
                .collect();
            rows.sort();
            rows
        };
        let profile = environment::profile_for_dialect("jim");
        let on_context = run_pipeline_with_inputs(
            source,
            "jim",
            tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
            profile,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            None,
        );
        let expected = unknown_calls(&on_context);
        assert!(
            expected.contains(&("::q".to_owned(), true)),
            "the control call is unknown: {expected:?}"
        );
        assert_eq!(unknown_calls(&run_pipeline(source, "jim")), expected);
    }

    #[test]
    fn supplied_analysis_input_controls_the_document_pipeline() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let result = run_pipeline_with_analysis_input("set value 1", "tcl8.6", &input);

        assert_eq!(result.dialect, "tcl8.6");
        assert_eq!(result.unit.ir_module.dialect_profile, Some(profile));
        assert_eq!(result.unit.ir_module.lexer_config, input.lexer_config());
        assert_eq!(
            result.unit.ir_module.source_metadata_input.as_ref(),
            Some(&input)
        );
        assert_eq!(
            result
                .unit
                .ir_module
                .source_entry
                .logical_source_input
                .as_ref(),
            Some(&input)
        );
        assert!(result.unit.ir_module.source_entry.native_entry.is_none());
    }

    #[test]
    fn original_document_pipeline_keeps_root_and_later_bom_roles() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        // Original document carriers supply no Native handler receipt.
        let source = "\u{feff}probe root\n\u{feff}probe later\n";
        for name in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "jim",
            "f5-irules",
            "f5-iapps",
            "f5-tmsh",
        ] {
            let environment = tcl_registry::model::ingress::resolve_known_environment(name)
                .expect("actual known provider source policy");
            let grammar = environment.grammar();
            let result = run_pipeline(source, name);
            let module = &result.unit.ir_module;
            assert_eq!(
                module.source,
                tcl_lexer::SourceImage::document(source),
                "{name}"
            );
            assert_eq!(
                module.lexer_config,
                tcl_lexer::LexerConfig::for_file_grammar(grammar),
                "{name}"
            );
            assert_eq!(
                module
                    .source_metadata_input
                    .as_ref()
                    .unwrap()
                    .lexer_config(),
                module.lexer_config,
                "{name}"
            );
            assert!(
                crate::serialise::retained_explorer_source(&result, source).is_some(),
                "{name}"
            );
            assert_eq!(module.top_level.statements.len(), 2, "{name}");
            let first = module.top_level.statements[0]
                .tokens()
                .expect("original root command");
            let expected = if grammar.script_skips_leading_bom {
                "probe"
            } else {
                "\u{feff}probe"
            };
            assert_eq!(first.argv_texts[0], expected, "{name}");
            assert_eq!(&source[first.argv[0].as_range()], expected, "{name}");
            let later = module.top_level.statements[1]
                .tokens()
                .expect("original later command");
            assert_eq!(later.argv_texts[0], "\u{feff}probe", "{name}");
            assert_eq!(&source[later.argv[0].as_range()], "\u{feff}probe", "{name}");
        }
    }

    #[test]
    fn original_document_pipeline_keeps_config_withdrawal_terminal() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        let source = "\u{feff}probe root";
        let mut result = run_pipeline(source, "tcl9.1");
        assert!(crate::serialise::retained_explorer_source(&result, source).is_some());
        result.unit.ir_module.lexer_config.leading_bom = tcl_lexer::LeadingBom::Content;
        assert!(crate::serialise::retained_explorer_source(&result, source).is_none());
        assert!(
            crate::serialise::retained_explorer_source(&result, &format!("{source} ")).is_none()
        );
    }

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
    fn deep_pipeline_preserves_the_actual_source_metadata_owner() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let result = run_pipeline(
            "::puts TOP; proc p {} {::puts PROC}; namespace eval n {::puts BODY}",
            "tcl8.6",
        );
        let snapshots = result.all_snapshots();
        assert!(
            snapshots
                .iter()
                .any(|snapshot| snapshot.kind == FunctionSnapshotKind::Procedure)
        );
        assert!(
            snapshots
                .iter()
                .any(|snapshot| snapshot.kind == FunctionSnapshotKind::BodyUnit)
        );
        for snapshot in snapshots {
            let input = snapshot.unit.source_metadata_input().unwrap();
            assert_eq!(
                snapshot.unit.semantic_facts.metadata_context(),
                Some(input.availability_context())
            );
            assert!(snapshot.unit.semantic_facts.context().is_none());
        }
        let procedure = result
            .unit
            .procedures
            .values()
            .find(|unit| unit.name == "::p")
            .unwrap();
        assert!(
            procedure
                .semantic_facts
                .executable()
                .world_state_ssa()
                .is_some()
        );
    }

    #[test]
    fn snapshots_retain_duplicate_procedures_and_named_top_separately() {
        let result = run_pipeline(
            "proc p {} {return FIRST}; proc p {} {return LAST}; proc ::top {} {return NAMED}",
            "tcl8.6",
        );
        let snapshots = result.snapshots();
        let procedure_rows: Vec<_> = snapshots
            .iter()
            .filter(|row| row.kind == FunctionSnapshotKind::Procedure)
            .collect();
        assert_eq!(procedure_rows.len(), 3);
        assert_eq!(
            procedure_rows
                .iter()
                .filter(|row| row.name == "::p")
                .count(),
            2
        );
        assert_eq!(
            procedure_rows
                .iter()
                .filter(|row| row.name == "::top")
                .count(),
            1
        );
        assert!(snapshots[0].unit.body_id.is_none());
        let ids: std::collections::HashSet<_> = procedure_rows
            .iter()
            .map(|row| row.unit.body_id.expect("retained procedure body"))
            .collect();
        assert_eq!(ids.len(), 3);
        for row in procedure_rows {
            let original = result
                .unit
                .ir_module
                .procedures
                .get(&row.unit.body_id.unwrap())
                .unwrap();
            assert_eq!(original.qualified_name, row.name);
        }
    }

    #[test]
    fn unparseable_dialect_does_not_panic() {
        let result = run_pipeline("set x 1", "not-a-real-dialect");
        assert_eq!(result.dialect, "not-a-real-dialect");
        assert!(!result.snapshots().is_empty());
    }
}
