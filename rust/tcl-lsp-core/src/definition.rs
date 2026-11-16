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

//! Definition and declaration source selection.
//!
//! Native naming profiles select retained original inputs before presenting a
//! location. The complete current source image, full lexer configuration and
//! selected Registry must agree with the analysis. A missing, stale or unknown
//! original query is terminal; reporting labels cannot reconstruct that query.
//!
//! The local provider composes the shared readonly owners:
//!
//! * [`crate::namespace_symbol`] selects exact namespace bytes and policy,
//!   including written implicit-parent geometry.
//! * [`crate::variable_symbol`] selects an authentic variable occurrence and
//!   its source frame, then exposes matching declaration spans. This grants no
//!   runtime cell, value or observer capability.
//! * [`crate::method_symbol`] keeps an actual call-point method entry separate
//!   from a possible source declaration. Own-object workers retain their own
//!   allocation and generation; class and instance tables cannot replace them.
//! * Original procedure and class lookups retain their selected source
//!   allocation. Alias and renamed routes can lead to that allocation without
//!   becoming direct declaration spellings or editable references.
//!
//! Source-declaration advice requires a unique current original candidate.
//! Repeated publications, earlier occupied non-class slots and unresolved
//! operands are preserved as selection boundaries. Exact expression function
//! occurrences use their independently selected fixed-table or command-lookup
//! purpose; an expression identifier is not fabricated as a command-head word.
//!
//! The server supplies independently owned current document inventories for
//! workspace selection. A returned byte span supplies source geometry only;
//! conversion to an LSP range does not issue identity or edit permission.
//! Type-definition and implementation navigation use their own typed readonly
//! declaration and relation owners rather than a nominal object-type map.
//!
//! Profiles that explicitly allow lexical declaration advice retain a separate
//! compatibility path. Its reporting procedure/class/variable maps, namespace
//! helpers and bounded legacy member scans do not participate in Native
//! original selection. BIG-IP configuration object navigation is a separate
//! dialect consumer and does not supply Tcl command or cell authority.

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::indirection;
use tcl_lexer::{LineIndex, Utf16Col};
use tcl_registry::CommandRegistry;
use tcl_registry::definer::DeclaredMemberVisibility;

use crate::hover::find_word_span_at_position;
use crate::namespace_import::{ExportVerdict, NamespaceExportOracle};
use crate::original_context::CurrentSourcePosition;
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
/// silently reversing the per-import-site export rule. Pairing them
/// structurally makes that unrepresentable.
#[derive(Clone, Copy)]
pub struct ProgramExports<'a> {
    /// The document being resolved, spelled as the workspace index knows it.
    pub uri: &'a str,
    /// The workspace's export records — typically
    /// `crate::workspace_index::WorkspaceIndex::export_snapshot`.
    pub oracle: &'a dyn NamespaceExportOracle,
}

/// Request-local compatibility context for [`resolve_called_proc`].
/// The optional Registry protects builtin bindings. Without a program export
/// oracle, in-document export records decide; absence is evidence only when
/// this document records an export for that namespace.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    let Some(position) = CurrentSourcePosition::capture_variable(source, analysis, line, character)
    else {
        return Vec::new();
    };
    let view = CallResolution {
        registry: None,
        program,
    };
    let (line, character) = position.line_character();
    let line_index = LineIndex::new(source);

    let decl_byte_offset = position.offset();
    if program_forced_import_abstains(analysis, program, decl_byte_offset) {
        return Vec::new();
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::vendor_declaration::select_at_offset(source, analysis, decl_byte_offset)
    {
        return selected
            .into_iter()
            .map(|selected| span_to_range(source, &line_index, selected.span()))
            .collect();
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::namespace_symbol::select_at_offset(source, analysis, decl_byte_offset)
    {
        return selected.map_or_else(Vec::new, |symbol| {
            crate::namespace_symbol::original_namespace_declaration_spans(source, analysis, &symbol)
                .into_iter()
                .map(|span| span_to_range(source, &line_index, span))
                .collect()
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::variable_symbol::select_navigation(source, analysis, line, character, view)
    {
        return selected.map_or_else(Vec::new, |occurrence| {
            occurrence
                .declaration_spans(analysis)
                .into_iter()
                .map(|span| span_to_range(source, &line_index, span))
                .collect()
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::method_symbol::local_candidate(source, analysis, line, character)
    {
        return selected.map_or_else(Vec::new, |candidate| {
            vec![span_to_range(
                source,
                &line_index,
                candidate.declaration_span(),
            )]
        });
    }
    // Steps 1a-1d are cursor-position decisions: a `$var` read, bareword
    // declaration, `expr` math-function call, and two pure-data positions.
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

    word_definition(
        source,
        &line_index,
        decl_byte_offset,
        DefCtx {
            line,
            character,
            analysis,
            resolution: view,
        },
    )
}

fn word_definition(
    source: &str,
    line_index: &LineIndex,
    decl_byte_offset: u32,
    ctx: DefCtx<'_>,
) -> Vec<LspRange> {
    let DefCtx {
        line,
        character,
        analysis,
        resolution: view,
    } = ctx;
    // 2. Bare word — proc, class, class-member, or alias.
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    if let Some(result) =
        list_built_self_method_definition(source, line_index, analysis, &word, decl_byte_offset)
    {
        return result;
    }
    // `$obj method` / `[$obj method]` — when the cursor sits on
    // the method-name token of an instance-method call and the
    // instance variable's class is known, jump to the method
    // declaration.  Checked before the proc lookup so a method
    // call resolves to the method even when a same-named proc
    // exists.
    if let Some(result) = instance_method_definition(analysis, source, line_index, line, character)
    {
        return result;
    }
    // `<ensemble> <subcommand>` — a static `namespace ensemble create
    // -map`/`-subcommands` mapping.  Checked before the
    // generic "otherwise it is a CALL" resolution below: real Tcl never
    // independently looks up `make` as a command (only the pair `widget make`
    // dispatches), so a coincidental same-named proc elsewhere in the
    // workspace must never win.
    if let Some((head, sub, false)) =
        original_instance_method_at_cursor(source, analysis, line, character)
        && let Some(proc_def) = ensemble_called_procedure(
            analysis,
            source,
            (&head, &sub),
            decl_byte_offset,
            view.with_registry(crate::registry_for_analysis(analysis)),
        )
    {
        return vec![span_to_range(source, line_index, proc_def.name_span)];
    }
    // `next` / `nextto` inside a method body — jump to the super-method in
    // the MRO chain that the enclosing method overrides (`next`), or to the
    // named class's copy of it (`nextto Cls`).
    if let Some(result) =
        next_dispatch_definition(analysis, source, line_index, line, character, &word)
    {
        return result;
    }
    // Prefer the declaration whose own name span covers the cursor (so a
    // same-named proc in another namespace's own decl resolves to *that*
    // one — mirrors `references::proc_references` / `rename::rename_proc`).
    // `proc_declaration_sites` rather than `all_procs`, so a declaration a
    // later same-named `proc` displaced still resolves to itself instead of
    // silently jumping to its own successor — `all_procs` only ever keeps the
    // winner's span.
    let cursor_offset = decl_byte_offset;
    if let Some(span) = procedure_name_at_cursor(analysis, cursor_offset) {
        return vec![span_to_range(source, line_index, span)];
    }
    // An **already-resolved indirect head** the analyser settled — a
    // `${ns}::setdef` whose `$ns` is a constant, a constant `$cmd` dispatch —
    // is asked before every name-based path below: the span does not carry
    // the written command name, so the bareword lookup can only ever answer
    // with a coincidentally same-named decoy.
    if !analysis.allows_retained_logical_declaration_advice()
        && let Some(span) = resolved_indirect_head_target(analysis, cursor_offset)
    {
        return vec![span_to_range(source, line_index, span)];
    }
    if let Some(target) = positioned_definition_target(analysis, source, line_index, cursor_offset)
    {
        return target;
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
    // alias has already displaced (tclsh 8.6.16/9.0.4:
    // after `interp alias {} ::ttk::spinbox {} ::tk::spinbox`, calling
    // `::ttk::spinbox` runs `::tk::spinbox`'s body, and the original proc is
    // unreachable under that name). The hop is order-gated, so a rename or
    // alias written *after* this call site does not apply and the ordinary
    // resolution below still wins (tclsh: a `hello` before `rename greet
    // hello` is `invalid command name "hello"`).
    if let Some(span) = indirect_definition_target(analysis, view, source, cursor_offset, &word) {
        return vec![span_to_range(source, line_index, span)];
    }
    // `Factory::make` — [incr Tcl]'s colon-qualified class-proc dispatch
    // Asked **before** the stock proc resolution below, because
    // itcl installs a custom command resolver on every class namespace and Tcl
    // consults it ahead of the ordinary lookup: inside a class's own bodies the
    // class proc wins even when a real `::Factory::make` proc exists (oracle in
    // `itcl_self_qualified_candidate`).  Order is safe because
    // `itcl_class_proc_target` applies the whole precedence itself — its own
    // resolver first, then stock resolution — and answers `None` unless the
    // winning candidate really is an itcl class-scoped `proc`, so a genuine
    // `::ns::helper` call still falls through to `resolve_called_proc`.
    if let Some(span) = itcl_class_proc_declaration(analysis, &namespace, &word) {
        return vec![span_to_range(source, line_index, span)];
    }
    // …and only when the cursor's word really is the enclosing command's
    // head. Without this the pure text scan lets an ordinary *argument*
    // resolve as a command name — see
    // [`offset_is_command_head`] for the gate and its one documented limit.
    if analysis.allows_retained_logical_declaration_advice() {
        if let Some(call) = tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_for_cursor(
                source, analysis, cursor_offset,
            )
        {
            return call.declaration(analysis).map_or_else(Vec::new, |header| {
                vec![span_to_range(source, line_index, header.metadata().name_span)]
            });
        }
    } else if offset_is_command_head(analysis, cursor_offset)
        && let Some(proc_def) = resolve_called_proc(
            analysis,
            source,
            &namespace,
            &word,
            cursor_offset,
            view.with_registry(crate::registry_for_analysis(analysis)),
        )
    {
        // A proc redefined later in the document is two definitions sharing
        // one name; the call reaches whichever was in effect where it is
        // written, not unconditionally the last.
        let in_effect = analysis
            .proc_def_in_effect_at(&proc_def.qualified_name, cursor_offset)
            .unwrap_or(proc_def);
        return vec![span_to_range(source, line_index, in_effect.name_span)];
    }
    if let Some(span) = class_declaration_at(source, analysis, view, &word, cursor_offset) {
        return vec![span_to_range(source, line_index, span)];
    }
    // Class-member lookup — when the cursor sits inside a
    // class body, walk that class's methods / properties /
    // constructors / destructor for a name match.  Covers
    // `my method` calls inside the body plus bare member
    // references.
    if let Some(span) = lookup_class_member(source, analysis, &word, cursor_offset) {
        return vec![span_to_range(source, line_index, span)];
    }
    Vec::new()
}

fn list_built_self_method_definition(
    source: &str,
    line_index: &LineIndex,
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<Vec<LspRange>> {
    let selected = crate::receiver_identity::method_at_cursor(analysis, source, cursor_offset)?;
    (selected.method.name == word)
        .then(|| vec![span_to_range(source, line_index, selected.method.name_span)])
}

fn next_dispatch_definition(
    analysis: &AnalysisResult,
    source: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
    _word: &str,
) -> Option<Vec<LspRange>> {
    next_dispatch_target(analysis, source, line_index, line, character)
        .map(|span| vec![span_to_range(source, line_index, span)])
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
/// jumps to the *first* header while `p` jumps to the second.
fn indirect_definition_target(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    source: &str,
    cursor_off: u32,
    word: &str,
) -> Option<tcl_lexer::Span> {
    if let Some(reference) = invocation_reference_at(analysis, cursor_off)
        .and_then(|invocation| invocation.resolved_command_reference.as_ref())
    {
        let definition = reference
            .linked_definition()
            .or_else(|| reference.definition())?;
        return analysis
            .proc_for_definition(definition, source)
            .map(|declaration| declaration.name_span)
            .or_else(|| {
                analysis
                    .class_for_definition(definition, source)
                    .map(|declaration| declaration.name_span)
            });
    }
    if analysis.allows_retained_logical_declaration_advice() {
        let call =
            tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_for_cursor(
                source,
                analysis,
                cursor_off,
            )?;
        return call
            .declaration(analysis)
            .map(|header| header.metadata().name_span);
    }
    let hop = command_indirection(analysis, word, cursor_off)?;
    let lookup = hop.lookup_spelling()?;
    let registry = crate::registry_for_analysis(analysis);
    if let Some(proc_def) = resolve_called_proc(
        analysis,
        source,
        "::",
        &lookup,
        cursor_off,
        ctx.with_registry(registry),
    ) {
        let captured = analysis
            .proc_def_in_effect_at(&proc_def.qualified_name, hop.resolve_at)
            .unwrap_or(proc_def);
        return Some(captured.name_span);
    }
    class_declaration_at(source, analysis, ctx, &hop.target, cursor_off)
}

/// Original written procedure header, including displaced declarations.
fn procedure_name_at_cursor(
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<tcl_lexer::Span> {
    if analysis.allows_retained_logical_declaration_advice() {
        let mut selected = analysis
            .logical_source_procedure_declarations()
            .filter_map(|header| {
                let span = header.original_name_word()?.span();
                (span.start() <= cursor_offset && cursor_offset < span.end()).then_some(span)
            });
        let span = selected.next()?;
        return selected.next().is_none().then_some(span);
    }
    analysis
        .proc_declaration_sites
        .iter()
        .find(|(_, span)| span.start() <= cursor_offset && cursor_offset < span.end())
        .map(|(_, span)| *span)
}

/// A known called slot settles local navigation even when it has no source
/// definition. `Some([])` preserves that refusal against name assistance.
fn positioned_definition_target(
    analysis: &AnalysisResult,
    source: &str,
    line_index: &LineIndex,
    cursor_offset: u32,
) -> Option<Vec<LspRange>> {
    if analysis.allows_retained_logical_declaration_advice() {
        let call = tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_for_cursor(
            source, analysis, cursor_offset,
        )?;
        return Some(call.declaration(analysis).map_or_else(Vec::new, |header| {
            vec![span_to_range(
                source,
                line_index,
                header.metadata().name_span,
            )]
        }));
    }
    let invocation = invocation_reference_at(analysis, cursor_offset)?;
    if let Some(record) =
        crate::original_oo::counted_class_for_invocation(source, analysis, invocation)
    {
        return Some(vec![span_to_range(
            source,
            line_index,
            record.name_input().span(),
        )]);
    }
    let reference = invocation.resolved_command_reference.as_ref()?;
    let span = reference
        .linked_definition()
        .or_else(|| reference.definition())
        .and_then(|definition| {
            analysis
                .proc_for_definition(definition, source)
                .map(|declaration| declaration.name_span)
                .or_else(|| {
                    analysis
                        .class_for_definition(definition, source)
                        .map(|declaration| declaration.name_span)
                })
        });
    Some(span.map_or_default(|span| vec![span_to_range(source, line_index, span)]))
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
    /// Hover threads the same view, so dropping it here would make the two
    /// providers disagree on one cursor.
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
///    substitutes ([`lookup_var_read_at`]).
/// 2. **A bareword variable declaration / same-cell write site** — a `set x` /
///    `variable x` target, a proc/method parameter, a `catch` result-var. See
///    [`var_def_at_declaration_offset`] for why this can't reuse the ordinary
///    scope-chain walk.
/// 3. **An `expr` math-function call** — inside an expression a `NAME(` word is
///    a function-call production resolved through `{ns}::tcl::mathfunc::NAME`
///    then `::tcl::mathfunc::NAME`, not an ordinary command lookup. Asked
///    before the bareword paths because the
///    word-span scan would hand them `sin(1.0)` / `li(` — the function-call
///    syntax glued on — and because a coincidentally same-named proc elsewhere
///    must never win. A built-in resolves to no Tcl source, so an empty result
///    is the right answer there.
/// 4. **A word inside an enclosing proc/method's own parameter list** — a
///    parameter name (answered at 2) or a default value, never a command
///    reference.
fn position_definition(
    source: &str,
    line_index: &LineIndex,
    cursor_off: u32,
    ctx: DefCtx<'_>,
) -> Option<Vec<LspRange>> {
    if crate::receiver_identity::definition_reference_at_cursor(ctx.analysis, source, cursor_off)
        .is_some()
    {
        return Some(
            crate::receiver_identity::method_at_cursor(ctx.analysis, source, cursor_off)
                .map_or_default(|selected| {
                    vec![span_to_range(source, line_index, selected.method.name_span)]
                }),
        );
    }
    let DefCtx {
        line,
        character,
        analysis,
        resolution,
    } = ctx;
    // Resolved through the shared gate, not the raw character scan: a cursor
    // inside a brace-quoted variable-name word (`set {$n} 1`) is not a `$n`
    // reference, and must fall through to the declaration-span search below
    // so it answers the *literal* cell.
    if let Some(var_name) =
        substituting_var_at_position(source, analysis, line, character, cursor_off)
    {
        if let Some(var_def) = lookup_var_read_at(analysis, source, cursor_off, &var_name) {
            return Some(vec![span_to_range(
                source,
                line_index,
                var_def.definition_span,
            )]);
        }
        // Nothing in this frame assigns it — a callee may create it here
        // through `upvar`, and then the call-site word that names it *is*
        // the creating write, the nearest thing the frame has to a
        // declaration.
        return Some(caller_frame_definition(
            source, line_index, analysis, resolution, cursor_off, &var_name,
        ));
    }
    let param_position = parameter_list_position_at(analysis, source, cursor_off);
    // No `Computed` guard is needed here: a computed parameter list registers
    // no per-parameter `VarDef` at all, so there is no stub named
    // `"[makeargs]"` for this lookup to land on.
    if let Some(var_def) = var_def_at_declaration_offset(&analysis.global_scope, cursor_off) {
        return Some(vec![span_to_range(
            source,
            line_index,
            var_def.definition_span,
        )]);
    }
    // A word naming a namespace (`namespace children ::tomato`, and the
    // `::tomato` of `namespace eval ::tomato { … }` itself).
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
            // word, which names a different one.
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
    if let std::ops::ControlFlow::Break(selected) =
        crate::math_function_symbol::select_at_offset(source, analysis, cursor_off)
    {
        return Some(
            selected
                .and_then(|selected| selected.procedure_metadata())
                .map_or_default(|row| {
                    vec![span_to_range(source, line_index, row.metadata().name_span)]
                }),
        );
    }
    if let Some(inv) = crate::expr_context::mathfunc_call_at(analysis, cursor_off) {
        if !analysis.allows_lexical_declaration_advice() {
            return Some(Vec::new());
        }
        return Some(
            inv.resolved_qualified_name
                .as_deref()
                .and_then(|resolved| analysis.all_procs.get(resolved))
                .map_or_default(|proc_def| {
                    vec![span_to_range(source, line_index, proc_def.name_span)]
                }),
        );
    }
    (param_position == ParamListPosition::LiteralData).then(Vec::new)
}

/// Go-to-definition target for a **caller-frame** variable read: the call-site
/// word whose callee creates the variable here through `upvar`.
///
/// Empty when no call site binds the name — the `$`-led read then abstains
/// rather than falling through to bareword resolution, which would let an
/// unrelated same-named method look like the answer.
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
        crate::profile_for_analysis(analysis),
        resolution.with_registry(
            resolution
                .registry
                .unwrap_or_else(|| crate::registry_for_analysis(analysis)),
        ),
        cursor_off,
        name,
    );
    crate::caller_frame::primary_binding(&bindings)
        .map_or_default(|binding| vec![span_to_range(source, line_index, binding.arg_span)])
}

/// Go-to-definition for an instance-method call site (`$obj method` /
/// `[$obj method]` / `my method`).  Extracted from [`definition`] to keep
/// that function within the line budget.
///
/// `None` means the cursor isn't on an instance-method call at all — the
/// caller falls through to the remaining checks (proc / class / alias).
/// `Some` is definitive: once the receiver's per-object override or class
/// is resolved, the call reaches exactly what this returns (possibly an
/// empty vector for an unexported/undefined method), never a same-named proc.
fn instance_method_definition(
    analysis: &AnalysisResult,
    source: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
) -> Option<Vec<LspRange>> {
    let cursor = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    if let Some(selected) = crate::receiver_identity::method_at_cursor(analysis, source, cursor) {
        return Some(vec![span_to_range(
            source,
            line_index,
            selected.method.name_span,
        )]);
    }
    if !analysis.allows_lexical_declaration_advice()
        && let Some((_, _, method)) = declared_self_dispatch_at(source, analysis, cursor)
    {
        return Some(vec![span_to_range(source, line_index, method.name_span)]);
    }
    None
}

/// Original linked command heads select their retained method declaration.
/// Constructor and destructor keywords retain their own declaration path.
fn lookup_class_member(
    source: &str,
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<tcl_lexer::Span> {
    if let Some(linked) = tcl_compiler::analyser::types::original_source_linked_member_at(
        analysis,
        source,
        cursor_offset,
    ) {
        return Some(linked.target().metadata().name_span);
    }
    let class_def = analysis
        .all_classes
        .get(enclosing_class_at(analysis, cursor_offset)?)?;
    // A constructor or destructor keyword: the member it declared says so.
    if let Some(c) = class_def
        .constructors
        .first()
        .filter(|c| c.is_declared_by_keyword(word))
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
    if let Some(d) = class_def
        .destructor
        .as_ref()
        .filter(|d| d.is_declared_by_keyword(word))
    {
        if !d.name_span.is_empty() {
            return Some(d.name_span);
        }
        return Some(d.body_span);
    }
    None
}

/// The folded per-object member state for the receiver binding at the call
/// site: the [`tcl_compiler::analyser::ObjectMemberState`]
/// whose anchor resolves to the same variable binding as the call — the
/// same lexical variable-scope extent used by the object-state summary. `None`
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
    let call_offset = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
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

/// Withhold workspace method fallback for a retained external visibility mask
/// or an unavailable current source query. Per-object source summaries remain
/// conditional source advice; they do not establish Native method visibility.
/// Missing source currency withholds fallback without asserting a hidden method.
#[must_use]
pub fn object_dispatch_mask(
    analysis: &AnalysisResult,
    source: &str,
    line: u32,
    character: u32,
) -> ExternalObjectDispatchMask {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    let Some(position) = CurrentSourcePosition::capture(source, analysis, line, character) else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    let cursor = position.offset();
    let mut mask = ExternalObjectDispatchMask::Clear;
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if !command
                .argv
                .get(1)
                .is_some_and(|word| word.span.start() <= cursor && cursor < word.span.end())
            {
                return false;
            }
            mask = object_dispatch_mask_at_command(analysis, source, command, &current, cursor);
            true
        },
    );
    mask
}

/// Original visibility and conditional source advice stay separate. None of
/// these outcomes proves method execution or permission to resolve a fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalObjectDispatchMask {
    /// This local source query supplies no visibility mask.
    Clear,
    /// A retained original Native receiver entry is not exported.
    Native,
    /// Selected per-object source configuration describes a possible mask.
    ConditionalSource,
    /// Current source or the mask's required original input is unavailable.
    Unavailable,
}

impl ExternalObjectDispatchMask {
    /// Withhold workspace selection for a mask or an unavailable source query.
    #[must_use]
    pub const fn withholds_workspace_fallback(self) -> bool {
        match self {
            Self::Clear => false,
            Self::Native | Self::ConditionalSource | Self::Unavailable => true,
        }
    }
}

fn object_dispatch_mask_at_command(
    analysis: &AnalysisResult,
    source: &str,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    current: &crate::original_context::CurrentSourceContext<'_>,
    cursor: u32,
) -> ExternalObjectDispatchMask {
    use tcl_compiler::command_binding::SourceOriginKind;
    let Some(head) = command.argv.first() else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    let Some(realm) = analysis.retained_command_realm() else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    let binding = realm.invocation_at_source(command.name(), head.span.start());
    if !matches!(binding.source_origin().map(|origin| origin.kind()),
        Some(SourceOriginKind::Authored(bytes)) if bytes.as_ref() == source.as_bytes())
    {
        return ExternalObjectDispatchMask::Unavailable;
    }
    let image = tcl_lexer::SourceImage::document(source);
    let tokens = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        current.config(),
        command,
    );
    let Some(original) = tcl_compiler::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        head.span.start(),
        tokens.native_lexer_config(current.config()),
    ) else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    let Some(head) = tokens.word_exprs.first() else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    if binding
        .receiver_self_method_entry(current.registry())
        .is_some()
        || original_internal_dispatch(source, analysis, command, current)
    {
        return ExternalObjectDispatchMask::Clear;
    }
    let variable_receiver = head.sole_variable_substitution().is_some();
    let accesses = variable_receiver
        .then(|| realm.variable_accesses_for_invocation_args(head.source().span.start()));
    let selected = if let Some(accesses) = &accesses {
        accesses
            .iter()
            .find_map(|access| binding.object_receiver_method_entry(access, head))
    } else {
        binding
            .frozen_object_receiver_method_entry(head)
            .or_else(|| binding.named_object_receiver_method_entry())
    };
    if let Some((class, entry, _)) = selected {
        if crate::receiver_identity::original_class(analysis, source, class).is_none()
            || !matches!(entry.declaration().source.kind(), SourceOriginKind::Authored(bytes)
                if bytes.as_ref() == source.as_bytes())
        {
            return ExternalObjectDispatchMask::Unavailable;
        }
        return if entry.is_exported() {
            ExternalObjectDispatchMask::Clear
        } else {
            ExternalObjectDispatchMask::Native
        };
    }
    if original
        .first()
        .is_some_and(|head| original_self_receiver_head(source, analysis, head, current))
    {
        // An original self result identifies the receiver only. External
        // visibility requires its independent actual method entry above.
        return ExternalObjectDispatchMask::Unavailable;
    }
    object_dispatch_configuration_mask(analysis, source, current, cursor)
}

fn object_dispatch_configuration_mask(
    analysis: &AnalysisResult,
    source: &str,
    current: &crate::original_context::CurrentSourceContext<'_>,
    cursor: u32,
) -> ExternalObjectDispatchMask {
    let position = LineIndex::new(source).position_at_utf16(cursor, source);
    let Some((receiver, method, _)) = original_instance_method_at_cursor(
        source,
        analysis,
        position.line,
        position.character.get(),
    ) else {
        return ExternalObjectDispatchMask::Clear;
    };
    let Some(state) = object_member_state_at(
        analysis,
        source,
        &receiver,
        position.line,
        position.character.get(),
    ) else {
        return if analysis.object_member_state.contains_key(&receiver) {
            ExternalObjectDispatchMask::Unavailable
        } else {
            ExternalObjectDispatchMask::Clear
        };
    };
    let same_namespace = analysis
        .original_namespace_scope_at(cursor)
        .zip(analysis.original_namespace_scope_at(state.anchor_offset))
        .is_some_and(|(call, anchor)| call == anchor);
    if !same_namespace
        || state.anchor_offset > cursor
        || state.conditional
        || !original_object_configuration_anchor(source, analysis, state.anchor_offset, current)
    {
        return ExternalObjectDispatchMask::Unavailable;
    }
    if state
        .methods
        .get(&method)
        .is_some_and(|method| method.visibility != DeclaredMemberVisibility::Public.as_str())
        || state.unexports.contains(&method)
    {
        ExternalObjectDispatchMask::ConditionalSource
    } else {
        ExternalObjectDispatchMask::Clear
    }
}

/// The summary's source anchor belongs to a selected per-object configuration.
/// This authenticates source advice, without identifying a Native receiver.
fn original_object_configuration_anchor(
    source: &str,
    analysis: &AnalysisResult,
    anchor: u32,
    current: &crate::original_context::CurrentSourceContext<'_>,
) -> bool {
    use tcl_compiler::registry_invocation::source_structure;
    let context = current.context();
    let mut selected = false;
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if !command.argv.iter().any(|word| word.span.start() == anchor) {
                return false;
            }
            let Some(words) = source_structure::source_registry_words(source, analysis, command)
            else {
                return false;
            };
            selected = words.with_source_schema(&context, |schema| {
                schema.authored_source_definition_body_grammar().is_some()
                    && schema.state_transitions().facts().iter().any(|fact| {
                        let tcl_registry::StateTransition::ObjectDispatch(
                            tcl_registry::ObjectDispatchTransition::Configure {
                                target,
                                layer: tcl_registry::ObjectDispatchLayer::Object,
                            },
                        ) = &fact.transition
                        else {
                            return false;
                        };
                        target
                            .argument_index()
                            .and_then(|index| words.original_argument_word(index))
                            .is_some_and(|word| word.span().start() == anchor)
                    })
            }) == Some(true);
            selected
        },
    );
    selected
}

/// Conditional internal dispatch through the original selected descriptor.
/// Self identity alone never changes an external call's visibility purpose.
fn original_internal_dispatch(
    source: &str,
    analysis: &AnalysisResult,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    current: &crate::original_context::CurrentSourceContext<'_>,
) -> bool {
    use tcl_compiler::registry_invocation::source_structure;
    let context = current.context();
    source_structure::source_registry_words(source, analysis, command).and_then(|words| {
        words.with_source_schema(&context, |schema| {
            schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
        })
    }) == Some(true)
}

/// The analysed class whose body holds `offset`, when its declared member set
/// is complete. A definition vocabulary that may rewrite or add members (Clay)
/// records an incomplete set, which answers no dispatch.
pub(crate) fn declared_class_at(
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<&tcl_compiler::analyser::types::ClassDef> {
    analysis
        .all_classes
        .get(enclosing_class_at(analysis, offset)?)
        .filter(|class| !class.member_set_incomplete)
}

/// The declared member a written self-dispatch reaches in the analysed class
/// whose body holds `offset`: the receiving class, the providing class and
/// the method. A method body the analysis never entered still belongs to its
/// class, so hover and go-to-definition read the class's declared members
/// there. This names no receiver and grants no edit.
pub(crate) fn declared_self_dispatch_at<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    offset: u32,
) -> Option<(
    &'a tcl_compiler::analyser::types::ClassDef,
    &'a tcl_compiler::analyser::types::ClassDef,
    &'a tcl_compiler::analyser::types::MethodDef,
)> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let receiver = declared_class_at(analysis, offset)?;
    let mut selector = None;
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if !command
                .argv
                .get(1)
                .is_some_and(|word| word.span.start() <= offset && offset <= word.span.end())
            {
                return false;
            }
            if original_internal_dispatch(source, analysis, command, &current)
                || command_head_is_self_receiver(source, analysis, command, &current)
            {
                selector = command.texts.get(1).cloned();
            }
            selector.is_some()
        },
    );
    let method = selector?;
    let provider = analysis
        .class_hierarchy()
        .method_target(&receiver.qualified_name, &method)?;
    let provider = analysis.all_classes.get(provider)?;
    Some((receiver, provider, provider.methods.get(&method)?))
}

/// Whether this command's head word is a self-receiver substitution
/// (`[self] m`), read from the head's own descriptor.
fn command_head_is_self_receiver(
    source: &str,
    analysis: &AnalysisResult,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    current: &crate::original_context::CurrentSourceContext<'_>,
) -> bool {
    let Some(head) = command.argv.first() else {
        return false;
    };
    let image = tcl_lexer::SourceImage::document(source);
    let tokens = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        current.config(),
        command,
    );
    tcl_compiler::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        head.span.start(),
        tokens.native_lexer_config(current.config()),
    )
    .is_some_and(|words| {
        words
            .first()
            .is_some_and(|head| original_self_receiver_head(source, analysis, head, current))
    })
}

/// The selected source head may describe the current object's identity.
/// This shape grants no internal dispatcher or external visibility receipt.
fn original_self_receiver_head(
    source: &str,
    analysis: &AnalysisResult,
    head: &tcl_lexer::NativeWord,
    current: &crate::original_context::CurrentSourceContext<'_>,
) -> bool {
    use tcl_compiler::registry_invocation::source_structure;
    let context = current.context();
    let Some(child) = source_structure::original_single_source_command_substitution(head) else {
        return false;
    };
    let Some(head) = child.command().words.first() else {
        return false;
    };
    let Some(words) =
        source_structure::source_registry_words_at(source, analysis, head.span().start())
    else {
        return false;
    };
    if words.head_source().and_then(|head| head.word()) != Some(head) {
        return false;
    }
    words.with_source_schema(&context, |schema| {
        let Some(shape) = schema.authored_source_arity() else {
            return false;
        };
        if shape.command.self_receiver_words.is_empty() {
            return false;
        }
        match schema.words.arguments().exact_argv_len() {
            Some(0) => shape.arity.accepts(0),
            Some(1) => {
                shape.arity.accepts(1)
                    && shape.command.self_receiver_words.iter().any(|word| {
                        match schema.words.arguments().get(0) {
                            Some(tcl_registry::InvocationWord::Literal(value)) => value == *word,
                            Some(tcl_registry::InvocationWord::KnownBytes(value)) => {
                                value == word.as_bytes()
                            }
                            _ => false,
                        }
                    })
            }
            _ => false,
        }
    }) == Some(true)
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
    source_frame_extent(analysis, offset)
}

/// Lexical source-frame boundary only; no entered-frame identity is issued.
pub(crate) fn source_frame_extent(analysis: &AnalysisResult, offset: u32) -> Option<(u32, u32)> {
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
/// covers the *first* one recorded.  Ties (nested definitions) resolve to
/// the narrowest containing span.
///
/// The single canonical implementation of this check: every other module in
/// this crate calls it rather than keeping its own copy, so the multi-span
/// rule applies everywhere uniformly.
pub(crate) fn enclosing_class_at(analysis: &AnalysisResult, offset: u32) -> Option<&str> {
    analysis
        .class_body_spans
        .iter()
        .filter(|(_, span)| span.start() < offset && offset < span.end())
        .min_by_key(|(_, span)| span.end() - span.start())
        .map(|(name, _)| name.as_str())
}

/// Which of a class's two method buckets a dispatch receiver reaches,
/// consulted by [`method_dispatch_definition`] and,
/// via [`logical_receiver_candidate_at`], by every other consumer of
/// [`logical_receiver_candidate_at`] that also needs its own method lookup
/// restricted to the matching bucket (`completion.rs`'s `method_items`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodBucket {
    /// `$obj method` / `my method` — dispatch on an *instance*:
    /// `methods` only, never `class_methods` (a class-side method is
    /// never itself instance-callable — real tclsh raises `unknown
    /// method` for it, so `ActiveRecord create rec1; rec1 find` must not
    /// resolve to the class-side `find`).
    Instance,
    /// `CLASS method` — dispatch on the class's own bound command:
    /// `class_methods` only (a class-side method is never
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

/// Resolve `TclOO` `next` / `nextto` at the cursor to the super-method's
/// `name_span`.
///
/// `next` inside `method m` of class `C` dispatches `m` one step further
/// down the object's MRO, resolved in `C`'s own MRO (the sound
/// single-dispatch approximation): the next class after `C` that provides
/// `m`.  `nextto Base` restarts the search at `Base`.  The enclosing class
/// and method are found from the cursor's byte offset.
fn next_dispatch_target(
    analysis: &AnalysisResult,
    source: &str,
    _line_index: &LineIndex,
    line: u32,
    character: u32,
) -> Option<tcl_lexer::Span> {
    let cursor = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    let input = analysis.resolved_input.as_ref()?;
    let (class_q, method, body) =
        tcl_compiler::analyser::class_hierarchy::original_source_member_at(
            analysis,
            &tcl_lexer::SourceImage::document(source),
            input.lexer_config(),
            cursor,
        )?;
    let (_, target) =
        crate::references::scan_next_dispatch_sites_with_target(source, analysis, body)
            .into_iter()
            .find(|(head, _)| head.start() <= cursor && cursor < head.end())?;
    let start_from = match target {
        Some(target) => Some(canonicalise_class(analysis, cursor, &target)?),
        None => None,
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

/// Canonicalise a written `nextto` class name to the qualified form keyed in
/// `all_classes`, or `None` when the name does not resolve unambiguously.
///
/// A `TclOO` member body resolves a command word through the object's own
/// namespace, the registry's implicit member-body path, then `::` — never the
/// class's declaring namespace (tclsh 8.6.18 / 9.0.4: `nextto A` inside
/// `::Ns::B` raises `A does not refer to an object`). A class created inside
/// the object namespace at run time is not distinguished from a root class,
/// so the root lookup answers unless the name reaches a class under a path
/// namespace first; that case is refused rather than guessed. Outside a
/// member body the written scope resolves it.
pub(crate) fn canonicalise_class(analysis: &AnalysisResult, at: u32, name: &str) -> Option<String> {
    use tcl_compiler::analyser::class_hierarchy::{
        resolve_written_class_name, resolve_written_class_name_in_scope, source_namespace_at,
    };
    let path = tcl_compiler::analyser::scope::implicit_command_namespace_path_at(
        &analysis.global_scope,
        at,
    );
    if path.is_empty() {
        let namespace = source_namespace_at(&analysis.global_scope, at)?;
        return resolve_written_class_name_in_scope(name, namespace, &analysis.all_classes);
    }
    let shadowed = !tcl_syntax::naming::is_absolute(name)
        && path.iter().any(|namespace| {
            resolve_written_class_name(
                &tcl_syntax::naming::qualify(namespace, name),
                &analysis.all_classes,
            )
            .is_some()
        });
    if shadowed {
        return None;
    }
    resolve_written_class_name(name, &analysis.all_classes)
}

/// Current complete command-head and selector syntax. Actual receiver identity,
/// method visibility and source edits require their independent owners.
pub(crate) fn original_instance_method_at_cursor(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<(String, String, bool)> {
    let cursor = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    crate::source_structure::original_method_site_at(source, analysis, cursor)
}

#[cfg(test)]
pub(crate) fn instance_method_at_cursor(
    source: &str,
    line: u32,
    character: u32,
    config: tcl_lexer::LexerConfig,
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

    // A complete variable at this line's head already has canonical geometry.
    // FirstClose names may contain a literal `{`; it is not an enclosing script.
    let line_prefix: String = chars[..wstart].iter().collect();
    let line_head = line_prefix.trim();
    let sole_line_head = tcl_lexer::word_parts::whole_var_ref(line_head.as_bytes(), config)
        .ok()
        .flatten()
        .map(|_| line_head);

    // Command-segment start: nearest *unmatched* `;` / `[` / `{` to the
    // left, else the line start. Depth-tracked — a `]`/`}`
    // seen first means the following `[`/`{` closes a balanced
    // substitution that is itself part of the head (`[self] m`'s own
    // brackets), not a boundary to stop at; only a `[`/`{` with no
    // pending closer is a genuine enclosing scope.
    let mut seg_start = 0;
    let mut depth: i32 = 0;
    for i in (0..wstart).rev() {
        match chars[i] {
            ']' | '}' => depth += 1,
            '[' | '{' if depth > 0 => depth -= 1,
            '[' | '{' => {
                seg_start = i + 1;
                break;
            }
            ';' if depth == 0 => {
                seg_start = i + 1;
                break;
            }
            _ => {}
        }
    }
    // The head must be exactly one whitespace-delimited token (the
    // receiver), so the method is word-index 1 — *except* a command
    // substitution, which is one token by Tcl's own quoting rules even
    // though it can contain internal whitespace (`[self object]`):
    // checked as a whole before falling back to
    // `split_whitespace`, which would otherwise split it in two.
    let prefix: String = chars[seg_start..wstart].iter().collect();
    let trimmed = prefix.trim();
    let head = if let Some(head) = sole_line_head {
        head
    } else if tcl_compiler::value_shapes::parse_command_substitution_with_config(trimmed, config)
        .is_some()
    {
        trimmed
    } else {
        let head_tokens: Vec<&str> = prefix.split_whitespace().collect();
        if head_tokens.len() != 1 {
            return None;
        }
        head_tokens[0]
    };
    if head.starts_with('$') {
        // A complete sole substitution is source geometry, not receiver identity.
        // Its configured closer must consume the whole head before a reporting
        // name can reach the independent method/source owner below.
        tcl_lexer::word_parts::whole_var_ref(head.as_bytes(), config).ok()??;
        let inst = tcl_syntax::naming::var_reference_for_style(head, config.braced_var);
        if inst.is_empty() {
            return None;
        }
        Some((inst.to_string(), method, true))
    } else if tcl_compiler::value_shapes::parse_command_substitution_with_config(head, config)
        .is_some()
    {
        // A well-formed single command substitution (`[cmd ?args?]`) — e.g.
        // `TclOO`'s `[self]`/`[self object]`, a receiver the registry
        // resolves at compile time regardless of the object's runtime name
        // This function only extracts the receiver text;
        // whether `head` names anything real is answered downstream by the
        // original selected descriptor (`original_internal_dispatch`)
        // exactly as for a bareword receiver below — most substitutions
        // resolve to nothing there, which is the correct outcome, not a
        // reason to reject the shape here.
        Some((head.to_string(), method, false))
    } else {
        // Bare `objcmd` receiver — a plain word naming an object command.
        // Any other decorated head (`{…}`, quoted, an ill-formed `[…]`) is
        // not a bare object command; `logical_receiver_candidate_at` further
        // gates this on `created_instance_commands`.
        if head.is_empty() || head.contains(['[', ']', '{', '}', '"', '(', ')', '$']) {
            return None;
        }
        Some((head.to_string(), method, false))
    }
}

/// Logical method-completion candidates retain their class and bucket together.
/// Bare names require an authored instance-command declaration or class name;
/// a variable receiver requires its positioned unique class projection. Native
/// source construction and temporal receiver identity retain separate owners.
pub(crate) fn logical_receiver_candidate_at<'a>(
    analysis: &'a AnalysisResult,
    receiver: &str,
    is_dollar: bool,
    offset: u32,
) -> Option<(&'a String, MethodBucket)> {
    // Logical declaration advice is the sole compatibility ingress. Native
    // source construction and current receiver identity retain separate owners.
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    if is_dollar {
        return lattice_singleton_class(analysis, receiver, offset)
            .map(|class| (class, MethodBucket::Instance));
    }
    if analysis.created_instance_commands.contains(receiver)
        && let Some(class) = analysis.instance_classes.get(receiver)
    {
        return Some((class, MethodBucket::Instance));
    }
    let qualified = tcl_compiler::analyser::class_hierarchy::resolve_written_class_name(
        receiver,
        &analysis.all_classes,
    )?;
    analysis
        .all_classes
        .get(&qualified)
        .map(|class| (&class.qualified_name, MethodBucket::Class))
}

/// The unique class of the original variable read containing `offset`, or
/// `None` when its reaching contents are unknown or admit multiple classes.
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

/// The command name a word written at `cursor_off` actually reaches once the
/// document's `rename` / `interp alias` statements are taken into account, or
/// `None` when the command table leaves the word alone there.
///
/// The single navigation entry to the shared hop walk
/// ([`tcl_compiler::analyser::indirection::walk`]) that the analyser's
/// constructor typing already uses — order-gated, hop-capped, and declining an
/// argument-prepending alias, all decided in one place so a class that W307
/// resolves through a rename and a class go-to-definition resolves through the
/// same rename can never disagree.
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
    let hop = command_indirection(analysis, word, cursor_off)?;
    hop.lookup_spelling()?;
    Some(hop.target)
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
/// link so its ordering matches the in-document provider's.
///
/// A name can carry both a `rename` and an `interp alias` record; the document
/// gains the name as soon as the *first* of them runs, so the question is
/// asked of the earliest — reading one map ahead of the other would call a
/// name pending on the strength of a mutation that is not the one that
/// introduced it.
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

/// Resolve `head sub` (as returned by [`original_instance_method_at_cursor`], with
/// `is_dollar == false`) to the target command it dispatches to, when
/// `head` names a known `namespace ensemble create -map`/`-subcommands`
/// ensemble and `sub` resolves against its literal, statically-recorded
/// subcommands — `namespace` is the call site's own
/// command-resolution namespace, so a relative `head` (the overwhelmingly
/// common case: an ensemble dispatches through its own short name) resolves
/// the same way an ordinary bare command call would.
///
/// `sub` need not be written in full: a real ensemble abbreviates, so
/// `e fo` dispatching to `-map {foo ::e::Foo}` is navigable here too
/// The rule is
/// [`AnalysisResult::resolve_ensemble_subcommand`]'s — exact first, then a
/// unique prefix unless the ensemble declared `-prefixes 0` — so navigation
/// lands on whatever the ensemble itself would dispatch, and abstains
/// wherever it would error.
pub(crate) fn ensemble_subcommand_target<'a>(
    analysis: &'a AnalysisResult,
    namespace: &str,
    head: &str,
    sub: &str,
) -> Option<&'a str> {
    tcl_syntax::naming::bareword_resolution_candidates(namespace, head)
        .into_iter()
        .find_map(|cand| analysis.resolve_ensemble_subcommand(&cand, sub))
        .map(|entry| entry.target.as_str())
}

/// The proc whose **own declaration name token** covers `cursor_off`.
///
/// The `greet` of `proc greet {…} {…}` is an *argument* of `proc`, never a
/// command head, so a provider gated on head position
/// ([`offset_is_command_head`]) must answer it here instead. Declaration
/// cursors in the retained Logical domain select their own genuine original
/// header, including a declaration displaced by a later same-named procedure.
/// The separate legacy branch retains its reporting-map behaviour.
#[must_use]
pub(crate) fn proc_declaration_at(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::analyser::ProcDef> {
    let image = analysis.retained_command_realm()?.original_source_image()?;
    let retained_source = std::str::from_utf8(image.bytes()).ok()?;
    crate::original_context::CurrentSourceContext::capture(retained_source, analysis)?;
    if analysis.allows_retained_logical_declaration_advice() {
        return logical_procedure_header_at(analysis, cursor_off)
            .map(tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration::metadata);
    }
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

/// Select the unique genuine original Logical header under this source cursor.
/// The header owner checks the complete retained image, input and realm.
fn logical_procedure_header_at(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration> {
    let mut selected = analysis
        .logical_source_procedure_declarations()
        .filter(|header| {
            header.original_name_word().is_some_and(|word| {
                word.span().start() <= cursor_off && cursor_off < word.span().end()
            })
        });
    let header = selected.next()?;
    selected.next().is_none().then_some(header)
}

/// Select readonly metadata through this actual original Logical source call.
fn logical_called_procedure_at<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cursor_off: u32,
) -> Option<&'a tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration> {
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let call =
        tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_for_cursor(
            source,
            analysis,
            cursor_off,
        )?;
    call.declaration(analysis)
}

/// The procedure `head sub` at `cursor_off` dispatches to: the dispatch's own
/// target receipt at the ensemble invocation, else the static ensemble table's
/// target resolved like a call.
pub(crate) fn ensemble_called_procedure<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    (head, sub): (&str, &str),
    cursor_off: u32,
    view: CallResolution<'_>,
) -> Option<&'a tcl_compiler::analyser::ProcDef> {
    let receipt = analysis
        .command_invocations
        .iter()
        .find(|inv| {
            inv.ensemble_dispatch.is_some()
                && inv.range.start() <= cursor_off
                && cursor_off < inv.range.end()
        })
        .and_then(|inv| inv.resolved_command_reference.as_ref()?.definition())
        .and_then(|definition| analysis.proc_for_definition(definition, source));
    if receipt.is_some() {
        return receipt;
    }
    let namespace = namespace_context_at(
        &analysis.global_scope,
        cursor_off,
        &analysis.namespace_overrides,
    );
    let target = ensemble_subcommand_target(analysis, &namespace, head, sub)?;
    resolve_called_proc(analysis, source, "::", target, cursor_off, view)
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
        .filter(|inv| {
            inv.lookup.is_execution_site()
                && inv.range.start() <= cursor_off
                && cursor_off < inv.range.end()
        })
        .min_by_key(|inv| inv.range.end() - inv.range.start())
}

/// A positioned name reference, including names consumed by another command.
/// Its lookup point is retained independently of the editable name span.
#[must_use]
pub(crate) fn invocation_reference_at(
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
/// Handing whatever word the text scan returned straight to
/// [`resolve_called_proc`] would let an ordinary *argument* that happens to
/// share a proc's name resolve onto that proc — a wrong answer, not an
/// abstention (`return [$types(callback:$tag) jsondump $huddle_object]`
/// resolving the ensemble subcommand `jsondump` onto the very proc containing
/// the call, which `anotherproc dump` reproduces with no indirection at all).
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
/// The analyser resolves `set ns ::tc; ${ns}::setdef` to `::tc::setdef` and
/// records it as a `command_invocations` entry with `indirect: true`.  Falling
/// through to the bareword lookup instead would answer an unrelated same-named
/// `::other::setdef`: the span does not carry the written command name, so
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
    let invocation = indirect_invocation_head_at(analysis, cursor_off)?;
    let definition = if let Some(reference) = &invocation.resolved_command_reference {
        Some(
            reference
                .linked_definition()
                .or_else(|| reference.definition())?,
        )
    } else {
        invocation.resolved_definition.as_ref()
    };
    if let Some(definition) = definition {
        let tcl_compiler::command_binding::SourceOriginKind::Authored(source) =
            definition.allocation().site.source.kind()
        else {
            return None;
        };
        return analysis
            .class_for_definition(definition, source.try_text().ok()?)
            .map(|declaration| declaration.name_span);
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
    indirect_invocation_head_at(analysis, cursor_off)?
        .resolved_qualified_name
        .as_deref()
}

fn indirect_invocation_head_at(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::signature_scan::types::SignatureCommandInvocation> {
    analysis
        .command_invocations
        .iter()
        .filter(|inv| {
            inv.lookup.is_execution_site()
                && inv.indirect
                && inv.range.start() <= cursor_off
                && cursor_off < inv.range.end()
        })
        .min_by_key(|inv| inv.range.end() - inv.range.start())
}

/// The `ProcDef` an already-resolved indirect head at `cursor_off` reaches —
/// the hover-side twin of [`resolved_indirect_head_target`], so
/// hover and go-to-definition answer the same indirection from one record.
#[must_use]
pub(crate) fn resolved_indirect_head_proc(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&tcl_compiler::analyser::ProcDef> {
    let invocation = indirect_invocation_head_at(analysis, cursor_off)?;
    let definition = if let Some(reference) = &invocation.resolved_command_reference {
        Some(
            reference
                .linked_definition()
                .or_else(|| reference.definition())?,
        )
    } else {
        invocation.resolved_definition.as_ref()
    };
    if let Some(definition) = definition {
        let tcl_compiler::command_binding::SourceOriginKind::Authored(source) =
            definition.allocation().site.source.kind()
        else {
            return None;
        };
        return analysis.proc_for_definition(definition, source.try_text().ok()?);
    }
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
/// Clamped geometry for compatibility/presentation callers. Exact source
/// selection uses `CurrentSourcePosition` instead. A returned offset alone
/// supplies no current-source, declaration or edit authority.
/// Takes a pre-built index so repeated presentation conversions can share it.
pub(crate) fn byte_offset_at(
    line_index: &LineIndex,
    source: &str,
    line: u32,
    character: u32,
) -> u32 {
    line_index.offset_at_utf16(line, Utf16Col::new(character), source)
}

/// Query declaration tables eligible under the checked Logical source owner.
pub(crate) fn lookup_var_in_scope_chain<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    byte_offset: u32,
    name: &str,
) -> Option<&'a tcl_compiler::analyser::VarDef> {
    let config = analysis.body_lexer_config?;
    let advice = analysis.logical_variable_scope_advice(
        &tcl_lexer::SourceImage::document(source),
        config,
        byte_offset,
    )?;
    lookup_var_in_logical_scope(&advice, name)
}

pub(crate) fn lookup_var_in_logical_scope<'a>(
    advice: &tcl_compiler::analyser::scope::LogicalVariableScopeAdvice<'a>,
    name: &str,
) -> Option<&'a tcl_compiler::analyser::VarDef> {
    advice.variable_definition(name)
}

/// Current original source occurrences related to the actual declaration.
/// Conditional aliases remain readonly source candidates, with no successful
/// link, physical cell or editable-reference-set claim. Missing provenance is
/// definitive; reporting names and shared declaration spans are not identities.
#[must_use]
pub(crate) fn linked_var_reference_spans(
    source: &str,
    analysis: &AnalysisResult,
    var_def: &tcl_compiler::analyser::VarDef,
    cursor: u32,
) -> Option<Vec<tcl_lexer::Span>> {
    let declaration =
        crate::variable_symbol::declaration_at_cursor(source, analysis, var_def, cursor)?;
    Some(
        crate::variable_symbol::alias_template_reference_spans(
            source,
            analysis,
            declaration.symbol(),
            true,
        )
        .into_iter()
        .filter(|span| *span != declaration.declaration_span())
        .collect(),
    )
}

/// Logical namespace-variable coordinate for the cross-document tier.
/// Relative qualified occurrences require the same checked source namespace
/// as the in-document lookup; unknown callers supply no namespace.
/// Bare declarations use their independently retained declaration table.
#[must_use]
pub fn qualified_variable_cell_at(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<String> {
    if !analysis.allows_lexical_declaration_advice()
        || !std::ptr::eq(analysis.resolved_profile()?, dialect)
    {
        return None;
    }
    let cursor_off = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    let scope = analysis.logical_variable_scope_advice(
        &tcl_lexer::SourceImage::document(source),
        analysis.body_lexer_config?,
        cursor_off,
    )?;
    if let Some(name) = substituting_var_at_position(source, analysis, line, character, cursor_off)
    {
        let base = name.as_str();
        if !tcl_syntax::naming::is_qualified(base.as_bytes()) {
            return None;
        }
        return scope.namespace_variable_coordinate(base);
    }
    var_def_at_declaration_offset(&analysis.global_scope, cursor_off)
        .and_then(|variable| {
            crate::variable_symbol::declaration_at_cursor(source, analysis, variable, cursor_off)
        })
        .and_then(|declaration| declaration.symbol().checked_c_namespace_key())
        .or_else(|| logical_variable_write_coordinate(source, analysis, cursor_off, &scope))
}

fn logical_variable_write_coordinate(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    scope: &tcl_compiler::analyser::scope::LogicalVariableScopeAdvice<'_>,
) -> Option<String> {
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let mut coordinate = None;
    for advice in &analysis.original_variable_write_advice {
        let span = advice.span();
        if !(span.start() <= cursor
            && cursor < span.end()
            && advice.matches_context(&context)
            && crate::original_name_edit::original_input_matches_source(
                source,
                analysis,
                advice.original_name_input(),
                span,
            ))
        {
            continue;
        }
        let input = advice.original_name_input();
        let protocol = input.policy().recipe();
        let selected = match advice.receiver_form().input_form(protocol, input.bytes())? {
            tcl_syntax::naming::NativeVariableInputForm::Combined(bytes) => {
                protocol.combined_variable_input(bytes)
            }
            tcl_syntax::naming::NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        let written = std::str::from_utf8(selected.root().selected()).ok()?;
        let current = scope.namespace_variable_coordinate(written)?;
        if coordinate
            .as_ref()
            .is_some_and(|previous| previous != &current)
        {
            return None;
        }
        coordinate = Some(current);
    }
    coordinate
}

/// Whether a retained original namespace-variable producer or conflict covers
/// this cursor, or whether exact current cursor ownership is unavailable.
/// This only withdraws legacy fallback; it issues no symbol.
#[must_use]
pub fn original_variable_cursor_retained(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> bool {
    let Some(position) = CurrentSourcePosition::capture_variable(source, analysis, line, character)
    else {
        return true;
    };
    let offset = position.offset();
    let contains = |span: tcl_lexer::Span| span.start() <= offset && offset < span.end();
    analysis
        .original_variable_symbols
        .iter()
        .any(|occurrence| contains(occurrence.span()))
        || analysis
            .original_variable_symbol_conflicts
            .iter()
            .copied()
            .any(contains)
        || analysis
            .qualified_var_refs
            .iter()
            .any(|reference| reference.original_name_input.is_some() && contains(reference.span))
}

/// Original namespace-variable occurrence at a cursor in the exact retained
/// document and grammar. Byte identity comes from the shared source issuer;
/// optional display keys supply no fallback for missing or stale receipts.
#[must_use]
pub fn original_variable_occurrence_at<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    line: u32,
    character: u32,
) -> Option<&'a tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
    let offset =
        CurrentSourcePosition::capture_variable(source, analysis, line, character)?.offset();
    analysis.original_variable_symbol_in_source(
        &tcl_lexer::SourceImage::document(source),
        analysis.body_lexer_config?,
        offset,
    )
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
/// the scope that owns it — go-to-definition / hover / find-references would
/// answer nothing for a cursor placed directly on a parameter's or a `catch`
/// result-var's own declaring token, even though every `$name` read of the
/// same variable resolves correctly. Instead this walks every scope in
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

/// A lexical `$name` read at the cursor, independently of the original
/// source syntax label. Scope lookup requires explicit lexical declaration
/// advice; Native providers consume their original variable naming owners.
pub(crate) fn lookup_var_read_at<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cursor_off: u32,
    name: &str,
) -> Option<&'a tcl_compiler::analyser::VarDef> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let var_def = lookup_var_in_scope_chain(analysis, source, cursor_off, name)?;
    (!offset_is_inert(source, analysis, cursor_off)?).then_some(var_def)
}

/// Original comment/data source classification shared by every variable
/// cursor and lexical caller-reference gate. Unknown ownership stays unknown.
#[must_use]
pub(crate) fn offset_is_inert(
    source: &str,
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<bool> {
    if crate::inert_text::offset_in_physical_comment_in_analysis(source, analysis, cursor_off)? {
        Some(true)
    } else {
        crate::inert_text::offset_in_data_brace_in_analysis(source, analysis, cursor_off)
    }
}

/// Shared lexical variable-cursor entry. The actual analysis, selected
/// grammar, complete original words and comment/data applicability issue the
/// syntax label. Scope resolution remains a separate Logical-only operation;
/// unknown source applicability declines rather than borrowing nominal roles.
#[must_use]
pub(crate) fn substituting_var_at_position(
    source: &str,
    analysis: &AnalysisResult,
    _line: u32,
    _character: u32,
    cursor_off: u32,
) -> Option<String> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let reference =
        crate::source_structure::original_variable_reference_at(source, analysis, cursor_off)?;
    Some(reference.root.to_owned())
}

/// Whether `off` sits inside a **literal** parameter-list word of a proc or
/// method — the word between its name token and its body.
///
/// Every word inside a literal parameter list is pure data: a parameter's own
/// name, or a default value.  Neither is a command reference, so bareword
/// command resolution must not run on it: `proc ::tk::RestoreFocusGrab {grab
/// focus {destroy destroy}}` would otherwise have both the parameter name
/// `destroy` and the default-value literal `destroy` resolve to Tk's `destroy`
/// **command**, where tclsh proves the bare word in a parameter list never
/// invokes anything.  A cursor on the parameter
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
    // The parameter-list word divides under the document's own grammar, the
    // same one every other provider in this module reads off the analysis.
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return ParamListPosition::Outside;
    };
    let config = current.config();
    let classify = |name_span: tcl_lexer::Span, body_span: tcl_lexer::Span| {
        // A token content span can omit its real name-word closing delimiter.
        let name = tcl_lexer::word_span_at(source, name_span);
        param_word_position(source, name.end(), body_span.start(), off, config)
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
    config: tcl_lexer::LexerConfig,
) -> ParamListPosition {
    let (start, end, off) = (region_start as usize, region_end as usize, off as usize);
    if start > end || off < start || off >= end || end > source.len() {
        return ParamListPosition::Outside;
    }
    let image = tcl_lexer::SourceImage::document(source);
    // Lex the original tail, not a truncated prefix ending at the body opener.
    // The first grouped word is the candidate; no closer or word is invented.
    let Some(source_end) = u32::try_from(source.len()).ok() else {
        return ParamListPosition::Outside;
    };
    let Ok(tokens) = tcl_lexer::source_region_tokens_in(
        &image,
        tcl_lexer::Span::new(region_start, source_end),
        config,
    ) else {
        return ParamListPosition::Outside;
    };
    let commands = tcl_lexer::group_commands_bytes(&tokens, image.bytes(), config);
    let Some(word) = commands.first().and_then(|command| command.words.first()) else {
        return ParamListPosition::Outside;
    };
    let Ok(span) =
        tcl_lexer::NativeWord::written_span_from_group(image.bytes(), config, &tokens, word)
    else {
        return ParamListPosition::Outside;
    };
    if span.start() < region_start
        || span.end() > region_end
        || off < span.start() as usize
        || off >= span.end() as usize
    {
        return ParamListPosition::Outside;
    }
    let Some(fragments) = tokens.get(word.tokens.clone()) else {
        return ParamListPosition::Outside;
    };
    let literal = !word.expand
        && fragments.len() == 1
        && fragments.first().is_some_and(|token| {
            tcl_compiler::signature_scan::params::param_word_is_literal(token.kind, true)
        });
    if literal {
        ParamListPosition::LiteralData
    } else {
        ParamListPosition::Computed
    }
}

/// Source declaration candidates from the shared Logical scope owner.
pub(crate) fn visible_variable_names(
    advice: &tcl_compiler::analyser::scope::LogicalVariableScopeAdvice<'_>,
) -> Vec<String> {
    let mut names = Vec::new();
    for scope in advice.eligible_scopes().rev() {
        for name in scope.variables.keys() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    if advice.root_names_require_qualification()
        || (advice.is_namespace_frame() && advice.namespace_global_fallback())
    {
        for name in advice.root().variables.keys() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    names
}

/// Independently declared root candidates requiring rooted completion insertion.
pub(crate) fn global_vars_needing_qualification(
    advice: &tcl_compiler::analyser::scope::LogicalVariableScopeAdvice<'_>,
) -> Option<FxHashSet<String>> {
    if !advice.root_names_require_qualification() {
        return None;
    }
    let mut globals = advice
        .root()
        .variables
        .keys()
        .cloned()
        .collect::<FxHashSet<_>>();
    globals.extend(
        advice
            .readonly_root_variable_names()
            .into_iter()
            .map(str::to_owned),
    );
    for name in advice.readonly_variable_names() {
        globals.remove(name);
    }
    for scope in advice.eligible_scopes() {
        if std::ptr::eq(scope, advice.root()) {
            continue;
        }
        for name in scope.variables.keys() {
            globals.remove(name);
        }
    }
    Some(globals)
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
/// `namespace eval` (however deeply nested — a non-accumulating "just the
/// innermost segment" reading misidentifies a 2+-level nested namespace, e.g.
/// `::a::b` read back as just `b`) and resets to a
/// proc's/method's own defining namespace inside its body, even when that
/// proc was declared with a fully-qualified name with no enclosing
/// `namespace eval` at all.
///
/// `overrides` is checked *first*: a runtime-context
/// namespace pin (`apply {{params} body ns}` runs `body` in `ns`, not its
/// lexical home) that the ordinary span-containment walk above can never
/// see, since `handle_apply_command` roots that `Scope` subtree under
/// fresh, `body_span`-less namespace wrapper nodes. On multiple containing
/// entries (nested `apply` calls), the smallest span wins — innermost,
/// mirroring the lexical walk's own innermost-scope preference. Pass `&[]`
/// to opt out (e.g. variable lookups, which the override deliberately does
/// not cover — see `lookup_var_in_scope_chain`).
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
        return tcl_syntax::naming::unroot_rooted_key(ns)
            .unwrap_or(ns)
            .to_owned();
    }
    let namespace = tcl_compiler::analyser::command_resolution_namespace_at(scope, byte_offset);
    tcl_syntax::naming::unroot_rooted_key(&namespace)
        .unwrap_or(&namespace)
        .to_owned()
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
    tcl_syntax::naming::root_unrooted_key(&ns)
}

/// Original C loader advice at one complete current command-head producer.
/// This retains caller geometry without proving loading or a callable target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalAutoloadRequest {
    name: tcl_compiler::signature_scan::original_name::SignatureSourceNameKey,
    invocation: tcl_compiler::command_binding::SourceInvocationBinding,
}

impl OriginalAutoloadRequest {
    /// Files selected by genuine `auto_index` entries and the retained caller scope.
    #[must_use]
    pub fn files(
        &self,
        resolver: &crate::package_resolver::PackageResolver,
    ) -> Vec<std::path::PathBuf> {
        resolver.resolve_original_auto_command(&self.name, &self.invocation)
    }
}

/// Select original autoload advice only at the retained complete invocation head.
#[must_use]
pub fn original_autoload_request_at(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<OriginalAutoloadRequest> {
    use tcl_compiler::signature_scan::scope::SignatureSourceNameInput;
    let input = analysis.resolved_input.as_ref()?;
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    let realm = analysis.retained_command_realm()?;
    if !analysis.matches_original_source_image(&image, config)
        || !realm.matches_resolved_analysis_input(input)
    {
        return None;
    }
    let offset = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    let mut sites = analysis.command_invocations.iter().filter(|site| {
        site.lookup == tcl_compiler::signature_scan::types::SignatureCommandLookup::InvocationHead
            && site.range.start() <= offset
            && offset < site.range.end()
    });
    let site = sites.next()?;
    if sites.next().is_some() {
        return None;
    }
    let invocation = realm.invocation_at_source("", site.range.start());
    let name_input = invocation.original_recorded_head_name_input()?;
    if site.original_name_input.as_ref() != Some(&name_input)
        || !crate::original_name_edit::original_input_matches_source(
            source,
            analysis,
            &name_input,
            site.range,
        )
    {
        return None;
    }
    let SignatureSourceNameInput::OriginalWord(name) = name_input else {
        return None;
    };
    invocation.original_autoload_command_candidates(&name)?;
    Some(OriginalAutoloadRequest { name, invocation })
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
    let config = analysis.body_lexer_config?;
    if analysis.allows_retained_logical_declaration_advice() {
        let image = tcl_lexer::SourceImage::document(source);
        if !analysis.matches_original_source_image(&image, config) {
            return None;
        }
        let offset = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
        let mut candidates =
            tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates(
                source, analysis,
            )?
            .into_iter()
            .filter(|candidate| {
                candidate
                    .original_head()
                    .is_some_and(|head| head.span().start() <= offset && offset < head.span().end())
            });
        let candidate = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        let head = candidate.original_head()?;
        let word = String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
            head,
        )?)
        .ok()?;
        let namespace = candidate
            .declared_source_context()
            .map_or_else(tcl_core_types::ByteNamespacePath::root, |context| {
                context.namespace_recipe().clone()
            });
        return Some((
            word,
            tcl_syntax::naming::checked_namespace_path_key(&namespace)?,
        ));
    }
    if !analysis.allows_retained_logical_declaration_advice()
        || !analysis
            .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
    {
        return None;
    }
    let (word, _start, _end) = crate::hover::find_word_span_at_position(source, line, character)?;
    let offset = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
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

/// Family metadata from the complete input and command store retained by the
/// analysis. Missing metadata is distinct from a known non-Itcl family; it does
/// not establish native class identity, package loading or edit permission.
pub(crate) fn class_definer_family(
    analysis: &AnalysisResult,
    class: &tcl_compiler::analyser::types::ClassDef,
) -> Option<tcl_registry::definer::DefinerFamily> {
    definer_family_in_registry(class, class_family_registry(analysis)?)
}

fn class_family_registry(analysis: &AnalysisResult) -> Option<&CommandRegistry> {
    if analysis.analysis_context_unavailable.is_some() {
        return None;
    }
    let input = analysis.resolved_input.as_ref()?;
    if !analysis
        .retained_command_realm()?
        .matches_resolved_analysis_input(input)
    {
        return None;
    }
    analysis.resolved_registry()
}

/// Family metadata in an explicitly supplied store. This does not establish a
/// native class, current command table, loader effect or editable identity.
fn definer_family_in_registry(
    class: &tcl_compiler::analyser::types::ClassDef,
    registry: &CommandRegistry,
) -> Option<tcl_registry::definer::DefinerFamily> {
    registry
        .get(&class.metaclass)?
        .definition_body
        .map(|grammar| grammar.family)
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
    qname: &str,
) -> Option<(&'a String, String)> {
    itcl_class_proc_at_in_registry(analysis, class_family_registry(analysis)?, qname)
}

/// Standalone constructed metadata can supply its store explicitly. Contextual
/// callers use their analysis-owned Registry and never recapture a profile.
fn itcl_class_proc_at_in_registry<'a>(
    analysis: &'a AnalysisResult,
    registry: &CommandRegistry,
    qname: &str,
) -> Option<(&'a String, String)> {
    let (parent, member) = tcl_syntax::naming::key_holder_and_tail(qname);
    if member.is_empty() {
        return None;
    }
    let (class_q, class_def) = analysis.all_classes.get_key_value(parent)?;
    (definer_family_in_registry(class_def, registry)
        == Some(tcl_registry::definer::DefinerFamily::Itcl)
        && class_def.class_methods.contains_key(member))
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
    namespace: &str,
    word: &str,
) -> Option<(&'a String, String)> {
    let winner = itcl_self_qualified_candidate(analysis, namespace, word)
        // The custom resolver only claims a member the class really declares;
        // anything else falls through to the stock candidates (oracle: a
        // sibling class's `Other::omake`, and an undeclared `Factory::nope`,
        // both raise `invalid command name` rather than resolving).
        .filter(|candidate| itcl_class_proc_at(analysis, candidate).is_some())
        .or_else(|| {
            resolved_command_name(analysis, namespace, word, &|candidate| {
                analysis.all_procs.contains_key(candidate)
                    || analysis.all_classes.contains_key(candidate)
                    || itcl_class_proc_at(analysis, candidate).is_some()
            })
        })?;
    itcl_class_proc_at(analysis, &winner)
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
    namespace: &str,
    word: &str,
) -> Option<String> {
    let class_def = analysis.all_classes.get(namespace)?;
    if class_definer_family(analysis, class_def) != Some(tcl_registry::definer::DefinerFamily::Itcl)
    {
        return None;
    }
    let member =
        tcl_syntax::naming::unroot_rooted_key(word.strip_prefix(class_def.name.as_str())?)?;
    (!member.is_empty() && !tcl_syntax::naming::is_qualified(member.as_bytes()))
        .then(|| tcl_syntax::naming::qualify(&class_def.qualified_name, member))
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
    source: &str,
    analysis: &AnalysisResult,
    resolution: CallResolution<'_>,
    word: &str,
    cursor_offset: u32,
) -> Option<tcl_lexer::Span> {
    if !analysis.allows_lexical_declaration_advice() {
        let std::ops::ControlFlow::Break(Some(identity)) =
            crate::original_declaration::select_at_offset("", source, analysis, cursor_offset)
        else {
            return None;
        };
        if identity.role() != crate::original_declaration::OriginalDeclarationRole::Class {
            return None;
        }
        return identity
            .class_metadata(analysis)
            .map(|declaration| declaration.metadata().name_span);
    }
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let (_, class) = resolve_class_target_at(analysis, source, resolution, cursor_offset, word)?;
    (class.name_span.start() <= cursor_offset && cursor_offset < class.name_span.end()
        || !class.via_define)
        .then_some(class.name_span)
}

/// The declaration name span of the [incr Tcl] class-scoped `proc` that the
/// written word `word`, called from `namespace`, dispatches — go-to-definition's
/// half of [`itcl_class_proc_target`].
///
/// Family metadata comes from the analysis-owned command store, independently
/// of the written member prefix and the original declaration-name span.
fn itcl_class_proc_declaration(
    analysis: &AnalysisResult,
    namespace: &str,
    word: &str,
) -> Option<tcl_lexer::Span> {
    let (class_q, member) = itcl_class_proc_target(analysis, namespace, word)?;
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
    _dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<(String, String)> {
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let (word, _start, _end) = crate::hover::find_word_span_at_position(source, line, character)?;
    let cursor = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    let namespace = namespace_context_at(
        &analysis.global_scope,
        cursor,
        &analysis.namespace_overrides,
    );
    itcl_class_proc_target(analysis, &namespace, &word)
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
/// question covers, and
/// [`crate::namespace_import::exported_at_import_site`] for the export
/// snapshot each import site is judged by.
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
    if tcl_syntax::naming::is_qualified(word.as_bytes()) {
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
    if tcl_syntax::naming::is_qualified(word.as_bytes()) {
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

/// Whether the original-name providers abstain at the call whose head covers
/// `offset` (user decision 2026-10-22T07:18Z, `force_import_shadow_direction`).
///
/// A `namespace import -force` that the document alone shows binding nothing
/// leaves the fresh-interpreter answer (the local command) in place. When
/// only the caller's whole-program export view says a sibling exports the
/// name, the shadow depends on cross-document execution order, which stays
/// conditional (decision 2026-10-17T20:30Z `export_order`): neither the
/// local definition nor the sibling's export is proven, so no provider
/// answers for that call.
#[must_use]
pub fn program_forced_import_abstains(
    analysis: &AnalysisResult,
    program: Option<ProgramExports<'_>>,
    offset: u32,
) -> bool {
    let Some(program) = program else {
        return false;
    };
    let Some(invocation) = invocation_head_at(analysis, offset) else {
        return false;
    };
    let word = invocation.name.as_str();
    let call_off = invocation.range.start();
    let namespace = namespace_context_at(
        &analysis.global_scope,
        call_off,
        &analysis.namespace_overrides,
    );
    forced_import_shadows(
        analysis,
        CallResolution::document_only().in_program(program),
        &namespace,
        word,
        call_off,
    ) && !forced_import_shadows(
        analysis,
        CallResolution::document_only(),
        &namespace,
        word,
        call_off,
    )
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
/// # Why an oracle at all
///
/// The document-only fallback is the whole problem. It is the in-document twin of
/// `WorkspaceIndex::observable_namespaces`, and cross-document that framing is
/// right — the index really does hold every file. In *one* document it is not:
/// a namespace can be partly visible here (its procs, and some of its exports)
/// while the one `namespace export` that covers this name sits in another
/// file. The false-positive and the true-negative then have **byte-identical**
/// single-document inputs and opposite correct answers (transcript on
/// [`NamespaceExportOracle`]), so no rule reading only this document can
/// separate them — hence the oracle, and hence its optionality: a caller with
/// no workspace keeps the document-only fallback.
fn export_verdict(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    source_ns: &str,
    word: &str,
    import_at: u32,
    order: &DocumentOrderCell,
) -> ExportVerdict {
    let Some(name) = original_export_command(analysis, source_ns, word) else {
        return ExportVerdict::Unknown;
    };
    export_verdict_for_slot_in(analysis, ctx, name.slot(), name.policy(), import_at, order)
}

#[cfg(test)]
fn export_verdict_for_slot(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    slot: &tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    import_at: u32,
) -> ExportVerdict {
    export_verdict_for_slot_in(
        analysis,
        ctx,
        slot,
        policy,
        import_at,
        &DocumentOrderCell::new(),
    )
}

fn export_verdict_for_slot_in(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    slot: &tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    import_at: u32,
    order: &DocumentOrderCell,
) -> ExportVerdict {
    let local = exported_original_slot_at_import_in(analysis, ctx, slot, policy, import_at, order);
    if local == ExportVerdict::Exported {
        return local;
    }
    if let Some(program) = ctx.program {
        return program.oracle.exported_at_original(
            slot,
            policy,
            in_document_point(analysis, ctx, import_at),
        );
    }
    let scope =
        tcl_compiler::signature_scan::scope::SignatureNamespaceScope::C(slot.namespace.clone());
    let observable = namespace_exports_observable(analysis, &scope, policy);
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        eprintln!(
            "ORIGINAL_EXPORT_SOURCE_QUERY at={import_at} scope={scope:?} tail={:?} local={local:?} observable={observable} exports={} unknown={}",
            slot.simple.as_bytes(),
            analysis.original_namespace_exports().count(),
            analysis.original_namespace_export_unknowns().len()
        );
    }
    if observable {
        local
    } else {
        ExportVerdict::Unknown
    }
}

/// Whether this document holds *any* `namespace export` record for
/// the exact namespace scope and policy — the document-only fallback
/// [`export_verdict`] uses when no
/// whole-program oracle is attached.
///
/// The proposition being asserted is about the export list, so the evidence
/// has to be the export list: a document that declares no export at all for
/// that namespace knows nothing about what it exports. Its **residual** — a
/// document holding some of the namespace's exports but not the covering one
/// still reads the gap as a fact — is precisely what the oracle removes when
/// one is available, and what remains, deliberately, when one is not.
fn namespace_exports_observable(
    analysis: &AnalysisResult,
    scope: &tcl_compiler::signature_scan::scope::SignatureNamespaceScope,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> bool {
    analysis
        .original_namespace_exports()
        .any(|event| event.context() == scope && event.policy() == policy)
}

/// Associate an advisory qualified label with already retained original source
/// publication records. A label cannot create bytes; disagreeing publications
/// and missing original records are terminal.
fn original_export_command<'a>(
    analysis: &'a AnalysisResult,
    source_ns: &str,
    word: &str,
) -> Option<&'a tcl_compiler::signature_scan::scope::SignatureSourceCommand> {
    let label = tcl_syntax::naming::qualify(source_ns, word);
    let names = analysis
        .original_procedure_declarations()
        .filter(|record| record.metadata().qualified_name == label)
        .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name)
        .chain(
            analysis
                .original_class_declarations()
                .filter(|record| record.metadata().qualified_name == label)
                .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name),
        );
    let mut found = None;
    for name in names {
        if found.is_some_and(
            |previous: &tcl_compiler::signature_scan::scope::SignatureSourceCommand| {
                previous.slot() != name.slot() || previous.policy() != name.policy()
            },
        ) {
            return None;
        }
        found = Some(name);
    }
    found
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
/// tails; a qualified call
/// (`inner::p`) never goes through an imported alias in real Tcl, so this
/// abstains rather than risk mismatching a namespace-path segment against
/// an unrelated import.
///
/// # Import chains
///
/// An import edge may land on a name that is *itself* imported: with `::C`
/// exporting `p`, `::B` importing `::C::*` and re-exporting, and `::A`
/// importing `::B::*`, `::A::p` runs `::C`'s body and `namespace origin
/// ::A::p` answers `::C::p` (oracle tclsh 8.6.14 / 9.0.4).
/// Statically the middle hop is in no `all_procs`, so a single-hop walk
/// finds nothing. This one follows the chain while every hop is provable,
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
    if tcl_syntax::naming::is_qualified(word.as_bytes()) {
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
    let candidates = tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
        namespace, path, word,
    );
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
        let (prefix, tail) = tcl_syntax::naming::key_holder_and_tail(candidate);
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
    forget_source.is_none_or(|src| same_namespace(src, source_ns))
}

/// The source namespace of the import alias `importing_ns` holds for `word`
/// **at `call_off`**, or `None` when it holds none — the same-document
/// binding of the shared lifecycle decision
/// [`crate::namespace_import::alias_live_at`].
///
/// An import edge is a link with a lifecycle, not a standing name-visibility
/// fact. Everything that writes the `<importing_ns>::<word>` slot is an event
/// on one ordered log ([`SlotEvent`]); all of it is oracle-confirmed
/// byte-identically on tclsh 9.0.4 and 8.6.14:
///
/// 1. **The import installs it** — gated by the source namespace's export
///    snapshot at the import's own position
///    ([`exported_original_slot_at_import`]).
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
/// It gates the *installs* as well as the removals: a
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
    let order = DocumentOrderCell::new();
    let events = slot_events(analysis, ctx, importing_ns, word, query, call_off, &order);
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
                // the source namespace. An import the
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
        document_order(&order, analysis, ctx),
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
///
/// The order passed with those points is **empty**: one document needs no
/// `source` graph — every point shares this key, so
/// [`crate::source_graph::RunOrder`] answers from the single-document rule
/// alone and never looks at an edge. Building it allocates nothing.
///
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
        enclosing_body: analysis.original_definition_body_span(at).or_else(|| {
            analysis
                .allows_lexical_declaration_advice()
                .then(|| analysis.innermost_definition_body_span(at))
                .flatten()
        }),
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
/// Folding that log in raw offset order gets *both* halves wrong: it lets the
/// body-local `::B` import install, and then conflicts the top-level `::A`
/// one away. Within one tier — all top level, or all inside one body — the
/// key degenerates to a plain offset comparison, so genuinely sequenced
/// statements are unaffected. Two events in *different* bodies have no static
/// order either way; they keep their offset order.
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
    call_off: u32,
    order: &DocumentOrderCell,
) -> Vec<SlotEvent<'a>> {
    let mut events: Vec<SlotEvent<'a>> = Vec::new();
    #[cfg(debug_assertions)]
    trace_slot_query(analysis, importing_ns, word, call_off);
    for imp in analysis
        .namespace_imports
        .iter()
        .filter(|i| i.ns == importing_ns)
    {
        if query == ImportQuery::ForcedShadow && !imp.forced {
            continue;
        }
        let Some(source) = &imp.source else {
            continue;
        };
        let source_ns = source.namespace.as_str();
        let export_tail = source.tail_pattern.as_str();
        // A pattern rooted at the global namespace (`namespace import ::p`,
        // `namespace import ::*`) splits to an *empty* source namespace, which
        // is easy to read as "no source" and skip — the one import shape that
        // would then bypass the export gate entirely. Real Tcl gives it no
        // special treatment: `proc p {}` with no
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
        let at = imp.range.start();
        let original = analysis.original_namespace_patterns().find(|pattern| {
            pattern.span() == imp.range
                && pattern.purpose()
                    == tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern
        });
        let input = slot_call_inputs(analysis, call_off);
        let verdict = if let (Some(original), Some(first)) = (original, input.first()) {
            if input.iter().any(|other| *other != *first) {
                continue;
            }
            let Some(slot) = original.source_slot_for_call_input(first) else {
                continue;
            };
            if original.matches_imported_command(&slot, first.policy()) != Some(true) {
                continue;
            }
            export_verdict_for_slot_in(analysis, ctx, &slot, first.policy(), at, order)
        } else if let Some(original) = original {
            // Explicit authored query labels associate existing original
            // publication tails; they cannot produce native name bytes.
            let names = slot_named_publications(analysis, word);
            let Some(first) = names.first() else {
                continue;
            };
            if names.iter().any(|other| {
                other.slot().simple != first.slot().simple || other.policy() != first.policy()
            }) {
                continue;
            }
            let Some(slot) = original.source_slot_for_command_slot(first.slot(), first.policy())
            else {
                continue;
            };
            if original.matches_imported_command(&slot, first.policy()) != Some(true) {
                continue;
            }
            export_verdict_for_slot_in(analysis, ctx, &slot, first.policy(), at, order)
        } else {
            if !analysis.allows_lexical_declaration_advice()
                || !tcl_syntax::glob::string_match(export_tail, word)
            {
                continue;
            }
            export_verdict(analysis, ctx, source_ns, word, at, order)
        };
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_IMPORT_SOURCE_PATTERN ns={importing_ns} source={source_ns} at={at} original={} call_inputs={} verdict={verdict:?}",
                original.is_some(),
                input.len()
            );
        }
        if !query.installs(verdict) {
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
    append_slot_lifecycle_events(analysis, importing_ns, word, &mut events);
    events
}

#[cfg(debug_assertions)]
fn trace_slot_query(analysis: &AnalysisResult, importing_ns: &str, word: &str, call_off: u32) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        eprintln!(
            "ORIGINAL_IMPORT_SOURCE_QUERY ns={importing_ns} word={word} at={call_off} imports={} patterns={} exports={} unknown={}",
            analysis
                .namespace_imports
                .iter()
                .filter(|imp| imp.ns == importing_ns)
                .count(),
            analysis.original_namespace_patterns().count(),
            analysis.original_namespace_exports().count(),
            analysis.original_namespace_export_unknowns().len()
        );
    }
}

fn slot_call_inputs(
    analysis: &AnalysisResult,
    call_off: u32,
) -> Vec<&tcl_compiler::signature_scan::scope::SignatureSourceNameInput> {
    analysis
        .command_invocations
        .iter()
        .filter(|invocation| {
            invocation.range.start() == call_off
                || invocation
                    .original_lookup
                    .as_ref()
                    .is_some_and(|lookup| lookup.site().offset == call_off)
        })
        .filter_map(|invocation| {
            invocation
                .original_lookup
                .as_ref()
                .map(tcl_compiler::command_binding::OriginalCommandLookup::name_input)
        })
        .collect::<Vec<_>>()
}

fn slot_named_publications<'a>(
    analysis: &'a AnalysisResult,
    word: &str,
) -> Vec<&'a tcl_compiler::signature_scan::scope::SignatureSourceCommand> {
    analysis
        .original_procedure_declarations()
        .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name)
        .chain(
            analysis
                .original_class_declarations()
                .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name),
        )
        .filter(|name| name.slot().simple.try_utf8().ok() == Some(word))
        .collect::<Vec<_>>()
}

fn append_slot_lifecycle_events<'a>(
    analysis: &'a AnalysisResult,
    importing_ns: &str,
    word: &str,
    events: &mut Vec<SlotEvent<'a>>,
) {
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
}

/// One lazily captured in-document source order per resolution, shared by
/// every import site's export snapshot and the alias lifecycle fold.
type DocumentOrderCell = std::cell::OnceCell<RunOrder>;

fn document_order<'o>(
    order: &'o DocumentOrderCell,
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
) -> &'o RunOrder {
    order.get_or_init(|| in_document_source_order(analysis, ctx))
}

/// Retain the actual analysis owner even without an attached workspace view.
/// Missing source transport stays conditional/unavailable, never constructed.
/// Straight-line positions of this document are ordered by its own sequence;
/// deferred bodies, conditional branches and possible early exits stay
/// Conditional.
fn in_document_source_order(analysis: &AnalysisResult, ctx: CallResolution<'_>) -> RunOrder {
    RunOrder::for_source_navigation(
        &[],
        crate::source_graph::SourceOrderContext::capture(analysis)
            .map(|owner| (ctx.uri().to_owned(), owner))
            .into_iter(),
    )
}

#[cfg(test)]
fn exported_original_slot_at_import(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    slot: &tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    import_at: u32,
) -> ExportVerdict {
    exported_original_slot_at_import_in(
        analysis,
        ctx,
        slot,
        policy,
        import_at,
        &DocumentOrderCell::new(),
    )
}

fn exported_original_slot_at_import_in(
    analysis: &AnalysisResult,
    ctx: CallResolution<'_>,
    slot: &tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    import_at: u32,
    order: &DocumentOrderCell,
) -> ExportVerdict {
    let site = in_document_point(analysis, ctx, import_at);
    let order = document_order(order, analysis, ctx);
    if analysis
        .original_namespace_export_unknowns()
        .iter()
        .any(|span| {
            order
                .navigation_precedence(in_document_point(analysis, ctx, span.start()), site)
                .and_then(crate::source_graph::SourceNavigationOrder::modeled_source_exclusion)
                .unwrap_or(true)
        })
    {
        return ExportVerdict::Unknown;
    }
    crate::namespace_import::exported_original_at_import_site(
        analysis.original_namespace_exports().map(|event| {
            (
                event,
                in_document_point(analysis, ctx, event.span().start()),
            )
        }),
        slot,
        policy,
        order,
        site,
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
fn same_namespace(a: &str, b: &str) -> bool {
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

/// Test-only display coordinate for isolated scope-walk geometry controls.
/// Production insertion uses current Logical source namespace advice.
#[cfg(test)]
fn qualified_var_name(scope: &tcl_compiler::analyser::Scope, var: &str) -> String {
    use tcl_compiler::analyser::ScopeKind;
    if tcl_syntax::naming::unroot_rooted_key(var).is_some() {
        var.to_string()
    } else if scope.kind == ScopeKind::Global {
        tcl_syntax::naming::root_unrooted_key(var)
    } else {
        // Join constructed Logical display coordinates without reparsing
        // literal colon components as a written name.
        format!("{}::{var}", scope.name)
    }
}

/// Walk the scope tree and return the fully-qualified names of every
/// namespace / global variable whose enclosing namespace is not in the
/// cursor's lexical `chain` (proc locals excluded).
#[cfg(test)]
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

/// Current readonly scan geometry. Document visibility is explicit, rather
/// than inferred from an empty or unavailable collection of body spans.
pub(crate) enum SourceScopeBodySpans {
    Document(tcl_lexer::Span),
    Bodies(Vec<tcl_lexer::Span>),
}

/// Join the actual Logical scope owner to genuine original script interiors.
/// Missing/foreign body geometry withdraws the scan, never widens it to a file.
pub(crate) fn scope_body_spans_at(
    position: &CurrentSourcePosition<'_>,
) -> Option<SourceScopeBodySpans> {
    use tcl_compiler::analyser::ScopeKind;
    let source = position.source();
    let analysis = position.analysis();
    let offset = position.offset();
    let image = tcl_lexer::SourceImage::document(source);
    let advice =
        analysis.logical_variable_scope_advice(&image, position.context().config(), offset)?;
    let root = advice.root();
    if !std::ptr::eq(root, &raw const analysis.global_scope)
        || root.kind != ScopeKind::Global
        || root.body_span.is_some()
    {
        return None;
    }
    let structure = crate::source_structure::SourceStructure::capture(
        source,
        Some(analysis),
        position.context().config(),
    )?;
    let mut pending = root
        .children
        .iter()
        .map(|scope| (scope, 1))
        .collect::<Vec<_>>();
    while let Some((scope, depth)) = pending.pop() {
        if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
            return None;
        }
        let body = scope.body_span?;
        source.get(body.as_range())?;
        if !structure
            .scripts
            .iter()
            .any(|(span, nesting)| *nesting > 0 && *span == body)
        {
            return None;
        }
        pending.extend(scope.children.iter().map(|child| (child, depth + 1)));
    }
    let mut selected = root;
    let mut spans = Vec::new();
    loop {
        let mut children = selected
            .children
            .iter()
            .filter_map(|child| {
                let body = child.body_span?;
                (body.start() <= offset && offset < body.end()).then_some((child, body))
            })
            .collect::<Vec<_>>();
        children.sort_by_key(|(_, body)| body.len());
        let Some((child, body)) = children.first().copied() else {
            break;
        };
        if children
            .get(1)
            .is_some_and(|(_, other)| other.len() == body.len())
        {
            return None;
        }
        spans.push(body);
        selected = child;
    }
    if let Some(body) = structure
        .caller_context_unavailable_bodies
        .iter()
        .filter(|body| body.start() <= offset && offset < body.end())
        .min_by_key(|body| body.len())
        && !spans.contains(body)
    {
        return None;
    }
    if spans.is_empty() {
        Some(SourceScopeBodySpans::Document(tcl_lexer::Span::new(
            0,
            u32::try_from(source.len()).ok()?,
        )))
    } else {
        spans.reverse();
        Some(SourceScopeBodySpans::Bodies(spans))
    }
}

/// Convert a byte extent to editor UTF-16 geometry in its owning source.
/// This supplies no name, declaration, source-currency or edit authority.
#[must_use]
pub fn span_to_range(source: &str, line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
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
mod tests;

#[cfg(test)]
mod original_namespace_export_advice_tests;

#[cfg(test)]
mod original_next_source_context_tests;

#[cfg(test)]
mod configured_receiver_head_tests;

#[cfg(test)]
mod itcl_family_context_tests;

#[cfg(test)]
mod logical_variable_scope_owner_tests;

#[cfg(test)]
mod logical_class_navigation_tests;

#[cfg(test)]
mod exact_source_position_advice_tests;

#[cfg(test)]
mod original_document_source_order_tests;

#[cfg(test)]
mod configured_parameter_word_extent_tests;
