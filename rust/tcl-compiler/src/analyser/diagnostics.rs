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
//! **Strip-by-strip status.**
//!
//! - **C41d1** — orchestrator scaffold + dedupe + disabled-
//!   codes filter.  ✅ landed.
//! - **C41d2** — `_diag_var_lifecycle.py`.  ✅ partial:
//!   W220 (dead store) and W214 (unused parameter) are wired
//!   today; W210 / W213 (read-before-set, unset on possibly-
//!   undef), W211 (unused variable), and H300 (paste error)
//!   are deferred to a follow-up strip — each needs extra
//!   plumbing (textual-reference filter, scope-alias
//!   detection, SSA-version-0 distinction between real proc
//!   params and synthetic RBS reads) that doesn't exist on
//!   the Rust side yet.
//! - **C41d3** — W230..W239 (`_diag_var_command.py`).
//! - **C41d4** — W120 / W121 / W122 / W242
//!   (`_diag_commands.py`).
//! - **C41d5** — W123 / W130..W138 (`_diag_branches.py` +
//!   `_diag_channel.py`).
//! - **C41d6** — IRULE3010..3019 (`_diag_ip.py`).
//! - **C41d7** — IRULE3020 (`_diag_racy.py`).

use std::collections::HashSet;

use tcl_lexer::SourceMap;

use super::state::Analyser;
use super::types::Severity;

/// Find a case-insensitive match for `variable` in `defined_vars`.
///
/// Mirrors `_find_case_mismatch` in
/// `core/analysis/_analyser/_diag_var_lifecycle.py:135-148`.
/// Returns the lexicographically smallest other-cased variant —
/// deterministic across runs.
fn find_case_mismatch<'a>(variable: &str, defined_vars: &'a HashSet<String>) -> Option<&'a str> {
    let lower = variable.to_lowercase();
    let mut matches: Vec<&str> = defined_vars
        .iter()
        .filter(|n| n.as_str() != variable && n.to_lowercase() == lower)
        .map(String::as_str)
        .collect();
    matches.sort_unstable();
    matches.into_iter().next()
}

/// Collect every variable name defined anywhere in `cfg`.
///
/// Mirrors `_collect_defined_vars` in
/// `_diag_var_lifecycle.py:123-133`.  Walks every block and pulls
/// the `defs` field off each [`crate::ir::Statement`] that has
/// one (assignments, ``incr``, ``Call`` statements with explicit
/// defs).  Used for the "did you mean…?" case-mismatch
/// suggestion in W210 / W211 / W220 messages.
fn collect_defined_vars(cfg: &crate::cfg::Function) -> HashSet<String> {
    use crate::ir::Statement;
    let mut names: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { name, .. }
                | Statement::AssignExpr { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::Incr { name, .. } => {
                    let normalised = crate::naming::normalise_var_name(name);
                    if !normalised.is_empty() {
                        names.insert(normalised.to_string());
                    }
                }
                Statement::Call { defs, .. } => {
                    for def in defs {
                        names.insert(def.clone());
                    }
                }
                _ => {}
            }
        }
    }
    names
}

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
    /// [`crate::compilation_unit::CompilationUnit`] for `source`,
    /// then walks the top-level + every procedure, dispatching
    /// per-function emitters.
    ///
    /// **C41d2 baseline.**  W220 (dead store hint) and W214
    /// (unused parameter hint) are wired through the per-function
    /// dispatcher.  W211 (unused variable), W210 / W213
    /// (read-before-set / unset on possibly-undef), and H300
    /// (paste error) are deferred to a follow-up — each needs
    /// extra plumbing (textual-reference filter, scope-alias
    /// detection, SSA-version-0 distinction between real proc
    /// params and synthetic RBS reads) that doesn't exist on
    /// the Rust side yet.  The cross-function post-passes
    /// (var-as-command for W307, interpolated-command resolution
    /// for W242) land in **C41d3** / **C41d4**.
    pub fn emit_cfg_ssa_diagnostics(&mut self, source: &str) {
        use tcl_registry::prelude::DialectSet;
        use tcl_registry::CommandRegistry;

        let mut registry = CommandRegistry::build_default();
        if let Some(d) = DialectSet::parse(&self.dialect) {
            registry.load_dialect(d);
        }
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);

        // Top-level first, then procedures in insertion order —
        // matches the iteration order of
        // ``CompilationUnit::functions``.
        // Iterate top-level explicitly so we can pass the IR
        // module through.
        self.emit_cfg_ssa_diagnostics_for_function(&cu.top_level, &cu.ir_module);
        for fu in cu.procedures.values() {
            self.emit_cfg_ssa_diagnostics_for_function(fu, &cu.ir_module);
        }
    }

    /// Per-function diagnostic dispatcher.
    ///
    /// Mirrors `_emit_cfg_ssa_diagnostics_for_function` in
    /// `_diagnostics.py:183-209`.  Called once for the top-level
    /// script and once per procedure.  Each per-emitter call is
    /// gated on its own predicate inside the helper.
    ///
    /// **C41d2 baseline.**  Only W220 + W214 are wired today.
    /// Each future C41d strip adds another emitter call here.
    pub fn emit_cfg_ssa_diagnostics_for_function(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        ir_module: &crate::ir::Module,
    ) {
        let defined = collect_defined_vars(&function_unit.cfg);
        self.emit_dead_store_diagnostics(function_unit, &defined);
        if let Some(ir_proc) = ir_module.procedures.get(&function_unit.name) {
            self.emit_unused_param_diagnostics(function_unit, ir_proc);
        }
        // C41d2 follow-up: emit_unused_variable_diagnostics (W211)
        // C41d2 follow-up: emit_read_before_set_diagnostics (W210/W213)
        // C41d2 follow-up: emit_possible_paste_error_diagnostics (H300)
        // C41d4: emit_invalid_ip_diagnostics (W122)
        // C41d5: emit_constant_branch_diagnostics (W123)
        // C41d5: emit_channel_diagnostics (W130..W138)
    }

    /// W220 — dead-store hint.
    ///
    /// Mirrors `_emit_dead_store_diagnostics` in
    /// `_diag_var_lifecycle.py:29-72`.  A *dead store* is an
    /// assignment whose value is overwritten before being read —
    /// some other SSA version of the same variable is live, so
    /// this version's value never reaches a user.
    ///
    /// Walks every dead [`Statement`](crate::ir::Statement) chain
    /// in `fu.def_use`, checks that another version of the same
    /// variable has live uses, and emits a Hint at the dead
    /// statement's span.  When the variable's name has a
    /// case-insensitive twin among `defined_vars`, the message
    /// includes a "did you mean…?" suggestion.
    ///
    /// **Known limitations** (deferred to a follow-up): no
    /// scope-alias filter (vars introduced via ``global`` /
    /// ``upvar`` are visible elsewhere and may be falsely
    /// flagged); no textual-reference filter (vars only ever
    /// referenced inside ``"$x"`` interpolations or inside a
    /// ``Return`` value miss the def-use scan).
    fn emit_dead_store_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
    ) {
        use crate::def_use::DefKind;
        use std::fmt::Write as _;
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (var, version) = &chain.key;
            // ``any_other_live`` — another SSA version of this
            // variable has live uses, so this assignment is
            // overwritten.  When no other version is live, the
            // variable is truly unused — that's W211 (deferred).
            let any_other_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *var && k.1 != *version && !c.is_dead());
            if !any_other_live {
                continue;
            }
            let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
                continue;
            };
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            let span = stmt.span();
            if span.is_empty() {
                continue;
            }
            let mut message = format!("Assignment to '{var}' is never read");
            if let Some(similar) = find_case_mismatch(var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W220".to_string(),
                span,
                message,
                severity: Severity::Hint,
            });
        }
    }

    /// W214 — unused-parameter hint.
    ///
    /// Mirrors `_emit_unused_param_diagnostics` in
    /// `_diag_var_lifecycle.py:260-274`.  For every parameter
    /// declared in `ir_proc.params`, check whether any def-use
    /// chain for the parameter (any SSA version) has live uses.
    /// When all chains are dead, the parameter is unused —
    /// emit a Hint at the proc's span.
    ///
    /// Diverges slightly from Python's ``analysis.unused_params``:
    /// Python pre-computes the unused-params list during
    /// ``analyse_ir_module``; the Rust port inlines the same
    /// def-use scan here because the Rust ``FunctionAnalysis``
    /// builder hasn't been ported yet.  The check is equivalent —
    /// a parameter is unused iff no SSA version of its name has
    /// live uses.
    fn emit_unused_param_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: &crate::ir::Procedure,
    ) {
        for param in &ir_proc.params {
            // Tcl's variadic ``args`` parameter is conventionally
            // declared even when unused (as a "consume the rest"
            // marker).  Skip it from W214.
            if param == "args" {
                continue;
            }
            let any_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *param && !c.is_dead());
            if any_live {
                continue;
            }
            let message = format!(
                "Parameter '{param}' of proc '{name}' is unused",
                name = ir_proc.qualified_name,
            );
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W214".to_string(),
                span: ir_proc.span,
                message,
                severity: Severity::Hint,
            });
        }
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
    fn emit_cfg_ssa_diagnostics_runs_without_panicking_on_empty_source() {
        // Smoke test — the orchestrator handles empty input
        // gracefully (an empty CompilationUnit yields no
        // diagnostics).
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("");
        assert!(a.result.diagnostics.is_empty());
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_no_w220_on_simple_assignment() {
        // ``set x 1`` — single assignment, no overwrite, no
        // W220.  Smoke test that pipeline runs without
        // emitting spurious W codes for clean code.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x 1");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W220"),
            "W220 must not fire on a single assignment; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w220_dead_store_overwritten() {
        // ``set x 1\nset x 2\nputs $x`` — the first ``set x 1``
        // is overwritten before being read.  W220 should fire
        // at the first assignment.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x 1\nset x 2\nputs $x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            !w220s.is_empty(),
            "W220 expected for overwritten ``set x 1``; got {:?}",
            a.result.diagnostics,
        );
        assert!(w220s.iter().any(|d| d.message.contains("'x'")));
        assert_eq!(w220s[0].severity, Severity::Hint);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w214_unused_param() {
        // ``proc foo {x y} { puts $x }`` — parameter ``y`` is
        // declared but never read in the body.  W214 should
        // fire on it.  Parameter ``x`` is read, so no W214.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {x y} { puts $x }");
        let w214s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W214")
            .collect();
        assert_eq!(
            w214s.len(),
            1,
            "expected exactly one W214 for unused param ``y``; got {:?}",
            a.result.diagnostics,
        );
        assert!(w214s[0].message.contains("'y'"));
        assert!(w214s[0].message.contains("'::foo'"));
        assert_eq!(w214s[0].severity, Severity::Hint);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w214_skips_args_param() {
        // The variadic ``args`` is conventional and frequently
        // declared without use; W214 must not fire on it.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {x args} { puts $x }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W214"),
            "W214 should not fire on ``args``; got {:?}",
            a.result.diagnostics,
        );
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
