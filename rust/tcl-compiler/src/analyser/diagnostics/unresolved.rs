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

use rustc_hash::FxHashSet;

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;

/// The "known command name" sets consulted by the W123 unresolved-command
/// pass: registry names enabled in the active dialect, the simple-name tails
/// of user procs / classes / aliases / rename targets / ensemble commands,
/// inline-stub names, and the deduplicated candidate list for "did you
/// mean…?" suggestions.
struct W123KnownNames {
    registry_names: HashSet<String>,
    proc_tail_names: HashSet<String>,
    class_tail_names: HashSet<String>,
    alias_names: HashSet<String>,
    rename_target_names: HashSet<String>,
    ensemble_cmds: HashSet<String>,
    stub_names: HashSet<String>,
    candidates: Vec<String>,
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
    /// - `cmd_name in registry_names` (built-in command).
    /// - `cmd_name` contains `::` (qualified — defer to
    ///   per-namespace logic, conservative skip).
    /// - `cmd_name` starts with `$` / `[` (interpolated /
    ///   substituted head — handled by W307 / W308).
    /// - User-defined proc tail or absolute name.
    /// - User-defined class tail or absolute name.
    /// - Command alias tail.
    /// - Static `rename OLD NEW` target tail.
    /// - Ensemble namespace tail.
    ///
    /// Idempotency: ``self.unresolved_commands_emitted`` guards
    /// against double-emission when ``analyse`` is called twice
    /// or the chunked entry runs both passes.
    ///
    /// **Not yet implemented:** ``has_dynamic_providers`` early-return;
    /// the CONSTSET-driven interpolation suppression for
    /// ``$``-bearing command names.
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
        // `unknown` proc) still suppress both, since resolution is then unknown.
        let emit_w123 = !self.disabled_diagnostics.contains("W123");

        // Conservative gate: if any ``package require`` was seen,
        // suppress W123 entirely.  The package may load arbitrary
        // commands at runtime that the analyser can't see.
        if !self.result.package_requires.is_empty() {
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

    /// Build the [`W123KnownNames`] sets consulted by the unresolved-command
    /// pass: registry names enabled in the active dialect, user proc / class /
    /// alias / ensemble simple-name tails, inline-stub names, and the
    /// suggestion candidate list.
    fn build_w123_known_names(&self, registry: &tcl_registry::CommandRegistry) -> W123KnownNames {
        // Only commands
        // *enabled in the active dialect* count as "known" for W123.  The
        // registry's `command_names()` returns every loaded spec —
        // including base tcl commands like `exec`/`glob` that `build_default`
        // loads but the active dialect (e.g. f5-irules) disables — so filter
        // by dialect support.  Without this, `exec`/`glob` under f5-irules
        // would draw W002 (disabled) but not the W123 (unknown-in-dialect)
        // that should also fire.  Base commands valid everywhere (`set`/`if`,
        // `dialects: None`) still pass `get_for_dialect`, so they are not
        // spuriously flagged.
        let active_dialect = tcl_registry::prelude::DialectSet::parse(&self.dialect)
            .unwrap_or(tcl_registry::prelude::DialectSet::ALL_TCL);
        // A registry command is "known" for W123 whenever the active dialect
        // enables it — including package-gated commands such as ``argparse`` or
        // the Tk widgets, which resolve under a Tcl version and are ambient in a
        // `wish` interpreter.  The *missing `package require`* case is reported
        // separately by W120 (see ``emit_missing_package_require_diagnostics``),
        // which carries an add-the-require code fix; firing W123 here as well
        // would double-report and would false-positive on ambient Tk widgets.
        let registry_names: HashSet<String> = registry
            .command_names()
            .filter(|name| registry.get_for_dialect(name, active_dialect).is_some())
            .map(str::to_string)
            .collect();
        // Inline ``# tcl-lsp: stub NAME ...``
        // declarations contribute to the candidate set and the
        // suppression set so users who declared a stub for a
        // command don't get spurious W123s.
        let stub_names: HashSet<String> = super::utils::scan_stub_command_names(&self.source);
        let proc_tail_names: HashSet<String> = self
            .result
            .all_procs
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let class_tail_names: HashSet<String> = self
            .result
            .all_classes
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        // An alias whose most recent action (by offset) in the file is a
        // deletion (`interp alias {} name {}`) is no longer a callable
        // command — `command_aliases` itself is never pruned on deletion
        // (a later re-declaration of the same name must still win, and this
        // whole-file set has no per-call-site position to gate against), so
        // check `deleted_commands` here directly. This mirrors the same-file
        // arity resolver's `fact_in_effect` convention for deleted aliases,
        // just at file-end granularity rather than per call site.
        let alias_names: HashSet<String> = self
            .result
            .command_aliases
            .keys()
            .filter(|qn| {
                self.deleted_commands
                    .get(qn.as_str())
                    .is_none_or(|&del_off| self.alias_offsets.get(qn.as_str()) > Some(&del_off))
            })
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        // A static `rename OLD NEW` binds `NEW` to whatever `OLD` denoted —
        // `NEW` is a callable command from the rename onward, so calls to it
        // must not draw W123 (the same-file arity resolver already validates
        // them against `OLD`'s signature via `renamed_commands`). Deletion is
        // order-gated exactly like `alias_names` above: a `NEW` whose most
        // recent action in the file is a deletion (`rename NEW {}`, or `NEW`
        // renamed away again) is no longer callable at file end.
        let rename_target_names: HashSet<String> = self
            .renamed_commands
            .keys()
            .filter(|qn| {
                self.deleted_commands
                    .get(qn.as_str())
                    .is_none_or(|&del_off| self.rename_offsets.get(qn.as_str()) > Some(&del_off))
            })
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let ensemble_cmds: HashSet<String> = self
            .ensemble_namespaces
            .iter()
            .filter_map(|ns| ns.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();

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
            proc_tail_names,
            class_tail_names,
            alias_names,
            rename_target_names,
            ensemble_cmds,
            stub_names,
            candidates,
        }
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
            if known.registry_names.contains(name) {
                continue;
            }
            if name.contains("::") {
                continue;
            }
            if name.starts_with('$') || name.starts_with('[') {
                continue;
            }
            if known.proc_tail_names.contains(name) {
                continue;
            }
            if known.class_tail_names.contains(name) {
                continue;
            }
            if known.alias_names.contains(name) {
                continue;
            }
            if known.rename_target_names.contains(name) {
                continue;
            }
            if known.ensemble_cmds.contains(name) {
                continue;
            }
            if known.stub_names.contains(name) {
                continue;
            }
            // User-declared extra commands (`tclLsp.extraCommands`) are known.
            if self.extra_commands.contains(name) {
                continue;
            }
            if let Some(info) = self.result.unknown_proc_info.as_ref()
                && info.dispatch_targets.contains(name)
            {
                continue;
            }
            // Absolute-form fallback — ``cmd`` may be defined as
            // ``::cmd`` in the global namespace.
            if self.result.all_procs.contains_key(&format!("::{name}")) {
                continue;
            }
            if self.result.all_classes.contains_key(&format!("::{name}")) {
                continue;
            }
            // A command bound by `CLASS create NAME` — later `NAME method`
            // dispatch is a real command call, not an unknown (issue #777).
            if self.result.created_instance_commands.contains(name) {
                continue;
            }
            // A bare head inside a scoped command environment (a
            // `report::defstyle` style script, …) resolves against that
            // environment's registry-declared command set — plus any sibling
            // definitions it exposes (#806).  Registry data drives the check;
            // no command name is matched here.
            if self.is_scoped_command_resolved(name, inv.range) {
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
            // ``&str`` references).
            let suggestions = crate::text::suggest_similar(
                name,
                candidate_strs.iter().copied(),
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
