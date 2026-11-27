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

//! Which call sites reference a procedure or class definition: namespace resolution, indirection, imports and displaced definitions.

use tcl_compiler::analyser::AnalysisResult;

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
pub(super) fn logical_proc_reference_spans(
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

pub(super) fn logical_proc_at_cursor<'a>(
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

pub(super) fn logical_class_reference_spans(
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
pub(super) fn invocation_references_via_wildcard_import(
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

pub(super) enum RetainedDefinition<'a> {
    Absent,
    Known(Option<&'a tcl_compiler::command_binding::SourceCommandDefinition>),
}

/// Exact selected declaration, with called-slot identity for edits and terminal
/// identity for call edges. A known non-definition retains an explicit refusal and
/// blocks lexical assistance; foreign allocations are validated against the
/// caller's document bytes by the declaration helpers.
pub(super) fn retained_definition<'a>(
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
            tcl_names::origin::SourceOriginKind::Authored(authored) if authored.as_ref() == source.as_bytes())
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
