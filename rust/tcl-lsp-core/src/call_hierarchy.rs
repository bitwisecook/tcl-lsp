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

//! Call-hierarchy provider.
//!
//! Three entry points:
//!
//! * [`prepare`] — resolves the proc *or class method* at the
//!   cursor into a single [`CallHierarchyItem`].
//! * [`incoming_calls`] — every call site in the document that
//!   targets the given proc / method, grouped by the enclosing
//!   proc / method.
//! * [`outgoing_calls`] — every call site inside the given
//!   proc's / method's body, grouped by the callee.
//!
//! Proc edge enumeration walks `analysis.command_invocations` and
//! intersects their byte spans with each proc's body span.
//!
//! Class-method edges are computed differently: the analyser's
//! `command_invocations` collection only records top-level
//! invocations, so method bodies are re-segmented on demand via
//! [`tcl_compiler::segmenter::segment_commands_with_offset`] —
//! the same strategy the rename / references / code-lens
//! class-member walks use.  A method item is identified by the
//! synthetic name `<class-qualified-name>::<method-name>` (e.g.
//! `::C::greet`); intra-class calls match on the bare method
//! name.
//!
//! The in-document computations above need no workspace index.
//! Cross-document edges are layered on top by the server: it
//! feeds the heads from [`unresolved_outgoing_calls`] (call sites
//! whose callee isn't defined locally) to the workspace index to
//! resolve sibling-file definitions, and runs
//! [`incoming_calls_for_target`] over each other document to find
//! sibling-file call sites.

use tcl_compiler::analyser::{AnalysisResult, ClassDef, MethodDef, ProcDef};
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

/// One hierarchy item — proc identification plus its name and
/// definition span for editor display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallHierarchyItem {
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
    let line_index = LineIndex::new(source);
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    // Prefer the proc whose declaration name span covers the cursor (so a
    // same-named proc in another namespace's own decl resolves to *that*
    // proc, not whichever `all_procs` entry hashes first — mirrors
    // `references::proc_references` / `rename::rename_proc`); else the
    // first proc matching the word.
    let cursor_off = crate::definition::byte_offset_at(&line_index, source, line, character);
    let proc_match = analysis
        .all_procs
        .iter()
        .find(|(_, p)| p.name_span.start() <= cursor_off && cursor_off < p.name_span.end())
        .or_else(|| {
            analysis.all_procs.iter().find(|(qname, p)| {
                p.name == word || *qname == &word || *qname == &format!("::{word}")
            })
        });
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

/// Synthetic call-hierarchy name for a class method:
/// `<class-qualified-name>::<method-name>` (e.g. `::C::greet`).
fn method_item_name(class_def: &ClassDef, method: &MethodDef) -> String {
    format!("{}::{}", class_def.qualified_name, method.name)
}

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
    crate::definition::resolve_proc_target_at(analysis, source, cursor_off, &item.name, None)
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
        name: method_item_name(class_def, method),
        detail,
        range: full_range,
        selection_range: name_range,
    }
}

/// Find the class + method whose body contains `cursor_offset`
/// and whose method name matches `word`.  Searches `methods`
/// then `class_methods`.
fn enclosing_class_method<'a>(
    analysis: &'a AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<(&'a ClassDef, &'a MethodDef)> {
    for class_def in analysis.all_classes.values() {
        let body = class_def.body_span;
        if !(body.start() < cursor_offset && cursor_offset < body.end()) {
            continue;
        }
        if let Some(m) = class_def.methods.get(word) {
            return Some((class_def, m));
        }
        if let Some(m) = class_def.class_methods.get(word) {
            return Some((class_def, m));
        }
    }
    None
}

/// Resolve a method item name (`<class-qual>::<method>`) back to
/// its [`ClassDef`] + [`MethodDef`].  Splits on the final `::`.
fn resolve_method_item<'a>(
    analysis: &'a AnalysisResult,
    item_name: &str,
) -> Option<(&'a ClassDef, &'a MethodDef)> {
    let idx = item_name.rfind("::")?;
    let class_q = &item_name[..idx];
    let method_name = &item_name[idx + 2..];
    let class_def = analysis
        .all_classes
        .values()
        .find(|c| c.qualified_name == class_q)?;
    let method = class_def
        .methods
        .get(method_name)
        .or_else(|| class_def.class_methods.get(method_name))?;
    Some((class_def, method))
}

/// Re-segment a method body and return `(head_word, head_span)`
/// for every command invocation in it.  Surrounding braces are
/// stripped so the segmenter descends into the body rather than
/// treating the leading `{` as a braced literal.
fn segment_body_calls(
    source: &str,
    dialect: &str,
    body_span: tcl_lexer::Span,
) -> Vec<(String, tcl_lexer::Span)> {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    if body_span.is_empty() {
        return Vec::new();
    }
    let mut start = body_span.start() as usize;
    let mut end = body_span.end() as usize;
    if start >= source.len() || end > source.len() || start > end {
        return Vec::new();
    }
    if source.as_bytes().get(start) == Some(&b'{') {
        start += 1;
    }
    if end > start && source.as_bytes().get(end - 1) == Some(&b'}') {
        end -= 1;
    }
    let body_text = &source[start..end];
    let commands = segment_commands_with_offset_and_config(
        body_text,
        u32::try_from(start).unwrap_or(body_span.start()),
        tcl_lexer::LexerConfig::for_dialect(dialect),
    );
    let mut out = Vec::new();
    for cmd in &commands {
        let Some(head) = cmd.argv.first() else {
            continue;
        };
        let h_start = head.span.start() as usize;
        let h_end = head.span.end() as usize;
        if h_start >= source.len() || h_end > source.len() {
            continue;
        }
        out.push((source[h_start..h_end].to_owned(), head.span));
    }
    out
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

/// `true` when `inv` (a command invocation) targets `proc_def`.
///
/// Delegates to [`crate::references::invocation_references_proc`] — the one
/// shared matching rule behind Find-All-References, the code-lens count,
/// and Rename — so Call Hierarchy can never disagree with them about
/// whether a given call site is a reference (in particular the namespace
/// gate that keeps a bare call in a different namespace from cross-matching
/// a same-named proc, `RUST_ISSUE_035`).
fn invocation_targets(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    proc_def: &ProcDef,
    qname: &str,
) -> bool {
    crate::references::invocation_references_proc(analysis, inv, qname, proc_def)
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
    dialect: &str,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<UnresolvedOutgoingCall> {
    let line_index = LineIndex::new(source);
    if let Some((_, source_proc)) = find_proc_for_item(source, analysis, item, &line_index) {
        let mut by_head: std::collections::BTreeMap<String, (Option<String>, Vec<LspRange>)> =
            std::collections::BTreeMap::new();
        for inv in &analysis.command_invocations {
            if !span_contains(source_proc.body_span, inv.range) {
                continue;
            }
            // Skip call sites the local pass already resolves.
            if analysis
                .all_procs
                .iter()
                .any(|(qname, proc_def)| invocation_targets(analysis, inv, proc_def, qname))
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
    unresolved_method_outgoing_calls(source, dialect, item, analysis, &line_index)
}

/// Method-body variant of [`unresolved_outgoing_calls`]: keeps
/// the call sites inside a class method that name neither a
/// sibling method nor a local top-level proc.
fn unresolved_method_outgoing_calls(
    source: &str,
    dialect: &str,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<UnresolvedOutgoingCall> {
    let Some((class_def, source_method)) = resolve_method_item(analysis, &item.name) else {
        return Vec::new();
    };
    let mut by_head: std::collections::BTreeMap<String, Vec<LspRange>> =
        std::collections::BTreeMap::new();
    for (head, span) in segment_body_calls(source, dialect, source_method.body_span) {
        // Sibling method?
        if class_def.methods.contains_key(&head) || class_def.class_methods.contains_key(&head) {
            continue;
        }
        // Local top-level proc?
        if analysis
            .all_procs
            .iter()
            .any(|(qn, p)| p.name == head || qn.as_str() == head || **qn == format!("::{head}"))
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
    dialect: &str,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<IncomingCall> {
    let line_index = LineIndex::new(source);
    let Some((target_qname, target_proc)) = find_proc_for_item(source, analysis, item, &line_index)
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
    let line_index = LineIndex::new(source);
    let mut by_caller: std::collections::BTreeMap<String, (CallHierarchyItem, Vec<LspRange>)> =
        std::collections::BTreeMap::new();
    for inv in &analysis.command_invocations {
        // Delegate to the shared matching rule (`invocation_references_proc`
        // takes a `ProcDef`; this caller may not have one — cross-document
        // callers are gathered from a target the calling document never
        // defines — so route through the string-keyed core directly). This
        // adds the namespace gate a bare simple-name match needs: without
        // it, a bare call in one namespace falsely credited *any* same-named
        // proc anywhere as an incoming caller.
        if !crate::references::invocation_references_named(
            analysis,
            inv,
            target_qualified,
            target_simple,
            target_qualified,
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
            .map_or_else(|| "<top-level>".to_owned(), |(qn, _)| qn.to_owned());
        let entry = by_caller.entry(caller_key.clone()).or_insert_with(|| {
            let caller_item = if caller_key == "<top-level>" {
                CallHierarchyItem {
                    name: caller_key.clone(),
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
                }
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
/// target proc (when the invocation names a user proc).  Calls
/// to built-in commands are dropped (they have no
/// [`CallHierarchyItem`] to point to).  Multiple call sites to
/// the same target group together.
#[must_use]
pub fn outgoing_calls(
    source: &str,
    dialect: &str,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
) -> Vec<OutgoingCall> {
    let line_index = LineIndex::new(source);
    let Some((_, source_proc)) = find_proc_for_item(source, analysis, item, &line_index) else {
        // Not a proc — try a class method.
        return method_outgoing_calls(source, dialect, item, analysis, &line_index);
    };
    // Map target qname → (target item, list of ranges).
    let mut by_target: std::collections::BTreeMap<String, (CallHierarchyItem, Vec<LspRange>)> =
        std::collections::BTreeMap::new();
    for inv in &analysis.command_invocations {
        if !span_contains(source_proc.body_span, inv.range) {
            continue;
        }
        // Find the user-proc this invocation targets, if any.
        for (qname, proc_def) in &analysis.all_procs {
            if invocation_targets(analysis, inv, proc_def, qname) {
                let inv_range = span_to_range(source, &line_index, inv.range);
                let entry = by_target.entry(qname.clone()).or_insert_with(|| {
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
    by_target
        .into_values()
        .map(|(to, from_ranges)| OutgoingCall { to, from_ranges })
        .collect()
}

/// Incoming calls for a class method — every call site naming
/// the method inside any sibling method body, grouped by the
/// enclosing method.  Intra-class calls match on the bare
/// method name.
fn method_incoming_calls(
    source: &str,
    dialect: &str,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<IncomingCall> {
    let Some((class_def, target_method)) = resolve_method_item(analysis, &item.name) else {
        return Vec::new();
    };
    let mut by_caller: std::collections::BTreeMap<String, (CallHierarchyItem, Vec<LspRange>)> =
        std::collections::BTreeMap::new();
    for caller in class_methods_iter(class_def) {
        for (head, span) in segment_body_calls(source, dialect, caller.body_span) {
            if head != target_method.name {
                continue;
            }
            // Skip the declaration site itself.
            if span == target_method.name_span {
                continue;
            }
            let key = method_item_name(class_def, caller);
            let entry = by_caller.entry(key).or_insert_with(|| {
                (
                    item_for_method(source, class_def, caller, line_index),
                    Vec::new(),
                )
            });
            entry.1.push(span_to_range(source, line_index, span));
        }
    }
    by_caller
        .into_values()
        .map(|(from, from_ranges)| IncomingCall { from, from_ranges })
        .collect()
}

/// Outgoing calls from a class method — every call site inside
/// the method's body that names a sibling method (→ method
/// item) or a top-level user proc (→ proc item).
fn method_outgoing_calls(
    source: &str,
    dialect: &str,
    item: &CallHierarchyItem,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<OutgoingCall> {
    let Some((class_def, source_method)) = resolve_method_item(analysis, &item.name) else {
        return Vec::new();
    };
    let mut by_target: std::collections::BTreeMap<String, (CallHierarchyItem, Vec<LspRange>)> =
        std::collections::BTreeMap::new();
    for (head, span) in segment_body_calls(source, dialect, source_method.body_span) {
        let range = span_to_range(source, line_index, span);
        // Sibling method?
        if let Some(callee) = class_def
            .methods
            .get(&head)
            .or_else(|| class_def.class_methods.get(&head))
        {
            // Skip self-recursion's own declaration site only;
            // recursive calls are legitimate outgoing edges.
            let key = method_item_name(class_def, callee);
            let entry = by_target.entry(key).or_insert_with(|| {
                (
                    item_for_method(source, class_def, callee, line_index),
                    Vec::new(),
                )
            });
            entry.1.push(range);
            continue;
        }
        // Top-level user proc?
        if let Some((qname, proc_def)) = analysis
            .all_procs
            .iter()
            .find(|(qn, p)| p.name == head || qn.as_str() == head || **qn == format!("::{head}"))
        {
            let entry = by_target.entry(qname.clone()).or_insert_with(|| {
                (
                    item_for_proc(source, proc_def, qname, line_index),
                    Vec::new(),
                )
            });
            entry.1.push(range);
        }
    }
    by_target
        .into_values()
        .map(|(to, from_ranges)| OutgoingCall { to, from_ranges })
        .collect()
}

/// Iterate every method + classmethod of a class (the bodies
/// that can host intra-class method calls).
fn class_methods_iter(class_def: &ClassDef) -> impl Iterator<Item = &MethodDef> {
    class_def
        .methods
        .values()
        .chain(class_def.class_methods.values())
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
        let outgoing = outgoing_calls(src, "tcl8.6", &items[0], &analysis);
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
        let incoming = incoming_calls(src, "tcl", target, &analysis);
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "caller");
        assert_eq!(incoming[0].from_ranges.len(), 1);
    }

    #[test]
    fn incoming_calls_group_by_caller() {
        // Two call sites from the same caller bucket together
        // into a single IncomingCall entry.
        let src = "proc target {} {}\nproc caller {} { target\n target }\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        let target = &items[0];
        let incoming = incoming_calls(src, "tcl", target, &analysis);
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from_ranges.len(), 2);
    }

    #[test]
    fn incoming_calls_top_level_bucketed_as_top_level() {
        let src = "proc target {} {}\ntarget\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        let target = &items[0];
        let incoming = incoming_calls(src, "tcl", target, &analysis);
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
        let outgoing = outgoing_calls(src, "tcl", &items[0], &analysis);
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
        let outgoing = outgoing_calls(src, "tcl", &items[0], &analysis);
        let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert_eq!(names, vec!["target"], "{outgoing:?}");
    }

    #[test]
    fn outgoing_calls_empty_for_unknown_proc() {
        let src = "proc greet {} {}\n";
        let analysis = analyse(src);
        let bogus = CallHierarchyItem {
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
        assert!(outgoing_calls(src, "tcl", &bogus, &analysis).is_empty());
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
        let unresolved = unresolved_outgoing_calls(src, "tcl", &items[0], &analysis);
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
        let unresolved = unresolved_outgoing_calls(src, "tcl", &items[0], &analysis);
        let sibling = unresolved
            .iter()
            .find(|u| u.name == "sibling")
            .expect("sibling head present");
        assert_eq!(sibling.from_ranges.len(), 2, "{unresolved:?}");
    }

    // class methods

    #[test]
    fn prepare_resolves_method_at_cursor() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `greet` declaration (line 1, col 11).
        let items = prepare(src, 1, 11, &analysis);
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].name, "::C::greet");
    }

    #[test]
    fn incoming_calls_for_method_grouped_by_caller_method() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        let items = prepare(src, 1, 11, &analysis);
        let incoming = incoming_calls(src, "tcl", &items[0], &analysis);
        // One caller method (`twice`) with two call ranges.
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "::C::twice");
        assert_eq!(incoming[0].from_ranges.len(), 2, "{incoming:?}");
    }

    #[test]
    fn outgoing_calls_from_method_to_sibling_method() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Resolve the `twice` method (line 2, col 11).
        let items = prepare(src, 2, 11, &analysis);
        assert_eq!(items[0].name, "::C::twice");
        let outgoing = outgoing_calls(src, "tcl", &items[0], &analysis);
        let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert_eq!(names, vec!["::C::greet"], "{outgoing:?}");
        // Two call sites collapse into one target entry.
        assert_eq!(outgoing[0].from_ranges.len(), 2, "{outgoing:?}");
    }

    #[test]
    fn outgoing_calls_from_method_to_top_level_proc() {
        let src = "proc helper {} {}\noo::class create C {\n    method use {} { helper }\n}\n";
        let analysis = analyse(src);
        // Resolve the `use` method (line 2, col 11).
        let items = prepare(src, 2, 11, &analysis);
        assert_eq!(items[0].name, "::C::use");
        let outgoing = outgoing_calls(src, "tcl", &items[0], &analysis);
        let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
        assert_eq!(names, vec!["helper"], "{outgoing:?}");
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

    // nested namespaces — `invocation_targets` delegates to
    // `references::invocation_references_proc`; regression coverage for the
    // namespace-accumulation bug fixed alongside issue #923 (bareword calls
    // inside a namespace nested 2+ levels deep were previously invisible to
    // Call Hierarchy, same root cause as the reference-finding bug).

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
        let incoming = incoming_calls(src, "tcl", &items[0], &analysis);
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
            incoming_calls(src, "tcl", &items_ab[0], &analysis).is_empty(),
            "::a::b::helper must not pick up ::c::d's caller"
        );
        // `::c::d::helper` has exactly one (its own).
        let items_cd = prepare(src, 7, 14, &analysis);
        assert_eq!(items_cd.len(), 1, "{items_cd:?}");
        let incoming_cd = incoming_calls(src, "tcl", &items_cd[0], &analysis);
        assert_eq!(incoming_cd.len(), 1, "{incoming_cd:?}");
        assert_eq!(incoming_cd[0].from.name, "caller");
    }
}
