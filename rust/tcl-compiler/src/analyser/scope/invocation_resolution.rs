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

//! The post-walk settlement of each invocation's resolved command: liveness of definitions at the call, reachable call offsets and ensemble targets.

use super::super::state::Analyser;
use super::super::types::{ClassDef, ProcDef};
use super::command_namespaces::implicit_command_namespace_path_at;
use std::collections::{HashMap, HashSet};
use tcl_lexer::Span;

/// Bundles the field-disjoint borrows [`Analyser::finalise_invocation_resolutions`]'s
/// call-site "is `qualified` known, for a call at `call_off`?" predicate
/// needs, and the small body/enclosing-definition/liveness helpers it's
/// built from — extracted so the loop that builds and consults it doesn't
/// blow that function's line budget. `A` is left generic over the
/// alias-map value type (only `contains_key` is used) so this doesn't need
/// to name `command_aliases`'s value type explicitly.
struct KnownPredicateCtx<'a, A> {
    builtins: &'a HashSet<String>,
    renamed_away: &'a HashSet<String>,
    procs: &'a HashMap<String, ProcDef>,
    classes: &'a HashMap<String, ClassDef>,
    aliases: &'a HashMap<String, A>,
    renames: &'a HashMap<String, String>,
    alias_offsets: &'a HashMap<String, u32>,
    rename_offsets: &'a HashMap<String, u32>,
    deleted_commands: &'a HashMap<String, u32>,
    reachable_call_offsets: &'a HashMap<String, u32>,
}

impl<A> KnownPredicateCtx<'_, A> {
    /// Whether byte offset `off` falls inside any proc/class body recorded
    /// so far — a local restatement of
    /// `AnalysisResult::offset_is_inside_any_definition_body` over the
    /// field-disjoint borrows above, since a method call on `result` (the
    /// whole struct) would conflict with the `&mut
    /// result.command_invocations` borrow the caller's loop holds.
    fn offset_inside_any_body(&self, off: u32) -> bool {
        self.procs
            .values()
            .any(|p| p.body_span.start() <= off && off < p.body_span.end())
            || self
                .classes
                .values()
                .any(|c| c.body_span.start() <= off && off < c.body_span.end())
    }

    /// The qualified name of the innermost proc/class body containing
    /// `off`, mirroring `AnalysisResult::enclosing_definition_qualified_name`
    /// for the same borrow-splitting reason as `offset_inside_any_body`.
    fn enclosing_definition(&self, off: u32) -> Option<&str> {
        self.procs
            .values()
            .map(|p| (p.qualified_name.as_str(), p.body_span))
            .chain(
                self.classes
                    .values()
                    .map(|c| (c.qualified_name.as_str(), c.body_span)),
            )
            .filter(|(_, span)| span.start() <= off && off < span.end())
            .min_by_key(|(_, span)| span.end() - span.start())
            .map(|(qn, _)| qn)
    }

    /// Whether `qualified`'s fact — a proc/class definition, a `rename`
    /// target, or an `interp alias` target, established at `fact_off` — is
    /// still live *for a call at `call_off`*: no `rename NAME {}` / `interp
    /// alias {} NAME {}` deletion of `qualified` itself has been recorded
    /// *after* that offset (a proc/class/rename/alias target
    /// that was later renamed away must not still count as known —
    /// calling it fails "invalid command name" in real Tcl, confirmed
    /// against tclsh 8.6.14). Mirrors `fact_superseded_by_deletion` in
    /// `diagnostics/validity.rs` (the arity resolver's answer to the same
    /// "most recent fact: live definition or deletion?" question) rather
    /// than re-deriving it: a name deleted and then re-established under
    /// the same name (a fresh `proc`, `rename`, or `interp alias`) is live
    /// again, so only a deletion that postdates this specific fact's own
    /// establishing offset disqualifies it. `deleted_commands` stores only
    /// the last-seen deletion offset per name, which — since the walk
    /// visits statements in source order — is always the most recent one.
    ///
    /// Call-site and conditional-body aware the same way
    /// [`crate::command_binding::SourceInvocationBinding::selected_slot_presence`] is (the W123 pass's answer to the
    /// identical question) — a namespaced local candidate must not lose to
    /// the global one just because *some later* deletion exists, when this
    /// specific call runs before it: `proc bar
    /// {}`, `namespace eval foo { proc bar {}; proc caller {} { bar } }`,
    /// `foo::caller`, `rename foo::bar {}` still resolves `bar` (called
    /// from `foo::caller`, before the rename) to `::foo::bar`, not the
    /// global `::bar` — confirmed against tclsh 8.6.14 — because
    /// `foo::caller`'s own top-level invocation already ran before the
    /// rename.
    ///
    /// The enclosing definition need not be called at the top level
    /// *itself* — [`Analyser::reachable_call_offsets`] answers the
    /// transitive question, so an arbitrarily deep chain of
    /// enclosing definitions bottoming out at a real top-level call counts,
    /// while a mutual-recursion cycle no top-level call enters does not.
    fn live_for_call(&self, qualified: &str, fact_off: u32, call_off: u32) -> bool {
        let Some(&del_off) = self.deleted_commands.get(qualified) else {
            return true;
        };
        if del_off <= fact_off {
            return true;
        }
        if self.offset_inside_any_body(del_off) {
            return true;
        }
        if !self.offset_inside_any_body(call_off) {
            return call_off <= del_off;
        }
        self.enclosing_definition(call_off)
            .and_then(|qn| self.reachable_call_offsets.get(qn))
            .is_some_and(|&t| t < del_off)
    }

    pub(super) fn known(&self, qualified: &str, call_off: u32) -> bool {
        self.user_definition_known(qualified, call_off)
            || (self.aliases.contains_key(qualified)
                && self
                    .alias_offsets
                    .get(qualified)
                    .is_none_or(|&off| self.live_for_call(qualified, off, call_off)))
            || (self.renames.contains_key(qualified)
                && self
                    .rename_offsets
                    .get(qualified)
                    .is_none_or(|&off| self.live_for_call(qualified, off, call_off)))
            || (self
                .builtins
                .contains(tcl_syntax::naming::unroot_rooted_key(qualified).unwrap_or(qualified))
                && !self.renamed_away.contains(qualified))
    }

    /// Whether this candidate is specifically a proc/class definition which
    /// is live for this call.  Kept separate from [`Self::known`] so indexed
    /// reference settlement can preserve a position-sensitive definition
    /// target without mistaking a builtin, alias, or rename target for that
    /// historical definition.
    fn user_definition_known(&self, qualified: &str, call_off: u32) -> bool {
        self.procs
            .get(qualified)
            .is_some_and(|p| self.live_for_call(qualified, p.name_span.start(), call_off))
            || self
                .classes
                .get(qualified)
                .is_some_and(|c| self.live_for_call(qualified, c.name_span.start(), call_off))
    }
}

/// Record the candidate which the local, position-sensitive command table
/// selected, including whether it is a navigable proc/class definition.
fn record_known_winner<A>(
    inv: &mut crate::signature_scan::types::SignatureCommandInvocation,
    resolved: &str,
    winner: &str,
    known_ctx: &KnownPredicateCtx<'_, A>,
    call_off: u32,
) {
    inv.resolved_user_definition = known_ctx.user_definition_known(winner, call_off);
    if winner != resolved {
        inv.resolved_qualified_name = Some(winner.to_owned());
    }
}

impl Analyser {
    /// Compute the scope-resolved qualified name for a command
    /// invocation at `scope_path`.
    ///
    /// Delegates the "what namespace does an unqualified call here
    /// resolve against" question to [`Self::command_resolution_namespace`]
    /// (the same rule [`command_resolution_namespace_at`] applies
    /// post-walk, so the two can't disagree), then picks the
    /// most-specific candidate from
    /// [`crate::naming::bareword_resolution_candidates`] — which already
    /// encodes Tcl's real "current namespace, then global" two-step rule
    /// for both bare names and relative names with embedded `::`
    /// (`inner::p` resolves against the current namespace before falling
    /// back to global — it is *not* unconditionally global-rooted).  A
    /// leading-`::` name is already absolute and comes back unchanged.
    ///
    /// Returned strings are intended for matching against
    /// [`super::super::types::ProcDef::qualified_name`] in references
    /// / document-highlight / rename providers.  The value is
    /// not authoritative for runtime dispatch — it's a
    /// candidate that lets call-site → declaration matching
    /// resolve relative call shapes.  During the walk this is the
    /// *local-first* candidate; once the whole file is walked,
    /// [`Self::finalise_invocation_resolutions`] applies Tcl's
    /// existence check and demotes the guess to the global
    /// candidate when the local one names nothing the file (or the
    /// registry) defines.
    #[must_use]
    pub fn resolve_command_qualified_name(&self, cmd_name: &str, scope_path: &[usize]) -> String {
        let ns = self.command_resolution_namespace(scope_path);
        crate::naming::bareword_resolution_candidates(&ns, cmd_name)
            .into_iter()
            .next()
            .unwrap_or_else(|| crate::naming::qualify("::", cmd_name))
    }

    /// Post-walk pass: settle each invocation's `resolved_qualified_name`
    /// with Tcl's real two-step existence rule.
    ///
    /// The walk stores the *local-first* candidate (`::ns::inner::p` for an
    /// `inner::p` call inside `::ns`), but Tcl only dispatches there when
    /// that command **exists** — otherwise it falls back to the global
    /// candidate (`::inner::p`).  Existence can't be tested mid-walk: a proc
    /// body's calls resolve at call time, so a local candidate defined
    /// *later in the file* still wins (confirmed against tclsh 8.6).  This
    /// runs after the walk, when `all_procs` / `all_classes` /
    /// `command_aliases` / `renamed_commands` are complete, and demotes the
    /// guess to the global candidate when the local one names nothing known
    /// — a user definition or a registry builtin (`puts` inside a namespace
    /// resolves to `::puts`, not `::ns::puts`).
    ///
    /// When *neither* candidate is known (a cross-file proc), the
    /// local-first guess is kept: same-file consumers
    /// (`graphs`, `minify`) find no same-file definition either way, and
    /// the cross-file reference matchers treat the field as a candidate,
    /// not ground truth.
    /// Registry builtin names renamed away (`rename puts ::a::p`) or
    /// deleted (`rename puts ""` / an `interp alias` deletion) anywhere in
    /// the file — no longer callable under their original name (C raises
    /// `invalid command name`), so the registry name must not count as
    /// existing. Gates only [`Self::finalise_invocation_resolutions`]'s
    /// registry-builtin clause; user procs/classes keep that function's
    /// whole-file semantics (a proc/class's own deletion is instead gated
    /// per qualified name, via `deleted_commands` directly).
    fn renamed_away_builtin_names(&self) -> std::collections::HashSet<String> {
        self.result
            .renamed_commands
            .values()
            .cloned()
            .chain(self.deleted_commands.keys().cloned())
            .collect()
    }

    /// Build [`Analyser::reachable_call_offsets`]: the earliest offset at
    /// which each resolved qualified name is *provably reached* by the
    /// file's own top-level execution.
    ///
    /// The base case is a call whose own site is not inside any proc/class
    /// body — that call runs where it is written. The recursive case is a
    /// call inside some definition `E`'s body: it runs whenever `E` runs, so
    /// it inherits `E`'s own earliest reachable offset
    /// (`proc helper {}`, `proc inner {} { helper }`, `proc outer {} { inner
    /// }`, `outer`, `rename helper {}` — tclsh8.6/9.0 run this clean,
    /// because `outer`'s top-level call reaches `helper` two bodies deep,
    /// before the rename).
    ///
    /// Computed as a monotone least-fixpoint over the call graph rather
    /// than a memoised recursion: offsets only ever decrease, so the loop
    /// terminates, and a mutual-recursion cycle with no top-level entry
    /// simply never acquires a value (`None` — unreachable) instead of
    /// recursing forever.
    ///
    /// Read by [`Self::finalise_invocation_resolutions`]'s `live_for_call`
    /// and by [`crate::command_binding::SourceInvocationBinding::selected_slot_presence`] as the "was this call's enclosing
    /// definition itself reached before the deletion" escape hatch.
    ///
    /// # A body edge may raise a callee's offset but never lower a base one
    ///
    /// A call site's *presence* in a body is not proof that the body reaches
    /// it. `proc a {} { if {0} { b } }` never calls `b`, so `a`'s own
    /// earliest offset says nothing about when `b` first runs — yet the
    /// unrestricted fixpoint would hand `b` (and everything `b` calls) `a`'s
    /// early offset, which then reads as "reached before the deletion" and
    /// silently withdraws a correct W123.
    ///
    /// Oracle (tclsh8.6): with `proc b {}
    /// { helper }`, `proc a {} { if {0} { b } }`, `a`, `rename helper {}`,
    /// `b` — the final `b` really does fail with `invalid command name
    /// "helper"`.
    ///
    /// The precise rule would drop edges inside regions a constant-branch
    /// analysis proves unreachable, but those facts do not exist yet here:
    /// SCCP runs later, over the IR this analyser's result feeds, so
    /// consulting it at edge-collection time would be circular. What is
    /// available is the base case itself, so the fixpoint keeps a
    /// **weaker, non-circular** rule:
    ///
    /// > a body edge may only ever *add* a callee offset, never *lower* a
    /// > base top-level one.
    ///
    /// A callee with its own top-level call site has an offset that is
    /// already a fact about real execution; a speculative path through some
    /// body may not undercut it. A callee with no top-level call site keeps
    /// the optimistic answer — the chain is the only evidence there is.
    ///
    /// This is deliberately approximate in the sound direction for the
    /// chained-call shapes above and deliberately optimistic elsewhere. It does not
    /// catch a dead edge to a callee that is never called at top level at
    /// all, and it can raise an offset for a *live* body edge whose callee
    /// also has a later top-level call.
    fn compute_reachable_call_offsets(&self) -> HashMap<String, u32> {
        let mut reachable: HashMap<String, u32> = HashMap::new();
        // Callees with a top-level (non-body) call site. Their offsets are
        // observations of real execution, so body edges may not lower them.
        let mut has_base_offset: HashSet<&str> = HashSet::new();
        // Call-graph edges: `enclosing definition -> callee`, for every
        // invocation whose own site sits inside a body.
        let mut body_edges: Vec<(&str, &str)> = Vec::new();
        for inv in &self.result.command_invocations {
            let Some(qualified) = inv.resolved_qualified_name.as_deref() else {
                continue;
            };
            let call_off = inv.range.start();
            if !self.result.offset_is_inside_any_definition_body(call_off) {
                has_base_offset.insert(qualified);
                reachable
                    .entry(qualified.to_string())
                    .and_modify(|off| *off = (*off).min(call_off))
                    .or_insert(call_off);
                continue;
            }
            if let Some(enclosing) = self.result.enclosing_definition_qualified_name(call_off) {
                body_edges.push((enclosing, qualified));
            }
        }
        loop {
            let mut changed = false;
            for &(enclosing, callee) in &body_edges {
                let Some(&reached) = reachable.get(enclosing) else {
                    continue;
                };
                match reachable.get_mut(callee) {
                    Some(off) if *off <= reached => {}
                    Some(_) if has_base_offset.contains(callee) => {}
                    Some(off) => {
                        *off = reached;
                        changed = true;
                    }
                    None => {
                        reachable.insert(callee.to_string(), reached);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        reachable
    }

    /// The implicit `namespace path` each recorded call site carries, indexed
    /// alongside `command_invocations`.
    ///
    /// Computed up front, before [`Self::finalise_invocation_resolutions`]
    /// borrows that vector mutably, because the lookup reads the scope tree
    /// living on the same `result`.
    fn implicit_invocation_namespace_paths(&self) -> Vec<&'static [&'static str]> {
        self.result
            .command_invocations
            .iter()
            .map(|inv| {
                implicit_command_namespace_path_at(&self.result.global_scope, inv.range.start())
            })
            .collect()
    }

    /// Preserve actual implementation origins independently of name assistance.
    /// A document-final declaration or lexical deletion gate cannot prove which
    /// implementation a reached call selected after evaluating its arguments.
    pub(in super::super) fn retain_positioned_command_definitions(&mut self) {
        let lexical_advice = self.result.allows_lexical_declaration_advice();
        let completed = self.result.original_command_world.clone();
        for invocation in &mut self.result.command_invocations {
            if invocation.is_mathfunc_call || invocation.callback_arity.is_some() {
                continue;
            }
            let Some(offset) = invocation.lookup.offset(invocation.range) else {
                continue;
            };
            let point = self.head_identities.invocation_at_source("", offset);
            let reference = match invocation.lookup {
                // The ensemble looks up its selected target, after argv.
                crate::signature_scan::types::SignatureCommandLookup::EnsembleDispatch {
                    ..
                } => invocation
                    .resolved_qualified_name
                    .as_deref()
                    .and_then(|target| point.command_reference(target)),
                // A unanimously consumed name is looked up by this consuming
                // command's handler, so the receipt is issued at its site.
                crate::signature_scan::types::SignatureCommandLookup::ConsumedName { .. } => {
                    point.command_reference(&invocation.name)
                }
                crate::signature_scan::types::SignatureCommandLookup::PossibleConsumedName {
                    ..
                } => point
                    .lookup_command_word(&invocation.name)
                    .command_reference(&invocation.name),
                crate::signature_scan::types::SignatureCommandLookup::InvocationHead
                    if point.original_receiver_namespace_commands().is_some() =>
                {
                    completed.as_ref().and_then(|world| {
                        point.original_receiver_command_reference_after_load(world)
                    })
                }
                crate::signature_scan::types::SignatureCommandLookup::InvocationHead => {
                    point.original_evaluated_command_reference().or_else(|| {
                        lexical_advice
                            .then(|| point.evaluated_command_reference())
                            .flatten()
                    })
                }
                crate::signature_scan::types::SignatureCommandLookup::EnsembleTarget { .. }
                | crate::signature_scan::types::SignatureCommandLookup::DeferredReference => None,
            };
            let Some(reference) = reference else {
                invocation.clear_positioned_reference();
                continue;
            };
            invocation.retain_reference(&reference);
        }
        self.retain_ensemble_target_references();
    }

    /// An ensemble target word names the command each dispatch to it looks
    /// up; it shares their receipt only when every such dispatch has one and
    /// all agree.
    fn retain_ensemble_target_references(&mut self) {
        let mut agreed: std::collections::HashMap<
            String,
            Option<crate::command_binding::SourceCommandReference>,
        > = std::collections::HashMap::new();
        for invocation in &self.result.command_invocations {
            if invocation.ensemble_dispatch.is_none() {
                continue;
            }
            let Some(target) = &invocation.resolved_qualified_name else {
                continue;
            };
            let reference = invocation.resolved_command_reference.as_ref();
            agreed
                .entry(target.clone())
                .and_modify(|known| {
                    if !known
                        .as_ref()
                        .zip(reference)
                        .is_some_and(|(known, reference)| known.selects_same_command(reference))
                    {
                        *known = None;
                    }
                })
                .or_insert_with(|| {
                    reference.map(
                        crate::command_binding::SourceCommandReference::without_invocation_site,
                    )
                });
        }
        for invocation in &mut self.result.command_invocations {
            if !matches!(
                invocation.lookup,
                crate::signature_scan::types::SignatureCommandLookup::EnsembleTarget { .. }
            ) {
                continue;
            }
            match invocation
                .resolved_qualified_name
                .as_ref()
                .and_then(|target| agreed.get(target))
            {
                Some(Some(reference)) => invocation.retain_reference(reference),
                _ => invocation.clear_positioned_reference(),
            }
        }
    }

    fn finalise_original_invocation_inputs(&mut self) {
        let config = self.file_lexer_config();
        let rules = self.word_rules();
        let policy = self.declaration_name_policy();
        for invocation in &mut self.result.command_invocations {
            if invocation.matches_by_receipt() {
                if invocation.original_name_input.is_none()
                    && let Some(policy) = policy
                {
                    invocation.original_name_input = ensemble_word_input(
                        &self.head_identities,
                        invocation,
                        config,
                        rules,
                        policy,
                    );
                }
                continue;
            }
            if invocation.is_mathfunc_call || invocation.callback_arity.is_some() {
                continue;
            }
            let Some(offset) = invocation.lookup.offset(invocation.range) else {
                continue;
            };
            let binding = self.head_identities.invocation_at_source("", offset);
            if invocation.original_name_input.is_none() && !invocation.indirect {
                let Some(policy) = policy else {
                    continue;
                };
                invocation.original_name_input = binding
                    .original_source_name_key_at_span(invocation.range, config, rules, policy)
                    .map(crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord);
            }
            if invocation.original_lookup.is_none() {
                let key = invocation.original_name_input.as_ref().and_then(
                    crate::signature_scan::scope::SignatureSourceNameInput::original_word_key,
                );
                // A head is looked up at its own invocation; a unanimously
                // consumed name operand at its consuming command, after argv.
                invocation.original_lookup = match invocation.lookup {
                    crate::signature_scan::types::SignatureCommandLookup::InvocationHead => {
                        key.and_then(|key| binding.original_static_command_lookup(key))
                    }
                    crate::signature_scan::types::SignatureCommandLookup::ConsumedName {
                        ..
                    } => key.and_then(|key| binding.original_consumed_command_lookup(key)),
                    crate::signature_scan::types::SignatureCommandLookup::PossibleConsumedName {
                        ..
                    }
                    | crate::signature_scan::types::SignatureCommandLookup::EnsembleDispatch {
                        ..
                    }
                    | crate::signature_scan::types::SignatureCommandLookup::EnsembleTarget {
                        ..
                    }
                    | crate::signature_scan::types::SignatureCommandLookup::DeferredReference => {
                        None
                    }
                };
            }
        }
    }

    pub(in super::super) fn finalise_invocation_resolutions(&mut self) {
        self.finalise_original_invocation_inputs();
        // Populate the per-dialect builtin-name cache before splitting field
        // borrows below (`builtin_command_names` needs `&mut self`).
        let _ = self.builtin_command_names();
        let Some(builtins) = self.builtin_names.as_ref() else {
            return;
        };
        // `&'static DialectProfile`, not a borrow of `self` — safe to read
        // after the field-splitting `result` borrow below, unlike an `&self`
        // method call (which is why the math-function existence check the
        // mathfunc branch uses is a free function, not `Analyser::profile`
        // plus an instance method).
        let dialect = self.profile;
        // Recorded `namespace path` declarations, cloned before borrowing
        // `result` mutably. Entries are passed as written — the shared
        // candidate builder roots a relative entry against the declaring
        // namespace (`namespace path inner` inside `::outer` means
        // `::outer::inner`, never `::inner` — tclsh-pinned; see
        // `command_resolution_candidates`).
        let paths = self.namespace_paths.clone();
        // Expose the recorded paths on the result so command-resolution
        // consumers (definition / hover / signature help) can honour a
        // `namespace path` the same way call-site settling does.
        self.result.namespace_paths.clone_from(&paths);
        // Earliest top-level (non-body) call-site offset per resolved
        // qualified name, from the invocations the walk has already
        // recorded — computed before the mutable loop below so
        // `live_for_call`'s body-call escape hatch can consult it, and cached
        // on `self` so the later W123 /
        // const-dispatch / variable-command passes' `Self::fact_live_for_call`
        // calls reuse the same map instead of rebuilding it.
        self.reachable_call_offsets = self.compute_reachable_call_offsets();
        let implicit_paths = self.implicit_invocation_namespace_paths();
        let renamed_away = self.renamed_away_builtin_names();
        let deleted_commands = &self.deleted_commands;
        let rename_offsets = &self.rename_offsets;
        let reachable_call_offsets = &self.reachable_call_offsets;
        let result = &mut self.result;
        let (procs, classes, aliases, renames) = (
            &result.all_procs,
            &result.all_classes,
            &result.command_aliases,
            &result.renamed_commands,
        );
        let alias_offsets = &result.alias_offsets;
        // See `KnownPredicateCtx` (module level, above `impl Analyser`) for
        // the call-site + conditional-body-aware "is `qualified` known?"
        // predicate this builds — extracted out of this function to stay
        // within the line budget.
        let known_ctx = KnownPredicateCtx {
            builtins,
            renamed_away: &renamed_away,
            procs,
            classes,
            aliases,
            renames,
            alias_offsets,
            rename_offsets,
            deleted_commands,
            reachable_call_offsets,
        };
        for (idx, inv) in result.command_invocations.iter_mut().enumerate() {
            let implicit_path = implicit_paths.get(idx).copied().unwrap_or(&[]);
            // Absolute names are exact — the sole candidate is the name itself.
            // `None` means a background scan that skipped the scope walk; leave
            // its (empty) candidate list untouched.
            if tcl_syntax::naming::is_absolute(&inv.name) {
                inv.resolution_candidates = vec![inv.name.clone()];
                inv.resolved_user_definition =
                    known_ctx.user_definition_known(&inv.name, inv.range.start());
                continue;
            }
            let Some(resolved) = inv.resolved_qualified_name.clone() else {
                continue;
            };
            // A math-function invocation's resolved name carries the fixed
            // `tcl::mathfunc` dispatch segment, never the calling namespace —
            // the generic one-hop `{ns}::{name}` strip just below assumes
            // `resolved` is exactly `{callingNamespace}::{name}`, which is
            // false here (it would misparse `::tcl::mathfunc::sin` as if
            // `::tcl::mathfunc` were the calling namespace, silently
            // mis-resolving to an unrelated global command that happens to
            // share the bare tail name — a same-file `proc sin {…}` would
            // hijack `expr {sin(x)}`). `is_mathfunc_call` — set once, at
            // record time, never re-derived by guessing at the resolved
            // string's shape — routes these through the real two-candidate
            // rule instead: the caller's own `tcl::mathfunc` (honouring its
            // `namespace path`, exactly like the VM's own
            // `resolve_command_fqn` does for every command lookup, math
            // functions included), else the global one — `known` plus, for
            // the global candidate only, genuine built-in membership (a
            // local slot is never a built-in; only the true global one
            // ever is).
            if inv.is_mathfunc_call {
                let global = tcl_registry::mathfunc::qualified_name(&inv.name);
                let ns: String = match resolved.strip_suffix(global.as_str()) {
                    Some("") => "::".to_owned(),
                    Some(prefix) => prefix.to_owned(),
                    // The walk always produces this shape for a mathfunc
                    // invocation; treat an unrecognised one conservatively,
                    // matching the generic path's own fallback below.
                    None => {
                        inv.resolution_candidates = vec![resolved];
                        continue;
                    }
                };
                let path: &[String] = paths.get(&ns).map_or(&[], Vec::as_slice);
                let rel = crate::naming::unroot_rooted_key(&global).unwrap_or(global.as_str());
                let candidates = crate::naming::command_resolution_candidates(&ns, path, rel);
                inv.resolution_candidates.clone_from(&candidates);
                let call_off = inv.range.start();
                let winner = candidates.into_iter().find(|c| {
                    known_ctx.known(c, call_off)
                        || (*c == global
                            && crate::tcl_expr_eval::is_known_mathfunc_in_dialect(
                                &inv.name, dialect,
                            ))
                });
                if let Some(w) = winner {
                    record_known_winner(inv, &resolved, &w, &known_ctx, call_off);
                }
                continue;
            }
            // The walk stored the local-first candidate (`{ns}::{name}`);
            // recover the call namespace and re-run the *shared* resolver
            // (`resolve_command_with`, the same rule the optimiser, the
            // uplevel inliner, and the VM dispatch on) now that `known` is
            // complete — with the namespace's recorded `namespace path`
            // (empty when none was declared), so a call the path would
            // catch settles to the path candidate, exactly as it
            // dispatches at run time.
            let written = crate::naming::canonical_written_command(&inv.name);
            let suffix = crate::naming::root_unrooted_key(&written);
            let ns: String = match resolved.strip_suffix(suffix.as_str()) {
                Some("") => "::".to_owned(),
                Some(prefix) => prefix.to_owned(),
                // Not the `{ns}::{name}` shape the walk produces.  A folded
                // dynamic head (`${ns}::setdef`) is exactly this case — the
                // written name is not the dispatched one — and the walk
                // already recorded its candidate list from the *folded* name,
                // so settle against that instead of discarding it (without
                // this the local-first guess
                // `::tk::tk::setdef` was never demoted to the global
                // `::tk::setdef` the call really reaches, and find-references
                // from the declaration missed the site).  An unusual spelling
                // with no recorded candidates falls back to the settled
                // name as the sole candidate.
                None => {
                    settle_prebuilt_candidates(inv, resolved, &known_ctx);
                    continue;
                }
            };
            // A frame carrying an *implicit* `namespace path` (a `TclOO`
            // member body's `::oo::Helpers`, registry data — see
            // [`implicit_command_namespace_path_at`]) searches that path
            // between its own current namespace and global.  That current
            // namespace is the object's (`::oo::ObjN`) and has no static
            // spelling, so the walk substitutes global for it — which, left
            // alone, would put the *global* candidate ahead of the path one
            // and invert Tcl's order (tclsh 8.6.16 / 9.0.4: with both
            // `::callback` and `::oo::Helpers::callback` defined, a bare
            // `callback` in a method body reaches the latter).  The
            // unspellable slot is therefore dropped and the implicit path
            // takes its place at the head of the search.
            let declared_path: &[String] = paths.get(&ns).map_or(&[], Vec::as_slice);
            let (search_ns, search_path): (&str, Vec<&str>) = match implicit_path.split_first() {
                Some((first, rest)) => (first, rest.to_vec()),
                None => (
                    ns.as_str(),
                    declared_path.iter().map(String::as_str).collect(),
                ),
            };
            // Record the full ordered candidate list so a cross-document
            // consumer can re-settle this call against a *workspace-wide*
            // existence check — the local-first guess below only sees this
            // file, so a call resolving (via `namespace path`) to a proc
            // defined in another file cannot settle correctly here.
            inv.resolution_candidates =
                crate::naming::command_resolution_candidates(search_ns, &search_path, &inv.name);
            let call_off = inv.range.start();
            let known_here = |c: &str| known_ctx.known(c, call_off);
            if let Some(winner) =
                crate::naming::resolve_command_with(search_ns, &search_path, &inv.name, &known_here)
            {
                record_known_winner(inv, &resolved, &winner, &known_ctx, call_off);
            }
            // No candidate known in this file: keep the local-first guess for
            // cross-file consumers (the reference matchers treat it, and the
            // candidate list above, as candidates rather than ground truth).
        }
        // The constant-`$cmd` dispatch sites are *not* settled here:
        // their value facts come from the compiler's flow-sensitive value
        // model, which needs the CFG/SSA `CompilationUnit` — see
        // `settle_const_dispatches` in the diagnostics phase.  A lexical
        // constant map is last-write-wins across `if`/loop joins and discards
        // the defining literal's writable span, so it cannot answer here.
    }
}

/// Settle an invocation whose candidate list was built at **walk** time
/// rather than by [`Analyser::finalise_invocation_resolutions`] — a folded
/// dynamic head, whose written name is not the dispatched one and so does
/// not carry the `{ns}::{name}` shape the namespace recovery needs.
///
/// The first known candidate wins, exactly as
/// [`crate::naming::resolve_command_with`] would.  With no recorded
/// candidates this keeps the pre-existing behaviour: the settled name is the
/// sole candidate.  Without the settlement the walk's local-first guess
/// `::tk::tk::setdef` is never demoted to the global `::tk::setdef` the call
/// really reaches, so find-references from the declaration misses the site
/// while go-to-definition finds it.
fn settle_prebuilt_candidates<A>(
    inv: &mut crate::signature_scan::types::SignatureCommandInvocation,
    resolved: String,
    known_ctx: &KnownPredicateCtx<'_, A>,
) {
    if inv.resolution_candidates.is_empty() {
        inv.resolution_candidates = vec![resolved];
        return;
    }
    let call_off = inv.range.start();
    let winner = inv
        .resolution_candidates
        .iter()
        .find(|c| known_ctx.known(c, call_off))
        .cloned();
    if let Some(winner) = winner {
        record_known_winner(inv, &resolved, &winner, known_ctx, call_off);
    }
}

/// The written name of an ensemble word, for edits: a dispatch word is a whole
/// word of the ensemble invocation; a target word is a list element (or the
/// head of a prefix element) of the configuring command's option value.
fn ensemble_word_input(
    realm: &crate::realm::CommandBindingRealm,
    invocation: &crate::signature_scan::types::SignatureCommandInvocation,
    config: tcl_lexer::LexerConfig,
    rules: tcl_syntax::word_rules::WordValueRules,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
    use crate::signature_scan::scope::SignatureSourceNameInput;
    use crate::signature_scan::types::SignatureCommandLookup;
    match invocation.lookup {
        SignatureCommandLookup::EnsembleDispatch { invocation_offset } => realm
            .invocation_at_source("", invocation_offset)
            .original_source_name_key_at_span(invocation.range, config, rules, policy)
            .map(SignatureSourceNameInput::OriginalWord),
        SignatureCommandLookup::EnsembleTarget {
            invocation_offset,
            container_start,
            container_end,
        } => {
            let mut current = realm
                .invocation_at_source("", invocation_offset)
                .original_source_name_key_at_span(
                    Span::new(container_start, container_end),
                    config,
                    rules,
                    policy,
                )
                .map(SignatureSourceNameInput::OriginalWord)?;
            let range = invocation.range;
            for _ in 0..2 {
                let (child, extent) = current
                    .original_list_elements_with_source_spans()?
                    .into_iter()
                    .find_map(|(child, extent)| {
                        extent
                            .filter(|extent| {
                                extent.start() <= range.start() && range.end() <= extent.end()
                            })
                            .map(|extent| (child, extent))
                    })?;
                if extent == range {
                    return Some(child);
                }
                current = child;
            }
            None
        }
        _ => None,
    }
}
