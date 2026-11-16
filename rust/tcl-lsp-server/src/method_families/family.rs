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

//! A method's override family grouped by declaring document.

use super::super::documents::DocumentSourceResult;
use super::super::{Backend, DocumentState};
use super::{FamilyClasses, FamilySpanScan, MethodFamily};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::{
    references as core_references, rename as core_rename, workspace_index as core_workspace_index,
};
use tower_lsp_server::ls_types::{Position, Uri};

/// Which span set a **definer** in this document contributes — the one place
/// the two cross-file legs differ.
#[derive(Clone, Copy)]
pub(super) enum DefinerSpans {
    /// Everything renaming the member must rewrite, declaration included.
    Rename,
    /// Everything Find All References reports, with the declaration included
    /// only when the client asked for it.
    References { include_declaration: bool },
}

/// Every span one document contributes to a method family: its **definers'**
/// own set (per `definer_spans`) followed by its pure **inheritors'** call
/// sites, in the order the classes were given.
///
/// Shared by the cross-file references and rename passes so the two cannot
/// disagree about what a document contributes, and so the receiver-scoped
/// `[self]`-capture permission is read the same way on both.
pub(super) fn family_spans_in_document(
    scan: FamilySpanScan<'_>,
    definers: &[String],
    inheritors: &[String],
    definer_spans: DefinerSpans,
) -> Vec<tcl_lexer::Span> {
    let mut all: Vec<tcl_lexer::Span> = Vec::new();
    for cq in definers {
        match definer_spans {
            DefinerSpans::Rename => all.extend(core_rename::method_spans_in_document(
                scan.source,
                scan.profile,
                scan.analysis,
                cq,
                scan.method,
                scan.is_classmethod,
            )),
            DefinerSpans::References {
                include_declaration,
            } => all.extend(core_references::method_reference_spans_in_document(
                scan.source,
                scan.profile,
                scan.analysis,
                cq,
                scan.method,
                include_declaration,
                scan.is_classmethod,
            )),
        }
    }
    for cq in inheritors {
        all.extend(core_rename::inherited_method_spans_in_document(
            scan.source,
            scan.profile,
            scan.analysis,
            cq,
            scan.method,
            scan.is_classmethod,
            core_references::InheritedReceiverFacts {
                extra_classmethod_cmd_names: scan.classmethod_cmd_names,
                external_callback_allowed: scan.external_receivers.contains(cq),
            },
        ));
    }
    all
}

impl Backend {
    /// Resolve the `TclOO` method `(class, name, access)` under the cursor.
    /// Tries the cached (single-file) analysis first; if that finds nothing
    /// — the common pure-consumer case where `$obj`'s class is defined in
    /// another file and so is invisible to the file-local instance
    /// inference — retries against an analysis carrying the workspace
    /// class oracle.
    /// [`Self::resolve_method_target`] without the access context — the shape
    /// the rename entry points want.
    ///
    /// Rename and Find-All-References must resolve the cursor through the
    /// *same* tiers go-to-definition does, or a class-side call in a pure
    /// consumer file navigates but cannot be renamed.
    pub(super) async fn resolve_method_rename_target(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<Option<(String, String, bool)>> {
        Ok(self
            .resolve_method_target(
                uri,
                &doc.text,
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
                analysis,
                pos,
            )
            .await?
            .map(|(class_q, method, is_classmethod, _access)| (class_q, method, is_classmethod)))
    }

    pub(in super::super) async fn resolve_method_target(
        &self,
        uri: &Uri,
        source: &str,
        dialect: &'static tcl_dialect::DialectProfile,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<Option<(String, String, bool, core_workspace_index::MethodAccess)>>
    {
        if let Some(target) =
            core_rename::method_target_with_access(source, pos.line, pos.character, analysis)
        {
            return Ok(Some(target));
        }
        let oracle = self
            .analyse_with_workspace_classes(uri, source, dialect)
            .await?;
        // Last tier: the workspace index itself.  The oracle reanalysis above
        // widens *instance* inference (it supplies class names, so `[Cls new]`
        // types `$obj`), but a bare `ClassName member` needs the class's
        // class-side member table, which no reanalysis of *this* document can
        // produce — a pure consumer never mentions the class's body.  Handing
        // the index in lets the classmethod arm ask the one tier that knows.
        let index = self.workspace_index.read().await;
        Ok(core_rename::method_target_with_access_in_workspace(
            source,
            pos.line,
            pos.character,
            &oracle,
            Some((&index, uri.as_str())),
        ))
    }

    /// Whether `method` is a `classmethod` on any of `definers`, and — if
    /// so — the fully `::`-qualified name of every class in
    /// `definers`/`inheritors` whose own command dispatches it
    /// (`Factory make`).
    ///
    /// **Qualified names only.**  The core scanner resolves each written head
    /// against the call site's own lexical namespace and compares the winner
    /// against these names, so handing it simple names too would re-introduce
    /// exactly the namespace-blind text match this avoids: with a
    /// `Factory` in `::a` and another in `::b`, a bare `Factory make` inside
    /// `namespace eval ::b` must be attributed to `::b::Factory` alone.
    ///
    /// A `classmethod` dispatches on the *class's own* command, never on an
    /// instance, so the same-document analyser can only widen its dispatch
    /// scan to bare `ClassName method` calls when the querying document's own
    /// `all_classes` has the defining `ClassDef` (with its `class_methods`
    /// populated) — true for a document that declares or `oo::define`-extends
    /// the class, false for a document that only *inherits* it (a pure
    /// subclass-only file) or only *consumes* it (`Factory make` with no
    /// class mention at all).  For those two cases, the workspace index's
    /// per-method `kind` (`"method"` / `"classmethod"` / `"forward"` —
    /// [`core_workspace_index::WorkspaceMethod::kind`]) is the centralised
    /// answer instead of guessing from the command name, exactly as the
    /// same-document path reads `ClassDef::class_methods` locally.
    ///
    /// A stock-`TclOO` `self method` is **not** inherited: it lives on the
    /// class object that declared it, and a subclass's own class command
    /// never reaches it (`Gadget make` against a parent's `self method make`
    /// errors `unknown method "make": must be create, destroy or new` under
    /// tclsh 8.6 and 9.0.4).  `ooutil`'s `classmethod` keyword does
    /// propagate.  Both share the `"classmethod"` receiver kind, so the
    /// inheritors are folded in only when an indexed definer declares a
    /// genuinely inheritable copy — otherwise a rename would rewrite a
    /// subclass consumer's `Gadget make`, which never called the renamed
    /// member at all.
    pub(in super::super) fn classmethod_dispatch_names(
        definers: &[&core_workspace_index::WorkspaceClass],
        inheritors: &[&core_workspace_index::WorkspaceClass],
        method: &str,
        is_classmethod: bool,
        _dialect: &'static tcl_dialect::DialectProfile,
    ) -> Vec<String> {
        // `is_classmethod` is the caller's already-resolved fact (from the
        // cursor, or the caller's own test fixture) about which of a
        // possibly-same-named `method` / `classmethod` pair is meant — not
        // re-derived from workspace membership, which can't disambiguate
        // when a class legally defines both.
        if !is_classmethod {
            return Vec::new();
        }
        let declarations: Vec<&core_workspace_index::WorkspaceMethod> = definers
            .iter()
            .filter_map(|wc| wc.class_method(method))
            .collect();
        // No indexed declaration at all (a class not yet indexed) keeps the
        // inheritors — the exclusion needs positive knowledge that every
        // declaration is a `self method`.
        let inheritable = declarations.is_empty() || declarations.iter().any(|m| !m.is_self_method);
        definers
            .iter()
            .chain(inheritors.iter().filter(|_| inheritable))
            .filter(|wc| {
                wc.definer_family()
                    .is_some_and(|family| family != tcl_registry::definer::DefinerFamily::Itcl)
            })
            .map(|wc| wc.qualified_name.clone())
            .collect()
    }

    /// The override-family classes of `(seed_class, method)` grouped by the
    /// document that declares them: per URI, the **definers** (whose
    /// declaration + call sites participate) and the pure **inheritors**
    /// (whose `my method` / `$obj method` sites participate, but which
    /// declare no copy of the method).  A document may contribute either or
    /// both.  Sorted by URI so the visit order — and so the order sites are
    /// emitted in — never rides on hash iteration order.
    ///
    /// `classmethod_cmd_names`: see [`Self::classmethod_dispatch_names`] — a
    /// pure-inheritor document's own `all_classes` never lists the definer's
    /// `class_methods`, so it cannot tell a classmethod's bare dispatch heads
    /// apart from any other bare word without this.
    ///
    /// Shared by rename ([`Self::cross_file_method_rename`]) and references
    /// ([`Self::cross_file_method_references`]) so the two resolve the same
    /// family from the same index read.
    pub(super) async fn method_family_by_document(
        &self,
        dialect: &'static tcl_dialect::DialectProfile,
        seed_class: &str,
        method: &str,
        is_classmethod: bool,
    ) -> MethodFamily {
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
        let mut by_uri: std::collections::BTreeMap<String, FamilyClasses> =
            std::collections::BTreeMap::new();
        for wc in definers {
            by_uri
                .entry(wc.uri.clone())
                .or_default()
                .definers
                .push(wc.qualified_name.clone());
        }
        for wc in inheritors {
            by_uri
                .entry(wc.uri.clone())
                .or_default()
                .inheritors
                .push(wc.qualified_name.clone());
        }
        for classes in by_uri.values_mut() {
            classes.definers.sort();
            classes.definers.dedup();
            classes.inheritors.sort();
            classes.inheritors.dedup();
        }
        // The family a receiver's capture may join is the set of definers
        // this pass is about; a receiver whose external dispatch lands
        // elsewhere is dispatching a different member.
        let definer_names: Vec<&str> = by_uri
            .values()
            .flat_map(|classes| classes.definers.iter().map(String::as_str))
            .collect();
        let external_callback_receivers = index.external_dispatch_receivers(
            by_uri
                .values()
                .flat_map(|classes| classes.inheritors.iter().map(String::as_str)),
            &definer_names,
            method,
            is_classmethod,
        );
        drop(index);
        MethodFamily {
            by_uri: by_uri.into_iter().collect(),
            classmethod_cmd_names,
            external_callback_receivers,
        }
    }
}
