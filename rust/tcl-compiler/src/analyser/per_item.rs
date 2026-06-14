//! Per-item incremental analysis (slice 2b/3 of
//! `docs/design/rust/incremental-analysis.md`).
//!
//! [`Analyser::analyse_per_item`] reproduces [`Analyser::analyse`]
//! **byte-for-byte** but decomposed so each top-level **proc / method body** is
//! analysed as a separate unit — the granularity at which slice 3 memoises the
//! expensive per-line walk (a body edit then re-analyses only that body).
//!
//! ## How it stays byte-identical
//!
//! The walk is split into two passes over the *same* analyser:
//! 1. **Shell pass** — walk the top level with `defer_proc_bodies = true`, so
//!    `handle_proc_command` / OO method walks create each body's scope (params
//!    defined, `body_span` set) but record the body in `deferred_bodies` instead
//!    of recursing. The whole top level (incl. `namespace eval` / control-flow
//!    bodies and the procs they contain) is walked here, so top-level variable
//!    flow is identical to `analyse`.
//! 2. **Body pass** — for each deferred body, `analyse_body` fills its
//!    already-created scope *in place*, with `defer_proc_bodies = false` so
//!    nested defs walk within their enclosing unit.
//!
//! Splitting top-level vs body work reorders the walk-populated collections
//! (`command_invocations`, `VarDef.references`, …) relative to `analyse`'s DFS,
//! but those are made order-independent by `canonicalize_result_order` in the
//! shared tail — so the merged result matches. The cross-item tail
//! (`run_diagnostic_emitters`: W123 / arity / whole-source CFG-SSA) runs once at
//! the end, exactly as in `analyse`.
//!
//! Correctness rests on the differential fuzzer + the `per_item == analyse`
//! corpus gate, with a **full-rebuild fallback** for anything not provably
//! equivalent (error recovery / partial commands / inline stub directives).

use tcl_lexer::Token;

use super::state::Analyser;
use super::types::AnalysisResult;

/// A proc / method body deferred by the shell pass for a second analysis pass.
/// Its scope already exists at `scope_path` (params defined); the body pass
/// calls `analyse_body` to fill it.
#[derive(Debug, Clone)]
pub(super) struct DeferredBody {
    /// Body text (braces stripped), as `analyse_body` expects.
    pub body_text: String,
    /// The body's `Str` token (absolute span) — drives offset arithmetic.
    pub body_tok: Token,
    /// Path to the already-created (empty) proc / method scope to fill.
    pub scope_path: Vec<usize>,
}

impl Analyser {
    /// Per-item analysis entry — byte-identical to [`Analyser::analyse`] for
    /// well-formed input, falling back to it otherwise. See module docs.
    pub fn analyse_per_item(&mut self, source: &str, dialect: &str) -> AnalysisResult {
        use std::collections::HashSet;
        use tcl_registry::CommandRegistry;

        // Recovery / stub overlays are only modelled on the full `analyse`
        // path; fall back so the per-item result can never diverge.
        if !tcl_lexer::script_is_complete(source) || source.contains("tcl-lsp: stub") {
            return self.analyse(source, dialect);
        }

        // --- setup, mirroring `analyse` (gated by the corpus `per_item ==
        // analyse` test) ---
        self.source = source.to_string();
        self.dialect = dialect.to_string();
        let file_codes = super::utils::parse_file_suppression(source);
        for code in &file_codes {
            self.disabled_diagnostics.insert(code.clone());
        }
        if !file_codes.is_empty() {
            self.result
                .suppressed_lines
                .insert(-1, file_codes.iter().cloned().collect());
        }
        super::state::merge_noqa_line_suppressions(
            &mut self.result.suppressed_lines,
            super::utils::parse_noqa_line_suppressions(source),
        );
        let (stub_cmds, stub_exprs) = super::utils::scan_source_for_stubs(source);
        self.stub_overlay = Some(super::types::build_stub_overlay(&stub_cmds));
        self.result.stub_commands = stub_cmds;
        self.result.stub_expr_defs = stub_exprs;

        let mut registry = CommandRegistry::build_default();
        if let Some(d) = tcl_registry::prelude::DialectSet::parse(&self.dialect) {
            registry.load_dialect(d);
        }
        self.registry = Some(registry);
        self.line_offsets = Some(super::state::compute_line_offsets(source));
        let known: HashSet<&str> = self
            .registry
            .as_ref()
            .expect("registry just stashed")
            .command_names()
            .collect();
        let mut commands = crate::segmenter::segment_commands_with_recovery_and_config(
            source,
            &known,
            self.lexer_config(),
        );
        drop(known);

        // Error recovery → fall back to full `analyse` (its ghost-recovery /
        // partial-command handling is not reproduced on the per-item path).
        let ghost = self.apply_ghost_recovery(source, &mut commands);
        if ghost || commands.iter().any(|c| c.is_partial) {
            return self.fresh_full_analyse(source, dialect);
        }

        // --- pass 1: shell (defer proc/method bodies) ---
        self.defer_proc_bodies = true;
        self.walk_commands_top_level(&commands, false);
        self.defer_proc_bodies = false;

        // --- pass 2: fill each deferred body in place ---
        let deferred = std::mem::take(&mut self.deferred_bodies);
        // Phase-A fallback: a deferred body that writes *enclosing*-scope state
        // (namespace vars via `variable`/`global`/`upvar`, classes via
        // `oo::define`/`oo::objdefine`, object instances) is order-sensitive
        // across the shell/body split — the 2-pass order can pick a different
        // first/last writer than `analyse`'s single DFS pass.  Until Phase B
        // models that, fall back to a full rebuild for such files.
        if deferred
            .iter()
            .any(|db| body_writes_enclosing_state(&db.body_text))
        {
            return self.fresh_full_analyse(source, dialect);
        }
        for db in deferred {
            // `analyse` takes `last_comment` before each body walk so the body
            // starts with no inherited doc-comment; mirror that here.
            self.last_comment = String::new();
            self.analyse_body(&db.body_text, db.body_tok, &db.scope_path);
        }

        // --- tail (cross-item passes; canonicalises order) ---
        self.run_diagnostic_emitters(source);

        let result = std::mem::take(&mut self.result);
        self.clear_run_state();
        result
    }
}

/// Heuristic: does a body (textually) contain a command that writes state
/// keyed *outside* its own scope subtree — making per-item analysis
/// order-sensitive?  Conservative (word-level scan; false positives only cost a
/// fallback, never correctness).  Phase A; Phase B will model these precisely.
fn body_writes_enclosing_state(body_text: &str) -> bool {
    // Standalone commands that link/define enclosing-scope or object state.
    const RISKY: &[&str] = &[
        "variable",
        "global",
        "upvar",
        "namespace",
        "oo::define",
        "oo::objdefine",
        "oo::class",
        "oo::copy",
        "itcl::class",
        "interp",
        "rename",
        "new",
        "create",
    ];
    // Var-writer commands; a `::`-qualified target writes a namespace/global
    // var from inside the body (e.g. `set ::ns::x`, `incr ::ns::n`).
    const WRITERS: &[&str] = &["set", "incr", "lappend", "append", "dict", "array"];
    let words: Vec<&str> = body_text
        .split(|c: char| !(c.is_alphanumeric() || c == ':' || c == '_'))
        .filter(|w| !w.is_empty())
        .collect();
    for (i, &w) in words.iter().enumerate() {
        if RISKY.contains(&w) {
            return true;
        }
        if WRITERS.contains(&w) {
            // a `::`-qualified target within the next two words (covers
            // `set ::x` and `dict set ::x` / `array set ::x`)
            if words[i + 1..].iter().take(2).any(|t| t.contains("::")) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eq(src: &str) {
        let want = Analyser::new().analyse(src, "tcl8.6");
        let got = Analyser::new().analyse_per_item(src, "tcl8.6");
        assert_eq!(got, want, "per_item != analyse for:\n{src}");
    }

    #[test]
    fn two_top_level_procs() {
        eq("proc a {} { set x 1 }\nproc b {} { puts hi }\n");
    }

    #[test]
    fn proc_calling_proc() {
        eq("proc a {x} { return $x }\nproc b {} { a 1 }\n");
    }

    #[test]
    fn namespace_and_top_level_code() {
        eq("namespace eval ns { proc foo {} {} }\nset g 1\nputs $g\n");
    }

    #[test]
    fn top_level_global_read_in_body() {
        eq("set g 1\nproc r {} { puts $::g }\nputs $g\n");
    }

    #[test]
    fn oo_class_with_method() {
        eq("oo::class create K {\n  variable n\n  method m {a} { set n $a; return $n }\n}\n");
    }

    #[test]
    fn nested_proc_in_body() {
        eq("proc outer {} { proc ::inner {} { set z 1 } ; inner }\n");
    }
}
