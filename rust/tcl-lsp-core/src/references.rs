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
//! `analysis.instance_classes`) matches — plus, when the member is a
//! `classmethod`, every bare `ClassName method` dispatch on the class's
//! own command (a classmethod is never dispatched via an instance).  See
//! [`find_obj_method_call_sites`] for the scan's full coverage.
//!
//! Limitations:
//!
//! * This module is single-document only.  Cross-document references are
//!   built *on top of* it — `tcl-lsp-server`'s `cross_document_references` /
//!   `cross_file_method_references` / `cross_file_consumer_method_references`
//!   call [`obj_method_call_sites`], [`method_reference_spans_in_document`],
//!   and [`inherited_method_call_sites`] once per candidate document, using
//!   the workspace index to find which documents to scan and (for a
//!   classmethod) which class names are valid bare-dispatch heads when the
//!   scanned document doesn't declare the class itself.
//! * A classmethod's class-command dispatch is matched by *resolving* the
//!   written head against the call site's own lexical namespace with C Tcl's
//!   rule (current namespace, then `namespace path`, then global — see
//!   [`CommandReceivers::class_head_matches`]), so two classes sharing a
//!   simple name in different namespaces are never cross-linked (issue
//!   #981).  A `CLASS create NAME` **object command** now resolves the same
//!   way: the analyser records each creation site's namespace
//!   (`AnalysisResult::instance_command_bindings`), so `::a::Factory create
//!   rex` binds `::a::rex` and `::b::Widget create rex` binds `::b::rex`, and
//!   a bare `rex make` reaches whichever its own namespace resolves to — the
//!   object-command half of #981, closed by PR C3.  An object command bound
//!   by a *registry* object factory (a Tk widget path, a tcllib naming
//!   factory) carries no creating user class, so it keeps the bare-name
//!   match; it has no class identity to mis-attribute in the first place.
//! * An **explicit** `namespace import ::a::Factory` is followed: the imported
//!   name is a real command in the importing namespace, so it joins the
//!   resolver's `exists` universe and resolves through to the source command
//!   for the class-identity test ([`explicit_import_aliases`]).  A **wildcard**
//!   import (`namespace import ::a::*`) is **not**: reproducing it needs the
//!   export-gated import *snapshot* — which commands existed in `::a` when the
//!   import ran — that issue #1027 tracks, and treating every exported command
//!   as imported regardless of definition order would invent aliases the
//!   runtime never created.  So a wildcard-imported class's bare dispatch is
//!   still not matched, and a rename still leaves it stale.
//! * [incr Tcl]'s class-scoped `proc` uses a different dispatch shape
//!   entirely — a single `::`-qualified command word (`Factory::make`), not
//!   two words — so it is matched by [`crate::definition::itcl_class_proc_target`]
//!   against `analysis.command_invocations`, with Tcl's own
//!   current-namespace-then-global resolution rather than by name-set
//!   membership.  That path is single-document: a call in a sibling file is
//!   not found, because the cross-file layer below still carries only the
//!   two-word shape's name sets.
//! * `uplevel`'s body is `BodyKind::Structural` (a different call frame in
//!   the general case — level `0` and level `1`+ can't be told apart from
//!   the static registry spec alone), so a `my`/`next`/`$obj method`
//!   dispatch written inside it is not found.
//! * A well-formed `apply {{arglist} {body}} …` lambda's body likewise runs
//!   in its own frame with no route back to the enclosing object's `my`
//!   unless the lambda is explicitly constructed with the object's
//!   namespace, and is not found — though for a different, incidental
//!   reason than `uplevel`: `apply`'s registry `arg_roles` marks its whole
//!   `{arglist body}` argument `Body` (not the body sub-element alone), so
//!   re-segmenting that span as a script sees one non-matching command
//!   (`{arglist}` `{body}`) rather than descending into the nested body at
//!   all. A source that omits the required `{arglist}` wrapper (invalid
//!   `apply` usage, e.g. `apply {my getOptions $k}`) collapses that span to
//!   one level and *can* incidentally re-parse as a matching `my` site —
//!   a narrow quirk of malformed input, not a real dispatch the runtime
//!   would ever reach.
//! * A `case_list` command with per-clause flags (Expect's `expect { -re
//!   pat body … }`) is not decomposed — only a plain `{pattern body …}`
//!   clause list (`switch`'s shape) is.
//!
//! Intra-class `my`/`$obj` dispatch and `next`/`nextto` super-dispatch scans
//! (`scan_my_method_sites`, [`find_obj_method_call_sites`],
//! [`method_next_dispatch_spans`]) all recurse into every `[...]`
//! command-substitution *and* every same-frame (`Plain` `BodyKind`)
//! control-flow / `eval` body — `if`/`while`/`foreach`/`switch` (both the
//! inline and single-braced-clause-list forms)/`try`/`catch`/`dict for`, any
//! nested combination of the two, and, generically, any future command
//! whose registry spec declares a `Plain`-body role — via
//! [`nested_dispatch_regions`].  Recursion is entirely registry-driven
//! ([`tcl_registry::CommandRegistry::plain_body_arg_indices`],
//! [`tcl_registry::CaseListSpec`]); no command name is hardcoded in the
//! walkers themselves (issue #957's general form).
//!
//! The `$obj`-dispatch scan goes one step further for its *command*-receiver
//! half — a class command (`Factory make`) or an object command bound by
//! `CLASS create NAME` (`rex bark`).  Those are ordinary commands, resolvable
//! from any frame, so that half also descends the **frame-shifting** regions
//! ([`frame_shifted_dispatch_regions`]): `Structural`-`BodyKind` bodies
//! (`namespace eval`, `uplevel`, `oo::define`, …) and `apply` lambda bodies.
//! The `$var`-receiver half stops at those boundaries, because a `$f` inside
//! a `namespace eval` body names that namespace's own `f` and inside an
//! `apply` lambda a fresh local (tclsh 9.0.4-verified).

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

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
) -> Vec<tcl_lexer::Span> {
    let indirect = indirect_names_reaching(analysis, &proc_def.qualified_name);
    analysis
        .command_invocations
        .iter()
        .filter(|inv| {
            (invocation_references_proc(analysis, inv, qname, proc_def)
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
/// definition and must not be listed among its references (issue #1116 item
/// 1). Without it the two providers contradict each other on the very same
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
    let owner = def_qualified
        .trim_start_matches("::")
        .rsplit_once("::")
        .map_or("", |(ns, _)| ns);
    let call_ns = crate::definition::namespace_context_at(
        &analysis.global_scope,
        inv.range.start(),
        &analysis.namespace_overrides,
    );
    if call_ns.trim_start_matches("::") != owner {
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
/// `hello` one (issue #923 idx 21).
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
/// code-lens count (PR #1075 review, P2).  Classes pass `None`: a class name
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
/// Whether call site `inv` reaches the definition named `def_name` /
/// `def_qualified` **only** through an in-scope, same-document wildcard
/// `namespace import NS::*` (issue #923 idx 18) — a case
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
/// the export snapshot at that import's own position (issue #1027), the
/// non-`-force` conflict that makes an import install nothing, a `namespace
/// forget` or a deletion of the source command that takes the alias away
/// again, and an import *chain* that reaches the definition through an
/// intermediate namespace (issue #1103). All of it comes from the one shared
/// entry point go-to-definition resolves through
/// (`definition::import_chain_target`), so references and definition cannot
/// disagree about what a call site reaches. Testing "some import matches" and
/// "the final export set covers the name" as two independent conditions — as
/// this originally did — gets both directions wrong.
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
    let target_ns = def_qualified
        .trim_start_matches("::")
        .rsplit_once("::")
        .map_or(String::new(), |(ns, _)| ns.to_owned());
    let call_ns = crate::definition::namespace_context_at(
        &analysis.global_scope,
        inv.range.start(),
        &analysis.namespace_overrides,
    );
    crate::definition::import_chain_target(analysis, ctx, &call_ns, def_name, inv.range.start())
        .is_some_and(|source_ns| source_ns.trim_start_matches("::") == target_ns)
}

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
    // since it's a single accumulated namespace string, not a path (issue
    // #923 idx 56, main audit wave).
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
/// call is a reference to (issue #1116 item 1), and find-references has to
/// answer that the same way go-to-definition does.
#[must_use]
pub fn references_in_program(
    source: &str,
    dialect: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    include_declaration: bool,
    program: Option<crate::definition::ProgramExports<'_>>,
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
    // resolvers below answered a caller-frame `$dataset` read with the
    // declaration of an unrelated same-named TclOO method (issue #923 audit
    // idx 58).  `variable_references` returns `None` for both "not a variable
    // position" and "a variable that resolved to nothing", so the stop has to
    // be made here, on the token kind.
    if crate::caller_frame::substituted_var_read_at(
        source,
        dialect,
        line,
        character,
        crate::definition::byte_offset_at(&line_index, source, line, character),
    )
    .is_some()
    {
        return Vec::new();
    }

    // Namespace references — a word the registry marks
    // `ArgRole::NamespaceName` (issue #1088).  Checked before every bareword
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

    // Class references (checked first, before proc names).
    if let Some(out) = class_references(&ctx, &word) {
        return out;
    }

    // `<ensemble> <subcommand>` — a static `namespace ensemble create
    // -map`/`-subcommands` mapping (issue #923 idx 106). Checked before
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
    // (issue #990). Tried *after* `proc_references` so an ordinary
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

    // `constructor` / `destructor` keyword — the next-chain reference story
    // (issue #992); neither has a name to dispatch on, so no `my`/`$obj`
    // call-site scan applies the way it does for a method. Tried *before*
    // the ordinary class-member lookup below: a class may legally also
    // declare a `method`/`property` literally named `constructor` or
    // `destructor` (the keyword form and a same-named ordinary member are
    // independent), and `class_member_references`'s cursor-outside-any-span
    // fallback (see `resolve_member_span`) would otherwise claim a cursor
    // sitting on the special keyword token for that unrelated same-named
    // member instead (Codex review on #1011, P2). This resolver only ever
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
/// `include_declaration` (issue #1088).
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
    if word != "constructor" && word != "destructor" {
        return None;
    }
    let cursor_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    let class_def = analysis
        .all_classes
        .values()
        .find(|cd| cd.body_span.start() < cursor_offset && cursor_offset < cd.body_span.end())?;
    let name_span = if word == "constructor" {
        class_def.constructors.last().map(|c| c.name_span)
    } else {
        class_def.destructor.as_ref().map(|d| d.name_span)
    }?;
    if !(name_span.start() <= cursor_offset && cursor_offset <= name_span.end()) {
        return None;
    }
    let (decl_span, call_spans) = if word == "constructor" {
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
    dialect: &'a str,
    line_index: &'a LineIndex,
    line: u32,
    character: u32,
    analysis: &'a AnalysisResult,
    include_declaration: bool,
    /// Everything outside this document that a call-site resolution may
    /// consult — in practice the whole-program export oracle (issue #1116
    /// item 1). Default (`document_only`) for a host with no workspace index.
    resolution: crate::definition::CallResolution<'a>,
}

/// Build `decl + references` ranges for a variable at the cursor.
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
    // go-to-definition on a `-force`-shadowed callee (issue #1116 item 1).
    let resolution = resolution.with_registry(tcl_registry::registry_for_dialect(dialect));
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
    // substring in a comment or a data brace is not a reference (issue #923
    // idx 24), and neither is the `$n` inside a brace-quoted *name* word
    // (`set {$n} 1`), which falls through to the declaration-span search in
    // the final `else` so it answers the literal cell (PR #1106 review, P2).
    let var_def = if let Some(var_name) = crate::definition::substituting_var_at_position(
        source,
        dialect,
        line,
        character,
        byte_offset,
    ) {
        match crate::definition::lookup_var_read_at(
            &analysis.global_scope,
            source,
            dialect,
            byte_offset,
            &var_name,
            analysis.ns_var_global_fallback(),
        ) {
            Some(def) => def,
            // Nothing in this frame assigns it — but a callee may create it
            // here through `upvar`, in which case the call-site word that
            // names it and every `$name` read are one variable (issue #923
            // audit idx 58).
            None => return caller_frame_references(ctx, byte_offset, &var_name),
        }
    } else if let Some(binding) = crate::caller_frame::binding_at_offset(
        analysis,
        source,
        dialect,
        resolution.with_registry(tcl_registry::registry_for_dialect(dialect)),
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
        // scope-chain walk (issue #923 differential-audit finding idx 9,
        // main audit wave).
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
) -> Vec<tcl_lexer::Span> {
    let indirect = indirect_names_reaching(analysis, &class_def.qualified_name);
    analysis
        .command_invocations
        .iter()
        .filter(|inv| {
            (invocation_references_class(analysis, inv, qname, class_def)
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
        crate::definition::CallResolution::document_only(),
        cursor_off,
        word,
    )?;
    let mut out = Vec::new();
    if include_declaration {
        out.push(span_to_range(source, line_index, class_def.name_span));
    }
    // `superclass <C>` / `mixin <C>` (and `forward … TARGET`) usages are
    // ordinary command references now — the analyser records each as a
    // `command_invocation` resolved in the referencing class's namespace — so
    // `class_reference_spans` (over `command_invocations`) already covers them,
    // in this document and, via the workspace index, across files.  Rename and
    // the code-lens count read the same collection, so the three never diverge.
    for span in class_reference_spans(analysis, ctx.resolution, qname, class_def) {
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
    let (qname, proc_def) = crate::definition::resolve_proc_target_at(
        analysis,
        source,
        cursor_off,
        word,
        crate::definition::CallResolution::document_only(),
    )?;
    let mut out = Vec::new();
    if include_declaration {
        out.push(span_to_range(source, line_index, proc_def.name_span));
    }
    for span in proc_reference_spans(analysis, ctx.resolution, qname, proc_def) {
        out.push(span_to_range(source, line_index, span));
    }
    dedup_ranges(&mut out);
    Some(out)
}

/// Build references for an ensemble-subcommand call site: when the cursor
/// sits on the subcommand word of a `<ensemble> <subcommand>` call and it
/// resolves through a static `namespace ensemble create -map`/
/// `-subcommands` mapping, surface the target proc's declaration plus every
/// call site — the reference twin of `definition()`'s identical check
/// (issue #923 idx 106). The actual per-call-site matching needs no new
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
        crate::definition::instance_method_at_cursor(source, line, character)?;
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
    for span in proc_reference_spans(analysis, ctx.resolution, qname, proc_def) {
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
    let (inst, method, is_dollar) =
        crate::definition::instance_method_at_cursor(source, line, character)?;
    // Two receiver spellings reach an instance method, and both must be
    // recognised here — `definition.rs` and `hover.rs` already handle the
    // pair. `$obj m` names an instance *variable* whose class is known;
    // `my m` is an internal dispatch whose receiver is the class whose body
    // lexically encloses the call, and it also reaches unexported methods
    // (issue #923 idx 34: find-references at a `my duplListCheck` call site
    // returned nothing at all, because `analysis.instance_classes` has no
    // entry named `my`).
    let line_index_local = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&line_index_local, source, line, character);
    let (class_q, external) =
        match crate::definition::receiver_instance_class_at(analysis, &inst, is_dollar, cursor) {
            Some(class_q) => (class_q.clone(), true),
            None if crate::definition::is_self_dispatch_keyword(&inst) => (
                crate::definition::enclosing_class_at(analysis, cursor)?.to_owned(),
                false,
            ),
            None => return None,
        };
    // The class that actually *declares* the implementation this call
    // reaches — which is not the receiver's own class when the method comes
    // from a `mixin` or a `superclass` (issue #923 idx 34/35: the reference
    // scan keyed off the receiver class alone and gave up whenever the
    // method was purely inherited, while go-to-definition resolved it).
    // One shared linearisation walk with definition and hover.
    let provider_q = crate::oo_dispatch::method_dispatch_provider(
        analysis,
        &class_q,
        &method,
        external,
        crate::definition::MethodBucket::Instance,
    )
    .map_or(class_q, |(provider, _)| provider.to_owned());
    // `$obj method` dispatch is always an instance-method receiver — a
    // `classmethod` is never reached this way.
    let (decl_span, mut call_spans) =
        method_references_for_class(source, dialect, analysis, &provider_q, &method, false)?;
    // `next` / `nextto` super-dispatch is a reference (but never a rename site).
    call_spans.extend(method_next_dispatch_spans(
        analysis,
        source,
        dialect,
        &provider_q,
        &method,
        false,
    ));
    Some(build_member_ranges(
        source,
        line_index,
        decl_span,
        call_spans,
        include_declaration,
    ))
}

/// Build references for a bare `ClassName method` call site: the reverse of
/// [`instance_method_references`], for a `classmethod` — which dispatches on
/// the class's own command, never an instance, so it is never found by
/// `$obj`/`my` resolution.  Without this, Find References / Rename
/// triggered from the actual dispatch site (as opposed to the declaration
/// or a code lens) silently found nothing (Codex review on #971, P2).
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
    let (inst, method, is_dollar) =
        crate::definition::instance_method_at_cursor(source, line, character)?;
    if is_dollar {
        return None;
    }
    let class_q = crate::definition::classmethod_dispatch_class(analysis, &inst, &method)?;
    let (decl_span, mut call_spans) =
        method_references_for_class(source, dialect, analysis, &class_q, &method, true)?;
    call_spans.extend(method_next_dispatch_spans(
        analysis, source, dialect, &class_q, &method, true,
    ));
    Some(build_member_ranges(
        source,
        line_index,
        decl_span,
        call_spans,
        include_declaration,
    ))
}

/// Build references for a `Factory::make` call site — [incr Tcl]'s
/// colon-qualified class-proc dispatch (issue #990).
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
    is_classmethod: bool,
) -> Option<(tcl_lexer::Span, Vec<tcl_lexer::Span>)> {
    use tcl_lexer::Span;
    let class_def = analysis.all_classes.get(class_q)?;
    let decl_span = if is_classmethod {
        class_def.class_methods.get(method)
    } else {
        class_def.methods.get(method)
    }
    .map(|m| m.name_span)?;

    let mut call_spans: Vec<Span> = Vec::new();
    // Intra-class `my method` dispatch: `self`/`my` is bound to the instance
    // for a plain method (and its constructors/destructor) and to the class
    // object itself for a classmethod — the two tables never merge
    // (confirmed against tclsh 9.0.4), so the re-segmented body set must
    // stay scoped to the same table `is_classmethod` selects on both sides
    // (see `collect_member_bodies_scoped`) — mixing them let a classmethod
    // wrongly "reach" an unrelated instance method (or vice versa) whenever
    // the two happened to share a name.
    //
    // For a plain method, scan `class_q`'s own bodies **and** the bodies of
    // every subclass that *inherits* this definition — a class whose MRO
    // resolves `method` to `class_q` (i.e. it does not override).  A pure
    // inheritor is not itself a rename family member (it declares no copy of
    // `method`), but its `my method` calls dispatch to `class_q`'s
    // definition, so they must rename with it; omitting them left those call
    // sites pointing at the old name.  A subclass that *overrides* `method`
    // resolves to itself, not `class_q`, so its bodies are handled under its
    // own family entry — never here.  A classmethod has no equivalent
    // inheriting-subclass walk here: unlike the instance MRO, there's no
    // established evidence in this codebase of how classmethod inheritance
    // should fold into this specific intra-class scan, so this stays scoped
    // to `class_q`'s own class-method bodies only.
    let hierarchy = analysis.class_hierarchy();
    if is_classmethod {
        let bodies: Vec<Span> = collect_member_bodies_scoped(class_def, true);
        call_spans.extend(scan_my_method_sites(
            source,
            dialect,
            &bodies,
            method,
            Some(decl_span),
        ));
    } else {
        let mut bodies: Vec<Span> = collect_member_bodies_scoped(class_def, false);
        for (other_q, other_cd) in &analysis.all_classes {
            if other_q.as_str() != class_q
                && hierarchy.method_target(other_q, method) == Some(class_q)
            {
                bodies.extend(collect_member_bodies_scoped(other_cd, false));
            }
        }
        call_spans.extend(scan_my_method_sites(
            source,
            dialect,
            &bodies,
            method,
            Some(decl_span),
        ));
    }
    // External `$obj method` / bare `ClassName method` sites.
    call_spans.extend(find_obj_method_call_sites(
        source,
        dialect,
        analysis,
        class_q,
        method,
        is_classmethod,
    ));
    // Definition-body words that *name* the member — `export m` / `unexport m`
    // / `filter m` / `deletemethod m` (the registry's
    // `MemberRefKind::Method` members).  These are references, and rewriting
    // them is load-bearing: an `export` left naming the old name leaves the
    // renamed method unexported and every call to it fails (tclsh 9.0.4 /
    // 8.6.16 alike).
    call_spans.extend(member_reference_spans(source, dialect, class_def, method));
    Some((decl_span, call_spans))
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
    dialect: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    property: &str,
) -> Option<(tcl_lexer::Span, Vec<tcl_lexer::Span>)> {
    let class_def = analysis.all_classes.get(class_q)?;
    let decl_span = class_def.properties.get(property)?.name_span;
    let call_spans = scan_my_method_sites(
        source,
        dialect,
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
    dialect: &str,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
) -> Vec<tcl_lexer::Span> {
    let Some(class_def) = analysis.all_classes.get(class_q) else {
        return Vec::new();
    };
    // `<constructor>` / `<destructor>` name the two slots that have no
    // method-table entry of their own; every other name is looked up in the
    // bucket the caller selected. Without the synthetic labels, references
    // to a constructor never surfaced an inheriting subclass's `next` call
    // (issue #923 idx 37).
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
    scan_next_dispatch_sites(source, dialect, m.body_span)
}

/// Canonicalise a written class name (`nextto`'s argument) to the qualified
/// form keyed in `analysis.all_classes`, owner-aware — the same resolution
/// `definition.rs`'s go-to-definition `next`/`nextto` handling uses (kept as
/// a separate copy rather than shared: it is a two-line wrapper around the
/// registry's own `resolve_class_name`, so a cross-module `pub(crate)`
/// promotion would cost more than it saves). Falls back to the written name
/// when nothing resolves, so the caller's MRO lookup simply finds no match.
fn canonicalise_class_name(analysis: &AnalysisResult, owner: &str, name: &str) -> String {
    let tail_index =
        tcl_compiler::analyser::class_hierarchy::build_tail_index(analysis.all_classes.keys());
    tcl_compiler::analyser::class_hierarchy::resolve_class_name(
        name,
        owner,
        |n| analysis.all_classes.contains_key(n),
        &tail_index,
    )
    .unwrap_or_else(|| name.to_owned())
}

/// Every `next` / `nextto` call site — across every other class in this
/// document — whose target resolves (via the class hierarchy's MRO) to
/// `class_q`'s own effective constructor: a subclass constructor chaining
/// up to its superclass's is a name-independent but still meaningful
/// "referenced by an overriding subclass" relationship (issue #992), the
/// constructor counterpart of [`method_next_dispatch_spans`]. Constructors
/// have no name-based dispatch, so this next-chain scan is the *whole*
/// reference story for one — no `my`/`$obj` call-site scan applies, unlike
/// [`method_references_for_class`]. Returns `None` when `class_q` declares
/// no explicit constructor.
///
/// Not gated by `DefinerFamily` (Codex review on #1011, P1): `next` /
/// `nextto` and the MRO this walks (`ClassHierarchy::mro_map`, built by
/// `tcloo_linearise` for every `ClassDef` alike) are `TclOO`-specific —
/// Snit / [incr Tcl] classes have different chaining models the registry
/// doesn't even register a `next`/`nextto` command for. This is not a new
/// gap: [`method_next_dispatch_spans`] / [`ClassHierarchy::next_provider`]
/// have run unconditionally across every definer family since they were
/// introduced, with no existing itcl/Snit exclusion (unlike
/// `find_obj_method_call_sites`'s classmethod dispatch-shape check, which
/// *does* consult `is_itcl_class` for an unrelated concern). Gating only
/// the constructor/destructor path here would be an inconsistent partial
/// fix, not a real one — a proper fix scopes the whole next/nextto
/// reference system by family, out of scope for this change.
pub(crate) fn constructor_next_chain_references(
    source: &str,
    dialect: &str,
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
        for (span, target) in scan_next_dispatch_sites_with_target(source, dialect, ctor.body_span)
        {
            let start_from = target.map(|t| canonicalise_class_name(analysis, other_q, &t));
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
    dialect: &str,
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
        for (span, target) in scan_next_dispatch_sites_with_target(source, dialect, dtor.body_span)
        {
            let start_from = target.map(|t| canonicalise_class_name(analysis, other_q, &t));
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
/// workspace index (same behaviour as before this parameter existed).
#[must_use]
pub fn obj_method_call_sites(
    source: &str,
    dialect: &str,
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
    dialect: &str,
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
    dialect: &str,
    bodies: &[tcl_lexer::Span],
    method: &str,
    skip: Option<tcl_lexer::Span>,
) -> Vec<tcl_lexer::Span> {
    let ctx = MyMethodScan {
        source,
        dialect,
        method,
        skip,
    };
    let mut out: Vec<tcl_lexer::Span> = Vec::new();
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    for &body_span in bodies {
        let mut sink = SpanSink {
            out: &mut out,
            seen: &mut seen,
        };
        scan_my_method_body(ctx, body_span, &mut sink);
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
    dialect: &'a str,
    method: &'a str,
    skip: Option<tcl_lexer::Span>,
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
/// combination of the two, is found too (issue #957). This is the same
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
        tcl_lexer::LexerConfig::for_dialect(ctx.dialect),
    );
    for cmd in &commands {
        if let (Some(head), Some(name_tok)) = (cmd.argv.first(), cmd.argv.get(1)) {
            let h_start = head.span.start() as usize;
            let h_end = head.span.end() as usize;
            // Registry query, not a `== "my"` literal: the self-dispatch
            // keyword is spec data, so a dialect that gains or loses it
            // propagates through `tcl-registry` rather than through this
            // walker (issue #1050).
            let head_is_self_dispatch = h_start < source.len()
                && h_end <= source.len()
                && crate::definition::method_dispatch_keyword_in(
                    ctx.dialect,
                    &source[h_start..h_end],
                ) == Some(tcl_registry::MethodDispatchKind::SelfDispatch);
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
        for (inner_start, inner_end) in nested_dispatch_regions(source, ctx.dialect, cmd) {
            scan_my_method_region(ctx, inner_start, inner_end, depth + 1, sink);
        }
        // `switch`'s brace-delimited arm bodies are Tcl scripts too, but
        // they're neither a `[...]` command substitution (the recursion
        // above never reaches them) nor reachable any other way — descend
        // each arm body as its own command-sequence region (issue #923 idx
        // 63, main audit wave: the corpus's own "assigned `Add`-dispatcher"
        // idiom keeps its `my AddBarSeries` calls inside exactly this
        // shape, `switch ... { barSeries { my AddBarSeries {*}$args } }`).
        // Matches `Analyser::switch_arm_bodies`'s own bare-name check for
        // which commands own this shape.
        if cmd.argv.first().is_some_and(|h| {
            let h_start = h.span.start() as usize;
            let h_end = h.span.end() as usize;
            h_start < source.len() && h_end <= source.len() && &source[h_start..h_end] == "switch"
        }) {
            let switch_arg_tokens: Vec<tcl_lexer::Token> =
                cmd.argv.iter().skip(1).copied().collect();
            for (_, body_tok) in
                tcl_compiler::analyser::commands::switch_arm_bodies(cmd.args(), &switch_arg_tokens)
            {
                scan_my_method_body(ctx, body_tok.span, sink);
            }
        }
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
/// body is found too (issue #957's general form).
///
/// Thin wrapper over [`scan_next_dispatch_sites_with_target`] that drops the
/// `nextto` target argument — a method's `next`/`nextto` is flagged as a
/// reference purely by presence, never MRO-resolved, so the target is
/// irrelevant here (unlike the constructor/destructor next-chain resolvers,
/// which need it to disambiguate `nextto`).
fn scan_next_dispatch_sites(
    source: &str,
    dialect: &str,
    body: tcl_lexer::Span,
) -> Vec<tcl_lexer::Span> {
    scan_next_dispatch_sites_with_target(source, dialect, body)
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
    dialect: &str,
    body: tcl_lexer::Span,
) -> Vec<(tcl_lexer::Span, Option<String>)> {
    let mut out = Vec::new();
    if body.is_empty() {
        return out;
    }
    let (start, end) = strip_outer_braces(source, body);
    if start < end {
        scan_next_dispatch_region_with_target(source, dialect, start, end, 0, &mut out);
    }
    out
}

/// Segment `source[start..end]` and append the head-token span (plus
/// `nextto`'s target argument, when present) of every `next` / `nextto`
/// command, recursing per [`nested_dispatch_regions`]. `depth` guards
/// against runaway recursion — see [`MAX_DISPATCH_SCAN_DEPTH`].
fn scan_next_dispatch_region_with_target(
    source: &str,
    dialect: &str,
    start: usize,
    end: usize,
    depth: u32,
    out: &mut Vec<(tcl_lexer::Span, Option<String>)>,
) {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    if start >= end || end > source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return;
    }
    let body_text = &source[start..end];
    let commands = segment_commands_with_offset_and_config(
        body_text,
        u32::try_from(start).unwrap_or(0),
        tcl_lexer::LexerConfig::for_dialect(dialect),
    );
    for cmd in &commands {
        if let Some(head) = cmd.argv.first() {
            let (h_start, h_end) = (head.span.start() as usize, head.span.end() as usize);
            if h_end <= source.len() && h_start < h_end {
                let h = &source[h_start..h_end];
                // The next-chain keywords come from the registry
                // (`TCLOO_NEXT_CHAIN`), not a name list (issue #1050).
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
                    // real class (Codex review on #1011, P2).
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
        for (inner_start, inner_end) in nested_dispatch_regions(source, dialect, cmd) {
            scan_next_dispatch_region_with_target(
                source,
                dialect,
                inner_start,
                inner_end,
                depth + 1,
                out,
            );
        }
    }
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
    dialect: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
    extra_classmethod_cmd_names: &[String],
) -> Vec<tcl_lexer::Span> {
    let Some(class_def) = analysis.all_classes.get(class_q) else {
        return Vec::new();
    };
    // Scoped to the same table `is_classmethod` selects (see
    // `collect_member_bodies_scoped`) — `class_q` here is the pure
    // inheritor itself, so its own classmethod bodies may still contain a
    // `my <method>` call reaching the inherited classmethod.
    let bodies = collect_member_bodies_scoped(class_def, is_classmethod);
    let mut spans = scan_my_method_sites(source, dialect, &bodies, method, None);
    spans.extend(find_obj_method_call_sites_with_extra_cmd_names(
        source,
        dialect,
        analysis,
        class_q,
        method,
        is_classmethod,
        extra_classmethod_cmd_names,
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
        dialect,
        &collect_member_bodies(class_def),
        word,
        None,
    );
    Some((member_span, call_spans))
}

/// Find every external `$v method` / `[$v method]` call site
/// in the document where `v` is an instance variable whose
/// class qualified-name is `class_q` (per
/// `analysis.instance_classes`), plus — when `method` is a
/// `classmethod` — every bare `ClassName method` dispatch on the
/// defining class's own command (and any inheriting subclass's own
/// command).  Returns the spans of the method-name tokens.
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
/// the two-word `Factory make` dispatch `classmethod` / `typemethod` use
/// (issue #990).
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
    dialect: &str,
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
/// The receiver sets a dispatch scan for `method` on `class_q` must match:
/// the `$v method` variables, and the bare-word command receivers.
///
/// `target` is `(class_q, method, is_classmethod)`, bundled to keep the
/// parameter count inside the lint budget; `is_classmethod` is the caller's
/// already-resolved fact about which of a possibly-same-named
/// `method` / `classmethod` pair is meant, never re-derived here.
fn dispatch_receivers<'a>(
    analysis: &'a AnalysisResult,
    dialect: &str,
    target: (&str, &str, bool),
    extra_cmd_names: &[String],
) -> (FxHashSet<&'a str>, CommandReceivers) {
    let (class_q, method, is_classmethod) = target;
    let hierarchy = analysis.class_hierarchy();
    // A classmethod dispatches on the class's own command, never an
    // instance — instance-receiver matching only ever applies to a `method`.
    // Variables whose `$obj method` dispatch resolves to `class_q`'s copy of
    // `method` — its own instances **plus** instances of any subclass that
    // *inherits* this definition (the subclass's MRO resolves `method` to
    // `class_q`).  An exact-class-equality filter dropped the inheriting-
    // subclass sites, silently leaving them pointing at the old name after an
    // inherited-method rename.  A subclass that *overrides* `method` resolves
    // to itself, so its instances are excluded here and rewritten under that
    // subclass's own family entry — each site is attributed to exactly one
    // family member (no double count).
    let var_set: FxHashSet<&str> = if is_classmethod {
        FxHashSet::default()
    } else {
        analysis
            .instance_classes
            .iter()
            .filter(|(_, c)| {
                c.as_str() == class_q || hierarchy.method_target(c, method) == Some(class_q)
            })
            .map(|(v, _)| v.as_str())
            .collect()
    };
    // Receivers that are also *object commands* — bound by `CLASS create NAME`
    // (so `NAME` is a command, dispatched bare as `NAME method`, not `$NAME
    // method`).  A `set v [CLASS new]` receiver is a *variable* only and never
    // enters this set, so a bare `v method` is (correctly) not matched.
    //
    // A name the analyser recorded a *namespace-qualified* binding for is
    // matched by resolving the written head against the call site's own
    // namespace (issue #981).  The bare-name set below is kept only for names
    // with no such binding — the registry object-factories (Tk widget paths,
    // tcllib naming factories) and the external-package `create NAME` shape,
    // neither of which carries a creating user class to attribute a dispatch
    // to in the first place.
    let bound_tails: FxHashSet<&str> = analysis
        .instance_command_bindings
        .iter()
        .filter_map(|b| {
            std::str::from_utf8(tcl_syntax::naming::written_command_tail(
                b.qualified_name.as_bytes(),
            ))
            .ok()
        })
        .collect();
    let mut receivers = CommandReceivers {
        object_commands: var_set
            .iter()
            .copied()
            .filter(|name| {
                analysis.created_instance_commands.contains(*name) && !bound_tails.contains(*name)
            })
            .map(str::to_owned)
            .collect(),
        class_targets: FxHashSet::default(),
        class_tails: FxHashSet::default(),
        object_command_targets: FxHashSet::default(),
        object_command_tails: FxHashSet::default(),
        object_command_universe: analysis
            .instance_command_bindings
            .iter()
            .map(|b| b.qualified_name.clone())
            .collect(),
        import_aliases: explicit_import_aliases(analysis),
    };
    // A `classmethod` never dispatches on an instance, so object commands are
    // only receivers for a plain `method` — the same rule that leaves
    // `var_set` empty above.
    if !is_classmethod {
        for binding in &analysis.instance_command_bindings {
            if binding.class_q == class_q
                || hierarchy.method_target(&binding.class_q, method) == Some(class_q)
            {
                receivers.add_object_command_target(&binding.qualified_name);
            }
        }
    }
    // A `classmethod` dispatches on the *class's own* command (`Factory
    // make`) — never on an instance: TclOO's `classmethod` sugar puts the
    // method on the class object's own class, which no instance's MRO
    // reaches.  When the caller says `method` is a classmethod, and
    // `class_q`'s definer family actually uses this two-word dispatch shape,
    // fold in the defining class's fully qualified name plus that of any
    // subclass that inherits (does not override) it — the same inheritance
    // test as the instance `var_set` above, so a `Sub make` dispatch on an
    // inheriting subclass counts too.  Never runs for a plain `method` — even
    // one that shares a name with a `classmethod` on the same class — which
    // must never gain a phantom `ClassName method` match.
    //
    // Only the *qualified* name goes in: a written head is matched by
    // resolving it against the call site's own namespace
    // ([`CommandReceivers::class_head_matches`]), not by name-set membership,
    // so a bare `Factory` inside `namespace eval ::b` reaches `::b::Factory`
    // and never `::a::Factory` (issue #981).
    //
    // The definer-family check matters because [incr Tcl]'s class-scoped
    // `proc` lands in this same `class_methods` bucket (so the declaration
    // side finds it) but dispatches as a single `::`-qualified identifier
    // (`Factory::make`) — a bare two-word `Factory make` in itcl source is
    // unrelated instance-creation syntax (`ClassName instanceName`), not a
    // call to this proc.  The dispatch shape is registry data
    // (`DefinerFamily`), not something to infer from the shared storage
    // bucket.
    //
    // Unlike `ooutil`'s `classmethod` keyword (which propagates to a
    // subclass's own bound command via its `Delegate`-mixin machinery —
    // confirmed against tclsh 9.0.4/8.6), a plain stock-`TclOO` `self
    // method` is visible ONLY on the exact class that declared it, so the
    // inheriting-subclass half of this loop is skipped for those
    // (`MethodDef::is_self_method`, issue #923 idx 120).
    if is_classmethod
        && let Some(cq_method) = analysis
            .all_classes
            .get(class_q)
            .filter(|cd| !is_itcl_class(cd, dialect))
            .and_then(|cd| cd.class_methods.get(method))
    {
        if let Some(cd) = analysis.all_classes.get(class_q) {
            receivers.add_class_target(&cd.qualified_name);
        }
        if !cq_method.is_self_method {
            for (other_q, other_cd) in &analysis.all_classes {
                if other_q.as_str() != class_q
                    && hierarchy.method_target(other_q, method) == Some(class_q)
                    && !is_itcl_class(other_cd, dialect)
                {
                    receivers.add_class_target(&other_cd.qualified_name);
                }
            }
        }
    }
    // The caller's workspace-wide classmethod knowledge, for a document where
    // the class itself isn't known locally.  Meaningless for a plain method.
    // Qualified names, so the same namespace-resolution rule applies to a
    // pure-consumer document as to one that declares the class.
    if is_classmethod {
        for name in extra_cmd_names {
            receivers.add_class_target(name);
        }
    }
    (var_set, receivers)
}

/// The classes whose instances dispatch `class_q`'s copy of `method`, for the
/// object-type-lattice half of the `$v method` scan (issue #994 C5b):
/// `class_q` itself plus every class that *inherits* (does not override) the
/// definition — the same inheritance rule as `dispatch_receivers`' `var_set`.
///
/// A `$v` site whose lattice binding (`by_scope`, resolved at the site's own
/// offset) is a **singleton** in this set is a call site of the method even
/// when the analyser's `instance_classes` walk never bound `v` — a handle
/// that flowed through an alias, a proc return, a proc/constructor parameter,
/// or a `$var method` return.  Empty for a classmethod (never dispatched on
/// an instance) and when the document has no lattice bindings at all.
fn lattice_dispatch_family(
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
) -> FxHashSet<String> {
    if is_classmethod || analysis.object_handle_facts.by_scope.is_empty() {
        return FxHashSet::default();
    }
    let hierarchy = analysis.class_hierarchy();
    let mut family: FxHashSet<String> = FxHashSet::default();
    for classes in analysis.object_handle_facts.by_scope.values() {
        for c in classes {
            if family.contains(c) {
                continue;
            }
            if c == class_q || hierarchy.method_target(c, method) == Some(class_q) {
                family.insert(c.clone());
            }
        }
    }
    family
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
/// caller mean" when both do (Codex review on #971, P2).
///
/// A same-document call (`extra_cmd_names` empty) derives everything from
/// `analysis` directly, as before.  The cross-file *pure-consumer* path
/// ([`obj_method_call_sites`]) cannot: a document that only calls `Factory
/// make` and never declares/extends `Factory` has no `::Factory` entry in
/// its own `all_classes` for the local classmethod check below to find, so
/// its caller supplies the workspace-wide answer here instead.
fn find_obj_method_call_sites_with_extra_cmd_names(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
    extra_cmd_names: &[String],
) -> Vec<tcl_lexer::Span> {
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
    let (var_set, receivers) = dispatch_receivers(
        analysis,
        dialect,
        (class_q, method, is_classmethod),
        extra_cmd_names,
    );
    let lattice_family = lattice_dispatch_family(analysis, class_q, method, is_classmethod);
    if var_set.is_empty() && lattice_family.is_empty() && !receivers.has_any() {
        return out;
    }
    let mut seen: FxHashSet<(u32, u32)> = out.iter().map(|s| (s.start(), s.end())).collect();
    let ctx = ObjMethodScan {
        source,
        dialect,
        analysis,
        var_set: &var_set,
        lattice_family: &lattice_family,
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

/// The bare-word command receivers a dispatch scan matches — the two kinds
/// are matched by *different* rules, because the analysis knows different
/// things about them.
///
/// * `class_targets` — the fully `::`-qualified names of the classes whose
///   own class command dispatches the method (`Factory make`).  A written
///   head is matched by **resolving** it against the call site's lexical
///   namespace with Tcl's own rule and comparing the winner against this
///   set, never by comparing the text.  So, with a class in `::a` and
///   another in `::b`, a bare `Factory make` written inside `namespace eval
///   ::b` reaches `::b::Factory` and is *not* attributed to `::a::Factory`
///   (issue #981; pinned against tclsh 8.6.14 and 9.0.4, which answer
///   `b-made` there, `invalid command name "Factory"` where no candidate
///   exists, and the global class where only that exists).
///
/// * `object_command_targets` — the object commands bound by `CLASS create
///   NAME`, by the **qualified** name the creation site's namespace produced
///   (`AnalysisResult::instance_command_bindings`).  Matched by the same
///   namespace resolution as `class_targets`, so `::a::Factory create rex`
///   and `::b::Widget create rex` are two commands, not one name (issue
///   #981's object-command half).
///
/// * `object_commands` — the residual bare-name set, for instance commands
///   with **no** qualified binding: those bound by a registry object factory
///   (a Tk widget path, a tcllib naming factory) or by an unresolved
///   external-package `create NAME`.  Neither records a creating user class,
///   so there is no class identity to mis-attribute; an exact text match is
///   all the data supports and all it needs to.
struct CommandReceivers {
    class_targets: FxHashSet<String>,
    /// The `::`-rooted **qualified** names of the object commands (`CLASS
    /// create NAME`) whose creating class dispatches the method — matched by
    /// resolving a written head against the call site's own namespace, the
    /// same rule `class_targets` uses.  Populated from
    /// [`tcl_compiler::analyser::AnalysisResult::instance_command_bindings`],
    /// which records the creation site's namespace (issue #981's
    /// object-command half): `::a::Factory create rex` binds `::a::rex` and
    /// `::b::Widget create rex` binds `::b::rex`, and a bare `rex make`
    /// reaches whichever of the two its own namespace resolves to — never
    /// both, as the bare-name set below could not help doing.
    object_command_targets: FxHashSet<String>,
    /// Simple (tail) names of every entry in `object_command_targets` — the
    /// same cheap pre-filter `class_tails` is for `class_targets`.
    object_command_tails: FxHashSet<String>,
    /// Every qualified object-command name the document binds, whatever class
    /// bound it — the `exists` universe for the resolution above, so a
    /// sibling namespace's same-named object command properly *shadows*
    /// rather than being invisible.
    object_command_universe: FxHashSet<String>,
    /// Simple (tail) names of every entry in `class_targets` — the cheap
    /// pre-filter that keeps the namespace resolution off every command head
    /// in the document.
    class_tails: FxHashSet<String>,
    object_commands: FxHashSet<String>,
    /// `namespace import`-created command aliases: the *imported* name in the
    /// importing namespace (`::b::Factory`) mapped to the source command
    /// (`::a::Factory`).  See [`explicit_import_aliases`].
    import_aliases: FxHashMap<String, String>,
}

/// The command aliases an **explicit** (pattern-free) `namespace import`
/// creates, as `imported qualified name → source qualified name`.
///
/// `namespace import ::a::Factory` inside `::b` creates a real command
/// `::b::Factory` that dispatches `::a::Factory` — tclsh 9.0.4 (probe
/// `ns981.tcl`) confirms both halves: `Factory make` inside `::b` prints
/// `A-MADE`, and `info commands ::b::Factory` lists `::b::Factory`.  Without
/// these entries the bare-dispatch resolver's `exists` universe has no
/// candidate for the imported name at all, so the call site is invisible and
/// a rename leaves it stale.
///
/// **Wildcard imports are deliberately excluded.**  `namespace import ::a::*`
/// needs the export-gated import *snapshot* model (which commands existed in
/// `::a` at the moment the import ran, filtered by `namespace export`) — issue
/// #1027.  Half-building it here, by treating every exported command as
/// imported regardless of definition order, would invent aliases the runtime
/// never created.  A pattern containing any glob metacharacter is skipped.
fn explicit_import_aliases(analysis: &AnalysisResult) -> FxHashMap<String, String> {
    analysis
        .namespace_imports
        .iter()
        .filter(|import| !import.pattern.contains(['*', '?', '[']))
        .filter_map(|import| {
            let tail = std::str::from_utf8(tcl_syntax::naming::written_command_tail(
                import.pattern.as_bytes(),
            ))
            .ok()?;
            (!tail.is_empty()).then(|| {
                (
                    tcl_syntax::naming::qualify(&import.ns, tail),
                    tcl_syntax::naming::normalise_qualified_name(&import.pattern),
                )
            })
        })
        .collect()
}

impl CommandReceivers {
    /// Register a class whose own command dispatches the method, by its
    /// fully qualified name.
    fn add_class_target(&mut self, qualified_name: &str) {
        let tail = tcl_syntax::naming::written_command_tail(qualified_name.as_bytes());
        if let Ok(tail) = std::str::from_utf8(tail) {
            self.class_tails.insert(tail.to_owned());
        }
        self.class_targets.insert(qualified_name.to_owned());
    }

    fn has_any(&self) -> bool {
        !self.class_targets.is_empty()
            || !self.object_commands.is_empty()
            || !self.object_command_targets.is_empty()
    }

    /// Register an object command bound by `CLASS create NAME`, by the
    /// qualified name the creation site's namespace produced.
    fn add_object_command_target(&mut self, qualified_name: &str) {
        if let Ok(tail) = std::str::from_utf8(tcl_syntax::naming::written_command_tail(
            qualified_name.as_bytes(),
        )) {
            self.object_command_tails.insert(tail.to_owned());
        }
        self.object_command_targets
            .insert(qualified_name.to_owned());
    }

    /// Whether the bare head `raw`, written at byte offset `offset`, is one of
    /// the object commands in `object_command_targets` — resolved the way Tcl
    /// resolves it, from the namespace lexically in effect there.
    ///
    /// The `exists` universe is every object command the document binds plus
    /// its procs and classes, so a same-named object command in a nearer
    /// namespace shadows a farther one exactly as it would at runtime
    /// (`Factory create rex` in `::a` and `Widget create rex` in `::b`:
    /// tclsh 9.0.4 / 8.6.16 dispatch `rex make` inside `::a` to `::a::rex`
    /// and inside `::b` to `::b::rex`).
    fn object_command_head_matches(
        &self,
        analysis: &AnalysisResult,
        raw: &str,
        offset: u32,
    ) -> bool {
        if self.object_command_targets.is_empty() {
            return false;
        }
        let Ok(tail) =
            std::str::from_utf8(tcl_syntax::naming::written_command_tail(raw.as_bytes()))
        else {
            return false;
        };
        if !self.object_command_tails.contains(tail) {
            return false;
        }
        let namespace = crate::definition::namespace_context_at(
            &analysis.global_scope,
            offset,
            &analysis.namespace_overrides,
        );
        crate::definition::resolved_command_name(analysis, &namespace, raw, &|candidate| {
            self.object_command_universe.contains(candidate)
                || analysis.all_classes.contains_key(candidate)
                || analysis.all_procs.contains_key(candidate)
        })
        .is_some_and(|winner| self.object_command_targets.contains(&winner))
    }

    /// Whether the bare head `raw`, written at byte offset `offset`, is a
    /// class command in `class_targets` — resolved the way Tcl resolves it,
    /// from the namespace lexically in effect there.
    ///
    /// The `exists` universe is this document's procs and classes, the
    /// caller-supplied targets themselves, and every explicit
    /// `namespace import` alias: a pure-consumer document does not declare the
    /// class it calls, so without the targets no candidate would ever "exist"
    /// and every cross-file consumer site would be lost.  A same-named proc or
    /// class at a higher-priority candidate still shadows the target, exactly
    /// as it would at runtime.
    ///
    /// An imported name is a real command in the importing namespace, so it
    /// both *exists* as a candidate and, when it wins, resolves through to the
    /// source command for the class-identity test — otherwise
    /// `namespace import ::a::Factory; Factory make` inside `::b` matches
    /// nothing (the winning `::b::Factory` is not itself a class target).
    fn class_head_matches(&self, analysis: &AnalysisResult, raw: &str, offset: u32) -> bool {
        if self.class_targets.is_empty() {
            return false;
        }
        let Ok(tail) =
            std::str::from_utf8(tcl_syntax::naming::written_command_tail(raw.as_bytes()))
        else {
            return false;
        };
        if !self.class_tails.contains(tail) {
            return false;
        }
        let namespace = crate::definition::namespace_context_at(
            &analysis.global_scope,
            offset,
            &analysis.namespace_overrides,
        );
        crate::definition::resolved_command_name(analysis, &namespace, raw, &|candidate| {
            self.class_targets.contains(candidate)
                || analysis.all_classes.contains_key(candidate)
                || analysis.all_procs.contains_key(candidate)
                || self.import_aliases.contains_key(candidate)
        })
        .is_some_and(|winner| {
            let dispatched = self.import_aliases.get(&winner).unwrap_or(&winner);
            self.class_targets.contains(dispatched)
        })
    }
}

/// Read-only context for the object-method call-site scan: the document
/// `source`, its `dialect`, its `analysis`, the `method` being looked up, and
/// the receiver sets — `var_set` matches `$v method` dispatch (variables
/// holding an object, keyed by bare name) and `receivers` matches `NAME
/// method` dispatch (object commands bound by `CLASS create NAME`, and class
/// commands resolved namespace-aware).
#[derive(Clone, Copy)]
struct ObjMethodScan<'a> {
    source: &'a str,
    dialect: &'a str,
    analysis: &'a AnalysisResult,
    var_set: &'a FxHashSet<&'a str>,
    /// [`lattice_dispatch_family`] — the classes a `$v` receiver's
    /// scope-keyed lattice binding must singleton-resolve to for the site to
    /// count when `var_set` has no entry for it (issue #994 C5b).
    lattice_family: &'a FxHashSet<String>,
    receivers: &'a CommandReceivers,
    method: &'a str,
    /// `false` once the scan has descended through a frame-shifting region
    /// ([`frame_shifted_dispatch_regions`]), where a `$var` receiver's bare
    /// name no longer names the enclosing frame's variable — so `var_set`
    /// (and the lattice lookup, which resolves the same frame's names)
    /// stops matching while `receivers` (ordinary commands, resolvable from
    /// any frame) keep going.
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
        self.receivers.has_any()
            || (self.var_receivers_in_scope
                && !(self.var_set.is_empty() && self.lattice_family.is_empty()))
    }

    /// Whether the `$var` receiver `name` at `offset` dispatches the method:
    /// bound by the analyser's `instance_classes` walk (`var_set`), or
    /// singleton-resolved by the object-type lattice's scope-keyed map to a
    /// class in [`Self::lattice_family`].  The singleton requirement keeps
    /// the scan inside the rename-grade soundness bar: a multi-class binding
    /// abstains rather than rewrite a site it cannot prove.
    fn var_receiver_matches(&self, name: &str, offset: u32) -> bool {
        self.var_set.contains(name)
            || (!self.lattice_family.is_empty()
                && crate::definition::lattice_singleton_class(self.analysis, name, offset)
                    .is_some_and(|c| self.lattice_family.contains(c)))
    }

    /// Whether the bare-word head `raw` at `offset` names a receiver whose
    /// dispatch this scan is looking for.
    fn command_receiver_matches(&self, raw: &str, offset: u32) -> bool {
        self.receivers.object_commands.contains(raw)
            || self
                .receivers
                .object_command_head_matches(self.analysis, raw, offset)
            || self
                .receivers
                .class_head_matches(self.analysis, raw, offset)
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
/// (issue #957's general form).  `var_set` holds the bare names of
/// in-scope instance variables.  `depth` guards against runaway recursion
/// — see [`MAX_DISPATCH_SCAN_DEPTH`].
fn scan_obj_method_region(
    ctx: ObjMethodScan<'_>,
    start: usize,
    end: usize,
    depth: u32,
    sink: &mut SpanSink<'_>,
) {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    use tcl_lexer::TokenType;
    let source = ctx.source;
    if start >= end || end > source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
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
                    ctx.var_receivers_in_scope
                        && strip_var_decoration(raw)
                            .is_some_and(|name| ctx.var_receiver_matches(name, head.span.start()))
                } else {
                    // A bare-word object command (`rex bark`) or class command
                    // (`Factory make`).  Both receiver sets hold only plain
                    // names, and a braced / bracketed / substituted head's
                    // source slice keeps its delimiters (`{rex}`, `[x]`), so
                    // neither can admit anything but a genuine bare word.
                    ctx.command_receiver_matches(raw, head.span.start())
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
        for (inner_start, inner_end) in nested_dispatch_regions(source, ctx.dialect, cmd) {
            scan_obj_method_region(ctx, inner_start, inner_end, depth + 1, sink);
        }
        let shifted = ctx.frame_shifted();
        if shifted.has_receivers() {
            for (inner_start, inner_end) in frame_shifted_dispatch_regions(source, ctx.dialect, cmd)
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

/// Every nested region reachable from one segmented command that a
/// dispatch scan (`my` / `next` / `nextto` / `$obj method` call-site search)
/// must also visit, so a dispatch written *inside* a nested construct is
/// still found as a reference from the enclosing method: every `[…]`
/// command-substitution fragment in any argument
/// ([`cmd_substitution_regions`]), plus every argument the command registry
/// marks [`tcl_registry::ArgRole::Body`] with a `Plain`
/// [`tcl_registry::BodyKind`] — a same-frame body (`if` / `while` /
/// `foreach` / `switch` / `try` / `catch` / `eval`, …) that still executes in
/// the enclosing method's own dispatch context.
///
/// `Structural` bodies (`proc`, `oo::class create`, `uplevel`, `namespace
/// eval`, …) are *not* descended here — those run in a different scope, so a
/// call written inside one is not a same-context dispatch from this site
/// (see [`tcl_registry::CommandRegistry::plain_body_arg_indices`]). This is
/// the one general mechanism behind the fix for issue #957 (a `my method`
/// call nested in `if` / `while` / `foreach` / `switch` / `try` / `catch` /
/// `eval` was invisible to Find-References, the code-lens reference count,
/// and Rename) — registry-driven, so it needs no per-command-name branch
/// here and covers any command whose spec declares a `Plain` body role, not
/// just the control-flow keywords a hand-written list would enumerate.
pub(crate) fn nested_dispatch_regions(
    source: &str,
    dialect: &str,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<(usize, usize)> {
    let mut regions: Vec<(usize, usize)> = Vec::new();
    for arg in &cmd.argv {
        regions.extend(cmd_substitution_regions(source, dialect, *arg));
    }
    let Some(cmd_name) = cmd.texts.first() else {
        return regions;
    };
    let registry = tcl_registry::registry_for_dialect(dialect);
    let args: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
    // A `case_list` command (`switch`'s braced-list form, Expect's `expect {
    // ... }`) marks its single trailing clause-list argument `ArgRole::Body`
    // too, but that argument is not itself a script — it's alternating
    // `pattern body …` words, so segmenting it directly would misparse each
    // `pattern body` pair as one bogus command (`default { … }`) and never
    // reach the pattern's own body.  When the call is in that single-braced
    // shape, flatten it via the registry's own clause-list vocabulary
    // ([`tcl_registry::CaseListSpec`], never a hardcoded "switch" check) and
    // recurse into each clause's own body word instead.
    if let Some(case_list) = registry.get(cmd_name).and_then(|s| s.case_list)
        && let Some(clause_regions) = case_list_clause_body_regions(source, case_list, &args, cmd)
    {
        regions.extend(clause_regions);
        return regions;
    }
    for idx in registry.plain_body_arg_indices(cmd_name, &args) {
        // `idx` is 0-based into `args` (post-command-name); `argv` is
        // 1-based (`argv[0]` is the command name itself).
        if let Some(tok) = cmd.argv.get(idx + 1) {
            let (start, end) = strip_outer_braces(source, tok.span);
            if start < end {
                regions.push((start, end));
            }
        }
    }
    regions
}

/// Every nested script region reachable from one segmented command whose body
/// runs in a **different frame** from the enclosing one, as `(start, end)`
/// byte offsets into `source`:
///
/// * an argument the registry marks [`tcl_registry::ArgRole::Body`] with a
///   `Structural` [`tcl_registry::BodyKind`] — `namespace eval`, `uplevel`,
///   `oo::define`, `interp eval`, `proc`, … (the complement of
///   [`nested_dispatch_regions`]'s `Plain` set); and
/// * the body element of an [`tcl_registry::ArgRole::LambdaLiteral`]
///   argument — `apply`'s `{argList body ?ns?}` — split by the shared
///   [`tcl_compiler::lambda_literal`] splitter, and only when that element is
///   `{braced}` (a bare / quoted one is backslash-decoded before `apply`
///   evaluates it, so its source slice is not the script that runs).
///
/// These carry the *command*-receiver half of the `$obj method` scan only.
/// A class command (`Factory make`) or a `CLASS create NAME` object command
/// (`rex bark`) is an ordinary command and resolves the same from a
/// `namespace eval` body, an `apply` lambda, or the top level, so a dispatch
/// written in one of them is a real reference to the method — Find All
/// References, rename, the code lens, and the call hierarchy all missed those
/// sites before (adversarial review of #1047, item 2; all three shapes
/// verified dispatching under tclsh 9.0.4).  A `$var` receiver does not
/// survive the boundary — `$f` inside `namespace eval ::zz` names `::zz::f`,
/// and inside an `apply` lambda a fresh local — so the caller drops
/// `var_set` for the descended subtree
/// ([`ObjMethodScan::frame_shifted`]).
///
/// Registry-driven throughout: which arguments are bodies, and whether they
/// are same-frame, comes from the command's spec, never from its name.
pub(crate) fn frame_shifted_dispatch_regions(
    source: &str,
    dialect: &str,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<(usize, usize)> {
    use tcl_lexer::TokenType;
    let mut regions: Vec<(usize, usize)> = Vec::new();
    let Some(cmd_name) = cmd.texts.first() else {
        return regions;
    };
    let registry = tcl_registry::registry_for_dialect(dialect);
    let args: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
    // `plain_body_arg_indices` is `arg_indices_for_role(Body)` gated on the
    // call's resolved `BodyKind`, so an empty plain list against a non-empty
    // body list means every body argument of *this* call is `Structural`.
    if registry.plain_body_arg_indices(cmd_name, &args).is_empty() {
        for idx in registry.arg_indices_for_role(cmd_name, &args, tcl_registry::ArgRole::Body) {
            let Some(tok) = cmd.argv.get(idx + 1) else {
                continue;
            };
            // Only a braced literal body is script at these exact offsets;
            // a `$var` / `[cmd]` body is assembled at runtime.
            if tok.kind != TokenType::Str {
                continue;
            }
            let (start, end) = strip_outer_braces(source, tok.span);
            if start < end {
                regions.push((start, end));
            }
        }
    }
    for idx in registry.arg_indices_for_role(cmd_name, &args, tcl_registry::ArgRole::LambdaLiteral)
    {
        let Some(&tok) = cmd.argv.get(idx + 1) else {
            continue;
        };
        if tok.kind != TokenType::Str {
            continue;
        }
        let Some(body) = tcl_compiler::lambda_literal::split_lambda_literal(source, tok)
            .and_then(|elems| elems.braced_body())
        else {
            continue;
        };
        let (start, end) = (body.start() as usize, body.end() as usize);
        if start < end && end <= source.len() {
            regions.push((start, end));
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
    dialect: &str,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<(usize, usize)> {
    let mut regions = nested_dispatch_regions(source, dialect, cmd);
    regions.extend(frame_shifted_dispatch_regions(source, dialect, cmd));
    regions
}

/// Every word inside `class_def`'s definition body that **references**
/// `method` as a method name — the registry's
/// [`tcl_registry::definer::MemberRefKind::Method`] members (`export m`, `unexport m`,
/// `filter m`, `deletemethod m`, `renamemethod from to`).
///
/// These are genuine references to the member, and load-bearing ones: an
/// `export` naming a method that no longer exists leaves the *renamed*
/// method unexported, so `$obj NewName` fails at run time (`unknown method
/// "Bar": must be destroy` — tclsh 9.0.4 and 8.6.16, identical).  A rename
/// that rewrote only the declaration and the call sites therefore broke the
/// program; rewriting these words is what keeps it running.
///
/// Which member keywords carry method references is **registry data** (the
/// definer's `definition_body` grammar, read through
/// [`crate::oo_body::member_ref_indices`]), so this walker names no member
/// keyword and works for `TclOO`, snit, and itcl alike.
///
/// Regions scanned: the class's own recorded body, plus every definition-body
/// argument in the document whose definer call names this class (an
/// `oo::define Cls { export m }` block written apart from the class's own
/// `create`).
pub(crate) fn member_reference_spans(
    source: &str,
    dialect: &str,
    class_def: &tcl_compiler::analyser::types::ClassDef,
    method: &str,
) -> Vec<tcl_lexer::Span> {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    let registry = tcl_registry::registry_for_dialect(dialect);
    let Some(grammar) = registry
        .get(&class_def.metaclass)
        .and_then(|spec| spec.definition_body)
    else {
        return Vec::new();
    };
    let mut regions: Vec<(usize, usize)> = Vec::new();
    if !class_def.body_span.is_empty() {
        let (start, end) = strip_outer_braces(source, class_def.body_span);
        if start < end {
            regions.push((start, end));
        }
    }
    regions.extend(definition_body_regions_naming(source, dialect, class_def));
    let mut out: Vec<tcl_lexer::Span> = Vec::new();
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    for (start, end) in regions {
        if start >= end || end > source.len() {
            continue;
        }
        let commands = segment_commands_with_offset_and_config(
            &source[start..end],
            u32::try_from(start).unwrap_or(0),
            tcl_lexer::LexerConfig::for_dialect(dialect),
        );
        for cmd in &commands {
            let Some(keyword) = cmd.texts.first() else {
                continue;
            };
            let args: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
            let Some((kind, indices)) = crate::oo_body::member_ref_indices(grammar, keyword, &args)
            else {
                continue;
            };
            if kind != tcl_registry::definer::MemberRefKind::Method {
                continue;
            }
            for idx in indices {
                if args.get(idx) != Some(&method) {
                    continue;
                }
                let Some(tok) = cmd.argv.get(idx + 1) else {
                    continue;
                };
                if seen.insert((tok.span.start(), tok.span.end())) {
                    out.push(tok.span);
                }
            }
        }
    }
    out
}

/// The definition-body argument regions of every top-level definer call in
/// `source` that names `class_def` — `oo::define Cls { … }` and the class's
/// own `oo::class create Cls { … }`.
///
/// Definer recognition is [`crate::oo_body::outer_definition_grammar`]
/// (registry `definition_body` data); the *target* is matched on the class
/// name text, which is a class name, not a command-name special case.
fn definition_body_regions_naming(
    source: &str,
    dialect: &str,
    class_def: &tcl_compiler::analyser::types::ClassDef,
) -> Vec<(usize, usize)> {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    let registry = tcl_registry::registry_for_dialect(dialect);
    let commands = segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::for_file_dialect(dialect),
    );
    let qualified = tcl_syntax::naming::normalise_qualified_name(&class_def.qualified_name);
    let mut regions: Vec<(usize, usize)> = Vec::new();
    for cmd in &commands {
        let Some(keyword) = cmd.texts.first() else {
            continue;
        };
        let args: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
        if crate::oo_body::outer_definition_grammar(keyword, &args, registry).is_none() {
            continue;
        }
        let body_indices =
            registry.arg_indices_for_role(keyword, &args, tcl_registry::ArgRole::Body);
        for idx in body_indices {
            let names_class = args.iter().take(idx).any(|w| {
                *w == class_def.name || tcl_syntax::naming::normalise_qualified_name(w) == qualified
            });
            if !names_class {
                continue;
            }
            if let Some(tok) = cmd.argv.get(idx + 1) {
                let (start, end) = strip_outer_braces(source, tok.span);
                if start < end {
                    regions.push((start, end));
                }
            }
        }
    }
    regions
}

/// For a `case_list` command whose call is in the single-braced-list shape
/// (`switch $x { pat1 body1 pat2 body2 }`) — exactly one non-option argument
/// remains after `case_list.subject_args` — the source regions of every
/// clause's own body word, skipping the literal `-` Tcl `switch`
/// fall-through marker (not a body of its own).
///
/// Returns `None` when the call is instead in the inline pattern/body-pairs
/// shape (any pair count) — each pair's body argument there is already a
/// standalone script `plain_body_arg_indices` finds directly, needing no
/// clause-list unpacking.  Also `None` for a clause list with `clause_flags`
/// (Expect's `expect { -re pat body … }`) — a clause there may carry a
/// variable number of leading flag words before its pattern and body, so
/// naively alternating pattern/body/pattern/body would misassign a flag word
/// as a body; conservative abstention rather than a wrong split, matching
/// this codebase's "fall through to the generic path when correctness can't
/// be proven" rule for constructs the compiler can't safely specialise.  The
/// option-skip loop is driven entirely by `case_list`'s own fields
/// (`value_options`, `subject_args`), never a hardcoded command name, so it
/// applies identically to `switch` and to any future plain (no
/// `clause_flags`) `case_list` command.
fn case_list_clause_body_regions(
    source: &str,
    case_list: &tcl_registry::CaseListSpec,
    args: &[&str],
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
) -> Option<Vec<(usize, usize)>> {
    if !case_list.clause_flags.is_empty() {
        return None;
    }
    // Locating the list is `tcl_syntax`'s one implementation, shared with the
    // semantic-token walker and the fold walk.  `None` = the inline pairs
    // shape (or no clause-list argument at all) — not this function's concern.
    let shape = tcl_syntax::case_list::CallShape {
        subject_args: usize::from(case_list.subject_args),
        regex_option: case_list.regex_option,
        value_options: case_list.value_options,
    };
    let i = tcl_syntax::case_list::clause_list_call(args, &shape)?.index;
    // `args` is 0-based post-command-name; `cmd.texts`/`cmd.argv` are
    // 1-based (index 0 is the command name), so the clause-list word is at
    // `i + 1` in both.
    let (Some(text), Some(tok)) = (cmd.texts.get(i + 1), cmd.argv.get(i + 1).copied()) else {
        return Some(Vec::new());
    };
    let elements = tcl_compiler::segmenter::flatten_clause_list_elements(text, tok);
    let mut out = Vec::new();
    let mut j = 0;
    while j + 1 < elements.len() {
        let (body_text, body_tok) = &elements[j + 1];
        if body_text != "-" {
            let (start, end) = strip_outer_braces(source, body_tok.span);
            if start < end {
                out.push((start, end));
            }
        }
        j += 2;
    }
    Some(out)
}

/// The inner regions of every `[…]` command substitution reachable from a
/// command argument `arg`, as `(start, end)` byte offsets into `source` with
/// the surrounding brackets stripped, for the caller to re-scan.
///
/// A bare `Cmd` arg yields its single bracket-stripped body.  A **bareword or
/// double-quoted** compound word (`"pre [x]"`, `[x]-suf`, `a[x]b`) is merged
/// by the segmenter into one `Esc` token whose embedded substitutions would
/// otherwise be skipped, so its slice is re-lexed and each `[…]` fragment
/// recovered — the same fragment recovery the analyser's `cmd_fragments`
/// performs.  This keeps intra-word `my` / `$obj` dispatch discoverable
/// (references, rename), not only dispatch that is a whole bare argument.
///
/// A braced `Str` word (`{…}`) is left alone: `[…]` inside braces is literal
/// text, not a substitution, so re-lexing it would invent phantom calls.
fn cmd_substitution_regions(
    source: &str,
    dialect: &str,
    arg: tcl_lexer::Token,
) -> Vec<(usize, usize)> {
    use tcl_lexer::TokenType;
    let a_start = arg.span.start() as usize;
    let a_end = arg.span.end() as usize;
    if a_start >= source.len() || a_end > source.len() || a_start >= a_end {
        return Vec::new();
    }
    // Strip the surrounding `[` `]` of a `[…]` fragment span.
    let strip = |f_start: usize, f_end: usize| -> (usize, usize) {
        let inner_start = if source.as_bytes().get(f_start) == Some(&b'[') {
            f_start + 1
        } else {
            f_start
        };
        let inner_end = if f_end > inner_start && source.as_bytes().get(f_end - 1) == Some(&b']') {
            f_end - 1
        } else {
            f_end
        };
        (inner_start, inner_end)
    };
    match arg.kind {
        TokenType::Cmd => vec![strip(a_start, a_end)],
        // Only a bareword / quoted word can carry an *active* `[…]`; re-lex it
        // (and only when it actually embeds one) to recover the fragments the
        // argv merge hid.
        TokenType::Esc => {
            let slice = &source[a_start..a_end];
            if !slice.as_bytes().contains(&b'[') {
                return Vec::new();
            }
            let config = tcl_lexer::LexerConfig::for_dialect(dialect);
            let Ok(tokens) =
                tcl_lexer::Lexer::with_source_map(tcl_lexer::SourceMap::new(slice), config)
                    .tokenise_all()
            else {
                return Vec::new();
            };
            tokens
                .into_iter()
                .filter(|t| t.kind == TokenType::Cmd)
                .filter_map(|t| {
                    let f_start = a_start + t.span.start() as usize;
                    let f_end = a_start + t.span.end() as usize;
                    (f_start < f_end && f_end <= source.len()).then(|| strip(f_start, f_end))
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Strip a `$name` / `${name}` decoration to the bare variable
/// name.  Returns `None` when the text isn't a `$`-prefixed
/// reference.
pub(crate) fn strip_var_decoration(raw: &str) -> Option<&str> {
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
) -> Option<Vec<(LspRange, HighlightKind)>> {
    let cursor_off = crate::definition::byte_offset_at(line_index, source, line, character);
    // Declaration-span hit, else the namespace-aware candidate resolution —
    // never a namespace-blind `c.name == word` first-hit scan (the M1
    // wrong-symbol drift class).
    let (qname, class_def) = crate::definition::resolve_class_target_at(
        analysis,
        crate::definition::CallResolution::document_only(),
        cursor_off,
        word,
    )?;
    // Non-variable symbols (procs / classes / methods) highlight as `Text` for
    // both declaration and uses — only variables carry the Write/Read
    // distinction.
    let mut out = vec![(
        span_to_range(source, line_index, class_def.name_span),
        HighlightKind::Text,
    )];
    for span in class_reference_spans(
        analysis,
        crate::definition::CallResolution::document_only(),
        qname,
        class_def,
    ) {
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
    dialect: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<(LspRange, HighlightKind)> {
    let line_index = LineIndex::new(source);

    let byte_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    // Shared `$ref` gate — see `substituting_var_at_position`.
    if let Some(var_name) = crate::definition::substituting_var_at_position(
        source,
        dialect,
        line,
        character,
        byte_offset,
    ) {
        let Some(var_def) = crate::definition::lookup_var_read_at(
            &analysis.global_scope,
            source,
            dialect,
            byte_offset,
            &var_name,
            analysis.ns_var_global_fallback(),
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

    if let Some(out) = class_highlights(source, &line_index, analysis, line, character, &word) {
        return out;
    }

    {
        let cursor_off = crate::definition::byte_offset_at(&line_index, source, line, character);
        if let Some((qname, proc_def)) = crate::definition::resolve_proc_target_at(
            analysis,
            source,
            cursor_off,
            &word,
            crate::definition::CallResolution::document_only(),
        ) {
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
    fn references_reach_the_call_site_from_the_stale_original_in_a_foreach_rename_reinstall_idiom()
    {
        // TP — issue #923 idx 86, mirroring the same-file precedent set by
        // `references_reach_the_call_site_from_a_shadowed_duplicate_proc_decl_same_document`
        // (idx 31): cursor on a superseded declaration resolves by *name*,
        // landing on whichever declaration currently wins under that name
        // — not the stale span the cursor happened to start on. Here the
        // "shadowing" declaration is the `tk/library/accessibility.tcl`
        // rename-and-reinstall idiom's own per-element wrapper (issue #923
        // idx 86), reached only by simulating each literal `foreach`
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
        let refs = references(src, "tcl", 0, 6, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&5), "winning wrapper decl missing: {refs:?}");
        assert!(lines.contains(&8), "call site missing: {refs:?}");
    }

    #[test]
    fn references_reach_a_method_return_captured_dispatch_site() {
        // Issue #994 C5b / #1143: `b` is typed only by the object-type
        // lattice (`set b [$a make]`, the method-return edge) — the
        // analyser's `instance_classes` never binds it, so before the
        // unification Find References on `greet` missed the `$b greet` site
        // that semantic tokens and hover already resolved.
        let src = "oo::class create A { method make {} { return [B new] } }\n\
                   oo::class create B { method greet {} { return \"hi\" } }\n\
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
        let refs = references(src, "tcl", 1, 29, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(
            lines.contains(&4),
            "the lattice-typed `$b greet` call site is missing: {refs:?}"
        );
        // …and from the call site itself, the declaration answers back.
        let refs = references(src, "tcl", 4, 4, &analysis, true);
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
        let src = "oo::class create A { method make {} { return [B new] } }\n\
                   oo::class create B { method greet {} { return \"hi\" } }\n\
                   proc mk {} { set a [A new]\n  set b [$a make]\n  $b greet }\n\
                   proc other {} { set b 7\n  $b greet }\n";
        let analysis = analyse(src);
        // Cursor on the `greet` declaration (line 1, col 29).
        let refs = references(src, "tcl", 1, 29, &analysis, true);
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
        let refs = references(src, "tcl", 1, 2, &analysis, true);
        assert!(refs.len() >= 2, "expected decl + call sites: {refs:?}");
        // First entry is the declaration on line 0.
        assert_eq!(refs[0].start_line, 0);
    }

    #[test]
    fn references_include_wildcard_imported_bareword_call_same_document() {
        // TP (same-document) — issue #923 idx 18: a wildcard `namespace
        // import ::Foo::*` reaches an exported proc via a bare call with no
        // real command recorded at any of the call's own candidate names,
        // so `invocation_references_named`'s ordinary rule can never catch
        // it; `proc_reference_spans` ORs in
        // `invocation_references_via_wildcard_import` for exactly this
        // case.
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
        let analysis = analyse(src);
        // Cursor on the `bar` declaration (line 1, col 9).
        let refs = references(src, "tcl", 1, 9, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(
            lines.contains(&5),
            "wildcard-imported bareword call site missing: {refs:?}"
        );
    }

    #[test]
    fn references_keep_a_wildcard_imported_call_after_a_later_export_clear() {
        // TP, issue #1027 direction A — the import bound `p` while `::src`
        // still exported it; the later `namespace export -clear` does not
        // revoke that alias (oracle tclsh 8.6.14/9.0.4: `::dst::p` still
        // runs, `info commands ::dst::*` still lists it). Before the
        // per-import-site snapshot, the export gate read the *final* export
        // set — which the `-clear` had emptied — and find-references
        // silently dropped this real call site.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n    p\n}\nnamespace eval src {\n    namespace export -clear\n}\n";
        let analysis = analyse(src);
        // Cursor on the `p` declaration (line 1, col 9).
        let refs = references(src, "tcl", 1, 9, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(
            lines.contains(&6),
            "the imported call site must survive a later `-clear`: {refs:?}"
        );
    }

    #[test]
    fn references_exclude_a_wildcard_imported_call_the_import_predates_the_export_of() {
        // FP guard (CRITICAL), issue #1027 direction B — `::src` exports `p`
        // only *after* `::dst` imported `::src::*`, so real Tcl never binds
        // `::dst::p` at all (oracle: `invalid command name "::dst::p"`) and
        // the bare `p` inside `::dst` is not a call to `::src::p`. Reading
        // the final export set reported it as one.
        let src = "namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n    p\n}\nnamespace eval src {\n    namespace export p\n}\n";
        let analysis = analyse(src);
        // Cursor on the `p` declaration (line 1, col 9).
        let refs = references(src, "tcl", 1, 9, &analysis, true);
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
        // TN, issue #1103 behaviour 1 — the alias the import installed is
        // gone by the time the second `p` runs (oracle: `invalid command
        // name "p"`), so that call is not a reference to `::src::p`. The
        // call *before* the forget still is.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n    p\n    namespace forget ::src::p\n    p\n}\n";
        let analysis = analyse(src);
        // Cursor on the `p` declaration (line 1, col 9).
        let refs = references(src, "tcl", 1, 9, &analysis, true);
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
        // FP guard (CRITICAL), issue #1103 behaviour 2 — `::dst` already has
        // its own `p`, so the non-`-force` import errors and installs
        // nothing (oracle: `can't import command "p": already exists`, and
        // `namespace origin ::dst::p` → `::dst::p`). The bare `p` inside
        // `::dst` runs the *local* proc and is not a reference to `::src::p`.
        let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n    namespace import ::src::*\n    p\n}\n";
        let analysis = analyse(src);
        // Cursor on `::src::p`'s declaration (line 1, col 9).
        let refs = references(src, "tcl", 1, 9, &analysis, true);
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
        let refs = references(src, "tcl", 1, 9, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(
            lines.contains(&7),
            "a `-force` import makes the bare call a reference to the source: {refs:?}"
        );
    }

    #[test]
    fn references_include_a_call_through_an_import_chain() {
        // TP, issue #1103 behaviour 4 — `::A` imports `::B::*`, `::B`
        // imported `::C::*` and re-exported; the bare `p` in `::A` runs
        // `::C::p` (oracle: `namespace origin ::A::p` → `::C::p`), so it is
        // a reference to it. The middle hop is in no `all_procs`, so this
        // previously found nothing.
        let src = "namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n    namespace export p\n}\nnamespace eval A {\n    namespace import ::B::*\n    p\n}\n";
        let analysis = analyse(src);
        // Cursor on `::C::p`'s declaration (line 1, col 9).
        let refs = references(src, "tcl", 1, 9, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "decl missing: {refs:?}");
        assert!(
            lines.contains(&10),
            "the chained bare call is a reference to ::C::p: {refs:?}"
        );
    }

    #[test]
    fn references_include_bare_call_to_a_proc_installed_into_oo_helpers() {
        // Issue #923 idx 56 (main audit wave, high severity): a proc
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
        let refs = references(src, "tcl", 0, 20, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {refs:?}");
        assert!(
            lines.contains(&9),
            "the bare classvar call site inside the method body is missing: {refs:?}"
        );
    }

    #[test]
    fn references_exclude_a_top_level_bare_call_with_the_same_name_as_an_oo_helpers_proc() {
        // FP guard (issue #923 idx 56): a bare call outside any TclOO
        // method body must not be treated as reaching a proc installed in
        // `::oo::Helpers` — real tclsh raises "invalid command name" there
        // (`::oo::Helpers` is on a *method body's* runtime namespace path
        // only, never the global one) — only a call genuinely inside a
        // method body's own runtime namespace path does.
        let src = "proc ::oo::Helpers::classvar {name} {}\nclassvar hits\n";
        let analysis = analyse(src);
        // Cursor on the `::oo::Helpers::classvar` declaration (line 0, col 20).
        let refs = references(src, "tcl", 0, 20, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(
            lines,
            vec![0],
            "a top-level bare call outside any method body must not be linked: {refs:?}"
        );
    }

    #[test]
    fn references_from_in_proc_global_alias_reach_the_callers_canonical_set() {
        // Issue #923 idx 68 (main audit wave, high severity, pix corpus):
        // reduces the real `isEqual`/`tolComp` shape from
        // nico-robert/pix's test/data_b64.test — a proc aliases a top-level
        // cell via `global`, and the caller overrides it via a plain `set
        // ::name` before invoking the proc. tclsh proves `tolComp` (via
        // `global`) and `::tolComp` (the caller's `set`) are the identical
        // storage cell. Querying from the in-proc `$tolComp` read must reach
        // the caller's `set ::tolComp` — before this fix, `collect_alias_spans`
        // only ever found *other aliases* of the same target, never the
        // target's own canonical (non-aliased) declaration.
        let src =
            "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
        let analysis = analyse(src);
        // Cursor on the `$tolComp` read inside the proc (line 2, col 14).
        let refs = references(src, "tcl", 2, 14, &analysis, true);
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
        // The reverse direction of the test above (issue #923 idx 68): before
        // this fix, querying from the caller's own `set ::tolComp` returned
        // only its own 2 spans (decl + any top-level reads), missing every
        // in-proc `global tolComp` occurrence — since a plain `set` has no
        // `link_target` of its own to search alias records by.
        let src =
            "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
        let analysis = analyse(src);
        // Cursor on the `set ::tolComp` declaration (line 4, col 8, inside
        // "tolComp" — `set ::tolComp 0.05` has "tolComp" starting at col 6).
        let refs = references(src, "tcl", 4, 8, &analysis, true);
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
        // Issue #923 idx 68's second repro: an *unqualified* `set tolComp
        // 0.05` at global scope reproduces the identical split, ruling out
        // the `::`-prefix as the sole cause — `handle_set_command` never
        // calls `set_var_link_target` regardless of how the name is spelled,
        // so the gap (and the fix) is the same either way.
        let src =
            "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset tolComp 0.05\nuse\n";
        let analysis = analyse(src);
        // Cursor on the unqualified `set tolComp` declaration (line 4, col 6).
        let refs = references(src, "tcl", 4, 6, &analysis, true);
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
        // FP guard (issue #923 idx 68): the new canonical-cell fold-in must
        // still be exact-qualified-name matched — two unrelated top-level
        // `tolComp` cells living in different namespaces, neither aliasing
        // the other, must never be unioned together just because they share
        // a bare name.
        let src = "namespace eval A {\n    variable tolComp 1\n}\nnamespace eval B {\n    variable tolComp 2\n}\n";
        let analysis = analyse(src);
        // Cursor on `A::tolComp`'s declaration (line 1, col 13).
        let refs = references(src, "tcl", 1, 13, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(
            lines,
            vec![1],
            "an unrelated same-named cell in a different namespace must not be pulled in: {refs:?}"
        );
    }

    #[test]
    fn references_for_a_multi_list_foreach_second_varlist_now_reach_every_use() {
        // Issue #923 idx 70 (main audit wave, high severity, pix corpus):
        // before the `handle_foreach_command` fix, the first loop's own
        // `name` (the second varList of `foreach dirName {...} name {...}
        // {...}`) was never bound at all, so `references()` from *any* use
        // inside the first loop's body fell through to whatever *other*
        // same-named `VarDef` existed anywhere in the flat top-level scope
        // — here, a second, later, textually unrelated `foreach name
        // {...}` — returning only that second loop's own 2 spans and
        // omitting every actual first-loop span, including the query site
        // itself. `foreach` (like `if`/`set`) introduces no new analyser
        // scope (correctly modelling Tcl's lack of block scoping), so at
        // the top level these two loops' `name` genuinely share one global
        // storage cell — same as any two sequential top-level `set name
        // ...` statements — so the fully correct fixed reference set spans
        // *both* loops, not just the first: the bug was under-reporting
        // (missing the first loop's spans entirely), not over-reporting.
        let src = "foreach dirName {src src {src core}} name {alpha beta gamma} {\n    puts \"$dirName $name\"\n    if {$name eq \"pixutils\"} { puts skip }\n}\nforeach name {examples color changes} {\n    puts $name.ruff\n}\n";
        let analysis = analyse(src);
        // Cursor on the first loop's own `$name` read (line 1, col 21) —
        // the exact query shape the finding's own repro used.
        let refs = references(src, "tcl", 1, 21, &analysis, true);
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
        let refs = references(src, "tcl", 2, 9, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(lines, vec![2], "only the declaration itself: {refs:?}");
    }

    #[test]
    fn references_reach_the_call_site_from_a_shadowed_duplicate_proc_decl_same_document() {
        // TN-shaped regression guard — issue #923 idx 31 (main audit wave):
        // this same-document path was already correct before that fix
        // (`resolve_proc_target_at`'s own fallback resolves the word text
        // via ordinary namespace lookup when the direct declaration-span
        // match misses, landing on the current winner regardless of which
        // occurrence's span the cursor sits on); pinned here so the
        // cross-document fix landing alongside it never regresses this
        // already-working case.
        let src = "proc List2array {lst} { return ONE }\nproc List2array {lst} { return TWO }\nList2array x\n";
        let analysis = analyse(src);
        // Cursor on the SHADOWED (first) declaration (line 0, col 6).
        let refs = references(src, "tcl", 0, 6, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&1), "winning decl missing: {refs:?}");
        assert!(lines.contains(&2), "call site missing: {refs:?}");
    }

    #[test]
    fn references_include_the_renames_own_old_word() {
        // TP — issue #923 idx 39 (main audit wave): `rename OLD NEW`'s own
        // `OLD` word is a genuine reference to the proc it names
        // (tclsh9.0/8.6-verified: `rename` requires `OLD` to exist,
        // "can't rename ...: command doesn't exist" otherwise) — the real
        // corpus shape is a tcltest `-setup`/`-body`/`-cleanup` idiom
        // (georgtree_tclopt test/arbitaryTest.tcl:46/113's `proc gaussfunc`
        // / `rename gaussfunc ""`). Go-to-definition/hover already resolved
        // this token (independent cursor-token walk); `references` missed
        // it entirely, so a rename built on the same list left this
        // occurrence pointing at a now-nonexistent command — a previously
        // passing tcltest crashes with "can't delete ...: command doesn't
        // exist" purely from applying the LSP's own rename edit.
        let src = "proc helperFunc {x} { return [expr {$x * 2}] }\nhelperFunc 21\nrename helperFunc \"\"\n";
        let analysis = analyse(src);
        let refs = references(src, "tcl", 0, 6, &analysis, true);
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
        let refs = references(src, "tcl", 0, 6, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert_eq!(lines, vec![0, 1], "{refs:?}");
    }

    #[test]
    fn references_include_unbraced_if_body_bareword_call() {
        // TP — differential-audit finding idx 61 (main audit wave,
        // nico-robert_ticklecharts): `if {$cond} mymod::foo` (an unbraced
        // if-then body — a single, statically-known bareword, valid Tcl
        // and used ~50 times in the real corpus this way) was invisible
        // to `command_invocations` entirely, since `analyse_body` only
        // ever recurses a braced (`Str`-kind) body. `references` from the
        // declaration silently missed it — go-to-definition and hover
        // still found it (they resolve independently off the cursor
        // token), producing a dangerous asymmetry: a `rename` built on
        // this same list would silently miss rewriting the call site.
        let src = "proc foo {} { return 1 }\nif {1} foo\n";
        let analysis = analyse(src);
        let refs = references(src, "tcl", 0, 6, &analysis, true);
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
        let refs = references(src, "tcl", 0, 6, &analysis, true);
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
        // behavior is that `if {1} $cb` (a bare-`$var` `if`-body, evaluated
        // as a script exactly like `eval`/`uplevel`'s bodies) genuinely
        // calls `foo` when `$cb` holds that constant value — printing
        // "CALLED" for a `proc foo {} { puts CALLED }`. Issue #923 idx 94
        // wires up exactly this dispatch (`dispatch_one_body_argument`'s
        // `TokenType::Var` branch, generic across every `ArgRole::Body`
        // argument, not just `eval`/`uplevel`'s), so `foo`'s own reference
        // set correctly grows to include this call site — the failure mode
        // this test now guards is a *literal* `$cb`-named command
        // reference ever appearing, which never happens (there is no such
        // command in `all_procs` to resolve to).
        let src = "proc foo {} { return 1 }\nset cb foo\nif {1} $cb\n";
        let analysis = analyse(src);
        let refs = references(src, "tcl", 0, 6, &analysis, true);
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
    fn ensemble_subcommand_references_include_decl_and_both_call_sites() {
        // TP — issue #923 idx 106: references on an ensemble subcommand call
        // site must return the target proc's declaration plus every call
        // site — proves the automatic pickup via
        // `proc_reference_spans`/`invocation_references_named`'s
        // `resolved_qualified_name` matching, not just a single hardcoded
        // case.
        let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e foo bar]\nputs [e foo baz]\n";
        let analysis = analyse(src);
        // Cursor on "foo" in the first call site (0-based line 7, col 8).
        let refs = references(src, "tcl", 7, 8, &analysis, true);
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

    #[test]
    fn method_references_include_next_dispatch() {
        // `Sub::greet`'s body invokes the super via `next`; that dispatch is a
        // polymorphic reference to `greet` and must appear among its
        // references.
        let src = "oo::class create Base {\n    method greet {} {}\n}\noo::class create Sub {\n    superclass Base\n    method greet {} { next }\n}\n";
        let analysis = analyse(src);
        // Cursor on `greet` in Sub's declaration (line 5).
        let refs = references(src, "tcl", 5, 13, &analysis, true);
        // The `next` token sits on line 5, past the method's own `greet` name
        // (col 11) — inside the `{ next }` body.
        assert!(
            refs.iter()
                .any(|r| r.start_line == 5 && r.start_character > 15),
            "expected the `next` dispatch among refs: {refs:?}",
        );
    }

    // Constructor / destructor next-chain references (issue #992).

    #[test]
    fn constructor_next_chain_reference_from_direct_subclass() {
        // `Sub`'s constructor calls plain `next`, chaining to `Base`'s —
        // cursor on `Base`'s own `constructor` keyword must surface that
        // chain as a reference.
        let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { next }\n}\n";
        let analysis = analyse(src);
        // `constructor` keyword on line 1, col 4.
        let refs = references(src, "tcl", 1, 6, &analysis, true);
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
        let refs = references(src, "tcl", 1, 6, &analysis, false);
        assert!(refs.is_empty(), "{refs:?}");
    }

    #[test]
    fn constructor_next_chain_reference_not_counted_without_next() {
        // `Sub` declares its own constructor but never calls `next` — a
        // legitimate full override, not a chain.
        let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { set x 1 }\n}\n";
        let analysis = analyse(src);
        let refs = references(src, "tcl", 1, 6, &analysis, false);
        assert!(refs.is_empty(), "{refs:?}");
    }

    #[test]
    fn constructor_next_chain_reference_skips_ancestor_with_no_own_constructor() {
        // `Mid` declares no constructor of its own; `Sub`'s `next` must
        // still reach `Base` (the actual MRO-effective provider), not `Mid`.
        let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Mid {\n    superclass Base\n}\noo::class create Sub {\n    superclass Mid\n    constructor {} { next }\n}\n";
        let analysis = analyse(src);
        // `Base`'s constructor (line 1) picks up the chain.
        let base_refs = references(src, "tcl", 1, 6, &analysis, false);
        assert_eq!(base_refs.len(), 1, "{base_refs:?}");
        assert_eq!(base_refs[0].start_line, 8, "{base_refs:?}");
    }

    #[test]
    fn constructor_next_chain_reference_via_nextto_explicit_target() {
        // `nextto Grandparent` jumps past `Base` even though `Base` also has
        // an effective constructor — only `Grandparent` picks up a reference.
        let src = "oo::class create Grandparent {\n    constructor {} { }\n}\noo::class create Base {\n    superclass Grandparent\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { nextto Grandparent }\n}\n";
        let analysis = analyse(src);
        let grandparent_refs = references(src, "tcl", 1, 6, &analysis, false);
        assert_eq!(grandparent_refs.len(), 1, "{grandparent_refs:?}");
        let base_refs = references(src, "tcl", 4, 6, &analysis, false);
        assert!(base_refs.is_empty(), "{base_refs:?}");
    }

    #[test]
    fn constructor_next_chain_reference_via_braced_nextto_target() {
        // Regression (Codex review on #1011, P2): `nextto {Grandparent}` (a
        // braced target, functionally identical to the bare form) must
        // resolve exactly like `constructor_next_chain_reference_via_nextto_explicit_target`'s
        // bare `nextto Grandparent` — the decoded word, not the raw
        // delimited span, is what gets resolved against the class map.
        let src = "oo::class create Grandparent {\n    constructor {} { }\n}\noo::class create Base {\n    superclass Grandparent\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { nextto {Grandparent} }\n}\n";
        let analysis = analyse(src);
        let grandparent_refs = references(src, "tcl", 1, 6, &analysis, false);
        assert_eq!(grandparent_refs.len(), 1, "{grandparent_refs:?}");
        let base_refs = references(src, "tcl", 4, 6, &analysis, false);
        assert!(base_refs.is_empty(), "{base_refs:?}");
    }

    #[test]
    fn destructor_next_chain_reference_from_direct_subclass() {
        let src = "oo::class create Base {\n    destructor { }\n}\noo::class create Sub {\n    superclass Base\n    destructor { next }\n}\n";
        let analysis = analyse(src);
        // `destructor` keyword on line 1, col 4.
        let refs = references(src, "tcl", 1, 5, &analysis, true);
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
        let refs = references(src, "tcl", 1, 6, &analysis, true);
        assert!(refs.is_empty(), "{refs:?}");
    }

    #[test]
    fn constructor_keyword_resolves_its_own_next_chain_even_with_a_same_named_method() {
        // Regression (Codex review on #1011, P2): a class can also declare a
        // `method` literally named `constructor` — an independent, ordinary
        // member sharing a name with the special keyword form. A cursor on
        // the special `constructor` keyword must still resolve its own
        // next-chain story, not fall through to the unrelated same-named
        // method's (ordinary) references.
        let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { next }\n    method constructor {} { }\n}\n";
        let analysis = analyse(src);
        // `Base`'s `constructor` keyword, line 1.
        let refs = references(src, "tcl", 1, 6, &analysis, false);
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
        let refs = references(src, "tcl", 1, 7, &analysis, true);
        // The analyser may or may not record the literal `$x`
        // as a reference depending on lowering; at minimum the
        // declaration should land in the result list.
        assert!(!refs.is_empty(), "{refs:?}");
        assert!(refs.iter().any(|r| r.start_line == 0));
    }

    #[test]
    fn references_from_proc_param_bareword_declaration_include_every_use() {
        // TP — differential-audit finding idx 9 (main audit wave): a cursor
        // on a proc parameter's own bareword name (not a `$`-prefixed
        // read) previously returned zero references, even though the same
        // query from any `$name` read resolved the full set.
        let src = "proc greet {name} { return $name }\ngreet hi\n";
        let analysis = analyse(src);
        // Cursor on `name` inside the parameter list (col 12-16).
        let refs = references(src, "tcl", 0, 13, &analysis, true);
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
        let src = "proc resolveSwitch {name def} {\n    catch {foo} name\n    return $name\n}\n";
        let analysis = analyse(src);
        // Cursor on the catch result-var `name` (line 1, col 16-20).
        let refs = references(src, "tcl", 1, 17, &analysis, true);
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
                link_target_span: None,
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
    fn idx63_two_block_class_my_dispatch_already_fixed_by_idx52() {
        // Issue #923 idx 63 (main audit wave, high severity): the finding's
        // own primary, corpus-verified claim — "go-to-definition AND
        // find-references both return zero results" for a `my
        // methodName` call when the class is created via `oo::class
        // create` with no body and every method (including the call site
        // itself) is added via a *separate*, later `oo::define ClassName
        // { ... }` block (the finding's own minimal repro shape, matching
        // the real corpus's `ticklecharts::chart`). This is the exact
        // root cause idx 52 already fixed (`class_body_spans` /
        // `enclosing_class_at`) — verified here independently, pinned as
        // a permanent regression using idx 63's own repro shape. No
        // production changes in this commit for this part of the finding.
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
        let refs = references(src, "tcl", 5, 11, &analysis, true);
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
        let refs = references(src, "tcl", 1, 11, &analysis, true);
        assert!(refs.len() >= 3, "expected ≥3 refs; got {refs:?}");
    }

    #[test]
    fn references_for_method_reach_a_my_dispatch_call_inside_a_switch_arm() {
        // Issue #923 idx 63 (main audit wave, high severity): a `my
        // methodName` call written inside a `switch` arm body is a
        // genuine, statically-known call site (tclsh9.0/8.6-verified) —
        // the real corpus shape (`ticklecharts::chart`'s `Add` dispatcher:
        // `switch ... { barSeries { my AddBarSeries {*}$args } ... }`).
        // `scan_my_method_region`'s `[...]`-substitution recursion never
        // reaches a switch arm's braced body (it isn't a command
        // substitution), so this was invisible to find-references even
        // though go-to-definition (an independent cursor-token walk)
        // already resolved it.
        let src = "oo::class create widget {\n    method bar {} { return \"bar-value\" }\n    method dispatch {args} {\n        switch -exact -- [lindex $args 0] {\n            bar { my bar {*}[lrange $args 1 end] }\n        }\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `bar` declaration (line 1, col 11).
        let refs = references(src, "tcl", 1, 11, &analysis, true);
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

    #[test]
    fn references_from_decl_reach_my_dispatch_when_class_extended_via_separate_oo_define() {
        // Issue #923 idx 52 (main audit wave, high severity): `Gadget` is
        // created via `oo::class create` with no body; every method
        // (including the `my Helper` call site) is added via a *separate*,
        // later `oo::define Gadget { ... }` block — the real corpus shape
        // (`ticklecharts::chart`). References from the `Helper` declaration
        // must reach the `my Helper` call site living in that separate
        // block, not silently return nothing.
        let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n";
        let analysis = analyse(src);
        // Cursor on the `Helper` declaration (line 4, col 11).
        let refs = references(src, "tcl", 4, 11, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&4), "decl missing: {refs:?}");
        assert!(
            lines.contains(&5),
            "the my Helper call site inside the separate oo::define block is missing: {refs:?}"
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
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Dog", "bark", false);
        // Two external sites: `$d bark` and `[$d bark]`.
        assert_eq!(sites.len(), 2, "{sites:?}");
    }

    #[test]
    fn find_obj_method_call_sites_finds_calls_in_proc_body() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nproc f {} { $d bark }\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Dog", "bark", false);
        assert_eq!(sites.len(), 1, "{sites:?}");
    }

    #[test]
    fn find_obj_method_call_sites_matches_bare_created_instance_command() {
        // `Dog create rex` binds `rex` as an object *command*; `rex bark` is a
        // bare-word method dispatch (not `$rex bark`).  The scan resolves it
        // through `created_instance_commands`.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Dog", "bark", false);
        assert_eq!(sites.len(), 1, "{sites:?}");
        // The matched span is the `bark` method-name token of `rex bark`.
        let s = sites[0];
        assert_eq!(
            &src[s.start() as usize..s.end() as usize],
            "bark",
            "{sites:?}"
        );
    }

    // class-command dispatch (issue #923 idx 120): `CLASS method` for a
    // classmethod / `self method`, a receiver set entirely separate from
    // `$obj method` / `NAME method` instance dispatch above.

    #[test]
    fn find_obj_method_call_sites_matches_class_command_and_inheriting_subclass() {
        // TP — both the finding's own repro (`ActiveRecord find`) and its
        // inherited-via-superclass sibling (`Table find`, ooutil's
        // `classmethod` propagates to a subclass's own bound command).
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\nActiveRecord find foo bar\n";
        let analysis = analyse(src);
        let sites =
            find_obj_method_call_sites(src, "tcl", &analysis, "::ActiveRecord", "find", true);
        assert_eq!(sites.len(), 2, "{sites:?}");
        for s in &sites {
            assert_eq!(
                &src[s.start() as usize..s.end() as usize],
                "find",
                "{sites:?}"
            );
        }
    }

    /// TP (adversarial review of #1047, item 2): a bare class-command
    /// dispatch written inside an `apply` lambda body or a `namespace eval`
    /// body — at the top level or nested inside a method — is a real call.
    /// All three shapes were confirmed dispatching under tclsh 9.0.4
    /// (`MAKE CALLED` printed three times).
    #[test]
    fn find_obj_method_call_sites_reaches_lambda_and_namespace_eval_bodies() {
        let src = "oo::class create Factory {\n\
                       classmethod make {} { return 1 }\n\
                       method inst {} { apply {{} { Factory make }} }\n\
                       method nsev {} { namespace eval ::zz { Factory make } }\n\
                   }\n\
                   namespace eval ::top2 { Factory make }\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Factory", "make", true);
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
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Dog", "bark", false);
        assert_eq!(sites.len(), 1, "{sites:?}");
        let s = sites[0];
        let line = src[..s.start() as usize].lines().count();
        assert_eq!(line, 7, "only the same-frame `$f bark` matches: {sites:?}");
    }

    /// TN — a bare (backslash-escaped) `apply` body element is decoded
    /// before `apply` evaluates it, so its source slice is not the script
    /// that runs; the scan must not re-parse it in place (Codex review on
    /// #1047).
    #[test]
    fn find_obj_method_call_sites_skips_escaped_lambda_body_element() {
        let src = "oo::class create Factory {\n\
                       classmethod make {} { return 1 }\n\
                   }\n\
                   apply {{} Factory\\ make}\n";
        let analysis = analyse(src);
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Factory", "make", true);
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
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Widget", "make", true);
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
        let analysis = analyse(src);
        // Cursor on the declaration (line 1, `find` at col 16).
        let refs = references(src, "tcl", 1, 16, &analysis, true);
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
        let analysis = analyse(src);
        // Cursor on `find` in `ActiveRecord find foo bar` (line 3, col 13).
        let refs = references(src, "tcl", 3, 13, &analysis, true);
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
        let sites = find_obj_method_call_sites(src, "tcl", &analysis, "::Dog", "bark", false);
        assert!(
            sites.is_empty(),
            "bare var receiver wrongly matched: {sites:?}"
        );
    }

    // tcl::OptProc — the `opt` package's automatic-option-parsing proc
    // definer (issue #923 idx 90): the missing analyser hook previously left
    // the call site unreachable from the declaration.

    #[test]
    fn references_from_opt_proc_declaration_reach_the_call_site() {
        let src = "::tcl::OptProc greet {child -use -display} { return $child }\ngreet foo\n";
        let analysis = analyse(src);
        // Line 0 — cursor on "greet" right after `::tcl::OptProc` (col 15).
        let refs = references(src, "tcl", 0, 15, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {refs:?}");
        assert!(lines.contains(&1), "call site missing: {refs:?}");
    }

    #[test]
    fn references_reach_a_proc_dispatched_through_an_eval_of_a_list_computed_var() {
        // Issue #923 idx 94: the finding's own minimal repro — `eval $cmdD`
        // where `$cmdD` is built via `[list greetD World]` — previously
        // returned only the declaration; the call site living inside
        // `eval $cmdD` was invisible.
        let src = "proc greetD {n} {puts \"D $n\"}\nset cmdD [list greetD World]\neval $cmdD\n";
        let analysis = analyse(src);
        // Line 0 — cursor on `greetD`'s declaration name (col 6).
        let refs = references(src, "tcl", 0, 6, &analysis, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        assert!(lines.contains(&0), "decl missing: {refs:?}");
        assert!(
            lines.contains(&1),
            "the `greetD` word inside `[list greetD World]` must be reachable too: {refs:?}"
        );
    }
}
