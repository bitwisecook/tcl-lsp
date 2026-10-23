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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Detached Salsa projection used for source proposals.
//!
//! A proposal is analysed in a new database. Salsa database clones share
//! storage, so changing a cloned live input would mutate the open document.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use salsa::Setter as _;
use tcl_compiler::analyser::{ClassFactoryIndex, NonAsciiMode, SubclassProvidedMethods};
use tcl_compiler::unit_scope::CallSiteEvidence;
use tcl_lsp_db::{AnalyserConfig, Project, SourceFile, TclDatabase};
use tower_lsp_server::ls_types::Uri;

/// Every selected Salsa source input, including the workspace evidence fields.
/// `text` is replaced only for `selected_uri`; the other rows are copied from
/// one retained source snapshot.
pub(super) struct SourceInput {
    pub(super) uri: Uri,
    pub(super) text: String,
    pub(super) dialect: String,
    pub(super) path: Option<String>,
    /// Actual admitted path for source-graph resolution. Never decode the URI
    /// to recover this provider coordinate.
    pub(super) source_path: Option<PathBuf>,
    pub(super) external_call_sites: Option<Arc<CallSiteEvidence>>,
    pub(super) workspace_class_factories: Option<Arc<ClassFactoryIndex>>,
    pub(super) workspace_subclass_methods: Option<Arc<SubclassProvidedMethods>>,
    pub(super) sidecar_stubs_epoch: u64,
    pub(super) config: ConfigInput,
}

/// The complete actual configuration input for one source row.
pub(super) struct ConfigInput {
    pub(super) disabled_diagnostics: Vec<String>,
    pub(super) non_ascii_mode: NonAsciiMode,
    pub(super) extra_commands: Vec<String>,
    pub(super) generic_variable_patterns: Option<Vec<String>>,
    pub(super) bigip_version: Option<String>,
    pub(super) spec_pack_key: u64,
    pub(super) targets: Vec<(String, String)>,
    pub(super) package_provides: Vec<(String, Vec<String>)>,
}

/// The isolated database and all remapped input handles for a project.
pub(super) struct Projection {
    pub(super) db: Arc<super::TrackedMutex<TclDatabase>>,
    pub(super) files: Vec<(Uri, SourceFile)>,
    pub(super) members: HashSet<Uri>,
    pub(super) project: Project,
    pub(super) selected: SourceFile,
    pub(super) selected_config: AnalyserConfig,
    pub(super) configurations: HashMap<SourceFile, AnalyserConfig>,
    pub(super) source_paths: HashMap<Uri, PathBuf>,
}

/// A projection cannot claim the selected resource when its source row or its
/// per-file configuration mapping is absent or duplicated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectionError {
    MissingSelectedSource,
    DuplicateSource,
    PathAdmissionMismatch,
}

/// Build a new database and remap the whole project into it. The caller must
/// supply rows captured from one coherent original project snapshot. Cross-file
/// evidence is copied only as an intermediate input; the proposal producer
/// must recompute it against the projected project before running diagnostics.
pub(super) fn build(
    rows: Vec<SourceInput>,
    selected_uri: &Uri,
    proposal_text: &str,
    evaluator_epoch: u64,
    overlay_epoch: u64,
) -> Result<Projection, ProjectionError> {
    let mut seen = HashSet::with_capacity(rows.len());
    for row in &rows {
        if !seen.insert(row.uri.clone()) {
            return Err(ProjectionError::DuplicateSource);
        }
        if row.path.is_some() != row.source_path.is_some() {
            return Err(ProjectionError::PathAdmissionMismatch);
        }
    }
    if !seen.contains(selected_uri) {
        return Err(ProjectionError::MissingSelectedSource);
    }

    let mut db = TclDatabase::default();
    tcl_lsp_db::set_evaluator_epoch(&mut db, evaluator_epoch);
    tcl_lsp_db::set_overlay_epoch(&mut db, overlay_epoch);
    let mut files = Vec::with_capacity(rows.len());
    let mut members = HashSet::with_capacity(rows.len());
    let mut source_paths = HashMap::with_capacity(rows.len());
    let mut configurations = HashMap::with_capacity(rows.len());
    let mut project_files = Vec::with_capacity(rows.len());
    let mut token_configurations = Vec::with_capacity(rows.len());
    let mut selected = None;
    let mut selected_config = None;

    for mut row in rows {
        if &row.uri == selected_uri {
            row.text.clear();
            row.text.push_str(proposal_text);
        }
        let file = SourceFile::with_call_site_evidence(
            &db,
            row.text,
            row.dialect,
            row.path,
            row.external_call_sites,
            row.workspace_class_factories,
            row.workspace_subclass_methods,
            row.sidecar_stubs_epoch,
        );
        let config = AnalyserConfig::new(
            &db,
            row.config.disabled_diagnostics,
            row.config.non_ascii_mode,
            row.config.extra_commands,
            row.config.generic_variable_patterns,
            row.config.bigip_version,
            row.config.spec_pack_key,
            row.config.targets,
            row.config.package_provides,
        );
        configurations.insert(file, config);
        if &row.uri == selected_uri {
            selected = Some(file);
            selected_config = Some(config);
        }
        members.insert(row.uri.clone());
        if let Some(path) = row.source_path {
            source_paths.insert(row.uri.clone(), path);
        }
        files.push((row.uri, file));
        project_files.push(file);
        token_configurations.push((file, config));
    }

    let project =
        Project::with_token_configurations(&db, project_files, Some(token_configurations));
    Ok(Projection {
        db: Arc::new(super::TrackedMutex::new("proposal_db", db)),
        files,
        members,
        project,
        selected: selected.ok_or(ProjectionError::MissingSelectedSource)?,
        selected_config: selected_config.ok_or(ProjectionError::MissingSelectedSource)?,
        configurations,
        source_paths,
    })
}

/// Require the shared original syntax-coverage owner before interpreting an
/// empty source-target result as closed project coverage. Syntax completeness
/// grants no call graph or dispatch closure; those remain with the existing
/// project source-target and call-site evidence owners below.
fn project_source_is_complete(
    db: &TclDatabase,
    file: SourceFile,
    project: Project,
    configurations: &HashMap<SourceFile, AnalyserConfig>,
) -> bool {
    let Some(&config) = configurations.get(&file) else {
        return false;
    };
    let text = file.text(db);
    let analysis = tcl_lsp_db::file_analysis_incremental(db, file, config);
    let Some(coverage) = tcl_lsp_core::SourceSyntaxCoverage::capture(text, &analysis) else {
        return false;
    };
    if coverage.commands(text, &analysis).is_none()
        || !matches!(
            coverage.applicability(text, &analysis),
            Some(tcl_lsp_core::SourceSyntaxCoverageApplicability::Complete)
        )
    {
        return false;
    }
    tcl_lsp_db::file_source_targets_for_inputs(db, file, project).is_some()
}

/// Recompute project-wide metaclass evidence on blocking workers, then apply
/// only the short Salsa input writes on the async task.
pub(super) async fn recompute_class_factories(projection: &mut Projection) -> bool {
    for _ in 0..super::CLASS_FACTORY_SYNC_ROUNDS {
        let snapshot = projection.db.snapshot("proposal_class_factory_plan").await;
        let project = projection.project;
        let files = projection.files.clone();
        let members = projection.members.clone();
        let configurations = projection.configurations.clone();
        let factories = crate::rt::spawn_blocking(move || {
            if files.iter().any(|(uri, file)| {
                !members.contains(uri)
                    || !project_source_is_complete(&snapshot, *file, project, &configurations)
                    || tcl_lsp_db::file_source_targets_for_inputs(&*snapshot, *file, project)
                        .is_none()
            }) {
                return None;
            }
            Some(tcl_lsp_db::project_class_factories_for_inputs(
                &*snapshot, project,
            ))
        })
        .await
        .ok()
        .flatten();
        let Some(factories) = factories else {
            return false;
        };
        let next = (!factories.is_empty()).then_some(factories);
        let mut db = projection.db.lock().await;
        let mut changed = false;
        for (_, file) in &projection.files {
            if file.workspace_class_factories(&*db).as_deref() != next.as_deref() {
                file.set_workspace_class_factories(&mut *db)
                    .to(next.clone());
                changed = true;
            }
        }
        drop(db);
        if !changed {
            return true;
        }
    }
    false
}

/// Recompute the project's source-call receipts using the same coverage rule
/// as the live index producer. An uncovered load target remains unavailable
/// (`None`) rather than manufacturing a complete empty caller set.
pub(super) async fn recompute_call_site_evidence(projection: &mut Projection) -> bool {
    let known_paths: HashSet<PathBuf> = projection
        .source_paths
        .iter()
        .filter(|(uri, _)| projection.members.contains(*uri))
        .map(|(_, path)| path.clone())
        .collect();
    let snapshot = projection.db.snapshot("proposal_call_site_plan").await;
    let files = projection.files.clone();
    let members = projection.members.clone();
    let source_paths = projection.source_paths.clone();
    let configurations = projection.configurations.clone();
    let project = projection.project;
    let evidence = crate::rt::spawn_blocking(move || {
        let mut covered = HashSet::new();
        for (uri, file) in &files {
            if !members.contains(uri)
                || !project_source_is_complete(&snapshot, *file, project, &configurations)
            {
                return None;
            }
            let targets = tcl_lsp_db::file_source_targets_for_inputs(&*snapshot, *file, project)?;
            let Some(parent) = source_paths.get(uri) else {
                if targets.is_empty() {
                    covered.insert(uri.clone());
                }
                continue;
            };
            if targets.iter().all(|site| {
                site.is_literal
                    && known_paths.contains(&tcl_lsp_core::source_graph::resolve_source_target(
                        parent,
                        &site.raw_path,
                    ))
            }) {
                covered.insert(uri.clone());
            }
        }
        Some(
            files
                .iter()
                .map(|(uri, file)| {
                    let value = covered
                        .contains(uri)
                        .then(|| {
                            tcl_lsp_db::file_external_call_sites_for_inputs(
                                &*snapshot, *file, project,
                            )
                        })
                        .flatten();
                    (*file, value)
                })
                .collect::<Vec<_>>(),
        )
    })
    .await
    .ok()
    .flatten();
    let Some(evidence) = evidence else {
        return false;
    };
    let mut db = projection.db.lock().await;
    for (file, value) in evidence {
        file.set_external_call_sites(&mut *db).to(value);
    }
    true
}

/// Copy subclass-method facts from the caller's hypothetical index into the
/// detached source inputs. The caller must replace the proposed URI row in
/// that hypothetical index with the analysis for this exact proposal before
/// invoking this projection step.
pub(super) async fn update_subclass_methods_from_index(
    projection: &mut Projection,
    index: &tcl_lsp_core::workspace_index::WorkspaceIndex,
) -> Option<bool> {
    let full = index.subclass_provided_methods();
    let mut methods_by_uri: HashMap<String, Vec<String>> = HashMap::new();
    for class in index.live_classes() {
        methods_by_uri
            .entry(class.uri.clone())
            .or_default()
            .push(class.qualified_name.clone());
    }
    let snapshot = projection.db.snapshot("proposal_subclass_plan").await;
    let files = projection.files.clone();
    let planned = crate::rt::spawn_blocking(move || {
        files
            .into_iter()
            .map(|(uri, file)| {
                let methods: SubclassProvidedMethods = methods_by_uri
                    .get(uri.as_str())
                    .into_iter()
                    .flatten()
                    .filter_map(|name| full.get(name).map(|rows| (name.clone(), rows.clone())))
                    .collect();
                let next = (!methods.is_empty()).then(|| Arc::new(methods));
                let previous = file.workspace_subclass_methods(&*snapshot);
                let equal = match (previous.as_ref(), next.as_ref()) {
                    (Some(previous), Some(next)) => {
                        Arc::ptr_eq(previous, next) || **previous == **next
                    }
                    (None, None) => true,
                    _ => false,
                };
                (file, next, !equal)
            })
            .collect::<Vec<_>>()
    })
    .await
    .ok();
    let planned = planned?;
    let mut db = projection.db.lock().await;
    let mut changed = false;
    for (file, next, needs_update) in planned {
        if needs_update {
            file.set_workspace_subclass_methods(&mut *db).to(next);
            changed = true;
        }
    }
    Some(changed)
}
