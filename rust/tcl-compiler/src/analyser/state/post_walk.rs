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

//! The tail after the walk: built-in names, implied package requires, the diagnostic emitters and the canonical result order.

use super::Analyser;
use std::collections::HashMap;

impl Analyser {
    /// Resolve (and cache) the set of built-in command names that
    /// **exist** under the active dialect — the registry tier of the one
    /// `exists` oracle: every store name
    /// the resolved context actually provides
    /// ([`tcl_registry::model::ResolvedContext::resolve_spec`]), plus the
    /// measured iRules §4b interpreter-present extension
    /// ([`Self::extend_with_irules_interpreter_present_names`]).
    ///
    /// This is the same set the W123 unresolved-command pass resolves
    /// registry names against, so settlement
    /// (`finalise_invocation_resolutions`), constant-dispatch, W113, and
    /// W123 cannot disagree about which registry commands exist.  Reading
    /// the *unfiltered* store name set at settlement instead would believe in
    /// commands W123 does not.
    ///
    /// The name set is held on ``self.builtin_names`` for subsequent
    /// proc / class registrations to consult without rebuilding.
    /// The dialect resolves through the profile catalogue; unknown dialect
    /// names sink to the permissive fallback profile's registry.
    pub(crate) fn builtin_command_names(&mut self) -> &std::collections::HashSet<String> {
        if self.builtin_dialect != Some(self.profile.name) || self.builtin_names.is_none() {
            let generation = self.analysis_context();
            let registry = generation.commands();
            let mut names: std::collections::HashSet<String> = registry
                .command_names()
                .filter(|name| generation.context().resolve_spec(registry, name).is_some())
                .map(str::to_string)
                .collect();
            self.extend_with_irules_interpreter_present_names(&mut names);
            self.builtin_names = Some(names);
            self.builtin_dialect = Some(self.profile.name);
        }
        // Safe: ``builtin_names`` was just set if it was missing.
        self.builtin_names
            .as_ref()
            .expect("builtin_names populated above")
    }

    /// measurements §4b: under `f5-irules` the 15 **compiler-refused**
    /// builtins are present in TMM's interpreter — reachable via `eval` at
    /// runtime — so the oracle must not claim they do not exist (no
    /// "Unknown command" W123). Their literal-source load refusal is
    /// IRULE2004's job (a policy warning, emitted by the disabled-command
    /// pass). The 16 interpreter-absent commands stay unknown here: their
    /// absence is a language fact. No-op outside the iRules profile.
    pub(in crate::analyser) fn extend_with_irules_interpreter_present_names(
        &self,
        names: &mut std::collections::HashSet<String>,
    ) {
        if self.profile.is_irules() {
            names.extend(
                tcl_registry::irules_policy::IRULES_COMPILER_REFUSED
                    .iter()
                    .map(|name| (*name).to_string()),
            );
        }
    }

    /// Run the post-walk diagnostic-emission tail: the W123 / arity / missing-
    /// package / variable-usage / CFG-SSA / lexer-warning emitters, then the
    /// disabled-code filter and the dedupe + canonical-order pass.  Shared by
    /// `analyse` / `analyse_chunked` / `analyse_commands` so the emitter set and
    /// ordering stay identical across every entry point.
    /// Drop the conditional entries from
    /// [`AnalysisResult::destroyed_commands`], keeping only destructions
    /// written at **load level**.
    ///
    /// A `rename OLD {}` inside a proc/class body runs at *call* time, when
    /// and if that definition is invoked — the same conditionality
    /// [`AnalysisResult::offset_is_inside_any_definition_body`] already
    /// governs for a nested `proc` shadow — and the consumer of this table
    /// revokes an import alias on it, which would drop a
    /// genuinely-live command. Removal events abstain toward keeping the
    /// alias, so the conditional ones are simply not published.
    ///
    /// It also keeps the incremental per-item path byte-identical to the
    /// whole-file one: an isolated body's destruction reaches the shell only
    /// on one of the two paths, and this filter removes it from both.
    fn publish_load_level_destructions(&mut self) {
        let all = std::mem::take(&mut self.result.destroyed_commands);
        self.result.destroyed_commands = all
            .into_iter()
            .filter(|&(_, off)| !self.result.offset_is_inside_any_definition_body(off))
            .collect();
    }

    /// Record the packages a declared edge says the document's own `package
    /// require`s also load.
    ///
    /// A binary extension whose `Init` calls `Tcl_PkgRequire` — or links Tk
    /// through `Tk_InitStubs` — makes a package available with nothing in any
    /// Tcl source to say so, so no amount of workspace scanning can find it
    /// (the `pkgIndex.tcl` route [`transitive_available_packages`] follows
    /// reads the *Tcl* files an `ifneeded` script sources, and a `load`-only
    /// extension has none). It can only be declared, and both places it can
    /// be declared state the same edge:
    ///
    /// - `# tcl-lsp: package myExtension provides Tk` in the file
    ///   ([`Self::directive_provides`]), which travels with it; or
    /// - a `[packages.provides]` / `tclLsp.packages.provides` entry
    ///   ([`Self::package_provides`]), which states it once for the project.
    ///
    /// Each package the edges reach becomes an ordinary
    /// [`SignaturePackageRequire`](crate::signature_scan::types::SignaturePackageRequire)
    /// carrying the span of the `package require` that loaded it — the point
    /// at which it really does become available, so the order-sensitive H301
    /// stays right — plus a
    /// [`PackageRequireOrigin`](crate::signature_scan::types::PackageRequireOrigin)
    /// naming the loader, for the consumers that want what the source
    /// literally says.
    ///
    /// Called from the head of [`Self::run_diagnostic_emitters`], the one
    /// tail both the whole-file and the per-item walk reach with their
    /// `package_requires` already merged — so the two strategies cannot
    /// disagree about what a declaration made available.
    pub(super) fn expand_implied_package_requires(&mut self) {
        let implied = self.implied_package_requires();
        self.result.package_requires.extend(implied);
    }

    /// Every package the declared edges reach from this document's own
    /// `package require`s, minus everything it already requires, as
    /// ready-to-record synthetic requires.
    ///
    /// Breadth-first, so each package is anchored at the earliest require
    /// that reaches it and the result is independent of hash iteration order;
    /// each name is visited once, so a cyclic declaration terminates. The
    /// file's own directives are searched before the configured edges, and a
    /// package reached through either can carry the closure onward. Empty —
    /// the fast path for every document — whenever nothing is declared or
    /// nothing is required.
    pub(in super::super) fn implied_package_requires(
        &self,
    ) -> Vec<crate::signature_scan::types::SignaturePackageRequire> {
        use crate::signature_scan::types::{PackageRequireOrigin, SignaturePackageRequire};

        if (self.directive_provides.is_empty() && self.package_provides.is_empty())
            || self.result.package_requires.is_empty()
        {
            return Vec::new();
        }
        // The earliest offset each package is already available from — not a
        // visited *set*. Seeding with the source requires keeps a redundant
        // entry out, but comparing offsets keeps the earlier anchor when a
        // declared edge makes a package available *above* the explicit
        // require for it. A set would discard the edge's entry outright and
        // leave only the later span, and H301 takes the **minimum** start per
        // package (`unresolved.rs`), so the command between the two would be
        // reported as used-before-available — a hint that appears only
        // because the file *also* requires the package explicitly.
        let mut earliest: HashMap<String, u32> = HashMap::new();
        for pr in &self.result.package_requires {
            earliest
                .entry(pr.name.clone())
                .and_modify(|at| *at = (*at).min(pr.range.start()))
                .or_insert_with(|| pr.range.start());
        }
        // `(package name, span, conditional, control_flow)` — the anchor an
        // edge's target inherits from whatever made its source available.
        let mut queue: Vec<(String, tcl_lexer::Span, bool, bool)> = self
            .result
            .package_requires
            .iter()
            .map(|pr| (pr.name.clone(), pr.range, pr.conditional, pr.control_flow))
            .collect();
        let mut out: Vec<SignaturePackageRequire> = Vec::new();
        let mut head = 0;
        while head < queue.len() {
            let (name, range, conditional, control_flow) = queue[head].clone();
            head += 1;
            let edges = self
                .directive_provides
                .iter()
                .map(|edge| (edge, true))
                .chain(self.package_provides.iter().map(|edge| (edge, false)));
            for ((from, to), from_directive) in edges.filter(|((from, _), _)| *from == name) {
                for loaded in to {
                    // `<=` rather than `<` is what keeps a cyclic declaration
                    // terminating: re-reaching a package with the identical
                    // anchor stops. Every anchor is one of the finitely many
                    // require starts, and a name is only re-enqueued when its
                    // anchor strictly decreases.
                    let start = range.start();
                    if earliest.get(loaded).is_some_and(|&at| at <= start) {
                        continue;
                    }
                    earliest.insert(loaded.clone(), start);
                    let origin = if from_directive {
                        PackageRequireOrigin::Directive(from.clone())
                    } else {
                        PackageRequireOrigin::Provides(from.clone())
                    };
                    out.push(SignaturePackageRequire {
                        original_name: None,
                        original_requirements: Vec::new(),
                        name: loaded.clone(),
                        version: None,
                        requirements: Vec::new(),
                        exact: false,
                        range,
                        conditional,
                        control_flow,
                        origin,
                    });
                    queue.push((loaded.clone(), range, conditional, control_flow));
                }
            }
        }
        out
    }

    pub(in super::super) fn run_diagnostic_emitters(&mut self, source: &str) {
        use crate::analysis_control::{AnalysisCheckpoint, checkpoint};
        // A source shell interpreted nothing, so there is nothing to report on.
        if self.source_shell {
            return;
        }
        let phase_start = ((cfg!(debug_assertions)
            && tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some())
            || tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_WORKSPACE_SCAN").is_some())
        .then(std::time::Instant::now);
        let trace_phase = |stage: &str| {
            if let Some(start) = phase_start {
                eprintln!(
                    "DIAGNOSTIC_PHASE bytes={} stage={stage} ms={}",
                    source.len(),
                    start.elapsed().as_millis()
                );
            }
        };
        trace_phase("initial");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        // Declared package availability first: every emitter below that asks
        // "is this package available here?" — W120, H301, the W123 widening,
        // the Tk activation gate — reads `package_requires`, so the declared
        // entries have to be in it before any of them run.
        self.expand_implied_package_requires();
        // Whole-source security checks belong in the analyser result so direct
        // consumers receive the same verdict as the LSP and CLI adapters. The
        // pure producer is also reused by non-Tcl F5 document adapters. Like
        // every other finding it is emitted raw: an inline `# noqa` reaches
        // the policy step through `suppressed_lines`, which records the
        // suppression with its reason.
        self.result
            .diagnostics
            .extend(super::super::source_integrity::bidi_control_diagnostics(
                source,
            ));
        // Replay the `<ensemble> <subcommand>` call sites the shell pass met
        // before the deferred body that declares the ensemble was walked
        // — before `finalise_invocation_resolutions`, so
        // the replayed invocations go through exactly the same settlement
        // the walk-time ones do. A no-op on every other entry point.
        self.flush_pending_ensemble_subcommand_invocations();
        // Settle every invocation's `resolved_qualified_name` with Tcl's
        // existence-checked two-step rule now that the walk has recorded
        // every definition in the file (a local candidate defined later in
        // the file still wins; an absent one falls back to global).
        self.finalise_invocation_resolutions();
        self.retain_positioned_command_definitions();
        self.publish_load_level_destructions();
        trace_phase("invocations");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        // Attach every namespace-qualified occurrence to the cell it names,
        // now that the whole file's `namespace eval` bodies have been walked
        // — a qualified read can precede its declaring `namespace eval`
        // textually and still resolve at run time.
        self.attach_qualified_var_references();
        trace_phase("variable-references");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        let diag_registry = self.profile_registry();
        self.flush_disabled_command_diagnostics();
        self.emit_unresolved_command_diagnostics(&diag_registry);
        self.flush_w143_diagnostics();
        self.flush_w304_diagnostics();
        // Decide every version-gated option relationship *before* the arity
        // flush: a conflict the resolved floor has is promoted onto
        // `pending_arity`, so it goes through the same shadowing suppression
        // an ungated one does.
        self.flush_gated_option_conflicts();
        // Same reason, one axis over: a call whose command changed shape
        // across releases could not be judged during the walk, and whatever
        // verdict the floor produces still has to face the shadowing check.
        self.flush_gated_arity_calls();
        self.flush_gated_bare_ensembles();
        self.flush_arity_diagnostics();
        self.flush_ctor_arity_diagnostics();
        self.flush_next_arity_diagnostics();
        self.emit_missing_package_require_diagnostics(&diag_registry);
        // Q8's assistance half. Disjoint from W120 by construction — that
        // fires when the requirement is absent, this when it is merely late.
        self.emit_package_require_ordering_hints(&diag_registry);
        trace_phase("command-diagnostics");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        self.emit_variable_usage_diagnostics();
        trace_phase("variable-usage");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        #[cfg(test)]
        let before_cfg_ssa = self
            .cfg_ssa_capture_for_test
            .is_some()
            .then(|| self.result.diagnostics.clone());
        self.emit_cfg_ssa_diagnostics(source);
        #[cfg(test)]
        if let Some(before) = before_cfg_ssa {
            self.cfg_ssa_capture_for_test = Some(
                self.result
                    .diagnostics
                    .iter()
                    .filter(|diagnostic| !before.contains(diagnostic))
                    .cloned()
                    .collect(),
            );
        }
        trace_phase("cfg-ssa");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        self.flush_loop_terminations();
        self.flush_objdefine_abort_diagnostics();
        self.emit_lexer_warning_diagnostics();
        self.emit_w116_w117_stub_shadows();
        self.flush_widget_dispatch_diagnostics(&diag_registry);
        self.flush_tk_geometry_diagnostics();
        self.flush_version_gate_diagnostics();
        self.flush_dsl_gate_diagnostics();
        self.publish_recovered_tail_diagnostics();
        self.publish_concatenated_script_diagnostics();
        self.apply_disabled_diagnostics();
        self.dedupe_diagnostics();
        self.canonicalize_result_order();
        trace_phase("complete");
    }

    pub(in super::super) fn retain_original_callback_signature_lookups(&mut self, source: &str) {
        let retained = self.result.command_invocations.iter().map(|invocation| {
            let prefix = std::sync::Arc::clone(invocation.original_callback_prefix.as_ref()?);
            let offset = prefix.source_registration().map(|registration| registration.site().offset)
                .or_else(|| prefix.lookup().map(|lookup| lookup.site().offset))?;
            let lookup = crate::registry_invocation::source_structure::source_callback_procedure_target_at(source, &self.result, offset, &prefix)?;
            super::super::SourceCallbackSignatureLookup::from_original_lookup(&self.result, prefix, std::sync::Arc::new(lookup)).map(std::sync::Arc::new)
        }).collect::<Vec<_>>();
        for (invocation, lookup) in self.result.command_invocations.iter_mut().zip(retained) {
            invocation.original_callback_signature_lookup = lookup;
        }
    }

    /// Canonicalise the order of the walk-populated, order-sensitive result
    /// collections so the output is independent of *how* the walk was driven
    /// (whole-file vs per-item).  Generalises the diagnostic sort in
    /// `dedupe_diagnostics`: `command_invocations` and every `VarDef.references`
    /// list are sorted by source position.  These are set-like for their
    /// consumers (W123 already emitted; LSP references/rename filter by
    /// name/position and the client orders by position), so a canonical
    /// source order is behaviour-preserving and lets per-item analysis merge
    /// facts without reconstructing DFS order.
    fn canonicalize_result_order(&mut self) {
        self.result.command_invocations.sort_by(|a, b| {
            a.range
                .start()
                .cmp(&b.range.start())
                .then(a.range.end().cmp(&b.range.end()))
                .then_with(|| a.name.cmp(&b.name))
                .then(a.lookup.cmp(&b.lookup))
        });
        sort_scope_refs(&mut self.result.global_scope);
        for v in self.result.all_variables.values_mut() {
            v.references.sort_by_key(|s| (s.start(), s.end()));
        }
        // The remaining walk-populated record vecs are likewise set-like
        // (consumed by version inference / workspace index / regex checks),
        // so sort each by source position for walk-strategy independence.
        self.result
            .package_requires
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        self.result
            .package_provides
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        self.result
            .source_targets
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        self.result
            .namespace_imports
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        self.result
            .auto_path_entries
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        self.result
            .qualified_var_refs
            .sort_by_key(|r| (r.span.start(), r.span.end()));
        self.result
            .namespace_refs
            .sort_by_key(|r| (r.span.start(), r.span.end()));
        self.result
            .regex_patterns
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        // `proc_declaration_sites` / `class_body_spans` are consumed purely
        // positionally (cursor-containment `find`s; `enclosing_class_at`'s
        // narrowest-span tie-break), so source order is canonical for them
        // too — the whole-file DFS interleaves nested declaration sites where
        // the per-item shell+graft appends them per body.
        self.result.proc_declaration_sites.sort_by(|a, b| {
            a.1.start()
                .cmp(&b.1.start())
                .then(a.1.end().cmp(&b.1.end()))
                .then_with(|| a.0.cmp(&b.0))
        });
        self.result.class_body_spans.sort_by(|a, b| {
            a.1.start()
                .cmp(&b.1.start())
                .then(a.1.end().cmp(&b.1.end()))
                .then_with(|| a.0.cmp(&b.0))
        });
        // `scoped_command_regions` is resolved purely by position
        // (containing-region lookups), so source order is canonical here too.
        self.result.scoped_command_regions.sort_by(|a, b| {
            a.span
                .start()
                .cmp(&b.span.start())
                .then(a.span.end().cmp(&b.span.end()))
                .then_with(|| a.env.name.cmp(b.env.name))
        });
        // `unresolved_command_sites` is a set of call sites consumed
        // order-independently (the cross-file arity resolver collects tail names +
        // per-site ranges); sort by `(span, name)` so the whole-file DFS and the
        // per-item shell+graft walks record it in the same order.
        self.result.unresolved_command_sites.sort_by(|a, b| {
            a.0.start()
                .cmp(&b.0.start())
                .then(a.0.end().cmp(&b.0.end()))
                .then_with(|| a.1.cmp(&b.1))
        });
    }
}

/// Precompute newline byte offsets for ``source``.  The returned
/// vector is sorted ascending — the byte offset of each ``\n``
/// in source order.  Callers (notably
/// ``apply_preceding_noqa``) use ``slice::partition_point`` /
/// ``binary_search`` on this vector to convert a byte offset to
/// a 0-based line number in ``O(log N)`` instead of a per-call
/// linear scan.
/// Recursively sort every `VarDef.references` list in a scope subtree by span,
/// for [`Analyser::canonicalize_result_order`].
fn sort_scope_refs(scope: &mut super::super::types::Scope) {
    for v in scope.variables.values_mut() {
        v.references.sort_by_key(|s| (s.start(), s.end()));
    }
    for child in &mut scope.children {
        sort_scope_refs(child);
    }
}
