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

//! Find-references / document-highlight provider.
//!
//! Locates every usage of the symbol at the cursor:
//!
//! * `$var` references → `VarDef.definition_span` plus every
//!   span in `VarDef.references` (already collected by the
//!   analyser's body walk).
//! * proc references → `ProcDef.name_span` plus every command
//!   invocation in `analysis.command_invocations` whose head
//!   matches the proc's simple or qualified name.
//! * class references → `ClassDef.name_span` plus every command
//!   invocation whose head matches the class's simple or
//!   qualified name.
//!
//! Two entry points:
//!
//! * [`references`] — returns plain `Vec<LspRange>` for the
//!   LSP `textDocument/references` request.
//! * [`document_highlights`] — returns
//!   `Vec<(LspRange, HighlightKind)>` for the LSP
//!   `textDocument/documentHighlight` request.  Variables get
//!   the `Write` / `Read` distinction;
//!   command-invocation matches
//!   stay `Text` because the analyser's
//!   `command_invocations` doesn't currently surface read /
//!   write semantics on call-head matches.
//!
//! Class-member references: when the cursor sits
//! on a method, classmethod, or property name inside the
//! class body, the provider re-segments every sibling method
//! body and surfaces each invocation that names the same
//! member.  `document_highlights` returns the declaration as
//! `Write` and every call site as `Text`.
//!
//! External `$obj method` references: when the
//! cursor sits on the method-name token of a `$obj method`
//! call (or inside the class body), the provider additionally
//! scans the whole document for `$v method` / `[$v method]`
//! call sites where `v`'s class (per
//! `analysis.instance_classes`) matches.  See
//! [`find_obj_method_call_sites`] for the scan's coverage.
//!
//! Limitations:
//!
//! * Cross-document references — surfacing references across
//!   every open document via a workspace-index integration —
//!   are not supported.
//! * `$obj method` sites embedded in quoted / word tokens
//!   (`"prefix[$d bark]"`) — the scan descends into
//!   command-substitution args and proc / method bodies but
//!   not into string interpolation.

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::{find_var_at_position, find_word_span_at_position};

/// Byte spans of every call site the namespace-aware proc resolver
/// attributes to `proc_def` (whose `all_procs` map key is `qname`),
/// excluding the declaration.
///
/// This is the matching core shared by [`references`] (the peek / Find
/// All References) and the code-lens reference count, so the two can never
/// disagree.  It takes the resolved `proc_def`
/// directly — no cursor, no `LineIndex`, no proc-table rescan — so a
/// caller iterating every proc (the code-lens provider) doesn't pay that
/// per-proc overhead.
#[must_use]
pub(crate) fn proc_reference_spans(
    analysis: &AnalysisResult,
    qname: &str,
    proc_def: &tcl_compiler::analyser::ProcDef,
) -> Vec<tcl_lexer::Span> {
    analysis
        .command_invocations
        .iter()
        .filter(|inv| invocation_references_proc(analysis, inv, qname, proc_def))
        .map(|inv| inv.range)
        .collect()
}

/// Whether a single call site `inv` references a named proc/class
/// definition — simple name `def_name`, fully-qualified name
/// `def_qualified` (whose lookup key is `qname`).
///
/// This is the **single** matching rule behind every proc/class-oriented
/// consumer: Find-All-References ([`proc_reference_spans`],
/// [`class_reference_spans`]), the code-lens reference count
/// (`code_lens::code_lenses`), Rename (`rename::rename_proc`,
/// `rename::rename_class`), and Call Hierarchy
/// (`call_hierarchy::invocation_targets`) all resolve through this one
/// function (via [`invocation_references_proc`] / [`invocation_references_class`])
/// so none of them can disagree about whether a given call site is a
/// reference — a rename that rewrote a call the reference finder does not
/// report would corrupt a *different* same-named definition, and a
/// call-hierarchy edge that disagreed with the reference count would be a
/// visible inconsistency in the same editor session.
///
/// A bare simple-name call (`helper`) counts only when it resolves to this
/// definition, or — since the analyser resolves a namespace-internal call to
/// the global guess (`::helper`) — when it sits in this definition's own
/// namespace; that namespace gate keeps `helper` inside `namespace eval b`
/// from matching `::a::helper`. Qualified spellings and a
/// resolved-qualified-name hit always count. Comparisons ignore the leading
/// `::`.
#[must_use]
pub(crate) fn invocation_references_named(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    qname: &str,
    def_name: &str,
    def_qualified: &str,
) -> bool {
    let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname);
    let target_q = def_qualified.trim_start_matches("::");
    // The definition's own namespace (`a::helper` → `a`; top-level → ``).
    let target_ns = target_q.rsplit_once("::").map_or("", |(ns, _)| ns);
    let resolved_norm = inv
        .resolved_qualified_name
        .as_deref()
        .map(|r| r.trim_start_matches("::"));
    let call_ns =
        crate::definition::innermost_namespace_at(&analysis.global_scope, inv.range.start());
    let simple_ok = inv.name == def_name
        && resolved_norm.is_none_or(|r| r == target_q || r == def_name)
        && call_ns == target_ns;
    if simple_ok || inv.name == def_qualified || resolved_norm == Some(target_q) {
        return true;
    }
    // Fallback tail-match: the call is spelled without a leading `::` but its
    // text otherwise equals this definition's qualified name (`ns::foo`
    // matching `::ns::foo` when called from *outside* `ns` — `resolved_norm`
    // can't help there since it's rooted at the call's own namespace, not
    // `ns`'s). Real Tcl commits to the first candidate that exists rather
    // than falling through to a same-tail alternative, so this only counts
    // when the call's own higher-priority resolution guess (`resolved_norm`,
    // current-namespace-first) doesn't already name a *different* real
    // proc/class — i.e. that candidate doesn't exist, so resolution
    // legitimately reaches this one instead. When `resolved_norm` is `None`
    // (a background-scanned, unfocused document — the analyser skips the
    // scope walk there), there's no resolution info to shadow it with, so
    // the tail-match applies unconditionally.
    if inv.name == qname_no_prefix {
        let shadowed_by_other = resolved_norm.is_some_and(|r| {
            r != target_q
                && (analysis
                    .all_procs
                    .keys()
                    .any(|k| k.trim_start_matches("::") == r)
                    || analysis
                        .all_classes
                        .keys()
                        .any(|k| k.trim_start_matches("::") == r))
        });
        return !shadowed_by_other;
    }
    false
}

/// [`invocation_references_named`] specialised for a [`ProcDef`](tcl_compiler::analyser::ProcDef).
#[must_use]
pub(crate) fn invocation_references_proc(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    qname: &str,
    proc_def: &tcl_compiler::analyser::ProcDef,
) -> bool {
    invocation_references_named(
        analysis,
        inv,
        qname,
        &proc_def.name,
        &proc_def.qualified_name,
    )
}

/// [`invocation_references_named`] specialised for a [`ClassDef`](tcl_compiler::analyser::ClassDef).
#[must_use]
pub(crate) fn invocation_references_class(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    qname: &str,
    class_def: &tcl_compiler::analyser::ClassDef,
) -> bool {
    invocation_references_named(
        analysis,
        inv,
        qname,
        &class_def.name,
        &class_def.qualified_name,
    )
}

/// Compute the locations of every reference to the symbol at
/// the cursor.
///
/// `include_declaration` mirrors the LSP `ReferenceContext`
/// flag — when `true`, the symbol's defining span is the first
/// element of the returned vector.
#[must_use]
pub fn references(
    source: &str,
    dialect: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    include_declaration: bool,
) -> Vec<LspRange> {
    let line_index = LineIndex::new(source);
    let ctx = RefCtx {
        source,
        dialect,
        line_index: &line_index,
        line,
        character,
        analysis,
        include_declaration,
    };

    if let Some(out) = variable_references(&ctx) {
        return out;
    }

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };

    // Class references (checked first, before proc names).
    if let Some(out) = class_references(&ctx, &word) {
        return out;
    }

    // Proc references.
    if let Some(out) = proc_references(&ctx, &word) {
        return out;
    }

    // `$obj method` external call site.
    if let Some(out) = instance_method_references(&ctx) {
        return out;
    }

    // Class-member references (cursor inside a class body on a member name).
    if let Some(out) = class_member_references(&ctx, &word) {
        return out;
    }

    Vec::new()
}

/// Shared immutable inputs for the per-kind reference resolvers, so each
/// helper takes a single context instead of re-threading the same seven
/// parameters.
#[derive(Clone, Copy)]
struct RefCtx<'a> {
    source: &'a str,
    dialect: &'a str,
    line_index: &'a LineIndex,
    line: u32,
    character: u32,
    analysis: &'a AnalysisResult,
    include_declaration: bool,
}

/// Build `decl + references` ranges for a variable at the cursor.
fn variable_references(ctx: &RefCtx<'_>) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
        ..
    } = *ctx;
    let var_name = find_var_at_position(source, line, character)?;
    let byte_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    let var_def = crate::definition::lookup_var_in_scope_chain(
        &analysis.global_scope,
        byte_offset,
        &var_name,
    )?;
    let mut out = Vec::new();
    if include_declaration {
        out.push(span_to_range(source, line_index, var_def.definition_span));
    }
    // Unify every alias Tcl treats as one cell — namespace/global aliases
    // (`global`/`variable`/`namespace upvar`) and a class instance variable's
    // per-method copies — so the reference set spans them all.
    for r in crate::definition::linked_var_reference_spans(&analysis.global_scope, var_def) {
        out.push(span_to_range(source, line_index, r));
    }
    dedup_ranges(&mut out);
    Some(out)
}

/// Byte spans of every call site the namespace-aware class resolver
/// attributes to `class_def` (whose `all_classes` map key is `qname`),
/// excluding the declaration.
///
/// The class analogue of [`proc_reference_spans`] — shared by
/// [`class_references`] (Find All References) and the code-lens class
/// reference count so the two can never disagree.
#[must_use]
pub(crate) fn class_reference_spans(
    analysis: &AnalysisResult,
    qname: &str,
    class_def: &tcl_compiler::analyser::ClassDef,
) -> Vec<tcl_lexer::Span> {
    analysis
        .command_invocations
        .iter()
        .filter(|inv| invocation_references_class(analysis, inv, qname, class_def))
        .map(|inv| inv.range)
        .collect()
}

/// Build references for a class name at the cursor (constructor invocations
/// plus `superclass`/`mixin` usages across every class body).  Prefers the
/// class whose declaration name span covers the cursor (so `Widget` at the
/// `::b::Widget` decl resolves to *that* namespace's class, not a same-named
/// one in another namespace — mirroring [`proc_references`]); else the first
/// class matching the word.
fn class_references(ctx: &RefCtx<'_>, word: &str) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
        ..
    } = *ctx;
    let cursor_off = crate::definition::byte_offset_at(line_index, source, line, character);
    // Declaration under the cursor, else namespace-aware resolution — never a
    // namespace-blind `c.name == word` scan (which from a call site could
    // surface an unrelated same-named class's reference set).
    let (qname, class_def) =
        crate::definition::resolve_class_target_at(analysis, cursor_off, word)?;
    let simple = class_def.name.clone();
    let qualified = class_def.qualified_name.clone();
    let mut out = Vec::new();
    if include_declaration {
        out.push(span_to_range(source, line_index, class_def.name_span));
    }
    for span in class_reference_spans(analysis, qname, class_def) {
        out.push(span_to_range(source, line_index, span));
    }
    // `superclass <C>` / `mixin <C>` usages across every class body
    // are references to the class too.
    let matches_name =
        |n: &str| n == simple || n == qualified || format!("::{n}") == qualified || n == word;
    for other in analysis.all_classes.values() {
        for (name, span) in other.superclass_refs.iter().chain(other.mixin_refs.iter()) {
            if matches_name(name) {
                out.push(span_to_range(source, line_index, *span));
            }
        }
    }
    dedup_ranges(&mut out);
    Some(out)
}

/// Build references for a proc name at the cursor.  Prefers the proc whose
/// declaration the cursor sits on (so `helper` at the `a::helper` decl
/// resolves to *that* namespace's proc, not a same-named one in another
/// namespace); else the first proc matching the word.
fn proc_references(ctx: &RefCtx<'_>, word: &str) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
        ..
    } = *ctx;
    let cursor_off = crate::definition::byte_offset_at(line_index, source, line, character);
    // Declaration under the cursor, else C Tcl's namespace-aware call-site
    // resolution — never a namespace-blind `p.name == word` scan (which from a
    // call site could surface an unrelated same-named proc's reference set).
    let (qname, proc_def) =
        crate::definition::resolve_proc_target_at(analysis, source, cursor_off, word, None)?;
    let mut out = Vec::new();
    if include_declaration {
        out.push(span_to_range(source, line_index, proc_def.name_span));
    }
    for span in proc_reference_spans(analysis, qname, proc_def) {
        out.push(span_to_range(source, line_index, span));
    }
    dedup_ranges(&mut out);
    Some(out)
}

/// Build references for a `$obj method` call site: when the cursor sits on
/// the method-name token of an instance-method call and `$obj`'s class is
/// known, surface the method declaration plus every call site (intra-class
/// + external).
fn instance_method_references(ctx: &RefCtx<'_>) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        dialect,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
    } = *ctx;
    let (inst, method, is_dollar) =
        crate::definition::instance_method_at_cursor(source, line, character)?;
    let class_q = crate::definition::receiver_instance_class(analysis, &inst, is_dollar)?;
    let (decl_span, call_spans) =
        method_references_for_class(source, dialect, analysis, class_q, &method)?;
    Some(build_member_ranges(
        source,
        line_index,
        decl_span,
        call_spans,
        include_declaration,
    ))
}

/// Build references for a class member (method / classmethod / property)
/// when the cursor sits inside a class body and `word` matches a member:
/// re-segment the sibling method bodies for every invocation naming the
/// same member, then append external `$obj method` call sites.  Mirrors the
/// `rename_method` walk in `crate::rename`.
fn class_member_references(ctx: &RefCtx<'_>, word: &str) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        dialect,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
    } = *ctx;
    let cursor_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    let (decl_span, call_spans) =
        find_class_member_references(source, dialect, word, analysis, cursor_offset)?;
    Some(build_member_ranges(
        source,
        line_index,
        decl_span,
        call_spans,
        include_declaration,
    ))
}

/// Shared range-builder for the `(decl_span, call_spans)` member-reference
/// shape: optional declaration first, then every call site, deduped.
fn build_member_ranges(
    source: &str,
    line_index: &LineIndex,
    decl_span: tcl_lexer::Span,
    call_spans: Vec<tcl_lexer::Span>,
    include_declaration: bool,
) -> Vec<LspRange> {
    let mut out = Vec::new();
    if include_declaration {
        out.push(span_to_range(source, line_index, decl_span));
    }
    for s in call_spans {
        out.push(span_to_range(source, line_index, s));
    }
    dedup_ranges(&mut out);
    out
}

/// Every method / classmethod / constructor / destructor body span of `cd`
/// — the regions re-segmented for intra-class `my <member>` call sites.
fn collect_member_bodies(cd: &tcl_compiler::analyser::types::ClassDef) -> Vec<tcl_lexer::Span> {
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

/// Resolve a method's declaration span plus every call site —
/// intra-class (re-segment the class's own method bodies plus any
/// inheriting subclass's bodies) and external (`$obj method` across the
/// document).  Returns `None` when `class_q` has no method / classmethod
/// named `method`.
pub(crate) fn method_references_for_class(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
) -> Option<(tcl_lexer::Span, Vec<tcl_lexer::Span>)> {
    use tcl_lexer::Span;
    let class_def = analysis.all_classes.get(class_q)?;
    let decl_span = class_def
        .methods
        .get(method)
        .map(|m| m.name_span)
        .or_else(|| class_def.class_methods.get(method).map(|m| m.name_span))?;

    let mut call_spans: Vec<Span> = Vec::new();
    // Intra-class `my method` dispatch: re-segment every method / classmethod
    // / ctor / dtor body for `my method` invocations.  Scan `class_q`'s own
    // bodies **and** the bodies of every subclass that *inherits* this
    // definition — a class whose MRO resolves `method` to `class_q` (i.e. it
    // does not override).  A pure inheritor is not itself a rename family
    // member (it declares no copy of `method`), but its `my method` calls
    // dispatch to `class_q`'s definition, so they must rename with it;
    // omitting them left those call sites pointing at the old name.  A
    // subclass that *overrides* `method` resolves to itself, not `class_q`,
    // so its bodies are handled under its own family entry — never here.
    let hierarchy = analysis.class_hierarchy();
    let mut bodies: Vec<Span> = collect_member_bodies(class_def);
    for (other_q, other_cd) in &analysis.all_classes {
        if other_q.as_str() != class_q && hierarchy.method_target(other_q, method) == Some(class_q)
        {
            bodies.extend(collect_member_bodies(other_cd));
        }
    }
    call_spans.extend(scan_my_method_sites(
        source,
        dialect,
        &bodies,
        method,
        Some(decl_span),
    ));
    // External `$obj method` sites.
    call_spans.extend(find_obj_method_call_sites(
        source, dialect, analysis, class_q, method,
    ));
    Some((decl_span, call_spans))
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
    dialect: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    include_decl: bool,
) -> Vec<tcl_lexer::Span> {
    match method_references_for_class(source, dialect, analysis, class_q, method) {
        Some((decl, mut calls)) => {
            if include_decl {
                calls.push(decl);
            }
            calls
        }
        None => Vec::new(),
    }
}

/// Re-segment each brace-delimited body span in `bodies` and return the
/// name-token span of every `my <method>` invocation whose method name is
/// `method`, skipping the token at `skip` (the declaration site, when the
/// scanned class declares `method`).
///
/// Intra-class dispatch of a `TclOO` method is `my <method>` — the method
/// name is argv[1], not the command head.  A bare head equal to the method
/// name is *not* a call (a `TclOO` method is not a command in the body's
/// namespace; `<method> …` without `my`/an object errors "invalid command
/// name"), so only `my`-headed sites match.
fn scan_my_method_sites(
    source: &str,
    dialect: &str,
    bodies: &[tcl_lexer::Span],
    method: &str,
    skip: Option<tcl_lexer::Span>,
) -> Vec<tcl_lexer::Span> {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    let mut out: Vec<tcl_lexer::Span> = Vec::new();
    for &body_span in bodies {
        if body_span.is_empty() {
            continue;
        }
        let mut start = body_span.start() as usize;
        let mut end = body_span.end() as usize;
        if start >= source.len() || end > source.len() || start > end {
            continue;
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
            u32::try_from(start).unwrap_or(0),
            tcl_lexer::LexerConfig::for_dialect(dialect),
        );
        for cmd in &commands {
            let Some(head) = cmd.argv.first() else {
                continue;
            };
            let h_start = head.span.start() as usize;
            let h_end = head.span.end() as usize;
            if h_start >= source.len() || h_end > source.len() || &source[h_start..h_end] != "my" {
                continue;
            }
            let Some(name_tok) = cmd.argv.get(1) else {
                continue;
            };
            let n_start = name_tok.span.start() as usize;
            let n_end = name_tok.span.end() as usize;
            if n_start >= source.len() || n_end > source.len() {
                continue;
            }
            if &source[n_start..n_end] == method && Some(name_tok.span) != skip {
                out.push(name_tok.span);
            }
        }
    }
    out
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
#[must_use]
pub(crate) fn inherited_method_call_sites(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
) -> Vec<tcl_lexer::Span> {
    let Some(class_def) = analysis.all_classes.get(class_q) else {
        return Vec::new();
    };
    let bodies = collect_member_bodies(class_def);
    let mut spans = scan_my_method_sites(source, dialect, &bodies, method, None);
    spans.extend(find_obj_method_call_sites(
        source, dialect, analysis, class_q, method,
    ));
    spans
}

/// Find a class member's declaration span plus every call
/// site inside any sibling method body.  Returns
/// `Some((decl_span, call_spans))` when the cursor sits
/// inside a class body and `word` matches one of that
/// class's members.
fn find_class_member_references(
    source: &str,
    dialect: &str,
    word: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<(tcl_lexer::Span, Vec<tcl_lexer::Span>)> {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    use tcl_lexer::Span;

    for class_def in analysis.all_classes.values() {
        let body = class_def.body_span;
        if !(body.start() < cursor_offset && cursor_offset < body.end()) {
            continue;
        }
        // Methods / classmethods: defer to the shared resolver — the *same*
        // one the code lens counts with — so the peek and the lens can never
        // drift.  It covers intra-class `my method` dispatch, external
        // `$obj method` / bare `objcmd method` sites, and the call sites of
        // any subclass that inherits (does not override) this definition
        // (which the class-local scan below would miss).
        if class_def.methods.contains_key(word) || class_def.class_methods.contains_key(word) {
            return method_references_for_class(
                source,
                dialect,
                analysis,
                &class_def.qualified_name,
                word,
            );
        }
        // Properties: no `$obj prop` dispatch and no inheritance model, so a
        // class-local `my <prop>` scan is the whole story.
        let decl_span = class_def.properties.get(word).map(|p| p.name_span)?;
        let mut call_spans: Vec<Span> = Vec::new();
        for body_span in collect_member_bodies(class_def) {
            if body_span.is_empty() {
                continue;
            }
            let mut start = body_span.start() as usize;
            let mut end = body_span.end() as usize;
            if start >= source.len() || end > source.len() || start > end {
                continue;
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
            for cmd in &commands {
                // A property is read as `$prop`, never as a command head; the
                // only command-form reference is `my <prop>` (its generated
                // accessor).  Match that, at argv[1].
                let Some(head) = cmd.argv.first() else {
                    continue;
                };
                let h_start = head.span.start() as usize;
                let h_end = head.span.end() as usize;
                if h_start >= source.len()
                    || h_end > source.len()
                    || &source[h_start..h_end] != "my"
                {
                    continue;
                }
                let Some(name_tok) = cmd.argv.get(1) else {
                    continue;
                };
                let n_start = name_tok.span.start() as usize;
                let n_end = name_tok.span.end() as usize;
                if n_start >= source.len() || n_end > source.len() {
                    continue;
                }
                if &source[n_start..n_end] != word {
                    continue;
                }
                call_spans.push(name_tok.span);
            }
        }
        return Some((decl_span, call_spans));
    }
    None
}

/// Find every external `$v method` / `[$v method]` call site
/// in the document where `v` is an instance variable whose
/// class qualified-name is `class_q` (per
/// `analysis.instance_classes`).  Returns the spans of the
/// method-name tokens.
///
/// Scans three region kinds — the top-level command stream,
/// each user proc body, and each class method body — and
/// recurses into command-substitution (`[...]`) args at every
/// level.  This covers the common call forms (`$d bark`,
/// `puts [$d bark]`, calls inside procs / methods).  Method
/// names embedded in quoted / word tokens
/// (`"prefix[$d bark]"`) are not descended — a rare form.
pub(crate) fn find_obj_method_call_sites(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
) -> Vec<tcl_lexer::Span> {
    // Variables whose `$obj method` dispatch resolves to `class_q`'s copy of
    // `method` — its own instances **plus** instances of any subclass that
    // *inherits* this definition (the subclass's MRO resolves `method` to
    // `class_q`).  An exact-class-equality filter dropped the inheriting-
    // subclass sites, silently leaving them pointing at the old name after an
    // inherited-method rename.  A subclass that *overrides* `method` resolves
    // to itself, so its instances are excluded here and rewritten under that
    // subclass's own family entry — each site is attributed to exactly one
    // family member (no double count).
    let hierarchy = analysis.class_hierarchy();
    let var_set: FxHashSet<&str> = analysis
        .instance_classes
        .iter()
        .filter(|(_, c)| {
            c.as_str() == class_q || hierarchy.method_target(c, method) == Some(class_q)
        })
        .map(|(v, _)| v.as_str())
        .collect();
    if var_set.is_empty() {
        return Vec::new();
    }
    // Receivers that are also *object commands* — bound by `CLASS create NAME`
    // (so `NAME` is a command, dispatched bare as `NAME method`, not `$NAME
    // method`).  A `set v [CLASS new]` receiver is a *variable* only and never
    // enters this set, so a bare `v method` is (correctly) not matched.
    let cmd_set: FxHashSet<&str> = var_set
        .iter()
        .copied()
        .filter(|name| analysis.created_instance_commands.contains(*name))
        .collect();
    let mut out: Vec<tcl_lexer::Span> = Vec::new();
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    let ctx = ObjMethodScan {
        source,
        dialect,
        var_set: &var_set,
        cmd_set: &cmd_set,
        method,
    };

    // Region 1: the whole document.
    {
        let mut sink = SpanSink {
            out: &mut out,
            seen: &mut seen,
        };
        scan_obj_method_region(ctx, 0, source.len(), &mut sink);
    }
    // Regions 2/3: proc + method bodies (the top-level scan
    // skips braced body args, so descend explicitly).
    for proc_def in analysis.all_procs.values() {
        let mut sink = SpanSink {
            out: &mut out,
            seen: &mut seen,
        };
        scan_obj_method_body(ctx, proc_def.body_span, &mut sink);
    }
    for class_def in analysis.all_classes.values() {
        for m in class_def
            .methods
            .values()
            .chain(class_def.class_methods.values())
            .chain(class_def.constructors.iter())
            .chain(class_def.destructor.iter())
        {
            let mut sink = SpanSink {
                out: &mut out,
                seen: &mut seen,
            };
            scan_obj_method_body(ctx, m.body_span, &mut sink);
        }
    }
    out
}

/// Read-only context for the object-method call-site scan: the document
/// `source`, its `dialect`, the `method` being looked up, and two receiver
/// sets — `var_set` matches `$v method` dispatch (variables holding an
/// object, keyed by bare name) and `cmd_set` matches `NAME method` dispatch
/// (object *commands* bound by `CLASS create NAME`).
#[derive(Clone, Copy)]
struct ObjMethodScan<'a> {
    source: &'a str,
    dialect: &'a str,
    var_set: &'a FxHashSet<&'a str>,
    cmd_set: &'a FxHashSet<&'a str>,
    method: &'a str,
}

/// Mutable sink for matched call-site spans plus the dedup set, threaded
/// alongside [`ObjMethodScan`] through the recursive scan.
struct SpanSink<'a> {
    out: &'a mut Vec<tcl_lexer::Span>,
    seen: &'a mut FxHashSet<(u32, u32)>,
}

/// Scan a brace-delimited body span for `$v method` call sites
/// (stripping the surrounding braces first).
fn scan_obj_method_body(
    ctx: ObjMethodScan<'_>,
    body_span: tcl_lexer::Span,
    sink: &mut SpanSink<'_>,
) {
    if body_span.is_empty() {
        return;
    }
    let source = ctx.source;
    let mut start = body_span.start() as usize;
    let mut end = body_span.end() as usize;
    if start >= source.len() || end > source.len() || start > end {
        return;
    }
    if source.as_bytes().get(start) == Some(&b'{') {
        start += 1;
    }
    if end > start && source.as_bytes().get(end - 1) == Some(&b'}') {
        end -= 1;
    }
    scan_obj_method_region(ctx, start, end, sink);
}

/// Segment `source[start..end]` and record every `$v method`
/// call site, recursing into command-substitution (`[...]`)
/// args.  `var_set` holds the bare names of in-scope instance
/// variables.
fn scan_obj_method_region(
    ctx: ObjMethodScan<'_>,
    start: usize,
    end: usize,
    sink: &mut SpanSink<'_>,
) {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    use tcl_lexer::TokenType;
    let source = ctx.source;
    if start >= end || end > source.len() {
        return;
    }
    let region = &source[start..end];
    let commands = segment_commands_with_offset_and_config(
        region,
        u32::try_from(start).unwrap_or(0),
        tcl_lexer::LexerConfig::for_dialect(ctx.dialect),
    );
    for cmd in &commands {
        // Head + method at argv[1].  Two dispatch shapes resolve to the same
        // receiver class: `$v method` (head is a `$`-var whose bare name is in
        // `var_set`) and `NAME method` (head is a bare word naming an object
        // command in `cmd_set`).
        if let (Some(head), Some(method_tok)) = (cmd.argv.first(), cmd.argv.get(1)) {
            let h_start = head.span.start() as usize;
            let h_end = head.span.end() as usize;
            if h_start < source.len() && h_end <= source.len() {
                let raw = &source[h_start..h_end];
                let receiver_matches = if head.kind == TokenType::Var {
                    strip_var_decoration(raw).is_some_and(|name| ctx.var_set.contains(name))
                } else {
                    // A bare-word object command (`rex bark`).  `cmd_set` holds
                    // only plain names, and a braced / bracketed / substituted
                    // head's source slice keeps its delimiters (`{rex}`, `[x]`),
                    // so an exact match admits only a genuine bare word.
                    ctx.cmd_set.contains(raw)
                };
                if receiver_matches {
                    let m_start = method_tok.span.start() as usize;
                    let m_end = method_tok.span.end() as usize;
                    if m_start < source.len()
                        && m_end <= source.len()
                        && &source[m_start..m_end] == ctx.method
                    {
                        let key = (method_tok.span.start(), method_tok.span.end());
                        if sink.seen.insert(key) {
                            sink.out.push(method_tok.span);
                        }
                    }
                }
            }
        }
        // Recurse into command-substitution args.
        for arg in &cmd.argv {
            if arg.kind != TokenType::Cmd {
                continue;
            }
            let a_start = arg.span.start() as usize;
            let a_end = arg.span.end() as usize;
            if a_start >= source.len() || a_end > source.len() || a_start >= a_end {
                continue;
            }
            // Strip the surrounding `[` `]`.
            let inner_start = if source.as_bytes().get(a_start) == Some(&b'[') {
                a_start + 1
            } else {
                a_start
            };
            let inner_end =
                if a_end > inner_start && source.as_bytes().get(a_end - 1) == Some(&b']') {
                    a_end - 1
                } else {
                    a_end
                };
            scan_obj_method_region(ctx, inner_start, inner_end, sink);
        }
    }
}

/// Strip a `$name` / `${name}` decoration to the bare variable
/// name.  Returns `None` when the text isn't a `$`-prefixed
/// reference.
fn strip_var_decoration(raw: &str) -> Option<&str> {
    let rest = raw.strip_prefix('$')?;
    let inner = rest
        .strip_prefix('{')
        .map_or(rest, |r| r.strip_suffix('}').unwrap_or(r));
    if inner.is_empty() { None } else { Some(inner) }
}

/// Read / write kind for a document-highlight span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    /// The cursor's symbol appears here as a read (`$var`,
    /// command-invocation head, etc.).
    Read,
    /// The cursor's symbol is being assigned / defined here
    /// (a `set` / `variable` / `upvar` write site, a proc
    /// declaration's name span, etc.).
    Write,
    /// The match has no read / write distinction — used for
    /// command-invocation heads whose call semantics aren't
    /// surfaced as read/write by the analyser.
    Text,
}

/// Compute the document-highlight spans for the symbol at the
/// cursor with read / write kinds.
///
/// Variables: `VarDef.definition_span` becomes `Write`; every
/// span in `VarDef.references` becomes `Read`.  Procs and
/// classes: the name span is `Write`; every matching command-
/// invocation head is `Text` (the analyser doesn't currently
/// distinguish read vs write semantics on command-invocation
/// heads, so we conservatively emit `Text`).
#[must_use]
pub fn document_highlights(
    source: &str,
    dialect: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<(LspRange, HighlightKind)> {
    let line_index = LineIndex::new(source);

    if let Some(var_name) = find_var_at_position(source, line, character) {
        let byte_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
        let Some(var_def) = crate::definition::lookup_var_in_scope_chain(
            &analysis.global_scope,
            byte_offset,
            &var_name,
        ) else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(1 + var_def.references.len());
        out.push((
            span_to_range(source, &line_index, var_def.definition_span),
            HighlightKind::Write,
        ));
        // Highlight every alias Tcl treats as one cell (namespace/global
        // aliases and a class instance variable's per-method copies).
        for r in crate::definition::linked_var_reference_spans(&analysis.global_scope, var_def) {
            out.push((span_to_range(source, &line_index, r), HighlightKind::Read));
        }
        return dedup_kinded(out);
    }

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };

    {
        let cursor_off = crate::definition::byte_offset_at(&line_index, source, line, character);
        let class_match = analysis
            .all_classes
            .iter()
            .find(|(_, c)| c.name_span.start() <= cursor_off && cursor_off < c.name_span.end())
            .or_else(|| {
                analysis.all_classes.iter().find(|(qname, c)| {
                    c.name == word || qname.as_str() == word || *qname == &format!("::{word}")
                })
            });
        if let Some((qname, class_def)) = class_match {
            let mut out = Vec::new();
            // Non-variable symbols (procs / classes / methods) highlight as
            // `Text` for both declaration and uses — only variables carry the
            // Write/Read distinction.
            out.push((
                span_to_range(source, &line_index, class_def.name_span),
                HighlightKind::Text,
            ));
            for span in class_reference_spans(analysis, qname, class_def) {
                out.push((
                    span_to_range(source, &line_index, span),
                    HighlightKind::Text,
                ));
            }
            return dedup_kinded(out);
        }
    }

    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
            let mut out = Vec::new();
            out.push((
                span_to_range(source, &line_index, proc_def.name_span),
                HighlightKind::Text,
            ));
            let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname.as_str());
            for inv in &analysis.command_invocations {
                if inv.name == proc_def.name
                    || inv.name == proc_def.qualified_name
                    || inv.name == qname_no_prefix
                    || inv
                        .resolved_qualified_name
                        .as_deref()
                        .is_some_and(|r| r == proc_def.qualified_name)
                {
                    out.push((
                        span_to_range(source, &line_index, inv.range),
                        HighlightKind::Text,
                    ));
                }
            }
            return dedup_kinded(out);
        }
    }

    // Class-member highlights — re-segment sibling method
    // bodies via `find_class_member_references` and mark the
    // declaration as Write, every call site as Text.
    let cursor_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some((decl_span, call_spans)) =
        find_class_member_references(source, dialect, &word, analysis, cursor_offset)
    {
        let mut out = Vec::new();
        out.push((
            span_to_range(source, &line_index, decl_span),
            HighlightKind::Text,
        ));
        for s in call_spans {
            out.push((span_to_range(source, &line_index, s), HighlightKind::Text));
        }
        return dedup_kinded(out);
    }

    Vec::new()
}

/// Deduplicate kinded highlight spans by (start, end) — keeps
/// the highest-kind for each duplicate range.  Write outranks
/// Read which outranks Text, so a span that the analyser
/// records both as a write and as a Read keeps the Write
/// label.
fn dedup_kinded(mut entries: Vec<(LspRange, HighlightKind)>) -> Vec<(LspRange, HighlightKind)> {
    let mut by_key: FxHashMap<(u32, u32, u32, u32), HighlightKind> = FxHashMap::default();
    for (range, kind) in &entries {
        let key = (
            range.start_line,
            range.start_character,
            range.end_line,
            range.end_character,
        );
        let kind = *kind;
        by_key
            .entry(key)
            .and_modify(|existing| {
                if priority(kind) > priority(*existing) {
                    *existing = kind;
                }
            })
            .or_insert(kind);
    }
    let mut seen: FxHashSet<(u32, u32, u32, u32)> = FxHashSet::default();
    entries.retain_mut(|(range, kind)| {
        let key = (
            range.start_line,
            range.start_character,
            range.end_line,
            range.end_character,
        );
        if !seen.insert(key) {
            return false;
        }
        *kind = by_key[&key];
        true
    });
    entries
}

fn priority(kind: HighlightKind) -> u8 {
    match kind {
        HighlightKind::Write => 2,
        HighlightKind::Read => 1,
        HighlightKind::Text => 0,
    }
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

fn dedup_ranges(ranges: &mut Vec<LspRange>) {
    let mut seen: FxHashSet<(u32, u32, u32, u32)> = FxHashSet::default();
    ranges.retain(|r| {
        let key = (r.start_line, r.start_character, r.end_line, r.end_character);
        seen.insert(key)
    });
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
    fn references_to_proc_include_decl_and_calls() {
        let src = "proc greet {} {}\ngreet\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the first `greet` reference (line 1).
        let refs = references(src, "tcl", 1, 2, &analysis, true);
        assert!(refs.len() >= 2, "expected decl + call sites: {refs:?}");
        // First entry is the declaration on line 0.
        assert_eq!(refs[0].start_line, 0);
    }

    #[test]
    fn references_exclude_decl_when_flag_false() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let with_decl = references(src, "tcl", 1, 2, &analysis, true);
        let without_decl = references(src, "tcl", 1, 2, &analysis, false);
        assert!(with_decl.len() > without_decl.len());
    }

    #[test]
    fn references_to_unknown_word_empty() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(references(src, "tcl", 0, 6, &analysis, true).is_empty());
    }

    #[test]
    fn references_to_var_includes_definition_and_uses() {
        let src = "set x 1\nputs $x\nputs $x\n";
        let analysis = analyse(src);
        // Cursor on `$x` first reference.
        let refs = references(src, "tcl", 1, 7, &analysis, true);
        // The analyser may or may not record the literal `$x`
        // as a reference depending on lowering; at minimum the
        // declaration should land in the result list.
        assert!(!refs.is_empty(), "{refs:?}");
        assert!(refs.iter().any(|r| r.start_line == 0));
    }

    // read/write distinction

    #[test]
    fn document_highlights_var_records_write_at_definition() {
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        // Cursor inside `$x`.
        let highlights = document_highlights(src, "tcl", 1, 7, &analysis);
        // The defining `set x` span should be tagged Write.
        let writes: Vec<_> = highlights
            .iter()
            .filter(|(_, k)| *k == HighlightKind::Write)
            .collect();
        assert!(
            !writes.is_empty(),
            "expected at least one Write for `set x 1`; got {highlights:?}",
        );
        // The Write should be on line 0 (the `set` line).
        assert!(
            writes.iter().any(|(r, _)| r.start_line == 0),
            "expected Write on line 0; got {highlights:?}",
        );
    }

    #[test]
    fn document_highlights_var_read_kind_is_correctly_tagged() {
        // The kind-tagging contract: every span in
        // `VarDef.references` becomes Read; the definition
        // span becomes Write.  Whether the analyser actually
        // populates `references` for a given source depends
        // on its body-walk heuristics (single-arg `set x`
        // reads are tracked, `$x` substitutions in arg
        // positions are not).  This
        // test injects a synthetic `VarDef` with a known
        // `references` entry to verify the tagging logic in
        // isolation from the body-walk gap.
        use tcl_compiler::analyser::{AnalysisResult as Result, Scope, VarDef};
        use tcl_lexer::Span;
        let mut scope = Scope::default();
        scope.variables.insert(
            "x".into(),
            VarDef {
                name: "x".into(),
                definition_span: Span::new(4, 5),
                references: vec![Span::new(13, 14)],
                warn_if_unused: false,
                array_indices: std::collections::BTreeSet::new(),
                link_target: None,
            },
        );
        let a = Result {
            global_scope: scope,
            ..Result::default()
        };
        // Source matches the spans we injected so
        // line/character translation works.
        let src = "set x 1\nputs $x\n";
        let highlights = document_highlights(src, "tcl", 1, 6, &a);
        // Write at definition.
        assert!(
            highlights
                .iter()
                .any(|(r, k)| r.start_line == 0 && *k == HighlightKind::Write),
            "expected Write at line 0; got {highlights:?}",
        );
        // Read at the injected reference.
        assert!(
            highlights
                .iter()
                .any(|(r, k)| r.start_line == 1 && *k == HighlightKind::Read),
            "expected Read at line 1; got {highlights:?}",
        );
    }

    #[test]
    fn document_highlights_proc_decl_is_text() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let highlights = document_highlights(src, "tcl", 0, 6, &analysis);
        // Declaration on line 0 should be Text — procs carry no Write/Read
        // distinction (only variables do).
        let line0 = highlights
            .iter()
            .find(|(r, k)| r.start_line == 0 && *k == HighlightKind::Text);
        assert!(
            line0.is_some(),
            "expected Text on line 0 (declaration); got {highlights:?}",
        );
        // Call site on line 1 should be Text (no read/write
        // semantics on command-invocation heads).
        assert!(
            highlights
                .iter()
                .any(|(r, k)| r.start_line == 1 && *k == HighlightKind::Text),
            "expected Text on line 1 (call site); got {highlights:?}",
        );
    }

    #[test]
    fn document_highlights_empty_for_unknown_symbol() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(document_highlights(src, "tcl", 0, 6, &analysis).is_empty());
    }

    // resolved-qualified-name matching

    #[test]
    fn resolved_qualified_name_matches_call_site_from_namespace() {
        // Source: a proc defined at the top level, called from
        // a namespace.  The call site's literal name (`greet`)
        // matches the proc name; the resolved qualified name
        // also matches.  We pin that the references provider
        // finds the call site.
        let src = "proc ::greet {} {}\nnamespace eval ::myns {\n    greet\n}\n";
        let analysis = analyse(src);
        // Cursor on the proc declaration.
        let refs = references(src, "tcl", 0, 8, &analysis, true);
        // Should include the declaration and the call site.
        assert!(
            refs.len() >= 2,
            "expected proc decl + namespace call site; got {refs:?}",
        );
    }

    #[test]
    fn document_highlights_surfaces_var_reads_from_arg_positions() {
        // With `record_arg_var_reads`, `$x`
        // reads in command arguments populate
        // `VarDef.references` and surface as `Read` spans in
        // the document-highlight provider.
        let src = "set x 1\nputs $x\nputs $x\n";
        let analysis = analyse(src);
        let highlights = document_highlights(src, "tcl", 1, 6, &analysis);
        let reads: Vec<_> = highlights
            .iter()
            .filter(|(_, k)| *k == HighlightKind::Read)
            .collect();
        assert!(
            reads.len() >= 2,
            "expected >= 2 Read entries (for two `$x` sites); got {highlights:?}",
        );
        // The defining `set x` span is Write.
        assert!(
            highlights
                .iter()
                .any(|(r, k)| r.start_line == 0 && *k == HighlightKind::Write),
            "expected Write on line 0; got {highlights:?}",
        );
    }

    #[test]
    fn resolved_qualified_name_field_populated_for_simple_call() {
        // Verify that the analyser actually populates
        // `resolved_qualified_name` on
        // `command_invocations`.  At the top level a `greet`
        // call should resolve to `::greet`.
        let src = "greet hi\n";
        let analysis = analyse(src);
        let inv = analysis
            .command_invocations
            .iter()
            .find(|i| i.name == "greet")
            .expect("expected a `greet` invocation");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::greet"),
            "expected resolved name to be `::greet`; got {inv:?}",
        );
    }

    // class-member references

    #[test]
    fn references_for_method_includes_decl_and_call_sites() {
        // Intra-class dispatch uses `my <method>` (the dispatch form tclsh
        // accepts; a bare `greet` head errors with "invalid command name", so it
        // is not a reference).
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `greet` declaration (line 1, col 11).
        let refs = references(src, "tcl", 1, 11, &analysis, true);
        assert!(refs.len() >= 3, "expected ≥3 refs; got {refs:?}");
    }

    #[test]
    fn references_for_method_excludes_decl_when_requested() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        let refs = references(src, "tcl", 1, 11, &analysis, false);
        // Only the two call sites — the declaration is
        // excluded when include_declaration=false.
        assert_eq!(refs.len(), 2, "{refs:?}");
    }

    #[test]
    fn document_highlights_for_method_marks_decl_and_calls_text() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        let h = document_highlights(src, "tcl", 1, 11, &analysis);
        // Methods carry no Write/Read distinction — declaration + both call
        // sites are all Text (only variables are Write/Read).
        let writes = h.iter().filter(|(_, k)| *k == HighlightKind::Write).count();
        let texts = h.iter().filter(|(_, k)| *k == HighlightKind::Text).count();
        assert_eq!(writes, 0, "{h:?}");
        assert_eq!(texts, 3, "{h:?}");
    }

    // external $obj method sites

    #[test]
    fn references_from_external_obj_method_site() {
        // Declaration + 2 external call sites (`$d bark`,
        // `[$d bark]`).
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
        let analysis = analyse(src);
        // Cursor on `bark` in `$d bark` (line 4, col 3).
        let refs = references(src, "tcl", 4, 3, &analysis, true);
        // Declaration (line 1) + two external sites (lines 4, 5).
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(lines.contains(&4), "line-4 call missing: {refs:?}");
        assert!(lines.contains(&5), "line-5 call missing: {refs:?}");
    }

    #[test]
    fn references_from_inside_class_includes_external_sites() {
        // Cursor on the declaration; refs include the external
        // `$d bark` site as well as the declaration.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
        let analysis = analyse(src);
        let refs = references(src, "tcl", 1, 11, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(lines.contains(&4), "external call missing: {refs:?}");
    }

    #[test]
    fn find_obj_method_call_sites_covers_top_level_and_subst() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Dog", "bark");
        // Two external sites: `$d bark` and `[$d bark]`.
        assert_eq!(sites.len(), 2, "{sites:?}");
    }

    #[test]
    fn find_obj_method_call_sites_finds_calls_in_proc_body() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nproc f {} { $d bark }\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Dog", "bark");
        assert_eq!(sites.len(), 1, "{sites:?}");
    }

    #[test]
    fn find_obj_method_call_sites_matches_bare_created_instance_command() {
        // `Dog create rex` binds `rex` as an object *command*; `rex bark` is a
        // bare-word method dispatch (not `$rex bark`).  The scan resolves it
        // through `created_instance_commands`.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Dog", "bark");
        assert_eq!(sites.len(), 1, "{sites:?}");
        // The matched span is the `bark` method-name token of `rex bark`.
        let s = sites[0];
        assert_eq!(
            &src[s.start() as usize..s.end() as usize],
            "bark",
            "{sites:?}"
        );
    }

    #[test]
    fn references_include_bare_created_instance_command_site() {
        // Full peek: cursor on the `method bark` decl surfaces the bare
        // `rex bark` dispatch as a reference.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
        let analysis = analyse(src);
        let refs = references(src, "tcl", 1, 11, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(lines.contains(&4), "bare `rex bark` site missing: {refs:?}");
    }

    #[test]
    fn references_from_cursor_on_bare_obj_command_call_site() {
        // Codex #881 (symmetry): invoking Find All References with the cursor
        // ON the `bark` token of a bare `rex bark` dispatch must resolve — not
        // only the declaration-based peek.  `rex` is at col 0, `bark` at col 4.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
        let analysis = analyse(src);
        let refs = references(src, "tcl", 4, 4, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(lines.contains(&4), "call site missing: {refs:?}");
    }

    #[test]
    fn bare_set_var_receiver_is_not_matched_without_dollar() {
        // FP guard: `set d [Dog new]` binds `d` as a *variable*, not a
        // command.  A bare `d bark` (no `$`) is NOT a valid dispatch in Tcl,
        // so it must not be matched — only `$d bark` counts.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nd bark\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Dog", "bark");
        assert!(
            sites.is_empty(),
            "bare var receiver wrongly matched: {sites:?}"
        );
    }
}
