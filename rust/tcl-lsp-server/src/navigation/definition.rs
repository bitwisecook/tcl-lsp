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

//! Go-to-definition: the local provider and its workspace and autoload fallbacks.

use std::sync::Arc;

use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::original_indexed_location::OriginalIndexedSourceLocation;
use tcl_lsp_core::{definition as core_definition, workspace_index as core_workspace_index};
use tower_lsp_server::jsonrpc;
use tower_lsp_server::ls_types::{Location, Position, Uri};

use crate::documents::{DocumentSourceResult, canonical_file_uri};
use crate::protocol::{lift_lsp_range, line_col_to_byte_offset_indexed};
use crate::{Backend, DocumentState, fresh_analysis_cache};

use super::{LogicalNamespaceLocationsPurpose, LogicalVariableLocationsPurpose};

/// Observations that can require reissuing a failed readonly autoload query.
/// They confer no header, lookup, completion or source-currentness authority.
struct AutoloadQueryObservation {
    request: core_definition::OriginalAutoloadRequest,
    pub(in super::super) inputs_epoch: u64,
    pub(in super::super) factory_generation: u64,
    sidecar: Arc<fresh_analysis_cache::SidecarRevision>,
    sidecar_published: bool,
    pub(in super::super) resolver_revision: u64,
    pub(in super::super) providers: Vec<(String, Option<u64>)>,
}

impl Backend {
    fn original_autoload_head_eligible(
        source: &str,
        analysis: &AnalysisResult,
        position: Position,
    ) -> bool {
        !analysis.allows_lexical_declaration_advice()
            && core_definition::original_autoload_request_at(
                source,
                analysis,
                position.line,
                position.character,
            )
            .is_some()
    }

    pub(in super::super) async fn definition_workspace_fallback(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
        (source_epoch, source_generation): (u64, u64),
        on_command_head: bool,
    ) -> jsonrpc::Result<Vec<Location>> {
        // A per-object visibility mask (`oo::objdefine $o { unexport m }`, or
        // an unexported per-object member) makes `$obj m` answer `unknown
        // method` regardless of the class chain.  The
        // in-document provider answers such a call with a *definitive* empty
        // result, which is indistinguishable from "no local answer" here —
        // without this gate the cross-file method tier below would resolve
        // the class's own member right past the mask.
        if core_definition::object_dispatch_mask(analysis, &doc.text, pos.line, pos.character)
            .withholds_workspace_fallback()
        {
            return Ok(Vec::new());
        }
        if analysis.resolved_input.as_ref().is_some_and(
            tcl_compiler::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
        ) && let Some(locations) = Box::pin(self.logical_class_member_locations_for_snapshot(
            (uri.clone(), doc.clone(), source_generation),
            analysis,
            pos,
            source_epoch,
        ))
        .await?
        {
            return Ok(locations);
        }
        // Cross-file TclOO method definition: a `$obj method` / `my method`
        // call (or a method-name cursor in a class body) whose defining class
        // lives in another file — e.g. an inherited method declared in the
        // base class's own document.  A method call's head is `$obj`, not the
        // method token, so this is not gated on `on_command_head`.  Resolved
        // oracle-aware so a pure-consumer receiver (class defined elsewhere)
        // still identifies the method, and access-context-aware so an
        // external call resolves the exported dispatch entry only.
        if analysis
            .resolved_input
            .as_ref()
            .is_some_and(|input| tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(&input.borrowed_context_registry().context().environment.id).is_some_and(tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::is_appliance_hosted))
            && let Some((class_q, method, is_classmethod, access)) = self
                .resolve_method_target(
                    uri,
                    &doc.text,
                    tcl_lsp_core::profile_for_dialect(&doc.dialect),
                    analysis,
                    pos,
                )
                .await?
            {
                let method_defs = self
                    .cross_file_method_definition(uri, &class_q, &method, access, is_classmethod)
                    .await?;
                if !method_defs.is_empty() {
                    return Ok(method_defs);
                }
            }
        // Cross-file namespace variable: `$::NS::var` whose declaring
        // `namespace eval NS { variable var }` is in another document.
        // Not gated on `on_command_head` — a `$var`
        // site never is one — and safe without that gate because the cell is
        // named exactly, not matched by simple name.
        let cross_var = if analysis.resolved_input.as_ref().is_some_and(
            tcl_compiler::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
        ) {
            self.logical_variable_locations_for_snapshot(
                (uri.clone(), doc.clone(), source_generation),
                analysis,
                pos,
                source_epoch,
                LogicalVariableLocationsPurpose::Definitions,
            )
            .await?
        } else {
            self.cross_document_variable_definition(uri, &doc.text, pos, analysis)
                .await?
        };
        if !cross_var.is_empty() {
            return Ok(cross_var);
        }
        if !on_command_head && !Self::original_autoload_head_eligible(&doc.text, analysis, pos) {
            return Ok(Vec::new());
        }
        // Cross-document fallback: resolve a proc / class
        // defined in a sibling document via the workspace index.
        if !analysis.allows_lexical_declaration_advice() {
            return Ok(self
                .original_definition_locations_for_snapshot(
                    (uri.clone(), doc.clone(), source_generation),
                    analysis,
                    pos,
                    source_epoch,
                )
                .await?);
        }
        if analysis.resolved_input.as_ref().is_some_and(
            tcl_compiler::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
        ) {
            return Ok(self
                .logical_definition_locations_for_snapshot(
                    (uri.clone(), doc.clone(), source_generation),
                    analysis,
                    pos,
                    source_epoch,
                )
                .await?);
        }
        let cross = self
            .cross_document_definition(uri, &doc.text, pos, analysis)
            .await?;
        if !cross.is_empty() {
            return Ok(cross);
        }
        // Autoload tier: the command is defined nowhere in the open
        // workspace, but the package / auto-load database may know which
        // library file (`tclIndex` / `pkgIndex.tcl` on the configured
        // `libraryPaths` / `TCLLIBPATH`) defines it.  Resolve that file, analyse
        // it on demand (memoised by `analysis_for`), and jump to the proc.
        self.autoload_definition(uri, &doc.text, pos, analysis)
            .await
    }

    /// Shared helper for the goto-definition family — runs the
    /// pure-CPU `tcl_lsp_core::definition::definition` provider
    /// off the LSP event loop and returns the matched ranges.
    pub(in super::super) async fn compute_definition(
        &self,
        uri: &Uri,
        pos: Position,
    ) -> jsonrpc::Result<Vec<Location>> {
        for _ in 0..3 {
            let source_epoch = self.diag_inputs_epoch();
            let source_generation = self.live_publication_uri_generation(uri);
            let factory_generation = *self.class_factory_generation.read().await;
            let sidecar = self
                .fresh_analysis_cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .revision();
            let sidecar_published = sidecar.is_published_current();
            let Some(doc) = self.read_document(uri).await? else {
                return Ok(Vec::new());
            };
            if let Some(locations) = self.ilx_definition(uri, &doc, pos).await? {
                return Ok(locations);
            }
            let analysis = self
                .analysis_for(uri, doc.text.clone(), doc.dialect.clone())
                .await?;
            let observation = if analysis.allows_lexical_declaration_advice() {
                None
            } else if let Some(request) = core_definition::original_autoload_request_at(
                &doc.text,
                &analysis,
                pos.line,
                pos.character,
            ) {
                let resolver = self.package_resolver.read().await;
                let resolver_revision = resolver.revision();
                let candidates = request.files(&resolver);
                drop(resolver);
                let index = self.workspace_index.read().await;
                let providers = candidates
                    .iter()
                    .filter_map(canonical_file_uri)
                    .map(|uri| {
                        (
                            uri.as_str().to_owned(),
                            index.document_revision(uri.as_str()),
                        )
                    })
                    .collect();
                drop(index);
                Some(AutoloadQueryObservation {
                    request,
                    inputs_epoch: source_epoch,
                    factory_generation,
                    sidecar,
                    sidecar_published,
                    resolver_revision,
                    providers,
                })
            } else {
                None
            };
            let locations = self
                .compute_definition_for_snapshot(
                    uri,
                    pos,
                    &doc,
                    &analysis,
                    (source_epoch, source_generation),
                )
                .await?;
            if !locations.is_empty() {
                return Ok(locations);
            }
            let Some(observation) = observation else {
                return Ok(locations);
            };
            if !self
                .autoload_query_requires_recapture(uri, &doc, pos, source_generation, &observation)
                .await?
            {
                return Ok(locations);
            }
        }
        Ok(Vec::new())
    }

    async fn autoload_query_requires_recapture(
        &self,
        uri: &Uri,
        document: &DocumentState,
        position: Position,
        generation: u64,
        observation: &AutoloadQueryObservation,
    ) -> DocumentSourceResult<bool> {
        if self
            .class_type_source(uri, document.clone(), generation)
            .await
            .is_none()
        {
            return Ok(false);
        }
        let Some(current) = self.read_document(uri).await? else {
            return Ok(false);
        };
        if current.text != document.text
            || current.revision != document.revision
            || current.version != document.version
            || current.dialect != document.dialect
            || current.saved_source_bytes != document.saved_source_bytes
            || self.live_publication_uri_generation(uri) != generation
        {
            return Ok(false);
        }
        let analysis = self
            .analysis_for(uri, current.text.clone(), current.dialect)
            .await?;
        let Some(request) = core_definition::original_autoload_request_at(
            &current.text,
            &analysis,
            position.line,
            position.character,
        ) else {
            return Ok(false);
        };
        let sidecar_current = self
            .fresh_analysis_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_current(&observation.sidecar);
        let index = self.workspace_index.read().await;
        let providers_changed = observation
            .providers
            .iter()
            .any(|(uri, revision)| index.document_revision(uri) != *revision);
        drop(index);
        let inputs_epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        let resolver_revision = self.package_resolver.read().await.revision();
        Ok(request != observation.request
            || inputs_epoch != observation.inputs_epoch
            || factory_generation != observation.factory_generation
            || !sidecar_current
            || observation.sidecar.is_published_current() != observation.sidecar_published
            || resolver_revision != observation.resolver_revision
            || providers_changed)
    }

    async fn compute_definition_for_snapshot(
        &self,
        uri: &Uri,
        pos: Position,
        doc: &DocumentState,
        analysis: &Arc<AnalysisResult>,
        (source_epoch, source_generation): (u64, u64),
    ) -> jsonrpc::Result<Vec<Location>> {
        // The cross-document fallback should only fire when the cursor
        // sits on a command head (a call site), matching what the
        // in-document provider resolves.  Compute that here, while
        // `analysis` is still in scope (it is moved into the worker
        // below).  Without this gate, an argument word that happens to
        // collide with a sibling proc/class name produces a
        // false-positive go-to-definition jump.
        let on_command_head = position_is_command_head(&doc.text, pos, analysis, &doc.line_index);
        // A namespace-name argument is answered here, in full, and never
        // reaches the tiers below.  Two reasons: the position is
        // *definitive* (a proc or class of the same
        // spelling is not what it names), and the answer is the union of the
        // local and workspace declaring blocks — reopening a namespace
        // extends the same namespace, so a local block does not make a
        // sibling document's block any less of a definition.
        if let Some(symbol) = Self::original_namespace_symbol(&doc.text, analysis, pos) {
            if analysis.resolved_input.as_ref().is_some_and(
                tcl_compiler::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
            ) {
                return Ok(Box::pin(self.logical_namespace_locations_for_snapshot(
                    (uri.clone(), doc.clone(), source_generation),
                    analysis,
                    pos,
                    source_epoch,
                    LogicalNamespaceLocationsPurpose::Definitions,
                ))
                .await?);
            }
            return Ok(self
                .original_namespace_locations(uri, &doc.text, analysis, &symbol, true, true)
                .await?);
        }
        if Self::original_namespace_cursor_retained(&doc.text, analysis, pos) {
            return Ok(Vec::new());
        }
        if let Some(cell) = Self::namespace_cell(&doc.text, analysis, pos) {
            return Ok(self
                .namespace_declaration_locations(uri, &doc.text, analysis, &cell)
                .await?);
        }
        if let std::ops::ControlFlow::Break(locations) =
            Box::pin(self.original_method_locations_for_snapshot(
                (uri.clone(), doc.clone(), source_generation),
                analysis,
                pos,
                source_epoch,
            ))
            .await?
        {
            return Ok(locations);
        }
        let text = doc.text.clone();
        let analysis_worker = Arc::clone(analysis);
        // The whole-program export view the in-document tier needs to decide
        // whether a `namespace import -force` really deleted this file's own
        // command of the name.  Snapshotted under the
        // index lock and moved into the worker, which runs with the lock
        // released.
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let in_doc = crate::rt::spawn_blocking(move || {
            core_definition::definition_with(
                &text,
                pos.line,
                pos.character,
                &analysis_worker,
                Some(core_definition::ProgramExports {
                    uri: &uri_key,
                    oracle: exports.as_ref(),
                }),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("definition worker panicked: {err}").into(),
            data: None,
        })?;
        if !in_doc.is_empty() {
            // The document's own walk answers exactly: a sibling's
            // `oo::define C { self unexport m }` needs `C` to exist, so it can
            // only run after this document's own create, and nothing here
            // loads it before this call (a load it does perform is walked).
            return Ok(in_doc
                .into_iter()
                .map(|r| Location {
                    uri: uri.clone(),
                    range: lift_lsp_range(r),
                })
                .collect());
        }
        self.definition_workspace_fallback(
            uri,
            doc,
            analysis,
            pos,
            (source_epoch, source_generation),
            on_command_head,
        )
        .await
    }

    /// Cross-document go-to-definition for a namespace variable: the
    /// declaration sites the workspace index holds for the cell at `pos`, in
    /// *other* documents.
    ///
    /// Only ever consulted after the in-document provider came back empty, so
    /// a locally-declared cell always answers locally.  The index lookup is an
    /// exact qualified-name match over the index's variable table — no scan of
    /// other documents, no re-analysis.
    pub(in super::super) async fn cross_document_variable_definition(
        &self,
        uri: &Uri,
        source: &str,
        pos: Position,
        analysis: &AnalysisResult,
    ) -> DocumentSourceResult<Vec<Location>> {
        if let std::ops::ControlFlow::Break(Some(occurrence)) =
            tcl_lsp_core::variable_symbol::select(source, analysis, pos.line, pos.character)
        {
            let rehoming_guard = self.rehomed_index_guard().await?;
            let targets = {
                let index = self.workspace_index.read().await;
                index
                    .original_variable_occurrences(occurrence.symbol(), uri.as_str())
                    .filter(|(_, occurrence)| occurrence.is_declaration())
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
            return self.resolve_original_indexed_locations(targets).await;
        }
        let Some(cell) = Self::qualified_variable_cell(
            source,
            tcl_lsp_core::profile_for_dialect(&analysis.dialect),
            analysis,
            pos,
        ) else {
            return Ok(Vec::new());
        };
        // The index answers under source-site namespaces too, so
        // reconcile before asking, exactly as the proc/class tier does.
        let rehoming_guard = self.rehomed_index_guard().await?;
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.read().await;
            index
                .variable_definitions_qualified(&cell, uri.as_str())
                .into_iter()
                .map(|v| (v.uri.clone(), v.name_span))
                .collect()
        };
        drop(rehoming_guard);
        self.resolve_target_locations(targets).await
    }

    /// Autoload-tier go-to-definition: resolve a command head that the
    /// workspace index cannot place to the library file the auto-load / package
    /// database says defines it, then jump to that proc's declaration.
    ///
    /// The library file need not be open — [`Self::ensure_autoload_indexed`]
    /// reads it from disk, analyses it, and merges it into the workspace index,
    /// so the jump (and every later references / rename / definition query)
    /// answers from the shared index.  Only fires on a genuine miss, so it
    /// never overrides an in-workspace definition.
    pub(in super::super) async fn autoload_definition(
        &self,
        uri: &Uri,
        source: &str,
        pos: Position,
        analysis: &AnalysisResult,
    ) -> jsonrpc::Result<Vec<Location>> {
        if !analysis.allows_retained_logical_declaration_advice() {
            if core_definition::original_autoload_request_at(
                source,
                analysis,
                pos.line,
                pos.character,
            )
            .is_none()
            {
                return Ok(Vec::new());
            }
            return self
                .cross_document_definition(uri, source, pos, analysis)
                .await;
        }
        let epoch = self.diag_inputs_epoch();
        let generation = self.live_publication_uri_generation(uri);
        let Some(document) = self.read_document(uri).await? else {
            return Ok(Vec::new());
        };
        if document.text.as_ref() != source {
            return Ok(Vec::new());
        }
        self.logical_definition_locations_for_snapshot(
            (uri.clone(), document, generation),
            analysis,
            pos,
            epoch,
        )
        .await
        .map_err(Into::into)
    }

    /// Resolve the symbol at `pos` against the workspace index
    /// when the current document has no local definition.  Only
    /// fires on bare command words (not `$var` references), and
    /// returns `Location`s pointing into the *defining*
    /// documents.
    pub(in super::super) async fn cross_document_definition(
        &self,
        uri: &Uri,
        source: &str,
        pos: Position,
        analysis: &AnalysisResult,
    ) -> jsonrpc::Result<Vec<Location>> {
        if !analysis.allows_lexical_declaration_advice() {
            let Some(identity) = self
                .original_declaration_identity_at(uri, source, analysis, pos)
                .await?
            else {
                return Ok(Vec::new());
            };
            let Ok(owner) = identity.uri().parse::<Uri>() else {
                return Ok(Vec::new());
            };
            let Some(document) = self.read_document(&owner).await? else {
                return Ok(Vec::new());
            };
            let current = self
                .analysis_for(&owner, document.text.clone(), document.dialect)
                .await?;
            if !identity.is_current(owner.as_str(), &document.text, &current) {
                return Ok(Vec::new());
            }
            return Ok(vec![Location {
                uri: owner,
                range: lift_lsp_range(core_definition::span_to_range(
                    &document.text,
                    &tcl_lexer::LineIndex::new(&document.text),
                    identity.span(),
                )),
            }]);
        }
        // Reconcile the index with the source graph first: sourced
        // documents answer under their source-site namespaces.
        let (rehoming_guard, symbols) = self
            .resolved_symbol_index_snapshot(uri, source, analysis, pos)
            .await?;
        // Resolve the call at the cursor to its qualified identity set
        // through the workspace oracle, then jump to *those* symbols'
        // definitions — never every same-simple-name proc/class across the
        // project, which a bare name lookup would surface as spurious extra
        // jump targets.  (A multi-seeded declaration cursor yields several
        // identities sharing one physical definition — the target set
        // dedupes to the physical site.)
        if symbols.is_empty() {
            return Ok(Vec::new());
        }
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.read().await;
            let mut targets: Vec<(String, tcl_lexer::Span)> = Vec::new();
            for qualified in &symbols {
                let classes = index.class_definitions_qualified(qualified, "");
                // Prefer real `oo::class create` sites over cross-file
                // `oo::define` extension stubs; fall back to every site only
                // when no true creation site is indexed (the class is defined
                // solely by `oo::define`, e.g. on a built-in).
                let creation_sites: Vec<_> = classes.iter().filter(|c| !c.via_define).collect();
                let class_targets: Vec<(String, tcl_lexer::Span)> = if creation_sites.is_empty() {
                    classes
                        .iter()
                        .map(|c| (c.uri.clone(), c.name_span))
                        .collect()
                } else {
                    creation_sites
                        .iter()
                        .map(|c| (c.uri.clone(), c.name_span))
                        .collect()
                };
                targets.extend(
                    index
                        .proc_definitions_qualified(qualified, "")
                        .into_iter()
                        .map(|p| (p.uri.clone(), p.name_span))
                        .chain(class_targets),
                );
            }
            targets.sort_by_key(|(u, s)| (u.clone(), s.start(), s.end()));
            targets.dedup();
            targets
        };
        drop(rehoming_guard);
        Ok(self.resolve_target_locations(targets).await?)
    }

    /// The genuine original declaration producer retains the requesting
    /// document through all remote analysis before lifting its target span.
    /// Missing or stale ownership is terminal for this Native source route.
    pub(in super::super) async fn original_definition_locations_for_snapshot(
        &self,
        caller: (Uri, DocumentState, u64),
        analysis: &AnalysisResult,
        position: Position,
        epoch: u64,
    ) -> DocumentSourceResult<Vec<Location>> {
        if analysis.allows_lexical_declaration_advice() {
            return Ok(Vec::new());
        }
        let Some(identity) = self
            .original_declaration_identity_at(&caller.0, &caller.1.text, analysis, position)
            .await?
        else {
            return Ok(Vec::new());
        };
        let Ok(owner) = identity.uri().parse::<Uri>() else {
            return Ok(Vec::new());
        };
        let generation = self.live_publication_uri_generation(&owner);
        let Some(document) = self.read_document(&owner).await? else {
            return Ok(Vec::new());
        };
        let current = self
            .analysis_for(&owner, document.text.clone(), document.dialect.clone())
            .await?;
        if !identity.is_current(owner.as_str(), &document.text, &current) {
            return Ok(Vec::new());
        }
        let Some(context) =
            core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(&current)
        else {
            return Ok(Vec::new());
        };
        let Some(location) = OriginalIndexedSourceLocation::from_declaration(&context, &identity)
        else {
            return Ok(Vec::new());
        };
        self.resolve_current_original_class_type_location(
            caller,
            (owner, document, current, generation),
            location,
            epoch,
        )
        .await
    }
}

/// Gate legacy navigation fallback on a current, complete original command
/// head and an exact LSP cursor. Source geometry grants no binding or dispatch.
pub(in super::super) fn position_is_command_head(
    source: &str,
    pos: Position,
    analysis: &AnalysisResult,
    index: &tcl_lexer::LineIndex,
) -> bool {
    let Some((image, config)) = tcl_compiler::source_graph::current_analysis(source, analysis)
    else {
        return false;
    };
    let current_index = tcl_lexer::LineIndex::new_lsp(source);
    if index.line_count() != current_index.line_count()
        || (0..current_index.line_count()).any(|line| {
            let line = u32::try_from(line).expect("source line count fits u32");
            index.line_start(line) != current_index.line_start(line)
        })
    {
        return false;
    }
    let Some(offset) = line_col_to_byte_offset_indexed(index, source, pos.line, pos.character)
    else {
        return false;
    };
    let offset = u32::try_from(offset).expect("source offset fits u32");
    let actual_position = index.position_at_utf16(offset, source);
    if actual_position.line != pos.line || actual_position.character.get() != pos.character {
        return false;
    }
    let Some(commands) = tcl_lsp_core::SourceSyntaxStructure::capture_commands(source, analysis)
    else {
        return false;
    };
    let map = tcl_lexer::SourceMap::from_image(&image);
    commands.iter().any(|command| {
        let extent = command.execution_span(source);
        if command.is_partial
            || !command.word_views_aligned()
            || offset < extent.start()
            || extent.end() <= offset
        {
            return false;
        }
        let Some(first) = command.argv.first() else {
            return false;
        };
        let tokens = tcl_compiler::ir::CommandTokens::from_segmented(&map, config, command);
        let Some(words) = tcl_compiler::registry_invocation::original_native_compiler_words(
            &image,
            tokens.words(),
            first.span.start(),
            tokens.native_lexer_config(config),
        ) else {
            return false;
        };
        let Some(head) = words.first() else {
            return false;
        };
        let span = head.span();
        !head.group().expand
            && span.start() <= offset
            && offset < span.end()
            && analysis.command_invocations.iter().any(|invocation| {
                invocation.lookup.is_execution_site()
                    && span.start() <= invocation.range.start()
                    && invocation.range.start() < invocation.range.end()
                    && invocation.range.end() <= span.end()
            })
    })
}
