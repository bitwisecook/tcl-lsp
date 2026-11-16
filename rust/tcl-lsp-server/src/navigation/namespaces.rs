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

//! Namespace symbols: declarations, hover and references across documents.

use super::{
    AnalysisResult, Backend, CoreHover, CoreHoverKind, DocumentSourceResult, FromStr, HashSet,
    Location, OriginalIndexedSourceLocation, Position, Uri, core_definition, core_hover,
    core_namespace_symbol, core_workspace_index, dedup_span_targets, line_col_to_byte_offset,
};

impl Backend {
    /// The `::`-rooted namespace-variable cell the cursor names, or `None`
    /// when it names none — see
    /// [`core_definition::qualified_variable_cell_at`] for the two cursor
    /// shapes that answer.  The one place the server decides "this position is
    /// about a namespace variable", shared by the cross-document definition,
    /// hover, and references tiers so they cannot disagree.
    pub(super) fn qualified_variable_cell(
        source: &str,
        dialect: &'static tcl_dialect::DialectProfile,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> Option<String> {
        if matches!(
            tcl_lsp_core::variable_symbol::select(source, analysis, pos.line, pos.character),
            std::ops::ControlFlow::Break(_)
        ) {
            return None;
        }
        core_definition::qualified_variable_cell_at(
            source,
            dialect,
            analysis,
            pos.line,
            pos.character,
        )
    }

    /// The `::`-rooted namespace the cursor names, or `None` when it names
    /// none — see [`core_namespace_symbol::namespace_cell_at`] for the two
    /// cursor shapes that answer.  The one place the server decides "this
    /// position is about a namespace", shared by the cross-document
    /// definition, hover, and references tiers so they cannot disagree.
    pub(in super::super) fn namespace_cell(
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> Option<String> {
        if Self::original_namespace_cursor_retained(source, analysis, pos) {
            return None;
        }
        core_namespace_symbol::namespace_cell_at(source, analysis, pos.line, pos.character)
    }

    pub(in super::super) fn original_namespace_symbol(
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> Option<core_namespace_symbol::OriginalNamespaceSymbol> {
        let offset =
            u32::try_from(line_col_to_byte_offset(source, pos.line, pos.character)?).ok()?;
        match core_namespace_symbol::select_at_offset(source, analysis, offset) {
            std::ops::ControlFlow::Break(symbol) => symbol,
            std::ops::ControlFlow::Continue(()) => None,
        }
    }

    pub(in super::super) fn original_namespace_cursor_retained(
        source: &str,
        analysis: &AnalysisResult,
        position: Position,
    ) -> bool {
        let Some(offset) = line_col_to_byte_offset(source, position.line, position.character)
            .and_then(|offset| u32::try_from(offset).ok())
        else {
            return false;
        };
        matches!(
            core_namespace_symbol::select_at_offset(source, analysis, offset),
            std::ops::ControlFlow::Break(_)
        )
    }

    pub(in super::super) async fn original_namespace_locations(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        symbol: &core_namespace_symbol::OriginalNamespaceSymbol,
        include_declaration: bool,
        declarations_only: bool,
    ) -> DocumentSourceResult<Vec<Location>> {
        let Some(context) =
            core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(analysis)
        else {
            return Ok(Vec::new());
        };
        if context.image() != &tcl_lexer::SourceImage::document(source) {
            return Ok(Vec::new());
        }
        let matches = |row: &tcl_compiler::analyser::types::NamespaceRef| {
            row.source_namespace.as_ref() == Some(symbol.scope())
                && row.name_policy == Some(symbol.policy())
                && (!declarations_only || row.declares)
                && (include_declaration || !row.declares)
        };
        let mut targets = analysis
            .namespace_refs
            .iter()
            .filter(|row| matches(row))
            .filter_map(|row| {
                OriginalIndexedSourceLocation::from_namespace(uri.as_str(), &context, row)
            })
            .collect::<Vec<_>>();
        let mut parents = analysis
            .namespace_refs
            .iter()
            .filter_map(|row| {
                OriginalIndexedSourceLocation::from_implicit_namespace_parent(
                    uri.as_str(),
                    &context,
                    row,
                    symbol,
                )
            })
            .collect::<Vec<_>>();
        {
            let _rehoming_guard = self.rehomed_index_guard().await?;
            let index = self.workspace_index.read().await;
            targets.extend(
                index
                    .original_namespace_occurrences(symbol, uri.as_str())
                    .filter(|row| matches(&row.source))
                    .filter_map(|row| {
                        OriginalIndexedSourceLocation::from_namespace(
                            &row.uri,
                            index.diagnostic_source_context(&row.uri)?,
                            &row.source,
                        )
                    }),
            );
            if declarations_only {
                parents.extend(
                    index
                        .original_namespace_descendant_declarations(symbol, uri.as_str())
                        .filter_map(|row| {
                            OriginalIndexedSourceLocation::from_implicit_namespace_parent(
                                &row.uri,
                                index.diagnostic_source_context(&row.uri)?,
                                &row.source,
                                symbol,
                            )
                        }),
                );
            }
        }
        let locations = self.resolve_original_indexed_locations(targets).await?;
        Ok(if declarations_only && locations.is_empty() {
            self.resolve_original_indexed_locations(parents).await?
        } else {
            locations
        })
    }

    pub(in super::super) async fn original_namespace_hover(
        &self,
        uri: &Uri,
        analysis: &AnalysisResult,
        symbol: &core_namespace_symbol::OriginalNamespaceSymbol,
    ) -> DocumentSourceResult<Option<CoreHover>> {
        let Some(context) =
            core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(analysis)
        else {
            return Ok(None);
        };
        let capture = |owner: &str,
                       context: &core_workspace_index::WorkspaceDiagnosticSourceContext,
                       row: &tcl_compiler::analyser::types::NamespaceRef| {
            if row.source_namespace.as_ref() == Some(symbol.scope())
                && row.name_policy == Some(symbol.policy())
            {
                OriginalIndexedSourceLocation::from_namespace(owner, context, row)
            } else {
                OriginalIndexedSourceLocation::from_implicit_namespace_parent(
                    owner, context, row, symbol,
                )
            }
        };
        let mut targets = analysis
            .namespace_refs
            .iter()
            .filter_map(|row| capture(uri.as_str(), &context, row))
            .collect::<Vec<_>>();
        {
            let _rehoming_guard = self.rehomed_index_guard().await?;
            let index = self.workspace_index.read().await;
            targets.extend(
                index
                    .original_namespace_occurrences(symbol, uri.as_str())
                    .chain(index.original_namespace_descendant_declarations(symbol, uri.as_str()))
                    .filter_map(|row| {
                        capture(
                            &row.uri,
                            index.diagnostic_source_context(&row.uri)?,
                            &row.source,
                        )
                    }),
            );
        }
        let mut facts = core_namespace_symbol::NamespaceFacts::default();
        let mut documents = HashSet::new();
        for (target, _) in self.validated_original_indexed_locations(targets).await? {
            if target.is_implicit_namespace_parent() {
                facts.implicit_declarations += 1;
            } else if target.is_declaration() {
                facts.declarations += 1;
            } else {
                facts.references += 1;
            }
            documents.insert(target.uri().to_owned());
        }
        facts.documents = documents.len();
        let Some(label) = symbol.source_word(
            tcl_lexer::SourceChannel::Document,
            match analysis.body_lexer_config {
                Some(__source_value) => __source_value,
                None => return Ok(None),
            },
        ) else {
            return Ok(None);
        };
        Ok(
            core_namespace_symbol::namespace_hover_markdown(&label, facts).map(|value| CoreHover {
                value,
                kind: CoreHoverKind::Markdown,
            }),
        )
    }

    /// Every declaring `namespace eval` block of `cell`, local **and**
    /// workspace, as LSP locations.
    ///
    /// The union is the contract, not an optimisation: reopening a namespace
    /// extends the same namespace (tclsh 9.0.4 / 8.6.16, byte-identical —
    /// `namespace eval ::a {}` twice leaves one namespace holding both
    /// blocks' variables), so every block is a definition site and a local
    /// one does not make a sibling document's block any less of one.  The
    /// answer must not stop as soon as the in-document provider replies, nor
    /// exclude the current URI from the index lookup: either would report only
    /// the local half for a namespace opened both here and next door.
    ///
    /// The local half is read from the request's own analysis rather than the
    /// index, so an unindexed or just-edited document still answers; rows are
    /// deduplicated by `(uri, span)`, which is what makes overlapping with
    /// the index harmless.
    ///
    /// When nothing anywhere declares the cell outright, the answer falls back
    /// to its **implicit** creators — the covering prefix of every deeper
    /// `namespace eval` name word, local and workspace alike.  Doing that only
    /// in the in-document provider would leave a namespace whose sole creating
    /// block lives in a sibling file answering nothing at all.
    pub(super) async fn namespace_declaration_locations(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        cell: &str,
    ) -> DocumentSourceResult<Vec<Location>> {
        let rehoming_guard = self.rehomed_index_guard().await?;
        let mut targets: Vec<(String, tcl_lexer::Span)> =
            core_namespace_symbol::namespace_declaration_spans(analysis, cell)
                .into_iter()
                .map(|span| (uri.as_str().to_owned(), span))
                .collect();
        // Rows whose name is a strict descendant of the cell, captured while
        // the lock is held so the (possibly disk-reading) text lookups below
        // happen outside it.
        let descendants: Vec<(String, tcl_compiler::analyser::types::NamespaceRef)> = {
            let index = self.workspace_index.read().await;
            targets.extend(
                index
                    .namespace_declarations_qualified(cell, "")
                    .into_iter()
                    .map(|n| (n.uri.clone(), n.span)),
            );
            index
                .namespace_declarations_under(cell, uri.as_str())
                .into_iter()
                .map(|n| (n.uri.clone(), n.source.clone()))
                .collect()
        };
        drop(rehoming_guard);
        if targets.is_empty() {
            // Nothing declares it outright anywhere.  The local implicit
            // sites first, in the same order the in-document provider gives
            // them, then the sibling documents'.
            targets.extend(
                core_namespace_symbol::namespace_implicit_parent_spans(source, analysis, cell)
                    .into_iter()
                    .map(|span| (uri.as_str().to_owned(), span)),
            );
            for (row_uri, reference) in descendants {
                let Ok(parsed) = Uri::from_str(&row_uri) else {
                    continue;
                };
                let Some(doc) = self.read_document(&parsed).await? else {
                    continue;
                };
                // The covering prefix is a sub-range of the written word, so
                // it can only be computed against that document's own text.
                if let Some(prefix) = core_namespace_symbol::retained_namespace_for_report(
                    analysis, cell,
                )
                .and_then(|(wanted, _)| {
                    core_namespace_symbol::namespace_implicit_parent_span_in(
                        &doc.text, &reference, &wanted,
                    )
                }) {
                    targets.push((row_uri, prefix));
                }
            }
        }
        dedup_span_targets(&mut targets);
        self.resolve_target_locations(targets).await
    }

    /// Hover for the namespace `cell`, counted over the local document **and**
    /// every indexed sibling.
    ///
    /// Counting only the document under the cursor would contradict
    /// go-to-definition, which offers every declaring block wherever it lives.
    /// The rendering itself stays in
    /// [`core_namespace_symbol::namespace_hover_markdown`], so the
    /// in-document provider and this tier cannot word the same fact
    /// differently — they differ only in how wide a set they counted, which
    /// the text states.
    ///
    /// `None` when nothing anywhere declares it — outright *or* implicitly:
    /// hover then shows nothing, which is the correct answer for a namespace
    /// no file in view creates — never a fall-through to command
    /// documentation.  A sibling file's deeper `namespace eval` counts as an
    /// implicit creator here exactly as it does in the local tally, so the
    /// "implicitly created by" wording is reachable cross-document; the count
    /// is pure name arithmetic, so unlike the
    /// definition tier it needs no document text.
    pub(super) async fn namespace_hover(
        &self,
        uri: &Uri,
        analysis: &AnalysisResult,
        cell: &str,
    ) -> DocumentSourceResult<Option<CoreHover>> {
        let rehoming_guard = self.rehomed_index_guard().await?;
        let local = core_namespace_symbol::namespace_facts(analysis, cell);
        let facts = {
            let index = self.workspace_index.read().await;
            let mut merged = local;
            let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for row in index
                .namespace_declarations_qualified(cell, uri.as_str())
                .into_iter()
                .chain(index.namespace_refs_of(cell, uri.as_str()))
            {
                if row.declares {
                    merged.declarations += 1;
                } else {
                    merged.references += 1;
                }
                if seen.insert(row.uri.as_str()) {
                    merged.documents += 1;
                }
            }
            // Deeper blocks in sibling documents create the cell as a parent.
            // Disjoint from the rows above by construction — that query is an
            // exact-name match, this one a strict-descendant match — so no
            // occurrence is counted twice.
            for row in index.namespace_declarations_under(cell, uri.as_str()) {
                merged.implicit_declarations += 1;
                if seen.insert(row.uri.as_str()) {
                    merged.documents += 1;
                }
            }
            merged
        };
        drop(rehoming_guard);
        Ok(core_hover::namespace_hover(cell, facts))
    }

    /// Every occurrence of the namespace `cell`, local **and** workspace —
    /// declarations included when `include_declaration`.
    ///
    /// Excludes no document, the caller's own included: a document that
    /// merely mentions `::tomato` holds no declaration to anchor a
    /// single-document pass on, and the local half is read from the request's
    /// own analysis so an unindexed document still contributes.  Rows are
    /// deduplicated by `(uri, span)`, so an occurrence the index also holds
    /// collapses rather than doubling.
    pub(super) async fn namespace_reference_locations(
        &self,
        uri: &Uri,
        analysis: &AnalysisResult,
        cell: &str,
        include_declaration: bool,
    ) -> DocumentSourceResult<Vec<Location>> {
        let rehoming_guard = self.rehomed_index_guard().await?;
        let mut targets: Vec<(String, tcl_lexer::Span)> =
            core_namespace_symbol::namespace_all_spans(analysis, cell, include_declaration)
                .into_iter()
                .map(|span| (uri.as_str().to_owned(), span))
                .collect();
        {
            let index = self.workspace_index.read().await;
            targets.extend(
                index
                    .namespace_refs_of(cell, "")
                    .into_iter()
                    .map(|n| (n.uri.clone(), n.span)),
            );
            if include_declaration {
                targets.extend(
                    index
                        .namespace_declarations_qualified(cell, "")
                        .into_iter()
                        .map(|n| (n.uri.clone(), n.span)),
                );
            }
        }
        drop(rehoming_guard);
        dedup_span_targets(&mut targets);
        self.resolve_target_locations(targets).await
    }
}
