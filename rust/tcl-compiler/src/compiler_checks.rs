//! Top-level diagnostic aggregator.
//!
//! Runs every per-pass diagnostic source (SCCP constant-branch,
//! GVN redundancies, shimmer warnings, taint warnings, optimiser
//! suggestions) against a [`CompilationUnit`] and returns a
//! single flat list of diagnostics ready for the LSP.
//!
//! Ported from `core/compiler/compiler_checks.py` (C31). Lands the
//! [`Diagnostic`] supertype and the [`run_all_checks`] entry point
//! that calls every analysis pass — SCCP, GVN, shimmer (S100–S102),
//! and taint (T100–T101) — and collects results into a flat list.

use tcl_lexer::Span;

use crate::compilation_unit::CompilationUnit;
use crate::gvn::{find_loop_invariants, find_partial_redundancies, find_redundancies};
use crate::irules_checks::{
    find_collect_flow_warnings, find_hoistable_set_warnings, find_http_flow_warnings,
    find_unguarded_drop_warnings, find_unnormalised_getter_warnings, IrulesCheckWarning,
};
use crate::path_concat::{find_path_concat_warnings, PathConcatWarning};
use crate::sccp::ConstantBranch;
use crate::shimmer::{
    find_shimmer_warnings, find_thunking_warnings, ShimmerWarning, ThunkingWarning,
};
use crate::taint::{
    find_setter_constraint_warnings, find_taint_warnings, is_irules_dialect, TaintWarning,
};
use crate::uri_split::find_uri_split_suggestions;
use tcl_registry::CommandRegistry;

// ---------------------------------------------------------------------------
// Unified diagnostic envelope
// ---------------------------------------------------------------------------

/// Severity of a compiler diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// Informational hint.
    Hint,
    /// Style suggestion / refactor.
    Suggestion,
    /// Warning that may indicate a bug.
    Warning,
    /// Error.
    Error,
}

impl Severity {
    /// Lower-case wire form (`"hint"`, `"suggestion"`, `"warning"`,
    /// `"error"`). The Python LSP layer and the future native
    /// `tcl-lsp-server` both consume this form, so it lives on the
    /// type rather than being re-implemented in each binding.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hint => "hint",
            Self::Suggestion => "suggestion",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// A unified diagnostic emitted by the compiler-checks pipeline.
///
/// Downstream consumers (LSP, CLI) lower this into their native
/// diagnostic types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// Source span.
    pub span: Span,
    /// Diagnostic code (e.g. `"O105"`, `"S100"`, `"T100"`).
    pub code: String,
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
}

impl Diagnostic {
    fn from_constant_branch(cb: &ConstantBranch) -> Self {
        Self {
            // Use the branch terminator's span when available so
            // editors / CLIs can highlight the condition that was
            // folded, rather than the start of the file.
            span: cb.span.unwrap_or_else(|| Span::new(0, 0)),
            code: "O100".into(),
            category: "sccp".into(),
            severity: Severity::Hint,
            message: format!(
                "condition in block {} is a constant {} — take target `{}`",
                cb.block, cb.value, cb.taken_target
            ),
            replacement: None,
        }
    }

    fn from_shimmer(w: &ShimmerWarning) -> Self {
        Self {
            span: w.span,
            code: w.code.clone(),
            category: "shimmer".into(),
            severity: Severity::Warning,
            message: w.message.clone(),
            replacement: None,
        }
    }

    fn from_thunking(w: &ThunkingWarning) -> Self {
        Self {
            span: w.span,
            code: w.code.clone(),
            category: "shimmer".into(),
            severity: Severity::Warning,
            message: w.message.clone(),
            replacement: None,
        }
    }

    fn from_taint(w: &TaintWarning) -> Self {
        Self {
            span: w.span,
            code: w.code.clone(),
            category: "taint".into(),
            severity: Severity::Error,
            message: w.message.clone(),
            replacement: w.replacement.clone(),
        }
    }

    fn from_irules_check(w: &IrulesCheckWarning) -> Self {
        Self {
            span: w.span,
            code: w.code.clone(),
            category: "irules".into(),
            severity: Severity::Warning,
            message: w.message.clone(),
            replacement: w.replacement.clone(),
        }
    }

    fn from_redundant(r: &crate::gvn::RedundantComputation) -> Self {
        Self {
            span: r.span,
            code: r.code.clone(),
            category: "gvn".into(),
            severity: Severity::Suggestion,
            message: r.message.clone(),
            replacement: None,
        }
    }

    fn from_path_concat(w: &PathConcatWarning) -> Self {
        Self {
            span: w.span,
            code: w.code.clone(),
            category: "taint".into(),
            severity: Severity::Warning,
            message: w.message.clone(),
            replacement: w.replacement.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run every landed check against a compilation unit and return a
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
    let mut out: Vec<Diagnostic> = Vec::new();

    // SCCP constant branches (per function).
    for fu in cu.functions() {
        for cb in &fu.sccp.constant_branches {
            out.push(Diagnostic::from_constant_branch(cb));
        }
    }

    // GVN full + partial redundancies + loop invariants.
    for fu in cu.functions() {
        for r in find_redundancies(registry, &fu.cfg, &fu.ssa, dialect) {
            out.push(Diagnostic::from_redundant(&r));
        }
        for r in find_partial_redundancies(registry, &fu.cfg, &fu.ssa, dialect) {
            out.push(Diagnostic::from_redundant(&r));
        }
        for r in find_loop_invariants(registry, &fu.cfg, &fu.ssa, dialect) {
            out.push(Diagnostic::from_redundant(&r));
        }
    }

    // Shimmer + thunking.
    for fu in cu.functions() {
        for w in find_shimmer_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            registry,
        ) {
            out.push(Diagnostic::from_shimmer(&w));
        }
        for w in find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks) {
            out.push(Diagnostic::from_thunking(&w));
        }
    }

    // Taint.
    for fu in cu.functions() {
        for w in find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            registry,
            dialect,
        ) {
            out.push(Diagnostic::from_taint(&w));
        }
        // IRULE3101 is iRules-only. `find_setter_constraint_warnings`
        // gates internally too; this outer check is defence-in-depth
        // and lets us skip the entire walk under non-iRules dialects.
        if is_irules_dialect(dialect) {
            for w in find_setter_constraint_warnings(
                &fu.cfg,
                &fu.ssa,
                &fu.taints,
                &fu.sccp.executable_blocks,
                dialect,
            ) {
                out.push(Diagnostic::from_taint(&w));
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
                out.push(Diagnostic::from_taint(&w));
            }
        }
        for w in find_path_concat_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.rendered_props,
            &fu.taints,
            &fu.sccp.executable_blocks,
        ) {
            out.push(Diagnostic::from_path_concat(&w));
        }
    }

    // iRules-dialect non-taint checks.  Each of these is dialect-
    // gated inside the helper (returns empty for non-iRules
    // dialects), so calling them unconditionally is correct.
    for w in find_unnormalised_getter_warnings(cu, registry, dialect) {
        out.push(Diagnostic::from_irules_check(&w));
    }
    // C44-irules-flow: control-flow checks (IRULE5002/5004 +
    // IRULE1005-1008/1201/1202/4004).  Without these the helpers
    // landed but never reached users via `run_all_checks` /
    // LSP flows; addresses Codex review on PR #389.
    for w in find_unguarded_drop_warnings(cu, dialect) {
        out.push(Diagnostic::from_irules_check(&w));
    }
    for w in find_collect_flow_warnings(cu, dialect) {
        out.push(Diagnostic::from_irules_check(&w));
    }
    for w in find_http_flow_warnings(cu, dialect) {
        out.push(Diagnostic::from_irules_check(&w));
    }
    for w in find_hoistable_set_warnings(cu, dialect) {
        out.push(Diagnostic::from_irules_check(&w));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        let mut r = CommandRegistry::build_default();
        r.load_irules();
        r
    }

    #[test]
    fn run_all_checks_on_empty_source_is_empty() {
        let cu = CompilationUnit::build_for("", &registry(), false);
        assert!(run_all_checks(&cu, &registry(), None).is_empty());
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
    fn diagnostic_from_redundant_preserves_code() {
        let r = crate::gvn::RedundantComputation::new(
            Span::new(0, 5),
            Span::new(10, 15),
            "llength $x",
            "O105",
            "msg",
        );
        let d = Diagnostic::from_redundant(&r);
        assert_eq!(d.code, "O105");
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
                .any(|d| d.code == "IRULE3001" && d.category == "taint"),
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
                .any(|d| d.code == "IRULE3101" && d.category == "taint"),
            "expected IRULE3101, got {diagnostics:?}",
        );
    }

    #[test]
    fn run_all_checks_reports_irule3102_end_to_end() {
        let cu = CompilationUnit::build_for("set u [HTTP::uri]", &registry(), false)
            .with_interprocedural(&registry(), Some("f5-irules"));
        let diagnostics = run_all_checks(&cu, &registry(), Some("f5-irules"));
        let hit = diagnostics
            .iter()
            .find(|d| d.code == "IRULE3102")
            .unwrap_or_else(|| panic!("expected IRULE3102, got {diagnostics:?}"));
        assert_eq!(hit.category, "irules");
        assert_eq!(hit.severity, Severity::Warning);
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
            irules_diags.iter().any(|d| d.code == "IRULE3101"),
            "expected IRULE3101 under f5-irules dialect, got {irules_diags:?}",
        );

        let plain_cu = CompilationUnit::build_for(src, &registry(), false);
        let plain_diags = run_all_checks(&plain_cu, &registry(), None);
        assert!(
            plain_diags.iter().all(|d| d.code != "IRULE3101"),
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
            diagnostics.iter().all(|d| !d.code.starts_with("IRULE")),
            "no IRULE diagnostics without dialect, got {diagnostics:?}",
        );
    }

    #[test]
    fn run_all_checks_reports_w201_path_concat() {
        let cu = CompilationUnit::build_for("set x 42\nset p \"/tmp/$x\"", &registry(), false);
        let diagnostics = run_all_checks(&cu, &registry(), None);
        let w201 = diagnostics
            .iter()
            .find(|d| d.code == "W201" && d.category == "taint")
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
