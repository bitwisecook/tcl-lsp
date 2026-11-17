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

//! Grafting an isolated body's fragment into the shell: the merged result and advice, the re-queued pending state and the replay of its global reads.

use super::super::state::Analyser;
use super::super::types::AnalysisResult;
use super::rebase::{rebase_fragment, shift};
use super::{BodyFragment, CapturedBodyGlobalRead, DeferredBody};

impl Analyser {
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
    /// Merge one grafted body's per-object records into the shell's result.
    /// Per-object methods accumulate per receiver name across bodies — the
    /// binding-identity consumer scopes them by objdefine site, so records
    /// from different procs coexist; the folded member state merges the same
    /// way (bindings from different bodies are distinct entries, keyed by
    /// their own anchor).
    fn merge_per_object_records(
        result: &mut AnalysisResult,
        object_methods: std::collections::HashMap<
            String,
            Vec<super::super::types::ObjectMethodDef>,
        >,
        object_member_state: std::collections::HashMap<
            String,
            Vec<super::super::types::ObjectMemberState>,
        >,
    ) {
        for (k, v) in object_methods {
            result.object_methods.entry(k).or_default().extend(v);
        }
        for (k, v) in object_member_state {
            result.object_member_state.entry(k).or_default().extend(v);
        }
    }

    pub(super) fn graft_proc_body(
        &mut self,
        db: &DeferredBody,
        mut frag: BodyFragment,
        shell_var_keys: &std::collections::HashSet<String>,
    ) {
        // Rebase: body facts are relative to the body content start.
        let delta = db.body_tok.span.start() + u32::from(db.body_tok.content_offset);
        let line_delta = self.line_offsets.as_deref().map_or(0, |lo| {
            super::super::state::line_at_offset(lo, delta as usize)
        });
        rebase_fragment(&mut frag, delta, line_delta);

        let proc_scope = std::mem::replace(
            &mut frag.proc_scope,
            super::super::types::Scope::new(super::super::types::ScopeKind::Global, ""),
        );
        if let Some(ps) =
            super::super::scope::scope_at_mut(&mut self.result.global_scope, &db.scope_path)
        {
            merge_scope_vars(&mut ps.variables, proc_scope.variables);
            for (key, span) in proc_scope.original_variable_declaration_spans {
                if ps
                    .variables
                    .get(&key)
                    .is_some_and(|variable| variable.definition_span == span)
                {
                    ps.original_variable_declaration_spans
                        .entry(key)
                        .or_insert(span);
                }
            }
            ps.procs = proc_scope.procs;
            ps.classes = proc_scope.classes;
            ps.children = proc_scope.children;
        }
        let r = std::mem::take(&mut frag.result);
        self.merge_fragment_result(r, shell_var_keys);
        self.graft_fragment_pending(db, frag, delta);
    }

    /// The [`Self::graft_proc_body`] half that merges a rebased fragment's
    /// `AnalysisResult` collections into the shell's.  Split out only to
    /// keep each function within the line budget.
    fn merge_fragment_result(
        &mut self,
        mut r: AnalysisResult,
        shell_var_keys: &std::collections::HashSet<String>,
    ) {
        self.result
            .all_procs
            .extend(std::mem::take(&mut r.all_procs));
        // Per qualified name, not a flat `extend`: a body that redefines a
        // proc it defines itself carries its own displaced definitions, and a
        // flat extend would replace rather than append to whatever the shell
        // already recorded for that name. (A body redefining a proc defined
        // *outside* it forces a full re-analyse above, so the shell and the
        // body never contend for the same key here.)
        for (qualified, defs) in std::mem::take(&mut r.superseded_procs) {
            self.result
                .superseded_procs
                .entry(qualified)
                .or_default()
                .extend(defs);
        }
        for (qualified, definitions) in std::mem::take(&mut r.superseded_classes) {
            self.result
                .superseded_classes
                .entry(qualified)
                .or_default()
                .extend(definitions);
        }
        for (qualified, class) in std::mem::take(&mut r.all_classes) {
            self.result.retain_class_declaration(qualified, class);
        }
        self.result
            .external_class_contributions
            .extend(std::mem::take(&mut r.external_class_contributions));
        self.merge_body_variables(std::mem::take(&mut r.all_variables), shell_var_keys);
        self.result
            .command_aliases
            .extend(std::mem::take(&mut r.command_aliases));
        self.result
            .alias_offsets
            .extend(std::mem::take(&mut r.alias_offsets));
        self.result
            .renamed_commands
            .extend(std::mem::take(&mut r.renamed_commands));
        self.result
            .rename_offsets
            .extend(std::mem::take(&mut r.rename_offsets));
        // Per-ensemble union, not a flat `.extend()` — the outer key is the
        // ensemble's own name, and a flat extend would replace one grafted
        // body's whole inner subcommand map instead of merging into it.
        for (ensemble, subs) in std::mem::take(&mut r.ensemble_subcommand_targets) {
            self.result
                .ensemble_subcommand_targets
                .entry(ensemble)
                .or_default()
                .extend(subs);
        }
        // A body may *create* an ensemble as well as call one, so the
        // `-prefixes 0` opt-out merges back on the same seam its subcommand
        // map does — otherwise an ensemble configured inside a body would
        // come back abbreviation-matching for every later reader.
        self.result
            .prefixless_ensembles
            .extend(std::mem::take(&mut r.prefixless_ensembles));
        Self::merge_per_object_records(
            &mut self.result,
            std::mem::take(&mut r.object_methods),
            std::mem::take(&mut r.object_member_state),
        );
        self.result
            .instance_classes
            .extend(std::mem::take(&mut r.instance_classes));
        // `object_handle_facts` is deliberately **not** merged here.  It has
        // exactly one producer — `emit_cfg_ssa_diagnostics_with_cu`
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
            .extend(std::mem::take(&mut r.created_instance_commands));
        self.result
            .ambiguous_instance_names
            .extend(std::mem::take(&mut r.ambiguous_instance_names));
        self.merge_fragment_source_shell_advice(&mut r);
        self.merge_fragment_original_advice(&mut r);
        self.result.qualified_var_refs.extend(r.qualified_var_refs);
        self.result.namespace_refs.extend(r.namespace_refs);
        self.result
            .namespace_name_unknowns
            .extend(r.namespace_name_unknowns);
        self.result.regex_patterns.extend(r.regex_patterns);
        self.result
            .namespace_overrides
            .extend(r.namespace_overrides);
        self.result
            .scoped_command_regions
            .extend(r.scoped_command_regions);
        // Per-environment union — the outer key is the scoped env's name,
        // and a flat extend would replace one grafted body's whole
        // sibling-name set instead of merging into it.
        for (env, names) in r.scoped_sibling_defs {
            self.result
                .scoped_sibling_defs
                .entry(env)
                .or_default()
                .extend(names);
        }
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
    }

    fn merge_fragment_source_shell_advice(&mut self, r: &mut AnalysisResult) {
        self.result
            .diagnostics
            .extend(std::mem::take(&mut r.diagnostics));
        self.result
            .command_invocations
            .extend(std::mem::take(&mut r.command_invocations));
        self.result
            .package_requires
            .extend(std::mem::take(&mut r.package_requires));
        self.result
            .package_provides
            .extend(std::mem::take(&mut r.package_provides));
        self.result
            .package_ifneededs
            .extend(std::mem::take(&mut r.package_ifneededs));
        self.result
            .package_prefer_latest
            .extend(std::mem::take(&mut r.package_prefer_latest));
        self.result
            .source_targets
            .extend(std::mem::take(&mut r.source_targets));
        self.result
            .namespace_imports
            .extend(std::mem::take(&mut r.namespace_imports));
        self.result
            .original_namespace_patterns
            .extend(std::mem::take(&mut r.original_namespace_patterns));
        self.result
            .namespace_exports
            .extend(std::mem::take(&mut r.namespace_exports));
        self.result
            .proc_declaration_sites
            .extend(std::mem::take(&mut r.proc_declaration_sites));
        self.result
            .class_body_spans
            .extend(std::mem::take(&mut r.class_body_spans));
        self.result
            .auto_path_entries
            .extend(std::mem::take(&mut r.auto_path_entries));
    }

    fn merge_fragment_original_advice(&mut self, r: &mut AnalysisResult) {
        self.result
            .original_vendor_procedure_metadata
            .extend(std::mem::take(&mut r.original_vendor_procedure_metadata));
        self.result
            .original_vendor_class_metadata
            .extend(std::mem::take(&mut r.original_vendor_class_metadata));
        self.result
            .original_vendor_symbol_metadata
            .extend(std::mem::take(&mut r.original_vendor_symbol_metadata));
        self.result
            .original_symbol_metadata
            .extend(std::mem::take(&mut r.original_symbol_metadata));
        // The merges below can update inventory rows in place.
        self.result.source_words_memo.clear();
        self.result
            .original_scoped_bodies
            .extend(std::mem::take(&mut r.original_scoped_bodies));
        for (offset, metadata) in std::mem::take(&mut r.original_conditional_registry_metadata) {
            match self
                .result
                .original_conditional_registry_metadata
                .entry(offset)
            {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(metadata);
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    if entry.get() != &metadata {
                        entry.insert(None);
                    }
                }
            }
        }
        for (span, producer) in std::mem::take(&mut r.original_vendor_source_names) {
            match self.result.original_vendor_source_names.entry(span) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(producer);
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    let combined =
                        entry.get().as_ref().zip(producer.as_ref()).and_then(
                            |(existing, incoming)| existing.merge_source_origin(incoming),
                        );
                    entry.insert(combined);
                }
            }
        }
        self.result
            .original_variable_symbols
            .extend(std::mem::take(&mut r.original_variable_symbols));
        self.result
            .original_vendor_variable_advice
            .extend(std::mem::take(&mut r.original_vendor_variable_advice));
        self.result
            .original_vendor_variable_bodies
            .extend(std::mem::take(&mut r.original_vendor_variable_bodies));
        self.result
            .original_variable_write_advice
            .extend(std::mem::take(&mut r.original_variable_write_advice));
        self.result
            .original_variable_roots
            .extend(std::mem::take(&mut r.original_variable_roots));
        self.result
            .original_variable_alias_sites
            .extend(std::mem::take(&mut r.original_variable_alias_sites));
        self.result
            .original_variable_name_unknowns
            .extend(std::mem::take(&mut r.original_variable_name_unknowns));
        self.result
            .original_variable_alias_obligations
            .extend(std::mem::take(&mut r.original_variable_alias_obligations));
        self.result
            .original_variable_alias_operands
            .extend(std::mem::take(&mut r.original_variable_alias_operands));
        self.result
            .original_variable_alias_source_operands
            .extend(std::mem::take(
                &mut r.original_variable_alias_source_operands,
            ));
        self.result
            .original_variable_alias_receipts
            .extend(std::mem::take(&mut r.original_variable_alias_receipts));
        self.result
            .original_variable_symbol_conflicts
            .extend(std::mem::take(&mut r.original_variable_symbol_conflicts));
    }

    /// The [`Self::graft_proc_body`] half that re-queues the fragment's own
    /// pending state (deferred diagnostics, arity / dispatch sites, the
    /// analyser-side alias tables) onto the shell and replays the captured
    /// qualified reads.  Split out only to keep each function within the
    /// line budget.
    fn graft_fragment_pending(&mut self, db: &DeferredBody, frag: BodyFragment, delta: u32) {
        self.ensemble_namespaces.extend(frag.ensembles);
        self.pending_arity.extend(frag.pending_arity);
        self.pending_option_conflicts
            .extend(frag.pending_option_conflicts);
        self.pending_user_call_arity
            .extend(frag.pending_user_call_arity);
        self.pending_ctor_arity.extend(frag.pending_ctor_arity);
        self.pending_next_arity.extend(frag.pending_next_arity);
        self.var_command_sites.extend(frag.var_sites);
        self.cmd_command_sites.extend(frag.cmd_sites);
        // Held for finalisation after `replay_deferred_instances` — the shell
        // is always in the deferred pass here (a graft
        // only runs from within it), so `pending_bareword_dispatch_sites` is
        // always `Some`.
        if let Some(pending) = self.pending_bareword_dispatch_sites.as_mut() {
            pending.extend(frag.bareword_dispatch_sites);
        }
        self.widget_dispatch_sites.extend(frag.widget_sites);
        self.command_aliases.extend(frag.walk_aliases);
        self.renamed_commands.extend(frag.walk_renames);
        self.alias_offsets.extend(frag.walk_alias_offsets);
        self.rename_offsets.extend(frag.walk_rename_offsets);
        self.deleted_commands.extend(frag.walk_deleted);
        self.pending_const_dispatches.extend(frag.const_dispatches);
        self.pending_instance_class_sites
            .extend(frag.instance_class_sites);
        self.proven_sites.extend(frag.proven_sites);
        // Ensembles declared *inside* this body: the shell needs their
        // absolute recording offsets to apply the whole-file DFS visibility
        // rule when it replays the call sites it deferred.
        // `rebase_fragment_pending` has already shifted them.
        self.ensemble_record_offsets.extend(frag.ensemble_offsets);
        self.loop_candidates.extend(frag.loop_candidates);
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
        // Replay the body's global-scope *definitions* first: a body that
        // names a fixed cell (`upvar ::tk::FocusGrab($i)
        // data`) is the only place that cell comes to exist, and the reads
        // replayed below — plus the tail's
        // `attach_qualified_var_references` — resolve against the shell's
        // scope tree, which the fragment's own throwaway root never reaches.
        self.replay_body_global_defs(frag.global_defs);
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
        // Queue captured instance creations (offsets already rebased to
        // absolute) for the global post-graft replay
        // ([`Self::replay_deferred_instances`]), tagged with whether they
        // came from a method body (whose replay recording an instance still
        // forces the fallback).
        for (cmd, args, creation_ns, off) in frag.instances {
            self.deferred_instance_replays
                .push((off, db.is_method, cmd, args, creation_ns));
        }
    }

    /// Merge a grafted body's `all_variables` into the shell's, keyed by the
    /// shell-ownership rule [`Self::graft_proc_body`]'s doc comment
    /// describes.  Extracted to keep that function within the line budget.
    fn merge_body_variables(
        &mut self,
        body_vars: std::collections::HashMap<String, super::super::types::VarDef>,
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
    /// The reference is pushed onto the enclosing cell **directly** rather
    /// than through [`Self::record_var_read`]: the isolated body pass already
    /// recorded the occurrence's [`super::super::types::QualifiedVarRef`] (merged by
    /// the graft), so going through the full read path would record it a
    /// second time — a duplicate the whole-file walk never produces (its
    /// adjacent-duplicate collapse cannot catch the replay's non-adjacent
    /// twin).
    /// Replay a grafted body's **global-scope definitions** on the shell's
    /// real global scope, so a fixed cell an `upvar` `otherVar` word names
    /// (`upvar ::tk::FocusGrab($i) data`, `upvar #0 counter c`) exists in
    /// the document's scope tree exactly as the whole-file walk leaves it.
    ///
    /// Spans are already absolute ([`rebase_fragment_pending`] shifted
    /// them).  The replay goes through the real
    /// [`Analyser::define_var`](super::super::state::Analyser) — so a cell the
    /// shell already owns keeps its first definition span and gains this
    /// word as a reference, matching `analyse`'s own second-definition
    /// handling — under `structural_rebind`, because the isolated body pass
    /// already recorded this word's [`super::super::types::QualifiedVarRef`] (the
    /// graft merged it) and already emitted its W215; doing either again
    /// would double-count.
    fn replay_body_global_defs(
        &mut self,
        global_defs: Vec<(String, tcl_lexer::Token, tcl_lexer::Span)>,
    ) {
        if global_defs.is_empty() {
            return;
        }
        let was_structural = self.structural_rebind;
        self.structural_rebind = true;
        for (name, tok, span) in global_defs {
            self.define_var(&name, tok, &[], false, Some(span));
        }
        self.structural_rebind = was_structural;
    }

    fn replay_body_global_reads(
        &mut self,
        global_reads: Vec<CapturedBodyGlobalRead>,
        body_start: u32,
        delta: u32,
    ) {
        for read in global_reads {
            if let Some(var) = self.result.global_scope.variables.get_mut(&read.base_name)
                && var.definition_span.start() < body_start
            {
                var.push_reference(shift(read.span, delta));
                if let Some(element) = read.element {
                    var.array_indices.insert(element);
                }
            }
        }
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
    dst: &mut std::collections::HashMap<String, super::super::types::VarDef>,
    src: std::collections::HashMap<String, super::super::types::VarDef>,
) {
    for (k, v) in src {
        merge_one_var(dst, k, v);
    }
}

/// Merge one body-derived variable into a map: an existing key keeps its
/// definition span (the shell's real param span) and accumulates the body's
/// references / warn flag / array indices; a new key is inserted.
pub(super) fn merge_one_var(
    dst: &mut std::collections::HashMap<String, super::super::types::VarDef>,
    k: String,
    v: super::super::types::VarDef,
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
