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

//! Residual coverage for several under-covered `tcl-compiler` analysis
//! files.  This suite deliberately targets the *remaining* uncovered branches
//! that the existing suites (`analyser.rs`, `checks.rs`, `core_analyses.rs`,
//! `fp_depth.rs`, `inlining.rs`, `dataflow.rs`, `optimiser_*`) and the
//! in-crate unit tests already miss:
//!
//!   * `src/scan_predicate.rs` — every `scan`/format-string conversion
//!     specifier, the width/precision/size/suppression skips, and the decline
//!     (`None`) paths.  Driven both directly (`scan_provably_no_match`, which is
//!     `pub`) and end-to-end through the W210 read-before-set surface.
//!   * `src/auto_path_eval.rs` — the `lappend auto_path` mini-evaluator: the
//!     `[file join]` / `[file dirname]` / `[info script]` idioms, the tokeniser
//!     quirks (brace groups, quoted words, braced-`[` as a literal,
//!     unterminated delimiters), and the POSIX path helpers.
//!   * `src/place_bridge.rs` — the place/SSA-bridge def/read resolution: dict
//!     paths, must-alias kills, braced-literal data words, upvar aliases,
//!     `namespace upvar`, trace targets, whole-array reads, and the
//!     terminator-read paths.
//!   * `src/lattice_rebase.rs` — the memoised-unit offset rebase (SCCP value
//!     rebasing across a shift): the `Foreach` / `Try` / `UpFrame` / constant-
//!     branch / loop-node statement variants the existing shift test misses.
//!   * `src/inlining/mod.rs` — `classify_proc` / `inline_module` decline reasons
//!     (size limits, recursion, side-effects, non-namespace-invariant calls,
//!     globals, `{*}` expansion) `inlining.rs` misses.
//!   * `src/analyser/diagnostics/fp/rch.rs` — the reachability (O107) FP
//!     suppressor carve-outs: code after `return` / `break` / `continue` /
//!     `error`, constant-false guards, and the various unreachable-region shapes,
//!     each with a fires/suppressed control pair.
//!
//! ## Proof discipline
//!
//! Where a test depends on a **Tcl-observable** fact — a `scan` conversion
//! result, what survives after `return` / `error`, a `file join` / `file
//! dirname` value, reachability after an unconditional exit — the expected value
//! is pinned to real `tclsh8.6` and `tclsh9.0` via `scripts/dev/tclsh_check.sh`
//! and cited with a `// tclsh:` comment (8.4 / 8.5 are not installed in this
//! environment).
//!
//! **Analysis-internal structure** — the lattice/place/predicate *shape*, the
//! rebased-span identity, the `InlineDecision` verdict, and which IR node a
//! splice produces — is **not** directly observable from a Tcl program's output,
//! so it is asserted structurally.  For `scan_predicate`, several inputs Tcl
//! itself leaves the output unset on (`scan nan %f`, the second `%d` of
//! `%*d%d`) are nonetheless *not* provable-no-match for the predicate, which is
//! a deliberate sound over-approximation (it never fires a false W210); those
//! cases assert the conservative `false` return and say so.

use tcl_registry::value_transfer::NumericValue;

use crate::body_identity;
use crate::function_identity;
use function_identity::OriginalFixtureFunctions;

use body_identity::{fixture_id, fixture_id_if_present};

use tcl_codegen::inlining::{
    InlineDecision, SMALL_BODY_THRESHOLD, classify_proc, count_statements, inline_module,
};
use tcl_compiler::analyser::Analyser;
use tcl_compiler::auto_path_eval::evaluate_auto_path_expr;
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::ir::{Module, Statement};
use tcl_compiler::place_bridge::{
    build_resolve_context, def_places_at, element_writes_observed_by_reads, read_places_at,
    terminator_read_places, terminator_read_places_at,
};
use tcl_compiler::scan_predicate::scan_provably_no_match;
use tcl_compiler::var_escape::{ProcEscapeSummary, analyse_var_escape};
use tcl_names::place;
use tcl_names::place::{Index, LOCAL_NS, Place, PlaceKind, overlap};
use tcl_registry::CommandRegistry;
use tcl_registry::model::ingress::static_context_for;

mod auto_path;
mod binding_order;
mod handler_barriers;
mod handler_widening;
mod inlining;
mod places;
mod reachability;
mod rebase;
mod scan_predicate;

// Shared harness

/// Default dialect for the diagnostic-surface reproducers.
const D: &str = "tcl8.6";

fn reg() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

/// Every diagnostic code the analyser + `run_all_checks` surface for `src`
/// (optimisation codes excluded) — the W210/W211/W220 surface used by the
/// place-bridge and scan-predicate end-to-end proofs.
fn codes(src: &str, dialect: &str) -> Vec<String> {
    use tcl_compiler::compiler_checks::run_all_checks;
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes(src, dialect).iter().any(|c| c == code)
}

/// `O107` (unreachable dead code) is an optimiser-pass suggestion, not an
/// analyser/`run_all_checks` diagnostic — union the optimiser output to test
/// the reachability FP suppressor.
fn o107_fires(src: &str, dialect: &str) -> bool {
    use tcl_compiler::optimiser::manager::optimise_with_dialect;
    use tcl_core_types::DiagCode;
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| o.code == DiagCode::O107)
}

/// The context the unit proved before the statement of `fu`'s body spelled
/// `spelling` in `source`.
fn context_before<'a>(
    fu: &'a tcl_compiler::compilation_unit::FunctionUnit,
    source: &str,
    spelling: &str,
) -> &'a tcl_compiler::var_resolve::ResolveContext {
    let points = fu.ssa.point_contexts.as_ref().expect("point contexts");
    for (&id, block) in &fu.cfg.blocks {
        for (index, stmt) in block.statements.iter().enumerate() {
            let span = stmt.span();
            if source.get(span.start() as usize..span.end() as usize) == Some(spelling) {
                return points.before_statement(id, index);
            }
        }
    }
    panic!("no statement spelled {spelling:?}")
}

/// `source → Module`, for the inliner.
fn module_for(source: &str) -> Module {
    CompilationUnit::build_for(source, &reg(), false).ir_module
}

/// Run the whole-module inline transform.
fn inlined(source: &str) -> Module {
    inline_module(module_for(source), &reg())
}

/// Count statement-position calls to `command` across the module top level.
fn top_calls_to(module: &Module, command: &str) -> usize {
    module
        .top_level
        .statements
        .iter()
        .filter(|s| matches!(s, Statement::Call { command: c, .. } if c == command))
        .count()
}

/// Collect every def-place and read-place across a proc's CFG (mirrors the
/// `place_bridge` unit-test helper, exposed here over the public API).
fn places_of(src: &str, qname: &str) -> (Vec<Place>, Vec<Place>) {
    places_of_with(&reg(), src, qname)
}

fn places_of_with(r: &CommandRegistry, src: &str, qname: &str) -> (Vec<Place>, Vec<Place>) {
    let cu = CompilationUnit::build_for(src, r, false);
    let fu = cu.fixture_function(qname).expect("proc not found");
    // Each statement reads its names in the context the unit proved at that
    // point, as the consumers (dead stores, elimination) read them.
    let points = fu.ssa.point_contexts.as_ref().expect("point contexts");
    let mut defs = Vec::new();
    let mut reads = Vec::new();
    for (&id, block) in &fu.cfg.blocks {
        for (index, stmt) in block.statements.iter().enumerate() {
            defs.extend(def_places_at(stmt, id, index, points, r));
            reads.extend(read_places_at(stmt, id, index, points, r));
        }
        if let Some(term) = &block.terminator {
            reads.extend(terminator_read_places_at(term, id, points, r));
        }
    }
    (defs, reads)
}

/// Build `src` through the per-procedure lattice memo, exactly the way
/// `tcl-lsp-db`'s `function_lattice` does: a single-CFG rebuild under the
/// request's analysis context, whose command-trust snapshot is the module's
/// whole-module view. Returns the unit, how many times the memo callback was
/// reached, and the analysis context each request carried.
fn memoised_unit(
    src: &str,
) -> (
    CompilationUnit,
    usize,
    Vec<tcl_compiler::value_transfer::AnalysisContextKey>,
) {
    let registry = reg();
    let mut hits = 0usize;
    let mut contexts = Vec::new();
    let cu = CompilationUnit::build_for_memoized(
        src,
        tcl_compiler::compilation_unit::UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::default(),
            dialect: Some(tcl_registry::model::ingress::resolve_environment(D).analyser_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        &mut |req: &tcl_compiler::compilation_unit::LatticeRequest<'_>| -> FunctionUnit {
            hits += 1;
            contexts.push(req.analysis_context.clone());
            let prepared = tcl_compiler::cfg_builder::PreparedCfgContext::from_source_input(
                (
                    req.upvar_procs.clone(),
                    req.proc_params.clone(),
                    req.global_write_procs.clone(),
                    req.command_bindings.clone(),
                    req.procedure_summary_owners.clone(),
                ),
                &registry,
                req.source_metadata_input,
            );
            let cfg = tcl_compiler::cfg_builder::build_cfg_function_with_prepared_context(
                req.qname,
                req.body,
                true,
                &registry,
                req.plain_command_dispatch,
                &prepared,
                req.lexer_config,
            );
            let pc = tcl_compiler::compilation_unit::decode_param_constants(req.param_constants);
            let traced: std::collections::BTreeSet<String> =
                req.traced_variables.iter().cloned().collect();
            let command_trust = req.analysis_context.bindings.to_mutations();
            FunctionUnit::build_with_param_constants_and_classes_under(
                req.qname,
                cfg,
                req.params,
                tcl_compiler::compilation_unit::UnitDialect {
                    registry: &registry,
                    config: req.lexer_config,
                    source_metadata_input: req.source_metadata_input,
                },
                pc.as_ref(),
                &req.known_classes.iter().cloned().collect(),
                tcl_compiler::compilation_unit::ModuleAnalysisFacts {
                    trace: tcl_compiler::compilation_unit::ModuleTraceFacts {
                        traced_variables: &traced,
                        has_dynamic_variable_trace: req.has_dynamic_variable_trace,
                        deferred_writes: &req.analysis_context.deferred_writes,
                    },
                    analysis_context: req.analysis_context,
                    command_trust: &command_trust,
                },
            )
        },
    );
    (cu, hits, contexts)
}

/// Whether `::g`'s lattice proves its local `v` is the integer 3.
fn g_proves_v_is_three(cu: &CompilationUnit) -> bool {
    let Some(fu) = cu.procedures.get(&fixture_id(&cu.procedures, "::g")) else {
        return false;
    };
    let Some(sym) = fu.ssa.var_symbol("v") else {
        return false;
    };
    fu.sccp.values.iter().any(|((s, _), lv)| {
        *s == sym
            && matches!(
                lv,
                tcl_compiler::analyses::LatticeValue::Const(
                    tcl_compiler::analyses::ConstValue::Number(NumericValue::Int(3))
                )
            )
    })
}
