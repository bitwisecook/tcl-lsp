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

//! References across documents for commands, namespace variables and classes.

use super::{
    AnalysisResult, Backend, CoreLspRange, DocumentSourceResult, DocumentState, Location,
    LogicalNamespaceLocationsPurpose, LogicalVariableLocationsPurpose,
    OriginalIndexedSourceLocation, Position, Uri, core_definition, core_references,
    dedup_locations, jsonrpc, lift_bigip_range, lift_lsp_range,
};

impl Backend {
    /// Cross-document references for the proc / class at `pos`:
    /// every invocation site in *other* documents, plus the
    /// definition sites in other documents when
    /// `include_declaration`.  Returns `Location`s resolved
    /// against their defining documents.
    ///
    /// Excludes the current document (`uri`) throughout — correct only when
    /// the single-document provider already covers it, i.e. `uri` holds a
    /// local declaration of the symbol. When it doesn't (a pure-consumer
    /// document whose only trace of the symbol is a `source`d-in or
    /// workspace-sibling declaration), use
    /// [`Self::workspace_resolved_references`] instead, which is identical
    /// but excludes nothing.
    pub(in super::super) async fn cross_document_references(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        include_declaration: bool,
    ) -> DocumentSourceResult<Vec<Location>> {
        self.gather_reference_targets(
            uri,
            source,
            analysis,
            pos,
            include_declaration,
            uri.as_str(),
        )
        .await
    }

    /// Consumer-document references fallback, mirroring the autoload-tier
    /// rename pattern ([`Self::add_workspace_resolved_rename_edits`]): the
    /// cursor's command has no local declaration in the current document,
    /// so the single-document reference pass found nothing to anchor on —
    /// not a genuine zero-references case, since
    /// [`Self::cross_document_references`]'s exclusion of the current
    /// document assumes that pass already covers it.  A file that `source`s
    /// a sibling and then calls its procs bare would otherwise lose every one
    /// of its *own* call sites, including the one the cursor sits on.
    /// Resolves through the workspace oracle and gathers every reference the
    /// index knows, current-document call sites included.
    pub(in super::super) async fn workspace_resolved_references(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        include_declaration: bool,
    ) -> DocumentSourceResult<Vec<Location>> {
        self.gather_reference_targets(uri, source, analysis, pos, include_declaration, "")
            .await
    }

    /// Shared core of [`Self::cross_document_references`] /
    /// [`Self::workspace_resolved_references`]: every invocation, name-link,
    /// and (when `include_declaration`) definition site the workspace index
    /// has for the symbol at `pos`, excluding `exclude_uri` — the caller's
    /// own document URI for the ordinary case, or `""` (matching no real
    /// document) to exclude nothing, mirroring
    /// [`core_rename::workspace_symbol_rename_edits`]'s identical trick.
    async fn gather_reference_targets(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        include_declaration: bool,
        exclude_uri: &str,
    ) -> DocumentSourceResult<Vec<Location>> {
        let (rehoming_guard, symbols) = self
            .resolved_symbol_index_snapshot(uri, source, analysis, pos)
            .await?;
        if symbols.is_empty() {
            return Ok(Vec::new());
        }
        // A multi-seeded declaration names several runtime identities —
        // the reference set is the **union over
        // every view**, so a `::x::helper` caller and a `::y::helper`
        // caller both surface from the one physical declaration.
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.read().await;
            let mut t: Vec<(String, tcl_lexer::Span)> = Vec::new();
            for qualified in &symbols {
                // References follow command name-links: a call reaching the
                // target through an `interp alias` / `rename` / `namespace
                // import` is a use of it, as is the word that names it in
                // such a declaration.
                t.extend(
                    index
                        .linked_invocations_of(qualified, exclude_uri)
                        .into_iter()
                        .map(|i| (i.uri.clone(), i.range)),
                );
                t.extend(index.link_target_spans(qualified, exclude_uri));
                if include_declaration {
                    // Match the declaration sites by *qualified* name — a same
                    // simple name in an unrelated namespace/file is a different
                    // symbol and must not be surfaced as this one's declaration.
                    for p in index.proc_definitions_qualified(qualified, exclude_uri) {
                        t.push((p.uri.clone(), p.name_span));
                    }
                    for c in index.class_definitions_qualified(qualified, exclude_uri) {
                        t.push((c.uri.clone(), c.name_span));
                    }
                }
            }
            t.sort_by_key(|(u, s)| (u.clone(), s.start(), s.end()));
            t.dedup();
            t
        };
        drop(rehoming_guard);
        self.resolve_target_locations(targets).await
    }

    async fn retained_reference_locations(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        position: Position,
        include_declaration: bool,
    ) -> DocumentSourceResult<Vec<Location>> {
        if let std::ops::ControlFlow::Break(selected) = tcl_lsp_core::variable_symbol::select(
            &doc.text,
            analysis,
            position.line,
            position.character,
        ) {
            let Some(occurrence) = selected else {
                return Ok(Vec::new());
            };
            let Some(config) = analysis.body_lexer_config else {
                return Ok(Vec::new());
            };
            if !analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(&doc.text), config)
            {
                return Ok(Vec::new());
            }
            let index = tcl_lexer::LineIndex::new(&doc.text);
            let mut locations = tcl_lsp_core::variable_symbol::reference_spans(
                analysis,
                occurrence.symbol(),
                include_declaration,
            )
            .into_iter()
            .map(|span| Location {
                uri: uri.clone(),
                range: lift_lsp_range(core_definition::span_to_range(&doc.text, &index, span)),
            })
            .collect::<Vec<_>>();
            locations.extend(
                self.cross_document_variable_references(
                    &doc.text,
                    analysis,
                    position,
                    include_declaration,
                )
                .await?,
            );
            dedup_locations(&mut locations);
            return Ok(locations);
        }
        let identity = self
            .original_declaration_identity_at(uri, &doc.text, analysis, position)
            .await?;
        let mut locations = match &identity {
            Some(identity) => self
                .readonly_original_declaration_reference_report(identity, uri, include_declaration)
                .await?
                .map_or_else(Vec::new, |report| report.locations),
            None => Vec::new(),
        };
        // The report covers what this document's own analysis binds. A call
        // whose definition lives in another document, and the calls other
        // documents make to a definition here, are listed from the workspace
        // index, as on the unversioned profile. A listing only: rename does
        // not read it. A declaration in this document keeps its own analysis
        // as the answer for this document's calls, an empty one included: the
        // index has no order, and would list a call that runs before the
        // declaration exists.
        let cross = if identity.is_none() {
            self.workspace_resolved_references(
                uri,
                &doc.text,
                analysis,
                position,
                include_declaration,
            )
            .await?
        } else {
            self.cross_document_references(uri, &doc.text, analysis, position, include_declaration)
                .await?
        };
        locations.extend(cross);
        dedup_locations(&mut locations);
        Ok(locations)
    }

    fn reference_range_locations(uri: &Uri, ranges: Vec<CoreLspRange>) -> Vec<Location> {
        ranges
            .into_iter()
            .map(|r| Location {
                uri: uri.clone(),
                range: lift_lsp_range(r),
            })
            .collect()
    }

    /// Every reference [`Location`] for the symbol at `position`: the
    /// single-document hits, the cross-document call sites (declarations too
    /// when `include_declaration`), and the `TclOO` method / classmethod
    /// cross-file layer — override family, inheritors, and pure consumers.
    ///
    /// The **one** path behind both `textDocument/references` and the code
    /// lens's click target.  They must not re-derive it separately: the lens
    /// listing fewer sites than the peek on the same symbol is a defect.
    pub(in super::super) async fn ordinary_references(
        &self,
        uri: &Uri,
        pos: Position,
        include_decl: bool,
    ) -> jsonrpc::Result<Option<Vec<Location>>> {
        let uri = uri.clone();
        let source_epoch = self.diag_inputs_epoch();
        let source_generation = self.live_publication_uri_generation(&uri);
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        // BIG-IP `.conf`/`.scf`: a reference is every identifier-bounded
        // occurrence of the TMSH path token at the cursor (tcl-bigip::refs),
        // a textual search the Tcl symbol analyser can't perform on a
        // non-Tcl config. Same-document only — cross-file references await a
        // workspace config index.
        if Self::is_bigip_dialect(&doc.dialect) {
            let text = doc.text.clone();
            let bigip_ranges = crate::rt::spawn_blocking(move || {
                tcl_bigip::refs::references_at(&text, pos.line, pos.character, include_decl)
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("bigip references worker panicked: {err}").into(),
                data: None,
            })?;
            if bigip_ranges.is_empty() {
                return Ok(None);
            }
            let locations = bigip_ranges
                .into_iter()
                .map(|r| Location {
                    uri: uri.clone(),
                    range: lift_bigip_range(r),
                })
                .collect();
            return Ok(Some(locations));
        }
        // The iRulesLX method relation, in both directions: from
        // a Tcl call site, and from the `addMethod` registration in the
        // extension's own JavaScript. Answered here, in full — the sites are
        // literal words in two languages, which the Tcl symbol analyser has no
        // way to relate.
        if let Some(locations) = self.ilx_references(&uri, &doc, pos, include_decl).await? {
            return Ok((!locations.is_empty()).then_some(locations));
        }
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if analysis.resolved_input.as_ref().is_some_and(
            tcl_compiler::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
        ) && Self::original_namespace_symbol(&doc.text, &analysis, pos).is_some()
        {
            let locations = Box::pin(self.logical_namespace_locations_for_snapshot(
                (uri.clone(), doc.clone(), source_generation),
                &analysis,
                pos,
                source_epoch,
                LogicalNamespaceLocationsPurpose::References {
                    include_declaration: include_decl,
                },
            ))
            .await?;
            return Ok((!locations.is_empty()).then_some(locations));
        }
        let logical_namespace_variable = analysis.resolved_input.as_ref().is_some_and(|input| input.has_logical_source_name_context()
            && matches!(tcl_lsp_core::variable_symbol::select(&doc.text, &analysis, pos.line, pos.character), std::ops::ControlFlow::Break(Some(occurrence)) if occurrence.symbol().is_namespace() && occurrence.matches_logical_source_view(&tcl_lexer::SourceImage::document(&doc.text), input)));
        let locations = if logical_namespace_variable {
            self.logical_variable_locations_for_snapshot(
                (uri.clone(), doc.clone(), source_generation),
                &analysis,
                pos,
                source_epoch,
                LogicalVariableLocationsPurpose::References {
                    include_declaration: include_decl,
                },
            )
            .await?
        } else {
            Box::pin(self.reference_locations(&uri, &doc, &analysis, pos, include_decl)).await?
        };
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(locations))
    }

    pub(in super::super) async fn reference_locations(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        position: Position,
        include_declaration: bool,
    ) -> DocumentSourceResult<Vec<Location>> {
        // A namespace-name argument is answered here, in full.  It must not
        // reach the tiers below: the position is definitive, and an *empty*
        // local set — which is what asking for references without
        // declarations from a namespace's only declaring block produces —
        // would otherwise route the query to `workspace_resolved_references`,
        // the proc/class tier.
        if let Some(symbol) = Self::original_namespace_symbol(&doc.text, analysis, position) {
            return self
                .original_namespace_locations(
                    uri,
                    &doc.text,
                    analysis,
                    &symbol,
                    include_declaration,
                    false,
                )
                .await;
        }
        if Self::original_namespace_cursor_retained(&doc.text, analysis, position) {
            return Ok(Vec::new());
        }
        if let Some(cell) = Self::namespace_cell(&doc.text, analysis, position) {
            return self
                .namespace_reference_locations(uri, analysis, &cell, include_declaration)
                .await;
        }
        if let std::ops::ControlFlow::Break(locations) =
            Box::pin(self.readonly_original_method_reference_locations_at(
                uri,
                &doc.text,
                analysis,
                position,
                include_declaration,
            ))
            .await?
        {
            return Ok(locations);
        }
        if !analysis.allows_lexical_declaration_advice() {
            return self
                .retained_reference_locations(uri, doc, analysis, position, include_declaration)
                .await;
        }
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let analysis_for_worker = analysis.clone();
        // Which definition a bare call is a reference *to* can turn on a
        // `namespace export` in another file, so the
        // single-document pass gets the same whole-program export view
        // go-to-definition uses — or the two contradict each other on one
        // cursor.
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let ranges = crate::rt::spawn_blocking(move || {
            core_references::references_in_program(
                &text,
                tcl_lsp_core::profile_for_dialect(&dialect),
                position.line,
                position.character,
                &analysis_for_worker,
                include_declaration,
                Some(core_definition::ProgramExports {
                    uri: &uri_key,
                    oracle: exports.as_ref(),
                }),
            )
        })
        .await
        .unwrap_or_default();
        // A pure-consumer document has no local
        // declaration to anchor the single-document pass on, so an empty
        // `ranges` here does not mean "genuinely zero" — see
        // `workspace_resolved_references`'s own doc.
        let local_found_nothing = ranges.is_empty();
        let mut locations = Self::reference_range_locations(uri, ranges);
        let cross = if local_found_nothing {
            self.workspace_resolved_references(
                uri,
                &doc.text,
                analysis,
                position,
                include_declaration,
            )
            .await?
        } else {
            self.cross_document_references(uri, &doc.text, analysis, position, include_declaration)
                .await?
        };
        locations.extend(cross);
        // Cross-document namespace-variable sites: `$::NS::var` consumers in
        // sibling documents, and the declaration(s) of the cell when asked
        // for. The single-document provider above only sees this file, and a
        // consumer document holds no `VarDef` for a cell it merely reads.
        locations.extend(
            self.cross_document_variable_references(
                &doc.text,
                analysis,
                position,
                include_declaration,
            )
            .await?,
        );
        // Cross-document `TclOO` method sites: when the cursor names a method
        // (its declaration inside a class body, or an `$obj method` / `my
        // method` call), gather the method's sites across its override family
        // in sibling documents, plus pure-consumer documents the family pass
        // can't see — the single-document provider above only sees this file.
        locations.extend(
            self.cross_file_method_and_consumer_references(
                uri,
                doc,
                analysis,
                position,
                include_declaration,
            )
            .await?,
        );
        dedup_locations(&mut locations);
        Ok(locations)
    }

    /// Workspace references for the namespace variable at `pos`: every
    /// qualified occurrence of the cell anywhere in the index, plus its
    /// declaration sites when `include_declaration`.
    ///
    /// Unlike the proc / class tier this excludes **no** document, including
    /// the caller's own.  Two reasons, both structural:
    ///
    /// * A pure consumer (`$::ns::v` with the declaring `namespace eval` in a
    ///   sibling file) holds no `VarDef` for the cell at all, so the
    ///   single-document pass reported nothing — the same case
    ///   [`Self::workspace_resolved_references`] exists for on the proc side.
    /// * A *relative*-qualified occurrence (`$app::colors::palette`) is not
    ///   attached to the declaring `VarDef` by the single-document walk
    ///   either, even when both are in one file.
    ///
    /// The caller dedupes ([`dedup_locations`]), and the index records the
    /// exact same spans the single-document pass does, so an occurrence
    /// already reported collapses rather than doubling.  A declaration site
    /// that is *itself* written qualified (`set ::ns::v 1`) is a declaration,
    /// not a reference, so it is filtered out of the occurrence set and only
    /// reappears under `include_declaration`.
    pub(in super::super) async fn cross_document_variable_references(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        include_declaration: bool,
    ) -> DocumentSourceResult<Vec<Location>> {
        if let std::ops::ControlFlow::Break(Some(occurrence)) =
            tcl_lsp_core::variable_symbol::select(source, analysis, pos.line, pos.character)
        {
            let rehoming_guard = self.rehomed_index_guard().await?;
            let targets = {
                let index = self.workspace_index.read().await;
                index
                    .original_variable_occurrences(occurrence.symbol(), "")
                    .filter(|(_, occurrence)| include_declaration || !occurrence.is_declaration())
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
        let rehoming_guard = self.rehomed_index_guard().await?;
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.read().await;
            let declarations: Vec<(String, tcl_lexer::Span)> = index
                .variable_definitions_qualified(&cell, "")
                .into_iter()
                .map(|v| (v.uri.clone(), v.name_span))
                .collect();
            let mut t: Vec<(String, tcl_lexer::Span)> = index
                .variable_refs_of(&cell, "")
                .into_iter()
                .map(|v| (v.uri.clone(), v.span))
                .filter(|site| !declarations.contains(site))
                .collect();
            if include_declaration {
                t.extend(declarations);
            }
            t.sort_by_key(|(u, s)| (u.clone(), s.start(), s.end()));
            t.dedup();
            t
        };
        drop(rehoming_guard);
        self.resolve_target_locations(targets).await
    }
}
