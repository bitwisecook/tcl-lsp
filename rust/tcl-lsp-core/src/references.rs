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

//! Find-references and document-highlight source selection.
//!
//! Native naming profiles use current original namespace, variable, method and
//! declaration owners. Each query retains the complete source image, full lexer
//! configuration and selected policy. A missing or stale original input is
//! terminal; a display spelling cannot replace it.
//!
//! [`references`] and [`document_highlights`] expose document-local ranges.
//! Original namespace occurrences share [`crate::namespace_symbol`]; original
//! variable occurrences share [`crate::variable_symbol`] and retain their
//! actual source frame and declaration/read/write roles. These source symbols
//! do not grant runtime cell identity, contents or observer closure.
//!
//! Procedure and class references use
//! [`crate::original_declaration::invocation_targets_declaration`] to match the
//! actual positioned lookup and canonical allocation. A terminal alias or moved
//! route can retain a call edge while its own spelling remains distinct from a
//! direct editable declaration reference. Repeated declarations and equal-byte
//! names with independent owners remain separate.
//!
//! Method references retain the selected entry and its genuine source worker.
//! [`crate::method_symbol`] keeps class, instance and own-object ownership
//! separate, including own-object allocation/generation and private or missing
//! entry barriers. Possible source candidates supply readonly advice; they do
//! not establish native dispatch, inheritance or edit permission.
//!
//! The server's shared original declaration document inventory carries each
//! participating URI, current source and configuration independently. Workspace
//! references, call edges and code-lens counts reuse the same canonical
//! declaration reference kernel. Unknown relevant providers, stale source or
//! ambiguous declarations decline selection rather than losing a blocker.
//! Including a declaration is an independent request choice.
//!
//! Highlights use retained variable roles for `Read` and `Write`; command calls
//! remain `Text`, with declaration geometry represented separately. Exact
//! expression function occurrences have an independent lookup purpose and do
//! not acquire command identity through their rendered identifier.
//!
//! Explicit lexical-advice profiles retain the reporting-map and segmented
//! member/body compatibility scans below. Definition metadata hazards use
//! genuine original body and class-source owners with the actual Registry
//! context; readonly source references cannot provide a Native original
//! query, entered worker or edit permission.

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::ir::MethodKind;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

mod member_metadata;

mod call_sites;
mod callback_targets;
mod dispatch_regions;
mod highlights;
mod member_references;
mod next_dispatch;
mod object_method_sites;
mod self_dispatch_scan;
mod unentered_callbacks;

#[cfg(test)]
pub(crate) use call_sites::invocation_references_named;
use call_sites::logical_proc_at_cursor;
#[cfg(test)]
use call_sites::{
    RetainedDefinition, logical_class_reference_spans, logical_proc_reference_spans,
    retained_definition,
};
pub(crate) use call_sites::{
    class_reference_spans, invocation_calls_named, invocation_calls_proc,
    invocation_references_class, invocation_references_proc, proc_reference_spans,
};
pub use callback_targets::callback_prefix_method_receiver_at_cursor;
#[cfg(test)]
use callback_targets::source_span_text;
pub(crate) use callback_targets::{
    authored_callback_reference_spans, list_built_self_method_target_at_cursor,
    unproved_callback_method_selector,
};
#[cfg(test)]
pub(crate) use dispatch_regions::strip_outer_braces;
pub(crate) use dispatch_regions::{
    DispatchRegionCensus, MAX_DISPATCH_SCAN_DEPTH, dispatch_scan_regions, dispatch_source_context,
    frame_shifted_dispatch_regions, nested_dispatch_regions,
};
#[cfg(test)]
use highlights::variable_occurrence_highlights;
pub use highlights::{HighlightKind, document_highlights, document_highlights_in_program};
use member_references::find_class_member_references;
pub use member_references::{InheritedReceiverFacts, method_reference_spans_in_document};
pub(crate) use member_references::{
    MemberSel, collect_member_bodies, collect_member_bodies_scoped, inherited_method_call_sites,
    member_reference_inventory, member_reference_spans, method_references_for_class,
    method_references_for_declaration, resolve_member_span,
};
pub use next_dispatch::method_next_dispatch_spans;
pub(crate) use next_dispatch::{
    constructor_next_chain_references, destructor_next_chain_references,
    original_special_next_chain_spans, scan_next_dispatch_sites,
    scan_next_dispatch_sites_with_target,
};
#[cfg(test)]
pub(crate) use object_method_sites::find_obj_method_call_sites;
#[cfg(test)]
use object_method_sites::itcl_class_proc_call_sites;
pub(crate) use object_method_sites::logical_class_command_method_sites;
pub use object_method_sites::obj_method_call_sites;
#[cfg(test)]
use self_dispatch_scan::scan_method_sites_by_kind;
pub(crate) use self_dispatch_scan::{scan_method_sites, scan_my_method_sites};
pub(crate) use unentered_callbacks::unentered_callback_reference_spans;

/// Compute the locations of every reference to the symbol at
/// the cursor.
///
/// `include_declaration` mirrors the LSP `ReferenceContext`
/// flag — when `true`, the symbol's defining span is the first
/// element of the returned vector.
#[must_use]
pub fn references(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    include_declaration: bool,
) -> Vec<LspRange> {
    references_in_program(
        source,
        dialect,
        line,
        character,
        analysis,
        include_declaration,
        None,
    )
}

/// [`references`] with the caller's whole-program export view attached — the
/// entry point a host with a workspace index should call.
///
/// `program` is `None` for a host without one, which reproduces [`references`]
/// exactly. It matters because a `namespace import -force` whose covering
/// `namespace export` lives in another file changes *which* definition a bare
/// call is a reference to, and find-references has to
/// answer that the same way go-to-definition does.
///
/// A call whose `-force` shadow only that view decides is not a proven
/// occurrence of either definition (user decision 2026-10-22T07:18Z,
/// `force_import_shadow_direction`), so it is never reported.
#[must_use]
pub fn references_in_program(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    include_declaration: bool,
    program: Option<crate::definition::ProgramExports<'_>>,
) -> Vec<LspRange> {
    let mut ranges = references_in_program_unchecked(
        source,
        dialect,
        line,
        character,
        analysis,
        include_declaration,
        program,
    );
    if program.is_some() {
        let line_index = LineIndex::new(source);
        ranges.retain(|range| {
            !crate::definition::program_forced_import_abstains(
                analysis,
                program,
                crate::definition::byte_offset_at(
                    &line_index,
                    source,
                    range.start_line,
                    range.start_character,
                ),
            )
        });
    }
    ranges
}

fn references_in_program_unchecked(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    include_declaration: bool,
    program: Option<crate::definition::ProgramExports<'_>>,
) -> Vec<LspRange> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
    if crate::definition::program_forced_import_abstains(analysis, program, cursor) {
        return Vec::new();
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::namespace_symbol::select_at_offset(source, analysis, cursor)
    {
        return selected.map_or_else(Vec::new, |symbol| {
            crate::namespace_symbol::original_namespace_spans(
                analysis,
                &symbol,
                include_declaration,
            )
            .into_iter()
            .map(|span| crate::definition::span_to_range(source, &line_index, span))
            .collect()
        });
    }

    if let std::ops::ControlFlow::Break(selected) = crate::variable_symbol::select_navigation(
        source,
        analysis,
        line,
        character,
        crate::definition::CallResolution {
            registry: None,
            program,
        },
    ) {
        return selected.map_or_else(Vec::new, |occurrence| {
            occurrence
                .reference_spans(source, analysis, include_declaration)
                .into_iter()
                .map(|span| span_to_range(source, &line_index, span))
                .collect()
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::method_symbol::local_candidate(source, analysis, line, character)
    {
        return selected.map_or_else(Vec::new, |candidate| {
            crate::method_symbol::readonly_local_reference_report(
                source,
                analysis,
                &candidate,
                include_declaration,
            )
            .map_or_else(
                Vec::new,
                crate::original_declaration::ReadonlyReferenceReport::into_spans,
            )
            .into_iter()
            .map(|span| span_to_range(source, &line_index, span))
            .collect()
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::original_declaration::select("", source, analysis, line, character)
    {
        return selected.map_or_else(Vec::new, |identity| {
            let mut spans = crate::original_declaration::readonly_reference_report(
                &identity, source, analysis, true,
            )
            .map_or_else(
                Vec::new,
                crate::original_declaration::ReadonlyReferenceReport::into_spans,
            );
            if include_declaration {
                spans.push(identity.span());
            }
            spans.sort_by_key(|span| (span.start(), span.end()));
            spans.dedup();
            spans
                .into_iter()
                .map(|span| span_to_range(source, &line_index, span))
                .collect()
        });
    }
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let ctx = RefCtx {
        source,
        dialect,
        line_index: &line_index,
        line,
        character,
        analysis,
        include_declaration,
        resolution: crate::definition::CallResolution {
            registry: None,
            program,
        },
    };

    lexical_references(&ctx)
}

fn lexical_references(ctx: &RefCtx<'_>) -> Vec<LspRange> {
    let RefCtx {
        source,
        dialect,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
        ..
    } = *ctx;
    if let Some(out) = variable_references(ctx) {
        return out;
    }

    // A `$`-led read is definitive even when nothing resolved: Tcl's variable
    // and command namespaces are disjoint, so falling through to the bareword
    // resolvers below would answer a caller-frame `$dataset` read with the
    // declaration of an unrelated same-named TclOO method.
    // `variable_references` returns `None` for both "not a variable
    // position" and "a variable that resolved to nothing", so the stop has to
    // be made here, on the token kind.
    if crate::caller_frame::substituted_var_read_at(
        source,
        analysis,
        line,
        character,
        crate::definition::byte_offset_at(line_index, source, line, character),
    )
    .is_some()
    {
        return Vec::new();
    }

    // Namespace references — a word the registry marks
    // `ArgRole::NamespaceName`.  Checked before every bareword
    // resolver below because it is span-precise: the cursor is provably
    // inside a namespace-name argument, so a class or proc that happens to
    // share the spelling must not claim it.  Namespaces are their own symbol
    // space in Tcl, disjoint from commands and variables.
    if let Some(out) = namespace_references(ctx) {
        return out;
    }

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };

    let cursor_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    if let Some(selected) =
        crate::receiver_identity::method_at_cursor(analysis, source, cursor_offset)
        && selected.method.name == word
    {
        let (decl, sites) = method_references_for_declaration(
            source,
            dialect,
            analysis,
            selected.class,
            selected.method,
            selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class,
        );
        let mut out = Vec::new();
        if include_declaration {
            out.push(span_to_range(source, line_index, decl));
        }
        out.extend(
            sites
                .into_iter()
                .map(|span| span_to_range(source, line_index, span)),
        );
        dedup_ranges(&mut out);
        return out;
    }

    if crate::receiver_identity::definition_reference_at_cursor(analysis, source, cursor_offset)
        .is_some()
    {
        return Vec::new();
    }

    // Class references (checked first, before proc names).
    if let Some(out) = class_references(ctx, &word) {
        return out;
    }

    // `<ensemble> <subcommand>` — a static `namespace ensemble create
    // -map`/`-subcommands` mapping. Checked before
    // `proc_references`: real Tcl never independently looks up `make` as a
    // command (only the pair `widget make` dispatches), so a coincidental
    // same-named proc elsewhere in the workspace — which `proc_references`'s
    // namespace-aware call-site resolution could otherwise match — must
    // never win.
    if let Some(out) = ensemble_subcommand_references(ctx) {
        return out;
    }

    // Proc references.
    if let Some(out) = proc_references(ctx, &word) {
        return out;
    }

    // `Factory::make` — [incr Tcl]'s colon-qualified class-proc dispatch
    // Tried *after* `proc_references` so an ordinary
    // namespace-qualified proc call of the same spelling keeps priority:
    // this only ever fires when nothing resolves as a proc and the written
    // word's parent namespace really is an itcl class declaring that member.
    if let Some(out) = itcl_class_proc_references(ctx) {
        return out;
    }

    // `$obj method` external call site.
    if let Some(out) = instance_method_references(ctx) {
        return out;
    }

    // Bare `ClassName method` external call site — a classmethod's own
    // dispatch shape, tried only once the instance path above has failed.
    if let Some(out) = classmethod_call_site_references(ctx) {
        return out;
    }

    // `constructor` / `destructor` keyword — the next-chain reference
    // story; neither has a name to dispatch on, so no `my`/`$obj`
    // call-site scan applies the way it does for a method. Tried *before*
    // the ordinary class-member lookup below: a class may legally also
    // declare a `method`/`property` literally named `constructor` or
    // `destructor` (the keyword form and a same-named ordinary member are
    // independent), and `class_member_references`'s cursor-outside-any-span
    // fallback (see `resolve_member_span`) would otherwise claim a cursor
    // sitting on the special keyword token for that unrelated same-named
    // member instead. This resolver only ever
    // matches when the cursor sits strictly on the keyword's own name span,
    // so trying it first never steals a real member reference.
    if let Some(out) = constructor_or_destructor_references(ctx, &word) {
        return out;
    }

    // Class-member references (cursor inside a class body on a member name).
    if let Some(out) = class_member_references(ctx, &word) {
        return out;
    }

    Vec::new()
}

/// References for the **namespace** the cursor names — every other spelling
/// of it in this document, plus its declaring `namespace eval` blocks when
/// `include_declaration`.
///
/// `None` means the cursor is not on a namespace-name word at all, so the
/// caller falls through.  `Some` is definitive, empty vector included: once
/// the position is provably a namespace reference, an unrelated same-spelled
/// proc or class is not the answer.
///
/// Resolution — including a relative name's rooting against the enclosing
/// namespace — happens once, in
/// [`crate::namespace_symbol::namespace_cell_at`], so this and
/// go-to-definition and hover cannot disagree about which namespace is meant.
fn namespace_references(ctx: &RefCtx<'_>) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
        ..
    } = *ctx;
    let cursor = crate::definition::byte_offset_at(line_index, source, line, character);
    if let std::ops::ControlFlow::Break(selected) =
        crate::namespace_symbol::select_at_offset(source, analysis, cursor)
    {
        return Some(selected.map_or_else(Vec::new, |symbol| {
            crate::namespace_symbol::original_namespace_spans(
                analysis,
                &symbol,
                include_declaration,
            )
            .into_iter()
            .map(|span| crate::definition::span_to_range(source, line_index, span))
            .collect()
        }));
    }
    let cell = crate::namespace_symbol::namespace_cell_at(source, analysis, line, character)?;
    Some(
        crate::namespace_symbol::namespace_all_spans(analysis, &cell, include_declaration)
            .into_iter()
            .map(|span| crate::definition::span_to_range(source, line_index, span))
            .collect(),
    )
}

/// Build references for a `constructor` / `destructor` keyword: when the
/// cursor sits inside a class body on the keyword token of that class's own
/// *effective* declaration (the last `constructor`, or the single
/// `destructor`), surface its declaration plus every `next`/`nextto` site —
/// in *any* class — whose MRO target resolves to it
/// ([`constructor_next_chain_references`] / [`destructor_next_chain_references`]).
///
/// A cursor on a shadowed (non-last) `constructor` declaration resolves to
/// nothing here — `oo::configurable` allows several, but only the last is
/// ever reachable, so an earlier one has no reference story worth surfacing.
fn constructor_or_destructor_references(ctx: &RefCtx<'_>, word: &str) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        dialect,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
        ..
    } = *ctx;
    let cursor_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    let class_def = analysis
        .all_classes
        .values()
        .find(|cd| cd.body_span.start() < cursor_offset && cursor_offset < cd.body_span.end())?;
    // The effective constructor or the destructor whose declaring keyword the
    // cursor is on — the recorded member says which it is, not the word.
    let member = class_def
        .constructors
        .last()
        .into_iter()
        .chain(class_def.destructor.as_ref())
        .find(|md| {
            md.is_declared_by_keyword(word)
                && md.name_span.start() <= cursor_offset
                && cursor_offset <= md.name_span.end()
        })?;
    let (decl_span, call_spans) = if member.kind == MethodKind::Constructor.as_str() {
        constructor_next_chain_references(source, dialect, analysis, &class_def.qualified_name)
    } else {
        destructor_next_chain_references(source, dialect, analysis, &class_def.qualified_name)
    }?;
    Some(build_member_ranges(
        source,
        line_index,
        decl_span,
        call_spans,
        include_declaration,
    ))
}

/// Shared immutable inputs for the per-kind reference resolvers, so each
/// helper takes a single context instead of re-threading the same seven
/// parameters.
#[derive(Clone, Copy)]
struct RefCtx<'a> {
    source: &'a str,
    dialect: &'static tcl_dialect::DialectProfile,
    line_index: &'a LineIndex,
    line: u32,
    character: u32,
    analysis: &'a AnalysisResult,
    include_declaration: bool,
    /// Everything outside this document that a call-site resolution may
    /// consult — in practice the whole-program export oracle. Default
    /// (`document_only`) for a host with no workspace index.
    resolution: crate::definition::CallResolution<'a>,
}

/// Find-All-References for a **caller-frame** variable — one no statement in
/// this frame assigns because a callee creates it here through `upvar`.
///
/// The reference set is deliberately both halves of the idiom: the bare
/// call-site word that names the variable *and* every `$name` read it feeds.
/// `include_declaration` selects the **creating** call-site words, which are
/// the nearest thing the frame has to a declaration; a call site whose callee
/// only upvar-*reads* the alias is a plain reference and survives either way.
/// `None` (rather than an empty set) when no call site binds the name, so the
/// caller keeps abstaining.
fn caller_frame_references(
    ctx: &RefCtx<'_>,
    byte_offset: u32,
    name: &str,
) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        dialect,
        line_index,
        analysis,
        include_declaration,
        resolution,
        ..
    } = *ctx;
    // The caller's whole-program view, not a fresh document-only one: which
    // proc a binding's call-site word reaches is itself a call resolution, so
    // dropping the oracle here would let find-references disagree with
    // go-to-definition on a `-force`-shadowed callee.
    let bindings = crate::caller_frame::caller_frame_bindings(
        analysis,
        source,
        dialect,
        resolution,
        byte_offset,
        name,
    );
    if bindings.is_empty() {
        return None;
    }
    // Only a *creating* call site is the declaration. A `read_only` binding —
    // a callee that upvar-READS through the alias and never writes it
    // (`peek x`) — is an ordinary reference to the variable, so dropping it
    // with `include_declaration = false` would lose a real use.
    let declarations: Vec<tcl_lexer::Span> = bindings
        .iter()
        .filter(|b| !b.read_only)
        .map(|b| b.arg_span)
        .collect();
    let mut out: Vec<LspRange> = crate::caller_frame::caller_frame_reference_spans(
        analysis,
        source,
        dialect,
        resolution,
        byte_offset,
        name,
    )
    .into_iter()
    .filter(|span| include_declaration || !declarations.contains(span))
    .map(|span| span_to_range(source, line_index, span))
    .collect();
    dedup_ranges(&mut out);
    Some(out)
}

/// Build `decl + references` ranges for a variable at the cursor.
fn variable_references(ctx: &RefCtx<'_>) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        dialect,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
        resolution,
        ..
    } = *ctx;
    let byte_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    // Resolved through the shared gate, not the raw character scan: the
    // occurrence must be one Tcl actually substitutes — a `$name`-shaped
    // substring in a comment or a data brace is not a reference, and neither
    // is the `$n` inside a brace-quoted *name* word (`set {$n} 1`), which
    // falls through to the declaration-span search in the final `else` so it
    // answers the literal cell.
    let var_def = if let Some(var_name) = crate::definition::substituting_var_at_position(
        source,
        analysis,
        line,
        character,
        byte_offset,
    ) {
        match crate::definition::lookup_var_read_at(analysis, source, byte_offset, &var_name) {
            Some(def) => def,
            // Nothing in this frame assigns it — but a callee may create it
            // here through `upvar`, in which case the call-site word that
            // names it and every `$name` read are one variable.
            None => return caller_frame_references(ctx, byte_offset, &var_name),
        }
    } else if let Some(binding) = crate::caller_frame::binding_at_offset(
        analysis,
        source,
        dialect,
        resolution,
        byte_offset,
        &find_word_span_at_position(source, line, character).map_or_default(|(w, _, _)| w),
    ) {
        // The cursor is on the bare call-site word itself.
        let name = source
            .get(binding.arg_span.as_range())
            .unwrap_or_default()
            .to_owned();
        return caller_frame_references(ctx, byte_offset, &name);
    } else {
        // Bareword declaration / same-cell write site (a `set x`/
        // `variable x` target, a proc/method parameter, a `catch`
        // result-var), not a `$`-prefixed read. See
        // `var_def_at_declaration_offset`'s own doc for why this needs a
        // dedicated byte-offset span search rather than the ordinary
        // scope-chain walk.
        crate::definition::var_def_at_declaration_offset(&analysis.global_scope, byte_offset)?
    };
    let mut out = Vec::new();
    if include_declaration {
        out.push(span_to_range(source, line_index, var_def.definition_span));
    }
    // Readonly alias candidates retain their independent binding obligations.
    for r in crate::definition::linked_var_reference_spans(source, analysis, var_def, byte_offset)?
    {
        out.push(span_to_range(source, line_index, r));
    }
    dedup_ranges(&mut out);
    Some(out)
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
    let (qname, class_def) = if analysis.allows_retained_logical_declaration_advice() {
        let cursor = crate::original_context::CurrentSourcePosition::capture(
            source, analysis, line, character,
        )?
        .offset();
        let image = tcl_lexer::SourceImage::document(source);
        let mut classes = Vec::new();
        for candidate in
            tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates(
                source, analysis,
            )?
        {
            let Some(head) = candidate.original_head() else {
                continue;
            };
            if head.span().start() <= cursor
                && cursor < head.span().end()
                && let Some(call) =
                    tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
                        source,
                        analysis,
                        head.span().start(),
                    )
                && let Some(class) = call.class_declaration().logical_source_class(analysis)
            {
                classes.push(class);
            }
        }
        for reference in
            tcl_compiler::registry_invocation::source_structure::source_logical_name_references(
                source, analysis,
            )?
        {
            if reference.operand().span().start() <= cursor
                && cursor < reference.operand().span().end()
                && let Some(class) = reference.class_declaration(analysis)
            {
                classes.push(class);
            }
        }
        for callback in
            tcl_compiler::registry_invocation::source_structure::source_logical_callback_candidates(
                source, analysis,
            )?
        {
            if callback
                .head()
                .source_span()
                .is_some_and(|span| span.start() <= cursor && cursor < span.end())
                && let Some(class) = callback.class_declaration(analysis)
            {
                classes.push(class);
            }
        }
        if let Some(class) =
            analysis.logical_source_class_at(&image, analysis.body_lexer_config?, cursor)
        {
            classes.push(class);
        }
        let class = *classes.first()?;
        classes
            .iter()
            .all(|other| std::ptr::eq(*other, class))
            .then_some(())?;
        analysis.logical_source_class_declaration(class)?;
        (&class.qualified_name, class)
    } else {
        crate::definition::resolve_class_target_at(
            analysis,
            source,
            ctx.resolution,
            cursor_off,
            word,
        )?
    };
    let mut out = Vec::new();
    if include_declaration {
        out.push(span_to_range(source, line_index, class_def.name_span));
    }
    // `superclass <C>` / `mixin <C>` (and `forward … TARGET`) usages are
    // ordinary command references — the analyser records each as a
    // `command_invocation` resolved in the referencing class's namespace — so
    // `class_reference_spans` (over `command_invocations`) already covers them,
    // in this document and, via the workspace index, across files.  Rename and
    // the code-lens count read the same collection, so the three never diverge.
    for span in class_reference_spans(analysis, ctx.resolution, qname, class_def, source) {
        out.push(span_to_range(source, line_index, span));
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
    // The caller's whole-program view, not a fresh document-only one: the
    // span pass below already filters with `ctx.resolution`, so resolving the
    // *target* without it would make this function disagree with itself: it
    // would seed from the local definition a `-force` import deleted and then
    // drop every span that definition owns.
    let (qname, proc_def) = if analysis.allows_retained_logical_declaration_advice() {
        let cursor = crate::original_context::CurrentSourcePosition::capture(
            source, analysis, line, character,
        )?
        .offset();
        let metadata = logical_proc_at_cursor(source, analysis, cursor)?.metadata();
        (&metadata.qualified_name, metadata)
    } else {
        crate::definition::resolve_proc_target_at(
            analysis,
            source,
            cursor_off,
            word,
            ctx.resolution,
        )?
    };
    let mut out = Vec::new();
    if include_declaration {
        out.push(span_to_range(source, line_index, proc_def.name_span));
    }
    for span in proc_reference_spans(analysis, ctx.resolution, qname, proc_def, source) {
        out.push(span_to_range(source, line_index, span));
    }
    dedup_ranges(&mut out);
    Some(out)
}

/// Build references for an ensemble-subcommand call site: when the cursor
/// sits on the subcommand word of a `<ensemble> <subcommand>` call and it
/// resolves through a static `namespace ensemble create -map`/
/// `-subcommands` mapping, surface the target proc's declaration plus every
/// call site — the reference twin of `definition()`'s identical check.
/// The actual per-call-site matching needs no new
/// code: `proc_reference_spans` already matches on `resolved_qualified_name`
/// (not `inv.name == def_name`), and `record_ensemble_subcommand_invocation`
/// (analyser side) already carries the target's resolved name on every
/// subcommand call site — so rename / call-hierarchy / code-lens reference
/// counts pick this up automatically too, no separate changes needed there.
fn ensemble_subcommand_references(ctx: &RefCtx<'_>) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
        ..
    } = *ctx;
    let (head, sub, is_dollar) =
        crate::definition::original_instance_method_at_cursor(source, analysis, line, character)?;
    if is_dollar {
        return None;
    }
    let cursor_off = crate::definition::byte_offset_at(line_index, source, line, character);
    let namespace = crate::definition::namespace_context_at(
        &analysis.global_scope,
        cursor_off,
        &analysis.namespace_overrides,
    );
    let target = crate::definition::ensemble_subcommand_target(analysis, &namespace, &head, &sub)?;
    let (qname, proc_def) = analysis.all_procs.get_key_value(target)?;
    let mut out = Vec::new();
    if include_declaration {
        out.push(span_to_range(source, line_index, proc_def.name_span));
    }
    for span in proc_reference_spans(analysis, ctx.resolution, qname, proc_def, source) {
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
        ..
    } = *ctx;
    // Method identity comes from the actual temporal receiver receipt,
    // independently of the selector's compatibility spelling.
    let line_index_local = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&line_index_local, source, line, character);
    if let Some(selected) = crate::receiver_identity::method_at_cursor(analysis, source, cursor) {
        let is_classmethod =
            selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class;
        let (decl_span, call_spans) = method_references_for_declaration(
            source,
            dialect,
            analysis,
            selected.class,
            selected.method,
            is_classmethod,
        );
        if decl_span != selected.method.name_span {
            return None;
        }
        return Some(build_member_ranges(
            source,
            line_index,
            decl_span,
            call_spans,
            include_declaration,
        ));
    }
    None
}

/// Build references for a bare `ClassName method` call site: the reverse of
/// [`instance_method_references`], for a `classmethod` — which dispatches on
/// the class's own command, never an instance, so it is never found by
/// `$obj`/`my` resolution.  Without this, Find References / Rename
/// triggered from the actual dispatch site (as opposed to the declaration
/// or a code lens) silently finds nothing.
fn classmethod_call_site_references(ctx: &RefCtx<'_>) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        dialect,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
        ..
    } = *ctx;
    let cursor = crate::definition::byte_offset_at(line_index, source, line, character);
    let selected = crate::receiver_identity::method_at_cursor(analysis, source, cursor)?;
    if selected.receiver != tcl_compiler::command_binding::SourceMethodReceiver::Class {
        return None;
    }
    let (decl_span, call_spans) = method_references_for_declaration(
        source,
        dialect,
        analysis,
        selected.class,
        selected.method,
        true,
    );
    if decl_span != selected.method.name_span {
        return None;
    }
    Some(build_member_ranges(
        source,
        line_index,
        decl_span,
        call_spans,
        include_declaration,
    ))
}

/// Build references for a `Factory::make` call site — [incr Tcl]'s
/// colon-qualified class-proc dispatch.
///
/// itcl's class-scoped `proc` is its equivalent of `TclOO`'s `classmethod`,
/// but it is invoked as a *single* `::`-qualified command word, not as a
/// two-word `Factory make` dispatch (which in itcl is the unrelated
/// `ClassName instanceName` object-creation syntax).  The word is resolved
/// with Tcl's own current-namespace-then-global rule
/// ([`crate::definition::itcl_class_proc_target`]), so it reaches the class
/// the runtime would reach and nothing else.
///
/// Returns `None` for every other spelling, leaving ordinary qualified proc
/// calls to [`proc_references`].
fn itcl_class_proc_references(ctx: &RefCtx<'_>) -> Option<Vec<LspRange>> {
    let RefCtx {
        source,
        dialect,
        line_index,
        line,
        character,
        analysis,
        include_declaration,
        ..
    } = *ctx;
    let (class_q, member) =
        crate::definition::itcl_class_proc_target_at(source, dialect, line, character, analysis)?;
    let (decl_span, call_spans) =
        method_references_for_class(source, dialect, analysis, &class_q, &member, true)?;
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
        ..
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

/// Project a whole configured source reference to its metadata name.
/// The caller separately selects a sole variable word. An absent configuration
/// is the explicit standalone decoration convention, without source admission.
pub(crate) fn strip_var_decoration(
    raw: &str,
    config: Option<tcl_lexer::LexerConfig>,
) -> Option<&str> {
    if !raw.starts_with('$') {
        return None;
    }
    let inner = if let Some(config) = config {
        tcl_lexer::word_parts::whole_var_ref(raw.as_bytes(), config).ok()??;
        tcl_syntax::naming::var_reference_for_style(raw, config.braced_var)
    } else {
        // Explicit standalone decoration metadata has no source admission.
        tcl_syntax::naming::var_reference(raw)
    };
    if inner.is_empty() { None } else { Some(inner) }
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
mod tests;

#[cfg(test)]
mod original_highlight_selection_tests;

#[cfg(test)]
mod original_next_dispatch_context_tests;

#[cfg(test)]
mod original_dispatch_parser_purpose_tests;

#[cfg(test)]
mod logical_class_selector_tests;

#[cfg(test)]
mod constructed_compatibility_tests;

#[cfg(test)]
mod configured_callback_variable_tests;

#[cfg(test)]
mod itcl_original_source_geometry_tests;

#[cfg(test)]
mod logical_reference_source_owner_tests;

#[cfg(test)]
mod logical_consumed_and_callback_reference_tests;

#[cfg(test)]
mod dispatch_region_census_tests;
