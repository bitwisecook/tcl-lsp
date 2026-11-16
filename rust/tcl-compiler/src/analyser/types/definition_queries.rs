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

//! `AnalysisResult` queries over definitions: ensembles, class factories and hierarchy, the declaration a call reaches and the body enclosing an offset.

use super::AnalysisResult;
use super::class_def::ClassDef;
use super::class_factories::ClassFactoryIndex;
use super::procedures::{EnsembleSubcommandTarget, ProcDef};
use tcl_lexer::Span;

impl AnalysisResult {
    /// Publish a class assistance record while retaining original declarations
    /// for navigation. Updating members of the same declaration does not mint
    /// a second declaration or a positioned execution fact.
    pub(crate) fn retain_class_declaration(&mut self, qualified: String, mut class: ClassDef) {
        if let Some(previous) = self.all_classes.get(&qualified) {
            class.source_name_ambiguous.observe_if(previous.source_name_ambiguous.is_observed()
                || matches!((&class.source_name, &previous.source_name), (Some(left), Some(right)) if left.slot() != right.slot() || left.policy() != right.policy()));
        }
        if let Some(previous) = self.all_classes.insert(qualified.clone(), class)
            && self.all_classes[&qualified].name_span != previous.name_span
        {
            self.superseded_classes
                .entry(qualified)
                .or_default()
                .push(previous);
        }
    }

    /// Does this document configure the ensemble resolving to `ensemble`
    /// with `namespace ensemble … -prefixes 0`?
    ///
    /// [`Self::prefixless_ensembles`] is keyed by the ensemble's resolved
    /// command name, which callers hold in either spelling depending on how
    /// they got it, so both are tried here rather than at each call site.
    #[must_use]
    pub fn ensemble_refuses_prefixes(&self, ensemble: &str) -> bool {
        self.prefixless_ensembles.contains(ensemble)
            || self
                .prefixless_ensembles
                .contains(&crate::naming::root_unrooted_key(
                    crate::naming::unroot_rooted_key(ensemble).unwrap_or(ensemble),
                ))
    }

    /// Resolve `sub` against the subcommands recorded for the ensemble
    /// command `ensemble`, the way the ensemble itself would
    /// (`NsEnsembleImplementationCmd`): an exact name first, then — unless
    /// the ensemble was configured `-prefixes 0` — a **unique** prefix.
    ///
    /// The match rule is not re-derived here: it is
    /// [`tcl_cmd_core::ensemble::resolve_subcommand`], the owner the engines
    /// dispatch through, so navigation follows exactly the subcommand real
    /// Tcl would run. An *ambiguous* prefix resolves to nothing and the
    /// caller keeps abstaining — the ensemble would error there, so there is
    /// no target to point at.
    ///
    /// `None` when the ensemble is unknown to this document, its `-map` was
    /// dynamic (nothing is ever recorded for it), or `sub` matches nothing.
    #[must_use]
    pub fn resolve_ensemble_subcommand(
        &self,
        ensemble: &str,
        sub: &str,
    ) -> Option<&EnsembleSubcommandTarget> {
        let subs = self.ensemble_subcommand_targets.get(ensemble)?;
        if let Some(entry) = subs.get(sub) {
            return Some(entry); // the exact-name probe C does first
        }
        // The recorded map is unordered; the scan needs a stable candidate
        // order to call a prefix unique, and C's own table is sorted.
        let mut names: Vec<&str> = subs.keys().map(String::as_str).collect();
        names.sort_unstable();
        let index = tcl_cmd_core::ensemble::resolve_subcommand(
            &names,
            sub.as_bytes(),
            !self.ensemble_refuses_prefixes(ensemble),
        )?;
        subs.get(names[index])
    }

    /// Every **class factory** this document declares, keyed by qualified
    /// name — the slice a host merges into the workspace factory index it
    /// feeds back through
    /// [`Analyser::with_workspace_class_factories`](super::super::Analyser::with_workspace_class_factories).
    ///
    /// A document that declares no user metaclass — nearly all of them —
    /// contributes an empty map, so the merged index stays empty and every
    /// consuming walk behaves exactly as it did before.
    #[must_use]
    pub fn class_factories(&self) -> ClassFactoryIndex {
        self.all_classes
            .iter()
            .filter_map(|(qname, class)| Some((qname.clone(), class.factory.clone()?)))
            .collect()
    }

    /// The [`ClassHierarchy`](super::super::class_hierarchy::ClassHierarchy) for
    /// this result's classes, built once and cached.  Prefer this over
    /// calling [`build_class_hierarchy`](super::super::class_hierarchy::build_class_hierarchy)
    /// directly so hover / completion / definition / rename /
    /// type-hierarchy requests share one MRO computation rather than
    /// rebuilding (and re-cloning `all_classes`) each time.
    #[must_use]
    pub fn class_hierarchy(&self) -> &super::super::class_hierarchy::ClassHierarchy {
        self.hierarchy_cache.0.get_or_init(|| {
            super::super::class_hierarchy::build_class_hierarchy(self.all_classes.clone())
        })
    }

    /// Whether this document's dialect keeps the TIP 278 namespace-scope
    /// global variable fallback (Tcl 8.x yes, 9.0+ no) — the registry's
    /// [`DialectProfile::namespace_var_global_fallback`](tcl_dialect::DialectProfile::namespace_var_global_fallback)
    /// applied to [`Self::dialect`]. An unresolved dialect answers `false`
    /// (the stricter 9.0 semantics).
    #[must_use]
    pub fn ns_var_global_fallback(&self) -> bool {
        self.resolved_profile()
            .or_else(|| tcl_dialect::DialectProfile::find(&self.dialect))
            .is_some_and(tcl_dialect::DialectProfile::namespace_var_global_fallback)
    }

    /// Whether byte offset `off` falls inside any recorded proc or class
    /// definition body — code there runs at *call* time, not load time, so
    /// anything it declares (a nested `proc`, a `rename`, an alias) exists
    /// only conditionally, when and if the enclosing definition is actually
    /// invoked. Shared by the analyser's own W123 gate
    /// (`registry_name_deleted_before`) and `tcl-lsp-core`'s call-target
    /// resolver (`resolve_called_proc`), so both agree on what "load-time"
    /// means for the same underlying question: a `proc ::set {...}` written
    /// inside another proc's body must not permanently outrank the real
    /// `set` builtin for every call site in the workspace, the same way a
    /// `rename ::set {}` written there must not permanently un-resolve it.
    #[must_use]
    pub fn offset_is_inside_any_definition_body(&self, off: u32) -> bool {
        self.all_procs
            .values()
            .map(|p| p.body_span)
            .chain(self.all_classes.values().map(|c| c.body_span))
            .any(|span| span.start() <= off && off < span.end())
    }

    /// Every declaration of `qualified` in this document, in source order —
    /// the definitions a later same-named `proc` displaced
    /// ([`Self::superseded_procs`]) followed by the winner
    /// ([`Self::all_procs`]).  One element for the overwhelmingly common
    /// single-declaration case.
    pub fn proc_declarations<'a>(
        &'a self,
        qualified: &str,
    ) -> impl DoubleEndedIterator<Item = &'a ProcDef> + 'a {
        self.superseded_procs
            .get(qualified)
            .map_or_default(Vec::as_slice)
            .iter()
            .chain(self.all_procs.get(qualified))
    }

    /// Match a retained implementation allocation to its original authored
    /// declaration. This is navigation only; no lexical-order or final-name
    /// reconstruction substitutes for the selected allocation. A materialised
    /// or loaded script has no editable declaration in this document.
    #[must_use]
    pub fn proc_for_definition(
        &self,
        definition: &crate::command_binding::SourceCommandDefinition,
        source: &str,
    ) -> Option<&ProcDef> {
        if definition.kind() != crate::command_binding::SourceCommandDefinitionKind::Procedure {
            return None;
        }
        let allocation = definition.allocation();
        if matches!(
            allocation.site.source.kind(),
            crate::command_binding::SourceOriginKind::InheritedPoint { .. }
        ) {
            if allocation.site.source.source_image().bytes() != source.as_bytes()
                || !self.matches_inherited_declaration_origin(&allocation.site.source)
            {
                return None;
            }
            let mut records = self
                .original_procedure_metadata
                .iter()
                .filter(|record| record.declaration_site() == &allocation.site);
            let first = records.next()?;
            return records
                .all(|record| record == first)
                .then_some(first.metadata());
        }
        if !matches!(allocation.site.source.kind(), crate::command_binding::SourceOriginKind::Authored(authored) if authored.bytes() == source.as_bytes())
        {
            return None;
        }
        let mut original = self
            .original_procedure_metadata
            .iter()
            .filter(|record| record.declaration_site() == &allocation.site);
        if let Some(first) = original.next() {
            return original
                .all(|record| record == first)
                .then_some(first.metadata());
        }
        let tokens = self.definition_tokens(definition, source)?;
        self.proc_declarations(&definition.allocation().command)
            .find(|declaration| tokens.contains(&declaration.name_span))
    }

    /// Navigate to the original authored class declaration matching a retained
    /// class receipt. Instance and procedure allocations cannot borrow a class
    /// record; overwritten or foreign records cannot supply its source span.
    #[must_use]
    pub fn class_for_definition(
        &self,
        definition: &crate::command_binding::SourceCommandDefinition,
        source: &str,
    ) -> Option<&ClassDef> {
        if definition.kind() != crate::command_binding::SourceCommandDefinitionKind::Class {
            return None;
        }
        let allocation = definition.allocation();
        if matches!(
            allocation.site.source.kind(),
            crate::command_binding::SourceOriginKind::InheritedPoint { .. }
        ) {
            if allocation.site.source.source_image().bytes() != source.as_bytes()
                || !self.matches_inherited_declaration_origin(&allocation.site.source)
            {
                return None;
            }
            let mut records = self
                .original_class_metadata
                .iter()
                .filter(|record| record.declaration_site() == &allocation.site);
            let first = records.next()?;
            return records
                .all(|record| record == first)
                .then_some(first.metadata());
        }
        if !matches!(allocation.site.source.kind(), crate::command_binding::SourceOriginKind::Authored(authored) if authored.bytes() == source.as_bytes())
        {
            return None;
        }
        let mut original = self
            .original_class_metadata
            .iter()
            .filter(|record| record.declaration_site() == &allocation.site);
        if let Some(first) = original.next() {
            return original
                .all(|record| record == first)
                .then_some(first.metadata());
        }
        let tokens = self.definition_tokens(definition, source)?;
        let qualified = &definition.allocation().command;
        self.superseded_classes
            .get(qualified)
            .into_iter()
            .flatten()
            .chain(self.all_classes.get(qualified))
            .find(|declaration| tokens.contains(&declaration.name_span))
    }

    fn matches_inherited_declaration_origin(
        &self,
        origin: &crate::command_binding::SourceOriginId,
    ) -> bool {
        let Some(input) = self.resolved_input.as_ref() else {
            return false;
        };
        self.matches_original_source_image(origin.source_image(), input.lexer_config())
            && self.retained_command_realm().is_some_and(|realm| {
                realm
                    .source_bindings_ref()
                    .matches_inherited_source_origin(origin)
            })
    }

    pub(super) fn definition_tokens(
        &self,
        definition: &crate::command_binding::SourceCommandDefinition,
        source: &str,
    ) -> Option<Vec<Span>> {
        let allocation = definition.allocation();
        if !matches!(allocation.site.source.kind(),
            crate::command_binding::SourceOriginKind::Authored(authored) if authored.bytes() == source.as_bytes())
        {
            return None;
        }
        let config = self.body_lexer_config?;
        let start = usize::try_from(allocation.site.offset).ok()?;
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            source.get(start..)?,
            allocation.site.offset,
            tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
        );
        Some(
            commands
                .first()?
                .argv
                .iter()
                .map(|word| word.span)
                .collect(),
        )
    }

    /// The definition of `qualified` a call at `call_off` actually reaches.
    ///
    /// Order-gated the same way every other command-table fact is
    /// ([`crate::analyser::indirection::in_effect`], which shares the rule):
    /// at load time the latest declaration written before the call wins, and
    /// a call *inside a definition body* additionally sees every declaration
    /// written outside that body, wherever it sits in the file, because the
    /// whole file loads — running every top-level `proc` — before any body
    /// runs.
    ///
    /// That load-before-body shortcut stops at the body's own edge.  A `proc`
    /// written as a statement *of the body now executing* is an ordinary
    /// statement of the running script, so it is gated by offset like any
    /// other, and — having run later than everything the file's load
    /// installed — it outranks them all.  Oracle (tclsh 8.6.14 and 9.0.4) for
    /// `proc outer {} { proc p {} {return one}; p; proc p {a} {…} }`: the bare
    /// `p` between the two returns `one`, and `p X` after them dispatches the
    /// one-parameter definition, while the counterpart
    /// `proc outer {} { later }` / `proc later {} {…}` — declared outside,
    /// after — resolves fine (`LATER`), which is what the shortcut exists for.
    ///
    /// A call that precedes *every* declaration reaches none of them, but the
    /// winner is still returned so a consumer that has its own order gate
    /// (go-to-definition's lenient "show me the declaration anyway") behaves
    /// exactly as it did before redefinitions were tracked.
    ///
    /// Oracle (tclsh 8.6.14 and 9.0.4) for `proc p {} {return first}` / `p` /
    /// `proc p {a} {…}` / `p x`: the first call returns `first`, the second
    /// dispatches the one-parameter definition, and a bare `p` after the
    /// redefinition fails `wrong # args: should be "p a"`.
    #[must_use]
    pub fn proc_def_in_effect_at(&self, qualified: &str, call_off: u32) -> Option<&ProcDef> {
        let mut retained = self.command_invocations.iter().filter_map(|invocation| {
            (invocation.range.start() == call_off
                && (invocation.resolved_qualified_name.as_deref() == Some(qualified)
                    || invocation
                        .resolved_definition
                        .as_ref()
                        .is_some_and(|definition| definition.allocation().command == qualified)))
            .then_some(invocation.resolved_definition.as_ref())
            .flatten()
        });
        if let Some(definition) = retained.next() {
            if !retained.all(|alternative| alternative == definition) {
                return None;
            }
            let crate::command_binding::SourceOriginKind::Authored(source) =
                definition.allocation().site.source.kind()
            else {
                return None;
            };
            return self.proc_for_definition(definition, source.try_text().ok()?);
        }
        let winner = self.all_procs.get(qualified)?;
        let Some(earlier) = self.superseded_procs.get(qualified) else {
            return Some(winner);
        };
        let declarations = || earlier.iter().chain(std::iter::once(winner));
        let Some(body) = self.innermost_definition_body_span(call_off) else {
            // Load time: plain textual order.
            return declarations()
                .rfind(|def| def.name_span.start() < call_off)
                .or(Some(winner));
        };
        let in_this_body = |def: &&ProcDef| {
            let at = def.name_span.start();
            body.start() <= at && at < body.end()
        };
        // A declaration this body already executed beats everything the file's
        // load installed; failing that, the last declaration from outside.
        declarations()
            .rfind(|def| in_this_body(def) && def.name_span.start() < call_off)
            .or_else(|| declarations().rfind(|def| !in_this_body(def)))
            .or(Some(winner))
    }

    /// Innermost declaration body extent retained by genuine original source
    /// metadata. It supplies source ordering geometry without entered execution.
    #[must_use]
    pub fn original_definition_body_span(&self, off: u32) -> Option<Span> {
        self.original_procedure_declarations()
            .map(|record| record.metadata().body_span)
            .chain(
                self.original_class_declarations()
                    .map(|record| record.metadata().body_span),
            )
            .filter(|span| span.start() <= off && off < span.end())
            .min_by_key(|span| span.end() - span.start())
    }

    /// The body span of the *innermost* recorded proc or class definition
    /// containing `off` — the body that is actually executing when the
    /// statement at `off` runs — or `None` at load-time (top level).
    ///
    /// The span form of [`Self::enclosing_definition_qualified_name`], for the
    /// order-gating rules that need to ask "is this other statement part of
    /// the *same* running body, or did it already run at load time?" —
    /// [`Self::proc_def_in_effect_at`] and
    /// [`crate::analyser::indirection::in_effect`].
    #[must_use]
    pub fn innermost_definition_body_span(&self, off: u32) -> Option<Span> {
        self.all_procs
            .values()
            .map(|p| p.body_span)
            .chain(self.all_classes.values().map(|c| c.body_span))
            .filter(|span| span.start() <= off && off < span.end())
            .min_by_key(|span| span.end() - span.start())
    }

    /// The qualified name of the *innermost* recorded proc or class
    /// definition whose body contains offset `off`, or `None` when `off`
    /// isn't inside any recorded definition body.  Bodies can nest (a
    /// `proc` written inside another proc's or a class's body), so this
    /// picks the smallest enclosing span rather than an arbitrary match —
    /// the definition whose body most tightly wraps `off` is the one that
    /// actually runs when the call at `off` executes.
    #[must_use]
    pub fn enclosing_definition_qualified_name(&self, off: u32) -> Option<&str> {
        self.all_procs
            .values()
            .map(|p| (p.qualified_name.as_str(), p.body_span))
            .chain(
                self.all_classes
                    .values()
                    .map(|c| (c.qualified_name.as_str(), c.body_span)),
            )
            .filter(|(_, span)| span.start() <= off && off < span.end())
            .min_by_key(|(_, span)| span.end() - span.start())
            .map(|(qn, _)| qn)
    }
}
