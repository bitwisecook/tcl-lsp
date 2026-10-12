// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Project token facts from checked per-file document inputs.

use super::*;

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
    let Ok(input) = document_analysis_input(db, file, config) else {
        return Arc::new(FileTokenFacts::default());
    };
    let result = document_analyser(db, file, config)
        .with_resolved_input((*input).clone())
        .structure_only()
        .analyse(file.text(db), file.dialect(db));
    if !result.matches_original_source_image(
        &tcl_lexer::SourceImage::document(file.text(db)),
        input.lexer_config(),
    ) || result.resolved_input.as_ref() != Some(input.as_ref())
    {
        return Arc::new(FileTokenFacts::default());
    }
    token_facts_from_analysis(result)
}

fn project_facts(
    db: &dyn TclDb,
    project: Project,
) -> impl Iterator<Item = Arc<FileTokenFacts>> + '_ {
    project.files(db).iter().filter_map(move |&file| {
        let config = configuration_for_file(db, project, file)?;
        Some(file_token_facts_for_config(db, file, config))
    })
}

/// Class source assistance merged from each member's actual input.
/// Missing mappings and unavailable source generations contribute no cards.
#[salsa::tracked(returns(clone))]
pub fn project_class_index_for_inputs(db: &dyn TclDb, project: Project) -> Arc<ClassHierarchy> {
    merge_project_classes(project_facts(db, project))
}

/// Inferred source parameter roles merged on the structure-only tier.
#[salsa::tracked(returns(clone))]
pub fn project_proc_var_index_for_inputs(db: &dyn TclDb, project: Project) -> Arc<VarNameArgRoles> {
    merge_project_proc_roles(project_facts(db, project))
}

/// Conditional named-instance hints, without Native allocation or dispatch.
#[salsa::tracked(returns(clone))]
pub fn project_named_instance_index_for_inputs(
    db: &dyn TclDb,
    project: Project,
) -> Arc<tcl_lsp_core::semantic_tokens::NamedInstanceMap> {
    merge_project_named_instances(project_facts(db, project))
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
    let proc_roles = project_proc_var_index_for_inputs(db, project);
    let named_instances = project_named_instance_index_for_inputs(db, project);
    tcl_lsp_core::semantic_tokens::full_with_cu_and_facts(
        file.text(db),
        input.analyser_profile(),
        registry,
        cu.as_deref(),
        tcl_lsp_core::semantic_tokens::WorkspaceTokenFacts {
            classes: Some(&classes),
            proc_roles: Some(&proc_roles),
            named_instances: Some(&named_instances),
            analysis: Some(&analysis),
        },
    )
}

#[cfg(test)]
mod tests;
