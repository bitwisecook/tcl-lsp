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

//! Hover for symbols another document or a library declares.

use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;
use std::str::FromStr;
use std::sync::Arc;

use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::hover::{Hover as CoreHover, HoverKind as CoreHoverKind};
use tcl_lsp_core::original_indexed_location::OriginalIndexedSourceLocation;
use tcl_lsp_core::{
    definition as core_definition, hover as core_hover, ilx_navigation as core_ilx,
};
use tower_lsp_server::jsonrpc;
use tower_lsp_server::ls_types::{Hover, Position, Uri};

use super::LogicalIndexedDefinitionTargets;
use super::symbol_resolution::{WorkspaceSymbolPurpose, WorkspaceSymbolSelection};
use crate::documents::DocumentSourceResult;
use crate::protocol::{lift_hover, line_col_to_byte_offset_indexed};
use crate::uri_norm::DocumentPath as _;
use crate::{Backend, DocumentState};

struct FactoryHoverSources {
    pub(in super::super) file: tcl_lsp_db::SourceFile,
    pub(in super::super) project: tcl_lsp_db::Project,
    pub(in super::super) sources: Vec<(Uri, tcl_lsp_db::SourceFile, Arc<str>)>,
}

impl Backend {
    async fn conditional_foreign_factory_sources(
        &self,
        uri: &Uri,
        caller: &DocumentState,
    ) -> Option<FactoryHoverSources> {
        let db = self.db.lock().await;
        let files = self.db_files.lock().await;
        let project = *self.db_project.lock().await.as_ref()?;
        let file = *files.get(uri)?;
        if file.text(&*db).as_str() != caller.text.as_ref() || file.dialect(&*db) != &caller.dialect
        {
            return None;
        }
        if file
            .workspace_class_factories(&*db)
            .as_ref()
            .is_none_or(|map| map.is_empty())
        {
            return None;
        }
        let mut owners = HashMap::new();
        for (member_uri, &member) in files.iter() {
            if owners.insert(member, member_uri).is_some() {
                return None;
            }
        }
        let mut seen = HashSet::new();
        let mut sources = Vec::new();
        for &member in project.files(&*db) {
            if seen.insert(member) {
                let member_uri = owners.get(&member)?;
                sources.push((
                    Uri::clone(member_uri),
                    member,
                    Arc::<str>::from(member.text(&*db).as_str()),
                ));
            }
        }
        if !seen.contains(&file) {
            return None;
        }
        Some(FactoryHoverSources {
            file,
            project,
            sources,
        })
    }

    /// Rejoin every declaring source before presenting conditional factory syntax.
    pub(in super::super) async fn conditional_foreign_factory_source_hover(
        &self,
        uri: &Uri,
        caller: &DocumentState,
        inputs_epoch: u64,
        caller_generation: u64,
        position: Position,
        caller_analysis: &AnalysisResult,
    ) -> Option<CoreHover> {
        {
            let db = self.db.lock().await;
            let files = self.db_files.lock().await;
            let file = *files.get(uri)?;
            if file
                .workspace_class_factories(&*db)
                .as_ref()
                .is_none_or(|map| map.is_empty())
            {
                return None;
            }
        }
        if !core_hover::may_have_foreign_factory_hover(
            &caller.text,
            position.line,
            position.character,
            caller_analysis,
        ) {
            return None;
        }
        let factory_generation = *self.class_factory_generation.read().await;
        let config = self.resolved_db_config(uri).await;
        let FactoryHoverSources {
            file,
            project,
            sources,
        } = self
            .conditional_foreign_factory_sources(uri, caller)
            .await?;
        let mut leases = Vec::with_capacity(sources.len());
        for (member_uri, _, source) in &sources {
            leases.push(
                self.capture_readonly_source_document(member_uri.as_str(), Arc::clone(source))
                    .await
                    .ok()??,
            );
        }
        let (snapshot, revision) = {
            let db = self.db.lock().await;
            let files = self.db_files.lock().await;
            let current_project = self.db_project.lock().await;
            let members: HashSet<_> = project.files(&*db).iter().copied().collect();
            if *current_project != Some(project)
                || members.len() != sources.len()
                || sources.iter().any(|(member_uri, member, source)| {
                    files.get(member_uri) != Some(member)
                        || !members.contains(member)
                        || member.text(&*db) != source.as_ref()
                })
            {
                return None;
            }
            let revision = tcl_lsp_db::database_revision(&db);
            let snapshot = self
                .db
                .snapshot_from(&db, "conditional_foreign_factory_source_hover")?;
            (snapshot, revision)
        };
        let hover = crate::rt::spawn_blocking(move || {
            salsa::Cancelled::catch(|| {
                use salsa::plumbing::AsId as _;
                let headers =
                    tcl_lsp_db::project_factory_header_census_for_inputs(&*snapshot, project)?;
                let analysis = tcl_lsp_db::file_analysis_incremental(&*snapshot, file, config);
                tcl_lsp_core::hover::conditional_foreign_factory_hover(
                    file.text(&*snapshot),
                    position.line,
                    position.character,
                    &analysis,
                    file.as_id().as_bits(),
                    &headers,
                )
            })
            .ok()
            .flatten()
        })
        .await
        .ok()
        .flatten()?;
        self.publish_factory_source_if_current(
            &leases,
            inputs_epoch,
            factory_generation,
            revision,
            (uri, caller, caller_generation),
            hover,
        )
        .await
        .ok()
        .flatten()
    }

    async fn hover_for_original_declaration(
        &self,
        identity: &tcl_lsp_core::original_declaration::OriginalDeclarationIdentity,
    ) -> DocumentSourceResult<Option<CoreHover>> {
        let Some(owner) = identity.uri().parse::<Uri>().ok() else {
            return Ok(None);
        };
        let Some(document) = self.read_document(&owner).await? else {
            return Ok(None);
        };
        let current = self
            .analysis_for(&owner, document.text.clone(), document.dialect)
            .await?;
        if !identity.is_current(owner.as_str(), &document.text, &current) {
            return Ok(None);
        }
        let position = tcl_lexer::LineIndex::new(&document.text)
            .position_at_utf16(identity.span().start(), &document.text);
        Ok(core_hover::hover(
            &document.text,
            position.line,
            position.character.get(),
            &current,
            None,
        ))
    }

    /// Hover for a command head the current document cannot resolve, using
    /// the same two fallback tiers `compute_definition` has: the
    /// cross-document workspace index, then the autoload / package database.
    ///
    /// Go-to-definition, find-references, and the unknown-command diagnostic
    /// all resolve a tcllib-shaped command whose `proc` lives in a sibling
    /// file reached through `source` or `auto_path`/`tclIndex`.  Without these
    /// tiers hover would be the one provider that gives up at the file
    /// boundary and silently shows nothing, at the very call site the other
    /// three resolve.
    ///
    /// The body is rendered from the *defining* document's own analysis
    /// ([`core_hover::qualified_symbol_hover`]) — the same renderer, and the
    /// same input, that hovering the declaration itself uses, so the two can
    /// never disagree.
    pub(in super::super) async fn cross_document_hover(
        &self,
        uri: &Uri,
        source: &str,
        pos: Position,
        analysis: &AnalysisResult,
    ) -> DocumentSourceResult<Option<CoreHover>> {
        if !analysis.allows_retained_logical_declaration_advice() {
            let Some(identity) = self
                .original_declaration_identity_at(uri, source, analysis, pos)
                .await?
            else {
                return Ok(None);
            };
            return self.hover_for_original_declaration(&identity).await;
        }
        // Cross-document tier: a proc / class defined in a sibling document
        // the workspace index knows about.
        let selection = self
            .resolve_workspace_symbol_selection_for_purpose(
                uri,
                source,
                analysis,
                pos,
                WorkspaceSymbolPurpose::ReadonlySourceAdvice,
            )
            .await?;
        let conditional = matches!(
            &selection,
            WorkspaceSymbolSelection::ConditionalEmpty
                | WorkspaceSymbolSelection::ConditionalCandidates(_)
        );
        let symbols = selection.report();
        for qualified in &symbols {
            if let Some(hover) = self.hover_for_indexed_symbol(qualified).await? {
                return Ok(Some(hover));
            }
        }
        // A conditional report has unresolved alternatives, even if a selected
        // source owner cannot supply hover text. It never proves an autoload miss.
        if conditional {
            return Ok(None);
        }
        // Autoload tier, hover's twin of the definition one: the command is
        // defined nowhere in the open workspace, but `tclIndex` /
        // `pkgIndex.tcl` on the configured library paths says which file
        // defines it.  `ensure_autoload_indexed`
        // merges that file into the workspace index, so the render below reads
        // it exactly like any other indexed document.
        self.logical_library_hover_for_snapshot(uri, source, analysis, pos)
            .await
    }

    async fn logical_library_hover_for_snapshot(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        position: Position,
    ) -> DocumentSourceResult<Option<CoreHover>> {
        let epoch = self.diag_inputs_epoch();
        let generation = self.live_publication_uri_generation(uri);
        let Some(document) = self.read_document(uri).await? else {
            return Ok(None);
        };
        if document.text.as_ref() != source {
            return Ok(None);
        }
        let Some(offset) = line_col_to_byte_offset_indexed(
            &document.line_index,
            source,
            position.line,
            position.character,
        )
        .and_then(|offset| u32::try_from(offset).ok()) else {
            return Ok(None);
        };
        let actual = document.line_index.position_at_utf16(offset, source);
        if actual.line != position.line || actual.character.get() != position.character {
            return Ok(None);
        }
        let Some((_, resolver_revision)) = self
            .ensure_logical_library_candidates(uri, source, analysis, position)
            .await?
        else {
            return Ok(None);
        };
        let Some(LogicalIndexedDefinitionTargets { call, targets }) = self
            .logical_indexed_definition_targets(uri, source, analysis, offset)
            .await?
        else {
            return Ok(None);
        };
        let selected = self
            .capture_logical_declaration_targets(&call, targets)
            .await?;
        let hovers = self
            .with_current_declaration_sources(
                (uri.clone(), document, generation),
                selected,
                epoch,
                |_, target, current_analysis, span| {
                    let resolver = self.package_resolver.try_read().ok()?;
                    if resolver.revision() != resolver_revision {
                        return None;
                    }
                    let position = target
                        .line_index
                        .position_at_utf16(span.start(), &target.text);
                    core_hover::hover(
                        &target.text,
                        position.line,
                        position.character.get(),
                        current_analysis,
                        None,
                    )
                },
            )
            .await?;
        Ok(hovers.into_iter().next())
    }

    /// Render `qualified` from whichever indexed document declares it.
    ///
    /// A name several documents declare renders from the first that yields a
    /// body; go-to-definition offers every site, but a hover has one popup, so
    /// picking one is the only option and the index's own order decides.
    /// Hover for a namespace variable declared in a sibling document — the
    /// hover twin of [`Self::cross_document_variable_definition`], rendered
    /// from the *declaring* document's own analysis (memoised by
    /// [`Self::analysis_for`], so this is a cache hit once that document has
    /// been analysed once).
    pub(in super::super) async fn cross_document_variable_hover(
        &self,
        uri: &Uri,
        source: &str,
        pos: Position,
        analysis: &AnalysisResult,
    ) -> DocumentSourceResult<Option<CoreHover>> {
        if let std::ops::ControlFlow::Break(Some(occurrence)) =
            tcl_lsp_core::variable_symbol::select(source, analysis, pos.line, pos.character)
        {
            let rehoming_guard = self.rehomed_index_guard().await?;
            let targets = {
                let index = self.workspace_index.read().await;
                index
                    .original_variable_occurrences(occurrence.symbol(), uri.as_str())
                    .filter_map(|(owner, occurrence)| {
                        OriginalIndexedSourceLocation::from_variable(
                            owner,
                            index.diagnostic_source_context(owner)?,
                            occurrence,
                        )
                    })
                    .collect()
            };
            drop(rehoming_guard);
            let counts = self
                .validated_original_indexed_locations(targets)
                .await?
                .into_iter()
                .fold((0, 0), |(declarations, references), (target, _)| {
                    if target.is_declaration() {
                        (declarations + 1, references)
                    } else {
                        (declarations, references + 1)
                    }
                });
            if counts.0 == 0 {
                return Ok(None);
            }
            let input = occurrence.original_name_input();
            let label = tcl_syntax::native_string::resident_name_label(input.bytes());
            return Ok(Some(CoreHover {
                kind: CoreHoverKind::Markdown,
                value: format!(
                    "**Variable** `{label}`\n\n{} declaration(s) and {} reference(s) in other documents",
                    counts.0, counts.1
                ),
            }));
        }
        let Some(cell) = Self::qualified_variable_cell(
            source,
            tcl_lsp_core::profile_for_dialect(&analysis.dialect),
            analysis,
            pos,
        ) else {
            return Ok(None);
        };
        let rehoming_guard = self.rehomed_index_guard().await?;
        let target_uris: Vec<String> = {
            let index = self.workspace_index.read().await;
            index
                .variable_definitions_qualified(&cell, uri.as_str())
                .into_iter()
                .map(|v| v.uri.clone())
                .collect()
        };
        drop(rehoming_guard);
        for target_uri in target_uris {
            let Ok(parsed) = Uri::from_str(&target_uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await? else {
                continue;
            };
            let target_analysis = self
                .analysis_for(&parsed, target_doc.text.clone(), target_doc.dialect.clone())
                .await?;
            if let Some(hover) = core_hover::qualified_variable_hover(&target_analysis, &cell) {
                return Ok(Some(hover));
            }
        }
        Ok(None)
    }

    async fn hover_for_indexed_symbol(
        &self,
        qualified: &str,
    ) -> DocumentSourceResult<Option<CoreHover>> {
        let target_uris: Vec<String> = {
            let index = self.workspace_index.read().await;
            index
                .proc_definitions_qualified(qualified, "")
                .into_iter()
                .map(|p| p.uri.clone())
                .chain(
                    index
                        .class_definitions_qualified(qualified, "")
                        .into_iter()
                        .map(|c| c.uri.clone()),
                )
                .collect()
        };
        for target_uri in target_uris {
            let Ok(parsed) = Uri::from_str(&target_uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await? else {
                continue;
            };
            // Memoised by `analysis_for`, so a hover over an already-analysed
            // sibling costs a cache hit, not a second whole-file walk.
            let target_analysis = self
                .analysis_for(&parsed, target_doc.text.clone(), target_doc.dialect.clone())
                .await?;
            if let Some(hover) = core_hover::qualified_symbol_hover(&target_analysis, qualified) {
                return Ok(Some(hover));
            }
        }
        Ok(None)
    }
}

impl Backend {
    pub(in super::super) async fn position_hover_before_method(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<ControlFlow<Option<Hover>>> {
        // A namespace-name argument is answered here, in full, and never
        // reaches the tiers below — the position is definitive, and the
        // counts describe the whole workspace rather than just this document.
        // `None` means no file in view declares the namespace, which is a real
        // "no hover", not a licence to fall through to command documentation.
        if let Some(symbol) = Self::original_namespace_symbol(&doc.text, analysis, pos) {
            return Ok(ControlFlow::Break(
                self.original_namespace_hover(uri, analysis, &symbol)
                    .await?
                    .map(lift_hover),
            ));
        }
        if Self::original_namespace_cursor_retained(&doc.text, analysis, pos) {
            return Ok(ControlFlow::Break(None));
        }
        if let Some(cell) = Self::namespace_cell(&doc.text, analysis, pos) {
            return Ok(ControlFlow::Break(
                self.namespace_hover(uri, analysis, &cell)
                    .await?
                    .map(lift_hover),
            ));
        }
        // An iRulesLX method word names a JavaScript function, not a Tcl one,
        // so it is answered here rather than by the command-documentation
        // tiers below. The body always says which of the two ILX
        // commands reached it — they share the method target but not the
        // semantics — and, when nothing resolved, why.
        if let Some(path) = uri.document_path()
            && let Some((call, target)) = self.ilx_method_at(uri, doc, &path, pos).await?
        {
            return Ok(ControlFlow::Break(Some(lift_hover(CoreHover {
                value: core_ilx::hover_markdown(&call, &target),
                kind: CoreHoverKind::Markdown,
            }))));
        }
        Ok(ControlFlow::Continue(()))
    }
}

impl Backend {
    pub(in super::super) async fn hover_workspace_fallback(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
        on_command_head: bool,
    ) -> jsonrpc::Result<Option<Hover>> {
        // Nothing in this document explains the word.  A per-object
        // visibility mask makes `$obj m` answer `unknown method` regardless
        // of the class chain, so it is a *definitive* no-hover and must not
        // fall through to the cross-file method tier — the same gate
        // `compute_definition` applies ahead of its own.
        if core_definition::object_dispatch_mask(analysis, &doc.text, pos.line, pos.character)
            .withholds_workspace_fallback()
        {
            return Ok(None);
        }
        // A `my method` / `$obj method` dispatch whose provider class lives
        // in a sibling document.  Answered before the
        // command-head tiers because a method-name token is never a command
        // head — its head is `my` / `$obj` — so it would otherwise never
        // reach any cross-document tier at all.
        if let Some(hover) = Box::pin(self.cross_file_method_hover(uri, doc, analysis, pos)).await?
        {
            return Ok(Some(lift_hover(hover)));
        }
        // A namespace-qualified variable whose declaration is in a sibling
        // document — a `$var` site is never a command head either, so it
        // would never reach the tiers below.
        if let Some(hover) = self
            .cross_document_variable_hover(uri, &doc.text, pos, analysis)
            .await?
        {
            return Ok(Some(lift_hover(hover)));
        }
        // If the word is a command head, resolve it the way go-to-definition
        // does — across the workspace, then through the autoload / package
        // database.
        if !on_command_head {
            return Ok(None);
        }
        Ok(self
            .cross_document_hover(uri, &doc.text, pos, analysis)
            .await?
            .map(lift_hover))
    }
}

impl Backend {
    pub(in super::super) fn hover_worker_error(err: &impl std::fmt::Display) -> jsonrpc::Error {
        jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("hover worker panicked: {err}").into(),
            data: None,
        }
    }
}
