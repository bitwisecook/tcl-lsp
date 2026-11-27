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

//! The workspace refinement of W120: package availability and `source`-graph inheritance.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lsp_core::package_resolver::PackageResolver;
use tcl_lsp_core::workspace_index as core_workspace_index;
use tokio::sync::RwLock;
use tower_lsp_server::ls_types::Uri;

use super::{OriginalSourceInheritance, SourceInheritance};
use crate::configuration::FolderConfig;
use crate::uri_norm::DocumentPath as _;
#[cfg(test)]
use crate::vfs;
use crate::workspace::{entry_point_uri, resolve_source_edge, uri_under_folder};
use crate::{Backend, provider_source};

impl Backend {
    /// The project entry points and folder root that govern the W120
    /// inheritance for `uri`: the longest matching workspace folder's
    /// `.tcl-lsp.ini [project] entryPoints` (empty ⇒ automatic `source`-graph
    /// mode) and that folder's filesystem root (to resolve relative entry
    /// paths).
    pub(super) async fn w120_inheritance_config(
        &self,
        uri: &Uri,
    ) -> (Vec<String>, Option<PathBuf>) {
        let configs = self.folder_configs.lock().await;
        let mut best: Option<&(Uri, FolderConfig)> = None;
        for entry in configs.iter() {
            if uri_under_folder(uri.as_str(), entry.0.as_str())
                && best.is_none_or(|b| entry.0.as_str().len() > b.0.as_str().len())
            {
                best = Some(entry);
            }
        }
        match best {
            Some((folder, fc)) => (
                fc.entry_points.clone().unwrap_or_default(),
                folder.document_path().map(std::borrow::Cow::into_owned),
            ),
            None => (Vec::new(), None),
        }
    }

    /// Everything the workspace `source` graph contributes to `uri` — both
    /// directions plus the abstention flag — resolved live against the current
    /// index. The pull path's counterpart of the push path's inline call, so
    /// the two surfaces answer identically.
    pub(in super::super) async fn source_inheritance_for(
        &self,
        uri: &Uri,
        analysis: &AnalysisResult,
    ) -> SourceInheritance {
        let (entry_points, folder_root) = self.w120_inheritance_config(uri).await;
        let index = self.workspace_index.read().await;
        compute_source_inheritance(
            &index,
            uri,
            analysis,
            &entry_points,
            folder_root.as_deref(),
            self.default_package_prefer().await,
        )
    }
}

fn analysis_tcl_version(analysis: &AnalysisResult) -> Option<tcl_dialect::TclVersion> {
    analysis
        .resolved_profile()
        .and_then(|profile| tcl_native_model::InvocationDialect::of_profile(profile).tcl_version)
}

fn w120_source_requirements(
    analysis: &AnalysisResult,
    prefer: tcl_lsp_core::package_resolver::PackagePrefer,
) -> Vec<tcl_lsp_core::package_resolver::PackageRequirementAdvice> {
    use tcl_lsp_core::package_resolver::PackageRequirementAdvice as Requirement;
    analysis
        .package_requires
        .iter()
        .map(|required| {
            Requirement::from_source_requirement(
                required,
                tcl_lsp_core::package_resolver::package_prefer_at(
                    analysis,
                    required.range.start(),
                    prefer,
                ),
            )
        })
        .collect::<Vec<_>>()
}

fn w120_metadata_roots(
    available: &[String],
) -> Vec<tcl_lsp_core::package_resolver::PackageRequirementAdvice> {
    available
        .iter()
        .map(|name| {
            tcl_lsp_core::package_resolver::PackageRequirementAdvice::authored_metadata(
                name,
                tcl_lsp_core::package_resolver::PackagePrefer::Stable,
            )
        })
        .collect::<Vec<_>>()
}

/// Workspace-level refinement of the analyser's single-file W120
/// (missing-`package require`) diagnostics, using the scanned package database.
///
/// The analyser only knows the packages required/provided *in the document*;
/// here the workspace + `TCLLIBPATH` `pkgIndex.tcl` files additionally say
/// conditional dependency advice from each registered implementation source.
/// Each provider keeps its own configured input; this scan proves no loader ran.
///
/// A W120 for a package is suppressed by a named requirement for that
/// provider in the dependency closure. Unknown requirements only limit the
/// closure through that dependency.
#[cfg(test)]
pub(in super::super) fn refine_w120_diagnostics(
    diags: Vec<tcl_compiler::analyser::Diagnostic>,
    package_requires: &[String],
    resolver: &PackageResolver,
    store: &dyn vfs::SourceStore,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    let availability = W120Availability::resolve(package_requires, resolver, store);
    diags
        .into_iter()
        .filter(|d| !availability.suppresses(d))
        .collect()
}

/// What one set of `package require`s makes available, as the W120 check reads
/// it in a reusable form.
///
/// Split out because the position-gated path evaluates a *different*
/// availability set per diagnostic offset and must be able to memoise the
/// expensive part (the transitive scan, which reads package implementation
/// files) independently of which diagnostic it is judging.
pub(in super::super) struct W120Availability {
    pub(in super::super) original_available:
        std::collections::HashSet<tcl_registry::native_package::NativePackageNameKey>,
    /// The conditional transitive closure of package dependency source advice.
    available: std::collections::HashSet<String>,
}

impl W120Availability {
    #[cfg(test)]
    pub(in super::super) fn resolve(
        package_requires: &[String],
        resolver: &PackageResolver,
        store: &dyn vfs::SourceStore,
    ) -> Self {
        Self {
            original_available: std::collections::HashSet::new(),
            available: resolver
                .transitive_available_packages(package_requires, &|p| store.read_to_string(p).ok())
                .into_iter()
                .collect(),
        }
    }

    pub(in super::super) fn resolve_original(
        roots: &[tcl_lsp_core::package_resolver::PackageRequirementAdvice],
        target: Option<tcl_dialect::TclVersion>,
        resolver: &PackageResolver,
        sources: &provider_source::ProviderSourceAccess<'_>,
    ) -> Self {
        use tcl_lsp_core::package_resolver::{
            PackageRequirementAdvice as Requirement, PackageRequirementAdviceKey as Key,
        };
        let mut result = Self {
            available: std::collections::HashSet::new(),
            original_available: std::collections::HashSet::new(),
        };
        for root in roots {
            if matches!(root.key(), Key::Unknown) {
                // There is no package identity to use for source advice.
                continue;
            }
            let version = root
                .key()
                .original()
                .and_then(|key| match key.policy().recipe() {
                    tcl_syntax::naming::NativeNameProtocol::C(version) => Some(version),
                    tcl_syntax::naming::NativeNameProtocol::Jim084 => None,
                })
                .or(target);
            let available = resolver.transitive_requirement_advice_with_context(
                std::slice::from_ref(root),
                version,
                &|path, parent| {
                    let Ok(inventory) = sources.inventory(path, version, resolver) else {
                        return Vec::new();
                    };
                    let analysis = &inventory.analysis;
                    analysis
                        .package_requires
                        .iter()
                        .map(|required| {
                            Requirement::from_source_requirement(
                                required,
                                tcl_lsp_core::package_resolver::package_prefer_at(
                                    analysis,
                                    required.range.start(),
                                    parent.prefer(),
                                ),
                            )
                        })
                        .collect()
                },
            );
            for dependency in available {
                match dependency.key() {
                    Key::Original(key) => {
                        result.original_available.insert(key.clone());
                    }
                    Key::AuthoredMetadata(name) => {
                        result.available.insert(name.clone());
                    }
                    Key::Unknown => {}
                }
            }
        }
        result
    }

    /// Whether this availability set removes `d` — always `false` for anything
    /// that is not a W120.
    pub(in super::super) fn suppresses(&self, d: &tcl_compiler::analyser::Diagnostic) -> bool {
        use tcl_registry::native_package::matches_package_descriptor;

        if d.code != DiagCode::W120 {
            return false;
        }
        let Some(package) = w120_required_package(d) else {
            return false;
        };
        self.original_available.contains(package)
            || std::str::from_utf8(package.bytes()).is_ok_and(|descriptor| {
                self.available
                    .iter()
                    .any(|available| matches_package_descriptor(available.as_bytes(), descriptor))
                    || self.original_available.iter().any(|available| {
                        available.policy() == package.policy()
                            && matches_package_descriptor(available.bytes(), descriptor)
                    })
            })
    }
}

impl OriginalSourceInheritance {
    pub(in super::super) fn available_at(
        &self,
        offset: u32,
        body: Option<tcl_lexer::Span>,
    ) -> Vec<tcl_lsp_core::package_resolver::PackageRequirementAdvice> {
        let mut keys = self.ambient.clone();
        for placed in &self.placed {
            if tcl_compiler::analyser::indirection::in_effect_within(placed.at, offset, body)
                && !keys.contains(&placed.name)
            {
                keys.push(placed.name.clone());
            }
        }
        keys
    }
}

impl SourceInheritance {
    /// Whether this contributes nothing — the common single-file case, where
    /// the caller can skip the package resolver entirely.
    pub(in super::super) fn is_empty(&self) -> bool {
        self.ambient.is_empty() && self.placed.is_empty() && !self.unresolvable_source
    }

    /// The package names available **at** `off` in this document: the ambient
    /// set plus every placed require whose `source` statement has already run
    /// by then.
    ///
    /// Order-gated with
    /// [`in_effect_within`](tcl_compiler::analyser::indirection::in_effect_within),
    /// so a load-level `source` counts for a call written inside any proc body
    /// (the whole file loads before any body runs) but *not* for a call written
    /// above it at load level — which is what C Tcl 9.0.4 does: `package
    /// present` before the `source` statement raises, after it succeeds.
    pub(in super::super) fn available_at(
        &self,
        off: u32,
        enclosing_body: Option<tcl_lexer::Span>,
    ) -> Vec<String> {
        use tcl_compiler::analyser::indirection::in_effect_within;
        let mut out = self.ambient.clone();
        out.extend(
            self.placed
                .iter()
                .filter(|p| in_effect_within(p.at, off, enclosing_body))
                .map(|p| p.name.clone()),
        );
        out.sort();
        out.dedup();
        out
    }
}

/// Apply the workspace W120 refinement to `analyser_diags`: resolve the
/// document's `package require`s through the shared package database and drop
/// any W120 whose flagged package is transitively available. Shared by the push
/// path (`refine_and_lift_diagnostics`) and the pull path
/// (`Backend::full_diagnostics_for`) so both stay behaviour-identical.
///
/// Three sources of availability feed it, and they are *not* interchangeable:
///
/// * the document's own `package require`s;
/// * [`SourceInheritance::ambient`] — the down direction;
/// * [`SourceInheritance::placed`] — the up direction, evaluated **per
///   diagnostic** at that diagnostic's own offset, because a `source` only
///   makes its packages present from its own statement onward.
///
/// An [`unresolvable_source`](SourceInheritance::unresolvable_source) short-
/// circuits all of it and drops every W120: with a file of unknown content in
/// the run, "this package is missing" is not a claim this server can make.
///
/// The common case (no W120, or nothing to inherit and no `package require`)
/// skips the resolver lock entirely; otherwise the (bounded) transitive scan
/// reads only the required packages' implementation files, memoised per
/// distinct availability set so a document with many W120s at the same
/// position pays for one scan.
pub(in super::super) async fn refine_workspace_w120(
    analyser_diags: Vec<tcl_compiler::analyser::Diagnostic>,
    analysis: &AnalysisResult,
    inheritance: &SourceInheritance,
    package_resolver: &Arc<RwLock<PackageResolver>>,
    sources: &provider_source::ProviderSourceAccess<'_>,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    if !analyser_diags.iter().any(|d| d.code == DiagCode::W120) {
        return analyser_diags;
    }
    // A `source` we cannot follow makes every W120 unprovable — abstain.
    if inheritance.unresolvable_source {
        return analyser_diags
            .into_iter()
            .filter(|d| d.code != DiagCode::W120)
            .collect();
    }
    if let Some(inherited) = &inheritance.original {
        let own = w120_source_requirements(analysis, inherited.prefer);
        let target = analysis_tcl_version(analysis);
        let resolver = package_resolver.read().await;
        let mut scans = HashMap::new();
        let mut result = Vec::new();
        for diagnostic in analyser_diags {
            if diagnostic.code != DiagCode::W120 {
                result.push(diagnostic);
                continue;
            }
            let mut roots = own.clone();
            for root in inherited.available_at(
                diagnostic.span.start(),
                analysis.innermost_definition_body_span(diagnostic.span.start()),
            ) {
                if !roots.contains(&root) {
                    roots.push(root);
                }
            }
            let available = scans.entry(roots.clone()).or_insert_with(|| {
                W120Availability::resolve_original(&roots, target, &resolver, sources)
            });
            if !available.suppresses(&diagnostic) {
                result.push(diagnostic);
            }
        }
        return result;
    }
    // Logical package-name advice requires the source-owned requirement and
    // preference transport above. Report strings do not recreate that owner.
    if analysis.allows_lexical_declaration_advice() {
        return analyser_diags;
    }
    let own: Vec<String> = analysis
        .package_requires
        .iter()
        .map(|pr| pr.name.clone())
        .collect();
    if own.is_empty() && inheritance.is_empty() {
        return analyser_diags;
    }
    let resolver = package_resolver.read().await;
    // Nothing placed ⇒ no position to gate on ⇒ the plain whole-set pass.
    // This is the shape of every document that does not `source` anything,
    // i.e. almost all of them.
    if inheritance.placed.is_empty() {
        let mut available = own;
        available.extend(inheritance.ambient.iter().cloned());
        let roots = w120_metadata_roots(&available);
        let availability = W120Availability::resolve_original(
            &roots,
            analysis_tcl_version(analysis),
            &resolver,
            sources,
        );
        return analyser_diags
            .into_iter()
            .filter(|diagnostic| !availability.suppresses(diagnostic))
            .collect();
    }
    // One transitive scan per distinct availability set, keyed by its sorted
    // contents — so the overwhelmingly common "every W120 sees the same
    // packages" shape pays for exactly one.
    let mut scans: HashMap<Vec<String>, W120Availability> = HashMap::new();
    let mut out: Vec<tcl_compiler::analyser::Diagnostic> = Vec::new();
    for diag in analyser_diags {
        if diag.code != DiagCode::W120 {
            out.push(diag);
            continue;
        }
        let off = diag.span.start();
        let body = analysis.innermost_definition_body_span(off);
        let mut available = own.clone();
        available.extend(inheritance.available_at(off, body));
        available.sort();
        available.dedup();
        let scan = scans.entry(available.clone()).or_insert_with(|| {
            let roots = w120_metadata_roots(&available);
            W120Availability::resolve_original(
                &roots,
                analysis_tcl_version(analysis),
                &resolver,
                sources,
            )
        });
        if !scan.suppresses(&diag) {
            out.push(diag);
        }
    }
    out
}

/// The extra `package require` names available to `uri` for the W120
/// refinement: from the project's configured entry points when set (which
/// disables auto-detection), else from the workspace `source` graph — every
/// file that transitively `source`s `uri` shares its requires.  Operates on an
/// already-locked index so both the push and pull paths can call it.
pub(in super::super) fn compute_inherited_requires(
    index: &core_workspace_index::WorkspaceIndex,
    uri: &Uri,
    entry_points: &[String],
    folder_root: Option<&Path>,
) -> Vec<String> {
    if entry_points.is_empty() {
        return index.source_ancestor_package_requires_from_resolved(uri.as_str());
    }
    // Explicit entry points: the union of their requires, project-wide.
    let mut out: Vec<String> = Vec::new();
    for ep in entry_points {
        if let Some(entry_uri) = entry_point_uri(ep, folder_root) {
            out.extend(index.package_requires_for(&entry_uri));
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The workspace `source` edges, resolved with this server's own resolver.
///
/// **One** construction of the edge list, shared by everything that walks the
/// graph, so the *same* `source` statements are followed everywhere.  In
/// particular it uses [`resolve_source_edge`] — the resolver with the
/// statically-foldable computed-path tier (`[file join [file dirname
/// [info script]] x.tcl]`, chained through the parent's single-assignment
/// constants) — not the literals-only [`resolve_source_uri`]: the common
/// computed idiom would otherwise be invisible, and two resolvers behind two
/// callers is how the two views come to diverge.
pub(super) fn workspace_source_edges(
    index: &core_workspace_index::WorkspaceIndex,
) -> Vec<tcl_lsp_core::source_graph::RunEdge> {
    index.resolved_source_edges()
}

fn indexed_package_requirement_advice(
    index: &core_workspace_index::WorkspaceIndex,
    default_prefer: tcl_lsp_core::package_resolver::PackagePrefer,
) -> HashMap<String, Vec<tcl_lsp_core::package_resolver::PackageRequirementAdvice>> {
    let mut advice: HashMap<String, Vec<tcl_lsp_core::package_resolver::PackageRequirementAdvice>> =
        HashMap::new();
    for required in index.package_requires() {
        let default = if index.source_ancestor_prefers_latest_from_resolved(&required.uri) {
            tcl_lsp_core::package_resolver::PackagePrefer::Latest
        } else {
            default_prefer
        };
        advice
            .entry(required.uri.clone())
            .or_default()
            .push(required.original_advice(default));
    }
    advice
}

/// Everything the `source` graph says about `uri` — both directions plus the
/// abstention flag.  Operates on an already-locked
/// index so the push and pull paths share it.
///
/// The **up** direction and the abstention flag are computed from this
/// document's own recorded `source` statements:
///
/// * a statement whose target [`resolve_source_edge`] can pin to a document the
///   index holds contributes that document's (transitive) `package require`s,
///   placed at the statement's offset;
/// * a statement whose target cannot be pinned — an unfoldable dynamic path, a
///   file outside the workspace, or one that does not exist — sets
///   [`SourceInheritance::unresolvable_source`], and the caller abstains.
///
/// A file sourced under a *conditional* (`if {0} { source x }`) or inside a
/// proc body is followed like any other. That is deliberate and it is the
/// direction to be wrong in: the graph has no conditional-execution analysis,
/// and treating "might not run" as "definitely did not run" is what produces
/// the false `package require Tk` this fixes. The cost is a false negative —
/// a genuinely missing require going unreported in a file that conditionally
/// sources a provider — which is the cheaper error on a real project.
pub(in super::super) fn compute_source_inheritance(
    index: &core_workspace_index::WorkspaceIndex,
    uri: &Uri,
    analysis: &AnalysisResult,
    entry_points: &[String],
    folder_root: Option<&Path>,
    default_prefer: tcl_lsp_core::package_resolver::PackagePrefer,
) -> SourceInheritance {
    let ambient = compute_inherited_requires(index, uri, entry_points, folder_root);
    let prefer = if index.source_ancestor_prefers_latest_from_resolved(uri.as_str()) {
        tcl_lsp_core::package_resolver::PackagePrefer::Latest
    } else {
        default_prefer
    };
    let advice = indexed_package_requirement_advice(index, default_prefer);
    let original_ambient =
        original_ambient_requirements(index, uri, entry_points, folder_root, &advice);
    // This document's *own* `source` statements come from the analysis in
    // hand, not from the index: the index is refreshed at the end of a
    // publish, so on a document's first (or freshly-edited) run its own
    // statements are not in there yet — reading them from the index made the
    // whole up direction silently inert exactly when it was needed. The index
    // is still the authority for every *other* document's edges and requires.
    let uri_str = uri.as_str();
    if analysis.source_targets.is_empty() {
        return SourceInheritance {
            original: Some(OriginalSourceInheritance {
                prefer,
                ambient: original_ambient,
                placed: Vec::new(),
            }),
            ambient,
            placed: Vec::new(),
            unresolvable_source: false,
        };
    }
    let mut unresolvable_source = false;
    let mut own_edges: Vec<tcl_lsp_core::source_graph::RunEdge> = Vec::new();
    for src in &analysis.source_targets {
        if let Some(original) = &src.original_interpreter_source_load
            && !original.matches_analysis(analysis)
        {
            unresolvable_source = true;
            continue;
        }
        match resolve_source_edge(
            uri_str,
            &src.raw_path,
            tcl_compiler::auto_path_eval::capture_source_target_path_expression(analysis, src)
                .as_ref(),
            src.range.start(),
            &analysis.path_constant_assignments,
            &index.imported_path_constants_for(uri_str),
        ) {
            Some(child) if index.contains_document(&child) => {
                own_edges.push(tcl_lsp_core::source_graph::RunEdge {
                    parent: uri_str.to_owned(),
                    child,
                    at: src.range.start(),
                    enclosing_body: analysis.innermost_definition_body_span(src.range.start()),
                    kind: tcl_lsp_core::source_graph::RunEdgeKind::Source,
                });
            }
            // Resolvable but not indexed (outside the workspace, or absent), or
            // not resolvable at all (a path no static fold can prove).
            _ => unresolvable_source = true,
        }
    }
    // Deeper hops come from the index, which does hold the *other* documents'
    // statements; this document's own index edges are dropped so the fresher
    // `own_edges` are the only ones speaking for it.
    let mut edges: Vec<tcl_lsp_core::source_graph::RunEdge> = workspace_source_edges(index)
        .into_iter()
        .filter(|e| e.parent != uri_str)
        .collect();
    edges.extend(own_edges);
    let mut requires: HashMap<String, Vec<String>> = HashMap::new();
    for pr in index.package_requires() {
        requires
            .entry(pr.uri.clone())
            .or_default()
            .push(pr.name.clone());
    }
    SourceInheritance {
        original: Some(OriginalSourceInheritance {
            prefer,
            ambient: original_ambient,
            placed: tcl_lsp_core::source_graph::descendant_requirements(uri_str, &edges, &advice),
        }),
        ambient,
        placed: tcl_lsp_core::source_graph::descendant_requires(uri_str, &edges, &requires),
        unresolvable_source,
    }
}

fn original_ambient_requirements(
    index: &core_workspace_index::WorkspaceIndex,
    uri: &Uri,
    entry_points: &[String],
    folder_root: Option<&Path>,
    advice: &HashMap<String, Vec<tcl_lsp_core::package_resolver::PackageRequirementAdvice>>,
) -> Vec<tcl_lsp_core::package_resolver::PackageRequirementAdvice> {
    if entry_points.is_empty() {
        let edges = workspace_source_edges(index)
            .into_iter()
            .map(|edge| (edge.parent, edge.child))
            .collect::<Vec<_>>();
        tcl_lsp_core::source_graph::ancestor_requirements(uri.as_str(), &edges, advice)
    } else {
        let mut roots = Vec::new();
        for entry in entry_points {
            if let Some(uri) = entry_point_uri(entry, folder_root)
                && let Some(keys) = advice.get(&uri)
            {
                for key in keys {
                    if !roots.contains(key) {
                        roots.push(key.clone());
                    }
                }
            }
        }
        roots
    }
}

/// Exact package key retained independently of diagnostic wording and fixes.
fn w120_required_package(
    diagnostic: &tcl_compiler::analyser::Diagnostic,
) -> Option<&tcl_registry::native_package::NativePackageNameKey> {
    diagnostic.required_package_key()
}
