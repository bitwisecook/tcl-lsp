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

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::ir::MethodKind;
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
    let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname);
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
    // A user proc installed directly into `::oo::Helpers` (the documented
    // "TclOO Tricks" idiom — `proc ::oo::Helpers::classvar {...} {...}`,
    // real corpus usage: nico-robert/ticklecharts) is bare-callable from
    // every method body in the program via TclOO's own fixed runtime
    // namespace path — a search member `call_ns` alone can't represent,
    // since it's a single accumulated namespace string, not a path.
    let call_reaches_target = call_ns == target_ns
        || (target_ns == "oo::Helpers"
            && tcl_compiler::analyser::innermost_scope_reaches_oo_helpers(
                &analysis.global_scope,
                inv.range.start(),
            ));
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
    let line_index = LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
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
            crate::method_symbol::local_reference_spans(
                source,
                analysis,
                &candidate,
                include_declaration,
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
            let mut spans =
                crate::original_declaration::reference_spans(&identity, source, analysis, true);
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

    if let Some(out) = variable_references(&ctx) {
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
        crate::definition::byte_offset_at(&line_index, source, line, character),
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
    if let Some(out) = namespace_references(&ctx) {
        return out;
    }

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };

    let cursor_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
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
            out.push(span_to_range(source, &line_index, decl));
        }
        out.extend(
            sites
                .into_iter()
                .map(|span| span_to_range(source, &line_index, span)),
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
    if let Some(out) = class_references(&ctx, &word) {
        return out;
    }

    // `<ensemble> <subcommand>` — a static `namespace ensemble create
    // -map`/`-subcommands` mapping. Checked before
    // `proc_references`: real Tcl never independently looks up `make` as a
    // command (only the pair `widget make` dispatches), so a coincidental
    // same-named proc elsewhere in the workspace — which `proc_references`'s
    // namespace-aware call-site resolution could otherwise match — must
    // never win.
    if let Some(out) = ensemble_subcommand_references(&ctx) {
        return out;
    }

    // Proc references.
    if let Some(out) = proc_references(&ctx, &word) {
        return out;
    }

    // `Factory::make` — [incr Tcl]'s colon-qualified class-proc dispatch
    // Tried *after* `proc_references` so an ordinary
    // namespace-qualified proc call of the same spelling keeps priority:
    // this only ever fires when nothing resolves as a proc and the written
    // word's parent namespace really is an itcl class declaring that member.
    if let Some(out) = itcl_class_proc_references(&ctx) {
        return out;
    }

    // `$obj method` external call site.
    if let Some(out) = instance_method_references(&ctx) {
        return out;
    }

    // Bare `ClassName method` external call site — a classmethod's own
    // dispatch shape, tried only once the instance path above has failed.
    if let Some(out) = classmethod_call_site_references(&ctx) {
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
    if let Some(out) = constructor_or_destructor_references(&ctx, &word) {
        return out;
    }

    // Class-member references (cursor inside a class body on a member name).
    if let Some(out) = class_member_references(&ctx, &word) {
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
        &find_word_span_at_position(source, line, character)
            .map(|(w, _, _)| w)
            .unwrap_or_default(),
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
    ctx: crate::definition::CallResolution<'_>,
    qname: &str,
    class_def: &tcl_compiler::analyser::ClassDef,
    source: &str,
) -> Vec<tcl_lexer::Span> {
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
    let (qname, class_def) = crate::definition::resolve_class_target_at(
        analysis,
        source,
        ctx.resolution,
        cursor_off,
        word,
    )?;
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
    let (qname, proc_def) = crate::definition::resolve_proc_target_at(
        analysis,
        source,
        cursor_off,
        word,
        ctx.resolution,
    )?;
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
    let (head, sub, is_dollar) = crate::definition::instance_method_at_cursor(
        source,
        line,
        character,
        tcl_lexer::LexerConfig::for_profile(Some(ctx.dialect)),
    )?;
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

/// Resolve a property's declaration span plus every `my <property>` call
/// site — the property counterpart of [`method_references_for_class`], and
/// the single source of truth the code lens and the reference peek both
/// defer to so their counts can never drift.  Properties have no `$obj
/// property` dispatch shape and no inheritance model (confirmed against
/// tclsh 9.0.4), so a class-local `my <name>` scan — the same matcher
/// [`scan_my_method_sites`] uses for methods — is the whole story; a
/// property's own `property <name>` declaration is never itself a `my
/// <name>` call site, so there's no declaration span to skip. Returns
/// `None` when `class_q` has no property named `property`.
pub(crate) fn property_references_for_class(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    property: &str,
) -> Option<(tcl_lexer::Span, Vec<tcl_lexer::Span>)> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let class_def = analysis.all_classes.get(class_q)?;
    let decl_span = class_def.properties.get(property)?.name_span;
    let call_spans = scan_my_method_sites(
        source,
        analysis,
        &collect_member_bodies(class_def),
        property,
        None,
    );
    Some((decl_span, call_spans))
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

/// Canonicalise a written class name (`nextto`'s argument) to the qualified
/// form keyed in `analysis.all_classes`, owner-aware — the same resolution
/// `definition.rs`'s go-to-definition `next`/`nextto` handling uses (kept as
/// a separate copy rather than shared: it is a two-line wrapper around the
/// registry's own `resolve_class_name`, so a cross-module `pub(crate)`
/// promotion would cost more than it saves). Falls back to the written name
/// when nothing resolves, so the caller's MRO lookup simply finds no match.
fn canonicalise_class_name(analysis: &AnalysisResult, at: u32, name: &str) -> Option<String> {
    let namespace =
        tcl_compiler::analyser::class_hierarchy::source_namespace_at(&analysis.global_scope, at)?;
    tcl_compiler::analyser::class_hierarchy::resolve_written_class_name_in_scope(
        name,
        namespace,
        &analysis.all_classes,
    )
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
/// *does* consult `is_itcl_class` for an unrelated concern), so gating only
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
                    let Some(target) = canonicalise_class_name(analysis, span.start(), &target)
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
                    let Some(target) = canonicalise_class_name(analysis, span.start(), &target)
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
/// [`find_obj_method_call_sites`] derives it from `analysis.all_classes`
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
/// name"), so only `my`-headed sites match.  `pub(crate)` so
/// `call_hierarchy`'s method incoming/outgoing-call edges resolve through
/// the same matcher find-references / rename / the code lens use, instead
/// of a bare-head comparison that never matches real (`my`-dispatched) Tcl.
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
        dialect,
        registry,
        identities,
        config,
        ..
    }) = retained_dispatch_context(source, analysis)
    else {
        return Vec::new();
    };
    let ctx = MyMethodScan {
        source,
        analysis,
        config,
        dialect,
        registry,
        identities,
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
    registry: &'a tcl_registry::CommandRegistry,
    identities: &'a tcl_compiler::realm::CommandBindingRealm,
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
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    use tcl_lexer::TokenType;
    use tcl_registry::ArgRole;
    use tcl_registry::hooks::LoweringHookId;

    if MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) || start >= end || end > ctx.source.len() {
        return false;
    }
    let commands = segment_commands_with_offset_and_config(
        &ctx.source[start..end],
        u32::try_from(start).unwrap_or(0),
        ctx.config,
    );
    let mut has_scope_alias = false;
    for cmd in commands {
        let (Some(head), Some(written)) = (cmd.argv.first(), cmd.texts.first()) else {
            continue;
        };
        let resolved = ctx
            .identities
            .head_words(written, head.span.start())
            .resolved;
        let args: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
        // Only the registry's typed `Set` lowering relation says that the
        // value immediately follows the VarWrite name.  Other VarWrite
        // commands (lassign, scan, array set, …) deliberately do not enter
        // this callback-value path.
        let resolved_call = ctx.registry.resolve_call(
            resolved,
            &args,
            Some(crate::document_context_for_profile(ctx.dialect).authoring_query()),
        );
        has_scope_alias |= resolved_call.is_some_and(|call| {
            matches!(
                call.lowering_hook,
                Some(LoweringHookId::Global | LoweringHookId::Variable | LoweringHookId::Upvar)
            ) || call.sub.is_some_and(|sub| sub.creates_scope_alias)
        });
        let is_scalar_assignment =
            resolved_call.is_some_and(|call| call.lowering_hook == Some(LoweringHookId::Set));
        for var_idx in ctx
            .registry
            .arg_indices_for_role(resolved, &args, ArgRole::VarWrite)
        {
            let (Some(var), Some(&var_tok)) =
                (cmd.texts.get(var_idx + 1), cmd.argv.get(var_idx + 1))
            else {
                continue;
            };
            if var_tok.kind != TokenType::Esc
                || cmd.single_token_word.get(var_idx + 1) != Some(&true)
                || var.contains("::")
                || var.contains('(')
            {
                continue;
            }
            let count = counts.entry(var.clone()).or_default();
            *count += 1;
            let value_target = is_scalar_assignment
                .then(|| {
                    cmd.argv.get(var_idx + 2).and_then(|&value_tok| {
                        (value_tok.kind == TokenType::Cmd
                            && cmd.single_token_word.get(var_idx + 2) == Some(&true))
                        .then(|| command_prefix_targets_from_word(ctx, &value_tok, 0))
                        .and_then(|targets| (targets.len() == 1).then(|| targets[0]))
                    })
                })
                .flatten();
            writes.insert(var.clone(), (cmd.span.end(), value_target));
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
        ctx.config,
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
        ctx.config,
    );
    let [child] = commands.as_slice() else {
        return false;
    };
    let Some(words) = source_structure::source_registry_words(ctx.source, ctx.analysis, child)
    else {
        return false;
    };
    words.with_source_schema(&context, |schema| {
        let spec = schema.command();
        if spec.self_receiver_words.is_empty() {
            return false;
        }
        match words.arguments() {
            [] => spec.arity.min == 0,
            [argument] => argument
                .as_registry_word()
                .literal()
                .is_some_and(|word| spec.self_receiver_words.contains(&word)),
            _ => false,
        }
    }) == Some(true)
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
    use tcl_lexer::TokenType;
    use tcl_registry::ArgRole;

    let (Some(head), Some(written_name)) = (cmd.argv.first(), cmd.texts.first()) else {
        return Vec::new();
    };
    let resolved = ctx
        .identities
        .head_words(written_name, head.span.start())
        .resolved;
    let args: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
    let mut out = Vec::new();
    let mut callback_indices = ctx
        .registry
        .arg_indices_for_role(resolved, &args, ArgRole::Body);
    callback_indices.extend(ctx.registry.arg_indices_for_role(
        resolved,
        &args,
        ArgRole::CommandPrefix,
    ));
    callback_indices.sort_unstable();
    callback_indices.dedup();
    for idx in callback_indices {
        let Some(&body_tok) = cmd.argv.get(idx + 1) else {
            continue;
        };
        // `{*}` splices the word's value into the argument list, so the
        // registry's role indices no longer describe where anything landed:
        // `lsort -command {*}[list [self] compare] $items` runs the *object
        // command* as the comparator and passes `compare` as a separate
        // argument. Reading the word as if it were the callback slot invents a
        // reference to `compare` — and Rename would rewrite it. The compiler's
        // own prefix scan gates on this
        // (`signature_scan::command_prefix`); this scan had not.
        if !positions_are_literal_through(cmd, idx + 1) {
            continue;
        }
        if body_tok.kind == TokenType::Var && cmd.single_token_word.get(idx + 1) == Some(&true) {
            // A stored callback is accepted only when exactly one static
            // assignment to this local spelling precedes the use.  The
            // assignment table was built from registry VarWrite facts, so
            // this remains independent of command names and rejects every
            // ambiguous/reassigned form conservatively.
            let variable = source_span_text(ctx.source, body_tok.span);
            let variable = variable.strip_prefix('$').unwrap_or(variable);
            let variable = variable
                .strip_prefix('{')
                .and_then(|name| name.strip_suffix('}'))
                .unwrap_or(variable);
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
        // A compound outer word changes the command prefix after the list
        // substitution has run: `[list [self] tick]Suffix` invokes
        // `tickSuffix`, not `tick`.  Only a sole substitution is an exact,
        // safely renameable representation of the built command.
        if body_tok.kind != TokenType::Cmd || cmd.single_token_word.get(idx + 1) != Some(&true) {
            continue;
        }
        out.extend(command_prefix_targets_from_word(ctx, &body_tok, 0));
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

/// Extract static method words from one sole command substitution used as a
/// callback value.  The recursion deliberately requires each hop to be a
/// single substitution, so concatenated and dynamically-built values abstain.
fn command_prefix_targets_from_word(
    ctx: MyMethodScan<'_>,
    word_tok: &tcl_lexer::Token,
    depth: u32,
) -> Vec<PrefixTargetAtSpan> {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    use tcl_lexer::TokenType;
    use tcl_registry::{ArgRole, Traits};

    if MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) || word_tok.kind != TokenType::Cmd {
        return Vec::new();
    }
    let regions =
        crate::executable_regions::command_substitution_regions(ctx.source, ctx.config, *word_tok);
    let [(inner_start, inner_end)] = regions.as_slice() else {
        return Vec::new();
    };
    let built = segment_commands_with_offset_and_config(
        &ctx.source[*inner_start..*inner_end],
        u32::try_from(*inner_start).unwrap_or(0),
        ctx.config,
    );
    let [builder] = built.as_slice() else {
        return Vec::new();
    };
    let (Some(builder_head), Some(builder_written)) = (builder.argv.first(), builder.texts.first())
    else {
        return Vec::new();
    };
    let builder_resolved = ctx
        .identities
        .head_words(builder_written, builder_head.span.start())
        .resolved;
    let args: Vec<&str> = builder.texts.iter().skip(1).map(String::as_str).collect();
    let Some(invocation) = ctx.registry.resolve_invocation(
        builder_resolved,
        &args,
        Some(crate::document_context_for_profile(ctx.dialect).authoring_query()),
    ) else {
        return Vec::new();
    };
    let traits = invocation.semantics.traits;

    // The built prefix's own words are read by position too, so an expansion
    // inside it is as disqualifying as one at the consumer's callback slot.
    if !positions_are_literal_through(builder, 2) {
        return Vec::new();
    }
    if traits.contains(Traits::BUILDS_COMMAND_PREFIX) {
        if let (Some(receiver), Some(&method_tok)) = (builder.texts.get(1), builder.argv.get(2))
            && method_tok.kind == TokenType::Esc
            && !method_tok.in_quote
            && builder.single_token_word.get(1) == Some(&true)
            && builder.single_token_word.get(2) == Some(&true)
        {
            if exact_self_receiver_call(ctx, receiver) {
                return vec![PrefixTargetAtSpan {
                    kind: tcl_registry::CommandPrefixTarget::CurrentObjectExternalMethod,
                    span: method_tok.span,
                }];
            }
            if crate::definition::method_dispatch_keyword_in(ctx.dialect, receiver)
                == Some(tcl_registry::MethodDispatchKind::SelfDispatch)
            {
                return vec![PrefixTargetAtSpan {
                    kind: tcl_registry::CommandPrefixTarget::CurrentObjectInternalMethod,
                    span: method_tok.span,
                }];
            }
        }
        let Some(&head_tok) = builder.argv.get(1) else {
            return Vec::new();
        };
        return (builder.single_token_word.get(1) == Some(&true)
            && head_tok.kind == TokenType::Esc
            && !head_tok.in_quote)
            .then_some(PrefixTargetAtSpan {
                kind: tcl_registry::CommandPrefixTarget::DirectCommandHead,
                span: head_tok.span,
            })
            .into_iter()
            .collect();
    }

    if !traits.contains(Traits::WRAPS_COMMAND_PREFIX) {
        return Vec::new();
    }
    ctx.registry
        .arg_indices_for_role(builder_resolved, &args, ArgRole::Body)
        .into_iter()
        .flat_map(|idx| {
            let Some(&body) = builder.argv.get(idx + 1) else {
                return Vec::new();
            };
            if builder.single_token_word.get(idx + 1) == Some(&true)
                && positions_are_literal_through(builder, idx + 1)
            {
                command_prefix_targets_from_word(ctx, &body, depth + 1)
            } else {
                Vec::new()
            }
        })
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
        dialect,
        registry,
        identities,
        config,
        ..
    } = retained_dispatch_context(source, analysis)?;
    let base_ctx = MyMethodScan {
        source,
        analysis,
        config,
        dialect,
        registry,
        identities,
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
        ctx.config,
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
        ctx.config,
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

/// Whether a command-substitution word is exactly the registry-declared
/// current `TclOO` receiver form valid on a method frame's command path.
fn exact_self_receiver_call(ctx: MyMethodScan<'_>, receiver: &str) -> bool {
    let Some((written, args)) =
        tcl_compiler::value_shapes::parse_command_substitution_with_config(receiver, ctx.config)
    else {
        return false;
    };
    // A rooted spelling bypasses a method frame's command path. Reject it
    // when its resolved registry spec exists only through that method
    // context; a genuinely qualified helper has its own unscoped spec and is
    // accepted. Command identity stays wholly registry-owned.
    if written.starts_with("::") && ctx.registry.resolves_only_in_method_context(&written) {
        return false;
    }
    // TclOO installs `::oo::Helpers` on a method frame's command path, ahead
    // of global fallback. A global `proc self` therefore does not shadow the
    // helper. The receiver form is nevertheless exact: `self` takes no word,
    // while `self object` takes exactly that one declared selector.
    match args.as_slice() {
        [] => ctx.registry.is_self_receiver_call(&written, None),
        [arg] => ctx.registry.is_self_receiver_call(&written, Some(arg)),
        _ => false,
    }
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
fn scan_next_dispatch_sites(
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
/// argument as written (`None` for plain `next`, which takes no argument, or
/// for a malformed `nextto` with no argument token). Feeds the
/// constructor/destructor next-chain resolvers
/// ([`constructor_next_chain_references`] / [`destructor_next_chain_references`]),
/// which must know *which* class a `nextto` names to decide whether it
/// chains to the class under a given lens.
fn scan_next_dispatch_sites_with_target(
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
        config,
    );
    for cmd in &commands {
        if let Some(head) = cmd.argv.first() {
            let (h_start, h_end) = (head.span.start() as usize, head.span.end() as usize);
            if h_end <= source.len() && h_start < h_end {
                let h = &source[h_start..h_end];
                // The next-chain keywords come from the registry
                // (`TCLOO_NEXT_CHAIN`), not a name list.
                if crate::definition::method_dispatch_keyword_in(dialect, h)
                    == Some(tcl_registry::MethodDispatchKind::NextChain)
                {
                    // `texts` is the segmenter's already-*decoded* per-word
                    // reconstruction — unlike `argv`'s token span (which
                    // covers a braced/quoted word's raw delimiters per this
                    // codebase's body-span convention, e.g. `{Grandparent`
                    // for `nextto {Grandparent}`, dropping only the closer),
                    // `texts[1]` is plain `"Grandparent"` regardless of
                    // whether the target was written bare, braced, or
                    // quoted. Slicing the raw span instead left a literal
                    // `{`/`"` in the target text, which
                    // `canonicalise_class_name` could never resolve to a
                    // real class.
                    // Only the spelling that declares an `ArgRole::Name` at
                    // argument 0 names an explicit resume-from class —
                    // `nextto`'s structural marker, per `TCLOO_NEXT_CHAIN`'s
                    // own doc. `next` declares none, so it captures no target.
                    let names_target = crate::definition::next_chain_names_a_target_in(dialect, h);
                    let target = names_target.then(|| cmd.texts.get(1).cloned()).flatten();
                    out.push((head.span, target));
                }
            }
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

use crate::definition::is_itcl_class;

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
    analysis: &AnalysisResult,
    dialect: &'static tcl_dialect::DialectProfile,
    class_q: &str,
    member: &str,
) -> Vec<tcl_lexer::Span> {
    analysis
        .command_invocations
        .iter()
        .filter_map(|inv| {
            let namespace = crate::definition::namespace_context_at(
                &analysis.global_scope,
                inv.range.start(),
                &analysis.namespace_overrides,
            );
            let (target_class, target_member) = crate::definition::itcl_class_proc_target(
                analysis, dialect, &namespace, &inv.name,
            )?;
            if target_class != class_q || target_member != member {
                return None;
            }
            let tail = tcl_syntax::naming::written_command_tail(inv.name.as_bytes());
            let tail_len = u32::try_from(tail.len()).ok()?;
            Some(tcl_lexer::Span::new(
                inv.range.end().saturating_sub(tail_len),
                inv.range.end(),
            ))
        })
        .collect()
}
/// Editable receiver targets retain their original declaration spans. Names
/// supplied by a workspace caller remain candidates until a local source
/// declaration and the temporal method entry agree.
fn dispatch_receivers(
    analysis: &AnalysisResult,
    dialect: &'static tcl_dialect::DialectProfile,
    target: (&str, &str, bool),
    extra_cmd_names: &[String],
) -> CommandReceivers {
    let (class_q, method, is_classmethod) = target;
    let mut receivers = CommandReceivers {
        class_targets: FxHashMap::default(),
        instance_target: None,
    };
    if let Some(class) = analysis.all_classes.get(class_q) {
        if is_classmethod && !is_itcl_class(class, dialect) {
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

/// [`find_obj_method_call_sites`], plus `extra_cmd_names` — bare command
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
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    // [incr Tcl] class-scoped `proc`s land in the same `class_methods`
    // bucket as a `classmethod`, but dispatch as a single `::`-qualified
    // identifier (`Factory::make`) — a shape the two-word scanner below
    // cannot see, and whose two-word look-alike is unrelated
    // instance-creation syntax.  Collected first so every consumer of this
    // scanner (references, rename, the code lens, call hierarchy) gets itcl
    // edges from one place.
    let mut out: Vec<tcl_lexer::Span> = if is_classmethod
        && analysis
            .all_classes
            .get(class_q)
            .is_some_and(|cd| is_itcl_class(cd, dialect))
    {
        itcl_class_proc_call_sites(analysis, dialect, class_q, method)
    } else {
        Vec::new()
    };
    let receivers = dispatch_receivers(
        analysis,
        dialect,
        (class_q, method, is_classmethod),
        extra_cmd_names,
    );
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
        scan_obj_method_region(ctx, 0, source.len(), 0, &mut sink);
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
    scan_obj_method_region(ctx, start, end, 0, sink);
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
        ctx.config,
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
            scan_obj_method_region(ctx, inner_start, inner_end, depth + 1, sink);
        }
        let shifted = ctx.frame_shifted();
        if shifted.has_receivers() {
            for (inner_start, inner_end) in
                frame_shifted_dispatch_regions(source, ctx.analysis, ctx.dialect, cmd)
            {
                scan_obj_method_region(shifted, inner_start, inner_end, depth + 1, sink);
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

struct RetainedDispatchContext<'a> {
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &'a tcl_registry::CommandRegistry,
    identities: &'a tcl_compiler::realm::CommandBindingRealm,
    config: tcl_lexer::LexerConfig,
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
}

fn retained_dispatch_context<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
) -> Option<RetainedDispatchContext<'a>> {
    let input = analysis.resolved_input.as_ref()?;
    let config = analysis.body_lexer_config?;
    (input.lexer_config() == config
        && analysis
            .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config))
    .then_some(())?;
    let identities = analysis.retained_command_realm()?;
    identities
        .matches_resolved_analysis_input(input)
        .then_some(())?;
    Some(RetainedDispatchContext {
        dialect: analysis.resolved_profile()?,
        registry: analysis.resolved_registry()?,
        identities,
        config,
        context: input.context_registry(),
    })
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
        .with_source_schema(&context, |schema| schema.authored_source_script_arguments())
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

/// Strip a `$name` / `${name}` decoration to the bare variable
/// name.  Returns `None` when the text isn't a `$`-prefixed
/// reference.
pub(crate) fn strip_var_decoration(raw: &str) -> Option<&str> {
    if !raw.starts_with('$') {
        return None;
    }
    // Keep an unclosed `${x` decorated; only a complete `${x}` is a
    // reference to the real variable `x`.
    let inner = tcl_syntax::naming::var_reference(raw);
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
    let line_index = LineIndex::new(source);
    if let std::ops::ControlFlow::Break(selected) =
        crate::variable_symbol::select(source, analysis, line, character)
    {
        return selected.map_or_else(Vec::new, |occurrence| {
            let mut highlights: Vec<_> =
                crate::variable_symbol::occurrences(analysis, occurrence.symbol())
                    .map(|site| {
                        (
                            span_to_range(source, &line_index, site.span()),
                            if site.is_declaration() {
                                HighlightKind::Write
                            } else {
                                HighlightKind::Read
                            },
                        )
                    })
                    .collect();
            highlights.sort_by_key(|(range, _)| (range.start_line, range.start_character));
            dedup_kinded(highlights)
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
                crate::method_symbol::local_reference_spans(source, analysis, &candidate, true),
            )
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::original_declaration::select("", source, analysis, line, character)
    {
        return selected.map_or_else(Vec::new, |identity| {
            let spans =
                crate::original_declaration::reference_spans(&identity, source, analysis, true)
                    .into_iter()
                    .chain(std::iter::once(identity.span()));
            original_text_highlights(source, &line_index, spans)
        });
    }
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    if let Some(highlights) =
        selected_method_highlights(source, dialect, analysis, &line_index, byte_offset)
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

    if let Some(out) = class_highlights(
        source,
        &line_index,
        analysis,
        line,
        character,
        &word,
        resolution,
    ) {
        return out;
    }

    {
        let cursor_off = crate::definition::byte_offset_at(&line_index, source, line, character);
        if let Some((qname, proc_def)) = crate::definition::resolve_proc_target_at(
            analysis, source, cursor_off, &word, resolution,
        ) {
            let mut out = Vec::new();
            out.push((
                span_to_range(source, &line_index, proc_def.name_span),
                HighlightKind::Text,
            ));
            for span in proc_reference_spans(analysis, resolution, qname, proc_def, source) {
                out.push((
                    span_to_range(source, &line_index, span),
                    HighlightKind::Text,
                ));
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
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn retained_colon_procedure_does_not_reference_the_empty_name() {
        let source = "proc {} {} {return EMPTY}; proc : {} {return COLON}; :";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse(source, dialect);
            let invocation = execution_at(&analysis, source, ":");
            assert!(invocation.resolved_definition.is_some(), "{dialect}");
            assert!(invocation_calls_named(
                &analysis, invocation, ":::", ":", ":::", source
            ));
            assert!(!invocation_calls_named(
                &analysis, invocation, "::", "", "::", source
            ));
        }
    }

    fn execution_at<'a>(
        analysis: &'a AnalysisResult,
        source: &str,
        written: &str,
    ) -> &'a tcl_compiler::signature_scan::types::SignatureCommandInvocation {
        let offset = u32::try_from(source.rfind(written).unwrap()).unwrap();
        analysis
            .command_invocations
            .iter()
            .find(|invocation| {
                invocation.lookup.is_execution_site() && invocation.range.start() == offset
            })
            .expect("actual command head must retain its positioned lookup")
    }

    #[test]
    fn loaded_terminal_can_report_an_external_call_without_local_edit_authority() {
        use std::sync::Arc;
        use tcl_compiler::command_binding::{
            SourceAnalysisEntry, SourceOriginKind, TrustedSourceModuleLoader,
        };
        let source = "proc greet {} {return local}\nsource provider.tcl\nset cmd greet\n$cmd\n";
        let entry = Arc::new(SourceAnalysisEntry {
            trusted_source_modules: vec![TrustedSourceModuleLoader::new(
                tcl_dialect::model::Family::Tcl,
                "provider.tcl".to_owned(),
                None,
                "loaded-provider".to_owned(),
                &Arc::from("proc greet {} {return loaded}"),
            )],
            invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V8_6,
            )),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                ..Default::default()
            },
            ..Default::default()
        });
        let analysis = Analyser::new()
            .with_source_analysis_entry(entry)
            .analyse(source, "tcl8.6")
            .clone();
        let local = &analysis.all_procs["::greet"];
        let call = execution_at(&analysis, source, "$cmd");
        let reference = call.resolved_command_reference.as_ref().unwrap();
        let definition = reference
            .linked_definition()
            .or_else(|| reference.definition())
            .unwrap();
        assert!(matches!(
            definition.allocation().site.source.kind(),
            SourceOriginKind::Loaded { .. }
        ));
        assert!(!invocation_calls_proc(
            &analysis, call, "::greet", local, source
        ));
        assert!(!invocation_references_proc(
            &analysis, call, "::greet", local, source
        ));
        assert!(!invocation_references_named(
            &analysis, call, "::greet", "greet", "::greet", source
        ));
        assert!(invocation_calls_named(
            &analysis, call, "::greet", "greet", "::greet", source
        ));
        assert!(
            call.indirect,
            "the frozen command value is not a written head"
        );
        assert!(crate::rename::prepare_rename(source, 2, 9, &analysis).is_none());

        let local_source = "proc greet {} {return local}\nset cmd greet\n$cmd\n";
        let local_analysis = analyse(local_source);
        let local_call = execution_at(&local_analysis, local_source, "$cmd");
        assert!(local_call.indirect && local_call.rename_safe);
        assert!(local_call.resolved_command_reference.is_some());
        assert!(crate::rename::prepare_rename(local_source, 1, 9, &local_analysis).is_some());
    }

    fn retained_method_sites(source: &str, is_classmethod: bool) -> Vec<tcl_lexer::Span> {
        let analysis = analyse(source);
        find_obj_method_call_sites(
            source,
            analysis.resolved_profile().unwrap(),
            &analysis,
            "::C",
            "ping",
            is_classmethod,
        )
    }

    #[test]
    fn class_method_references_require_the_original_temporal_entry() {
        let source = "oo::class create C {self method ping {} {return CLASS}}; C ping";
        let sites = retained_method_sites(source, true);
        assert_eq!(sites.len(), 1, "{sites:?}");
        assert_eq!(sites[0].start() as usize, source.rfind("ping").unwrap());
    }

    #[test]
    fn class_method_references_refuse_replacement_during_arguments() {
        for source in [
            "oo::class create C {self method ping {x} {return CLASS}}; C ping [oo::objdefine C method ping {x} {return REPLACED}]",
            "oo::class create C {self method ping {} {return CLASS}}; rename C old; proc C args {return DECOY}; C ping",
        ] {
            assert!(retained_method_sites(source, true).is_empty(), "{source}");
        }
    }

    #[test]
    fn instance_method_references_require_the_retained_head_object() {
        let source =
            "oo::class create C {method ping {} {return INSTANCE}}; set obj [C new]; $obj ping";
        let sites = retained_method_sites(source, false);
        assert_eq!(sites.len(), 1, "{sites:?}");
        assert_eq!(sites[0].start() as usize, source.rfind("ping").unwrap());
    }

    #[test]
    fn instance_method_references_refuse_mutated_dispatch_and_nominal_decoys() {
        for source in [
            "oo::class create C {method ping {x} {return INSTANCE}}; set obj [C new]; $obj ping [oo::define C method ping {x} {return REPLACED}]",
            "oo::class create C {method ping {} {return INSTANCE}}; set obj [C new]; set obj ::puts; $obj ping",
        ] {
            assert!(retained_method_sites(source, false).is_empty(), "{source}");
        }
    }

    #[test]
    fn retained_alias_calls_do_not_become_target_rename_edits() {
        let source =
            "proc target {} {return OK}\ninterp alias {} wrapper {} target\nwrapper\ntarget\n";
        let analysis = analyse(source);
        let proc_def = &analysis.all_procs["::target"];
        let alias = execution_at(&analysis, source, "wrapper\n");
        assert!(
            alias
                .resolved_command_reference
                .as_ref()
                .unwrap()
                .linked_definition()
                .is_some()
        );
        assert!(invocation_calls_proc(
            &analysis, alias, "::target", proc_def, source
        ));
        assert!(!invocation_references_proc(
            &analysis, alias, "::target", proc_def, source
        ));
        let direct = execution_at(&analysis, source, "target\n");
        assert!(invocation_calls_proc(
            &analysis, direct, "::target", proc_def, source
        ));
        assert!(invocation_references_proc(
            &analysis, direct, "::target", proc_def, source
        ));
    }

    #[test]
    fn retained_proc_redefinitions_keep_original_reference_sets_separate() {
        let source = "proc p {} {return OLD}\np\nrename p old\nproc p {} {return NEW}\nold\np\n";
        let analysis = analyse(source);
        let first = analysis
            .proc_declarations("::p")
            .min_by_key(|proc_def| proc_def.name_span.start())
            .unwrap();
        let second = &analysis.all_procs["::p"];
        assert_ne!(first.name_span, second.name_span);
        let old = execution_at(&analysis, source, "old\n");
        let new = execution_at(&analysis, source, "p\n");
        assert!(invocation_calls_proc(&analysis, old, "::p", first, source));
        assert!(!invocation_calls_proc(
            &analysis, old, "::p", second, source
        ));
        assert!(invocation_references_proc(
            &analysis, new, "::p", second, source
        ));
        assert!(!invocation_references_proc(
            &analysis, new, "::p", first, source
        ));
        let first_spans = proc_reference_spans(
            &analysis,
            crate::definition::CallResolution::document_only(),
            "::p",
            first,
            source,
        );
        assert!(first_spans.contains(&old.range));
        assert!(!first_spans.contains(&new.range));
    }

    #[test]
    fn retained_builtin_wrapper_cannot_borrow_an_old_class_name() {
        let source = "oo::class create C {}\nrename C saved\ninterp alias {} C {} list\nC VALUE\n";
        let analysis = analyse(source);
        let class_def = &analysis.all_classes["::C"];
        let call = execution_at(&analysis, source, "C VALUE");
        let reference = call
            .resolved_command_reference
            .as_ref()
            .expect("known alias slot");
        assert!(reference.definition().is_none());
        assert!(reference.linked_definition().is_none());
        assert!(!invocation_references_class(
            &analysis, call, "::C", class_def, source
        ));
        assert!(!invocation_references_named(
            &analysis, call, "::C", "C", "::C", source
        ));
        assert!(
            !class_reference_spans(
                &analysis,
                crate::definition::CallResolution::document_only(),
                "::C",
                class_def,
                source
            )
            .contains(&call.range)
        );
    }

    #[test]
    fn references_reach_the_call_site_from_the_stale_original_in_a_foreach_rename_reinstall_idiom()
    {
        // TP — mirroring the same-file precedent set by
        // `references_reach_the_call_site_from_a_shadowed_duplicate_proc_decl_same_document`:
        // cursor on a superseded declaration resolves by *name*,
        // landing on whichever declaration currently wins under that name
        // — not the stale span the cursor happened to start on. Here the
        // "shadowing" declaration is the `tk/library/accessibility.tcl`
        // rename-and-reinstall idiom's own per-element wrapper, reached only
        // by simulating each literal `foreach`
        // element rather than by a second textual `proc` statement.
        let src = "proc button {args} {return orig_button}\n\
                   proc entry {args} {return orig_entry}\n\
                   namespace eval ::tk::accessible {\n    \
                   foreach wtype {button entry} {\n        \
                   rename ::$wtype ::tk::accessible::orig_$wtype\n        \
                   proc ::$wtype {args} {return wrapped}\n    \
                   }\n\
                   }\n\
                   set r1 [button .b1]\n\
                   set r2 [entry .e1]\n";
        let analysis = analyse(src);
        // Cursor on the STALE original `proc button` declaration (line 0).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&5), "winning wrapper decl missing: {refs:?}");
        assert!(lines.contains(&8), "call site missing: {refs:?}");
    }

    #[test]
    fn references_reach_a_method_return_captured_dispatch_site() {
        // `b` is typed only by the object-type lattice (`set b [$a make]`, the
        // method-return edge) — the analyser's `instance_classes` never binds
        // it, so only the lattice reading lets Find References reach the `$b
        // greet` site that semantic tokens and hover resolve.
        let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   set a [A new]\n\
                   set b [$a make]\n\
                   $b greet\n";
        let analysis = analyse(src);
        assert!(
            !analysis.instance_classes.contains_key("b"),
            "premise: the analyser walk alone must not bind `b` \
             (instance_classes: {:?})",
            analysis.instance_classes
        );
        // Cursor on the `greet` declaration (line 1, col 29).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            29,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(
            lines.contains(&4),
            "the lattice-typed `$b greet` call site is missing: {refs:?}"
        );
        // …and from the call site itself, the declaration answers back.
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            4,
            4,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(
            lines.contains(&1),
            "references from the call site must reach the declaration: {refs:?}"
        );
    }

    #[test]
    fn references_skip_a_same_named_untyped_variable_in_another_scope() {
        // FP guard for the lattice half of the scan: the lattice types `b`
        // inside `::mk` only (`by_scope`); a same-named integer in `::other`
        // must not become a reference — the scope-keyed map is exactly what
        // stops the bare-name collision.
        let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   proc mk {} { set a [A new]\n  set b [$a make]\n  $b greet }\n\
                   proc other {} { set b 7\n  $b greet }\n";
        let analysis = analyse(src);
        // Cursor on the `greet` declaration (line 1, col 29).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            29,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(
            lines.contains(&4),
            "the lattice-typed site inside ::mk is missing: {refs:?}"
        );
        assert!(
            !lines.contains(&6),
            "::other's `b` is an untyped integer; its `$b greet` must not be \
             a reference: {refs:?}"
        );
    }

    #[test]
    fn references_to_proc_include_decl_and_calls() {
        let src = "proc greet {} {}\ngreet\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the first `greet` reference (line 1).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            2,
            &analysis,
            true,
        );
        assert!(refs.len() >= 2, "expected decl + call sites: {refs:?}");
        // First entry is the declaration on line 0.
        assert_eq!(refs[0].start_line, 0);
    }

    #[test]
    fn references_include_wildcard_imported_bareword_call_same_document() {
        // TP (same-document) — a wildcard `namespace
        // import ::Foo::*` reaches an exported proc via a bare call with no
        // real command recorded at any of the call's own candidate names,
        // so `invocation_references_named`'s ordinary rule can never catch
        // it; `proc_reference_spans` ORs in
        // `invocation_references_via_wildcard_import` for exactly this
        // case.
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
        let analysis = analyse(src);
        // Cursor on the `bar` declaration (line 1, col 9).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            9,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(
            lines.contains(&5),
            "wildcard-imported bareword call site missing: {refs:?}"
        );
    }

    #[test]
    fn references_keep_a_wildcard_imported_call_after_a_later_export_clear() {
        // TP, direction A — the import bound `p` while `::src`
        // still exported it; the later `namespace export -clear` does not
        // revoke that alias (oracle tclsh 8.6.14/9.0.4: `::dst::p` still
        // runs, `info commands ::dst::*` still lists it). Without the
        // per-import-site snapshot the export gate reads the *final* export
        // set — which the `-clear` has emptied — and find-references silently
        // drops this real call site.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n    p\n}\nnamespace eval src {\n    namespace export -clear\n}\n";
        let analysis = analyse(src);
        // Cursor on the `p` declaration (line 1, col 9).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            9,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(
            lines.contains(&6),
            "the imported call site must survive a later `-clear`: {refs:?}"
        );
    }

    #[test]
    fn references_exclude_a_wildcard_imported_call_the_import_predates_the_export_of() {
        // FP guard (CRITICAL), direction B — `::src` exports `p`
        // only *after* `::dst` imported `::src::*`, so real Tcl never binds
        // `::dst::p` at all (oracle: `invalid command name "::dst::p"`) and
        // the bare `p` inside `::dst` is not a call to `::src::p`. Reading
        // the final export set would report it as one.
        let src = "namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n    p\n}\nnamespace eval src {\n    namespace export p\n}\n";
        let analysis = analyse(src);
        // Cursor on the `p` declaration (line 1, col 9).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            9,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(
            !lines.contains(&5),
            "an export written after the import must not make the bare call a \
             reference: {refs:?}"
        );
    }

    #[test]
    fn references_drop_a_call_after_a_namespace_forget() {
        // TN, the `namespace forget` behaviour — the alias the import installed is
        // gone by the time the second `p` runs (oracle: `invalid command
        // name "p"`), so that call is not a reference to `::src::p`. The
        // call *before* the forget still is.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n    p\n    namespace forget ::src::p\n    p\n}\n";
        let analysis = analyse(src);
        // Cursor on the `p` declaration (line 1, col 9).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            9,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(
            lines.contains(&6),
            "the call before the forget is still a reference: {refs:?}"
        );
        assert!(
            !lines.contains(&8),
            "the call after the forget reaches no command: {refs:?}"
        );
    }

    #[test]
    fn references_drop_a_conflicting_unforced_imports_call_site() {
        // FP guard (CRITICAL), the import-conflict behaviour — `::dst` already has
        // its own `p`, so the non-`-force` import errors and installs
        // nothing (oracle: `can't import command "p": already exists`, and
        // `namespace origin ::dst::p` → `::dst::p`). The bare `p` inside
        // `::dst` runs the *local* proc and is not a reference to `::src::p`.
        let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n    namespace import ::src::*\n    p\n}\n";
        let analysis = analyse(src);
        // Cursor on `::src::p`'s declaration (line 1, col 9).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            9,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(
            !lines.contains(&7),
            "a failed import makes no call site of the source: {refs:?}"
        );
    }

    #[test]
    fn references_include_a_forced_imports_call_site() {
        // TP, the other half of the row above — with `-force` the import
        // replaces the local `p`, so the bare call *is* a reference to
        // `::src::p` (oracle: `namespace origin ::dst::p` → `::src::p`).
        let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n    namespace import -force ::src::*\n    p\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            9,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(
            lines.contains(&7),
            "a `-force` import makes the bare call a reference to the source: {refs:?}"
        );
    }

    #[test]
    fn references_include_a_call_through_an_import_chain() {
        // TP, the import-chain behaviour — `::A` imports `::B::*`, `::B`
        // imported `::C::*` and re-exported; the bare `p` in `::A` runs
        // `::C::p` (oracle: `namespace origin ::A::p` → `::C::p`), so it is
        // a reference to it. The middle hop is in no `all_procs`, so a
        // single-hop walk finds nothing.
        let src = "namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n    namespace export p\n}\nnamespace eval A {\n    namespace import ::B::*\n    p\n}\n";
        let analysis = analyse(src);
        // Cursor on `::C::p`'s declaration (line 1, col 9).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            9,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(
            lines.contains(&10),
            "the chained bare call is a reference to ::C::p: {refs:?}"
        );
    }

    #[test]
    fn references_include_bare_call_to_a_proc_installed_into_oo_helpers() {
        // A proc
        // installed directly into `::oo::Helpers` (the documented "TclOO
        // Tricks" idiom — nico-robert/ticklecharts installs `classvar` /
        // `callback` this way) becomes bare-callable from every TclOO
        // method body in the program via TclOO's own fixed runtime
        // namespace path. tclsh9.0/8.6 both prove the bare `classvar hits`
        // call genuinely dispatches to `::oo::Helpers::classvar` —
        // find-references must reach it, not just the declaration.
        let src = "proc ::oo::Helpers::classvar {name} {\n    set ns [uplevel 1 {my getONSClass}]\n    tailcall namespace upvar $ns $name $name\n}\noo::class create Counter {\n    variable _label\n    constructor {label} { set _label $label }\n    method getONSClass {} { return [self class] }\n    method bump {} {\n        classvar hits\n        incr hits\n        return \"$_label:$hits\"\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `classvar` declaration (line 0, col 20).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            20,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {refs:?}");
        assert!(
            lines.contains(&9),
            "the bare classvar call site inside the method body is missing: {refs:?}"
        );
    }

    #[test]
    fn references_exclude_a_top_level_bare_call_with_the_same_name_as_an_oo_helpers_proc() {
        // FP guard: a bare call outside any TclOO
        // method body must not be treated as reaching a proc installed in
        // `::oo::Helpers` — real tclsh raises "invalid command name" there
        // (`::oo::Helpers` is on a *method body's* runtime namespace path
        // only, never the global one) — only a call genuinely inside a
        // method body's own runtime namespace path does.
        let src = "proc ::oo::Helpers::classvar {name} {}\nclassvar hits\n";
        let analysis = analyse(src);
        // Cursor on the `::oo::Helpers::classvar` declaration (line 0, col 20).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            20,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(
            lines,
            vec![0],
            "a top-level bare call outside any method body must not be linked: {refs:?}"
        );
    }

    #[test]
    fn references_from_in_proc_global_alias_reach_the_callers_canonical_set() {
        // Reduces the real `isEqual`/`tolComp` shape from
        // nico-robert/pix's test/data_b64.test — a proc aliases a top-level
        // cell via `global`, and the caller overrides it via a plain `set
        // ::name` before invoking the proc. tclsh proves `tolComp` (via
        // `global`) and `::tolComp` (the caller's `set`) are the identical
        // storage cell. Querying from the in-proc `$tolComp` read must reach
        // the caller's `set ::tolComp`: `collect_alias_spans` on its own finds
        // *other aliases* of the same target, never the target's own canonical
        // (non-aliased) declaration.
        let src =
            "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
        let analysis = analyse(src);
        // Cursor on the `$tolComp` read inside the proc (line 2, col 14).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            2,
            14,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(
            lines.contains(&1),
            "the `global tolComp` decl itself must still be present: {refs:?}"
        );
        assert!(
            lines.contains(&2),
            "the in-proc $tolComp read must still be present: {refs:?}"
        );
        assert!(
            lines.contains(&4),
            "the caller's canonical `set ::tolComp 0.05` must now be reached: {refs:?}"
        );
    }

    #[test]
    fn references_from_the_callers_canonical_set_reach_the_in_proc_global_alias() {
        // The reverse direction of the test above: querying from the caller's
        // own `set ::tolComp` must not return only its own 2 spans (decl + any
        // top-level reads) and miss every in-proc `global tolComp` occurrence,
        // which is what a plain `set` — with no `link_target` of its own to
        // search alias records by — would otherwise do.
        let src =
            "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
        let analysis = analyse(src);
        // Cursor on the `set ::tolComp` declaration (line 4, col 8, inside
        // "tolComp" — `set ::tolComp 0.05` has "tolComp" starting at col 6).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            4,
            8,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(
            lines.contains(&4),
            "the caller's own decl must still be present: {refs:?}"
        );
        assert!(
            lines.contains(&1),
            "the in-proc `global tolComp` decl must now be reached: {refs:?}"
        );
        assert!(
            lines.contains(&2),
            "the in-proc $tolComp read must now be reached: {refs:?}"
        );
    }

    #[test]
    fn references_unify_global_alias_and_canonical_set_when_the_set_is_unqualified() {
        // A second repro: an *unqualified* `set tolComp
        // 0.05` at global scope reproduces the identical split, ruling out
        // the `::`-prefix as the sole cause — `set`'s binding never
        // calls `set_var_link_target` regardless of how the name is spelled,
        // so the shape is the same either way.
        let src =
            "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset tolComp 0.05\nuse\n";
        let analysis = analyse(src);
        // Cursor on the unqualified `set tolComp` declaration (line 4, col 6).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            4,
            6,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&4), "the caller's own decl: {refs:?}");
        assert!(
            lines.contains(&1),
            "the in-proc `global tolComp` decl must be reached even from an unqualified set: {refs:?}"
        );
        assert!(
            lines.contains(&2),
            "the in-proc $tolComp read must be reached even from an unqualified set: {refs:?}"
        );
    }

    #[test]
    fn references_do_not_conflate_unrelated_same_named_cells_in_different_namespaces() {
        // FP guard: the canonical-cell fold-in must
        // still be exact-qualified-name matched — two unrelated top-level
        // `tolComp` cells living in different namespaces, neither aliasing
        // the other, must never be unioned together just because they share
        // a bare name.
        let src = "namespace eval A {\n    variable tolComp 1\n}\nnamespace eval B {\n    variable tolComp 2\n}\n";
        let analysis = analyse(src);
        // Cursor on `A::tolComp`'s declaration (line 1, col 13).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            13,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(
            lines,
            vec![1],
            "an unrelated same-named cell in a different namespace must not be pulled in: {refs:?}"
        );
    }

    #[test]
    fn references_for_a_multi_list_foreach_second_varlist_now_reach_every_use() {
        // Unless the first loop's own `name` (the second varList of `foreach
        // dirName {...} name {...} {...}`) is bound, `references()` from *any*
        // use inside the first loop's body falls through to whatever *other*
        // same-named `VarDef` exists anywhere in the flat top-level scope
        // — here, a second, later, textually unrelated `foreach name
        // {...}` — returning only that second loop's own 2 spans and
        // omitting every actual first-loop span, including the query site
        // itself. `foreach` (like `if`/`set`) introduces no new analyser
        // scope (correctly modelling Tcl's lack of block scoping), so at
        // the top level these two loops' `name` genuinely share one global
        // storage cell — same as any two sequential top-level `set name
        // ...` statements — so the correct reference set spans
        // *both* loops, not just the first: the bug was under-reporting
        // (missing the first loop's spans entirely), not over-reporting.
        let src = "foreach dirName {src src {src core}} name {alpha beta gamma} {\n    puts \"$dirName $name\"\n    if {$name eq \"pixutils\"} { puts skip }\n}\nforeach name {examples color changes} {\n    puts $name.ruff\n}\n";
        let analysis = analyse(src);
        // Cursor on the first loop's own `$name` read (line 1, col 21) —
        // the exact query shape the finding's own repro used.
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            21,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(
            lines.contains(&0),
            "the first loop's own `name` decl (line 0) must no longer be missing: {refs:?}"
        );
        assert!(
            lines.contains(&1),
            "the query site itself (line 1): {refs:?}"
        );
        assert!(
            lines.contains(&2),
            "the first loop's other in-body use (line 2): {refs:?}"
        );
        assert!(
            lines.contains(&4) && lines.contains(&5),
            "the second loop shares the same global cell (no block scoping) so its spans stay unified too: {refs:?}"
        );
    }

    #[test]
    fn references_do_not_include_unexported_sibling_wildcard_call_same_document() {
        // FP guard — the unexported sibling must not surface as a reference
        // through the wildcard import either.
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    proc other {} { return 2 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nother\n";
        let analysis = analyse(src);
        // Cursor on the `other` declaration (line 2, col 9).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            2,
            9,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(lines, vec![2], "only the declaration itself: {refs:?}");
    }

    #[test]
    fn references_reach_the_call_site_from_a_shadowed_duplicate_proc_decl_same_document() {
        // TN-shaped guard: `resolve_proc_target_at`'s own fallback resolves
        // the word text via ordinary namespace lookup when the direct
        // declaration-span match misses, landing on the current winner
        // regardless of which occurrence's span the cursor sits on.  Pinned so
        // the cross-document tier cannot change this same-document answer.
        let src = "proc List2array {lst} { return ONE }\nproc List2array {lst} { return TWO }\nList2array x\n";
        let analysis = analyse(src);
        // Cursor on the SHADOWED (first) declaration (line 0, col 6).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "winning decl missing: {refs:?}");
        assert!(lines.contains(&2), "call site missing: {refs:?}");
    }

    #[test]
    fn references_include_the_renames_own_old_word() {
        // TP — `rename OLD NEW`'s own
        // `OLD` word is a genuine reference to the proc it names
        // (tclsh9.0/8.6-verified: `rename` requires `OLD` to exist,
        // "can't rename ...: command doesn't exist" otherwise) — the real
        // corpus shape is a tcltest `-setup`/`-body`/`-cleanup` idiom
        // (georgtree_tclopt test/arbitaryTest.tcl:46/113's `proc gaussfunc`
        // / `rename gaussfunc ""`). Go-to-definition/hover resolve this token
        // through their own cursor-token walk; if `references` missed it, a
        // rename built on the same list would leave the occurrence pointing at
        // a now-nonexistent command — a passing tcltest then crashes with
        // "can't delete ...: command doesn't exist" purely from applying the
        // LSP's own rename edit.
        let src = "proc helperFunc {x} { return [expr {$x * 2}] }\nhelperFunc 21\nrename helperFunc \"\"\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {refs:?}");
        assert!(lines.contains(&1), "call site missing: {refs:?}");
        assert!(
            lines.contains(&2),
            "rename's own OLD word missing: {refs:?}"
        );
    }

    #[test]
    fn references_do_not_include_the_renames_new_word() {
        // FP guard — `rename OLD NEW`'s `NEW` word is not itself a
        // reference to `OLD`'s proc; only `OLD` is. A bare later call to
        // `NEW` reaches the target through the rename *link* (a
        // cross-document concern — see `workspace_index.rs`'s
        // `rename_new_name_call_site_references_the_old_command`), not
        // through this same-document text-reference scan.
        let src = "proc helperFunc {x} { return [expr {$x * 2}] }\nrename helperFunc renamedFunc\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(lines, vec![0, 1], "{refs:?}");
    }

    #[test]
    fn references_include_unbraced_if_body_bareword_call() {
        // TP — `if {$cond} mymod::foo` (an unbraced if-then body — a single,
        // statically-known bareword, valid Tcl and used ~50 times in the
        // nico-robert_ticklecharts corpus this way) is invisible to
        // `command_invocations` whenever `analyse_body` recurses only a braced
        // (`Str`-kind) body, and `references` from the
        // declaration then misses it — go-to-definition and hover
        // still found it (they resolve independently off the cursor
        // token), producing a dangerous asymmetry: a `rename` built on
        // this same list would silently miss rewriting the call site.
        let src = "proc foo {} { return 1 }\nif {1} foo\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {refs:?}");
        assert!(
            lines.contains(&1),
            "unbraced if-body call site missing: {refs:?}"
        );
    }

    #[test]
    fn references_include_unbraced_uplevel_body_bareword_call() {
        // TP — same root cause, the finding's other confirmed shape:
        // `uplevel 1 mymod::qux` (unbraced). `handle_uplevel_command`
        // itself only handles a braced body and otherwise falls through
        // to the same generic `ArgRole::Body` dispatch this fix covers.
        let src = "proc qux {} { return 1 }\nproc caller {} { uplevel 1 qux }\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {refs:?}");
        assert!(
            lines.contains(&1),
            "unbraced uplevel-body call site missing: {refs:?}"
        );
    }

    #[test]
    fn references_resolve_a_constant_var_body_through_its_real_value_not_its_literal_text() {
        // Was an FP guard against treating `$cb` as a static call to a
        // command literally *named* `$cb` — that concern still holds (this
        // test's own name reflects it), but real tclsh9.0/8.6-verified
        // behaviour is that `if {1} $cb` (a bare-`$var` `if`-body, evaluated
        // as a script exactly like `eval`/`uplevel`'s bodies) genuinely
        // calls `foo` when `$cb` holds that constant value — printing
        // "CALLED" for a `proc foo {} { puts CALLED }`. The analyser wires up
        // exactly this dispatch (`dispatch_one_body_argument`'s
        // `TokenType::Var` branch, generic across every `ArgRole::Body`
        // argument, not just `eval`/`uplevel`'s), so `foo`'s own reference
        // set correctly grows to include this call site — the failure mode
        // this test now guards is a *literal* `$cb`-named command
        // reference ever appearing, which never happens (there is no such
        // command in `all_procs` to resolve to).
        let src = "proc foo {} { return 1 }\nset cb foo\nif {1} $cb\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {refs:?}");
        assert!(
            lines.contains(&2),
            "`if {{1}} $cb` really does dispatch to `foo` (tclsh9.0/8.6-verified) \
             and must be found: {refs:?}"
        );
    }

    #[test]
    fn references_exclude_decl_when_flag_false() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let with_decl = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            2,
            &analysis,
            true,
        );
        let without_decl = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            2,
            &analysis,
            false,
        );
        assert!(with_decl.len() > without_decl.len());
    }

    #[test]
    fn references_to_unknown_word_empty() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(
            references(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                0,
                6,
                &analysis,
                true
            )
            .is_empty()
        );
    }

    #[test]
    fn ensemble_subcommand_references_include_decl_and_both_call_sites() {
        // TP — references on an ensemble subcommand call
        // site must return the target proc's declaration plus every call
        // site — proves the automatic pickup via
        // `proc_reference_spans`/`invocation_references_named`'s
        // `resolved_qualified_name` matching, not just a single hardcoded
        // case.
        let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e foo bar]\nputs [e foo baz]\n";
        let analysis = analyse(src);
        // Cursor on "foo" in the first call site (0-based line 7, col 8).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            7,
            8,
            &analysis,
            true,
        );
        // decl + the `-map`'s own target-text reference (line 2, pre-existing
        // — needed so renaming the proc also updates the map entry) + both
        // nested-`[...]` call sites.
        assert_eq!(
            refs.len(),
            4,
            "expected decl + map entry + 2 call sites: {refs:?}"
        );
        assert_eq!(refs[0].start_line, 5, "declaration: {refs:?}");
        assert!(
            refs.iter().any(|r| r.start_line == 2),
            "expected the -map target-text reference: {refs:?}",
        );
        assert!(
            refs.iter().any(|r| r.start_line == 7) && refs.iter().any(|r| r.start_line == 8),
            "expected both call sites: {refs:?}",
        );
    }

    /// The analyser's own dispatch recording resolves the
    /// subcommand the way the ensemble does, so an **abbreviated** call site
    /// is a reference too. Oracle (tclsh 8.6.16 / 9.0.4): with `-map {foo
    /// ::e::Foo}` and the default `-prefixes 1`, `e fo` returns `foo: bar`.
    ///
    /// The abbreviated site sits inside a proc body so the deferred replay
    /// path (`flush_pending_ensemble_subcommand_invocations`) resolves it,
    /// not only the direct one.
    #[test]
    fn ensemble_abbreviated_dispatch_is_a_reference_to_the_target() {
        let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\nproc caller {} {\n    return [e fo bar]\n}\n";
        let analysis = analyse(src);
        // Cursor on the declaration of ::e::Foo (0-based line 5).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            5,
            11,
            &analysis,
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line == 7),
            "expected the abbreviated call site inside the proc body: {refs:?}",
        );
    }

    /// The `-prefixes 0` counterpart: prefix matching is off, so the
    /// abbreviated word is a plain unknown subcommand and must NOT be
    /// recorded as a dispatch reference. Oracle: `g fo` errors
    /// `unknown subcommand "fo": must be foo`.
    #[test]
    fn prefixless_ensemble_abbreviation_is_not_a_reference() {
        let src = "namespace eval ::g {\n    namespace ensemble create -map {\n        foo ::g::Foo\n    } -prefixes 0\n}\nproc ::g::Foo {args} { return G }\nproc caller {} {\n    return [g fo bar]\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            5,
            11,
            &analysis,
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 7),
            "a -prefixes 0 ensemble must not resolve the abbreviation: {refs:?}",
        );
    }

    /// The ensemble is created by
    /// `namespace ensemble create -map` inside a proc declared with a
    /// fully-qualified name at top level, with no enclosing `namespace eval`,
    /// so it homes to `::app::widget`. Both reference directions must reach
    /// the dispatch call site: from the mapped target's declaration, and from
    /// the call site itself.
    ///
    /// Oracle (tclsh 8.6.16 and 9.0.4, identical): the script prints `shown`,
    /// so `::app::widget show` really does dispatch to `::app::widget::Show`.
    ///
    /// Go-to-definition already answered this (it resolves on demand against
    /// the finished analysis); references enumerate recorded invocations, and
    /// the invocation was missing. The analyser fix is what supplies it — this
    /// pins that the reference provider consumes it from both cursor
    /// positions and returns the *same* answer for each.
    #[test]
    fn ensemble_subcommand_references_reach_the_call_site_through_a_qualified_proc_923_idx85() {
        let src = "namespace eval ::app::widget {\n    variable state 0\n}\n\
                   proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map {\n        \
                   show      ::app::widget::Show\n    }\n}\n\
                   proc ::app::widget::Show {} { puts \"shown\" }\n\
                   ::app::widget::Setup\n\
                   puts [::app::widget show]\n";
        let analysis = analyse(src);
        // Line 8 is `proc ::app::widget::Show …`; `Show` starts at column 20.
        let from_decl = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            8,
            21,
            &analysis,
            true,
        );
        // Line 10 is `puts [::app::widget show]`; `show` starts at column 20.
        let from_call = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            10,
            21,
            &analysis,
            true,
        );
        let lines = |refs: &[LspRange]| -> Vec<u32> {
            let mut l: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
            l.sort_unstable();
            l
        };
        assert!(
            lines(&from_decl).contains(&10),
            "the `::app::widget show` dispatch is missing from the declaration's \
             reference set: {from_decl:?}",
        );
        assert!(
            lines(&from_decl).contains(&8),
            "the declaration itself is missing: {from_decl:?}",
        );
        assert_eq!(
            lines(&from_call),
            lines(&from_decl),
            "both query directions must agree: {from_call:?} vs {from_decl:?}",
        );
    }

    /// TN for the above — a `-map` built by `[list …]` is not a literal
    /// mapping, so no `show -> ::app::widget::Show` fact exists and the
    /// dispatch site must stay unattributed rather than guessed at.
    #[test]
    fn a_dynamic_ensemble_map_yields_no_call_site_reference_923_idx85() {
        let src = "proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map [list show ::app::widget::Show]\n}\n\
                   proc ::app::widget::Show {} {}\n\
                   ::app::widget::Setup\n\
                   ::app::widget show\n";
        let analysis = analyse(src);
        // Cursor on `Show` in its declaration (line 3, column 20).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            3,
            21,
            &analysis,
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 5),
            "a dynamic -map must not manufacture a navigation edge: {refs:?}",
        );
    }

    #[test]
    fn method_references_include_next_dispatch() {
        // `Sub::greet`'s body invokes the super via `next`; that dispatch is a
        // polymorphic reference to `greet` and must appear among its
        // references.
        let src = "oo::class create Base {\n    method greet {} {}\n}\noo::class create Sub {\n    superclass Base\n    method greet {} { next }\n}\n";
        let analysis = analyse(src);
        // Cursor on `greet` in Sub's declaration (line 5).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            5,
            13,
            &analysis,
            true,
        );
        // The `next` token sits on line 5, past the method's own `greet` name
        // (col 11) — inside the `{ next }` body.
        assert!(
            refs.iter()
                .any(|r| r.start_line == 5 && r.start_character > 15),
            "expected the `next` dispatch among refs: {refs:?}",
        );
    }

    // Constructor / destructor next-chain references.

    #[test]
    fn constructor_next_chain_reference_from_direct_subclass() {
        // `Sub`'s constructor calls plain `next`, chaining to `Base`'s —
        // cursor on `Base`'s own `constructor` keyword must surface that
        // chain as a reference.
        let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { next }\n}\n";
        let analysis = analyse(src);
        // `constructor` keyword on line 1, col 4.
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            6,
            &analysis,
            true,
        );
        // decl (line 1) + the `next` call site (line 5).
        assert_eq!(refs.len(), 2, "{refs:?}");
        assert!(refs.iter().any(|r| r.start_line == 5), "{refs:?}");
    }

    #[test]
    fn constructor_next_chain_reference_skips_non_overriding_subclass() {
        // `Sub` inherits `Base`'s constructor outright (declares none of its
        // own) — nothing to chain, so `Base`'s constructor stays unreferenced.
        let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            6,
            &analysis,
            false,
        );
        assert!(refs.is_empty(), "{refs:?}");
    }

    #[test]
    fn constructor_next_chain_reference_not_counted_without_next() {
        // `Sub` declares its own constructor but never calls `next` — a
        // legitimate full override, not a chain.
        let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { set x 1 }\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            6,
            &analysis,
            false,
        );
        assert!(refs.is_empty(), "{refs:?}");
    }

    #[test]
    fn constructor_next_chain_reference_skips_ancestor_with_no_own_constructor() {
        // `Mid` declares no constructor of its own; `Sub`'s `next` must
        // still reach `Base` (the actual MRO-effective provider), not `Mid`.
        let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Mid {\n    superclass Base\n}\noo::class create Sub {\n    superclass Mid\n    constructor {} { next }\n}\n";
        let analysis = analyse(src);
        // `Base`'s constructor (line 1) picks up the chain.
        let base_refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            6,
            &analysis,
            false,
        );
        assert_eq!(base_refs.len(), 1, "{base_refs:?}");
        assert_eq!(base_refs[0].start_line, 8, "{base_refs:?}");
    }

    #[test]
    fn constructor_next_chain_reference_via_nextto_explicit_target() {
        // `nextto Grandparent` jumps past `Base` even though `Base` also has
        // an effective constructor — only `Grandparent` picks up a reference.
        let src = "oo::class create Grandparent {\n    constructor {} { }\n}\noo::class create Base {\n    superclass Grandparent\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { nextto Grandparent }\n}\n";
        let analysis = analyse(src);
        let grandparent_refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            6,
            &analysis,
            false,
        );
        assert_eq!(grandparent_refs.len(), 1, "{grandparent_refs:?}");
        let base_refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            4,
            6,
            &analysis,
            false,
        );
        assert!(base_refs.is_empty(), "{base_refs:?}");
    }

    #[test]
    fn constructor_next_chain_reference_via_braced_nextto_target() {
        // `nextto {Grandparent}` (a
        // braced target, functionally identical to the bare form) must
        // resolve exactly like `constructor_next_chain_reference_via_nextto_explicit_target`'s
        // bare `nextto Grandparent` — the decoded word, not the raw
        // delimited span, is what gets resolved against the class map.
        let src = "oo::class create Grandparent {\n    constructor {} { }\n}\noo::class create Base {\n    superclass Grandparent\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { nextto {Grandparent} }\n}\n";
        let analysis = analyse(src);
        let grandparent_refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            6,
            &analysis,
            false,
        );
        assert_eq!(grandparent_refs.len(), 1, "{grandparent_refs:?}");
        let base_refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            4,
            6,
            &analysis,
            false,
        );
        assert!(base_refs.is_empty(), "{base_refs:?}");
    }

    #[test]
    fn destructor_next_chain_reference_from_direct_subclass() {
        let src = "oo::class create Base {\n    destructor { }\n}\noo::class create Sub {\n    superclass Base\n    destructor { next }\n}\n";
        let analysis = analyse(src);
        // `destructor` keyword on line 1, col 4.
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            5,
            &analysis,
            true,
        );
        assert_eq!(refs.len(), 2, "{refs:?}");
        assert!(refs.iter().any(|r| r.start_line == 5), "{refs:?}");
    }

    #[test]
    fn constructor_shadowed_by_a_later_redeclaration_has_no_references() {
        // `oo::configurable` allows several constructors; only the last is
        // ever effective. A cursor on the shadowed first one resolves to
        // nothing (it has no reference story worth surfacing).
        let src = "oo::class create C {\n    constructor {} { }\n    constructor {} { }\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            6,
            &analysis,
            true,
        );
        assert!(refs.is_empty(), "{refs:?}");
    }

    #[test]
    fn constructor_keyword_resolves_its_own_next_chain_even_with_a_same_named_method() {
        // A class can also declare a
        // `method` literally named `constructor` — an independent, ordinary
        // member sharing a name with the special keyword form. A cursor on
        // the special `constructor` keyword must still resolve its own
        // next-chain story, not fall through to the unrelated same-named
        // method's (ordinary) references.
        let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { next }\n    method constructor {} { }\n}\n";
        let analysis = analyse(src);
        // `Base`'s `constructor` keyword, line 1.
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            6,
            &analysis,
            false,
        );
        assert_eq!(
            refs.len(),
            1,
            "must resolve the next-chain, not the unrelated same-named method: {refs:?}"
        );
        assert_eq!(refs[0].start_line, 5, "{refs:?}");
    }

    #[test]
    fn references_to_var_includes_definition_and_uses() {
        let src = "set x 1\nputs $x\nputs $x\n";
        let analysis = analyse(src);
        // Cursor on `$x` first reference.
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            7,
            &analysis,
            true,
        );
        // The analyser may or may not record the literal `$x`
        // as a reference depending on lowering; at minimum the
        // declaration should land in the result list.
        assert!(!refs.is_empty(), "{refs:?}");
        assert!(refs.iter().any(|r| r.start_line == 0));
    }

    #[test]
    fn references_from_proc_param_bareword_declaration_include_every_use() {
        // TP — a cursor on a proc parameter's own bareword name (not a
        // `$`-prefixed read) must return the same reference set a query from
        // any `$name` read resolves.
        let src = "proc greet {name} { return $name }\ngreet hi\n";
        let analysis = analyse(src);
        // Cursor on `name` inside the parameter list (col 12-16).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            13,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {refs:?}");
        assert!(
            lines.contains(&0) && refs.len() >= 2,
            "read missing: {refs:?}"
        );
    }

    #[test]
    fn references_from_catch_resultvar_bareword_include_the_original_declaration() {
        // TP — the finding's other confirmed shape: a `catch script name`
        // result-var reuses an existing variable; a cursor placed on its
        // own bareword token must still surface the full reference set,
        // including the original declaration.
        let src =
            "proc resolveSwitch {name def} {\n    catch {error boom} name\n    return $name\n}\n";
        let analysis = analyse(src);
        // Cursor on the catch result-var `name` (line 1, col 23-27).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            24,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "original decl missing: {refs:?}");
        assert!(
            lines.contains(&2),
            "the later `$name` read missing: {refs:?}"
        );
    }

    // read/write distinction

    #[test]
    fn document_highlights_var_records_write_at_definition() {
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        // Cursor inside `$x`.
        let highlights = document_highlights(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            7,
            &analysis,
        );
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
                link_target_span: None,
            },
        );
        let mut a = Result::default();
        a.global_scope = scope;
        // Source matches the spans we injected so
        // line/character translation works.
        let src = "set x 1\nputs $x\n";
        let highlights = document_highlights(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            6,
            &a,
        );
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
        let highlights = document_highlights(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis,
        );
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
        assert!(
            document_highlights(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                0,
                6,
                &analysis
            )
            .is_empty()
        );
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
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            8,
            &analysis,
            true,
        );
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
        let highlights = document_highlights(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            6,
            &analysis,
        );
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
    fn idx63_two_block_class_my_dispatch_already_fixed_by_idx52() {
        // Go-to-definition and find-references must both answer for a `my
        // methodName` call when the class is created via `oo::class create`
        // with no body and every method (including the call site itself) is
        // added via a *separate*, later `oo::define ClassName { ... }` block —
        // the real corpus's `ticklecharts::chart` shape.  The load-bearing
        // machinery is `class_body_spans` / `enclosing_class_at`, pinned here
        // from the `my`-dispatch direction.
        let src = "oo::class create foo::widget {\n    variable _x\n    constructor {} { set _x 0 }\n}\noo::define foo::widget {\n    method bar {} { return \"bar-value\" }\n    method baz {} { return [my bar] }\n}\nputs [[foo::widget new] baz]\n";
        let analysis = analyse(src);
        // `definition` at the `my bar` call site (line 6, col 31).
        let locs = crate::definition::definition(src, 6, 31, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 5,
            "must resolve to method bar's declaration"
        );
        // `references` from `bar`'s declaration (line 5, col 11).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            5,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&5), "decl missing: {refs:?}");
        assert!(
            lines.contains(&6),
            "the my bar call site must be reachable from the declaration: {refs:?}"
        );
    }

    #[test]
    fn references_for_method_includes_decl_and_call_sites() {
        // Intra-class dispatch uses `my <method>` (the dispatch form tclsh
        // accepts; a bare `greet` head errors with "invalid command name", so it
        // is not a reference).
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `greet` declaration (line 1, col 11).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analysis,
            true,
        );
        assert!(refs.len() >= 3, "expected ≥3 refs; got {refs:?}");
    }

    #[test]
    fn references_for_method_includes_one_hop_stored_callback_prefixes() {
        let src = "package require Tk\noo::class create C {\n    method tick {} {}\n    method setup {} {\n        set external [list [self] tick]\n        bind .w <Button-1> $external\n        set internal [namespace code [list my tick]]\n        after 0 $internal\n    }\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            2,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&2), "declaration missing: {refs:?}");
        assert!(
            lines.contains(&4),
            "stored [self] callback missing: {refs:?}"
        );
        assert!(
            lines.contains(&6),
            "stored namespace-code callback missing: {refs:?}"
        );
    }

    #[test]
    fn stored_callback_prefixes_abstain_on_reassignment_and_dynamic_methods() {
        let src = "oo::class create C {\n    method tick {} {}\n    method setup {method} {\n        set cb [list my tick]\n        lappend cb suffix\n        set cb [list my $method]\n        bind .w <Button-1> $cb\n    }\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            1,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(
            lines,
            vec![1],
            "ambiguous stored callback must abstain: {refs:?}"
        );
    }

    #[test]
    fn stored_callback_prefixes_count_nested_same_frame_reassignment() {
        let src = "oo::class create C {\n    method tick {} {}\n    method other {} {}\n    method setup {} {\n        set cb [list my tick]\n        if 1 {\n            set cb [list my other]\n            after 0 $cb\n        }\n    }\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(
            lines,
            vec![1],
            "a nested reassignment makes the stored callback ambiguous: {refs:?}"
        );
    }

    #[test]
    fn stored_callback_prefixes_abstain_on_non_set_varwrite_reassignment() {
        let src = "oo::class create C {\n    method tick {} {}\n    method setup {} {\n        set cb [list my tick]\n        lappend cb suffix\n        after 0 $cb\n    }\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(
            lines,
            vec![1],
            "non-Set VarWrite reassignment must abstain: {refs:?}"
        );
    }

    #[test]
    fn stored_callback_prefixes_abstain_on_qualified_and_aliased_variables() {
        let src = "oo::class create C {\n    method tick {} {}\n    method setup {} {\n        set ::cb [list my tick]\n        bind .w <Button-1> $::cb\n        global local\n        set local [list my tick]\n        after 0 $local\n    }\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(
            lines,
            vec![1],
            "qualified/aliased storage must abstain: {refs:?}"
        );
    }

    #[test]
    fn stored_callback_prefix_cursor_resolves_to_method_declaration() {
        let src = "oo::class create C {\n    method tick {} {}\n    method setup {} {\n        set cb [list my tick]\n        after 0 $cb\n    }\n}\n";
        let analysis = analyse(src);
        let col = u32::try_from(src.lines().nth(3).unwrap().find("tick").unwrap()).unwrap();
        let locations = crate::definition::definition(src, 3, col, &analysis);
        assert_eq!(
            locations.len(),
            1,
            "stored callback cursor must resolve: {locations:?}"
        );
        assert_eq!(
            locations[0].start_line, 1,
            "wrong stored callback target: {locations:?}"
        );
    }

    #[test]
    fn references_do_not_resolve_an_unclosed_braced_instance_head() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset x [Dog new]\n${x bark\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analysis,
            true,
        );
        assert_eq!(
            refs.len(),
            1,
            "only the declaration may resolve; malformed `${{x` is not `x`: {refs:?}"
        );
        assert_eq!(refs[0].start_line, 1);
        assert_eq!(strip_var_decoration("${x}"), Some("x"));
        assert_eq!(strip_var_decoration("$x"), Some("x"));
        assert_eq!(strip_var_decoration("$arr(idx)"), Some("arr(idx)"));
        assert_eq!(strip_var_decoration("$::a:::b"), Some("::a:::b"));
        assert_eq!(strip_var_decoration("$foo:::"), Some("foo:::"));
        assert_eq!(strip_var_decoration("${x"), Some("{x"));
        assert_eq!(strip_var_decoration("x"), None);
    }

    #[test]
    fn references_for_method_reach_a_my_dispatch_call_inside_a_switch_arm() {
        // A `my methodName` call written inside a `switch` arm body is a
        // genuine, statically-known call site (tclsh9.0/8.6-verified) —
        // the real corpus shape (`ticklecharts::chart`'s `Add` dispatcher:
        // `switch ... { barSeries { my AddBarSeries {*}$args } ... }`).
        // `scan_my_method_region`'s `[...]`-substitution recursion alone never
        // reaches a switch arm's braced body (it isn't a command
        // substitution), which would leave this invisible to find-references even
        // though go-to-definition (an independent cursor-token walk)
        // already resolved it.
        let src = "oo::class create widget {\n    method bar {} { return \"bar-value\" }\n    method dispatch {args} {\n        switch -exact -- [lindex $args 0] {\n            bar { my bar {*}[lrange $args 1 end] }\n        }\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `bar` declaration (line 1, col 11).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(
            lines.contains(&4),
            "the my bar call site inside the switch arm is missing: {refs:?}"
        );
    }

    #[test]
    fn references_for_method_excludes_decl_when_requested() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analysis,
            false,
        );
        // Only the two call sites — the declaration is
        // excluded when include_declaration=false.
        assert_eq!(refs.len(), 2, "{refs:?}");
    }

    #[test]
    fn document_highlights_for_method_marks_decl_and_calls_text() {
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        let h = document_highlights(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analysis,
        );
        // Methods carry no Write/Read distinction — declaration + both call
        // sites are all Text (only variables are Write/Read).
        let writes = h.iter().filter(|(_, k)| *k == HighlightKind::Write).count();
        let texts = h.iter().filter(|(_, k)| *k == HighlightKind::Text).count();
        assert_eq!(writes, 0, "{h:?}");
        assert_eq!(texts, 3, "{h:?}");
    }

    #[test]
    fn references_from_decl_reach_my_dispatch_when_class_extended_via_separate_oo_define() {
        // `Gadget` is created via `oo::class create` with no body; every method
        // (including the `my Helper` call site) is added via a *separate*,
        // later `oo::define Gadget { ... }` block — the real corpus shape
        // (`ticklecharts::chart`). References from the `Helper` declaration
        // must reach the `my Helper` call site living in that separate
        // block, not silently return nothing.
        let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `Helper` declaration (line 4, col 11).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            4,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&4), "decl missing: {refs:?}");
        assert!(
            lines.contains(&5),
            "the my Helper call site inside the separate oo::define block is missing: {refs:?}"
        );
    }

    #[test]
    fn references_from_decl_reach_a_self_bracket_dispatch_call_site() {
        // `[self] m` is TclOO's own same-object dispatch
        // idiom — reaches the enclosing class exactly like `my m`, but
        // through a bracketed command substitution. References from the
        // declaration must reach it, not silently return only the
        // declaration itself.
        let src = "oo::class create C {\n    method animTick {} { return 1 }\n    method anim {} { [self] animTick }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `animTick` declaration (line 1, col 11).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(
            lines.contains(&2),
            "the [self] animTick call site is missing: {refs:?}"
        );
    }

    #[test]
    fn references_from_decl_reach_a_list_built_self_callback() {
        // `bind` receives a deferred script built as a Tcl list.
        // `[self]` is substituted while the method frame is live, leaving an
        // object-command prefix that later dispatches `animTick` externally.
        let src = "package require Tk\noo::class create C {\n    method animTick {} { return 1 }\n    method anim {wl} {\n        bind $wl <ButtonPress-1> [list [self] animTick %x %y]\n    }\n}\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            2,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&2), "decl missing: {refs:?}");
        assert!(
            lines.contains(&4),
            "the list-built bind callback is missing: {refs:?}"
        );
    }

    #[test]
    fn references_reach_a_list_built_self_object_callback() {
        let src = "package require Tk\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> [list [self object] tick]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            2,
            11,
            &analyse(src),
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line == 4),
            "the `[self object]` callback is missing: {refs:?}"
        );
    }

    #[test]
    fn references_reach_a_list_built_self_after_callback() {
        // A real corpus shape: Pave and Zesty both schedule methods this
        // way. `after` exposes its script through the same registry Body role
        // as `bind`, so no scheduler-specific branch belongs in this scan.
        let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line == 3),
            "the list-built after callback is missing: {refs:?}"
        );
    }

    /// A tcllib-shaped fixture read from the real files rather than a
    /// hand-written imitation.
    ///
    /// Gated on corpus presence with a loud skip, matching
    /// `rust/tcl-lsp-db/tests/compiler_check_corpus.rs`.
    #[test]
    fn the_real_tcllib_callback_shapes_resolve_to_their_methods() {
        let corpus = std::env::var_os("TCLLIB_2_0_DIR").map_or_else(
            || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/tcllib-2.0"),
            std::path::PathBuf::from,
        );
        // (file, method declared in the file, a method named by a callback
        // prefix inside it). `cat.tcl` is the `after … [namespace code [list
        // my Post $c]]` shape.  `httpd.tcl` carries the `socket -server
        // [namespace code [list my connect]]` CommandPrefix shape, but it
        // defines its classes with `::clay::define`, which the analyser records
        // no class for at all — `all_classes` is empty for that file, so there
        // is no method to navigate from and the shape is unreachable for
        // reasons that have nothing to do with callback prefixes, so it is not
        // asserted here.
        let (relative, method) = ("modules/virtchannel_base/cat.tcl", "Post");
        {
            let path = corpus.join(relative);
            let Ok(src) = std::fs::read_to_string(&path) else {
                eprintln!(
                    "skip: {} not present (fetch tcllib 2.0 under tmp/, or set TCLLIB_2_0_DIR)",
                    path.display()
                );
                return;
            };
            let analysis = analyse(&src);
            let declaration = src
                .find(&format!("method {method} "))
                .unwrap_or_else(|| panic!("{relative} declares `method {method}`"));
            let line = u32::try_from(src[..declaration].lines().count() - 1).expect("line fits");
            let column = u32::try_from(
                src[..declaration]
                    .rsplit('\n')
                    .next()
                    .unwrap_or_default()
                    .len()
                    + "method ".len(),
            )
            .expect("column fits");
            let refs = references(
                &src,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                line,
                column,
                &analysis,
                true,
            );
            assert!(
                refs.len() > 1,
                "{relative}: `{method}` must reach its callback prefix as well as its \
                 declaration, got {refs:?}"
            );
        }
    }

    /// A `{*}`-expanded callback word must abstain.
    ///
    /// `{*}` splices the word's value into the argument list, so the registry's
    /// role indices no longer describe where anything landed. Reading the word
    /// as if it were the callback slot is not merely unjustified, it invents
    /// references: after expansion `lsort -command {*}[list [self] compare]
    /// $items` runs the bare *object command* as the comparator and passes
    /// `compare` as a separate argument, so `compare` is not dispatched at all
    /// — and Rename would have rewritten it.
    #[test]
    fn an_expanded_callback_word_invents_no_reference() {
        for src in [
            // The comparator is the object command; `compare` is an argument.
            "oo::class create C {\n    method compare {a b} { return 0 }\n    method sort {items} {\n        lsort -command {*}[list [self] compare] $items\n    }\n}\n",
            // Same rule for a Body slot, where the arity happens to work out.
            "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle {*}[list [self] tick]\n    }\n}\n",
            // Inside a `WRAPS_COMMAND_PREFIX` wrapper:
            // `namespace code` then has two arguments and errors rather than
            // dispatching anything, so there is nothing to reference.
            "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [namespace code {*}[list my tick]]\n    }\n}\n",
            // And inside the builder itself: the receiver is no longer word 1.
            "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list {*}$prefix tick]\n    }\n}\n",
        ] {
            let refs = references(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                1,
                11,
                &analyse(src),
                true,
            );
            assert!(
                !refs.iter().any(|r| r.start_line == 3),
                "an expanded word is not a callback slot: {src:?} gave {refs:?}"
            );
        }
    }

    /// The gate is about expansion, not about the shape around it: the same
    /// prefix written as a sole substitution still resolves.
    #[test]
    fn references_reach_a_list_built_self_command_prefix() {
        // CommandPrefix is distinct from Body: the registry says this word is
        // a partial command and the consumer appends its own arguments. The
        // object-method reference is nevertheless the same external dispatch.
        let src = "oo::class create C {\n    method compare {a b} { return 0 }\n    method sort {items} {\n        lsort -command [list [self] compare] $items\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line == 3),
            "the list-built CommandPrefix callback is missing: {refs:?}"
        );
    }

    #[test]
    fn references_reach_namespace_wrapped_my_callback_even_when_private() {
        // tcllib's virtchannel_base shape: the registry declares
        // `namespace code` as a WRAPS_COMMAND_PREFIX and `list` as a
        // BUILDS_COMMAND_PREFIX.  `my` keeps private current-object dispatch,
        // unlike a captured `[self]` object command.
        let src = "oo::class create C {\n    method read {} { return 1 }\n    unexport read\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my read]]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line == 4),
            "the wrapped private `my read` callback is missing: {refs:?}"
        );
    }

    #[test]
    fn references_reach_namespace_wrapped_self_callback_only_when_public() {
        let src = "oo::class create C {\n    method changed {} { return 1 }\n    method wire {} {\n        trace add variable v write [namespace code [list [self] changed]]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line == 3),
            "the wrapped public `[self] changed` callback is missing: {refs:?}"
        );
    }

    #[test]
    fn a_mixin_branch_decides_its_own_providers_visibility() {
        // A mixin that inherits the member from its
        // own superclass and unexports the name empties *its* branch only —
        // the spine still answers.  tclsh 8.6.16 / 9.0.4 both run `A`'s body:
        //   oo::class create MChild { superclass MBase } ; unexport m
        //   oo::class create D { superclass A ; mixin MChild }
        //   [D new] m  ->  A-m   (not MBase-m)
        let src = "oo::class create A {\n    method m {} { return 1 }\n}\noo::class create MBase {\n    method m {} { return 2 }\n}\noo::class create MChild {\n    superclass MBase\n    unexport m\n}\noo::class create D {\n    superclass A\n    mixin MChild\n    method wire {} {\n        after idle [list [self] m]\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let cursor = u32::try_from(src.find("] m]").unwrap() + 2).unwrap();
        assert_eq!(
            list_built_self_method_target_at_cursor(src, dialect, &analysis, "m", cursor),
            Some(("::A".to_owned(), false)),
            "the suppressed mixin branch must not provide the capture"
        );
        assert!(
            !references(src, dialect, 4, 11, &analysis, true)
                .iter()
                .any(|range| range.start_line == 14),
            "`MBase::m` must not collect a capture its branch cannot dispatch"
        );
        assert!(
            references(src, dialect, 1, 11, &analysis, true)
                .iter()
                .any(|range| range.start_line == 14),
            "`A::m` — the provider the receiver actually reaches — must collect it"
        );
    }

    #[test]
    fn a_live_mixin_branch_still_outranks_the_spine() {
        // The control for the case above: with the branch left public, the
        // mixin's inherited `MBase::m` is what `[D new] m` enters (tclsh
        // 8.6.16 / 9.0.4 -> MBase-m), so the capture belongs to *its* family.
        let src = "oo::class create A {\n    method m {} { return 1 }\n}\noo::class create MBase {\n    method m {} { return 2 }\n}\noo::class create MPub {\n    superclass MBase\n}\noo::class create D {\n    superclass A\n    mixin MPub\n    method wire {} {\n        after idle [list [self] m]\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let cursor = u32::try_from(src.find("] m]").unwrap() + 2).unwrap();
        assert_eq!(
            list_built_self_method_target_at_cursor(src, dialect, &analysis, "m", cursor),
            Some(("::MBase".to_owned(), false)),
            "a live mixin branch provides the capture ahead of the spine"
        );
    }

    #[test]
    fn a_mixin_branch_can_revive_its_own_bases_unexported_member() {
        // The mirror direction inside a branch: `NBase` unexports its own `n`,
        // the mixin `NChild` exports the inherited name, and tclsh 8.6.16 /
        // 9.0.4 run `NBase`'s body for `[D new] n` — ahead of the spine's
        // public `A2::n`.
        let src = "oo::class create A2 {\n    method n {} { return 1 }\n}\noo::class create NBase {\n    method n {} { return 2 }\n    unexport n\n}\noo::class create NChild {\n    superclass NBase\n    export n\n}\noo::class create D {\n    superclass A2\n    mixin NChild\n    method wire {} {\n        after idle [list [self] n]\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let cursor = u32::try_from(src.find("] n]").unwrap() + 2).unwrap();
        assert_eq!(
            list_built_self_method_target_at_cursor(src, dialect, &analysis, "n", cursor),
            Some(("::NBase".to_owned(), false)),
            "the branch's own export revives its base's body"
        );
    }

    #[test]
    fn a_mixin_declared_on_a_superclass_governs_its_branch_too() {
        // Branch roots are collected transitively, so a `mixin` written on a
        // superclass behaves exactly as one written on the receiver: tclsh
        // 8.6.16 / 9.0.4 answer `[D new] m` with `A-m`.
        let src = "oo::class create A {\n    method m {} { return 1 }\n}\noo::class create MBase {\n    method m {} { return 2 }\n}\noo::class create MChild {\n    superclass MBase\n    unexport m\n}\noo::class create S {\n    superclass A\n    mixin MChild\n}\noo::class create D {\n    superclass S\n    method wire {} {\n        after idle [list [self] m]\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let cursor = u32::try_from(src.find("] m]").unwrap() + 2).unwrap();
        assert_eq!(
            list_built_self_method_target_at_cursor(src, dialect, &analysis, "m", cursor),
            Some(("::A".to_owned(), false)),
            "a superclass-declared mixin's suppression empties that branch only"
        );
    }

    #[test]
    fn inert_list_built_self_value_is_not_a_method_reference() {
        let src = "oo::class create C {\n    method tick {} { return 1 }\n    method build {} {\n        set inert [list [self] tick]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 3),
            "an inert list value must not become a call site: {refs:?}"
        );
    }

    #[test]
    fn dynamic_list_built_self_method_remains_unresolved() {
        let src = "oo::class create C {\n    method tick {} { return 1 }\n    method build {methodName} {\n        after idle [list [self] $methodName]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 3),
            "a dynamic method word must remain an abstention: {refs:?}"
        );
    }

    #[test]
    fn compound_list_built_self_word_is_not_the_spelled_method() {
        // The list substitution produces an object command ending in `tick`,
        // then Tcl concatenates `Suffix` onto that word. The invoked method is
        // `tickSuffix`; rewriting the inner `tick` token would be incorrect.
        let src = "oo::class create C {\n    method tick {} { return 1 }\n    method build {} {\n        after idle [list [self] tick]Suffix\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 3),
            "a method fragment inside a compound word is not an exact reference: {refs:?}"
        );
    }

    #[test]
    fn deferred_braced_self_script_is_not_a_method_reference() {
        // The braces delay `[self]` itself until Tk runs the binding, after
        // the defining method frame has gone.  This script cannot resolve the
        // object and must not be credited as a method call.
        let src = "package require Tk\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> {[self] tick}\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            2,
            11,
            &analyse(src),
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 4),
            "a later, out-of-frame `self` must not become a call site: {refs:?}"
        );
    }

    #[test]
    fn list_built_self_callback_cannot_reach_an_unexported_method() {
        // A captured `[self]` result is an object command.  Its later call is
        // therefore external and cannot dispatch an unexported method.
        let src = "package require Tk\noo::class create C {\n    method tick {} { return 1 }\n    unexport tick\n    method wire {wl} {\n        bind $wl <Expose> [list [self] tick]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            2,
            11,
            &analyse(src),
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 5),
            "an unexported method is not callable through the captured object: {refs:?}"
        );
    }

    #[test]
    fn wrapped_my_callback_cursor_resolves_private_dispatch() {
        let src = "oo::class create C {\n    method tick {} { return 1 }\n    unexport tick\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my tick]]\n    }\n}\n";
        let analysis = analyse(src);
        let cursor = u32::try_from(src.rfind("tick]").unwrap()).unwrap();
        assert_eq!(
            list_built_self_method_target_at_cursor(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                &analysis,
                "tick",
                cursor,
            ),
            Some(("::C".to_owned(), false)),
        );
    }

    #[test]
    fn a_shadowing_list_command_does_not_build_a_callback_reference() {
        let src = "package require Tk\nproc list args { return not-a-command-prefix }\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> [list [self] tick]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            3,
            11,
            &analyse(src),
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 5),
            "a user command shadowing the registry builder must not inherit its semantics: {refs:?}"
        );
    }

    #[test]
    fn a_global_self_command_does_not_shadow_the_tcloo_method_helper() {
        let src = "package require Tk\nproc self args { return ::notTheCurrentObject }\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> [list [self] tick]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            3,
            11,
            &analyse(src),
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line == 5),
            "::oo::Helpers precedes global fallback on a method frame: {refs:?}"
        );
    }

    #[test]
    fn malformed_self_object_receiver_is_not_a_callback_reference() {
        let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [self object junk] tick]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 3),
            "a receiver call that errors cannot build a callback: {refs:?}"
        );
    }

    #[test]
    fn explicitly_global_self_is_not_the_tcloo_method_helper() {
        let src = "proc ::self {} { return ::notTheCurrentObject }\noo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [::self] tick]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            2,
            11,
            &analyse(src),
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 4),
            "an absolute global receiver must not inherit method-path semantics: {refs:?}"
        );
    }

    #[test]
    fn qualified_tcloo_self_helper_builds_the_same_callback() {
        let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [::oo::Helpers::self] tick]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line == 3),
            "the registry-declared qualified helper must remain valid: {refs:?}"
        );
    }

    #[test]
    fn callback_cursor_classifier_reaches_constructor_and_destructor_bodies() {
        let src = "oo::class create C {\n    method tick {} { return 1 }\n    constructor {} {\n        after idle [list [self] tick]\n    }\n    destructor {\n        after idle [list [self] tick]\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let first = src.find("] tick").unwrap() + 2;
        let second = src[first + 1..].find("] tick").unwrap() + first + 3;
        for cursor in [first, second] {
            assert_eq!(
                list_built_self_method_target_at_cursor(
                    src,
                    dialect,
                    &analysis,
                    "tick",
                    u32::try_from(cursor).unwrap(),
                ),
                Some(("::C".to_owned(), false)),
            );
        }
    }

    #[test]
    fn callback_cursor_classifier_rejects_ordinary_method_dispatches() {
        let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        my tick\n        [self] tick\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let my_cursor = u32::try_from(src.find("my tick").unwrap() + 3).unwrap();
        let self_cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
        assert!(
            list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", my_cursor,)
                .is_none()
        );
        assert!(
            list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", self_cursor,)
                .is_none()
        );
    }

    #[test]
    fn inherited_callback_abstains_without_effective_receiver_visibility() {
        let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    unexport tick\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 7),
            "provider visibility cannot be reused for the child receiver: {refs:?}"
        );
    }

    #[test]
    fn inherited_callback_joins_the_providers_reference_set() {
        // A captured `[self]` object command in an inheriting class
        // reaches the provider's exported implementation — tclsh 8.6.16 /
        // 9.0.4 both run `Base`'s body for `[list [Child new] tick]`.  Both
        // directions must agree on that, or rename would edit one and not the
        // other.
        let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
        assert_eq!(
            list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor),
            Some(("::Base".to_owned(), false)),
            "the inherited callback resolves to its effective provider"
        );

        let refs = references(src, dialect, 1, 11, &analysis, true);
        assert!(
            refs.iter().any(|range| range.start_line == 6),
            "declaration-origin references must collect the same site: {refs:?}"
        );
    }

    #[test]
    fn a_receiver_that_unexports_the_inherited_name_abstains_both_ways() {
        // The receiver's own `unexport` decides, not the provider's
        // declaration: tclsh 8.6.16 / 9.0.4 answer `[Child new] tick` with
        // `unknown method "tick"` even though `Base` exports it, so the
        // capture is not a call site of `Base::tick` at all.
        let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    unexport tick\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
        assert!(
            list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor)
                .is_none(),
            "an unexported receiver name must not resolve a captured [self] callback"
        );

        let refs = references(src, dialect, 1, 11, &analysis, true);
        assert!(
            !refs.iter().any(|range| range.start_line == 7),
            "declaration-origin references must make the same abstention: {refs:?}"
        );
    }

    #[test]
    fn a_receiver_that_reexports_an_unexported_provider_resolves_both_ways() {
        // The mirror case, and the one a provider-only reading gets wrong in
        // the other direction: `Base` unexports its own `tock`, `Child`
        // exports the inherited name, and tclsh 8.6.16 / 9.0.4 run `Base`'s
        // body for `[Child new] tock`.
        let src = "oo::class create Base {\n    method tock {} { return 1 }\n    unexport tock\n}\noo::class create Child {\n    superclass Base\n    export tock\n    method wire {} {\n        after idle [list [self] tock]\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let cursor = u32::try_from(src.find("] tock").unwrap() + 2).unwrap();
        assert_eq!(
            list_built_self_method_target_at_cursor(src, dialect, &analysis, "tock", cursor),
            Some(("::Base".to_owned(), false)),
            "a re-exporting receiver revives the inherited callback target"
        );

        let refs = references(src, dialect, 1, 11, &analysis, true);
        assert!(
            refs.iter().any(|range| range.start_line == 8),
            "declaration-origin references must collect the same site: {refs:?}"
        );
    }

    #[test]
    fn an_overriding_receiver_keeps_its_callback_in_its_own_family() {
        // `Child` declares its own `tick`, so the capture dispatches there.
        // `Base::tick`'s family must not claim it (renaming `Base::tick`
        // would otherwise rewrite a word that never called it).
        let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    method tick {} { return 2 }\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
        assert_eq!(
            list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor),
            Some(("::Child".to_owned(), false)),
            "the override, not the base, provides this capture"
        );

        let refs = references(src, dialect, 1, 11, &analysis, true);
        assert!(
            !refs.iter().any(|range| range.start_line == 7),
            "the base declaration must not collect the override's capture: {refs:?}"
        );
    }

    #[test]
    fn an_unrelated_class_does_not_collect_a_same_named_callback() {
        // No `superclass` edge, so nothing links the two `tick`s.
        let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Other {\n    method tick {} { return 2 }\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let refs = references(src, dialect, 1, 11, &analysis, true);
        assert!(
            !refs.iter().any(|range| range.start_line == 6),
            "an unrelated class's capture is not a reference: {refs:?}"
        );
    }

    #[test]
    fn a_mixins_unexport_does_not_suppress_the_superclass_capture() {
        // C enters each mixin with a fresh copy of the dispatch flags, so a
        // mixin that unexports the name empties only its own branch: tclsh
        // 8.6.16 / 9.0.4 still run `Base`'s `tick` for `[Child new] tick`.
        let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Mix {\n    unexport tick\n}\noo::class create Child {\n    superclass Base\n    mixin Mix\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
        let analysis = analyse(src);
        let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
        assert_eq!(
            list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor),
            Some(("::Base".to_owned(), false)),
            "a mixin's unexport must not suppress the spine's provider"
        );
    }

    // external $obj method sites

    #[test]
    fn references_from_external_obj_method_site() {
        // Declaration + 2 external call sites (`$d bark`,
        // `[$d bark]`).
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
        let analysis = analyse(src);
        // Cursor on `bark` in `$d bark` (line 4, col 3).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            4,
            3,
            &analysis,
            true,
        );
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
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(lines.contains(&4), "external call missing: {refs:?}");
    }

    #[test]
    fn find_obj_method_call_sites_covers_top_level_and_subst() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &analysis,
            "::Dog",
            "bark",
            false,
        );
        // Two external sites: `$d bark` and `[$d bark]`.
        assert_eq!(sites.len(), 2, "{sites:?}");
    }

    #[test]
    fn find_obj_method_call_sites_finds_calls_in_proc_body() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nproc f {} { $d bark }\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &analysis,
            "::Dog",
            "bark",
            false,
        );
        assert_eq!(sites.len(), 1, "{sites:?}");
    }

    #[test]
    fn find_obj_method_call_sites_matches_bare_created_instance_command() {
        // `Dog create rex` binds `rex` as an object *command*; `rex bark` is a
        // bare-word method dispatch (not `$rex bark`).  The scan resolves it
        // through `created_instance_commands`.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &analysis,
            "::Dog",
            "bark",
            false,
        );
        assert_eq!(sites.len(), 1, "{sites:?}");
        // The matched span is the `bark` method-name token of `rex bark`.
        let s = sites[0];
        assert_eq!(
            &src[s.start() as usize..s.end() as usize],
            "bark",
            "{sites:?}"
        );
    }

    // class-command dispatch: `CLASS method` for a
    // classmethod / `self method`, a receiver set entirely separate from
    // `$obj method` / `NAME method` instance dispatch above.

    #[test]
    fn find_obj_method_call_sites_matches_class_command_and_inheriting_subclass() {
        // TP — both the plain shape (`ActiveRecord find`) and its
        // inherited-via-superclass sibling (`Table find`, ooutil's
        // `classmethod` propagates to a subclass's own bound command).
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\nActiveRecord find foo bar\n";
        let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
        let sites = find_obj_method_call_sites(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
            &analysis,
            "::ActiveRecord",
            "find",
            true,
        );
        assert_eq!(sites.len(), 2, "{sites:?}");
        for s in &sites {
            assert_eq!(
                &src[s.start() as usize..s.end() as usize],
                "find",
                "{sites:?}"
            );
        }
    }

    /// TP — a bare class-command dispatch written inside an `apply` lambda
    /// body or a `namespace eval` body — at the top level or nested inside a
    /// method — is a real call.
    /// All three shapes were confirmed dispatching under tclsh 9.0.4
    /// (`MAKE CALLED` printed three times).
    #[test]
    fn find_obj_method_call_sites_reaches_lambda_and_namespace_eval_bodies() {
        let src = "oo::class create Factory {\n\
                       classmethod make {} { return 1 }\n\
                       method inst {} { apply {{} { Factory make }} }\n\
                       method nsev {} { namespace eval ::zz { Factory make } }\n\
                   }\n\
                   namespace eval ::top2 { Factory make }\n\
                   Factory create obj\n\
                   obj inst\n\
                   obj nsev\n";
        let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
        let sites = find_obj_method_call_sites(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
            &analysis,
            "::Factory",
            "make",
            true,
        );
        assert_eq!(sites.len(), 3, "{sites:?}");
        for s in &sites {
            assert_eq!(
                &src[s.start() as usize..s.end() as usize],
                "make",
                "{sites:?}"
            );
        }
    }

    /// TN — the instance half must **not** follow the class-command half
    /// through a frame shift.  `$f` inside `namespace eval ::zz` names
    /// `::zz::f`, and inside an `apply` lambda a fresh local, so neither is
    /// a dispatch on the outer `f` (tclsh 9.0.4: both raise `can't read
    /// "f": no such variable`).
    #[test]
    fn find_obj_method_call_sites_excludes_var_receivers_across_a_frame_shift() {
        let src = "oo::class create Dog {\n\
                       method bark {} {}\n\
                   }\n\
                   set f [Dog new]\n\
                   namespace eval ::zz { $f bark }\n\
                   apply {{} { $f bark }}\n\
                   $f bark\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &analysis,
            "::Dog",
            "bark",
            false,
        );
        assert_eq!(sites.len(), 1, "{sites:?}");
        let s = sites[0];
        let line = src[..s.start() as usize].lines().count();
        assert_eq!(line, 7, "only the same-frame `$f bark` matches: {sites:?}");
    }

    /// TN — a bare (backslash-escaped) `apply` body element is decoded
    /// before `apply` evaluates it, so its source slice is not the script
    /// that runs; the scan must not re-parse it in place.
    #[test]
    fn find_obj_method_call_sites_skips_escaped_lambda_body_element() {
        let src = "oo::class create Factory {\n\
                       classmethod make {} { return 1 }\n\
                   }\n\
                   apply {{} Factory\\ make}\n";
        let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
        let sites = find_obj_method_call_sites(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
            &analysis,
            "::Factory",
            "make",
            true,
        );
        assert!(sites.is_empty(), "{sites:?}");
    }

    #[test]
    fn find_obj_method_call_sites_excludes_non_inheriting_self_method_subclass() {
        // TN — the references-level precision guard mirroring
        // `self_method_not_inherited_by_a_non_overriding_subclass` in
        // definition.rs: unlike `ooutil`'s `classmethod`, a plain `self
        // method` is not inherited, so `Gadget make` must not be counted
        // as a call site of `Widget`'s `make`.
        let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\noo::class create Gadget {\n    superclass Widget\n}\nWidget make foo\nGadget make foo\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &analysis,
            "::Widget",
            "make",
            true,
        );
        assert_eq!(sites.len(), 1, "{sites:?}");
        assert_eq!(
            &src[sites[0].start() as usize..sites[0].end() as usize],
            "make",
            "{sites:?}"
        );
    }

    #[test]
    fn references_enumerates_class_command_declaration_and_both_call_sites() {
        // TP — the full end-to-end peek from the finding's own repro:
        // declaration, the inherited-subclass call, and the
        // declaring-class's own call — three references, no duplicates,
        // none missed (requires Part 3 in addition to Part 2: Part 2 alone
        // only fixes single-cursor lookups, not this whole-document scan).
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\nActiveRecord find foo bar\n";
        let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
        // Cursor on the declaration (line 1, `find` at col 16).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
            1,
            16,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(refs.len(), 3, "{refs:?}");
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(lines.contains(&6), "Table find call missing: {refs:?}");
        assert!(
            lines.contains(&7),
            "ActiveRecord find call missing: {refs:?}"
        );
    }

    #[test]
    fn references_from_cursor_on_class_command_call_site() {
        // Symmetry with `references_from_cursor_on_bare_obj_command_call_site`:
        // invoking Find All References with the cursor ON the class-command
        // call site (not the declaration) must resolve identically.
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\nActiveRecord find foo bar\n";
        let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
        // Cursor on `find` in `ActiveRecord find foo bar` (line 3, col 13).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
            3,
            13,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(lines.contains(&3), "call site missing: {refs:?}");
    }

    #[test]
    fn references_include_bare_created_instance_command_site() {
        // Full peek: cursor on the `method bark` decl surfaces the bare
        // `rex bark` dispatch as a reference.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(lines.contains(&4), "bare `rex bark` site missing: {refs:?}");
    }

    #[test]
    fn references_from_cursor_on_bare_obj_command_call_site() {
        // Symmetry: invoking Find All References with the cursor
        // ON the `bark` token of a bare `rex bark` dispatch must resolve — not
        // only the declaration-based peek.  `rex` is at col 0, `bark` at col 4.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
        let analysis = analyse(src);
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            4,
            4,
            &analysis,
            true,
        );
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
        let sites = find_obj_method_call_sites(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &analysis,
            "::Dog",
            "bark",
            false,
        );
        assert!(
            sites.is_empty(),
            "bare var receiver wrongly matched: {sites:?}"
        );
    }

    // tcl::OptProc — the `opt` package's automatic-option-parsing proc
    // definer: without its analyser hook the call site is unreachable from
    // the declaration.

    #[test]
    fn references_from_opt_proc_declaration_reach_the_call_site() {
        let src = "::tcl::OptProc greet {child -use -display} { return $child }\ngreet foo\n";
        let analysis = analyse(src);
        // Line 0 — cursor on "greet" right after `::tcl::OptProc` (col 15).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            15,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {refs:?}");
        assert!(lines.contains(&1), "call site missing: {refs:?}");
    }

    #[test]
    fn references_reach_a_proc_dispatched_through_an_eval_of_a_list_computed_var() {
        // `eval $cmdD`, where `$cmdD` is built via `[list greetD World]`, is a
        // real call site: without the constant-value dispatch it is invisible
        // and only the declaration comes back.
        let src = "proc greetD {n} {puts \"D $n\"}\nset cmdD [list greetD World]\neval $cmdD\n";
        let analysis = analyse(src);
        // Line 0 — cursor on `greetD`'s declaration name (col 6).
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {refs:?}");
        assert!(
            lines.contains(&1),
            "the `greetD` word inside `[list greetD World]` must be reachable too: {refs:?}"
        );
    }

    #[test]
    fn method_source_dispatch_keeps_actual_store_availability_and_lookup_barriers() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        // Conditional source reference geometry, not a live receiver or method entry.
        use std::sync::Arc;
        let profile = tcl_dialect::DialectProfile::find("tcl").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "source_dispatch",
            arity: tcl_registry::Arity::at_least(1),
            traits: tcl_registry::Traits::TCLOO_SELF_DISPATCH,
            surface: registry.get("dict").unwrap().surface,
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let store = Arc::new(registry);
        let source = "source_dispatch selected";
        for (release, expected) in [("tcl8.6", 1), ("tcl8.4", 0)] {
            let context = Arc::new(
                tcl_registry::model::ingress::static_context_for(release)
                    .with_command_store(Arc::clone(&store)),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile, profile, context, config,
            );
            let mut analysis = Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name);
            let body = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
            assert_eq!(
                scan_my_method_sites(source, &analysis, &[body], "selected", None).len(),
                expected
            );
            analysis.body_lexer_config.as_mut().unwrap().expand_syntax = !config.expand_syntax;
            assert!(scan_my_method_sites(source, &analysis, &[body], "selected", None).is_empty());
            analysis.body_lexer_config = Some(config);
            let foreign = tcl_registry::model::ingress::resolve_environment("tcl8.6")
                .default_context_registry();
            analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile, profile, foreign, config,
            ));
            assert!(scan_my_method_sites(source, &analysis, &[body], "selected", None).is_empty());
            analysis.resolved_input = None;
            assert!(scan_my_method_sites(source, &analysis, &[body], "selected", None).is_empty());
        }
        let source = "proc source_dispatch args {}; source_dispatch selected";
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.6").with_command_store(store),
        );
        let input =
            tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let body = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
        assert!(scan_my_method_sites(source, &analysis, &[body], "selected", None).is_empty());
    }

    #[test]
    fn caller_frame_reference_component_keeps_retained_registry_and_input() {
        // naming.core.original-caller-frame-navigation
        // docs/design/analysis/name-resolution-proofs/original-caller-frame-navigation.md
        // Readonly source template/reference geometry, not a completed native store.
        let source = "proc setdef {d} {upvar 1 $d dst; set dst SET}\nproc caller {} {setdef shared; puts $shared}\n";
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "context-marker",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let context = tcl_registry::model::ingress::context_for_profile(profile)
            .with_command_store(std::sync::Arc::new(registry));
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let line_index = LineIndex::new(source);
        let read = u32::try_from(source.find("$shared").unwrap()).unwrap() + 1;
        let ctx = RefCtx {
            source,
            dialect: profile,
            line_index: &line_index,
            line: 1,
            character: 0,
            analysis: &analysis,
            include_declaration: true,
            resolution: crate::definition::CallResolution::document_only(),
        };
        let found = caller_frame_references(&ctx, read, "shared").unwrap();
        assert_eq!(found.len(), 2);
        let bare = u32::try_from(source.find("setdef shared").unwrap() + 7).unwrap();
        let position = line_index.position_at_utf16(bare, source);
        let bare_ctx = RefCtx {
            line: position.line,
            character: position.character.get(),
            ..ctx
        };
        assert_eq!(variable_references(&bare_ctx), Some(found));
        let stale = format!("{source}# changed");
        let stale_ctx = RefCtx {
            source: &stale,
            ..ctx
        };
        assert!(caller_frame_references(&stale_ctx, read, "shared").is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        let missing_ctx = RefCtx {
            analysis: &missing,
            ..ctx
        };
        assert!(caller_frame_references(&missing_ctx, read, "shared").is_none());
        let foreign_ctx = RefCtx {
            resolution: ctx
                .resolution
                .with_registry(crate::registry_for_dialect_profile(profile)),
            ..ctx
        };
        assert!(caller_frame_references(&foreign_ctx, read, "shared").is_none());
    }

    #[test]
    fn method_dispatch_scan_uses_retained_custom_schema_and_script_purpose() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        fn reference_only(
            _args: tcl_registry::InvocationArguments<'_>,
        ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
            vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
        }
        let source = "oo::class create C {method m {} {}; method run {} {custom-body {my m}; reference-script {my m}}}";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "custom-body",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::Body)],
            ..tcl_registry::CommandSpec::DEFAULT
        });
        registry.insert(tcl_registry::CommandSpec {
            name: "reference-script",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::Body)],
            script_timing_resolver: Some(reference_only),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let context = tcl_registry::model::context_for_profile(profile)
            .with_command_store(std::sync::Arc::new(registry));
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let mut analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        assert!(analysis.allows_retained_logical_declaration_advice());
        let body = analysis.all_classes["::C"].methods["run"].body_span;
        let sites = scan_my_method_sites(source, &analysis, &[body], "m", None);
        assert_eq!(
            sites.len(),
            1,
            "custom schema is retained; reference-only script is excluded"
        );
        assert_eq!(sites[0].start() as usize, source.find("my m").unwrap() + 3);
        let stale = source.replace("custom-body", "custom-bodz");
        assert!(scan_my_method_sites(&stale, &analysis, &[body], "m", None).is_empty());
        analysis.resolved_input = None;
        assert!(scan_my_method_sites(source, &analysis, &[body], "m", None).is_empty());
    }

    #[test]
    fn shifted_dispatch_regions_follow_original_moves_and_known_source_barriers() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        for (source, visible) in [
            (
                "rename namespace shifted; shifted eval ::N {puts nested}",
                true,
            ),
            (
                "rename namespace shifted; proc shifted {args} {}; shifted eval ::N {puts hidden}",
                false,
            ),
            (
                "rename namespace shifted; rename shifted {}; shifted eval ::N {puts hidden}",
                false,
            ),
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl8.6");
            let command = tcl_compiler::segmenter::segment_commands(source)
                .pop()
                .unwrap();
            let regions = frame_shifted_dispatch_regions(source, &analysis, profile, &command);
            assert_eq!(
                regions
                    .iter()
                    .any(|&(start, end)| &source[start..end] == "puts nested"),
                visible
            );
            assert!(nested_dispatch_regions(source, &analysis, profile, &command).is_empty());
            assert!(
                frame_shifted_dispatch_regions(
                    &source.replace("puts", "gets"),
                    &analysis,
                    profile,
                    &command
                )
                .is_empty()
            );
            analysis.resolved_input = None;
            assert!(
                frame_shifted_dispatch_regions(source, &analysis, profile, &command).is_empty()
            );
        }
    }

    #[test]
    fn expect_clause_flags_reach_each_clause_body() {
        let source = "expect {-re {^ready$} {puts ready} -timeout 5 timeout {puts slow}}";
        let command = tcl_compiler::segmenter::segment_commands(source)
            .into_iter()
            .next()
            .expect("expect command");
        let regions = nested_dispatch_regions(
            source,
            &tcl_compiler::analyser::Analyser::new().analyse(source, "expect"),
            tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
            &command,
        );
        assert_eq!(
            regions.len(),
            2,
            "Expect clause flags must not shift bodies"
        );
        assert!(
            regions
                .iter()
                .any(|&(start, end)| source[start..end].contains("puts ready"))
        );
        assert!(
            regions
                .iter()
                .any(|&(start, end)| source[start..end].contains("puts slow"))
        );
    }

    #[test]
    fn inline_expect_clause_flags_reach_each_clause_body() {
        let source = "expect -re {^ready$} {puts ready} -timeout 5 timeout {puts slow}";
        let command = tcl_compiler::segmenter::segment_commands(source)
            .into_iter()
            .next()
            .expect("expect command");
        let regions = nested_dispatch_regions(
            source,
            &tcl_compiler::analyser::Analyser::new().analyse(source, "expect"),
            tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
            &command,
        );
        assert_eq!(
            regions.len(),
            2,
            "inline Expect flags must not shift bodies"
        );
        assert!(
            regions
                .iter()
                .any(|&(start, end)| source[start..end].contains("puts ready"))
        );
        assert!(
            regions
                .iter()
                .any(|&(start, end)| source[start..end].contains("puts slow"))
        );
    }

    #[test]
    fn malformed_case_lists_do_not_expose_nested_dispatch_regions() {
        for source in [
            "switch subject {a {puts hidden} orphan}",
            "switch subject {}",
        ] {
            let command = tcl_compiler::segmenter::segment_commands(source)
                .into_iter()
                .next()
                .expect("case-list command");
            assert!(
                nested_dispatch_regions(
                    source,
                    &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
                    tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                    &command
                )
                .is_empty(),
                "semantic references must abstain for malformed case list: {source:?}"
            );
        }
    }

    #[test]
    fn case_list_aliases_use_positioned_resolved_registry_identity() {
        let source = concat!(
            "pick subject {default {puts before_alias}}\n",
            "interp alias {} pick {} switch\n",
            "pick subject {default {puts through_alias}}\n",
        );
        let commands = tcl_compiler::segmenter::segment_commands(source);
        assert!(
            nested_dispatch_regions(
                source,
                &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                &commands[0]
            )
            .is_empty(),
            "a later alias must not apply before its declaration"
        );
        let regions = nested_dispatch_regions(
            source,
            &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &commands[2],
        );
        assert!(
            regions
                .iter()
                .any(|&(start, end)| source[start..end].contains("through_alias")),
            "a live static switch alias must expose its clause body: {regions:?}"
        );
    }

    #[test]
    fn deleted_or_rebound_case_list_aliases_do_not_expose_clause_bodies() {
        for source in [
            concat!(
                "interp alias {} pick {} switch\n",
                "interp alias {} pick {}\n",
                "pick subject {default {puts deleted_alias}}\n",
            ),
            concat!(
                "interp alias {} pick {} switch\n",
                "interp alias {} pick {} puts\n",
                "pick subject {default {puts rebound_alias}}\n",
            ),
        ] {
            let command = tcl_compiler::segmenter::segment_commands(source)
                .into_iter()
                .last()
                .expect("alias call");
            assert!(
                nested_dispatch_regions(
                    source,
                    &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
                    tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                    &command
                )
                .is_empty(),
                "a deleted or rebound alias must not use switch grammar: {source:?}"
            );
        }
    }

    #[test]
    fn rooted_and_namespaced_case_list_aliases_expose_clause_bodies() {
        let source = concat!(
            "namespace eval ::case_alias {}\n",
            "interp alias {} ::root_pick {} ::switch\n",
            "interp alias {} ::case_alias::pick {} switch\n",
            "::root_pick subject {default {puts rooted_alias}}\n",
            "::case_alias::pick subject {default {puts namespaced_alias}}\n",
        );
        let commands = tcl_compiler::segmenter::segment_commands(source);
        for (command, marker) in [
            (&commands[3], "rooted_alias"),
            (&commands[4], "namespaced_alias"),
        ] {
            let regions = nested_dispatch_regions(
                source,
                &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                command,
            );
            assert!(
                regions
                    .iter()
                    .any(|&(start, end)| source[start..end].contains(marker)),
                "rooted/namespaced alias must use its resolved switch grammar: {marker}; {regions:?}"
            );
        }
    }

    #[test]
    fn references_reach_my_dispatch_inside_a_static_switch_alias() {
        let source = concat!(
            "interp alias {} pick {} switch\n",
            "oo::class create C {\n",
            "    method target {} {}\n",
            "    method caller {} {\n",
            "        pick subject {default {my target}}\n",
            "    }\n",
            "}\n",
        );
        let analysis = analyse(source);
        // `target` begins after the four-space indent plus `method `.
        let refs = references(
            source,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            2,
            11,
            &analysis,
            true,
        );
        let lines: Vec<u32> = refs.iter().map(|range| range.start_line).collect();
        assert!(lines.contains(&2), "declaration missing: {refs:?}");
        assert!(
            lines.contains(&4),
            "`my target` inside the resolved alias case body is missing: {refs:?}"
        );
    }

    #[test]
    fn switch_braced_body_references_reach_nested_command() {
        let source = "proc ready {} {}\nswitch $state { ready {ready} default {set x 1}}\n";
        let analysis = analyse(source);
        let refs = references(
            source,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            5,
            &analysis,
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line == 0) && refs.iter().any(|r| r.start_line == 1),
            "switch braced body reference missing: {refs:?}"
        );
    }
    #[test]
    fn stale_instance_candidate_does_not_add_a_variable_method_reference() {
        let source =
            "oo::class create C {method run {} {return 1}}\nset receiver 0\n$receiver run\n";
        let mut analysis = analyse(source);
        analysis
            .instance_classes
            .insert("receiver".to_owned(), "::C".to_owned());
        let calls = find_obj_method_call_sites(
            source,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &analysis,
            "::C",
            "run",
            false,
        );
        assert!(
            calls.is_empty(),
            "a filewide candidate is not an actual object read: {calls:?}"
        );
    }
}

#[cfg(test)]
mod original_highlight_selection_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_highlights_preserve_opaque_declarations_and_whole_source_currency() {
        // Implementation contract: naming.editor.original-reference-highlight-selection
        // docs/design/analysis/name-resolution-proofs/original-reference-highlight-selection.md
        let source = r"proc p\uD800 {} {}
proc p\uD801 {} {}
p\uD800";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(analysis.original_procedure_declarations().count(), 2);
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        for invocation in &mut analysis.command_invocations {
            invocation.name.clear();
            invocation.resolved_qualified_name = None;
            invocation.resolution_candidates.clear();
        }
        let profile = crate::profile_for_analysis(&analysis);
        let highlights = document_highlights(source, profile, 0, 7, &analysis);
        assert_eq!(highlights.len(), 2, "declaration and actual selected call");
        assert!(
            highlights
                .iter()
                .all(|(_, kind)| *kind == HighlightKind::Text)
        );
        assert_eq!(highlights[0].0.start_line, 0);
        assert_eq!(highlights[1].0.start_line, 2);
        assert_eq!(references(source, profile, 0, 7, &analysis, true).len(), 2);
        let stale = source.replace(r"p\uD800", r"q\uD800");
        assert!(document_highlights(&stale, profile, 0, 7, &analysis).is_empty());
        assert!(references(&stale, profile, 0, 7, &analysis, true).is_empty());
    }

    #[test]
    fn original_method_reference_helpers_do_not_select_from_reporting_names() {
        // Implementation contract: naming.editor.original-reference-highlight-selection
        // docs/design/analysis/name-resolution-proofs/original-reference-highlight-selection.md
        let source = "oo::class create C {method m {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(analysis.all_classes.contains_key("::C"));
        let profile = crate::profile_for_analysis(&analysis);
        let offset = u32::try_from(source.find("m {}").unwrap()).unwrap();
        let std::ops::ControlFlow::Break(Some(candidate)) =
            crate::method_symbol::local_candidate(source, &analysis, 0, offset)
        else {
            panic!("authentic source member remains available through the typed query");
        };
        let highlights = document_highlights(source, profile, 0, offset, &analysis);
        assert_eq!(highlights.len(), 1);
        assert_eq!(candidate.declaration_span().start(), offset);
        assert!(
            list_built_self_method_target_at_cursor(source, profile, &analysis, "m", offset,)
                .is_none(),
            "typed original declarations cannot enter lexical tuple selection"
        );
        assert!(method_reference_spans_in_document(
            source, profile, &analysis, "::C", "m", true, false,
        ).is_empty());
        assert!(
            obj_method_call_sites(source, profile, &analysis, "::C", "m", false, &[],).is_empty()
        );
        assert!(
            method_next_dispatch_spans(&analysis, source, profile, "::C", "m", false,).is_empty()
        );
    }
}
