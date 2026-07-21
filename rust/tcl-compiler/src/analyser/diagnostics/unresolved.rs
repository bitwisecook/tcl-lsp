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

//! Unknown-command and missing-`package require` checks.
//!
//! [`Analyser::emit_unresolved_command_diagnostics`] flags command heads
//! that resolve to no known command, user procedure, imported ensemble, or
//! runtime-provided name (W123), after the cross-function walk has recorded
//! every invocation. [`Analyser::emit_missing_package_require_diagnostics`]
//! flags use of a command that a package provides without a matching
//! `package require` (W120) and offers an insertion fix at the computed
//! offset.

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;
use tcl_registry::ProfileQueries;

use rustc_hash::FxHashSet;

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;

/// The "known command name" sets consulted by the W123 unresolved-command
/// pass: registry names enabled in the active dialect, the simple-name tails
/// of user procs / classes / aliases / rename targets / ensemble commands,
/// inline-stub names, the tails of literal `namespace import` patterns, and
/// the deduplicated candidate list for "did you mean…?" suggestions.
struct W123KnownNames {
    registry_names: HashSet<String>,
    /// Per-tail proc definitions (qualified name, establishing offset) —
    /// a tail may match several qualified names (same simple name in
    /// different namespaces), each with its own deletion history, so
    /// resolution checks every one for a call-site-specific live match
    /// (issue #973) rather than a plain tail-membership test.
    proc_defs_by_tail: HashMap<String, Vec<(String, u32)>>,
    /// [`Self::proc_defs_by_tail`]'s twin for classes.
    class_defs_by_tail: HashMap<String, Vec<(String, u32)>>,
    /// [`Self::proc_defs_by_tail`]'s twin for `interp alias` targets
    /// (issue #1006 — previously a plain tail `HashSet`, checked only at
    /// file-end granularity via `fact_live_at_file_end`).
    alias_defs_by_tail: HashMap<String, Vec<(String, u32)>>,
    /// [`Self::proc_defs_by_tail`]'s twin for static `rename OLD NEW`
    /// targets (issue #1006, same history as `alias_defs_by_tail`).
    rename_defs_by_tail: HashMap<String, Vec<(String, u32)>>,
    ensemble_cmds: HashSet<String>,
    stub_names: HashSet<String>,
    /// Final `::`-segment of each literal (non-conjectured) `namespace
    /// import` pattern — glob text (`*` from `::acme::*`, `render_*` from
    /// `::acme::render_*`) or an exact name (`render_box`).  An unqualified
    /// call matching one of these resolves to the imported command.
    import_pattern_tails: Vec<String>,
    candidates: Vec<String>,
}

/// Group `(qualified_name, establishing_offset)` pairs by their
/// `::`-tail — shared by the proc and class def maps in
/// [`Analyser::build_w123_known_names`] (issue #973) and its siblings
/// (`var_command.rs`'s `build_w307_known_names` / interpolated-W123
/// resolution, issue #1010): a tail may match several qualified names
/// (the same simple name in different namespaces), each kept with its
/// own offset for a later per-call live check
/// ([`Analyser::fact_live_for_call`]). `pub(super)` (not private) so
/// those sibling passes reuse it rather than reimplementing the same
/// grouping loop.
pub(super) fn group_defs_by_tail<'a>(
    entries: impl Iterator<Item = (&'a String, u32)>,
) -> HashMap<String, Vec<(String, u32)>> {
    let mut map: HashMap<String, Vec<(String, u32)>> = HashMap::new();
    for (qn, off) in entries {
        if let Some((_, tail)) = qn.rsplit_once("::")
            && !tail.is_empty()
        {
            map.entry(tail.to_string())
                .or_default()
                .push((qn.clone(), off));
        }
    }
    map
}

impl Analyser {
    /// W123 — unknown / unresolved command head.
    ///
    /// Walks every command invocation recorded during the
    /// analyser walk and emits W123 ("Unknown command 'X'")
    /// when no matching definition is in scope.
    ///
    /// Resolution paths checked in order — first match
    /// suppresses W123:
    ///
    /// - `cmd_name in registry_names` (built-in command), unless the
    ///   built-in was renamed away / deleted earlier in the file.
    /// - The call is an `expr` math-function application (`sin($x)`,
    ///   `max($a, $b)`) whose name is a genuine built-in `::tcl::mathfunc`
    ///   function available under the dialect's `expr` grammar version,
    ///   unless that qualified name was renamed away / deleted earlier.
    /// - `cmd_name` contains `::` (qualified — defer to
    ///   per-namespace logic, conservative skip).
    /// - `cmd_name` starts with `$` / `[` (interpolated /
    ///   substituted head — handled by W307 / W308).
    /// - User-defined proc tail or absolute name.
    /// - User-defined class tail or absolute name.
    /// - Command alias tail.
    /// - Static `rename OLD NEW` target tail.
    /// - Ensemble namespace tail.
    /// - Tail of a literal `namespace import` pattern (glob-matched).
    ///
    /// Idempotency: ``self.unresolved_commands_emitted`` guards
    /// against double-emission when ``analyse`` is called twice
    /// or the chunked entry runs both passes.
    ///
    /// **Not yet implemented:** the CONSTSET-driven interpolation
    /// suppression for ``$``-bearing command names.
    pub fn emit_unresolved_command_diagnostics(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.unresolved_commands_emitted {
            return;
        }
        self.unresolved_commands_emitted = true;
        // The W123 *diagnostic* honours `disabled_diagnostics`, but the
        // unresolved-command *call sites* are recorded regardless (below), so a
        // cross-file consumer can run its arity check independently of the W123
        // toggle.  The knowability gates that follow (dynamic `package require` /
        // dynamic providers / `unknown` proc) still suppress both, since
        // resolution is then unknown.
        let emit_w123 = !self.disabled_diagnostics.contains("W123");

        // Conservative gate: if any ``package require`` was seen,
        // suppress W123 entirely.  The package may load arbitrary
        // commands at runtime that the analyser can't see.
        if !self.result.package_requires.is_empty() {
            return;
        }

        // Dynamic providers ⇒ unknowable command set ⇒ no W123 — the same
        // gate W120 applies (see
        // [`Self::emit_missing_package_require_diagnostics`]).  Set by
        // `load`, a dynamic `rename` / `package require` name, a dynamic
        // `namespace import` pattern, an `auto_path` mutation, and a
        // `namespace unknown` handler installation.
        if self.result.has_dynamic_providers {
            return;
        }

        // When the document defines a
        // user-level ``unknown`` proc with a *dynamic* dispatch
        // shape — chains the original handler, case-folds,
        // uses pattern (glob / regexp) dispatch, calls
        // ``exec``, or calls ``auto_load`` — the analyser can't
        // statically prove which commands are resolvable, so
        // suppress W123 entirely.  For the *non-dynamic* shape
        // (only explicit ``dispatch_targets`` listed), W123
        // still fires below; the per-invocation loop checks
        // ``dispatch_targets`` membership and lets unrelated
        // commands surface their warnings.  Empty-stub
        // ``unknown`` (``proc unknown {cmd args} {}``) resolves
        // nothing so we never hit this gate.
        if let Some(info) = self.result.unknown_proc_info.as_ref() {
            let is_dynamic = info.chains_original
                || info.case_insensitive
                || info.has_pattern_dispatch
                || info.has_exec
                || info.has_auto_load;
            if is_dynamic {
                return;
            }
        }

        let known = self.build_w123_known_names(registry);
        self.emit_w123_for_invocations(&known, emit_w123);
    }

    /// Whether `qualified`'s establishing fact — an `interp alias`
    /// (`alias_offsets`) or a `rename` target (`rename_offsets`), recorded
    /// at `fact_off` — is still live at file end: no `rename NAME {}` /
    /// `interp alias {} NAME {}` deletion of `qualified` itself has a
    /// *later* offset recorded in `deleted_commands` (issue #973: a
    /// rename/alias target that was later renamed away must not still
    /// count as known — calling it fails "invalid command name" in real
    /// Tcl, confirmed against tclsh 8.6.14).
    ///
    /// File-end granularity, not per-call-site — the alias / rename
    /// candidate sets this feeds have no call site to gate against, unlike
    /// [`Self::qualified_name_deleted_before`] (registry builtins) or
    /// [`Self::fact_live_for_call`] (procs / classes, which — unlike an
    /// alias or rename target — can have namesakes across namespaces, so
    /// resolution needs the specific qualified name each call resolves
    /// against, not just a bare tail). `deleted_commands` holds only the
    /// last-seen deletion offset per name, which — since the walk visits
    /// statements in source order — is always the most recent one, so a
    /// name re-established after its deletion (a fresh `rename` or
    /// `interp alias` under the same name) reads as live again.
    ///
    /// `pub(super)`: also reused by `var_command.rs`'s
    /// `compute_factory_object_ranges` (issue #1010), whose
    /// `is_object_returning_head` predicate classifies a bare command
    /// head with no specific call site in hand — the same file-end
    /// question, not [`Self::fact_live_for_call`]'s per-call one.
    pub(super) fn fact_live_at_file_end(&self, qualified: &str, fact_off: u32) -> bool {
        self.deleted_commands
            .get(qualified)
            .is_none_or(|&del_off| fact_off > del_off)
    }

    /// Whether `qualified`'s establishing fact — recorded at `fact_off` —
    /// is still in effect for a call at `call_off`. Unlike
    /// [`Self::fact_live_at_file_end`] (used for aliases / rename targets,
    /// whose candidate sets have no call site to gate against), a proc or
    /// class *definition* has one real fact per qualified name that a
    /// specific call resolves against, so this additionally applies the
    /// same call-site + conditional-body awareness
    /// [`Self::qualified_name_deleted_before`] already gives registry
    /// builtins (issue #973's "conditional deletion never triggered" and
    /// "call textually before a later deletion" cases — confirmed against
    /// tclsh 8.6.14 that both still resolve):
    ///
    /// - No recorded deletion, or the fact was re-established *after* the
    ///   last one (`fact_off` postdates `del_off`) — live.
    /// - A deletion recorded inside a proc/class/method body is
    ///   conditional — it executes only if that body is ever invoked,
    ///   which the textual load-order gate can't know — so it never
    ///   disqualifies.
    /// - Otherwise the deletion is unconditional (top level) and in
    ///   effect for every call inside *any* body (the whole file loads —
    ///   running every top-level statement, including the deletion —
    ///   before any body ever runs) and, for a top-level call, only once
    ///   the call's own textual position is after it.
    ///
    /// `pub(super)` (not private) so sibling passes over the same
    /// `command_invocations` question — `const_dispatch.rs`'s constant-
    /// `$cmd` settlement (issue #1009) — reuse this rather than
    /// reimplementing it.
    ///
    /// A call *inside* a body carries no execution-order meaning from its
    /// own textual position — it runs whenever the enclosing definition is
    /// invoked, not when its text was written — so a call there falls back
    /// to a narrower, still-sound question: does [`Self::top_level_call_offsets`]
    /// record a *top-level* invocation of the innermost enclosing
    /// definition ([`super::super::types::AnalysisResult::enclosing_definition_qualified_name`])
    /// that provably ran before the deletion? If so, that invocation's own
    /// nested calls already resolved (issue #1009 Codex review: `proc
    /// helper {}`, `proc caller {} { helper }`, `caller`, `rename helper
    /// {}` resolves in real Tcl — confirmed against tclsh 8.6.14 — because
    /// `caller`'s own top-level call runs before the rename). Absent such
    /// proof (the enclosing definition is never called at the top level in
    /// this file, or only after the deletion), the existing conservative
    /// default holds: an unconditional top-level deletion is in effect for
    /// every call inside any body.
    pub(super) fn fact_live_for_call(&self, qualified: &str, fact_off: u32, call_off: u32) -> bool {
        let Some(&del_off) = self.deleted_commands.get(qualified) else {
            return true;
        };
        if del_off <= fact_off {
            return true;
        }
        if self.offset_is_inside_definition_body(del_off) {
            return true;
        }
        if !self.offset_is_inside_definition_body(call_off) {
            return call_off <= del_off;
        }
        self.result
            .enclosing_definition_qualified_name(call_off)
            .and_then(|qn| self.top_level_call_offsets.get(qn))
            .is_some_and(|&t| t < del_off)
    }

    /// The tail set for a "known command" map whose own fact is still
    /// live at file end — shared by `alias_names` and
    /// `rename_target_names` in [`Self::build_w123_known_names`] (both
    /// ask the identical [`Self::fact_live_at_file_end`] question, just
    /// against a different qualified-name / establishing-offset map).
    fn live_tail_names<'a>(
        &self,
        names: impl Iterator<Item = &'a String>,
        offsets: &HashMap<String, u32>,
    ) -> HashSet<String> {
        names
            .filter(|qn| {
                offsets
                    .get(qn.as_str())
                    .is_some_and(|&off| self.fact_live_at_file_end(qn, off))
            })
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Build the [`W123KnownNames`] sets consulted by the unresolved-command
    /// pass: registry names enabled in the active dialect, user proc / class /
    /// alias / ensemble simple-name tails, inline-stub names, and the
    /// suggestion candidate list.
    fn build_w123_known_names(&self, registry: &tcl_registry::CommandRegistry) -> W123KnownNames {
        // Only commands
        // *enabled in the active dialect profile* count as "known" for W123.
        // The registry's `command_names()` returns every loaded spec —
        // including base tcl commands like `exec`/`glob` that `build_default`
        // loads but the active dialect (e.g. f5-irules) disables — so filter
        // through the profile's availability query: the precise
        // (version|vendor) mask membership plus the subtractive iRules
        // disable list.  Without this, `exec`/`glob` under f5-irules would
        // draw W002 (disabled) but not the W123 (unknown-in-dialect) that
        // should also fire — and a vendor profile's embedded Tcl core (8.5
        // `dict` under f5-iapps, 8.6 `coroutine` under expect) would be
        // wrongly unknown, the confirmed bare-bit defect the profile fixes.
        let profile = tcl_dialect::DialectProfile::by_name(self.dialect());
        // A registry command is "known" for W123 whenever the active dialect
        // enables it — including package-gated commands such as ``argparse`` or
        // the Tk widgets, which resolve under a Tcl version and are ambient in a
        // `wish` interpreter.  The *missing `package require`* case is reported
        // separately by W120 (see ``emit_missing_package_require_diagnostics``),
        // which carries an add-the-require code fix; firing W123 here as well
        // would double-report and would false-positive on ambient Tk widgets.
        let registry_names: HashSet<String> = registry
            .command_names()
            .filter(|name| profile.resolve_command(registry, name).is_some())
            .map(str::to_string)
            .collect();
        // Inline ``# tcl-lsp: stub NAME ...``
        // declarations contribute to the candidate set and the
        // suppression set so users who declared a stub for a
        // command don't get spurious W123s.
        let stub_names: HashSet<String> = super::utils::scan_stub_command_names(&self.source);
        // These two tail sets feed only the "did you mean…?" candidate
        // list below — resolution itself uses `proc_defs_by_tail` /
        // `class_defs_by_tail` (built further down), which check each
        // matching qualified name's own deletion history per call site
        // (`fact_live_for_call`). Filtering by `fact_live_at_file_end`
        // here keeps a proc/class with no live definition anywhere in the
        // file (deleted, never re-established) from being suggested as a
        // fix for an unrelated typo.
        let proc_tail_names: HashSet<String> = self
            .result
            .all_procs
            .iter()
            .filter(|(qn, def)| self.fact_live_at_file_end(qn, def.name_span.start()))
            .filter_map(|(qn, _)| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let class_tail_names: HashSet<String> = self
            .result
            .all_classes
            .iter()
            .filter(|(qn, def)| self.fact_live_at_file_end(qn, def.name_span.start()))
            .filter_map(|(qn, _)| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        // Grouped by tail (unfiltered by deletion — the per-call live check
        // in `w123_invocation_resolves` does that, since a top-level call
        // textually before a later deletion, or a deletion recorded inside
        // a never-triggered proc/class body, must still resolve; see
        // `fact_live_for_call`). A tail may match several qualified names
        // (the same simple name in different namespaces), each tracked
        // with its own establishing offset.
        let proc_defs_by_tail = group_defs_by_tail(
            self.result
                .all_procs
                .iter()
                .map(|(qn, def)| (qn, def.name_span.start())),
        );
        let class_defs_by_tail = group_defs_by_tail(
            self.result
                .all_classes
                .iter()
                .map(|(qn, def)| (qn, def.name_span.start())),
        );
        // These two tail sets feed only the "did you mean…?" candidate list
        // below (same convention as `proc_tail_names` / `class_tail_names`
        // above) — resolution itself uses `alias_defs_by_tail` /
        // `rename_defs_by_tail` (built further down), issue #1006.
        let alias_names =
            self.live_tail_names(self.result.command_aliases.keys(), &self.alias_offsets);
        let rename_target_names =
            self.live_tail_names(self.renamed_commands.keys(), &self.rename_offsets);
        // Grouped by tail, unfiltered by deletion — same per-call live
        // check as `proc_defs_by_tail` / `class_defs_by_tail` (issue
        // #1006: an alias/rename-target call textually before a later
        // deletion, or a deletion recorded inside a never-triggered
        // proc/class body, must still resolve; previously these two were
        // plain tail `HashSet`s checked only via `fact_live_at_file_end`
        // — file-end granularity, no call site or conditional-body
        // awareness).
        let alias_defs_by_tail = group_defs_by_tail(
            self.result
                .command_aliases
                .keys()
                .filter_map(|qn| self.alias_offsets.get(qn).map(|&off| (qn, off))),
        );
        let rename_defs_by_tail = group_defs_by_tail(
            self.renamed_commands
                .keys()
                .filter_map(|qn| self.rename_offsets.get(qn).map(|&off| (qn, off))),
        );
        let ensemble_cmds: HashSet<String> = self
            .ensemble_namespaces
            .iter()
            .filter_map(|ns| ns.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        // Literal `namespace import` patterns make their matching source
        // commands callable by bare name — keep each pattern's final
        // `::`-segment for the per-invocation glob match (a `::acme::*`
        // import provides an unknowable subset of `::acme`, so its `*` tail
        // conservatively resolves every bare name; `::acme::render_*` only
        // names matching the glob; a non-glob import exactly that name).
        // Conjectured tcllib-wrapper imports (`X::import alias`) re-export
        // under the *alias* namespace — qualified names, which W123 already
        // skips — so they contribute no bare-name tails.
        let import_pattern_tails: Vec<String> = {
            let mut tails: Vec<String> = self
                .result
                .namespace_imports
                .iter()
                .filter(|imp| !imp.conjectured)
                .filter_map(|imp| imp.pattern.rsplit_once("::").map(|(_, t)| t.to_string()))
                .filter(|t| !t.is_empty())
                .collect();
            tails.sort_unstable();
            tails.dedup();
            tails
        };

        // Build the candidate set for "did you mean…?"
        // suggestions — every name a real command
        // could resolve to (including unknown-proc dispatch
        // targets and inline-stub declarations).
        let mut candidates: Vec<String> = Vec::new();
        candidates.extend(registry_names.iter().cloned());
        candidates.extend(proc_tail_names.iter().cloned());
        candidates.extend(class_tail_names.iter().cloned());
        candidates.extend(alias_names.iter().cloned());
        candidates.extend(rename_target_names.iter().cloned());
        candidates.extend(ensemble_cmds.iter().cloned());
        candidates.extend(stub_names.iter().cloned());
        // User-declared extra commands (`tclLsp.extraCommands`) are known.
        candidates.extend(self.extra_commands.iter().cloned());
        if let Some(info) = self.result.unknown_proc_info.as_ref() {
            for t in &info.dispatch_targets {
                candidates.push(t.clone());
            }
        }

        W123KnownNames {
            registry_names,
            proc_defs_by_tail,
            class_defs_by_tail,
            alias_defs_by_tail,
            rename_defs_by_tail,
            ensemble_cmds,
            stub_names,
            import_pattern_tails,
            candidates,
        }
    }

    /// Whether the registry built-in `name` has been renamed away or deleted
    /// (`rename puts myputs` / `rename puts {}` / an `interp alias`
    /// deletion) at a source offset before `call_off` — from that point the
    /// global name no longer denotes the built-in (confirmed against tclsh
    /// 9.0.4: calling it fails "invalid command name").
    ///
    /// The same top-level order-gating convention as the arity resolver's
    /// `fact_in_effect` / `fact_superseded_by_deletion`: `deleted_commands`
    /// holds the *last* deletion offset per name, so a call textually before
    /// it stays resolved by the registry.  A re-binding after the deletion
    /// (a fresh `proc puts …`, `rename … puts`, or `interp alias … puts`) is
    /// not checked here — the caller falls through to the proc / alias /
    /// rename-target sets, which already carry those (deletion-gated at
    /// file-end granularity, their existing convention).
    fn registry_name_deleted_before(&self, name: &str, call_off: u32) -> bool {
        // `handle_rename` / `handle_interp_alias` record deletions under the
        // normalised qualified name; an unqualified built-in lives at `::`.
        self.qualified_name_deleted_before(&format!("::{name}"), call_off)
    }

    /// [`Self::registry_name_deleted_before`] for a name that is already
    /// fully qualified (`::tcl::mathfunc::sin`) rather than an unqualified
    /// built-in living at the global `::`. Shared so the deletion-gating
    /// rule — a deletion recorded inside a proc/method/class body is
    /// conditional and never disqualifies — cannot drift between the two
    /// callers.
    fn qualified_name_deleted_before(&self, qualified: &str, call_off: u32) -> bool {
        let Some(&del_off) = self.deleted_commands.get(qualified) else {
            return false;
        };
        if del_off >= call_off {
            return false;
        }
        // A deletion recorded from inside a proc / method / class body is
        // *conditional* — it executes only if and when that procedure is
        // called, which the load-order textual gate can't know — so it
        // never disqualifies the built-in.  (This also keeps the per-item
        // path byte-identical: an isolated body's analyser state, including
        // its deletions, is not grafted back into the shell.)
        !self.offset_is_inside_definition_body(del_off)
    }

    /// Whether `name` is a genuine built-in `expr` math function (`sin`,
    /// `max`, …) for W123 purposes — i.e. a real name in
    /// [`tcl_syntax::expr::mathfunc`], the single shared function/version
    /// table the const-folder, the runtime, and
    /// [`Self::emit_expr_function_dialect_diagnostics`] (W002) already
    /// consult.
    ///
    /// Deliberately does **not** gate on the dialect's `expr`-grammar
    /// version ceiling the way W002 does: a function that predates the
    /// active dialect (`min(…)` under `tcl8.4`) is still a *real* name, just
    /// disabled here — exactly the same "known but disabled" split the
    /// generic registry-builtin path draws for a dialect-gated command like
    /// `dict` under `tcl8.4` (`build_w123_known_names`'s profile filter): W002
    /// explains why the call is disabled, and W123 fires alongside it since
    /// the name still doesn't dispatch to anything in *this* dialect. A
    /// dialect with no `expr` grammar base at all
    /// (`math_func_ceiling_for_dialect` returns `None`) applies no ceiling,
    /// matching W002's own "don't restrict" rule for that case.
    #[must_use]
    fn expr_mathfunc_name_known(&self, name: &str) -> bool {
        crate::tcl_expr_eval::is_known_mathfunc_in_dialect(name, self.dialect())
    }

    /// Whether this dialect exposes `::tcl::mathfunc::*` as literal,
    /// bareword-callable commands (TIP 232, Tcl 8.5+) — see
    /// [`crate::tcl_expr_eval::mathfunc_command_wrappers_available_in_dialect`].
    #[must_use]
    fn mathfunc_command_wrappers_available(&self) -> bool {
        crate::tcl_expr_eval::mathfunc_command_wrappers_available_in_dialect(self.dialect())
    }

    /// Whether byte offset `off` falls inside any recorded proc or class
    /// definition body — code there runs at *call* time, not load time.
    fn offset_is_inside_definition_body(&self, off: u32) -> bool {
        self.result.offset_is_inside_any_definition_body(off)
    }

    /// Whether the command head `name`, invoked at `range`, resolves through
    /// any W123 resolution path — first match wins (see
    /// [`Self::emit_unresolved_command_diagnostics`] for the ordered list).
    /// A head that resolves nowhere falls through to the diagnostic emitter.
    #[must_use]
    fn w123_invocation_resolves(
        &self,
        known: &W123KnownNames,
        name: &str,
        range: tcl_lexer::Span,
        resolved_qualified_name: Option<&str>,
        is_mathfunc_call: bool,
        resolution_candidates: &[String],
    ) -> bool {
        // A built-in renamed away / deleted at an earlier offset no longer
        // resolves here — fall through to the user-defined paths below,
        // which carry any later re-binding of the name (a fresh `proc` /
        // `rename … NAME` / `interp alias`).  Calls lexically before the
        // deletion stay resolved by the registry name.
        if known.registry_names.contains(name)
            && !self.registry_name_deleted_before(name, range.start())
        {
            return true;
        }
        // An `expr` math-function application (`sin($x)`, `max($a, $b)`) is
        // recorded by `record_expr_function_invocations` with the *bare*
        // function word as `name` and `::tcl::mathfunc::<name>` as the
        // settled qualified name — never a bareword registry entry, unlike
        // `tcl::mathop`'s `+`/`!`. A bare `sin`/`abs`/`round`/`bool`
        // registration would misdirect *every other* consumer of the bare
        // registry-name set (an unrelated `proc abs {x} {…}` would misread
        // as "renaming a builtin") — precisely the defect `tcl::mathop`
        // deliberately avoids by leaving `max`/`min` unregistered bare (see
        // `mathop_generated.rs`). So this checks the settled qualified name
        // directly against the single shared math-function name/version
        // table (`tcl_syntax::expr::mathfunc`) — the same table
        // `emit_expr_function_dialect_diagnostics` (W002) already consults —
        // rather than the registry's bare-name set, and is gated by the same
        // deletion rule as a registry builtin: `rename ::tcl::mathfunc::sin
        // {}` breaks `expr {sin(…)}` in C Tcl (confirmed by the WASM
        // runtime's `expr_routes_through_the_command_table` test), so a call
        // after that point falls through to the user-defined paths below.
        //
        // `is_mathfunc_call` gates which of two distinct facts applies. A
        // genuine `expr` function-call site is governed by expr-grammar
        // per-function availability alone (`expr_mathfunc_name_known`) —
        // `expr {sin(1)}` is valid under an 8.4-based dialect even though
        // TIP 232 (and the `::tcl::mathfunc` *command* namespace it
        // introduced) did not land until 8.5. An *ordinary* call that
        // merely happens to resolve to the same qualified shape (a bareword
        // `sin` invoked from inside a real `::tcl::mathfunc` namespace) has
        // no such exemption — it can only be that literal command, which
        // exists only where the wrapper mechanism itself does
        // (`mathfunc_command_wrappers_available_in_dialect`). Without this
        // split, an 8.4-based dialect would wrongly resolve the latter.
        if let Some(resolved) = resolved_qualified_name {
            let mathfunc_qualified = format!("::tcl::mathfunc::{name}");
            if resolved == mathfunc_qualified
                && self.expr_mathfunc_name_known(name)
                && (is_mathfunc_call || self.mathfunc_command_wrappers_available())
                && !self.qualified_name_deleted_before(&mathfunc_qualified, range.start())
            {
                return true;
            }
        }
        // A bare name resolved relative to the call's *enclosing lexical
        // namespace* (not the global bare name checked above) may name a
        // registry command whose only registered spelling is qualified —
        // e.g. `exists`/`get` called bare from inside `proc
        // ::tcl::dict::getnull {...}` resolve to the real, separately
        // -callable `::tcl::dict::exists` / `::tcl::dict::get` (issue #923
        // idx 105), not the ensemble-subcommand-only `dict exists` spec.
        // `resolution_candidates` already carries the correctly-qualified,
        // Tcl-priority-ordered candidate list for this exact call
        // (`finalise_invocation_resolutions` / `command_resolution_candidates`);
        // this reuses the same `registry_names` set already built above
        // rather than a second, namespace-blind lookup.
        if resolution_candidates
            .iter()
            .any(|cand| known.registry_names.contains(cand))
        {
            return true;
        }
        // Qualified names defer to per-namespace logic (conservative skip);
        // `$`-interpolated / `[…]`-substituted heads are W307 / W308's
        // domain.
        if name.contains("::") || name.starts_with('$') || name.starts_with('[') {
            return true;
        }
        // A tail may match several qualified names (the same simple name
        // in different namespaces) — resolve if *any* of them has a fact
        // still live for this specific call (issue #973 for procs/classes,
        // issue #1006 for aliases/rename targets: a name renamed or
        // deleted away, with no later re-establishment, must not resolve
        // here; `fact_live_for_call` also keeps a top-level call textually
        // before a later deletion, and a deletion recorded inside a
        // never-triggered proc/class body, correctly resolving).
        let tail_has_live_def = |defs_by_tail: &HashMap<String, Vec<(String, u32)>>| {
            defs_by_tail.get(name).is_some_and(|defs| {
                defs.iter().any(|(qualified, fact_off)| {
                    self.fact_live_for_call(qualified, *fact_off, range.start())
                })
            })
        };
        if tail_has_live_def(&known.proc_defs_by_tail)
            || tail_has_live_def(&known.class_defs_by_tail)
            || tail_has_live_def(&known.alias_defs_by_tail)
            || tail_has_live_def(&known.rename_defs_by_tail)
            || known.ensemble_cmds.contains(name)
            || known.stub_names.contains(name)
        {
            return true;
        }
        // A bare name matching the tail of a literal `namespace import`
        // pattern resolves to the imported command (`namespace import
        // ::acme::widgets::*` makes `render_box` callable unqualified).
        // Glob semantics via `tcl_syntax::glob::string_match`, so a
        // non-glob import suppresses exactly that name.
        if known
            .import_pattern_tails
            .iter()
            .any(|tail| tcl_syntax::glob::string_match(tail, name))
        {
            return true;
        }
        // User-declared extra commands (`tclLsp.extraCommands`) are known.
        if self.extra_commands.contains(name) {
            return true;
        }
        if let Some(info) = self.result.unknown_proc_info.as_ref()
            && info.dispatch_targets.contains(name)
        {
            return true;
        }
        // Absolute-form fallback — ``cmd`` may be defined as ``::cmd`` in
        // the global namespace. Same per-call deletion gate as the
        // proc/class tail check above (issue #973): a `::cmd` renamed or
        // deleted away, with no later re-establishment, must not resolve
        // here either.
        let absolute = format!("::{name}");
        if self.result.all_procs.get(&absolute).is_some_and(|def| {
            self.fact_live_for_call(&absolute, def.name_span.start(), range.start())
        }) || self.result.all_classes.get(&absolute).is_some_and(|def| {
            self.fact_live_for_call(&absolute, def.name_span.start(), range.start())
        }) {
            return true;
        }
        // A command bound by `CLASS create NAME` (or a registry
        // `defines_command_at` argument — `coroutine NAME cmd`, `interp
        // create NAME`) — later calls dispatch on a real command, not an
        // unknown (issue #777).
        if self.result.created_instance_commands.contains(name) {
            return true;
        }
        // A bare head inside a scoped command environment (a
        // `report::defstyle` style script, …) resolves against that
        // environment's registry-declared command set — plus any sibling
        // definitions it exposes (#806).  Registry data drives the check;
        // no command name is matched here.
        self.is_scoped_command_resolved(name, range)
    }

    /// Walk every recorded command invocation, record the unresolved ones as
    /// call sites, and (when `emit_w123`) push a W123 with a "did you mean…?"
    /// suggestion.  Restores `command_invocations` on exit.
    fn emit_w123_for_invocations(&mut self, known: &W123KnownNames, emit_w123: bool) {
        // Pre-compute the deduplicated ``Vec<&str>`` over the
        // candidate set once, instead of rebuilding it per
        // unresolved invocation.  ``candidates`` may carry
        // duplicates because each contributor (registry / proc
        // tails / class tails / aliases / ensemble cmds /
        // stubs / unknown-proc dispatch_targets) is unioned
        // independently — dedupe via a ``HashSet`` filter
        // while preserving stable iteration order.
        let mut seen_candidate_strs: FxHashSet<&str> = FxHashSet::default();
        let candidate_strs: Vec<&str> = known
            .candidates
            .iter()
            .map(String::as_str)
            .filter(|candidate| seen_candidate_strs.insert(*candidate))
            .collect();

        // Drain so the iteration loop can mutate
        // ``self.result.diagnostics`` freely; restore at the end
        // (matches the snapshot/restore round-trip contract).
        let invocations = std::mem::take(&mut self.result.command_invocations);
        for inv in &invocations {
            let name = &inv.name;
            // An existence probe (`namespace which -command NAME`, exact
            // `info commands NAME`) asserts nothing about the name's
            // existence — reference identity and existence are orthogonal
            // (issue #945 fault 9), so the record never feeds W123.
            if inv.existence_probe {
                continue;
            }
            if self.w123_invocation_resolves(
                known,
                name,
                inv.range,
                inv.resolved_qualified_name.as_deref(),
                inv.is_mathfunc_call,
                &inv.resolution_candidates,
            ) {
                continue;
            }

            // Unresolved.  Record the call site so a cross-file consumer can run
            // its arity check independently of the W123 toggle, then emit the W123
            // diagnostic unless it is disabled.
            self.result
                .unresolved_command_sites
                .push((inv.range, name.clone()));
            if !emit_w123 {
                continue;
            }

            // "Did you mean…?" suggestion via edit distance (max 1
            // suggestion, budget scaled to the name's length so a short
            // typo can't match an unrelated short command).
            // ``candidate_strs`` was
            // deduplicated above so every name in it is unique;
            // copying the slice per invocation is cheap (Vec of
            // ``&str`` references).  The name itself is excluded — a
            // renamed-away builtin is still in the registry candidate set,
            // and suggesting the very name that no longer resolves would be
            // a self-referential fix.
            let suggestions = crate::text::suggest_similar(
                name,
                candidate_strs
                    .iter()
                    .copied()
                    .filter(|candidate| *candidate != name.as_str()),
                1,
                crate::text::scaled_max_distance(name),
            );
            let mut message = format!("Unknown command '{name}'");
            let mut fixes: Vec<super::types::CodeFix> = Vec::new();
            if let Some(best) = suggestions.first() {
                use std::fmt::Write as _;
                let _ = write!(message, "; did you mean '{best}'?");
                fixes.push(super::types::CodeFix {
                    span: inv.range,
                    new_text: (*best).to_string(),
                    description: format!("Replace with '{best}'"),
                });
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W123,
                span: inv.range,
                message,
                severity: Severity::Hint,
                fixes,
            });
        }
        self.result.command_invocations = invocations;
    }

    /// Whether the bare command head `name`, invoked at `range`, resolves
    /// against a scoped command environment active at that position.
    ///
    /// A head is resolved when its call site falls inside a recorded
    /// [`ScopedBodyRegion`](super::super::types::ScopedBodyRegion) whose
    /// environment either lists `name` as one of its commands, exposes sibling
    /// definitions of that name (a previously-defined `report::defstyle`
    /// style), or accepts unknown heads outright.  Purely registry-data driven
    /// — the scoped command set lives on the definer's spec, never here.
    #[must_use]
    fn is_scoped_command_resolved(&self, name: &str, range: tcl_lexer::Span) -> bool {
        let offset = range.start();
        self.result.scoped_command_regions.iter().any(|region| {
            if !region.contains(offset) {
                return false;
            }
            let env = region.env;
            env.is_command(name)
                || env.allow_unknown_commands
                || (env.include_sibling_definitions
                    && self
                        .result
                        .scoped_sibling_defs
                        .get(env.name)
                        .is_some_and(|names| names.contains(name)))
        })
    }

    /// W120 — command used without a corresponding
    /// `package require`.
    ///
    /// For every command
    /// invocation whose registry spec carries a
    /// `required_package`, emit W120 (once per command name)
    /// unless that package is already imported (a
    /// `package require` / `package provide` in this file).
    /// Attaches a `CodeFix` that inserts
    /// `package require <pkg>` after the last existing
    /// `package require`, or at the top of the file.
    ///
    /// Gated off entirely when:
    /// * the dialect has no `package` command (iRules);
    /// * the file loads packages dynamically
    ///   (`has_dynamic_providers`) — the runtime set of
    ///   commands is then unknowable;
    /// * W120 is in `disabled_diagnostics`.
    pub fn emit_missing_package_require_diagnostics(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.disabled_diagnostics.contains("W120") {
            return;
        }
        // Dialects without a `package` command (e.g. iRules)
        // can't `package require`, so W120 never applies.
        if registry.get("package").is_none() {
            return;
        }
        // Dynamic providers ⇒ unknowable command set ⇒ no W120.
        if self.result.has_dynamic_providers {
            return;
        }

        // This is the **single-file** W120: it knows only the packages
        // required / provided *in this document*.  Workspace-level
        // refinement — resolving a `package require X` through the
        // project's `pkgIndex.tcl` files to learn what `X` (transitively)
        // pulls in, e.g. a wrapper package whose body does `package
        // require Tk` (#723) — is layered on top by the LSP server, which
        // owns the `tcl-lsp-core::package_resolver` package database and
        // the workspace/`auto_path` it was scanned from.  Keeping the
        // analyser single-file mirrors C Tcl, where the set of available
        // commands is only known after the `auto_path` is searched and the
        // `ifneeded` scripts run — knowledge the document text alone does
        // not carry.

        // Packages already available in this file: every
        // `package require` name plus every `package provide`
        // name (a file that provides a package needn't require
        // it).
        let mut imported: FxHashSet<&str> = FxHashSet::default();
        for pr in &self.result.package_requires {
            imported.insert(pr.name.as_str());
        }
        for pp in &self.result.package_provides {
            imported.insert(pp.name.as_str());
        }

        // Insertion point for the code fix: just after the last
        // `package require` line, else the top of the file.
        let insert_offset = self.package_require_insert_offset();

        // Emit once per command name, anchored at its **source-earliest**
        // invocation.  Selecting by position (rather than the first in
        // `command_invocations` iteration order) makes the result independent of
        // *how* the walk was driven — the whole-file DFS and the per-item
        // shell+graft order record invocations in different orders, but both
        // pick the same anchor here (the per-item path's `command_invocations`
        // is only sorted by `canonicalize_result_order`, which runs after this
        // emitter).  This keeps the result walk-strategy-independent, as the
        // tail already enforces for other order-sensitive collections.
        let mut best: HashMap<&str, &crate::signature_scan::types::SignatureCommandInvocation> =
            HashMap::new();
        for inv in &self.result.command_invocations {
            let Some(spec) = registry.get(&inv.name) else {
                continue;
            };
            if spec.required_package.is_none() {
                continue;
            }
            // A head resolved by a scoped command environment at its call
            // site is that environment's command, not the package-gated
            // registry command it happens to share a name with — `entry`
            // in a tclpkg manifest is the entry-point directive, never the
            // Tk widget, so no `package require Tk` is missing.
            if self.is_scoped_command_resolved(&inv.name, inv.range) {
                continue;
            }
            best.entry(inv.name.as_str())
                .and_modify(|cur| {
                    if (inv.range.start(), inv.range.end()) < (cur.range.start(), cur.range.end()) {
                        *cur = inv;
                    }
                })
                .or_insert(inv);
        }
        let mut new_diags: Vec<super::types::Diagnostic> = Vec::new();
        for inv in best.values() {
            let spec = registry
                .get(&inv.name)
                .expect("invocation selected only when registry-known");
            let pkg = spec
                .required_package
                .expect("invocation selected only when it requires a package");
            if imported.contains(pkg) {
                continue;
            }
            // A package the profile ships ambiently (an F5 surface, an EDA
            // shell's own tool commands) is part of the runtime — no
            // `package require` exists for it (§7.1 axis C).
            if self.profile.is_ambient_package(pkg) {
                continue;
            }
            let fix = super::types::CodeFix {
                span: tcl_lexer::Span::new(insert_offset, insert_offset),
                new_text: format!("package require {pkg}\n"),
                description: format!("Add 'package require {pkg}'"),
            };
            new_diags.push(super::types::Diagnostic {
                code: DiagCode::W120,
                span: inv.range,
                message: format!("\"{}\" requires `package require {pkg}`", inv.name),
                severity: Severity::Warning,
                fixes: vec![fix],
            });
        }
        self.result.diagnostics.extend(new_diags);
    }

    /// Byte offset at which a `package require <pkg>` line
    /// should be inserted: just past the newline after the
    /// last existing `package require`, else `0` (top of
    /// file).
    fn package_require_insert_offset(&self) -> u32 {
        let Some(last) = self
            .result
            .package_requires
            .iter()
            .max_by_key(|p| p.range.end())
        else {
            return 0;
        };
        let bytes = self.source.as_bytes();
        let mut off = last.range.end() as usize;
        while off < bytes.len() && bytes[off] != b'\n' {
            off += 1;
        }
        if off < bytes.len() {
            off += 1; // past the newline
        }
        u32::try_from(off).unwrap_or(0)
    }
}
