// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Checked project source topology and caller evidence.

use super::*;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct FileSourceStructure {
    pub items: Arc<ItemTree>,
    targets: Arc<Vec<FileSourceTarget>>,
    declared: Arc<DeclaredSurface>,
}

/// One structure-only producer supplies cards, source sites and declared roles.
/// A missing actual input remains distinct from a successfully empty inventory.
#[salsa::tracked(returns(clone))]
pub(super) fn file_source_structure(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<Arc<FileSourceStructure>> {
    let input = document_analysis_input(db, file, config).ok()?;
    let _sidecar_stubs_epoch = file.sidecar_stubs_epoch(db);
    let mut analyser = document_analyser(db, file, config)
        .with_resolved_input((*input).clone())
        .structure_only();
    let result = analyser.analyse(file.text(db), file.dialect(db));
    if result.resolved_input.as_ref() != Some(input.as_ref())
        || !result.matches_original_source_image(
            &tcl_lexer::SourceImage::document(file.text(db)),
            input.lexer_config(),
        )
    {
        return None;
    }
    Some(Arc::new(FileSourceStructure {
        items: Arc::new(ItemTree::from_analysis(
            &result,
            &analyser.ensemble_namespaces,
        )),
        targets: Arc::new(
            result
                .source_targets
                .iter()
                .map(|site| FileSourceTarget {
                    raw_path: site.raw_path.clone(),
                    is_literal: site.is_literal,
                })
                .collect(),
        ),
        declared: Arc::new(tcl_compiler::analyser::types::build_declared_surface(
            &result.stub_commands,
        )),
    }))
}

fn checked_file(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> Option<(AnalyserConfig, Arc<FileSourceStructure>)> {
    let config = project_token_inputs::configuration_for_file(db, project, file)?;
    Some((config, file_source_structure(db, file, config)?))
}

/// Source-site coverage under this file's actual published input.
/// `None` is unavailable coverage, not an empty list of loaded units.
#[salsa::tracked(returns(clone))]
pub fn file_source_targets_for_inputs(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> Option<Arc<Vec<FileSourceTarget>>> {
    Some(Arc::clone(&checked_file(db, file, project)?.1.targets))
}

#[salsa::tracked(returns(clone))]
fn file_decls_for_inputs(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> Option<Arc<FileDecls>> {
    Some(Arc::new(
        checked_file(db, file, project)?.1.items.file_decls(),
    ))
}

#[salsa::tracked(returns(clone))]
fn project_proc_names_for_inputs(
    db: &dyn TclDb,
    project: Project,
) -> Option<Arc<BTreeSet<String>>> {
    let mut names = BTreeSet::new();
    for &file in project.files(db) {
        names.extend(
            file_decls_for_inputs(db, file, project)?
                .procs
                .iter()
                .cloned(),
        );
    }
    Some(Arc::new(names))
}

#[salsa::tracked(returns(clone))]
fn project_dispatch_components_for_inputs(
    db: &dyn TclDb,
    project: Project,
) -> Option<Arc<DispatchComponents>> {
    let rows = project
        .files(db)
        .iter()
        .map(|&file| {
            let targets = file_source_targets_for_inputs(db, file, project)?;
            let path = file.path(db).clone();
            let links = path.as_deref().map_or_else(BTreeSet::new, |path| {
                targets
                    .iter()
                    .filter(|site| site.is_literal)
                    .map(|site| {
                        tcl_lsp_core::source_graph::resolve_source_target(
                            std::path::Path::new(path),
                            &site.raw_path,
                        )
                        .to_string_lossy()
                        .into_owned()
                    })
                    .collect()
            });
            Some((
                path,
                Arc::new(links),
                file_decls_for_inputs(db, file, project)?,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(merge_dispatch_components(rows))
}

#[salsa::tracked(returns(clone))]
fn file_call_site_evidence_for_inputs(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> Option<Arc<CallSiteEvidence>> {
    let (config, structure) = checked_file(db, file, project)?;
    let input = document_analysis_input(db, file, config).ok()?;
    let known = project_proc_names_for_inputs(db, project)?
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let components = project_dispatch_components_for_inputs(db, project)?;
    let index = project
        .files(db)
        .iter()
        .position(|&member| member == file)?;
    let reach = components.reach(index)?.iter().cloned().collect::<Vec<_>>();
    let context = input.context_registry();
    let entry = tcl_compiler::command_binding::SourceAnalysisEntry::for_supplied_source(
        context.commands(),
        &input,
        input.lexer_config(),
        Some(input.unit_profile()),
    );
    let scanned = tcl_compiler::unit_scope::scan_source_call_sites_with_source_input(
        file.text(db),
        Some(&structure.declared),
        &known,
        &reach,
        &input,
        &entry,
    );
    let declarations = file_decls_for_inputs(db, file, project)?;
    Some(Arc::new(
        scanned.slice_for(
            scanned
                .callees()
                .filter(|name| !declarations.procs.contains(*name)),
        ),
    ))
}

#[salsa::tracked(returns(clone))]
fn project_call_site_evidence_for_inputs(
    db: &dyn TclDb,
    project: Project,
) -> Option<Arc<CallSiteEvidence>> {
    let mut all = CallSiteEvidence::default();
    for &file in project.files(db) {
        all.merge_from(&file_call_site_evidence_for_inputs(db, file, project)?);
    }
    Some(Arc::new(all))
}

/// A closed project caller table requires every contributor's checked input.
/// Missing source metadata withdraws the table instead of asserting no callers.
#[salsa::tracked(returns(clone))]
pub fn file_external_call_sites_for_inputs(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> Option<Arc<CallSiteEvidence>> {
    let declarations = file_decls_for_inputs(db, file, project)?;
    let all = project_call_site_evidence_for_inputs(db, project)?;
    Some(Arc::new(
        all.slice_for(declarations.procs.iter().map(String::as_str)),
    ))
}

#[cfg(test)]
mod tests;
