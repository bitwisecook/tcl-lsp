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

//! Top-level diagnostic aggregator.
//!
//! Runs every per-pass diagnostic source (SCCP constant-branch,
//! GVN redundancies, shimmer warnings, taint warnings, optimiser
//! suggestions) against a [`CompilationUnit`] and returns a
//! single flat list of diagnostics ready for the LSP.
//!
//! Hosts the [`Diagnostic`] supertype and the [`run_all_checks`] entry
//! point that calls every analysis pass — SCCP, GVN, shimmer (S100–S102),
//! and taint (T100–T101) — and collects results into a flat list.

use tcl_lexer::Span;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::gvn::{find_loop_invariants, find_partial_redundancies, find_redundancies};
use crate::irules_checks::{
    CodeFix, IrulesCheckWarning, find_collect_flow_warnings, find_generic_static_name_warnings,
    find_hoistable_set_warnings, find_http_flow_warnings, find_unguarded_drop_warnings,
};
use crate::path_concat::{PathConcatWarning, find_path_concat_warnings};
use crate::sccp::ConstantBranch;
use crate::shimmer::{
    ShimmerWarning, ThunkingWarning, find_byte_array_warnings, find_shimmer_warnings,
    find_thunking_warnings,
};
use crate::taint::{
    TaintWarning, find_destructive_file_warnings, find_setter_constraint_warnings,
    find_taint_warnings, is_irules_dialect,
};
use crate::uri_split::find_uri_split_suggestions;
use tcl_registry::CommandRegistry;

// Unified diagnostic envelope

pub use tcl_core_types::DiagCode;
/// Severity of a compiler diagnostic — the shared [`tcl_core_types::Severity`].
pub use tcl_core_types::Severity;

/// A unified diagnostic emitted by the compiler-checks pipeline.
///
/// Downstream consumers (LSP, CLI) lower this into their native
/// diagnostic types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// Source span.
    pub span: Span,
    /// Diagnostic code (e.g. `"O105"`, `"S100"`, `"T100"`).
    pub code: DiagCode,
    /// Short category name (`"gvn"`, `"shimmer"`, `"taint"`, …).
    pub category: String,
    /// Severity.
    pub severity: Severity,
    /// Formatted message.
    pub message: String,
    /// Optional replacement text for a code-action fix that rewrites
    /// the diagnostic's span. `None` when the check has no suggested
    /// fix or could not build one. Consumers that surface code actions
    /// (LSP `CodeAction`, CLI auto-fix) should plumb this through.
    pub replacement: Option<String>,
    /// Suggested fixes whose range is independent of `span` — insertions or
    /// out-of-span replacements that `replacement` (a same-span rewrite) cannot
    /// express (e.g. the IRULE5002/5004 "insert `return`" fixes). Empty when the
    /// check has no such fix. Consumers emit one `TextEdit` per fix at its own
    /// `span`.
    pub fixes: Vec<CodeFix>,
}

impl Diagnostic {
    fn from_constant_branch(cb: &ConstantBranch) -> Self {
        Self {
            // Use the branch terminator's span when available so
            // editors / CLIs can highlight the condition that was
            // folded, rather than the start of the file.
            span: cb.span.unwrap_or_else(|| Span::new(0, 0)),
            code: DiagCode::O100,
            category: "sccp".into(),
            severity: Severity::Hint,
            message: format!(
                "condition in block {} is a constant {} — take target `{}`",
                cb.block, cb.value, cb.taken_target
            ),
            replacement: None,
            fixes: Vec::new(),
        }
    }

    fn from_shimmer(w: &ShimmerWarning) -> Self {
        Self {
            span: w.span,
            code: w.code,
            category: "shimmer".into(),
            // S100 (single shimmer outside a loop) is informational; S101
            // (per-iteration loop shimmer) is a warning.
            severity: if w.code == DiagCode::S100 {
                Severity::Info
            } else {
                Severity::Warning
            },
            message: w.message.clone(),
            replacement: None,
            fixes: Vec::new(),
        }
    }

    fn from_thunking(w: &ThunkingWarning) -> Self {
        Self {
            span: w.span,
            code: w.code,
            category: "shimmer".into(),
            severity: Severity::Warning,
            message: w.message.clone(),
            replacement: None,
            fixes: Vec::new(),
        }
    }

    fn from_taint(w: &TaintWarning) -> Self {
        Self {
            span: w.span,
            code: w.code,
            category: "taint".into(),
            // The taint family is a Warning by default (not an Error); T106
            // and IRULE3103 are informational.
            severity: match w.code.as_str() {
                "T106" | "IRULE3103" => Severity::Info,
                _ => Severity::Warning,
            },
            message: w.message.clone(),
            replacement: w.replacement.clone(),
            fixes: w.fixes.clone(),
        }
    }

    fn from_irules_check(w: &IrulesCheckWarning) -> Self {
        Self {
            span: w.span,
            code: w.code,
            category: "irules".into(),
            // IRULE1007/1008 (collect-without-release / release-without-
            // collect) are hard connection-state errors; IRULE4004 is
            // informational. Everything else is a Warning.
            severity: match w.code.as_str() {
                "IRULE1007" | "IRULE1008" => Severity::Error,
                "IRULE4004" => Severity::Info,
                _ => Severity::Warning,
            },
            message: w.message.clone(),
            replacement: w.replacement.clone(),
            fixes: w.fixes.clone(),
        }
    }

    fn from_redundant(r: &crate::gvn::RedundantComputation) -> Self {
        Self {
            span: r.span,
            code: r.code,
            category: "gvn".into(),
            severity: Severity::Suggestion,
            message: r.message.clone(),
            replacement: None,
            fixes: Vec::new(),
        }
    }

    fn from_path_concat(w: &PathConcatWarning) -> Self {
        Self {
            span: w.span,
            code: w.code,
            category: "taint".into(),
            // W201 (path concatenation) is a Hint, not a Warning.
            severity: match w.code.as_str() {
                "W201" => Severity::Hint,
                _ => Severity::Warning,
            },
            message: w.message.clone(),
            replacement: w.replacement.clone(),
            fixes: Vec::new(),
        }
    }
}

// Entry point

/// Absolutise a per-function diagnostic's span by adding the unit's
/// `base_offset`.  The per-function checks read spans from `fu.cfg`/`fu.ssa`/
/// `fu.sccp`, which are relative to that offset (0 for a real-position build,
/// the body offset for a memoised offset-0 unit, Approach B).
fn shift(fu: &FunctionUnit, mut d: Diagnostic) -> Diagnostic {
    d.span = fu.abs_span(d.span);
    d
}

/// Run every check against a compilation unit and return a
/// flat list of diagnostics.
///
/// Order of the returned diagnostics is stable but not guaranteed
/// across releases — consumers that care about ordering sort on
/// `(span.start(), code)` themselves.
#[must_use]
pub fn run_all_checks(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<Diagnostic> {
    run_all_checks_with_generic_patterns(cu, registry, dialect, None)
}

/// Like [`run_all_checks`] but with explicit IRULE4002 generic-name patterns
/// (`None` → default set, `Some(&[])` → disabled, `Some(custom)` → replace).
/// Used by the no-salsa-input fallback so user `genericVariablePatterns`
/// configuration is honoured even off the memoised path.
#[must_use]
pub fn run_all_checks_with_generic_patterns(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    generic_patterns: Option<&[String]>,
) -> Vec<Diagnostic> {
    let solved = crate::taint_interproc::solve_interprocedural_taints(cu, registry, dialect);
    run_all_checks_with_solved_and_patterns(cu, registry, dialect, &solved, generic_patterns)
}

/// Like [`run_all_checks`] but consumes a pre-computed interprocedural taint
/// solve, so a caller can supply a memoised one (SRV-INCREMENTAL 2b) instead of
/// re-solving the whole module on every edit — that solve is ~95% of this pass.
#[must_use]
pub fn run_all_checks_with_solved(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    solved: &crate::taint_interproc::InterprocTaintResult,
) -> Vec<Diagnostic> {
    run_all_checks_with_solved_and_patterns(cu, registry, dialect, solved, None)
}

/// Like [`run_all_checks_with_solved`] but with explicit IRULE4002 generic-name
/// patterns (see [`run_all_checks_with_generic_patterns`]).
#[must_use]
pub fn run_all_checks_with_solved_and_patterns(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    solved: &crate::taint_interproc::InterprocTaintResult,
    generic_patterns: Option<&[String]>,
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();

    // Per-function non-taint checks (SCCP constant branches, GVN redundancies,
    // shimmer / thunking / byte-array), computed on each procedure — **and**,
    // via `analysable_body_function_units`, every `TclOO` method body and
    // synthetic body unit (`apply` lambda, `namespace eval` body) too, so the
    // shimmer family `function_nontaint_checks` folds in reaches those places
    // as well (no separate widening loop needed here: unlike
    // `tcl-lsp-db::proc_taint_solve`'s memoised path below, which still
    // iterates the proc-only `analysable_functions` and so keeps its own
    // explicit `shimmer_family_checks` pass over methods/body units) —
    // rebased to its offset. Factored into [`function_nontaint_checks`] so
    // the LSP db can memoise it per procedure on the offset-0 `FnLatticeKey`
    // (SRV-INCREMENTAL 2a): an unedited procedure's checks are a cache hit
    // instead of recomputed over the whole unit every edit.
    for fu in cu.analysable_body_function_units() {
        for d in function_nontaint_checks(fu, registry, dialect, cu.method_instance_vars(&fu.name))
        {
            out.push(shift(fu, d));
        }
    }

    // Taint (per-function, reading the interprocedural `solved`) + iRules
    // module-level checks — the half that is not per-function-pure.
    push_taint_and_module_checks(cu, registry, dialect, solved, generic_patterns, &mut out);

    // Deterministic ordering (producers emit in `HashMap`-iteration order);
    // see [`sort_diagnostics`].
    sort_diagnostics(&mut out);

    out
}

/// Every per-function **non-taint** check for one procedure, returned at the
/// procedure's **offset-0** spans (the caller rebases — `shift` for a built
/// unit, or `+ body_offset` for the LSP db's offset-0 memo): SCCP constant
/// branches, GVN full / partial / loop redundancies, and the intrep-shimmer
/// family (shimmer S100/S101, thunking S102, byte-array S110).
///
/// These read only the `FunctionUnit`, so the LSP db wraps this in a salsa query
/// keyed on the offset-0 `FnLatticeKey` (SRV-INCREMENTAL 2a) — an unedited
/// procedure's checks are a cache hit.  The S110 `*::payload` byte-command set is
/// dialect-gated (empty outside iRules).
#[must_use]
pub fn function_nontaint_checks<S: std::hash::BuildHasher>(
    fu: &FunctionUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    instance_vars: Option<&std::collections::HashSet<String, S>>,
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();
    for cb in &fu.sccp.constant_branches {
        out.push(Diagnostic::from_constant_branch(cb));
    }
    for r in find_redundancies(registry, &fu.cfg, &fu.ssa, dialect) {
        out.push(Diagnostic::from_redundant(&r));
    }
    for r in find_partial_redundancies(registry, &fu.cfg, &fu.ssa, dialect) {
        out.push(Diagnostic::from_redundant(&r));
    }
    for r in find_loop_invariants(registry, &fu.cfg, &fu.ssa, dialect) {
        out.push(Diagnostic::from_redundant(&r));
    }
    out.extend(shimmer_family_checks(fu, registry, dialect, instance_vars));
    out
}

/// The intrep-shimmer diagnostic family alone for one function body: use-site
/// / expression / phi shimmer (S100/S101), loop-oscillation thunking (S102),
/// and byte-array corruption (S110).
///
/// Split out of [`function_nontaint_checks`] so it can also run over `TclOO`
/// method bodies and synthetic body units (`apply` lambdas, `namespace eval`
/// bodies) — [`CompilationUnit.methods`](crate::compilation_unit::CompilationUnit::methods)
/// and `.body_units` are excluded from the SCCP/GVN half (`cu.analysable_functions()`
/// only walks the top level and procedures), so without this split every
/// shimmer diagnostic was silently unreachable inside a method or a
/// `namespace eval` body. The S110 `*::payload` byte-command set is
/// dialect-gated (empty outside iRules).
#[must_use]
pub fn shimmer_family_checks<S: std::hash::BuildHasher>(
    fu: &FunctionUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    instance_vars: Option<&std::collections::HashSet<String, S>>,
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();
    for w in find_shimmer_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.types,
        &fu.sccp.executable_blocks,
        registry,
        &fu.sccp.values,
    ) {
        out.push(Diagnostic::from_shimmer(&w));
    }
    for w in find_thunking_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.types,
        &fu.sccp.executable_blocks,
        instance_vars,
    ) {
        out.push(Diagnostic::from_thunking(&w));
    }
    let payload_layouts = if is_irules_dialect(dialect) {
        registry.byte_array_payload_layouts()
    } else {
        std::collections::HashMap::new()
    };
    for w in find_byte_array_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.sccp.executable_blocks,
        registry,
        &payload_layouts,
    ) {
        out.push(Diagnostic::from_shimmer(&w));
    }
    out
}

/// Append the taint-family warnings (per function, reading the interprocedural
/// `solved` taints) and the iRules module-level flow checks — the half of
/// [`run_all_checks_with_solved`] that is *not* per-function-pure (taint already
/// arrives pre-solved), so SRV-INCREMENTAL 2a does not memoise it.
pub fn push_taint_and_module_checks(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    solved: &crate::taint_interproc::InterprocTaintResult,
    generic_patterns: Option<&[String]>,
    out: &mut Vec<Diagnostic>,
) {
    // The interprocedural solve produces colour-aware return summaries and
    // parameter entry taints; its `top_taints` / `proc_taints` supersede the bare
    // per-function `fu.taints` for the warning families so a tainted argument
    // flowing into a callee parameter and then a sink is reported (cross-proc
    // entry-taint).
    let proc_qnames: std::collections::HashSet<&str> =
        cu.procedures.keys().map(String::as_str).collect();
    // `trace add variable` / `trace variable` lets a runtime callback rewrite
    // a variable's value in ways static analysis can't see; a name traced
    // anywhere in the module is conservatively treated as tainted everywhere
    // — see `taint::apply_module_variable_traces`.
    let module_traces = crate::compilation_unit::ModuleTraceFacts {
        traced_variables: &cu.ir_module.traced_variables,
        has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
    };
    // `analysable_body_function_units` (not `analysable_functions`) so a sink
    // inside a TclOO method body — or an `apply` lambda / `namespace eval`
    // body — is still checked; see its doc comment.
    for fu in cu.analysable_body_function_units() {
        let taints = crate::taint::apply_module_variable_traces(
            solved.taints_for(&fu.name, &fu.taints).clone(),
            &fu.ssa,
            module_traces,
        );
        // A namespace-scoped proc of the same name shadows a builtin for
        // every call made from that namespace (`proc ::myns::puts {...}`
        // shadows `puts` inside `::myns`) — see `taint::shadowed_builtin_names`.
        let namespace = crate::optimiser::helpers::naming::namespace_from_qualified(&fu.name);
        let shadowed =
            crate::taint::shadowed_builtin_names(&namespace, proc_qnames.iter().copied(), registry);
        for w in find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &taints,
            &fu.sccp.executable_blocks,
            registry,
            dialect,
            &shadowed,
        ) {
            out.push(shift(fu, Diagnostic::from_taint(&w)));
        }
        // IRULE3101 is iRules-only. `find_setter_constraint_warnings`
        // gates internally too; this outer check is defence-in-depth
        // and lets us skip the entire walk under non-iRules dialects.
        if is_irules_dialect(dialect) {
            for w in find_setter_constraint_warnings(
                registry,
                &fu.cfg,
                &fu.ssa,
                &taints,
                &fu.sccp.executable_blocks,
                dialect,
            ) {
                out.push(shift(fu, Diagnostic::from_taint(&w)));
            }
            // IRULE3103 — `*::uri` getter + manual decomposition.
            for w in find_uri_split_suggestions(
                &fu.cfg,
                &fu.ssa,
                Some(&fu.sccp.values),
                &fu.sccp.executable_blocks,
                registry,
                dialect,
            ) {
                out.push(shift(fu, Diagnostic::from_taint(&w)));
            }
        }
        for w in find_path_concat_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.rendered_props,
            &taints,
            &fu.sccp.executable_blocks,
        ) {
            out.push(shift(fu, Diagnostic::from_path_concat(&w)));
        }
        // W313: destructive `file` ops on a variable/tainted path.
        for w in find_destructive_file_warnings(
            &fu.cfg,
            &fu.ssa,
            &taints,
            &fu.sccp.executable_blocks,
            registry,
        ) {
            out.push(shift(fu, Diagnostic::from_taint(&w)));
        }
    }

    // iRules-dialect non-taint (module-level / control-flow) checks.
    push_irules_flow_checks(cu, registry, dialect, generic_patterns, out);
}

/// Append the iRules-dialect module-level checks (IRULE5002/5004 +
/// IRULE1005-1008/1201/1202/4004).
///
/// Each helper is dialect-gated internally (returns empty for non-iRules
/// dialects), so calling them unconditionally is correct.  Split out of
/// [`run_all_checks`] to keep that aggregator within the line budget.
///
/// The unnormalised-getter warning (IRULE3102) is deliberately *not*
/// aggregated here: the analyser's per-command emitter
/// (`analyser::irules_event_checks`) owns it for the LSP/CLI stream — it
/// anchors on the getter word itself and reaches nested `[…]`
/// substitutions through the nested-segment dispatch.  Emitting the
/// CFG-side scan (`find_unnormalised_getter_warnings`, still used by the
/// compiler explorer's own view) here as well double-reported every site
/// with a second, wider span.
fn push_irules_flow_checks(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    generic_patterns: Option<&[String]>,
    out: &mut Vec<Diagnostic>,
) {
    for w in find_unguarded_drop_warnings(cu, dialect) {
        out.push(Diagnostic::from_irules_check(&w));
    }
    for w in find_collect_flow_warnings(cu, registry, dialect) {
        out.push(Diagnostic::from_irules_check(&w));
    }
    for w in find_http_flow_warnings(cu, dialect) {
        out.push(Diagnostic::from_irules_check(&w));
    }
    for w in find_hoistable_set_warnings(cu, dialect) {
        out.push(Diagnostic::from_irules_check(&w));
    }
    for w in find_generic_static_name_warnings(cu, dialect, generic_patterns) {
        out.push(Diagnostic::from_irules_check(&w));
    }
}

/// Impose a deterministic total order on diagnostics.
///
/// See [`run_all_checks`] — the per-pass producers emit in `HashMap`-iteration
/// order, so a stable sort on `(span, code, category, severity, message,
/// replacement)` is what makes the output byte-identical between the memoised
/// per-procedure build and the whole-module build.
pub fn sort_diagnostics(out: &mut [Diagnostic]) {
    out.sort_by(|a, b| {
        (
            a.span.start(),
            a.span.end(),
            &a.code,
            &a.category,
            a.severity.as_str(),
            &a.message,
            &a.replacement,
        )
            .cmp(&(
                b.span.start(),
                b.span.end(),
                &b.code,
                &b.category,
                b.severity.as_str(),
                &b.message,
                &b.replacement,
            ))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        let mut r = CommandRegistry::build_default();
        r.load_irules();
        r
    }

    /// Regression: `function_nontaint_checks` (called per unit from the
    /// `analysable_body_function_units` loop, which already reaches `TclOO`
    /// method bodies) folds in `shimmer_family_checks` itself — a second,
    /// separate loop explicitly re-walking methods/body-units for shimmer
    /// would double-emit every S100/S101/S102/S110 inside one. Exactly this
    /// double-count was introduced transiently when two independent fixes
    /// (this shimmer-coverage widening and #872's taint-coverage widening)
    /// were rebased together: #872 widened the base loop from
    /// `analysable_functions` to `analysable_body_function_units`, which
    /// made the shimmer-only top-up loop redundant.
    #[test]
    fn shimmer_not_double_counted_in_tcloo_method_body() {
        let src = "oo::class create C {\n    method m {} {\n        set x hello\n        incr x\n    }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let diags = run_all_checks(&cu, &registry(), None);
        let s100: Vec<_> = diags.iter().filter(|d| d.code.as_str() == "S100").collect();
        assert_eq!(s100.len(), 1, "expected exactly one S100, got {s100:?}");
    }

    /// Same double-count guard for a `namespace eval` body (the other
    /// synthetic body-unit kind `analysable_body_function_units` reaches).
    #[test]
    fn shimmer_not_double_counted_in_namespace_eval_body() {
        let src = "namespace eval ns {\n    set x hello\n    incr x\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let diags = run_all_checks(&cu, &registry(), None);
        let s100: Vec<_> = diags.iter().filter(|d| d.code.as_str() == "S100").collect();
        assert_eq!(s100.len(), 1, "expected exactly one S100, got {s100:?}");
    }

    #[test]
    fn run_all_checks_on_empty_source_is_empty() {
        let cu = CompilationUnit::build_for("", &registry(), false);
        assert!(run_all_checks(&cu, &registry(), None).is_empty());
    }

    /// (FP guard) `my variable count` links an object-instance variable whose
    /// true intrep depends on another method's last write, not the nominal
    /// return type of `my`. Before the registry fix (`oo_my.rs`'s `variable`
    /// subcommand), this spuriously claimed a String->Int shimmer on `incr`.
    #[test]
    fn no_shimmer_for_tcloo_instance_variable_linked_via_my_variable() {
        let src = "oo::class create Counter {\n    method bump {} {\n        my variable count\n        incr count\n    }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let diags = run_all_checks(&cu, &registry(), None);
        let shimmer: Vec<_> = diags
            .iter()
            .filter(|d| matches!(d.code.as_str(), "S100" | "S101"))
            .collect();
        assert!(
            shimmer.is_empty(),
            "'my variable'-linked instance var must not spuriously shimmer: {shimmer:?}"
        );
    }

    /// (TN control) The same method body, minus `my variable`, still fires
    /// S100/S101 — proves the guard above is keyed on the scope-alias link,
    /// not a blanket regression of `incr`-on-String detection inside methods.
    #[test]
    fn shimmer_still_fires_in_tcloo_method_without_my_variable() {
        let src = "oo::class create Counter {\n    method bump {} {\n        set count hello\n        incr count\n    }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let diags = run_all_checks(&cu, &registry(), None);
        let shimmer: Vec<_> = diags
            .iter()
            .filter(|d| matches!(d.code.as_str(), "S100" | "S101"))
            .collect();
        assert!(
            !shimmer.is_empty(),
            "expected a shimmer for the untraced-and-unlinked case: {diags:?}"
        );
    }

    #[test]
    fn run_all_checks_reports_sccp_constant_branch() {
        let cu =
            CompilationUnit::build_for("if {1} { set x 1 } else { set y 2 }", &registry(), false);
        let diagnostics = run_all_checks(&cu, &registry(), None);
        // SCCP should have flagged the constant-true branch.
        let has_sccp = diagnostics.iter().any(|d| d.category == "sccp");
        assert!(has_sccp, "expected an SCCP diagnostic, got {diagnostics:?}");
    }

    #[test]
    fn shimmer_s100_is_info_s101_is_warning() {
        // S100 — a single shimmer outside any loop is informational.
        let cu = CompilationUnit::build_for("set x \"hello\"\nincr x", &registry(), false);
        let diags = run_all_checks(&cu, &registry(), None);
        let s100 = diags
            .iter()
            .find(|d| d.code == DiagCode::S100)
            .expect("expected an S100 shimmer");
        assert_eq!(s100.severity, Severity::Info, "S100 must be Info: {s100:?}");
        assert_eq!(s100.severity.as_str(), "info");

        // S101 — a per-iteration shimmer inside a loop is a warning.
        let cu = CompilationUnit::build_for(
            "proc f {l} {\n  foreach x $l {\n    set b [lindex $x 0]\n  }\n}\n",
            &registry(),
            false,
        );
        let diags = run_all_checks(&cu, &registry(), None);
        let s101 = diags
            .iter()
            .find(|d| d.code == DiagCode::S101)
            .expect("expected an S101 shimmer");
        assert_eq!(
            s101.severity,
            Severity::Warning,
            "S101 must be Warning: {s101:?}"
        );
    }

    #[test]
    fn diagnostic_from_redundant_preserves_code() {
        let r = crate::gvn::RedundantComputation::new(
            Span::new(0, 5),
            Span::new(10, 15),
            "llength $x",
            DiagCode::O105,
            "msg",
        );
        let d = Diagnostic::from_redundant(&r);
        assert_eq!(d.code, DiagCode::O105);
        assert_eq!(d.category, "gvn");
    }

    #[test]
    fn run_all_checks_reports_irule3001_end_to_end() {
        let cu = CompilationUnit::build_for(
            "set u [HTTP::uri]\nHTTP::respond 200 content $u",
            &registry(),
            false,
        )
        .with_interprocedural(&registry(), Some("f5-irules"));
        let diagnostics = run_all_checks(&cu, &registry(), Some("f5-irules"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == DiagCode::Irule3001 && d.category == "taint"),
            "expected IRULE3001, got {diagnostics:?}",
        );
    }

    #[test]
    fn run_all_checks_reports_irule3101_end_to_end() {
        let cu = CompilationUnit::build_for("HTTP::uri foo", &registry(), false)
            .with_interprocedural(&registry(), Some("f5-irules"));
        let diagnostics = run_all_checks(&cu, &registry(), Some("f5-irules"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == DiagCode::Irule3101 && d.category == "taint"),
            "expected IRULE3101, got {diagnostics:?}",
        );
    }

    #[test]
    fn run_all_checks_reports_s110_payload_roundtrip_end_to_end() {
        let cu = CompilationUnit::build_for(
            "when CLIENT_DATA {\n  set p [TCP::payload]\n  set q [string map {a b} $p]\n  \
             TCP::payload replace 0 100 $q\n}",
            &registry(),
            false,
        )
        .with_interprocedural(&registry(), Some("f5-irules"));
        let diagnostics = run_all_checks(&cu, &registry(), Some("f5-irules"));
        let hit = diagnostics
            .iter()
            .find(|d| d.code == DiagCode::S110)
            .unwrap_or_else(|| panic!("expected S110, got {diagnostics:?}"));
        assert_eq!(hit.category, "shimmer");
        assert_eq!(hit.severity, Severity::Warning, "S110 must be a Warning");
    }

    /// Under a non-iRules dialect the `*::payload` layout set is empty, so a
    /// plain-Tcl document that merely names a `*::payload` command is silent.
    #[test]
    fn run_all_checks_s110_silent_outside_irules_dialect() {
        let src = "proc f {} {\n  set p [TCP::payload]\n  set q [string map {a b} $p]\n  \
                   TCP::payload replace 0 100 $q\n}";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let diagnostics = run_all_checks(&cu, &registry(), None);
        assert!(
            diagnostics.iter().all(|d| d.code != DiagCode::S110),
            "S110 must not fire under plain Tcl: {diagnostics:?}"
        );
    }

    #[test]
    fn run_all_checks_does_not_double_report_irule3102() {
        // IRULE3102 ownership: the analyser's per-command emitter
        // (`analyser::irules_event_checks`) reports it for the LSP/CLI
        // stream with a tight getter-word anchor; aggregating the CFG-side
        // scan here as well double-reported every site with a second,
        // wider span. `run_all_checks` must therefore stay silent — the
        // analyser end-to-end assertion lives alongside.
        let cu = CompilationUnit::build_for("set u [HTTP::uri]", &registry(), false)
            .with_interprocedural(&registry(), Some("f5-irules"));
        let diagnostics = run_all_checks(&cu, &registry(), Some("f5-irules"));
        assert!(
            diagnostics.iter().all(|d| d.code != DiagCode::Irule3102),
            "run_all_checks must not double-report IRULE3102: {diagnostics:?}"
        );

        // The analyser stream owns the code (single, tight instance).
        let mut analyser = crate::analyser::Analyser::new();
        let result = analyser.analyse(
            "when HTTP_REQUEST {\n    set u [HTTP::uri]\n}\n",
            "f5-irules",
        );
        let hits: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::Irule3102)
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "the analyser stream reports IRULE3102 exactly once: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn run_all_checks_irule3101_gated_by_dialect() {
        // Regression: `HTTP::uri foo` is a non-slash-prefixed literal.
        // Under `f5-irules` it should warn with IRULE3101; under plain
        // Tcl it must stay silent (user-defined proc named "HTTP::uri"
        // must not trigger a high-severity error).
        let src = "HTTP::uri foo";

        let irules_cu = CompilationUnit::build_for(src, &registry(), false)
            .with_interprocedural(&registry(), Some("f5-irules"));
        let irules_diags = run_all_checks(&irules_cu, &registry(), Some("f5-irules"));
        assert!(
            irules_diags.iter().any(|d| d.code == DiagCode::Irule3101),
            "expected IRULE3101 under f5-irules dialect, got {irules_diags:?}",
        );

        let plain_cu = CompilationUnit::build_for(src, &registry(), false);
        let plain_diags = run_all_checks(&plain_cu, &registry(), None);
        assert!(
            plain_diags.iter().all(|d| d.code != DiagCode::Irule3101),
            "no IRULE3101 without dialect, got {plain_diags:?}",
        );
    }

    #[test]
    fn run_all_checks_irule_codes_gated_by_dialect() {
        let cu = CompilationUnit::build_for(
            "set u [HTTP::uri]\nHTTP::respond 200 content $u",
            &registry(),
            false,
        );
        let diagnostics = run_all_checks(&cu, &registry(), None);
        assert!(
            diagnostics
                .iter()
                .all(|d| !d.code.as_str().starts_with("IRULE")),
            "no IRULE diagnostics without dialect, got {diagnostics:?}",
        );
    }

    #[test]
    fn run_all_checks_reports_w201_path_concat() {
        let cu = CompilationUnit::build_for("set x 42\nset p \"/tmp/$x\"", &registry(), false);
        let diagnostics = run_all_checks(&cu, &registry(), None);
        let w201 = diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W201 && d.category == "taint")
            .unwrap_or_else(|| panic!("expected W201 path-concat diagnostic, got {diagnostics:?}"));
        // The `[file join …]` suggestion must survive the lowering
        // from `PathConcatWarning` to `Diagnostic`.
        let replacement = w201
            .replacement
            .as_deref()
            .expect("W201 diagnostic should carry a [file join] replacement");
        assert!(
            replacement.starts_with("[file join ") && replacement.ends_with(']'),
            "unexpected replacement text: {replacement:?}",
        );
    }
}
