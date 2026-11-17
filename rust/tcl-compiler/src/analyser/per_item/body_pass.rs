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

//! The body pass: analysing each deferred body, the predicates that force the full-rebuild fallback and the replay of deferred instance sites.

use super::super::state::Analyser;
use super::super::types::AnalysisResult;
use super::{BodyFragment, DeferredBody, OVERSIZED_BODY_BYTES, PerItemFallback};

impl Analyser {
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
    /// Attach the ensemble facts each deferred body can see: the
    /// `subcommand → target` maps whose `namespace ensemble` recording
    /// **precedes** the body in source order (the whole-file DFS's own
    /// visibility rule), filtered to ensembles whose tail name occurs in the
    /// body text so an unrelated ensemble edit re-keys no body, canonically
    /// sorted so the snapshot can key `tcl-lsp-db`'s `ItemBodyKey`.
    ///
    /// The **one** site that decides what an isolated body knows about
    /// ensembles. `-prefixes 0` is attached here beside the map it modifies,
    /// rather than at a second site, because
    /// [`super::super::types::AnalysisResult::resolve_ensemble_subcommand`] reads the
    /// two together: attaching only the map let an isolated body resolve
    /// `<ensemble> fo` against an ensemble that refuses abbreviations and
    /// record a dispatch real Tcl rejects.
    pub(super) fn attach_ensemble_facts(&self, deferred: &mut [DeferredBody]) {
        if self.result.ensemble_subcommand_targets.is_empty() {
            return;
        }
        let mut all: Vec<(
            &String,
            &std::collections::HashMap<String, super::super::types::EnsembleSubcommandTarget>,
        )> = self.result.ensemble_subcommand_targets.iter().collect();
        all.sort_by_key(|(k, _)| *k);
        for db in deferred {
            let body_start = db.body_tok.span.start();
            let visible: Vec<(
                String,
                Vec<(String, super::super::types::EnsembleSubcommandTarget)>,
            )> = all
                .iter()
                .filter(|(k, _)| {
                    self.ensemble_record_offsets
                        .get(*k)
                        .is_some_and(|&off| off < body_start)
                        && db.body_text.contains(crate::naming::key_tail(k))
                })
                .map(|(k, subs)| {
                    let mut inner: Vec<(String, super::super::types::EnsembleSubcommandTarget)> =
                        subs.iter().map(|(s, t)| (s.clone(), t.clone())).collect();
                    inner.sort();
                    ((*k).clone(), inner)
                })
                .collect();
            db.prefixless_ensembles = visible
                .iter()
                .map(|(k, _)| k.clone())
                .filter(|k| self.result.ensemble_refuses_prefixes(k))
                .collect();
            db.ensemble_targets = visible;
        }
    }

    pub(super) fn fill_deferred_bodies(
        &mut self,
        source: &str,
        dialect: &str,
        body_fn: &mut dyn FnMut(&DeferredBody) -> BodyFragment,
    ) -> Result<(), Box<AnalysisResult>> {
        let deferred = std::mem::take(&mut self.deferred_bodies);
        // Whole declaration words cannot be reconstructed from a body-only
        // fragment by rebasing coordinates. Preserve the existing source owner.
        if deferred.iter().any(|body| {
            super::super::scope::scope_at(&self.result.global_scope, &body.scope_path)
                .is_some_and(|scope| scope.original_member_context.is_some())
        }) {
            self.per_item_fallback = Some(PerItemFallback::OriginalSourceOwnership);
            return Err(Box::new(self.fresh_full_analyse(source, dialect)));
        }
        // Scaling guard.  Analysing a body in isolation costs
        // markedly more than analysing the identical content in place, and the
        // gap widens with body size, so a single enormous body turns the
        // incremental path into a large *pessimisation* — on tcllib's generated
        // `fumagic/filetypes.tcl` (one ~1.2 MB body) the decomposition measured
        // 26x the whole-file walk, per keystroke.  Nor could the memoisation
        // above it repay that: re-analysing "one body" there *is* re-analysing
        // the file.  Such a document should not take the path at all.
        //
        // Checked before any `body_fn` call, so an oversized body costs only
        // the shell pass that discovered it.
        let oversized_body = deferred
            .iter()
            .any(|db| db.body_text.len() > OVERSIZED_BODY_BYTES);
        if oversized_body {
            self.per_item_fallback = Some(PerItemFallback::OversizedBody);
            return Err(Box::new(self.fresh_full_analyse(source, dialect)));
        }
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
        // Checked for proc *and* method bodies alike: a `variable ns::v`
        // (or in-body `namespace import`/`export`) inside a method body
        // leaks into whole-file state exactly as it does from a proc body
        // (caught by the `per_item_matches_analyse_under_edits` fuzzer).
        let enclosing_fallback = !self.probe_skip_enclosing_fallback
            && deferred.iter().any(|db| {
                body_needs_enclosing_context(&db.body_text)
                    || body_sources_dynamic_path(&db.body_text)
            });
        if duplicate_method || enclosing_fallback {
            self.per_item_fallback = Some(if duplicate_method {
                PerItemFallback::DuplicateMethod
            } else {
                PerItemFallback::EnclosingContext
            });
            return Err(Box::new(self.fresh_full_analyse(source, dialect)));
        }
        // Attach the whole-file command-mutation trust snapshot to every
        // body whose text could fold a command substitution:
        // the isolated body pass cannot see a `rename` elsewhere in the
        // file, so without this it must distrust everything and would
        // diverge from the whole-file walk's folds. Bodies with no fold
        // candidate stay `None`, so their memo keys are untouched by an
        // unrelated rename appearing elsewhere. The oracle itself is
        // computed at most once (it lowers the whole file).
        let mut deferred = deferred;
        if let Some(registry) = self.registry.clone() {
            let mut trust: Option<std::sync::Arc<crate::command_binding::CommandTrustSnapshot>> =
                None;
            for db in &mut deferred {
                if crate::const_subst::body_has_fold_candidate(&db.body_text, &registry) {
                    if trust.is_none() {
                        trust = self
                            .whole_file_command_trust()
                            .map(|m| std::sync::Arc::new(m.snapshot()));
                    }
                    db.command_trust.clone_from(&trust);
                }
            }
        }
        self.attach_ensemble_facts(&mut deferred);
        let deferred = deferred;
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
            if fragment_needs_original_source_owner(&frag) {
                self.per_item_fallback = Some(PerItemFallback::OriginalSourceOwnership);
                return Err(Box::new(self.fresh_full_analyse(source, dialect)));
            }
            self.graft_proc_body(db, frag, &shell_var_keys);
        }
        // Object-instance tracking (W308) resolves the class at the walk
        // point.  Every captured creation — the shell's and each body's —
        // replays here in source order against only the classes whose
        // definition precedes the site, reproducing the whole-file DFS's
        // progressive `all_classes` view.  A *method* body's TclOO instance
        // tracking still can't be replayed faithfully (object-namespace
        // resolution), so a method-origin replay that actually records an
        // instance forces the fallback, as before.
        if let Some(fallback) = self.replay_deferred_instances() {
            self.per_item_fallback = Some(fallback);
            return Err(Box::new(self.fresh_full_analyse(source, dialect)));
        }
        // Canonical source declarations are complete; join the original receiver
        // sites through their conditional source carriers.
        self.finalise_bareword_dispatch_sites();
        Ok(())
    }

    /// Apply the same original source-instance gate as the whole-document scan.
    /// Source receipts carry the original ordered context and known mutation
    /// blockers; final presentation maps supply no receiver authority.
    fn finalise_bareword_dispatch_sites(&mut self) {
        let Some(candidates) = self.pending_bareword_dispatch_sites.take() else {
            return;
        };
        self.var_command_sites
            .extend(candidates.into_iter().filter(|site| {
                crate::registry_invocation::source_structure::source_class_instance_words_at(
                    &self.source,
                    &self.result,
                    site.cmd_span.start(),
                )
                .is_some()
            }));
    }

    /// Replay captured construction-reporting sites in source order after graft.
    /// A method-origin reporting change requests a whole-file cache fallback.
    /// Native naming and receiver metadata require the genuine source-instance
    /// issuer against the current complete source image; the report maps and
    /// the temporary class-map filtering supply no command or lifetime proof.
    pub(super) fn replay_deferred_instances(&mut self) -> Option<PerItemFallback> {
        let mut sites = std::mem::take(&mut self.deferred_instance_replays);
        if let Some(shell) = self.pending_instances.take() {
            sites.extend(
                shell
                    .into_iter()
                    .map(|(cmd, args, ns, off)| (off, false, cmd, args, ns)),
            );
        }
        // Source order = the whole-file walk's recording order
        // (last-assignment-wins therefore matches).  Offsets are command-head
        // positions, unique per site, so the order is total.
        sites.sort_by_key(|s| s.0);
        for (off, from_method, cmd, args, ns) in sites {
            // Hide not-yet-defined classes: the DFS at this site had walked
            // exactly the defining commands positioned before it.
            let hidden: Vec<String> = self
                .result
                .all_classes
                .iter()
                .filter(|(_, c)| c.name_span.start() >= off)
                .map(|(k, _)| k.clone())
                .collect();
            let mut stash = Vec::with_capacity(hidden.len());
            for k in hidden {
                if let Some(v) = self.result.all_classes.remove(&k) {
                    stash.push((k, v));
                }
            }
            let before = from_method.then(|| self.result.instance_classes.clone());
            self.record_instance_creation(&cmd, &args, &ns, off);
            for (k, v) in stash {
                self.result.all_classes.insert(k, v);
            }
            if let Some(before) = before
                && self.result.instance_classes != before
            {
                return Some(PerItemFallback::MethodInstanceReplay);
            }
        }
        None
    }
}

/// Isolated source facts cannot publish names or retain call-site producers
/// in another document by shifting their coordinates. The full analyser is
/// the existing owner that can issue these receipts from original command rows.
pub(super) fn fragment_needs_original_source_owner(fragment: &BodyFragment) -> bool {
    let result = &fragment.result;
    if !fragment.private_ns_calls.is_empty()
        || !fragment.pending_ctor_arity.is_empty()
        || !fragment.pending_next_arity.is_empty()
        || !result.all_procs.is_empty()
        || !result.all_classes.is_empty()
        || !result.external_class_contributions.is_empty()
        || !result.original_procedure_metadata.is_empty()
        || !result.original_class_metadata.is_empty()
        || !result.original_counted_class_metadata.is_empty()
        || !result.logical_class_declarations.is_empty()
        || result
            .object_member_state
            .values()
            .flatten()
            .any(|state| state.logical_source_members.is_some())
        || !result.logical_procedure_declarations.is_empty()
        || !result.original_vendor_source_names.is_empty()
        || !result.original_vendor_procedure_metadata.is_empty()
        || !result.original_vendor_class_metadata.is_empty()
        || !result.original_vendor_symbol_metadata.is_empty()
        || !result.original_symbol_metadata.is_empty()
        || !result.original_scoped_bodies.is_empty()
        || !result.original_conditional_registry_metadata.is_empty()
        || !result.command_aliases.is_empty()
        || !result.renamed_commands.is_empty()
        || !result.package_requires.is_empty()
        || !result.package_provides.is_empty()
        || !result.package_ifneededs.is_empty()
        || result
            .source_targets
            .iter()
            .any(|target| target.original_interpreter_source_load.is_some())
        || result.command_invocations.iter().any(|invocation| {
            invocation.original_callback_signature_lookup.is_some()
                || invocation.original_callback_prefix.is_some()
                || invocation.original_name_input.is_some()
                || invocation.original_lookup.is_some()
                || invocation.resolved_command_reference.is_some()
                || invocation.resolved_definition.is_some()
                || (invocation.indirect && invocation.lookup.is_execution_site())
        })
        || result.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.subject(),
                Some(
                    super::super::DiagnosticSubject::RegistrySource(_)
                        | super::super::DiagnosticSubject::CallbackSourceArity(_)
                        | super::super::DiagnosticSubject::ObjectSourceArity(_)
                        | super::super::DiagnosticSubject::PrivateTclNamespace(_)
                        | super::super::DiagnosticSubject::UserSourceArity(_)
                        | super::super::DiagnosticSubject::CommandAvailability(_)
                        | super::super::DiagnosticSubject::RegisteredInstanceSource(_)
                        | super::super::DiagnosticSubject::UnresolvedCommand(_)
                        | super::super::DiagnosticSubject::UnresolvedMathFunction(_)
                        | super::super::DiagnosticSubject::ConditionalInterpreterVisibility(_)
                )
            )
        })
    {
        return true;
    }
    let mut scopes = vec![&fragment.proc_scope];
    while let Some(scope) = scopes.pop() {
        if scope.kind == super::super::types::ScopeKind::Namespace
            || scope.original_member_context.is_some()
        {
            return true;
        }
        scopes.extend(&scope.children);
    }
    false
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
/// Does the body `source` a `$var` path?  `handle_source_command` resolves
/// such a path through the walk-time constant-string lattice, whose scope
/// chain reaches the *enclosing* scopes on the whole-file walk (`set p
/// "e.tcl"` at top level, `source $p` inside a proc — a corpus idiom).  An
/// isolated body has no enclosing chain, so the resolved
/// `source_targets` entry (`raw_path` / `is_literal`) would diverge; fall
/// back to a full rebuild instead.  A path word containing `[` stays
/// conservatively dynamic on **both** walks (`resolve_dynamic_word` rejects
/// command substitution outright), so only a `$`-bearing, bracket-free word
/// forces the fallback.  Checked for proc *and* method bodies alike.
fn body_sources_dynamic_path(body_text: &str) -> bool {
    for line in body_text.lines() {
        let mut words = line.split_whitespace();
        while let Some(w) = words.next() {
            if w == "source" && words.clone().any(|a| a.contains('$') && !a.contains('[')) {
                return true;
            }
        }
    }
    false
}

pub(super) fn body_needs_enclosing_context(body_text: &str) -> bool {
    for line in body_text.lines() {
        let mut words = line.split_whitespace();
        while let Some(w) = words.next() {
            // A `variable` command (anywhere on the line) with a `::`-qualified
            // argument links to a sub-namespace variable.  Checking the rest of
            // the line is deliberately conservative (it may include a following
            // `;`-separated command), which only over-triggers the fallback.
            if w == "variable"
                && words
                    .clone()
                    .any(|a| tcl_syntax::naming::is_qualified(a.as_bytes()))
            {
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
