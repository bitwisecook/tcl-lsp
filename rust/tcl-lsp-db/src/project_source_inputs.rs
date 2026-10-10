// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Body-free project source headers and factories from checked document inputs.

use super::*;

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

fn project_signatures(
    db: &dyn TclDb,
    project: Project,
) -> impl Iterator<Item = Arc<Vec<ItemSig>>> + '_ {
    project.files(db).iter().filter_map(move |&file| {
        let config = project_token_inputs::configuration_for_file(db, project, file)?;
        Some(item_sigs_for_config(db, file, config))
    })
}

/// Factory hints use each member's checked input and published source oracle.
/// Missing mappings/generations withdraw hints without a recursive query cycle.
#[salsa::tracked(returns(clone))]
pub fn project_class_factories_for_inputs(
    db: &dyn TclDb,
    project: Project,
) -> Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    merge_class_factories(project.files(db).iter().filter_map(|&file| {
        let config = project_token_inputs::configuration_for_file(db, project, file)?;
        Some(Arc::clone(
            &item_tree_for_config(db, file, config).class_factories,
        ))
    }))
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
    merge_original_command_signatures(project_signatures(db, project))
}

#[salsa::tracked(returns(clone))]
fn project_command_arities_for_inputs(
    db: &dyn TclDb,
    project: Project,
) -> Arc<HashMap<String, Vec<(usize, usize)>>> {
    merge_command_arities(project_signatures(db, project))
}

#[salsa::tracked(returns(clone))]
pub(super) fn command_arity_for_inputs<'db>(
    db: &'db dyn TclDb,
    project: Project,
    name: CommandTail<'db>,
) -> Option<Arc<Vec<(usize, usize)>>> {
    project_command_arities_for_inputs(db, project)
        .get(name.name(db))
        .cloned()
        .map(Arc::new)
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
