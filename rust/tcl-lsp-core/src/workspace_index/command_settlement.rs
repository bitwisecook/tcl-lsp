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

//! The command each invocation settles to: live links, the settled-target index and the call sites of a declaration.

use super::namespace_imports::{CallSite, WildcardImportIndex, importing_namespace_of};
use super::records::WorkspaceCommandLink;
use super::{WorkspaceIndex, WorkspaceInvocation};
use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;
use tcl_lexer::Span;
use tcl_syntax::naming::{key_holder_and_tail, root_unrooted_key, unroot_rooted_key};

/// One document's contribution to the settled-target index: target name and
/// the invocation indices in that document which reach it.
///
/// [`WorkspaceIndex::invocations_of`] backs code-lens's per-proc / per-class
/// reference count, which calls it once per proc and once per class, so
/// re-settling *every* invocation in the workspace against the candidate
/// target on each call would cost an N-proc document an O(N × invocations)
/// walk, each rebuilding [`WildcardImportIndex`] (five `HashMap`s over the
/// whole index) from scratch. Settling every invocation once per generation
/// and grouping the results turns each `invocations_of` call into a hash
/// lookup plus a direct index into the owning document's slot — the indices
/// are stable because a document's `Vec<WorkspaceInvocation>` is only ever
/// cleared and refilled wholesale (`DocumentRecords::clear` /
/// `index_document`), never reordered in place, and any mutation that could
/// invalidate a stored `(slot, index)` pair also bumps the generation and
/// drops the [`Derived`] view holding it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum SettledCommandTarget {
    Authored(String),
    OriginalSlot(
        tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
    ),
}

pub(super) type DocumentSettledTargets = Vec<(SettledCommandTarget, usize)>;

/// The workspace-wide reverse index used by find-references and code lenses:
/// every invocation's settled target, grouped by that target's `::`-stripped
/// qualified name (`target -> [(doc slot, index within that slot's
/// invocations)]`).
///
/// A `BTreeSet` permits removal of the old contribution from one changed
/// document without scanning every caller of a popular command, while keeping
/// a stable document-slot / source-order answer.
pub(super) type SettledTargets =
    std::collections::HashMap<SettledCommandTarget, BTreeSet<(usize, usize)>>;

/// Incremental storage for one of the direct or link-following settlement
/// modes.  It is deliberately separate from [`Derived`]: an edit in a proc
/// body changes that document's invocation rows but not the command tables
/// those rows resolve against, so throwing the complete reverse index away is
/// needless work.
#[derive(Debug, Default)]
pub(super) struct IncrementalSettledTargets {
    pub(super) state: std::sync::Mutex<IncrementalSettledState>,
}

#[derive(Debug, Default)]
pub(super) struct IncrementalSettledState {
    /// `None` means this slot has never been settled (or has been removed).
    by_document: Vec<Option<DocumentSettledTargets>>,
    by_target: SettledTargets,
    /// A resolution-input change invalidates every document.  Otherwise this
    /// holds exactly the document slots whose own invocation rows changed.
    rebuild_all: bool,
    dirty_slots: BTreeSet<usize>,
    /// Test-visible accounting for the incremental firewall.
    settled_documents: usize,
}

impl Clone for IncrementalSettledTargets {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl IncrementalSettledTargets {
    pub(super) fn invalidate_all(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.by_document.clear();
        state.by_target.clear();
        state.rebuild_all = true;
        state.dirty_slots.clear();
    }

    pub(super) fn mark_document_dirty(&self, slot: usize) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !state.rebuild_all {
            state.dirty_slots.insert(slot);
        }
    }

    #[cfg(test)]
    pub(super) fn settled_documents(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .settled_documents
    }
}

impl WorkspaceIndex {
    pub(super) fn import_source_matches(
        &self,
        report: &str,
        source: Option<&tcl_syntax::naming::NativeNamespacePatternSource>,
        name: &str,
    ) -> bool {
        use tcl_syntax::naming::{NativeNameProtocol, NativeNamespacePatternSource};
        let Some(source) = source else { return false };
        let declarations: Vec<_> = self
            .procs()
            .filter(|p| p.qualified_name == report)
            .map(|p| p.source_name.as_ref())
            .chain(
                self.classes()
                    .filter(|c| c.qualified_name == report)
                    .map(|c| {
                        if c.source_name_ambiguous {
                            None
                        } else {
                            c.source_name.as_ref()
                        }
                    }),
            )
            .collect();
        !declarations.is_empty()
            && declarations.into_iter().all(|receipt| {
                receipt.is_some_and(|receipt| {
                    let slot = receipt.slot();
                    match (source, receipt.policy().recipe()) {
                        (NativeNamespacePatternSource::C(namespace), NativeNameProtocol::C(_)) => {
                            &slot.namespace == namespace
                                && slot.simple.as_bytes() == name.as_bytes()
                        }
                        (
                            NativeNamespacePatternSource::Jim(namespace),
                            NativeNameProtocol::Jim084,
                        ) => {
                            let recipe = receipt.policy().recipe();
                            recipe.namespace_qualifier_bytes(slot.simple.as_bytes())
                                == namespace.as_bytes()
                                && recipe.namespace_tail_bytes(slot.simple.as_bytes())
                                    == name.as_bytes()
                        }
                        _ => false,
                    }
                })
            })
    }

    /// The command name-links that are actually installed — every `interp
    /// alias` / `rename` link, plus each exact `namespace import` link whose
    /// [`WorkspaceCommandLink::import_gate`] its source namespace's export
    /// timeline admits.
    ///
    /// Every consumer of `command_links` that answers "does this name exist /
    /// what does it reach" goes through here, so an import Tcl never
    /// installed cannot leak into definition, references, or the existence
    /// oracle through one of them. The mask is built once per
    /// [`Self::generation`] (a [`Derived`] view); this call is then a `Vec` of
    /// borrows, the same order of cost as the `HashMap`/`HashSet` each caller
    /// already builds.
    ///
    /// The gate only fires for a source namespace the workspace can actually
    /// *observe* ([`Self::observable_namespaces`]).  `namespace import
    /// ::msgcat::mc` names a namespace that lives in an installed package, not
    /// in any indexed document: the index holds no export declaration for it
    /// and never will, so treating that silence as "not exported" would revoke
    /// a real command and hand W123 a fresh false positive on every bare `mc`
    /// call.  Absence of an export is evidence only where the definitions are
    /// visible too — otherwise this abstains and keeps the link, the same
    /// abstain-toward-silence rule the unknown-command pass follows.
    pub(super) fn live_command_links(&self) -> Vec<&WorkspaceCommandLink> {
        let mask = self.live_links.get_or_build(|| {
            let wci = WildcardImportIndex::build(self);
            let observable = self.observable_namespaces();
            self.command_links()
                .map(|l| {
                    l.import_gate.as_ref().is_none_or(|g| {
                        if self.defines_command(&l.target_qname)
                            && !self.import_source_matches(
                                &l.target_qname,
                                g.native_source.as_ref(),
                                &g.name,
                            )
                        {
                            return false;
                        }
                        if !observable
                            .contains(unroot_rooted_key(&g.source_ns).unwrap_or(&g.source_ns))
                        {
                            return true;
                        }
                        if !wci.exports_name_at(&g.source_ns, &g.name, g.site(&l.uri)) {
                            return false;
                        }
                        // A non-`-force` import onto a name the target
                        // namespace already holds installs nothing at all
                        // (oracle on `WorkspaceGlobImport::forced`), so
                        // the link it would introduce is not live either.
                        // "Already holds" is a real *definition*, a command
                        // a fresh interpreter of this document's dialect
                        // already has (the exact-import twin of
                        // `GlobImportRow::declares_builtin_at`), or an
                        // earlier live alias from a **different** source
                        // in the same document;
                        // a same-source re-import is a silent no-op, and
                        // two links cancelling each other out is what the
                        // different-source test rules out.
                        let importing_ns = importing_namespace_of(&l.linked_qname);
                        if !g.forced
                            && (self.defines_command(&l.linked_qname)
                                || self.registry_for(&l.uri).is_some_and(|r| {
                                    r.declares_command_at_rooted_key(&l.linked_qname)
                                })
                                || wci.conflicting_alias_at(
                                    importing_ns,
                                    &g.source_ns,
                                    &g.name,
                                    g.site(&l.uri),
                                ))
                        {
                            return false;
                        }
                        // …and the alias it installed can be taken away
                        // again: `namespace forget`, a redefinition of the
                        // imported name, or destruction of the source
                        // command.
                        wci.link_alias_live(importing_ns, g, &l.uri)
                    })
                })
                .collect()
        });
        self.command_links()
            .zip(mask.iter())
            .filter_map(|(link, &live)| live.then_some(link))
            .collect()
    }

    /// Whether the workspace holds a real proc or class definition at
    /// `qualified_name` — the "already exists" side of a non-`-force`
    /// `namespace import` conflict.
    ///
    /// Deliberately *not* [`Self::workspace_command_exists`], which also
    /// admits linked names: a link is what this question is being asked
    /// about, so counting one would make an import conflict with itself.
    ///
    /// Answered from the link-free reading of [`Self::defined_command_names`]
    /// — which is that same proc + class name set, already derived once per
    /// generation — rather than by re-walking every indexed proc and class.
    /// This sits inside `import_hop`'s per-call filter chain, where a re-walk
    /// would cost O(procs) *per call site per in-scope import*.
    pub(super) fn defines_command(&self, qualified_name: &str) -> bool {
        self.defined_command_names(false)
            .contains(unroot_rooted_key(qualified_name).unwrap_or(qualified_name))
    }

    /// The `::`-stripped namespaces the workspace can say anything about: one
    /// that owns an indexed proc or class, that declares a `namespace export`
    /// somewhere, or that an indexed `namespace eval` block declares.
    ///
    /// The discriminator between "this namespace does not export the name"
    /// (a fact) and "this namespace is not in the workspace at all" (no
    /// information) — see [`Self::live_command_links`].
    ///
    /// The declaring-block source closes a hole the first two leave: a
    /// `namespace eval ::ns { namespace import ::other::* }` block that
    /// declares no proc, class, or export of its own *is* a namespace the
    /// workspace can see, and treating it as unknown would make the import
    /// gate abstain where it has the evidence to decide.
    pub(super) fn observable_namespaces(&self) -> HashSet<&str> {
        fn owning_ns(qualified: &str) -> &str {
            {
                let (holder, _) = key_holder_and_tail(qualified);
                unroot_rooted_key(holder).unwrap_or(holder)
            }
        }
        self.procs()
            .map(|p| owning_ns(&p.qualified_name))
            .chain(self.classes().map(|c| owning_ns(&c.qualified_name)))
            .chain(
                self.namespace_exports()
                    .map(|e| unroot_rooted_key(&e.ns).unwrap_or(&e.ns)),
            )
            .chain(
                self.namespace_refs()
                    .filter(|n| n.declares)
                    .map(|n| unroot_rooted_key(&n.qualified_name).unwrap_or(&n.qualified_name)),
            )
            .collect()
    }

    /// Every indexed `namespace import` / `interp alias` / `rename` link.
    pub fn command_links(&self) -> impl Iterator<Item = &WorkspaceCommandLink> {
        self.docs.iter().flat_map(|doc| doc.command_links.iter())
    }

    /// Invocation sites that target the proc identified by
    /// `simple_name` / `qualified_name`, excluding any in
    /// `exclude_uri` (the caller's own document, whose call
    /// sites the single-doc provider already surfaces).
    ///
    /// Each call site is settled against a **workspace-wide** command-existence
    /// oracle: its [`resolution_candidates`](WorkspaceInvocation::resolution_candidates)
    /// (caller namespace, then each `namespace path` entry, then global — Tcl's
    /// real priority order) are walked, and the first that names a proc/class
    /// defined *anywhere in the workspace* is the call's true target.  A call is
    /// a reference iff that target is `qualified_name`.
    ///
    /// This is the canonical resolver ([`tcl_syntax::naming::resolve_command_with`])
    /// widened from one file to the whole project: a bare call reaching a
    /// namespaced proc in another file via `namespace path` resolves correctly
    /// (the file-local guess could not settle it), and a call whose simple name
    /// collides with an unrelated proc resolves to the one it actually names —
    /// no textual heuristic, no ambiguity gate.
    #[must_use]
    pub fn invocations_of<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceInvocation> {
        self.invocations_settling_to(qualified_name, exclude_uri, false)
    }

    /// References to an independently retained original declaration slot.
    /// Linked references keep their selected implementation's declaration slot;
    /// direct references keep the actual called slot. Source assistance grants
    /// neither interpreter loading nor writable edits.
    #[must_use]
    pub fn original_invocations_of<'a>(
        &'a self,
        declaration: &tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        exclude_uri: &str,
        follow_links: bool,
    ) -> Vec<&'a WorkspaceInvocation> {
        let target =
            SettledCommandTarget::OriginalSlot(declaration.slot().clone(), declaration.policy());
        self.settled_sites(&[target], follow_links)
            .into_iter()
            .map(|(slot, index)| &self.docs[slot].invocations[index])
            .filter(|invocation| invocation.uri != exclude_uri)
            .collect()
    }

    /// Whether renaming `qualified_name` must be refused outright: some
    /// invocation of it, anywhere in the workspace, is marked
    /// `rename_safe: false` — an indirect dispatch at least one of whose
    /// contributing constants has no exact writable source span, so no
    /// edit set can keep that dispatch running the renamed command.
    #[must_use]
    pub fn rename_blocked(&self, qualified_name: &str) -> bool {
        self.invocations_of(qualified_name, "")
            .iter()
            .any(|inv| !inv.rename_safe)
    }

    /// Invocation sites that reach `qualified_name` **through** a command
    /// name-link — an `interp alias`, a `rename`, or a `namespace import`.
    ///
    /// The same candidate settling as [`Self::invocations_of`], but the
    /// existence oracle also admits the linked names an import / alias /
    /// rename introduces, and the winning candidate is followed along those
    /// links to its ultimate target before matching.  So a bare `helper` call
    /// in a namespace that `namespace import`ed `::mod::helper` counts as a
    /// reference to `::mod::helper`, and a call through an alias counts as a
    /// reference to the aliased command.  Used by find-references, which shows
    /// every use; **not** by rename, which must not text-rewrite a call that
    /// names the local imported / aliased command (the token follows the
    /// source rename at runtime, it is not edited).
    #[must_use]
    pub fn linked_invocations_of<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceInvocation> {
        self.invocations_settling_to(qualified_name, exclude_uri, true)
    }

    /// The qualified names of the workspace declarations the call written at
    /// `range` in `uri` settles to in the link-following view. A listing aid
    /// for find-references on a release profile; rename does not read it.
    #[must_use]
    pub fn linked_declaration_names_of_call(&self, uri: &str, range: Span) -> Vec<String> {
        let Some(document) = self.slots.get(uri).and_then(|slot| self.docs.get(*slot)) else {
            return Vec::new();
        };
        let Some(invocation) = document.invocations.iter().find(|inv| inv.range == range) else {
            return Vec::new();
        };
        let defined = self.defined_command_names(true);
        let links = self.command_link_map();
        let wci = WildcardImportIndex::build(self);
        match self.settle_invocation(invocation, &defined, Some(&links), &wci) {
            Some(SettledCommandTarget::Authored(name)) => {
                vec![tcl_syntax::naming::root_unrooted_key(&name)]
            }
            Some(SettledCommandTarget::OriginalSlot(slot, policy)) => {
                let names = self
                    .original_procedure_declarations()
                    .filter(|(_, declaration)| {
                        declaration.name().slot() == &slot && declaration.name().policy() == policy
                    })
                    .map(|(_, declaration)| declaration.metadata().qualified_name.clone())
                    .chain(
                        self.original_class_declarations()
                            .filter(|(_, declaration)| {
                                declaration.name().slot() == &slot
                                    && declaration.name().policy() == policy
                            })
                            .map(|(_, declaration)| declaration.metadata().qualified_name.clone()),
                    )
                    .collect::<Vec<_>>();
                // Two documents publishing the slot leave the call's owner
                // unknown; the listing abstains rather than pick one.
                if names.len() == 1 { names } else { Vec::new() }
            }
            None => Vec::new(),
        }
    }

    /// Shared core of [`Self::invocations_of`] / [`Self::linked_invocations_of`]:
    /// call sites whose settled target is `qualified_name`, excluding
    /// `exclude_uri`.  With `follow_links`, the existence oracle admits linked
    /// names and the winning candidate is chased along the link map to its
    /// ultimate target; without it, only real proc/class definitions settle a
    /// call (the direct-reference behaviour rename relies on).
    ///
    /// A hash lookup into [`Self::settled_sites`] plus a
    /// direct index into each hit's owning document slot — the settlement
    /// walk itself (which needs [`Self::defined_command_names`],
    /// [`Self::command_link_map`] and a [`WildcardImportIndex`]) runs at most
    /// once per generation, not once per call (`code_lenses` calls this once
    /// per proc *and* once per class in the document).
    fn invocations_settling_to<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
        follow_links: bool,
    ) -> Vec<&'a WorkspaceInvocation> {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        let mut targets = vec![SettledCommandTarget::Authored(target.to_owned())];
        for declaration in self
            .original_procedure_declarations()
            .filter(|(_, declaration)| {
                unroot_rooted_key(&declaration.metadata().qualified_name)
                    .unwrap_or(&declaration.metadata().qualified_name)
                    == target
            })
        {
            targets.push(SettledCommandTarget::OriginalSlot(
                declaration.1.name().slot().clone(),
                declaration.1.name().policy(),
            ));
        }
        for declaration in self
            .original_class_declarations()
            .filter(|(_, declaration)| {
                unroot_rooted_key(&declaration.metadata().qualified_name)
                    .unwrap_or(&declaration.metadata().qualified_name)
                    == target
            })
        {
            targets.push(SettledCommandTarget::OriginalSlot(
                declaration.1.name().slot().clone(),
                declaration.1.name().policy(),
            ));
        }
        let sites = self.settled_sites(&targets, follow_links);
        sites
            .iter()
            .map(|(slot, idx)| &self.docs[*slot].invocations[*idx])
            .filter(|i| i.uri != exclude_uri)
            .collect()
    }

    /// Return the sites settled to `target`, incrementally repairing the
    /// changed documents first.  The cold path settles every document once.
    /// Thereafter a replacement which did not change the command-resolution
    /// inputs removes and re-inserts only that document's contribution in the
    /// reverse index; querying a common target never scans its callers.
    pub(super) fn settled_sites(
        &self,
        targets: &[SettledCommandTarget],
        follow_links: bool,
    ) -> Vec<(usize, usize)> {
        let cache = &self.settled_invocations[usize::from(follow_links)];
        let mut state = cache
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let slots: Vec<usize> = if state.rebuild_all || state.by_document.len() != self.docs.len() {
            state.by_document.clear();
            state.by_document.resize_with(self.docs.len(), || None);
            state.by_target.clear();
            state.rebuild_all = false;
            state.dirty_slots.clear();
            (0..self.docs.len()).collect()
        } else {
            std::mem::take(&mut state.dirty_slots).into_iter().collect()
        };

        if !slots.is_empty() {
            // These are exactly the three workspace-wide inputs
            // `settle_invocation` reads.  They were retained across an
            // invocation-only replacement by `invalidate`, and are rebuilt
            // only when `SettlementDependencies` proved they changed.
            let defined = self.defined_command_names(follow_links);
            let links = follow_links.then(|| self.command_link_map());
            let wci = WildcardImportIndex::build(self);
            for slot in slots {
                if let Some(previous) = state.by_document[slot].take() {
                    for (old_target, idx) in previous {
                        let remove_key = (slot, idx);
                        let empty = state.by_target.get_mut(&old_target).is_some_and(|sites| {
                            sites.remove(&remove_key);
                            sites.is_empty()
                        });
                        if empty {
                            state.by_target.remove(&old_target);
                        }
                    }
                }
                let mut contribution = Vec::new();
                for (idx, inv) in self.docs[slot].invocations.iter().enumerate() {
                    if let Some(resolved) =
                        self.settle_invocation(inv, &defined, links.as_deref(), &wci)
                    {
                        state
                            .by_target
                            .entry(resolved.clone())
                            .or_default()
                            .insert((slot, idx));
                        contribution.push((resolved, idx));
                    }
                }
                state.by_document[slot] = Some(contribution);
                state.settled_documents = state.settled_documents.saturating_add(1);
            }
        }
        targets
            .iter()
            .filter_map(|target| state.by_target.get(target))
            .flat_map(|sites| sites.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn original_definition_target(
        &self,
        definition: &tcl_compiler::command_binding::SourceCommandDefinition,
    ) -> Option<SettledCommandTarget> {
        let site = &definition.allocation().site;
        self.original_procedure_declarations()
            .find(|(_, declaration)| declaration.declaration_site() == site)
            .map(|(_, declaration)| {
                SettledCommandTarget::OriginalSlot(
                    declaration.name().slot().clone(),
                    declaration.name().policy(),
                )
            })
            .or_else(|| {
                self.original_class_declarations()
                    .find(|(_, declaration)| declaration.declaration_site() == site)
                    .map(|(_, declaration)| {
                        SettledCommandTarget::OriginalSlot(
                            declaration.name().slot().clone(),
                            declaration.name().policy(),
                        )
                    })
            })
    }

    pub(super) fn original_assistance_target(
        &self,
        lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
        follow_links: bool,
    ) -> Option<SettledCommandTarget> {
        let mut candidates = Vec::new();
        for document in &self.docs {
            if let Some(world) = &document.original_command_world {
                for publication in world.declarations() {
                    let target = if follow_links {
                        publication
                            .definition()
                            .and_then(|definition| self.original_definition_target(definition))
                    } else {
                        matches!(publication.kind(),
                            tcl_compiler::command_binding::OriginalCommandPublicationKind::Procedure
                            | tcl_compiler::command_binding::OriginalCommandPublicationKind::Object)
                        .then(|| {
                            SettledCommandTarget::OriginalSlot(
                                publication.slot().clone(),
                                publication.policy(),
                            )
                        })
                    };
                    candidates.push((publication.slot(), publication.policy(), target));
                }
            } else {
                // Header-only source assistance has a separate purpose. It
                // grants no completed world, current implementation or edit.
                candidates.extend(
                    document
                        .original_procedure_source_candidates()
                        .into_iter()
                        .map(|(declaration, slot, policy)| {
                            (
                                slot,
                                policy,
                                Some(SettledCommandTarget::OriginalSlot(
                                    declaration.name().slot().clone(),
                                    declaration.name().policy(),
                                )),
                            )
                        }),
                );
                candidates.extend(document.original_class_source_candidates().into_iter().map(
                    |(declaration, slot, policy, _)| {
                        (
                            slot,
                            policy,
                            Some(SettledCommandTarget::OriginalSlot(
                                declaration.name().slot().clone(),
                                declaration.name().policy(),
                            )),
                        )
                    },
                ));
            }
        }
        let selected = lookup.matching_slot_publications(candidates)?;
        let target = selected.first()?.as_ref()?;
        selected
            .iter()
            .all(|candidate| candidate.as_ref() == Some(target))
            .then(|| target.clone())
    }

    /// A release-profile call the document's own analysis did not bind
    /// settles to the workspace definition its lookup selects: the current
    /// namespace, then the global one, or the exact qualified name. The
    /// link-following view lists references and answers whenever that
    /// selection is unanimous. The direct view feeds rename, so it answers
    /// only for a single definition nothing visible can rebind.
    fn workspace_assistance_target(
        &self,
        lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
        follow_links: bool,
    ) -> Option<SettledCommandTarget> {
        let target = self.original_assistance_target(lookup, follow_links)?;
        (follow_links || self.original_target_is_unrebound(&target)).then_some(target)
    }

    /// Whether exactly one workspace declaration publishes this slot and no
    /// `rename`, `interp alias` or `namespace import` in any document names it
    /// as its link or its target.
    fn original_target_is_unrebound(&self, target: &SettledCommandTarget) -> bool {
        let SettledCommandTarget::OriginalSlot(slot, policy) = target else {
            return false;
        };
        let mut publications = 0usize;
        for document in &self.docs {
            if let Some(world) = &document.original_command_world {
                publications += world
                    .declarations()
                    .filter(|publication| {
                        publication.slot() == slot && publication.policy() == *policy
                    })
                    .count();
            } else {
                publications += document
                    .original_procedure_source_candidates()
                    .into_iter()
                    .filter(|(_, candidate, candidate_policy)| {
                        *candidate == slot && candidate_policy == policy
                    })
                    .count();
                publications += document
                    .original_class_source_candidates()
                    .into_iter()
                    .filter(|(_, candidate, candidate_policy, _)| {
                        *candidate == slot && candidate_policy == policy
                    })
                    .count();
            }
        }
        if publications != 1 {
            return false;
        }
        let Ok(name) = String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(slot))
        else {
            return false;
        };
        let name = unroot_rooted_key(&name).unwrap_or(&name);
        !self.command_links().any(|link| {
            unroot_rooted_key(&link.linked_qname).unwrap_or(&link.linked_qname) == name
                || unroot_rooted_key(&link.target_qname).unwrap_or(&link.target_qname) == name
        })
    }

    /// The command `inv` settles to, `::`-stripped, or `None` when nothing in
    /// the workspace resolves it: the first of its candidates defined
    /// anywhere in the workspace, chased along `links` (when supplied) to its
    /// ultimate target — falling back, when following links, to a wildcard
    /// `namespace import NS::*` in scope.  The question asked is "what does
    /// this call settle to" (grouped by the answer) rather than "does it
    /// settle to the one target the caller named", which would be re-checked
    /// once per candidate target.
    pub(super) fn settle_invocation(
        &self,
        inv: &WorkspaceInvocation,
        defined: &HashSet<String>,
        links: Option<&std::collections::HashMap<String, String>>,
        wci: &WildcardImportIndex<'_>,
    ) -> Option<SettledCommandTarget> {
        if let Some(input) = &inv.original_name_input {
            let lookup = inv.original_lookup.as_ref()?;
            if lookup.name_input() != input {
                return None;
            }
        }
        if let Some(reference) = &inv.resolved_command_reference
            && (inv.original_name_input.is_some() || reference.original_name_policy().is_some())
        {
            return if let Some(links) = links {
                reference
                    .linked_definition()
                    .or_else(|| reference.definition())
                    .and_then(|definition| self.original_definition_target(definition))
                    .or_else(|| {
                        let slot = reference.original_slot()?;
                        let name =
                            String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(slot))
                                .ok()?;
                        Self::linked_name_target(&name, links, defined)
                    })
            } else {
                reference.is_direct_definition().then_some(())?;
                Some(SettledCommandTarget::OriginalSlot(
                    reference.original_slot()?.clone(),
                    reference.original_name_policy()?,
                ))
            };
        }
        if let Some(lookup) = inv.original_lookup.as_ref()
            && lookup.policy().authority() == tcl_syntax::naming::NamePolicyAuthority::Native
        {
            return self
                .original_assistance_target(lookup, links.is_some())
                .or_else(|| Self::linked_candidate_target(inv, links?, defined));
        }
        if let Some(lookup) = inv.original_lookup.as_ref()
            && inv.logical_source_domain == Some(false)
        {
            return self
                .workspace_assistance_target(lookup, links.is_some())
                .or_else(|| Self::linked_candidate_target(inv, links?, defined));
        }
        if inv.original_name_input.is_some() || inv.logical_source_domain != Some(false) {
            return None;
        }
        // A positioned source receipt precedes the workspace's assistance
        // inventory. Later imports, renames and unrelated same-named records
        // cannot change the implementation this invocation already selected.
        // The direct view edits a called definition slot; imported spellings
        // belong only to the linked view, which retains the actual declaration.
        if let Some(reference) = &inv.resolved_command_reference {
            return if links.is_some() {
                reference
                    .linked_definition()
                    .or_else(|| reference.definition())
                    .map(|definition| {
                        unroot_rooted_key(&definition.allocation().command)
                            .unwrap_or(&definition.allocation().command)
                            .to_owned()
                    })
                    .map(SettledCommandTarget::Authored)
            } else {
                reference.is_direct_definition().then_some(())?;
                reference
                    .slot()
                    .map(|slot| unroot_rooted_key(slot).unwrap_or(slot).to_owned())
                    .map(SettledCommandTarget::Authored)
            };
        }
        if let Some(definition) = &inv.resolved_definition {
            return if links.is_some() {
                Some(SettledCommandTarget::Authored(
                    unroot_rooted_key(&definition.allocation().command)
                        .unwrap_or(&definition.allocation().command)
                        .to_owned(),
                ))
            } else {
                inv.resolved_user_definition.as_deref().map(|slot| {
                    SettledCommandTarget::Authored(
                        unroot_rooted_key(slot).unwrap_or(slot).to_owned(),
                    )
                })
            };
        }
        let call = CallSite {
            uri: &inv.uri,
            at: inv.range.start(),
            enclosing_body: inv.enclosing_body,
        };
        // A live `namespace import -force` has *replaced* the importing
        // namespace's own command of this name, so no candidate naming that
        // command may settle the call — it reaches the import's source, which
        // the wildcard tier below resolves. The same rule
        // `definition::resolve_called_proc` applies in-document, so without it
        // find-references files the call under the definition the import
        // deleted while go-to-definition jumps to the source.
        //
        // Only in the link-following view, matching the rule below: a glob
        // import introduces no fixed link, so the direct-only view rename
        // relies on does not see it in either direction.
        let forced_shadow = links.is_some()
            && wci.forced_shadow_over_candidates(&inv.name, &inv.resolution_candidates, call);
        if !forced_shadow && let Some(winner) = inv.resolved_user_definition.as_deref() {
            return Some(SettledCommandTarget::Authored(
                unroot_rooted_key(winner).unwrap_or(winner).to_owned(),
            ));
        }
        if !forced_shadow
            && let Some(winner) = inv
                .resolution_candidates
                .iter()
                .find(|c| defined.contains(unroot_rooted_key(c).unwrap_or(c)))
        {
            let winner = unroot_rooted_key(winner).unwrap_or(winner);
            return Some(SettledCommandTarget::Authored(links.map_or_else(
                || winner.to_owned(),
                |m| Self::follow_links(m, winner),
            )));
        }
        // No real command or name-link settled this call — try a wildcard
        // `namespace import NS::*` in scope for the call's own namespace
        // Only when following links: this mirrors an
        // exact import's `WorkspaceCommandLink`, which likewise only
        // participates in the *linked* view (`linked_invocations_of`, used
        // by find-references) and never the direct-only view rename relies
        // on — a call spelling the local imported name is not text-rewritten
        // just because its ultimate source is renamed.
        links?;
        self.resolve_wildcard_import_indexed(&inv.name, &inv.resolution_candidates, call, wci)
            .map(|resolved| {
                SettledCommandTarget::Authored(
                    unroot_rooted_key(&resolved).unwrap_or(&resolved).to_owned(),
                )
            })
    }

    fn linked_candidate_target(
        inv: &WorkspaceInvocation,
        links: &std::collections::HashMap<String, String>,
        defined: &HashSet<String>,
    ) -> Option<SettledCommandTarget> {
        inv.resolution_candidates
            .iter()
            .find_map(|candidate| Self::linked_name_target(candidate, links, defined))
    }

    /// A call whose own document could not follow its name to a definition
    /// (an alias or rename whose target another document declares) settles,
    /// in the link-following view only, where the workspace link map chases
    /// the called name to a defined command.
    fn linked_name_target(
        name: &str,
        links: &std::collections::HashMap<String, String>,
        defined: &HashSet<String>,
    ) -> Option<SettledCommandTarget> {
        let name = unroot_rooted_key(name).unwrap_or(name);
        links.contains_key(name).then_some(())?;
        let target = Self::follow_links(links, name);
        defined
            .contains(&target)
            .then_some(SettledCommandTarget::Authored(target))
    }

    /// The command name-link map (`::`-stripped `linked → immediate target`)
    /// used to chase an import / alias / rename to the command it names.
    /// A [`Derived`] view rather than a fresh walk of
    /// [`Self::live_command_links`] per call; owned (`String`,
    /// not `&str`) so the view is independent of any one call's borrow.
    pub(super) fn command_link_map(&self) -> Arc<std::collections::HashMap<String, String>> {
        self.command_link_map.get_or_build(|| {
            self.live_command_links()
                .into_iter()
                .filter(|link| {
                    link.linked_source_name
                        .as_ref()
                        .is_none_or(|name| name.source_spelling().is_some())
                        && link
                            .target_source_name
                            .as_ref()
                            .is_none_or(|name| name.source_spelling().is_some())
                })
                .map(|l| {
                    (
                        unroot_rooted_key(&l.linked_qname)
                            .unwrap_or(&l.linked_qname)
                            .to_owned(),
                        unroot_rooted_key(&l.target_qname)
                            .unwrap_or(&l.target_qname)
                            .to_owned(),
                    )
                })
                .collect()
        })
    }

    /// Chase `start` along the link map to its ultimate target, stopping at a
    /// name that is not itself a linked name.  Bounded by cycle detection (an
    /// alias-of-an-alias loop) so a malformed chain cannot spin.
    pub(super) fn follow_links(
        links: &std::collections::HashMap<String, String>,
        start: &str,
    ) -> String {
        let mut cur = start.to_owned();
        let mut seen = std::collections::HashSet::new();
        while let Some(next) = links.get(&cur) {
            if !seen.insert(cur.clone()) {
                break;
            }
            cur.clone_from(next);
        }
        cur
    }

    /// The ultimate command `name` denotes after following every
    /// import / alias / rename link, `::`-rooted.  A name that is not linked
    /// (an ordinary proc/class, or an unknown) returns unchanged.  Lets a
    /// cursor sitting on an imported / aliased call resolve to the command it
    /// really names, so its references gather with that command's.
    #[must_use]
    pub fn resolve_command_target(&self, name: &str) -> String {
        let links = self.command_link_map();
        let settled = Self::follow_links(&links, unroot_rooted_key(name).unwrap_or(name));
        root_unrooted_key(&settled)
    }

    /// The declaration spans that *name* the command `qualified_name` in an
    /// `interp alias` / `rename` / `namespace import` — the alias `TARGET`
    /// word, the `rename` `OLD` word, the import pattern.  Each is a reference
    /// to the command that a rename of it must rewrite.  Excludes
    /// `exclude_uri` (the caller's own document, whose spans the single-doc
    /// provider already surfaces) and any link whose source scan recorded no
    /// span.
    #[must_use]
    pub fn link_target_spans(
        &self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<(String, Span)> {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.live_command_links()
            .into_iter()
            .filter(|l| l.uri != exclude_uri)
            .filter(|l| unroot_rooted_key(&l.target_qname).unwrap_or(&l.target_qname) == target)
            .filter_map(|l| l.target_span.map(|sp| (l.uri.clone(), sp)))
            .collect()
    }

    /// Whether the command `qualified_name` (leading `::` ignored) resolves
    /// anywhere in the workspace — either a real proc/class definition, or a
    /// name an `interp alias` / `rename` / `namespace import` introduces.  The
    /// existence oracle that widens the single-file command resolver to the
    /// whole project; the linked names are admitted so a cursor on an
    /// imported / aliased call still finds a symbol to resolve.
    #[must_use]
    pub fn workspace_command_exists(&self, qualified_name: &str) -> bool {
        self.defined_command_names(true)
            .contains(unroot_rooted_key(qualified_name).unwrap_or(qualified_name))
    }

    /// [`Self::workspace_command_exists`], but a proc, or an `interp alias` /
    /// `rename` / `namespace import` link, whose own declaration is nested
    /// inside another proc's or class's body (so it exists only
    /// conditionally, when and if that enclosing definition actually runs)
    /// counts only when `has_builtin` is `false`. An unconditional
    /// (top-level) proc or link still counts regardless of `has_builtin`.
    ///
    /// A nested `proc ::set {...}` written to temporarily shadow the real
    /// `set` builtin — `rename` it away, install the shadow, `rename` it
    /// back — must not make `::set` permanently "exist in the workspace" for
    /// every cross-file call-site resolution the way a real top-level
    /// definition would; that call always reaches the builtin unless a
    /// caller can prove the shadow's narrow active window actually contains
    /// it, which this index does not attempt to prove. The same reasoning
    /// applies to a `rename`/alias/import written inside a body — e.g.
    /// `proc withRealSet {} { rename set ::real_set; ... }` locally
    /// redirecting `set` — while a *top-level* `interp alias {} set {}
    /// ::my_set` permanently overrides the builtin for the rest of the file,
    /// exactly like a top-level `proc set`, and must keep counting as
    /// existing. Mirrors the same judgement `resolve_called_proc` already
    /// applies same-file
    /// (`AnalysisResult::offset_is_inside_any_definition_body`), extended to
    /// the cross-file existence oracle so hover / definition / references
    /// agree instead of one abstaining and the other still finding the
    /// shadow.
    #[must_use]
    pub fn workspace_command_exists_for_call(
        &self,
        qualified_name: &str,
        has_builtin: bool,
    ) -> bool {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.live_procs().any(|p| {
            (!has_builtin || !p.nested)
                && unroot_rooted_key(&p.qualified_name).unwrap_or(&p.qualified_name) == target
        }) || self
            .live_classes()
            .any(|c| unroot_rooted_key(&c.qualified_name).unwrap_or(&c.qualified_name) == target)
            || self.live_command_links().into_iter().any(|l| {
                (!has_builtin || !l.nested)
                    && unroot_rooted_key(&l.linked_qname).unwrap_or(&l.linked_qname) == target
            })
    }

    /// The set of `::`-stripped qualified names of every indexed proc and
    /// class, for O(1) membership in the candidate-resolution loop of
    /// [`Self::invocations_of`].  With `include_links`, the names an import /
    /// alias / rename introduces join the set, so a call reaching one of them
    /// settles (and is then chased to its ultimate target).
    ///
    /// A [`Derived`] view rather than a fresh walk: it
    /// is `O(procs + classes + links)` to build, and
    /// [`Self::workspace_command_exists`] asks for it *per candidate* inside
    /// [`Self::follow_import_chain`]'s loop — and
    /// [`Self::settled_sites`] once per settling pass — so rebuilding it per
    /// ask would turn an existence test into a workspace-wide scan. Owned (`String`,
    /// not `&str`) so the view is independent of any one call's borrow. The
    /// two `include_links` readings are two separate views because a consumer
    /// wants exactly one of them: the direct-only set is what rename relies
    /// on, and folding the link names into it would let a call spelling an
    /// imported name be text-rewritten.
    pub(super) fn defined_command_names(&self, include_links: bool) -> Arc<HashSet<String>> {
        self.defined_names[usize::from(include_links)].get_or_build(|| {
            let mut names: HashSet<String> = self
                .live_procs()
                .map(|p| {
                    unroot_rooted_key(&p.qualified_name)
                        .unwrap_or(&p.qualified_name)
                        .to_owned()
                })
                .chain(self.live_classes().map(|c| {
                    unroot_rooted_key(&c.qualified_name)
                        .unwrap_or(&c.qualified_name)
                        .to_owned()
                }))
                .collect();
            if include_links {
                names.extend(self.live_command_links().into_iter().map(|l| {
                    unroot_rooted_key(&l.linked_qname)
                        .unwrap_or(&l.linked_qname)
                        .to_owned()
                }));
            }
            names
        })
    }
}
