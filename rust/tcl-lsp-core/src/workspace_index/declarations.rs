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

//! Procedure, class and namespace-variable declarations in the index: listings, live definitions, source candidates and lookups by name.

use super::records::{WorkspaceDiagnosticSourceContext, WorkspaceProc, WorkspaceVariable};
use super::{WorkspaceClass, WorkspaceIndex};
use std::collections::HashSet;
use std::sync::Arc;
use tcl_syntax::naming::{root_unrooted_key, unroot_rooted_key};

impl WorkspaceIndex {
    /// Whether the definition named by `qualified_name` remains callable in
    /// its document's final command table.
    ///
    /// A document can define a name again after a `rename`/destruction, so
    /// retirement is compared with the declaration's own offset rather than
    /// being a workspace-wide tombstone.  Ordering across documents is not a
    /// static Tcl fact; a lifecycle record therefore only retires a
    /// definition from the same URI.
    fn definition_name_is_live(&self, uri: &str, qualified_name: &str, at: u32) -> bool {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.slots.get(uri).is_none_or(|&slot| {
            !self.docs[slot]
                .command_retirements
                .iter()
                .any(|retirement| {
                    unroot_rooted_key(&retirement.qualified_name)
                        .unwrap_or(&retirement.qualified_name)
                        == target
                        && retirement.at > at
                })
        })
    }

    /// Every command name the workspace defines — each indexed proc and class
    /// in the three forms a call site may spell (`::ns::name`, `ns::name`,
    /// `name`).
    ///
    /// This is the cross-file "does this command exist anywhere in the
    /// project?" set the unknown-command (W123) refinement consults.  It is
    /// **derived**, so it is built once and cached until the next index
    /// mutation rather than walked per request: on a 400-file / 10 000-proc
    /// workspace it is ~20 000 names and ~7 ms to build, against ~120 ns to
    /// serve from the cache.
    #[must_use]
    pub fn command_names(&self) -> Arc<HashSet<String>> {
        self.command_names.get_or_build(|| {
            let mut names: HashSet<String> = HashSet::new();
            for p in self.live_procs() {
                tcl_compiler::analyser::utils::insert_qualified_and_tail(
                    &mut names,
                    &p.qualified_name,
                );
            }
            for c in self.live_classes() {
                tcl_compiler::analyser::utils::insert_qualified_and_tail(
                    &mut names,
                    &c.qualified_name,
                );
            }
            names
        })
    }

    /// Every indexed proc.
    pub fn procs(&self) -> impl Iterator<Item = &WorkspaceProc> {
        self.docs.iter().flat_map(|doc| doc.procs.iter())
    }

    /// Original source declarations without presentation-key collapse.
    /// These records grant declaration assistance. Positioned references and
    /// command lifetime must be supplied independently by the source owner.
    pub fn original_procedure_declarations(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
                tcl_compiler::analyser::ProcDef,
            >,
        ),
    > {
        self.docs.iter().flat_map(|doc| {
            doc.original_procs
                .iter()
                .map(|declaration| (doc.uri.as_str(), declaration))
        })
    }

    /// Current readonly procedure source suggestions. Actual completed publications
    /// and conditional authored publications retain independent owners; unrepresented
    /// original declarations remain source cards. No loading, dispatch or edit follows.
    pub fn original_procedure_source_candidates(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
                tcl_compiler::analyser::ProcDef,
            >,
            &tcl_core_types::ByteCommandSlot,
            tcl_syntax::naming::NamePolicyProtocol,
        ),
    > {
        // naming.source.original-procedure-publications
        // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
        self.docs.iter().flat_map(|document| {
            document
                .original_procedure_source_candidates()
                .into_iter()
                .map(move |(declaration, slot, policy)| {
                    (document.uri.as_str(), declaration, slot, policy)
                })
        })
    }

    /// Current readonly class suggestions with independently selected slots.
    /// Canonical declaration identity remains separate from a moved source slot;
    /// neither conditional nor completed source inventories prove workspace loading.
    pub fn original_class_source_candidates(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
                tcl_compiler::analyser::ClassDef,
            >,
            &tcl_core_types::ByteCommandSlot,
            tcl_syntax::naming::NamePolicyProtocol,
        ),
    > {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        self.docs.iter().flat_map(|document| {
            document.original_class_source_candidates().into_iter().map(
                move |(declaration, slot, policy, _)| {
                    (document.uri.as_str(), declaration, slot, policy)
                },
            )
        })
    }

    /// Current retained hosted source headers and their independent owner context.
    /// This readonly inventory grants no native recipe, command publication or lookup.
    pub fn original_vendor_declarations(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &WorkspaceDiagnosticSourceContext,
            &crate::vendor_declaration::RetainedOriginalVendorDeclaration,
        ),
    > {
        self.docs.iter().flat_map(|document| {
            document
                .original_vendor_declarations
                .iter()
                .filter_map(move |record| {
                    let context = document.diagnostic_source.as_ref()?;
                    let declaration = record.declaration();
                    (context.uses_original_names()
                        && declaration
                            .input()
                            .matches_source(context.image(), context.config()))
                    .then_some((document.uri.as_str(), context, record))
                })
        })
    }

    /// Original object declarations, independent of optional displayed names.
    pub fn original_class_declarations(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
                tcl_compiler::analyser::ClassDef,
            >,
        ),
    > {
        self.docs.iter().flat_map(|doc| {
            doc.original_classes
                .iter()
                .map(|declaration| (doc.uri.as_str(), declaration))
        })
    }

    /// Class declarations in other documents that the `superclass` or `mixin`
    /// word at `cursor` selects through its own recorded lookup (current
    /// namespace, then global). A navigation listing for a relation whose class
    /// another document declares; rename does not read it.
    #[must_use]
    pub fn class_relation_targets_at(
        &self,
        uri: &str,
        source: &str,
        analysis: &tcl_compiler::analyser::AnalysisResult,
        cursor: u32,
    ) -> Vec<(String, tcl_lexer::Span)> {
        let Some(relation) = crate::original_oo::relation_word_at(source, analysis, cursor) else {
            return Vec::new();
        };
        let declarations = self
            .original_class_declarations()
            .filter(|(owner, _)| *owner != uri)
            .collect::<Vec<_>>();
        relation
            .lookup()
            .first_matching_publications(
                declarations
                    .iter()
                    .map(|(owner, declaration)| (declaration.name(), (*owner, *declaration))),
            )
            .into_iter()
            .map(|(owner, declaration)| (owner.to_owned(), declaration.metadata().name_span))
            .collect()
    }

    /// Publications surviving an independently completed modeled source root.
    /// Document ownership is retained; no workspace load order or physical
    /// interpreter existence follows from this source inventory.
    pub fn original_command_publications(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::command_binding::OriginalCommandPublication,
        ),
    > {
        self.docs.iter().flat_map(|doc| {
            doc.original_command_world.iter().flat_map(move |world| {
                world
                    .declarations()
                    .map(move |publication| (doc.uri.as_str(), publication))
            })
        })
    }

    /// Select source publications through the call's original ordered paths.
    /// Unknown or disagreeing alternatives remain unknown. An empty selection
    /// concerns only this inventory, not absence from a running interpreter.
    #[must_use]
    pub fn original_command_candidates(
        &self,
        lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
    ) -> Option<
        Vec<(
            &str,
            &tcl_compiler::command_binding::OriginalCommandPublication,
        )>,
    > {
        lookup.matching_slot_publications(self.original_command_publications().map(
            |(uri, publication)| (publication.slot(), publication.policy(), (uri, publication)),
        ))
    }

    /// Procedure declaration candidates using the shared publication matcher.
    /// This provides workspace source assistance, without proving loading,
    /// normal completion, current command lifetime or a writable reference.
    pub fn original_procedure_candidates(
        &self,
        lookup: &tcl_compiler::signature_scan::scope::SignatureSourceLookup,
    ) -> Vec<(
        &str,
        &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ProcDef,
        >,
    )> {
        let Some(candidates) = lookup.candidates() else {
            return Vec::new();
        };
        tcl_compiler::signature_scan::scope::first_matching_byte_slots(
            lookup.policy(),
            &candidates,
            self.original_procedure_source_candidates()
                .map(|(uri, declaration, slot, policy)| (slot, policy, (uri, declaration))),
        )
    }

    /// Object declaration candidates under the same shared lookup purpose.
    pub fn original_class_candidates(
        &self,
        lookup: &tcl_compiler::signature_scan::scope::SignatureSourceLookup,
    ) -> Vec<(
        &str,
        &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ClassDef,
        >,
    )> {
        let Some(candidates) = lookup.candidates() else {
            return Vec::new();
        };
        tcl_compiler::signature_scan::scope::first_matching_byte_slots(
            lookup.policy(),
            &candidates,
            self.original_class_source_candidates()
                .map(|(uri, declaration, slot, policy)| (slot, policy, (uri, declaration))),
        )
    }

    /// Every proc whose name was not later retired at load level in its own
    /// document.  Cross-document command consumers should use this view;
    /// [`Self::procs`] remains the source-record view for declaration tools.
    pub fn live_procs(&self) -> impl Iterator<Item = &WorkspaceProc> {
        self.procs().filter(|proc_def| {
            self.definition_name_is_live(
                &proc_def.uri,
                &proc_def.qualified_name,
                proc_def.name_span.start(),
            )
        })
    }

    /// Live procs contributed by one document, without walking every other
    /// document's definition table.
    #[must_use]
    pub fn live_procs_in(&self, uri: &str) -> Vec<&WorkspaceProc> {
        let Some(&slot) = self.slots.get(uri) else {
            return Vec::new();
        };
        self.docs[slot]
            .procs
            .iter()
            .filter(|proc_def| {
                self.definition_name_is_live(
                    &proc_def.uri,
                    &proc_def.qualified_name,
                    proc_def.name_span.start(),
                )
            })
            .collect()
    }

    /// Every indexed class.
    pub fn classes(&self) -> impl Iterator<Item = &WorkspaceClass> {
        self.docs.iter().flat_map(|doc| doc.classes.iter())
    }

    /// Every class whose command name was not later retired at load level in
    /// its own document.  `TclOO`'s class command is a command like any other:
    /// `rename Dog {}` makes `Dog new` unknown, not an object construction.
    pub fn live_classes(&self) -> impl Iterator<Item = &WorkspaceClass> {
        self.classes().filter(|class_def| {
            self.definition_name_is_live(
                &class_def.uri,
                &class_def.qualified_name,
                class_def.name_span.start(),
            )
        })
    }

    /// Live classes contributed by one document, without walking every other
    /// document's definition table.
    #[must_use]
    pub fn live_classes_in(&self, uri: &str) -> Vec<&WorkspaceClass> {
        let Some(&slot) = self.slots.get(uri) else {
            return Vec::new();
        };
        self.docs[slot]
            .classes
            .iter()
            .filter(|class_def| {
                self.definition_name_is_live(
                    &class_def.uri,
                    &class_def.qualified_name,
                    class_def.name_span.start(),
                )
            })
            .collect()
    }

    /// Workspace classes whose qualified name matches `name` exactly or via
    /// the leading-`::` normalisation (`Animal` ↔ `::Animal`).  Used to
    /// resolve the class **at the cursor**, whose name arrives already
    /// qualified.
    ///
    /// Deliberately does *not* fall back to a bare simple-name (tail) match:
    /// superclass / mixin names are namespace-relative in Tcl, so an
    /// ownerless tail match (`Base` → `::Other::Base`) could manufacture a
    /// wrong cross-file link.  Owner-aware resolution of written super/mixin
    /// names is done by [`Self::supertype_classes`] / [`Self::subclasses_of`]
    /// via [`resolve_class_name`], which walks the defining class's
    /// namespace ancestry before considering a *unique* tail.
    #[must_use]
    pub fn classes_named<'a>(&'a self, name: &str) -> Vec<&'a WorkspaceClass> {
        let q = root_unrooted_key(unroot_rooted_key(name).unwrap_or(name));
        self.classes()
            .filter(|c| c.qualified_name == name || c.qualified_name == q)
            .collect()
    }

    /// Procs whose simple *or* qualified name starts with
    /// `prefix`, excluding any defined in `exclude_uri` (the
    /// caller's current document, whose procs the single-doc
    /// provider already surfaces).  Empty `prefix` matches all.
    #[must_use]
    pub fn procs_matching<'a>(&'a self, prefix: &str, exclude_uri: &str) -> Vec<&'a WorkspaceProc> {
        self.procs()
            .filter(|p| p.uri != exclude_uri)
            .filter(|p| {
                prefix.is_empty()
                    || p.name.starts_with(prefix)
                    || p.qualified_name.starts_with(prefix)
            })
            .collect()
    }

    /// Proc definitions matching `name` (simple, qualified, or
    /// `::`-prefixed simple form), excluding `exclude_uri`.
    /// Used by cross-document go-to-definition: when the
    /// current document has no matching proc, the index
    /// resolves one defined elsewhere.
    #[must_use]
    pub fn proc_definitions<'a>(&'a self, name: &str, exclude_uri: &str) -> Vec<&'a WorkspaceProc> {
        let qualified = root_unrooted_key(name);
        self.procs()
            .filter(|p| p.uri != exclude_uri)
            .filter(|p| p.name == name || p.qualified_name == name || p.qualified_name == qualified)
            .collect()
    }

    /// Class definitions matching `name`, excluding
    /// `exclude_uri`.
    #[must_use]
    pub fn class_definitions<'a>(
        &'a self,
        name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceClass> {
        let qualified = root_unrooted_key(name);
        self.classes()
            .filter(|c| c.uri != exclude_uri)
            .filter(|c| c.name == name || c.qualified_name == name || c.qualified_name == qualified)
            .collect()
    }

    /// Proc definitions whose **fully-qualified** name equals `qualified_name`
    /// (leading `::` ignored), excluding `exclude_uri`.
    ///
    /// This is the correct matcher for cross-document **rename**: a proc in
    /// another file is the *same* proc only when its qualified name matches, so
    /// renaming `::a::helper` must not touch a `proc helper` inside
    /// `namespace eval ::b` (whose qualified name is `::b::helper`). The looser
    /// [`Self::proc_definitions`] matches by simple name for go-to-definition
    /// and must not be reused here.
    #[must_use]
    pub fn proc_definitions_qualified<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceProc> {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.procs()
            .filter(|p| p.uri != exclude_uri)
            .filter(|p| unroot_rooted_key(&p.qualified_name).unwrap_or(&p.qualified_name) == target)
            .collect()
    }

    /// Class definitions whose fully-qualified name equals `qualified_name`
    /// (leading `::` ignored), excluding `exclude_uri`. The class analogue of
    /// [`Self::proc_definitions_qualified`] for cross-document rename.
    #[must_use]
    pub fn class_definitions_qualified<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceClass> {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.classes()
            .filter(|c| c.uri != exclude_uri)
            .filter(|c| unroot_rooted_key(&c.qualified_name).unwrap_or(&c.qualified_name) == target)
            .collect()
    }

    /// Every indexed namespace-qualified variable declaration.
    pub fn variables(&self) -> impl Iterator<Item = &WorkspaceVariable> {
        self.docs.iter().flat_map(|doc| doc.variables.iter())
    }

    /// Original declaration candidates whose checked outward C key matches
    /// this compatibility query, excluding `exclude_uri`. Source coordinates
    /// supply no runtime namespace incarnation or complete target census.
    #[must_use]
    pub fn variable_definitions_qualified<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceVariable> {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.variables()
            .filter(|v| v.uri != exclude_uri)
            .filter(|v| unroot_rooted_key(&v.qualified_name).unwrap_or(&v.qualified_name) == target)
            .collect()
    }

    /// The fully-qualified names of every indexed class — the workspace class
    /// set the cross-file analysis feeds to
    /// [`tcl_compiler::analyser::Analyser::with_workspace_classes`] so a
    /// consumer document's `set d [::other::Cls new]` resolves cross-file.
    #[must_use]
    pub fn all_class_qnames(&self) -> std::collections::HashSet<String> {
        self.classes().map(|c| c.qualified_name.clone()).collect()
    }

    /// The qualified names of indexed classes whose own command constructs an
    /// instance from a bare unrecognised word — the workspace half of Tk's
    /// `::tk::IconList .il` idiom.
    ///
    /// Empty for every workspace with no such metaclass, which is nearly all
    /// of them, so a consumer given it pays nothing.
    #[must_use]
    pub fn bare_word_construction_class_qnames(&self) -> std::collections::HashSet<String> {
        self.classes()
            .filter(|c| c.bare_word_construction)
            .map(|c| c.qualified_name.clone())
            .collect()
    }

    /// The URIs of documents that invoke (a constructor of) any class in
    /// `class_qnames` — the *candidate consumer* documents whose `$obj method`
    /// sites a cross-file method reference must scan.  A call qualifies when any
    /// of its resolution candidates names one of the classes (leading `::`
    /// ignored), which catches `Cls new` / `Cls create obj` however the class
    /// was spelled at the call site.  Bounds the consumer scan to documents that
    /// actually mention a family class rather than the whole workspace.
    #[must_use]
    pub fn documents_invoking_classes(
        &self,
        class_qnames: &std::collections::HashSet<&str>,
    ) -> std::collections::HashSet<String> {
        self.invocations()
            .filter(|i| {
                i.resolution_candidates
                    .iter()
                    .any(|c| class_qnames.contains(unroot_rooted_key(c).unwrap_or(c)))
            })
            .map(|i| i.uri.clone())
            .collect()
    }
}
