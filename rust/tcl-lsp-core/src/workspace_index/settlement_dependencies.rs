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

//! The per-document inputs command settlement reads, compared across an edit to decide what must settle again.

use super::DocumentRecords;
use super::records::{
    WorkspaceCommandDeletion, WorkspaceCommandLink, WorkspaceCommandRetirement,
    WorkspaceGlobImport, WorkspaceNamespaceExport, WorkspaceNamespaceForget,
    WorkspacePackageIfneeded, WorkspacePackagePrefer, WorkspacePackageProvide,
    WorkspacePackageRequire, WorkspaceSource,
};

#[derive(Clone, PartialEq, Eq)]
pub(super) struct OriginalPublicationDependency {
    slot: tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    pub(super) kind: tcl_compiler::command_binding::OriginalCommandPublicationKind,
    definition: Option<(
        tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
    )>,
}

/// The complete per-document input surface the command-settlement walk reads,
/// directly or through its three shared tables.  Keeping this as an exact
/// value, rather than a lossy fingerprint, makes the incremental boundary a
/// correctness property: a body edit may retain other documents' answers only
/// when every resolution-relevant fact is byte-for-byte unchanged.
///
/// Invocation sites intentionally do not appear here.  Replacing them changes
/// the one document which must be re-settled, but cannot change how another
/// document's call resolves.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct SettlementDependencies {
    pub(super) dialect: String,
    /// Command-store currency read by the import metadata conflict gates.
    pub(super) registry: Option<tcl_registry::RegistrySemanticKey>,
    /// Only the facts `defined_command_names` and the nested-builtin gate
    /// read.  Parameter edits and a shifted declaration span cannot change a
    /// call's target, so retaining the whole `WorkspaceProc` would turn those
    /// ordinary body edits into false full invalidations.
    pub(super) procs: Vec<(String, bool)>,
    /// Classes contribute command existence by qualified name; their member
    /// table is used by a different cross-file query, not settlement.
    pub(super) classes: Vec<String>,
    /// `observable_namespaces` reads declaration identity only.  References
    /// to a namespace and their spans do not affect an import gate.
    declared_namespaces: Vec<String>,
    pub(super) original_declarations: Vec<(
        tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
    )>,
    original_publications: Vec<OriginalPublicationDependency>,
    original_world_closed: bool,
    original_source_procedure_candidates: Vec<(
        tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
        tcl_core_types::ByteCommandSlot,
    )>,
    pub(super) original_source_class_candidates: Vec<(
        tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
        tcl_core_types::ByteCommandSlot,
        Vec<tcl_compiler::command_binding::SourceCommandTransitionObligation>,
    )>,
    pub(super) sources: Vec<WorkspaceSource>,
    /// A changed constant can change which child a computed source row
    /// resolves to, so the raw assignments are resolution-relevant exactly
    /// as the source rows themselves are: in a document with a `source`
    /// row, and nowhere else.
    pub(super) path_constant_assignments: tcl_compiler::auto_path_eval::PathConstantAssignments,
    pub(super) package_requires: Vec<WorkspacePackageRequire>,
    pub(super) package_provides: Vec<WorkspacePackageProvide>,
    pub(super) package_ifneededs: Vec<WorkspacePackageIfneeded>,
    pub(super) package_prefers: Vec<WorkspacePackagePrefer>,
    pub(super) command_links: Vec<WorkspaceCommandLink>,
    pub(super) glob_imports: Vec<WorkspaceGlobImport>,
    pub(super) original_namespace_patterns:
        Vec<tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
    pub(super) namespace_exports: Vec<WorkspaceNamespaceExport>,
    pub(super) namespace_forgets: Vec<WorkspaceNamespaceForget>,
    pub(super) command_deletions: Vec<WorkspaceCommandDeletion>,
    pub(super) command_retirements: Vec<WorkspaceCommandRetirement>,
}

impl DocumentRecords {
    pub(super) fn settlement_dependencies(&self) -> SettlementDependencies {
        SettlementDependencies {
            dialect: self.dialect.clone(),
            registry: self
                .registry()
                .map(|registry| registry.snapshot().semantic_key()),
            procs: self
                .procs
                .iter()
                .map(|proc_def| (proc_def.qualified_name.clone(), proc_def.nested))
                .collect(),
            classes: self
                .classes
                .iter()
                .map(|class_def| class_def.qualified_name.clone())
                .collect(),
            declared_namespaces: self
                .namespace_refs
                .iter()
                .filter(|namespace| namespace.declares)
                .map(|namespace| namespace.qualified_name.clone())
                .collect(),
            original_declarations: self
                .original_procs
                .iter()
                .map(|declaration| {
                    (
                        declaration.name().slot().clone(),
                        declaration.name().policy(),
                    )
                })
                .chain(self.original_classes.iter().map(|declaration| {
                    (
                        declaration.name().slot().clone(),
                        declaration.name().policy(),
                    )
                }))
                .collect(),
            original_publications: self.original_publication_dependencies(),
            original_world_closed: self.original_command_world.is_some(),
            original_source_procedure_candidates: self
                .original_procedure_source_candidates()
                .into_iter()
                .map(|(declaration, slot, policy)| {
                    (slot.clone(), policy, declaration.name().slot().clone())
                })
                .collect(),
            original_source_class_candidates: self
                .original_class_source_candidates()
                .into_iter()
                .map(|(declaration, slot, policy, obligations)| {
                    (
                        slot.clone(),
                        policy,
                        declaration.name().slot().clone(),
                        obligations.to_vec(),
                    )
                })
                .collect(),
            sources: self.sources.clone(),
            // Read only through this document's own `source` rows, and they
            // carry the source image, which every body edit changes.
            path_constant_assignments: if self.sources.is_empty() {
                tcl_compiler::auto_path_eval::PathConstantAssignments::default()
            } else {
                self.path_constant_assignments.clone()
            },
            package_requires: self.package_requires.clone(),
            package_provides: self.package_provides.clone(),
            package_ifneededs: self.package_ifneededs.clone(),
            package_prefers: self.package_prefers.clone(),
            command_links: self.command_links.clone(),
            glob_imports: self.glob_imports.clone(),
            original_namespace_patterns: self.original_namespace_patterns.clone(),
            namespace_exports: self.namespace_exports.clone(),
            namespace_forgets: self.namespace_forgets.clone(),
            command_deletions: self.command_deletions.clone(),
            command_retirements: self.command_retirements.clone(),
        }
    }

    fn original_publication_dependencies(&self) -> Vec<OriginalPublicationDependency> {
        self.original_command_world
            .iter()
            .flat_map(tcl_compiler::command_binding::OriginalCompletedCommandWorld::declarations)
            .map(|publication| {
                let site = publication
                    .definition()
                    .map(|definition| &definition.allocation().site);
                let origin = self
                    .original_procs
                    .iter()
                    .find(|declaration| Some(declaration.declaration_site()) == site)
                    .map(|declaration| {
                        (
                            declaration.name().slot().clone(),
                            declaration.name().policy(),
                        )
                    })
                    .or_else(|| {
                        self.original_classes
                            .iter()
                            .find(|declaration| Some(declaration.declaration_site()) == site)
                            .map(|declaration| {
                                (
                                    declaration.name().slot().clone(),
                                    declaration.name().policy(),
                                )
                            })
                    });
                OriginalPublicationDependency {
                    slot: publication.slot().clone(),
                    policy: publication.policy(),
                    kind: publication.kind(),
                    definition: origin,
                }
            })
            .collect()
    }

    pub(super) fn original_procedure_source_candidates(
        &self,
    ) -> Vec<(
        &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ProcDef,
        >,
        &tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
    )> {
        let Some(context) = self.diagnostic_source.as_ref() else {
            return Vec::new();
        };
        if let Some(world) = self.original_command_world.as_ref() {
            world
                .declarations()
                .flat_map(|publication| {
                    self.original_procs.iter().filter_map(move |declaration| {
                        (publication.declaration_site() == declaration.declaration_site()
                            && publication.policy() == declaration.name().policy())
                        .then_some((declaration, publication.slot(), publication.policy()))
                    })
                })
                .collect()
        } else if let Some(inventory) = self.original_source_procedures.as_ref() {
            inventory
                .candidates(self.original_procs.iter())
                .into_iter()
                .map(|candidate| {
                    (
                        candidate.declaration(),
                        candidate.source_slot(),
                        candidate.policy(),
                    )
                })
                .collect()
        } else {
            self.original_procs
                .iter()
                .filter(|declaration| {
                    declaration.name_input().source_image() == context.image()
                        && declaration.name_input().lexer_config() == context.config()
                })
                .map(|declaration| {
                    (
                        declaration,
                        declaration.name().slot(),
                        declaration.name().policy(),
                    )
                })
                .collect()
        }
    }

    pub(super) fn original_class_source_candidates(
        &self,
    ) -> Vec<(
        &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ClassDef,
        >,
        &tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
        &[tcl_compiler::command_binding::SourceCommandTransitionObligation],
    )> {
        if self.diagnostic_source.is_none() {
            return Vec::new();
        }
        if let Some(world) = self.original_command_world.as_ref() {
            world
                .declarations()
                .flat_map(|publication| {
                    self.original_classes.iter().filter_map(move |declaration| {
                        (publication.declaration_site() == declaration.declaration_site()
                            && publication.policy() == declaration.name().policy())
                        .then_some((
                            declaration,
                            publication.slot(),
                            publication.policy(),
                            &[][..],
                        ))
                    })
                })
                .collect()
        } else {
            self.original_source_classes
                .as_ref()
                .map_or_else(Vec::new, |inventory| {
                    inventory
                        .candidates_from_retained_records(
                            self.original_source_input.as_ref(),
                            self.original_classes.iter(),
                        )
                        .into_iter()
                        .map(|candidate| {
                            (
                                candidate.declaration(),
                                candidate.source_slot(),
                                candidate.policy(),
                                candidate.obligations(),
                            )
                        })
                        .collect()
                })
        }
    }
}
