// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Project token facts from checked per-file document inputs.

use super::{
    AnalyserConfig, ClassHierarchy, FileTokenFacts, Project, SemanticTokens, SourceFile, TclDb,
    VarNameArgRoles, document_analysis_input, document_compilation_unit_for,
    file_analysis_incremental, merge_project_classes, merge_project_named_instances,
    project_call_inputs::file_project_source_facts,
};
use std::collections::HashSet;
use std::sync::Arc;

#[cfg(test)]
use super::{
    NonAsciiMode, TclDatabase, project_class_index, semantic_tokens, semantic_tokens_project,
    set_overlay_epoch,
};

pub(super) fn configuration_for_file(
    db: &dyn TclDb,
    project: Project,
    file: SourceFile,
) -> Option<AnalyserConfig> {
    if !project.files(db).contains(&file) {
        return None;
    }
    let mut configurations = project
        .token_configurations(db)
        .as_ref()?
        .iter()
        .filter_map(|&(member, config)| (member == file).then_some(config));
    let first = configurations.next()?;
    configurations
        .all(|config| config == first)
        .then_some(first)
}

/// Structure-only source cards under this file's checked complete input.
/// Missing generations retain no cards and never enter deep analysis.
#[salsa::tracked(returns(clone))]
pub fn file_token_facts_for_config(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<FileTokenFacts> {
    checked_file_token_facts_for_config(db, file, config)
        .unwrap_or_else(|| Arc::new(FileTokenFacts::default()))
}

#[salsa::tracked(returns(clone))]
fn checked_file_token_facts_for_config(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<Arc<FileTokenFacts>> {
    Some(Arc::clone(
        &file_project_source_facts(db, file, config)?.tokens,
    ))
}

fn project_facts(db: &dyn TclDb, project: Project) -> Option<Vec<Arc<FileTokenFacts>>> {
    let mut providers = HashSet::new();
    project
        .files(db)
        .iter()
        .filter(|&&file| providers.insert(file))
        .map(|&file| {
            let config = configuration_for_file(db, project, file)?;
            checked_file_token_facts_for_config(db, file, config)
        })
        .collect()
}

/// Class source assistance merged from each member's actual input.
/// Missing mappings or unavailable source generations withdraw project cards.
#[salsa::tracked(returns(clone))]
pub fn project_class_index_for_inputs(db: &dyn TclDb, project: Project) -> Arc<ClassHierarchy> {
    merge_project_classes(project_facts(db, project).unwrap_or_default())
}

/// Source-role declarations and original formals under each checked input.
/// Caller selection remains the compiler's conditional original occurrence query.
#[salsa::tracked(returns(clone))]
pub fn project_proc_var_index_for_inputs(db: &dyn TclDb, project: Project) -> Arc<VarNameArgRoles> {
    let mut seen = HashSet::new();
    let providers = project
        .files(db)
        .iter()
        .filter(|&&file| seen.insert(file))
        .map(|&file| {
            let config = configuration_for_file(db, project, file)?;
            checked_file_source_roles_for_config(db, file, config)
        })
        .collect::<Option<Vec<_>>>();
    Arc::new(VarNameArgRoles::from_project_source_roles(
        providers
            .unwrap_or_default()
            .into_iter()
            .map(|provider| (*provider).clone())
            .collect(),
    ))
}

#[salsa::tracked(returns(clone))]
fn checked_file_source_roles_for_config(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<Arc<tcl_compiler::registry_invocation::source_structure::OriginalProjectProcedureRoles>>
{
    file_project_source_facts(db, file, config)?.roles.clone()
}

/// Conditional named-instance hints, without Native allocation or dispatch.
#[salsa::tracked(returns(clone))]
pub fn project_named_instance_index_for_inputs(
    db: &dyn TclDb,
    project: Project,
) -> Arc<tcl_lsp_core::semantic_tokens::NamedInstanceMap> {
    merge_project_named_instances(project_facts(db, project).unwrap_or_default())
}

/// Current document tokens with checked per-file cross-document source facts.
/// A missing current-file mapping or unavailable input returns no tokens.
#[salsa::tracked(returns(clone))]
pub fn semantic_tokens_project_for_inputs(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> SemanticTokens {
    let Some(config) = configuration_for_file(db, project, file) else {
        return SemanticTokens::default();
    };
    let Ok(input) = document_analysis_input(db, file, config) else {
        return SemanticTokens::default();
    };
    let registry = input.borrowed_context_registry().commands();
    let cu = document_compilation_unit_for(db, file, config);
    let analysis = file_analysis_incremental(db, file, config);
    if !analysis.matches_original_source_image(
        &tcl_lexer::SourceImage::document(file.text(db)),
        input.lexer_config(),
    ) || analysis.resolved_input.as_ref() != Some(input.as_ref())
    {
        return SemanticTokens::default();
    }
    let classes = project_class_index_for_inputs(db, project);
    let mut class_providers = HashSet::new();
    let class_cards = project
        .files(db)
        .iter()
        .filter(|&&member| member != file && class_providers.insert(member))
        .map(|&member| {
            let member_config = configuration_for_file(db, project, member)?;
            checked_file_token_facts_for_config(db, member, member_config)?
                .class_cards
                .clone()
        })
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default();
    let proc_roles = project_proc_var_index_for_inputs(db, project);
    let named_instances = project_named_instance_index_for_inputs(db, project);
    tcl_lsp_core::semantic_tokens::full_with_cu_and_facts(
        file.text(db),
        input.analyser_profile(),
        registry,
        cu.as_deref(),
        tcl_lsp_core::semantic_tokens::WorkspaceTokenFacts {
            classes: Some(&classes),
            project_classes: Some(&class_cards),
            proc_roles: Some(&proc_roles),
            named_instances: Some(&named_instances),
            analysis: Some(&analysis),
        },
    )
}

#[cfg(test)]
mod tests;
