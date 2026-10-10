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

//! Source call-hierarchy advice.
//!
//! [`prepare`], [`incoming_calls`] and [`outgoing_calls`] route Native source
//! queries through the shared sealed declaration identity and
//! `original_call_hierarchy` kernel. Genuine original command or member inputs
//! retain their selected allocation, role, side and complete source/configuration.
//! Independently owned URI/source inventories supply cross-document candidates;
//! ambiguous providers, stale receipts and unavailable original targets decline
//! advice. Item labels and wire ranges cannot reconstruct a declaration.
//!
//! These edges describe readonly source relationships. They do not establish
//! live native dispatch, an entered object/frame or source edit permission.
//! Alias routes may retain the canonical target allocation while direct editable
//! spelling remains a separate question.
//!
//! Explicit lexical declaration advice uses the compatibility scanners below:
//! procedure body spans and registry-selected method shapes group local call
//! sites. Synthetic names and reporting maps in that path do not supply Native
//! identity. The compatibility server can use [`unresolved_outgoing_calls`] and
//! [`incoming_calls_for_target`] under the same independently selected advice
//! contract.

use tcl_compiler::analyser::{AnalysisResult, ClassDef, MethodDef, ProcDef};
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

/// One hierarchy item — proc identification plus its name and
/// definition span for editor display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallHierarchyItem {
    /// Sealed current source declaration; reporting item fields do not reissue it.
    pub identity: Option<crate::original_declaration::OriginalDeclarationIdentity>,
    /// Proc name (qualified).
    pub name: String,
    /// Detail (e.g. parameter list summary).
    pub detail: Option<String>,
    /// Range of the entire definition.
    pub range: LspRange,
    /// Range of just the name token.
    pub selection_range: LspRange,
}

/// Resolve a "prepare call hierarchy" request to a single
/// item — the proc whose name is at the cursor.
#[must_use]
pub fn prepare(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<CallHierarchyItem> {
    prepare_in_program(
        source,
        line,
        character,
        analysis,
        crate::definition::CallResolution::document_only(),
    )
}

/// [`prepare`] with the caller's whole-program export view attached — the
/// entry point a host with a workspace index should call.
///
/// Which proc a call edge points at is a call resolution, so a `namespace
/// import -force` whose covering `namespace export` lives in another file
/// changes the hierarchy. Without the oracle the graph
/// would draw an edge to a definition go-to-definition refuses to open.
#[must_use]
pub fn prepare_in_program(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<CallHierarchyItem> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }

    if let std::ops::ControlFlow::Break(selected) =
        crate::original_declaration::select("", source, analysis, line, character)
    {
        let document = crate::original_call_hierarchy::OriginalCallDocument {
            uri: "",
            source,
            analysis,
        };
        return selected
            .and_then(|identity| crate::original_call_hierarchy::item(&identity, &document))
            .into_iter()
            .collect();
    }

    // The original selector can continue for variables and non-command
    // source positions. Those positions cannot borrow a same-named method
    // from the separate Logical compatibility maps.
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    // Declaration-span hit, else the namespace-aware candidate resolution
    // (mirrors `references::proc_references` / `rename::rename_proc`) —
    // never a namespace-blind `p.name == word` first-`HashMap`-hit scan.
    let cursor_off = crate::definition::byte_offset_at(&line_index, source, line, character);
    let proc_match =
        crate::definition::resolve_proc_target_at(analysis, source, cursor_off, &word, resolution);
    if let Some((qname, proc_def)) = proc_match {
        return vec![item_for_proc(source, proc_def, qname, &line_index)];
    }
    // Class-method fallback — cursor inside a class body on a
    // method / classmethod name.
    let cursor_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some((class_def, method)) = enclosing_class_method(analysis, &word, cursor_offset) {
        return vec![item_for_method(source, class_def, method, &line_index)];
    }
    Vec::new()
}

fn method_item_name(class_def: &ClassDef, method: &MethodDef) -> String {
    format!("{}::{}", class_def.qualified_name, method.name)
}

/// Grouping identity for one end of a call-hierarchy edge — the caller
/// bucket of an incoming list, the callee bucket of an outgoing one.
///
/// The display name alone is **not** an identity.  A `TclOO` class may
/// define an instance `method make` and a `classmethod make` at the same
/// time: the two live in independent method tables, and tclsh 9.0.4 sends
/// `my make` to the instance copy and `C make` to the class copy.  Both
/// render as `::C::make`, so keying the grouping on the name text alone
/// would merge them into a single item whose ranges span both call sites and
/// whose declaration range is whichever entry was inserted first.  Pairing
/// the name with the declaration's own name-token span separates them while
/// keeping the emitted order name-first, hence reproducible.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CallItemKey {
    /// Qualified display name — the primary sort key.
    name: String,
    /// The declaration's name-token span, as `(start, end)`.  `(0, 0)` for
    /// the synthetic top-level item, which declares nothing.
    decl: (u32, u32),
}

impl CallItemKey {
    fn for_method(class_def: &ClassDef, method: &MethodDef) -> Self {
        Self {
            name: method_item_name(class_def, method),
            decl: (method.name_span.start(), method.name_span.end()),
        }
    }

    fn for_proc(qname: &str, proc_def: &ProcDef) -> Self {
        Self {
            name: qname.to_owned(),
            decl: (proc_def.name_span.start(), proc_def.name_span.end()),
        }
    }

    fn top_level() -> Self {
        Self {
            name: TOP_LEVEL_NAME.to_owned(),
            decl: (0, 0),
        }
    }
}

/// Display name of the synthetic item standing for a document's top-level
/// command stream.
const TOP_LEVEL_NAME: &str = "<top-level>";

/// Per-edge accumulator: the far end's item plus every call-site range
/// attributed to it, keyed by [`CallItemKey`].
type EdgeBuckets = std::collections::BTreeMap<CallItemKey, (CallHierarchyItem, Vec<LspRange>)>;

/// Find the proc a call-hierarchy item refers to.
///
/// Items carry only the *short* display name (`helper`), which is ambiguous
/// when a document defines same-named procs in different namespaces
/// (`::a::helper` / `::b::helper`).  Disambiguate first by the item's
/// `selection_range` — it is the proc name token's exact location, a stable
/// identity that round-trips through the LSP incoming/outgoing call requests —
/// and only fall back to display-name matching when no definition's name span
/// lines up (e.g. a synthetic or hand-built item).
fn find_proc_for_item<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    item: &CallHierarchyItem,
    line_index: &LineIndex,
    resolution: crate::definition::CallResolution<'_>,
) -> Option<(&'a String, &'a ProcDef)> {
    if let Some(hit) = analysis
        .all_procs
        .iter()
        .find(|&(_, p)| span_to_range(source, line_index, p.name_span) == item.selection_range)
    {
        return Some(hit);
    }
    // No declaration's name span lines up (a synthetic / hand-built item):
    // resolve namespace-aware from the item's own location rather than a
    // namespace-blind `name == item.name` scan, which could bind the item to a
    // same-named proc in an unrelated namespace.
    let cursor_off = crate::definition::byte_offset_at(
        line_index,
        source,
        item.selection_range.start_line,
        item.selection_range.start_character,
    );
    crate::definition::resolve_proc_target_at(analysis, source, cursor_off, &item.name, resolution)
}

/// Build a [`CallHierarchyItem`] for a class method.
fn item_for_method(
    source: &str,
    class_def: &ClassDef,
    method: &MethodDef,
    line_index: &LineIndex,
) -> CallHierarchyItem {
    let name_range = span_to_range(source, line_index, method.name_span);
    let body_range = span_to_range(source, line_index, method.body_span);
    let detail = Some(format!(
        "{} of {} ({} params)",
        method.kind,
        class_def.qualified_name,
        method.params.len(),
    ));
    let full_range = LspRange {
        start_line: name_range.start_line,
        start_character: name_range.start_character,
        end_line: body_range.end_line,
        end_character: body_range.end_character,
    };
    CallHierarchyItem {
        identity: None,
        name: method_item_name(class_def, method),
        detail,
        range: full_range,
        selection_range: name_range,
    }
}

/// `true` when `offset` falls within `span` (inclusive of both ends — a
/// cursor sitting right at either edge of a name token still counts).
fn span_contains_offset(span: tcl_lexer::Span, offset: u32) -> bool {
    span.start() <= offset && offset <= span.end()
}

/// Find the class + method whose body contains `cursor_offset`
/// and whose method name matches `word`.  Searches `methods`
/// then `class_methods` — except when `cursor_offset` sits inside a
/// `classmethod`'s own declaration or body, in which case `class_methods`
/// is tried first.
///
/// `my <word>` dispatch scope depends on *whose* body the cursor is
/// currently inside: a classmethod's body runs with `self` bound to the
/// class object (its own method table is `class_methods`); an ordinary
/// method's / constructor's / destructor's body runs with `self` bound to
/// the instance (`methods`) — the two tables are never merged (confirmed
/// against tclsh 9.0.4, see
/// [`tcl_compiler::analyser::diagnostics::var_command`]'s dispatch-scope
/// note). Preferring whichever table the surrounding body belongs to
/// resolves a name shared by both kinds (rare, but real) to the one
/// actually reachable from the cursor's own scope.
fn enclosing_class_method<'a>(
    analysis: &'a AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<(&'a ClassDef, &'a MethodDef)> {
    let class_def = analysis
        .all_classes
        .get(crate::definition::enclosing_class_at(
            analysis,
            cursor_offset,
        )?)?;
    let in_classmethod_territory = class_def.class_methods.values().any(|m| {
        span_contains_offset(m.name_span, cursor_offset)
            || span_contains_offset(m.body_span, cursor_offset)
    });
    if in_classmethod_territory && let Some(m) = class_def.class_methods.get(word) {
        return Some((class_def, m));
    }
    if let Some(m) = class_def.methods.get(word) {
        return Some((class_def, m));
    }
    if let Some(m) = class_def.class_methods.get(word) {
        return Some((class_def, m));
    }
    None
}

/// Resolve a call-hierarchy `item` back to its [`ClassDef`] + [`MethodDef`].
///
/// The item name is `<class-key>::<method>` — a construction; split it by
/// the construction-inverse rule so a colon-bearing class key (or method
/// name) survives.  A method and a classmethod sharing a name (rare,
/// but `TclOO` keeps them in independent tables, so it's legal) collide on
/// this name alone, so disambiguate first by the item's exact
/// `selection_range` — the declaration's name-token location, which
/// round-trips through every incoming/outgoing-calls request — mirroring
/// [`find_proc_for_item`]'s same-named-proc disambiguation.  Only falls back
/// to the methods-first default when neither declaration's range lines up
/// (a synthetic / hand-built item).
fn resolve_method_item<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    item: &CallHierarchyItem,
    line_index: &LineIndex,
) -> Option<(&'a ClassDef, &'a MethodDef)> {
    let (class_q, method_name) = tcl_syntax::naming::key_holder_and_tail(&item.name);
    if class_q.is_empty() && method_name.len() == item.name.len() {
        return None;
    }
    let class_def = analysis
        .all_classes
        .values()
        .find(|c| c.qualified_name == class_q)?;
    let by_range = [
        class_def.methods.get(method_name),
        class_def.class_methods.get(method_name),
    ]
    .into_iter()
    .flatten()
    .find(|m| span_to_range(source, line_index, m.name_span) == item.selection_range);
    let method = by_range
        .or_else(|| class_def.methods.get(method_name))
        .or_else(|| class_def.class_methods.get(method_name))?;
    Some((class_def, method))
}

/// Whether `my <word>` dispatch from a body of kind `caller_kind` can reach
/// a member of kind `target_kind`.  `self` is the class object inside a
/// `classmethod` body (its own method table is `class_methods`) and the
/// instance everywhere else — a `method` / `forward` / `constructor` /
/// `destructor` body all run with `self` bound to the instance (`methods`)
/// — so the two tables never merge (confirmed against tclsh 9.0.4, see
/// [`tcl_compiler::analyser::diagnostics::var_command`]'s dispatch-scope
/// note).
fn dispatch_reaches(caller_kind: &str, target_kind: &str) -> bool {
    (caller_kind == "classmethod") == (target_kind == "classmethod")
}

/// Conditional source calls in the same lexical method frame. Selected
/// helper traits exclude method dispatch without reparsing a written head.
fn segment_body_calls(
    source: &str,
    analysis: &AnalysisResult,
    body_span: tcl_lexer::Span,
) -> Vec<(String, tcl_lexer::Span)> {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let Some(frame) =
        crate::definition::source_frame_extent(analysis, body_span.start().saturating_add(1))
    else {
        return Vec::new();
    };
    let mut calls = Vec::new();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            let Some(head) = command.argv.first() else {
                return false;
            };
            if !span_contains(body_span, head.span)
                || crate::definition::source_frame_extent(analysis, head.span.start())
                    != Some(frame)
            {
                return false;
            }
            let is_helper =
                crate::original_invocation::source_registry_words(source, analysis, command)
                    .and_then(|words| {
                        words.with_source_schema(&current.context(), |schema| {
                            schema.semantics.traits.intersects(
                                tcl_registry::Traits::TCLOO_SELF_DISPATCH
                                    | tcl_registry::Traits::TCLOO_NEXT_CHAIN
                                    | tcl_registry::Traits::TCLOO_INTROSPECTION,
                            )
                        })
                    })
                    .unwrap_or(false);
            if !is_helper {
                calls.push((command.name().to_owned(), head.span));
            }
            false
        },
    );
    calls
}

/// Build a [`CallHierarchyItem`] for a given proc definition.
fn item_for_proc(
    source: &str,
    proc_def: &ProcDef,
    _qname: &str,
    line_index: &LineIndex,
) -> CallHierarchyItem {
    let name_range = span_to_range(source, line_index, proc_def.name_span);
    let body_range = span_to_range(source, line_index, proc_def.body_span);
    let detail = if proc_def.params.is_empty() {
        None
    } else {
        Some(format!("({} params)", proc_def.params.len()))
    };
    let full_range = LspRange {
        start_line: name_range.start_line,
        start_character: name_range.start_character,
        end_line: body_range.end_line,
        end_character: body_range.end_character,
    };
    CallHierarchyItem {
        identity: None,
        // Short display name (`helper`), not the qualified key (`::helper`) —
        // matches the editor's call-hierarchy UI.  The
        // incoming/outgoing lookups match this against both forms.
        name: proc_def.name.clone(),
        detail,
        range: full_range,
        selection_range: name_range,
    }
}

/// `true` when `inv_span` lies inside `proc_body_span`.  Used
/// to bucket each call site into its enclosing proc.
fn span_contains(outer: tcl_lexer::Span, inner: tcl_lexer::Span) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end()
}

/// Find the proc whose body span contains `inv_span` — i.e.
/// the proc the call site sits inside.  Returns `None` for
/// top-level call sites.
fn enclosing_proc<'a>(
    analysis: &'a AnalysisResult,
    inv_span: tcl_lexer::Span,
) -> Option<(&'a str, &'a ProcDef)> {
    let mut best: Option<(&'a str, &'a ProcDef)> = None;
    for (qname, proc_def) in &analysis.all_procs {
        if !span_contains(proc_def.body_span, inv_span) {
            continue;
        }
        // Pick the smallest enclosing body so nested procs
        // bucket calls into the inner proc rather than the
        // outer.
        let body_len = proc_def.body_span.end() - proc_def.body_span.start();
        let pick = match best {
            None => true,
            Some((_, prev)) => {
                let prev_len = prev.body_span.end() - prev.body_span.start();
                body_len < prev_len
            }
        };
        if pick {
            best = Some((qname.as_str(), proc_def));
        }
    }
    best
}

/// `true` when an execution site targets `proc_def`.
///
/// Retains the lookup-purpose distinction before delegating target identity
/// to [`crate::references::invocation_calls_proc`]. Introspection and
/// deferred registrations remain references without becoming calls. Actual
/// heads share the namespace and allocation checks used by references and
/// rename.
fn invocation_targets(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    proc_def: &ProcDef,
    qname: &str,
    source: &str,
) -> bool {
    crate::references::invocation_calls_proc(analysis, inv, qname, proc_def, source)
}

/// One incoming-call entry: the caller proc plus the spans at
/// which it calls the target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingCall {
    /// The proc that contains the call sites.
    pub from: CallHierarchyItem,
    /// Spans at which `from` calls the target proc.
    pub from_ranges: Vec<LspRange>,
}

/// One outgoing-call entry: the called proc plus the spans
/// from which it's called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingCall {
    /// The proc being called.
    pub to: CallHierarchyItem,
    /// Spans at which the source proc calls `to`.
    pub from_ranges: Vec<LspRange>,
}

/// A call site inside the queried item's body whose callee is
/// *not* defined in the current document.  The local
/// [`outgoing_calls`] pass can only resolve callees present in
/// `analysis.all_procs` / the enclosing class; cross-document
/// outgoing-call resolution feeds these unresolved heads to the
/// workspace index, which knows the sibling-file definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedOutgoingCall {
    /// Command head as written at the call site.
    pub name: String,
    /// Scope-resolved qualified name when the analyser inferred
    /// one for the call site, else `None`.
    pub resolved_qualified_name: Option<String>,
    /// Ranges (in the *current* document) of each call site,
    /// grouped under this callee head.
    pub from_ranges: Vec<LspRange>,
}

/// Collect the call sites inside the queried item's body whose
/// callee is not resolvable within the current document — the
/// raw material for cross-document outgoing-call edges.
///
/// Mirrors [`outgoing_calls`]'s body-scan but inverts the filter:
/// it keeps the invocations that *don't* match a local proc /
/// sibling method, grouped by call-site head, so the server can
/// resolve each head against the workspace index.  Builtins and
/// unknown commands are returned too (the index lookup discards
/// the ones that aren't user-defined elsewhere).
#[must_use]
pub fn unresolved_outgoing_calls(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<UnresolvedOutgoingCall> {
    unresolved_outgoing_calls_in_program(
        source,
        dialect,
        item,
        analysis,
        crate::definition::CallResolution::document_only(),
    )
}

/// [`unresolved_outgoing_calls`] with the caller's whole-program export view attached — the
/// entry point a host with a workspace index should call.
///
/// Which proc a call edge points at is a call resolution, so a `namespace
/// import -force` whose covering `namespace export` lives in another file
/// changes the hierarchy. Without the oracle the graph
/// would draw an edge to a definition go-to-definition refuses to open.
#[must_use]
pub fn unresolved_outgoing_calls_in_program(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<UnresolvedOutgoingCall> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }

    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }

    let line_index = LineIndex::new(source);
    if let Some((_, source_proc)) =
        find_proc_for_item(source, analysis, item, &line_index, resolution)
    {
        let mut by_head: std::collections::BTreeMap<String, (Option<String>, Vec<LspRange>)> =
            std::collections::BTreeMap::new();
        for inv in &analysis.command_invocations {
            if !inv.lookup.is_execution_site() || !span_contains(source_proc.body_span, inv.range) {
                continue;
            }
            // Skip call sites the local pass already resolves.
            if analysis
                .all_procs
                .iter()
                .any(|(qname, proc_def)| invocation_targets(analysis, inv, proc_def, qname, source))
            {
                continue;
            }
            let range = span_to_range(source, &line_index, inv.range);
            let entry = by_head
                .entry(inv.name.clone())
                .or_insert_with(|| (inv.resolved_qualified_name.clone(), Vec::new()));
            entry.1.push(range);
        }
        return by_head
            .into_iter()
            .map(
                |(name, (resolved_qualified_name, from_ranges))| UnresolvedOutgoingCall {
                    name,
                    resolved_qualified_name,
                    from_ranges,
                },
            )
            .collect();
    }
    unresolved_method_outgoing_calls(source, dialect, item, analysis, &line_index, resolution)
}

/// Method-body variant of [`unresolved_outgoing_calls`]: keeps
/// the call sites inside a class method that name neither a
/// local top-level proc nor a `TclOO` dispatch keyword.
///
/// A bare head matching a *sibling method* name is **not** excluded here —
/// unlike a proc, a method is never a bare-callable command (`greet` alone
/// errors "invalid command name"; only `my greet` dispatches), so a bare
/// `greet` site names no real target and is correctly surfaced as
/// unresolved, not silently treated as if it had resolved to the method.
fn unresolved_method_outgoing_calls(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<UnresolvedOutgoingCall> {
    let Some((class_def, source_method)) = resolve_method_item(source, analysis, item, line_index)
    else {
        return Vec::new();
    };
    let mut by_head: std::collections::BTreeMap<String, Vec<LspRange>> =
        std::collections::BTreeMap::new();
    for (head, span) in segment_body_calls(source, analysis, source_method.body_span) {
        // Local top-level proc?  Resolved from the class's namespace (a
        // method body's commands resolve there), with the deterministic
        // simple-name fallback — not a namespace-blind `any` scan.
        let class_ns = tcl_syntax::naming::key_holder_and_tail(&class_def.qualified_name).0;
        if crate::definition::resolve_called_proc(
            analysis,
            source,
            class_ns,
            &head,
            span.start(),
            resolution,
        )
        .is_some()
        {
            continue;
        }
        by_head
            .entry(head)
            .or_default()
            .push(span_to_range(source, line_index, span));
    }
    by_head
        .into_iter()
        .map(|(name, from_ranges)| UnresolvedOutgoingCall {
            name,
            resolved_qualified_name: None,
            from_ranges,
        })
        .collect()
}

/// Enumerate incoming calls for the proc identified by
/// `item.name`.  Walks every command invocation in
/// `analysis.command_invocations` that targets the proc and
/// groups by enclosing proc.  Top-level call sites are bucketed
/// under a synthetic `<top-level>` caller.
///
/// Returns an empty `Vec` when `item.name` doesn't match any
/// proc in `analysis.all_procs`.
#[must_use]
pub fn incoming_calls(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<IncomingCall> {
    incoming_calls_in_program(
        source,
        dialect,
        item,
        analysis,
        crate::definition::CallResolution::document_only(),
    )
}

/// [`incoming_calls`] with the caller's whole-program export view attached — the
/// entry point a host with a workspace index should call.
///
/// Which proc a call edge points at is a call resolution, so a `namespace
/// import -force` whose covering `namespace export` lives in another file
/// changes the hierarchy. Without the oracle the graph
/// would draw an edge to a definition go-to-definition refuses to open.
#[must_use]
pub fn incoming_calls_in_program(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<IncomingCall> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }

    if !analysis.allows_lexical_declaration_advice() {
        let Some(identity) = &item.identity else {
            return Vec::new();
        };
        return crate::original_call_hierarchy::incoming(
            identity,
            &[crate::original_call_hierarchy::OriginalCallDocument {
                uri: identity.uri(),
                source,
                analysis,
            }],
            None,
        )
        .into_iter()
        .map(|(_, call)| call)
        .collect();
    }

    let line_index = LineIndex::new(source);
    let Some((target_qname, target_proc)) =
        find_proc_for_item(source, analysis, item, &line_index, resolution)
    else {
        // Not a proc — try a class method.
        return method_incoming_calls(source, dialect, item, analysis, &line_index);
    };
    incoming_calls_for_target(
        source,
        analysis,
        &target_proc.name,
        target_qname,
        Some(target_proc.name_span),
    )
}

/// Enumerate incoming calls to a proc identified *externally*
/// by `(target_simple, target_qualified)`, within the document
/// described by `source` / `analysis`.  Unlike [`incoming_calls`],
/// the target needn't be defined in this document — used by the
/// server to gather cross-document callers (one call per
/// indexed document).  `target_name_span`, when `Some`, skips
/// an invocation that overlaps the proc's own declaration in
/// *this* document (avoids self-linking); pass `None` for
/// documents that don't define the proc.
#[must_use]
pub fn incoming_calls_for_target(
    source: &str,
    analysis: &AnalysisResult,
    target_simple: &str,
    target_qualified: &str,
    target_name_span: Option<tcl_lexer::Span>,
) -> Vec<IncomingCall> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }

    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }

    let line_index = LineIndex::new(source);
    let mut by_caller: std::collections::BTreeMap<String, (CallHierarchyItem, Vec<LspRange>)> =
        std::collections::BTreeMap::new();
    for inv in &analysis.command_invocations {
        if !inv.lookup.is_execution_site() {
            continue;
        }
        // Delegate to the shared matching rule (`invocation_calls_proc`
        // takes a `ProcDef`; this caller may not have one — cross-document
        // callers are gathered from a target the calling document never
        // defines — so route through the string-keyed core directly). This
        // adds the namespace gate a bare simple-name match needs: without
        // it, a bare call in one namespace falsely credited *any* same-named
        // proc anywhere as an incoming caller.
        if !crate::references::invocation_calls_named(
            analysis,
            inv,
            target_qualified,
            target_simple,
            target_qualified,
            source,
        ) {
            continue;
        }
        // Skip the proc's own declaration site in this document.
        if let Some(decl) = target_name_span
            && (span_contains(decl, inv.range) || span_contains(inv.range, decl))
        {
            continue;
        }
        let inv_range = span_to_range(source, &line_index, inv.range);
        let caller_key = enclosing_proc(analysis, inv.range)
            .map_or_else(|| TOP_LEVEL_NAME.to_owned(), |(qn, _)| qn.to_owned());
        let entry = by_caller.entry(caller_key.clone()).or_insert_with(|| {
            let caller_item = if caller_key == TOP_LEVEL_NAME {
                top_level_item()
            } else {
                let proc = &analysis.all_procs[&caller_key];
                item_for_proc(source, proc, &caller_key, &line_index)
            };
            (caller_item, Vec::new())
        });
        entry.1.push(inv_range);
    }
    by_caller
        .into_values()
        .map(|(from, from_ranges)| IncomingCall { from, from_ranges })
        .collect()
}

/// Enumerate outgoing calls from the proc identified by
/// `item.name`.  Walks every command invocation whose span
/// sits inside the proc's body span, then resolves each to its
/// target proc (when the invocation names a user proc), and adds every bare
/// `ClassName <classmethod>` dispatch in the same body.  Calls
/// to built-in commands are dropped (they have no
/// [`CallHierarchyItem`] to point to).  Multiple call sites to
/// the same target group together.
///
/// The classmethod half matters for symmetry: a class command is an
/// ordinary global command, so a proc body may dispatch one, and
/// [`add_classmethod_incoming`] lists that proc under the classmethod's
/// Incoming Calls.  Without the matching collection here the edge would
/// exist in one direction only.
#[must_use]
pub fn outgoing_calls(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<OutgoingCall> {
    outgoing_calls_in_program(
        source,
        dialect,
        item,
        analysis,
        crate::definition::CallResolution::document_only(),
    )
}

/// [`outgoing_calls`] with the caller's whole-program export view attached — the
/// entry point a host with a workspace index should call.
///
/// Which proc a call edge points at is a call resolution, so a `namespace
/// import -force` whose covering `namespace export` lives in another file
/// changes the hierarchy. Without the oracle the graph
/// would draw an edge to a definition go-to-definition refuses to open.
#[must_use]
pub fn outgoing_calls_in_program(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<OutgoingCall> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }

    if !analysis.allows_lexical_declaration_advice() {
        let Some(identity) = &item.identity else {
            return Vec::new();
        };
        return crate::original_call_hierarchy::outgoing(
            identity,
            &[crate::original_call_hierarchy::OriginalCallDocument {
                uri: identity.uri(),
                source,
                analysis,
            }],
            None,
        )
        .into_iter()
        .map(|(_, call)| call)
        .collect();
    }

    let line_index = LineIndex::new(source);
    let Some((_, source_proc)) =
        find_proc_for_item(source, analysis, item, &line_index, resolution)
    else {
        // Not a proc — try a class method.
        return method_outgoing_calls(source, dialect, item, analysis, &line_index, resolution);
    };
    let mut by_target: EdgeBuckets = EdgeBuckets::new();
    for inv in &analysis.command_invocations {
        if !span_contains(source_proc.body_span, inv.range) {
            continue;
        }
        // Find the user-proc this invocation targets, if any.
        for (qname, proc_def) in &analysis.all_procs {
            if invocation_targets(analysis, inv, proc_def, qname, source) {
                let inv_range = span_to_range(source, &line_index, inv.range);
                let entry = by_target
                    .entry(CallItemKey::for_proc(qname, proc_def))
                    .or_insert_with(|| {
                        (
                            item_for_proc(source, proc_def, qname, &line_index),
                            Vec::new(),
                        )
                    });
                entry.1.push(inv_range);
                break;
            }
        }
    }
    add_classmethod_outgoing(
        source,
        dialect,
        analysis,
        source_proc.body_span,
        &line_index,
        &mut by_target,
    );
    by_target
        .into_values()
        .map(|(to, from_ranges)| OutgoingCall { to, from_ranges })
        .collect()
}

/// Incoming calls for a class method — every `my <method>` dispatch site
/// inside any sibling method body, grouped by the enclosing method, plus
/// (for a `classmethod`) every bare `ClassName <method>` dispatch anywhere
/// in the document, grouped by whichever body it sits in.
///
/// Intra-class dispatch of a `TclOO` method is `my <method>`, never a bare
/// `<method>` call (a method is not a command in the body's namespace — a
/// bare head errors "invalid command name" at runtime), so this matches
/// through [`crate::references::scan_my_method_sites`] — the same
/// control-flow-recursing matcher Find-References / rename / the code lens
/// use — rather than comparing a bare head.
fn method_incoming_calls(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<IncomingCall> {
    let Some((class_def, target_method)) = resolve_method_item(source, analysis, item, line_index)
    else {
        return Vec::new();
    };
    let mut by_caller: EdgeBuckets = EdgeBuckets::new();
    for caller in class_methods_iter(class_def) {
        if !dispatch_reaches(&caller.kind, &target_method.kind) {
            continue;
        }
        let spans = crate::references::scan_method_sites(
            source,
            analysis,
            &[caller.body_span],
            &target_method.name,
            Some(target_method.name_span),
            target_method.visibility == "public",
        );
        if spans.is_empty() {
            continue;
        }
        let entry = by_caller
            .entry(CallItemKey::for_method(class_def, caller))
            .or_insert_with(|| {
                (
                    item_for_method(source, class_def, caller, line_index),
                    Vec::new(),
                )
            });
        entry.1.extend(
            spans
                .into_iter()
                .map(|s| span_to_range(source, line_index, s)),
        );
    }
    add_classmethod_incoming(
        source,
        dialect,
        analysis,
        class_def,
        target_method,
        line_index,
        &mut by_caller,
    );
    by_caller
        .into_values()
        .map(|(from, from_ranges)| IncomingCall { from, from_ranges })
        .collect()
}

/// Incoming instance-method calls found in one known receiver class.
///
/// Hosts use this for a sibling document whose class inherits the target's
/// provider from the workspace MRO: the document has the caller bodies, while
/// the provider lives elsewhere. `external_callback_allowed` comes from the
/// workspace visibility chain; internal `my` callbacks remain eligible.
#[must_use]
pub fn incoming_instance_method_calls_in_class(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    receiver_class: &str,
    method: &str,
    external_callback_allowed: bool,
) -> Vec<IncomingCall> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }

    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }

    let Some(class_def) = analysis.all_classes.get(receiver_class) else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    instance_method_frames_iter(class_def)
        .filter_map(|caller| {
            let spans = crate::references::scan_method_sites(
                source,
                analysis,
                &[caller.body_span],
                method,
                None,
                external_callback_allowed,
            );
            (!spans.is_empty()).then(|| IncomingCall {
                from: item_for_method(source, class_def, caller, &line_index),
                from_ranges: spans
                    .into_iter()
                    .map(|span| span_to_range(source, &line_index, span))
                    .collect(),
            })
        })
        .collect()
}

/// Add the bare `ClassName <classmethod>` dispatch sites of `target_method`
/// to `by_caller`, each attributed to the innermost body it sits in.  A
/// no-op unless `target_method` really is a `classmethod`.
///
/// The sites come from [`crate::references::find_obj_method_call_sites`] —
/// the same scanner Find-References / rename / the code lens use, which also
/// carries the registry-driven definer-family rule that keeps [incr Tcl]'s
/// `Factory::make` class-proc shape (and its unrelated two-word
/// instance-creation syntax) out of this — rather than a head-word compare
/// re-derived here.
///
/// No [`dispatch_reaches`] gate: that rule is about `my`, whose scope
/// depends on the calling body.  A class command is an ordinary global
/// command, so `Factory make` reaches the classmethod from a classmethod
/// body, an instance-method body, a proc, or the top level alike
/// (tclsh9.0-verified).
fn add_classmethod_incoming(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_def: &ClassDef,
    target_method: &MethodDef,
    line_index: &LineIndex,
    by_caller: &mut EdgeBuckets,
) {
    if target_method.kind != "classmethod" {
        return;
    }
    for span in crate::references::find_obj_method_call_sites(
        source,
        dialect,
        analysis,
        &class_def.qualified_name,
        &target_method.name,
        true,
    ) {
        if span_contains(target_method.name_span, span) {
            continue;
        }
        let (key, caller_item) = enclosing_dispatch_caller(source, analysis, span, line_index);
        let entry = by_caller
            .entry(key)
            .or_insert_with(|| (caller_item, Vec::new()));
        entry.1.push(span_to_range(source, line_index, span));
    }
}

/// The caller a bare class-command dispatch site is attributed to: the
/// innermost class-member body containing it, else the innermost proc body,
/// else the top level.
///
/// This one list mixes both kinds of caller, and a method caller has no
/// short name that could ever be unambiguous — `::C::make` is the only
/// thing to call it.  So the proc callers alongside it are named by their
/// qualified name too (`::util::helper`, not `helper`), rather than reading
/// as a different kind of label in the same list.  Elsewhere a proc item
/// keeps the short display name the
/// editor's call-hierarchy UI expects.
fn enclosing_dispatch_caller(
    source: &str,
    analysis: &AnalysisResult,
    span: tcl_lexer::Span,
    line_index: &LineIndex,
) -> (CallItemKey, CallHierarchyItem) {
    // Ranked by (body length, body start) so the innermost body wins and ties
    // never depend on `all_classes`' hash iteration order.
    let mut best: Option<((u32, u32), CallItemKey, CallHierarchyItem)> = None;
    let mut consider = |body: tcl_lexer::Span, key: CallItemKey, item: CallHierarchyItem| {
        if !span_contains(body, span) {
            return;
        }
        let rank = (body.end() - body.start(), body.start());
        if best
            .as_ref()
            .is_none_or(|(best_rank, _, _)| rank < *best_rank)
        {
            best = Some((rank, key, item));
        }
    };
    for class_def in analysis.all_classes.values() {
        for member in class_methods_iter(class_def) {
            consider(
                member.body_span,
                CallItemKey::for_method(class_def, member),
                item_for_method(source, class_def, member, line_index),
            );
        }
    }
    for (qname, proc_def) in &analysis.all_procs {
        let mut item = item_for_proc(source, proc_def, qname, line_index);
        item.name.clone_from(qname);
        consider(
            proc_def.body_span,
            CallItemKey::for_proc(qname, proc_def),
            item,
        );
    }
    best.map_or_else(
        || (CallItemKey::top_level(), top_level_item()),
        |(_, key, item)| (key, item),
    )
}

/// The synthetic item standing for the document's top-level command stream.
fn top_level_item() -> CallHierarchyItem {
    let origin = LspRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 0,
    };
    CallHierarchyItem {
        identity: None,
        name: TOP_LEVEL_NAME.to_owned(),
        detail: None,
        range: origin,
        selection_range: origin,
    }
}

/// Outgoing calls from a class method — every `my <method>` dispatch site
/// inside the method's body that names a sibling method (→ method item),
/// every bare `ClassName <classmethod>` dispatch in it (→ that
/// classmethod's item), and every bare-headed call to a
/// top-level user proc (→ proc item).
///
/// The sibling-method half matches through
/// [`crate::references::scan_my_method_sites`] (the same matcher
/// Find-References / rename / the code lens use): a bare `<method>` call is
/// never a valid `TclOO` dispatch (it
/// errors "invalid command name" at runtime), so bare-head comparison
/// against a sibling method name would never match real code.  The proc
/// half keeps bare-head matching — an ordinary proc call genuinely is
/// bare-headed — but skips `my` / `next` / `nextto` heads so a `TclOO`
/// dispatch keyword is never misread as a proc-call attempt.
fn method_outgoing_calls(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<OutgoingCall> {
    let Some((class_def, source_method)) = resolve_method_item(source, analysis, item, line_index)
    else {
        return Vec::new();
    };
    let mut by_target: EdgeBuckets = EdgeBuckets::new();
    for callee in class_methods_iter(class_def) {
        if !dispatch_reaches(&source_method.kind, &callee.kind) {
            continue;
        }
        let spans = crate::references::scan_method_sites(
            source,
            analysis,
            &[source_method.body_span],
            &callee.name,
            None,
            callee.visibility == "public",
        );
        if spans.is_empty() {
            continue;
        }
        let entry = by_target
            .entry(CallItemKey::for_method(class_def, callee))
            .or_insert_with(|| {
                (
                    item_for_method(source, class_def, callee, line_index),
                    Vec::new(),
                )
            });
        entry.1.extend(
            spans
                .into_iter()
                .map(|s| span_to_range(source, line_index, s)),
        );
    }
    add_classmethod_outgoing(
        source,
        dialect,
        analysis,
        source_method.body_span,
        line_index,
        &mut by_target,
    );
    let class_ns = tcl_syntax::naming::key_holder_and_tail(&class_def.qualified_name).0;
    for (head, span) in segment_body_calls(source, analysis, source_method.body_span) {
        // Top-level user proc?  Resolved from the class's namespace (a
        // method body's commands resolve there) — a deterministic
        // simple-name fallback, never a namespace-blind `p.name == head`
        // first-hit scan that could edge the hierarchy to an arbitrary
        // same-named proc in an unrelated namespace.
        if let Some(proc_def) = crate::definition::resolve_called_proc(
            analysis,
            source,
            class_ns,
            &head,
            span.start(),
            resolution,
        ) {
            let qname = &proc_def.qualified_name;
            let entry = by_target
                .entry(CallItemKey::for_proc(qname, proc_def))
                .or_insert_with(|| {
                    (
                        item_for_proc(source, proc_def, qname, line_index),
                        Vec::new(),
                    )
                });
            entry.1.push(span_to_range(source, line_index, span));
        }
    }
    by_target
        .into_values()
        .map(|(to, from_ranges)| OutgoingCall { to, from_ranges })
        .collect()
}

/// Add every bare `ClassName <classmethod>` dispatch inside `body` to
/// `by_target`, keyed by the classmethod it names.
///
/// Every class the document declares is a candidate, not just the calling
/// method's own: a class command is global, so an instance method of one
/// class may perfectly well dispatch another class's classmethod
/// (tclsh9.0-verified).  Sites come from
/// [`crate::references::find_obj_method_call_sites`], the shared scanner —
/// so the [incr Tcl] definer-family exclusion and the inheriting-subclass
/// dispatch heads it already knows about apply here too — filtered to the
/// calling body's span.
fn add_classmethod_outgoing(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    body: tcl_lexer::Span,
    line_index: &LineIndex,
    by_target: &mut EdgeBuckets,
) {
    for class_def in analysis.all_classes.values() {
        for callee in class_def.class_methods.values() {
            let spans: Vec<tcl_lexer::Span> = crate::references::find_obj_method_call_sites(
                source,
                dialect,
                analysis,
                &class_def.qualified_name,
                &callee.name,
                true,
            )
            .into_iter()
            .filter(|span| span_contains(body, *span))
            .collect();
            if spans.is_empty() {
                continue;
            }
            let entry = by_target
                .entry(CallItemKey::for_method(class_def, callee))
                .or_insert_with(|| {
                    (
                        item_for_method(source, class_def, callee, line_index),
                        Vec::new(),
                    )
                });
            entry.1.extend(
                spans
                    .into_iter()
                    .map(|s| span_to_range(source, line_index, s)),
            );
        }
    }
}

/// Iterate every method + classmethod of a class (the bodies
/// that can host intra-class method calls).
fn class_methods_iter(class_def: &ClassDef) -> impl Iterator<Item = &MethodDef> {
    class_def
        .methods
        .values()
        .chain(class_def.class_methods.values())
}

/// Every instance-side execution frame that can contain `my` dispatches.
/// Constructors and the destructor share the same object dispatch context as
/// ordinary methods and therefore participate in cross-document incoming
/// hierarchy edges too.
fn instance_method_frames_iter(class_def: &ClassDef) -> impl Iterator<Item = &MethodDef> {
    class_def
        .methods
        .values()
        .chain(class_def.constructors.iter())
        .chain(class_def.destructor.iter())
}

fn span_to_range(source: &str, line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    LspRange {
        start_line: start.line,
        start_character: start.character.get(),
        end_line: end.line,
        end_character: end.character.get(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn original_hierarchy_preparation_keeps_variable_cursors_and_method_declarations_separate() {
        // Implementation contract: naming.consumer.original-call-hierarchy
        // docs/design/analysis/name-resolution-proofs/original-call-hierarchy.md
        let source = "oo::class create C {method x {} {set x 1; puts $x}}";
        let mut analysis = analyse(source);
        let variable = u32::try_from(source.find("$x").unwrap() + 1).unwrap();
        assert!(!analysis.allows_lexical_declaration_advice());
        assert!(crate::definition::original_variable_cursor_retained(
            source, &analysis, 0, variable
        ));
        assert!(
            analysis
                .all_classes
                .values()
                .any(|class| class.methods.contains_key("x"))
        );
        assert!(prepare(source, 0, variable, &analysis).is_empty());
        let declaration =
            u32::try_from(source.find("method x").unwrap() + "method ".len()).unwrap();
        analysis.all_classes.clear();
        analysis.global_scope.classes.clear();
        let items = prepare(source, 0, declaration, &analysis);
        assert_eq!(items.len(), 1);
        let identity = items[0]
            .identity
            .as_ref()
            .expect("genuine source declaration identity");
        assert_eq!(identity.span().start(), declaration);
        assert!(matches!(
            identity.role(),
            crate::original_declaration::OriginalDeclarationRole::Method(_)
        ));
        assert!(prepare(&format!("#{source}"), 0, declaration, &analysis).is_empty());
    }

    #[test]
    fn prepare_resolves_proc_at_cursor() {
        let src = "proc greet {} {}\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "greet");
    }

    #[test]
    fn prepare_returns_empty_for_unknown_word() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(prepare(src, 0, 6, &analysis).is_empty());
    }

    #[test]
    fn same_named_procs_in_different_namespaces_disambiguate_by_span() {
        // `::a::helper` and `::b::helper` share the short display name
        // `helper`; the call-hierarchy item must resolve to the definition at
        // its own `selectionRange`, not whichever `all_procs` entry hashes
        // first. Each helper calls a distinct callee so the wrong resolution
        // is observable.
        let src = "proc ::a::helper {} { aCallee }\n\
                   proc ::b::helper {} { bCallee }\n\
                   proc aCallee {} {}\n\
                   proc bCallee {} {}\n";
        let analysis = analyse(src);
        // Cursor on the `helper` of the second (`::b::helper`) definition —
        // line 1, inside `helper` (after the `proc ::b::` prefix, 10 chars).
        let items = prepare(src, 1, 12, &analysis);
        assert_eq!(items.len(), 1, "{items:?}");
        let outgoing = outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &items[0],
            &analysis,
        );
        let callees: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert!(
            callees.contains(&"bCallee"),
            "b::helper must resolve to its own body (bCallee); got {callees:?}"
        );
        assert!(
            !callees.contains(&"aCallee"),
            "b::helper must not pick up a::helper's callee; got {callees:?}"
        );
    }

    // incoming + outgoing calls

    #[test]
    fn incoming_calls_from_other_procs() {
        // `caller` calls `target`; `target` is being asked
        // for its incoming calls.
        let src = "proc target {} {}\nproc caller {} { target }\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        let target = &items[0];
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            target,
            &analysis,
        );
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "caller");
        assert_eq!(incoming[0].from_ranges.len(), 1);
    }

    #[test]
    fn consumed_command_names_remain_references_without_becoming_calls() {
        let source =
            "proc target {} {return yes}\nproc caller {} {info body target; target}\ncaller\n";
        let analysis = analyse(source);
        let dialect =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let target = prepare(source, 0, 6, &analysis).remove(0);
        let caller = prepare(source, 1, 6, &analysis).remove(0);
        let references = analysis
            .command_invocations
            .iter()
            .filter(|invocation| invocation.name == "target")
            .collect::<Vec<_>>();
        assert_eq!(references.len(), 2);
        assert!(references.iter().any(|invocation| matches!(
            invocation.lookup,
            tcl_compiler::signature_scan::types::SignatureCommandLookup::ConsumedName { .. }
        )));
        let incoming = incoming_calls(source, dialect, &target, &analysis);
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].from_ranges.len(), 1);
        let outgoing = outgoing_calls(source, dialect, &caller, &analysis);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].to.name, "target");
        assert_eq!(outgoing[0].from_ranges.len(), 1);
    }

    #[test]
    fn deferred_command_prefixes_do_not_create_incoming_call_edges() {
        let source = "proc target args {}\nproc registrar {} {trace add command target delete target}\nregistrar\n";
        let analysis = analyse(source);
        let target = prepare(source, 0, 6, &analysis).remove(0);
        assert!(analysis.command_invocations.iter().any(|invocation| {
            invocation.name == "target"
                && invocation.lookup
                    == tcl_compiler::signature_scan::types::SignatureCommandLookup::DeferredReference
        }));
        let incoming = incoming_calls(
            source,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &target,
            &analysis,
        );
        assert!(incoming.is_empty(), "{incoming:?}");
    }

    #[test]
    fn incoming_calls_group_by_caller() {
        // Two call sites from the same caller bucket together
        // into a single IncomingCall entry.
        let src = "proc target {} {}\nproc caller {} { target\n target }\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        let target = &items[0];
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            target,
            &analysis,
        );
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from_ranges.len(), 2);
    }

    #[test]
    fn incoming_calls_top_level_bucketed_as_top_level() {
        let src = "proc target {} {}\ntarget\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        let target = &items[0];
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            target,
            &analysis,
        );
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "<top-level>");
    }

    #[test]
    fn outgoing_calls_from_proc_body() {
        // `caller` calls `target` and `other`; outgoing-calls
        // for `caller` should include both.
        let src = "proc target {} {}\nproc other {} {}\nproc caller {} { target\n other }\n";
        let analysis = analyse(src);
        let items = prepare(src, 2, 6, &analysis);
        assert_eq!(items[0].name, "caller");
        let outgoing = outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        let target_names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert!(target_names.contains(&"target"), "{outgoing:?}");
        assert!(target_names.contains(&"other"), "{outgoing:?}");
    }

    #[test]
    fn outgoing_calls_skip_builtins() {
        // `caller` calls `puts` (built-in) and `target`
        // (user proc).  Only `target` should appear in the
        // outgoing list.
        let src = "proc target {} {}\nproc caller {} { puts hi\n target }\n";
        let analysis = analyse(src);
        let items = prepare(src, 1, 6, &analysis);
        assert_eq!(items[0].name, "caller");
        let outgoing = outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert_eq!(names, vec!["target"], "{outgoing:?}");
    }

    #[test]
    fn outgoing_calls_empty_for_unknown_proc() {
        let src = "proc greet {} {}\n";
        let analysis = analyse(src);
        let bogus = CallHierarchyItem {
            identity: None,
            name: "::not_a_real_proc".to_string(),
            detail: None,
            range: LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
            selection_range: LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
        };
        assert!(
            outgoing_calls(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                &bogus,
                &analysis
            )
            .is_empty()
        );
    }

    #[test]
    fn unresolved_outgoing_calls_lists_non_local_heads() {
        // `caller` calls `local` (defined here), `sibling` (would
        // live in another file) and `puts` (builtin).  Only the
        // heads not resolvable in this document come back; the
        // server filters those against the workspace index.
        let src = "proc local {} {}\nproc caller {} { local\n sibling\n puts hi }\n";
        let analysis = analyse(src);
        let items = prepare(src, 1, 6, &analysis);
        assert_eq!(items[0].name, "caller");
        let unresolved = unresolved_outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        let names: Vec<&str> = unresolved.iter().map(|u| u.name.as_str()).collect();
        assert!(names.contains(&"sibling"), "{unresolved:?}");
        assert!(names.contains(&"puts"), "{unresolved:?}");
        assert!(
            !names.contains(&"local"),
            "local resolves in-document: {unresolved:?}"
        );
    }

    #[test]
    fn unresolved_outgoing_calls_groups_repeated_heads() {
        let src = "proc caller {} { sibling\n sibling }\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        assert_eq!(items[0].name, "caller");
        let unresolved = unresolved_outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        let sibling = unresolved
            .iter()
            .find(|u| u.name == "sibling")
            .expect("sibling head present");
        assert_eq!(sibling.from_ranges.len(), 2, "{unresolved:?}");
    }

    // class methods

    #[test]
    fn prepare_resolves_method_at_cursor() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `greet` declaration (line 1, col 11).
        let items = prepare(src, 1, 11, &analysis);
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].name, "::C::greet");
    }

    /// Regression: a bare `greet` call (no `my`) is not valid `TclOO`
    /// dispatch — it errors "invalid command name" at runtime (confirmed
    /// against tclsh 9.0.4 elsewhere in this crate, e.g.
    /// `references::tests::fp_bare_head_is_not_a_call`) — so intra-class
    /// incoming/outgoing method calls must match `my <method>`, never a
    /// bare head.
    #[test]
    fn incoming_calls_for_method_grouped_by_caller_method() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        let items = prepare(src, 1, 11, &analysis);
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        // One caller method (`twice`) with two call ranges.
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "::C::twice");
        assert_eq!(incoming[0].from_ranges.len(), 2, "{incoming:?}");
    }

    #[test]
    fn hierarchy_follows_registry_wrapped_my_callback() {
        let src = "oo::class create C {\n    method read {} {}\n    unexport read\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my read]]\n    }\n}\n";
        let analysis = analyse(src);
        let items = prepare(src, 1, 11, &analysis);
        let dialect =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let incoming = incoming_calls(src, dialect, &items[0], &analysis);
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "::C::wire", "{incoming:?}");
        assert_eq!(incoming[0].from_ranges.len(), 1, "{incoming:?}");

        let wire = prepare(src, 3, 11, &analysis);
        let outgoing = outgoing_calls(src, dialect, &wire[0], &analysis);
        assert_eq!(outgoing.len(), 1, "{outgoing:?}");
        assert_eq!(outgoing[0].to.name, "::C::read", "{outgoing:?}");
        assert_eq!(outgoing[0].from_ranges.len(), 1, "{outgoing:?}");
    }

    #[test]
    fn hierarchy_follows_one_hop_stored_my_callback() {
        let src = "oo::class create C {\n    method tick {} {}\n    method wire {} {\n        set cb [list my tick]\n        after 0 $cb\n    }\n}\n";
        let analysis = analyse(src);
        let dialect =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let tick = prepare(src, 1, 11, &analysis);
        let incoming = incoming_calls(src, dialect, &tick[0], &analysis);
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "::C::wire", "{incoming:?}");
        assert_eq!(incoming[0].from_ranges.len(), 1, "{incoming:?}");

        let wire = prepare(src, 2, 11, &analysis);
        let outgoing = outgoing_calls(src, dialect, &wire[0], &analysis);
        assert_eq!(outgoing.len(), 1, "{outgoing:?}");
        assert_eq!(outgoing[0].to.name, "::C::tick", "{outgoing:?}");
        assert_eq!(outgoing[0].from_ranges.len(), 1, "{outgoing:?}");
    }

    #[test]
    fn cross_document_instance_hierarchy_includes_constructor_and_destructor_frames() {
        let src = "oo::class create Child {\n    constructor {} { my read }\n    destructor { my read }\n}\n";
        let analysis = analyse(src);
        let mut incoming = incoming_instance_method_calls_in_class(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            "::Child",
            "read",
            true,
        );
        incoming.sort_by(|a, b| a.from.name.cmp(&b.from.name));
        let callers: Vec<_> = incoming
            .iter()
            .map(|call| (call.from.name.as_str(), call.from_ranges.len()))
            .collect();
        assert_eq!(
            callers,
            vec![("::Child::<constructor>", 1), ("::Child::<destructor>", 1)],
            "cross-document callers must cover every instance execution frame"
        );
    }

    /// FP guard: a bare `greet` call inside a sibling method is *not* a
    /// `TclOO` dispatch (unlike a plain proc, a method is never a
    /// bare-callable command), so it must not be counted as an incoming
    /// call — the previous bare-head matcher would have (wrongly) found
    /// this, while missing the one real (`my`-prefixed) shape entirely.
    #[test]
    fn incoming_calls_excludes_bare_head_that_is_not_valid_tcl() {
        let src =
            "oo::class create C {\n    method greet {} {}\n    method twice {} { greet }\n}\n";
        let analysis = analyse(src);
        let items = prepare(src, 1, 11, &analysis);
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        assert!(incoming.is_empty(), "{incoming:?}");
    }

    /// Regression: `prepare()` must resolve the *classmethod* when the
    /// cursor sits on its own declaration, even though a same-named
    /// instance method exists in the same class — the previous
    /// methods-first lookup ignored where the cursor actually was and would
    /// have silently returned the instance method's item instead.
    #[test]
    fn prepare_resolves_classmethod_over_same_named_method_by_cursor() {
        let src = "oo::class create C {\n    method greet {} {}\n    classmethod greet {} {}\n}\n";
        let analysis = analyse(src);
        // Cursor on the *classmethod* declaration (line 2, col 16).
        let items = prepare(src, 2, 16, &analysis);
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(
            items[0].detail.as_deref(),
            Some("classmethod of ::C (0 params)"),
            "{items:?}"
        );
    }

    /// Regression: a `method` and a `classmethod` sharing a name (rare, but
    /// `TclOO` keeps them in independent tables, so it's legal) must not
    /// collide. `my greet` dispatched from an ordinary method's body can
    /// only reach the *instance* method `greet` — never the classmethod of
    /// the same name, since `self` inside an instance method is the
    /// instance, whose method table never includes classmethods.
    /// Matching both by name alone double-counts the single real call site
    /// under one item key.
    #[test]
    fn outgoing_calls_does_not_conflate_method_and_classmethod_sharing_a_name() {
        let src = "oo::class create C {\n    method greet {} {}\n    classmethod greet {} {}\n    method twice {} { my greet }\n}\n";
        let analysis = analyse(src);
        let items = prepare(src, 3, 11, &analysis);
        assert_eq!(items[0].name, "::C::twice");
        let outgoing = outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        assert_eq!(outgoing.len(), 1, "{outgoing:?}");
        assert_eq!(
            outgoing[0].from_ranges.len(),
            1,
            "must not double-count the single `my greet` site: {outgoing:?}"
        );
        // The resolved item is the instance method (line 1), not the
        // classmethod (line 2).
        assert_eq!(outgoing[0].to.selection_range.start_line, 1, "{outgoing:?}");
    }

    #[test]
    fn outgoing_calls_from_method_to_sibling_method() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        // Resolve the `twice` method (line 2, col 11).
        let items = prepare(src, 2, 11, &analysis);
        assert_eq!(items[0].name, "::C::twice");
        let outgoing = outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert_eq!(names, vec!["::C::greet"], "{outgoing:?}");
        // Two call sites collapse into one target entry.
        assert_eq!(outgoing[0].from_ranges.len(), 2, "{outgoing:?}");
    }

    /// FN→TP: a `my method` dispatch nested
    /// inside `if` / `foreach` / `switch` control flow is an outgoing call
    /// too — `scan_my_method_sites` recurses generically via the
    /// registry's `Plain`-`BodyKind` body roles, so call hierarchy inherits
    /// the same coverage Find-References / rename / the code lens gained.
    #[test]
    fn outgoing_calls_from_method_nested_in_control_flow() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} {\n        if {1} {\n            switch -- 1 {\n                default {\n                    my greet\n                }\n            }\n        }\n    }\n}\n";
        let analysis = analyse(src);
        let items = prepare(src, 2, 11, &analysis);
        assert_eq!(items[0].name, "::C::twice");
        let outgoing = outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert_eq!(names, vec!["::C::greet"], "{outgoing:?}");
    }

    /// FN→TP: a `my method` dispatch nested
    /// inside control flow is an *incoming* call edge too, mirroring
    /// `outgoing_calls_from_method_nested_in_control_flow`.
    #[test]
    fn incoming_calls_for_method_nested_in_control_flow() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} {\n        if {1} {\n            switch -- 1 {\n                default {\n                    my greet\n                }\n            }\n        }\n    }\n}\n";
        let analysis = analyse(src);
        let items = prepare(src, 1, 11, &analysis);
        assert_eq!(items[0].name, "::C::greet");
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "::C::twice", "{incoming:?}");
    }

    #[test]
    fn outgoing_calls_from_method_to_top_level_proc() {
        let src = "proc helper {} {}\noo::class create C {\n    method use {} { helper }\n}\n";
        let analysis = analyse(src);
        // Resolve the `use` method (line 2, col 11).
        let items = prepare(src, 2, 11, &analysis);
        assert_eq!(items[0].name, "::C::use");
        let outgoing = outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert_eq!(names, vec!["helper"], "{outgoing:?}");
    }

    /// Regression: a `my <method>` dispatch inside a method body must not
    /// leak "my" itself into the unresolved-outgoing-calls list — `my` is
    /// a `TclOO` dispatch keyword, not an unresolved command reference.
    #[test]
    fn unresolved_method_outgoing_calls_excludes_my_keyword() {
        let src =
            "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet }\n}\n";
        let analysis = analyse(src);
        let items = prepare(src, 2, 11, &analysis);
        assert_eq!(items[0].name, "::C::twice");
        let unresolved = unresolved_outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        assert!(unresolved.iter().all(|u| u.name != "my"), "{unresolved:?}");
    }

    // workspace-index: cross-document incoming calls

    #[test]
    fn incoming_calls_for_target_finds_callers_in_other_doc() {
        // A consumer document that *doesn't* define `helper`
        // but calls it from inside `caller` and at top level.
        let src = "proc caller {} { helper }\nhelper\n";
        let analysis = analyse(src);
        let calls = incoming_calls_for_target(src, &analysis, "helper", "helper", None);
        // Callers: `caller` (one call) + `<top-level>` (one).
        let from: Vec<&str> = calls.iter().map(|c| c.from.name.as_str()).collect();
        assert!(from.contains(&"caller"), "{calls:?}");
        assert!(from.contains(&"<top-level>"), "{calls:?}");
    }

    #[test]
    fn incoming_calls_for_target_no_self_skip_without_span() {
        // With target_name_span = None nothing is skipped as a
        // declaration, so a doc that calls the proc once yields
        // exactly one caller bucket.
        let src = "proc c {} { helper }\n";
        let analysis = analyse(src);
        let calls = incoming_calls_for_target(src, &analysis, "helper", "helper", None);
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].from.name, "c");
    }

    // Nested namespaces: `invocation_targets` delegates to
    // `references::invocation_references_proc`, so a bareword call inside a
    // namespace nested 2+ levels deep must reach Call Hierarchy just as it
    // reaches find-references.

    #[test]
    fn incoming_calls_finds_bare_call_from_two_level_nested_namespace() {
        let src = concat!(
            "namespace eval a {\n",
            "    namespace eval b {\n",
            "        proc target {} {}\n",
            "        proc caller {} { target }\n",
            "    }\n",
            "}\n",
        );
        let analysis = analyse(src);
        // Cursor on `target`'s declaration (line 2).
        let items = prepare(src, 2, 14, &analysis);
        assert_eq!(items.len(), 1, "{items:?}");
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items[0],
            &analysis,
        );
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "caller");
    }

    #[test]
    fn incoming_calls_two_level_nested_namespace_does_not_leak_across_namespaces() {
        let src = concat!(
            "namespace eval a {\n",
            "    namespace eval b {\n",
            "        proc helper {} {}\n",
            "    }\n",
            "}\n",
            "namespace eval c {\n",
            "    namespace eval d {\n",
            "        proc helper {} {}\n",
            "        proc caller {} { helper }\n",
            "    }\n",
            "}\n",
        );
        let analysis = analyse(src);
        // `::a::b::helper` has no callers.
        let items_ab = prepare(src, 2, 14, &analysis);
        assert_eq!(items_ab.len(), 1, "{items_ab:?}");
        assert!(
            incoming_calls(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                &items_ab[0],
                &analysis
            )
            .is_empty(),
            "::a::b::helper must not pick up ::c::d's caller"
        );
        // `::c::d::helper` has exactly one (its own).
        let items_cd = prepare(src, 7, 14, &analysis);
        assert_eq!(items_cd.len(), 1, "{items_cd:?}");
        let incoming_cd = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items_cd[0],
            &analysis,
        );
        assert_eq!(incoming_cd.len(), 1, "{incoming_cd:?}");
        assert_eq!(incoming_cd[0].from.name, "caller");
    }

    // Bare `ClassName <classmethod>` dispatch.  `classmethod`
    // is Tcl 9.0+, so these analyse at 9.0.

    fn analyse_tcl9(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl9.0").clone()
    }

    /// FN→TP: a `classmethod` dispatches on the
    /// class's own command (`Factory make`), so its callers are the
    /// top-level statement *and* the sibling classmethod's body — neither
    /// of which has the member's own name as its head word, which is why
    /// the head-word comparison found nothing.  tclsh9.0-verified: both
    /// `Factory build` and the bare `Factory make` really do enter `make`.
    #[test]
    fn incoming_calls_for_classmethod_find_bare_class_dispatch() {
        let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n    classmethod build {} { Factory make }\n}\nFactory make\n";
        let analysis = analyse_tcl9(src);
        // Cursor on `make`'s declaration name (line 1, col 16).
        let items = prepare(src, 1, 16, &analysis);
        assert_eq!(items[0].name, "::Factory::make", "{items:?}");
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &items[0],
            &analysis,
        );
        let callers: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
        assert_eq!(
            callers,
            vec!["::Factory::build", "<top-level>"],
            "{incoming:?}"
        );
        assert!(
            incoming.iter().all(|c| c.from_ranges.len() == 1),
            "one call site each: {incoming:?}"
        );
    }

    /// FN→TP: the outgoing direction of the same repro — `build`'s body
    /// dispatches `Factory make`, so `make` is its callee.
    #[test]
    fn outgoing_calls_from_classmethod_reach_bare_class_dispatch() {
        let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n    classmethod build {} { Factory make }\n}\nFactory make\n";
        let analysis = analyse_tcl9(src);
        // Cursor on `build`'s declaration name (line 2, col 16).
        let items = prepare(src, 2, 16, &analysis);
        assert_eq!(items[0].name, "::Factory::build", "{items:?}");
        let outgoing = outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &items[0],
            &analysis,
        );
        let callees: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert_eq!(callees, vec!["::Factory::make"], "{outgoing:?}");
        assert_eq!(outgoing[0].from_ranges.len(), 1, "{outgoing:?}");
    }

    /// FN→TP: [incr Tcl]'s class-scoped `proc` dispatches as a
    /// single `::`-qualified word, so the two-word scan that finds a
    /// `classmethod`'s edges never saw it and itcl call hierarchies were
    /// empty.  Oracle (tclsh 8.6.14 + Itcl 3.4): a bare `make` inside a
    /// sibling class `proc`, `Factory::make` inside a method body, and a
    /// top-level `Factory::make` all enter `make`.
    #[test]
    fn incoming_calls_for_an_itcl_class_proc_find_colon_qualified_dispatch() {
        let src = "itcl::class Factory {\n    proc make {} { return 1 }\n    proc build {} { return [make] }\n    method viaInstance {} { return [Factory::make] }\n}\nFactory::make\n";
        let analysis = analyse(src);
        // Cursor on `make`'s declaration name (line 1, col 10).
        let items = prepare(src, 1, 10, &analysis);
        assert_eq!(items[0].name, "::Factory::make", "{items:?}");
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &items[0],
            &analysis,
        );
        let callers: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
        assert_eq!(
            callers,
            vec!["::Factory::build", "::Factory::viaInstance", "<top-level>"],
            "{incoming:?}"
        );
    }

    /// FN→TP: the outgoing direction of the same shape — `build`'s body
    /// dispatches `make`, so `make` is its callee.
    #[test]
    fn outgoing_calls_from_an_itcl_class_proc_reach_the_sibling_class_proc() {
        let src = "itcl::class Factory {\n    proc make {} { return 1 }\n    proc build {} { return [make] }\n}\nFactory::make\n";
        let analysis = analyse(src);
        let items = prepare(src, 2, 10, &analysis);
        assert_eq!(items[0].name, "::Factory::build", "{items:?}");
        let outgoing = outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &items[0],
            &analysis,
        );
        let callees: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert_eq!(callees, vec!["::Factory::make"], "{outgoing:?}");
    }

    /// TN: itcl's two-word `Factory make` is object *creation*
    /// (`ClassName instanceName`), never a class-proc dispatch, so it adds
    /// no call-hierarchy edge.
    #[test]
    fn itcl_two_word_object_creation_is_not_a_call_hierarchy_edge() {
        let src = "itcl::class Factory {\n    proc make {} { return 1 }\n}\nFactory make\n";
        let analysis = analyse(src);
        let items = prepare(src, 1, 10, &analysis);
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &items[0],
            &analysis,
        );
        assert!(incoming.is_empty(), "{incoming:?}");
    }

    /// TP: a class command is an ordinary global command, so an *instance*
    /// method's body dispatching `Factory make` is an incoming edge too
    /// (tclsh9.0-verified) — the `my`-scope rule that keeps instance and
    /// class method tables apart does not apply to this shape.
    #[test]
    fn incoming_calls_for_classmethod_include_an_instance_method_caller() {
        let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n    method viaInstance {} { Factory make }\n}\n";
        let analysis = analyse_tcl9(src);
        let items = prepare(src, 1, 16, &analysis);
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &items[0],
            &analysis,
        );
        let callers: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
        assert_eq!(callers, vec!["::Factory::viaInstance"], "{incoming:?}");
    }

    /// FN→TP: a **proc** body dispatching a bare class command must list the
    /// classmethod under its Outgoing Calls, matching the proc that the
    /// classmethod's Incoming Calls lists — the edge must exist in both
    /// directions.
    #[test]
    fn outgoing_calls_from_proc_reach_bare_class_dispatch() {
        let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n}\nproc build {} { Factory make }\n";
        let analysis = analyse_tcl9(src);
        // Cursor on `build`'s declaration name (line 3, col 6).
        let items = prepare(src, 3, 6, &analysis);
        assert_eq!(items[0].name, "build", "{items:?}");
        let outgoing = outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &items[0],
            &analysis,
        );
        let callees: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert_eq!(callees, vec!["::Factory::make"], "{outgoing:?}");
        assert_eq!(outgoing[0].from_ranges.len(), 1, "{outgoing:?}");
        // Symmetric with the incoming direction from the same fixture.
        let make = prepare(src, 1, 16, &analysis);
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &make[0],
            &analysis,
        );
        let calling_procs: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
        assert_eq!(calling_procs, vec!["::build"], "{incoming:?}");
    }

    /// An instance `method make` and a `classmethod
    /// make` on the same class are two distinct declarations (tclsh 9.0.4:
    /// `my make` returns `inst-make`, `C make` returns `class-make`), so a
    /// caller that dispatches both must show two callee items with their own
    /// declaration ranges — not one merged item keyed on the shared display
    /// name `::C::make`.
    #[test]
    fn outgoing_calls_separate_same_named_instance_method_and_classmethod() {
        let src = "oo::class create C {\n\
                   \x20   classmethod make {} { return 1 }\n\
                   \x20   method make {} { return 2 }\n\
                   \x20   method caller {} { my make ; C make }\n\
                   }\n";
        let analysis = analyse_tcl9(src);
        // Cursor on `caller`'s declaration name (line 3, col 11).
        let items = prepare(src, 3, 11, &analysis);
        assert_eq!(items[0].name, "::C::caller", "{items:?}");
        let outgoing = outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &items[0],
            &analysis,
        );
        assert_eq!(outgoing.len(), 2, "{outgoing:?}");
        for call in &outgoing {
            assert_eq!(call.to.name, "::C::make", "{outgoing:?}");
            assert_eq!(call.from_ranges.len(), 1, "{outgoing:?}");
        }
        // One item points at the classmethod's declaration (line 1), the
        // other at the instance method's (line 2).
        let mut decl_lines: Vec<u32> = outgoing
            .iter()
            .map(|c| c.to.selection_range.start_line)
            .collect();
        decl_lines.sort_unstable();
        assert_eq!(decl_lines, vec![1, 2], "{outgoing:?}");
    }

    /// A proc caller listed beside a
    /// method caller is named the same way — qualified — rather than by its
    /// short name.
    #[test]
    fn classmethod_incoming_names_proc_callers_qualified() {
        let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n    method viaInstance {} { Factory make }\n}\nnamespace eval ::util {\n    proc helper {} { Factory make }\n}\n";
        let analysis = analyse_tcl9(src);
        let items = prepare(src, 1, 16, &analysis);
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &items[0],
            &analysis,
        );
        let callers: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
        assert_eq!(
            callers,
            vec!["::Factory::viaInstance", "::util::helper"],
            "{incoming:?}"
        );
    }

    /// FN→TP: a bare class dispatch
    /// inside an `apply` lambda body or a `namespace eval` body is a real
    /// call site, so the call hierarchy must attribute it to the body it
    /// sits in.  tclsh 9.0.4 runs all three of these dispatches.
    #[test]
    fn classmethod_incoming_covers_lambda_and_namespace_eval_bodies() {
        let src = "oo::class create Factory {\n\
                   \x20   classmethod make {} { return 1 }\n\
                   \x20   method inst {} { apply {{} { Factory make }} }\n\
                   \x20   method nsev {} { namespace eval ::zz { Factory make } }\n\
                   }\n\
                   namespace eval ::top2 { Factory make }\n";
        let analysis = analyse_tcl9(src);
        let items = prepare(src, 1, 16, &analysis);
        let incoming = incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &items[0],
            &analysis,
        );
        let callers: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
        assert_eq!(
            callers,
            vec!["::Factory::inst", "::Factory::nsev", "<top-level>"],
            "{incoming:?}"
        );
    }

    /// FP guard: `Factory make` where `Factory` is an ordinary proc calls
    /// *that proc* with the literal argument `make` (tclsh8.6/9.0-verified:
    /// it prints `proc Factory: make`), so it is no edge at all to an
    /// unrelated class's same-named classmethod.
    #[test]
    fn bare_dispatch_on_a_same_named_proc_is_not_a_classmethod_edge() {
        let src = "oo::class create Widget {\n    classmethod make {} { return 1 }\n}\nproc Factory {args} { return $args }\nFactory make\n";
        let analysis = analyse_tcl9(src);
        let items = prepare(src, 1, 16, &analysis);
        assert_eq!(items[0].name, "::Widget::make", "{items:?}");
        assert!(
            incoming_calls(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &items[0],
                &analysis
            )
            .is_empty(),
            "a proc call spelled `Factory make` is not a dispatch of Widget's classmethod"
        );
    }

    /// FP guard: an instance `method` is *not* bare-dispatchable on the
    /// class command (`Factory make` reaches only the class object's own
    /// method table), so the classmethod leg must not fire for one.
    #[test]
    fn bare_class_dispatch_is_not_an_edge_to_a_same_named_instance_method() {
        let src = "oo::class create Factory {\n    method make {} { return 1 }\n}\nFactory make\n";
        let analysis = analyse_tcl9(src);
        let items = prepare(src, 1, 11, &analysis);
        assert_eq!(items[0].name, "::Factory::make", "{items:?}");
        assert!(
            incoming_calls(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &items[0],
                &analysis
            )
            .is_empty(),
            "an instance method has no bare class-command dispatch"
        );
    }
}

#[cfg(test)]
mod original_source_context_tests {
    use super::*;
    use std::sync::Arc;
    use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};

    fn analyse(
        source: &str,
        environment: &str,
        store: Arc<tcl_registry::CommandRegistry>,
    ) -> AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for(environment).with_command_store(store),
        );
        Analyser::new()
            .with_resolved_input(ResolvedAnalysisInput::new(
                profile, profile, context, config,
            ))
            .analyse(source, profile.name)
    }

    fn store() -> Arc<tcl_registry::CommandRegistry> {
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let surface = Some(tcl_dialect::model::SpecSurface::TCL90_PLUS);
        registry.insert(tcl_registry::CommandSpec {
            name: "source_dispatch",
            traits: tcl_registry::Traits::TCLOO_SELF_DISPATCH,
            surface,
            ..tcl_registry::CommandSpec::DEFAULT
        });
        Arc::new(registry)
    }

    fn method_body(analysis: &AnalysisResult) -> tcl_lexer::Span {
        analysis.all_classes.values().next().unwrap().methods["m"].body_span
    }

    #[test]
    fn original_method_hierarchy_uses_selected_helpers_and_same_source_frame() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        let source = "oo::class create C {method m {} {source_dispatch; if 1 {puts x}; proc later {} {hidden}; missing}}";
        let store = store();
        let current = analyse(source, "tcl9.0", store.clone());
        let calls = segment_body_calls(source, &current, method_body(&current));
        let heads = calls
            .iter()
            .map(|(head, _)| head.as_str())
            .collect::<Vec<_>>();
        assert!(
            heads.contains(&"puts") && heads.contains(&"missing"),
            "{heads:?}"
        );
        assert!(
            !heads.contains(&"source_dispatch") && !heads.contains(&"hidden"),
            "{heads:?}"
        );
        let old = analyse(source, "tcl8.6", store);
        assert!(
            segment_body_calls(source, &old, method_body(&old))
                .iter()
                .any(|(head, _)| head == "source_dispatch")
        );
    }

    #[test]
    fn original_method_hierarchy_keeps_replaced_helpers_unresolved_and_withdraws_stale_source() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        let source =
            "proc source_dispatch args {}; oo::class create C {method m {} {source_dispatch}}";
        let mut current = analyse(source, "tcl9.0", store());
        let body = method_body(&current);
        assert!(
            segment_body_calls(source, &current, body)
                .iter()
                .any(|(head, _)| head == "source_dispatch")
        );
        assert!(
            segment_body_calls(
                &source.replace("source_dispatch", "other_dispatch"),
                &current,
                body
            )
            .is_empty()
        );
        current.resolved_input = None;
        assert!(segment_body_calls(source, &current, body).is_empty());
    }
}
