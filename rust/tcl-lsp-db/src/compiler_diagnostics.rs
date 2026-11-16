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

//! The compiler-check and optimiser diagnostics of one document, memoised and uncached.

use super::check_solve::proc_taint_solve_for_config;
use super::compilation_units::{compilation_unit_for_config, lexer_cfg_key};
use super::document_analysis::walked_document_input;
use super::{AnalyserConfig, SourceFile, TclDb};
use std::sync::Arc;
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::compiler_checks::Diagnostic as CompilerCheck;
use tcl_compiler::optimiser::Optimisation;
use tcl_compiler::unit_scope::CallSiteEvidence;
use tcl_registry::CommandRegistry;

/// [`abstain`] for the checks-and-optimisations pass: no findings, and no
/// rewrites.
pub(super) fn abstain_diagnostics() -> Arc<CompilerDiagnostics> {
    Arc::new(CompilerDiagnostics {
        checks: Vec::new(),
        optimisations: Vec::new(),
        taint_replays: tcl_compiler::taint::TaintReplayAnalysis::unavailable(),
    })
}

/// The compiler-checks + optimiser diagnostics for one document, unfiltered.
///
/// Returned by [`compiler_check_diagnostics`]. Unfiltered: every surface
/// converts these to findings, and the policy step decides what shows.
/// Kept independent of the runtime gate so the query caches across config
/// toggles.  `Clone + PartialEq` for salsa early-cutoff.
#[derive(Clone)]
pub struct CompilerDiagnostics {
    /// `run_all_checks` output (GVN / shimmer / thunking / taint / iRules-flow /
    /// SCCP), severities preserved.
    pub checks: Vec<CompilerCheck>,
    /// `optimise_unit` rewrites (`O1xx`), surfaced as HINT-severity suggestions.
    pub optimisations: Vec<Optimisation>,
    /// Command-head replay availability for compiler checks, independent of warning count.
    /// Stored callbacks belong to the separate whole-unit taint report.
    pub taint_replays: tcl_compiler::taint::TaintReplayAnalysis,
}

impl PartialEq for CompilerDiagnostics {
    fn eq(&self, other: &Self) -> bool {
        self.checks == other.checks
            && self.optimisations == other.optimisations
            && self.taint_replays == other.taint_replays
            && same_optimisation_provenance(&self.optimisations, &other.optimisations)
    }
}

// Salsa must not retain an old source-action receipt when report text agrees.
// Optimisation's public payload equality remains independent of that currency.
pub(super) fn same_optimisation_provenance(left: &[Optimisation], right: &[Optimisation]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.same_original_rewrite_provenance(right))
}

/// Run the compiler-checks + optimiser passes over a built unit.  Shared by the
/// memoised [`compiler_check_diagnostics`] query and the no-salsa-input
/// fallback so both produce byte-identical diagnostics.
fn compiler_diagnostics_from_unit(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    generic_patterns: Option<&[String]>,
) -> CompilerDiagnostics {
    let report = tcl_compiler::compiler_checks::compiler_check_report_with_generic_patterns(
        cu,
        registry,
        dialect,
        generic_patterns,
    );
    CompilerDiagnostics {
        checks: report.diagnostics,
        optimisations: tcl_compiler::optimiser::optimise_unit(cu, registry, dialect),
        taint_replays: report.taint_replays,
    }
}

/// Compiler-checks + optimiser diagnostics for one document, with the unit's
/// per-procedure lattices memoised by the salsa-native [`function_lattice`]
/// query (so an unchanged procedure is built once and shared with the analyser
/// tail).  The optimiser lowers with the dialect lexer config — distinct from
/// the analyser tail's default config, so the two intern different bodies and
/// never cross-pollute.  Byte-identical to
/// [`compiler_check_diagnostics_uncached`].
// LRU-capped: per-file key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 64, returns(clone))]
pub fn compiler_check_diagnostics(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<CompilerDiagnostics> {
    compiler_check_diagnostics_if_available(db, file, config).unwrap_or_else(abstain_diagnostics)
}

/// The actual compiler diagnostic pass, preserving unavailable preparation.
/// `Some` follows successful checked unit preparation and the real solve/check
/// producer. `None` carries no completed empty report. This supplies no Native
/// execution, normal completion or closure of unavailable nested analyses.
#[salsa::tracked(lru = 64, returns(clone))]
pub fn compiler_check_diagnostics_if_available(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<Arc<CompilerDiagnostics>> {
    let dialect = file.dialect(db).clone();
    let Ok(input) = walked_document_input(db, file, config) else {
        return None;
    };
    let registry: &CommandRegistry = input.borrowed_context_registry().commands();
    let dialect_opt = Some(input.unit_profile());
    // The analyser tail and compiler checks share the complete document
    // policy key. The supplied input must still pass its exact guard.
    let cfg_key = lexer_cfg_key(db, &dialect);
    let _shared_unit = compilation_unit_for_config(db, file, cfg_key, config)?;
    // Both halves of `run_all_checks` come from the memoised [`proc_taint_solve`]:
    // the interprocedural taint solve (`solve.taints`)
    // and the per-procedure non-taint checks (`solve.fn_checks`, already rebased),
    // so an unchanged procedure contributes neither a re-solve nor a re-check.
    // The remaining taint-family + iRules module checks (which read the solved
    // taints) are appended over the solve's retained unit, then the set is sorted
    // into the same deterministic order `run_all_checks` produces.  Byte-identical
    // to the in-line build; guarded by the corpus differential.  Optimiser
    // unchanged.
    let solve = proc_taint_solve_for_config(db, file, cfg_key, config)?;
    // Procedure IDs belong to this solve's genuine construction. A separate
    // shared build retains its own source ingress and cannot select these maps.
    let cu = solve.unit.as_ref();
    let mut checks = solve.fn_checks.clone();
    let generic_patterns = config.generic_variable_patterns(db).as_deref();
    let taint_replays = tcl_compiler::compiler_checks::push_taint_and_module_checks(
        cu,
        registry,
        dialect_opt,
        &solve.taints,
        generic_patterns,
        &mut checks,
    );
    tcl_compiler::compiler_checks::retain_diagnostic_source_context(cu, registry, &mut checks);
    tcl_compiler::compiler_checks::sort_diagnostics(&mut checks);
    Some(Arc::new(CompilerDiagnostics {
        checks,
        optimisations: solve.optimisations.clone(),
        taint_replays,
    }))
}

/// No-salsa-input fallback for [`compiler_check_diagnostics`]: build the unit
/// directly (no per-procedure memoisation) and run the same passes.  Used when
/// a document has no [`SourceFile`] input yet (mirrors the analyser fallback).
#[must_use]
pub fn compiler_check_diagnostics_uncached(
    text: &str,
    registry: &CommandRegistry,
    dialect: &str,
    generic_patterns: Option<&[String]>,
    external_call_sites: Option<&CallSiteEvidence>,
) -> CompilerDiagnostics {
    let cu = fresh_compiler_document_unit(text, registry, dialect, external_call_sites);
    compiler_diagnostics_from_unit(
        &cu,
        registry,
        tcl_lsp_core::stated_profile_for_dialect(dialect),
        generic_patterns,
    )
}

/// Fresh document source ingress over the exact caller-selected command store.
/// This driver resolves its own complete editing input; supplied-owner callers
/// use their independent checked route and cannot reopen this constructor.
pub(super) fn fresh_compiler_document_unit(
    text: &str,
    registry: &CommandRegistry,
    dialect: &str,
    external_call_sites: Option<&CallSiteEvidence>,
) -> CompilationUnit {
    let environment = tcl_registry::model::ingress::resolve_environment(dialect);
    let baseline = environment.default_context_registry();
    let context =
        if baseline.commands().snapshot().semantic_key() == registry.snapshot().semantic_key() {
            baseline
        } else {
            Arc::new(baseline.with_command_store(registry.snapshot().shared_registry()))
        };
    let config = tcl_lexer::LexerConfig::for_file_grammar(environment.grammar());
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        context,
        config,
    );
    let declared = tcl_compiler::analyser::utils::document_declared_surface(text, None, dialect);
    let entry = tcl_compiler::command_binding::SourceAnalysisEntry::for_supplied_source(
        registry,
        &input,
        config,
        Some(input.unit_profile()),
    );
    CompilationUnit::build_with_analysis_input(
        text,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(input.unit_profile()),
            external_call_sites,
            declared_commands: Some(&declared),
        },
        Some(&entry),
        &input,
    )
    .with_interprocedural(registry, Some(input.unit_profile()))
}

/// Build a source-advice unit from the document's complete retained analysis.
/// Missing, changed-source, foreign-store or changed-grammar ownership refuses
/// the build. This does not supply a native execution entry or erasure licence.
#[must_use]
pub fn compilation_unit_uncached_from_analysis(
    text: &str,
    registry: &CommandRegistry,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    external_call_sites: Option<&CallSiteEvidence>,
) -> Option<CompilationUnit> {
    if analysis.analysis_context_unavailable.is_some() {
        return None;
    }
    let (_image, config) = tcl_compiler::source_graph::current_analysis(text, analysis)?;
    let input = analysis.resolved_input.as_ref()?;
    tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
        registry,
        input,
        config,
        Some(input.unit_profile()),
    )?;
    if let Some(unit) = analysis.original_optimiser_source_unit(text, registry, external_call_sites)
    {
        return Some(unit);
    }
    let declared = tcl_compiler::analyser::utils::document_declared_surface(
        text,
        None,
        input.unit_profile().name,
    );
    let entry = tcl_compiler::command_binding::SourceAnalysisEntry::for_supplied_source(
        registry,
        input,
        config,
        Some(input.unit_profile()),
    );
    Some(CompilationUnit::build_with_analysis_input(
        text,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(input.unit_profile()),
            external_call_sites,
            declared_commands: Some(&declared),
        },
        Some(&entry),
        input,
    ))
}

/// Uncached compiler findings for an already analysed document. The actual
/// input owns availability, unit profile and lexer configuration. A refused
/// supplied build returns no compiler projections; it never enters the scalar
/// standalone API. Diagnostic subjects and source contexts stay intact.
#[must_use]
pub fn compiler_check_diagnostics_uncached_from_analysis(
    text: &str,
    registry: &CommandRegistry,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    generic_patterns: Option<&[String]>,
    external_call_sites: Option<&CallSiteEvidence>,
) -> CompilerDiagnostics {
    let Some(cu) =
        compilation_unit_uncached_from_analysis(text, registry, analysis, external_call_sites)
    else {
        return CompilerDiagnostics {
            checks: Vec::new(),
            optimisations: Vec::new(),
            taint_replays: tcl_compiler::taint::TaintReplayAnalysis::unavailable(),
        };
    };
    let profile = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::unit_profile);
    let cu = cu.with_interprocedural(registry, profile);
    compiler_diagnostics_from_unit(&cu, registry, profile, generic_patterns)
}
