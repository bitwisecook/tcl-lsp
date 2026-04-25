//! Diagnostic-emission orchestrator — Rust port of
//! `core/analysis/_analyser/_diagnostics.py`.
//!
//! Three top-level methods, mirroring the Python file 1:1:
//!
//! - [`Analyser::emit_variable_usage_diagnostics`] — kept as a
//!   no-op hook for future scope-tree consumers (Python's W211
//!   moved to the SSA-based pass; same here).
//! - [`Analyser::emit_cfg_ssa_diagnostics`] — main entry; builds
//!   a [`CompilationUnit`] on demand, walks the top-level
//!   function and every procedure, dispatches per-function
//!   diagnostics, and runs the cross-function post-passes
//!   (var-as-command, interpolated-command resolution).
//! - [`Analyser::emit_cfg_ssa_diagnostics_for_function`] —
//!   per-function dispatcher; calls each landed emitter in
//!   declaration order.
//!
//! Two utility passes round out the Python file:
//!
//! - [`Analyser::dedupe_diagnostics`] — drop exact duplicates
//!   plus the line-based pairs (E002 swallowed by E101 on the
//!   same line; W122 swallowed by W124 on the same line).
//! - [`Analyser::apply_disabled_diagnostics`] — filter out
//!   codes the caller asked to silence.
//!
//! **C41d1 baseline.**  This strip lands the orchestrator
//! scaffold + the dedupe / disabled-codes filters.  The
//! per-emitter calls are stubbed as TODO comments — each one
//! gets filled in by a subsequent C41d strip:
//!
//! - **C41d2** — W210 / W211 / W212 / W214 / W220 / W221
//!   (`_diag_var_lifecycle.py`).
//! - **C41d3** — W230..W239 (`_diag_var_command.py`).
//! - **C41d4** — W120 / W121 / W122 / W242
//!   (`_diag_commands.py`).
//! - **C41d5** — W123 / W130..W138 (`_diag_branches.py` +
//!   `_diag_channel.py`).
//! - **C41d6** — IRULE3010..3019 (`_diag_ip.py`).
//! - **C41d7** — IRULE3020 (`_diag_racy.py`).
//!
//! Wiring the orchestrator into [`Analyser::analyse`] also
//! lands here so dedupe + disabled-codes filtering applies to
//! W113 (the only currently-emitting code).  The emitter
//! dispatch path stays inert until the first C41d2 emitter
//! lands — see [`Analyser::emit_cfg_ssa_diagnostics`] for the
//! ``defer to C41d2`` short-circuit.

use std::collections::HashSet;

use tcl_lexer::SourceMap;

use super::state::Analyser;
use super::types::Severity;

impl Analyser {
    /// Scope-tree-driven variable diagnostic emitter.
    ///
    /// Mirrors `_emit_variable_usage_diagnostics` in
    /// `_diagnostics.py:111-116`.  Python keeps this method as
    /// an empty hook because W211 (unused-variable) moved to the
    /// SSA-based pass in `_emit_cfg_ssa_diagnostics_for_function`.
    /// The Rust port preserves the hook so future scope-tree-
    /// driven emitters (none currently planned) have a target.
    pub fn emit_variable_usage_diagnostics(&mut self) {
        // Intentionally empty — see module docstring.
    }

    /// CFG/SSA-backed diagnostic orchestrator.
    ///
    /// Mirrors `_emit_cfg_ssa_diagnostics` in
    /// `_diagnostics.py:118-181`.  Builds a
    /// [`crate::compilation_unit::CompilationUnit`] for `source`
    /// when one isn't provided, then walks the top-level + every
    /// procedure, dispatching per-function emitters and the
    /// cross-function post-passes.
    ///
    /// **C41d1 baseline.**  None of the per-function emitters
    /// have landed yet, so the dispatch body is currently empty.
    /// Each future C41d strip extends
    /// [`Self::emit_cfg_ssa_diagnostics_for_function`] in place.
    /// The [`crate::compilation_unit::CompilationUnit`] build is
    /// also deferred — there's nothing to feed yet — so this
    /// method short-circuits without touching the registry.
    /// When the first emitter (C41d2) lands, the CU build wires
    /// up here.
    pub fn emit_cfg_ssa_diagnostics(&mut self, source: &str) {
        let _ = source;
        // C41d2 wiring point: build CompilationUnit and dispatch.
        // Until at least one emitter lands the CU build is wasted
        // work, so leave it inert.
    }

    /// Per-function diagnostic dispatcher.
    ///
    /// Mirrors `_emit_cfg_ssa_diagnostics_for_function` in
    /// `_diagnostics.py:183-209`.  Called once for the top-level
    /// script and once per procedure.  Each per-emitter call is
    /// gated on its own dialect / context predicate inside the
    /// helper, so this dispatcher just runs them in declaration
    /// order — same shape as Python.
    ///
    /// **C41d1 baseline.**  Per-emitter calls are stubbed; each
    /// future strip wires its own line into this body.
    pub fn emit_cfg_ssa_diagnostics_for_function(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
    ) {
        let _ = function_unit;
        // C41d2: emit_constant_branch_diagnostics
        // C41d2: emit_dead_store_diagnostics
        // C41d4: emit_possible_paste_error_diagnostics
        // C41d2: emit_read_before_set_diagnostics
        // C41d2: emit_unused_variable_diagnostics
        // C41d4: emit_invalid_ip_diagnostics
        // C41d5: emit_channel_diagnostics (gated on ssa)
    }

    /// Drop exact-duplicate diagnostics + line-based suppression
    /// pairs.
    ///
    /// Mirrors `_dedupe_diagnostics` in
    /// `_diagnostics.py` (lives in `_core.py:595-630` — the
    /// orchestrator file imports it through the mixin
    /// hierarchy).  Two passes:
    ///
    /// 1. Compute the set of source lines on which `E101`
    ///    (missing-open-brace) and `W124` (SSA-based IP check)
    ///    fired.  These are sentinels for the related
    ///    redundant-message codes.
    /// 2. Walk diagnostics in source order, deduplicating by
    ///    `(code, span, message, severity)` and dropping:
    ///    - `E002` on a line where `E101` fired (the recovered
    ///      switch makes the arity message a false positive).
    ///    - `W122` on a line where `W124` fired (the SSA check
    ///      is more precise).
    ///
    /// Lines come from the [`SourceMap`] over `self.source`.
    pub fn dedupe_diagnostics(&mut self) {
        let sm = SourceMap::new(&self.source);
        let mut e101_lines: HashSet<u32> = HashSet::new();
        let mut w124_lines: HashSet<u32> = HashSet::new();
        for d in &self.result.diagnostics {
            let line = sm.range_positions(d.span).0.line;
            match d.code.as_str() {
                "E101" => {
                    e101_lines.insert(line);
                }
                "W124" => {
                    w124_lines.insert(line);
                }
                _ => {}
            }
        }

        let mut seen: HashSet<(String, u32, u32, String, Severity)> = HashSet::new();
        let drained = std::mem::take(&mut self.result.diagnostics);
        let mut deduped = Vec::with_capacity(drained.len());
        for d in drained {
            let key = (
                d.code.clone(),
                d.span.start(),
                d.span.end(),
                d.message.clone(),
                d.severity,
            );
            if seen.contains(&key) {
                continue;
            }
            let line = sm.range_positions(d.span).0.line;
            if d.code == "E002" && e101_lines.contains(&line) {
                continue;
            }
            if d.code == "W122" && w124_lines.contains(&line) {
                continue;
            }
            seen.insert(key);
            deduped.push(d);
        }
        self.result.diagnostics = deduped;
    }

    /// Filter out diagnostics whose codes are in
    /// [`Self::disabled_diagnostics`].
    ///
    /// Mirrors the per-emitter `if "Wxxx" in
    /// self._disabled_diagnostics:` early-returns in Python's
    /// emitter files.  Centralising the filter on the orchestrator
    /// side keeps the per-emitter code (in C41d2 / C41d3 / etc.)
    /// from having to thread the check at every emit site —
    /// emitters can push freely and the orchestrator drops the
    /// silenced codes at the end.
    ///
    /// Idempotent on an empty filter set (no allocations).
    pub fn apply_disabled_diagnostics(&mut self) {
        if self.disabled_diagnostics.is_empty() {
            return;
        }
        // Borrow-checker dance: `retain` closure can't capture
        // `&self.disabled_diagnostics` while ``self.result`` is
        // mut-borrowed; clone the set into a local first.  The
        // disabled set is small (LSP-config-scale) so the clone
        // cost is negligible vs. the rest of the diagnostics
        // pipeline.
        let disabled = self.disabled_diagnostics.clone();
        self.result
            .diagnostics
            .retain(|d| !disabled.contains(&d.code));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::types::Diagnostic;
    use tcl_lexer::Span;

    fn diag(code: &str, span: Span, msg: &str) -> Diagnostic {
        Diagnostic {
            code: code.to_string(),
            span,
            message: msg.to_string(),
            severity: Severity::Warning,
        }
    }

    #[test]
    fn emit_variable_usage_diagnostics_is_a_noop() {
        // Hook is intentionally empty — running it must leave
        // the diagnostics list untouched.
        let mut a = Analyser::new();
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "x"));
        a.emit_variable_usage_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_is_inert_until_c41d2() {
        // Baseline orchestrator runs without panicking and
        // doesn't add or remove any diagnostics on its own.
        let mut a = Analyser::new();
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "x"));
        a.emit_cfg_ssa_diagnostics("set x 1");
        assert_eq!(a.result.diagnostics.len(), 1);
    }

    #[test]
    fn dedupe_drops_exact_duplicates() {
        // Same code + span + message + severity → kept once.
        let mut a = Analyser::new();
        a.source = "set x 1".to_string();
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x not set"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x not set"));
        a.dedupe_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }

    #[test]
    fn dedupe_keeps_distinct_diagnostics_at_different_spans() {
        let mut a = Analyser::new();
        a.source = "set x 1\nset y 2".to_string();
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(12, 13), "y"));
        a.dedupe_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 2);
    }

    #[test]
    fn dedupe_drops_e002_on_e101_line() {
        // E101 fires on a line; any E002 on the same line is
        // a false positive (arity check confused by the
        // recovered switch) and gets dropped.
        let mut a = Analyser::new();
        a.source = "switch $x { foo {puts foo}".to_string();
        let switch_span = Span::new(0, 6);
        a.result
            .diagnostics
            .push(diag("E101", switch_span, "missing open brace"));
        a.result
            .diagnostics
            .push(diag("E002", switch_span, "too few args"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "E101"));
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "E002"));
    }

    #[test]
    fn dedupe_drops_w122_on_w124_line() {
        // W124 (SSA-based IP check) on a line → W122 (regex IP
        // check) on the same line is redundant.
        let mut a = Analyser::new();
        a.source = "if {[IP::addr $ip]} {}".to_string();
        let ip_span = Span::new(15, 18);
        a.result
            .diagnostics
            .push(diag("W124", ip_span, "invalid IP"));
        a.result
            .diagnostics
            .push(diag("W122", ip_span, "regex IP check"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "W124"));
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "W122"));
    }

    #[test]
    fn dedupe_keeps_e002_on_unrelated_line() {
        // E101 on line 0, E002 on line 1 — different lines, so
        // the suppression rule doesn't fire.
        let mut a = Analyser::new();
        a.source = "switch $x {\nset y 1".to_string();
        a.result
            .diagnostics
            .push(diag("E101", Span::new(0, 6), "missing brace"));
        a.result
            .diagnostics
            .push(diag("E002", Span::new(12, 15), "too few args"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "E002"));
    }

    #[test]
    fn apply_disabled_diagnostics_removes_listed_codes() {
        let mut a = Analyser::with_disabled_diagnostics(
            ["W113"].iter().map(|s| (*s).to_string()).collect(),
        );
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "shadows"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(0, 3), "unset"));
        a.apply_disabled_diagnostics();
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "W113"));
        assert!(a.result.diagnostics.iter().any(|d| d.code == "W210"));
    }

    #[test]
    fn apply_disabled_diagnostics_no_op_when_empty() {
        let mut a = Analyser::new();
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "x"));
        a.apply_disabled_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }
}
