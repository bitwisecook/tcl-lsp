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

use crate::original_invocation::schema_script_arguments;
use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::types::OriginalSourceSpecialMemberMetadata;
use tcl_compiler::ir::MethodKind;
use tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

mod member_metadata;

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
    ctx: crate::definition::CallResolution<'_>,
    qname: &str,
    proc_def: &tcl_compiler::analyser::ProcDef,
    source: &str,
) -> Vec<tcl_lexer::Span> {
    if analysis.allows_retained_logical_declaration_advice() {
        return logical_proc_reference_spans(source, analysis, proc_def);
    }
    if !analysis.allows_lexical_declaration_advice() {
        let Some(declaration) = analysis
            .original_procedure_declarations()
            .find(|row| row.metadata() == proc_def)
        else {
            return Vec::new();
        };
        return analysis
            .command_invocations
            .iter()
            .filter(|invocation| {
                crate::original_declaration::invocation_targets_declaration(
                    source,
                    analysis,
                    invocation,
                    declaration,
                    true,
                )
            })
            .map(|invocation| invocation.range)
            .collect();
    }

    let indirect = indirect_names_reaching(analysis, &proc_def.qualified_name);
    analysis
        .command_invocations
        .iter()
        .filter(|inv| {
            if let RetainedDefinition::Known(definition) =
                retained_definition(inv, true, &proc_def.qualified_name)
            {
                return definition
                    .and_then(|definition| analysis.proc_for_definition(definition, source))
                    .is_some_and(|selected| selected.name_span == proc_def.name_span);
            }
            (invocation_references_proc(analysis, inv, qname, proc_def, source)
                && !forced_shadow_takes_the_call(
                    analysis,
                    ctx,
                    inv,
                    &proc_def.name,
                    &proc_def.qualified_name,
                ))
                || invocation_references_via_wildcard_import(
                    analysis,
                    ctx,
                    inv,
                    &proc_def.name,
                    &proc_def.qualified_name,
                )
                || invocation_references_via_indirection(analysis, inv, &indirect, Some(proc_def))
        })
        .map(|inv| inv.range)
        .collect()
}

/// Represented conditional source references only; no absence, dispatch or
/// rename permission follows from an empty or incomplete Logical inventory.
fn logical_proc_reference_spans(
    source: &str,
    analysis: &AnalysisResult,
    target: &tcl_compiler::analyser::ProcDef,
) -> Vec<tcl_lexer::Span> {
    let collect = || -> Option<Vec<tcl_lexer::Span>> {
        crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let image = tcl_lexer::SourceImage::document(source);
        let input = analysis.resolved_input.as_ref()?;
        let mut selected = analysis
            .logical_source_procedure_declarations()
            .filter(|row| {
                row.matches_source(&image, input)
                    && row.metadata().name_span == target.name_span
                    && row.metadata().body_span == target.body_span
            });
        let target = selected.next()?;
        selected.next().is_none().then_some(())?;
        let mut spans: Vec<_> =
            tcl_compiler::registry_invocation::source_structure::source_logical_procedure_calls(
                source, analysis,
            )?
            .into_iter()
            .filter(|call| {
                call.declaration(analysis)
                    .is_some_and(|row| std::ptr::eq(row, target))
            })
            .filter_map(|call| {
                call.original_words()
                    .first()
                    .map(tcl_lexer::NativeWord::span)
            })
            .collect();
        for reference in
            tcl_compiler::registry_invocation::source_structure::source_logical_name_references(
                source, analysis,
            )?
        {
            if reference
                .declaration(analysis)
                .is_some_and(|row| std::ptr::eq(row, target))
            {
                spans.push(reference.operand().span());
            }
        }
        for callback in
            tcl_compiler::registry_invocation::source_structure::source_logical_callback_candidates(
                source, analysis,
            )?
        {
            if callback
                .procedure_declaration(analysis)
                .is_some_and(|row| std::ptr::eq(row, target))
                && let Some(span) = callback.head().source_span()
            {
                spans.push(span);
            }
        }
        spans.sort_by_key(|span| (span.start(), span.end()));
        spans.dedup();
        Some(spans)
    };
    collect().unwrap_or_default()
}

fn logical_proc_at_cursor<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    cursor: u32,
) -> Option<&'a tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration> {
    let image = tcl_lexer::SourceImage::document(source);
    let input = analysis.resolved_input.as_ref()?;
    let contains = |span: tcl_lexer::Span| span.start() <= cursor && cursor < span.end();
    let mut headers: Vec<_> = analysis
        .logical_source_procedure_declarations()
        .filter(|header| {
            header.matches_source(&image, input) && contains(header.metadata().name_span)
        })
        .collect();
    if let Some(call) = tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_for_cursor(source, analysis, cursor)
        && let Some(header) = call.declaration(analysis)
    { headers.push(header); }
    for reference in
        tcl_compiler::registry_invocation::source_structure::source_logical_name_references(
            source, analysis,
        )?
    {
        if contains(reference.operand().span())
            && let Some(header) = reference.declaration(analysis)
        {
            headers.push(header);
        }
    }
    for callback in
        tcl_compiler::registry_invocation::source_structure::source_logical_callback_candidates(
            source, analysis,
        )?
    {
        if callback.head().source_span().is_some_and(contains)
            && let Some(header) = callback.procedure_declaration(analysis)
        {
            headers.push(header);
        }
    }
    let first = *headers.first()?;
    headers
        .iter()
        .all(|header| std::ptr::eq(*header, first))
        .then_some(first)
}

fn logical_class_reference_spans(
    source: &str,
    analysis: &AnalysisResult,
    target: &tcl_compiler::analyser::ClassDef,
) -> Vec<tcl_lexer::Span> {
    let collect = || -> Option<Vec<tcl_lexer::Span>> {
        crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        analysis.logical_source_class_declaration(target)?;
        let mut spans = Vec::new();
        for candidate in
            tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates(
                source, analysis,
            )?
        {
            let Some(head) = candidate.original_head() else {
                continue;
            };
            let Some(call) =
                tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
                    source,
                    analysis,
                    head.span().start(),
                )
            else {
                continue;
            };
            if call
                .class_declaration()
                .logical_source_class(analysis)
                .is_some_and(|class| std::ptr::eq(class, target))
            {
                spans.push(head.span());
            }
        }
        for reference in
            tcl_compiler::registry_invocation::source_structure::source_logical_name_references(
                source, analysis,
            )?
        {
            if reference
                .class_declaration(analysis)
                .is_some_and(|class| std::ptr::eq(class, target))
            {
                spans.push(reference.operand().span());
            }
        }
        for callback in
            tcl_compiler::registry_invocation::source_structure::source_logical_callback_candidates(
                source, analysis,
            )?
        {
            if callback
                .class_declaration(analysis)
                .is_some_and(|class| std::ptr::eq(class, target))
                && let Some(span) = callback.head().source_span()
            {
                spans.push(span);
            }
        }
        for receiver in analysis.all_classes.values() {
            if analysis
                .logical_source_class_declaration(receiver)
                .is_none()
            {
                continue;
            }
            let Some(statements) = analysis.logical_source_class_relation_candidates(receiver)
            else {
                continue;
            };
            for statement in statements {
                for operand in statement.operands() {
                    if matches!(operand.target(analysis, receiver), Some(tcl_compiler::command_binding::LogicalSourceClassRelationTarget::Source(class)) if std::ptr::eq(class, target))
                    {
                        spans.push(operand.operand().span());
                    }
                }
            }
        }
        spans.sort_by_key(|span| (span.start(), span.end()));
        spans.dedup();
        Some(spans)
    };
    collect().unwrap_or_default()
}

/// Whether a live `namespace import -force` has taken this call away from the
/// definition it would otherwise name.
///
/// The reference-side statement of the fact go-to-definition applies in
/// `resolve_called_proc`: `-force` *replaces* the importing namespace's own
/// command, so from the import onward a bare call does not reach the local
/// definition and must not be listed among its references.
/// Without it the two providers contradict each other on the very same
/// cursor — definition jumps to the import's source while find-references
/// still files the call under the definition the import deleted.
///
/// Narrow on purpose: it fires only for a call written in the *importing*
/// namespace itself, spelled as the bare name, with the shadow live at that
/// call. Everything else — a qualified call, a call in another namespace, a
/// call before the import — is untouched, and the shared
/// [`crate::definition::forced_import_shadows`] applies the whole ordered
/// lifecycle (export snapshot, conflict, forget, redefinition) rather than a
/// second copy of it.
fn forced_shadow_takes_the_call(
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    def_name: &str,
    def_qualified: &str,
) -> bool {
    if inv.name != def_name {
        return false;
    }
    let (owner, _) = tcl_syntax::naming::key_holder_and_tail(def_qualified);
    let call_ns = crate::definition::namespace_context_at(
        &analysis.global_scope,
        inv.range.start(),
        &analysis.namespace_overrides,
    );
    if call_ns != owner {
        return false;
    }
    crate::definition::forced_import_shadows(analysis, ctx, &call_ns, def_name, inv.range.start())
}

/// Every command name whose in-document `rename` / `interp alias` chain
/// terminates on `qualified`, with how it gets there — the reverse of
/// go-to-definition's forward hop ([`crate::definition::command_indirection`]),
/// so both directions of navigation read one table.
///
/// Built once per reference query by walking the two (normally tiny) command
/// -mutation maps, never by rescanning the tree, so the per-invocation test
/// below stays a single hash lookup.
fn indirect_names_reaching(
    analysis: &AnalysisResult,
    qualified: &str,
) -> std::collections::HashMap<String, tcl_compiler::analyser::indirection::Reaching> {
    tcl_compiler::analyser::indirection::names_reaching(
        analysis,
        qualified,
        &tcl_syntax::naming::normalise_qualified_name,
    )
}

/// Whether call site `inv` reaches this definition **only** through a live
/// command-table mutation — `interp alias {} sayHi {} greet` makes every
/// `[sayHi]` a real call site of `greet` (tclsh 8.6.16/9.0.4: both calls
/// execute `greet`'s body), and `rename greet hello` makes every later
/// `hello` one.
///
/// Order-gated against the offset that established the chain, so a call
/// written *before* the alias — which tclsh answers with `invalid command
/// name` — is not attributed to the target.
///
/// `target` additionally pins the chain's *identity* when the terminal name
/// has more than one declaration.  A `rename` hands over the command object,
/// so `proc p {} {return first}; rename p oldp; proc p {} {return second}`
/// leaves `oldp` and `p` naming two genuinely different commands (oracle,
/// tclsh 8.6.14/9.0.4: `oldp` → `first`, `p` → `second`).  Attributing the
/// `oldp` call sites to whichever declaration currently wins the name `p`
/// would merge two distinct commands' reference sets and double the winner's
/// code-lens count.  Classes pass `None`: a class name
/// has exactly one declaration, so there is no identity to disambiguate.
///
/// Additive to the shared matching rule and deliberately **not** wired into
/// [`invocation_references_proc`] / [`invocation_references_class`], for
/// exactly the reason the wildcard-import fallback above isn't: the call
/// spells the *alias's* name, which a rename of the target must not rewrite
/// (`rename::rename_proc` calls those two directly and so never sees this),
/// while Find All References and the code-lens count legitimately want it.
#[must_use]
fn invocation_references_via_indirection(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    indirect: &std::collections::HashMap<String, tcl_compiler::analyser::indirection::Reaching>,
    target: Option<&tcl_compiler::analyser::ProcDef>,
) -> bool {
    if indirect.is_empty() {
        return false;
    }
    let call_off = inv.range.start();
    let captures_target = |reaching: &tcl_compiler::analyser::indirection::Reaching| {
        let Some(def) = target else {
            return true;
        };
        // An alias re-resolves by name at every invocation, so its as-of time
        // is this call site's own offset; a rename froze one.
        let as_of = reaching.resolve_at.unwrap_or(call_off);
        analysis
            .proc_def_in_effect_at(&def.qualified_name, as_of)
            .is_some_and(|captured| captured.name_span == def.name_span)
    };
    let candidates = inv
        .resolution_candidates
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(inv.name.as_str()));
    candidates.into_iter().any(|cand| {
        indirect
            .get(&tcl_syntax::naming::normalise_qualified_name(cand))
            .is_some_and(|reaching| {
                tcl_compiler::analyser::indirection::in_effect(
                    analysis,
                    reaching.established,
                    call_off,
                ) && captures_target(reaching)
            })
    })
}

/// Whether call site `inv` reaches the definition named `def_name` /
/// `def_qualified` **only** through an in-scope, same-document wildcard
/// `namespace import NS::*` — a case
/// [`invocation_references_named`] can never catch, since a glob import
/// creates no real command at any of the call's candidate names for the
/// analyser's own resolution to have recorded.
///
/// Additive to that shared rule, **not** a replacement for it, and
/// deliberately **not** wired into [`invocation_references_proc`] /
/// [`invocation_references_class`] themselves: [`proc_reference_spans`] /
/// [`class_reference_spans`] (Find All References, the code-lens reference
/// count) OR this in, but `rename::rename_proc` / `rename::rename_class`
/// call [`invocation_references_proc`] / [`invocation_references_class`]
/// directly and so never see it — the call names the *local* imported
/// command, which keeps its own spelling regardless of a rename of the
/// source, exactly like the cross-document analogue
/// (`WorkspaceIndex::linked_invocations_of`, used by cross-document
/// references only, never by cross-document rename's
/// `invocations_of`-based edit gathering).
///
/// The whole import **lifecycle** is applied, not just "some import matches":
/// the export snapshot at that import's own position, the
/// non-`-force` conflict that makes an import install nothing, a `namespace
/// forget` or a deletion of the source command that takes the alias away
/// again, and an import *chain* that reaches the definition through an
/// intermediate namespace. All of it comes from the one shared
/// entry point go-to-definition resolves through
/// (`definition::import_chain_target`), so references and definition cannot
/// disagree about what a call site reaches. Testing "some import matches" and
/// "the final export set covers the name" as two independent conditions gets
/// both directions wrong.
#[must_use]
fn invocation_references_via_wildcard_import(
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    def_name: &str,
    def_qualified: &str,
) -> bool {
    if inv.name != def_name {
        return false;
    }
    let (target_ns, _) = tcl_syntax::naming::key_holder_and_tail(def_qualified);
    let call_ns = crate::definition::namespace_context_at(
        &analysis.global_scope,
        inv.range.start(),
        &analysis.namespace_overrides,
    );
    crate::definition::import_chain_target(analysis, ctx, &call_ns, def_name, inv.range.start())
        .is_some_and(|source_ns| source_ns == target_ns)
}

/// Whether a single call site `inv` references a named proc/class
/// definition — simple name `def_name`, fully-qualified name
/// `def_qualified` (whose lookup key is `qname`).
///
/// Retained declaration identity wins over all lexical name assistance. Edit
/// consumers select direct called slots through [`invocation_references_proc`]
/// and [`invocation_references_class`]; call-edge consumers separately select
/// executed terminal declarations through [`invocation_calls_proc`] and
/// [`invocation_calls_named`]. A known non-definition cannot borrow a final
/// namespace table entry. The compatibility rules below apply only when no
/// positioned declaration/reference receipt is available.
///
/// A bare simple-name call (`helper`) counts only when it resolves to this
/// definition, or — since the analyser resolves a namespace-internal call to
/// the global guess (`::helper`) — when it sits in this definition's own
/// namespace; that namespace gate keeps `helper` inside `namespace eval b`
/// from matching `::a::helper`. Qualified spellings and a
/// resolved-qualified-name hit always count. Constructed keys preserve every
/// component; only compatibility presentation removes one root marker.
#[must_use]
pub(crate) fn invocation_references_named(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    qname: &str,
    def_name: &str,
    def_qualified: &str,
    source: &str,
) -> bool {
    if let RetainedDefinition::Known(definition) = retained_definition(inv, true, def_qualified) {
        return definition.is_some_and(|definition| {
            analysis
                .proc_for_definition(definition, source)
                .is_some_and(|selected| selected.qualified_name == def_qualified)
                || analysis
                    .class_for_definition(definition, source)
                    .is_some_and(|selected| selected.qualified_name == def_qualified)
        });
    }
    let qname_no_prefix = tcl_syntax::naming::unroot_rooted_key(qname).unwrap_or(qname);
    let target_q = tcl_syntax::naming::unroot_rooted_key(def_qualified).unwrap_or(def_qualified);
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(def_qualified);
    let target_ns = tcl_syntax::naming::unroot_rooted_key(holder).unwrap_or(holder);
    let resolved_norm = inv
        .resolved_qualified_name
        .as_deref()
        .map(|r| tcl_syntax::naming::unroot_rooted_key(r).unwrap_or(r));
    let call_ns = crate::definition::innermost_namespace_at(
        &analysis.global_scope,
        inv.range.start(),
        &analysis.namespace_overrides,
    );
    // Readonly compatibility follows the shared implicit path for this scope.
    let call_reaches_target = call_ns == target_ns
        || tcl_compiler::analyser::scope::implicit_command_namespace_path_at(
            &analysis.global_scope,
            inv.range.start(),
        )
        .iter()
        .any(|namespace| {
            tcl_syntax::naming::unroot_rooted_key(namespace).unwrap_or(namespace) == target_ns
        });
    let simple_ok = inv.name == def_name
        && resolved_norm.is_none_or(|r| r == target_q || r == def_name)
        && call_reaches_target;
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
                    .any(|k| tcl_syntax::naming::unroot_rooted_key(k).unwrap_or(k) == r)
                    || analysis
                        .all_classes
                        .keys()
                        .any(|k| tcl_syntax::naming::unroot_rooted_key(k).unwrap_or(k) == r))
        });
        return !shadowed_by_other;
    }
    false
}

/// Whether the definition declared at `decl_off` under the name
/// `def_qualified` has been **displaced from that name** by a `rename` /
/// `interp alias` by the time the call at `call_off` runs — so a call
/// spelling that name does not reach this definition at all.
///
/// The mirror of the forward hop go-to-definition takes
/// (`definition::indirect_definition_target`): that one asks "where
/// does this call site really land?", this one asks "is *this* declaration
/// still what the name holds?". Both read the one indirection walk, so the
/// two directions of navigation cannot disagree about the same document.
///
/// Oracle (tclsh 8.6.16 and 9.0.4, byte-identical):
///
/// ```tcl
/// proc ::ttk::spinbox {w args} { puts "themed ttk::spinbox: $w $args" }
/// proc ::tk::spinbox  {w args} { puts "classic tk::spinbox: $w $args" }
/// interp alias {} ::ttk::spinbox {} ::tk::spinbox
/// ::ttk::spinbox .sb          ;# -> classic tk::spinbox: .sb
/// ```
///
/// The `::ttk::spinbox` proc body is unreachable under that name from the
/// alias onward, so the call is not one of its references — and renaming it
/// must not rewrite that word. Rewriting it would turn the same script into
/// `themed ttk::spinbox: .sb`: a rename that silently changes which body runs.
///
/// # Two ordering facts, both required
///
/// 1. **The binding must outrank the declaration.** A `proc` written *after*
///    an `interp alias` on the same name replaces the alias — oracle:
///    `interp alias {} greet {} target` then `proc greet {…}` then `greet`
///    prints `greet body`, not `target body`. So a binding established
///    before `decl_off` displaces nothing.
/// 2. **The binding must be unconditional at the call.** A mutation written
///    at top level has certainly run by the time anything else does; one
///    written inside the *same* body as the call is in genuine statement
///    order. A mutation sitting in some *other* proc's body only runs if
///    that proc is ever called, which is not statically decidable — there
///    the call site may still reach this declaration, so it stays in the
///    reference set (and so in the rename edit set) rather than being
///    silently dropped from an edit the user cannot inspect.
#[must_use]
fn definition_displaced_from_its_name(
    analysis: &AnalysisResult,
    def_qualified: &str,
    decl_off: u32,
    call_off: u32,
) -> bool {
    let canonical = tcl_syntax::naming::normalise_qualified_name(def_qualified);
    let Some(hop) = tcl_compiler::analyser::indirection::walk(
        analysis,
        &canonical,
        call_off,
        &tcl_syntax::naming::normalise_qualified_name,
    ) else {
        return false;
    };
    if hop.target == canonical || hop.established < decl_off {
        return false;
    }
    match analysis.innermost_definition_body_span(hop.established) {
        None => true,
        Some(body) => body.start() <= call_off && call_off < body.end(),
    }
}

enum RetainedDefinition<'a> {
    Absent,
    Known(Option<&'a tcl_compiler::command_binding::SourceCommandDefinition>),
}

/// Exact selected declaration, with called-slot identity for edits and terminal
/// identity for call edges. A known non-definition retains an explicit refusal and
/// blocks lexical assistance; foreign allocations are validated against the
/// caller's document bytes by the declaration helpers.
fn retained_definition<'a>(
    inv: &'a tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    terminal: bool,
    qualified: &str,
) -> RetainedDefinition<'a> {
    if let Some(reference) = &inv.resolved_command_reference {
        let definition = if terminal {
            reference
                .linked_definition()
                .or_else(|| reference.definition())
        } else if reference.is_direct_definition() && reference.slot() == Some(qualified) {
            reference.definition()
        } else {
            None
        };
        return RetainedDefinition::Known(definition);
    }
    inv.resolved_definition
        .as_ref()
        .map_or(RetainedDefinition::Absent, |definition| {
            RetainedDefinition::Known(Some(definition))
        })
}

/// Match a real executed call to its terminal retained procedure declaration.
/// Alias names remain call edges without becoming editable target spellings.
#[must_use]
pub(crate) fn invocation_calls_proc(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    qname: &str,
    proc_def: &tcl_compiler::analyser::ProcDef,
    source: &str,
) -> bool {
    if analysis.allows_retained_logical_declaration_advice() {
        if !inv.lookup.is_execution_site() {
            return false;
        }
        return tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_at(source, analysis, inv.range.start())
            .and_then(|call| call.declaration(analysis))
            .is_some_and(|header| header.metadata().name_span == proc_def.name_span && header.metadata().body_span == proc_def.body_span);
    }
    if !analysis.allows_lexical_declaration_advice() {
        if !inv.lookup.is_execution_site() {
            return false;
        }
        return analysis
            .original_procedure_declarations()
            .find(|row| row.metadata() == proc_def)
            .is_some_and(|declaration| {
                crate::original_declaration::invocation_targets_declaration(
                    source,
                    analysis,
                    inv,
                    declaration,
                    true,
                )
            });
    }

    if !inv.lookup.is_execution_site() {
        return false;
    }
    if let RetainedDefinition::Known(definition) =
        retained_definition(inv, true, &proc_def.qualified_name)
    {
        return definition
            .and_then(|definition| analysis.proc_for_definition(definition, source))
            .is_some_and(|selected| selected.name_span == proc_def.name_span);
    }
    invocation_references_proc(analysis, inv, qname, proc_def, source)
}

/// Match an executed procedure call for an external target query. A retained
/// foreign procedure identifies its terminal name for reporting a call edge;
/// it never supplies an editable declaration in this document. Local receipts
/// must match their exact original declaration against the actual source.
#[must_use]
pub(crate) fn invocation_calls_named(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    qname: &str,
    def_name: &str,
    def_qualified: &str,
    source: &str,
) -> bool {
    if !inv.lookup.is_execution_site() {
        return false;
    }
    if let RetainedDefinition::Known(definition) = retained_definition(inv, true, def_qualified) {
        let Some(definition) = definition else {
            return false;
        };
        if definition.kind()
            != tcl_compiler::command_binding::SourceCommandDefinitionKind::Procedure
        {
            return false;
        }
        if let Some(selected) = analysis.proc_for_definition(definition, source) {
            return selected.qualified_name == def_qualified;
        }
        if matches!(definition.allocation().site.source.kind(),
            tcl_compiler::command_binding::SourceOriginKind::Authored(authored) if authored.as_ref() == source.as_bytes())
        {
            return false;
        }
        return definition.allocation().command == def_qualified;
    }
    invocation_references_named(analysis, inv, qname, def_name, def_qualified, source)
}

/// [`invocation_references_named`] specialised for a [`ProcDef`](tcl_compiler::analyser::ProcDef).
///
/// Gated by [`definition_displaced_from_its_name`]: a call written after an
/// `interp alias` / `rename` that took this proc's own name over reaches the
/// binding's target, not this proc.
#[must_use]
pub(crate) fn invocation_references_proc(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    qname: &str,
    proc_def: &tcl_compiler::analyser::ProcDef,
    source: &str,
) -> bool {
    if !analysis.allows_lexical_declaration_advice() {
        return analysis
            .original_procedure_declarations()
            .find(|row| row.metadata() == proc_def)
            .is_some_and(|declaration| {
                crate::original_declaration::invocation_targets_declaration(
                    source,
                    analysis,
                    inv,
                    declaration,
                    false,
                )
            });
    }

    if let RetainedDefinition::Known(definition) =
        retained_definition(inv, false, &proc_def.qualified_name)
    {
        return definition
            .and_then(|definition| analysis.proc_for_definition(definition, source))
            .is_some_and(|selected| selected.name_span == proc_def.name_span);
    }
    if definition_displaced_from_its_name(
        analysis,
        &proc_def.qualified_name,
        proc_def.name_span.start(),
        inv.range.start(),
    ) {
        return false;
    }
    invocation_references_named(
        analysis,
        inv,
        qname,
        &proc_def.name,
        &proc_def.qualified_name,
        source,
    )
}

/// [`invocation_references_named`] specialised for a [`ClassDef`](tcl_compiler::analyser::ClassDef).
///
/// Same command-table gate as [`invocation_references_proc`] — a class
/// command's name is an ordinary command-table slot and an `interp alias`
/// takes it over just as thoroughly.
#[must_use]
pub(crate) fn invocation_references_class(
    analysis: &AnalysisResult,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    qname: &str,
    class_def: &tcl_compiler::analyser::ClassDef,
    source: &str,
) -> bool {
    if !analysis.allows_lexical_declaration_advice() {
        return analysis
            .original_class_declarations()
            .find(|row| row.metadata() == class_def)
            .is_some_and(|declaration| {
                crate::original_declaration::invocation_targets_declaration(
                    source,
                    analysis,
                    inv,
                    declaration,
                    false,
                )
            });
    }

    if let RetainedDefinition::Known(definition) =
        retained_definition(inv, false, &class_def.qualified_name)
    {
        return definition
            .and_then(|definition| analysis.class_for_definition(definition, source))
            .is_some_and(|selected| selected.name_span == class_def.name_span);
    }
    if definition_displaced_from_its_name(
        analysis,
        &class_def.qualified_name,
        class_def.name_span.start(),
        inv.range.start(),
    ) {
        return false;
    }
    invocation_references_named(
        analysis,
        inv,
        qname,
        &class_def.name,
        &class_def.qualified_name,
        source,
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
    ctx: crate::definition::CallResolution<'_>,
    qname: &str,
    class_def: &tcl_compiler::analyser::ClassDef,
    source: &str,
) -> Vec<tcl_lexer::Span> {
    if analysis.allows_retained_logical_declaration_advice() {
        return logical_class_reference_spans(source, analysis, class_def);
    }
    if !analysis.allows_lexical_declaration_advice() {
        let Some(declaration) = analysis
            .original_class_declarations()
            .find(|row| row.metadata() == class_def)
        else {
            return Vec::new();
        };
        return analysis
            .command_invocations
            .iter()
            .filter(|invocation| {
                crate::original_declaration::invocation_targets_declaration(
                    source,
                    analysis,
                    invocation,
                    declaration,
                    true,
                )
            })
            .map(|invocation| invocation.range)
            .collect();
    }

    let indirect = indirect_names_reaching(analysis, &class_def.qualified_name);
    analysis
        .command_invocations
        .iter()
        .filter(|inv| {
            if let RetainedDefinition::Known(definition) =
                retained_definition(inv, true, &class_def.qualified_name)
            {
                return definition
                    .and_then(|definition| analysis.class_for_definition(definition, source))
                    .is_some_and(|selected| selected.name_span == class_def.name_span);
            }
            (invocation_references_class(analysis, inv, qname, class_def, source)
                && !forced_shadow_takes_the_call(
                    analysis,
                    ctx,
                    inv,
                    &class_def.name,
                    &class_def.qualified_name,
                ))
                || invocation_references_via_wildcard_import(
                    analysis,
                    ctx,
                    inv,
                    &class_def.name,
                    &class_def.qualified_name,
                )
                || invocation_references_via_indirection(analysis, inv, &indirect, None)
        })
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

/// Readonly stored selectors under a genuine original method-body issuer.
/// Source roles retain conditional declaration advice, independently of capture
/// results, a live receiver, callback entry and replacement permission.
pub(crate) fn authored_callback_reference_spans(
    source: &str,
    analysis: &AnalysisResult,
    target: &crate::method_symbol::OriginalMethodCandidate,
) -> Vec<tcl_lexer::Span> {
    use tcl_registry::definer::DefinitionMemberNamePurpose;

    let Some(current) = retained_dispatch_context(source, analysis) else {
        return Vec::new();
    };
    let Some(class) = target.declaring_class_in(source, analysis) else {
        return Vec::new();
    };
    let method = target.method();
    if method.name_purpose() != DefinitionMemberNamePurpose::TclOoMethod {
        return Vec::new();
    }
    let name = method.original_name_input();
    let mut declarations = class
        .metadata()
        .original_members
        .declarations()
        .filter(|row| {
            row.side() == method.side()
                && row.original_name_input().policy() == name.policy()
                && row.original_name_input().bytes() == name.bytes()
        });
    if declarations.next() != Some(method) || declarations.next().is_some() {
        return Vec::new();
    }
    let ctx = MyMethodScan {
        source,
        analysis,
        config: current.config,
        dialect: current.dialect,
        method: "",
        skip: None,
        external_callback_allowed: false,
        callbacks_only: true,
        stored_callbacks: &[],
    };
    let mut spans = Vec::new();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if let Some(span) = authored_stored_callback_selector(
                ctx,
                &current.context,
                class.metadata(),
                method,
                command,
            ) {
                spans.push(span);
            }
            false
        },
    );
    spans.sort_unstable_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    spans
}

fn authored_stored_callback_selector(
    ctx: MyMethodScan<'_>,
    context: &tcl_registry::model::ContextRegistry,
    class: &tcl_compiler::analyser::ClassDef,
    method: &tcl_compiler::analyser::types::OriginalSourceMethodMetadata,
    command: &tcl_compiler::segmenter::SegmentedCommand,
) -> Option<tcl_lexer::Span> {
    use tcl_compiler::registry_invocation::{InvocationWordOrigin, source_structure};
    use tcl_compiler::signature_scan::original_name::SignatureSourceNameKey;

    let prefix =
        source_structure::source_authored_command_prefix(ctx.source, ctx.analysis, command)?;
    if !prefix.matches_source_context(
        &tcl_lexer::SourceImage::document(ctx.source),
        ctx.config,
        context,
    ) || prefix.with_source_schema(context, |schema| {
        schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
    }) != Some(true)
    {
        return None;
    }
    let owner = authored_callback_body_owner(class, method, prefix.definition_member_body()?)?;
    let word = prefix.original_argument(0)?;
    let InvocationWordOrigin::Written(written) = prefix.argument_origin(0)? else {
        return None;
    };
    if written == 0 || prefix.producer().original_words().get(written) != Some(word) {
        return None;
    }
    let name = method.original_name_input();
    let key = SignatureSourceNameKey::from_original_native_word(
        word,
        tcl_syntax::word_rules::WordValueRules::from_config(&ctx.config),
        name.policy(),
    )?;
    if key.bytes() != name.bytes()
        || !method
            .name_purpose()
            .admits(key.policy().recipe(), key.bytes())
    {
        return None;
    }
    let span = bare_original_prefix_span(word)?;
    let body = owner.metadata().body_span;
    let stored = stored_callback_targets(ctx, body);
    let body_ctx = MyMethodScan {
        stored_callbacks: &stored,
        ..ctx
    };
    let (start, end) = strip_outer_braces(ctx.source, body);
    let candidate = PrefixTargetAtSpan {
        kind: tcl_registry::CommandPrefixTarget::CurrentObjectInternalMethod,
        span,
    };
    (stored.iter().any(|stored| stored.target == candidate)
        && stored_callback_is_consumed(body_ctx, start, end, candidate, 0))
    .then_some(span)
}

fn authored_callback_body_owner<'a>(
    class: &'a tcl_compiler::analyser::ClassDef,
    method: &tcl_compiler::analyser::types::OriginalSourceMethodMetadata,
    issuer: &tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody,
) -> Option<&'a tcl_compiler::analyser::types::OriginalSourceMethodMetadata> {
    use tcl_compiler::analyser::types::MemberSide;
    use tcl_registry::definer::{
        CallableRole, DefinitionMemberNamePurpose, MemberEffect, MemberReceiver,
    };

    let row = issuer.member_row()?;
    if !matches!(
        row.effect,
        MemberEffect::Callable {
            role: CallableRole::Method,
            ..
        }
    ) {
        return None;
    }
    let side = match row.receiver {
        MemberReceiver::Instance => MemberSide::Instance,
        MemberReceiver::TypeObject => MemberSide::ClassObject,
        MemberReceiver::Both => return None,
    };
    if side != method.side() {
        return None;
    }
    let mut owners = class.original_members.declarations().filter(|owner| {
        owner.side() == side
            && owner.name_purpose() == DefinitionMemberNamePurpose::TclOoMethod
            && owner.body_word() == Some(issuer.original_container())
    });
    let owner = owners.next()?;
    owners.next().is_none().then_some(owner)
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

/// Resolve a method operand only through its actual receiver entry.
/// Stored prefix syntax alone cannot grant a captured receiver allocation.
pub(crate) fn list_built_self_method_target_at_cursor(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<(String, bool)> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let selected = crate::receiver_identity::method_at_cursor(analysis, source, cursor_offset)?;
    let side = if selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class {
        tcl_compiler::analyser::types::MemberSide::ClassObject
    } else {
        tcl_compiler::analyser::types::MemberSide::Instance
    };
    let (current, member) =
        analysis
            .class_hierarchy()
            .declared_member(&selected.class.qualified_name, word, side)?;
    // The shared compatibility owner must retain the same declaration and side.
    (selected.editable_selector().is_some()
        && member == selected.method
        && current.name_span == selected.class.name_span)
        .then(|| {
            (
                selected.class.qualified_name.clone(),
                selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class,
            )
        })
}

/// A retained receiver at the cursor, independently of lexical class scopes.
/// Unexecuted callback prefixes need their own capture receipt.
#[must_use]
pub fn callback_prefix_method_receiver_at_cursor(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<(String, bool, bool)> {
    let (class, is_classmethod) =
        list_built_self_method_target_at_cursor(source, dialect, analysis, word, cursor_offset)?;
    Some((class, is_classmethod, true))
}

/// A syntactically possible captured selector without an actual entry receipt.
/// This is a whole-rename hazard only, never a reference or replacement span.
pub(crate) fn unproved_callback_method_selector(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    method: &str,
) -> Option<tcl_lexer::Span> {
    for class in analysis
        .all_classes
        .values()
        .chain(analysis.superseded_classes.values().flatten())
    {
        for body in collect_member_bodies(class) {
            for span in
                scan_method_sites_by_kind(source, analysis, &[body], method, None, true, true)
            {
                if command_prefix_target_at_cursor(source, analysis, body, span.start()).is_some()
                    && crate::receiver_identity::method_at_cursor(analysis, source, span.start())
                        .is_none()
                {
                    return Some(span);
                }
            }
        }
    }
    None
}

fn span_contains_offset(span: tcl_lexer::Span, offset: u32) -> bool {
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

/// The external `$obj method` / bare `objcmd method` call sites for `method`
/// on instances of `class_q` **within `source`**, independent of whether
/// `class_q` is *defined* in this document.
///
/// The building block for cross-file references from a **pure-consumer**
/// document — one that only creates and uses instances of a class defined
/// elsewhere (`set d [::other::Cls new]; $d method`).  Such a document defines
/// no class body, so [`method_reference_spans_in_document`] /
/// [`inherited_method_spans_in_document`] (which key off a local class body)
/// find nothing; this keys off `instance_classes`, which the cross-file
/// analysis populates from the workspace class set.
///
/// `classmethod_class_names` carries the caller's *workspace-wide* knowledge
/// of which class names are valid bare-dispatch heads for `method` when it is
/// a `classmethod` — a pure-consumer document has no local `ClassDef` for
/// `class_q` to derive this from (unlike the same-document path, whose
/// [`method_references_for_class`] derives it from `analysis.all_classes`
/// directly), so the caller supplies it from the workspace index's
/// [`WorkspaceMethod`](crate::workspace_index::WorkspaceMethod) `kind`
/// instead.  Empty when `method` is not a classmethod, or the caller has no
/// workspace index.
#[must_use]
pub fn obj_method_call_sites(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
    classmethod_class_names: &[String],
) -> Vec<tcl_lexer::Span> {
    find_obj_method_call_sites_with_extra_cmd_names(
        source,
        dialect,
        analysis,
        class_q,
        method,
        is_classmethod,
        classmethod_class_names,
    )
}

/// Direct class-command selector sites in the actual retained Logical source.
/// The shared original class-call owner supplies factory/name lineage and full
/// context; this query grants neither Native receiver identity nor dispatch.
/// Captured selector prefixes cannot donate a written call-site range.
pub(crate) fn logical_class_command_method_sites(
    source: &str,
    analysis: &AnalysisResult,
    receiver: &tcl_compiler::analyser::ClassDef,
    method: &str,
) -> Vec<tcl_lexer::Span> {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    if !analysis.allows_retained_logical_declaration_advice() {
        return Vec::new();
    }
    let image = tcl_lexer::SourceImage::document(source);
    let context = current.context();
    let mut seen = FxHashSet::default();
    analysis
        .command_invocations
        .iter()
        .filter_map(|invocation| {
            if !invocation.lookup.is_execution_site() {
                return None;
            }
            let call =
                tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
                    source,
                    analysis,
                    invocation.range.start(),
                )?;
            if !call.matches_source_context(&image, current.config(), &context)
                || !std::ptr::eq(
                    call.class_declaration().logical_source_class(analysis)?,
                    receiver,
                )
                || call.arguments().first()?.literal_bytes()? != method.as_bytes()
            {
                return None;
            }
            let written = call.written_argument(0)?;
            let selector = call.argument_word(0)?;
            if call.original_words().get(written + 1) != Some(selector)
                || selector.tokens().len() != 1
            {
                return None;
            }
            let span = selector.tokens()[0].span;
            seen.insert((span.start(), span.end())).then_some(span)
        })
        .collect()
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

/// Re-segment each brace-delimited body span in `bodies` and return the
/// name-token span of every `my <method>` invocation whose method name is
/// `method`, skipping the token at `skip` (the declaration site, when the
/// scanned class declares `method`).
///
/// Intra-class dispatch of a `TclOO` method is `my <method>` — the method
/// name is argv[1], not the command head.  A bare head equal to the method
/// name is *not* a call (a `TclOO` method is not a command in the body's
/// namespace; `<method> …` without `my`/an object errors "invalid command
/// name"), so only `my`-headed sites match. Call hierarchy, references,
/// rename and code lenses share this positioned selector matcher.
pub(crate) fn scan_my_method_sites(
    source: &str,
    analysis: &AnalysisResult,
    bodies: &[tcl_lexer::Span],
    method: &str,
    skip: Option<tcl_lexer::Span>,
) -> Vec<tcl_lexer::Span> {
    scan_method_sites(source, analysis, bodies, method, skip, false)
}

/// [`scan_my_method_sites`] with registry-declared deferred callback
/// recognition.  `external_callback_allowed` is the target method's
/// visibility gate: `[self] method` captures the externally-dispatched object
/// command and therefore requires `public`, whereas `[list my method]` stays
/// in the current object's private dispatch path.
pub(crate) fn scan_method_sites(
    source: &str,
    analysis: &AnalysisResult,
    bodies: &[tcl_lexer::Span],
    method: &str,
    skip: Option<tcl_lexer::Span>,
    external_callback_allowed: bool,
) -> Vec<tcl_lexer::Span> {
    scan_method_sites_by_kind(
        source,
        analysis,
        bodies,
        method,
        skip,
        external_callback_allowed,
        false,
    )
}

fn scan_method_sites_by_kind(
    source: &str,
    analysis: &AnalysisResult,
    bodies: &[tcl_lexer::Span],
    method: &str,
    skip: Option<tcl_lexer::Span>,
    external_callback_allowed: bool,
    callbacks_only: bool,
) -> Vec<tcl_lexer::Span> {
    let Some(RetainedDispatchContext {
        dialect, config, ..
    }) = retained_dispatch_context(source, analysis)
    else {
        return Vec::new();
    };
    let ctx = MyMethodScan {
        source,
        analysis,
        config,
        dialect,
        method,
        skip,
        external_callback_allowed,
        callbacks_only,
        stored_callbacks: &[],
    };
    let mut out: Vec<tcl_lexer::Span> = Vec::new();
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    for &body_span in bodies {
        let stored_callbacks = stored_callback_targets(ctx, body_span);
        let body_ctx = MyMethodScan {
            stored_callbacks: &stored_callbacks,
            ..ctx
        };
        let mut sink = SpanSink {
            out: &mut out,
            seen: &mut seen,
        };
        scan_my_method_body(body_ctx, body_span, &mut sink);
    }
    out
}

/// Read-only context for the intra-class `my method` call-site scan: the
/// document `source`, its `dialect`, the `method` being looked up, and the
/// declaration span to `skip` (so the method's own name token isn't reported
/// as a call to itself).
#[derive(Clone, Copy)]
struct MyMethodScan<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    config: tcl_lexer::LexerConfig,
    dialect: &'static tcl_dialect::DialectProfile,
    method: &'a str,
    skip: Option<tcl_lexer::Span>,
    /// `[list [self] METHOD ...]` later dispatches through the external
    /// object command and therefore counts only for an exported method.
    /// `[list my METHOD ...]` is always eligible because it retains the
    /// current method frame's private dispatch semantics.
    external_callback_allowed: bool,
    /// Suppress direct dispatches while proving a cursor sits on a deferred
    /// callback target rather than an ordinary method reference.
    callbacks_only: bool,
    /// At most one same-scope static callback assignment per variable.  This
    /// is deliberately source-local and conservative; ambiguous writes are
    /// omitted before the callback walker sees them.
    stored_callbacks: &'a [StoredCallbackTarget],
}

#[derive(Clone)]
struct StoredCallbackTarget {
    variable: String,
    assignment_end: u32,
    target: PrefixTargetAtSpan,
}

/// Recover only the one-hop, same-frame constant form of a stored callback.
///
/// This deliberately treats every write to a variable as a candidate write:
/// if there is more than one write, or the sole write is dynamic, the variable
/// is omitted.  That makes reassignment, branch joins, parameters, and other
/// incomplete SSA cases abstain without teaching this LSP scanner a command
/// name or inventing a target.
fn stored_callback_targets(
    ctx: MyMethodScan<'_>,
    body_span: tcl_lexer::Span,
) -> Vec<StoredCallbackTarget> {
    use std::collections::HashMap;

    let (start, end) = strip_outer_braces(ctx.source, body_span);
    if start >= end || end > ctx.source.len() {
        return Vec::new();
    }
    let mut writes: HashMap<String, (u32, Option<PrefixTargetAtSpan>)> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let has_scope_alias =
        collect_stored_callback_writes(ctx, start, end, 0, &mut writes, &mut counts);
    if has_scope_alias {
        return Vec::new();
    }
    writes
        .into_iter()
        .filter_map(|(name, (assignment_end, target))| {
            if counts.get(&name) != Some(&1) {
                return None;
            }
            target.map(|target| StoredCallbackTarget {
                variable: name,
                assignment_end,
                target,
            })
        })
        .collect()
}

/// Collect callback assignments from every registry-declared same-frame
/// region. The callback consumer already descends through these regions; the
/// write inventory must cover the identical control-flow surface or a nested
/// reassignment could leave a stale outer callback looking unique.
fn collect_stored_callback_writes(
    ctx: MyMethodScan<'_>,
    start: usize,
    end: usize,
    depth: u32,
    writes: &mut std::collections::HashMap<String, (u32, Option<PrefixTargetAtSpan>)>,
    counts: &mut std::collections::HashMap<String, usize>,
) -> bool {
    use tcl_compiler::registry_invocation::{InvocationWordOrigin, source_structure};
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    use tcl_lexer::TokenType;

    if MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) || end > ctx.source.len() {
        return true;
    }
    if start >= end {
        return false;
    }
    let Some(input) = ctx.analysis.resolved_input.as_ref() else {
        return true;
    };
    let context = input.context_registry();
    let Ok(offset) = u32::try_from(start) else {
        return true;
    };
    let commands = segment_commands_with_offset_and_config(
        &ctx.source[start..end],
        offset,
        ctx.config.nested(),
    );
    let mut has_scope_alias = false;
    for cmd in commands {
        let Some(words) = source_structure::source_registry_words(ctx.source, ctx.analysis, &cmd)
        else {
            has_scope_alias = true;
            continue;
        };
        let Some((aliases, setter, variables)) = stored_callback_schema(&words, &context) else {
            has_scope_alias = true;
            continue;
        };
        has_scope_alias |= aliases;
        for (ordinal, variable) in variables {
            let Some(var) = variable else {
                has_scope_alias = true;
                continue;
            };
            if tcl_syntax::naming::is_qualified(var.as_bytes())
                || tcl_syntax::naming::split_element_ref(&var).is_some()
            {
                continue;
            }
            *counts.entry(var.clone()).or_default() += 1;
            // Selected Set describes adjacent effective operands. Captured names
            // retain their value but never borrow a caller's value-token index.
            let value_target = setter
                .then(|| {
                    let origin = words.origins().get(ordinal.checked_add(2)?)?;
                    let InvocationWordOrigin::Written(written) = origin else {
                        return None;
                    };
                    let &token = cmd.argv.get(*written)?;
                    (token.kind == TokenType::Cmd
                        && cmd.single_token_word.get(*written) == Some(&true)
                        && positions_are_literal_through(&cmd, *written))
                    .then(|| command_prefix_targets_from_word(ctx, &token, 0))
                    .and_then(|targets| (targets.len() == 1).then(|| targets[0]))
                })
                .flatten();
            writes.insert(var, (cmd.span.end(), value_target));
        }
        for (nested_start, nested_end) in
            nested_dispatch_regions(ctx.source, ctx.analysis, ctx.dialect, &cmd)
        {
            has_scope_alias |= collect_stored_callback_writes(
                ctx,
                nested_start,
                nested_end,
                depth + 1,
                writes,
                counts,
            );
        }
    }
    has_scope_alias
}

type StoredCallbackSchema = (bool, bool, Vec<(usize, Option<String>)>);

fn stored_callback_schema(
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
) -> Option<StoredCallbackSchema> {
    use tcl_registry::hooks::LoweringHookId;
    use tcl_registry::{ArgRole, Traits};
    words
        .with_source_schema(context, |schema| {
            let roles = words.roles()?;
            let variables = roles
                .iter()
                .filter(|(_, role)| *role == ArgRole::VarWrite)
                .map(|&(ordinal, _)| {
                    (
                        ordinal,
                        schema
                            .words
                            .arguments()
                            .get(ordinal)
                            .and_then(tcl_registry::InvocationWord::literal)
                            .map(str::to_owned),
                    )
                })
                .collect::<Vec<_>>();
            Some((
                schema
                    .semantics
                    .traits
                    .contains(Traits::CREATES_SCOPE_ALIAS)
                    || matches!(
                        schema.semantics.lowering_hook,
                        Some(
                            LoweringHookId::Global
                                | LoweringHookId::Variable
                                | LoweringHookId::Upvar
                        )
                    ),
                schema.semantics.lowering_hook == Some(LoweringHookId::Set),
                variables,
            ))
        })
        .flatten()
}

/// Scan a brace-delimited body span for `my method` call sites (stripping the
/// surrounding braces first), mirroring [`scan_obj_method_body`].
fn scan_my_method_body(ctx: MyMethodScan<'_>, body_span: tcl_lexer::Span, sink: &mut SpanSink<'_>) {
    if body_span.is_empty() {
        return;
    }
    let (start, end) = strip_outer_braces(ctx.source, body_span);
    if start >= end {
        return;
    }
    scan_my_method_region(ctx, start, end, 0, sink);
}

/// Segment `source[start..end]` and record the argv[1] span of every
/// `my <method>` invocation whose method name is `ctx.method`, recursing into
/// command-substitution (`[...]`) args **and** every same-frame
/// (`Plain`-`BodyKind`) control-flow / `eval` body argument
/// ([`nested_dispatch_regions`]) so a dispatch nested inside `return [my …]`,
/// an `if` / `while` / `foreach` / `switch` / `try` / `catch` body, or any
/// combination of the two, is found too. This is the same
/// recursion [`scan_obj_method_region`] performs, keeping intra-class `my`
/// dispatch and external `$obj` dispatch at parity.  The declaration span
/// (`ctx.skip`) and already-seen spans are elided.  `depth` guards against
/// runaway recursion on pathological input — see [`MAX_DISPATCH_SCAN_DEPTH`].
///
/// A bare head equal to the method name is *not* a call (a `TclOO` method is
/// not a command in the body's namespace; `<method> …` without `my`/an object
/// errors "invalid command name"), so only `my`-headed sites match.
fn scan_my_method_region(
    ctx: MyMethodScan<'_>,
    start: usize,
    end: usize,
    depth: u32,
    sink: &mut SpanSink<'_>,
) {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    let source = ctx.source;
    if start >= end || end > source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return;
    }
    let region = &source[start..end];
    let commands = segment_commands_with_offset_and_config(
        region,
        u32::try_from(start).unwrap_or(0),
        ctx.config.nested(),
    );
    for cmd in &commands {
        if !ctx.callbacks_only
            && let (Some(head), Some(name_tok)) = (cmd.argv.first(), cmd.argv.get(1))
        {
            let h_start = head.span.start() as usize;
            let h_end = head.span.end() as usize;
            // Readonly source roles retain this actual point's availability
            // and grammar; the written keyword cannot reselect a catalogue row.
            let head_is_self_dispatch =
                h_start < source.len() && h_end <= source.len() && original_self_dispatch(ctx, cmd);
            if head_is_self_dispatch {
                let n_start = name_tok.span.start() as usize;
                let n_end = name_tok.span.end() as usize;
                if n_start < source.len()
                    && n_end <= source.len()
                    && &source[n_start..n_end] == ctx.method
                    && Some(name_tok.span) != ctx.skip
                {
                    let key = (name_tok.span.start(), name_tok.span.end());
                    if sink.seen.insert(key) {
                        sink.out.push(name_tok.span);
                    }
                }
            }
        }
        for span in list_built_self_reference_spans(ctx, cmd) {
            let key = (span.start(), span.end());
            if sink.seen.insert(key) {
                sink.out.push(span);
            }
        }
        for (inner_start, inner_end) in
            nested_dispatch_regions(source, ctx.analysis, ctx.dialect, cmd)
        {
            scan_my_method_region(ctx, inner_start, inner_end, depth + 1, sink);
        }
    }
}

/// Authored self-dispatch at an unchanged original command. A bracketed
/// receiving-object result keeps its own original child lookup; lexical
/// parsing supplies no current object, entered method frame or edit authority.
fn original_self_dispatch(
    ctx: MyMethodScan<'_>,
    command: &tcl_compiler::segmenter::SegmentedCommand,
) -> bool {
    use tcl_compiler::registry_invocation::source_structure;
    let Some(input) = ctx.analysis.resolved_input.as_ref() else {
        return false;
    };
    let context = input.context_registry();
    if let Some(words) = source_structure::source_registry_words(ctx.source, ctx.analysis, command)
        && words.origins().get(1)
            == Some(&tcl_compiler::registry_invocation::InvocationWordOrigin::Written(1))
        && words.with_source_schema(&context, |schema| {
            schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
        }) == Some(true)
    {
        return true;
    }
    let Some(&head) = command.argv.first() else {
        return false;
    };
    if command.single_token_word.first() != Some(&true) || head.kind != tcl_lexer::TokenType::Cmd {
        return false;
    }
    original_self_receiver_word(ctx, head)
}

/// Matching method words captured into this command's executable callback.
///
/// A command-prefix builder can be wrapped by another registry-declared
/// prefix wrapper, so this recognises both direct `[list [self] METHOD]` /
/// `[list my METHOD]` forms and values such as `[namespace code [list my
/// METHOD]]`.  The traversal is trait-driven: it follows only
/// `BUILDS_COMMAND_PREFIX` and `WRAPS_COMMAND_PREFIX`, never a command name.
///
/// The enclosing resolved registry spec must mark the argument as either a
/// complete executable [`tcl_registry::ArgRole::Body`] or a first-class
/// [`tcl_registry::ArgRole::CommandPrefix`], and the inner resolved spec must carry
/// `BUILDS_COMMAND_PREFIX`; neither a callback consumer nor its builder is
/// named here. Inert data such as `set x [list [self] METHOD]` therefore
/// remains data.
fn list_built_self_reference_spans(
    ctx: MyMethodScan<'_>,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<tcl_lexer::Span> {
    callback_targets_from_command(ctx, cmd)
        .into_iter()
        .filter_map(|target| match target {
            PrefixTargetAtSpan {
                kind: tcl_registry::CommandPrefixTarget::CurrentObjectInternalMethod,
                span,
            } if source_span_text(ctx.source, span) == ctx.method => Some(span),
            PrefixTargetAtSpan {
                kind: tcl_registry::CommandPrefixTarget::CurrentObjectExternalMethod,
                span,
            } if ctx.external_callback_allowed
                && source_span_text(ctx.source, span) == ctx.method =>
            {
                Some(span)
            }
            PrefixTargetAtSpan { .. } => None,
        })
        .collect()
}

/// Resolve registry-declared callback argument positions and extract their
/// typed static prefix target.  This is intentionally independent of a
/// particular method name so cursor navigation and call hierarchy can use the
/// exact same source classification as references/rename/code-lens.
fn callback_targets_from_command(
    ctx: MyMethodScan<'_>,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<PrefixTargetAtSpan> {
    use tcl_compiler::registry_invocation::{InvocationWordOrigin, source_structure};
    use tcl_lexer::TokenType;

    let Some(input) = ctx.analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let context = input.context_registry();
    let Some(words) = source_structure::source_registry_words(ctx.source, ctx.analysis, cmd) else {
        return Vec::new();
    };
    let Some(positions) = words
        .with_source_schema(&context, |schema| {
            if ctx.analysis.allows_retained_logical_declaration_advice() {
                schema.authored_logical_source_plain_script_arguments()
            } else {
                schema.authored_source_plain_script_arguments()
            }
        })
        .flatten()
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for ordinal in positions {
        let Some(InvocationWordOrigin::Written(written)) = words.origins().get(ordinal + 1) else {
            continue;
        };
        let Some(&body_tok) = cmd.argv.get(*written) else {
            continue;
        };
        if !positions_are_literal_through(cmd, *written) {
            continue;
        }
        if body_tok.kind == TokenType::Var && cmd.single_token_word.get(*written) == Some(&true) {
            let original_words = tcl_compiler::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(ctx.source),
                ctx.config,
                cmd,
            );
            let Some(variable) = original_words
                .word_exprs
                .get(*written)
                .and_then(tcl_compiler::ir::WordExpr::sole_variable_substitution)
                .and_then(|(spelling, _)| strip_var_decoration(spelling, Some(ctx.config)))
            else {
                continue;
            };
            let matches: Vec<_> = ctx
                .stored_callbacks
                .iter()
                .filter(|stored| {
                    stored.variable == variable && stored.assignment_end < cmd.span.start()
                })
                .collect();
            if matches.len() == 1 {
                out.push(matches[0].target);
            }
            continue;
        }
        // Only a genuine sole written substitution is renameable here. A
        // captured operand retains its producer, not this call's source span.
        if body_tok.kind == TokenType::Cmd && cmd.single_token_word.get(*written) == Some(&true) {
            out.extend(command_prefix_targets_from_word(ctx, &body_tok, 0));
        }
    }
    out
}

/// Whether every word of `cmd` up to and including `upto` is written out
/// rather than `{*}`-expanded.
///
/// A consumer that reads word *N* is relying on the registry's role indices,
/// and those describe words. `{*}` splices a value's elements into the
/// argument list, so one expansion at or before *N* makes every later index
/// unreliable — not just the expanded word itself. Checking the whole prefix
/// is what makes `[namespace code {*}[list my tick]]` abstain: the expansion
/// is at the wrapper's body position, and after it `namespace code` has two
/// arguments and errors rather than dispatching anything.
///
/// `expand_word` is `None` for the overwhelming majority of commands — no word
/// uses expansion — and a per-word flag list otherwise.
fn positions_are_literal_through(
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
    upto: usize,
) -> bool {
    cmd.expand_word
        .as_ref()
        .is_none_or(|flags| flags.iter().take(upto + 1).all(|expanded| !expanded))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PrefixTargetAtSpan {
    kind: tcl_registry::CommandPrefixTarget,
    span: tcl_lexer::Span,
}

/// Map an effective builder operand to its genuine whole written source token.
fn written_prefix_operand(
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    ordinal: usize,
) -> Option<tcl_lexer::Token> {
    use tcl_compiler::registry_invocation::InvocationWordOrigin;
    let InvocationWordOrigin::Written(written) = words.origins().get(ordinal.checked_add(1)?)?
    else {
        return None;
    };
    (command.single_token_word.get(*written) == Some(&true)
        && positions_are_literal_through(command, *written))
    .then_some(())?;
    command.argv.get(*written).copied()
}

fn bare_original_prefix_span(word: &tcl_lexer::NativeWord) -> Option<tcl_lexer::Span> {
    let [token] = word.tokens() else {
        return None;
    };
    (token.kind == tcl_lexer::TokenType::Esc && !token.in_quote && !word.group().expand)
        .then_some(token.span)
}

/// Extract static method words from one sole command substitution used as a
/// callback value.  The recursion deliberately requires each hop to be a
/// single substitution, so concatenated and dynamically-built values abstain.
fn command_prefix_targets_from_word(
    ctx: MyMethodScan<'_>,
    word_tok: &tcl_lexer::Token,
    depth: u32,
) -> Vec<PrefixTargetAtSpan> {
    use tcl_compiler::registry_invocation::source_structure;
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    use tcl_lexer::TokenType;
    use tcl_registry::Traits;

    if MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) || word_tok.kind != TokenType::Cmd {
        return Vec::new();
    }
    let Some(input) = ctx.analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let context = input.context_registry();
    let regions =
        crate::executable_regions::command_substitution_regions(ctx.source, ctx.config, *word_tok);
    let [(inner_start, inner_end)] = regions.as_slice() else {
        return Vec::new();
    };
    let Ok(offset) = u32::try_from(*inner_start) else {
        return Vec::new();
    };
    let built = segment_commands_with_offset_and_config(
        &ctx.source[*inner_start..*inner_end],
        offset,
        ctx.config.nested(),
    );
    let [builder] = built.as_slice() else {
        return Vec::new();
    };
    let Some(words) = source_structure::source_registry_words(ctx.source, ctx.analysis, builder)
    else {
        return Vec::new();
    };
    let Some((builds, wraps)) = words.with_source_schema(&context, |schema| {
        (
            schema
                .semantics
                .traits
                .contains(Traits::BUILDS_COMMAND_PREFIX)
                && schema.semantics.native_result
                    == Some(
                        tcl_registry::native_result::NativeResultContract::ListArguments {
                            from: 0,
                        },
                    )
                && schema.semantics.argument_offset == 0
                && schema.facts().arity_accepts_frozen_arguments() == Some(true),
            schema
                .semantics
                .traits
                .contains(Traits::WRAPS_COMMAND_PREFIX),
        )
    }) else {
        return Vec::new();
    };
    if builds {
        return built_command_prefix_targets(ctx, builder, &words, &context);
    }
    if !wraps {
        return Vec::new();
    }
    let Some(positions) = words
        .with_source_schema(&context, |schema| {
            if ctx.analysis.allows_retained_logical_declaration_advice() {
                schema.authored_logical_source_plain_script_arguments()
            } else {
                schema.authored_source_plain_script_arguments()
            }
        })
        .flatten()
    else {
        return Vec::new();
    };
    positions
        .into_iter()
        .filter_map(|ordinal| written_prefix_operand(&words, builder, ordinal))
        .flat_map(|body| command_prefix_targets_from_word(ctx, &body, depth + 1))
        .collect()
}

fn built_command_prefix_targets(
    ctx: MyMethodScan<'_>,
    builder: &tcl_compiler::segmenter::SegmentedCommand,
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<PrefixTargetAtSpan> {
    use tcl_compiler::registry_invocation::source_structure;
    use tcl_lexer::TokenType;
    use tcl_registry::Traits;
    if let Some(prefix) =
        source_structure::source_authored_command_prefix(ctx.source, ctx.analysis, builder)
        && prefix.with_source_schema(context, |schema| {
            schema
                .semantics
                .traits
                .contains(Traits::TCLOO_SELF_DISPATCH)
        }) == Some(true)
        && let Some(span) = prefix
            .original_argument(0)
            .and_then(bare_original_prefix_span)
    {
        return vec![PrefixTargetAtSpan {
            kind: tcl_registry::CommandPrefixTarget::CurrentObjectInternalMethod,
            span,
        }];
    }
    if let (Some(receiver), Some(method)) = (
        written_prefix_operand(words, builder, 0),
        written_prefix_operand(words, builder, 1),
    ) && method.kind == TokenType::Esc
        && !method.in_quote
        && original_self_receiver_word(ctx, receiver)
    {
        return vec![PrefixTargetAtSpan {
            kind: tcl_registry::CommandPrefixTarget::CurrentObjectExternalMethod,
            span: method.span,
        }];
    }
    // A literal list head is source syntax only. Its declaration or current
    // target, if any, is resolved independently by the navigation consumer.
    written_prefix_operand(words, builder, 0)
        .filter(|token| token.kind == TokenType::Esc && !token.in_quote)
        .map(|token| PrefixTargetAtSpan {
            kind: tcl_registry::CommandPrefixTarget::DirectCommandHead,
            span: token.span,
        })
        .into_iter()
        .collect()
}

fn source_span_text(source: &str, span: tcl_lexer::Span) -> &str {
    let start = span.start() as usize;
    let end = span.end() as usize;
    source.get(start..end).unwrap_or_default()
}

/// Classify the deferred callback target exactly under `cursor_offset` in one
/// method body.  This reuses the same registry-role and prefix-result walker
/// as the reference scan, then descends only through registry-declared nested
/// executable regions.
fn command_prefix_target_at_cursor(
    source: &str,
    analysis: &AnalysisResult,
    body: tcl_lexer::Span,
    cursor_offset: u32,
) -> Option<PrefixTargetAtSpan> {
    let RetainedDispatchContext {
        dialect, config, ..
    } = retained_dispatch_context(source, analysis)?;
    let base_ctx = MyMethodScan {
        source,
        analysis,
        config,
        dialect,
        method: "",
        skip: None,
        external_callback_allowed: true,
        callbacks_only: true,
        stored_callbacks: &[],
    };
    let (start, end) = strip_outer_braces(source, body);
    let stored = stored_callback_targets(base_ctx, body);
    let ctx = MyMethodScan {
        stored_callbacks: &stored,
        ..base_ctx
    };
    if let Some(target) = stored.iter().find(|stored| {
        span_contains_offset(stored.target.span, cursor_offset)
            && stored_callback_is_consumed(ctx, start, end, stored.target, 0)
    }) {
        return Some(target.target);
    }
    command_prefix_target_in_region(ctx, start, end, 0, cursor_offset)
}

fn stored_callback_is_consumed(
    ctx: MyMethodScan<'_>,
    start: usize,
    end: usize,
    target: PrefixTargetAtSpan,
    depth: u32,
) -> bool {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    if start >= end || end > ctx.source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return false;
    }
    let commands = segment_commands_with_offset_and_config(
        &ctx.source[start..end],
        u32::try_from(start).unwrap_or(0),
        ctx.config.nested(),
    );
    commands.iter().any(|cmd| {
        callback_targets_from_command(ctx, cmd)
            .into_iter()
            .any(|candidate| candidate == target)
            || nested_dispatch_regions(ctx.source, ctx.analysis, ctx.dialect, cmd)
                .into_iter()
                .any(|(nested_start, nested_end)| {
                    stored_callback_is_consumed(ctx, nested_start, nested_end, target, depth + 1)
                })
    })
}

fn command_prefix_target_in_region(
    ctx: MyMethodScan<'_>,
    start: usize,
    end: usize,
    depth: u32,
    cursor_offset: u32,
) -> Option<PrefixTargetAtSpan> {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;

    if start >= end || end > ctx.source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return None;
    }
    let commands = segment_commands_with_offset_and_config(
        &ctx.source[start..end],
        u32::try_from(start).unwrap_or(0),
        ctx.config.nested(),
    );
    for cmd in &commands {
        for target in callback_targets_from_command(ctx, cmd) {
            let span = target.span;
            if span_contains_offset(span, cursor_offset) {
                return Some(target);
            }
        }
        for (inner_start, inner_end) in
            nested_dispatch_regions(ctx.source, ctx.analysis, ctx.dialect, cmd)
        {
            if let Some(target) = command_prefix_target_in_region(
                ctx,
                inner_start,
                inner_end,
                depth + 1,
                cursor_offset,
            ) {
                return Some(target);
            }
        }
    }
    None
}

/// A sole original receiver substitution retains its own selected child schema.
/// Method receiver applicability supplies no entered object or callback frame.
fn original_self_receiver_word(ctx: MyMethodScan<'_>, head: tcl_lexer::Token) -> bool {
    use tcl_compiler::registry_invocation::source_structure;
    if head.kind != tcl_lexer::TokenType::Cmd {
        return false;
    }
    let Some(input) = ctx.analysis.resolved_input.as_ref() else {
        return false;
    };
    let context = input.context_registry();
    let regions =
        crate::executable_regions::command_substitution_regions(ctx.source, ctx.config, head);
    let [(start, end)] = regions.as_slice() else {
        return false;
    };
    let Ok(offset) = u32::try_from(*start) else {
        return false;
    };
    let commands = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        &ctx.source[*start..*end],
        offset,
        ctx.config.nested(),
    );
    let [child] = commands.as_slice() else {
        return false;
    };
    let Some(words) = source_structure::source_registry_words(ctx.source, ctx.analysis, child)
    else {
        return false;
    };
    words.with_source_schema(&context, |schema| {
        let spec = schema.authored_source_descriptors().command;
        if spec.self_receiver_words.is_empty() {
            return false;
        }
        match schema.words.arguments().exact_argv_len() {
            Some(0) => spec.arity.min == 0,
            Some(1) => schema
                .words
                .arguments()
                .get(0)
                .and_then(tcl_registry::InvocationWord::literal)
                .is_some_and(|word| spec.self_receiver_words.contains(&word)),
            _ => false,
        }
    }) == Some(true)
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
    source: &'a str,
    analysis: &'a AnalysisResult,
    config: tcl_lexer::LexerConfig,
    dialect: &'static tcl_dialect::DialectProfile,
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
fn find_class_member_references(
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

/// Find external object and class method reads whose temporal receiver entry
/// identifies the desired original declaration. Candidate class maps and
/// command spellings cannot supply editable reference authority.
/// Returns spans of the written method-name tokens.
///
/// Scans three region kinds — the top-level command stream,
/// each user proc body, and each class method body — and
/// recurses into command-substitution (`[...]`) args at every
/// level.  This covers the common call forms (`$d bark`,
/// `puts [$d bark]`, calls inside procs / methods).  Method
/// names embedded in quoted / word tokens
/// (`"prefix[$d bark]"`) are not descended — a rare form.
#[cfg(test)]
pub(crate) fn find_obj_method_call_sites(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
) -> Vec<tcl_lexer::Span> {
    find_obj_method_call_sites_with_extra_cmd_names(
        source,
        dialect,
        analysis,
        class_q,
        method,
        is_classmethod,
        &[],
    )
}

/// Native profiles: the written selectors whose actual receiver entry
/// selects `class_q`'s own `method` declaration on the requested side. Self
/// dispatch stays with the member-body scan, and a receiver with no retained
/// entry contributes no site.
fn original_obj_method_call_sites(
    source: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
) -> Vec<tcl_lexer::Span> {
    let Some(registry) = analysis.resolved_registry() else {
        return Vec::new();
    };
    let receiver = if is_classmethod {
        tcl_compiler::command_binding::SourceMethodReceiver::Class
    } else {
        tcl_compiler::command_binding::SourceMethodReceiver::Instance
    };
    let mut sites = Vec::new();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            let self_dispatch = command.texts.first().is_some_and(|head| {
                registry.method_dispatch_keyword(head)
                    == Some(tcl_registry::MethodDispatchKind::SelfDispatch)
            });
            if !self_dispatch
                && let Some(selected) =
                    crate::receiver_identity::method_at_command(analysis, source, command)
                && selected.receiver == receiver
                && selected.class.qualified_name == class_q
                && selected.method.name == method
                && !sites.contains(&selected.selector)
            {
                sites.push(selected.selector);
            }
            false
        },
    );
    sites.sort_by_key(|span| (span.start(), span.end()));
    sites
}

use crate::definition::class_definer_family;

/// Every call site in this document that dispatches `class_q`'s [incr Tcl]
/// class-scoped `proc` named `member` — the single `::`-qualified
/// `Factory::make` shape, which is a *different call shape entirely* from
/// the two-word `Factory make` dispatch `classmethod` / `typemethod` use.
///
/// Each returned span covers only the call's final `::`-segment (the `make`
/// of `Factory::make`), so a rename rewrites the member name and leaves the
/// as-written qualifier alone — `Factory::make` → `Factory::produce`.
///
/// Sites come from `analysis.command_invocations` rather than a text scan:
/// an itcl class proc really is an ordinary command, so the analyser has
/// already indexed every call to it, including those nested inside `[...]`
/// substitutions and control-flow bodies.  Each candidate is then resolved
/// with [`crate::definition::itcl_class_proc_target`], which applies Tcl's
/// own current-namespace-then-global rule to the call's *lexical* namespace
/// — so `Factory::make` written inside `namespace eval ::app` reaches
/// `::app::Factory`, and the same text written at the top level (where only
/// `::app::Factory` exists) reaches nothing at all.
fn itcl_class_proc_call_sites(
    source: &str,
    analysis: &AnalysisResult,
    _dialect: &'static tcl_dialect::DialectProfile,
    class_q: &str,
    member: &str,
) -> Vec<tcl_lexer::Span> {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    let image = tcl_lexer::SourceImage::document(source);
    let Some(realm) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    analysis
        .command_invocations
        .iter()
        .filter_map(|inv| {
            if !inv.lookup.is_execution_site() {
                return None;
            }
            let (word, _site) = realm.original_written_head_at_span_in_source(
                &image,
                inv.range,
                current.config(),
            )?;
            if word.group().expand {
                return None;
            }
            let value = tcl_syntax::word_rules::original_static_word_unicode_value(&word)?;
            let written = std::str::from_utf8(&value).ok()?;
            let namespace = crate::definition::namespace_context_at(
                &analysis.global_scope,
                word.span().start(),
                &analysis.namespace_overrides,
            );
            let (target_class, target_member) =
                crate::definition::itcl_class_proc_target(analysis, &namespace, written)?;
            if target_class != class_q || target_member != member {
                return None;
            }
            crate::original_name_edit::original_plain_source_command_tail(source, analysis, &word)
        })
        .collect()
}
/// Editable receiver targets retain their original declaration spans. Names
/// supplied by a workspace caller remain candidates until a local source
/// declaration and the temporal method entry agree.
fn dispatch_receivers(
    analysis: &AnalysisResult,
    target: (&str, &str, bool),
    extra_cmd_names: &[String],
) -> CommandReceivers {
    let (class_q, method, is_classmethod) = target;
    let mut receivers = CommandReceivers {
        class_targets: FxHashMap::default(),
        instance_target: None,
    };
    if let Some(class) = analysis.all_classes.get(class_q) {
        if is_classmethod
            && class_definer_family(analysis, class)
                .is_some_and(|family| family != tcl_registry::definer::DefinerFamily::Itcl)
        {
            if let Some(member) = class.class_methods.get(method) {
                receivers.add_class_target(
                    &class.qualified_name,
                    Some((class.name_span, member.name_span)),
                );
            }
        } else if !is_classmethod && let Some(member) = class.methods.get(method) {
            receivers.instance_target = Some((class.name_span, member.name_span));
        }
    }
    if is_classmethod {
        for name in extra_cmd_names {
            receivers.class_targets.entry(name.clone()).or_insert(None);
        }
    }
    receivers
}

/// External method selector with `extra_cmd_names` — bare command
/// names to treat as valid classmethod-dispatch heads for `method`
/// regardless of what this document's own `analysis.all_classes` knows.
///
/// `is_classmethod` selects `method`'s dispatch shape explicitly rather than
/// inferring it from map membership: a class may legally define a `method`
/// and a `classmethod` of the *same name* (they occupy separate dispatch
/// tables — the instance's and the class object's own), so "does
/// `class_methods` contain this name" cannot answer "which one does the
/// caller mean" when both do.
///
/// A same-document call (`extra_cmd_names` empty) derives everything from
/// `analysis` directly.  The cross-file *pure-consumer* path
/// ([`obj_method_call_sites`]) cannot: a document that only calls `Factory
/// make` and never declares/extends `Factory` has no `::Factory` entry in
/// its own `all_classes` for the local classmethod check below to find, so
/// its caller supplies the workspace-wide answer here instead.
fn find_obj_method_call_sites_with_extra_cmd_names(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
    extra_cmd_names: &[String],
) -> Vec<tcl_lexer::Span> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }
    if !analysis.allows_lexical_declaration_advice() {
        return original_obj_method_call_sites(source, analysis, class_q, method, is_classmethod);
    }
    // [incr Tcl] class-scoped `proc`s land in the same `class_methods`
    // bucket as a `classmethod`, but dispatch as a single `::`-qualified
    // identifier (`Factory::make`) — a shape the two-word scanner below
    // cannot see, and whose two-word look-alike is unrelated
    // instance-creation syntax.  Collected first so every consumer of this
    // scanner (references, rename, the code lens, call hierarchy) gets itcl
    // edges from one place.
    let mut out: Vec<tcl_lexer::Span> = if is_classmethod
        && analysis.all_classes.get(class_q).is_some_and(|cd| {
            class_definer_family(analysis, cd) == Some(tcl_registry::definer::DefinerFamily::Itcl)
        }) {
        itcl_class_proc_call_sites(source, analysis, dialect, class_q, method)
    } else {
        Vec::new()
    };
    let receivers =
        dispatch_receivers(analysis, (class_q, method, is_classmethod), extra_cmd_names);
    if !receivers.has_any() {
        return out;
    }
    let mut seen: FxHashSet<(u32, u32)> = out.iter().map(|s| (s.start(), s.end())).collect();
    let Some(_) = analysis.retained_command_realm() else {
        return out;
    };
    let Some(config) = analysis.body_lexer_config else {
        return out;
    };
    let ctx = ObjMethodScan {
        source,
        dialect,
        config,
        analysis,
        receivers: &receivers,
        method,
        var_receivers_in_scope: true,
    };

    // Region 1: the whole document.
    {
        let mut sink = SpanSink {
            out: &mut out,
            seen: &mut seen,
        };
        scan_obj_method_region(ctx, 0, source.len(), false, 0, &mut sink);
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

/// Candidate declarations for an editable method reference. Every match also
/// needs the actual retained receiver and original method-entry receipt.
struct CommandReceivers {
    class_targets: FxHashMap<String, Option<(tcl_lexer::Span, tcl_lexer::Span)>>,
    instance_target: Option<(tcl_lexer::Span, tcl_lexer::Span)>,
}

impl CommandReceivers {
    /// Register a class whose own command dispatches the method, by its
    /// fully qualified name.
    fn add_class_target(
        &mut self,
        qualified_name: &str,
        declaration: Option<(tcl_lexer::Span, tcl_lexer::Span)>,
    ) {
        self.class_targets
            .insert(qualified_name.to_owned(), declaration);
    }

    fn has_any(&self) -> bool {
        !self.class_targets.is_empty() || self.instance_target.is_some()
    }

    fn matches(&self, selected: &crate::receiver_identity::RetainedReceiverMethod<'_>) -> bool {
        use tcl_compiler::command_binding::SourceMethodReceiver;
        let expected = match selected.receiver {
            SourceMethodReceiver::Instance => self.instance_target,
            SourceMethodReceiver::Class => self
                .class_targets
                .get(&selected.class.qualified_name)
                .copied()
                .flatten(),
        };
        expected == Some((selected.class.name_span, selected.method.name_span))
    }
}

/// Actual analysis context and editable declaration targets for method reads.
/// The retained realm and grammar came from the original analysis ingress.
#[derive(Clone, Copy)]
struct ObjMethodScan<'a> {
    source: &'a str,
    dialect: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
    analysis: &'a AnalysisResult,
    receivers: &'a CommandReceivers,
    method: &'a str,
    /// Variable receivers are withheld in frame-shifting regions until the
    /// scan retains that region's exact body and read coordinate mapping.
    var_receivers_in_scope: bool,
}

impl ObjMethodScan<'_> {
    /// This context as it applies inside a frame-shifting region.
    fn frame_shifted(self) -> Self {
        Self {
            var_receivers_in_scope: false,
            ..self
        }
    }

    /// Whether this context can still match anything — a frame-shifted scan
    /// with no command receivers to look for has nothing left to do.
    fn has_receivers(&self) -> bool {
        !self.receivers.class_targets.is_empty()
            || (self.var_receivers_in_scope && self.receivers.instance_target.is_some())
    }
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
    let (start, end) = strip_outer_braces(ctx.source, body_span);
    if start >= end {
        return;
    }
    scan_obj_method_region(ctx, start, end, true, 0, sink);
}

/// Segment `source[start..end]` and record every `$v method` call site,
/// recursing into command-substitution (`[...]`) args **and** every
/// same-frame (`Plain` `BodyKind`) control-flow / `eval` body argument
/// ([`nested_dispatch_regions`]), so a dispatch nested inside an `if` /
/// `while` / `foreach` / `switch` / `try` / `catch` body is found too
/// `depth` guards against runaway recursion
/// — see [`MAX_DISPATCH_SCAN_DEPTH`].
fn scan_obj_method_region(
    ctx: ObjMethodScan<'_>,
    start: usize,
    end: usize,
    nested: bool,
    depth: u32,
    sink: &mut SpanSink<'_>,
) {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    let source = ctx.source;
    if start >= end || end > source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return;
    }
    let region = &source[start..end];
    let commands = segment_commands_with_offset_and_config(
        region,
        u32::try_from(start).unwrap_or(0),
        if nested {
            ctx.config.nested()
        } else {
            ctx.config
        },
    );
    for cmd in &commands {
        // Only a written method selector may become an editable reference.
        // Receiver matching uses its retained post-argv declaration receipt.
        if let (Some(head), Some(method_tok)) = (cmd.argv.first(), cmd.argv.get(1)) {
            let h_start = head.span.start() as usize;
            let h_end = head.span.end() as usize;
            if h_start < source.len() && h_end <= source.len() {
                let receiver_matches =
                    crate::receiver_identity::method_at_command(ctx.analysis, source, cmd)
                        .is_some_and(|selected| {
                            selected.editable_selector().is_some()
                                && (ctx.var_receivers_in_scope
                                || selected.receiver
                                    == tcl_compiler::command_binding::SourceMethodReceiver::Class)
                                && ctx.receivers.matches(&selected)
                        });
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
        for (inner_start, inner_end) in
            nested_dispatch_regions(source, ctx.analysis, ctx.dialect, cmd)
        {
            scan_obj_method_region(ctx, inner_start, inner_end, true, depth + 1, sink);
        }
        let shifted = ctx.frame_shifted();
        if shifted.has_receivers() {
            for (inner_start, inner_end) in
                frame_shifted_dispatch_regions(source, ctx.analysis, ctx.dialect, cmd)
            {
                scan_obj_method_region(shifted, inner_start, inner_end, true, depth + 1, sink);
            }
        }
    }
}

/// Strip a single layer of `{`/`}` delimiters from `span`'s source text, if
/// present.  The analyser's body / member spans are inclusive of the braces,
/// but the segmenter treats a leading `{` as a braced-literal opener and
/// refuses to descend into it, so every re-scanned body needs the braces
/// stripped first.  Out-of-bounds / empty input is returned unchanged — the
/// caller is expected to bounds-check before segmenting.
pub(crate) fn strip_outer_braces(source: &str, span: tcl_lexer::Span) -> (usize, usize) {
    let mut start = span.start() as usize;
    let mut end = span.end() as usize;
    if start >= source.len() || end > source.len() || start > end {
        return (start, end);
    }
    if source.as_bytes().get(start) == Some(&b'{') {
        start += 1;
    }
    if end > start && source.as_bytes().get(end - 1) == Some(&b'}') {
        end -= 1;
    }
    (start, end)
}

/// Recursion bound for [`nested_dispatch_regions`] — mirrors the analyser's
/// own `MAX_BODY_DEPTH` (`tcl_compiler::analyser::commands`): a guard against
/// a stack overflow on pathologically nested / generated / minified Tcl, not
/// a limit any hand-written script should ever approach.
pub(crate) const MAX_DISPATCH_SCAN_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

struct RetainedDispatchContext {
    dialect: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
}

fn retained_dispatch_context(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<RetainedDispatchContext> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    Some(RetainedDispatchContext {
        dialect: current.profile(),
        config: current.config(),
        context: current.context(),
    })
}

/// Validated full source context shared by source scans and rename hazards.
/// This does not select an execution frame, target identity or edit permission.
pub(crate) fn dispatch_source_context(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<(
    tcl_lexer::LexerConfig,
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
)> {
    let selected = retained_dispatch_context(source, analysis)?;
    Some((selected.config, selected.context))
}

/// Active lexical command substitutions and potential same-frame script bodies
/// at the unchanged original call. The shared source schema retains moved and
/// captured operands, availability, script timing and known command barriers.
/// Conditional source regions grant neither runtime dispatch nor edit authority.
pub(crate) fn nested_dispatch_regions(
    source: &str,
    analysis: &AnalysisResult,
    _dialect: &'static tcl_dialect::DialectProfile,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<(usize, usize)> {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let Some(RetainedDispatchContext {
        config, context, ..
    }) = retained_dispatch_context(source, analysis)
    else {
        return Vec::new();
    };
    let mut regions = cmd
        .argv
        .iter()
        .flat_map(|token| {
            crate::executable_regions::command_substitution_regions(source, config, *token)
        })
        .collect::<Vec<_>>();
    if let Some(words) = tcl_compiler::registry_invocation::source_structure::source_registry_words(
        source, analysis, cmd,
    ) && words.with_source_schema(&context, |schema| {
        schema.semantics.body_kind == tcl_registry::BodyKind::Plain
    }) == Some(true)
    {
        regions.extend(words.source_script_bodies_for(&context,
            tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation)
            .into_iter().map(|body| {
                let span = body.content_span();
                (span.start() as usize, span.end() as usize)
            }));
    }
    regions
}

/// Potential structural bodies and genuinely braced lambda bodies selected by
/// the original source schema. Object scans drop variable receivers across this
/// boundary; command receivers still require their own positioned identity proof.
/// These regions describe source topology, never an entered runtime frame.
pub(crate) fn frame_shifted_dispatch_regions(
    source: &str,
    analysis: &AnalysisResult,
    _dialect: &'static tcl_dialect::DialectProfile,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<(usize, usize)> {
    // Source structure identifies possible shifted bodies, never an entered frame.
    let Some(RetainedDispatchContext { context, .. }) = retained_dispatch_context(source, analysis)
    else {
        return Vec::new();
    };
    let Some(words) = tcl_compiler::registry_invocation::source_structure::source_registry_words(
        source, analysis, cmd,
    ) else {
        return Vec::new();
    };
    let mut regions = Vec::new();
    if words.with_source_schema(&context, |schema| {
        schema.semantics.body_kind == tcl_registry::BodyKind::Structural
    }) == Some(true)
    {
        regions.extend(words.source_script_bodies_for(&context,
            tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation)
            .into_iter().map(|body| {
                let span = body.content_span();
                (span.start() as usize, span.end() as usize)
            }));
    }
    let executable = words
        .with_source_schema(&context, schema_script_arguments)
        .flatten()
        .unwrap_or_default();
    for (index, role) in words.roles().unwrap_or_default() {
        if *role != tcl_registry::ArgRole::LambdaLiteral || !executable.contains(index) {
            continue;
        }
        let Some(word) = words
            .operands()
            .get(*index)
            .and_then(Option::as_ref)
            .and_then(|operand| operand.word())
        else {
            continue;
        };
        if let Some(body) = tcl_compiler::lambda_literal::split_original_lambda_literal(word)
            .and_then(|fields| fields.braced_body())
        {
            regions.push((body.start() as usize, body.end() as usize));
        }
    }
    regions
}
/// Complete potential-source traversal with its actual per-command positive
/// child projections. Completeness belongs to the checked source walker;
/// neither an empty projection nor a missing command can issue a census.
struct RetainedDispatchRegions {
    command: tcl_compiler::segmenter::SegmentedCommand,
    regions: Vec<(usize, usize)>,
}

pub(crate) struct DispatchRegionCensus {
    image: tcl_lexer::SourceImage,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    commands: std::collections::BTreeMap<(u32, u32), Vec<RetainedDispatchRegions>>,
}

impl DispatchRegionCensus {
    pub(crate) fn capture(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let input = analysis.resolved_input.as_ref()?.clone();
        let mut commands = std::collections::BTreeMap::<_, Vec<_>>::new();
        let mut regions = Vec::new();
        let coverage =
            crate::executable_regions::visit_checked_analysis_executable_commands_with_regions(
                source,
                analysis,
                &mut |command, _, _| {
                    commands
                        .entry((command.span.start(), command.span.end()))
                        .or_default()
                        .push(RetainedDispatchRegions {
                            command: command.clone(),
                            regions: Vec::new(),
                        });
                    false
                },
                &mut regions,
            );
        if coverage != crate::executable_regions::PotentialSourceVisit::Complete {
            return None;
        }
        regions.sort_unstable_by_key(|(span, depth)| (span.start(), span.end(), *depth));
        for bucket in commands.values_mut() {
            for retained in bucket {
                let first = regions
                    .partition_point(|(span, _)| span.start() < retained.command.span.start());
                let last = regions
                    .partition_point(|(span, _)| span.start() <= retained.command.span.end());
                retained.regions = regions[first..last]
                    .iter()
                    .filter_map(|(span, _)| {
                        let command = &retained.command;
                        (command.span.start() <= span.start()
                            && span.end() <= command.span.end()
                            && *span != command.span)
                            .then_some((span.start() as usize, span.end() as usize))
                    })
                    .collect();
                retained.regions.sort_unstable();
                retained.regions.dedup();
            }
        }
        Some(Self {
            image: analysis
                .retained_command_realm()?
                .original_source_image()?
                .clone(),
            input,
            commands,
        })
    }

    /// Exact retained source/input/command membership, including genuine empty
    /// children. A source mismatch or absent command remains unavailable.
    pub(crate) fn regions_for(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        command: &tcl_compiler::segmenter::SegmentedCommand,
    ) -> Option<&[(usize, usize)]> {
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        if self.image.bytes() != source.as_bytes()
            || analysis.resolved_input.as_ref() != Some(&self.input)
            || current.config() != self.input.lexer_config()
        {
            return None;
        }
        let mut selected = self
            .commands
            .get(&(command.span.start(), command.span.end()))?
            .iter()
            .filter(|retained| retained.command.all_tokens == command.all_tokens);
        let selected_regions = &selected.next()?.regions;
        if selected.any(|other| other.regions != *selected_regions) {
            return None;
        }
        Some(selected_regions)
    }
}

/// Every nested script region a **rename-safety** scan must also visit from
/// one segmented command: the union of the same-frame set
/// ([`nested_dispatch_regions`]) and the frame-shifted set
/// ([`frame_shifted_dispatch_regions`]).
///
/// The reference scan keeps the two apart because a `$var` receiver's bare
/// name stops naming the enclosing frame's variable across a frame shift.
/// The safety gate has no such distinction to make: it is asking "is there a
/// dispatch anywhere in this document that this rename cannot account for",
/// and a hazard written inside a `namespace eval` / `uplevel` / `apply` body
/// is every bit as unrewritable as one written beside it.
pub(crate) fn dispatch_scan_regions(
    source: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    dialect: &'static tcl_dialect::DialectProfile,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<(usize, usize)> {
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let mut regions = nested_dispatch_regions(source, analysis, dialect, cmd);
    regions.extend(frame_shifted_dispatch_regions(
        source, analysis, dialect, cmd,
    ));
    regions
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

/// The class-name highlight set at the cursor, or `None` when the cursor is
/// not on one — the class arm of [`document_highlights`], lifted out so the
/// entry point stays a readable dispatch over symbol kinds.
fn class_highlights(
    source: &str,
    line_index: &LineIndex,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
    word: &str,
    resolution: crate::definition::CallResolution<'_>,
) -> Option<Vec<(LspRange, HighlightKind)>> {
    let cursor_off = crate::definition::byte_offset_at(line_index, source, line, character);
    // Declaration-span hit, else the namespace-aware candidate resolution —
    // never a namespace-blind `c.name == word` first-hit scan (the
    // wrong-symbol drift class).
    let (qname, class_def) =
        crate::definition::resolve_class_target_at(analysis, source, resolution, cursor_off, word)?;
    // Non-variable symbols (procs / classes / methods) highlight as `Text` for
    // both declaration and uses — only variables carry the Write/Read
    // distinction.
    let mut out = vec![(
        span_to_range(source, line_index, class_def.name_span),
        HighlightKind::Text,
    )];
    for span in class_reference_spans(analysis, resolution, qname, class_def, source) {
        out.push((span_to_range(source, line_index, span), HighlightKind::Text));
    }
    Some(dedup_kinded(out))
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
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<(LspRange, HighlightKind)> {
    document_highlights_in_program(
        source,
        dialect,
        line,
        character,
        analysis,
        crate::definition::CallResolution::document_only(),
    )
}

/// Convert already selected variable declaration/read spans into highlight
/// ranges. This is a pure report conversion: source identity and selection are
/// enforced by the caller before it supplies these spans.
fn variable_occurrence_highlights(
    source: &str,
    line_index: &LineIndex,
    sites: impl IntoIterator<Item = (tcl_lexer::Span, bool)>,
) -> Vec<(LspRange, HighlightKind)> {
    let mut highlights: Vec<_> = sites
        .into_iter()
        .map(|(span, is_declaration)| {
            (
                span_to_range(source, line_index, span),
                if is_declaration {
                    HighlightKind::Write
                } else {
                    HighlightKind::Read
                },
            )
        })
        .collect();
    highlights.sort_by_key(|(range, _)| (range.start_line, range.start_character));
    dedup_kinded(highlights)
}

/// [`document_highlights`] with the caller's whole-program export view
/// attached — the entry point a host with a workspace index should call.
///
/// Highlighting is find-references narrowed to one document, so it must pick
/// the same target: a bare call a live `namespace import -force` shadows is
/// not an occurrence of the local definition, and which definition it *does*
/// reach can only be settled with whole-program export knowledge.
#[must_use]
pub fn document_highlights_in_program(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<(LspRange, HighlightKind)> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    if let std::ops::ControlFlow::Break(selected) =
        crate::variable_symbol::select(source, analysis, line, character)
    {
        return selected.map_or_else(Vec::new, |occurrence| {
            let sites = crate::variable_symbol::occurrences(analysis, occurrence.symbol())
                .map(|site| (site.span(), site.is_declaration()))
                .collect::<Vec<_>>();
            // A read the document never declares anywhere resolves to no
            // definition, so there is nothing to highlight it against.
            if !sites.iter().any(|(_, is_declaration)| *is_declaration) {
                return Vec::new();
            }
            variable_occurrence_highlights(source, &line_index, sites)
        });
    }

    let byte_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let std::ops::ControlFlow::Break(selected) =
        crate::namespace_symbol::select_at_offset(source, analysis, byte_offset)
    {
        return selected.map_or_else(Vec::new, |symbol| {
            original_text_highlights(
                source,
                &line_index,
                crate::namespace_symbol::original_namespace_spans(analysis, &symbol, true),
            )
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::method_symbol::local_candidate(source, analysis, line, character)
    {
        return selected.map_or_else(Vec::new, |candidate| {
            original_text_highlights(
                source,
                &line_index,
                crate::method_symbol::readonly_local_reference_report(
                    source, analysis, &candidate, true,
                )
                .map_or_else(
                    Vec::new,
                    crate::original_declaration::ReadonlyReferenceReport::into_spans,
                ),
            )
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::original_declaration::select("", source, analysis, line, character)
    {
        return selected.map_or_else(Vec::new, |identity| {
            let spans = crate::original_declaration::readonly_reference_report(
                &identity, source, analysis, true,
            )
            .map_or_else(
                Vec::new,
                crate::original_declaration::ReadonlyReferenceReport::into_spans,
            )
            .into_iter()
            .chain(std::iter::once(identity.span()));
            original_text_highlights(source, &line_index, spans)
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
        include_declaration: true,
        resolution,
    };
    lexical_document_highlights(&ctx, byte_offset)
}

fn lexical_document_highlights(
    ctx: &RefCtx<'_>,
    byte_offset: u32,
) -> Vec<(LspRange, HighlightKind)> {
    let RefCtx {
        source,
        dialect,
        line_index,
        line,
        character,
        analysis,
        resolution,
        ..
    } = *ctx;
    if let Some(highlights) =
        selected_method_highlights(source, dialect, analysis, line_index, byte_offset)
    {
        return highlights;
    }
    if crate::receiver_identity::definition_reference_at_cursor(analysis, source, byte_offset)
        .is_some()
    {
        return Vec::new();
    }
    // Shared `$ref` gate — see `substituting_var_at_position`.
    if let Some(var_name) = crate::definition::substituting_var_at_position(
        source,
        analysis,
        line,
        character,
        byte_offset,
    ) {
        let Some(var_def) =
            crate::definition::lookup_var_read_at(analysis, source, byte_offset, &var_name)
        else {
            return Vec::new();
        };
        let Some(references) =
            crate::definition::linked_var_reference_spans(source, analysis, var_def, byte_offset)
        else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(1 + references.len());
        out.push((
            span_to_range(source, line_index, var_def.definition_span),
            HighlightKind::Write,
        ));
        // Highlight original source relationships without asserting alias execution.
        for r in references {
            out.push((span_to_range(source, line_index, r), HighlightKind::Read));
        }
        return dedup_kinded(out);
    }

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };

    if let Some(out) = class_highlights(
        source, line_index, analysis, line, character, &word, resolution,
    ) {
        return out;
    }

    {
        let cursor_off = crate::definition::byte_offset_at(line_index, source, line, character);
        if let Some((qname, proc_def)) = crate::definition::resolve_proc_target_at(
            analysis, source, cursor_off, &word, resolution,
        ) {
            let mut out = Vec::new();
            out.push((
                span_to_range(source, line_index, proc_def.name_span),
                HighlightKind::Text,
            ));
            for span in proc_reference_spans(analysis, resolution, qname, proc_def, source) {
                out.push((span_to_range(source, line_index, span), HighlightKind::Text));
            }
            return dedup_kinded(out);
        }
    }

    // Class-member highlights — re-segment sibling method
    // bodies via `find_class_member_references` and mark the
    // declaration as Write, every call site as Text.
    let cursor_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    if let Some((decl_span, call_spans)) =
        find_class_member_references(source, dialect, &word, analysis, cursor_offset)
    {
        let mut out = Vec::new();
        out.push((
            span_to_range(source, line_index, decl_span),
            HighlightKind::Text,
        ));
        for s in call_spans {
            out.push((span_to_range(source, line_index, s), HighlightKind::Text));
        }
        return dedup_kinded(out);
    }

    Vec::new()
}

fn original_text_highlights(
    source: &str,
    line_index: &LineIndex,
    spans: impl IntoIterator<Item = tcl_lexer::Span>,
) -> Vec<(LspRange, HighlightKind)> {
    let mut entries = spans
        .into_iter()
        .map(|span| (span_to_range(source, line_index, span), HighlightKind::Text))
        .collect::<Vec<_>>();
    entries.sort_by_key(|(range, _)| {
        (
            range.start_line,
            range.start_character,
            range.end_line,
            range.end_character,
        )
    });
    dedup_kinded(entries)
}

fn selected_method_highlights(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    byte_offset: u32,
) -> Option<Vec<(LspRange, HighlightKind)>> {
    let selected = crate::receiver_identity::method_at_cursor(analysis, source, byte_offset)?;
    let (declaration, references) = method_references_for_declaration(
        source,
        dialect,
        analysis,
        selected.class,
        selected.method,
        selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class,
    );
    Some(
        std::iter::once(declaration)
            .chain(references)
            .map(|span| (span_to_range(source, line_index, span), HighlightKind::Text))
            .collect(),
    )
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
