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

#[cfg(test)]
use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LineIndex, Utf16Col};
#[cfg(test)]
use tcl_registry::CommandRegistry;

#[cfg(test)]
use crate::namespace_import::ExportVerdict;
use crate::namespace_import::NamespaceExportOracle;
use crate::original_context::CurrentSourcePosition;
#[cfg(test)]
use crate::source_graph::RunPoint;

mod call_resolution;
mod command_heads;
mod import_slot_events;
mod itcl_class_procs;
mod method_receivers;
mod namespace_imports;
mod object_dispatch;
mod variables;

#[cfg(test)]
pub(crate) use call_resolution::fallback_proc_by_simple_name;
#[cfg(test)]
use call_resolution::proc_visible_from_namespace;
pub(crate) use call_resolution::{
    resolve_called_proc, resolve_class_target_at, resolve_proc_target_at,
};
#[cfg(test)]
use command_heads::logical_procedure_header_at;
use command_heads::resolved_indirect_head_target;
pub use command_heads::{
    OriginalAutoloadRequest, command_head_and_namespace_at, indirection_pending_at,
    original_autoload_request_at,
};
pub(crate) use command_heads::{
    command_indirection, ensemble_called_procedure, ensemble_subcommand_target, invocation_head_at,
    invocation_reference_at, offset_is_command_head, proc_declaration_at,
    resolved_indirect_head_proc,
};
use import_slot_events::IN_DOCUMENT_URI;
#[cfg(test)]
use import_slot_events::{exported_original_slot_at_import, in_document_source_order};
#[cfg(test)]
use import_slot_events::{in_document_point, live_import_at};
#[cfg(test)]
pub(crate) use itcl_class_procs::itcl_class_proc_at;
#[cfg(test)]
use itcl_class_procs::itcl_class_proc_at_in_registry;
use itcl_class_procs::itcl_class_proc_declaration;
pub use itcl_class_procs::itcl_class_proc_target_at;
#[cfg(test)]
use itcl_class_procs::itcl_self_qualified_candidate;
pub(crate) use itcl_class_procs::{
    class_definer_family, class_definition_grammar, itcl_class_proc_target,
};
#[cfg(test)]
pub(crate) use method_receivers::instance_method_at_cursor;
use method_receivers::next_dispatch_target;
pub(crate) use method_receivers::{
    MethodBucket, canonicalise_class, enclosing_class_at, lattice_singleton_class,
    logical_receiver_candidate_at, original_instance_method_at_cursor,
};
#[cfg(test)]
use namespace_imports::{ImportQuery, proc_visible_via_wildcard_import};
#[cfg(test)]
use namespace_imports::{export_verdict_for_slot, namespace_exports_observable};
pub(crate) use namespace_imports::{forced_import_shadows, import_chain_target};
pub use namespace_imports::{forced_import_shadows_call, program_forced_import_abstains};
pub use object_dispatch::{ExternalObjectDispatchMask, object_dispatch_mask};
pub(crate) use object_dispatch::{
    declared_class_at, declared_self_dispatch_at, object_member_state_at,
};
use object_dispatch::{instance_method_definition, lookup_class_member};
#[cfg(test)]
pub(crate) use variables::cross_namespace_qualified_vars;
#[cfg(test)]
use variables::variable_scope_extent;
pub(crate) use variables::{
    ParamListPosition, SourceScopeBodySpans, global_vars_needing_qualification,
    linked_var_reference_spans, lookup_var_in_scope_chain, lookup_var_read_at,
    offset_is_in_parameter_list, offset_is_inert, parameter_list_position_at, scope_body_spans_at,
    source_frame_extent, substituting_var_at_position, var_def_at_declaration_offset,
    visible_variable_names,
};
pub use variables::{
    original_variable_cursor_retained, original_variable_occurrence_at, qualified_variable_cell_at,
};

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
    // A computed head may have a positioned target without a static spelling.
    let word =
        crate::source_structure::original_cursor_word_value(source, analysis, decl_byte_offset);
    if let Some(word) = word.as_deref()
        && let Some(result) =
            list_built_self_method_definition(source, line_index, analysis, word, decl_byte_offset)
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
    if let Some(word) = word.as_deref()
        && let Some(result) =
            next_dispatch_definition(analysis, source, line_index, line, character, word)
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
    let Some(word) = word else {
        return Vec::new();
    };
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
