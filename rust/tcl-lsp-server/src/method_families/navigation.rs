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

//! References, definition and hover of a method across its override family.

use super::super::documents::DocumentSourceResult;
use super::super::{Backend, DocumentState};
use super::family::{DefinerSpans, family_spans_in_document};
use super::{FamilyClasses, FamilySpanScan};
use std::str::FromStr;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::hover::Hover as CoreHover;
use tcl_lsp_core::{hover as core_hover, workspace_index as core_workspace_index};
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};

impl Backend {
    /// Cross-document references for a `TclOO` method across its override
    /// family — the reference analogue of [`Self::cross_file_method_rename`].
    ///
    /// Resolves the workspace-wide override family of `(seed_class, method)`,
    /// then for every *sibling* document that defines a family class collects
    /// the method's `$obj method` / `my method` call sites (and, when
    /// `include_declaration`, its declaration), plus the inherited call sites in
    /// documents holding a purely-inheriting subclass.  The current document is
    /// excluded — [`Self::references`] already gathers its method sites from the
    /// single-document provider.  Coverage is bounded the same way rename is: a
    /// site resolves only in a document the index knows defines or inherits the
    /// family class.
    pub(in super::super) async fn cross_file_method_references(
        &self,
        current_uri: &Uri,
        seed_class: &str,
        method: &str,
        include_declaration: bool,
        is_classmethod: bool,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> DocumentSourceResult<Vec<Location>> {
        let family = self
            .method_family_by_document(dialect, seed_class, method, is_classmethod)
            .await;
        let external_callback_receivers = family.external_callback_receivers;
        let classmethod_cmd_names = family.classmethod_cmd_names;
        let mut out = Vec::new();
        for (
            u,
            FamilyClasses {
                definers,
                inheritors,
            },
        ) in family.by_uri
        {
            if u == current_uri.as_str() {
                continue;
            }
            let Ok(parsed) = Uri::from_str(&u) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await? else {
                continue;
            };
            let analysis = self
                .analysis_for(&parsed, target_doc.text.clone(), target_doc.dialect.clone())
                .await?;
            let src = target_doc.text.clone();
            let dialect = target_doc.dialect.clone();
            let method_owned = method.to_owned();
            let classmethod_cmd_names_cl = classmethod_cmd_names.clone();
            let external_receivers_cl = external_callback_receivers.clone();
            let spans: Vec<tcl_lexer::Span> = crate::rt::spawn_blocking(move || {
                family_spans_in_document(
                    FamilySpanScan {
                        source: &src,
                        // Resolved once: the profile is a pure function of
                        // `dialect`, so re-resolving it per class only repeats
                        // the lookup.
                        profile: tcl_lsp_core::profile_for_dialect(&dialect),
                        analysis: &analysis,
                        method: &method_owned,
                        is_classmethod,
                        classmethod_cmd_names: &classmethod_cmd_names_cl,
                        external_receivers: &external_receivers_cl,
                    },
                    &definers,
                    &inheritors,
                    DefinerSpans::References {
                        include_declaration,
                    },
                )
            })
            .await
            .unwrap_or_default();
            let line_index = target_doc.line_index.clone();
            for span in spans {
                let start = line_index.position_at_utf16(span.start(), &target_doc.text);
                let end = line_index.position_at_utf16(span.end(), &target_doc.text);
                out.push(Location {
                    uri: parsed.clone(),
                    range: Range {
                        start: Position {
                            line: start.line,
                            character: start.character.get(),
                        },
                        end: Position {
                            line: end.line,
                            character: end.character.get(),
                        },
                    },
                });
            }
        }
        Ok(out)
    }

    /// The method-dispatch half of [`Self::references`]: resolves the
    /// cursor to a `(class, method)` target (oracle-aware, so a
    /// pure-consumer cursor whose `$obj`'s class lives in another file still
    /// identifies it), then gathers its sites across the override family in
    /// sibling documents plus pure-consumer documents the family pass can't
    /// see.  Empty when the cursor names no method.
    pub(in super::super) async fn cross_file_method_and_consumer_references(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
        include_decl: bool,
    ) -> DocumentSourceResult<Vec<Location>> {
        let mut locations = Vec::new();
        if let Some((seed_class, method, is_classmethod, _access)) = self
            .resolve_method_target(
                uri,
                &doc.text,
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
                analysis,
                pos,
            )
            .await?
        {
            locations.extend(
                self.cross_file_method_references(
                    uri,
                    &seed_class,
                    &method,
                    include_decl,
                    is_classmethod,
                    tcl_lsp_core::profile_for_dialect(&doc.dialect),
                )
                .await?,
            );
            // Pure-consumer documents (including this one) whose `$obj method`
            // sites neither the single-document provider nor the family pass
            // sees without the workspace class oracle.
            locations.extend(
                self.cross_file_consumer_method_references(
                    uri,
                    &doc.text,
                    tcl_lsp_core::profile_for_dialect(&doc.dialect),
                    &seed_class,
                    &method,
                    is_classmethod,
                )
                .await?,
            );
        }
        Ok(locations)
    }

    /// Cross-file go-to-definition for a `TclOO` method: the declaration
    /// site of the **dispatch entry** — the first implementation on the
    /// receiver class's C-faithful linearisation that is callable under
    /// `access` (a definition request identifies the
    /// implementation the call actually enters, never the whole override
    /// family; an externally-uncallable method resolves to nothing, mirroring
    /// C's `unknown method`).
    ///
    /// The current document participates: a mixin override in this file
    /// outranks the receiver class's own method in another.  (Same-file
    /// simple cases are answered by the in-document provider before this
    /// runs, with the same chain rule.)
    ///
    /// `is_classmethod` — the caller's already-resolved fact about which of a
    /// possibly-same-named `method` / `classmethod` pair the cursor means —
    /// picks the **side** the chain is walked on.  A `ClassName member`
    /// dispatch goes through the class object's own tables, so it must honour
    /// the class-side visibility channel: without it a `self unexport m` in one
    /// file would leave this resolving a `::C m` the interpreter rejects with
    /// `unknown method "m"`.
    pub(in super::super) async fn cross_file_method_definition(
        &self,
        _current_uri: &Uri,
        class_q: &str,
        method: &str,
        access: core_workspace_index::MethodAccess,
        is_classmethod: bool,
    ) -> DocumentSourceResult<Vec<Location>> {
        let chain: Vec<(String, String)> = {
            let index = self.workspace_index.read().await;
            if is_classmethod {
                index.class_method_dispatch_chain(class_q, method, access)
            } else {
                index.method_dispatch_chain(class_q, method, access)
            }
            .into_iter()
            .map(|wc| (wc.uri.clone(), wc.qualified_name.clone()))
            .collect()
        };
        // The chain's first record is the dispatch entry; later records are
        // the `next` chain, reachable through the call-hierarchy /
        // references views rather than definition.
        for (u, cq) in chain {
            let Ok(parsed) = Uri::from_str(&u) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await? else {
                continue;
            };
            let analysis = self
                .analysis_for(&parsed, target_doc.text.clone(), target_doc.dialect.clone())
                .await?;
            // The chain already picked the side; look the declaration up on
            // that side first so a class that legally defines *both* a `method
            // m` and a `self method m` renders the one the cursor's dispatch
            // actually enters (the other table stays as a fallback for a record
            // whose kinds the index and the re-analysis disagree on).
            if let Some(class_def) = analysis.all_classes.get(&cq)
                && let Some(m) = {
                    let (a, b) = if is_classmethod {
                        (&class_def.class_methods, &class_def.methods)
                    } else {
                        (&class_def.methods, &class_def.class_methods)
                    };
                    a.get(method).or_else(|| b.get(method))
                }
            {
                return self.resolve_target_locations(vec![(u, m.name_span)]).await;
            }
        }
        Ok(Vec::new())
    }

    /// Cross-file **hover** for a `TclOO` method dispatch: the same dispatch
    /// entry [`Self::cross_file_method_definition`] jumps to, rendered as the
    /// one-line summary the in-document hover renders.
    ///
    /// The in-document provider (`core_hover`'s `method_dispatch_hover`) is
    /// MRO-aware, but only over `analysis.all_classes` — the classes *this*
    /// document declares.  The common shape is cross-file: `my ArgsPreprocess`
    /// in a device file, the `mixin` that provides it in the library file.
    /// Definition and references cross that boundary through the workspace
    /// index, and this is what lets hover do the same.
    ///
    /// Deliberately built on the *same* resolution and the *same* chain walk
    /// as the definition tier — [`Self::resolve_method_target`] then
    /// `method_dispatch_chain` / `class_method_dispatch_chain` — rather than
    /// a parallel lookup, so hover can never name a provider the definition
    /// jump would not land on.
    pub(in super::super) async fn cross_file_method_hover(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<Option<CoreHover>> {
        let Some(input) = analysis.resolved_input.as_ref() else {
            return Ok(None);
        };
        if input.has_logical_source_name_context() {
            return Box::pin(self.logical_member_source_hover(uri, doc, analysis, pos)).await;
        }
        // Hosted semantics remain in their separately deferred consumer. Generic
        // source advice cannot seed this report-selected dispatch implementation.
        if !tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
            &input.borrowed_context_registry().context().environment.id,
        )
        .is_some_and(
            tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::is_appliance_hosted,
        ) {
            return Ok(None);
        }
        let Some((class_q, method, is_classmethod, access)) = self
            .resolve_method_target(
                uri,
                &doc.text,
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
                analysis,
                pos,
            )
            .await?
        else {
            return Ok(None);
        };
        let chain: Vec<(String, String)> = {
            let index = self.workspace_index.read().await;
            if is_classmethod {
                index.class_method_dispatch_chain(&class_q, &method, access)
            } else {
                index.method_dispatch_chain(&class_q, &method, access)
            }
            .into_iter()
            .map(|wc| (wc.uri.clone(), wc.qualified_name.clone()))
            .collect()
        };
        // The chain's first record is the dispatch entry; the rest is the
        // `next` chain, which hover does not describe.
        for (u, provider_q) in chain {
            let Ok(parsed) = Uri::from_str(&u) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await? else {
                continue;
            };
            let target_analysis = self
                .analysis_for(&parsed, target_doc.text.clone(), target_doc.dialect.clone())
                .await?;
            let Some(class_def) = target_analysis.all_classes.get(&provider_q) else {
                continue;
            };
            let (a, b) = if is_classmethod {
                (&class_def.class_methods, &class_def.methods)
            } else {
                (&class_def.methods, &class_def.class_methods)
            };
            if let Some(m) = a.get(&method).or_else(|| b.get(&method)) {
                return Ok(Some(core_hover::cross_document_method_hover(
                    &class_q,
                    &provider_q,
                    &m.name,
                    m.params.len(),
                    is_classmethod,
                )));
            }
        }
        Ok(None)
    }
}
