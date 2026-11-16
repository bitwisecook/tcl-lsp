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

//! `namespace import` across documents: wildcard and exact imports, export gates, import chains and the export snapshot.

use super::WorkspaceIndex;
use super::records::{
    WorkspaceCommandDeletion, WorkspaceGlobImport, WorkspaceImportGate, WorkspaceNamespaceExport,
    WorkspaceNamespaceForget,
};
use crate::namespace_import::ExportVerdict;
use crate::source_graph::RunPoint;
use std::collections::HashSet;
use std::sync::Arc;
use tcl_lexer::Span;
use tcl_syntax::naming::unroot_rooted_key;

mod export_snapshot;
mod wildcard_index;

pub use export_snapshot::NamespaceExportSnapshot;
pub(super) use wildcard_index::{ExactImportRow, ExportGate};

impl WorkspaceGlobImport {
    /// This import's site, for the export-snapshot gate.
    pub(super) fn site(&self) -> ImportSite<'_> {
        ImportSite {
            uri: &self.uri,
            at: self.at,
            enclosing_body: self.enclosing_body,
        }
    }
}

impl WorkspaceImportGate {
    /// This gate's import site, for the export-snapshot query. `uri` is the
    /// owning link's document — the gate itself does not duplicate it.
    pub(super) fn site<'a>(&self, uri: &'a str) -> ImportSite<'a> {
        ImportSite {
            uri,
            at: self.at,
            enclosing_body: self.enclosing_body,
        }
    }
}

impl WorkspaceIndex {
    /// All genuine original namespace patterns, including opaque operands with
    /// no reporting import row. This retains source geometry, not live aliases.
    pub fn original_namespace_patterns(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::signature_scan::original_name::SourceNamespacePattern,
        ),
    > {
        self.docs.iter().flat_map(|doc| {
            doc.original_namespace_patterns
                .iter()
                .map(move |pattern| (doc.uri.as_str(), pattern))
        })
    }

    /// Source candidate export advice for a genuine import pattern and genuine
    /// source publication name. It grants no import completion or table entry.
    #[must_use]
    pub fn original_import_export_verdict(
        &self,
        pattern: &tcl_compiler::signature_scan::original_name::SourceNamespacePattern,
        name: &tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        import_uri: &str,
    ) -> ExportVerdict {
        if !self
            .original_namespace_patterns()
            .any(|(uri, retained)| uri == import_uri && retained == pattern)
            || pattern.matches_imported_command(name.slot(), name.policy()) != Some(true)
        {
            return ExportVerdict::Unknown;
        }
        let body = self
            .docs
            .iter()
            .find(|doc| doc.uri == import_uri)
            .and_then(|doc| {
                doc.original_procs
                    .iter()
                    .map(|record| record.metadata().body_span)
                    .chain(
                        doc.original_classes
                            .iter()
                            .map(|record| record.metadata().body_span),
                    )
                    .filter(|span| {
                        span.start() <= pattern.span().start()
                            && pattern.span().start() < span.end()
                    })
                    .min_by_key(|span| span.len())
            });
        crate::namespace_import::NamespaceExportOracle::exported_at_original(
            self.export_snapshot().as_ref(),
            name.slot(),
            name.policy(),
            RunPoint {
                uri: import_uri,
                at: pattern.span().start(),
                enclosing_body: body,
            },
        )
    }

    /// Project readonly conditional namespace-import source advice for a
    /// bare call. The legacy qualified-name string is presentation only: a
    /// positive establishes no live alias or Native command identity, and
    /// None establishes no complete candidate inventory or namespace absence.
    /// Unknown entry/completion/observer/lifecycle alternatives remain retained
    /// in [`Self::conditional_namespace_import_advice`]. Edit/binding consumers
    /// must use the independently guarded [`Self::resolve_wildcard_import_for_binding`].
    ///
    /// `resolution_candidates` preserves the caller's authored order; `call`
    /// supplies an actual written point or the explicit legacy report horizon.
    /// Qualified words decline, matching the existing bareword query purpose.
    #[must_use]
    pub fn resolve_wildcard_import(
        &self,
        word: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
    ) -> Option<String> {
        self.conditional_namespace_import_advice(word, resolution_candidates, call)?
            .compatibility_report()
            .map(str::to_owned)
    }

    /// Existing binding/edit interpretation, independently retaining modeled
    /// exclusion guards. Conditional report advice cannot enter this path.
    #[must_use]
    pub fn resolve_wildcard_import_for_binding(
        &self,
        word: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
    ) -> Option<String> {
        self.resolve_wildcard_import_indexed(
            word,
            resolution_candidates,
            call,
            &WildcardImportIndex::build(self),
        )
    }

    /// Source-only namespace relationship under its retained statement horizon.
    /// No candidate is a live-slot absence or a completed namespace operation.
    #[must_use]
    pub fn conditional_namespace_import_advice(
        &self,
        word: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
    ) -> Option<ConditionalNamespaceImportAdvice> {
        let documents = self
            .docs
            .iter()
            .map(|document| {
                Some(NamespaceAdviceDocument {
                    uri: document.uri.clone(),
                    context: document.source_order_context.clone()?,
                    revision: document.revision,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let wci = WildcardImportIndex::build_for_purpose(
            self,
            crate::namespace_import::NamespaceStatementPurpose::SourceHorizon,
        );
        let selected =
            self.resolve_wildcard_import_target_indexed(word, resolution_candidates, call, &wci);
        let subjects = selected
            .as_ref()
            .and_then(|target| target.pattern)
            .map_or_default(|pattern| {
                wci.original_subjects
                    .iter()
                    .copied()
                    .filter(|command| {
                        pattern.matched_imported_command_tail(command) == Some(word.as_bytes())
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            });
        let target = subjects.first().and_then(|first| {
            subjects
                .iter()
                .all(|subject| subject.slot() == first.slot() && subject.policy() == first.policy())
                .then(|| first.source_spelling())
                .flatten()
        });
        Some(ConditionalNamespaceImportAdvice {
            target,
            subjects,
            documents,
            order: self.run_order(),
            generation: self.generation(),
            patterns: self
                .docs
                .iter()
                .flat_map(|document| document.original_namespace_patterns.iter().cloned())
                .collect(),
            exports: self
                .namespace_exports()
                .filter_map(|event| event.original.clone())
                .collect(),
        })
    }

    /// The indexed core of [`Self::resolve_wildcard_import`], taking a
    /// precomputed [`WildcardImportIndex`] instead of scanning
    /// `glob_imports`/`namespace_exports` in full — the fast path
    /// [`Self::settle_invocation`] uses so a workspace-wide
    /// invocation-settling pass builds the index once (O(every glob import
    /// / export in the workspace)) rather than once per invocation (which
    /// would be O(invocation count × workspace-wide glob-import count), and
    /// measurably slows find-references on a codebase using
    /// `namespace import NS::*` in more than a handful of files).
    pub(super) fn resolve_wildcard_import_indexed(
        &self,
        word: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
        wci: &WildcardImportIndex<'_>,
    ) -> Option<String> {
        self.resolve_wildcard_import_target_indexed(word, resolution_candidates, call, wci)
            .map(|target| target.report)
    }

    fn resolve_wildcard_import_target_indexed<'i>(
        &self,
        word: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
        wci: &WildcardImportIndex<'i>,
    ) -> Option<ImportChainTarget<'i>> {
        if tcl_syntax::naming::is_qualified(word.as_bytes()) {
            return None;
        }
        for cand in resolution_candidates {
            let (prefix, tail) = tcl_syntax::naming::key_holder_and_tail(cand);
            if tail != word {
                continue;
            }
            let candidate_ns = if prefix.is_empty() { "::" } else { prefix };
            if let Some(target) = self.follow_import_chain(candidate_ns, word, call, wci) {
                return Some(target);
            }
        }
        None
    }

    /// Follow the import edges out of `ns` until they reach a command the
    /// workspace actually defines, and return that qualified name.
    ///
    /// An import edge may land on a name that is *itself* imported: with
    /// `::C` exporting `p`, `::B` importing `::C::*` and re-exporting, and
    /// `::A` importing `::B::*`, `::A::p` runs `::C`'s body and `namespace
    /// origin ::A::p` answers `::C::p` (oracle tclsh 8.6.14 / 9.0.4). The
    /// middle hop is in no workspace proc/class table, so a single-hop walk
    /// finds nothing and go-to-definition silently abstains.
    ///
    /// Bounded by [`tcl_compiler::analyser::indirection::MAX_COMMAND_NAME_HOPS`]
    /// — the same cap the `rename` / `interp alias` walk applies to the same
    /// kind of chain, so a mutually-importing pair cannot spin. The whole
    /// chain is judged at the *call's* offset: a forget anywhere along it
    /// kills the call (oracle: forgetting `::C::p` inside `::B` makes
    /// `::A::p` an `invalid command name` too, because deleting an imported
    /// command deletes the commands imported from it).
    pub(super) fn follow_import_chain<'i>(
        &self,
        ns: &str,
        word: &str,
        call: CallSite<'_>,
        wci: &WildcardImportIndex<'i>,
    ) -> Option<ImportChainTarget<'i>> {
        let mut current = ns.to_owned();
        for _ in 0..tcl_compiler::analyser::indirection::MAX_COMMAND_NAME_HOPS {
            let row = self.import_hop(&current, word, call, wci)?;
            let import: &'i WorkspaceGlobImport = row.imp;
            let hop = &import.source_ns;
            let target = tcl_syntax::naming::qualify(hop, word);
            if self.workspace_command_exists(&target) {
                return Some(ImportChainTarget {
                    report: target,
                    pattern: import.original_pattern.as_ref(),
                });
            }
            current = hop.clone();
        }
        None
    }

    /// One hop of [`Self::follow_import_chain`]: the source namespace of the
    /// live import that makes `word` callable from `ns` at `call`, or `None`.
    ///
    /// Three gates, in the order real Tcl applies them:
    ///
    /// 1. the pattern must cover `word`;
    /// 2. the source namespace must have exported it **at that import's own
    ///    position** ([`WildcardImportIndex::exports_name_at`]);
    /// 3. without `-force`, the importing namespace must not already hold a
    ///    command of that name — such an import raises `can't import command
    ///    "p": already exists` and installs nothing, so a bare call still
    ///    reaches the local definition. With `-force` it
    ///    replaces the local one instead, which is why the check is skipped
    ///    there.
    ///
    /// …and then the edge must still be *there*
    /// ([`WildcardImportIndex::alias_live_at`]).
    pub(super) fn import_hop<'w, 'i>(
        &self,
        ns: &str,
        word: &str,
        call: CallSite<'_>,
        wci: &'w WildcardImportIndex<'i>,
    ) -> Option<&'w GlobImportRow<'i>> {
        let imports = wci.imports_by_ns.get(ns)?;
        // The **latest** live install wins, not the first match: a second
        // import of the same name — `-force`, or after a forget — replaces
        // the first alias (oracle on `conflicting_alias_at`), exactly as the
        // same-document tier's fold decides it. Offsets only order rows in
        // the calling document, so same-document imports rank above
        // unordered foreign ones, and among foreign ones the index's own
        // stable source order breaks the tie.
        imports
            .admitting(word)
            .filter(|row| {
                let target = tcl_syntax::naming::qualify(&row.imp.source_ns, word);
                !self.workspace_command_exists(&target)
                    || self.import_source_matches(&target, row.imp.native_source.as_ref(), word)
            })
            .filter(|row| {
                let target = tcl_syntax::naming::qualify(ns, word);
                row.imp.forced
                    || !((wci.purpose
                        == crate::namespace_import::NamespaceStatementPurpose::Modeled
                        && (self.defines_command(&target) || row.declares_builtin_at(&target)))
                        || wci.conflicting_alias_at(ns, &row.imp.source_ns, word, row.site()))
            })
            .filter(|row| wci.alias_live_at(ns, &row.imp.source_ns, word, row.site(), call))
            .enumerate()
            .max_by_key(|(seq, row)| (row.imp.uri == call.uri, row.imp.at, *seq))
            .map(|(_, row)| row)
    }
}

pub(super) struct ImportChainTarget<'a> {
    report: String,
    pub(super) pattern:
        Option<&'a tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
}

/// One actual document input consumed by conditional namespace source advice.
#[derive(Debug, Clone)]
pub struct NamespaceAdviceDocument {
    pub(super) uri: String,
    context: crate::source_graph::SourceOrderContext,
    pub(super) revision: u64,
}

impl NamespaceAdviceDocument {
    /// The independently retained indexed document URI.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }
    /// Full original source text; unavailable text never becomes an empty image.
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.context.source_text()
    }
    /// Match the exact retained full analysis input and its original Realm.
    #[must_use]
    pub fn matches_input(&self, input: &tcl_compiler::analyser::ResolvedAnalysisInput) -> bool {
        self.context.matches_input(input)
    }
}

/// Remaining obligations of a conditional namespace relationship report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalNamespaceDebt {
    /// Source geometry does not establish entry or successful completion.
    EntryAndCompletion,
    /// Namespace observers and effects have not been discharged.
    NamespaceObservers,
    /// Unmodeled alternative command/namespace outcomes remain possible.
    UnknownAlternatives,
    /// Raw lifecycle/body-replacement rows supply no operation receipt.
    UnrepresentedLifecycle,
}

/// A readonly conditional relationship, not a command identity or edit proof.
/// Unknown entry/completion/observer and unmodeled alternatives always remain.
#[derive(Debug, Clone)]
pub struct ConditionalNamespaceImportAdvice {
    pub(super) target: Option<String>,
    subjects: Vec<tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
    documents: Vec<NamespaceAdviceDocument>,
    order: Arc<crate::source_graph::RunOrder>,
    pub(super) generation: u64,
    patterns: Vec<tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
    pub(super) exports: Vec<tcl_compiler::signature_scan::original_name::SourceNamespaceExport>,
}

impl ConditionalNamespaceImportAdvice {
    /// Explicit legacy presentation only. None is not live namespace absence.
    #[must_use]
    pub fn compatibility_report(&self) -> Option<&str> {
        self.target.as_deref()
    }
    /// Actual source contexts consumed by this conditional relationship.
    #[must_use]
    pub fn documents(&self) -> &[NamespaceAdviceDocument] {
        &self.documents
    }
    /// Always conditional, including an empty modeled candidate set.
    #[must_use]
    pub const fn has_unresolved_operation_debt(&self) -> bool {
        true
    }
    /// These debts survive both positive and empty compatibility projections.
    #[must_use]
    pub const fn debts(&self) -> &[ConditionalNamespaceDebt] {
        &[
            ConditionalNamespaceDebt::EntryAndCompletion,
            ConditionalNamespaceDebt::NamespaceObservers,
            ConditionalNamespaceDebt::UnknownAlternatives,
            ConditionalNamespaceDebt::UnrepresentedLifecycle,
        ]
    }

    /// Authenticate the original subjects, contexts, revisions and order allocation.
    #[must_use]
    pub fn matches_index(&self, index: &WorkspaceIndex) -> bool {
        self.subjects.iter().all(|subject| {
            index.docs.iter().any(|document| {
                document
                    .original_procs
                    .iter()
                    .any(|declaration| declaration.name() == subject)
                    || document
                        .original_classes
                        .iter()
                        .any(|declaration| declaration.name() == subject)
            })
        }) && self.generation == index.generation()
            && Arc::ptr_eq(&self.order, &index.run_order())
            && self.documents.iter().all(|document| {
                index
                    .slots
                    .get(&document.uri)
                    .and_then(|slot| index.docs.get(*slot))
                    .is_some_and(|current| {
                        current.revision == document.revision
                            && current.source_order_context.as_ref() == Some(&document.context)
                    })
            })
            && self.patterns.iter().all(|pattern| {
                index
                    .docs
                    .iter()
                    .any(|document| document.original_namespace_patterns.contains(pattern))
            })
            && self.exports.iter().all(|export| {
                index
                    .namespace_exports()
                    .any(|event| event.original.as_ref() == Some(export))
            })
    }
}

/// One recorded wildcard `namespace import NS::*`, with its export gate
/// already decided — see [`ExportGate`].
#[derive(Debug)]
pub(super) struct GlobImportRow<'a> {
    pub(super) imp: &'a WorkspaceGlobImport,
    pub(super) exported: ExportGate<'a>,
    /// Declared-command metadata from this import document's retained store.
    /// Original import/export/source capabilities remain independently guarded.
    /// `None` when the actual source input cannot supply that store.
    pub(super) registry: Option<&'a tcl_registry::CommandRegistry>,
}

/// Every wildcard import recorded for one importing namespace, indexed by the
/// names its imports can actually admit.
///
/// Scanning every row in the namespace on every call, applying
/// `string_match(tail_pattern, word)` and `ExportGate::covers(word)` to each,
/// costs ~140 rows per call on a large corpus — 28 369 calls scanning
/// 3 987 739 rows, and ~390 ms of the ~420 ms a `textDocument/references`
/// takes after an edit.
///
/// Both of those filters depend only on the *word*, never on the call, so the
/// admissible set per word is a build-time fact. A row whose gate exports
/// nothing can admit no word at all and disappears from every lookup — which
/// is most of them in practice, since a workspace typically imports several
/// namespaces it has no `namespace export` records for (`::tcl::mathop`,
/// `::tcltest`).
#[derive(Debug, Default)]
pub(super) struct NamespaceImports<'a> {
    /// The namespace's rows, in recorded order. [`Self::admitting`] yields
    /// indices into this, ascending, so callers see exactly the subsequence a
    /// full scan would have produced.
    pub(super) rows: Vec<GlobImportRow<'a>>,
    /// Row indices keyed by a **literal** name that both the row's tail
    /// pattern and its export gate admit.
    by_name: rustc_hash::FxHashMap<&'a str, Vec<u32>>,
    /// Rows whose export gate holds a glob pattern: any word may pass, so
    /// these keep the per-word check.
    glob_gated: Vec<u32>,
    /// `-force` rows, for [`WildcardImportIndex::forced_shadow_at`] — which
    /// can bypass the export gate entirely for an unobservable source
    /// namespace, so it cannot use [`Self::by_name`].
    pub(super) forced: Vec<u32>,
}

/// Precomputed per-namespace grouping of [`WorkspaceIndex::glob_imports`]
/// and [`WorkspaceIndex::namespace_exports`], built once per query rather
/// than re-scanned per call site — see
/// [`WorkspaceIndex::resolve_wildcard_import_indexed`]'s doc for why.
pub(super) struct WildcardImportIndex<'a> {
    pub(super) source_patterns: Vec<(
        &'a str,
        &'a tcl_compiler::signature_scan::original_name::SourceNamespacePattern,
    )>,
    pub(super) purpose: crate::namespace_import::NamespaceStatementPurpose,
    pub(super) original_subjects:
        Vec<&'a tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
    pub(super) imports_by_ns: std::collections::HashMap<&'a str, NamespaceImports<'a>>,
    /// Every **exact** `namespace import` link, by importing namespace. The
    /// exact-pattern twin of [`Self::imports_by_ns`]: the two tables are one
    /// command table as far as the import-conflict rule is concerned, which is
    /// why [`Self::conflicting_alias_at`] reads both.
    pub(super) exact_by_ns: std::collections::HashMap<&'a str, Vec<ExactImportRow<'a>>>,
    pub(super) exports_by_ns: std::collections::HashMap<&'a str, Vec<&'a WorkspaceNamespaceExport>>,
    pub(super) forgets_by_ns: std::collections::HashMap<&'a str, Vec<&'a WorkspaceNamespaceForget>>,
    pub(super) deletions_by_name:
        std::collections::HashMap<&'a str, Vec<&'a WorkspaceCommandDeletion>>,
    /// Every proc / class declaration site, by `::`-rooted qualified name —
    /// a redefinition of an *imported* name ends the alias, which is a
    /// question about where the declaration sits, not
    /// merely whether one exists.
    pub(super) declarations_by_qname: std::collections::HashMap<&'a str, Vec<(&'a str, u32)>>,
    /// The workspace's `source`-graph load order — the relation every
    /// decision below ranks its events with, so a cross-document event is
    /// ordered wherever the graph proves an order and unrankable everywhere
    /// else.
    pub(super) order: Arc<crate::source_graph::RunOrder>,
    /// The `::`-stripped namespaces this workspace can say anything about —
    /// [`WorkspaceIndex::observable_namespaces`]. Only
    /// [`Self::forced_shadow_at`] reads it, and only to abstain the way the
    /// single-document tier does: a `-force` import of a namespace no indexed
    /// file declares deletes the local command whether or not the export can
    /// be proven here.
    pub(super) observable: HashSet<&'a str>,
}

/// A namespace's indexed export rows as [`crate::namespace_import::ExportEvent`]s
/// — the one place `WorkspaceNamespaceExport` is turned into one.
///
/// No enclosing-body span: the index stores none per export row, so every
/// consumer ranks a `-clear` against a pattern by position alone. Symmetric,
/// and the safe direction — a body-local export the tombstone cannot be proven
/// to follow keeps the name exported.
fn export_events<'e>(
    exports: impl Iterator<Item = &'e WorkspaceNamespaceExport>,
) -> impl Iterator<Item = crate::namespace_import::ExportEvent<'e>> {
    exports.map(|e| crate::namespace_import::ExportEvent {
        pattern: e.pattern.as_str(),
        clears: e.clears,
        at: RunPoint {
            uri: e.uri.as_str(),
            at: e.at,
            enclosing_body: e.enclosing_body,
        },
    })
}

/// [`crate::namespace_import::exported_at_import_site`] over a namespace's
/// indexed export rows.
///
/// Shared by the per-call wildcard walk ([`WildcardImportIndex::exports_name_at`])
/// and the standalone [`NamespaceExportSnapshot`] the single-document tier
/// borrows, so the two cannot answer the same question differently.
fn exported_at_site<'e>(
    exports: impl Iterator<Item = &'e WorkspaceNamespaceExport>,
    name: &str,
    order: &crate::source_graph::RunOrder,
    site: RunPoint<'_>,
) -> bool {
    let mut events = export_events(exports);
    crate::namespace_import::exported_at_import_site(&mut events, name, order, site)
}

/// The namespace an imported name is installed *into*, read off the link's
/// `linked_qname` (`::app::helper` → `::app`, a global-level import → `::`).
pub(super) fn importing_namespace_of(linked_qname: &str) -> &str {
    let (ns, _) = tcl_syntax::naming::key_holder_and_tail(linked_qname);
    if ns.is_empty() { "::" } else { ns }
}

/// Namespace-name equality, ignoring a leading `::` one spelling carries and
/// the other does not.
fn ns_eq(a: &str, b: &str) -> bool {
    unroot_rooted_key(a).unwrap_or(a) == unroot_rooted_key(b).unwrap_or(b)
}

/// Where a `namespace import` sits, as the export-snapshot gate needs to see
/// it: the document, the offset, and the innermost proc/class body containing
/// it (`None` at load level).
///
/// The body span is what makes the workspace tier's ordering rule the *same*
/// rule the same-document tier applies rather than a weaker offset compare —
/// see [`WorkspaceGlobImport::enclosing_body`] and
/// [`tcl_compiler::analyser::indirection::in_effect_within`]. Carried as one
/// value so no caller can pass two of the three and silently drop the third.
#[derive(Debug, Clone, Copy)]
pub(super) struct ImportSite<'a> {
    pub(super) uri: &'a str,
    pub(super) at: u32,
    pub(super) enclosing_body: Option<Span>,
}

impl<'a> ImportSite<'a> {
    /// This site as the workspace-wide execution-timeline point
    /// [`crate::source_graph::RunOrder`] ranks events by.
    pub(super) fn point(self) -> RunPoint<'a> {
        RunPoint {
            uri: self.uri,
            at: self.at,
            enclosing_body: self.enclosing_body,
        }
    }
}

/// Where the **call** being resolved sits: the document and the offset of its
/// command-head token.
///
/// The import edge's lifecycle is a question about the call, not about the
/// import — a `namespace forget` written after the import kills calls after
/// it and leaves calls before it alone — so the resolver needs the call's own
/// position.
///
/// Carries the call's own **enclosing body span** as well, for the same reason
/// [`WorkspaceGlobImport::enclosing_body`] exists on the import side: ordering
/// an import against a call is not a plain offset compare. A call inside a
/// proc body observes every top-level statement of its own file, wherever
/// written, because the whole file loads before any body runs — so a body-local
/// call of a name imported further down the same file still resolves, and a
/// top-level `namespace forget` written after such a body does revoke it
/// too. Comparing offsets alone gets the first of those wrong once installs
/// are order-gated, which is why the span is required rather than optional.
///
/// `None` means "top level" — the same encoding
/// [`tcl_compiler::analyser::AnalysisResult::innermost_definition_body_span`]
/// uses, which is where every producer gets it: directly for a caller holding
/// the calling document's analysis, and from
/// [`WorkspaceInvocation::enclosing_body`] for the index-driven sweep.
#[derive(Debug, Clone, Copy)]
pub struct CallSite<'a> {
    /// Document the call is in.
    pub uri: &'a str,
    /// Byte offset of the call's command-head token within `uri`.
    pub at: u32,
    /// Span of the innermost proc/class **body** containing the call, within
    /// `uri`; `None` when the call sits at load level.
    pub enclosing_body: Option<Span>,
}

impl<'a> CallSite<'a> {
    /// This call as the workspace-wide execution-timeline point
    /// [`crate::source_graph::RunOrder`] ranks events against.
    pub(super) fn point(self) -> RunPoint<'a> {
        RunPoint {
            uri: self.uri,
            at: self.at,
            enclosing_body: self.enclosing_body,
        }
    }
}
