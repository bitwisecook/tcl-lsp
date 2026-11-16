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

//! The procedure or class a written call denotes under Tcl's command resolution.

use super::command_heads::{
    command_indirection_target, invocation_head_at, logical_called_procedure_at,
    logical_procedure_header_at,
};
use super::namespace_imports::{
    forced_import_shadows, proc_visible_via_wildcard_import, wildcard_import_source_namespace,
};
use super::{CallResolution, namespace_context_at};
use tcl_compiler::analyser::AnalysisResult;

/// Resolve a written call `word` to the user proc C Tcl's command resolution
/// would pick (`Tcl_FindCommand`, `tclNamesp.c`): each candidate qualified name
/// from [`tcl_syntax::naming::command_resolution_candidates`] — the caller's
/// namespace, then each `namespace path` entry, then global; an absolute
/// `::`-prefixed word is exact — looked up in `all_procs`.  `namespace` is the
/// caller's `::`-prefixed namespace (`"::"` at global scope).
///
/// Honouring the caller namespace's recorded `namespace path` (from
/// `analysis.namespace_paths`) is what lets a bare call reach a proc on the
/// path before a same-named global — matching how call-site settling already
/// resolves, so definition / hover / signature help agree with references.
pub(super) fn proc_visible_from_namespace<'a>(
    analysis: &'a AnalysisResult,
    namespace: &str,
    word: &str,
    call_off: u32,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    if let Some(input) = invocation_head_at(analysis, call_off)
        .and_then(|invocation| invocation.original_name_input.as_ref())
    {
        let scope = analysis.original_namespace_scope_at(call_off)?;
        // This is declaration assistance. The positioned implementation
        // receipt is selected first by resolve_called_proc.
        let candidates = analysis.procedures_for_original_name(input, scope);
        return match candidates.as_slice() {
            [candidate] => Some(*candidate),
            _ => None,
        };
    }
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let path = analysis
        .namespace_paths
        .get(namespace)
        .map_or(&[][..], Vec::as_slice);
    tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(namespace, path, word)
        .into_iter()
        .find_map(|qname| analysis.all_procs.get(&qname))
}

/// The qualified name C Tcl's command resolution **commits to** when the
/// written word `word` is called from `namespace`: the first candidate from
/// [`tcl_syntax::naming::command_resolution_candidates`] (the caller's
/// namespace, then each `namespace path` entry, then global; an absolute
/// `::`-prefixed word is exact) for which `exists` reports a real command.
///
/// Tcl commits to the first existing candidate and never falls through to a
/// same-tail alternative, which is precisely what a name-set match cannot
/// express.  A caller asking "does this call reach *my* definition?" must
/// therefore compare the returned name against its own qualified name, not
/// against the word's tail: written inside `namespace eval ::b`, a bare
/// `Factory` reaches `::b::Factory` whenever that exists, and only otherwise
/// `::Factory` — never `::a::Factory`, however identically the tail is
/// spelled (verified against tclsh 8.6.14 / 9.0.4).
///
/// This is the single namespace-resolution primitive behind the bare
/// object-command / classmethod dispatch matcher and the [incr Tcl]
/// class-proc resolver, so those two cannot disagree about
/// which definition a written word reaches.
///
/// Deliberate limits: `exists` sees only what this document's analysis
/// knows, so a command defined in a sibling file does not shadow a local
/// candidate here (the cross-document layer supplies its own predicate), and
/// runtime-only bindings — `rename`, `interp alias`, `unknown` — are not
/// modelled.
pub(crate) fn resolved_command_name(
    analysis: &AnalysisResult,
    namespace: &str,
    word: &str,
    exists: &dyn Fn(&str) -> bool,
) -> Option<String> {
    let path = analysis
        .namespace_paths
        .get(namespace)
        .map_or(&[][..], Vec::as_slice);
    tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(namespace, path, word)
        .into_iter()
        .find(|qname| exists(qname))
}

/// Resolve a call `word` written in `namespace` to the proc it denotes:
/// C Tcl's rule first ([`proc_visible_from_namespace`]); then, unless the
/// word names a `registry` builtin — which C Tcl would resolve instead, so a
/// same-named proc in an unrelated namespace must not shadow it — the
/// lenient tail match kept for procs whose defining namespace isn't
/// statically visible at the call, made deterministic by
/// [`fallback_proc_by_simple_name`].
///
/// A candidate whose own declaration is nested inside another proc's (or
/// class's) body is never allowed to win this gate: nested code runs only
/// conditionally, when and if the enclosing definition is actually invoked,
/// so a `proc ::set {...}` written inside some unrelated proc must not
/// permanently outrank the real `set` builtin for every call site in the
/// workspace — the classic "rename the builtin away, install a shadow,
/// restore it" idiom (e.g. `rest.tcl`'s `describe`) otherwise leaves hover /
/// go-to-definition / references pointing at the shadow forever, even at a
/// call site that runs strictly after the shadow has been renamed back off.
/// [`AnalysisResult::offset_is_inside_any_definition_body`] is the same
/// judgement the analyser's own W123 gate (`registry_name_deleted_before`)
/// already applies to a *deletion* recorded inside a body — reused here
/// rather than re-derived, so both agree on what "load-time" means.
///
/// `ctx` carries everything outside the document: its
/// [`CallResolution::registry`], when `Some`, supplies the builtin gate
/// (`None` skips it — callers without a registry keep the lenient behaviour,
/// including the nested-shadow one, since there is no builtin to protect), and
/// its [`CallResolution::program`], when `Some`, supplies the whole-program
/// export oracle the `-force` shadow needs. Both absent is the document-only
/// behaviour.
pub(crate) fn resolve_called_proc<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    namespace: &str,
    word: &str,
    call_off: u32,
    ctx: CallResolution<'_>,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    if !analysis.allows_lexical_declaration_advice() {
        let std::ops::ControlFlow::Break(Some(identity)) =
            crate::original_declaration::select_at_offset(ctx.uri(), source, analysis, call_off)
        else {
            return None;
        };
        return Some(identity.procedure_metadata(analysis)?.metadata());
    }
    if analysis.allows_retained_logical_declaration_advice() {
        return logical_called_procedure_at(analysis, source, call_off)
            .map(tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration::metadata);
    }
    if let Some(reference) = invocation_head_at(analysis, call_off)
        .and_then(|invocation| invocation.resolved_command_reference.as_ref())
    {
        return reference
            .linked_definition()
            .or_else(|| reference.definition())
            .and_then(|definition| analysis.proc_for_definition(definition, source));
    }
    if let Some(invocation) = invocation_head_at(analysis, call_off) {
        if let Some(definition) = &invocation.resolved_definition {
            // A retained actual implementation outranks all document-final name
            // assistance, including definitions installed later in a reached body.
            return analysis.proc_for_definition(definition, source);
        }
        // A member-body head is looked up first in its receiver's unnamed
        // namespace; the namespace-text tables below cannot see that lookup.
        if invocation
            .original_lookup
            .as_ref()
            .is_some_and(|lookup| lookup.receiver_namespace_commands().is_some())
        {
            return None;
        }
    }
    let has_builtin = ctx.registry.is_some_and(|r| r.get(word).is_some());
    // A `namespace import -force` *replaces* a same-named command the
    // importing namespace already holds, so from its own position onward the
    // local definition is no longer what a bare call reaches (oracle on
    // [`forced_import_shadows`]). The ordinary namespace lookup is therefore
    // skipped entirely while such a shadow is live — including when the
    // import's source lives in another file, where answering with the local
    // definition would be the wrong answer and the cross-document resolver is
    // the one that can answer.
    let forced_shadow = forced_import_shadows(analysis, ctx, namespace, word, call_off);
    if !forced_shadow
        && let Some(proc_def) = proc_visible_from_namespace(analysis, namespace, word, call_off)
    {
        let nested_shadow = has_builtin
            && analysis.offset_is_inside_any_definition_body(proc_def.name_span.start());
        if !nested_shadow {
            return Some(proc_def);
        }
    }
    // A wildcard `namespace import NS::*` creates no real `ProcDef` of its
    // own — `proc_visible_from_namespace` can never see it — but it makes
    // an exported command in `NS` callable bare, exactly as if a real proc
    // existed at the candidate's qualified name. Tried at the same
    // priority tier (same nested-shadow gate) so an unconditional
    // top-level import still outranks a same-named builtin, matching how
    // an equivalent top-level `proc` redefinition already would.
    if let Some(proc_def) =
        proc_visible_via_wildcard_import(analysis, ctx, namespace, word, call_off)
    {
        let nested_shadow = has_builtin
            && analysis.offset_is_inside_any_definition_body(proc_def.name_span.start());
        if !nested_shadow {
            return Some(proc_def);
        }
    }
    if has_builtin || forced_shadow {
        // With a `-force` shadow live, this document's own command of that
        // name is gone; the lenient tail match must not resurrect it (nor
        // reach for a same-named proc in some unrelated namespace). Whatever
        // the import's source namespace holds is the answer, and the
        // cross-document resolver is the one that can find it when the source
        // lives in another file.
        return None;
    }
    let hit = fallback_proc_by_simple_name(analysis, source, word)?;
    // …and the lenient tail match must not smuggle back in the one answer the
    // import gate has just refused.
    if only_route_is_a_dead_import(
        analysis,
        ctx,
        namespace,
        word,
        call_off,
        &hit.qualified_name,
    ) {
        return None;
    }
    Some(hit)
}

/// Whether the *only* way the call at `call_off` could have reached
/// `target_qname` is a `namespace import` that this document has already
/// judged not to be in force — the case where
/// [`fallback_proc_by_simple_name`]'s tail match would silently overrule the
/// gate.
///
/// The false positive it closes is an import whose export runs only after it:
///
/// ```tcl
/// namespace eval ::src { proc p {} {return P} }
/// namespace eval ::dst { namespace import ::src::* }
/// namespace eval ::src { namespace export p }
/// namespace eval ::dst { p }        ;# → invalid command name "p"
/// ```
///
/// every ordered route says "no", yet the tail match would answer `::src::p`
/// anyway, because it never asks *how* the name would be reachable.
///
/// # Why this is narrow on purpose
///
/// Other same-file fallbacks are deliberate and pinned by tests: a proc whose
/// defining namespace simply is not statically visible at the call still
/// answers, and must keep answering. So this refuses only when **both** hold:
///
/// 1. an import in scope for the call (its own namespace, a `namespace path`
///    entry, or global) syntactically covers `word` *from the namespace the
///    fallback answer lives in* — i.e. that import is the route, not some
///    unrelated leniency; and
/// 2. the live-import walk ([`wildcard_import_source_namespace`]) does not
///    land on that namespace — the route exists in the text but is not in
///    force here (unexported at the import, forgotten, conflicted away, or
///    simply not run yet).
///
/// A call with no import naming that namespace at all is untouched, and so is
/// one whose import *does* resolve (which never reaches the fallback anyway).
fn only_route_is_a_dead_import(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    namespace: &str,
    word: &str,
    call_off: u32,
    target_qname: &str,
) -> bool {
    if tcl_syntax::naming::is_qualified(word.as_bytes()) {
        return false;
    }
    let (target_ns, tail) = tcl_syntax::naming::key_holder_and_tail(target_qname);
    if tail != word {
        return false;
    }
    let target_ns = if target_ns.is_empty() {
        "::"
    } else {
        target_ns
    };
    let path = analysis
        .namespace_paths
        .get(namespace)
        .map_or(&[][..], Vec::as_slice);
    let in_scope: Vec<String> =
        tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
            namespace, path, word,
        )
        .into_iter()
        .map(|candidate| {
            tcl_syntax::naming::key_holder_and_tail(&candidate)
                .0
                .to_owned()
        })
        .collect();
    let covered = analysis.namespace_imports.iter().any(|imp| {
        in_scope.contains(&imp.ns)
            && imp.source.as_ref().is_some_and(|source| {
                same_namespace(&source.namespace, target_ns)
                    && tcl_syntax::glob::string_match(&source.tail_pattern, word)
            })
    });
    covered
        && wildcard_import_source_namespace(analysis, ctx, namespace, word, call_off)
            .is_none_or(|live| !same_namespace(&live, target_ns))
}

/// Namespace-name equality, ignoring the leading `::` one spelling carries and
/// the other does not.
pub(super) fn same_namespace(a: &str, b: &str) -> bool {
    tcl_syntax::naming::unroot_rooted_key(a).unwrap_or(a)
        == tcl_syntax::naming::unroot_rooted_key(b).unwrap_or(b)
}

/// Readonly procedure metadata at the cursor. Retained Logical declarations
/// and calls use their genuine original header; its qualified name is reporting
/// data, never an independent allocation or edit permission.
///
/// The separate legacy `(all_procs key, ProcDef)` route resolves:
///
/// 1. the proc whose declaration name span covers the cursor (a declaration
///    -site invocation), else
/// 2. the namespace-aware call-site resolution ([`resolve_called_proc`], C
///    Tcl's own rule from the caller's namespace).
///
/// It never falls back to a namespace-blind `p.name == word` scan — that would
/// let a rename triggered from a bareword call site pick an *arbitrary*
/// same-named proc in an unrelated namespace (`HashMap` order) and rewrite the
/// wrong definition while leaving the one under the cursor untouched.  The
/// returned key equals `ProcDef::qualified_name` (the map is keyed by it).
pub(crate) fn resolve_proc_target_at<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cursor_off: u32,
    word: &str,
    ctx: CallResolution<'_>,
) -> Option<(&'a String, &'a tcl_compiler::analyser::ProcDef)> {
    if let std::ops::ControlFlow::Break(selected) =
        crate::math_function_symbol::select_at_offset(source, analysis, cursor_off)
    {
        let declaration = selected?.procedure_metadata()?;
        return Some((
            &declaration.metadata().qualified_name,
            declaration.metadata(),
        ));
    }
    if !analysis.allows_lexical_declaration_advice() {
        let std::ops::ControlFlow::Break(Some(identity)) =
            crate::original_declaration::select_at_offset(ctx.uri(), source, analysis, cursor_off)
        else {
            return None;
        };
        let declaration = identity.procedure_metadata(analysis)?;
        return Some((
            &declaration.metadata().qualified_name,
            declaration.metadata(),
        ));
    }
    if analysis.allows_retained_logical_declaration_advice() {
        crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let header = logical_procedure_header_at(analysis, cursor_off)
            .or_else(|| logical_called_procedure_at(analysis, source, cursor_off))?;
        return Some((&header.metadata().qualified_name, header.metadata()));
    }
    if let Some(hit) = analysis
        .all_procs
        .iter()
        .find(|(_, p)| p.name_span.start() <= cursor_off && cursor_off < p.name_span.end())
    {
        return Some(hit);
    }
    // A declaration a later same-named `proc` displaced keeps its own name
    // token, which `all_procs` (winner-only) can never match; it still
    // declares that qualified name, so resolve through to whichever
    // definition currently wins for it — the same rule
    // `tcl-lsp-server::resolve_workspace_symbols` applies cross-document.
    if let Some((qualified, _)) = analysis
        .proc_declaration_sites
        .iter()
        .find(|(_, span)| span.start() <= cursor_off && cursor_off < span.end())
        && let Some(hit) = analysis.all_procs.get_key_value(qualified)
    {
        return Some(hit);
    }
    // An `expr` math-function call site resolves through the analyser's own
    // two-candidate mathfunc rule, not the ordinary bareword one — so
    // find-references / rename / call-hierarchy / linked-editing triggered
    // *at a call site* reach the same `proc ::tcl::mathfunc::NAME` a query
    // from the declaration reaches. `word` is unusable at such a cursor
    // anyway — the word-span scan hands back `li(`.
    if let Some(inv) = crate::expr_context::mathfunc_call_at(analysis, cursor_off) {
        if !analysis.allows_lexical_declaration_advice() {
            return None;
        }
        return inv
            .resolved_qualified_name
            .as_deref()
            .and_then(|resolved| analysis.all_procs.get_key_value(resolved));
    }
    // A word the command table has moved onto (a `rename` destination, an
    // `interp alias` name) targets the definition it really reaches, so
    // find-references / rename / call-hierarchy issued from either spelling
    // converge on one proc. Order-gated, so a
    // call written before the mutation still resolves the ordinary way.
    if let Some(target) = command_indirection_target(analysis, word, cursor_off)
        && let Some(hit) = analysis.all_procs.get_key_value(&target)
    {
        return Some(hit);
    }
    let ns = namespace_context_at(
        &analysis.global_scope,
        cursor_off,
        &analysis.namespace_overrides,
    );
    let proc_def = resolve_called_proc(analysis, source, &ns, word, cursor_off, ctx)?;
    analysis.all_procs.get_key_value(&proc_def.qualified_name)
}

/// The class analogue of [`resolve_proc_target_at`]: the `(all_classes key,
/// ClassDef)` a class-oriented editor operation targets at `cursor_off` — the
/// class whose declaration name span covers the cursor, else the
/// namespace-aware candidate resolution (a class name *is* a command name, so
/// the same `bareword_resolution_candidates` order applies: caller namespace,
/// then global).  Never a namespace-blind `c.name == word` scan.  The returned
/// key equals `ClassDef::qualified_name`.
pub(crate) fn resolve_class_target_at<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    ctx: CallResolution<'_>,
    cursor_off: u32,
    word: &str,
) -> Option<(&'a String, &'a tcl_compiler::analyser::ClassDef)> {
    if !analysis.allows_lexical_declaration_advice() {
        let std::ops::ControlFlow::Break(Some(identity)) =
            crate::original_declaration::select_at_offset(ctx.uri(), source, analysis, cursor_off)
        else {
            return None;
        };
        let declaration = identity.class_metadata(analysis)?;
        return Some((
            &declaration.metadata().qualified_name,
            declaration.metadata(),
        ));
    }
    if let Some(hit) = analysis
        .all_classes
        .iter()
        .find(|(_, c)| c.name_span.start() <= cursor_off && cursor_off < c.name_span.end())
    {
        return Some(hit);
    }
    // A `rename Dog Cat` / `interp alias {} Pup {} Dog` makes the written word
    // reach a class keyed under a different name; the class's own identity
    // (its `ClassDef`, its method table) is unchanged by the move, so the
    // canonical name is what the lookup needs. Order-gated by the shared
    // walk, so `Cat` written before the rename still resolves to nothing here
    // (the diagnostics side of the same fact is
    // `class_reachable_by_indirection`).
    if let Some(target) = command_indirection_target(analysis, word, cursor_off)
        && let Some(hit) = analysis.all_classes.get_key_value(&target)
    {
        return Some(hit);
    }
    let ns = namespace_context_at(
        &analysis.global_scope,
        cursor_off,
        &analysis.namespace_overrides,
    );
    if let Some(hit) = tcl_syntax::naming::bareword_resolution_candidates(&ns, word)
        .into_iter()
        .find_map(|cand| analysis.all_classes.get_key_value(&cand))
    {
        return Some(hit);
    }
    // A wildcard `namespace import NS::*` makes an exported class in `NS`
    // callable bare — see [`proc_visible_via_wildcard_import`] for the full
    // rationale; classes never consult `namespace path`,
    // matching `bareword_resolution_candidates` above.
    let source_ns = wildcard_import_source_namespace(analysis, ctx, &ns, word, cursor_off)?;
    analysis
        .all_classes
        .get_key_value(&tcl_syntax::naming::qualify(&source_ns, word))
}

/// Simple-name reporting assistance, separate from actual source call targets.
/// Retained Logical candidates are genuine headers from the current image;
/// their deterministic order grants no lookup, allocation or edit authority.
///
/// The separate legacy deterministic tail match considers
/// every proc whose simple name equals `word`, prefer one defined in this
/// document ([`name_token_in_document`]), then the lexicographically
/// smallest qualified name.  `HashMap` iteration order never decides the
/// result.
///
/// A proc living in a `tcl::mathfunc` namespace is excluded: real Tcl reaches
/// it only through `expr`'s function-call production or its own fully-
/// qualified name, never as a bare command word (`li {10 20 30} 1` →
/// `invalid command name "li"` on tclsh 8.6 and 9.0), so without the
/// exclusion this lenient tail match resolves a guaranteed-to-crash bare call
/// to the mathfunc proc.  The namespace fact
/// is the registry's ([`tcl_registry::mathfunc::is_in_mathfunc_namespace`]),
/// not a name test written here.
pub(crate) fn fallback_proc_by_simple_name<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    word: &str,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    if analysis.allows_retained_logical_declaration_advice() {
        crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        // Reporting assistance only: actual call/target selection is terminal
        // through its independent original source receipt before this path.
        return analysis
            .logical_source_procedure_declarations()
            .filter(|header| header.name() == Some(word))
            .filter(|header| {
                !tcl_registry::mathfunc::is_in_mathfunc_namespace(&header.metadata().qualified_name)
            })
            .min_by(|left, right| {
                left.metadata()
                    .qualified_name
                    .cmp(&right.metadata().qualified_name)
                    .then_with(|| {
                        left.metadata()
                            .name_span
                            .start()
                            .cmp(&right.metadata().name_span.start())
                    })
            })
            .map(tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration::metadata);
    }
    analysis
        .all_procs
        .iter()
        .filter(|(qname, _)| !tcl_registry::mathfunc::is_in_mathfunc_namespace(qname))
        .filter(|(_, proc_def)| proc_def.name == word)
        .min_by(|(qname_a, proc_a), (qname_b, proc_b)| {
            let a_foreign = !name_token_in_document(source, proc_a);
            let b_foreign = !name_token_in_document(source, proc_b);
            a_foreign.cmp(&b_foreign).then_with(|| qname_a.cmp(qname_b))
        })
        .map(|(_, proc_def)| proc_def)
}

/// Whether `proc_def`'s name token actually spells its name in `source` —
/// the available evidence that the proc was defined in *this* document
/// rather than carried in from another file's analysis.
fn name_token_in_document(source: &str, proc_def: &tcl_compiler::analyser::ProcDef) -> bool {
    source
        .get(proc_def.name_span.start() as usize..proc_def.name_span.end() as usize)
        .is_some_and(|text| text.ends_with(proc_def.name.as_str()))
}
