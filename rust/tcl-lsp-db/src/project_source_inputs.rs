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

//! Body-free project source headers and factories from checked document inputs.

use super::{
    AnalyserConfig, AnalysisResult, ItemSig, ItemTree, OriginalCommandSlot, Project,
    ProjectSourceInputMode, SourceFile, TclDb, document_analyser_without_factory_oracle,
    document_analysis_input, file_analysis_incremental, merge_class_factories,
    merge_original_command_signatures, original_lookup_command_signatures_using,
    project_call_inputs, project_callback_diagnostics_with_mode, project_token_inputs,
    select_original_command_signatures,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[cfg(test)]
use super::{
    DiagCode, LogicalSourceCommandSlot, NonAsciiMode, TclDatabase, file_analysis, item_tree,
    logical_source_command_arity, project_class_factories, project_diagnostics,
    project_diagnostics_for_inputs, set_overlay_epoch,
};

/// Structure-only declaration inventory under this file's complete source input.
/// The published factory oracle is a source hint, not a reached metaclass.
#[salsa::tracked(returns(clone))]
pub fn item_tree_for_config(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<ItemTree> {
    project_call_inputs::file_source_structure(db, file, config).map_or_else(
        || Arc::new(ItemTree::default()),
        |facts| Arc::clone(&facts.items),
    )
}

/// Body-free source signatures share the existing item/header projection.
#[salsa::tracked(returns(clone))]
pub fn item_sigs_for_config(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<Vec<ItemSig>> {
    Arc::new(item_tree_for_config(db, file, config).sigs())
}

/// Preserve unavailable source ownership across the body-free header firewall.
#[salsa::tracked(returns(clone))]
pub(super) fn checked_item_sigs_for_config(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<Arc<Vec<ItemSig>>> {
    project_call_inputs::file_source_structure(db, file, config)?;
    Some(item_sigs_for_config(db, file, config))
}

fn project_signatures(db: &dyn TclDb, project: Project) -> Option<Vec<Arc<Vec<ItemSig>>>> {
    project
        .files(db)
        .iter()
        .map(|&file| {
            let config = project_token_inputs::configuration_for_file(db, project, file)?;
            checked_item_sigs_for_config(db, file, config)
        })
        .collect()
}

#[salsa::tracked(returns(clone))]
fn checked_class_factories_for_config(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<Arc<tcl_compiler::analyser::ClassFactoryIndex>> {
    project_call_inputs::file_source_structure(db, file, config)?;
    Some(Arc::clone(
        &item_tree_for_config(db, file, config).class_factories,
    ))
}

/// Capture headers without a published factory oracle: the retained source
/// owner cannot contain a previous generation of the census it produces.
#[salsa::tracked(returns(clone))]
fn file_factory_headers(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<tcl_compiler::registry_invocation::source_structure::OriginalProjectFactoryHeaders> {
    let input = document_analysis_input(db, file, config).ok()?;
    let _sidecar_stubs_epoch = file.sidecar_stubs_epoch(db);
    let mut analyser = document_analyser_without_factory_oracle(db, file, config)
        .with_resolved_input((*input).clone())
        .structure_only();
    let result = analyser.analyse(file.text(db), file.dialect(db));
    tcl_compiler::registry_invocation::source_structure::OriginalProjectFactoryHeaders::capture(
        &result,
    )
}

/// Every member must retain its checked source owner before hints are merged.
/// Missing mappings/generations withdraw hints without a recursive query cycle.
#[salsa::tracked(returns(clone))]
pub fn project_class_factories_for_inputs(
    db: &dyn TclDb,
    project: Project,
) -> Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    let mut providers = HashSet::new();
    let parts = project
        .files(db)
        .iter()
        .filter(|&&file| providers.insert(file))
        .map(|&file| {
            let config = project_token_inputs::configuration_for_file(db, project, file)?;
            checked_class_factories_for_config(db, file, config)
        })
        .collect::<Option<Vec<_>>>();
    merge_class_factories(parts.unwrap_or_default())
}

/// Complete source-header capture for an explicitly requested project snapshot.
/// The returned cards retain conditional source evidence only; callers must
/// check the snapshot is current and account for unrepresented occupants.
#[salsa::tracked(returns(clone))]
pub fn project_factory_header_census_for_inputs(
    db: &dyn TclDb,
    project: Project,
) -> Option<
    Arc<
        Vec<(
            u64,
            tcl_compiler::registry_invocation::source_structure::OriginalProjectFactoryHeaders,
        )>,
    >,
> {
    use salsa::plumbing::AsId as _;
    let mut seen = HashSet::new();
    let mut headers = project
        .files(db)
        .iter()
        .filter(|&&file| seen.insert(file))
        .map(|&file| {
            let config = project_token_inputs::configuration_for_file(db, project, file)?;
            Some((
                file.as_id().as_bits(),
                file_factory_headers(db, file, config)?,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    headers.sort_unstable_by_key(|(identity, _)| *identity);
    Some(Arc::new(headers))
}

/// Genuine original source headers, independent of body entry and publication.
#[salsa::tracked(returns(clone))]
pub fn project_original_command_signatures_for_inputs(
    db: &dyn TclDb,
    project: Project,
) -> Arc<
    HashMap<
        tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        Vec<tcl_compiler::analyser::SourceDeclarationSignature>,
    >,
> {
    merge_original_command_signatures(project_signatures(db, project).unwrap_or_default())
}

#[salsa::tracked(returns(clone))]
pub(super) fn original_command_signatures_for_inputs<'db>(
    db: &'db dyn TclDb,
    project: Project,
    name: OriginalCommandSlot<'db>,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    let table = project_original_command_signatures_for_inputs(db, project);
    select_original_command_signatures(db, name, &table)
}

pub(super) fn original_lookup_command_signatures_for_inputs(
    db: &dyn TclDb,
    project: Project,
    lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    original_lookup_command_signatures_using(db, lookup, |key| {
        original_command_signatures_for_inputs(db, project, key)
    })
}

/// Project callback projection with checked per-library source headers.
/// The actual callback subject keeps its own registration/name/count owner;
/// library source metadata grants no installed callback or entered frame.
#[must_use]
pub fn project_callback_diagnostics_for_analysis_with_inputs(
    db: &dyn TclDb,
    project: Project,
    source: &str,
    analysis: &AnalysisResult,
    is_disabled: impl Fn(&str) -> bool,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    project_callback_diagnostics_with_mode(
        (db, project, ProjectSourceInputMode::Supplied),
        source,
        analysis,
        is_disabled,
    )
}

/// Tracked supplied-document callback projection for the Server push path.
#[salsa::tracked(returns(clone))]
pub fn project_callback_diagnostics_for_inputs(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
    project: Project,
) -> Arc<Vec<tcl_compiler::analyser::Diagnostic>> {
    let analysis = file_analysis_incremental(db, file, config);
    Arc::new(project_callback_diagnostics_for_analysis_with_inputs(
        db,
        project,
        file.text(db),
        &analysis,
        |code| {
            config
                .disabled_diagnostics(db)
                .iter()
                .any(|disabled| disabled == code)
        },
    ))
}

#[cfg(test)]
mod tests;
