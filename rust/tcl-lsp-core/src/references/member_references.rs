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

//! Class-member declarations and their reference spans: member bodies, member selection and inherited call sites.

use super::member_metadata;
use super::next_dispatch::method_next_dispatch_spans;
use super::self_dispatch_scan::scan_my_method_sites;
use tcl_compiler::analyser::AnalysisResult;

/// Every method / classmethod / constructor / destructor body span of `cd`
/// — the regions re-segmented for intra-class `my <member>` call sites.
/// `pub(crate)` so `rename`'s property-rename path can reuse it instead of
/// duplicating the same body-span collection.
pub(crate) fn collect_member_bodies(
    cd: &tcl_compiler::analyser::types::ClassDef,
) -> Vec<tcl_lexer::Span> {
    let mut bodies: Vec<tcl_lexer::Span> = cd
        .methods
        .values()
        .map(|m| m.body_span)
        .chain(cd.class_methods.values().map(|m| m.body_span))
        .chain(cd.constructors.iter().map(|c| c.body_span))
        .collect();
    if let Some(d) = &cd.destructor {
        bodies.push(d.body_span);
    }
    bodies
}

/// Every member body span of `cd` reachable for `my <name>` dispatch from a
/// body scoped the same way `is_classmethod` selects: instance-scoped
/// (`false` — methods, constructors, the destructor, everywhere `self` is
/// the instance) or class-scoped (`true` — class methods only, where `self`
/// is the class object itself).  The two tables never merge (confirmed
/// against tclsh 9.0.4, see
/// `tcl_compiler::analyser::diagnostics::var_command`'s dispatch-scope
/// note): a `my` dispatch written in one can never reach a member of the
/// other, so the re-segmented body set passed to [`scan_my_method_sites`]
/// must stay scoped to the same table `is_classmethod` selects — unlike
/// [`collect_member_bodies`], which mixes both (used only where the caller
/// has no per-table dispatch-scope concern of its own, e.g. `rename`'s
/// property scan).
pub(crate) fn collect_member_bodies_scoped(
    cd: &tcl_compiler::analyser::types::ClassDef,
    is_classmethod: bool,
) -> Vec<tcl_lexer::Span> {
    if is_classmethod {
        return cd.class_methods.values().map(|m| m.body_span).collect();
    }
    let mut bodies: Vec<tcl_lexer::Span> = cd
        .methods
        .values()
        .map(|m| m.body_span)
        .chain(cd.constructors.iter().map(|c| c.body_span))
        .collect();
    if let Some(d) = &cd.destructor {
        bodies.push(d.body_span);
    }
    bodies
}

/// Which of a class's independent member tables a name resolves to —
/// methods, classmethods, and properties never share one table, so a name
/// collision between them (rare, but real) needs an explicit tag alongside
/// its span; see [`resolve_member_span`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemberSel {
    Method,
    ClassMethod,
    Property,
}

/// Resolve which member of `class_def` named `word` a cursor at
/// `cursor_offset` refers to.
///
/// Methods, classmethods, and properties are independent tables, so a name
/// shared by more than one (rare, but real — `TclOO` never merges them)
/// disambiguates by which declaration's own span the cursor sits on;
/// otherwise falls back to the methods → classmethods → properties priority
/// order (the cursor sits on a call site, not any declaration, or there is
/// no collision at all).  `None` when `word` matches nothing in `class_def`.
pub(crate) fn resolve_member_span(
    class_def: &tcl_compiler::analyser::types::ClassDef,
    word: &str,
    cursor_offset: u32,
) -> Option<(MemberSel, tcl_lexer::Span)> {
    let candidates: Vec<(MemberSel, tcl_lexer::Span)> = [
        class_def
            .methods
            .get(word)
            .map(|m| (MemberSel::Method, m.name_span)),
        class_def
            .class_methods
            .get(word)
            .map(|m| (MemberSel::ClassMethod, m.name_span)),
        class_def
            .properties
            .get(word)
            .map(|p| (MemberSel::Property, p.name_span)),
    ]
    .into_iter()
    .flatten()
    .collect();
    candidates
        .iter()
        .find(|(_, span)| span.start() <= cursor_offset && cursor_offset <= span.end())
        .or_else(|| candidates.first())
        .copied()
}

/// Resolve the current declaration's receipt-backed method call sites.
pub(crate) fn method_references_for_class(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
) -> Option<(tcl_lexer::Span, Vec<tcl_lexer::Span>)> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let class = analysis.all_classes.get(class_q)?;
    let method = if is_classmethod {
        &class.class_methods
    } else {
        &class.methods
    }
    .get(method)?;
    Some(method_references_for_declaration(
        source,
        dialect,
        analysis,
        class,
        method,
        is_classmethod,
    ))
}

/// Select an original declaration independently of a later same-name class.
pub(crate) fn method_references_for_declaration(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class: &tcl_compiler::analyser::ClassDef,
    method: &tcl_compiler::analyser::MethodDef,
    is_classmethod: bool,
) -> (tcl_lexer::Span, Vec<tcl_lexer::Span>) {
    let receiver = if is_classmethod {
        tcl_compiler::command_binding::SourceMethodReceiver::Class
    } else {
        tcl_compiler::command_binding::SourceMethodReceiver::Instance
    };
    let mut calls =
        crate::receiver_identity::method_call_spans(analysis, source, class, method, receiver);
    calls.extend(crate::receiver_identity::captured_method_reference_spans(
        analysis, source, class, method, receiver,
    ));
    calls.extend(crate::receiver_identity::definition_method_reference_spans(
        analysis, source, class, method, receiver,
    ));
    calls.extend(
        tcl_compiler::analyser::types::original_source_linked_member_reference_spans(
            analysis,
            source,
            class,
            method,
            if is_classmethod {
                tcl_compiler::analyser::MemberSide::ClassObject
            } else {
                tcl_compiler::analyser::MemberSide::Instance
            },
        )
        .unwrap_or_default(),
    );
    (method.name_span, calls)
}

/// Reference spans for `method` on `class_q` **within `source`**, for
/// cross-document aggregation: every `$obj method` / `my method` call site,
/// plus the declaration when `include_decl`.  Empty when `class_q` does not
/// define `method` in this document.
///
/// The reference analogue of [`crate::rename::method_spans_in_document`]: the
/// server calls it per definer document in a method's override family (the
/// current document is already covered by [`references`]).
#[must_use]
pub fn method_reference_spans_in_document(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    include_decl: bool,
    is_classmethod: bool,
) -> Vec<tcl_lexer::Span> {
    match method_references_for_class(source, dialect, analysis, class_q, method, is_classmethod) {
        Some((decl, mut calls)) => {
            if include_decl {
                calls.push(decl);
            }
            // `next` / `nextto` super-dispatch is a reference (references path
            // only; rename uses `method_spans_in_document`, which excludes it).
            calls.extend(method_next_dispatch_spans(
                analysis,
                source,
                dialect,
                class_q,
                method,
                is_classmethod,
            ));
            calls
        }
        None => Vec::new(),
    }
}

/// What a **subclass-only** document cannot derive from its own tables, and
/// so must be handed by the workspace tier that can — the two facts
/// [`inherited_method_call_sites`] needs about a class it holds the
/// `ClassDef` for but whose method's *definer* may live in a document it
/// never mentions.
#[derive(Clone, Copy, Debug)]
pub struct InheritedReceiverFacts<'a> {
    /// The workspace-wide class names that are valid bare-dispatch heads for
    /// `method` when it is a `classmethod`.  A pure inheritor never declares a
    /// copy of `method`, so its own `class_methods` map never lists it.  See
    /// [`obj_method_call_sites`]'s identical parameter for the pure-consumer
    /// case this mirrors.
    pub extra_classmethod_cmd_names: &'a [String],
    /// Whether this receiver can dispatch `method` **externally**, so a
    /// captured `[self]` object command in its bodies really reaches the
    /// declaration.  A subclass-only document cannot decide that
    /// alone: its own `export` / `unexport` stub is local, but the provider's
    /// declared visibility is next door.  A `my` capture needs no such
    /// permission and is collected either way.
    pub external_callback_allowed: bool,
}

/// Call sites of an **inherited** `method` inside `class_q`, a class that
/// does *not* declare `method` itself but inherits it (its MRO resolves
/// `method` to an ancestor).  Returns the intra-class `my method` sites in
/// `class_q`'s own bodies plus the external `$obj method` sites for its
/// instances — no declaration span (there is none in this class).
///
/// This is the same-document scan a purely-inheriting subclass needs when it
/// lives in a *different* file from the method's definer: the cross-file
/// rename opens the subclass's document and collects these sites so an
/// inherited-method rename doesn't leave them pointing at the old name.
///
/// `extra_classmethod_cmd_names` carries the caller's workspace-wide
/// knowledge of which class names are valid bare-dispatch heads for `method`
/// when it is a `classmethod` — this document has `class_q`'s own `ClassDef`
/// (it *is* declared here), but not necessarily the *definer's* (a pure
/// inheritor never declares a copy of `method`, so `class_q`'s own
/// `class_methods` map never lists it, and the definer may live in a document
/// this one never mentions).  See
/// [`obj_method_call_sites`]'s identical parameter for the pure-consumer
/// case this mirrors.
#[must_use]
pub(crate) fn inherited_method_call_sites(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
    _workspace: InheritedReceiverFacts<'_>,
) -> Vec<tcl_lexer::Span> {
    let Some(class) = analysis.all_classes.get(class_q) else {
        return Vec::new();
    };
    let receiver = if is_classmethod {
        tcl_compiler::command_binding::SourceMethodReceiver::Class
    } else {
        tcl_compiler::command_binding::SourceMethodReceiver::Instance
    };
    crate::receiver_identity::method_calls_on_receiver(analysis, source, class, method, receiver)
}

/// Find a class member's declaration span plus every call
/// site inside any sibling method body.  Returns
/// `Some((decl_span, call_spans))` when the cursor sits
/// inside a class body and `word` matches one of that
/// class's members.
pub(super) fn find_class_member_references(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    word: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<(tcl_lexer::Span, Vec<tcl_lexer::Span>)> {
    let class_def = analysis
        .all_classes
        .get(crate::definition::enclosing_class_at(
            analysis,
            cursor_offset,
        )?)?;
    // Methods, classmethods, and properties are independent tables; a
    // name shared by more than one (rare, but real — `TclOO` never
    // merges them) disambiguates by which declaration's own span the
    // cursor sits on rather than a fixed priority — see
    // `resolve_member_span`.
    let (kind, member_span) = resolve_member_span(class_def, word, cursor_offset)?;
    if matches!(kind, MemberSel::Method | MemberSel::ClassMethod) {
        let is_classmethod = kind == MemberSel::ClassMethod;
        // Methods / classmethods: defer to the shared resolver — the *same*
        // one the code lens counts with — so the peek and the lens can never
        // drift.  It covers intra-class `my method` dispatch, external
        // `$obj method` / bare `objcmd method` sites, and the call sites of
        // any subclass that inherits (does not override) this definition
        // (which the class-local scan below would miss).
        let (decl, mut calls) = method_references_for_class(
            source,
            dialect,
            analysis,
            &class_def.qualified_name,
            word,
            is_classmethod,
        )?;
        // `next` / `nextto` super-dispatch is a reference / highlight (both
        // callers are read-only); rename resolves methods elsewhere and
        // must not see these keyword tokens.
        calls.extend(method_next_dispatch_spans(
            analysis,
            source,
            dialect,
            &class_def.qualified_name,
            word,
            is_classmethod,
        ));
        return Some((decl, calls));
    }
    // Properties: no `$obj prop` dispatch and no inheritance model, so a
    // class-local `my <prop>` scan is the whole story — the same
    // intra-class `my <name>` matcher [`scan_my_method_sites`] uses for
    // methods (recursing into `[...]` substitutions and same-frame
    // control-flow / `eval` bodies), just with no declaration span to
    // skip: a property's declaration is `property <name>` in the
    // definer body, never itself a `my <name>` call site.
    let call_spans = scan_my_method_sites(
        source,
        analysis,
        &collect_member_bodies(class_def),
        word,
        None,
    );
    Some((member_span, call_spans))
}

/// Readonly original definition metadata operands naming this source class's
/// method. Genuine class-factory/configuration correspondence and retained
/// vocabulary locate hazards; native worker/allocation receipts authorise edits.
pub(crate) fn member_reference_spans(
    source: &str,
    analysis: &AnalysisResult,
    dialect: &'static tcl_dialect::DialectProfile,
    class_def: &tcl_compiler::analyser::types::ClassDef,
    method: &str,
) -> Vec<tcl_lexer::Span> {
    member_metadata::spans(source, analysis, dialect, class_def, method)
}

/// Complete readonly source hazard inventory. `None` retains unavailable
/// class/body/selector coverage; it cannot license a rename through emptiness.
pub(crate) fn member_reference_inventory(
    source: &str,
    analysis: &AnalysisResult,
    dialect: &'static tcl_dialect::DialectProfile,
    class_def: &tcl_compiler::analyser::types::ClassDef,
    method: &str,
) -> Option<Vec<tcl_lexer::Span>> {
    member_metadata::checked_spans(source, analysis, dialect, class_def, method)
}
