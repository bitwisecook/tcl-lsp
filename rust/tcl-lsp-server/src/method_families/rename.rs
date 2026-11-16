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

//! Renaming a method across its override family, and the refusals that guard it.

use super::super::documents::DocumentSourceResult;
use super::super::rename::merge_span_edits;
use super::super::{Backend, DocumentState};
use super::consumer_scans::ConsumerCoverage;
use super::family::{DefinerSpans, family_spans_in_document};
use super::{ConsumerTarget, FamilyClasses, FamilySpanScan};
use std::str::FromStr;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::{rename as core_rename, rename_safety as core_rename_safety};
use tower_lsp_server::ls_types::{Position, TextEdit, Uri};

impl Backend {
    /// The cross-file **override-family** method rename, or `None` when the
    /// cursor names no method (or the family produced no edit).
    ///
    /// A method (re)defined up or down the hierarchy is one polymorphic name,
    /// and the override family can span files, so this handles the current
    /// document *and* every sibling that defines a family class uniformly —
    /// the single-document family is incomplete when the connecting ancestor
    /// lives in another file.  Falling through to the single-document path on
    /// an empty result keeps an unpopulated index from regressing anything.
    pub(in super::super) async fn method_family_rename_tier(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
    ) -> DocumentSourceResult<Option<std::collections::HashMap<Uri, Vec<TextEdit>>>> {
        if !analysis.allows_lexical_declaration_advice()
            || !core_rename::is_safe_symbol_name(new_name)
        {
            return Ok(None);
        }
        let Some((seed_class, method, is_classmethod)) = self
            .resolve_method_rename_target(uri, doc, analysis, pos)
            .await?
        else {
            return Ok(None);
        };
        let changes = self
            .cross_file_method_rename(uri, doc, &seed_class, &method, new_name, is_classmethod)
            .await?;
        Ok((!changes.is_empty()).then_some(changes))
    }

    /// The rename **safety refusal** for the `TclOO` member at `pos`, if any —
    /// checked across every document the rename would touch before a single
    /// edit is built.
    ///
    /// The gate itself is [`core_rename_safety::method_rename_hazard`]; this
    /// is the workspace fan-out.  A hazard in a *sibling* document is every
    /// bit as fatal as one in the request's own: the edit set spans all of
    /// them, so an unrewritable dispatch anywhere refuses the whole rename
    /// rather than half-applying it.
    ///
    /// # The gate's set is the edit collector's set
    ///
    /// Coverage comes from [`Self::consumer_scan_plan`] — the *same* index
    /// read [`Self::cross_file_method_rename`] resolves its own document set
    /// from — so the two can never diverge.  A gate narrower than the edit
    /// collector is a hollow guarantee: a pure-consumer document that
    /// dispatches the method on an untracked receiver is rewritten by the
    /// consumer leg but, if never scanned, contributes no refusal, and the
    /// declaration moves out from under a call that still names the old
    /// member.  The set is therefore
    /// the definers' and pure inheritors' documents, every consumer document,
    /// and the request's own.
    ///
    /// Each document is scanned against the **workspace-class oracle**
    /// analysis, which is exactly what the consumer edit leg resolves its
    /// call sites with — so a receiver the oracle types (and the edit
    /// collector therefore rewrites) is not mistaken for an untracked one.
    /// That analysis is memoised per document in
    /// [`Self::analyse_with_workspace_classes`], so scanning a document here
    /// and rewriting it there analyses it once between them, not twice.
    ///
    /// Falls back to the request's own document and the seed class when the
    /// family is not indexed at all (a freshly-opened buffer): the gate still
    /// applies to what the single-document rename would edit.
    pub(in super::super) async fn method_rename_refusal(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
    ) -> DocumentSourceResult<Option<core_rename_safety::RenameRefusal>> {
        let Some((seed_class, method, is_classmethod)) = self
            .resolve_method_rename_target(uri, doc, analysis, pos)
            .await?
        else {
            return Ok(None);
        };
        let plan = self
            .consumer_scan_plan(
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
                ConsumerTarget {
                    seed_class: &seed_class,
                    method: &method,
                    is_classmethod,
                    // The same coverage the rename's edit collector uses:
                    // every indexed document, so a consumer holding an
                    // instance it never constructed is scanned rather than
                    // invisible.
                    coverage: ConsumerCoverage::WholeWorkspace,
                },
            )
            .await;
        let (family, mut uris) = match plan {
            Some(plan) => {
                let mut uris = plan.family_uris;
                uris.extend(plan.consumer_uris);
                (plan.family, uris)
            }
            None => (vec![seed_class.clone()], Vec::new()),
        };
        uris.push(uri.as_str().to_owned());
        uris.sort();
        uris.dedup();
        for u in uris {
            let Ok(parsed) = Uri::from_str(&u) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await? else {
                continue;
            };
            let target_analysis = self
                .analyse_with_workspace_classes(
                    &parsed,
                    &target_doc.text,
                    tcl_lsp_core::profile_for_dialect(&target_doc.dialect),
                )
                .await?;
            if let Some(refusal) = core_rename_safety::method_rename_hazard(
                &target_doc.text,
                tcl_lsp_core::profile_for_dialect(&target_doc.dialect),
                &target_analysis,
                core_rename_safety::MethodRenameTarget {
                    family: &family,
                    method: &method,
                    is_classmethod,
                    new_name,
                },
                &target_doc.line_index,
            ) {
                return Ok(Some(refusal));
            }
        }
        Ok(None)
    }

    /// Cross-file rename of a `TclOO` method across its override family.
    ///
    /// Resolves the workspace-wide override family of `(seed_class, method)`
    /// via the class index, then — for every document that defines a family
    /// class — re-analyses it and collects the method's declaration,
    /// intra-class `my method` calls, and resolvable `$obj method` sites,
    /// converting each to an edit in that document.  Returns the per-URI
    /// edit map (empty when the family is empty, e.g. the class isn't
    /// indexed yet, so the caller can fall back to the single-document
    /// path).
    ///
    /// Documents holding a purely-inheriting subclass (a family member's
    /// descendant that doesn't override the method) are visited too, so their
    /// `my method` / `$obj method` sites are not missed just because the
    /// subclass lives in a different file from the definer.  **Pure-consumer**
    /// documents — ones that neither define nor extend any family class and
    /// only call `$obj method` / `Class method` — are covered by the same
    /// resolver Find All References uses
    /// ([`Self::consumer_method_site_ranges`]); leaving them out would rewrite
    /// the declaration while the consumer kept calling a name that no longer
    /// existed.
    pub(super) async fn cross_file_method_rename(
        &self,
        current_uri: &Uri,
        current_doc: &DocumentState,
        seed_class: &str,
        method: &str,
        new_name: &str,
        is_classmethod: bool,
    ) -> DocumentSourceResult<std::collections::HashMap<Uri, Vec<TextEdit>>> {
        let dialect = current_doc.dialect.as_str();
        let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> =
            std::collections::HashMap::new();
        let family = self
            .method_family_by_document(
                tcl_lsp_core::profile_for_dialect(dialect),
                seed_class,
                method,
                is_classmethod,
            )
            .await;
        let external_callback_receivers = family.external_callback_receivers;
        let classmethod_cmd_names = family.classmethod_cmd_names;
        for (
            u,
            FamilyClasses {
                definers,
                inheritors,
            },
        ) in family.by_uri
        {
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
                    DefinerSpans::Rename,
                )
            })
            .await
            .unwrap_or_default();
            if spans.is_empty() {
                continue;
            }
            merge_span_edits(
                changes.entry(parsed).or_default(),
                spans,
                &target_doc.line_index,
                &target_doc.text,
                new_name,
            );
        }
        // The consumer leg: documents that only *call* the
        // method.  Same resolver as the references path, so what Find All
        // References reports as a call site is exactly what rename rewrites.
        // A document already visited above contributes here too (its own
        // `$obj method` sites may resolve only through the workspace class
        // oracle); the per-range guard keeps the edit set duplicate-free.
        let consumer = self
            .consumer_method_site_ranges(
                current_uri,
                &current_doc.text,
                tcl_lsp_core::profile_for_dialect(dialect),
                ConsumerTarget {
                    seed_class,
                    method,
                    is_classmethod,
                    // The rename leg covers every indexed document, so a
                    // consumer that only *receives* an instance is rewritten
                    // rather than silently missed.
                    coverage: ConsumerCoverage::WholeWorkspace,
                },
            )
            .await?;
        for (uri, ranges) in consumer {
            let bucket = changes.entry(uri).or_default();
            for range in ranges {
                if !bucket.iter().any(|e| e.range == range) {
                    bucket.push(TextEdit {
                        range,
                        new_text: new_name.to_owned(),
                    });
                }
            }
        }
        Ok(changes)
    }
}
