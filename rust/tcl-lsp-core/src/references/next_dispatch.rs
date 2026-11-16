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

//! `next` and `nextto` call sites: method super-dispatch and constructor and destructor chains.

use super::dispatch_regions::{
    MAX_DISPATCH_SCAN_DEPTH, RetainedDispatchContext, nested_dispatch_regions,
    retained_dispatch_context, strip_outer_braces,
};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::types::OriginalSourceSpecialMemberMetadata;
use tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata;

pub(super) fn span_contains_offset(span: tcl_lexer::Span, offset: u32) -> bool {
    span.start() <= offset && offset < span.end()
}

/// The `next` / `nextto` super-dispatch spans inside `class_q`'s own `method`
/// body — polymorphic **references** to `method`.  Kept out of
/// [`method_references_for_class`] because that set also drives *rename*, and
/// `next` / `nextto` are keywords that must never be rewritten to the new
/// name; only the reference paths add these.  Empty when `class_q` does not
/// define `method` as a member of the stated kind.
#[must_use]
pub fn method_next_dispatch_spans(
    analysis: &AnalysisResult,
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
) -> Vec<tcl_lexer::Span> {
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let Some(class_def) = analysis.all_classes.get(class_q) else {
        return Vec::new();
    };
    // `<constructor>` / `<destructor>` name the two slots that have no
    // method-table entry of their own; every other name is looked up in the
    // bucket the caller selected. Without the synthetic labels, references
    // to a constructor never surface an inheriting subclass's `next` call.
    let member = match method {
        tcl_compiler::analyser::class_hierarchy::CONSTRUCTOR_MEMBER => {
            class_def.constructors.last()
        }
        tcl_compiler::analyser::class_hierarchy::DESTRUCTOR_MEMBER => class_def.destructor.as_ref(),
        _ if is_classmethod => class_def.class_methods.get(method),
        _ => class_def.methods.get(method),
    };
    let Some(m) = member else {
        return Vec::new();
    };
    scan_next_dispatch_sites(source, analysis, m.body_span)
}

/// Every `next` / `nextto` call site — across every other class in this
/// document — whose target resolves (via the class hierarchy's MRO) to
/// `class_q`'s own effective constructor: a subclass constructor chaining
/// up to its superclass's is a name-independent but still meaningful
/// "referenced by an overriding subclass" relationship, the
/// constructor counterpart of [`method_next_dispatch_spans`]. Constructors
/// have no name-based dispatch, so this next-chain scan is the *whole*
/// reference story for one — no `my`/`$obj` call-site scan applies, unlike
/// [`method_references_for_class`]. Returns `None` when `class_q` declares
/// no explicit constructor.
///
/// Not gated by `DefinerFamily`: `next` / `nextto` and the MRO this walks
/// (`ClassHierarchy::mro_map`, built by `tcloo_linearise` for every
/// `ClassDef` alike) are `TclOO`-specific — Snit / [incr Tcl] classes have
/// different chaining models the registry does not even register a
/// `next`/`nextto` command for.  [`method_next_dispatch_spans`] /
/// [`ClassHierarchy::next_provider`] likewise run across every definer
/// family, with no itcl/Snit exclusion (unlike
/// `find_obj_method_call_sites`'s classmethod dispatch-shape check, which
/// *does* consult `class_definer_family` for an unrelated concern), so gating only
/// the constructor/destructor path here would be inconsistent: scoping the
/// whole next/nextto reference system by family is the coherent alternative.
pub(crate) fn constructor_next_chain_references(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
) -> Option<(tcl_lexer::Span, Vec<tcl_lexer::Span>)> {
    let class_def = analysis.all_classes.get(class_q)?;
    let decl_span = class_def.constructors.last()?.name_span;
    let hierarchy = analysis.class_hierarchy();
    let mut call_spans = Vec::new();
    for (other_q, other_cd) in &analysis.all_classes {
        if other_q == class_q {
            continue;
        }
        let Some(ctor) = other_cd.constructors.last() else {
            continue;
        };
        for (span, target) in scan_next_dispatch_sites_with_target(source, analysis, ctor.body_span)
        {
            let start_from = match target {
                Some(target) => {
                    let Some(target) =
                        crate::definition::canonicalise_class(analysis, span.start(), &target)
                    else {
                        continue;
                    };
                    Some(target)
                }
                None => None,
            };
            if hierarchy.constructor_next_provider(other_q, start_from.as_deref(), source)
                == Some(class_q)
            {
                call_spans.push(span);
            }
        }
    }
    Some((decl_span, call_spans))
}

/// The selected original lifecycle declaration's `next` / `nextto` sites:
/// the counterpart of [`constructor_next_chain_references`] /
/// [`destructor_next_chain_references`] for an analysis without lexical
/// declaration advice. The selected row must be the instance-side keyword the
/// live provider walk names, as native go-to-definition resolves it. Each
/// other original class's effective body of the same kind is scanned; a site
/// counts when the shared MRO provider walk from that class reaches `class`,
/// and a `nextto` must name `class` itself, after the scanning class.
pub(crate) fn original_special_next_chain_spans(
    source: &str,
    analysis: &AnalysisResult,
    class: &SourceDeclarationMetadata<tcl_compiler::analyser::ClassDef>,
    special: &OriginalSourceSpecialMemberMetadata,
) -> Vec<tcl_lexer::Span> {
    use tcl_compiler::analyser::class_hierarchy::{SourceMemberKind, class_member_def};
    let class_q = class.metadata().qualified_name.as_str();
    let member = SourceMemberKind::Special(special.kind());
    let hierarchy = analysis.class_hierarchy();
    if special.side() != tcl_compiler::analyser::types::MemberSide::Instance
        || effective_special_member(class, special) != Some(special)
        || hierarchy
            .classes
            .get(class_q)
            .and_then(|live| class_member_def(live, &member))
            .is_none_or(|live| live.name_span != special.keyword_word().span())
    {
        return Vec::new();
    }
    let mut call_spans = Vec::new();
    for other in analysis.original_class_declarations() {
        let other_q = other.metadata().qualified_name.as_str();
        if other_q == class_q {
            continue;
        }
        let Some(row) = effective_special_member(other, special) else {
            continue;
        };
        for (span, target) in
            scan_next_dispatch_sites_with_target(source, analysis, row.metadata().body_span)
        {
            let start_from = match target {
                Some(target) => {
                    let Some(target) =
                        crate::definition::canonicalise_class(analysis, span.start(), &target)
                    else {
                        continue;
                    };
                    let reachable = target == class_q
                        && hierarchy.mro_map.get(other_q).is_some_and(|mro| {
                            let at = |name: &str| mro.iter().position(|entry| entry == name);
                            at(other_q)
                                .zip(at(target.as_str()))
                                .is_some_and(|(from, to)| from < to)
                        });
                    if !reachable {
                        continue;
                    }
                    Some(target)
                }
                None => None,
            };
            if hierarchy.member_next_provider(
                other_q,
                &member,
                other_q,
                start_from.as_deref(),
                source,
            ) == Some(class_q)
            {
                call_spans.push(span);
            }
        }
    }
    call_spans
}

/// The last original lifecycle declaration of `special`'s kind and side in
/// `record`: the one a `next` / `nextto` chain can reach.
fn effective_special_member<'a>(
    record: &'a SourceDeclarationMetadata<tcl_compiler::analyser::ClassDef>,
    special: &OriginalSourceSpecialMemberMetadata,
) -> Option<&'a OriginalSourceSpecialMemberMetadata> {
    record
        .metadata()
        .original_special_members
        .declarations()
        .filter(|row| row.kind() == special.kind() && row.side() == special.side())
        .last()
}

/// The destructor counterpart of [`constructor_next_chain_references`].
/// Returns `None` when `class_q` declares no explicit destructor.
pub(crate) fn destructor_next_chain_references(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
) -> Option<(tcl_lexer::Span, Vec<tcl_lexer::Span>)> {
    let class_def = analysis.all_classes.get(class_q)?;
    let decl_span = class_def.destructor.as_ref()?.name_span;
    let hierarchy = analysis.class_hierarchy();
    let mut call_spans = Vec::new();
    for (other_q, other_cd) in &analysis.all_classes {
        if other_q == class_q {
            continue;
        }
        let Some(dtor) = &other_cd.destructor else {
            continue;
        };
        for (span, target) in scan_next_dispatch_sites_with_target(source, analysis, dtor.body_span)
        {
            let start_from = match target {
                Some(target) => {
                    let Some(target) =
                        crate::definition::canonicalise_class(analysis, span.start(), &target)
                    else {
                        continue;
                    };
                    Some(target)
                }
                None => None,
            };
            if hierarchy.destructor_next_provider(other_q, start_from.as_deref()) == Some(class_q) {
                call_spans.push(span);
            }
        }
    }
    Some((decl_span, call_spans))
}

/// Re-segment a single `method`-named method `body` and return the head-token
/// span of every `next` / `nextto` command.  `TclOO`'s super-dispatch invokes
/// the next `method` of the same name up the MRO, so a `next` / `nextto` inside
/// a `method` body is a polymorphic reference to `method` itself.
///
/// Recurses into `[...]` command substitutions and same-frame (`Plain`
/// `BodyKind`) control-flow / `eval` bodies via [`nested_dispatch_regions`],
/// exactly like [`scan_my_method_region`] / [`scan_obj_method_region`] — a
/// `next` inside an `if` / `while` / `foreach` / `switch` / `try` / `catch`
/// body is found too.
///
/// Thin wrapper over [`scan_next_dispatch_sites_with_target`] that drops the
/// `nextto` target argument — a method's `next`/`nextto` is flagged as a
/// reference purely by presence, never MRO-resolved, so the target is
/// irrelevant here (unlike the constructor/destructor next-chain resolvers,
/// which need it to disambiguate `nextto`).
pub(crate) fn scan_next_dispatch_sites(
    source: &str,
    analysis: &AnalysisResult,
    body: tcl_lexer::Span,
) -> Vec<tcl_lexer::Span> {
    scan_next_dispatch_sites_with_target(source, analysis, body)
        .into_iter()
        .map(|(span, _target)| span)
        .collect()
}

/// [`scan_next_dispatch_sites`], but paired with `nextto`'s target-class
/// argument from the selected original argv (`None` for a selected helper
/// with no name-role operand). Unknown target values remain unavailable. Feeds the
/// constructor/destructor next-chain resolvers
/// ([`constructor_next_chain_references`] / [`destructor_next_chain_references`]),
/// which must know *which* class a `nextto` names to decide whether it
/// chains to the class under a given lens.
pub(crate) fn scan_next_dispatch_sites_with_target(
    source: &str,
    analysis: &AnalysisResult,
    body: tcl_lexer::Span,
) -> Vec<(tcl_lexer::Span, Option<String>)> {
    let Some(RetainedDispatchContext {
        dialect, config, ..
    }) = retained_dispatch_context(source, analysis)
    else {
        return Vec::new();
    };
    let ctx = NextDispatchScan {
        source,
        analysis,
        config,
        dialect,
    };
    let mut out = Vec::new();
    if body.is_empty() {
        return out;
    }
    let (start, end) = strip_outer_braces(source, body);
    if start < end {
        scan_next_dispatch_region_with_target(ctx, start, end, 0, &mut out);
    }
    out
}

/// Segment `source[start..end]` and append the head-token span (plus
/// `nextto`'s target argument, when present) of every `next` / `nextto`
/// command, recursing per [`nested_dispatch_regions`]. `depth` guards
/// against runaway recursion — see [`MAX_DISPATCH_SCAN_DEPTH`].
#[derive(Clone, Copy)]
struct NextDispatchScan<'a> {
    pub(super) source: &'a str,
    pub(super) analysis: &'a AnalysisResult,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) dialect: &'static tcl_dialect::DialectProfile,
}

fn scan_next_dispatch_region_with_target(
    ctx: NextDispatchScan<'_>,
    start: usize,
    end: usize,
    depth: u32,
    out: &mut Vec<(tcl_lexer::Span, Option<String>)>,
) {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    let NextDispatchScan {
        source,
        analysis,
        config,
        dialect,
        ..
    } = ctx;
    if start >= end || end > source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return;
    }
    let body_text = &source[start..end];
    let commands = segment_commands_with_offset_and_config(
        body_text,
        u32::try_from(start).unwrap_or(0),
        config.nested(),
    );
    let Some(input) = analysis.resolved_input.as_ref() else {
        return;
    };
    let context = input.context_registry();
    for cmd in &commands {
        if let Some(head) = cmd.argv.first()
            && let Some(words) =
                tcl_compiler::registry_invocation::source_structure::source_registry_words(
                    source, analysis, cmd,
                )
            && let Some(target) = words
                .with_source_schema(&context, |schema| {
                    if !schema
                        .semantics
                        .traits
                        .contains(tcl_registry::Traits::TCLOO_NEXT_CHAIN)
                    {
                        return None;
                    }
                    let roles = words.roles()?;
                    let target = roles.iter().find_map(|&(ordinal, role)| {
                        (role == tcl_registry::ArgRole::Name).then_some(ordinal)
                    });
                    Some(match target {
                        Some(ordinal) => {
                            Some(schema.words.arguments().get(ordinal)?.literal()?.to_owned())
                        }
                        None => None,
                    })
                })
                .flatten()
        {
            // The selected descriptor owns the target value, including an
            // authentic captured operand; the call keeps its own head span.
            out.push((head.span, target));
        }
        for (inner_start, inner_end) in nested_dispatch_regions(source, analysis, dialect, cmd) {
            scan_next_dispatch_region_with_target(ctx, inner_start, inner_end, depth + 1, out);
        }
    }
}
