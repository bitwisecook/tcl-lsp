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

//! Positioned unresolved-slot advice and package assistance diagnostics.
//!
//! W123 consumes the shared source owner's exact selected-slot presence. Its
//! suggestions may use catalogue metadata, but metadata never proves dispatch.
//! Default autoloading remains possible; custom or uncertain fallback handlers
//! suppress absence advice. Missing-package advice uses its separate surface.

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use rustc_hash::FxHashSet;
use tcl_registry::model::{BindingKnowledge, BindingTarget};

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;

/// Advisory suggestion candidates, independent of positioned command presence.
pub(crate) struct CommandExistenceOracle {
    known: KnownNameTiers,
}

struct KnownNameTiers {
    candidates: Vec<String>,
}

impl Analyser {
    /// W123 advises that the selected callable slot is unresolved at this
    /// source point. It does not predict failure of an autoload handler.
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
        // toggle. Each invocation retains its own positioned lookup evidence.
        let emit_w123 = !self.disabled_diagnostics.contains("W123");

        let oracle = self.command_existence_oracle(registry);
        self.emit_w123_for_invocations(&oracle, emit_w123);
    }

    /// Build suggestions only; presence comes from the shared positioned owner.
    pub(crate) fn command_existence_oracle(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> CommandExistenceOracle {
        CommandExistenceOracle {
            known: self.build_w123_known_names(registry),
        }
    }

    /// Names declared by inline or sidecar stubs.  The analysis setup
    /// assembles this shared, provenance-aware declaration surface, so this
    /// must not rescan just the source document and omit sidecar bundles.
    fn stub_command_names(&self) -> HashSet<String> {
        self.result
            .stub_commands
            .iter()
            .map(|stub| stub.name.clone())
            .collect()
    }

    /// Catalogue and lexical names are candidates for a reviewed spelling fix.
    /// They provide no command-presence, namespace or implementation authority.
    fn build_w123_known_names(&self, registry: &tcl_registry::CommandRegistry) -> KnownNameTiers {
        let mut candidates = registry
            .command_names()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        candidates.extend(self.result.all_procs.keys().cloned());
        candidates.extend(self.result.all_classes.keys().cloned());
        candidates.extend(self.result.command_aliases.keys().cloned());
        candidates.extend(self.renamed_commands.keys().cloned());
        candidates.extend(self.stub_command_names());
        candidates.extend(self.extra_commands.iter().cloned());
        candidates.sort_unstable();
        candidates.dedup();
        KnownNameTiers { candidates }
    }

    /// Diagnostic-only presence. Qualified names, implementation effects and
    /// lookup order come from the exact retained execution or future entry.
    #[must_use]
    pub(crate) fn command_binding_knowledge(
        &self,
        _oracle: &CommandExistenceOracle,
        name: &str,
        range: tcl_lexer::Span,
        lookup: crate::signature_scan::types::SignatureCommandLookup,
        is_mathfunc_call: bool,
        _resolution_candidates: &[String],
    ) -> BindingKnowledge {
        use crate::command_binding::SourceCommandSlotPresence as Presence;
        if name.starts_with('$') || name.starts_with('[') {
            return BindingKnowledge::Unknown;
        }
        // Explicit assistance declarations can suppress this advisory without
        // establishing a callable implementation or donating a registry key.
        if self.extra_commands.contains(name)
            || self
                .result
                .stub_commands
                .iter()
                .any(|stub| stub.name == name)
            || self.is_scoped_command_resolved(name, range)
        {
            return BindingKnowledge::Unknown;
        }
        let Some(offset) = lookup.offset(range) else {
            return BindingKnowledge::Unknown;
        };
        let presence = if is_mathfunc_call {
            self.head_identities
                .diagnostic_math_function_presence_at(name, offset)
        } else {
            match lookup {
                crate::signature_scan::types::SignatureCommandLookup::InvocationHead => {
                    self.head_identities.diagnostic_slot_presence_at(offset)
                }
                crate::signature_scan::types::SignatureCommandLookup::ConsumedName { .. } => self
                    .head_identities
                    .diagnostic_command_slot_presence_at(name, offset),
                crate::signature_scan::types::SignatureCommandLookup::DeferredReference
                | crate::signature_scan::types::SignatureCommandLookup::PossibleConsumedName {
                    ..
                } => Presence::Unknown,
            }
        };
        match presence {
            Presence::Present => BindingKnowledge::Must(BindingTarget::document(name)),
            Presence::Absent => BindingKnowledge::Absent,
            Presence::MayPresent | Presence::Unknown => BindingKnowledge::Unknown,
        }
    }

    /// Walk every recorded command invocation, record the ones the oracle
    /// proves `Absent` as call sites, and (when `emit_w123`) push a W123
    /// with a "did you mean…?" suggestion.  Restores
    /// `command_invocations` on exit.
    fn emit_w123_for_invocations(&mut self, oracle: &CommandExistenceOracle, emit_w123: bool) {
        let known = &oracle.known;
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
            // existence — reference identity and existence are orthogonal, so
            // the record never feeds W123.
            if inv.existence_probe {
                continue;
            }
            // Only diagnostic-purpose selected-slot absence feeds W123.
            if self.command_binding_knowledge(
                oracle,
                name,
                inv.range,
                inv.lookup,
                inv.is_mathfunc_call,
                &inv.resolution_candidates,
            ) != BindingKnowledge::Absent
            {
                continue;
            }

            // A missing selected slot can still be serviced by autoloading.
            // Cross-file assistance may supply a declaration for this site.
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
            let mut message = format!("Unresolved command '{name}' at this source point");
            let mut fixes: Vec<super::types::CodeFix> = Vec::new();
            if let Some(best) = suggestions.first() {
                use std::fmt::Write as _;
                let _ = write!(message, "; did you mean '{best}'?");
                fixes.push(super::types::CodeFix {
                    span: inv.range,
                    new_text: (*best).to_string(),
                    description: format!("Replace with '{best}'"),
                    // W123: an edit-distance guess at the intended command.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                });
            }
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W123,
                    inv.range,
                    message,
                    Severity::Hint,
                )
                .with_fixes(fixes),
            );
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
        let generation = self.analysis_context();
        if generation
            .context()
            .resolve_spec(registry, "package")
            .is_none()
        {
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
        // require Tk` — is layered on top by the LSP server, which
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
        // The document's own declarations, for the shadowing gate below —
        // the same fact table the arity path's suppression is built from, so
        // the two agree about what "this file defines that command" means.
        let declared = super::validity::UserResolutionFacts::build(self);
        let mut best: HashMap<&str, &crate::signature_scan::types::SignatureCommandInvocation> =
            HashMap::new();
        for inv in &self.result.command_invocations {
            // Dialect-aware, not the bare `registry.get` (which ignores
            // dialect entirely and would pick an arbitrary same-name spec —
            // e.g. `link`'s 8.6-`ooutil`-gated spec even under a 9.0+
            // dialect where the unconditional core spec is the one that's
            // actually visible). Matches
            // W120 queries package assistance independently of W123's
            // positioned command-slot advice.
            let Some(spec) = generation.context().resolve_spec(registry, &inv.name) else {
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
            // The document defines the command itself — a `proc`, a class, an
            // `interp alias`, a static `rename` target, an ensemble, or a
            // declared stub. Then the name resolves to *that*, and no
            // `package require` is missing however the registry happens to
            // spell the same word. The real corpus case is the package's own
            // implementation file: georgtree/argparse's `proc ::argparse
            // {args}` beside its own uses was told to `package require
            // argparse` — i.e. to require the very package it is.  The
            // sibling W113 false positive on the same declaration is gated by
            // `is_package_gated_non_ambient`.
            let candidates: Vec<String> = if inv.resolution_candidates.is_empty() {
                crate::naming::bareword_resolution_candidates("", &inv.name)
            } else {
                inv.resolution_candidates.clone()
            };
            let bare = inv.name.rsplit("::").next().unwrap_or(&inv.name);
            if declared.declares_any(&candidates, bare) {
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
            let spec = generation
                .context()
                .resolve_spec(registry, &inv.name)
                .expect("invocation selected only when registry-known");
            let pkg = spec
                .required_package
                .expect("invocation selected only when it requires a package");
            if imported.contains(pkg) {
                continue;
            }
            // A package the runtime ships ambiently (an F5 surface, an EDA
            // shell's own tool commands, or a package a loaded pack declared
            // with `ambient_package`) is part of the runtime — no
            // `package require` exists for it (§7.1 axis C).
            if generation.context().ambient_package(pkg) {
                continue;
            }
            let fix = super::types::CodeFix {
                span: tcl_lexer::Span::new(insert_offset, insert_offset),
                new_text: format!("package require {pkg}\n"),
                description: format!("Add 'package require {pkg}'"),
                // W120: a `package require` loads the package, running its
                // initialisation code and changing what commands exist.
                safety: crate::irules_checks::FixSafety::BehaviourHardening,
            };
            new_diags.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W120,
                    inv.range,
                    format!("\"{}\" requires `package require {pkg}`", inv.name),
                    Severity::Warning,
                )
                .with_fixes(vec![fix]),
            );
        }
        self.result.diagnostics.extend(new_diags);
    }

    /// H301 — a command used *above* the `package require` that provides it.
    ///
    /// On by default.  The semantic view is position-insensitive and stays
    /// that way: a `package require` anywhere in the file makes its commands
    /// available for the whole file, because Tcl only resolves a command
    /// name when the call actually runs, so
    ///
    /// ```tcl
    /// proc later {} { csv::join {a b} }
    /// package require csv
    /// ```
    ///
    /// is correct and must not be reported as broken. What this reports is
    /// the *reading* problem: top-down, the call appears before the thing
    /// that provides it. It is a hint, it carries no fix, and it never
    /// changes what is available.
    ///
    /// Disjoint from W120 by construction: W120 fires when the package is
    /// **not** required at all, this when it **is**.
    ///
    /// Silent when:
    /// * the dialect has no `package` command, or the file loads packages
    ///   dynamically — the same two gates W120 takes;
    /// * the package is ambient (part of the runtime, so no `package
    ///   require` exists for it at all);
    /// * the file `package provide`s the package — it is the package's own
    ///   implementation, and requiring yourself first is not a rule;
    /// * every `package require` for it is conditional — inside a guarded
    ///   branch there is no unconditional "before" to be after;
    /// * H301 is in `disabled_diagnostics`.
    ///
    /// One hint per package, not per command: a package with twenty
    /// commands used above its requirement is one ordering mistake with one
    /// edit behind it, and twenty hints would be twenty ways of saying so.
    pub fn emit_package_require_ordering_hints(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.disabled_diagnostics.contains("H301") {
            return;
        }
        let generation = self.analysis_context();
        if generation
            .context()
            .resolve_spec(registry, "package")
            .is_none()
            || self.result.has_dynamic_providers
        {
            return;
        }
        // The package's own implementation file requires nothing of itself.
        let provided: FxHashSet<&str> = self
            .result
            .package_provides
            .iter()
            .map(|pp| pp.name.as_str())
            .collect();

        // The earliest *unconditional* requirement per package. A
        // conditional one cannot anchor an ordering claim.
        let mut required_at: HashMap<&str, u32> = HashMap::new();
        for pr in &self.result.package_requires {
            if pr.conditional {
                continue;
            }
            required_at
                .entry(pr.name.as_str())
                .and_modify(|at| *at = (*at).min(pr.range.start()))
                .or_insert_with(|| pr.range.start());
        }
        if required_at.is_empty() {
            return;
        }
        let declared = super::validity::UserResolutionFacts::build(self);

        // The earliest offending invocation per package, and the command
        // name it was — the message names one command, because naming
        // twenty would not help.
        let mut earliest: HashMap<&str, (u32, tcl_lexer::Span, &str)> = HashMap::new();
        for inv in &self.result.command_invocations {
            let Some(spec) = generation.context().resolve_spec(registry, &inv.name) else {
                continue;
            };
            let Some(pkg) = spec.required_package else {
                continue;
            };
            if provided.contains(pkg) || generation.context().ambient_package(pkg) {
                continue;
            }
            let Some(&require_start) = required_at.get(pkg) else {
                continue;
            };
            if inv.range.start() >= require_start {
                continue;
            }
            // The same two suppressions W120 takes: a head a scoped command
            // environment resolved is not the package's command, and a
            // command this document defines resolves to that definition.
            if self.is_scoped_command_resolved(&inv.name, inv.range) {
                continue;
            }
            let candidates: Vec<String> = if inv.resolution_candidates.is_empty() {
                crate::naming::bareword_resolution_candidates("", &inv.name)
            } else {
                inv.resolution_candidates.clone()
            };
            let bare = inv.name.rsplit("::").next().unwrap_or(&inv.name);
            if declared.declares_any(&candidates, bare) {
                continue;
            }
            let row = (inv.range.start(), inv.range, inv.name.as_str());
            earliest
                .entry(pkg)
                .and_modify(|cur| {
                    if row.0 < cur.0 {
                        *cur = row;
                    }
                })
                .or_insert(row);
        }

        // Sorted so the emitted order does not depend on hash iteration.
        let mut rows: Vec<(&str, (u32, tcl_lexer::Span, &str))> = earliest.into_iter().collect();
        rows.sort_by_key(|(pkg, (start, _, _))| (*start, *pkg));
        let new_diags: Vec<super::types::Diagnostic> = rows
            .into_iter()
            .map(|(pkg, (_, range, name))| {
                crate::analyser::types::Diagnostic::new(
                    DiagCode::H301,
                    range,
                    format!(
                        "\"{name}\" is used above the `package require {pkg}` that provides it"
                    ),
                    Severity::Hint,
                )
            })
            .collect();
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

#[cfg(test)]
mod require_ordering_tests {
    use crate::analyser::state::Analyser;

    /// `(code, message)` for the two package-requirement codes only — H301
    /// and the W120 it must stay disjoint from.
    fn diags(source: &str) -> Vec<(String, String)> {
        Analyser::new()
            .analyse(source, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "H301" | "W120"))
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    fn count(source: &str, code: &str) -> usize {
        diags(source).iter().filter(|(c, _)| c == code).count()
    }

    fn slot_diags(source: &str) -> Vec<(String, String)> {
        Analyser::new()
            .analyse(source, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code.as_str() == "W123")
            .map(|diagnostic| (diagnostic.code.to_string(), diagnostic.message.clone()))
            .collect()
    }

    fn slot_count(source: &str, code: &str) -> usize {
        slot_diags(source)
            .iter()
            .filter(|(candidate, _)| candidate == code)
            .count()
    }

    #[test]
    fn unresolved_slot_advice_uses_current_namespace_and_exported_import() {
        assert_eq!(
            slot_count(
                "namespace eval unrelated {proc helper {} {return wrong}}; helper",
                "W123"
            ),
            1
        );
        assert_eq!(
            slot_count(
                "namespace eval n {proc helper {} {return right}; namespace export helper}; namespace import ::n::*; helper",
                "W123"
            ),
            0
        );
        assert_eq!(
            slot_count(
                "namespace eval n {proc helper {} {return wrong}}; namespace import ::n::*; helper",
                "W123"
            ),
            1
        );
        assert_eq!(
            slot_count(
                "namespace eval n {proc helper {} {return right}}; namespace eval call {namespace path ::n; helper}",
                "W123"
            ),
            0
        );
    }

    #[test]
    fn unresolved_slot_advice_respects_temporal_deletion_and_custom_fallback() {
        assert_eq!(slot_count("proc p {} {}; p; rename p {}; p", "W123"), 1);
        assert_eq!(
            slot_count("proc unknown args {return handled}; missing", "W123"),
            0
        );
        assert_eq!(slot_count("rename unknown {}; missing", "W123"), 1);
        let diagnostics = slot_diags("missing");
        assert!(
            diagnostics
                .iter()
                .any(|(code, text)| code == "W123" && text.contains("at this source point")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn consumed_command_names_use_the_post_argument_lookup_point() {
        use crate::signature_scan::types::SignatureCommandLookup;
        for source in [
            "rename missing moved",
            "rename missing {}",
            "info body missing",
        ] {
            assert_eq!(slot_count(source, "W123"), 1, "{source}");
        }
        assert_eq!(slot_count("proc p {} {}; rename p moved", "W123"), 0);
        assert_eq!(slot_count("proc p {} {}; rename p $destination", "W123"), 0);
        assert_eq!(
            slot_count("proc p {} {}; rename p [rename p {}]", "W123"),
            1
        );
        for source in [
            "info commands missing*",
            "namespace which -command missing",
            "interp alias {} later {} missing",
            "lsort -command missing {}",
        ] {
            assert_eq!(slot_count(source, "W123"), 0, "{source}");
        }
        let source = "proc p {} {}; rename p moved";
        let result = Analyser::new().analyse(source, "tcl8.6");
        let occurrence = result
            .command_invocations
            .iter()
            .find(|invocation| {
                invocation.name == "p"
                    && matches!(
                        invocation.lookup,
                        SignatureCommandLookup::ConsumedName { .. }
                    )
            })
            .unwrap();
        assert_eq!(
            occurrence.lookup,
            SignatureCommandLookup::ConsumedName {
                invocation_offset: u32::try_from(source.find("rename").unwrap()).unwrap(),
            }
        );
        assert!(occurrence.resolved_command_reference.is_some());
        assert!(!occurrence.lookup.is_execution_site());
    }

    #[test]
    fn possible_consumed_names_preserve_navigation_without_absence_or_rename_authority() {
        use crate::signature_scan::types::SignatureCommandLookup;
        let source = "if {$unknown} {rename info original; proc info args {return ordinary}}; info body missing";
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(slot_count(source, "W123"), 0);
        let reference = result
            .command_invocations
            .iter()
            .find(|reference| reference.name == "missing")
            .expect("possible native role retains navigation");
        assert!(matches!(
            reference.lookup,
            SignatureCommandLookup::PossibleConsumedName { .. }
        ));
        assert!(!reference.rename_safe);
        assert!(!reference.lookup.is_execution_site());
        assert_eq!(slot_count("info body missing", "W123"), 1);
    }

    #[test]
    fn a_command_above_its_require_is_hinted() {
        let src = "csv::join {a b}\npackage require csv\n";
        assert_eq!(count(src, "H301"), 1, "{:?}", diags(src));
        // The requirement is present, so the missing-require warning must not
        // also fire: the two are disjoint by construction.
        assert_eq!(count(src, "W120"), 0, "{:?}", diags(src));
    }

    #[test]
    fn a_command_below_its_require_is_silent() {
        let src = "package require csv\ncsv::join {a b}\n";
        assert_eq!(count(src, "H301"), 0, "{:?}", diags(src));
    }

    /// The semantic view is position-insensitive and stays that way: a call
    /// inside a proc body runs after the file has been sourced, so requiring
    /// at the bottom is correct Tcl. It still *reads* as out of order, which
    /// is the whole point of a hint — but W120 must not fire, because
    /// nothing is missing.
    #[test]
    fn a_deferred_call_above_its_require_is_a_hint_and_not_a_warning() {
        let src = "proc later {} { csv::join {a b} }\npackage require csv\n";
        assert_eq!(count(src, "W120"), 0, "{:?}", diags(src));
        assert_eq!(count(src, "H301"), 1, "{:?}", diags(src));
    }

    #[test]
    fn a_missing_require_stays_w120_and_is_not_also_hinted() {
        let src = "csv::join {a b}\n";
        assert_eq!(count(src, "W120"), 1, "{:?}", diags(src));
        assert_eq!(count(src, "H301"), 0, "{:?}", diags(src));
    }

    /// One ordering mistake, one hint — not one per command.
    #[test]
    fn many_commands_above_one_require_hint_once() {
        let src = "csv::join {a b}\ncsv::split x\ncsv::report x\npackage require csv\n";
        assert_eq!(count(src, "H301"), 1, "{:?}", diags(src));
    }

    /// The package's own implementation file requires nothing of itself.
    #[test]
    fn a_providing_file_is_silent() {
        let src = "csv::join {a b}\npackage provide csv 1.0\npackage require csv\n";
        assert_eq!(count(src, "H301"), 0, "{:?}", diags(src));
    }

    /// Inside a guarded branch there is no unconditional "before" to be
    /// after, so the ordering claim cannot be made.
    #[test]
    fn a_conditional_require_anchors_nothing() {
        let src = "csv::join {a b}\nif {$x} { package require csv }\n";
        assert_eq!(count(src, "H301"), 0, "{:?}", diags(src));
    }

    #[test]
    fn the_hint_is_off_when_disabled() {
        let src = "csv::join {a b}\npackage require csv\n";
        let out = Analyser::with_disabled_diagnostics(["H301".to_owned()].into_iter().collect())
            .analyse(src, "tcl8.6");
        assert!(
            !out.diagnostics.iter().any(|d| d.code.as_str() == "H301"),
            "{:?}",
            out.diagnostics
        );
    }
}
