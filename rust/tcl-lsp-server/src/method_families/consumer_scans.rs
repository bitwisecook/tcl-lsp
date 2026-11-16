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

//! Call sites in documents that only consume a class.

use super::super::Backend;
use super::super::documents::DocumentSourceResult;
use super::{ConsumerScan, ConsumerTarget};
use std::str::FromStr;
use std::sync::Arc;
use tcl_lsp_core::references as core_references;
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};

/// How wide a document set a pure-consumer method scan covers.
///
/// The narrow set is "documents that invoke a family constructor", which is
/// what the index can answer cheaply — and it is a real ceiling: a consumer
/// that *receives* an instance without constructing one (through a global, a
/// dict entry, a callback registered elsewhere, a proc parameter fed from
/// another file) invokes no constructor, so it appears in no index table and
/// is invisible to both the scan and the gate.
///
/// A **rename** cannot live with that ceiling: an unseen `$who speak` is a
/// call left naming a member that no longer exists, and a gate that never
/// scans the document cannot refuse either.  So rename widens to every
/// indexed document — the gate then sees the untracked receiver and refuses,
/// or the collector types it through the workspace class oracle and rewrites
/// it.  Find All References keeps the narrow set: it is an interactive query
/// whose worst outcome is a missing result, and widening it would re-analyse
/// the whole workspace on every invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum ConsumerCoverage {
    /// Documents invoking a family constructor, plus the request's own.
    ConstructorSites,
    /// Every indexed document.
    WholeWorkspace,
}

/// One document a consumer scan visits, with the text and index needed to
/// lift its spans to LSP ranges.
struct ConsumerDoc {
    pub(in super::super) uri: Uri,
    /// Shares the document snapshot's text handle rather than copying the
    /// buffer per candidate document.
    pub(in super::super) text: Arc<str>,
    pub(in super::super) dialect: String,
    pub(in super::super) line_index: tcl_lexer::LineIndex,
}

impl Backend {
    /// Cross-file references from **pure-consumer** documents: `$obj method`
    /// sites where `$obj` is an instance of a class in `(seed_class, method)`'s
    /// override family / inheritor set, in a document that only *uses* the class
    /// (defines no part of it) and so is invisible to
    /// [`Self::cross_file_method_references`].
    ///
    /// Bounds the scan to documents that construct a family instance (via the
    /// index's invocation records) plus the current document — whose consumer
    /// sites the single-document provider also missed — and re-analyses each
    /// with the workspace class oracle so `instance_classes` resolves the
    /// cross-file constructor.  Declarations are never here (a consumer declares
    /// none), so this is independent of `include_declaration`.
    pub(in super::super) async fn cross_file_consumer_method_references(
        &self,
        current_uri: &Uri,
        current_source: &str,
        current_dialect: &'static tcl_dialect::DialectProfile,
        seed_class: &str,
        method: &str,
        is_classmethod: bool,
    ) -> DocumentSourceResult<Vec<Location>> {
        Ok(self
            .consumer_method_site_ranges(
                current_uri,
                current_source,
                current_dialect,
                ConsumerTarget {
                    seed_class,
                    method,
                    is_classmethod,
                    coverage: ConsumerCoverage::ConstructorSites,
                },
            )
            .await?
            .into_iter()
            .flat_map(|(uri, ranges)| {
                ranges.into_iter().map(move |range| Location {
                    uri: uri.clone(),
                    range,
                })
            })
            .collect())
    }

    /// The pure-consumer `$obj method` / bare classmethod-dispatch sites of
    /// `(seed_class, method)`, grouped per document as LSP ranges.
    ///
    /// The one resolver behind **both** the consumer leg of Find All
    /// References ([`Self::cross_file_consumer_method_references`]) and the
    /// consumer leg of rename ([`Self::cross_file_method_rename`]).  They
    /// must agree site-for-site: a site references sees but rename misses is
    /// a call left bound to a name that no longer exists, so the two never
    /// re-derive the set independently.
    pub(in super::super) async fn consumer_method_site_ranges(
        &self,
        current_uri: &Uri,
        current_source: &str,
        current_dialect: &'static tcl_dialect::DialectProfile,
        target: ConsumerTarget<'_>,
    ) -> DocumentSourceResult<Vec<(Uri, Vec<Range>)>> {
        let (method, is_classmethod) = (target.method, target.is_classmethod);
        let Some(plan) = self.consumer_scan_plan(current_dialect, target).await else {
            return Ok(Vec::new());
        };
        let mut out: Vec<(Uri, Vec<Range>)> = Vec::new();
        let mut scanned: std::collections::HashSet<String> = std::collections::HashSet::new();
        for u in plan
            .consumer_uris
            .iter()
            .cloned()
            .chain(std::iter::once(current_uri.as_str().to_owned()))
        {
            if !scanned.insert(u.clone()) {
                continue;
            }
            let doc = if u == current_uri.as_str() {
                ConsumerDoc {
                    uri: current_uri.clone(),
                    text: Arc::from(current_source),
                    dialect: current_dialect.name.to_owned(),
                    // The LSP EOL model, matching the `line_index` the sibling
                    // branch takes straight off the stored `DocumentState`.
                    line_index: tcl_lexer::LineIndex::new_lsp(current_source),
                }
            } else {
                let Ok(parsed) = Uri::from_str(&u) else {
                    continue;
                };
                let Some(state) = self.read_document(&parsed).await? else {
                    continue;
                };
                ConsumerDoc {
                    uri: parsed,
                    text: state.text.clone(),
                    dialect: state.dialect.clone(),
                    line_index: state.line_index.clone(),
                }
            };
            let ranges = self
                .consumer_ranges_in_document(&doc, &plan, method, is_classmethod)
                .await?;
            if !ranges.is_empty() {
                out.push((doc.uri, ranges));
            }
        }
        Ok(out)
    }

    /// The family classes, candidate consumer documents, and classmethod
    /// dispatch heads for `(seed_class, method)` — one index read.  `None`
    /// when the family is empty (the class isn't indexed yet), which means
    /// there is nothing for a consumer scan to match against.
    ///
    /// `classmethod_cmd_names` carries the workspace-wide answer to "given
    /// `is_classmethod` is true, which class names bare-dispatch it" — a
    /// pure-consumer document (e.g. one that only calls `Factory make`) has
    /// no local `ClassDef` for `Factory`, so
    /// `core_references::obj_method_call_sites` cannot derive this itself the
    /// way the same-document path does.
    pub(super) async fn consumer_scan_plan(
        &self,
        dialect: &'static tcl_dialect::DialectProfile,
        target: ConsumerTarget<'_>,
    ) -> Option<ConsumerScan> {
        let ConsumerTarget {
            seed_class,
            method,
            is_classmethod,
            coverage,
        } = target;
        let index = self.workspace_index.read().await;
        let definers = index.method_override_family(seed_class, method);
        let inheritors = index.method_inheritor_classes(seed_class, method);
        let classmethod_cmd_names = Self::classmethod_dispatch_names(
            &definers,
            &inheritors,
            method,
            is_classmethod,
            dialect,
        );
        let mut family: Vec<String> = definers
            .iter()
            .map(|wc| wc.qualified_name.clone())
            .collect();
        family.extend(inheritors.iter().map(|wc| wc.qualified_name.clone()));
        if family.is_empty() {
            return None;
        }
        family.sort();
        family.dedup();
        let mut family_uris: Vec<String> = definers
            .iter()
            .chain(inheritors.iter())
            .map(|wc| wc.uri.clone())
            .collect();
        family_uris.sort();
        family_uris.dedup();
        let family_norm: std::collections::HashSet<&str> = family
            .iter()
            .map(|s| tcl_compiler::naming::unroot_rooted_key(s).unwrap_or(s))
            .collect();
        // The index answers with a `HashSet`; sort so the per-document scan
        // order (and so the emitted edit / location order) does not ride on
        // hash iteration order.
        let mut consumer_uris: Vec<String> = match coverage {
            ConsumerCoverage::ConstructorSites => index
                .documents_invoking_classes(&family_norm)
                .into_iter()
                .collect(),
            // A document that receives an instance without constructing one
            // invokes no family constructor, so it is in no index table at
            // all — the only set that provably contains it is every document.
            ConsumerCoverage::WholeWorkspace => index.document_uris(),
        };
        consumer_uris.sort();
        consumer_uris.dedup();
        drop(index);
        Some(ConsumerScan {
            family,
            family_uris,
            consumer_uris,
            classmethod_cmd_names,
        })
    }

    /// The consumer call-site ranges of `method` within one document, scanned
    /// against an analysis carrying the workspace class oracle so a
    /// constructor whose class lives in another file still types `$obj`.
    async fn consumer_ranges_in_document(
        &self,
        doc: &ConsumerDoc,
        plan: &ConsumerScan,
        method: &str,
        is_classmethod: bool,
    ) -> DocumentSourceResult<Vec<Range>> {
        let analysis = self
            .analyse_with_workspace_classes(
                &doc.uri,
                &doc.text,
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
            )
            .await?;
        let source = doc.text.clone();
        let dialect = doc.dialect.clone();
        let family = plan.family.clone();
        let method_owned = method.to_owned();
        let classmethod_cmd_names = plan.classmethod_cmd_names.clone();
        let spans: Vec<tcl_lexer::Span> = crate::rt::spawn_blocking(move || {
            // Resolved once for the whole closure rather than per class in the
            // family — the profile is a pure function of `dialect`.
            let profile = tcl_lsp_core::profile_for_dialect(&dialect);
            let mut all: Vec<tcl_lexer::Span> = Vec::new();
            for cq in &family {
                all.extend(core_references::obj_method_call_sites(
                    &source,
                    profile,
                    &analysis,
                    cq,
                    &method_owned,
                    is_classmethod,
                    &classmethod_cmd_names,
                ));
            }
            all
        })
        .await
        .unwrap_or_default();
        let mut ranges: Vec<Range> = spans
            .into_iter()
            .map(|span| {
                let start = doc.line_index.position_at_utf16(span.start(), &doc.text);
                let end = doc.line_index.position_at_utf16(span.end(), &doc.text);
                Range {
                    start: Position {
                        line: start.line,
                        character: start.character.get(),
                    },
                    end: Position {
                        line: end.line,
                        character: end.character.get(),
                    },
                }
            })
            .collect();
        // Stable order regardless of which family class matched a site first,
        // so a rename's edit list and the reference peek's location list are
        // reproducible across runs.
        ranges.sort_by_key(|r| (r.start.line, r.start.character, r.end.line, r.end.character));
        ranges.dedup();
        Ok(ranges)
    }
}
