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
/// fills it — proc bodies via an **isolated** analysis (memoisable), method
/// bodies in place (byte-identical, not yet memoised).
#[derive(Debug, Clone)]
pub(super) struct DeferredBody {
    /// Body text (braces stripped), as `analyse_body` expects.
    pub body_text: String,
    /// The body's `Str` token (absolute span) — drives offset arithmetic.
    pub body_tok: Token,
    /// Path to the already-created (empty) proc / method scope to fill.
    pub scope_path: Vec<usize>,
    /// `true` for an OO method/constructor/destructor body (filled in place);
    /// `false` for a `proc` body (analysed in isolation).
    pub is_method: bool,
    /// Lexical namespace of the proc (e.g. `"::ns"`), for reconstructing the
    /// isolated analysis context. (Proc bodies only.)
    pub namespace: String,
    /// The proc scope's name (as written) — used for `all_variables` keys.
    pub scope_name: String,
    /// The proc's declared parameters (locals in the body). (Proc bodies only.)
    pub params: Vec<crate::signature_scan::types::ParamDef>,
    /// Span the proc anchors its parameter definitions to (the proc-name token),
    /// so the isolated analysis records identical param `VarDef`s. (Proc only.)
    pub name_span: tcl_lexer::Span,
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

        // --- pass 2: fill each deferred body ---
        let deferred = std::mem::take(&mut self.deferred_bodies);
        // Phase-A fallback: a PROC body analysed in isolation can't see
        // enclosing-scope state, so a body that reads/writes a namespace/global
        // var or defines classes/instances would diverge.  Fall back to a full
        // rebuild for those files (method bodies are filled in place below, so
        // they don't need this guard).  Phase B will pre-seed enclosing defs.
        if deferred
            .iter()
            .any(|db| !db.is_method && body_needs_enclosing_context(&db.body_text))
        {
            return self.fresh_full_analyse(source, dialect);
        }
        for db in &deferred {
            if db.is_method {
                // Method bodies: fill in place (byte-identical; not memoised).
                self.last_comment = String::new();
                self.analyse_body(&db.body_text, db.body_tok, &db.scope_path);
            } else {
                // Proc bodies: analyse in isolation (a pure function of the
                // body — the unit slice 3 memoises) and graft into the shell.
                let frag = analyse_proc_body_isolated(
                    db,
                    &self.dialect,
                    &self.disabled_diagnostics,
                    self.non_ascii_mode,
                    self.stub_overlay.clone(),
                );
                self.graft_proc_body(db, frag);
            }
        }

        // --- tail (cross-item passes; canonicalises order) ---
        self.run_diagnostic_emitters(source);

        let result = std::mem::take(&mut self.result);
        self.clear_run_state();
        result
    }

    /// Graft an isolated proc body's facts (`frag`) into `self` (the shell):
    /// replace the empty proc scope's contents and union the flat maps / vecs /
    /// walk-state.  Order is canonicalised by the tail, so plain extend is fine.
    fn graft_proc_body(&mut self, db: &DeferredBody, frag: BodyFragment) {
        if let Some(ps) = super::scope::scope_at_mut(&mut self.result.global_scope, &db.scope_path)
        {
            ps.variables = frag.proc_scope.variables;
            ps.procs = frag.proc_scope.procs;
            ps.classes = frag.proc_scope.classes;
            ps.children = frag.proc_scope.children;
        }
        let r = frag.result;
        self.result.all_procs.extend(r.all_procs);
        self.result.all_classes.extend(r.all_classes);
        self.result.all_variables.extend(r.all_variables);
        self.result.command_aliases.extend(r.command_aliases);
        self.result.instance_classes.extend(r.instance_classes);
        self.result.diagnostics.extend(r.diagnostics);
        self.result
            .command_invocations
            .extend(r.command_invocations);
        self.result.package_requires.extend(r.package_requires);
        self.result.package_provides.extend(r.package_provides);
        self.result.source_targets.extend(r.source_targets);
        self.result.namespace_imports.extend(r.namespace_imports);
        self.result.auto_path_entries.extend(r.auto_path_entries);
        self.result.regex_patterns.extend(r.regex_patterns);
        self.result.has_dynamic_providers |= r.has_dynamic_providers;
        if self.result.unknown_proc_info.is_none() {
            self.result.unknown_proc_info = r.unknown_proc_info;
        }
        for (line, codes) in r.suppressed_lines {
            self.result
                .suppressed_lines
                .entry(line)
                .or_default()
                .extend(codes);
        }
        self.ensemble_namespaces.extend(frag.ensembles);
        self.pending_arity.extend(frag.pending_arity);
        self.var_command_sites.extend(frag.var_sites);
        self.cmd_command_sites.extend(frag.cmd_sites);
    }
}

/// The facts produced by analysing one proc body in isolation.
struct BodyFragment {
    result: AnalysisResult,
    proc_scope: super::types::Scope,
    ensembles: std::collections::HashSet<String>,
    pending_arity: Vec<(String, String, bool, super::types::Diagnostic)>,
    var_sites: Vec<super::state::VarCommandSite>,
    cmd_sites: Vec<super::state::CmdCommandSite>,
}

/// Analyse one `proc` body as an isolated unit — a pure function of
/// `(offset, body_text, namespace, params)` (slice 3 memoises this).  The body
/// is walked at its real absolute offset (the source is space-padded up to that
/// offset so the handlers' span re-slicing stays in range, and the proc's
/// enclosing namespace + params are reconstructed) so the facts need no
/// rebasing.  Offset-invariant memoisation (offset-0 + rebase) is a follow-up.
fn analyse_proc_body_isolated(
    db: &DeferredBody,
    dialect: &str,
    disabled: &std::collections::HashSet<String>,
    non_ascii: super::state::NonAsciiMode,
    stub_overlay: Option<tcl_registry::stub_overlay::StubOverlay>,
) -> BodyFragment {
    use tcl_registry::CommandRegistry;
    let mut a =
        Analyser::with_disabled_diagnostics(disabled.clone()).with_non_ascii_mode(non_ascii);
    a.dialect = dialect.to_string();
    a.stub_overlay = stub_overlay;
    // `analyse_body` segments `body_text` at `span.start() + content_offset`
    // (skipping the opening `{`), so place the content there in the padded
    // source for the handlers' absolute span re-slicing to line up.
    let abs = db.body_tok.span.start() as usize + db.body_tok.content_offset as usize;
    let mut src = " ".repeat(abs);
    src.push_str(&db.body_text);
    a.source = src;
    let mut registry = CommandRegistry::build_default();
    if let Some(d) = tcl_registry::prelude::DialectSet::parse(dialect) {
        registry.load_dialect(d);
    }
    a.registry = Some(registry);
    a.line_offsets = Some(super::state::compute_line_offsets(&a.source));
    let proc_path =
        reconstruct_proc_scope(&mut a.result.global_scope, &db.namespace, &db.scope_name);
    let dummy = Token::new(tcl_lexer::TokenType::Str, db.name_span);
    for p in &db.params {
        a.define_var(&p.name, dummy, &proc_path, false, Some(db.name_span));
    }
    a.analyse_body(&db.body_text, db.body_tok, &proc_path);
    let proc_scope = super::scope::scope_at_mut(&mut a.result.global_scope, &proc_path)
        .expect("reconstructed proc scope")
        .clone();
    BodyFragment {
        result: a.result,
        proc_scope,
        ensembles: a.ensemble_namespaces,
        pending_arity: a.pending_arity,
        var_sites: a.var_command_sites,
        cmd_sites: a.cmd_command_sites,
    }
}

/// Build `global -> namespace* -> proc` scopes mirroring the proc's lexical
/// context (so nested defs qualify identically), returning the proc scope path.
fn reconstruct_proc_scope(
    root: &mut super::types::Scope,
    namespace: &str,
    scope_name: &str,
) -> Vec<usize> {
    use super::types::{Scope, ScopeKind};
    let comps: Vec<&str> = namespace
        .trim_start_matches(':')
        .split("::")
        .filter(|s| !s.is_empty())
        .collect();
    let mut path: Vec<usize> = Vec::new();
    for comp in comps {
        let parent = super::scope::scope_at_mut(root, &path).expect("scope path");
        let i = parent.children.len();
        parent.children.push(Scope::new(ScopeKind::Namespace, comp));
        path.push(i);
    }
    let parent = super::scope::scope_at_mut(root, &path).expect("scope path");
    let pi = parent.children.len();
    parent
        .children
        .push(Scope::new(ScopeKind::Proc, scope_name));
    path.push(pi);
    path
}

/// Does an isolated proc body need enclosing-scope context to match `analyse`?
/// True if it references a `::`-qualified variable (read `$::x` / `${ns::x}` or
/// a writer command targeting `::x`), or runs a command that defines/links
/// enclosing-scope or object state.  Conservative — a false positive only costs
/// a full-rebuild fallback, never correctness.  Phase B pre-seeds the enclosing
/// defs to shrink this set.
fn body_needs_enclosing_context(body_text: &str) -> bool {
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
    const WRITERS: &[&str] = &["set", "incr", "lappend", "append", "dict", "array"];
    let words: Vec<&str> = body_text
        .split(|c: char| !(c.is_alphanumeric() || c == ':' || c == '_'))
        .filter(|w| !w.is_empty())
        .collect();
    for (i, &w) in words.iter().enumerate() {
        if RISKY.contains(&w) {
            return true;
        }
        if WRITERS.contains(&w) && words[i + 1..].iter().take(2).any(|t| t.contains("::")) {
            return true;
        }
    }
    // Qualified variable *read*: `$::x`, `${::x}`, `$ns::x`, `${ns::x}`.
    let b = body_text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'$' {
            let mut j = i + 1;
            if j < b.len() && b[j] == b'{' {
                j += 1;
            }
            let start = j;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b':' || b[j] == b'_') {
                j += 1;
            }
            if body_text[start..j].contains("::") {
                return true;
            }
            i = j.max(i + 1);
        } else {
            i += 1;
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
