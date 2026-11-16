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

//! The cross-file call view: `source` targets, the dispatch components they connect and the call-site evidence each file contributes.

use super::declarations::{
    file_decls, file_original_procedure_facts, project_original_procedure_facts, project_proc_names,
};
use super::declared_commands::declared_command_surface;
use super::document_analysis::walked_document_input;
use super::{Project, SourceFile, TclDb, project_token_inputs};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, FileDecls};
use tcl_compiler::unit_scope::CallSiteEvidence;

/// The coverage-relevant projection of one `source` site.
///
/// Original ownership remains attached; the derived link query keeps unchanged
/// project associations equal when original source geometry moves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSourceTarget {
    /// Genuine conditional current-interpreter path, independent of report text.
    pub original_source_load:
        Option<Arc<tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad>>,
    /// Verbatim path text, including any substitution markers.
    pub raw_path: String,
    /// Whether the path is a plain literal known at analysis time.
    pub is_literal: bool,
}

/// Every `source` target in the file's current Salsa revision.
///
/// Kept as a separate signature-level query so consumers that must distinguish
/// a literal target from a computed one share the same scan as
/// [`file_link_targets`] instead of consulting the independently-published
/// workspace index.
#[salsa::tracked(returns(clone))]
pub fn file_source_targets(db: &dyn TclDb, file: SourceFile) -> Arc<Vec<FileSourceTarget>> {
    let result = Analyser::new()
        .structure_only()
        .analyse(file.text(db), file.dialect(db));
    Arc::new(
        result
            .source_targets
            .into_iter()
            .map(|target| FileSourceTarget {
                original_source_load: target.original_project_source_load,
                raw_path: target.raw_path,
                is_literal: target.is_literal,
            })
            .collect(),
    )
}

/// Conditional script-directory source associations for display consumers.
/// An empty compatibility projection does not establish complete source coverage.
/// Dispatch bounds use the checked projection and retain unknown paths.
#[salsa::tracked(returns(clone))]
pub fn file_link_targets(db: &dyn TclDb, file: SourceFile) -> Arc<BTreeSet<String>> {
    checked_file_link_targets(db, file).unwrap_or_default()
}

#[salsa::tracked(returns(clone))]
fn checked_file_link_targets(db: &dyn TclDb, file: SourceFile) -> Option<Arc<BTreeSet<String>>> {
    conditional_source_links(file.path(db).as_deref(), &file_source_targets(db, file)).map(Arc::new)
}

pub(super) fn conditional_source_links(
    path: Option<&str>,
    targets: &[FileSourceTarget],
) -> Option<BTreeSet<String>> {
    targets
        .iter()
        .map(|target| {
            let load = target.original_source_load.as_ref()?;
            let target = tcl_lsp_core::source_graph::conditional_project_source_target(
                std::path::Path::new(path?),
                load,
            )?;
            Some(target.to_str()?.to_owned())
        })
        .collect()
}

/// Union-find root of `i`, with path halving.
fn component_root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The `source`-connected components of a project, each carrying the merged
/// declarations of its members — the per-project half of
/// [`file_dispatch_reach`].
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DispatchComponents {
    /// Component number of each project file, positionally aligned with
    /// [`Project::files`].
    component_of: Vec<usize>,
    /// Per component, every procedure its member files declare.  Shared by
    /// `Arc` with each member's [`file_dispatch_reach`], so a component is
    /// merged once and stored once however many files belong to it.
    pub(super) procs: Vec<Arc<BTreeSet<String>>>,
}

impl DispatchComponents {
    /// Conditional procedure names in this represented source component.
    /// Missing project membership supplies no bound or actual interpreter proof.
    #[must_use]
    pub fn reach(&self, index: usize) -> Option<&Arc<BTreeSet<String>>> {
        self.procs.get(*self.component_of.get(index)?)
    }
}

/// Decompose the declared project source associations once per revision.
/// Each genuine literal path uses the explicit script-directory policy, without
/// proving actual loads, shared interpreters or runtime working directories.
/// Dynamic/unowned paths and targets outside the represented project withdraw
/// the finite bound. Parameter evidence then retains only unknown callers.
/// Per-file checked associations and declarations retain signature early cutoff.
#[salsa::tracked(returns(clone))]
pub fn project_dispatch_components(
    db: &dyn TclDb,
    project: Project,
) -> Option<Arc<DispatchComponents>> {
    let rows = project
        .files(db)
        .iter()
        .map(|&file| {
            Some((
                file.path(db).clone(),
                checked_file_link_targets(db, file)?,
                file_decls(db, file),
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    merge_dispatch_components(rows)
}

pub(super) fn merge_dispatch_components(
    rows: impl IntoIterator<Item = (Option<String>, Arc<BTreeSet<String>>, Arc<FileDecls>)>,
) -> Option<Arc<DispatchComponents>> {
    let rows: Vec<_> = rows.into_iter().collect();
    // Index files by path so `source` targets can be matched to project files.
    let mut by_path: HashMap<&str, usize> = HashMap::new();
    for (i, (path, _, _)) in rows.iter().enumerate() {
        if let Some(p) = path.as_deref() {
            by_path.insert(p, i);
        }
    }
    // Union-find over the undirected `source` graph.
    let mut parent: Vec<usize> = (0..rows.len()).collect();
    for (i, (_, links, _)) in rows.iter().enumerate() {
        for target in links.iter() {
            let &j = by_path.get(target.as_str())?;
            let (a, b) = (
                component_root(&mut parent, i),
                component_root(&mut parent, j),
            );
            if a != b {
                parent[a] = b;
            }
        }
    }
    // Number the roots densely, then merge each component's declarations once.
    let mut number: HashMap<usize, usize> = HashMap::new();
    let mut component_of: Vec<usize> = Vec::with_capacity(rows.len());
    for i in 0..rows.len() {
        let root = component_root(&mut parent, i);
        let next = number.len();
        component_of.push(*number.entry(root).or_insert(next));
    }
    let mut merged: Vec<BTreeSet<String>> = vec![BTreeSet::new(); number.len()];
    for (i, (_, _, declarations)) in rows.iter().enumerate() {
        merged[component_of[i]].extend(declarations.procs.iter().cloned());
    }
    Some(Arc::new(DispatchComponents {
        component_of,
        procs: merged.into_iter().map(Arc::new).collect(),
    }))
}

/// Conditional project procedure names for an unenumerable source dispatch.
/// A missing association bound remains unavailable, including unrepresented loads.
///
/// An index into [`project_dispatch_components`], which does the work; see
/// there for what the component bound means and why it is not computed here.
/// Missing project membership or incomplete source paths leave the bound unavailable.
///
/// Kept as its own query so the result still early-cutoffs per file: a decl
/// change in one component recomputes the project decomposition, but every
/// file outside that component gets back an equal set and backdates, leaving
/// [`file_call_site_evidence`] untouched.
#[salsa::tracked(returns(clone))]
pub fn file_dispatch_reach(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> Option<Arc<BTreeSet<String>>> {
    let me = project
        .files(db)
        .iter()
        .position(|member| *member == file)?;
    let components = project_dispatch_components(db, project)?;
    components.reach(me).cloned()
}

/// Every call site **this** file contributes, resolved against the whole
/// project's procedure names.
///
/// The per-file half of the cross-file interprocedural seed:
/// `lib.tcl`'s own compilation unit can never see `main.tcl`'s `helper dev`,
/// so each file publishes its call sites here and
/// [`project_call_site_evidence`] merges them.
///
/// Depends on the file's text **and** on [`project_proc_names`] — the
/// signature firewall — so a body edit anywhere else in the project leaves
/// this query's inputs unchanged and it is not re-run.  Its own result
/// early-cutoffs too: an edit that does not move a call site's literal
/// arguments produces an equal `CallSiteEvidence` and backdates, so
/// [`project_call_site_evidence`] (and every compilation unit downstream of
/// it) is left alone.
#[salsa::tracked(returns(clone))]
pub fn file_call_site_evidence(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> Arc<CallSiteEvidence> {
    let known: HashSet<String> = project_proc_names(db, project).iter().cloned().collect();
    let Some(reach) = file_dispatch_reach(db, file, project) else {
        let mut unavailable = CallSiteEvidence::default();
        unavailable.record_unenumerable_caller(&known.iter().cloned().collect::<Vec<_>>());
        return Arc::new(unavailable);
    };
    let reach = reach.iter().cloned().collect::<Vec<_>>();
    let config = project_token_inputs::configuration_for_file(db, project, file);
    let own = config.and_then(|config| file_original_procedure_facts(db, file, config));
    let scanned = match config {
        Some(config) => {
            if let Ok(input) = walked_document_input(db, file, config) {
                let declared = declared_command_surface(
                    db,
                    file,
                    input.borrowed_context_registry().commands(),
                );
                let external: Vec<_> = project_original_procedure_facts(db, project)
                    .iter()
                    .filter(|(provider, _)| *provider != file)
                    .filter_map(|(_, facts)| {
                        tcl_compiler::unit_scope::ExternalSourceProcedureFacts::capture(
                            facts.input(),
                            facts.image(),
                            facts.declarations(),
                        )
                    })
                    .collect();
                tcl_compiler::unit_scope::scan_source_call_sites_with_original_source(
                    file.text(db),
                    Some(&declared),
                    &known,
                    &reach,
                    &input,
                    None,
                    &external,
                )
            } else {
                let mut unavailable = CallSiteEvidence::default();
                unavailable.record_unenumerable_caller(&known.iter().cloned().collect::<Vec<_>>());
                unavailable
            }
        }
        None => tcl_compiler::unit_scope::scan_source_call_sites(
            file.text(db),
            db.registry(file.dialect(db)),
            Some(&declared_command_surface(
                db,
                file,
                db.registry(file.dialect(db)),
            )),
            tcl_lsp_core::profile_for_dialect(file.dialect(db)),
            &known,
            &reach,
        ),
    };
    let sites: Vec<_> = own
        .iter()
        .flat_map(|facts| facts.declarations())
        .map(|declaration| declaration.declaration_site().clone())
        .collect();
    Arc::new(scanned.without_original_declarations(&sites))
}

/// The project-wide merge of every file's [`file_call_site_evidence`].
///
/// One table serves every file: a unit's own call sites are already in it, and
/// merging is monotone, so handing a file the whole project's view is exactly
/// what it would have collected had the project been one source text.
#[salsa::tracked(returns(clone))]
pub fn project_call_site_evidence(db: &dyn TclDb, project: Project) -> Arc<CallSiteEvidence> {
    let mut merged = CallSiteEvidence::default();
    for &file in project.files(db) {
        merged.merge_from(&file_call_site_evidence(db, file, project));
    }
    Arc::new(merged)
}

/// The slice of [`project_call_site_evidence`] that concerns the procedures
/// `file` itself declares — what a host sets on
/// [`SourceFile::external_call_sites`].
///
/// Narrowing to the file's own declarations is what keeps invalidation
/// precise: editing a call site in `main.tcl` changes only the evidence of the
/// file that *defines* the callee, so only that file's compilation unit is
/// rebuilt.
#[salsa::tracked(returns(clone))]
pub fn file_external_call_sites(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> Arc<CallSiteEvidence> {
    let all = project_call_site_evidence(db, project);
    let own = project_token_inputs::configuration_for_file(db, project, file)
        .and_then(|config| file_original_procedure_facts(db, file, config));
    let sites: Vec<_> = own
        .iter()
        .flat_map(|facts| facts.declarations())
        .map(|declaration| declaration.declaration_site().clone())
        .collect();
    Arc::new(all.slice_for_original_declarations(&sites))
}
