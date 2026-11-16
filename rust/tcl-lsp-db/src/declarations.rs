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

//! Declaration queries: original procedure facts, the item tree and its signatures, declared procedure names and class factories.

use super::document_analysis::{file_analysis, walked_document_input};
use super::{AnalyserConfig, Project, SourceFile, TclDb, project_token_inputs};
use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, FileDecls, ItemSig, ItemTree};

/// Complete current source owner for independently retained procedure headers.
/// Declarations supply source correspondence, never loaded implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOriginalProcedureFacts {
    pub(super) input: tcl_compiler::analyser::ResolvedAnalysisInput,
    image: tcl_lexer::SourceImage,
    declarations: Vec<
        tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ProcDef,
        >,
    >,
}

impl FileOriginalProcedureFacts {
    #[must_use]
    pub fn input(&self) -> &tcl_compiler::analyser::ResolvedAnalysisInput {
        &self.input
    }

    #[must_use]
    pub fn image(&self) -> &tcl_lexer::SourceImage {
        &self.image
    }

    #[must_use]
    pub fn declarations(
        &self,
    ) -> &[tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ProcDef,
    >] {
        &self.declarations
    }
}

#[salsa::tracked(returns(clone))]
pub fn file_original_procedure_facts(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<Arc<FileOriginalProcedureFacts>> {
    let input = walked_document_input(db, file, config).ok()?;
    let analysis = file_analysis(db, file, config);
    let (image, source_config) =
        tcl_compiler::source_graph::current_analysis(file.text(db), &analysis)?;
    if analysis.resolved_input.as_ref() != Some(input.as_ref())
        || source_config != input.lexer_config()
    {
        return None;
    }
    let context = input.context_registry();
    tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
        context.commands(),
        &input,
        source_config,
        Some(input.unit_profile()),
    )?;
    let declarations = analysis
        .original_procedure_declarations()
        .filter(|declaration| {
            declaration.name_input().source_image() == &image
                && declaration.name_input().lexer_config() == source_config
        })
        .cloned()
        .collect();
    Some(Arc::new(FileOriginalProcedureFacts {
        input: (*input).clone(),
        image,
        declarations,
    }))
}

/// Each configured file retains its own source declarations and input owner.
/// Missing mappings remain unavailable beside the project's label roster.
#[salsa::tracked(returns(clone))]
pub fn project_original_procedure_facts(
    db: &dyn TclDb,
    project: Project,
) -> Arc<Vec<(SourceFile, Arc<FileOriginalProcedureFacts>)>> {
    Arc::new(
        project
            .files(db)
            .iter()
            .filter_map(|&file| {
                let config = project_token_inputs::configuration_for_file(db, project, file)?;
                Some((file, file_original_procedure_facts(db, file, config)?))
            })
            .collect(),
    )
}

/// Explicit standalone item inventory, keyed by stable declaration name/kind.
/// Configured document providers use [`item_tree_for_config`] so selected
/// pack definitions, source grammar and available generations stay attached.
///
/// `ensemble_namespaces` is retained on the analyser, so this structure-only
/// query reads the item/header projection directly from that same source walk.
/// The published workspace factory oracle remains an input to the file.
#[salsa::tracked(returns(clone))]
pub fn item_tree(db: &dyn salsa::Database, file: SourceFile) -> Arc<ItemTree> {
    // `structure_only` skips diagnostic emission (the dominant analyse cost)
    // while building the identical declaration/scope structure — a cheap,
    // non-divergent item extractor (gated by `file_decls_corpus`).
    let mut analyser = Analyser::new()
        .structure_only()
        // The workspace class-factory oracle is part of the *file's* input,
        // not the analyser config, so the item tree stays a
        // function of `SourceFile` alone. It has to be here: a class a
        // cross-file metaclass manufactures is a declaration of this file, and
        // leaving it out of the item tree would leave it out of `file_decls`
        // too — so cross-file W123 would call `IconList` an unknown command
        // while the full analysis, hover, and the outline all describe it.
        .with_workspace_class_factories(file.workspace_class_factories(db).clone());
    let result = analyser.analyse(file.text(db), file.dialect(db));
    Arc::new(ItemTree::from_analysis(
        &result,
        &analyser.ensemble_namespaces,
    ))
}

/// Explicit standalone item signatures — cross-item headers with bodies stripped
/// (`item_sig*` in the design graph). A body-only edit leaves these equal, so
/// [`file_decls`] and the future cross-item passes early-cutoff.
#[salsa::tracked(returns(clone))]
pub fn item_sigs(db: &dyn salsa::Database, file: SourceFile) -> Arc<Vec<ItemSig>> {
    Arc::new(item_tree(db, file).sigs())
}

/// Aggregate declaration sets (`file_decls ← item_sig*`): the set of declared
/// procs / classes / aliases / ensembles + the namespace tree the cross-item
/// passes (W123 / arity) read read-only.
#[salsa::tracked(returns(clone))]
pub fn file_decls(db: &dyn salsa::Database, file: SourceFile) -> Arc<FileDecls> {
    Arc::new(FileDecls::from_sigs(item_sigs(db, file).iter()))
}

/// The project-wide set of declared `proc` qualified names — the cross-file
/// command-resolution domain (e.g. W123 unresolved-command suppression against
/// the workspace's procs).  The salsa-native replacement for `WorkspaceIndex`'s
/// proc-name set: it lifts the project signature table into salsa.
///
/// Depends **only** on each file's [`file_decls`] — the signature firewall — so a
/// **body edit in any file leaves it unchanged** → it backdates → **zero
/// cross-file work**; only a signature/decl change (a proc added / removed /
/// renamed) recomputes it. This is the input discipline extended across
/// files: a keystroke that does not alter a signature wakes nobody project-wide.
/// Proven by `project_proc_names_firewall` (a body edit re-runs zero
/// `project_proc_names`; a decl change re-runs exactly one).
#[salsa::tracked(returns(clone))]
pub fn project_proc_names(db: &dyn salsa::Database, project: Project) -> Arc<BTreeSet<String>> {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for &file in project.files(db) {
        names.extend(file_decls(db, file).procs.iter().cloned());
    }
    Arc::new(names)
}

/// The **class factories** one file declares — the user-defined `TclOO`
/// metaclasses (`oo::class create Meta { superclass oo::class; self method
/// create … }`) it publishes to the rest of the workspace.
///
/// Read straight off [`item_tree`], which every project file already
/// computes for [`file_decls`] / [`project_proc_names`], so the factory index
/// costs no extra analysis pass. Structure-level, like [`file_decls`]: a body
/// edit that does not change a metaclass's `create` override leaves this
/// equal, so it backdates and no cross-file query re-runs.
///
/// [`item_tree`] itself reads [`SourceFile::workspace_class_factories`], so
/// this query is a function of the very input the host computes *from* it. A
/// metaclass that is **itself** manufactured by another file's metaclass is
/// therefore not provable until the manufacturing metaclass has already been
/// published: the file holding `MetaA create MetaB` proves `MetaB` only on a
/// round whose index already names `MetaA`. One publish is consequently one
/// link of the chain deep, and no more — so the host must **iterate the
/// publish to a fixpoint**, not publish once: a single pass would leave a
/// cross-file `MetaA` → `MetaB` → `Widget` chain's `Widget` unknown, with a
/// call site on one of its methods resolving to nothing.
///
/// The fixpoint is cheap and terminates: a round adds an entry only when some
/// file *proved* it — never a guess — so the round function is normally
/// monotone over a set bounded by the project's creation calls, and the host
/// stops as soon as a round moves nothing. A workspace with no metaclass at
/// all computes an empty index, which the host stores as `None`, so it settles
/// in one round that writes nothing and invalidates nothing. The host caps the
/// loop regardless rather than assuming convergence — see
/// `sync_workspace_class_factories` in `tcl-lsp-server`.
#[salsa::tracked(returns(clone))]
pub fn file_class_factories(
    db: &dyn salsa::Database,
    file: SourceFile,
) -> Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    Arc::clone(&item_tree(db, file).class_factories)
}

/// The project-wide merge of every file's [`file_class_factories`] — the
/// oracle the server sets on [`SourceFile::workspace_class_factories`].
///
/// Unordered, exactly as the workspace index's other cross-file facts are: a
/// `source` graph can prove a load order, but nothing requires one here —
/// a metaclass declaration either exists in the project or it does not.
/// Duplicate qualified names retain no unique provider and withdraw that hint.
#[salsa::tracked(returns(clone))]
pub fn project_class_factories(
    db: &dyn salsa::Database,
    project: Project,
) -> Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    let mut providers = HashSet::new();
    merge_class_factories(
        project
            .files(db)
            .iter()
            .filter(|&&file| providers.insert(file))
            .map(|&file| file_class_factories(db, file)),
    )
}

pub(super) fn merge_class_factories(
    parts: impl IntoIterator<Item = Arc<tcl_compiler::analyser::ClassFactoryIndex>>,
) -> Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    let mut merged = tcl_compiler::analyser::ClassFactoryIndex::new();
    let mut ambiguous = HashSet::new();
    for part in parts {
        for (qname, factory) in part.iter() {
            if ambiguous.contains(qname) {
                continue;
            }
            match merged.entry(qname.clone()) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(factory.clone());
                }
                std::collections::btree_map::Entry::Occupied(entry) => {
                    entry.remove();
                    ambiguous.insert(qname.clone());
                }
            }
        }
    }
    Arc::new(merged)
}
