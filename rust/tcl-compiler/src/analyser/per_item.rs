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
//! 2. **Body pass** — for each deferred body (proc *and* method),
//!    `analyse_proc_body_isolated` analyses it as an offset-0 isolated unit and
//!    `graft_proc_body` rebases + merges the result into the shell.  Both are
//!    memoisable, so a body edit re-analyses only that body.
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

/// Why [`Analyser::analyse_per_item_with`] abandoned the incremental path and
/// paid for a full whole-file walk.
///
/// Every fallback costs a complete re-analysis of the document on **every
/// keystroke** — the per-body memoisation above it is dead weight for a file
/// that trips one of these. Knowing the rate is a perf question; knowing the
/// *distribution* is what says which gate is worth engineering away, so the
/// variants are ordered by where they fire in the pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PerItemFallback {
    /// Source is not a complete script (unbalanced braces / quotes) — mid-edit
    /// state, so this one is expected to fire transiently while typing.
    IncompleteScript,
    /// An inline `tcl-lsp: stub` directive; stub overlays are modelled only on
    /// the full path.
    StubDirective,
    /// Tk checks are live, and their whole-file state (created widgets,
    /// per-parent geometry) is not visible to an isolated body.
    TkActive,
    /// Ghost-token error recovery engaged.
    GhostRecovery,
    /// A partial (unterminated) command survived segmentation.
    PartialCommand,
    /// An `E…` diagnostic was emitted — `analyse` would have run recovery
    /// machinery the per-item walk only partially reproduces.
    ErrorDiagnostic,
    /// The same method qualified name is defined more than once.
    DuplicateMethod,
    /// A body declares a link to a qualified sub-namespace variable, or
    /// `namespace import`/`export`s from inside a body
    /// ([`body_needs_enclosing_context`]).
    EnclosingContext,
    /// A body defines a proc whose name is already defined elsewhere.
    DuplicateProcInBody,
    /// A body defines/extends a class whose facts collide with existing ones.
    ClassFactsCollide,
    /// A method body's object-instance tracking could not be replayed.
    MethodInstanceReplay,
}

impl PerItemFallback {
    /// Stable short name for histograms / logs.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IncompleteScript => "incomplete-script",
            Self::StubDirective => "stub-directive",
            Self::TkActive => "tk-active",
            Self::GhostRecovery => "ghost-recovery",
            Self::PartialCommand => "partial-command",
            Self::ErrorDiagnostic => "error-diagnostic",
            Self::DuplicateMethod => "duplicate-method",
            Self::EnclosingContext => "enclosing-context",
            Self::DuplicateProcInBody => "duplicate-proc-in-body",
            Self::ClassFactsCollide => "class-facts-collide",
            Self::MethodInstanceReplay => "method-instance-replay",
        }
    }
}

/// A proc / method body deferred by the shell pass for a second analysis pass.
/// Its scope already exists at `scope_path` (params + instance vars defined);
/// the body pass fills it via an **isolated** analysis (memoisable) and grafts
/// the result back.
#[derive(Debug, Clone)]
pub struct DeferredBody {
    /// Body text (braces stripped), as `analyse_body` expects.
    ///
    /// Shared behind an `Arc` so the LSP query database can intern it into its
    /// per-body memo key without a second copy of every proc body's full source
    /// on every edit (issue #1159 — `tcl-lsp-db`'s `ItemBodyKey`).
    pub body_text: std::sync::Arc<str>,
    /// The body's `Str` token (absolute span) — drives offset arithmetic.
    pub body_tok: Token,
    /// Path to the already-created (empty) proc / method scope to fill.
    pub scope_path: Vec<usize>,
    /// `true` for an OO method/constructor/destructor body (analysed in an
    /// isolated `Method` scope with instance variables pre-bound); `false` for a
    /// `proc` body.
    pub is_method: bool,
    /// Mirrors [`super::types::Scope::oo_global_resolution`] for the isolated
    /// rebuild: `true` for a `TclOO` method body (bare commands resolve
    /// globally — object-namespace semantics), `false` for procs and
    /// snit / itcl members.
    pub oo_global_resolution: bool,
    /// Lexical namespace of the proc (e.g. `"::ns"`), or — for a method — the
    /// class's **defining** namespace, used to reconstruct the isolated analysis
    /// context.
    pub namespace: String,
    /// The proc scope's name (as written) — or, for a method, its qualified name
    /// (`::Class::method`) — used for `all_variables` keys and the per-item
    /// duplicate detector.
    pub scope_name: String,
    /// The proc / method's declared parameters (locals in the body).
    pub params: Vec<crate::signature_scan::types::ParamDef>,
    /// Class instance variables pre-bound in every method body (`variable`
    /// declarations at class level).  Empty for proc bodies.
    ///
    /// The seeded var's real `definition_span` (the `variable v` declaration)
    /// is supplied by the *shell* walk (`oo::walk_method_body`), which the
    /// graft keeps for shell-owned keys (`merge_one_var`); the isolated body
    /// pass below only needs the names, so this stays `Vec<String>` (and salsa
    /// -interning-friendly — see `tcl-lsp-db`'s `ItemBodyKey`).
    pub class_variables: Vec<String>,
    /// A flattened snapshot of `self.safe_interp_stack`'s *top* entry (issue
    /// #1001 follow-up) at the moment this body was deferred — `(base_hidden,
    /// hidden_extra, exposed)`, sorted `Vec<String>`s rather than the live
    /// `HashSet`-based `SafeInterpCtx` (which isn't `Hash`, so can't key
    /// `tcl-lsp-db`'s `ItemBodyKey` directly) so this stays deterministic and
    /// salsa-interning-friendly, matching `class_variables` above.
    /// `None` outside any tracked safe interpreter (the overwhelming common
    /// case — no new work, no behaviour change from before this field
    /// existed). Every safe-interp check only ever consults the *top* of the
    /// stack (`safe_interp_visibility_gate`'s `.last()`), never an older
    /// entry, so this single flattened snapshot is sufficient — a proc/apply
    /// body nested several `interp eval`s deep still only needs the
    /// innermost one. [`analyse_proc_body_isolated`] seeds the isolated
    /// `Analyser`'s own `safe_interp_stack` from this before walking the
    /// body, so a hidden call inside the body hits the same, unmodified gate
    /// a directly-written call already does — without this, W129 silently
    /// missed *any* hidden call inside *any* proc/apply body nested in a
    /// safe interpreter under incremental (`analyse_per_item`) analysis,
    /// which is what the live LSP server always uses for diagnostics.
    pub safe_interp_ctx: Option<(bool, Vec<String>, Vec<String>)>,
}

impl Analyser {
    /// Per-item analysis entry — byte-identical to [`Analyser::analyse`] for
    /// well-formed input, falling back to it otherwise. Analyses each proc body
    /// directly. See module docs.
    pub fn analyse_per_item(&mut self, source: &str, dialect: &str) -> AnalysisResult {
        let disabled = self.disabled_diagnostics.clone();
        let non_ascii = self.non_ascii_mode;
        let empty_overlay = super::types::build_stub_overlay(&[]);
        let mut body_fn = |db: &DeferredBody| {
            analyse_proc_body_isolated(
                db,
                dialect,
                &disabled,
                non_ascii,
                Some(empty_overlay.clone()),
            )
        };
        self.analyse_per_item_with(source, dialect, &mut body_fn)
    }

    /// As [`Self::analyse_per_item`], but each proc / method body's isolated
    /// analysis is produced by `body_fn` — letting a caller (the LSP query DB)
    /// memoise it in salsa so a body edit recomputes only that body.
    /// Byte-identical to `analyse` whenever `body_fn` returns what
    /// [`analyse_proc_body_isolated`] would.
    pub fn analyse_per_item_with(
        &mut self,
        source: &str,
        dialect: &str,
        body_fn: &mut dyn FnMut(&DeferredBody) -> BodyFragment,
    ) -> AnalysisResult {
        self.took_fast_path = false;
        self.per_item_fallback = None;
        // Recovery / stub overlays are only modelled on the full `analyse`
        // path; fall back so the per-item result can never diverge.  Tk's
        // TK100x checks accumulate whole-file state (created widgets, per-parent
        // geometry) that an isolated proc body cannot see, so a Tk document
        // also falls back to full analysis rather than diverge (a parent
        // created outside a proc would otherwise look missing inside it, and a
        // `pack`/`grid` conflict spanning a proc body would never flush).
        //
        // Evaluated one gate at a time (rather than as one `||` chain) so the
        // telemetry can name which fired — these are checked in cheapest-first
        // order, so the split costs nothing.
        let entry_gate = if tcl_lexer::script_is_complete(source) {
            if source.contains("tcl-lsp: stub") {
                Some(PerItemFallback::StubDirective)
            } else if super::tk_checks::tk_possibly_active(source, dialect) {
                Some(PerItemFallback::TkActive)
            } else {
                None
            }
        } else {
            Some(PerItemFallback::IncompleteScript)
        };
        if let Some(reason) = entry_gate {
            self.per_item_fallback = Some(reason);
            return self.analyse(source, dialect);
        }

        // --- setup, mirroring `analyse` (gated by the corpus `per_item ==
        // analyse` test) ---
        let mut commands = self.per_item_setup(source, dialect);

        // Error recovery → fall back to full `analyse` (its ghost-recovery /
        // partial-command handling is not reproduced on the per-item path).
        let ghost = self.apply_ghost_recovery(source, &mut commands);
        if ghost || commands.iter().any(|c| c.is_partial) {
            self.per_item_fallback = Some(if ghost {
                PerItemFallback::GhostRecovery
            } else {
                PerItemFallback::PartialCommand
            });
            return self.fresh_full_analyse(source, dialect);
        }

        // --- pass 1: shell (defer proc/method bodies) ---
        // Whole-file scoped environment (tclpkg manifests) — same seeding as
        // `analyse`, so the per-item path resolves file-scoped directives
        // identically (gated by the corpus `per_item == analyse` test).
        let file_env_pushed = self.seed_file_scope_env(source);
        self.defer_proc_bodies = true;
        self.walk_commands_top_level(&commands, false);
        self.defer_proc_bodies = false;
        if file_env_pushed {
            self.body_scope_stack.pop();
        }

        // --- pass 2: fill each deferred body ---
        // Returns `Err(fallback_result)` for the patterns the isolated-body
        // decomposition can't reproduce byte-for-byte.
        if let Err(fallback) = self.fill_deferred_bodies(source, dialect, body_fn) {
            return *fallback;
        }

        // --- tail (cross-item passes; canonicalises order) ---
        self.run_diagnostic_emitters(source);

        // Correctness backstop: a syntax error
        // (`E…` diagnostic) means `analyse` engaged its error-*recovery* machinery
        // (ghost-token re-lexing, recovery segmentation, body-level partial
        // detection) that the per-item walk only partially reproduces — a
        // locally-unbalanced `}` that `script_is_complete` still accepts is the
        // classic case.  Re-analyse fully so the result is byte-identical to a
        // from-scratch `analyse`.  Well-formed edits (the common case) carry no
        // E-codes and stay on the fast path.
        if self.result.diagnostics.iter().any(|d| d.code.is_error()) {
            self.per_item_fallback = Some(PerItemFallback::ErrorDiagnostic);
            return self.fresh_full_analyse(source, dialect);
        }

        self.took_fast_path = true;
        let result = std::mem::take(&mut self.result);
        self.clear_run_state();
        result
    }

    /// Per-item setup, mirroring `analyse`: record source/dialect, apply file +
    /// `noqa` suppressions, build the stub overlay, stash a dialect-aware
    /// registry + line offsets, and segment the source with recovery.  Returns
    /// the segmented top-level commands.  (Gated by the corpus `per_item ==
    /// analyse` test.)
    fn per_item_setup(
        &mut self,
        source: &str,
        dialect: &str,
    ) -> Vec<crate::segmenter::SegmentedCommand> {
        use std::collections::HashSet;

        self.source = source.to_string();
        self.profile = tcl_dialect::DialectProfile::by_name(dialect);
        self.result.dialect = dialect.to_string();
        self.result.library_versions = self.library_versions.clone();
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

        self.registry = Some(tcl_registry::cache::registry_for_profile(self.profile));
        self.line_offsets = Some(super::state::compute_line_offsets(source));
        // Same recovery known-command universe as `Analyser::analyse` — see
        // `recovery_known_commands` — so per-item analysis matches the
        // full-file walk byte-for-byte (the corpus `per_item == analyse`
        // test gates this).
        self.recovery_known_commands = super::utils::recovery_known_commands(
            source,
            self.registry.expect("registry just stashed"),
            &self.extra_commands,
        );
        let known: HashSet<&str> = self.recovery_known_commands.iter().collect();
        crate::segmenter::segment_commands_with_recovery_and_config(
            source,
            &known,
            self.lexer_config(),
        )
    }

    /// Pass 2 of per-item analysis: analyse each deferred proc/method body in
    /// isolation (via `body_fn`) and graft it into the shell.
    ///
    /// Returns `Err(result)` carrying a full `fresh_full_analyse` rebuild for the
    /// patterns the isolated-body decomposition can't reproduce byte-for-byte:
    ///
    /// 1. **Duplicate definitions** — the same proc/method qualified name
    ///    defined more than once.  A whole-file walk processes the
    ///    redefinitions in source order (last-definition-wins for the shared
    ///    scope/`all_variables` key); the per-item graft *merges* every
    ///    body's facts instead.  Covers both top-level duplicates (e.g.
    ///    platform-conditional `proc auto_execok` in `if {…} else {…}`) and a
    ///    body that **redefines an enclosing/sibling proc** — the lazy-init
    ///    pattern `proc p {a} { …; proc p {a} {real} }`, where the inner def
    ///    wins in a whole-file walk but merges here.
    /// 2. **Class definition / extension in a proc body** (`oo::class` /
    ///    `oo::define` / `itcl::class` / object aliasing) — a class's methods
    ///    accumulate across bodies in a whole-file walk (remove → add →
    ///    re-insert), which the graft's overwrite-on-key cannot reproduce.
    ///
    /// Both are rare; everything else (namespace/global/`variable` state,
    /// qualified `$::g` reads — replayed at graft) takes the fast incremental
    /// path.  Guarded by the corpus `per_item == analyse` gate + the
    /// `incremental == fresh` fuzzer.
    fn fill_deferred_bodies(
        &mut self,
        source: &str,
        dialect: &str,
        body_fn: &mut dyn FnMut(&DeferredBody) -> BodyFragment,
    ) -> Result<(), Box<AnalysisResult>> {
        let deferred = std::mem::take(&mut self.deferred_bodies);
        // A **method** defined more than once (same qualified name) still forces a
        // fallback: method bodies fill their scope in place, so a genuine
        // duplicate would accumulate rather than last-definition-win.  (Distinct
        // methods now carry distinct `scope_name`s, so a multi-method class is no
        // longer a false duplicate.)
        let duplicate_method = !self.probe_skip_duplicate_fallback && {
            let mut seen = std::collections::HashSet::new();
            deferred
                .iter()
                .any(|db| db.is_method && !seen.insert(db.scope_name.as_str()))
        };
        let enclosing_fallback = !self.probe_skip_enclosing_fallback
            && deferred
                .iter()
                .any(|db| !db.is_method && body_needs_enclosing_context(&db.body_text));
        if duplicate_method || enclosing_fallback {
            self.per_item_fallback = Some(if duplicate_method {
                PerItemFallback::DuplicateMethod
            } else {
                PerItemFallback::EnclosingContext
            });
            return Err(Box::new(self.fresh_full_analyse(source, dialect)));
        }
        // **Proc duplicates take the fast path.**  A whole-file walk's
        // `all_variables` for a duplicated proc is the *union* of every
        // definition's locals, with last-definition-wins for keys shared across
        // definitions (`define_var` overwrites on each redefinition).  The graft
        // reproduces that with overwrite-on-collision for body-owned keys, while
        // *param* keys keep the shell's real definition (it recorded them in pass
        // 1, last-def-wins): the `shell_var_keys` snapshot distinguishes the two.
        // (For a non-duplicate proc every local key is unique, so this is a plain
        // insert — byte-identical to before.)  A body contributing *colliding*
        // class facts (accumulation) still falls back — see `class_facts_collide`.
        let shell_var_keys: std::collections::HashSet<String> =
            self.result.all_variables.keys().cloned().collect();
        // Track every defined proc qualified name (top-level + nested) so a body
        // that redefines an already-defined proc forces a fallback.
        let mut defined_procs: std::collections::HashSet<String> =
            self.result.all_procs.keys().cloned().collect();
        for db in &deferred {
            // Both proc and method bodies are analysed in isolation (a pure
            // function of the body — the unit `body_fn` memoises) and grafted into
            // the shell.  Methods reconstruct a `Method` scope with the class's
            // instance variables pre-bound (see `analyse_proc_body_isolated`).
            let frag = body_fn(db);
            // A body that defines a proc whose name is defined elsewhere is a
            // duplicate at the shared `all_procs` key — the graft can't reproduce
            // last-definition-wins there, so fall back.
            if frag
                .result
                .all_procs
                .keys()
                .any(|name| !defined_procs.insert(name.clone()))
            {
                self.per_item_fallback = Some(PerItemFallback::DuplicateProcInBody);
                return Err(Box::new(self.fresh_full_analyse(source, dialect)));
            }
            // A body that defines/extends a class whose `all_classes` entry
            // **collides** with one already present (a *different* value) can't be
            // reproduced by the graft's overwrite-on-key: a whole-file walk
            // *accumulates* there (`oo::define` adds methods to the existing
            // class).  A fresh (non-colliding) class definition stays fast.
            if class_facts_collide(&self.result, &frag.result) {
                self.per_item_fallback = Some(PerItemFallback::ClassFactsCollide);
                return Err(Box::new(self.fresh_full_analyse(source, dialect)));
            }
            // Object-instance tracking (W308) resolves the class at the walk
            // point.  A proc body's creations are captured and replayed by the
            // graft against the shell's full `all_classes` (source order →
            // last-assignment-wins matches the whole-file walk).  A *method* body
            // would resolve against the whole file's classes rather than analyse's
            // DFS prefix, so its instance tracking can't be replayed faithfully —
            // snapshot when it captured a candidate (a real `Cls new` *or* a
            // benign `dict create`) and fall back only if the replay actually
            // recorded an instance.
            let inst_snapshot = (db.is_method && !frag.instances.is_empty())
                .then(|| self.result.instance_classes.clone());
            self.graft_proc_body(db, frag, &shell_var_keys);
            if let Some(before) = inst_snapshot
                && self.result.instance_classes != before
            {
                self.per_item_fallback = Some(PerItemFallback::MethodInstanceReplay);
                return Err(Box::new(self.fresh_full_analyse(source, dialect)));
            }
        }
        Ok(())
    }

    /// Graft an isolated (offset-0) proc body's facts into `self` (the shell):
    /// **rebase** every span by the body's real position, then merge.  The shell
    /// already holds the params (with their real name-span), so variables are
    /// merge-or-inserted (an existing key is a param → keep its def span, add the
    /// body's reads; a new key is a body local → insert).  Order is canonicalised
    /// by the tail, so plain `extend` is fine for the rest.
    ///
    /// `shell_var_keys` holds the `all_variables` keys the shell already owns
    /// before any body is grafted (params + top-level vars).  A body re-records
    /// its params, so those keys *merge* (keep the shell's real definition span,
    /// add the body's references); a key the shell doesn't own is a body local,
    /// *inserted* (overwrite-on-collision) — so a duplicate proc's locals union
    /// across definitions with last-definition-wins for shared keys, matching the
    /// whole-file `define_var`.  (For a non-duplicate proc every local key is
    /// unique, so insert and merge coincide — byte-identical to before.)
    fn graft_proc_body(
        &mut self,
        db: &DeferredBody,
        mut frag: BodyFragment,
        shell_var_keys: &std::collections::HashSet<String>,
    ) {
        // Rebase: body facts are relative to the body content start.
        let delta = db.body_tok.span.start() + u32::from(db.body_tok.content_offset);
        let line_delta = self
            .line_offsets
            .as_deref()
            .map_or(0, |lo| super::state::line_at_offset(lo, delta as usize));
        rebase_fragment(&mut frag, delta, line_delta);

        if let Some(ps) = super::scope::scope_at_mut(&mut self.result.global_scope, &db.scope_path)
        {
            merge_scope_vars(&mut ps.variables, frag.proc_scope.variables);
            ps.procs = frag.proc_scope.procs;
            ps.classes = frag.proc_scope.classes;
            ps.children = frag.proc_scope.children;
        }
        let r = frag.result;
        self.result.all_procs.extend(r.all_procs);
        // Per qualified name, not a flat `extend`: a body that redefines a
        // proc it defines itself carries its own displaced definitions, and a
        // flat extend would replace rather than append to whatever the shell
        // already recorded for that name. (A body redefining a proc defined
        // *outside* it forces a full re-analyse above, so the shell and the
        // body never contend for the same key here.)
        for (qualified, defs) in r.superseded_procs {
            self.result
                .superseded_procs
                .entry(qualified)
                .or_default()
                .extend(defs);
        }
        self.result.all_classes.extend(r.all_classes);
        self.merge_body_variables(r.all_variables, shell_var_keys);
        self.result.command_aliases.extend(r.command_aliases);
        self.result.alias_offsets.extend(r.alias_offsets);
        self.result.renamed_commands.extend(r.renamed_commands);
        self.result.rename_offsets.extend(r.rename_offsets);
        // Per-ensemble union, not a flat `.extend()` — the outer key is the
        // ensemble's own name, and a flat extend would replace one grafted
        // body's whole inner subcommand map instead of merging into it.
        for (ensemble, subs) in r.ensemble_subcommand_targets {
            self.result
                .ensemble_subcommand_targets
                .entry(ensemble)
                .or_default()
                .extend(subs);
        }
        // Per-object methods accumulate per receiver name across bodies —
        // the binding-identity consumer scopes them by objdefine site
        // (issue #945 fault 5), so records from different procs coexist.
        for (k, v) in r.object_methods {
            self.result.object_methods.entry(k).or_default().extend(v);
        }
        self.result.instance_classes.extend(r.instance_classes);
        // `object_handle_facts` is deliberately **not** merged here (issue #994
        // C5a).  It has exactly one producer — `emit_cfg_ssa_diagnostics_with_cu`
        // — which runs on the *shell*, once, against the whole-file compilation
        // unit the per-item path supplies via `set_cu_override`.  An isolated
        // body fragment never reaches that producer (`analyse_proc_body_isolated`
        // calls `analyse_body`, not `analyse`, so the diagnostic tail never
        // runs), so `r.object_handle_facts` is always the empty default and a
        // merge here could only ever be a no-op — or, if the fragment ever did
        // gain facts, a *wrong* one: the fragment's owner keys are body-local
        // and its spans are offset-0, neither of which `rebase_fragment` knows
        // how to correct.  The `per_item_corpus` differential gate pins the
        // equality this comment asserts; the assertion below pins the premise
        // it rests on, so a future change that starts producing facts inside a
        // body fragment fails loudly here instead of silently dropping them.
        assert_no_body_object_facts(&r.object_handle_facts);
        self.result
            .created_instance_commands
            .extend(r.created_instance_commands);
        self.result
            .ambiguous_instance_names
            .extend(r.ambiguous_instance_names);
        self.result.diagnostics.extend(r.diagnostics);
        self.result
            .command_invocations
            .extend(r.command_invocations);
        self.result.package_requires.extend(r.package_requires);
        self.result.package_provides.extend(r.package_provides);
        self.result.source_targets.extend(r.source_targets);
        self.result.namespace_imports.extend(r.namespace_imports);
        self.result.namespace_exports.extend(r.namespace_exports);
        self.result
            .proc_declaration_sites
            .extend(r.proc_declaration_sites);
        self.result.class_body_spans.extend(r.class_body_spans);
        self.result.auto_path_entries.extend(r.auto_path_entries);
        self.result.qualified_var_refs.extend(r.qualified_var_refs);
        self.result.namespace_refs.extend(r.namespace_refs);
        self.result.regex_patterns.extend(r.regex_patterns);
        self.result
            .namespace_overrides
            .extend(r.namespace_overrides);
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
        self.pending_user_call_arity
            .extend(frag.pending_user_call_arity);
        self.pending_ctor_arity.extend(frag.pending_ctor_arity);
        self.pending_next_arity.extend(frag.pending_next_arity);
        self.var_command_sites.extend(frag.var_sites);
        self.cmd_command_sites.extend(frag.cmd_sites);
        // Replay the body's qualified global reads against the shell's real
        // global scope (rebased to the body's position): a `$::g` read lands as
        // a reference on the enclosing `::g` exactly as a whole-file walk would.
        // Replay each captured qualified read on the shell's real globals — but
        // only when the enclosing definition **precedes this body** in source
        // order.  A whole-file DFS walks a proc body at the proc's definition
        // point, so a top-level `set ::ns::v` that appears *after* the proc is
        // not yet defined when the body's `$::ns::v` read runs (no reference
        // recorded).  The per-item shell processes every top-level `set` before
        // any deferred body, so without this guard a later definition would
        // spuriously absorb the read.  Gate on the body token start (the proc
        // header precedes it, and no top-level command can fall inside the body).
        self.replay_body_global_reads(frag.global_reads, db.body_tok.span.start(), delta);
        // Re-defer the body's would-be-W002 sites (already rebased by
        // `rebase_fragment`); the tail re-applies the shadow check against
        // the merged `all_procs` / `command_aliases` / `renamed_commands` /
        // `all_classes` / `ensemble_namespaces`.
        self.pending_disabled_commands
            .extend(frag.disabled_commands);
        // Same treatment for the body's deferred W143 sites.
        self.pending_w143.extend(frag.private_ns_calls);
        // Re-defer the body's source-dependent W304 sites, rebasing the token,
        // fix, and diagnostic spans to absolute; the tail classifies each `$var`
        // against the full file source.
        for (mut tok, label, mut fixes, diag_span) in frag.w304 {
            tok.span = shift(tok.span, delta);
            for fix in &mut fixes {
                fix.span = shift(fix.span, delta);
            }
            self.pending_w304
                .push((tok, label, fixes, shift(diag_span, delta)));
        }
        // Replay captured instance creations against the shell's full
        // `all_classes` (in body/source order, so last-assignment-wins matches
        // the whole-file walk).  `instance_classes` carries no spans, so no
        // rebasing is needed.
        for (cmd, args, creation_ns) in frag.instances {
            self.record_instance_creation(&cmd, &args, &creation_ns);
        }
    }

    /// Merge a grafted body's `all_variables` into the shell's, keyed by the
    /// shell-ownership rule [`Self::graft_proc_body`]'s doc comment
    /// describes.  Extracted to keep that function within the line budget.
    fn merge_body_variables(
        &mut self,
        body_vars: std::collections::HashMap<String, super::types::VarDef>,
        shell_var_keys: &std::collections::HashSet<String>,
    ) {
        for (k, v) in body_vars {
            match self.result.all_variables.get_mut(&k) {
                Some(existing) if shell_var_keys.contains(&k) => {
                    // Param / top-level key the shell already owns: take this
                    // body's references / warn / indices (last-definition-wins for
                    // a duplicate proc — analyse's `define_var` resets them on each
                    // redefinition) but keep the shell's real definition span.
                    let def = existing.definition_span;
                    *existing = v;
                    existing.definition_span = def;
                }
                Some(existing) => {
                    // Body-owned local already present (an earlier duplicate
                    // definition): last-definition-wins.
                    *existing = v;
                }
                None => {
                    self.result.all_variables.insert(k, v);
                }
            }
        }
    }

    /// Replay a grafted body's qualified global reads against the shell's
    /// real global scope: a `$::g` read lands as a reference on the
    /// enclosing `::g` exactly as a whole-file walk would.  Extracted from
    /// [`Self::graft_proc_body`], whose doc comment covers the
    /// visibility-ordering rule (`body_start` gates out a definition that
    /// only appears *after* this body in source order).
    fn replay_body_global_reads(
        &mut self,
        global_reads: Vec<(String, tcl_lexer::Span)>,
        body_start: u32,
        delta: u32,
    ) {
        for (name, span) in global_reads {
            let base = crate::naming::normalise_var_name(&name);
            let visible = self
                .result
                .global_scope
                .variables
                .get(base)
                .is_some_and(|v| v.definition_span.start() < body_start);
            if visible {
                self.record_var_read(&name, shift(span, delta), &[]);
            }
        }
    }
}

/// The facts produced by analysing one proc body in isolation.  Cloneable +
/// `PartialEq` so the LSP query DB can memoise it (salsa early-cutoff).
#[derive(Clone, PartialEq)]
pub struct BodyFragment {
    result: AnalysisResult,
    proc_scope: super::types::Scope,
    ensembles: std::collections::HashSet<String>,
    pending_arity: Vec<(String, String, bool, super::types::Diagnostic)>,
    pending_user_call_arity: Vec<super::types::PendingUserCallArity>,
    pending_ctor_arity: Vec<super::types::PendingCtorArity>,
    pending_next_arity: Vec<super::types::PendingNextArity>,
    var_sites: Vec<super::state::VarCommandSite>,
    cmd_sites: Vec<super::state::CmdCommandSite>,
    /// Qualified (`::`/`static::`) reads that missed the isolated body's empty
    /// enclosing global scope; replayed on the shell's real globals at graft.
    global_reads: Vec<(String, tcl_lexer::Span)>,
    /// W002 (disabled-in-dialect command) sites — always deferred (see
    /// [`super::state::Analyser::pending_disabled_commands`]), so this is
    /// simply the isolated body's own queue: `(command name, call-site
    /// namespace, enforce_order, deferred diagnostic)`, the same shape as
    /// [`Self::pending_arity`]. `rebase_fragment` rebases the diagnostic span
    /// in place; the graft re-defers it onto the shell's queue and the tail
    /// re-checks the shadow against the merged `all_procs` /
    /// `command_aliases` / `renamed_commands` / `all_classes` /
    /// `ensemble_namespaces`.
    disabled_commands: Vec<(String, String, bool, super::types::Diagnostic)>,
    /// W143 (private `::tcl::` namespace) sites — likewise always deferred
    /// (see [`super::state::Analyser::pending_w143`]) and carried in the same
    /// shape, so the tail can apply the whole-file `proc` / `package require`
    /// suppressions an isolated body cannot see.
    private_ns_calls: Vec<(String, String, bool, super::types::Diagnostic)>,
    /// Deferred W304 (missing `--`) sites whose `$var` classification needs the
    /// whole-file most-recent-`set` resolution (invisible to an isolated body).
    /// The graft rebases the token + fix + diagnostic spans; the tail classifies
    /// them against the full source.
    w304: Vec<(
        tcl_lexer::Token,
        String,
        Vec<super::types::CodeFix>,
        tcl_lexer::Span,
    )>,
    /// Captured `TclOO` instance-creation candidates (`(command, args, creation
    /// namespace)`); the graft replays them against the shell's full
    /// `all_classes`.
    instances: Vec<(String, Vec<String>, String)>,
}

/// Analyse one `proc` **or method** body as an isolated unit at **offset 0** — a
/// pure function of `(body_text, namespace, scope_name, params, is_method,
/// class_variables)`, so a shifted-but-unedited body is a cache hit
/// (offset-invariant).  The enclosing namespace + params are reconstructed
/// (a `Method` scope with instance variables pre-bound for methods, a `Proc`
/// scope otherwise); params / instance vars get a placeholder definition span
/// (the aggregator keeps the shell's real one).  Spans/lines are relative to the
/// body start; [`Analyser::graft_proc_body`] rebases them by the body's real
/// position.
#[must_use]
pub fn analyse_proc_body_isolated<S: std::hash::BuildHasher>(
    db: &DeferredBody,
    dialect: &str,
    disabled: &std::collections::HashSet<String, S>,
    non_ascii: super::state::NonAsciiMode,
    stub_overlay: Option<tcl_registry::stub_overlay::StubOverlay>,
) -> BodyFragment {
    // Rebuild into the default-hasher set `Analyser` stores.
    let disabled: std::collections::HashSet<String> = disabled.iter().cloned().collect();
    let mut a = Analyser::with_disabled_diagnostics(disabled).with_non_ascii_mode(non_ascii);
    a.profile = tcl_dialect::DialectProfile::by_name(dialect);
    a.stub_overlay = stub_overlay;
    // Offset 0: the body content is the whole source; a synthetic `Str` body
    // token spans it with `content_offset = 0` (no `{` to skip).
    a.source = db.body_text.to_string();
    // Source spans are `u32`; a proc body longer than 4 GiB is not
    // representable (and never occurs), so clamp to `u32::MAX`.
    let body_len = u32::try_from(db.body_text.len()).unwrap_or(u32::MAX);
    let body_tok = Token::new(tcl_lexer::TokenType::Str, tcl_lexer::Span::new(0, body_len));
    a.registry = Some(tcl_registry::cache::registry_for_profile(a.profile));
    a.line_offsets = Some(super::state::compute_line_offsets(&a.source));
    // Capture qualified (`::`/`static::`) reads that miss the (empty) enclosing
    // global scope, so the graft can replay them on the shell's real globals.
    a.capture_global_reads = Some(Vec::new());
    // Capture object-instance creations: the isolated body's `all_classes` is
    // empty, so the class can't be resolved here — the graft replays them.
    a.pending_instances = Some(Vec::new());
    // A method body is analysed in a `Method` scope (so `in_method` dispatch
    // recording fires) under the class's defining namespace, with the class
    // instance variables pre-bound; a proc body in a `Proc` scope.
    let kind = if db.is_method {
        super::types::ScopeKind::Method
    } else {
        super::types::ScopeKind::Proc
    };
    let proc_path = reconstruct_proc_scope(
        &mut a.result.global_scope,
        &db.namespace,
        &db.scope_name,
        kind,
    );
    // A `TclOO` method's isolated scope resolves bare commands globally, like
    // its whole-file twin (`Scope::oo_global_resolution`).
    if db.oo_global_resolution
        && let Some(scope) = super::scope::scope_at_mut(&mut a.result.global_scope, &proc_path)
    {
        scope.oo_global_resolution = true;
        // Every `DeferredBody` is a real method body — `walk_class_init_body`
        // walks the class-level `initialise` frame inline and never defers —
        // so the isolated scope is a method frame too (`Scope::oo_method_frame`).
        scope.oo_method_frame = true;
    }
    let placeholder = tcl_lexer::Span::new(0, 0);
    let dummy = Token::new(tcl_lexer::TokenType::Str, placeholder);
    // Re-binding params / instance variables into the isolated scope is purely
    // structural (so body references resolve). The shell walk already emitted
    // W215 for these declarations byte-identically to the full `analyse` path,
    // and the synthetic `dummy` token's kind would otherwise flip the W215
    // `braced` reachability heuristic — so suppress W215 across the rebind to
    // avoid a spurious duplicate. The body walk below re-enables it so genuine
    // in-body declarations are still checked.
    a.suppress_w215 = true;
    for p in &db.params {
        a.define_var(&p.name, dummy, &proc_path, false, Some(placeholder));
    }
    // Class instance variables — visible in every method body (skip ones that a
    // formal parameter already shadows, matching `walk_method_body`).
    for var in &db.class_variables {
        let base = crate::naming::normalise_var_name(var);
        if base.is_empty() || db.params.iter().any(|p| p.name == base) {
            continue;
        }
        // The isolated body only needs the binding to exist so `$v` reads
        // resolve; the authoritative `definition_span` comes from the shell
        // walk and is preserved by the graft (`merge_one_var`), so a
        // `placeholder` here is discarded for this shell-owned key.
        a.define_var(base, dummy, &proc_path, false, Some(placeholder));
    }
    a.suppress_w215 = false;
    // Restore the enclosing safe-interpreter visibility context (issue #1001
    // follow-up) so a hidden call inside this body — reached only via
    // incremental analysis's isolated second pass — still hits
    // `safe_interp_visibility_gate` the same way it would in a directly-
    // written body under the whole-file `analyse` path. `None` (the
    // overwhelming common case) leaves the fresh analyser's stack empty,
    // exactly as before this field existed.
    if let Some((base_hidden, hidden_extra, exposed)) = &db.safe_interp_ctx {
        a.safe_interp_stack.push(super::state::SafeInterpCtx {
            base_hidden: *base_hidden,
            hidden_extra: hidden_extra.iter().cloned().collect(),
            exposed: exposed.iter().cloned().collect(),
        });
    }
    a.analyse_body(&db.body_text, body_tok, &proc_path);
    let proc_scope = super::scope::scope_at_mut(&mut a.result.global_scope, &proc_path)
        .expect("reconstructed proc scope")
        .clone();
    BodyFragment {
        result: a.result,
        proc_scope,
        ensembles: a.ensemble_namespaces,
        pending_arity: a.pending_arity,
        pending_user_call_arity: a.pending_user_call_arity,
        pending_ctor_arity: a.pending_ctor_arity,
        pending_next_arity: a.pending_next_arity,
        var_sites: a.var_command_sites,
        cmd_sites: a.cmd_command_sites,
        global_reads: a.capture_global_reads.unwrap_or_default(),
        disabled_commands: a.pending_disabled_commands,
        private_ns_calls: a.pending_w143,
        w304: a.pending_w304,
        instances: a.pending_instances.unwrap_or_default(),
    }
}

/// Debug-only pin for the premise `graft_proc_body`'s `object_handle_facts`
/// comment rests on: an isolated body fragment never reaches the fact's one
/// producer, so there is nothing to merge.  A future change that starts
/// producing facts inside a body fails here instead of silently dropping them.
fn assert_no_body_object_facts(facts: &crate::object_types::ObjectHandleFacts) {
    debug_assert_eq!(
        *facts,
        crate::object_types::ObjectHandleFacts::default(),
        "an isolated body fragment must not produce object_handle_facts — see \
         `graft_proc_body` before adding a merge"
    );
}

/// Merge body-derived variables into a scope/`all_variables` map: an existing
/// key is a param the shell already recorded (keep its real definition span;
/// add the body's reads / warn flag / array indices); a new key is a body local
/// (insert).
fn merge_scope_vars(
    dst: &mut std::collections::HashMap<String, super::types::VarDef>,
    src: std::collections::HashMap<String, super::types::VarDef>,
) {
    for (k, v) in src {
        merge_one_var(dst, k, v);
    }
}

/// Would grafting `frag`'s `all_classes` *overwrite* an entry the shell already
/// holds with a **different** value?  That is the accumulation case a whole-file
/// walk handles but the graft's overwrite-on-key cannot (`oo::define` adds
/// methods to an existing class), so the caller falls back.  A *fresh*
/// (non-colliding) class definition returns `false` and stays on the fast path.
fn class_facts_collide(shell: &AnalysisResult, frag: &AnalysisResult) -> bool {
    frag.all_classes
        .iter()
        .any(|(k, v)| shell.all_classes.get(k).is_some_and(|e| e != v))
}

/// Merge one body-derived variable into a map: an existing key keeps its
/// definition span (the shell's real param span) and accumulates the body's
/// references / warn flag / array indices; a new key is inserted.
fn merge_one_var(
    dst: &mut std::collections::HashMap<String, super::types::VarDef>,
    k: String,
    v: super::types::VarDef,
) {
    if let Some(existing) = dst.get_mut(&k) {
        existing.references.extend(v.references);
        existing.warn_if_unused |= v.warn_if_unused;
        existing.array_indices.extend(v.array_indices);
        // Keep the namespace/global alias link if the body pass discovered one
        // the shell didn't (a `global`/`variable`/`namespace upvar` in a
        // deferred proc body).
        if existing.link_target.is_none() {
            existing.link_target = v.link_target;
            // The span of the word naming that cell travels with it — the two
            // are one fact (see `VarDef::link_target_span`), so they must never
            // be merged apart.
            existing.link_target_span = v.link_target_span;
        }
    } else {
        dst.insert(k, v);
    }
}

fn shift(s: tcl_lexer::Span, d: u32) -> tcl_lexer::Span {
    tcl_lexer::Span::new(s.start() + d, s.end() + d)
}

fn rebase_vardef(v: &mut super::types::VarDef, d: u32) {
    v.definition_span = shift(v.definition_span, d);
    // The cell-naming word is a span into the same body and rebases with the
    // rest; leaving it body-relative would point a rename at the wrong bytes
    // of the whole document.
    if let Some(span) = v.link_target_span {
        v.link_target_span = Some(shift(span, d));
    }
    for r in &mut v.references {
        *r = shift(*r, d);
    }
}

fn rebase_proc(p: &mut super::types::ProcDef, d: u32) {
    p.name_span = shift(p.name_span, d);
    p.body_span = shift(p.body_span, d);
}

fn rebase_method(m: &mut super::types::MethodDef, d: u32) {
    m.name_span = shift(m.name_span, d);
    m.body_span = shift(m.body_span, d);
}

fn rebase_class(c: &mut super::types::ClassDef, d: u32) {
    c.name_span = shift(c.name_span, d);
    c.body_span = shift(c.body_span, d);
    for m in c.methods.values_mut().chain(c.class_methods.values_mut()) {
        rebase_method(m, d);
    }
    for m in &mut c.constructors {
        rebase_method(m, d);
    }
    if let Some(m) = &mut c.destructor {
        rebase_method(m, d);
    }
    for p in c.properties.values_mut() {
        p.name_span = shift(p.name_span, d);
    }
}

fn rebase_diag(diag: &mut super::types::Diagnostic, d: u32) {
    diag.span = shift(diag.span, d);
    for f in &mut diag.fixes {
        f.span = shift(f.span, d);
    }
}

fn rebase_scope(s: &mut super::types::Scope, d: u32) {
    if let Some(bs) = &mut s.body_span {
        *bs = shift(*bs, d);
    }
    for v in s.variables.values_mut() {
        rebase_vardef(v, d);
    }
    for p in s.procs.values_mut() {
        rebase_proc(p, d);
    }
    for c in s.classes.values_mut() {
        rebase_class(c, d);
    }
    for ch in &mut s.children {
        rebase_scope(ch, d);
    }
}

/// Shift every span in an offset-0 body fragment to the body's real position
/// (`d` bytes; suppressed-line keys by `line_delta` lines).  E2 (offset-shift
/// invariance) guarantees this reproduces an in-place walk exactly.
fn rebase_fragment(frag: &mut BodyFragment, d: u32, line_delta: i32) {
    rebase_scope(&mut frag.proc_scope, d);
    let r = &mut frag.result;
    for p in r.all_procs.values_mut() {
        rebase_proc(p, d);
    }
    for defs in r.superseded_procs.values_mut() {
        for p in defs.iter_mut() {
            rebase_proc(p, d);
        }
    }
    for c in r.all_classes.values_mut() {
        rebase_class(c, d);
    }
    for v in r.all_variables.values_mut() {
        rebase_vardef(v, d);
    }
    for diag in &mut r.diagnostics {
        rebase_diag(diag, d);
    }
    for inv in &mut r.command_invocations {
        inv.range = shift(inv.range, d);
    }
    for x in &mut r.package_requires {
        x.range = shift(x.range, d);
    }
    for x in &mut r.package_provides {
        x.range = shift(x.range, d);
    }
    for x in &mut r.source_targets {
        x.range = shift(x.range, d);
    }
    for x in &mut r.namespace_imports {
        x.range = shift(x.range, d);
    }
    for x in &mut r.namespace_exports {
        x.range = shift(x.range, d);
    }
    for (_, span) in &mut r.proc_declaration_sites {
        *span = shift(*span, d);
    }
    for (_, span) in &mut r.class_body_spans {
        *span = shift(*span, d);
    }
    for off in r.alias_offsets.values_mut() {
        *off += d;
    }
    for off in r.rename_offsets.values_mut() {
        *off += d;
    }
    for records in r.object_methods.values_mut() {
        for om in records {
            om.objdefine_offset += d;
            om.def.name_span = shift(om.def.name_span, d);
            om.def.body_span = shift(om.def.body_span, d);
        }
    }
    for x in &mut r.auto_path_entries {
        x.range = shift(x.range, d);
    }
    for x in &mut r.qualified_var_refs {
        x.span = shift(x.span, d);
    }
    for x in &mut r.namespace_refs {
        x.span = shift(x.span, d);
    }
    for x in &mut r.regex_patterns {
        x.range = shift(x.range, d);
    }
    for (span, _) in &mut r.namespace_overrides {
        *span = shift(*span, d);
    }
    if !r.suppressed_lines.is_empty() {
        let old = std::mem::take(&mut r.suppressed_lines);
        for (line, codes) in old {
            let nl = if line < 0 { line } else { line + line_delta };
            r.suppressed_lines.entry(nl).or_default().extend(codes);
        }
    }
    rebase_fragment_pending(frag, d);
}

/// The [`rebase_fragment`] half that shifts the fragment's own *pending*
/// state — the deferred diagnostics and arity / site candidates the
/// aggregator flushes later — as opposed to its `AnalysisResult`.  Split out
/// only to keep each function within the line budget.
fn rebase_fragment_pending(frag: &mut BodyFragment, d: u32) {
    for (_, _, _, diag) in &mut frag.pending_arity {
        rebase_diag(diag, d);
    }
    for (_, _, _, diag) in &mut frag.disabled_commands {
        rebase_diag(diag, d);
    }
    for (_, _, _, diag) in &mut frag.private_ns_calls {
        rebase_diag(diag, d);
    }
    for cand in &mut frag.pending_user_call_arity {
        cand.call_off += d;
        cand.full_span = shift(cand.full_span, d);
    }
    for cand in &mut frag.pending_ctor_arity {
        cand.call_off += d;
        cand.full_span = shift(cand.full_span, d);
    }
    for cand in &mut frag.pending_next_arity {
        cand.full_span = shift(cand.full_span, d);
    }
    for s in &mut frag.var_sites {
        s.cmd_span = shift(s.cmd_span, d);
    }
    for s in &mut frag.cmd_sites {
        s.cmd_span = shift(s.cmd_span, d);
    }
}

/// Build `global -> namespace* -> proc` scopes mirroring the proc's lexical
/// context (so nested defs qualify identically), returning the proc scope path.
/// Build a `namespace`-rooted scope chain under `root` and push a `kind` scope
/// named `scope_name` at its end, returning the path to that scope.
///
/// Used both by the per-item isolated re-analysis (to reconstruct a proc /
/// method context) and by `handle_apply_command` (to root an `apply` lambda
/// scope at the lambda's namespace rather than the caller's), so the inline and
/// per-item walks build byte-identical structure.
pub(crate) fn reconstruct_proc_scope(
    root: &mut super::types::Scope,
    namespace: &str,
    scope_name: &str,
    kind: super::types::ScopeKind,
) -> Vec<usize> {
    use super::types::Scope;
    use super::types::ScopeKind;
    // `namespace` is a constructed key: split it by the construction-inverse
    // rule so a legitimately colon-named segment survives (#934) — a
    // char-pattern trim + `split("::")` would collapse it.
    let comps = crate::naming::key_segments(namespace);
    let mut path: Vec<usize> = Vec::new();
    for comp in comps {
        let parent = super::scope::scope_at_mut(root, &path).expect("scope path");
        let i = parent.children.len();
        parent
            .children
            .push(Scope::new(ScopeKind::Namespace, &comp));
        path.push(i);
    }
    let parent = super::scope::scope_at_mut(root, &path).expect("scope path");
    let pi = parent.children.len();
    parent.children.push(Scope::new(kind, scope_name));
    path.push(pi);
    path
}

/// Does an isolated proc body interact with enclosing-scope state in a way the
/// per-item graft can't reproduce byte-for-byte, forcing a full-rebuild
/// fallback?  True when the body declares a link to a **qualified (sub-namespace)
/// variable** — a `variable ns::name` command inside the body.
///
/// Such a name resolves to a variable owned by *another* item (the
/// `namespace eval ns { variable name … }` body), so its `all_variables` entry
/// accumulates a definition from both this body and that namespace block.  The
/// order-sensitive facts of the merged entry — `warn_if_unused` and the winning
/// `definition_span` — depend on the cross-item walk order, which the whole-file
/// DFS and the per-item shell-first walk disagree on; the graft's
/// position-independent merge can't reproduce the DFS result.  (The visible
/// *diagnostics* already match; only this internal book-keeping diverges, but the
/// correctness contract is full-`AnalysisResult` byte-identity.)
///
/// Everything else an enclosing-touching body does is reproduced on the fast
/// path and proven byte-identical over the corpus: `global` / `upvar` / plain
/// `variable x` declarations, qualified writes (`set ::x`), class definitions
/// (`oo::class` / `oo::define`), and qualified reads (`$::g`, captured and
/// replayed at graft).  Backstopped by the differential `incremental == fresh`
/// fuzzer + this fallback — a false positive costs only a redundant rebuild.
///
/// Line-scoped so it doesn't trip on a plain `variable x` whose body merely also
/// mentions a `::`-qualified name elsewhere (which would needlessly defeat the
/// fast path on large files like `practcl.tcl`).
fn body_needs_enclosing_context(body_text: &str) -> bool {
    for line in body_text.lines() {
        let mut words = line.split_whitespace();
        while let Some(w) = words.next() {
            // A `variable` command (anywhere on the line) with a `::`-qualified
            // argument links to a sub-namespace variable.  Checking the rest of
            // the line is deliberately conservative (it may include a following
            // `;`-separated command), which only over-triggers the fallback.
            if w == "variable" && words.clone().any(|a| a.contains("::")) {
                return true;
            }
            // A `namespace import` *inside a body* leaks into the whole-file walk's
            // global `namespace_imports` (the DFS records every import it walks,
            // wherever it sits), so a later sibling body resolves a bare imported
            // command (e.g. `test` ⇒ `::tcltest::test`) and walks its body-role
            // args — a cross-body effect the isolated per-item walk can't reproduce
            // from this body alone.  Rare (imports are normally top-level, where the
            // shell pass records them and the deferred bodies inherit them), so a
            // fallback here costs only a redundant rebuild.
            if w == "namespace" && words.clone().next() == Some("import") {
                return true;
            }
            // Symmetric with the `import` guard above: a `namespace export`
            // *inside a body* leaks into the whole-file walk's global
            // `namespace_exports` the same way, and its `-clear` variant only
            // makes sense relative to the shell's already-recorded entries —
            // book-keeping the isolated per-item walk can't reproduce from
            // this body alone. Rare (export declarations are normally
            // top-level), so a fallback here costs only a redundant rebuild.
            if w == "namespace" && words.clone().next() == Some("export") {
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
    fn quoted_var_substitution_does_not_false_fire_e202() {
        eq("puts \"$sum\"\n");
    }

    #[test]
    fn same_file_arity_e003_matches_across_proc_body_call() {
        // A proc-body call to a same-file proc with the wrong argument
        // count — exercises `pending_user_call_arity`'s isolated-body
        // capture + span-rebase-on-graft path, not just the whole-file
        // walk.
        eq("proc need2 {a b} {}\nproc caller {} { need2 1 2 3 }\n");
    }

    #[test]
    fn same_file_arity_via_alias_and_rename_matches_across_proc_body_call() {
        eq("proc target {a b c} {}\n\
            interp alias {} shortcut {} target 100\n\
            rename target target_orig\n\
            proc caller {} { shortcut 1; target_orig 1 2 }\n");
    }

    #[test]
    fn namespace_import_inside_body_falls_back() {
        // A `namespace import` buried in a proc body leaks into the whole-file
        // walk's global import set (reached by a later sibling body's bare
        // imported command); the per-item decomposition can't reproduce that
        // cross-body effect from the isolated body alone, so it falls back to a
        // full rebuild — still byte-identical. See `body_needs_enclosing_context`.
        eq("proc setup {} { namespace import -force ::tcltest::* }\n\
            proc p {cmd} { test t desc -body { {*}$cmd } -result {} }\n");
    }

    #[test]
    fn proc_calling_proc() {
        eq("proc a {x} { return $x }\nproc b {} { a 1 }\n");
    }

    #[test]
    fn renamed_away_builtin_w123_matches() {
        // A top-level rename-away of a builtin plus a later call — the W123
        // deleted-builtin gate reads shell-pass state (`deleted_commands`),
        // which both paths populate identically for top-level statements.
        eq("rename puts {}\nputs x\n");
        eq("puts early\nrename puts myputs\nmyputs late\n");
        // An in-body rename is *conditional* (runs at call time): the gate
        // ignores it on both paths — the whole-file walk records it into
        // `deleted_commands` but filters it as inside a proc body, while an
        // isolated body's analyser state never reaches the shell at all.
        eq("proc hook {} { rename puts _p }\nputs hi\n");
    }

    #[test]
    fn registry_defined_command_names_match() {
        // `coroutine NAME cmd` / `interp create NAME` bind NAME via the
        // registry's `defines_command_at` — recorded eagerly in the isolated
        // body too, and merged at graft, so both paths agree.
        eq("proc gen {} { while 1 { yield 1 } }\ncoroutine nextNum gen\nnextNum\n");
        eq("proc mk {} { coroutine nextNum ::gen }\ninterp create child\nchild eval { puts hi }\n");
    }

    #[test]
    fn literal_namespace_import_w123_matches() {
        // Top-level literal imports feed the W123 tail-glob suppression from
        // `result.namespace_imports`, recorded by the shell pass on both
        // paths (a body-local import falls back — see
        // `namespace_import_inside_body_falls_back`).
        eq("namespace import ::acme::widgets::render_*\nrender_box 10\nfrobnicate 1\n");
    }

    #[test]
    fn wildcard_namespace_import_gated_by_export_matches() {
        // The headline shape (issue #923 idx 18) exercised through the
        // incremental/whole-file equivalence harness: `namespace export`
        // gates which commands a wildcard import can resolve
        // (`result.namespace_exports`), recorded identically by the shell
        // pass on both paths.
        eq("namespace eval Foo {\n    proc bar {} { return 1 }\n    \
             proc other {} { return 2 }\n    namespace export bar\n}\n\
             namespace import ::Foo::*\nbar\nother\n");
    }

    #[test]
    fn namespace_export_clear_tombstone_ordering_matches() {
        // Issue #1027: `-clear` is recorded as an ordered tombstone rather
        // than applied by deleting the namespace's earlier entries, so the
        // export log now carries *more* rows and their relative order is
        // load-bearing. Both walks must produce the identical log — a rebased
        // offset or a dropped tombstone on either path would change what a
        // per-import-site snapshot answers.
        eq("namespace eval src {\n    proc p {} { return P }\n    \
             namespace export p\n}\nnamespace eval dst {\n    \
             namespace import ::src::*\n}\nnamespace eval src {\n    \
             namespace export -clear q\n}\n");
    }

    #[test]
    fn namespace_export_inside_body_falls_back() {
        // A `namespace export` buried in a proc body leaks into the
        // whole-file walk's global export set (reached by a later
        // sibling's wildcard import); the per-item decomposition can't
        // reproduce that cross-body effect from the isolated body alone,
        // so it falls back to a full rebuild — still byte-identical. See
        // `body_needs_enclosing_context`.
        eq("namespace eval Foo {\n    proc bar {} { return 1 }\n    \
             proc setup {} { namespace export bar }\n}\n\
             namespace import ::Foo::*\nbar\n");
    }

    #[test]
    fn namespace_unknown_handler_suppression_matches() {
        // `namespace unknown HANDLER` flips `has_dynamic_providers` (merged
        // with `|=` at graft), suppressing W123 identically on both paths.
        eq("proc h {args} { puts $args }\nnamespace unknown h\nmystery_cmd 1\n");
        eq("proc inst {} { namespace unknown ::h }\nmystery_cmd 1\n");
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
    fn method_body_defines_duplicate_proc() {
        // A method that defines `::p`, plus a later top-level `proc p`: a
        // whole-file walk ends with the top-level definition (source order),
        // so the per-item path must fall back rather than let the pass-2
        // method body win the shared `all_procs["::p"]` key.
        eq(
            "oo::class create C { method m {} { proc p {} { return 1 } } }\nproc p {} { return 2 }\n",
        );
        // Same, reversed order (top-level first, then the method).
        eq(
            "proc p {} { return 2 }\noo::class create C { method m {} { proc p {} { return 1 } } }\n",
        );
        // A method that defines a non-colliding proc stays byte-identical too.
        eq("oo::class create C { method m {} { proc helper {} { return 1 } } }\n");
    }

    #[test]
    fn nested_proc_in_body() {
        eq("proc outer {} { proc ::inner {} { set z 1 } ; inner }\n");
    }

    #[test]
    fn qualified_read_marks_enclosing_global_used() {
        // A body's `$::g` read must land as a reference on the enclosing `::g`
        // (captured + replayed at graft), so the "unused" state matches a
        // whole-file walk even though the body is analysed in isolation.
        eq("set ::g 1\nproc r {} { return [expr {$::g + 1}] }\n");
        eq("set ::g 1\nproc r {} { return 0 }\n"); // unused global stays unused
    }

    #[test]
    fn duplicate_top_level_proc_byte_identical() {
        // Platform-conditional redefinition: last-def-wins in a whole-file walk.
        // The per-item path reproduces it on the fast path (the graft unions each
        // definition's `all_variables` with last-definition-wins on shared keys).
        eq(
            "if {1} {\n  proc p {} { set file a; puts $file }\n} else {\n  proc p {} { return 2 }\n}\n",
        );
    }

    #[test]
    fn duplicate_proc_same_local_last_wins() {
        // Same-named local across two definitions: `all_variables` keeps the last
        // definition's span + references (not the first, not the union of refs).
        eq("proc p {} { set a 1 }\nproc p {} { set a 2 }\n");
        eq("proc p {} { set a 1; incr a }\nproc p {} { set a 2; incr a }\n");
    }

    #[test]
    fn duplicate_proc_unique_locals_unioned() {
        // A local defined only in an earlier definition survives (union), while a
        // shared local is last-wins — the whole-file `define_var` semantics.
        eq("proc p {} { set early 1 }\nproc p {} { set late 2 }\n");
    }

    #[test]
    fn duplicate_proc_with_params() {
        // Param references on a duplicate proc are last-definition-wins (the
        // earlier body's reads are dropped), and the shell's real param span is
        // kept (not the isolated body's placeholder).
        eq("proc p {key} { return $key }\nproc p {key} { puts $key }\n");
    }

    #[test]
    fn multi_method_class_byte_identical() {
        // A class with several methods must NOT be treated as a set of mutual
        // "duplicates" (the empty-scope-name bug) — it takes the fast path.
        eq(
            "oo::class create K {\n  method a {} { set x 1 }\n  method b {} { set y 2 }\n  method c {x} { return $x }\n}\n",
        );
    }

    #[test]
    fn instance_creation_in_proc_body() {
        // An isolated proc body can't resolve the sibling class; the creation is
        // captured and replayed at graft so `instance_classes` matches.
        eq("oo::class create Dog {}\nproc make {} { set d [Dog new] }\n");
        eq("oo::class create Dog {}\nproc make {} { Dog create rex }\n");
    }

    #[test]
    fn class_extension_in_proc_falls_back() {
        // A proc body that *extends* an existing class (oo::define adds a method)
        // accumulates across the definition site; the per-item graft can't
        // reproduce that, so it falls back — and stays byte-identical.
        eq(
            "oo::class create K { method a {} {} }\nproc ext {} { oo::define K method b {} {} }\next\n",
        );
    }

    #[test]
    fn self_redefining_proc_falls_back() {
        // Lazy-init: the inner `proc p` redefines the outer; nested-redefinition
        // detection forces a fallback so the result matches a whole-file walk.
        eq("proc p {a} { proc p {a} { return $a }\n p $a }\n");
    }

    #[test]
    fn param_list_with_backslash_continuation_no_spurious_w215() {
        // A param list split across lines by `\<newline>` list-parses as
        // separate parameters (`parse_param_list` treats the continuation as an
        // element separator, issue #743), so no param name ends in a raw
        // backslash and neither the full `analyse` walk nor the isolated
        // per-item rebind emits a spurious W215 unreachable-name warning.
        eq("proc foo {a b staticsok\\\n    c d} {\n  set x 1\n}\n");
        // Method form (instance vars + params rebind under the same suppression).
        eq("oo::class create K {\n  method m {a b\\\n    c} { return $a }\n}\n");
    }
}
