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

//! Resolving the symbol at a cursor against the workspace index.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::{hover as core_hover, workspace_index as core_workspace_index};
use tower_lsp_server::ls_types::{Position, Uri};

use crate::diagnostics::{settle_call_against_workspace, workspace_reporting_context};
use crate::documents::{DocumentSourceResult, DocumentSourceUnavailable};
use crate::protocol::line_col_to_byte_offset;

use crate::Backend;

/// Readonly source relationships cannot be used by edit/binding consumers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WorkspaceSymbolPurpose {
    ReadonlySourceAdvice,
    Binding,
}

/// A conditional empty report is terminal, never an autoload absence proof.
pub(super) enum WorkspaceSymbolSelection {
    Miss,
    Candidates(Vec<String>),
    ConditionalCandidates(Vec<String>),
    ConditionalEmpty,
}

impl WorkspaceSymbolSelection {
    pub(in super::super) fn report(self) -> Vec<String> {
        match self {
            Self::Candidates(names) | Self::ConditionalCandidates(names) => names,
            Self::Miss | Self::ConditionalEmpty => Vec::new(),
        }
    }
}

impl Backend {
    /// Resolve proc/class source report candidates at `pos` (a bare command
    /// word, not a `$var`). Readonly navigation may retain conditional namespace
    /// relationships; those report names are not runtime identities. The
    /// explicit Binding path keeps the independent existing edit/binding fold.
    ///
    /// Two cursor shapes resolve, both namespace-exact:
    ///
    /// 1. On a proc / class **declaration name** in this document — the symbol
    ///    whose name span covers the cursor.  A declaration in a document
    ///    sourced under several namespaces is one physical token with one
    ///    runtime identity **per source-site view**, so
    ///    every seed-mapped identity is returned, never an arbitrary first.
    /// 2. On a **command-head call** — the invocation's ordered resolution
    ///    candidates (caller namespace, each `namespace path` entry, then
    ///    global) walked in Tcl priority order; the first defined in the current
    ///    document or anywhere in the workspace is the call's target.  This is
    ///    the workspace-scoped resolution oracle, rather than a namespace-blind
    ///    `name == word` scan and an arbitrary same-simple-name sibling pick.
    ///
    /// Anything else (a bareword argument, whitespace, a `$var`) resolves to
    /// nothing (an empty set), so no coincidental word links to a sibling
    /// symbol.
    /// The workspace symbols a release-profile cursor names, for listing
    /// references across documents: the qualified name of the procedure or
    /// class declared at the cursor, or the declarations the call at the
    /// cursor settles to in the workspace index. Binding consumers get nothing.
    async fn release_workspace_symbols(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        purpose: WorkspaceSymbolPurpose,
    ) -> WorkspaceSymbolSelection {
        let current = analysis.body_lexer_config.is_some_and(|config| {
            analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        });
        let Some(offset) = line_col_to_byte_offset(source, pos.line, pos.character)
            .and_then(|offset| u32::try_from(offset).ok())
            .filter(|_| current && purpose == WorkspaceSymbolPurpose::ReadonlySourceAdvice)
        else {
            return WorkspaceSymbolSelection::Miss;
        };
        let covers = |span: tcl_lexer::Span| span.start() <= offset && offset < span.end();
        // A document sourced inside a namespace is indexed under the
        // identities its source views give it (`::x::helper` for a `proc
        // helper` sourced in `namespace eval ::x`), not under the standalone
        // name its own analysis states. The declaration at the cursor is the
        // index's procedure whose name token is the one under the cursor, so
        // the index names it — every runtime identity of a multi-seeded one.
        let indexed_procedures: Vec<String> = {
            let index = self.workspace_index.read().await;
            let mut found: Vec<String> = index
                .live_procs()
                .filter(|proc| proc.uri == uri.as_str() && covers(proc.name_span))
                .map(|proc| proc.qualified_name.clone())
                .collect();
            found.sort();
            found.dedup();
            found
        };
        let has_indexed_procedures = !indexed_procedures.is_empty();
        let mut names = analysis
            .original_procedure_declarations()
            .filter(|record| covers(record.name_input().span()))
            .map(|record| record.metadata().qualified_name.clone())
            .filter(|_| !has_indexed_procedures)
            .chain(indexed_procedures)
            .chain(
                analysis
                    .original_class_declarations()
                    .filter(|record| covers(record.name_input().span()))
                    .map(|record| record.metadata().qualified_name.clone()),
            )
            .collect::<Vec<_>>();
        if names.is_empty()
            && let Some(call) = analysis
                .command_invocations
                .iter()
                .find(|invocation| covers(invocation.range))
        {
            names = self
                .workspace_index
                .read()
                .await
                .linked_declaration_names_of_call(uri.as_str(), call.range);
        }
        if names.is_empty() {
            WorkspaceSymbolSelection::Miss
        } else {
            WorkspaceSymbolSelection::Candidates(names)
        }
    }

    async fn workspace_symbols_indexed(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        purpose: WorkspaceSymbolPurpose,
    ) -> DocumentSourceResult<WorkspaceSymbolSelection> {
        if !analysis.allows_lexical_declaration_advice() {
            return Ok(self
                .release_workspace_symbols(uri, source, analysis, pos, purpose)
                .await);
        }
        self.resolve_workspace_symbols_indexed_for_purpose(uri, source, analysis, pos, purpose)
            .await
    }

    async fn resolve_workspace_symbols_indexed_for_purpose(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        purpose: WorkspaceSymbolPurpose,
    ) -> DocumentSourceResult<WorkspaceSymbolSelection> {
        if Self::workspace_symbol_source_is_unavailable(source, analysis) {
            return Ok(WorkspaceSymbolSelection::Miss);
        }
        if core_hover::find_var_at_position(source, pos.line, pos.character).is_some() {
            return Ok(WorkspaceSymbolSelection::Miss);
        }
        let Some(offset) = line_col_to_byte_offset(source, pos.line, pos.character) else {
            return Ok(WorkspaceSymbolSelection::Miss);
        };
        let covers =
            |sp: tcl_lexer::Span| (sp.start() as usize) <= offset && offset < (sp.end() as usize);
        let offset_u32 = u32::try_from(offset).unwrap_or(u32::MAX);

        if let Some(names) = self
            .workspace_index
            .read()
            .await
            .logical_declaration_source_names_at(uri.as_str(), source, analysis, offset_u32)
        {
            return Ok(
                if purpose == WorkspaceSymbolPurpose::Binding && names.is_empty() {
                    WorkspaceSymbolSelection::Miss
                } else {
                    WorkspaceSymbolSelection::Candidates(names)
                },
            );
        }

        let Some(inv) = analysis
            .command_invocations
            .iter()
            .find(|i| covers(i.range))
        else {
            return Ok(WorkspaceSymbolSelection::Miss);
        };
        {
            let index = self.workspace_index.read().await;
            // Pack-aware: a call site settled against the workspace index must
            // see the same command universe hover and diagnostics do.
            let registry = self.registry_for_dialect(&analysis.dialect).await;
            if workspace_reporting_context(analysis, &registry).is_none() {
                return Ok(WorkspaceSymbolSelection::Miss);
            }
            let exports = index.export_snapshot();
            if let Some(cand) = settle_call_against_workspace(
                &index,
                analysis,
                &registry,
                uri.as_str(),
                inv,
                exports.as_ref(),
                offset_u32,
            ) {
                // A candidate that resolves through an `interp alias` /
                // `rename` / `namespace import` names the linked command
                // locally; follow the link so the cursor resolves to the
                // command it ultimately runs, gathering its references with
                // that command's.
                return Ok(WorkspaceSymbolSelection::Candidates(vec![
                    index.resolve_command_target(cand),
                ]));
            }
            // The exact-candidate loop above only ever matches a *real*
            // command (or an exact `namespace import`'s `WorkspaceCommandLink`)
            // — a wildcard `namespace import NS::*` introduces no fixed
            // linked name, so it never appears there. Try it once more,
            // separately: does an in-scope glob import (`namespace import`
            // binds to the namespace, not the file it was written in, so
            // this is not restricted to imports recorded in this document)
            // whose source namespace actually exported the name cover this
            // call? This is the same gate
            // `definition::resolve_called_proc` / `resolve_class_target_at`
            // apply in-document, extended to a source namespace defined in
            // another file.
            if purpose == WorkspaceSymbolPurpose::ReadonlySourceAdvice {
                let advice = index
                    .conditional_namespace_import_advice(
                        &inv.name,
                        &inv.resolution_candidates,
                        core_workspace_index::CallSite {
                            uri: uri.as_str(),
                            at: inv.range.start(),
                            enclosing_body: analysis
                                .innermost_definition_body_span(inv.range.start()),
                        },
                    )
                    .ok_or_else(|| DocumentSourceUnavailable::ConditionalAdvice {
                        uri: uri.clone(),
                    })?;
                drop(index);
                return self
                    .finish_conditional_namespace_advice(&advice)
                    .await?
                    .map(|names| {
                        if names.is_empty() {
                            WorkspaceSymbolSelection::ConditionalEmpty
                        } else {
                            WorkspaceSymbolSelection::ConditionalCandidates(names)
                        }
                    })
                    .ok_or_else(|| DocumentSourceUnavailable::ConditionalAdvice {
                        uri: uri.clone(),
                    });
            }
            if let Some(target) = index.resolve_wildcard_import_for_binding(
                &inv.name,
                &inv.resolution_candidates,
                core_workspace_index::CallSite {
                    uri: uri.as_str(),
                    at: inv.range.start(),
                    enclosing_body: analysis.innermost_definition_body_span(inv.range.start()),
                },
            ) {
                return Ok(WorkspaceSymbolSelection::Candidates(vec![target]));
            }
        }
        Ok(WorkspaceSymbolSelection::Miss)
    }

    fn workspace_symbol_source_is_unavailable(source: &str, analysis: &AnalysisResult) -> bool {
        !analysis.allows_lexical_declaration_advice()
            || analysis.body_lexer_config.is_none_or(|config| {
                !analysis.matches_original_source_image(
                    &tcl_lexer::SourceImage::document(source),
                    config,
                )
            })
    }

    /// Resolve against the current index, then populate the autoload/package
    /// tier on a genuine miss. This function deliberately takes no source-site
    /// transaction guard: [`Self::ensure_source_queried_library_indexed`] may wait for
    /// `workspace_scan_gate`, and a workspace scan takes `rehoming_gate` while
    /// merging its results. Nesting those gates in the opposite order would
    /// deadlock a navigation request against an in-flight scan.
    #[cfg(test)]
    pub(in super::super) async fn resolve_workspace_symbols(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<Vec<String>> {
        self.resolve_workspace_symbols_for_purpose(
            uri,
            source,
            analysis,
            pos,
            WorkspaceSymbolPurpose::ReadonlySourceAdvice,
        )
        .await
    }

    pub(in super::super) async fn resolve_workspace_symbols_for_binding(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<Vec<String>> {
        self.resolve_workspace_symbols_for_purpose(
            uri,
            source,
            analysis,
            pos,
            WorkspaceSymbolPurpose::Binding,
        )
        .await
    }

    async fn resolve_workspace_symbols_for_purpose(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        purpose: WorkspaceSymbolPurpose,
    ) -> DocumentSourceResult<Vec<String>> {
        self.resolve_workspace_symbol_selection_for_purpose(uri, source, analysis, pos, purpose)
            .await
            .map(WorkspaceSymbolSelection::report)
    }

    pub(super) async fn resolve_workspace_symbol_selection_for_purpose(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        purpose: WorkspaceSymbolPurpose,
    ) -> DocumentSourceResult<WorkspaceSymbolSelection> {
        if !analysis.allows_lexical_declaration_advice() {
            return self
                .workspace_symbols_indexed(uri, source, analysis, pos, purpose)
                .await;
        }
        let indexed = self
            .workspace_symbols_indexed(uri, source, analysis, pos, purpose)
            .await?;
        match indexed {
            WorkspaceSymbolSelection::Candidates(_)
            | WorkspaceSymbolSelection::ConditionalCandidates(_)
            | WorkspaceSymbolSelection::ConditionalEmpty => return Ok(indexed),
            WorkspaceSymbolSelection::Miss => {}
        }

        // Autoload tier: the command resolves nowhere in the open
        // workspace. Ask the auto-load / package database, merging the
        // defining library file into the index so this query — and every
        // later references / rename / definition — sees its definitions.
        let names = self
            .ensure_logical_library_candidates(uri, source, analysis, pos)
            .await?
            .map_or_default(|(candidates, _)| candidates)
            .into_iter()
            .map(|candidate| candidate.qualified_name().to_owned())
            .collect::<Vec<_>>();
        Ok(if names.is_empty() {
            WorkspaceSymbolSelection::Miss
        } else {
            WorkspaceSymbolSelection::Candidates(names)
        })
    }

    /// Prepare any on-demand library indexing without holding
    /// `rehoming_gate`, then reconcile and return the symbol identities while
    /// keeping that gate held for the caller's complete workspace-index read.
    ///
    /// The second indexed lookup is essential: reconciliation may replace a
    /// sourced document's standalone identity with one or more source-site
    /// identities. If that replacement invalidates a preflight hit, retry the
    /// autoload path once outside the transaction before taking the final
    /// snapshot.
    pub(in super::super) async fn resolved_symbol_index_snapshot<'a>(
        &'a self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<(tokio::sync::MutexGuard<'a, ()>, Vec<String>)> {
        self.resolved_symbol_index_snapshot_for_purpose(
            uri,
            source,
            analysis,
            pos,
            WorkspaceSymbolPurpose::ReadonlySourceAdvice,
        )
        .await
    }

    pub(in super::super) async fn resolved_symbol_index_snapshot_for_binding<'a>(
        &'a self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<(tokio::sync::MutexGuard<'a, ()>, Vec<String>)> {
        self.resolved_symbol_index_snapshot_for_purpose(
            uri,
            source,
            analysis,
            pos,
            WorkspaceSymbolPurpose::Binding,
        )
        .await
    }

    async fn resolved_symbol_index_snapshot_for_purpose<'a>(
        &'a self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        purpose: WorkspaceSymbolPurpose,
    ) -> DocumentSourceResult<(tokio::sync::MutexGuard<'a, ()>, Vec<String>)> {
        let preflight = self
            .resolve_workspace_symbols_for_purpose(uri, source, analysis, pos, purpose)
            .await?;
        let guard = self.rehomed_index_guard().await?;
        let symbols = self
            .workspace_symbols_indexed(uri, source, analysis, pos, purpose)
            .await?
            .report();
        if !symbols.is_empty() || preflight.is_empty() {
            return Ok((guard, symbols));
        }

        // The preflight resolved only through a stale standalone view that
        // reconciliation removed. Any package lookup must happen after
        // releasing the transaction gate, then the final snapshot reconciles
        // once more around the resulting index state.
        drop(guard);
        let _ = self
            .resolve_workspace_symbols_for_purpose(uri, source, analysis, pos, purpose)
            .await?;
        let guard = self.rehomed_index_guard().await?;
        let symbols = self
            .workspace_symbols_indexed(uri, source, analysis, pos, purpose)
            .await?
            .report();
        Ok((guard, symbols))
    }
}
