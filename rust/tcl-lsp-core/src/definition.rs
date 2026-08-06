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

//! Definition / declaration / type-definition / implementation
//! provider.
//!
//! The four LSP methods all answer the same fundamental question
//! ("where is the symbol at this position defined?") with slightly
//! different priorities for proc / class / variable matches.  The
//! four share the same core function and the server lifts each
//! method onto it.
//!
//! Resolved here:
//!
//! * `$var` references resolve to the `definition_span` of the
//!   matching `VarDef` in the global scope.  Scope-chain
//!   descent is not done — the analyser's body-span line index
//!   isn't currently threaded into the search path.
//! * Bare-word references resolve to a user-defined `proc` or
//!   `TclOO` class via `name_span`.  Proc resolution follows C
//!   Tcl's command lookup (`Tcl_FindCommand`, `tclNamesp.c`):
//!   the caller's namespace first, then the global namespace;
//!   an absolute `::`-prefixed word resolves exactly.  When no
//!   candidate is defined and the word doesn't name a registry
//!   builtin, a deterministic tail match keeps the lenient
//!   behaviour for procs whose defining namespace isn't
//!   statically visible at the call.
//!
//! Command-table indirection: Tcl's command table is mutable, so a
//! word's spelling does not by itself name the definition the call
//! reaches. When `analysis.renamed_commands` / `analysis.command_aliases`
//! record a `rename OLD NEW` or an `interp alias {} ALIAS {} TARGET`
//! that is **in effect at the cursor**, the provider follows the chain
//! (via the shared walk in `tcl_compiler::analyser::indirection`, which
//! the analyser's own constructor typing uses) and resolves the terminal
//! name from the global namespace — where an alias target and a rename
//! source are both looked up when the binding fires. This is asked
//! *before* the ordinary call resolution, because an alias silently
//! replaces a same-named command. It is order-gated: a call written
//! before the mutation resolves the ordinary way, matching tclsh, where
//! it is `invalid command name`. An argument-prepending alias is
//! declined — it is not the call the target would receive.
//!
//! Redefinition: a proc declared twice in one document is two
//! definitions of one command. `all_procs` keeps the last (plain Tcl's
//! own rule) and `AnalysisResult::superseded_procs` keeps the rest, so a
//! call between the two resolves to the definition in effect there and a
//! cursor on either header stays on that header.
//!
//! Class-member lookup: when the cursor sits on a
//! word inside a class body span, the provider walks that
//! class's `methods` / `class_methods` / `properties` /
//! `constructors` / `destructor` looking for a name match
//! and jumps to the member's `name_span`.  Catches `my
//! method` calls inside the body and bare references to the
//! class's own members.
//!
//! `$obj method` dispatch: when the cursor sits on
//! the method-name token of a `$obj method` / `[$obj method]`
//! call and `$obj`'s class is known (recorded in
//! `analysis.instance_classes` from a `set obj [Cls new]` /
//! `Cls create obj` site), the provider jumps to the method
//! declaration on that class.
//!
//! `apply` namespace override (issue #923 idx 116): a bareword call
//! inside `apply {{params} body ns}`'s body resolves against `ns`, not
//! wherever the `apply` call is lexically written — `namespace_context_at`
//! / `innermost_namespace_at` consult `analysis.namespace_overrides`
//! (populated by `Analyser::handle_apply_command`) ahead of the ordinary
//! lexical scope-chain walk. Also resolves one hop through a `$var` or
//! `[list {params} $body ns]` indirection (`set lambda {...}; apply
//! $lambda`), bounded to that one hop by design.
//!
//! Limitations:
//!
//! * Flow-sensitive / scope-aware instance-class tracking —
//!   `analysis.instance_classes` is a best-effort global
//!   var-name → class map (last assignment wins).  Re-binding
//!   the same name to a different class, or two locals of the
//!   same name in different procs, isn't disambiguated.
//! * `BigIP` definition — a separate provider keyed off iRules
//!   dialect that resolves pool / data-group / iRule /
//!   virtual-server names against a parsed `bigip.conf` — is
//!   not implemented here.
//! * `apply` reached only through a registry `command_prefixes` slot
//!   (`coroutine co ::apply $lambda`) is not modelled — that slot is
//!   consulted today only for IR-level interprocedural call-graph
//!   reachability, never re-dispatched through `AnalyserHookId::Apply`.
//!   Nor is a proc that re-injects its own arguments as a script via a
//!   captured `uplevel`-namespace + trace/callback (tcllib generator.tcl's
//!   `finally`, the exact idiom issue #923 idx 116's confirmed finding
//!   traces through) — there is no static/lexical connection between such
//!   a call and the token it eventually invokes, so this is a considered,
//!   permanent limitation rather than an oversight. Deeper `$var`-to-`$var`
//!   indirection beyond one hop (`set a $lambda; apply $a`) is the same
//!   kind of deliberate, bounded gap, not a special case of this one.

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::indirection;
use tcl_lexer::{LineIndex, Utf16Col};

use crate::hover::{find_var_at_position, find_word_span_at_position};
use crate::namespace_import::{ExportVerdict, NamespaceExportOracle};
use crate::source_graph::{RunOrder, RunPoint};

/// Whole-program export knowledge, paired with the URI of the document being
/// resolved.
///
/// The two travel together because neither is usable alone: the oracle ranks
/// the workspace's `namespace export` events against the *importing* statement,
/// and that statement is a byte offset in a named document. Handed the wrong
/// URI — or the placeholder one the document-only tier uses — an export
/// written **after** the import in this very file becomes unrankable, which
/// [`crate::namespace_import::exported_at_import_site`] reads as "in effect",
/// silently reversing issue #1027's Direction B. Pairing them structurally
/// makes that unrepresentable.
#[derive(Clone, Copy)]
pub struct ProgramExports<'a> {
    /// The document being resolved, spelled as the workspace index knows it.
    pub uri: &'a str,
    /// The workspace's export records — typically
    /// `crate::workspace_index::WorkspaceIndex::export_snapshot`.
    pub oracle: &'a dyn NamespaceExportOracle,
}

/// Everything a call-site resolution consults **besides the document in front
/// of it** — the context [`resolve_called_proc`] and its helpers carry.
///
/// Two facts, both optional, both meaning "keep the document-only answer" when
/// absent:
///
/// * [`registry`](Self::registry) — the dialect's command registry, the
///   builtin gate. A caller without one keeps the lenient behaviour; there is
///   no builtin to protect.
/// * [`program`](Self::program) — the whole-program export oracle. A caller
///   without one (a single-document unit test, the `tcl` CLI, a buffer the
///   workspace has not indexed) gets exactly the behaviour this tier had
///   before issue #1116 item 1: an in-document export record decides, and its
///   absence is read as evidence only where this document holds *some* export
///   for the namespace.
///
/// One context rather than two more parameters at each of the fourteen
/// [`resolve_called_proc`] call sites, and rather than a global: the facts are
/// per-request, and a global would make a single-document unit test and a
/// live server disagree about what "no workspace" means.
#[derive(Clone, Copy, Default)]
pub struct CallResolution<'a> {
    /// The dialect's command registry, when the caller has one.
    pub registry: Option<&'a tcl_registry::CommandRegistry>,
    /// Whole-program export knowledge, when the caller has a workspace index.
    pub program: Option<ProgramExports<'a>>,
}

impl<'a> CallResolution<'a> {
    /// The document-only context: no registry, no oracle.
    #[must_use]
    pub const fn document_only() -> Self {
        Self {
            registry: None,
            program: None,
        }
    }

    /// This context with `registry` installed — how the providers add the
    /// builtin gate to whatever whole-program view their caller handed them.
    #[must_use]
    pub const fn with_registry(self, registry: &'a tcl_registry::CommandRegistry) -> Self {
        Self {
            registry: Some(registry),
            ..self
        }
    }

    /// This context with whole-program export knowledge installed.
    #[must_use]
    pub const fn in_program(self, program: ProgramExports<'a>) -> Self {
        Self {
            program: Some(program),
            ..self
        }
    }

    /// The URI to stamp on this document's own run points — the real one when
    /// a workspace view is attached, else the placeholder
    /// [`IN_DOCUMENT_URI`].
    const fn uri(self) -> &'a str {
        match self.program {
            Some(p) => p.uri,
            None => IN_DOCUMENT_URI,
        }
    }
}

/// LSP `Range` analogue — line/character pairs in UTF-16 code
/// units per the LSP spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LspRange {
    /// Start position (0-based inclusive).
    pub start_line: u32,
    /// Start character (0-based UTF-16 code units).
    pub start_character: u32,
    /// End position (0-based exclusive).
    pub end_line: u32,
    /// End character (0-based, exclusive).
    pub end_character: u32,
}

pub(crate) fn utf16_col_to_char_col(line_text: &str, character: u32) -> usize {
    let mut utf16 = 0u32;
    for (idx, ch) in line_text.chars().enumerate() {
        if utf16 >= character {
            return idx;
        }
        utf16 = utf16.saturating_add(u32::try_from(ch.len_utf16()).expect("len_utf16 fits u32"));
    }
    line_text.chars().count()
}

pub(crate) fn utf16_len(text: &str) -> u32 {
    u32::try_from(text.encode_utf16().count()).expect("UTF-16 length fits u32")
}

/// Compute "go-to-definition" locations for the symbol at the
/// cursor.
///
/// Returns an empty vector when no recognisable symbol is at
/// the position or when the symbol's definition isn't in the
/// current document.
///
/// Document-only: equivalent to [`definition_with`] with no whole-program
/// view. A host that has a workspace index should call that instead — a
/// `namespace import -force` whose export lives in another file is invisible
/// here (see [`NamespaceExportOracle`]).
#[must_use]
pub fn definition(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<LspRange> {
    definition_with(source, line, character, analysis, None)
}

/// [`definition`] with the caller's whole-program export view attached.
///
/// `program` is `None` for a host with no workspace index (a single-document
/// test, the `tcl` CLI), which reproduces [`definition`] exactly.
#[must_use]
pub fn definition_with(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    program: Option<ProgramExports<'_>>,
) -> Vec<LspRange> {
    let view = CallResolution {
        registry: None,
        program,
    };
    let line_index = LineIndex::new(source);

    let decl_byte_offset = byte_offset_at(&line_index, source, line, character);
    // Steps 1a-1d: everything decided by the cursor's *position* rather than
    // by resolving a bareword — a `$var` read, a bareword declaration, an
    // `expr` math-function call, and the two positions that are pure data.
    // `Some` is definitive (including `Some(vec![])`).
    if let Some(result) = position_definition(
        source,
        &line_index,
        decl_byte_offset,
        DefCtx {
            line,
            character,
            analysis,
            resolution: view,
        },
    ) {
        return result;
    }

    // 2. Bare word — proc, class, class-member, or alias.
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    // `$obj method` / `[$obj method]` — when the cursor sits on
    // the method-name token of an instance-method call and the
    // instance variable's class is known, jump to the method
    // declaration.  Checked before the proc lookup so a method
    // call resolves to the method even when a same-named proc
    // exists.
    if let Some(result) = instance_method_definition(analysis, source, &line_index, line, character)
    {
        return result;
    }
    // `<ensemble> <subcommand>` — a static `namespace ensemble create
    // -map`/`-subcommands` mapping (issue #923 idx 106).  Checked before the
    // generic "otherwise it is a CALL" resolution below: real Tcl never
    // independently looks up `make` as a command (only the pair `widget
    // make` dispatches), so a coincidental same-named proc elsewhere in the
    // workspace must never win.
    if let Some((head, sub, false)) = instance_method_at_cursor(source, line, character) {
        let cursor_offset = byte_offset_at(&line_index, source, line, character);
        let namespace = namespace_context_at(
            &analysis.global_scope,
            cursor_offset,
            &analysis.namespace_overrides,
        );
        if let Some(target) = ensemble_subcommand_target(analysis, &namespace, &head, &sub)
            && let Some(proc_def) = resolve_called_proc(
                analysis,
                source,
                "::",
                target,
                cursor_offset,
                view.with_registry(tcl_registry::registry_for_dialect(&analysis.dialect)),
            )
        {
            return vec![span_to_range(source, &line_index, proc_def.name_span)];
        }
    }
    // `next` / `nextto` inside a method body — jump to the super-method in
    // the MRO chain that the enclosing method overrides (`next`), or to the
    // named class's copy of it (`nextto Cls`).
    if is_next_chain_keyword_in(&analysis.dialect, &word)
        && let Some(span) =
            next_dispatch_target(analysis, source, &line_index, line, character, &word)
    {
        return vec![span_to_range(source, &line_index, span)];
    }
    // Prefer the declaration whose own name span covers the cursor (so a
    // same-named proc in another namespace's own decl resolves to *that*
    // one — mirrors `references::proc_references` / `rename::rename_proc`;
    // #924).  `proc_declaration_sites` rather than `all_procs`, so a
    // declaration a later same-named `proc` displaced still resolves to
    // itself instead of silently jumping to its own successor (issue #923
    // idx 45) — the map only ever keeps the winner's span.
    let cursor_offset = byte_offset_at(&line_index, source, line, character);
    if let Some((_, span)) = analysis
        .proc_declaration_sites
        .iter()
        .find(|(_, span)| span.start() <= cursor_offset && cursor_offset < span.end())
    {
        return vec![span_to_range(source, &line_index, *span)];
    }
    // An **already-resolved indirect head** the analyser settled — a
    // `${ns}::setdef` whose `$ns` is a constant, a constant `$cmd` dispatch —
    // is asked before every name-based path below: the span does not carry
    // the written command name, so the bareword lookup can only ever answer
    // with a coincidentally same-named decoy (issue #1133).
    if let Some(span) = resolved_indirect_head_target(analysis, cursor_offset) {
        return vec![span_to_range(source, &line_index, span)];
    }
    // Otherwise it is a CALL — resolve namespace-aware, following C Tcl's
    // command resolution (`Tcl_FindCommand`, `tclNamesp.c`): the caller's
    // namespace first, then the global namespace; an absolute `::`-prefixed
    // word resolves exactly. A word no candidate defines falls back — unless
    // it names a registry builtin, which is what the call actually reaches —
    // to the lenient tail match for procs whose defining namespace isn't
    // statically visible, resolved deterministically (see
    // [`resolve_called_proc`]).
    let namespace = namespace_context_at(
        &analysis.global_scope,
        cursor_offset,
        &analysis.namespace_overrides,
    );
    // The command table may have been *mutated* before this call runs: a
    // `rename OLD NEW` moved a definition onto this word, or an `interp
    // alias` installed (or silently replaced) it. That is what the call
    // actually reaches, so it is asked ahead of the ordinary resolution
    // below — which would otherwise answer with a same-named `proc` the
    // alias has already displaced (issue #923 idx 89; tclsh 8.6.16/9.0.4:
    // after `interp alias {} ::ttk::spinbox {} ::tk::spinbox`, calling
    // `::ttk::spinbox` runs `::tk::spinbox`'s body, and the original proc is
    // unreachable under that name). The hop is order-gated, so a rename or
    // alias written *after* this call site does not apply and the ordinary
    // resolution below still wins (tclsh: a `hello` before `rename greet
    // hello` is `invalid command name "hello"`).
    if let Some(span) = indirect_definition_target(analysis, view, source, cursor_offset, &word) {
        return vec![span_to_range(source, &line_index, span)];
    }
    // `Factory::make` — [incr Tcl]'s colon-qualified class-proc dispatch
    // (issue #990).  Asked **before** the stock proc resolution below, because
    // itcl installs a custom command resolver on every class namespace and Tcl
    // consults it ahead of the ordinary lookup: inside a class's own bodies the
    // class proc wins even when a real `::Factory::make` proc exists (oracle in
    // `itcl_self_qualified_candidate`).  Order is safe because
    // `itcl_class_proc_target` applies the whole precedence itself — its own
    // resolver first, then stock resolution — and answers `None` unless the
    // winning candidate really is an itcl class-scoped `proc`, so a genuine
    // `::ns::helper` call still falls through to `resolve_called_proc`.
    if let Some(span) = itcl_class_proc_declaration(analysis, &namespace, &word) {
        return vec![span_to_range(source, &line_index, span)];
    }
    // …and only when the cursor's word really is the enclosing command's
    // head. Without this the pure text scan let an ordinary *argument*
    // resolve as a command name (issue #1137 idx 50) — see
    // [`offset_is_command_head`] for the gate and its one documented limit.
    if offset_is_command_head(analysis, cursor_offset)
        && let Some(proc_def) = resolve_called_proc(
            analysis,
            source,
            &namespace,
            &word,
            cursor_offset,
            view.with_registry(tcl_registry::registry_for_dialect(&analysis.dialect)),
        )
    {
        // A proc redefined later in the document is two definitions sharing
        // one name; the call reaches whichever was in effect where it is
        // written, not unconditionally the last (issue #923 idx 45).
        let in_effect = analysis
            .proc_def_in_effect_at(&proc_def.qualified_name, cursor_offset)
            .unwrap_or(proc_def);
        return vec![span_to_range(source, &line_index, in_effect.name_span)];
    }
    if let Some(span) = class_declaration_at(analysis, &word, cursor_offset) {
        return vec![span_to_range(source, &line_index, span)];
    }
    // Class-member lookup — when the cursor sits inside a
    // class body, walk that class's methods / properties /
    // constructors / destructor for a name match.  Covers
    // `my method` calls inside the body plus bare member
    // references.
    if let Some(span) = lookup_class_member(analysis, &word, cursor_offset) {
        return vec![span_to_range(source, &line_index, span)];
    }
    Vec::new()
}

/// The declaration span a word reaches through the document's command-table
/// mutations — `rename OLD NEW` and `interp alias {} ALIAS {} TARGET` — or
/// `None` when no in-effect indirection covers it.
///
/// One tier for both hop kinds and both target kinds, so nothing that
/// navigates has to know which mutation produced the binding.  The chain
/// itself is resolved by [`command_indirection`]; the terminal name is then
/// resolved as an ordinary command would be: as a user `proc` (from the
/// global namespace, where an alias target and a rename source are both
/// looked up when the binding fires) and then as a class.  A chain ending on a
/// registry builtin (`interp alias {} mycmd {} puts`) has no Tcl source to
/// jump to, so it correctly answers nothing rather than guessing.
///
/// The terminal *name* is not by itself the answer when that name has been
/// redeclared: a `rename` hands over the command **object**, so the definition
/// the chain captured is the one in effect at the hop's own as-of time
/// (`Indirection::resolve_at`), not whichever declaration ends up winning the
/// name.  Oracle (tclsh 8.6.14/9.0.4): `proc p {} {return first}; rename p
/// oldp; proc p {} {return second}` leaves `oldp` running `first` — so `oldp`
/// jumps to the *first* header while `p` jumps to the second (PR #1075
/// review, P2).
fn indirect_definition_target(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    source: &str,
    cursor_off: u32,
    word: &str,
) -> Option<tcl_lexer::Span> {
    let hop = command_indirection(analysis, word, cursor_off)?;
    let registry = tcl_registry::registry_for_dialect(&analysis.dialect);
    if let Some(proc_def) = resolve_called_proc(
        analysis,
        source,
        "::",
        &hop.target,
        cursor_off,
        ctx.with_registry(registry),
    ) {
        let captured = analysis
            .proc_def_in_effect_at(&proc_def.qualified_name, hop.resolve_at)
            .unwrap_or(proc_def);
        return Some(captured.name_span);
    }
    class_declaration_at(analysis, &hop.target, cursor_off)
}

/// The cursor context [`position_definition`] needs beyond the offset.
#[derive(Clone, Copy)]
struct DefCtx<'a> {
    line: u32,
    character: u32,
    analysis: &'a AnalysisResult,
    /// The request's whole-program view, carried this far because the
    /// caller-frame path resolves *call sites* to find the callee that
    /// `upvar`s the name — a resolution the `-force` shadow can change
    /// (issue #1116 item 1). Hover already threads it; dropping it here is
    /// what made the two providers disagree on one cursor.
    resolution: CallResolution<'a>,
}

/// The go-to-definition answers decided by the cursor's *position* alone,
/// before any bareword command resolution.  `Some` is definitive — including
/// `Some(vec![])`, which means "this position is not a reference to anything,
/// stop looking".
///
/// In order:
///
/// 1. **A `$var` read** — the scope chain inward from the global scope to the
///    innermost scope whose body span contains the offset, then outward
///    looking for the name; gated on the occurrence being one Tcl actually
///    substitutes ([`lookup_var_read_at`], issue #923 idx 24).
/// 2. **A bareword variable declaration / same-cell write site** — a `set x` /
///    `variable x` target, a proc/method parameter, a `catch` result-var. See
///    [`var_def_at_declaration_offset`] for why this can't reuse the ordinary
///    scope-chain walk (issue #923 idx 9).
/// 3. **An `expr` math-function call** — inside an expression a `NAME(` word is
///    a function-call production resolved through `{ns}::tcl::mathfunc::NAME`
///    then `::tcl::mathfunc::NAME`, not an ordinary command lookup (issue #923
///    idx 30 / issue #974). Asked before the bareword paths because the
///    word-span scan would hand them `sin(1.0)` / `li(` — the function-call
///    syntax glued on — and because a coincidentally same-named proc elsewhere
///    must never win. A built-in resolves to no Tcl source, so an empty result
///    is the right answer there.
/// 4. **A word inside an enclosing proc/method's own parameter list** — a
///    parameter name (answered at 2) or a default value, never a command
///    reference (issue #923 idx 104).
fn position_definition(
    source: &str,
    line_index: &LineIndex,
    cursor_off: u32,
    ctx: DefCtx<'_>,
) -> Option<Vec<LspRange>> {
    let DefCtx {
        line,
        character,
        analysis,
        resolution,
    } = ctx;
    // Resolved through the shared gate, not the raw character scan: a cursor
    // inside a brace-quoted variable-name word (`set {$n} 1`) is not a `$n`
    // reference, and must fall through to the declaration-span search below
    // so it answers the *literal* cell (PR #1106 review, P2).
    if let Some(var_name) = substituting_var_at_position(source, "", line, character, cursor_off) {
        if let Some(var_def) = lookup_var_read_at(
            &analysis.global_scope,
            source,
            "",
            cursor_off,
            &var_name,
            analysis.ns_var_global_fallback(),
        ) {
            return Some(vec![span_to_range(
                source,
                line_index,
                var_def.definition_span,
            )]);
        }
        // Nothing in this frame assigns it — a callee may create it here
        // through `upvar`, and then the call-site word that names it *is*
        // the creating write, the nearest thing the frame has to a
        // declaration (issue #923 audit idx 58).
        return Some(caller_frame_definition(
            source, line_index, analysis, resolution, cursor_off, &var_name,
        ));
    }
    let param_position = parameter_list_position_at(analysis, source, cursor_off);
    // No `Computed` guard is needed here any more: since #1079 a computed
    // parameter list registers no per-parameter `VarDef` at all, so there is
    // no stub named `"[makeargs]"` for this lookup to land on. (#1073 added
    // the guard while the stub still existed.)
    if let Some(var_def) = var_def_at_declaration_offset(&analysis.global_scope, cursor_off) {
        return Some(vec![span_to_range(
            source,
            line_index,
            var_def.definition_span,
        )]);
    }
    // A word naming a namespace (`namespace children ::tomato`, and the
    // `::tomato` of `namespace eval ::tomato { … }` itself) — issue #1088.
    // Definitive: the position is inside a registry-declared
    // `ArgRole::NamespaceName` word, so it is about a namespace and nothing
    // else.  Every declaring `namespace eval` block answers, in source order,
    // because reopening a namespace extends the same one (tclsh 9.0.4 /
    // 8.6.16).  An empty answer means this document declares it nowhere —
    // the cross-document tier picks it up from there.
    if let Some(cell) =
        crate::namespace_symbol::namespace_cell_at_offset(source, analysis, cursor_off)
    {
        let mut spans = crate::namespace_symbol::namespace_declaration_spans(analysis, &cell);
        if spans.is_empty() {
            // Nothing declares it outright, but a deeper block may create it
            // as a parent (`namespace eval ::p::q::r {}` really creates
            // `::p::q`).  The answer is that word's covering prefix — the
            // sub-range spelling exactly this namespace — never the whole
            // word, which names a different one (issue #1113 item 1).
            spans =
                crate::namespace_symbol::namespace_implicit_parent_spans(source, analysis, &cell);
        }
        return Some(
            spans
                .into_iter()
                .map(|span| span_to_range(source, line_index, span))
                .collect(),
        );
    }
    if let Some(inv) = crate::expr_context::mathfunc_call_at(analysis, cursor_off) {
        return Some(
            inv.resolved_qualified_name
                .as_deref()
                .and_then(|resolved| analysis.all_procs.get(resolved))
                .map(|proc_def| vec![span_to_range(source, line_index, proc_def.name_span)])
                .unwrap_or_default(),
        );
    }
    (param_position == ParamListPosition::LiteralData).then(Vec::new)
}

/// Go-to-definition target for a **caller-frame** variable read: the call-site
/// word whose callee creates the variable here through `upvar`.
///
/// Empty when no call site binds the name — the `$`-led read then abstains
/// rather than falling through to bareword resolution, which is what made an
/// unrelated same-named method look like the answer (issue #923 audit idx 58).
fn caller_frame_definition(
    source: &str,
    line_index: &LineIndex,
    analysis: &AnalysisResult,
    resolution: CallResolution<'_>,
    cursor_off: u32,
    name: &str,
) -> Vec<LspRange> {
    let bindings = crate::caller_frame::caller_frame_bindings(
        analysis,
        source,
        "",
        resolution.with_registry(tcl_registry::registry_for_dialect("")),
        cursor_off,
        name,
    );
    crate::caller_frame::primary_binding(&bindings)
        .map(|binding| vec![span_to_range(source, line_index, binding.arg_span)])
        .unwrap_or_default()
}

/// Go-to-definition for an instance-method call site (`$obj method` /
/// `[$obj method]` / `my method`).  Extracted from [`definition`] to keep
/// that function within the line budget.
///
/// `None` means the cursor isn't on an instance-method call at all — the
/// caller falls through to the remaining checks (proc / class / alias).
/// `Some` is definitive: once the receiver's per-object override or class
/// is resolved, the call reaches exactly what this returns (possibly an
/// empty vector for an unexported/undefined method — issue #945 faults 4
/// + 5 + 6), never a same-named proc.
fn instance_method_definition(
    analysis: &AnalysisResult,
    source: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
) -> Option<Vec<LspRange>> {
    let (inst, method, is_dollar) = instance_method_at_cursor(source, line, character)?;
    // The folded per-object member state for this receiver binding, when
    // `oo::objdefine` gave it one (issue #1170) — carries the per-object
    // visibility flips the throwaway walk used to drop.
    let object_state = object_member_state_at(analysis, source, &inst, line, character);
    // A per-object method (`oo::objdefine $obj { method m … }`) is layered
    // ahead of the object's class methods, so resolve it first — `$obj m`
    // must reach the per-object override, not a same-named class method.
    // Binding-identity keyed (issue #945 fault 5): the lookup honours the
    // receiver's scope, not just its textual tail.
    if let Some(span) = lookup_object_method(analysis, source, &inst, &method, line, character) {
        // An unexported per-object member masks the name for an external
        // dispatch outright — oracle, tclsh 9.0.4 / 8.6.14: after
        // `oo::objdefine $o { method M {} {…} }` (name-rule unexported) or
        // `… { method m {} {…}; unexport m }`, `$o M` / `$o m` answer
        // `unknown method`, the class chain notwithstanding (issue #1170).
        if let Some(st) = object_state
            && let Some(md) = st.methods.get(&method)
            && md.visibility != "public"
        {
            return Some(Vec::new());
        }
        return Some(vec![span_to_range(source, line_index, span)]);
    }
    // `my m` — an internal call: dispatch starts at the *enclosing*
    // class and reaches unexported methods too (issue #945 fault 4).
    if is_self_dispatch_keyword(&inst) {
        let cursor = byte_offset_at(line_index, source, line, character);
        if let Some(class_q) = enclosing_class_at(analysis, cursor) {
            return Some(method_dispatch_definition(
                analysis,
                source,
                line_index,
                class_q,
                &method,
                false,
                MethodBucket::Instance,
            ));
        }
    }
    let cursor = byte_offset_at(line_index, source, line, character);
    let class_q = receiver_instance_class_at(analysis, &inst, is_dollar, cursor)?;
    let bucket = receiver_method_bucket(analysis, &inst, is_dollar);
    let class_q = class_q.clone();
    // Per-object visibility flips on a *class*-provided member (issue
    // #1170): `oo::objdefine $o { unexport m }` masks the member for this
    // object's external dispatch even though the class exports it (`$o m` →
    // `unknown method`, tclsh 9.0.4 / 8.6.14), and `… { export m }` revives
    // an unexported one — the dispatch then enters the first implementation
    // regardless of the class-side export state, which is what an internal
    // walk resolves.
    if bucket == MethodBucket::Instance
        && let Some(st) = object_state
    {
        if st.unexports.contains(&method) {
            return Some(Vec::new());
        }
        if st.exports.contains(&method) {
            return Some(method_dispatch_definition(
                analysis,
                source,
                line_index,
                &class_q,
                &method,
                false,
                MethodBucket::Instance,
            ));
        }
    }
    // External `$obj m` / `CLASS m`: the C-faithful dispatch entry — the
    // first exported implementation on the receiver's linearisation
    // (mixins before the class, subclasses before bases; issue #945
    // faults 4 + 6).  A resolved receiver is a definitive answer either
    // way: an unexported/undefined method yields *nothing* rather than
    // falling through to a same-named proc.
    let dispatch = method_dispatch_definition(
        analysis, source, line_index, &class_q, &method, true, bucket,
    );
    if !dispatch.is_empty() {
        return Some(dispatch);
    }
    // `configure` / `cget` accessors and properties have no method entry;
    // keep the property fallback before giving up.
    if let Some(p) = analysis
        .all_classes
        .get(&class_q)
        .and_then(|cd| cd.properties.get(method.as_str()))
    {
        return Some(vec![span_to_range(source, line_index, p.name_span)]);
    }
    Some(Vec::new())
}

/// Walk every class whose `body_span` contains the cursor
/// offset and look up `word` in that class's methods,
/// class-methods, or properties — but ONLY when `word` is
/// itself genuinely bareword-callable from a method body of
/// this class: a bareword sibling call only actually dispatches
/// when `link` (`oo::Helpers::link`) exposed it that way
/// (`ClassDef::linked_members`, issue #923 idx 113); an
/// un-linked member of the same name errors "invalid command
/// name" at runtime, so it must not resolve here either. Also
/// checks constructors/destructor, unconditionally (a distinct,
/// unrelated feature — see [`ClassDef::linked_members`]'s doc).
/// Returns the matched member's `name_span` when found.
///
/// `"constructor"` matches any defined constructor;
/// `"destructor"` matches the destructor.  Other words match
/// against the member's `name`.
fn lookup_class_member(
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<tcl_lexer::Span> {
    let class_def = analysis
        .all_classes
        .get(enclosing_class_at(analysis, cursor_offset)?)?;
    if let Some(target) = class_def.linked_members.get(word) {
        if let Some(m) = class_def.methods.get(target) {
            return Some(m.name_span);
        }
        if let Some(m) = class_def.class_methods.get(target) {
            return Some(m.name_span);
        }
        if let Some(p) = class_def.properties.get(target) {
            return Some(p.name_span);
        }
    }
    if word == "constructor"
        && let Some(c) = class_def.constructors.first()
    {
        if !c.name_span.is_empty() {
            return Some(c.name_span);
        }
        // Analyser doesn't store a name span for the
        // constructor keyword (it has no name token).
        // Fall back to the body span's start so the
        // editor at least lands on the constructor's
        // body opener.
        return Some(c.body_span);
    }
    if word == "destructor"
        && let Some(d) = &class_def.destructor
    {
        if !d.name_span.is_empty() {
            return Some(d.name_span);
        }
        return Some(d.body_span);
    }
    None
}

/// Look up `method` among the per-object methods added by `oo::objdefine`
/// to the object the receiver variable at the *call site* denotes — the
/// per-object override `TclOO` layers ahead of the object's class methods.
/// Returns the declaration's `name_span`.
///
/// Keyed by **binding identity**, not the receiver's textual tail (issue
/// #945 fault 5): two unrelated locals both named `o` in different procs
/// are different objects with different per-object methods, so the
/// candidate set is scoped to the `oo::objdefine` sites whose receiver
/// resolves to the *same variable binding* as the call site's receiver —
/// the innermost scope (proc / method body, else the top level) declaring
/// the name that encloses both.  A dispatch whose binding cannot be
/// matched to exactly one `objdefine` receiver abstains.
fn lookup_object_method(
    analysis: &AnalysisResult,
    source: &str,
    receiver: &str,
    method: &str,
    line: u32,
    character: u32,
) -> Option<tcl_lexer::Span> {
    let records = analysis.object_methods.get(receiver)?;
    let line_index = LineIndex::new(source);
    let call_offset = byte_offset_at(&line_index, source, line, character);
    let call_scope = variable_scope_extent(analysis, receiver, call_offset);
    let matched: Vec<&tcl_compiler::analyser::ObjectMethodDef> = records
        .iter()
        .filter(|m| variable_scope_extent(analysis, receiver, m.objdefine_offset) == call_scope)
        .filter(|m| m.def.name == method)
        .collect();
    // Exactly one binding-compatible override may answer; several distinct
    // same-scope objects (reassignment) or none at all abstain to the
    // class chain.
    match matched.as_slice() {
        [only] => Some(only.def.name_span),
        _ => None,
    }
}

/// The folded per-object member state for the receiver binding at the call
/// site (issue #1170): the [`tcl_compiler::analyser::ObjectMemberState`]
/// whose anchor resolves to the same variable binding as the call — the
/// same binding-identity rule [`lookup_object_method`] applies.  `None`
/// when no `oo::objdefine` touched the receiver, or when the binding is
/// ambiguous (several same-scope states — abstain rather than guess).
pub(crate) fn object_member_state_at<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    receiver: &str,
    line: u32,
    character: u32,
) -> Option<&'a tcl_compiler::analyser::ObjectMemberState> {
    let states = analysis.object_member_state.get(receiver)?;
    let line_index = LineIndex::new(source);
    let call_offset = byte_offset_at(&line_index, source, line, character);
    let call_scope = variable_scope_extent(analysis, receiver, call_offset);
    let matched: Vec<&tcl_compiler::analyser::ObjectMemberState> = states
        .iter()
        .filter(|st| variable_scope_extent(analysis, receiver, st.anchor_offset) == call_scope)
        .collect();
    match matched.as_slice() {
        [only] => Some(only),
        _ => None,
    }
}

/// Whether the cursor sits on an **external** instance-method call whose
/// receiver's per-object member state masks the method name (issue #1170):
/// an `oo::objdefine` unexport of the name, or an unexported per-object
/// member — either makes `$obj m` answer `unknown method` (tclsh 9.0.4 /
/// 8.6.14) regardless of what the class chain provides.
///
/// The in-document provider already answers a masked call with a definitive
/// empty result, but an empty result is indistinguishable from "no local
/// answer" at the server boundary, so the cross-file method tier would
/// resolve the class's own member right past it — this predicate is the
/// gate that tier (and the in-document hover) consults.  `false` for `my`
/// dispatches (internal — the mask is about external visibility) and
/// whenever the binding has no state or an ambiguous one.
#[must_use]
pub fn object_masks_external_dispatch(
    analysis: &AnalysisResult,
    source: &str,
    line: u32,
    character: u32,
) -> bool {
    let Some((inst, method, _is_dollar)) = instance_method_at_cursor(source, line, character)
    else {
        return false;
    };
    if is_self_dispatch_keyword(&inst) {
        return false;
    }
    let Some(st) = object_member_state_at(analysis, source, &inst, line, character) else {
        return false;
    };
    if let Some(md) = st.methods.get(&method) {
        return md.visibility != "public";
    }
    st.unexports.contains(&method)
}

/// The extent (byte range) of the innermost proc / method body scope that
/// declares or contains variable `name` at `offset` — the binding-identity
/// key for per-object method matching.  Two uses of the same simple name
/// belong to the same binding iff their innermost enclosing proc/method
/// body is the same span (or both sit at the top level, extent `None`).
fn variable_scope_extent(
    analysis: &AnalysisResult,
    _name: &str,
    offset: u32,
) -> Option<(u32, u32)> {
    fn innermost(
        scope: &tcl_compiler::analyser::Scope,
        offset: u32,
        best: &mut Option<(u32, u32)>,
        depth: u32,
    ) {
        if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
            return;
        }
        for child in &scope.children {
            if let Some(span) = child.body_span
                && span.start() <= offset
                && offset <= span.end()
            {
                let is_frame = matches!(
                    child.kind,
                    tcl_compiler::analyser::ScopeKind::Proc
                        | tcl_compiler::analyser::ScopeKind::Method
                );
                if is_frame {
                    let width = span.end() - span.start();
                    if best.is_none_or(|(s, e)| width < e - s) {
                        *best = Some((span.start(), span.end()));
                    }
                }
                innermost(child, offset, best, depth + 1);
            }
        }
    }
    let mut best = None;
    innermost(&analysis.global_scope, offset, &mut best, 0);
    best
}

/// The qualified name of the class whose body contains `offset`, if any —
/// the enclosing class for `my`-call dispatch and every other "which class
/// am I lexically inside" query in this crate.
///
/// Consults [`AnalysisResult::class_body_spans`] rather than
/// `ClassDef::body_span` alone: a class extended via a *separate*
/// `oo::define ClassName { ... }` block (real corpus shape:
/// `ticklecharts::chart`'s `oo::class create` at one line, every method
/// added via a later, separate `oo::define` block) has textually disjoint
/// body spans, not one contiguous range — `body_span` alone only ever
/// covers the *first* one recorded (issue #923 idx 52, main audit wave).
/// Ties (nested definitions) resolve to the narrowest containing span,
/// same tie-break the single-span version always used.
///
/// The single canonical implementation of this check — every other
/// module in this crate that used to keep its own copy now calls this
/// one instead, so the multi-span fix applies everywhere uniformly.
pub(crate) fn enclosing_class_at(analysis: &AnalysisResult, offset: u32) -> Option<&str> {
    analysis
        .class_body_spans
        .iter()
        .filter(|(_, span)| span.start() < offset && offset < span.end())
        .min_by_key(|(_, span)| span.end() - span.start())
        .map(|(name, _)| name.as_str())
}

/// Which of a class's two method buckets a dispatch receiver reaches
/// (issue #923 idx 120), consulted by [`method_dispatch_definition`] and,
/// via [`receiver_method_bucket`], by every other consumer of
/// [`receiver_instance_class`] that also needs its own method lookup
/// restricted to the matching bucket (`completion.rs`'s `method_items`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodBucket {
    /// `$obj method` / `my method` — dispatch on an *instance*:
    /// `methods` only, never `class_methods` (a class-side method is
    /// never itself instance-callable — real tclsh raises `unknown
    /// method` for it; this closes a pre-existing false-positive
    /// go-to-definition on e.g. `ActiveRecord create rec1; rec1 find`,
    /// bundled alongside idx 120 since this function's signature was
    /// already changing here).
    Instance,
    /// `CLASS method` — dispatch on the class's own bound command (issue
    /// #923 idx 120): `class_methods` only (a class-side method is never
    /// itself instance-callable — `completion.rs`'s `method_items` already
    /// documents and implements this same exclusion for its instance-side
    /// suggestions). An ancestor-provided entry is accepted only when it
    /// is NOT `is_self_method` — a plain `TclOO` `self method` is visible
    /// only on the exact class that declared it (confirmed against
    /// tclsh: a subclass with no override does not gain it), unlike
    /// `ooutil`'s `classmethod` keyword, which does propagate to a
    /// subclass's own bound command via its `Delegate`-mixin machinery. A
    /// provider matching the receiver class itself is always accepted
    /// regardless (it's not "inherited" there, it's the direct owner).
    Class,
}

/// Go-to-definition for a method call dispatched on an instance of
/// `class_q`: the **first applicable implementation** on the class's
/// `TclOO` linearisation (the entry `$obj m` / `my m` actually runs —
/// issue #945 fault 6), visibility-filtered (issue #945 fault 4):
/// `external` keeps exported implementations only; an internal (`my`)
/// dispatch reaches unexported ones, and `private` ones only in the
/// receiver's own class.  Empty when no implementation is callable in
/// this context — a definitive "unknown method" answer, mirroring C.
fn method_dispatch_definition(
    analysis: &AnalysisResult,
    source: &str,
    line_index: &LineIndex,
    class_q: &str,
    method: &str,
    external: bool,
    bucket: MethodBucket,
) -> Vec<LspRange> {
    // The walk itself lives in `crate::oo_dispatch` so hover and
    // find-references answer from the same linearisation (issue #923 idx
    // 28/34/35 — they each used to do a direct-only lookup on the
    // receiver's own class and silently disagree with this function).
    crate::oo_dispatch::method_dispatch_provider(analysis, class_q, method, external, bucket)
        .map(|(_, md)| vec![span_to_range(source, line_index, md.name_span)])
        .unwrap_or_default()
}

/// Resolve `TclOO` `next` / `nextto` at the cursor to the super-method's
/// `name_span`.
///
/// `next` inside `method m` of class `C` dispatches `m` one step further
/// down the object's MRO — statically we resolve it in `C`'s own MRO (the
/// sound single-dispatch approximation): the next class after `C` that
/// provides `m`.  `nextto Base` restarts the search at `Base`.  The
/// The `TclOO` method-context keyword `word` is under `dialect`, or `None`.
///
/// The crate's single entry to
/// [`tcl_registry::CommandRegistry::method_dispatch_keyword`], so every
/// provider that used to carry a `head == "my"` /
/// `matches!(head, "my" | "next" | "nextto")` literal now asks the registry
/// through one place (issue #1050). A dialect that gains or loses one of
/// these keywords propagates through its `CommandSpec`, never through a
/// walker edit.
///
/// `definition` threads `AnalysisResult::dialect` — the dialect the document
/// was actually analysed under — through its own registry lookups (issue
/// #1064). The remaining `""` callers are the ones with no analysis in scope;
/// `""` resolves to the permissive plain-Tcl profile (availability mask
/// `ALL_TCL`), so every 8.6+ `TclOO` keyword still resolves there.
pub(crate) fn method_dispatch_keyword_in(
    dialect: &str,
    word: &str,
) -> Option<tcl_registry::MethodDispatchKind> {
    tcl_registry::registry_for_dialect(dialect).method_dispatch_keyword(word)
}

/// Whether `word` is the `TclOO` self-dispatch keyword (`my`) — the word
/// after it names a method on the *enclosing* class, reaching non-exported
/// methods a `$obj` dispatch cannot.
pub(crate) fn is_self_dispatch_keyword(word: &str) -> bool {
    method_dispatch_keyword_in("", word) == Some(tcl_registry::MethodDispatchKind::SelfDispatch)
}

/// Whether `word` is a `TclOO` next-chain keyword (`next` / `nextto`) under
/// `dialect` — a registry that predates `TclOO` answers `false`.
fn is_next_chain_keyword_in(dialect: &str, word: &str) -> bool {
    method_dispatch_keyword_in(dialect, word) == Some(tcl_registry::MethodDispatchKind::NextChain)
}

/// Whether a next-chain keyword names an explicit resume-from class in its
/// first argument — `nextto`'s structural marker, an
/// [`tcl_registry::ArgRole::Name`] at index 0 on the spec, which `next` does
/// not declare. Distinguishing the two structurally rather than by name is
/// what `TCLOO_NEXT_CHAIN`'s own documentation asks consumers to do.
pub(crate) fn next_chain_names_a_target_in(dialect: &str, word: &str) -> bool {
    method_dispatch_keyword_in(dialect, word) == Some(tcl_registry::MethodDispatchKind::NextChain)
        && tcl_registry::registry_for_dialect(dialect)
            .get(word)
            .is_some_and(|spec| spec.arg_role_at(0) == Some(tcl_registry::ArgRole::Name))
}

/// [`next_chain_names_a_target_in`] against the dialect-less plain-Tcl
/// profile.
fn next_chain_names_a_target(word: &str) -> bool {
    next_chain_names_a_target_in("", word)
}

/// enclosing class + method are found from the cursor's byte offset.
fn next_dispatch_target(
    analysis: &AnalysisResult,
    source: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
    keyword: &str,
) -> Option<tcl_lexer::Span> {
    let cursor = byte_offset_at(line_index, source, line, character);
    let (class_q, method) = enclosing_method(analysis, cursor)?;
    let start_from: Option<String> = if next_chain_names_a_target(keyword) {
        // Byte offset of the cursor within its line, so `word_after` can pick
        // the occurrence the cursor is on (not merely the first).
        let line_start = byte_offset_at(line_index, source, line, 0);
        let cursor_in_line = cursor.saturating_sub(line_start) as usize;
        let target = word_after(source, line, cursor_in_line, keyword)?;
        Some(canonicalise_class(analysis, &class_q, &target))
    } else {
        None
    };
    let hierarchy = analysis.class_hierarchy();
    let next_class = hierarchy.member_next_provider(
        &class_q,
        &method,
        &class_q,
        start_from.as_deref(),
        source,
    )?;
    let cd = analysis.all_classes.get(next_class)?;
    tcl_compiler::analyser::class_hierarchy::class_member_def(cd, &method).map(|md| md.name_span)
}

/// The `(qualified_class, member_name)` whose member body contains the
/// cursor offset, or `None` when the cursor is not inside one.
///
/// Covers all four member slots, not just the named ones: a `constructor`
/// / `destructor` body reports the synthetic
/// `<constructor>` / `<destructor>` label
/// (`tcl_compiler::analyser::class_hierarchy::CONSTRUCTOR_MEMBER`), which
/// `member_next_provider` routes to the matching provider. Before that a
/// cursor inside a constructor matched nothing here, so `next` — the
/// ordinary way a subclass forwards to its superclass's constructor —
/// resolved to no location at all (issue #923 idx 37).
fn enclosing_method(analysis: &AnalysisResult, cursor: u32) -> Option<(String, String)> {
    use tcl_compiler::analyser::class_hierarchy::{CONSTRUCTOR_MEMBER, DESTRUCTOR_MEMBER};
    let cd = analysis
        .all_classes
        .get(enclosing_class_at(analysis, cursor)?)?;
    let named = cd
        .methods
        .iter()
        .chain(cd.class_methods.iter())
        .map(|(mname, m)| (mname.as_str(), m));
    let ctors = cd
        .constructors
        .iter()
        .map(|c| (CONSTRUCTOR_MEMBER, c))
        .chain(cd.destructor.iter().map(|d| (DESTRUCTOR_MEMBER, d)));
    named
        .chain(ctors)
        .find(|(_, m)| m.body_span.start() <= cursor && cursor <= m.body_span.end())
        .map(|(mname, _)| (cd.qualified_name.clone(), mname.to_owned()))
}

/// The whitespace-delimited word that follows `keyword` on the cursor's
/// line (used to read the class name in `nextto Class`).
///
/// `cursor_in_line` is the cursor's byte offset within the line.  When a
/// line has several `keyword` occurrences (a comment, a string, or a second
/// statement), the occurrence the cursor sits on — or the nearest one
/// starting at or before the cursor — is chosen, so `nextto` go-to-def
/// resolves the class the user is actually pointing at rather than the
/// first match on the line.
fn word_after(source: &str, line: u32, cursor_in_line: usize, keyword: &str) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    // Select the keyword occurrence anchored on the cursor.
    let mut chosen: Option<usize> = None;
    let mut search = 0;
    while let Some(rel) = line_text[search..].find(keyword) {
        let idx = search + rel;
        let end = idx + keyword.len();
        if idx <= cursor_in_line && cursor_in_line <= end {
            chosen = Some(idx); // cursor is on the keyword itself
            break;
        }
        if idx <= cursor_in_line {
            chosen = Some(idx); // best occurrence at/before the cursor so far
        }
        search = end;
    }
    // Fall back to the first occurrence when the cursor precedes them all.
    let idx = chosen.or_else(|| line_text.find(keyword))?;
    let rest = line_text[idx + keyword.len()..].trim_start();
    let word: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
        .collect();
    (!word.is_empty()).then_some(word)
}

/// Canonicalise a written class name to the qualified form keyed in
/// `all_classes`, **owner-aware** — resolved relative to `owner`'s namespace
/// (the class writing the `nextto`) via the shared [`resolve_class_name`]
/// resolver: exact, `::name`, an outward namespace walk, then a unique tail.
/// This lets `nextto Base` reach a namespaced base class (`::Ns::Base`) named
/// bare from a sibling in the same namespace, instead of only matching a
/// global `::Base`.  Falls back to the written name when nothing resolves so
/// the caller's `next_provider` lookup simply finds no target.
fn canonicalise_class(analysis: &AnalysisResult, owner: &str, name: &str) -> String {
    let tail_index =
        tcl_compiler::analyser::class_hierarchy::build_tail_index(analysis.all_classes.keys());
    tcl_compiler::analyser::class_hierarchy::resolve_class_name(
        name,
        owner,
        |cand| analysis.all_classes.contains_key(cand),
        &tail_index,
    )
    .unwrap_or_else(|| name.to_string())
}

/// Detect a `$obj method ...` / `[$obj method ...]` or
/// `objcmd method ...` call where the cursor sits on the *method-name*
/// token.  Returns `(receiver_name, method_name, receiver_is_dollar)`:
/// `receiver_is_dollar` is `true` for a `$var` / `${var}` receiver (a
/// variable holding an object) and `false` for a bare object-command
/// receiver (`objcmd`, e.g. one bound by `CLASS create objcmd`).
///
/// The receiver must be the command-segment head (a single token
/// immediately preceding the method), so the method sits at word-index 1.
/// Command segments are delimited by `;`, `[`, `{`, and the line start —
/// a single-line approximation that covers the common editor cases.
///
/// Whether a bare receiver actually resolves to a class is decided by
/// [`receiver_instance_class`], which gates bare receivers on
/// `created_instance_commands` (so a plain variable's bare name — never a
/// valid dispatch — does not resolve).
///
/// The method-name word is bounded by [`crate::hover::word_char_bounds`] —
/// the same lexer-faithful rule every other cursor-word consumer uses — so
/// a method whose name is not a programming-language identifier still
/// resolves.  Tcl imposes no character restriction on a method name:
/// `$obj with-dash`, `$obj a.b`, and TIP 558's generated property accessors
/// (`my <ReadProp-x>` / `my <WriteProp-x>`, produced by `oo::configurable`'s
/// `property`) all dispatch for real (verified against tclsh 8.6.14 and
/// 9.0.4), and an identifier rule that stopped at `-` / `<` / `>` silently
/// truncated those names to something no class declares (issue #1019 idx
/// 16).
///
/// A word carrying no alphanumeric / `_` content at all — `<`, `<=`, `-`,
/// `::` — is rejected: those are expression operators and separators, so
/// `expr {$a < $b}` must not read as `$a` dispatching a method named `<`.
/// Tcl *would* let a class declare a method literally named `+` or `<`, and
/// this deliberately does not resolve those: `expr` bodies are far more
/// common than operator-named methods, and rejecting them keeps the
/// pre-existing behaviour rather than trading one gap for a false positive.
pub(crate) fn instance_method_at_cursor(
    source: &str,
    line: u32,
    character: u32,
) -> Option<(String, String, bool)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());

    // Method-name word bounds around the cursor.
    let (wstart, wend) = crate::hover::word_char_bounds(&chars, col)?;
    let method: String = chars[wstart..wend].iter().collect();
    if !method.chars().any(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    // Command-segment start: nearest `;` / `[` / `{` to the
    // left, else the line start.
    let mut seg_start = 0;
    for i in (0..wstart).rev() {
        if matches!(chars[i], ';' | '[' | '{') {
            seg_start = i + 1;
            break;
        }
    }
    // The head must be exactly one whitespace-delimited token
    // (the receiver), so the method is word-index 1.
    let prefix: String = chars[seg_start..wstart].iter().collect();
    let head_tokens: Vec<&str> = prefix.split_whitespace().collect();
    if head_tokens.len() != 1 {
        return None;
    }
    let head = head_tokens[0];
    if let Some(rest) = head.strip_prefix('$') {
        // `$var` / `${var}` receiver — a variable holding an object.
        let inst = rest
            .strip_prefix('{')
            .map_or(rest, |r| r.strip_suffix('}').unwrap_or(r));
        if inst.is_empty() {
            return None;
        }
        Some((inst.to_string(), method, true))
    } else {
        // Bare `objcmd` receiver — a plain word naming an object command.
        // Substituted / decorated heads (`[…]`, `{…}`, quoted) are not bare
        // object commands; `receiver_instance_class` further gates this on
        // `created_instance_commands`.
        if head.is_empty() || head.contains(['[', ']', '{', '}', '"', '(', ')', '$']) {
            return None;
        }
        Some((head.to_string(), method, false))
    }
}

/// Resolve a method-dispatch receiver at the cursor (as returned by
/// [`instance_method_at_cursor`]) to its class's qualified name.
///
/// A `$var` receiver (`is_dollar`) is any object-holding variable, looked
/// up in `instance_classes`.  A bare receiver is a valid *instance*
/// dispatch only when it names an object *command* (`CLASS create NAME`) —
/// a plain variable's bare name (`set v [CLASS new]` then `v method`) is
/// not a command and must not resolve — so it is additionally gated on
/// `created_instance_commands`.
///
/// A bare receiver that instead names a class *directly* (`ActiveRecord
/// find`, `TclOO`'s and `ooutil`'s class-command dispatch — issue #923 idx
/// 120) resolves too: `oo::class create NAME` always binds `NAME` as the
/// class's own command, so any class name written in the source is
/// unconditionally dispatchable this way. A `$var` can't textually denote
/// a class — it holds an object handle read from a variable, never the
/// class's own written name — so this path is bare-word only.
///
/// Shared by the definition / references / rename / hover cursor paths so
/// they agree on which receivers dispatch (and, via
/// `method_references_for_class`, so those match the code-lens count).
pub(crate) fn receiver_instance_class<'a>(
    analysis: &'a AnalysisResult,
    receiver: &str,
    is_dollar: bool,
) -> Option<&'a String> {
    if let Some(class) = analysis.instance_classes.get(receiver)
        && (is_dollar || analysis.created_instance_commands.contains(receiver))
    {
        return Some(class);
    }
    if is_dollar {
        return None;
    }
    let qualified = tcl_compiler::analyser::class_hierarchy::resolve_written_class_name(
        receiver,
        &analysis.all_classes,
    )?;
    analysis
        .all_classes
        .get(&qualified)
        .map(|cd| &cd.qualified_name)
}

/// [`receiver_instance_class`] plus the compiler's object-type lattice as the
/// fallback for a `$var` receiver — issue #994's consumer unification (C5b).
///
/// `instance_classes` answers first, exactly as before.  When it has no
/// binding, a **singleton** class in the lattice's scope-keyed map
/// ([`ObjectHandleFacts::by_scope`], read through `classes_in_scope` at the
/// dispatch site's own `offset`) resolves the receiver — the same sound map
/// the compiler's W307 / W308 / E001 emitters consult, so definition,
/// references, rename, hover, completion, and the code lens can no longer
/// disagree with the diagnostics (or with each other) about one document.
/// A multi-class binding abstains: every consumer of this accessor edits or
/// navigates, and a guess there is a wrong edit, not a missed one.
///
/// [`ObjectHandleFacts::by_scope`]: tcl_compiler::object_types::ObjectHandleFacts
pub(crate) fn receiver_instance_class_at<'a>(
    analysis: &'a AnalysisResult,
    receiver: &str,
    is_dollar: bool,
    offset: u32,
) -> Option<&'a String> {
    if let Some(class) = receiver_instance_class(analysis, receiver, is_dollar) {
        return Some(class);
    }
    if !is_dollar {
        return None;
    }
    lattice_singleton_class(analysis, receiver, offset)
}

/// The one class the object-type lattice's scope-keyed map binds `receiver`
/// to in the scope containing `offset`, or `None` when there is no binding or
/// the binding is ambiguous (multi-class).  The lattice's "no evidence is not
/// proof of absence" contract carries over verbatim.
pub(crate) fn lattice_singleton_class<'a>(
    analysis: &'a AnalysisResult,
    receiver: &str,
    offset: u32,
) -> Option<&'a String> {
    let classes = analysis
        .object_handle_facts
        .classes_in_scope(offset, receiver)?;
    if classes.len() != 1 {
        return None;
    }
    classes.iter().next()
}

/// Which [`MethodBucket`] `receiver` reaches, given how
/// [`receiver_instance_class`] resolved it (issue #923 idx 120):
/// recomputes that function's own first condition (the *instance* path —
/// an object handle, `$var`, or a bound `CLASS create NAME` command)
/// rather than inferring it from the outside (e.g. merely checking
/// `instance_classes.contains_key`), so a same-named instance-command /
/// class collision can't misclassify it. `false` on that check, with
/// `receiver_instance_class` having still resolved a `class_q`, can only
/// mean the bare-word-names-a-class-directly path fired instead.
pub(crate) fn receiver_method_bucket(
    analysis: &AnalysisResult,
    receiver: &str,
    is_dollar: bool,
) -> MethodBucket {
    // A `$var` receiver is always an *instance* dispatch: a variable holds an
    // object handle read at run time, never the class's own written command
    // name — so the bucket does not depend on which map resolved its class
    // (`instance_classes` or the object-type lattice's scoped fallback in
    // [`receiver_instance_class_at`]).  A bare receiver reaches the instance
    // bucket only through a `CLASS create NAME` object-command binding.
    let is_instance_path = is_dollar
        || (analysis.instance_classes.contains_key(receiver)
            && analysis.created_instance_commands.contains(receiver));
    if is_instance_path {
        MethodBucket::Instance
    } else {
        MethodBucket::Class
    }
}

/// Resolve a bare `ClassName method` call-site receiver directly to the
/// class whose `classmethod` provides `method` — the reverse of
/// [`crate::references::find_obj_method_call_sites`]'s forward "given the
/// class, which bare names dispatch it" fold, needed to resolve
/// Find-References / Rename / go-to-definition triggered *from* the call
/// site itself rather than from the declaration or a code lens.
///
/// `receiver` must name a class (simple or fully-qualified) that declares
/// or *inherits* (does not override) `method` as a `classmethod` — `None`
/// for an instance-command receiver (that's [`receiver_instance_class`]'s
/// job) or a class with no such classmethod.  Doesn't itself exclude
/// [incr Tcl] classes (whose class-scoped `proc` dispatches as
/// `Factory::method`, not this bare two-word shape): every downstream
/// consumer of the `(class, method, is_classmethod)` triple this feeds —
/// `find_obj_method_call_sites` and friends — already re-checks the
/// definer's family before treating any site as a dispatch, so a
/// same-named itcl class-proc resolves here but then correctly yields no
/// call sites rather than a false one.
#[must_use]
pub(crate) fn classmethod_dispatch_class(
    analysis: &AnalysisResult,
    receiver: &str,
    method: &str,
) -> Option<String> {
    classmethod_dispatch_class_in_workspace(analysis, receiver, method, None)
}

/// [`classmethod_dispatch_class`] with the **workspace tier** available.
///
/// The local analysis answers whenever this document declares or extends the
/// class. When it does not — an ordinary *pure consumer* file, holding nothing
/// but `C cm` — the class's member tables live in another document, and the
/// server's workspace-class oracle reanalysis does not help: it supplies class
/// **names** only (`Analyser::with_workspace_classes`, which exists to let
/// instance inference resolve `[Cls new]`), never their `class_methods`. So
/// classification fell through both receiver branches and definition /
/// references / rename from a consumer answered nothing, even though the
/// workspace index held the fact all along (issue #1119 review).
///
/// `index` closes that gap by consulting the same class-side dispatch chain
/// the cross-file resolvers use — [`WorkspaceIndex::class_method_dispatch_chain`],
/// not a second walk of its own, so the tier that classifies a call and the
/// tier that resolves it can never disagree about which members exist.
///
/// [`MethodAccess::Internal`] is deliberate here: this asks *"is there a
/// class-side member of this name at all"*, which is a question about the
/// tables, not about the caller's access. An unexported class-side member must
/// still classify as class-side — the caller's own access is applied afterwards
/// by the resolver, which is what makes a `self unexport`ed member classify
/// here and then correctly resolve to nothing.
#[must_use]
pub(crate) fn classmethod_dispatch_class_in_workspace(
    analysis: &AnalysisResult,
    receiver: &str,
    method: &str,
    workspace: Option<WorkspaceReceiver<'_>>,
) -> Option<String> {
    let Some(named) = analysis
        .all_classes
        .values()
        .find(|cd| cd.name == receiver || cd.qualified_name == receiver)
    else {
        // No local record for the receiver at all — the pure-consumer shape.
        return workspace.and_then(|w| workspace_classmethod_class(&w, receiver, method));
    };
    if named.class_methods.contains_key(method) {
        return Some(named.qualified_name.clone());
    }
    let local = local_inherited_classmethod(analysis, &named.qualified_name, method);
    if local.is_some() {
        return local;
    }
    // A local record that does not itself declare the member is no proof there
    // is none: this document may hold only an `oo::define C { … }` extension
    // while the `self method` lives in the class's own file. Same workspace
    // tier as the no-local-record case above.
    workspace.and_then(|w| workspace_classmethod_class(&w, receiver, method))
}

/// The class an *inherited* `classmethod` resolves to, using this document's
/// own hierarchy — the local half of
/// [`classmethod_dispatch_class_in_workspace`].
fn local_inherited_classmethod(
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
) -> Option<String> {
    let hierarchy = analysis.class_hierarchy();
    let provider_q = hierarchy.method_target(class_q, method)?;
    analysis
        .all_classes
        .get(provider_q)
        .and_then(|cd| cd.class_methods.get(method))
        // A stock `TclOO` `self method` is **not** inherited: it lives on the
        // class object that declared it, and a subclass's own command never
        // reaches it — `Gadget make` against a parent's `self method make`
        // errors `unknown method "make": must be create, destroy or new` under
        // tclsh 8.6 and 9.0.4 alike. An `ooutil`-style `classmethod` shares the
        // receiver bucket but does propagate, which is what
        // [`MethodDef::is_self_method`] distinguishes. Same rule
        // [`crate::oo_dispatch::method_dispatch_provider`] applies for the
        // in-document answer; without it here an inherited `self method` was
        // reported as the receiver's dispatch target.
        .filter(|md| !md.is_self_method)
        .map(|_| provider_q.to_string())
}

/// Where a bare receiver word was written, with the workspace tier to resolve
/// it against — everything the cross-file classmethod arm needs to reach a
/// class this document never mentions.
#[derive(Clone, Copy)]
pub(crate) struct WorkspaceReceiver<'a> {
    /// The workspace index.
    pub index: &'a crate::workspace_index::WorkspaceIndex,
    /// The `::`-prefixed namespace in effect at the cursor
    /// ([`namespace_context_at`]).
    pub namespace: &'a str,
    /// The call's own site — its offset and enclosing body span — so
    /// import resolution is gated the way the interpreter orders it.
    pub call: crate::workspace_index::CallSite<'a>,
}

/// The qualified class a bare `receiver method` names, decided from the
/// **workspace index** — the tier that knows a class this document never
/// mentions.
///
/// The receiver is resolved the way Tcl resolves any command word, not by a
/// bare-name match: [`tcl_syntax::naming::bareword_resolution_candidates`] is
/// the canonical encoding of `Tcl_FindCommand`'s order (current namespace
/// first, then the global one), so `namespace eval ::a { C cm }` reaches
/// `::a::C` and — when only a global `::C` exists — falls through to it. A
/// literal `C`/`::C` check missed the relative spelling entirely, which is the
/// shape a namespaced class is normally *called* in (issue #1178 review).
///
/// Two tiers, in Tcl's own order:
///
/// 1. **Lexical.** The first candidate naming an indexed class that carries the
///    member wins, so an inner `::a::C` shadows a global `::C` rather than
///    racing it.
/// 2. **Import-mediated.** `namespace import` makes a class reachable under a
///    name no candidate spells. Both spellings are asked through the machinery
///    that already models them — an exact import is a `WorkspaceCommandLink`
///    ([`WorkspaceIndex::resolve_command_target`]) and a glob one is resolved
///    per call ([`WorkspaceIndex::resolve_wildcard_import`]) — so the import
///    lifecycle and ordering rules come for free rather than being restated
///    here. That includes the invariant that an import which has not run at
///    this call's site does not resolve.
///
/// The two-classes-one-name abstention applies at each tier: guessing between
/// them is the namespace-blind match issue #981 removed.
fn workspace_classmethod_class(
    workspace: &WorkspaceReceiver<'_>,
    receiver: &str,
    method: &str,
) -> Option<String> {
    let index = workspace.index;
    let candidates =
        tcl_syntax::naming::bareword_resolution_candidates(workspace.namespace, receiver);
    // Tier 1 — lexical, in Tcl's resolution order.
    for cand in &candidates {
        if let Some(hit) = class_side_member_owner(index, cand, method) {
            return Some(hit);
        }
    }
    // Tier 2 — import-mediated. An exact `namespace import ::a::C` records a
    // fixed link, a glob one is decided per call; ask each through its own
    // existing resolver.
    for cand in &candidates {
        let linked = index.resolve_command_target(cand);
        if linked != *cand
            && let Some(hit) = class_side_member_owner(index, &linked, method)
        {
            return Some(hit);
        }
    }
    let imported = index.resolve_wildcard_import(receiver, &candidates, workspace.call)?;
    class_side_member_owner(index, &imported, method)
}

/// The qualified name of the class `candidate` names, when the workspace has
/// exactly one such class and its **class-object side** carries `method`.
///
/// Membership goes through [`WorkspaceIndex::class_method_dispatch_chain`]
/// rather than reading `WorkspaceClass::class_method` here, so the
/// `self method`-is-not-inherited rule, the class-side export union and the
/// class-side retraction tombstones are applied exactly once, in the walk this
/// and the resolver share. [`MethodAccess::Internal`] is deliberate: this asks
/// *"is there a class-side member of this name at all"*, a question about the
/// tables, not about the caller's access — which is applied afterwards by the
/// resolver, and is what lets a `self unexport`ed member classify here and then
/// correctly resolve to nothing.
fn class_side_member_owner(
    index: &crate::workspace_index::WorkspaceIndex,
    candidate: &str,
    method: &str,
) -> Option<String> {
    let mut named = index
        .classes_named(candidate)
        .into_iter()
        .map(|c| c.qualified_name.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter();
    // Exactly one indexed class may answer: two same-named classes leave the
    // receiver genuinely ambiguous, and guessing one is the namespace-blind
    // match issue #981 removed.
    let class_q = named.next()?;
    if named.next().is_some() {
        return None;
    }
    (!index
        .class_method_dispatch_chain(
            &class_q,
            method,
            crate::workspace_index::MethodAccess::Internal,
        )
        .is_empty())
    .then_some(class_q)
}

/// The command name a word written at `cursor_off` actually reaches once the
/// document's `rename` / `interp alias` statements are taken into account, or
/// `None` when the command table leaves the word alone there.
///
/// The single navigation entry to the shared hop walk
/// ([`tcl_compiler::analyser::indirection::walk`]) that the analyser's
/// constructor typing already uses — order-gated, hop-capped, and declining an
/// argument-prepending alias, all decided in one place so a class that W307
/// resolves through a rename and a class go-to-definition resolves through the
/// same rename can never disagree (issues #1062 B1/B2, #1064).
///
/// Navigation asks only "which name does this reach"; whether that name has a
/// user `proc` / class to jump to is the caller's own question, so no liveness
/// predicate is applied here.  Canonicalisation is the plain
/// qualified-name normalisation the alias/rename tables are keyed by — a
/// bare `mycmd` matches an alias stored as `::mycmd`, exactly like the
/// `lookup_alias` it replaces.
///
/// `O(1)` hash lookups bounded by the hop cap: no invocation scan, no source
/// rescan, so it is safe to call on every request.
#[must_use]
pub(crate) fn command_indirection(
    analysis: &AnalysisResult,
    word: &str,
    cursor_off: u32,
) -> Option<tcl_compiler::analyser::indirection::Indirection> {
    tcl_compiler::analyser::indirection::walk(
        analysis,
        word,
        cursor_off,
        &tcl_syntax::naming::normalise_qualified_name,
    )
}

/// [`command_indirection`] for the callers that only need the terminal name.
#[must_use]
pub(crate) fn command_indirection_target(
    analysis: &AnalysisResult,
    word: &str,
    cursor_off: u32,
) -> Option<String> {
    command_indirection(analysis, word, cursor_off).map(|hop| hop.target)
}

/// Whether `qualified` is a name **this document** only gains through a
/// `rename` / `interp alias` that has not run yet at `cursor_off`.
///
/// The workspace index's command links are position-free by design — a
/// mutation in *another* file is unconditionally in effect, since that file
/// loads wholesale — but a link this same document established later in its
/// own text is not, and following it would resolve a call tclsh answers with
/// `invalid command name`. The cross-document resolver
/// (`tcl-lsp-server::resolve_workspace_symbols`) asks this before chasing a
/// link so its ordering matches the in-document provider's (issue #1064).
///
/// A name can carry both a `rename` and an `interp alias` record; the document
/// gains the name as soon as the *first* of them runs, so the question is
/// asked of the earliest — reading one map ahead of the other would call a
/// name pending on the strength of a mutation that is not the one that
/// introduced it (PR #1075 review, P2).
#[must_use]
pub fn indirection_pending_at(analysis: &AnalysisResult, qualified: &str, cursor_off: u32) -> bool {
    let key = tcl_syntax::naming::normalise_qualified_name(qualified);
    let earliest = analysis
        .rename_offsets
        .get(&key)
        .into_iter()
        .chain(analysis.alias_offsets.get(&key))
        .min();
    earliest.is_some_and(|&at| {
        !tcl_compiler::analyser::indirection::in_effect(analysis, at, cursor_off)
    })
}

/// Resolve `head sub` (as returned by [`instance_method_at_cursor`], with
/// `is_dollar == false`) to the target command it dispatches to, when
/// `head` names a known `namespace ensemble create -map`/`-subcommands`
/// ensemble and `sub` exactly matches one of its literal, statically
/// -recorded subcommands (issue #923 idx 106) — `namespace` is the call
/// site's own command-resolution namespace, so a relative `head` (the
/// overwhelmingly common case: an ensemble dispatches through its own short
/// name) resolves the same way an ordinary bare command call would.
pub(crate) fn ensemble_subcommand_target<'a>(
    analysis: &'a AnalysisResult,
    namespace: &str,
    head: &str,
    sub: &str,
) -> Option<&'a str> {
    tcl_syntax::naming::bareword_resolution_candidates(namespace, head)
        .into_iter()
        .find_map(|cand| analysis.ensemble_subcommand_targets.get(&cand))
        .and_then(|subs| subs.get(sub))
        .map(|entry| entry.target.as_str())
}

/// The proc whose **own declaration name token** covers `cursor_off`.
///
/// The `greet` of `proc greet {…} {…}` is an *argument* of `proc`, never a
/// command head, so a provider gated on head position
/// ([`offset_is_command_head`]) must answer it here instead. Declaration
/// sites are consulted after `all_procs` so a declaration a later same-named
/// `proc` displaced still resolves — to whichever definition currently wins
/// the name, the same rule `resolve_proc_target_at` applies (issue #923
/// idx 31 / idx 45).
#[must_use]
pub(crate) fn proc_declaration_at(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::analyser::ProcDef> {
    let covers = |span: tcl_lexer::Span| span.start() <= cursor_off && cursor_off < span.end();
    if let Some(proc_def) = analysis.all_procs.values().find(|p| covers(p.name_span)) {
        return Some(proc_def);
    }
    let (qualified, _) = analysis
        .proc_declaration_sites
        .iter()
        .find(|(_, span)| covers(*span))?;
    analysis.all_procs.get(qualified)
}

/// The command invocation the analyser recorded whose **head token** covers
/// `cursor_off`, if any.
///
/// The one place definition / hover ask "is this position a command head at
/// all, and what did the resolution machinery decide it names?".  Recorded
/// invocations are the analyser's own answer: an entry exists exactly because
/// the analyser walked that word *as a command name*, in a region it proved
/// was live script — so it is a far stronger statement than the pure text
/// word-boundary scan ([`crate::hover::find_word_span_at_position`]) the
/// bareword fallbacks otherwise run on.
#[must_use]
pub(crate) fn invocation_head_at(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::signature_scan::types::SignatureCommandInvocation> {
    analysis
        .command_invocations
        .iter()
        .filter(|inv| inv.range.start() <= cursor_off && cursor_off < inv.range.end())
        .min_by_key(|inv| inv.range.end() - inv.range.start())
}

/// Whether the word at `cursor_off` occupies its enclosing command's **head**
/// position — i.e. whether resolving it as a command name can be right at all.
///
/// Issue #1137 idx 50: `definition()` / `hover()` used to hand whatever word
/// the text scan returned straight to [`resolve_called_proc`], so an ordinary
/// *argument* that happened to share a proc's name resolved onto that proc —
/// a wrong answer, not an abstention (`return [$types(callback:$tag) jsondump
/// $huddle_object]` resolved the ensemble subcommand `jsondump` onto the very
/// proc containing the call, and the reduced control `anotherproc dump`
/// reproduces it with no indirection at all).
///
/// The gate is the analyser's recorded invocation set, not a second text
/// scan, so it agrees with find-references / rename / call-hierarchy by
/// construction — those already key off the same records.
///
/// **Deliberate limit.** A word inside a *braced argument of an unregistered
/// command* (`mycustom { helper }`) is not recorded, because nothing proves
/// that brace holds script rather than data, so the gate reads it as "not a
/// command head" and the provider abstains.  That is the honest answer: the
/// call may never happen.  Registry-known script arguments (`if`, `foreach`,
/// `switch` arms, `try`, `catch`, `eval`, `uplevel`, `namespace eval`,
/// `apply` bodies, `oo::define` member bodies, command-prefix callbacks such
/// as `lsort -command cb`, and `{*}`-expanded constant heads) *are* recorded
/// and keep resolving.
#[must_use]
pub(crate) fn offset_is_command_head(analysis: &AnalysisResult, cursor_off: u32) -> bool {
    invocation_head_at(analysis, cursor_off).is_some()
}

/// The declaration span an **already-resolved indirect** command head at
/// `cursor_off` reaches, or `None` when no such head covers the cursor.
///
/// Issue #1133: the analyser resolves `set ns ::tc; ${ns}::setdef` to
/// `::tc::setdef` and records it as a `command_invocations` entry with
/// `indirect: true`, but `definition()` never consulted the entry and fell
/// through to the bareword lookup, which answered an unrelated same-named
/// `::other::setdef`.  The span does not carry the written command name, so
/// there is nothing for a text scan to resolve — the recorded resolution is
/// the only correct answer, and it must win.  The same record settles a
/// constant `$cmd` head (`set cmd greet; $cmd`), which the text scan can
/// likewise never resolve.
///
/// Abstains — rather than blocking the ordinary fallback — whenever the
/// indirect entry is *unresolved* (`resolved_qualified_name` is `None`, or it
/// names nothing this document declares): an unknown indirection is no reason
/// to withhold an answer the bareword path can still give.
fn resolved_indirect_head_target(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<tcl_lexer::Span> {
    if let Some(proc_def) = resolved_indirect_head_proc(analysis, cursor_off) {
        return Some(proc_def.name_span);
    }
    let resolved = resolved_indirect_head_name(analysis, cursor_off)?;
    analysis
        .all_classes
        .get(resolved)
        .map(|class_def| class_def.name_span)
}

/// The qualified name an already-resolved indirect head at `cursor_off`
/// settled on — the shared first half of [`resolved_indirect_head_target`]
/// and [`resolved_indirect_head_proc`].
fn resolved_indirect_head_name(analysis: &AnalysisResult, cursor_off: u32) -> Option<&str> {
    analysis
        .command_invocations
        .iter()
        .filter(|inv| {
            inv.indirect && inv.range.start() <= cursor_off && cursor_off < inv.range.end()
        })
        .min_by_key(|inv| inv.range.end() - inv.range.start())?
        .resolved_qualified_name
        .as_deref()
}

/// The `ProcDef` an already-resolved indirect head at `cursor_off` reaches —
/// the hover-side twin of [`resolved_indirect_head_target`] (issue #1133), so
/// hover and go-to-definition answer the same indirection from one record.
#[must_use]
pub(crate) fn resolved_indirect_head_proc(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::analyser::ProcDef> {
    let resolved = resolved_indirect_head_name(analysis, cursor_off)?;
    let proc_def = analysis.all_procs.get(resolved)?;
    Some(
        analysis
            .proc_def_in_effect_at(&proc_def.qualified_name, cursor_off)
            .unwrap_or(proc_def),
    )
}

/// Compute the byte offset of a 0-based LSP `(line, character)`
/// pair in `source`. `character` is a UTF-16 code-unit offset.
///
/// Takes a pre-built `line_index` so callers that perform several
/// position conversions per request share a single index.
pub(crate) fn byte_offset_at(
    line_index: &LineIndex,
    source: &str,
    line: u32,
    character: u32,
) -> u32 {
    line_index.offset_at_utf16(line, Utf16Col::new(character), source)
}

/// Walk the scope tree to find the variable definition that
/// the cursor's `byte_offset` would see — the innermost
/// scope whose body span contains the offset takes precedence
/// over any enclosing scope.
///
/// Descend into the innermost matching child, then walk the
/// scope chain outward for the var lookup.
/// Whether an `uplevel` body's frame semantics hide the scope at index `i` of
/// the resolution `chain` (outermost-first) from the cursor.
///
/// Inside `uplevel #0 { … }` the script runs in the **global** frame, so the
/// enclosing proc's locals are dropped and resolution reaches the
/// global/namespace variable (a same-named proc-local must not shadow it).
/// Inside a non-`#0` `uplevel N { … }` the script runs in a **caller** frame
/// that is statically unknown, so everything *outside* the uplevel body is
/// dropped — the body's own locals resolve, but a name it does not declare
/// abstains rather than mis-attributing to the enclosing proc *or* the global
/// frame (the level word, recorded as the uplevel scope's name, distinguishes
/// the two).
fn uplevel_hides_scope(chain: &[&tcl_compiler::analyser::Scope], i: usize) -> bool {
    use tcl_compiler::analyser::ScopeKind;
    let Some(up) = chain.iter().rposition(|sc| sc.kind == ScopeKind::Uplevel) else {
        return false;
    };
    if chain[up].name == "#0" {
        chain[i].kind == ScopeKind::Proc
    } else {
        i < up
    }
}

pub(crate) fn lookup_var_in_scope_chain<'a>(
    scope: &'a tcl_compiler::analyser::Scope,
    byte_offset: u32,
    name: &str,
    ns_global_fallback: bool,
) -> Option<&'a tcl_compiler::analyser::VarDef> {
    use tcl_compiler::analyser::ScopeKind;
    // A `::`-qualified name first tries a direct, single-table lookup in
    // exactly the namespace the qualifier names (tclsh 9.0.4/8.6.16-verified:
    // `$other::v` referenced from inside namespace ::mine never tries
    // ::mine::other::v — only the one namespace the qualifier actually
    // resolves to). An absolute qualifier (leading `::`) resolves from the
    // global namespace; a relative one resolves under the cursor's own
    // command-resolution namespace — empirically the *same* namespace an
    // unqualified command there would resolve against, including the
    // `oo::class` method global-only special case and the `uplevel #0`
    // reset, so [`innermost_namespace_at`] (already the single canonical
    // implementation of that rule) doubles as the qualifier's base with no
    // separate variable-specific namespace logic. This alone covers every
    // `variable`-declared namespace cell from *anywhere* in the file — the
    // case the qualified outward walk below can't reach, since it only
    // ever sees scopes on the cursor's own lexical chain.
    //
    // A miss falls through to that walk rather than returning early: a
    // literal-qualified write with no preceding `namespace eval` / `variable`
    // (`set ns::var val`) records its `VarDef` verbatim under the writer's
    // own active scope, with no namespace-tree node to find by path — that
    // shape is only reachable when the cursor's own chain happens to include
    // the scope that wrote it (`chain[0]`, the global scope, always does),
    // exactly as before this lookup existed.
    if name.contains("::") {
        // Variable ($var) resolution deliberately does not consult
        // `namespace_overrides` (issue #923 idx 116 scoped this to
        // command/bareword resolution only) — `&[]` keeps this call
        // provably unaffected; `scope` alone (no `analysis`) is available
        // here regardless.
        let here = innermost_namespace_at(scope, byte_offset, &[]);
        let qualified = tcl_compiler::naming::qualify(&here, name);
        let (target_ns, base_name) = tcl_compiler::naming::key_holder_and_tail(&qualified);
        if let Some(v) =
            tcl_compiler::analyser::lookup_var_in_namespace(scope, target_ns, base_name)
        {
            return Some(v);
        }
    }
    // First, find the innermost scope containing the cursor.
    let chain = scope_chain_at(scope, byte_offset);
    // A bare name at a **namespace frame** (cursor directly in a `namespace
    // eval` body) follows C Tcl's two-table rule: the namespace's own
    // variables, then — only under a Tcl 8.x dialect (TIP 278;
    // `ns_global_fallback`, from
    // `AnalysisResult::ns_var_global_fallback`) — the global namespace.
    // Enclosing *intermediate* namespaces are never consulted in **any**
    // version.  Qualified names (the direct namespace lookup above already
    // missed) and proc-like frames (proc / method / uplevel bodies) keep the
    // lenient outward walk below.  (An `interp eval` child scope shares the
    // `Namespace` kind, so under 8.x the global leg can still cross the
    // interp boundary — a pre-existing leniency this rule already narrows
    // from "every enclosing scope" to "the global table only".)
    if !name.contains("::")
        && chain.len() > 1
        && chain
            .last()
            .is_some_and(|sc| sc.kind == ScopeKind::Namespace)
    {
        let ns = chain.last().expect("guarded: chain has a last element");
        if let Some(v) = ns.variables.get(name) {
            return Some(v);
        }
        return ns_global_fallback
            .then(|| {
                chain
                    .first()
                    .expect("guarded: chain has a first element")
                    .variables
                    .get(name)
            })
            .flatten();
    }
    // Walk outward (innermost-first) looking for the var, honouring the
    // `uplevel` frame semantics (see [`uplevel_hides_scope`]).
    for (i, sc) in chain.iter().enumerate().rev() {
        if uplevel_hides_scope(&chain, i) {
            continue;
        }
        if let Some(v) = sc.variables.get(name) {
            return Some(v);
        }
    }
    None
}

/// Every reference span (other than `var_def`'s own declaration) for the
/// variable `var_def` denotes, gathered across the whole scope tree by
/// unifying the aliases Tcl treats as one cell:
///
/// * **Namespace / global aliases** — `global v`, `variable v`, `namespace
///   upvar ns v local` each record a `link_target` (the qualified cell name);
///   every alias of that cell, and the namespace-level declaration itself,
///   shares the target, so their declarations (each spells the name) and uses
///   all unify.  This is the analyser analogue of Tcl's `VAR_LINK`.
/// * **`TclOO` instance variables** — pre-bound into every method scope with
///   the *same* `variable v` declaration span; unioning by that shared span
///   links the per-method copies into the one per-object cell.
///
/// A zero-width declaration span (the fallback for a declaration-less
/// grammar-injected implicit) can't be unioned safely — several such seeds in
/// one body share it — so that case returns exactly `var_def`'s own
/// references.  The caller adds `var_def`'s own declaration span itself (when
/// including the declaration), so it is excluded here.
#[must_use]
pub(crate) fn linked_var_reference_spans(
    scope: &tcl_compiler::analyser::Scope,
    var_def: &tcl_compiler::analyser::VarDef,
) -> Vec<tcl_lexer::Span> {
    let mut out = Vec::new();
    match var_def.link_target.as_deref() {
        Some(target) => {
            collect_alias_spans(scope, target, var_def.definition_span, &mut out, 0);
            // `target`'s own canonical cell (a plain `set` / declaration with
            // no `link_target` of its own) is never found by the alias-only
            // scan above, since that scan only unions records that name
            // `target` as *their own* link target (issue #923 idx 68): fold
            // it in directly.
            if let Some(canonical) =
                tcl_compiler::analyser::lookup_var_by_qualified_name(scope, target)
                && canonical.definition_span != var_def.definition_span
            {
                out.push(canonical.definition_span);
                out.extend(canonical.references.iter().copied());
            }
        }
        None if !var_def.definition_span.is_empty() => {
            collect_shared_span_refs(scope, var_def.definition_span, &mut out, 0);
            // The inverse of the fold-in above: `var_def` might itself BE the
            // canonical cell some `global` / `variable` / `namespace upvar`
            // alias elsewhere points at (issue #923 idx 68) — its own
            // `link_target` is `None` precisely because it isn't an alias, so
            // this can only be discovered by finding its own qualified name
            // and searching for aliases of it.
            if let Some(own_qualified) =
                tcl_compiler::analyser::qualified_name_for_var_decl(scope, var_def.definition_span)
            {
                collect_alias_spans(scope, &own_qualified, var_def.definition_span, &mut out, 0);
            }
        }
        None => out.extend(var_def.references.iter().copied()),
    }
    out
}

/// Declarations (other than `own_decl`) and uses of every variable aliasing the
/// cell `target`.
fn collect_alias_spans(
    scope: &tcl_compiler::analyser::Scope,
    target: &str,
    own_decl: tcl_lexer::Span,
    out: &mut Vec<tcl_lexer::Span>,
    depth: u32,
) {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for v in scope.variables.values() {
        if v.link_target.as_deref() == Some(target) {
            if v.definition_span != own_decl {
                out.push(v.definition_span);
            }
            out.extend(v.references.iter().copied());
        }
    }
    for child in &scope.children {
        collect_alias_spans(child, target, own_decl, out, depth + 1);
    }
}

/// Uses of every variable sharing the declaration span `def_span` (the `TclOO`
/// instance-variable case).
fn collect_shared_span_refs(
    scope: &tcl_compiler::analyser::Scope,
    def_span: tcl_lexer::Span,
    out: &mut Vec<tcl_lexer::Span>,
    depth: u32,
) {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for v in scope.variables.values() {
        if v.definition_span == def_span {
            out.extend(v.references.iter().copied());
        }
    }
    for child in &scope.children {
        collect_shared_span_refs(child, def_span, out, depth + 1);
    }
}

/// The `::`-rooted **namespace variable cell** the cursor names, when it names
/// one — the single entry point the cross-document variable tier
/// (go-to-definition / hover / find-references against
/// [`WorkspaceIndex`](crate::workspace_index::WorkspaceIndex)'s variable
/// tables) resolves through.
///
/// Two cursor shapes answer, and only these two:
///
/// 1. A **qualified occurrence** — `$::tomato::version`, `$app::colors::palette`,
///    `set ::ns::v 1`.  A relative qualifier roots against the cursor's own
///    command-resolution namespace, exactly as
///    [`lookup_var_in_scope_chain`] already roots the in-document lookup, so
///    the local and cross-document tiers agree on which cell a written name
///    means.
/// 2. A **namespace-scoped declaration** the current document holds — the
///    `version` token of `namespace eval tomato { variable version 1.3 }`, or
///    any same-cell write.  This is what lets find-references run *from* the
///    declaration and reach consumers in sibling files.
///
/// An **unqualified** occurrence that is not such a declaration answers
/// `None`: a bare `$v` names whichever cell the local scope chain supplies,
/// which no other document can know, so widening it to a workspace search
/// would be a guess.  A `$name`-shaped substring inside a comment or a data
/// brace answers `None` too — Tcl substitutes nothing there (issue #923
/// idx 24), and the cross-document tier must not invent a reference the
/// in-document one correctly refuses.
///
/// Findings idx 65 / 75 / 78 of the issue #923 differential audit are all
/// this one gap: `$::NS::var` whose declaring `namespace eval NS { variable
/// var }` lives in a sibling file.
#[must_use]
pub fn qualified_variable_cell_at(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<String> {
    let line_index = LineIndex::new(source);
    let cursor_off = byte_offset_at(&line_index, source, line, character);
    if let Some(name) = substituting_var_at_position(source, dialect, line, character, cursor_off) {
        let base = tcl_compiler::naming::normalise_var_name(&name);
        if !base.contains("::") {
            return None;
        }
        let here = innermost_namespace_at(&analysis.global_scope, cursor_off, &[]);
        return Some(tcl_compiler::naming::qualify(&here, base));
    }
    // A bareword sitting on a namespace-scoped declaration (or a later
    // same-cell write): the scope tree already knows the cell's qualified
    // name, so read it off rather than re-deriving one from the word.
    tcl_compiler::analyser::namespace_variables(&analysis.global_scope)
        .into_iter()
        .find(|(_, var)| {
            let contains =
                |span: tcl_lexer::Span| span.start() <= cursor_off && cursor_off < span.end();
            contains(var.definition_span) || var.references.iter().any(|&r| contains(r))
        })
        .map(|(qualified, _)| qualified)
}

/// Resolve the [`VarDef`](tcl_compiler::analyser::VarDef) whose declaration
/// or a later same-cell bareword occupies `byte_offset` — i.e. the cursor
/// sits on a plain name token (a `set` / `variable` / `global` / proc- or
/// method-parameter declaration, or a later bareword write to that same
/// variable such as a `catch script name` result-var) rather than a `$ref`.
///
/// Deliberately does **not** route through [`scope_chain_at`]'s lexical
/// containment walk: a proc/method parameter's own name token sits in the
/// parameter list, textually *before* that scope's `body_span` even
/// starts, so a scope-chain lookup keyed on `byte_offset` never reaches
/// the scope that owns it (issue #923 differential-audit finding idx 9,
/// main audit wave — go-to-definition/hover/find-references all returned
/// nothing for a cursor placed directly on a parameter's or a `catch`
/// result-var's own declaring token, even though every `$name` read of the
/// same variable resolved correctly). Instead this walks every scope in
/// the whole tree unconditionally, checking each `VarDef`'s own
/// `definition_span` and every span in `references` for byte-offset
/// containment. This is safe without any scope-visibility filtering: a
/// byte-offset span match is unambiguous by construction (no two distinct
/// declaration/write occurrences in a file share a byte range), unlike a
/// *name*-based lookup which would need scope-aware disambiguation.
pub(crate) fn var_def_at_declaration_offset(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> Option<&tcl_compiler::analyser::VarDef> {
    let contains = |span: tcl_lexer::Span| span.start() <= byte_offset && byte_offset < span.end();
    for v in scope.variables.values() {
        if contains(v.definition_span) || v.references.iter().any(|&r| contains(r)) {
            return Some(v);
        }
    }
    scope
        .children
        .iter()
        .find_map(|child| var_def_at_declaration_offset(child, byte_offset))
}

/// The [`VarDef`](tcl_compiler::analyser::VarDef) a `$name` **read** at
/// `cursor_off` resolves to — [`lookup_var_in_scope_chain`] gated on the
/// occurrence being one Tcl actually substitutes.
///
/// The cursor-word scan ([`crate::hover::find_var_at_position`]) is a
/// delimiter-based character scan with no idea whether the `$name` text it
/// matched is in a substituting position.  It therefore matches a
/// `$level`-shaped substring inside an inert Tcl comment, or inside a
/// brace-quoted word Tcl emits byte-for-byte unsubstituted (`set t {plain
/// $level here}` prints `$level` verbatim on tclsh 8.6 and 9.0) — and hover /
/// go-to-definition / find-references / rename then confidently resolved it to
/// a real declaration, contradicting the LSP's own semantic tokens (one opaque
/// comment / string token, no nested variable) and its own W220
/// "assignment is never read" (which correctly treats those as non-reads).
/// Issue #923 differential-audit finding idx 24.
///
/// [`crate::inert_text`] holds both proofs, and both are conservative — they
/// answer "inert" only when the position provably is, so abstaining on them
/// can never drop a genuine reference.  The scope-chain lookup runs first
/// because it is the cheap half: the data-brace proof re-segments the source,
/// and there is nothing to suppress when no `VarDef` resolved anyway.
pub(crate) fn lookup_var_read_at<'a>(
    global: &'a tcl_compiler::analyser::Scope,
    source: &str,
    dialect: &str,
    cursor_off: u32,
    name: &str,
    ns_global_fallback: bool,
) -> Option<&'a tcl_compiler::analyser::VarDef> {
    let var_def = lookup_var_in_scope_chain(global, cursor_off, name, ns_global_fallback)?;
    (!offset_is_inert(source, dialect, cursor_off)).then_some(var_def)
}

/// Whether `cursor_off` sits in text Tcl substitutes nothing in — a `#`
/// comment or a braced *data* word.  The two [`crate::inert_text`] proofs
/// under one name, so every caller asks the same question the same way.
#[must_use]
pub(crate) fn offset_is_inert(source: &str, dialect: &str, cursor_off: u32) -> bool {
    crate::inert_text::offset_in_comment(source, cursor_off)
        || crate::inert_text::offset_in_data_brace(
            source,
            cursor_off,
            tcl_registry::registry_for_dialect(dialect),
            dialect,
        )
}

/// **The one entry point** for "is the cursor on a live `$`-substitution, and
/// which variable does it name?" — every variable provider (go-to-definition,
/// hover, find-references, document-highlight, rename) resolves its `$ref`
/// cursor through this, so the five cannot disagree.
///
/// [`crate::hover::find_var_at_position`] alone cannot answer it: it is a
/// delimiter-based character scan, so it happily reports `n` for a cursor on
/// the `n` of `set {$n} 1`. That word is **brace-quoted** — Tcl substitutes
/// nothing inside it, so the `$n` there is not a reference to `n` at all; it
/// is part of the literal *name* of a different variable (tclsh 9.0.4 /
/// 8.6.14: `set {$n} v; info exists {$n}` → 1 while `info exists n` → 0).
/// Answering `n` there made every provider report the unrelated plain cell's
/// sites — or, once the scope gate rejected it, nothing at all — for a cursor
/// sitting inside the literal cell's own declaration word (PR #1106 review,
/// P2; the cursor half of issue #1108).
///
/// Returning `None` for such a cursor is what lets each provider fall through
/// to its declaration-span search
/// ([`var_def_at_declaration_offset`]), which resolves the literal cell
/// correctly — exactly as it already did for a cursor on the word's opening
/// `{`, where the character scan happened to find no `$`. The two columns of
/// the same word now answer alike.
///
/// The same `None` covers the pre-existing inert cases (`puts {$v}`, a `$v`
/// inside a `#` comment — issue #923 idx 24), which every caller already
/// suppressed one layer further down; hoisting the proof here removes the
/// short-circuit that hid the declaration-span search behind it.
#[must_use]
pub(crate) fn substituting_var_at_position(
    source: &str,
    dialect: &str,
    line: u32,
    character: u32,
    cursor_off: u32,
) -> Option<String> {
    let name = find_var_at_position(source, line, character)?;
    (!offset_is_inert(source, dialect, cursor_off)).then_some(name)
}

/// Whether `off` sits inside a **literal** parameter-list word of a proc or
/// method — the word between its name token and its body.
///
/// Every word inside a literal parameter list is pure data: a parameter's own
/// name, or a default value.  Neither is a command reference, so bareword
/// command resolution must not run on it: `proc ::tk::RestoreFocusGrab {grab
/// focus {destroy destroy}}` had both the parameter name `destroy` and the
/// default-value literal `destroy` resolving to Tk's `destroy` **command**
/// (issue #923 differential-audit finding idx 104 — tclsh proves the bare word
/// in a parameter list never invokes anything).  A cursor on the parameter
/// *name* is answered earlier, by [`var_def_at_declaration_offset`]; this
/// guard is what stops the remaining data words falling through.
///
/// **Literal** is the load-bearing word.  `proc`'s parameter-list argument is
/// an ordinary Tcl word, so it may be *computed*, and then it holds live code
/// that must stay navigable.  Verified on tclsh 9.0.4 and 8.6.16:
///
/// ```tcl
/// proc makeargs {} { return {a b} }
/// proc p [makeargs] { return "p got $a $b" }
/// puts [p 1 2]          ;# p got 1 2
/// puts [info args p]    ;# a b
/// set params {x y}
/// proc q $params { … }  ;# works the same way
/// proc r "m n" { … }    ;# and so does a quoted one
/// ```
///
/// `[makeargs]` really is a call, made at definition time, so go-to-definition
/// and hover on it must work.  Only a braced word (`{a {b 1} args}` —
/// substitution suppressed outright) or a plain bareword with no substitution
/// syntax is treated as data; anything containing `$`, `[`, `"`, or a
/// backslash escape stays navigable.
pub(crate) fn offset_is_in_parameter_list(
    analysis: &AnalysisResult,
    source: &str,
    off: u32,
) -> bool {
    parameter_list_position_at(analysis, source, off) == ParamListPosition::LiteralData
}

/// What a cursor offset is, with respect to the parameter-list word of the
/// routine that encloses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamListPosition {
    /// Not inside any routine's parameter-list word.
    Outside,
    /// Inside a **literal** parameter list — pure data, no references.
    LiteralData,
    /// Inside a **computed** one (`proc p [makeargs] …`, `proc q $params …`,
    /// `proc r "m n" …`): live code that must stay navigable, and *not* a
    /// place a parameter declaration can be trusted either — the analyser
    /// records a stub `VarDef` named after the whole word (`"[makeargs]"`),
    /// which would otherwise make go-to-definition point at the cursor's own
    /// token instead of resolving the call.
    Computed,
}

/// Classify `off` against the parameter-list word of whichever proc or method
/// encloses it — see [`ParamListPosition`] and
/// [`offset_is_in_parameter_list`].
pub(crate) fn parameter_list_position_at(
    analysis: &AnalysisResult,
    source: &str,
    off: u32,
) -> ParamListPosition {
    let classify = |name_span: tcl_lexer::Span, body_span: tcl_lexer::Span| {
        param_word_position(source, name_span.end(), body_span.start(), off)
    };
    let routines = analysis
        .all_procs
        .values()
        .map(|p| (p.name_span, p.body_span));
    let members = analysis.all_classes.values().flat_map(|c| {
        c.methods
            .values()
            .chain(c.class_methods.values())
            .map(|m| (m.name_span, m.body_span))
    });
    routines
        .chain(members)
        .map(|(name_span, body_span)| classify(name_span, body_span))
        .find(|p| *p != ParamListPosition::Outside)
        .unwrap_or(ParamListPosition::Outside)
}

/// Classify `off` against the parameter-list word sitting in
/// `[region_start, region_end)` — a routine's name-token end to its body
/// start.  See [`offset_is_in_parameter_list`] for the oracle behind the
/// literal test.
fn param_word_position(
    source: &str,
    region_start: u32,
    region_end: u32,
    off: u32,
) -> ParamListPosition {
    let (start, end, off) = (region_start as usize, region_end as usize, off as usize);
    if start > end || off < start || off >= end || end > source.len() {
        return ParamListPosition::Outside;
    }
    // The parameter list is the first word of the region; skip the whitespace
    // the segmenter left between the name token and it.
    let bytes = source.as_bytes();
    let mut word_start = start;
    while word_start < end && matches!(bytes[word_start], b' ' | b'\t' | b'\r' | b'\n') {
        word_start += 1;
    }
    if word_start >= end {
        return ParamListPosition::Outside;
    }
    let word_end = crate::expr_context::word_end(source, word_start, end);
    if off < word_start || off >= word_end {
        return ParamListPosition::Outside;
    }
    let Some(word) = source.get(word_start..word_end) else {
        return ParamListPosition::Outside;
    };
    // The literalness rule is shared with the analyser and the signature-scan
    // tier (issue #1107) so the three cannot disagree about a word. The local
    // character scan this replaced called the substitution-free quoted list
    // `proc r "m n" {…}` computed, while the analyser — correctly, per the
    // oracle (`info args r` → `m n`) — treated it as literal.
    if tcl_compiler::signature_scan::params::param_word_text_is_literal(word) {
        ParamListPosition::LiteralData
    } else {
        ParamListPosition::Computed
    }
}

/// Collect every variable name visible at `byte_offset` — the union
/// of `variables` across the scope chain (innermost first, then
/// enclosing scopes up to the global root).  Used by variable
/// completion so a `$`-trigger inside a proc / namespace body offers
/// that scope's locals + params alongside the globals.
///
/// Inside an `uplevel #0 { … }` body the script runs in the global
/// frame, so the enclosing proc's locals are *not* visible — only the
/// global / namespace frame's variables plus the uplevel body's own
/// locals.  When the chain contains an [`ScopeKind::Uplevel`] scope,
/// proc-scope variables are dropped from the visible set.
pub(crate) fn visible_variable_names(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> Vec<String> {
    let chain = scope_chain_at(scope, byte_offset);
    let mut names: Vec<String> = Vec::new();
    for (i, sc) in chain.iter().enumerate().rev() {
        // Honour the `uplevel` frame semantics (see [`uplevel_hides_scope`]):
        // `#0` drops the enclosing proc's locals, a non-`#0` level drops
        // everything outside the uplevel body.
        if uplevel_hides_scope(&chain, i) {
            continue;
        }
        for k in sc.variables.keys() {
            if !names.iter().any(|n| n == k) {
                names.push(k.clone());
            }
        }
    }
    names
}

/// For completion: the set of variable names defined in the *global/root*
/// scope, returned only when the cursor sits where an unqualified reference
/// does NOT resolve to the global — inside a proc (unqualified names are the
/// proc's locals) or inside a nested `namespace eval` (they resolve within
/// that namespace).  Such a global must be offered `::`-qualified (`$::foo`)
/// so the inserted reference actually reaches it, matching Tcl's name
/// resolution.  Returns `None` at global scope, where bare names are correct.
///
/// `ns_global_fallback` carries the document dialect's TIP 278 semantics
/// (`AnalysisResult::ns_var_global_fallback`): under a Tcl 8.x dialect a
/// bare name at a namespace frame *does* reach an unshadowed global, so a
/// namespace context only forces qualification under 9.0+ semantics.  Proc
/// bodies never had the fallback in any version.
pub(crate) fn global_vars_needing_qualification(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
    ns_global_fallback: bool,
) -> Option<FxHashSet<String>> {
    use tcl_compiler::analyser::ScopeKind;
    let chain = scope_chain_at(scope, byte_offset);
    let in_local_context = chain.iter().any(|sc| {
        matches!(sc.kind, ScopeKind::Proc)
            || (matches!(sc.kind, ScopeKind::Namespace)
                && !sc.name.is_empty()
                && sc.name != "::"
                && !ns_global_fallback)
    });
    if !in_local_context {
        return None;
    }
    // Collect root/global names, but drop any that a closer (proc / namespace /
    // uplevel) scope also defines: a local of the same name *shadows* the
    // global, so the bare `$name` correctly resolves to the local and must not
    // be rewritten to `$::name` (which would silently retarget the reference).
    let mut globals = FxHashSet::default();
    let mut shadows = FxHashSet::default();
    for sc in &chain {
        if matches!(sc.kind, ScopeKind::Global) {
            globals.extend(sc.variables.keys().cloned());
        } else {
            shadows.extend(sc.variables.keys().cloned());
        }
    }
    for name in &shadows {
        globals.remove(name);
    }
    Some(globals)
}

/// Names of every namespace / global scope in the cursor's lexical chain.
/// Used by completion to skip cross-namespace candidates already offered as
/// bare names.
pub(crate) fn lexical_namespace_chain(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> FxHashSet<String> {
    use tcl_compiler::analyser::ScopeKind;
    let mut chain = FxHashSet::default();
    for sc in scope_chain_at(scope, byte_offset) {
        if matches!(sc.kind, ScopeKind::Namespace | ScopeKind::Global) {
            chain.insert(sc.name.clone());
        }
    }
    chain
}

/// Namespace an unqualified command invoked at `byte_offset` resolves
/// against (without the leading `::`), or `""` at global scope.  Used to
/// attribute an unqualified call to the proc in its own namespace when the
/// analyser's resolution falls back to the global guess.
///
/// Thin wrapper over [`tcl_compiler::analyser::command_resolution_namespace_at`]
/// — the single canonical implementation of Tcl's command-resolution
/// namespace rule, shared with the analyser's own `resolved_qualified_name`
/// computation (`Analyser::command_resolution_namespace`) so the two can
/// never disagree.  Correctly accumulates through every enclosing
/// `namespace eval` (however deeply nested — a bare, non-accumulating
/// "just the innermost segment" reading previously misidentified a 2+-level
/// nested namespace, e.g. `::a::b` read back as just `b`) and resets to a
/// proc's/method's own defining namespace inside its body, even when that
/// proc was declared with a fully-qualified name with no enclosing
/// `namespace eval` at all.
///
/// `overrides` (issue #923 idx 116) is checked *first*: a runtime-context
/// namespace pin (`apply {{params} body ns}` runs `body` in `ns`, not its
/// lexical home) that the ordinary span-containment walk above can never
/// see, since `handle_apply_command` roots that `Scope` subtree under
/// fresh, `body_span`-less namespace wrapper nodes. On multiple containing
/// entries (nested `apply` calls), the smallest span wins — innermost,
/// mirroring the lexical walk's own innermost-scope preference. Pass `&[]`
/// to opt out (e.g. variable lookups, which this fix deliberately doesn't
/// extend — see `lookup_var_in_scope_chain`).
pub(crate) fn innermost_namespace_at(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
    overrides: &[(tcl_lexer::Span, String)],
) -> String {
    let pinned = overrides
        .iter()
        .filter(|(span, _)| span.start() <= byte_offset && byte_offset < span.end())
        .min_by_key(|(span, _)| span.end() - span.start());
    if let Some((_, ns)) = pinned {
        return ns.trim_start_matches("::").to_string();
    }
    tcl_compiler::analyser::command_resolution_namespace_at(scope, byte_offset)
        .trim_start_matches("::")
        .to_string()
}

/// The `::`-prefixed namespace context at `byte_offset` (`"::"` at global
/// scope), in the shape [`tcl_syntax::naming::bareword_resolution_candidates`]
/// expects.  Built on [`innermost_namespace_at`] so call attribution here
/// agrees with the reference / rename gates
/// (`references::invocation_references_proc`).
pub(crate) fn namespace_context_at(
    scope: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
    overrides: &[(tcl_lexer::Span, String)],
) -> String {
    let ns = innermost_namespace_at(scope, byte_offset, overrides);
    if ns.is_empty() {
        "::".to_owned()
    } else {
        format!("::{ns}")
    }
}

/// The bare command word at `(line, character)` together with the
/// `::`-prefixed namespace in effect there.
///
/// For the autoload-definition fallback: when a command head resolves to
/// nothing in the current document *or* the workspace index, the caller feeds
/// this `(word, namespace)` to the package / auto-load database
/// ([`crate::package_resolver::PackageResolver::resolve_auto_command`]) to find
/// the library file that defines it.  Returns `None` off any word.  The word
/// is returned verbatim (the resolver auto-qualifies it against the
/// namespace); `namespace` is `"::"` at global scope.
#[must_use]
pub fn command_head_and_namespace_at(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<(String, String)> {
    let (word, _start, _end) = crate::hover::find_word_span_at_position(source, line, character)?;
    let line_index = LineIndex::new(source);
    let offset = byte_offset_at(&line_index, source, line, character);
    let namespace = namespace_context_at(
        &analysis.global_scope,
        offset,
        &analysis.namespace_overrides,
    );
    Some((word, namespace))
}

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
fn proc_visible_from_namespace<'a>(
    analysis: &'a AnalysisResult,
    namespace: &str,
    word: &str,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    let path = analysis
        .namespace_paths
        .get(namespace)
        .map_or(&[][..], Vec::as_slice);
    tcl_syntax::naming::command_resolution_candidates(namespace, path, word)
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
/// object-command / classmethod dispatch matcher (issue #981) and the [incr
/// Tcl] class-proc resolver (issue #990), so those two cannot disagree about
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
    tcl_syntax::naming::command_resolution_candidates(namespace, path, word)
        .into_iter()
        .find(|qname| exists(qname))
}

/// Whether `cd`'s definer command dispatches its class-scoped members as a
/// single `::`-qualified identifier (`Factory::make`) rather than the
/// two-word `Factory make` shape — true for [incr Tcl] only.  Registry data
/// ([`tcl_registry::definer::DefinerFamily`]), not a hardcoded command-name
/// check: `cd.metaclass` is looked up in `dialect`'s registry and its
/// attached [`tcl_registry::definer::DefinitionBodyGrammar::family`]
/// compared.  A metaclass the registry does not recognise as a definer
/// (which should not happen for a real `ClassDef`) is conservatively treated
/// as *not* itcl.
pub(crate) fn is_itcl_class(cd: &tcl_compiler::analyser::types::ClassDef, dialect: &str) -> bool {
    tcl_registry::registry_for_dialect(dialect)
        .get(&cd.metaclass)
        .and_then(|spec| spec.definition_body)
        .is_some_and(|g| g.family == tcl_registry::definer::DefinerFamily::Itcl)
}

/// Read `qname` as an [incr Tcl] class-scoped `proc` — itcl's equivalent of
/// `TclOO`'s `classmethod` — returning `(class qualified name, member name)`
/// when its parent namespace names an itcl class that declares that member.
///
/// itcl gives every class a real namespace of the same name and installs its
/// class-scoped `proc`s as ordinary commands inside it, so `::app::Factory`'s
/// `proc make` is genuinely the command `::app::Factory::make`.  Verified on
/// tclsh 8.6.14 with Itcl 3.4: `info commands ::Factory::*` lists it, and
/// `::app::Factory::make`, `app::Factory::make`, and a bare `Factory::make`
/// from inside `::app` all dispatch it.  An *instance* method is not
/// reachable this way — calling `::Factory::inst` errors `namespace "::" is
/// not a class namespace` — so only `class_methods` is consulted.
pub(crate) fn itcl_class_proc_at<'a>(
    analysis: &'a AnalysisResult,
    dialect: &str,
    qname: &str,
) -> Option<(&'a String, String)> {
    let separator = qname.rfind("::")?;
    let (parent, tail) = qname.split_at(separator);
    let member = tail.trim_start_matches(':');
    if member.is_empty() {
        return None;
    }
    let (class_q, class_def) = analysis.all_classes.get_key_value(parent)?;
    (is_itcl_class(class_def, dialect) && class_def.class_methods.contains_key(member))
        .then(|| (class_q, member.to_owned()))
}

/// Resolve a written command word to the [incr Tcl] class-scoped `proc` it
/// dispatches.
///
/// Two resolvers, in the order the interpreter applies them:
///
/// 1. [`itcl_self_qualified_candidate`] — itcl installs a **custom command
///    resolver** on each class namespace, and it is consulted *before* the
///    ordinary namespace lookup, so inside a class's own namespace the class's
///    simple name qualifies its members ahead of every stock candidate.
/// 2. [`resolved_command_name`] — stock Tcl resolution (current namespace,
///    `namespace path`, then global) for every other call site, so a call is
///    attributed to the class the runtime would actually reach and an ordinary
///    proc or class command shadows it exactly as it would at runtime.
pub(crate) fn itcl_class_proc_target<'a>(
    analysis: &'a AnalysisResult,
    dialect: &str,
    namespace: &str,
    word: &str,
) -> Option<(&'a String, String)> {
    let winner = itcl_self_qualified_candidate(analysis, dialect, namespace, word)
        // The custom resolver only claims a member the class really declares;
        // anything else falls through to the stock candidates (oracle: a
        // sibling class's `Other::omake`, and an undeclared `Factory::nope`,
        // both raise `invalid command name` rather than resolving).
        .filter(|candidate| itcl_class_proc_at(analysis, dialect, candidate).is_some())
        .or_else(|| {
            resolved_command_name(analysis, namespace, word, &|candidate| {
                analysis.all_procs.contains_key(candidate)
                    || analysis.all_classes.contains_key(candidate)
                    || itcl_class_proc_at(analysis, dialect, candidate).is_some()
            })
        })?;
    itcl_class_proc_at(analysis, dialect, &winner)
}

/// [incr Tcl]'s own class-namespace command resolver: written *inside* a
/// class's namespace — any method or class-`proc` body — the class's own
/// **simple** name qualifies its members, so `Factory::make` reaches
/// `::app::Factory::make` even though stock Tcl resolution would try only
/// `::app::Factory::Factory::make` and `::Factory::make`, neither of which
/// exists.
///
/// This resolver **pre-empts** the stock candidates rather than backing them
/// up: itcl installs a custom command resolver (`Itcl_ClassCmdResolver`) on
/// every class namespace, and Tcl consults a namespace's resolver *before* its
/// ordinary command lookup.  A real command at a stock candidate — even the
/// current-namespace-relative one — does **not** win.
///
/// Pinned against tclsh 8.6 + Itcl 3.4 (probes `itcl2.tcl`, `itcl2b.tcl`,
/// `itcl2c.tcl`), with a class `::app::Factory` declaring `proc make`:
///
/// | Written          | Called from            | Also defined                      | Dispatches |
/// |------------------|------------------------|-----------------------------------|------------|
/// | `Factory::make`  | the class's own bodies | `::Factory::make` (a plain proc)   | the **class proc** |
/// | `Factory::make`  | the class's own bodies | `::app::Factory::Factory::make`    | the **class proc** |
/// | `Factory::make`  | `::app`                | —                                 | the class proc, by stock relative lookup |
/// | `Factory::make`  | `::` or `::other`      | `::Factory::make`                 | the **plain proc** |
/// | `Other::omake`   | `::app::Factory`       | class `::app::Other` with `omake`  | nothing — `invalid command name` |
/// | `Factory::nope`  | `::app::Factory`       | —                                 | nothing — `invalid command name` |
///
/// So the rule covers the *enclosing* class only, and only for members it
/// declares; the ordinary spellings (`app::Factory::make`,
/// `::app::Factory::make`) keep resolving through
/// [`resolved_command_name`].
fn itcl_self_qualified_candidate(
    analysis: &AnalysisResult,
    dialect: &str,
    namespace: &str,
    word: &str,
) -> Option<String> {
    let class_def = analysis.all_classes.get(namespace)?;
    if !is_itcl_class(class_def, dialect) {
        return None;
    }
    let member = word
        .strip_prefix(class_def.name.as_str())?
        .strip_prefix("::")?;
    (!member.is_empty() && !member.contains("::"))
        .then(|| format!("{}::{member}", class_def.qualified_name))
}

/// The declaration name span of the class the cursor names: the class whose
/// own declaration name span contains `cursor_offset`, else the class `word`
/// refers to as a call or reference (`superclass X`, `X new`, …).
///
/// A document-local `oo::define` extension stub (`via_define`) is skipped
/// for the second case: the real `oo::class create` site lives in another
/// file and the cross-document resolver prefers it, so pinning navigation to
/// the local extension would shadow the true definition.  When the class *is*
/// created here, `via_define` is false and this is that creation site.
fn class_declaration_at(
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<tcl_lexer::Span> {
    if let Some((_, class_def)) = analysis
        .all_classes
        .iter()
        .find(|(_, c)| c.name_span.start() <= cursor_offset && cursor_offset < c.name_span.end())
    {
        return Some(class_def.name_span);
    }
    analysis
        .all_classes
        .iter()
        .find(|(qname, c)| {
            !c.via_define
                && (c.name == word || qname.as_str() == word || *qname == &format!("::{word}"))
        })
        .map(|(_, class_def)| class_def.name_span)
}

/// The declaration name span of the [incr Tcl] class-scoped `proc` that the
/// written word `word`, called from `namespace`, dispatches — go-to-definition's
/// half of [`itcl_class_proc_target`].
///
/// The empty dialect string picks the default Tcl registry, which is where
/// the itcl definer grammar lives; `definition` has no dialect of its own to
/// thread through.
fn itcl_class_proc_declaration(
    analysis: &AnalysisResult,
    namespace: &str,
    word: &str,
) -> Option<tcl_lexer::Span> {
    let (class_q, member) = itcl_class_proc_target(analysis, "", namespace, word)?;
    analysis
        .all_classes
        .get(class_q)
        .and_then(|cd| cd.class_methods.get(&member))
        .map(|decl| decl.name_span)
}

/// [`itcl_class_proc_target`] for the word under the cursor: the
/// `(class qualified name, member name)` of the [incr Tcl] class-scoped
/// `proc` a `Factory::make` call site at `(line, character)` dispatches.
///
/// The one cursor entry point shared by go-to-definition, Find All
/// References, rename, and the cross-file rename seed, so none of them can
/// disagree about which class-proc a given call site names.  Returns `None`
/// off any word, and for every spelling that is not this dispatch shape.
#[must_use]
pub fn itcl_class_proc_target_at(
    source: &str,
    dialect: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<(String, String)> {
    let (word, _start, _end) = crate::hover::find_word_span_at_position(source, line, character)?;
    let line_index = LineIndex::new(source);
    let cursor = byte_offset_at(&line_index, source, line, character);
    let namespace = namespace_context_at(
        &analysis.global_scope,
        cursor,
        &analysis.namespace_overrides,
    );
    itcl_class_proc_target(analysis, dialect, &namespace, &word)
        .map(|(class_q, member)| (class_q.clone(), member))
}

/// Resolve a bareword `word` (`namespace` context, honouring `namespace
/// path` exactly like [`proc_visible_from_namespace`]) through an
/// in-document `namespace import NS::*` — a glob (or exact) pattern
/// recorded in `analysis.namespace_imports`, as the call at `call_off` sees
/// it.
///
/// For each candidate qualified name Tcl's own lookup would try (the
/// caller's namespace, then each `namespace path` entry, then global — the
/// same order [`proc_visible_from_namespace`] walks), this checks whether
/// that candidate's namespace **still holds a live imported alias** for
/// `word` at `call_off` — see [`live_import_at`] for the lifecycle that
/// question covers (issue #1103), and
/// [`crate::namespace_import::exported_at_import_site`] for the export
/// snapshot each import site is judged by (issue #1027).
///
/// Only resolves a source namespace defined in **this** document
/// (`analysis.all_procs`); a source namespace defined in another file is
/// the cross-document oracle's job
/// (`tcl_lsp_core::workspace_index::WorkspaceIndex::resolve_wildcard_import`).
fn proc_visible_via_wildcard_import<'a>(
    analysis: &'a AnalysisResult,
    ctx: CallResolution<'_>,
    namespace: &str,
    word: &str,
    call_off: u32,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    let source_ns = wildcard_import_source_namespace(analysis, ctx, namespace, word, call_off)?;
    analysis
        .all_procs
        .get(&tcl_syntax::naming::qualify(&source_ns, word))
}

/// Whether a live `namespace import -force` covering `word` has taken the
/// name away from `namespace`'s own command table by the time the call at
/// `call_off` runs — the only import that outranks a same-named command the
/// importing namespace already holds.
///
/// Oracle (tclsh 8.6.14 / 9.0.4, byte-identical): with `proc ::dst::p`
/// returning `LOCAL` and `::src::p` returning `SRC`, `namespace eval ::dst
/// {namespace import -force ::src::*}` makes `::dst::p` return `SRC` and
/// `namespace origin ::dst::p` answer `::src::p`. Without `-force` the same
/// import raises `can't import command "p": already exists` and installs
/// nothing at all — which [`live_import_at`] models by declining the import
/// outright, so the two cases agree by construction.
///
/// Consulted *before* [`proc_visible_from_namespace`] in
/// [`resolve_called_proc`], because that is the whole point of `-force`:
/// from the import's own position onward, the local definition is gone. It
/// answers a plain boolean rather than a `ProcDef` because the shadow holds
/// whether or not this document can see the source — a `-force` import of a
/// namespace defined in *another* file still deletes the local command, and
/// answering with that local definition would be wrong; the cross-document
/// resolver takes it from there.
pub(crate) fn forced_import_shadows(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    namespace: &str,
    word: &str,
    call_off: u32,
) -> bool {
    if word.contains("::") {
        return false;
    }
    let path = analysis
        .namespace_paths
        .get(namespace)
        .map_or(&[][..], Vec::as_slice);
    import_hop(
        analysis,
        ctx,
        namespace,
        path,
        word,
        call_off,
        ImportQuery::ForcedShadow,
    )
    .is_some()
}

/// [`forced_import_shadows`] over a call's already-computed candidate list —
/// the shape the cross-document resolver has in hand.
///
/// `resolve_workspace_symbols` (`tcl-lsp-server`) is a separate resolver, not
/// a caller of [`resolve_called_proc`], so it needs its own application of the
/// same rule: a candidate naming this document's own `proc bar` must not
/// settle the call while a live `namespace import -force ::Lib::*` has
/// replaced it — the call reaches `::Lib::bar`, which only the workspace tier
/// can find. Without this the two resolvers disagree, and the one that answers
/// first wins with the definition the program no longer has.
#[must_use]
pub fn forced_import_shadows_call(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    word: &str,
    resolution_candidates: &[String],
    call_off: u32,
) -> bool {
    if word.contains("::") {
        return false;
    }
    live_import_over_candidates(
        analysis,
        ctx,
        resolution_candidates.iter().map(String::as_str),
        word,
        call_off,
        ImportQuery::ForcedShadow,
    )
    .is_some()
}

/// Which question [`import_hop`] / [`live_import_at`] are being asked, and
/// therefore what an *absent* `namespace export` record means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportQuery {
    /// "Which namespace does this call really reach?" Every import counts,
    /// and the export gate is strict: only a proven export
    /// ([`ExportVerdict::Exported`]) installs, because an unproven one gives
    /// nothing to resolve *to*.
    Resolve,
    /// "Has a `namespace import -force` taken this name away from the
    /// importing namespace's own command table?" ([`forced_import_shadows`])
    /// Only forced imports count, and the export gate abstains the other way:
    /// everything but a proven [`ExportVerdict::NotExported`] counts as having
    /// installed. `namespace import -force ::Lib::*` deletes the local command
    /// whether or not anything here can see `::Lib`'s export declaration, and
    /// answering with the deleted definition would be worse than abstaining.
    /// The same "absence of an export is evidence only where the exports are
    /// visible too" rule `WorkspaceIndex::live_command_links` applies
    /// cross-document.
    ForcedShadow,
}

impl ImportQuery {
    /// Whether an import whose export gate returned `verdict` counts as having
    /// installed, for this question.
    ///
    /// The two arms are the two abstention directions in one place, so neither
    /// can drift: [`Resolve`](Self::Resolve) needs proof to answer,
    /// [`ForcedShadow`](Self::ForcedShadow) needs proof to *stop* answering.
    fn installs(self, verdict: ExportVerdict) -> bool {
        match self {
            ImportQuery::Resolve => verdict == ExportVerdict::Exported,
            ImportQuery::ForcedShadow => verdict != ExportVerdict::NotExported,
        }
    }
}

/// Whether `source_ns` had exported `word` when the import at `import_at` ran,
/// as everything available to this resolution can tell — the single decision
/// both [`ImportQuery`] arms read, differing only in which way they abstain.
///
/// Three sources of evidence, in order of authority:
///
/// 1. **This document's own export records.** A covering export that had run
///    at the import is proof, and it is proof even when the workspace index is
///    stale relative to the buffer being edited — which is why it is consulted
///    first rather than left to the oracle.
/// 2. **The whole-program oracle**, when one is attached. It sees every
///    indexed file's exports, ranks them against this import with the `source`
///    graph's run order, and may itself answer
///    [`ExportVerdict::Unknown`].
/// 3. **The document-only fallback**, when no oracle is attached: absence of
///    an export is evidence exactly where this document holds *some* export
///    record for `source_ns`, and no information otherwise.
///
/// # Why an oracle at all (issue #1116 item 1)
///
/// Step 3 is the whole problem. It is the in-document twin of
/// `WorkspaceIndex::observable_namespaces`, and cross-document that framing is
/// right — the index really does hold every file. In *one* document it is not:
/// a namespace can be partly visible here (its procs, and some of its exports)
/// while the one `namespace export` that covers this name sits in another
/// file. The false-positive and the true-negative then have **byte-identical**
/// single-document inputs and opposite correct answers (transcript on
/// [`NamespaceExportOracle`]), so no rule reading only this document can
/// separate them — hence the oracle, and hence its optionality: a caller with
/// no workspace keeps step 3 exactly as it was.
fn export_verdict(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    source_ns: &str,
    word: &str,
    import_at: u32,
) -> ExportVerdict {
    if exported_at_import(analysis, ctx, source_ns, word, import_at) {
        return ExportVerdict::Exported;
    }
    if let Some(program) = ctx.program {
        return program.oracle.exported_at(
            source_ns,
            word,
            in_document_point(analysis, ctx, import_at),
        );
    }
    if namespace_exports_observable(analysis, source_ns) {
        ExportVerdict::NotExported
    } else {
        ExportVerdict::Unknown
    }
}

/// Whether this document holds *any* `namespace export` record for
/// `source_ns` — the document-only fallback [`export_verdict`] step 3 uses
/// when no whole-program oracle is attached.
///
/// The proposition being asserted is about the export list, so the evidence
/// has to be the export list: a document that declares no export at all for
/// `source_ns` knows nothing about what it exports. Its **residual** — a
/// document holding some of `source_ns`'s exports but not the covering one
/// still reads the gap as a fact — is precisely what the oracle removes when
/// one is available, and what remains, deliberately, when one is not.
fn namespace_exports_observable(analysis: &AnalysisResult, source_ns: &str) -> bool {
    let bare = source_ns.trim_start_matches("::");
    analysis
        .namespace_exports
        .iter()
        .any(|e| e.ns.trim_start_matches("::") == bare)
}

/// Shared candidate walk behind [`proc_visible_via_wildcard_import`],
/// [`proc_visible_via_forced_import`] and [`resolve_class_target_at`]'s
/// import fallback: find the namespace whose command `word` really names
/// when the call at `call_off` reaches it through one or more import edges,
/// or `None`.
///
/// The caller joins the result with `word` ([`tcl_syntax::naming::qualify`])
/// and looks the qualified name up in whichever map (`all_procs` /
/// `all_classes`) applies — which is also the map this walk consults to
/// decide whether a chain has terminated.
///
/// Restricted to a genuine bareword `word` (no embedded `::`) — `namespace
/// import` / `namespace export` patterns only ever name simple command
/// tails, matching the bug's scope (issue #923 idx 18); a qualified call
/// (`inner::p`) never goes through an imported alias in real Tcl, so this
/// abstains rather than risk mismatching a namespace-path segment against
/// an unrelated import.
///
/// # Import chains
///
/// An import edge may land on a name that is *itself* imported: with `::C`
/// exporting `p`, `::B` importing `::C::*` and re-exporting, and `::A`
/// importing `::B::*`, `::A::p` runs `::C`'s body and `namespace origin
/// ::A::p` answers `::C::p` (oracle tclsh 8.6.14 / 9.0.4 — issue #1103).
/// Statically the middle hop is in no `all_procs`, so a single-hop walk
/// found nothing. This one follows the chain while every hop is provable,
/// bounded by [`indirection::MAX_COMMAND_NAME_HOPS`] — the same cap the
/// `rename` / `interp alias` walk applies to the same kind of chain, so a
/// mutually-importing pair cannot spin — and abstains otherwise.
///
/// The whole chain is judged at the *call's* offset, not at each import's:
/// a forget anywhere along it kills the call (oracle: forgetting `::C::p`
/// inside `::B` makes `::A::p` an `invalid command name` too, because
/// deleting an imported command deletes the commands imported *from* it).
fn wildcard_import_source_namespace(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    namespace: &str,
    word: &str,
    call_off: u32,
) -> Option<String> {
    if word.contains("::") {
        return None;
    }
    let mut current = namespace.to_owned();
    let mut first_hop = true;
    for _ in 0..indirection::MAX_COMMAND_NAME_HOPS {
        let path = if first_hop {
            analysis
                .namespace_paths
                .get(current.as_str())
                .map_or(&[][..], Vec::as_slice)
        } else {
            // A chain's inner hop starts from the previous hop's source
            // namespace, where the alias is an ordinary command of that
            // namespace — `namespace path` is a *caller*-side search rule and
            // has no bearing on it.
            &[][..]
        };
        let hop = import_hop(
            analysis,
            ctx,
            &current,
            path,
            word,
            call_off,
            ImportQuery::Resolve,
        )?;
        // The chain terminates where a real definition lives: everything the
        // caller can look the name up in.
        let qualified = tcl_syntax::naming::qualify(&hop, word);
        if analysis.all_procs.contains_key(&qualified)
            || analysis.all_classes.contains_key(&qualified)
        {
            return Some(hop);
        }
        current = hop;
        first_hop = false;
    }
    None
}

/// One hop of [`wildcard_import_source_namespace`]: the source namespace of
/// the live import that makes `word` callable from `namespace` (or one of its
/// `path` entries) at `call_off`.
fn import_hop<S: AsRef<str>>(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    namespace: &str,
    path: &[S],
    word: &str,
    call_off: u32,
    query: ImportQuery,
) -> Option<String> {
    let candidates = tcl_syntax::naming::command_resolution_candidates(namespace, path, word);
    live_import_over_candidates(
        analysis,
        ctx,
        candidates.iter().map(String::as_str),
        word,
        call_off,
        query,
    )
}

/// [`import_hop`] over an already-computed candidate list: the first candidate
/// namespace holding a live import edge for `word` wins, in Tcl's own
/// resolution order.
fn live_import_over_candidates<'c>(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    candidates: impl Iterator<Item = &'c str>,
    word: &str,
    call_off: u32,
    query: ImportQuery,
) -> Option<String> {
    for candidate in candidates {
        let Some((prefix, tail)) = candidate.rsplit_once("::") else {
            continue;
        };
        if tail != word {
            continue;
        }
        let candidate_ns = if prefix.is_empty() { "::" } else { prefix };
        if let Some(hop) = live_import_at(analysis, ctx, candidate_ns, word, call_off, query) {
            return Some(hop);
        }
    }
    None
}

/// One ordered event on an importing namespace's `word` slot, as the
/// same-document tier sees it.
///
/// The four things that can happen to `<importing_ns>::<word>` over the life
/// of a script. Folding them in order answers both questions
/// [`live_import_at`] asks: *what did this namespace already hold when import
/// I ran?* (the non-`-force` conflict) and *what does it hold at the call?*
#[derive(Debug, Clone, Copy)]
enum SlotEvent<'a> {
    /// A `namespace import` covering `word` whose export snapshot passed.
    Import {
        at: u32,
        source_ns: &'a str,
        forced: bool,
    },
    /// A `namespace forget` covering `word`; `source_ns` is `None` for the
    /// simple pattern form, which matches whatever the alias was imported
    /// from.
    Forget { at: u32, source_ns: Option<&'a str> },
    /// A `proc` / class declaration of `<importing_ns>::<word>` — the
    /// namespace's *own* command of that name.
    Declare { at: u32 },
    /// A destroying `rename <source_ns>::<word> {}` — the command object the
    /// alias holds is gone.
    Destroy { at: u32, source_ns: &'a str },
}

impl SlotEvent<'_> {
    fn at(self) -> u32 {
        match self {
            SlotEvent::Import { at, .. }
            | SlotEvent::Forget { at, .. }
            | SlotEvent::Declare { at }
            | SlotEvent::Destroy { at, .. } => at,
        }
    }
}

/// Whether a forget event covers an alias taken from `source_ns`.
fn forget_covers(forget_source: Option<&str>, source_ns: &str) -> bool {
    forget_source
        .is_none_or(|src| src.trim_start_matches("::") == source_ns.trim_start_matches("::"))
}

/// The source namespace of the import alias `importing_ns` holds for `word`
/// **at `call_off`**, or `None` when it holds none — the same-document
/// binding of the shared lifecycle decision
/// [`crate::namespace_import::alias_live_at`] (issue #1103).
///
/// An import edge is a link with a lifecycle, not a standing name-visibility
/// fact. Everything that writes the `<importing_ns>::<word>` slot is an event
/// on one ordered log ([`SlotEvent`]); all of it is oracle-confirmed
/// byte-identically on tclsh 9.0.4 and 8.6.14:
///
/// 1. **The import installs it** — gated by the source namespace's export
///    snapshot at the import's own position
///    ([`exported_at_import`], issue #1027).
/// 2. **A conflict makes it install nothing.** Without `-force`, importing
///    onto a name the target namespace already holds raises `can't import
///    command "p": already exists`; the existing command survives and
///    `namespace origin` still answers it. "Already holds" covers a local
///    `proc`/class **and a live alias from a different source**: with `::dst`
///    already importing `::A::*`, a later unforced `namespace import ::B::*`
///    fails with exactly that error and leaves `namespace origin ::dst::p` →
///    `::A::p` (oracle). Re-importing from the *same* source is a silent
///    no-op instead, so it is not a conflict. With `-force` the import wins
///    and replaces whichever of the two was there (oracle: `origin` → `::B::p`),
///    which is what [`ImportQuery::ForcedShadow`] selects for. (Modelling the
///    *error* — that the rest of that script never runs — is deliberately out
///    of scope; only the "installed nothing" half is modelled.)
/// 3. **`namespace forget` removes it** — `namespace forget ::src::p`, or the
///    simple form `namespace forget p`, and a later bare call is `invalid
///    command name`.
/// 4. **Deleting the source command removes it** — the alias holds the
///    command *object*, so `rename ::src::p {}` kills `::dst::p` too, while a
///    plain `rename ::src::p ::src::pp` leaves it alive (only the origin
///    moves) and a redefinition of `::src::p` is seen straight through the
///    link. That asymmetry is `indirection`'s own
///    rename-captures-object-identity rule, which is why only the *deletion*
///    appears here.
/// 5. **Redefining the imported name removes it.** A `proc ::dst::p` written
///    after the import silently recreates `::dst::p` as an ordinary command:
///    oracle (9.0.4 / 8.6.14, `namespace import -force ::src::*` then `proc
///    ::dst::p`) — the redefinition raises no error, `::dst::p` runs the new
///    body, `namespace origin ::dst::p` becomes `::dst::p`, and `::src::p`
///    is untouched. A call written *between* the import and the redefinition
///    still reaches `::src::p`, so this is an ordered event like every other,
///    not a file-wide fact.
///
/// Ordering is [`indirection::in_effect`] — the same "had this statement
/// run?" primitive the export snapshot and the `rename` / `interp alias`
/// timeline use, so none of them can disagree about what "already ran" means.
/// It gates the *installs* as well as the removals (issue #1104 item 1): a
/// bare call written before its own `namespace import` reaches nothing
/// (oracle on [`crate::namespace_import::alias_live_at`]), and because the
/// primitive is `in_effect` rather than a raw offset test, a call inside a
/// proc body still observes a top-level import written later in the same file
/// — the whole file loads before any body runs.
///
/// The conflict fold below is deliberately *not* gated that way: which import
/// installs anything is decided at each import's **own** position, wherever
/// the call sits. Only the choice of winning edge, and the edge's own
/// lifecycle, are judged from the call.
fn live_import_at(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    importing_ns: &str,
    word: &str,
    call_off: u32,
    query: ImportQuery,
) -> Option<String> {
    use crate::namespace_import::{AliasEvent, AliasEventKind};
    let events = slot_events(analysis, ctx, importing_ns, word, query);
    // Which imports actually install something: an unforced import onto a
    // slot the namespace already holds (a local declaration, or a live alias
    // from a *different* source) raises `already exists` and binds nothing.
    // Decided by folding the log in order, each import judged by what the
    // slot held when *it* ran — the same question the call-site gate below
    // asks at its own point.
    let mut held: Option<&str> = None;
    let mut declared = false;
    let mut installs: Vec<(u32, &str)> = Vec::new();
    let mut latest: Option<(u32, &str)> = None;
    for ev in &events {
        match *ev {
            SlotEvent::Declare { .. } => {
                declared = true;
                held = None;
            }
            SlotEvent::Forget { source_ns, .. } => {
                if held.is_some_and(|src| forget_covers(source_ns, src)) {
                    held = None;
                }
            }
            SlotEvent::Destroy { source_ns, .. } => {
                if held == Some(source_ns) {
                    held = None;
                }
            }
            SlotEvent::Import {
                at,
                source_ns,
                forced,
            } => {
                let conflicts = declared || held.is_some_and(|current| current != source_ns);
                if !forced && conflicts {
                    continue;
                }
                held = Some(source_ns);
                // A `-force` import replaces whatever was there, including a
                // local declaration.
                declared = false;
                installs.push((at, source_ns));
                // The latest install *that has already run at the call* wins
                // the source namespace (issue #1104 item 1). An import the
                // call has not reached yet cannot decide which edge the call
                // takes — the same gate `alias_live_at` then applies to the
                // edge's own log, so the two cannot pick different imports.
                if indirection::in_effect(analysis, at, call_off)
                    && latest.is_none_or(|(best, _)| at > best)
                {
                    latest = Some((at, source_ns));
                }
            }
        }
    }
    let (_, source_ns) = latest?;
    // Only the winning source's own installs order against its removals: an
    // import of the same name from a *different* namespace is a different
    // edge, and letting it out-date this edge's forget would resurrect it.
    // The install carries its enclosing body, the removals do not — not an
    // oversight: the fold reads a body span only off the point it is ranking
    // *against* (`ran_after(removal, install)` asks whether the removal had run
    // at the install), and a removal is never that point. Computing one per
    // removal would be a body scan that decides nothing.
    let install_point = |at: u32| in_document_point(analysis, ctx, at);
    let removal_point = |at: u32| RunPoint {
        uri: ctx.uri(),
        at,
        enclosing_body: None,
    };
    let mut log = events
        .iter()
        .filter_map(|ev| match *ev {
            SlotEvent::Import {
                at, source_ns: s, ..
            } if s == source_ns => installs
                .iter()
                .any(|&(i, _)| i == at)
                .then_some(AliasEvent {
                    kind: AliasEventKind::Install,
                    at: install_point(at),
                }),
            SlotEvent::Forget { at, source_ns: s } if forget_covers(s, source_ns) => {
                Some(AliasEvent {
                    kind: AliasEventKind::Remove,
                    at: removal_point(at),
                })
            }
            SlotEvent::Destroy { at, source_ns: s } if s == source_ns => Some(AliasEvent {
                kind: AliasEventKind::Remove,
                at: removal_point(at),
            }),
            SlotEvent::Declare { at } => Some(AliasEvent {
                kind: AliasEventKind::Remove,
                at: removal_point(at),
            }),
            _ => None,
        })
        .collect::<Vec<_>>()
        .into_iter();
    crate::namespace_import::alias_live_at(
        &mut log,
        &RunOrder::default(),
        Some(in_document_point(analysis, ctx, call_off)),
    )
    .then(|| source_ns.to_owned())
}

/// The document key every in-document import event carries **when no
/// workspace view is attached**.
///
/// This tier holds exactly one document, so the *identity* of the key is
/// irrelevant — what matters is that every point shares it, which is what makes
/// [`crate::source_graph::RunOrder`] fall through to the single-document rule
/// ([`tcl_compiler::analyser::indirection::in_effect_within`]) without
/// consulting any graph. The cross-document tier passes real URIs to the same
/// functions, so the two cannot drift.
/// The order the in-document tier passes with them: **empty**. One document
/// needs no `source` graph — every point shares [`IN_DOCUMENT_URI`], so
/// [`crate::source_graph::RunOrder`] answers from the single-document rule
/// alone and never looks at an edge. Building it allocates nothing.
/// With a [`ProgramExports`] view attached the points carry the document's
/// **real** URI instead, so the workspace's own order can rank a foreign
/// export against them — and, just as importantly, can rank *this* document's
/// exports against them by offset rather than shrugging.
const IN_DOCUMENT_URI: &str = "";

/// An offset in the document being analysed, as a
/// [`crate::source_graph::RunPoint`] — with the enclosing definition body the
/// load-order rule needs, which is the only thing
/// [`indirection::in_effect`] reads out of the analysis.
fn in_document_point<'a>(
    analysis: &AnalysisResult,
    ctx: CallResolution<'a>,
    at: u32,
) -> RunPoint<'a> {
    RunPoint {
        uri: ctx.uri(),
        at,
        enclosing_body: analysis.innermost_definition_body_span(at),
    }
}

/// Every [`SlotEvent`] bearing on `<importing_ns>::<word>` in this document,
/// in **execution** order.
///
/// Ordering the four record kinds against each other once, here, is what lets
/// [`live_import_at`] answer the conflict question and the call-site question
/// with the same log instead of two ad-hoc scans.
///
/// # Why not source order
///
/// The whole file loads — running every top-level statement — before any
/// proc/class body runs, so a top-level statement written *below* a
/// body-local one still happens first. That is the same load-order rule
/// [`indirection::in_effect`] states pointwise; sorted here it is the total
/// order the conflict fold needs. Oracle, byte-identical on tclsh 8.6.14 and
/// 9.0.4:
///
/// ```tcl
/// namespace eval ::A { proc x {} {return A}; namespace export x }
/// namespace eval ::B { proc x {} {return B}; namespace export x }
/// namespace eval ::dst { proc p {} { namespace import ::B::x } }
/// namespace eval ::dst { namespace import ::A::* }
/// namespace origin ::dst::x   ;# → ::A::x
/// ::dst::p                    ;# → can't import command "x": already exists
/// namespace origin ::dst::x   ;# → ::A::x   (unchanged)
/// ```
///
/// Folding that log in raw offset order got *both* halves wrong: it let the
/// body-local `::B` import install, and then conflicted the top-level `::A`
/// one away. Within one tier — all top level, or all inside one body — the
/// key degenerates to the offset comparison it replaces, so genuinely
/// sequenced statements are unaffected. Two events in *different* bodies have
/// no static order either way; they keep their offset order, as before.
///
/// The workspace tier reaches the same answer through
/// `WildcardImportIndex::conflicting_alias_at`, which asks
/// [`indirection::in_effect_within`] per candidate rather than sorting a log —
/// same rule, stated over the facts each tier holds.
fn slot_events<'a>(
    analysis: &'a AnalysisResult,
    ctx: CallResolution<'_>,
    importing_ns: &str,
    word: &str,
    query: ImportQuery,
) -> Vec<SlotEvent<'a>> {
    let mut events: Vec<SlotEvent<'a>> = Vec::new();
    for imp in analysis
        .namespace_imports
        .iter()
        .filter(|i| i.ns == importing_ns)
    {
        if query == ImportQuery::ForcedShadow && !imp.forced {
            continue;
        }
        let Some((source_ns, export_tail)) = imp.pattern.rsplit_once("::") else {
            continue;
        };
        // A pattern rooted at the global namespace (`namespace import ::p`,
        // `namespace import ::*`) splits to an *empty* source namespace, which
        // both tiers used to read as "no source" and skip — leaving the one
        // import shape that bypassed the export gate entirely (#1104's review
        // note). Real Tcl gives it no special treatment: `proc p {}` with no
        // `namespace export p` at global level makes `namespace import ::p` a
        // silent no-op like any other unexported import, and `info commands
        // ::dst::*` stays empty (oracle 8.6.14 / 9.0.4). `::` *is* the source
        // namespace — the spelling every export record for global scope
        // already uses.
        let source_ns = if source_ns.is_empty() {
            "::"
        } else {
            source_ns
        };
        if !tcl_syntax::glob::string_match(export_tail, word) {
            continue;
        }
        let at = imp.range.start();
        if !query.installs(export_verdict(analysis, ctx, source_ns, word, at)) {
            continue;
        }
        events.push(SlotEvent::Import {
            at,
            source_ns,
            forced: imp.forced,
        });
        // The source command's own destruction, ordered against this edge.
        if let Some(&at) = analysis
            .destroyed_commands
            .get(&tcl_syntax::naming::qualify(source_ns, word))
        {
            events.push(SlotEvent::Destroy { at, source_ns });
        }
    }
    for f in analysis
        .namespace_forgets
        .iter()
        .filter(|f| f.ns == importing_ns && tcl_syntax::glob::string_match(&f.pattern, word))
    {
        events.push(SlotEvent::Forget {
            at: f.range.start(),
            source_ns: f.source_ns.as_deref(),
        });
    }
    let qualified = tcl_syntax::naming::qualify(importing_ns, word);
    for at in analysis
        .proc_declarations(&qualified)
        .map(|p| p.name_span.start())
        .chain(
            analysis
                .all_classes
                .get(&qualified)
                .map(|c| c.name_span.start()),
        )
    {
        events.push(SlotEvent::Declare { at });
    }
    // Load level first, then bodies — see the doc comment. The key is the one
    // the cross-document tier compares its two import sites with, so neither
    // can drift. `sort_by_cached_key` because the body test is a scan of the
    // recorded definition bodies, so it must run once per event, not once per
    // comparison. Equal offsets imply equal body-ness, so the dedup below
    // still sees its duplicates adjacent.
    events.sort_by_cached_key(|ev| {
        crate::namespace_import::load_order(
            ev.at(),
            analysis.offset_is_inside_any_definition_body(ev.at()),
        )
    });
    events.dedup_by_key(|ev| (ev.at(), std::mem::discriminant(ev)));
    events
}

/// Whether `source_ns` had exported `word` by the time the `namespace import`
/// at `import_at` ran — the same-document binding of the shared decision
/// function [`crate::namespace_import::exported_at_import_site`].
///
/// Every export event in this document is ordered against the import, so all
/// of them carry an `at`; the "had it run?" primitive is
/// [`tcl_compiler::analyser::indirection::in_effect`], reused rather than
/// re-derived so an export written inside a proc body is judged by exactly the
/// same load-order rule the `rename` / `interp alias` timeline applies (the
/// whole file loads before any body runs, but a statement of the *same* body
/// stays offset-ordered).
///
/// `pub(crate)` because `references.rs` asks the identical question about the
/// identical records — one decision, not two.
pub(crate) fn exported_at_import(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    source_ns: &str,
    word: &str,
    import_at: u32,
) -> bool {
    let mut events = analysis
        .namespace_exports
        .iter()
        .filter(|e| e.ns == source_ns)
        .map(|e| crate::namespace_import::ExportEvent {
            pattern: &e.pattern,
            clears: e.clears,
            // No enclosing-body span: the index stores none per export row
            // either, so both tiers rank a `-clear` against a pattern by
            // position alone. Symmetric, and the safe direction — a
            // body-local export the tombstone cannot be proven to follow
            // keeps the name exported.
            at: RunPoint {
                uri: ctx.uri(),
                at: e.range.start(),
                enclosing_body: None,
            },
        });
    crate::namespace_import::exported_at_import_site(
        &mut events,
        word,
        &RunOrder::default(),
        in_document_point(analysis, ctx, import_at),
    )
}

/// Whether the call at `call_off` in `namespace` reaches `def_qualified`
/// through a live import edge — the entry point `references.rs` shares with
/// go-to-definition so the two cannot disagree about an import's lifecycle.
///
/// Returns the terminal source namespace the chain lands on, which the caller
/// compares against the definition it is gathering references for.
pub(crate) fn import_chain_target(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    namespace: &str,
    word: &str,
    call_off: u32,
) -> Option<String> {
    wildcard_import_source_namespace(analysis, ctx, namespace, word, call_off)
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
/// export oracle the `-force` shadow needs (issue #1116 item 1). Both absent
/// is the document-only behaviour this resolver has always had.
pub(crate) fn resolve_called_proc<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    namespace: &str,
    word: &str,
    call_off: u32,
    ctx: CallResolution<'_>,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
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
    if !forced_shadow && let Some(proc_def) = proc_visible_from_namespace(analysis, namespace, word)
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
    // import gate has just refused (issue #1104 item 2).
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
/// gate (issue #1104 item 2).
///
/// The false positive it closes is #1027's Direction B, still visible through
/// single-file go-to-definition after #1102 gated everything else: with
///
/// ```tcl
/// namespace eval ::src { proc p {} {return P} }
/// namespace eval ::dst { namespace import ::src::* }
/// namespace eval ::src { namespace export p }
/// namespace eval ::dst { p }        ;# → invalid command name "p"
/// ```
///
/// every ordered route said "no" and the tail match answered `::src::p`
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
    if word.contains("::") {
        return false;
    }
    let Some((target_ns, tail)) = target_qname.rsplit_once("::") else {
        return false;
    };
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
        tcl_syntax::naming::command_resolution_candidates(namespace, path, word)
            .into_iter()
            .filter_map(|cand| {
                cand.rsplit_once("::").map(|(prefix, _)| {
                    if prefix.is_empty() {
                        "::".to_owned()
                    } else {
                        prefix.to_owned()
                    }
                })
            })
            .collect();
    let covered = analysis.namespace_imports.iter().any(|imp| {
        in_scope.contains(&imp.ns)
            && imp.pattern.rsplit_once("::").is_some_and(|(src, pat)| {
                let src = if src.is_empty() { "::" } else { src };
                same_namespace(src, target_ns) && tcl_syntax::glob::string_match(pat, word)
            })
    });
    covered
        && wildcard_import_source_namespace(analysis, ctx, namespace, word, call_off)
            .is_none_or(|live| !same_namespace(&live, target_ns))
}

/// Namespace-name equality, ignoring the leading `::` one spelling carries and
/// the other does not.
fn same_namespace(a: &str, b: &str) -> bool {
    a.trim_start_matches("::") == b.trim_start_matches("::")
}

/// Resolve the `(all_procs key, ProcDef)` that a proc-oriented editor
/// operation (rename / references / call-hierarchy) targets at `cursor_off`:
///
/// 1. the proc whose declaration name span covers the cursor (a declaration
///    -site invocation), else
/// 2. the namespace-aware call-site resolution ([`resolve_called_proc`], C
///    Tcl's own rule from the caller's namespace).
///
/// It never falls back to a namespace-blind `p.name == word` scan — the shape
/// that let a rename triggered from a bareword call site pick an *arbitrary*
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
    // `tcl-lsp-server::resolve_workspace_symbols` applies cross-document
    // (issue #923 idx 31 / idx 45).
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
    // from the declaration already did (issue #923 idx 30: the asymmetry was
    // this resolution being missing here). `word` is unusable at such a
    // cursor anyway — the word-span scan hands back `li(`.
    if let Some(inv) = crate::expr_context::mathfunc_call_at(analysis, cursor_off) {
        return inv
            .resolved_qualified_name
            .as_deref()
            .and_then(|resolved| analysis.all_procs.get_key_value(resolved));
    }
    // A word the command table has moved onto (a `rename` destination, an
    // `interp alias` name) targets the definition it really reaches, so
    // find-references / rename / call-hierarchy issued from either spelling
    // converge on one proc (issues #1062 B1/B2, #1064). Order-gated, so a
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
    ctx: CallResolution<'_>,
    cursor_off: u32,
    word: &str,
) -> Option<(&'a String, &'a tcl_compiler::analyser::ClassDef)> {
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
    // (issue #1064; the diagnostics side of the same fact is
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
    // rationale (issue #923 idx 18); classes never consult `namespace path`,
    // matching `bareword_resolution_candidates` above.
    let source_ns = wildcard_import_source_namespace(analysis, ctx, &ns, word, cursor_off)?;
    analysis
        .all_classes
        .get_key_value(&tcl_syntax::naming::qualify(&source_ns, word))
}

/// Deterministic replacement for the old first-`HashMap`-hit tail match: of
/// every proc whose simple name equals `word`, prefer one defined in this
/// document ([`name_token_in_document`]), then the lexicographically
/// smallest qualified name.  `HashMap` iteration order never decides the
/// result.
///
/// A proc living in a `tcl::mathfunc` namespace is excluded: real Tcl reaches
/// it only through `expr`'s function-call production or its own fully-
/// qualified name, never as a bare command word (`li {10 20 30} 1` →
/// `invalid command name "li"` on tclsh 8.6 and 9.0 — issue #923 idx 30's
/// secondary false positive, where this lenient tail match resolved a
/// guaranteed-to-crash bare call to the mathfunc proc).  The namespace fact
/// is the registry's ([`tcl_registry::mathfunc::is_in_mathfunc_namespace`]),
/// not a name test written here.
pub(crate) fn fallback_proc_by_simple_name<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    word: &str,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
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

/// Fully-qualified `::ns::var` form for a var stored in a namespace / global
/// scope.
fn qualified_var_name(scope: &tcl_compiler::analyser::Scope, var: &str) -> String {
    use tcl_compiler::analyser::ScopeKind;
    if var.starts_with("::") {
        var.to_string()
    } else if scope.kind == ScopeKind::Global {
        format!("::{var}")
    } else {
        format!("{}::{var}", scope.name)
    }
}

/// Walk the scope tree and return the fully-qualified names of every
/// namespace / global variable whose enclosing namespace is not in the
/// cursor's lexical `chain` (proc locals excluded).
pub(crate) fn cross_namespace_qualified_vars(
    global: &tcl_compiler::analyser::Scope,
    chain: &FxHashSet<String>,
) -> Vec<String> {
    use tcl_compiler::analyser::{Scope, ScopeKind};
    fn visit(scope: &Scope, chain: &FxHashSet<String>, out: &mut Vec<String>, depth: u32) {
        if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
            return;
        }
        if matches!(scope.kind, ScopeKind::Namespace | ScopeKind::Global)
            && !chain.contains(&scope.name)
        {
            for vname in scope.variables.keys() {
                out.push(qualified_var_name(scope, vname));
            }
        }
        for child in &scope.children {
            visit(child, chain, out, depth + 1);
        }
    }
    let mut out = Vec::new();
    visit(global, chain, &mut out, 0);
    out
}

/// Return the chain of scopes from `root` down to the
/// innermost child whose `body_span` contains `byte_offset`.
/// The chain is ordered outermost (`root`) to innermost.
fn scope_chain_at(
    root: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> Vec<&tcl_compiler::analyser::Scope> {
    let mut chain = vec![root];
    let mut cursor = root;
    loop {
        let next = cursor.children.iter().find(|c| {
            // `Span` is half-open `[start, end)` — the byte at
            // `s.end()` lives outside the scope.
            c.body_span
                .is_some_and(|s| s.start() <= byte_offset && byte_offset < s.end())
        });
        match next {
            Some(child) => {
                chain.push(child);
                cursor = child;
            }
            None => break,
        }
    }
    chain
}

/// Return the `body_span`s of the scope chain containing
/// `byte_offset`, innermost first.  The global scope has no
/// `body_span`, so an empty result means "the whole file is
/// visible" — an empty list signals a file-wide walk.
pub(crate) fn scope_body_spans_at(
    root: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> Vec<tcl_lexer::Span> {
    scope_chain_at(root, byte_offset)
        .into_iter()
        .rev()
        .filter_map(|s| s.body_span)
        .collect()
}

pub(crate) fn span_to_range(
    source: &str,
    line_index: &LineIndex,
    span: tcl_lexer::Span,
) -> LspRange {
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
    use std::fmt::Write as _;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    /// The document-only resolution context: no registry, no whole-program
    /// export oracle. What every caller had before issue #1116 item 1, and
    /// what a single-document test must keep getting.
    const DOC: CallResolution<'static> = CallResolution::document_only();

    /// Regression coverage for issue #996: `variable_scope_extent`'s
    /// nested `innermost` fn and `linked_var_reference_spans`'s
    /// `collect_alias_spans`/`collect_shared_span_refs` all recurse once
    /// per nested namespace/proc scope, with no depth cap before this fix
    /// (`MAX_SCOPE_WALK_DEPTH`, `crate::lib`). 80 nested `namespace eval`
    /// levels is past the point (confirmed empirically: 100+) where
    /// unguarded namespace-scope recursion overflows `cargo test`'s bare
    /// ~2 MiB per-test default. The assertion is that both return at all,
    /// not what they return.
    #[test]
    fn deeply_nested_namespaces_survive_scope_extent_and_alias_scans() {
        const DEPTH: usize = 80;
        let mut source = String::new();
        for i in 0..DEPTH {
            let _ = writeln!(source, "namespace eval ns{i} {{");
        }
        source.push_str("set x 1\nputs $x\n");
        for _ in 0..DEPTH {
            source.push_str("}\n");
        }
        let analysis = analyse(&source);
        let _ = variable_scope_extent(&analysis, "x", 0);
        if let Some(var_def) = analysis.all_variables.values().next() {
            let _ = linked_var_reference_spans(&analysis.global_scope, var_def);
        }
        let _ = cross_namespace_qualified_vars(&analysis.global_scope, &FxHashSet::default());
    }

    #[test]
    fn jump_to_proc_definition() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the second `greet` (line 1, char 2).
        let locs = definition(src, 1, 2, &analysis);
        assert_eq!(locs.len(), 1);
        // The proc name span is on line 0 starting at column 5.
        assert_eq!(locs[0].start_line, 0);
        assert_eq!(locs[0].start_character, 5);
    }

    // foreach multi-list lock-step form (issue #923 idx 70): `foreach
    // varList1 list1 varList2 list2 ... body` binds every varList, not
    // just the first.

    #[test]
    fn multi_list_foreach_name_resolves_to_its_own_loop_not_an_unrelated_later_one() {
        // The finding's own demonstrated in-vivo failure mode on the real,
        // unmodified pix corpus file (docs/pixdoc.tcl) — a *second*
        // `foreach` reusing the bare name `name` (a wholly unrelated,
        // later loop) previously "won" by being the only `VarDef` named
        // `name` anywhere in the flat top-level scope, since the first
        // loop's own `name` (the second varList of a `foreach dirName
        // {...} name {...} {...}` multi-list form) was never bound at
        // all. tclsh9.0/8.6 both prove `name` inside the first loop's body
        // is that loop's own variable, never reaching the second loop.
        let src = "foreach dirName {src src {src core}} name {alpha beta gamma} {\n    puts \"$dirName $name\"\n    if {$name eq \"pixutils\"} { puts skip }\n}\nforeach name {examples color changes} {\n    puts $name.ruff\n}\n";
        let analysis = analyse(src);
        // Cursor on the first loop's own `$name` read (line 1, col 21).
        let locs = definition(src, 1, 21, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 0,
            "must resolve to the first loop's own `name` clause (line 0), not the unrelated second loop (line 4): {locs:?}"
        );
        assert_eq!(locs[0].start_character, 37);

        // Same wrong-location bug reproduced from a second usage inside
        // the same loop body (line 2, `if {$name eq ...}`).
        let locs2 = definition(src, 2, 10, &analysis);
        assert_eq!(locs2.len(), 1, "{locs2:?}");
        assert_eq!(locs2[0].start_line, 0, "{locs2:?}");
    }

    #[test]
    fn definition_resolves_a_method_return_captured_dispatch() {
        // Issue #1143's definition half: hover/definition resolved the
        // producing `[$a make]` call, but go-to-definition on the consuming
        // `$b greet` answered nothing because `instance_classes` never bound
        // `b`.  The lattice's scope-keyed singleton now resolves it through
        // the same accessor every dispatch consumer reads.
        let src = "oo::class create A { method make {} { return [B new] } }\n\
                   oo::class create B { method greet {} { return \"hi\" } }\n\
                   set a [A new]\n\
                   set b [$a make]\n\
                   $b greet\n";
        let analysis = analyse(src);
        // Cursor on `greet` at the `$b greet` call site (line 4, col 4).
        let locs = definition(src, 4, 4, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to ::B::greet's declaration: {locs:?}"
        );
    }

    // namespace-aware proc resolution (C Tcl `Tcl_FindCommand` order)

    #[test]
    fn unqualified_call_resolves_in_callers_namespace_first() {
        // Two namespaces each define `helper`; the unqualified call inside
        // ::b must land on ::b::helper — C Tcl resolves the current
        // namespace before global, never a sibling namespace.
        let src = "namespace eval a {\n    proc helper {} { return 1 }\n}\nnamespace eval b {\n    proc helper {} { return 2 }\n    helper\n}\n";
        let analysis = analyse(src);
        // Cursor on the bare `helper` call (line 5).
        let locs = definition(src, 5, 6, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 4, "must resolve to ::b::helper");
        assert_eq!(locs[0].start_character, 9);
    }

    // apply namespace override (issue #923 idx 116): a bareword called
    // inside `apply {{params} body ns}`'s body resolves against `ns`, not
    // wherever the `apply` call is lexically written.

    #[test]
    fn bareword_inside_apply_body_resolves_against_the_lambdas_own_namespace() {
        // TP — the finding's own literal repro: `real` and `lexical` both
        // define `cleanup`; `apply {{} {cleanup done} ::real}` must
        // resolve the bareword `cleanup` inside its body to `::real`'s
        // copy, not `::lexical`'s (tclsh9.0/8.6-verified: prints "real
        // done"). Before this fix: resolved to `::lexical::cleanup`
        // purely by lexical-nearest-definition coincidence.
        let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n}\napply {{} {cleanup done} ::real}\n";
        let analysis = analyse(src);
        // Cursor on `cleanup` inside the apply body (line 6, col 11).
        let locs = definition(src, 6, 11, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to real::cleanup (line 1), not lexical::cleanup (line 4): {locs:?}"
        );
        assert_eq!(locs[0].start_character, 9);
    }

    #[test]
    fn bareword_inside_var_indirected_apply_body_resolves_against_the_lambdas_namespace() {
        // TP — one-hop `$var` indirection to a literal lambda triple:
        // `set lambda {{} {cleanup done} ::real}` then `apply $lambda`.
        // tclsh9.0/8.6-verified: prints "real done". Exercises
        // `resolve_dynamic_apply_lambda`'s `TokenType::Var` branch.
        let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n}\nset lambda {{} {cleanup done} ::real}\napply $lambda\n";
        let analysis = analyse(src);
        // Cursor on `cleanup` inside the `set`'s braced value (line 6, col 16).
        let locs = definition(src, 6, 16, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to real::cleanup (line 1), not lexical::cleanup (line 4): {locs:?}"
        );
    }

    #[test]
    fn bareword_inside_list_and_qualified_var_apply_resolves_against_the_lambdas_namespace() {
        // TP — direct list-substitution + namespace-qualified var, `apply`
        // as a literal command word: `apply [list {} $lexical::body
        // ::real]`, where `lexical::body` itself holds the literal
        // `{cleanup done}`. Exercises BOTH new pieces together
        // (`lookup_const_string_in_namespace` + the list-folding
        // `TokenType::Cmd` branch).
        let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n    set body {cleanup done}\n}\napply [list {} $lexical::body ::real]\n";
        let analysis = analyse(src);
        // Cursor on `cleanup` inside `set body {cleanup done}` (line 5, col 14).
        let locs = definition(src, 5, 14, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to real::cleanup (line 1), not lexical::cleanup (line 4): {locs:?}"
        );
    }

    #[test]
    fn two_hop_apply_var_indirection_stays_unresolved() {
        // FN — deliberately out of scope: `set a $lambda; apply $a` is an
        // extra level of $var-to-$var forwarding beyond the one hop
        // `resolve_dynamic_apply_lambda` follows from `apply`'s own
        // argument (`set a $lambda`'s value is itself a `$`-prefixed Var
        // token, which `handle_set_command` never records as a constant
        // string at all, so `apply $a` can't resolve even one hop back to
        // the literal lambda). Two conflicting `cleanup` procs, matching
        // the other scenarios' shape, so a wrong resolution would be
        // visible rather than accidentally right via a single-candidate
        // fallback. Confirm this stays unresolved after the fix — a
        // documented depth bound, not a regression.
        let src = "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n}\nset lambda {{} {cleanup done} ::real}\nset a $lambda\napply $a\n";
        let analysis = analyse(src);
        // Cursor on `cleanup` inside the `set lambda`'s braced value (line 6, col 16).
        let locs = definition(src, 6, 16, &analysis);
        // Since `apply $a` never resolves (not even one hop), this text is
        // never walked as a script body at all, so `namespace_overrides`
        // gets no entry for it — whatever answer comes back is from the
        // SAME pre-existing, coincidental lexical-nearest fallback that
        // would fire with no `apply`/`namespace_overrides` mechanism in
        // the picture at all (confirmed: resolves to lexical::cleanup,
        // line 4 — the nearer declaration, not real::cleanup on line 1).
        // The one assertion that actually matters: the override must NOT
        // have kicked in and pointed this at real::cleanup.
        assert!(
            locs.iter().all(|l| l.start_line != 1),
            "two-hop indirection is a deliberate depth bound — must not resolve via the \
             override to real::cleanup: {locs:?}"
        );
    }

    #[test]
    fn apply_namespace_matching_lexical_nesting_still_resolves() {
        // TN — the "control" shape: apply's namespace override happens to
        // equal its lexical home. Must resolve correctly both before and
        // after the fix (before: coincidentally, via the lexical fallback;
        // after: genuinely, via the override) — proves the fix doesn't
        // disturb the already-working common case.
        let src = "namespace eval app {\n    proc cleanup {tag} { puts $tag }\n    apply {{} {cleanup done} ::app}\n}\n";
        let analysis = analyse(src);
        // Cursor on `cleanup` inside the apply body (line 2, col 15).
        let locs = definition(src, 2, 15, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn apply_namespace_override_does_not_leak_past_its_own_span() {
        // FP guard — a bareword with the SAME text, immediately outside
        // the apply body's span, must resolve exactly as it would with no
        // apply call in the file at all (i.e. to the lexically-nearest
        // `lexical::cleanup`, not real::cleanup). Catches an off-by-one in
        // the recorded span or a "last override wins globally" bug.
        let src = "namespace eval real {\n    proc cleanup {tag} { puts $tag }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts $tag }\n    apply {{} {cleanup done} ::real}\n    cleanup done\n}\n";
        let analysis = analyse(src);
        // Cursor on the second `cleanup` (line 6, the bare call OUTSIDE the
        // apply body, col 4).
        let locs = definition(src, 6, 4, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 4,
            "must resolve to lexical::cleanup, the override must not leak past its span: {locs:?}"
        );
    }

    #[test]
    fn two_apply_calls_with_different_namespace_overrides_never_cross_resolve() {
        // FP guard — two independent, non-overlapping `apply` calls with
        // different namespace overrides in the same file must each
        // resolve to their own namespace, never the other's (catches an
        // implementation that only tracks a single override, or picks the
        // wrong entry when more than one exists).
        let src = "namespace eval a {\n    proc cleanup {tag} { puts $tag }\n}\nnamespace eval b {\n    proc cleanup {tag} { puts $tag }\n}\napply {{} {cleanup 1} ::a}\napply {{} {cleanup 2} ::b}\n";
        let analysis = analyse(src);
        // First apply body's `cleanup` (line 6, col 11) -> ::a::cleanup (line 1).
        let locs_a = definition(src, 6, 11, &analysis);
        assert_eq!(locs_a.len(), 1, "{locs_a:?}");
        assert_eq!(locs_a[0].start_line, 1);
        // Second apply body's `cleanup` (line 7, col 11) -> ::b::cleanup (line 4).
        let locs_b = definition(src, 7, 11, &analysis);
        assert_eq!(locs_b.len(), 1, "{locs_b:?}");
        assert_eq!(locs_b[0].start_line, 4);
    }

    // wildcard namespace import bareword resolution (issue #923 idx 18):
    // `namespace import NS::*` makes every command `NS` has `namespace
    // export`ed callable bare wherever the import is in scope — but real
    // Tcl only imports *exported* names (`Tcl_Export`), so an unexported
    // sibling in `NS` stays unreachable through the import.

    #[test]
    fn wildcard_namespace_import_resolves_exported_proc() {
        // TP — single command in `Foo`, exported, reached only through the
        // wildcard import (tclsh9.0/8.6-verified: `bar` prints 1).
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
        let analysis = analyse(src);
        // Cursor on the bareword `bar` call (line 5, col 0).
        let locs = definition(src, 5, 0, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to Foo::bar's declaration"
        );
        assert_eq!(locs[0].start_character, 9);
    }

    #[test]
    fn wildcard_namespace_import_resolves_exported_proc_alongside_unexported_sibling() {
        // TP — the finding's own headline shape: `Foo` has both an exported
        // `bar` and an unrelated, unexported `other`. Exhaustive tracing
        // found the old code blocked *every* route identically regardless
        // of sibling count — this proves the fix does not regress to some
        // sibling-count-dependent "ambiguity threshold" that doesn't exist
        // in real Tcl either.
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    proc other {} { return 2 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
        let analysis = analyse(src);
        // Cursor on the bareword `bar` call (line 6, col 0).
        let locs = definition(src, 6, 0, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to Foo::bar's declaration"
        );
        assert_eq!(locs[0].start_character, 9);
    }

    #[test]
    fn wildcard_namespace_import_unexported_sibling_stays_unresolved() {
        // FP guard (CRITICAL) — `other` lives in `Foo` but is never
        // exported, so real Tcl's `namespace import ::Foo::*` never binds
        // it: a bare `other` call errors "invalid command name" at runtime
        // (Tcl manual, `namespace` — "namespace import" only imports
        // exported names). The wildcard-import resolver must not
        // manufacture a resolution the interpreter itself would reject.
        //
        // Exercises `proc_visible_via_wildcard_import` directly rather than
        // the full `definition()` provider: `resolve_called_proc`'s own
        // pre-existing, unrelated lenient fallback
        // (`fallback_proc_by_simple_name`, "any proc with this simple name,
        // anywhere") would otherwise also resolve a bare `other` call
        // regardless of import/export state — a longstanding, documented
        // leniency for procs whose defining namespace isn't statically
        // visible, not the gate this test is proving.
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    proc other {} { return 2 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nother\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "::", "other", u32::MAX);
        assert!(
            hit.is_none(),
            "an unexported sibling must stay unresolved through the wildcard import: {hit:?}"
        );
    }

    #[test]
    fn wildcard_namespace_import_resolves_exported_class() {
        // TP — the class analogue of
        // `wildcard_namespace_import_resolves_exported_proc`:
        // `resolve_class_target_at` is the fixed function directly (the
        // top-level `definition()` provider's own bareword-class match is
        // namespace-blind already and would not exercise this fix).
        let src = "namespace eval Foo {\n    oo::class create Widget {}\n    namespace export Widget\n}\nnamespace import ::Foo::*\nWidget new\n";
        let analysis = analyse(src);
        let line_index = LineIndex::new(src);
        let cursor_off = byte_offset_at(&line_index, src, 5, 0);
        let hit = resolve_class_target_at(&analysis, DOC, cursor_off, "Widget");
        assert!(
            hit.is_some(),
            "must resolve Widget through the wildcard import"
        );
        let (qname, _) = hit.unwrap();
        assert_eq!(qname, "::Foo::Widget");
    }

    #[test]
    fn wildcard_namespace_import_unexported_sibling_class_stays_unresolved() {
        // FP guard — the class analogue: `Other` lives in `Foo` but is
        // never exported, so it must stay unresolved through the wildcard
        // import exactly like the proc case.
        let src = "namespace eval Foo {\n    oo::class create Widget {}\n    oo::class create Other {}\n    namespace export Widget\n}\nnamespace import ::Foo::*\nOther new\n";
        let analysis = analyse(src);
        let line_index = LineIndex::new(src);
        let cursor_off = byte_offset_at(&line_index, src, 6, 0);
        let hit = resolve_class_target_at(&analysis, DOC, cursor_off, "Other");
        assert!(
            hit.is_none(),
            "an unexported sibling class must stay unresolved: {hit:?}"
        );
    }

    #[test]
    fn wildcard_namespace_import_wins_over_an_unrelated_decoy_with_the_same_simple_name() {
        // TP — differential-audit finding idx 29 (main audit wave), its
        // "cleanest repro": before this fix, a bareword call reached via a
        // wildcard import never consulted `namespace_imports` at all, so
        // it fell straight to `fallback_proc_by_simple_name`'s lenient
        // "any proc with this simple name, prefer lexicographically
        // smallest qualified name" tie-break — silently resolving (and
        // hovering) to an entirely unrelated, never-imported decoy
        // (`::decoyns::helper`, "d" < "r") instead of the real,
        // oracle-proven target (`::realns::helper`, tclsh9.0/8.6-verified:
        // prints `REAL`). The wildcard-import resolution added by idx 18
        // is tried *before* that fallback, so it must win here.
        let src = "namespace eval realns {\n    namespace export helper\n    proc helper {} { return REAL }\n}\nnamespace eval decoyns {\n    proc helper {} { return DECOY }\n}\nnamespace eval userns {\n    namespace import ::realns::*\n    proc run {} {\n        return [helper]\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on the bareword `helper` call inside `userns::run` (line
        // 10, col 16 — `        return [helper]`).
        let locs = definition(src, 10, 16, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 2,
            "must resolve to realns::helper (the exported, imported target), \
             not the decoy: {locs:?}"
        );
    }

    // Per-import-site export snapshots (issue #1027). `namespace import`
    // binds the names exported *when it runs*; the export list keeps
    // changing afterwards and none of it reaches back. Every case below is
    // oracle-pinned against tclsh 8.6.14 and 9.0.4.
    //
    // These drive `proc_visible_via_wildcard_import` rather than the full
    // `definition()` provider for the same reason
    // `wildcard_namespace_import_unexported_sibling_stays_unresolved` does:
    // `resolve_called_proc`'s unrelated lenient `fallback_proc_by_simple_name`
    // would answer either way and would prove nothing about this gate.

    #[test]
    fn import_site_snapshot_survives_a_later_export_clear() {
        // TP, issue #1027 direction A — `namespace export -clear` after the
        // import does NOT revoke the alias the import already installed.
        // Oracle: `namespace eval ::src {proc p {} {return P}; namespace
        // export p}; namespace eval ::dst {namespace import ::src::*};
        // namespace eval ::src {namespace export -clear}; ::dst::p` → `P`,
        // and `info commands ::dst::*` still lists `::dst::p`.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export -clear\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::p"),
            "a `-clear` written after the import must not revoke it: {hit:?}"
        );
    }

    #[test]
    fn import_site_snapshot_ignores_a_later_export() {
        // FP guard (CRITICAL), issue #1027 direction B — exporting a name
        // *after* an import does not add it retroactively. Oracle:
        // `namespace eval ::src {proc p {} {return P}}; namespace eval ::dst
        // {namespace import ::src::*}; namespace eval ::src {namespace export
        // p}; ::dst::p` → `invalid command name "::dst::p"`.
        let src = "namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export p\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_none(),
            "an export written after the import must not apply retroactively: {hit:?}"
        );
    }

    #[test]
    fn a_global_rooted_import_is_export_gated_in_document_too() {
        // TN / TP pair for #1104's review note — `namespace import ::p` reads
        // as an *empty* source namespace, which both tiers skipped, leaving
        // the last import shape that bypassed the gate. Oracle (8.6.14 /
        // 9.0.4): without `namespace export p` at global level the import is a
        // silent no-op and `info commands ::dst::*` stays empty; with it,
        // `::dst::p` binds.
        let ungated =
            "proc p {} { return GLOBAL }\nnamespace eval dst {\n    namespace import ::p\n}\n";
        let analysis = analyse(ungated);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
            "an unexported global command installs nothing",
        );
        let gated = "proc p {} { return GLOBAL }\nnamespace export p\nnamespace eval dst {\n    namespace import ::p\n}\n";
        let analysis = analyse(gated);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX)
                .is_some_and(|p| p.qualified_name == "::p"),
            "with the export it binds",
        );
    }

    // The same gate, as the **provider** answers it (issue #1104 item 2).
    // Everything above drives `proc_visible_via_wildcard_import` directly
    // because `resolve_called_proc`'s lenient tail match used to overrule it;
    // these pin that it no longer does.

    #[test]
    fn single_file_definition_refuses_direction_b_through_the_fallback() {
        // FP guard (CRITICAL), issue #1104 item 2 — the whole Direction-B
        // fixture in one document. Oracle (8.6.14 / 9.0.4): the bare call is
        // `invalid command name "p"` and `info commands ::dst::*` is empty, so
        // go-to-definition must not answer `::src::p` — which it did, via
        // `fallback_proc_by_simple_name`, long after #1102 gated every other
        // route.
        let src = "namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export p\n}\nnamespace eval dst {\n    p\n}\n";
        let analysis = analyse(src);
        let locs = definition(src, 10, 4, &analysis);
        assert!(
            locs.is_empty(),
            "the lenient tail match must not overrule the import gate: {locs:?}"
        );
    }

    #[test]
    fn single_file_definition_still_answers_when_the_import_is_live() {
        // TP — the same fixture with the export written *before* the import
        // must still resolve, or the tightening has simply broken the feature.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    p\n}\n";
        let analysis = analyse(src);
        let locs = definition(src, 8, 4, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn the_tail_match_still_answers_where_no_import_names_that_namespace() {
        // FN guard (CRITICAL) — the deliberate same-file leniency other tests
        // pin: a proc whose defining namespace is not statically visible at
        // the call still resolves. Nothing imports `::other` here, so the
        // tightening must not touch it.
        let src = "namespace eval other {\n    proc helper {} { return H }\n}\nnamespace eval dst {\n    helper\n}\n";
        let analysis = analyse(src);
        let locs = definition(src, 4, 4, &analysis);
        assert_eq!(
            locs.len(),
            1,
            "an unrelated same-file tail match must keep answering: {locs:?}"
        );
    }

    #[test]
    fn the_tail_match_still_answers_when_a_gated_import_names_another_namespace() {
        // FN guard — the gate is scoped to the namespace the fallback answer
        // lives in. A dead import of `::src` says nothing about a tail match
        // landing in `::other`, so that one must survive.
        let src = "namespace eval src {\n    proc helper {} { return S }\n}\nnamespace eval other {\n    proc helper2 {} { return O }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    helper2\n}\n";
        let analysis = analyse(src);
        let locs = definition(src, 10, 4, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 4);
    }

    #[test]
    fn the_tail_match_refuses_a_forgotten_import_too() {
        // TN — the same tightening covers every way the route can be dead,
        // not just the export snapshot: after `namespace forget ::src::p` the
        // bare call is `invalid command name` (oracle on `alias_live_at`), so
        // the fallback must not answer either.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\nnamespace eval dst {\n    p\n}\n";
        let analysis = analyse(src);
        let locs = definition(src, 11, 4, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn the_tail_match_refuses_a_call_written_before_its_import() {
        // TN — and the new call-site ordering (issue #1104 item 1) reaches the
        // provider through the same door: the call above the import resolves
        // nowhere, the one below it resolves.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    p\n}\n";
        let analysis = analyse(src);
        assert!(
            definition(src, 5, 4, &analysis).is_empty(),
            "the call before the import must not resolve",
        );
        assert_eq!(
            definition(src, 11, 4, &analysis).len(),
            1,
            "the call after it must",
        );
    }

    #[test]
    fn a_second_import_after_a_later_export_does_pick_the_name_up() {
        // TP — each import takes its own snapshot, so the *second* import,
        // written after the export, binds `q` even though the first could
        // not. Oracle: `namespace eval ::src {proc p …; proc q …; namespace
        // export p}; namespace eval ::dst {namespace import ::src::*};
        // namespace eval ::src {namespace export -clear p q}; namespace eval
        // ::dst {namespace import ::src::*}` → both `::dst::p` and
        // `::dst::q` run.
        let src = "namespace eval src {\n    proc p {} { return P }\n    proc q {} { return Q }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export -clear p q\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        for name in ["p", "q"] {
            let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", name, u32::MAX);
            assert!(
                hit.is_some(),
                "the second import must see `{name}`: {hit:?}"
            );
        }
    }

    #[test]
    fn export_clear_before_the_import_does_revoke() {
        // TN — the ordering gate is real in both directions: a `-clear`
        // written *before* the import leaves nothing for it to bind.
        // Oracle: after `namespace export -clear`, `namespace export`
        // returns the empty list, and the later import binds no command.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n    namespace export -clear\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_none(),
            "a `-clear` before the import leaves nothing exported: {hit:?}"
        );
    }

    #[test]
    fn export_clear_with_patterns_on_the_same_call_keeps_only_the_new_ones() {
        // TP + TN — `namespace export a b; namespace export -clear p` leaves
        // exactly `p` exported (oracle: `namespace export` → `p`), so an
        // import after it binds `p` and not `a`.
        let src = "namespace eval src {\n    proc a {} { return A }\n    proc p {} { return P }\n    namespace export a\n    namespace export -clear p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_some(),
            "the `-clear` call's own pattern survives it"
        );
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "a", u32::MAX).is_none(),
            "the pattern cleared by the same call must not survive"
        );
    }

    #[test]
    fn separate_export_calls_are_additive() {
        // TP — `namespace export a` then `namespace export b` exports both
        // (oracle: `namespace export` → `a b`); `export` appends, it does
        // not replace. A snapshot model that kept only the newest call's
        // patterns would lose `a`.
        let src = "namespace eval src {\n    proc a {} { return A }\n    proc b {} { return B }\n    namespace export a\n    namespace export b\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        for name in ["a", "b"] {
            assert!(
                proc_visible_via_wildcard_import(&analysis, DOC, "dst", name, u32::MAX).is_some(),
                "`{name}` must stay exported — `namespace export` is additive"
            );
        }
    }

    #[test]
    fn export_glob_patterns_use_tcl_glob_semantics() {
        // TP + TN — export patterns are glob *patterns*: oracle `namespace
        // export get*` with `getX`/`setX` in the namespace imports only
        // `getX` (`::dst::setX` → `invalid command name`).
        let src = "namespace eval src {\n    proc getX {} { return GX }\n    proc setX {} { return SX }\n    namespace export get*\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "getX", u32::MAX).is_some()
        );
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "setX", u32::MAX).is_none()
        );
    }

    #[test]
    fn an_export_written_before_the_proc_still_exports_it() {
        // TP — an export pattern is a *name pattern*, not a reference to a
        // command: oracle `namespace eval ::src {namespace export p; proc p
        // {} {return P}}` then importing `::src::*` yields a working
        // `::dst::p`. Nothing in the snapshot may require the proc to have
        // been defined by the time the export ran.
        let src = "namespace eval src {\n    namespace export p\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_some(),
            "`namespace export` names a pattern, not an existing command"
        );
    }

    #[test]
    fn an_import_inside_a_body_sees_a_later_top_level_export() {
        // TP — the same-document half of the ordering rule the workspace tier
        // must match (PR #1102 review finding 1, pinned on both sides so they
        // cannot drift): the import runs only when `setup` is called, by which
        // time the whole file — including the `namespace export` written below
        // it — has loaded. Oracle (tclsh 8.6.14 / 9.0.4): `::app::setup;
        // ::app::run` → `HELP`.
        //
        // `indirection::in_effect` already said so here; the workspace tier's
        // plain offset compare did not, which is what the review caught.
        let src = "namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* }\n}\nnamespace eval src {\n    namespace export p\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::p"),
            "a body-local import observes a top-level export written later in \
             the same file: {hit:?}"
        );
    }

    #[test]
    fn an_export_in_the_importing_body_itself_stays_ordered() {
        // FN guard for the leniency above: the "whole file loads first"
        // exception does not extend to a statement of the *same* body, where
        // the offsets are in genuine execution order. Oracle: `proc setup {}
        // {namespace import ::src::*; namespace eval ::src {namespace export
        // p}}` then `::dst::setup` leaves `::dst::p` an invalid command name.
        let src = "namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* ; namespace eval ::src { namespace export p } }\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_none(),
            "an export written after the import in the same body has not run \
             yet: {hit:?}"
        );
    }

    #[test]
    fn a_command_named_like_the_export_flag_is_exportable() {
        // TP — the user-visible consequence of consuming only one `-clear`
        // (PR #1102 review finding 3): `namespace export -clear -clear`
        // exports a command genuinely *named* `-clear`, and a wildcard import
        // then binds it. Oracle (tclsh 8.6.14 / 9.0.4): `namespace export`
        // returns `-clear`, and `info commands ::dst::*` lists `::dst::-clear`.
        // Consuming both words as flags dropped the export entirely.
        let src = "namespace eval src {\n    proc -clear {} { return DC }\n    namespace export -clear -clear\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "-clear", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::-clear"),
            "the second `-clear` is an export pattern, not a second flag: {hit:?}"
        );
    }

    #[test]
    fn exact_namespace_import_resolves_source_proc_in_document() {
        // FP guard / bonus TP — a plain (non-glob) `namespace import
        // ::Foo::bar` must keep working through the same in-document path
        // (the wildcard resolver's glob match degrades to exact equality
        // for a literal pattern), matching the already-passing
        // cross-document `namespace_import_call_site_references_the_source_command`
        // (`tcl-lsp-core::workspace_index`) — this exercises the same-file
        // case that resolver never covers.
        let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::bar\nbar\n";
        let analysis = analyse(src);
        let locs = definition(src, 5, 0, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must resolve to Foo::bar's declaration"
        );
    }

    // ---- the import edge's own lifecycle (issue #1103) -------------------
    //
    // #1102 made the import edge a *snapshot* fact; these pin it as a **link
    // with a lifetime**. Every case is oracle-pinned against tclsh 9.0.4 and
    // 8.6.14, byte-identical on both, and drives
    // `proc_visible_via_wildcard_import` / `proc_visible_via_forced_import`
    // directly for the same reason the #1027 block above does: the lenient
    // `fallback_proc_by_simple_name` in `resolve_called_proc` would answer
    // either way and prove nothing about this gate.

    /// Byte offset just past the last occurrence of `needle`.
    fn after_last(src: &str, needle: &str) -> u32 {
        u32::try_from(src.rfind(needle).expect("needle present") + needle.len())
            .expect("tiny test source")
    }

    /// Byte offset of the first occurrence of `needle`.
    fn at_first(src: &str, needle: &str) -> u32 {
        u32::try_from(src.find(needle).expect("needle present")).expect("tiny test source")
    }

    // Behaviour 1 — `namespace forget`.

    #[test]
    fn a_forget_after_the_import_stops_resolving_the_alias() {
        // TN (the bug): oracle
        //   namespace eval ::src { proc p {} {return P}; namespace export p }
        //   namespace eval ::dst { namespace import ::src::* }
        //   namespace eval ::dst { p }                        → P
        //   namespace eval ::dst { namespace forget ::src::p }
        //   namespace eval ::dst { p }                        → invalid command name "p"
        //   info commands ::dst::*                            → (empty)
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_none(),
            "a forgotten alias must stop resolving after the forget: {hit:?}"
        );
    }

    #[test]
    fn a_call_before_the_forget_still_resolves() {
        // TP — the forget is order-gated, not file-wide: a call written
        // between the import and the forget still reaches `::src::p`.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\np\nnamespace eval dst {\n    namespace forget ::src::p\n}\n";
        let analysis = analyse(src);
        let call = at_first(src, "\np\n") + 1;
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::p"),
            "a call before the forget is unaffected by it: {hit:?}"
        );
    }

    #[test]
    fn a_glob_forget_removes_every_matching_alias() {
        // TN — oracle: `namespace forget ::src::*` empties `info commands
        // ::dst::*` entirely.
        let src = "namespace eval src {\n    proc p {} { return P }\n    proc q {} { return Q }\n    namespace export *\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::*\n}\n";
        let analysis = analyse(src);
        for name in ["p", "q"] {
            assert!(
                proc_visible_via_wildcard_import(&analysis, DOC, "dst", name, u32::MAX).is_none(),
                "`{name}` must be forgotten by the glob pattern"
            );
        }
    }

    /// FN guard pinning issue #1116 item 5: a **dynamic** forget pattern
    /// revokes nothing, so the alias keeps resolving.
    ///
    /// `namespace forget ::src::$name` really does remove the alias when
    /// `$name` is `p` (oracle for the literal spelling is on
    /// `a_forget_after_the_import_stops_resolving_the_alias`) — but
    /// nothing static says what `$name` holds, and the pattern is a *glob*, so
    /// a guess would have to be either "revokes `p`" or "revokes everything".
    /// Both silently drop references the program still has, which is the one
    /// direction this family never takes: a removal that cannot be proved is
    /// not applied. The literal prefix (`::src::`) narrows the *source* but
    /// not the name, so it decides nothing on its own.
    ///
    /// The scanner-side twin of this abstention is
    /// `signature_scan::handlers::handle_namespace_forget_skips_dynamic_patterns`;
    /// this pins the consequence a consumer actually sees.
    #[test]
    fn a_dynamic_forget_pattern_revokes_nothing() {
        let dynamic = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::$name\n}\n";
        let analysis = analyse(dynamic);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::p"),
            "a dynamic forget pattern must not revoke on a guess: {hit:?}"
        );
        // Control — the same source with the pattern written literally does
        // revoke, so the difference is the substitution and nothing else.
        let literal = dynamic.replace("::src::$name", "::src::p");
        let analysis = analyse(&literal);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
            "the literal spelling of the same forget still revokes"
        );
    }

    /// FN guard pinning #1104's dynamic-export note — the export-side twin of
    /// [`a_dynamic_forget_pattern_revokes_nothing`], and the same sign
    /// argument.
    ///
    /// `namespace export $pat` really does export whatever `$pat` expands to,
    /// but nothing static says what that is and the word is a *glob*. The
    /// scanner therefore records no pattern at all
    /// (`Analyser::handle_namespace_export` skips a word containing `$` or
    /// `[`), so the import takes a snapshot that does not cover the name and
    /// the resolver abstains. That is the safe direction here precisely
    /// because the sign is inverted from the forget case: a *guessed export*
    /// can only add names an import never took, inventing a definition the
    /// program has no path to.
    ///
    /// The abstention costs a real answer, which is why it is pinned rather
    /// than merely documented — a future change that starts guessing here
    /// will fail this test, not silently start resolving.
    #[test]
    fn a_dynamic_export_pattern_exports_nothing() {
        let dynamic = "set pat p\nnamespace eval src {\n    proc p {} { return P }\n    namespace export $pat\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(dynamic);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_none(),
            "a dynamic export pattern must not be guessed into a covering export: {hit:?}"
        );
        // Control — the same source with the pattern written literally does
        // export, so the difference is the substitution and nothing else.
        let literal = dynamic.replace("namespace export $pat", "namespace export p");
        let analysis = analyse(&literal);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX)
                .is_some_and(|p| p.qualified_name == "::src::p"),
            "the literal spelling of the same export does resolve"
        );
    }

    /// TN for the interaction the dynamic-export abstention has with a
    /// tombstone (#1104's minor note): a `namespace export -clear` written
    /// after a dynamic pattern leaves the snapshot empty either way, so the
    /// unrecorded pattern costs nothing there — the abstention is only ever
    /// visible *before* a clear, which the test above covers.
    #[test]
    fn a_clear_after_a_dynamic_export_is_still_empty() {
        let src = "set pat p\nnamespace eval src {\n    proc p {} { return P }\n    namespace export $pat\n    namespace export -clear\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
            "a `-clear` after any export leaves nothing exported"
        );
    }

    /// TP (both tiers agree): a `namespace forget` at the file's **load
    /// level** runs before a body-local `namespace import`, which then
    /// installs the alias — so a call in that body still resolves.
    ///
    /// The in-document twin of
    /// `workspace_index::tests::a_body_local_exact_import_survives_a_top_level_forget`.
    /// Ranking the two by raw offset said the forget "came later" and dropped
    /// the alias; the load-order rule the shared fold now uses reads a removal
    /// written outside the import's own body as having run before it, which is
    /// what actually happens — the whole file loads, forget included, before
    /// any body runs. Oracle (8.6.14 / 9.0.4): with `proc ::dst::setup {}
    /// {namespace import ::src::* ; p}` and a top-level `namespace eval ::dst
    /// {namespace forget ::src::p}` below it, `::dst::setup` returns `P` — the
    /// forget ran during load, found nothing imported, and the import inside
    /// the body then installed the alias the call reaches.
    #[test]
    fn a_load_level_forget_before_a_body_local_import_revokes_nothing() {
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* ; p }\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\n";
        let analysis = analyse(src);
        let call =
            u32::try_from(src.rfind("; p }").expect("call in source") + 2).expect("tiny source");
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::p"),
            "the body-local import runs after the load-level forget and reinstalls: {hit:?}"
        );
        // FN guard — a forget written *inside that same body*, after the
        // import, is an ordinary later statement and does revoke.
        let inner = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* ; namespace forget ::src::p ; p }\n}\n";
        let analysis = analyse(inner);
        let call =
            u32::try_from(inner.rfind("; p }").expect("call in source") + 2).expect("tiny source");
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call).is_none(),
            "a forget in the same body before the call does revoke"
        );
    }

    #[test]
    fn a_simple_forget_pattern_removes_the_alias_too() {
        // TN — `Tcl_ForgetImport`'s unqualified branch: the pattern is matched
        // against this namespace's own imported command names, whatever their
        // origin. Oracle: `namespace eval ::dst {namespace forget p}` leaves
        // `info commands ::dst::*` empty and raises no error.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget p\n}\n";
        let analysis = analyse(src);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
            "an unqualified forget pattern removes the alias too"
        );
    }

    #[test]
    fn a_forget_of_an_unrelated_source_leaves_the_alias_alone() {
        // FP guard — a qualified forget names the source whose imports it
        // removes; one naming a *different* namespace must not touch this
        // alias.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval other {\n    proc p {} { return O }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::other::p\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::p"),
            "a forget aimed at another source namespace revokes nothing: {hit:?}"
        );
    }

    #[test]
    fn a_re_import_after_a_forget_resolves_again() {
        // TP — the log is ordered, not a one-way tombstone: re-running the
        // import after the forget installs the alias again (oracle: the bare
        // call works once more).
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::p"),
            "a later re-import reinstates the alias: {hit:?}"
        );
    }

    // Behaviour 1b — the install is order-gated too (issue #1104 item 1).
    //
    // Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
    //
    //   namespace eval ::src { proc p {} {return P}; namespace export p }
    //   namespace eval ::dst { p }                    → invalid command name "p"
    //   namespace eval ::dst { namespace import ::src::* }
    //   namespace eval ::dst { p }                    → P
    //
    //   namespace eval ::app { proc run {} { helper } }
    //   namespace eval ::app { namespace import ::src::* }
    //   ::app::run                                    → HELP
    //
    //   namespace eval ::app2 { proc run2 {} {
    //       catch {helper} e ; namespace import ::src::* ; return "$e [helper]" } }
    //   ::app2::run2       → {invalid command name "helper"} HELP

    #[test]
    fn a_top_level_call_before_its_own_import_resolves_nothing() {
        // TN (the leniency #1102 deferred): both statements are top level, so
        // the offsets are in genuine execution order.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\np\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        let call = at_first(src, "\np\n") + 1;
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
        assert!(
            hit.is_none(),
            "a call written before its own import reaches nothing: {hit:?}"
        );
        // TP — the same call after the import does resolve.
        let after = after_last(src, "namespace import ::src::*\n}\n");
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", after)
                .is_some_and(|p| p.qualified_name == "::src::p"),
        );
    }

    #[test]
    fn a_body_local_call_sees_an_import_written_later_in_the_file() {
        // TP (CRITICAL) — the gate is `in_effect`, not `at < call_off`: the
        // whole file loads, imports included, before any body runs. This is
        // the shape `tcllib`'s `modules/uev/uevent.tcl` is written in (procs
        // first, `namespace import` at the bottom); a plain-offset gate broke
        // every one of its call sites.
        let src = "namespace eval src {\n    proc helper {} { return HELP }\n    namespace export helper\n}\nnamespace eval app {\n    proc run {} { helper }\n}\nnamespace eval app {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        let call = at_first(src, "helper }");
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "app", "helper", call);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::helper"),
            "a body-local call observes its own file's later top-level import: {hit:?}"
        );
    }

    #[test]
    fn a_call_before_an_import_in_that_same_body_resolves_nothing() {
        // TN — the FN guard: an import written in the *same* body after the
        // call is an ordinary later statement of the running script, exactly
        // as the export snapshot already treats one.
        let src = "namespace eval src {\n    proc helper {} { return HELP }\n    namespace export helper\n}\nnamespace eval app {\n    proc run {} { helper ; namespace import ::src::* ; helper }\n}\n";
        let analysis = analyse(src);
        let first = at_first(src, "helper ;");
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "app", "helper", first).is_none(),
            "the first call runs before the same body's import",
        );
        let second = after_last(src, "namespace import ::src::* ; ");
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "app", "helper", second)
                .is_some_and(|p| p.qualified_name == "::src::helper"),
            "the second call runs after it",
        );
    }

    #[test]
    fn a_call_before_a_forced_import_is_not_shadowed_yet() {
        // TP — the same gate through `ImportQuery::ForcedShadow`: a `-force`
        // import has not replaced the local command until it runs, so a call
        // written before it still reaches the local definition (oracle:
        // `::dst::p` → LOCAL before the forced import, → SRC after).
        let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\np\nnamespace eval dst {\n    namespace import -force ::src::*\n}\n";
        let analysis = analyse(src);
        let before = at_first(src, "\np\n") + 1;
        assert!(
            !forced_import_shadows(&analysis, DOC, "dst", "p", before),
            "the shadow is not in effect before the forced import runs",
        );
        let after = after_last(src, "namespace import -force ::src::*\n}\n");
        assert!(forced_import_shadows(&analysis, DOC, "dst", "p", after));
    }

    #[test]
    fn a_later_conflicting_import_does_not_decide_an_earlier_call() {
        // TP — the winning source is chosen from the installs already in
        // effect, so a `-force` re-import from a different namespace written
        // *after* the call cannot retarget it (oracle: with `::A` imported
        // first, a call between the two imports runs `::A::p`).
        let src = "namespace eval A {\n    proc p {} { return A }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return B }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\np\nnamespace eval dst {\n    namespace import -force ::B::*\n}\n";
        let analysis = analyse(src);
        let call = at_first(src, "\np\n") + 1;
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::A::p"),
            "the call between the two imports takes the first edge: {hit:?}"
        );
        let after = after_last(src, "namespace import -force ::B::*\n}\n");
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", after)
                .is_some_and(|p| p.qualified_name == "::B::p"),
            "…and a call after the forced re-import takes the second",
        );
    }

    // Behaviour 2 — `-force` / import-conflict semantics.

    #[test]
    fn a_forced_import_shadows_the_local_command() {
        // TP (the bug): oracle
        //   namespace eval ::src { proc p {} {return SRC}; namespace export p }
        //   namespace eval ::dst { proc p {} {return LOCAL} }
        //   ::dst::p                                   → LOCAL
        //   namespace eval ::dst { namespace import -force ::src::* }
        //   ::dst::p                                   → SRC
        //   namespace origin ::dst::p                  → ::src::p
        let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    namespace import -force ::src::*\n}\nnamespace eval dst {\n    p\n}\n";
        let analysis = analyse(src);
        let call = after_last(src, "    p\n");
        let hit = resolve_called_proc(&analysis, src, "::dst", "p", call, DOC);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::p"),
            "a `-force` import replaces the local command: {hit:?}"
        );
    }

    #[test]
    fn an_unforced_import_onto_an_existing_command_installs_nothing() {
        // TN (the other bug): the same program without `-force` raises `can't
        // import command "p": already exists`, the local survives, and
        // `namespace origin ::dst::p` answers `::dst::p`. The import must not
        // make the call a reference to `::src::p`.
        let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
            "a conflicting non-`-force` import installs nothing"
        );
        // …and the ordinary namespace lookup still answers the local.
        let hit = resolve_called_proc(&analysis, src, "::dst", "p", u32::MAX, DOC);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::dst::p"),
            "the local definition survives the failed import: {hit:?}"
        );
    }

    #[test]
    fn a_local_defined_after_the_import_is_not_a_conflict_but_does_end_the_alias() {
        // The three-phase ordering (issue #1116 finding 3). The conflict is
        // judged at the *import's* position, so a `proc` written after it does
        // not retroactively make the import a no-op — a call between the two
        // still reaches `::src::p`. But the `proc` *does* recreate
        // `::dst::p` as an ordinary command, silently, so a call after it
        // reaches the local one. Oracle (9.0.4 / 8.6.14, byte-identical):
        //   namespace eval ::dst {namespace import ::src::*}
        //   ::dst::p                    → SRC   ; origin → ::src::p
        //   namespace eval ::dst {proc p {} {return LOCAL}}    (rc 0, silent)
        //   ::dst::p                    → LOCAL ; origin → ::dst::p
        //   ::src::p                    → SRC   (untouched)
        let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\np\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\n";
        let analysis = analyse(src);
        // TP — between the import and the redefinition, the alias is live.
        let between = at_first(src, "\np\n") + 1;
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", between);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::p"),
            "a later local definition is not a conflict at the import: {hit:?}"
        );
        // TN — after it, the alias is gone and the local command is what the
        // ordinary namespace lookup answers.
        let after = after_last(src, "return LOCAL }");
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", after).is_none(),
            "redefining the imported name ends the alias"
        );
        let hit = resolve_called_proc(&analysis, src, "::dst", "p", after, DOC);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::dst::p"),
            "after the redefinition the call reaches the local proc: {hit:?}"
        );
    }

    #[test]
    fn a_redefinition_ends_a_forced_import_shadow() {
        // TN, issue #1116 finding 3 — the same rule reached through
        // `forced_import_shadows`, which is what made this a P2: with the
        // shadow stuck on, `definition` / `hover` / `signature_help` skipped
        // the valid local definition and answered the import source forever.
        // Oracle: `proc ::dst::p` after `namespace import -force ::src::*`
        // returns LOCAL2 with `namespace origin ::dst::p` → `::dst::p`.
        let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    namespace import -force ::src::*\n}\nnamespace eval dst {\n    proc p {} { return LOCAL2 }\n}\n";
        let analysis = analyse(src);
        let between = at_first(src, "namespace import -force")
            + u32::try_from("namespace import -force ::src::*".len()).expect("tiny");
        let hit = resolve_called_proc(&analysis, src, "::dst", "p", between, DOC);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::p"),
            "between the forced import and the redefinition the source wins: {hit:?}"
        );
        let after = after_last(src, "return LOCAL2 }");
        let hit = resolve_called_proc(&analysis, src, "::dst", "p", after, DOC);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::dst::p"),
            "the redefinition ends the forced shadow: {hit:?}"
        );
    }

    #[test]
    fn a_live_alias_is_an_import_conflict_for_a_different_source() {
        // TN, issue #1116 finding 4 (CRITICAL) — with `::dst` already
        // importing `::A::*`, a later *unforced* `namespace import ::B::*`
        // fails and changes nothing. Oracle (9.0.4 / 8.6.14):
        //   namespace eval ::dst {namespace import ::A::*}   ; origin → ::A::p
        //   namespace eval ::dst {namespace import ::B::*}
        //     → can't import command "p": already exists
        //   ::dst::p → AP ; origin → ::A::p
        // Before this, both imports entered the install set and the *newest*
        // won, resolving later calls to `::B::p`.
        let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace import ::B::*\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::A::p"),
            "the failed second import leaves the first alias in place: {hit:?}"
        );
    }

    #[test]
    fn a_forced_import_from_a_second_source_replaces_the_first_alias() {
        // TP, the other half of finding 4 — with `-force` the second import
        // wins (oracle: `::dst::p` → BP, `namespace origin` → `::B::p`).
        let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace import -force ::B::*\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::B::p"),
            "a `-force` import replaces a live alias too: {hit:?}"
        );
    }

    #[test]
    fn a_body_local_import_conflicts_with_a_later_top_level_import() {
        // TN (CRITICAL) — the in-document twin of the workspace tier's own
        // check: which import installs is decided in *execution* order, not
        // written order. Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
        //
        //   namespace eval ::dst { proc p {} { namespace import ::B::x } }
        //   namespace eval ::dst { namespace import ::A::* }
        //   namespace origin ::dst::x  -> ::A::x
        //   ::dst::p                   -> can't import command "x": already exists
        //   namespace origin ::dst::x  -> ::A::x   (unchanged)
        //
        // The whole file loads before any body runs, so the top-level `::A`
        // import owns the name and the body-local `::B` one installs nothing.
        // Folding the slot log in raw offset order got both halves wrong: it
        // installed `::B` and then conflicted `::A` away.
        let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    proc runner {} { namespace import ::B::p }\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::A::p"),
            "the top-level import runs first and the body-local one installs \
             nothing: {hit:?}"
        );
    }

    #[test]
    fn a_later_import_in_the_same_body_still_loses_to_the_earlier_one() {
        // TN guard — inside one body the offsets *are* execution order again.
        // Oracle: `proc q {} { namespace import ::A::* ; namespace import
        // ::B::p }` → the first succeeds, the second raises `already exists`,
        // `namespace origin` stays `::A::p`.
        let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    proc q {} { namespace import ::A::* ; namespace import ::B::p }\n}\n";
        let analysis = analyse(src);
        let call = after_last(src, "namespace import ::B::p");
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::A::p"),
            "the second import of the same body installs nothing: {hit:?}"
        );
    }

    #[test]
    fn a_forget_lets_the_next_unforced_import_install() {
        // FN guard for finding 4 — the conflict is the *live* alias, not the
        // fact that one was ever installed. Oracle: forgetting `::A::p` first
        // makes the unforced `::B::*` import succeed (`origin` → `::B::p`).
        let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace forget ::A::p\n}\nnamespace eval dst {\n    namespace import ::B::*\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::B::p"),
            "after the forget the next unforced import installs: {hit:?}"
        );
    }

    #[test]
    fn a_re_import_from_the_same_source_is_not_a_conflict() {
        // FN guard — re-importing the *same* thing is a silent no-op, not the
        // `already exists` error (oracle: rc 0, `origin` still `::A::p`), so
        // the second import must not be treated as conflicting with the alias
        // it would reinstall.
        let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::A::p"),
            "a same-source re-import stays live: {hit:?}"
        );
    }

    #[test]
    fn an_unforced_import_without_a_conflict_still_installs() {
        // FN guard — the conflict rule must not fire when the importing
        // namespace holds nothing of that name (the overwhelmingly common
        // case, oracle `H` row: `::dst::p` → `SRC`).
        let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
        let analysis = analyse(src);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_some(),
            "an unconflicted import still installs"
        );
    }

    #[test]
    fn a_forced_import_before_the_call_does_not_shadow_a_later_call_site() {
        // FP guard on the `-force` shortcut: it outranks the local lookup
        // only where a live import edge really covers the word. With the
        // forced import forgotten again, the local definition wins back.
        let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    namespace import -force ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\nnamespace eval dst {\n    p\n}\n";
        let analysis = analyse(src);
        let call = after_last(src, "    p\n");
        let hit = resolve_called_proc(&analysis, src, "::dst", "p", call, DOC);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::dst::p"),
            "after the forget the local definition is what the call reaches: {hit:?}"
        );
    }

    // Behaviour 2b — what an *absent* export means for `-force`
    // (issue #1116 item 1).

    #[test]
    fn a_forced_import_shadows_when_this_file_holds_no_export_for_the_source() {
        // TP (CRITICAL) — the partly-observable source. `::src`'s proc is in
        // this document, its `namespace export` is not, so the old
        // "does this file hold *anything* of `::src`" test read the gap as
        // "not exported" and single-file go-to-definition kept answering the
        // local `::app::helper`. Oracle (8.6.14 / 9.0.4) with the export
        // present anywhere in the program: `::app::helper` → SRC and
        // `namespace origin ::app::helper` → `::src::helper`.
        let src = "namespace eval src {\n    proc helper {} { return SRC }\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";
        let analysis = analyse(src);
        let call = after_last(src, "    helper\n");
        assert!(
            forced_import_shadows(&analysis, DOC, "::app", "helper", call),
            "a `-force` import of a namespace whose exports this file cannot \
             see must presume the shadow",
        );
        assert!(
            resolve_called_proc(&analysis, src, "::app", "helper", call, DOC).is_none(),
            "…and the local definition the import deleted must not be the answer",
        );
    }

    #[test]
    fn a_forced_import_does_not_shadow_when_this_file_holds_the_export_list() {
        // TN / FP guard — with an export declaration for `::src` in this
        // document the gap *is* evidence: `helper` is not among what `::src`
        // exports, so the forced import installs nothing and the local
        // definition is still what the call reaches (oracle: with only
        // `namespace export other`, `::app::helper` → LOCAL).
        let src = "namespace eval src {\n    proc helper {} { return SRC }\n    proc other {} { return O }\n    namespace export other\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";
        let analysis = analyse(src);
        let call = after_last(src, "    helper\n");
        assert!(!forced_import_shadows(
            &analysis, DOC, "::app", "helper", call
        ));
        let hit = resolve_called_proc(&analysis, src, "::app", "helper", call, DOC);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::app::helper"),
            "the local definition survives an import that binds nothing: {hit:?}"
        );
    }

    // Behaviour 2c — the whole-program export oracle (issue #1116 item 1).
    //
    // The single document [`PARTLY_OBSERVABLE`] is the *same bytes* in every
    // test below. Only the program around it changes, and with it the correct
    // answer — which is the whole reason this tier needed an oracle. Oracle
    // transcript (tclsh 8.6.14 and 9.0.4, byte-identical), the file loaded
    // from a two-line loader that does or does not also source an
    // `exports.tcl` holding `namespace eval ::src {namespace export helper}`:
    //
    //   with exports.tcl:      call -> SRC     origin -> ::src::helper
    //   without exports.tcl:   call -> LOCAL   origin -> ::app::helper

    /// The pinned two-file shape's single document, byte-for-byte.
    ///
    /// `::src` is *partly* observable here: its procs and one of its exports
    /// (`other`) are in this file. Whether it also exports `helper` is decided
    /// somewhere this document cannot see.
    const PARTLY_OBSERVABLE: &str = "namespace eval src {\n    proc helper {} { return SRC }\n    proc other {} { return O }\n    namespace export other\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";

    /// A stand-in [`NamespaceExportOracle`] over an explicit table: a
    /// namespace absent from it is [`ExportVerdict::Unknown`], one present
    /// answers from its listed names.
    ///
    /// Deliberately position-blind: what this fixture isolates is the
    /// three-way verdict itself. Ordering a real workspace's export events
    /// against an import site is `WorkspaceIndex`'s job, pinned against the
    /// real snapshot in
    /// [`the_real_oracle_does_not_resurrect_an_export_written_after_the_import`]
    /// and in `workspace_index`'s own both-tiers tests.
    struct FakeExports(&'static [(&'static str, &'static [&'static str])]);

    impl NamespaceExportOracle for FakeExports {
        fn exported_at(
            &self,
            source_ns: &str,
            name: &str,
            _import_site: RunPoint<'_>,
        ) -> ExportVerdict {
            let bare = source_ns.trim_start_matches("::");
            match self.0.iter().find(|(ns, _)| *ns == bare) {
                None => ExportVerdict::Unknown,
                Some((_, names)) if names.contains(&name) => ExportVerdict::Exported,
                Some(_) => ExportVerdict::NotExported,
            }
        }
    }

    /// [`PARTLY_OBSERVABLE`] resolved against `oracle`.
    fn partly_observable_target(oracle: &dyn NamespaceExportOracle) -> Option<String> {
        let analysis = analyse(PARTLY_OBSERVABLE);
        let call = after_last(PARTLY_OBSERVABLE, "    helper\n");
        let ctx = DOC.in_program(ProgramExports {
            uri: "file:///main.tcl",
            oracle,
        });
        let shadow = forced_import_shadows(&analysis, ctx, "::app", "helper", call);
        let hit = resolve_called_proc(&analysis, PARTLY_OBSERVABLE, "::app", "helper", call, ctx);
        // The two halves must not contradict: a live shadow means the local
        // command is gone, so the resolver may never answer with it.
        assert!(
            !(shadow && hit.is_some_and(|p| p.qualified_name == "::app::helper")),
            "shadow fired yet the resolver answered the deleted local command",
        );
        hit.map(|p| p.qualified_name.clone())
    }

    #[test]
    fn a_forced_import_shadows_when_another_file_holds_the_covering_export() {
        // TP (the bug, CRITICAL) — `::src` exports `helper` somewhere else in
        // the program. The `-force` import therefore *did* delete
        // `::app::helper`, and the call reaches `::src::helper` — which this
        // document happens to hold, so the in-document tier can answer it
        // outright. Before the oracle, the document-only rule read "this file
        // has an export record for ::src and none of them covers helper" as a
        // fact and kept answering the deleted local definition.
        assert_eq!(
            partly_observable_target(&FakeExports(&[("src", &["other", "helper"])])),
            Some("::src::helper".to_owned()),
        );
    }

    #[test]
    fn a_forced_import_does_not_shadow_when_the_whole_program_exports_nothing() {
        // TN, byte-identical input to the test above — nothing anywhere in the
        // program exports `helper`, so the `-force` import binds only `other`
        // and the local definition survives (oracle: LOCAL / ::app::helper).
        // This is the case the pre-#1116 rule got right and must keep getting
        // right; it is also the pinned true negative
        // `a_forced_import_does_not_shadow_when_this_file_holds_the_export_list`
        // covers without an oracle.
        assert_eq!(
            partly_observable_target(&FakeExports(&[("src", &["other"])])),
            Some("::app::helper".to_owned()),
        );
    }

    #[test]
    fn a_forced_import_from_a_namespace_the_program_cannot_see_still_shadows() {
        // TP / abstention — `::src` is in no indexed file (an installed
        // package, a document outside the workspace), so the oracle answers
        // `Unknown`. `-force` deletes the local command whether or not
        // anything here can prove the export, and answering with a command the
        // import may have removed is worse than answering nothing: the
        // resolver abstains and the cross-document tier takes over.
        assert_eq!(partly_observable_target(&FakeExports(&[])), None);
    }

    #[test]
    fn the_same_document_without_an_oracle_keeps_the_document_only_answer() {
        // FP guard on the plumbing itself: the oracle is *optional*. A caller
        // with no workspace index — a single-document unit test, the `tcl`
        // CLI, a buffer the workspace has not indexed — must get exactly the
        // pre-#1116 answer rather than a panic or the abstention above.
        let analysis = analyse(PARTLY_OBSERVABLE);
        let call = after_last(PARTLY_OBSERVABLE, "    helper\n");
        assert!(!forced_import_shadows(
            &analysis, DOC, "::app", "helper", call
        ));
        let hit = resolve_called_proc(&analysis, PARTLY_OBSERVABLE, "::app", "helper", call, DOC);
        assert_eq!(
            hit.map(|p| p.qualified_name.clone()),
            Some("::app::helper".to_owned()),
        );
    }

    #[test]
    fn the_real_oracle_does_not_resurrect_an_export_written_after_the_import() {
        // FP guard, issue #1027 Direction B under the oracle, and the reason
        // [`ProgramExports`] pairs the oracle with the document's **real**
        // URI. The export covering the name is in this very document but
        // written *after* the import, so it is not retroactive (oracle:
        // `invalid command name "p"`). The workspace oracle holds that same
        // row, tagged with the same URI, so it ranks it against the import by
        // offset and answers `NotExported`. Handed a placeholder URI instead
        // it could not rank the row at all, and
        // `exported_at_import_site`'s abstain-toward-exported rule would have
        // installed an import Tcl never made.
        //
        // Deliberately run against the real `WorkspaceIndex` snapshot rather
        // than [`FakeExports`], which is position-blind by design.
        let src = "namespace eval src {\n    proc p {} { return SRC }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export p\n}\nnamespace eval dst {\n    p\n}\n";
        let analysis = analyse(src);
        let call = after_last(src, "    p\n");
        let index = crate::workspace_index::WorkspaceIndex::from_documents([(
            "file:///main.tcl",
            &analysis,
        )]);
        let exports = index.export_snapshot();
        let ctx = DOC.in_program(ProgramExports {
            uri: "file:///main.tcl",
            oracle: exports.as_ref(),
        });
        assert!(
            proc_visible_via_wildcard_import(&analysis, ctx, "::dst", "p", call).is_none(),
            "an export written after the import must not install it retroactively",
        );
        // …and a *second* import written after the export does pick it up,
        // so the guard above is the ordering rule and not a blanket refusal.
        let src2 = format!("{src}namespace eval dst {{\n    namespace import ::src::*\n}}\n");
        let analysis2 = analyse(&src2);
        let index2 = crate::workspace_index::WorkspaceIndex::from_documents([(
            "file:///main.tcl",
            &analysis2,
        )]);
        let exports2 = index2.export_snapshot();
        let ctx2 = DOC.in_program(ProgramExports {
            uri: "file:///main.tcl",
            oracle: exports2.as_ref(),
        });
        let after = u32::try_from(src2.len()).expect("tiny test source");
        assert!(
            proc_visible_via_wildcard_import(&analysis2, ctx2, "::dst", "p", after)
                .is_some_and(|p| p.qualified_name == "::src::p"),
            "each import site takes its own export snapshot",
        );
    }

    // Behaviour 3 — source rename / delete / redefine after the import.

    #[test]
    fn deleting_the_source_command_kills_the_alias() {
        // TN (the bug): oracle
        //   namespace eval ::src { proc p {} {return P}; namespace export p }
        //   namespace eval ::dst { namespace import ::src::* }
        //   ::dst::p              → P
        //   rename ::src::p {}
        //   ::dst::p              → invalid command name "::dst::p"
        //   info commands ::dst::* → (empty)
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nrename ::src::p {}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_none(),
            "the alias holds the command object, so deleting it kills the alias: {hit:?}"
        );
    }

    #[test]
    fn renaming_the_source_command_leaves_the_alias_alive() {
        // TP — the crucial asymmetry: oracle
        //   rename ::src::p ::src::pp
        //   ::dst::p                  → P          (still works)
        //   namespace origin ::dst::p → ::src::pp  (only the origin moved)
        //   info commands ::dst::*    → ::dst::p
        // A model that treated any `rename` of the source as a removal would
        // lose a genuinely-live command here.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nrename ::src::p ::src::pp\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::src::p"),
            "a rename moves the origin but keeps the alias: {hit:?}"
        );
    }

    #[test]
    fn a_call_before_the_source_deletion_still_resolves() {
        // FP guard — the deletion is order-gated like every other lifecycle
        // event, so a call written before it is unaffected.
        let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\np\nrename ::src::p {}\n";
        let analysis = analyse(src);
        let call = at_first(src, "\np\n") + 1;
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call).is_some(),
            "a call before the deletion still reaches the source"
        );
    }

    #[test]
    fn a_redefinition_of_the_source_is_seen_through_the_link() {
        // TP — oracle: redefining `::src::p` after the import makes `::dst::p`
        // run the *new* body (`SECOND`), with `namespace origin ::dst::p`
        // still `::src::p`. The alias re-reads the source at call time, so the
        // definition a call reaches is the one in effect at the *call*, not
        // the one in effect at the import.
        let src = "namespace eval src {\n    proc p {} { return FIRST }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    proc p {} { return SECOND }\n}\ndst::p\n";
        let analysis = analyse(src);
        let call = after_last(src, "dst::p");
        let hit =
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call).expect("alias");
        let captured = analysis
            .proc_def_in_effect_at(&hit.qualified_name, call)
            .expect("definition in effect at the call");
        assert_eq!(
            captured.name_span.start(),
            at_first(src, "p {} { return SECOND }"),
            "the redefinition is seen through the link"
        );
    }

    // Behaviour 4 — import chains.

    #[test]
    fn an_import_chain_follows_to_the_original_source() {
        // TP (the bug): oracle
        //   namespace eval ::C { proc p {} {return CP}; namespace export p }
        //   namespace eval ::B { namespace import ::C::*; namespace export p }
        //   namespace eval ::A { namespace import ::B::* }
        //   ::A::p                    → CP
        //   namespace origin ::A::p   → ::C::p
        // Statically `::B::p` is in no `all_procs`, so a single-hop walk found
        // nothing and navigation abstained.
        let src = "namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n    namespace export p\n}\nnamespace eval A {\n    namespace import ::B::*\n}\n";
        let analysis = analyse(src);
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX);
        assert!(
            hit.is_some_and(|p| p.qualified_name == "::C::p"),
            "the chain must follow ::A → ::B → ::C: {hit:?}"
        );
    }

    #[test]
    fn a_forget_in_the_middle_of_a_chain_kills_the_whole_chain() {
        // TN — oracle: `namespace eval ::B {namespace forget ::C::p}` makes
        // `::A::p` an `invalid command name` too, because deleting an imported
        // command deletes the commands imported *from* it. Both `info commands
        // ::A::*` and `::B::*` come back empty.
        let src = "namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n    namespace export p\n}\nnamespace eval A {\n    namespace import ::B::*\n}\nnamespace eval B {\n    namespace forget ::C::p\n}\n";
        let analysis = analyse(src);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX).is_none(),
            "a forget in the middle hop kills the outer alias too"
        );
    }

    #[test]
    fn a_chain_hop_that_was_never_exported_does_not_resolve() {
        // FN guard — each hop keeps its own export gate: with `::B` importing
        // `::C::*` but never re-exporting `p`, `::A`'s import of `::B::*`
        // binds nothing (oracle: `::A::p` → invalid command name).
        let src = "namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n}\nnamespace eval A {\n    namespace import ::B::*\n}\n";
        let analysis = analyse(src);
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX).is_none(),
            "the middle hop must have exported the name for the chain to run"
        );
    }

    #[test]
    fn a_mutually_importing_pair_terminates() {
        // FP/hang guard — two namespaces importing each other's `*` form a
        // cycle with no definition at either end; the walk is bounded by
        // `indirection::MAX_COMMAND_NAME_HOPS` and must simply abstain.
        let src = "namespace eval A {\n    namespace import ::B::*\n    namespace export *\n}\nnamespace eval B {\n    namespace import ::A::*\n    namespace export *\n}\n";
        let analysis = analyse(src);
        assert!(proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX).is_none());
        assert!(proc_visible_via_wildcard_import(&analysis, DOC, "B", "p", u32::MAX).is_none());
    }

    #[test]
    fn unqualified_call_at_global_prefers_global_proc() {
        // A global ::helper exists alongside ::a::helper; a global-scope
        // call resolves the global proc (the only candidate C Tcl tries).
        let src = "namespace eval a {\n    proc helper {} { return 1 }\n}\nproc helper {} { return 0 }\nhelper\n";
        let analysis = analyse(src);
        let locs = definition(src, 4, 2, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 3, "must resolve to ::helper");
    }

    #[test]
    fn global_call_with_only_namespaced_procs_falls_back_deterministically() {
        // No ::helper exists, so no candidate resolves; the lenient tail
        // fallback fires and must be deterministic — the lexicographically
        // smallest qualified name (::a::helper), on every repeat, never a
        // `HashMap`-iteration-order pick.
        let src = "namespace eval z {\n    proc helper {} { return 26 }\n}\nnamespace eval a {\n    proc helper {} { return 1 }\n}\nhelper\n";
        let analysis = analyse(src);
        for attempt in 0..8 {
            let locs = definition(src, 6, 2, &analysis);
            assert_eq!(locs.len(), 1, "attempt {attempt}: {locs:?}");
            assert_eq!(
                locs[0].start_line, 4,
                "attempt {attempt}: fallback must pick ::a::helper"
            );
        }
    }

    #[test]
    fn builtin_named_namespace_proc_resolves_inside_its_namespace_only() {
        // `proc set` inside ::ns shadows the builtin *within* the namespace;
        // a global-scope `set` reaches the builtin, so no user-proc jump.
        let src = "namespace eval ns {\n    proc set {key value} { return $value }\n    set x 1\n}\nset y 2\n";
        let analysis = analyse(src);
        // Inside the namespace: the proc wins.
        let inside = definition(src, 2, 5, &analysis);
        assert_eq!(inside.len(), 1, "{inside:?}");
        assert_eq!(inside[0].start_line, 1);
        // At global scope the builtin wins — no definition to jump to.
        let global = definition(src, 4, 1, &analysis);
        assert!(global.is_empty(), "{global:?}");
    }

    #[test]
    fn nested_shadow_of_builtin_never_outranks_the_builtin() {
        // TP — the "rename the builtin away, install a same-named shadow,
        // restore it" idiom (`rest.tcl`'s `describe`, tcllib): a `proc ::set
        // {...}` written *inside* another proc's body exists only
        // conditionally (when and if the enclosing proc runs), and is
        // renamed back off again before the enclosing proc returns. A call
        // site that runs strictly after the enclosing proc returns is
        // unambiguously the real builtin — the shadow definition existing
        // *somewhere in the file* must not make it win.
        let src = "proc outer {} {\n    rename ::set ::_set\n    proc ::set {var val} { ::_set $var $val }\n    rename ::set {}\n    rename ::_set ::set\n}\nouter\nset final restored\n";
        let analysis = analyse(src);
        // `set` on the last line, well after `outer` (and its nested shadow)
        // has returned.
        let after_restore = definition(src, 7, 0, &analysis);
        assert!(
            after_restore.is_empty(),
            "a call after the shadow is restored must resolve to the \
             builtin (no jump target), not the nested shadow: {after_restore:?}",
        );
    }

    #[test]
    fn ordinary_nested_proc_still_resolves_when_not_shadowing_a_builtin() {
        // TN / regression guard — the nested-shadow gate is scoped to names
        // that collide with a real registry builtin; an ordinary nested
        // helper proc (no such collision) must keep resolving exactly as
        // before.
        let src = "proc outer {} {\n    proc helper {} { return 1 }\n    return [helper]\n}\n";
        let analysis = analyse(src);
        let locs = definition(src, 2, 13, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "must still resolve to the nested helper"
        );
    }

    #[test]
    fn top_level_builtin_override_still_resolves_to_itself() {
        // FN guard — a proc that shadows a builtin at *file* (top-level)
        // scope is not a conditional nested definition; it unconditionally
        // replaces the builtin for the rest of the file, exactly as real
        // Tcl's `proc puts {args} {...}` would, and must keep resolving to
        // itself rather than being caught by the nested-shadow gate.
        let src = "proc puts {args} { return }\nputs hello\n";
        let analysis = analyse(src);
        let locs = definition(src, 1, 0, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 0,
            "must resolve to the top-level override"
        );
    }

    #[test]
    fn qualified_relative_call_prefers_current_namespace_then_global() {
        // `sub::p` written inside ::ns resolves ::ns::sub::p before
        // ::sub::p; the absolute `::sub::p` resolves exactly (confirmed
        // against tclsh — see `bareword_resolution_candidates`).
        let src = "namespace eval ns {\n    namespace eval sub {\n        proc p {} { return 1 }\n    }\n    sub::p\n}\nnamespace eval sub {\n    proc p {} { return 2 }\n}\n::sub::p\n";
        let analysis = analyse(src);
        let relative = definition(src, 4, 5, &analysis);
        assert_eq!(relative.len(), 1, "{relative:?}");
        assert_eq!(relative[0].start_line, 2, "must prefer ::ns::sub::p");
        let absolute = definition(src, 9, 4, &analysis);
        assert_eq!(absolute.len(), 1, "{absolute:?}");
        assert_eq!(absolute[0].start_line, 7, "::sub::p is absolute");
    }

    #[test]
    fn jump_to_proc_definition_in_two_level_nested_namespace_via_qualified_call() {
        // Issue #923: go-to-definition on a fully-qualified call to a proc
        // nested two `namespace eval` levels deep must land on its own decl.
        let src = concat!(
            "namespace eval modelTestVerTool {\n",
            "    namespace eval gui {\n",
            "        proc specAddButtonPopUp {x y} { return \"$x $y\" }\n",
            "    }\n",
            "}\n",
            "::modelTestVerTool::gui::specAddButtonPopUp 1 2\n",
        );
        let analysis = analyse(src);
        // Cursor on the qualified call (line 5).
        let locs = definition(src, 5, 30, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 2, "should land on the decl");
    }

    #[test]
    fn jump_to_proc_definition_disambiguates_same_named_procs_by_cursor() {
        // Two procs share the simple name `helper` in different two-level
        // nested namespaces. Go-to-definition from each one's own
        // declaration (a no-op jump, but exercises the lookup) must resolve
        // to *that* proc, not whichever `all_procs` entry a HashMap happens
        // to iterate first.
        let src = concat!(
            "namespace eval a {\n",
            "    namespace eval b {\n",
            "        proc helper {} { return \"a-b\" }\n",
            "    }\n",
            "}\n",
            "namespace eval c {\n",
            "    namespace eval d {\n",
            "        proc helper {} { return \"c-d\" }\n",
            "    }\n",
            "}\n",
        );
        let analysis = analyse(src);
        // Cursor on ::a::b::helper's own decl (line 2).
        let locs_ab = definition(src, 2, 14, &analysis);
        assert_eq!(locs_ab.len(), 1, "{locs_ab:?}");
        assert_eq!(locs_ab[0].start_line, 2, "must resolve to ::a::b::helper");
        // Cursor on ::c::d::helper's own decl (line 7).
        let locs_cd = definition(src, 7, 14, &analysis);
        assert_eq!(locs_cd.len(), 1, "{locs_cd:?}");
        assert_eq!(locs_cd[0].start_line, 7, "must resolve to ::c::d::helper");
    }

    #[test]
    fn jump_to_class_definition_disambiguates_same_named_classes_by_cursor() {
        // Class analogue of the proc test above.
        let src = concat!(
            "namespace eval a {\n",
            "    namespace eval b {\n",
            "        oo::class create Widget {}\n",
            "    }\n",
            "}\n",
            "namespace eval c {\n",
            "    namespace eval d {\n",
            "        oo::class create Widget {}\n",
            "    }\n",
            "}\n",
        );
        let analysis = analyse(src);
        if analysis.all_classes.len() < 2 {
            return;
        }
        let locs_ab = definition(src, 2, 26, &analysis);
        assert_eq!(locs_ab.len(), 1, "{locs_ab:?}");
        assert_eq!(locs_ab[0].start_line, 2, "must resolve to ::a::b::Widget");
        let locs_cd = definition(src, 7, 26, &analysis);
        assert_eq!(locs_cd.len(), 1, "{locs_cd:?}");
        assert_eq!(locs_cd[0].start_line, 7, "must resolve to ::c::d::Widget");
    }

    #[test]
    fn jump_to_next_super_method() {
        // `next` inside B::greet jumps to the overridden A::greet.
        let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\n";
        let analysis = analyse(src);
        // Cursor on `next` (line 5). `    method greet {} { next }`
        let locs = definition(src, 5, 23, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // A::greet's name is on line 1 at column 11.
        assert_eq!(locs[0].start_line, 1);
        assert_eq!(locs[0].start_character, 11);
    }

    #[test]
    fn jump_to_nextto_named_class_method() {
        // `nextto A` inside C::greet jumps to A::greet (skipping B).
        let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\noo::class create C {\n    superclass B\n    method greet {} { nextto A }\n}\n";
        let analysis = analyse(src);
        // Cursor on `nextto` (line 9). `    method greet {} { nextto A }`
        let locs = definition(src, 9, 22, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1, "should land on A::greet");
        assert_eq!(locs[0].start_character, 11);
    }

    #[test]
    fn jump_to_nextto_namespaced_class_method() {
        // `nextto A` names a namespaced sibling bare from within `::Ns::C`.
        // Owner-aware canonicalisation resolves it to `::Ns::A` (previously
        // only a global `::A` would have matched, so this produced nothing).
        let src = "namespace eval Ns {\n    oo::class create A {\n        method greet {} { return hi }\n    }\n    oo::class create B {\n        superclass A\n        method greet {} { next }\n    }\n    oo::class create C {\n        superclass B\n        method greet {} { nextto A }\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on `nextto` inside C::greet (line 10).
        let locs = definition(src, 10, 28, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // ::Ns::A::greet's name token is on line 2.
        assert_eq!(locs[0].start_line, 2, "should land on ::Ns::A::greet");
    }

    #[test]
    fn nextto_picks_occurrence_at_cursor_not_first_on_line() {
        // The line has a decoy `nextto` earlier (inside a string) before the
        // real `nextto A`.  With the cursor on the real one, resolution must
        // read the class after *that* occurrence (A), not the first match.
        let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\noo::class create C {\n    superclass B\n    method greet {} { set x \"nextto Z\" ; nextto A }\n}\n";
        let analysis = analyse(src);
        // Cursor inside the real `nextto` (line 9, col 44).
        let locs = definition(src, 9, 44, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "should land on A::greet, not the string decoy"
        );
        assert_eq!(locs[0].start_character, 11);
    }

    #[test]
    fn jump_to_var_definition() {
        let src = "set greeting hi\nputs $greeting\n";
        let analysis = analyse(src);
        // Cursor inside `$greeting` on line 1.
        let locs = definition(src, 1, 8, &analysis);
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
    }

    #[test]
    fn jump_to_proc_param_bareword_declaration_resolves_to_itself() {
        // TP — differential-audit finding idx 9 (main audit wave): a cursor
        // placed directly on a proc parameter's own bareword name (not a
        // `$`-prefixed read) previously resolved to nothing at all, even
        // though every `$name` read of the same parameter resolved fine —
        // `scope_chain_at` never reaches the proc's own scope for a byte
        // offset inside the parameter list, which sits textually *before*
        // that scope's `body_span` even starts.
        let src = "proc greet {name} { return $name }\n";
        let analysis = analyse(src);
        // Cursor on `name` inside the parameter list (col 12-16).
        let locs = definition(src, 0, 13, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
        assert_eq!(locs[0].start_character, 12, "{:?}", locs[0]);
        assert_eq!(locs[0].end_character, 16, "{:?}", locs[0]);
    }

    #[test]
    fn jump_to_catch_resultvar_bareword_resolves_to_the_reused_variable() {
        // TP — the finding's other confirmed shape: `catch script name`
        // reuses (writes back into) an *existing* variable — its own
        // bareword token is recorded in `VarDef.references`, not
        // `definition_span` (that stays pinned to the variable's original
        // declaration) — so a cursor placed directly on it must still
        // resolve, to the *original* declaration site.
        let src = "proc resolveSwitch {name def} {\n    catch {foo} name\n    return $name\n}\n";
        let analysis = analyse(src);
        // Cursor on the catch result-var `name` (line 1, col 16-20).
        let locs = definition(src, 1, 17, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 0,
            "must resolve to the original param declaration: {:?}",
            locs[0]
        );
        assert_eq!(locs[0].start_character, 20, "{:?}", locs[0]);
        assert_eq!(locs[0].end_character, 24, "{:?}", locs[0]);
    }

    #[test]
    fn bareword_that_is_not_any_variable_occurrence_does_not_resolve_as_a_variable() {
        // FP guard — an ordinary bareword that never appears as any
        // variable's declaration or reference span (an unrelated command's
        // plain argument) must not be mistaken for a variable declaration
        // site, even in a file containing unrelated variables.
        let src = "proc greet {name} { return $name }\nputs literal\n";
        let analysis = analyse(src);
        // Cursor on `literal` (line 1, col 5-12) — not a variable anywhere.
        let locs = definition(src, 1, 7, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn jump_to_absolutely_qualified_var_definition() {
        // TP — the exact shape both mined findings (defer.tcl's
        // `$::defer::idVar`, uri.tcl's namespace-current pattern) reduce to:
        // a `variable` declared at namespace-eval scope, referenced
        // elsewhere via its fully `::`-qualified name. Previously empty
        // (lookup_var_in_scope_chain only special-cased bare names).
        let src = "namespace eval ::simple {\n    variable v \"hello\"\n}\nputs $::simple::v\n";
        let analysis = analyse(src);
        // Cursor on `v` inside `$::simple::v` (line 3, col 16).
        let locs = definition(src, 3, 16, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1, "{:?}", locs[0]);
        assert_eq!(locs[0].start_character, 13, "{:?}", locs[0]);
        assert_eq!(locs[0].end_character, 14, "name-sized span: {:?}", locs[0]);
    }

    #[test]
    fn jump_to_relatively_qualified_var_definition_from_sibling_namespace() {
        // TP — a relative qualifier (`B::v`, no leading `::`) resolves under
        // the *cursor's* command-resolution namespace (tclsh 9.0.4/8.6.16
        // -verified: identical rule to what an unqualified command there
        // would resolve against), not the file's top level. `show` is
        // defined in `::A`, so `B::v` must reach `::A::B::v`, never a
        // top-level `::B::v`.
        let src = "namespace eval ::A {\n    namespace eval ::A::B {\n        variable v \"nested\"\n    }\n    proc show {} {\n        puts $B::v\n    }\n}\n";
        let analysis = analyse(src);
        // Cursor on `v` inside `$B::v` (line 5, col 17).
        let locs = definition(src, 5, 17, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 2, "{:?}", locs[0]);
        assert_eq!(locs[0].start_character, 17, "{:?}", locs[0]);
    }

    #[test]
    fn qualified_var_definition_for_var_declared_in_a_reopened_namespace() {
        // TP — a namespace `namespace eval`-reopened in two unrelated
        // places: the qualified reference must find the declaration
        // regardless of which block declares it (no single lexical block
        // may be assumed to hold every variable a qualified reference
        // might mean).
        let src = "namespace eval ::A {\n    proc noop {} {}\n}\nnamespace eval ::A {\n    variable v 1\n}\nputs $::A::v\n";
        let analysis = analyse(src);
        let locs = definition(src, 6, 11, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 4, "{:?}", locs[0]);
    }

    #[test]
    fn qualified_var_definition_abstains_for_unknown_namespace() {
        // TN — a qualified reference into a namespace that's never declared
        // anywhere in the file must abstain (empty), not guess at an
        // unrelated variable.
        let src = "namespace eval ::real {\n    variable v 1\n}\nputs $::nonexistent::v\n";
        let analysis = analyse(src);
        let locs = definition(src, 3, 20, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn qualified_var_definition_does_not_match_a_same_named_proc_local() {
        // FP guard — a proc's local variable must never satisfy a
        // namespace-qualified lookup even though the proc's own
        // command-resolution namespace coincides with the target: `f`'s
        // local `v` is unrelated to `::A::v`, which is never declared here.
        let src =
            "namespace eval ::A {\n    proc f {} {\n        set v 1\n    }\n}\nputs $::A::v\n";
        let analysis = analyse(src);
        let locs = definition(src, 5, 11, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    /// Issue #727: go-to-definition of a formal-parameter use must resolve to
    /// the parameter *name* in the declaration, not the proc name (proc) or the
    /// whole method/constructor body (`TclOO`). The returned range must be a
    /// single-line, name-sized span over `arg1`.
    #[test]
    fn param_definition_points_at_the_name_not_the_body() {
        let cases = [
            // (source, usage line, usage char, expected decl line, decl start col)
            (
                "proc greet {arg1 arg2} {\n    puts $arg1\n}\n",
                1,
                11,
                0,
                12,
            ),
            (
                "oo::class create C {\n    method m {arg1 arg2} {\n        puts $arg1\n    }\n}\n",
                2,
                15,
                1,
                14,
            ),
            (
                "oo::class create C {\n    constructor {arg1 arg2} {\n        puts $arg1\n    }\n}\n",
                2,
                15,
                1,
                17,
            ),
        ];
        for (src, ul, uc, dl, dc) in cases {
            let analysis = analyse(src);
            let locs = definition(src, ul, uc, &analysis);
            assert_eq!(locs.len(), 1, "one def for {src:?}: {locs:?}");
            let r = locs[0];
            assert_eq!(r.start_line, dl, "decl line for {src:?}: {r:?}");
            assert_eq!(r.start_character, dc, "decl col for {src:?}: {r:?}");
            // Name-sized: same line, spans the 4-char `arg1`.
            assert_eq!(r.end_line, dl, "decl is single-line for {src:?}: {r:?}");
            assert_eq!(
                r.end_character - r.start_character,
                4,
                "decl spans `arg1` (4 chars) for {src:?}: {r:?}",
            );
        }
    }

    #[test]
    fn no_definition_for_unknown_word() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(definition(src, 0, 6, &analysis).is_empty());
    }

    #[test]
    fn jump_to_class_definition() {
        let src = "oo::class create Greeter {}\nGreeter\n";
        let analysis = analyse(src);
        // Cursor on `Greeter` on line 1.
        let locs = definition(src, 1, 2, &analysis);
        if !locs.is_empty() {
            assert_eq!(locs[0].start_line, 0);
        }
    }

    // alias resolution

    #[test]
    fn jump_to_alias_target_proc() {
        // `interp alias {} mycmd {} greet` aliases `mycmd` to
        // the user proc `greet`.  Cursor on `mycmd` should
        // resolve to `greet`'s definition.
        let src = "proc greet {} {}\ninterp alias {} mycmd {} greet\nmycmd\n";
        let analysis = analyse(src);
        // Cursor on `mycmd` invocation on line 2.
        let locs = definition(src, 2, 2, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The target's name span is on line 0 starting at
        // column 5 (after `proc `).
        assert_eq!(locs[0].start_line, 0);
        assert_eq!(locs[0].start_character, 5);
    }

    #[test]
    fn alias_with_no_user_proc_returns_empty() {
        // `mycmd` aliases to a built-in (`puts`).  No user
        // proc; the provider returns empty rather than
        // pretending to know the location.
        let src = "interp alias {} mycmd {} puts\nmycmd\n";
        let analysis = analyse(src);
        let locs = definition(src, 1, 2, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn alias_lookup_accepts_qualified_form() {
        // The alias's qualified form (`::mycmd`) should also
        // resolve.
        let src = "proc greet {} {}\ninterp alias {} mycmd {} greet\n::mycmd\n";
        let analysis = analyse(src);
        let locs = definition(src, 2, 2, &analysis);
        // Whether find_word_span_at_position includes leading
        // `::` depends on the implementation; both should
        // jump to the target proc when matched.
        if !locs.is_empty() {
            assert_eq!(locs[0].start_line, 0);
        }
    }

    #[test]
    fn ensemble_map_subcommand_jumps_to_target_proc() {
        // TP — issue #923 idx 106, the confirmed finding's own minimal repro:
        // `namespace ensemble create -map {foo ::e::Foo}` inside `::e`, then
        // `e foo bar` at the call site. Cursor on the call-site "foo" must
        // resolve to ::e::Foo's declaration.
        let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e foo bar]\n";
        let analysis = analyse(src);
        // Cursor on "foo" in `puts [e foo bar]` (0-based line 7, col 8).
        let locs = definition(src, 7, 8, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 5);
    }

    #[test]
    fn ensemble_map_subcommand_resolves_forward_declared_differently_cased_target() {
        // TP — the subcommand's spelling ("make") differs from the target's
        // simple name ("Make") in case, and the target is declared AFTER
        // the ensemble and the call site. A fix matching on literal name
        // equality (rather than the resolved qualified name) would fail
        // this the same way it would fail the real widget.tcl repro.
        let src = "namespace eval ::widget {\n    namespace ensemble create -map {\n        make ::widget::Make\n    }\n}\nputs [widget make hello]\nproc ::widget::Make {args} { return \"make: $args\" }\n";
        let analysis = analyse(src);
        let locs = definition(src, 5, 15, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 6);
    }

    #[test]
    fn ensemble_subcommands_form_jumps_to_target_proc() {
        // TP — the `-subcommands` sibling form (not `-map`), the folded-in
        // secondary fix from the same research plan.
        let src = "namespace eval ::box {\n    proc show {} { return \"shown\" }\n    namespace ensemble create -subcommands {show}\n}\nbox show\n";
        let analysis = analyse(src);
        let locs = definition(src, 4, 5, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn ensemble_dynamic_map_mutation_still_abstains() {
        // TN — must not regress the audit's own explicitly-correct
        // abstention: a subcommand added only via a runtime
        // `namespace ensemble configure -map $var` dict-set mutation (never
        // a literal `-map {...}` list) is not statically resolvable —
        // `ensemble_subcommand_targets` never gets an entry for it.
        let src = "namespace eval ::widget {\n    namespace ensemble create -map {\n        make ::widget::Make\n    }\n}\nproc ::widget::addType {t} {\n    set m [namespace ensemble configure ::widget -map]\n    dict set m dyn ::widget::Dyn\n    namespace ensemble configure ::widget -map $m\n}\nproc ::widget::Dyn {args} { return dyn }\nwidget addType foo\nputs [widget dyn hello]\n";
        let analysis = analyse(src);
        // Cursor on "dyn" in `puts [widget dyn hello]` (0-based line 12).
        let locs = definition(src, 12, 13, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn two_ensembles_with_shared_subcommand_name_do_not_cross_contaminate() {
        // TN — validates the outer table is keyed per-ensemble, not a
        // single flat map (the reason `command_aliases` can't be reused
        // directly for this fix).
        let src = "namespace eval ::a {\n    namespace ensemble create -map {go ::a::Go}\n}\nnamespace eval ::b {\n    namespace ensemble create -map {go ::b::Go}\n}\nproc ::a::Go {} { return a }\nproc ::b::Go {} { return b }\nputs [a go]\nputs [b go]\n";
        let analysis = analyse(src);
        // `puts [a go]` is 0-based line 8; `puts [b go]` is line 9.
        let locs_a = definition(src, 8, 8, &analysis);
        assert_eq!(locs_a.len(), 1, "{locs_a:?}");
        assert_eq!(locs_a[0].start_line, 6);
        let locs_b = definition(src, 9, 8, &analysis);
        assert_eq!(locs_b.len(), 1, "{locs_b:?}");
        assert_eq!(locs_b[0].start_line, 7);
    }

    #[test]
    fn ensemble_unmapped_subcommand_abstains_rather_than_guessing() {
        // FN (accepted, documented) — a typo'd/genuinely-unmapped
        // subcommand silently abstains rather than crashing or guessing.
        // Also documents that this fix deliberately does not implement
        // Tcl's unique-prefix ensemble-subcommand matching (a real tclsh
        // would dispatch "mak" to "make" here since it's an unambiguous
        // prefix and `-prefixes` defaults to true) — a reusable, named
        // follow-up (`CommandSpec::resolve_subcommand_for_dialect`-style
        // logic), not attempted here.
        let src = "namespace eval ::widget {\n    namespace ensemble create -map {\n        make ::widget::Make\n    }\n}\nproc ::widget::Make {args} {}\nputs [widget mak hello]\n";
        let analysis = analyse(src);
        // Cursor on "mak" in `puts [widget mak hello]` (0-based line 6).
        let locs = definition(src, 6, 13, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn tk_ensemble_configure_splice_resolves_the_subcommand_to_its_real_target_not_a_decoy() {
        // TP — the finding's own repro shape (issue #923 idx 84):
        // `tk`'s built-in ensemble is extended at runtime via `namespace
        // ensemble configure tk -map [dict merge [namespace ensemble
        // configure tk -map] {systray ::tk::systray}]`, the real
        // `tk/library/systray.tcl` idiom. tclsh9.0/8.6 both confirm `tk
        // systray` really dispatches to `::tk::systray`, never a
        // same-tail-name decoy proc in an unrelated namespace — previously
        // this fell through to `fallback_proc_by_simple_name` and wrongly
        // landed on the decoy (or, with no decoy present, abstained despite
        // `systray -> ::tk::systray` being a literal, static fact).
        let src = "namespace eval ::decoy {\n    proc systray {args} { return \"DECOY\" }\n}\nproc ::tk::systray {args} { return \"real systray: $args\" }\nnamespace ensemble configure tk -map [dict merge [namespace ensemble configure tk -map] {systray ::tk::systray}]\ntk systray create -image book\n";
        let analysis = analyse(src);
        // Cursor on "systray" in the final `tk systray create ...` call
        // (0-based line 5, "tk " is 3 chars so "systray" starts at col 3).
        let locs = definition(src, 5, 5, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // Resolves to the REAL `proc ::tk::systray` (line 3), not the
        // same-tail-name decoy inside `::decoy` (line 1).
        assert_eq!(locs[0].start_line, 3);
    }

    #[test]
    fn ensemble_wholly_dynamic_map_value_abstains() {
        // FN (accepted, zero regression) — the whole `-map` value itself is
        // a variable, not a literal list; `ensemble_subcommand_targets`
        // never gets an entry for `::widget` at all.
        let src = "namespace eval ::widget {\n    variable dynamicMap {make ::widget::Make}\n    namespace ensemble create -map $dynamicMap\n}\nproc ::widget::Make {args} {}\nputs [widget make hello]\n";
        let analysis = analyse(src);
        // Cursor on "make" in `puts [widget make hello]` (0-based line 5).
        let locs = definition(src, 5, 13, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    // scope-chain $var descent

    #[test]
    fn proc_local_var_jumps_to_proc_scope_definition() {
        // `local` is defined inside `proc f`.  Cursor on
        // `$local` inside `f`'s body must jump to the
        // proc-local definition, not the global scope
        // (which has none).
        let src = "proc f {} {\n    set local 1\n    puts $local\n}\n";
        let analysis = analyse(src);
        // Cursor inside `$local` on line 2.
        let locs = definition(src, 2, 12, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The proc-local `set local 1` is on line 1.
        assert_eq!(locs[0].start_line, 1, "{:?}", locs[0]);
    }

    #[test]
    fn byte_offset_at_handles_newlines() {
        let src = "abc\ndef\nghi\n";
        let line_index = LineIndex::new(src);
        assert_eq!(byte_offset_at(&line_index, src, 0, 0), 0);
        assert_eq!(byte_offset_at(&line_index, src, 0, 3), 3);
        assert_eq!(byte_offset_at(&line_index, src, 1, 0), 4);
        assert_eq!(byte_offset_at(&line_index, src, 2, 2), 10);
    }

    #[test]
    fn byte_offset_at_uses_lsp_utf16_columns() {
        let src = "a😀b";
        let line_index = LineIndex::new(src);
        assert_eq!(byte_offset_at(&line_index, src, 0, 0), 0);
        assert_eq!(byte_offset_at(&line_index, src, 0, 1), 1);
        assert_eq!(byte_offset_at(&line_index, src, 0, 3), 5);
        assert_eq!(byte_offset_at(&line_index, src, 0, 4), 6);
    }

    #[test]
    fn span_to_range_translates_offsets() {
        let src = "abc\ndef\n";
        let line_index = LineIndex::new(src);
        // Span covering `def` (offsets 4..7).
        let span = tcl_lexer::Span::new(4, 7);
        let range = span_to_range(src, &line_index, span);
        assert_eq!(range.start_line, 1);
        assert_eq!(range.start_character, 0);
        assert_eq!(range.end_line, 1);
        assert_eq!(range.end_character, 3);
    }

    #[test]
    fn span_to_range_uses_lsp_utf16_columns() {
        let src = "é😀x\n";
        let line_index = LineIndex::new(src);
        let range = span_to_range(src, &line_index, tcl_lexer::Span::new(6, 7));
        assert_eq!(range.start_line, 0);
        assert_eq!(range.start_character, 3);
        assert_eq!(range.end_line, 0);
        assert_eq!(range.end_character, 4);
    }

    // class-member lookup

    #[test]
    fn definition_bare_sibling_method_call_without_link_abstains() {
        // FP (issue #923 idx 113) — a bareword sibling method call is NOT
        // actually reachable from another method's body unless `link`
        // exposed it that way; real tclsh: "invalid command name". Must
        // abstain rather than falsely resolve.
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the first `greet` in the `twice` body.
        // Line 2: `    method twice {} { greet ; greet }`
        // Col 22 lands on the `g` of the first `greet`.
        let locs = definition(src, 2, 22, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn definition_linked_sibling_method_call_resolves() {
        // TP (issue #923 idx 113) — `link greet` (called from the
        // constructor) makes `greet` genuinely bareword-callable from
        // every method body of this class, so the bare call now
        // correctly resolves to the method's declaration.
        let src = "oo::class create C {\n    constructor {} { link greet }\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Line 3: `    method twice {} { greet ; greet }` — same text/col
        // as the un-linked test above, one line further down.
        let locs = definition(src, 3, 22, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The method declaration is now on line 2.
        assert_eq!(locs[0].start_line, 2);
    }

    #[test]
    fn definition_my_dispatch_resolves_when_class_extended_via_separate_oo_define() {
        // Issue #923 idx 52 (main audit wave, high severity): `Gadget` is
        // created via `oo::class create` with no body, then every method —
        // including the `my Helper` call site itself — is added via a
        // *separate*, later `oo::define Gadget { ... }` block. This is
        // exactly the real corpus shape (`ticklecharts::chart`: `oo::class
        // create` at one line, all methods added via a later `oo::define`).
        // tclsh9.0/8.6 both prove `my Helper` genuinely dispatches to
        // `Helper` here — go-to-definition must resolve it, not abstain.
        let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n";
        let analysis = analyse(src);
        // Line 5: `    method Caller {} { my Helper }` — cursor on the
        // `Helper` word inside `my Helper` (col 26).
        let locs = definition(src, 5, 26, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The `Helper` method declaration is on line 4.
        assert_eq!(locs[0].start_line, 4);
    }

    #[test]
    fn definition_bare_sibling_classmethod_call_without_link_abstains() {
        // FP (issue #923 idx 113) — same shape as the method case above,
        // for `classmethod`.
        let src = "oo::class create C {\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
        let analysis = analyse(src);
        // Line 2: `    method use {} { factory }`
        // Cursor on `factory` (col 20).
        let locs = definition(src, 2, 20, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn definition_linked_sibling_classmethod_call_resolves() {
        // TP (issue #923 idx 113) — `link factory` makes the classmethod
        // call resolve.
        let src = "oo::class create C {\n    constructor {} { link factory }\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
        let analysis = analyse(src);
        // Line 3: `    method use {} { factory }` — same text/col as the
        // un-linked test above, one line further down.
        let locs = definition(src, 3, 20, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The classmethod declaration is now on line 2.
        assert_eq!(locs[0].start_line, 2);
    }

    #[test]
    fn definition_jumps_to_constructor_keyword() {
        // Bare `constructor` inside a class body jumps to the
        // constructor's declaration.
        let src = "oo::class create C {\n    constructor {arg} {}\n    method touch_ctor {} { constructor }\n}\n";
        let analysis = analyse(src);
        // Cursor on `constructor` in the `touch_ctor` body
        // (line 2 col 27).
        let locs = definition(src, 2, 27, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The analyser anchors the constructor's ``name_span``
        // on the ``constructor`` keyword token (declared on
        // line 1).  The provider keeps a body-span fallback
        // for the empty-span case, but the keyword span is
        // populated now, so the jump lands on line 1.
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_bare_sibling_property_call_without_link_abstains() {
        // FP (issue #923 idx 113) — same rationale as the method/classmethod
        // cases above, for a Tcl 9.0+ `oo::configurable` `property`
        // accessor bareword call: `[length]` is not actually reachable
        // without a `link`; real tclsh: "invalid command name".
        let src = "oo::configurable create Widget {\n    property length -get {return 42}\n    method use {} { return [length] }\n}\n";
        let analysis = analyse(src);
        // Line 2: `    method use {} { return [length] }` — col 27 lands
        // on the `l` of `length`.
        let locs = definition(src, 2, 27, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn definition_member_lookup_skipped_outside_class_body() {
        // Same word outside the class body must not surface
        // the method definition.
        let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the bare `greet` on line 3.
        let locs = definition(src, 3, 2, &analysis);
        // No proc / class / member named `greet` is in scope
        // here — the class-member lookup must not leak.
        assert!(locs.is_empty(), "{locs:?}");
    }

    // $obj method dispatch

    #[test]
    fn definition_resolves_obj_method_call() {
        // `set d [Dog new]` then `$d bark` — cursor on `bark`
        // jumps to the method declaration on Dog.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
        let analysis = analyse(src);
        // Line 4 `$d bark` — `bark` starts at col 3.
        let locs = definition(src, 4, 3, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // `method bark` is on line 1.
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_resolves_a_literal_foreach_installed_method() {
        // Issue #1277: `alpha` has no written `method alpha` word anywhere —
        // its only textual appearance is inside the loop's own literal list
        // — but go-to-definition must still land somewhere real rather than
        // finding nothing, exactly as hover/outline/completion do.
        let src = concat!(
            "oo::class create Widget {\n",
            "    foreach m {alpha beta gamma} {\n",
            "        method $m {args} { return $args }\n",
            "    }\n",
            "}\n",
            "set d [Widget new]\n",
            "$d alpha\n",
        );
        let analysis = analyse(src);
        // Line 6 `$d alpha` — `alpha` starts at col 3.
        let locs = definition(src, 6, 3, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The list `{alpha beta gamma}` is on line 1.
        assert_eq!(locs[0].start_line, 1, "{locs:?}");
    }

    #[test]
    fn definition_resolves_obj_method_in_bracket() {
        // `[$d bark]` bracketed form.
        let src =
            "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nputs [$d bark]\n";
        let analysis = analyse(src);
        // Line 4 `puts [$d bark]` — `bark` starts at col 9.
        let locs = definition(src, 4, 9, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_obj_method_unknown_instance_falls_through() {
        // `$x bark` where `x` has no recorded class — no
        // instance-method resolution, falls through (and finds
        // nothing here).
        let src = "oo::class create Dog {\n    method bark {} {}\n}\n$x bark\n";
        let analysis = analyse(src);
        let locs = definition(src, 3, 3, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn definition_resolves_bare_created_instance_command_method() {
        // Codex #881: `Dog create rex` then `rex bark` — cursor on the bare
        // `bark` jumps to the method declaration, mirroring `$obj bark`.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
        let analysis = analyse(src);
        // Line 4 `rex bark` — `bark` starts at col 4.
        let locs = definition(src, 4, 4, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_bare_var_receiver_without_dollar_does_not_resolve() {
        // `set d [Dog new]` binds `d` as a variable; a bare `d bark` (no `$`)
        // is not a valid dispatch, so it must not resolve to the method.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nd bark\n";
        let analysis = analyse(src);
        let locs = definition(src, 4, 2, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn instance_method_at_cursor_detects_dollar_head() {
        let src = "$d bark\n";
        let got = instance_method_at_cursor(src, 0, 4);
        assert_eq!(got, Some(("d".to_string(), "bark".to_string(), true)));
    }

    #[test]
    fn instance_method_at_cursor_reports_bare_head_as_non_dollar() {
        // `foo bark` — a bare-word receiver (an object command); reported
        // with `is_dollar == false`.  Whether it actually resolves to a
        // class is decided later by `receiver_instance_class`.
        let src = "foo bark\n";
        assert_eq!(
            instance_method_at_cursor(src, 0, 5),
            Some(("foo".to_string(), "bark".to_string(), false))
        );
    }

    #[test]
    fn instance_method_at_cursor_rejects_substituted_head() {
        // `[x] bark` — a command-substitution head is not a bare object
        // command.
        let src = "[x] bark\n";
        assert_eq!(instance_method_at_cursor(src, 0, 5), None);
    }

    // Method names with non-identifier characters (issue #1019 idx 16).
    // Oracle (tclsh 8.6.14 + 9.0.4): `method with-dash`, `method a.b`, and
    // TIP 558's `<ReadProp-x>` / `<WriteProp-x>` all dispatch for real.

    #[test]
    fn instance_method_at_cursor_keeps_a_hyphenated_method_name_whole() {
        // TP — `-` is part of the method word, not a boundary.
        let src = "$d with-dash\n";
        assert_eq!(
            instance_method_at_cursor(src, 0, 8),
            Some(("d".to_string(), "with-dash".to_string(), true))
        );
    }

    #[test]
    fn instance_method_at_cursor_keeps_an_angle_bracketed_method_name_whole() {
        // TP — the TIP 558 property-accessor shape, cursor inside the name.
        let src = "my <ReadProp-colour>\n";
        assert_eq!(
            instance_method_at_cursor(src, 0, 8),
            Some(("my".to_string(), "<ReadProp-colour>".to_string(), false))
        );
    }

    #[test]
    fn instance_method_at_cursor_keeps_a_dotted_method_name_whole() {
        // TP — `method a.b` dispatches in real Tcl; `.` is not a boundary.
        let src = "$d a.b\n";
        assert_eq!(
            instance_method_at_cursor(src, 0, 4),
            Some(("d".to_string(), "a.b".to_string(), true))
        );
    }

    #[test]
    fn instance_method_at_cursor_rejects_subtraction_in_an_expression() {
        // TN — `$x-1` is arithmetic. The whole `x-1` run is one word whose
        // head slice is a bare `$`, so nothing dispatches.
        let src = "expr {$x-1}\n";
        assert_eq!(instance_method_at_cursor(src, 0, 8), None);
        assert_eq!(instance_method_at_cursor(src, 0, 9), None);
    }

    #[test]
    fn instance_method_at_cursor_rejects_a_bare_comparison_operator() {
        // TN — `expr {$a < $b}` must not read as `$a` dispatching `<`:
        // the word carries no alphanumeric content.
        let src = "expr {$a < $b}\n";
        assert_eq!(instance_method_at_cursor(src, 0, 9), None);
    }

    #[test]
    fn hyphenated_and_angle_bracketed_methods_resolve_from_a_call_site() {
        // TP — end-to-end. `with-dash` starts with an ASCII lowercase
        // letter so it is exported and dispatches externally; the TIP 558
        // accessor `<ReadProp-x>` starts with `<` so it is *unexported* and
        // only reachable from inside the class via `my` (oracle: tclsh
        // 9.0.4 answers `unknown method "<foo>"` for an external call but
        // runs `my <ReadProp-x>` fine).
        let src = concat!(
            "oo::class create C {\n",
            "    method with-dash {} { return 1 }\n",
            "    method <ReadProp-x> {} { return 2 }\n",
            "    method probe {} { my <ReadProp-x> }\n",
            "}\n",
            "C create rex\n",
            "rex with-dash\n",
        );
        let analysis = analyse(src);
        let dash = definition(src, 6, 6, &analysis);
        assert_eq!(dash.len(), 1, "{dash:?}");
        assert_eq!(dash[0].start_line, 1);
        assert_eq!(dash[0].start_character, 11);
        let prop = definition(src, 3, 28, &analysis);
        assert_eq!(prop.len(), 1, "{prop:?}");
        assert_eq!(prop[0].start_line, 2);
        assert_eq!(prop[0].start_character, 11);
    }

    #[test]
    fn unexported_angle_bracketed_method_is_not_externally_dispatchable() {
        // TN — the export rule still applies to the widened word: an
        // external `rex <ReadProp-x>` is `unknown method` in real Tcl
        // (verified, tclsh 8.6.14 + 9.0.4), so it must resolve to nothing
        // rather than to the declaration.
        let src = concat!(
            "oo::class create C {\n",
            "    method <ReadProp-x> {} { return 2 }\n",
            "}\n",
            "C create rex\n",
            "rex <ReadProp-x>\n",
        );
        let analysis = analyse(src);
        assert!(definition(src, 4, 8, &analysis).is_empty());
    }

    #[test]
    fn receiver_instance_class_gates_bare_on_created_commands() {
        // `set b [Bar new]` binds `b` as a variable; `Bar create rex` binds
        // `rex` as an object command.  A bare receiver resolves only for the
        // command (`rex`), not the variable (`b`); a `$`-receiver resolves
        // for either.
        let src =
            "oo::class create Bar {\n    method get {} {}\n}\nset b [Bar new]\nBar create rex\n";
        let analysis = analyse(src);
        assert!(receiver_instance_class(&analysis, "rex", false).is_some());
        assert!(receiver_instance_class(&analysis, "b", false).is_none());
        assert!(receiver_instance_class(&analysis, "b", true).is_some());
    }

    // Class-command dispatch (issue #923 idx 120): `CLASS method` for a
    // classmethod / `self method`, distinct from `$obj method` / `NAME
    // method` instance dispatch above.

    #[test]
    fn classmethod_call_on_its_own_class_resolves() {
        // TP — the finding's own `ActiveRecord find` shape: a classmethod
        // called directly on the class that declares it (Part 2 alone;
        // ooutil's `classmethod` keyword was already correctly extracted
        // into `class_methods` by the existing class-body walker).
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\nActiveRecord find foo bar\n";
        let analysis = analyse(src);
        let locs = definition(src, 3, 14, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
        assert_eq!(locs[0].start_character, 16);
    }

    #[test]
    fn classmethod_call_on_an_inheriting_subclass_resolves() {
        // TP — the finding's second repro shape: `Table find`, where
        // `Table` inherits `find` from `ActiveRecord` via the ordinary
        // superclass MRO walk (ooutil's `classmethod` propagates to a
        // subclass's own bound command — confirmed against tclsh).
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\n";
        let analysis = analyse(src);
        let locs = definition(src, 6, 7, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
        assert_eq!(locs[0].start_character, 16);
    }

    #[test]
    fn self_method_call_on_its_own_class_resolves() {
        // TP — requires BOTH Part 1 (self recognised at all) and Part 2
        // (the class-command receiver path).
        let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\nWidget make gadget\n";
        let analysis = analyse(src);
        let locs = definition(src, 3, 8, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
        assert_eq!(locs[0].start_character, 16);
    }

    #[test]
    fn self_method_not_inherited_by_a_non_overriding_subclass() {
        // TN — the critical precision test for `is_self_method`: unlike
        // `ooutil`'s `classmethod`, a plain `self method` is NOT inherited
        // by a subclass's own bound command (confirmed against tclsh:
        // `Gadget make` raises `unknown method "make"`). A naive reuse of
        // the ordinary MRO walk (which DOES include Widget in Gadget's
        // MRO) would incorrectly resolve this.
        let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\noo::class create Gadget {\n    superclass Widget\n}\nGadget make foo\n";
        let analysis = analyse(src);
        let locs = definition(src, 6, 8, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn instance_calling_a_classmethod_does_not_resolve() {
        // TN — a real instance can never dispatch a classmethod (real
        // tclsh: `unknown method "find": must be <cloned>, create, ...`).
        // This is the companion tightening bundled with Part 2 (the two
        // pre-existing instance-dispatch call sites now pass
        // `MethodBucket::Instance`, which excludes `class_methods`
        // entirely) — `completion.rs`'s `method_items` already documented
        // and implemented this same exclusion for its own suggestions.
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\nActiveRecord create rec1\nrec1 find foo bar\n";
        let analysis = analyse(src);
        let locs = definition(src, 4, 5, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn bare_var_receiver_without_a_bound_command_does_not_resolve_class_dispatch() {
        // TN — regression guard mirroring
        // `receiver_instance_class_gates_bare_on_created_commands`: `d` is
        // a plain variable (never bound as a command by `create`), so
        // neither the existing instance-command branch nor the new
        // class-command branch may fire for it.
        let src = "oo::class create ActiveRecord {\n    classmethod find {args} {}\n}\nset d [ActiveRecord new]\nd find foo bar\n";
        let analysis = analyse(src);
        let locs = definition(src, 4, 2, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn foreach_rename_reinstall_call_site_resolves_to_the_wrapper_not_the_stale_original() {
        // TP — issue #923 idx 86, the finding's own `tk/library/
        // accessibility.tcl` repro shape: a literal-list `foreach` renames
        // each classic widget command away and reinstalls a wrapper proc
        // under the same original name. tclsh9.0/8.6 both prove `button`
        // is the *new* wrapper proc after the loop runs — the old body is
        // reachable only via `::tk::accessible::orig_button`. Go-to
        // -definition on a `button` call made after the loop must resolve
        // to the wrapper's own declaration (inside the loop), not the
        // stale, pre-rename `proc button` at the top of the file.
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
        // Line 8: `set r1 [button .b1]` — cursor on "button".
        let locs = definition(src, 8, 9, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // Resolves to the wrapper's own `proc ::$wtype ...` declaration
        // (line 5), not the stale original `proc button` (line 0).
        assert_eq!(locs[0].start_line, 5);
    }

    #[test]
    fn foreach_rename_reinstall_second_element_also_resolves_to_its_own_wrapper() {
        // TP sibling — proves the fix isn't overfit to the *first* literal
        // list element: `entry` (the second element) resolves too.
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
        // Line 9: `set r2 [entry .e1]` — cursor on "entry".
        let locs = definition(src, 9, 9, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 5);
    }

    // tcl::OptProc — the `opt` package's automatic-option-parsing proc
    // definer (issue #923 idx 90): a call site must resolve to the real
    // `tcl::OptProc` declaration, never a stale stub.

    #[test]
    fn opt_proc_call_site_resolves_to_its_declaration() {
        let src = "::tcl::OptProc greet {child -use -display} { return $child }\ngreet foo\n";
        let analysis = analyse(src);
        // Line 1: `greet foo` — cursor on "greet".
        let locs = definition(src, 1, 0, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 0, "{locs:?}");
        assert_eq!(locs[0].start_character, 15, "{locs:?}");
    }

    // Command-position gate + resolved-indirect-head preference —
    // issues #1137 idx 50 and #1133. Both land on the same seam: the
    // "otherwise it is a CALL" fallback, which used to hand whatever word
    // the text scan returned straight to `resolve_called_proc`.

    #[test]
    fn fp_argument_word_sharing_a_procs_name_is_not_resolved_as_a_command() {
        // FP guard — issue #1137 idx 50, the reduced control with zero
        // indirection. `dump` is the *first argument* of `anotherproc`, not
        // a command; tclsh never looks it up as one, so answering with the
        // same-named proc is a wrong answer, not a miss. Must abstain.
        let src = "proc anotherproc {a b} { return $a }\nproc dump {x} { return $x }\nproc caller {} {\n    return [anotherproc dump 5]\n}\n";
        let analysis = analyse(src);
        // Line 3: `    return [anotherproc dump 5]` — cursor on `dump`.
        let locs = definition(src, 3, 24, &analysis);
        assert!(
            locs.is_empty(),
            "an argument word must not resolve as a command: {locs:?}"
        );
    }

    #[test]
    fn tp_the_head_beside_that_argument_still_resolves() {
        // TP — the very same call's real head must keep resolving, so the
        // gate above narrows nothing it should not.
        let src = "proc anotherproc {a b} { return $a }\nproc dump {x} { return $x }\nproc caller {} {\n    return [anotherproc dump 5]\n}\n";
        let analysis = analyse(src);
        // Line 3, col 12 — inside `anotherproc`.
        let locs = definition(src, 3, 12, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 0, "{locs:?}");
    }

    #[test]
    fn tn_a_command_head_in_every_registry_known_script_argument_still_resolves() {
        // TN — the gate reads the analyser's recorded invocations, which
        // cover every registry-declared script argument. A call written in
        // an `if` body, a `switch` arm, a `foreach` body, and a command
        // -prefix callback must all still resolve.
        let src = "proc helper {} {}\nif {1} {\n    helper\n}\nswitch x {\n    a { helper }\n}\nforeach i {1} {\n    helper\n}\nlsort -command helper {1 2}\n";
        let analysis = analyse(src);
        for (line, col) in [(2, 4), (5, 8), (8, 4), (10, 15)] {
            let locs = definition(src, line, col, &analysis);
            assert_eq!(locs.len(), 1, "line {line} col {col}: {locs:?}");
            assert_eq!(locs[0].start_line, 0, "line {line} col {col}: {locs:?}");
        }
    }

    #[test]
    fn tp_resolved_indirect_head_wins_over_a_same_named_decoy() {
        // TP — issue #1133. The analyser settles `set ns ::tc;
        // ${ns}::setdef` to `::tc::setdef` and records it as an `indirect`
        // invocation. Before this fix `definition()` ignored the record and
        // fell through to the bareword lookup, which answered the unrelated
        // `::other::setdef`.
        let src = "namespace eval ::tc { proc setdef {} { return 1 } }\nnamespace eval ::other { proc setdef {} { return 2 } }\nset ns ::tc\n${ns}::setdef\n";
        let analysis = analyse(src);
        // Line 3, col 7 — inside the `${ns}::setdef` head word.
        let locs = definition(src, 3, 7, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 0,
            "must reach ::tc::setdef (line 0), not the ::other decoy (line 1): {locs:?}"
        );
    }

    #[test]
    fn tn_a_bare_dollar_cmd_head_still_answers_the_variable_it_reads() {
        // TN / documented limit — a whole-word `$cmd` head *is* a variable
        // read, and the position tier answers that first (it is asked long
        // before any command resolution). The indirect-head preference
        // therefore covers the spellings whose word is not itself a bare
        // variable reference — `${ns}::setdef`, a dispatch-table literal —
        // and deliberately leaves the plain `$cmd` cursor on its variable.
        let src = "proc greet {} { return hi }\nset cmd greet\n$cmd\n";
        let analysis = analyse(src);
        let locs = definition(src, 2, 1, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(
            locs[0].start_line, 1,
            "a `$cmd` cursor answers the variable declaration: {locs:?}"
        );
    }

    #[test]
    fn fn_unresolved_indirect_head_does_not_block_the_bareword_fallback() {
        // FP guard for the preference itself — an indirect head the
        // analyser could NOT settle must not suppress an answer the
        // ordinary path can still give. Here the neighbouring plain call
        // still resolves.
        let src = "proc greet {} { return hi }\nset cmd $env(WHATEVER)\n$cmd\ngreet\n";
        let analysis = analyse(src);
        let locs = definition(src, 3, 0, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 0, "{locs:?}");
    }

    #[test]
    fn tn_a_proc_declaration_name_still_resolves_to_itself() {
        // TN — a declaration's own name token is an *argument* of `proc`,
        // so the head gate would refuse it; the declaration-site check runs
        // first and must keep answering.
        let src = "proc greet {name} { puts $name }\n";
        let analysis = analyse(src);
        let locs = definition(src, 0, 6, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 0, "{locs:?}");
    }

    #[test]
    fn tp_definition_on_an_implicit_parent_namespace_answers_the_covering_prefix() {
        // Issue #1113 item 1 — `namespace eval ::p::q::r {}` really creates
        // `::p::q` (tclsh 8.6.16 / 9.0.4: `namespace exists ::p::q` is 1),
        // but the name is written nowhere on its own.  The answer is the
        // prefix of the deeper word that spells exactly it.
        let src = "namespace eval ::p::q::r {}\nnamespace children ::p::q\n";
        let analysis = analyse(src);
        // Line 1, col 20 — inside the `::p::q` reference.
        let locs = definition(src, 1, 20, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 0, "{locs:?}");
        assert_eq!(locs[0].start_character, 15, "{locs:?}");
        assert_eq!(
            locs[0].end_character, 21,
            "the answer must stop at `::p::q`, not cover `::p::q::r`: {locs:?}",
        );
    }

    #[test]
    fn tn_a_declared_namespace_still_answers_its_own_block() {
        // TN — the implicit fallback only runs when nothing declares the
        // namespace outright.
        let src =
            "namespace eval ::p::q {}\nnamespace eval ::p::q::r {}\nnamespace children ::p::q\n";
        let analysis = analyse(src);
        let locs = definition(src, 2, 20, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 0, "{locs:?}");
    }
}
