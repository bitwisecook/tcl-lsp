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

//! Hover provider.
//!
//! Resolves the word or `$var` reference at a given LSP position
//! and produces a [`Hover`] with markdown-formatted content for
//! one of:
//!
//! * a user-defined `proc` the cursor word resolves to under C
//!   Tcl's command lookup (the cursor's namespace first, then
//!   global; absolute `::`-prefixed words exact — see
//!   `crate::definition::resolve_called_proc`) — formats the
//!   signature plus the harvested doc-comment;
//! * a `TclOO` class whose name matches — formats the
//!   metaclass-qualified declaration plus method / property
//!   summaries;
//! * a `$var` reference whose name resolves through the
//!   enclosing-scope chain to a [`VarDef`] — formats the
//!   reference count.
//!
//! Inferred-interp / taint annotations on `$var` hovers: when a
//! registry is supplied the provider builds a compiler
//! [`tcl_compiler::compilation_unit::CompilationUnit`] and reads
//! the per-variable type / taint lattices off it.
//!
//! Cache + debounce + `spawn_blocking` + `Ok(None)`-on-no-cached-
//! analysis ride on top of this provider in
//! `tcl-lsp-server::Backend::hover`; this module is the pure-CPU
//! computation, no I/O, no async.

use tcl_compiler::analyser::AnalysisResult;
#[cfg(test)]
use tcl_compiler::analyser::VarDef;
use tcl_registry::CommandRegistry;

mod original_registry;

use crate::definition::utf16_col_to_char_col;

mod builtin_commands;
mod class_members;
mod classes;
mod embedded_operands;
mod expression_precedence;
mod format_specifiers;
mod pattern_syntax;
mod procedures;
mod registered_instance;
mod source_documentation;
mod variables;

use builtin_commands::{EventRelease, append_command_event_metadata};
#[cfg(test)]
use builtin_commands::{
    append_valid_events, builtin_command_hover_text, effective_event_requires, valid_events,
};
#[cfg(test)]
use class_members::oo_method_resolution_note;
use class_members::{
    class_member_hover_text, declared_class_hover, method_dispatch_hover,
    original_linked_member_hover_text, source_constructor_hover,
};
pub use class_members::{cross_document_method_hover, declared_member_hover};
use classes::{
    class_hover_text, counted_initial_method_hover, original_property_source_hover,
    positioned_class_hover,
};
#[cfg(test)]
use embedded_operands::original_pattern_format_hover;
pub use embedded_operands::{
    registry_pattern_format_hover, registry_pattern_format_hover_in_workspace,
};
#[cfg(test)]
use format_specifiers::{
    BinaryContext, binary_format_hover_text, clock_format_hover_text, sprintf_format_hover_text,
};
#[cfg(test)]
use format_specifiers::{
    binary_field_bytes, scan_binary_fields, scan_clock_specifiers, scan_sprintf_specifiers,
};
#[cfg(test)]
use pattern_syntax::{glob_hover_text, regex_hover_text, regsub_hover_text, scan_regsub_backrefs};
use procedures::{
    alias_hover_at, ensemble_subcommand_hover, format_docstring, lenient_declared_procedure_hover,
    proc_hover_at, proc_hover_text,
};
use source_documentation::conditional_native_formal_hover;
pub use source_documentation::source_documentation_hover;
pub(crate) use source_documentation::{
    original_source_procedure_hover, source_documentation_region_owns_cursor,
};
#[cfg(test)]
use variables::var_hover_text;
use variables::{PositionHover, navigation_variable_hover, variable_position_hover};

/// LSP markup-content kind for a hover body.
///
/// We only emit Markdown today; the variant exists so the lift in
/// `tcl-lsp-server` is exhaustive when we add `PlainText` support
/// later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverKind {
    /// GitHub-flavoured Markdown, suitable for VS Code rendering.
    Markdown,
}

/// A single hover result — markdown-formatted body.
///
/// The subset this provider emits today carries no `range` and no
/// `PlainText`.  The lift in `tcl-lsp-server` materialises this
/// onto `ls_types::Hover`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    /// Markdown body of the hover.
    pub value: String,
    /// Markup kind. Always `Markdown` for the minimal port.
    pub kind: HoverKind,
}

impl Hover {
    fn markdown(value: String) -> Self {
        Self {
            value,
            kind: HoverKind::Markdown,
        }
    }
}

/// Word-delimiter set used by `find_word_span_at_position`.
const WORD_DELIMS: &[char] = &[' ', '\t', '\n', ';', '{', '}', '[', ']', '"', '$'];

/// Cheap local shape gate for the conditional foreign factory source query.
/// It grants no command, class or member identity.
#[must_use]
pub fn may_have_foreign_factory_hover(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> bool {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    if tcl_compiler::source_graph::current_analysis(source, analysis).is_none() {
        return false;
    }
    let index = tcl_lexer::LineIndex::new(source);
    let offset = crate::definition::byte_offset_at(&index, source, line, character);
    tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        input.lexer_config(),
    )
    .iter()
    .any(|command| {
        !command.is_partial
            && command
                .argv
                .get(2)
                .is_some_and(|word| word.span.start() <= offset && offset < command.span.end())
    })
}

/// Present an original conditional foreign class or member occurrence.
/// The host must rejoin the complete current source census and owners before
/// publishing this source-only answer; it does not identify runtime dispatch.
#[must_use]
pub fn conditional_foreign_factory_hover(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    caller_document: u64,
    headers: &[(
        u64,
        tcl_compiler::registry_invocation::source_structure::OriginalProjectFactoryHeaders,
    )],
) -> Option<Hover> {
    let index = tcl_lexer::LineIndex::new(source);
    let offset = crate::definition::byte_offset_at(&index, source, line, character);
    let mut results = Vec::new();
    for body in tcl_compiler::registry_invocation::source_structure::original_foreign_factory_bodies(
        source,
        analysis,
        caller_document,
        headers,
    ) {
        let class = body.class_syntax()?;
        let name = std::str::from_utf8(class.name_input().bytes()).ok()?;
        let name_span = class.name().span();
        if name_span.start() <= offset && offset < name_span.end() {
            results.push(Hover::markdown(format!(
                "`{name}`\n\nConditional source class."
            )));
        }
        for member in class.callable_syntax()? {
            let span = member.name_word().span();
            if span.start() <= offset && offset < span.end() {
                results.push(Hover::markdown(format!(
                    "`{}`\n\nConditional source method of `{name}`.",
                    member.name()
                )));
            }
        }
    }
    let [only] = results.as_slice() else {
        return None;
    };
    Some(only.clone())
}

/// Compute hover text for a position in `source`.
///
/// `analysis` is the pre-computed analyser result; the caller is
/// expected to cache it. Returns `None` when:
///
/// * `line` / `character` falls outside the source extents,
/// * the cursor isn't on any recognisable identifier or `$var`,
/// * no proc / class / var matches the resolved word.
///
/// Positions use LSP UTF-16 code units. Shared position conversion selects
/// the corresponding character or byte offset before source lookup, including
/// supplementary-plane characters.
#[must_use]
pub fn hover(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> Option<Hover> {
    // Source-backed queries select the retained document owner; caller metadata
    // remains a compatibility parameter and does not replace it.
    hover_with_profile(
        source,
        line,
        character,
        analysis,
        registry,
        crate::profile_for_analysis(analysis),
    )
}

/// [`hover_with_profile`] with the caller's whole-program export view
/// attached — the entry point a host with a workspace index should call.
///
/// `program` is `None` for a host without one, which reproduces
/// [`hover_with_profile`] exactly. It matters because a `namespace import
/// -force` whose covering `namespace export` lives in another file changes
/// *which proc* the hovered call reaches.
#[must_use]
pub fn hover_in_program(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    _profile: &'static tcl_dialect::DialectProfile,
    program: Option<crate::definition::ProgramExports<'_>>,
) -> Option<Hover> {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let position = crate::original_context::CurrentSourcePosition::capture_variable(
        source, analysis, line, character,
    )?;
    let (line, character) = position.line_character();
    let current = position.context();
    let offset = position.offset();
    if crate::definition::program_forced_import_abstains(analysis, program, offset) {
        return None;
    }
    // The caller's registry never replaces the retained one, but supplying it
    // is how a caller opts into the compiler-pipeline intrep/taint lines.
    let annotate = registry.is_some();
    hover_impl(
        source,
        line,
        character,
        analysis,
        crate::definition::CallResolution {
            registry: Some(current.registry()),
            program,
        },
        current.profile(),
        annotate,
    )
}

/// Render the hover body for a proc or class that lives in **another**
/// document, identified by its qualified name.
///
/// `defining_analysis` is the analyser result for the document the symbol is
/// declared in — the same result an in-document hover there would use, so the
/// rendering is identical whichever file the cursor is in.
/// Class hover in particular reads superclass and mixin methods out of that
/// analysis, which is why the *defining* file's result is required rather than
/// the caller's.
///
/// `qualified` is the absolute name (`::math::zzqfrobnicate`); a proc is
/// preferred over a class of the same name, matching go-to-definition's own
/// ordering. Returns `None` when the name names neither.
#[must_use]
pub fn qualified_symbol_hover(
    defining_analysis: &AnalysisResult,
    qualified: &str,
) -> Option<Hover> {
    if let Some(proc_def) = defining_analysis.all_procs.get(qualified) {
        return Some(Hover::markdown(proc_hover_text(proc_def)));
    }
    let class_def = defining_analysis.all_classes.get(qualified)?;
    Some(Hover::markdown(class_hover_text(
        defining_analysis,
        class_def,
    )))
}

/// Hover for a **namespace variable** declared in another document, rendered
/// from that document's own analysis — the variable twin of
/// [`qualified_symbol_hover`].
///
/// `qualified` is the `::`-rooted cell name
/// ([`crate::definition::qualified_variable_cell_at`]); the reference count
/// shown is the declaring document's, matching exactly what hovering the
/// declaration itself renders.
#[must_use]
pub fn qualified_variable_hover(
    defining_analysis: &AnalysisResult,
    qualified: &str,
) -> Option<Hover> {
    let (qualified, var_def) =
        tcl_compiler::analyser::namespace_variables(&defining_analysis.global_scope)
            .into_iter()
            .find(|(q, _)| q == qualified)?;
    // The qualified name, not the tail: the cursor is in a *different*
    // document, so `palette` alone would not say which cell was found.  The
    // reference count is the declaring document's own, and says so — the
    // workspace-wide count is the code lens's job, not hover's.
    Some(Hover::markdown(format!(
        "**Variable** `{qualified}`\n\n{} reference(s) in the declaring document",
        var_def.references.len()
    )))
}

/// Hover for a **namespace**, rendered from counts the caller gathered — one
/// document for the in-document provider, the whole workspace index for the
/// server's cross-document tier.
///
/// `qualified` is the `::`-rooted namespace name
/// ([`crate::namespace_symbol::namespace_cell_at`]).  The markdown itself
/// comes from [`crate::namespace_symbol::namespace_hover_markdown`], the one
/// renderer, so the two tiers cannot word the same fact differently; this
/// wrapper exists because [`Hover`]'s constructors are crate-private.
///
/// `None` when nothing in the gathered set declares the namespace — a genuine
/// "no hover", never a licence for the caller to fall through to command
/// documentation.
#[must_use]
pub fn namespace_hover(
    qualified: &str,
    facts: crate::namespace_symbol::NamespaceFacts,
) -> Option<Hover> {
    crate::namespace_symbol::namespace_hover_markdown(qualified, facts).map(Hover::markdown)
}

/// Original-mode hover: a statically dispatched ensemble subcommand, else the
/// registry operand at the cursor.
fn original_hover(
    source: &str,
    (line, character): (u32, u32),
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
) -> Option<Hover> {
    ensemble_subcommand_hover(source, line, character, analysis, ctx).or_else(|| {
        let cursor_offset = crate::definition::byte_offset_at(
            &tcl_lexer::LineIndex::new(source),
            source,
            line,
            character,
        );
        original_registry::hover(source, analysis, cursor_offset)
            .or_else(|| declared_class_hover(source, analysis, cursor_offset))
    })
}

/// Conditional Logical function documentation from the genuine checked
/// expression, original source scope and independently retained header cards.
/// Native function/table selection remains a separate terminal path.
fn math_function_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<Hover> {
    use std::fmt::Write;
    let selected = crate::expr_context::math_function_target_at(source, analysis, cursor_offset)?;
    let mut text = format!(
        "**`{}`** — `expr` math function\n",
        selected.occurrence.function()
    );
    let mut rendered = false;
    if let Some(declarations) = &selected.declarations {
        for declaration in declarations {
            text.push_str("\n**Conditional source declaration**\n\n");
            text.push_str(&proc_hover_text(declaration.metadata()));
            text.push('\n');
            rendered = true;
        }
    }
    if let Some(metadata) = selected.builtin.and_then(|spec| spec.hover.as_ref()) {
        text.push_str("\n**Built-in expression metadata**\n");
        if !metadata.summary.is_empty() {
            let _ = writeln!(text, "\n{}", metadata.summary);
        }
        if let Some(synopsis) = metadata.synopsis.first() {
            let _ = write!(text, "\n```tcl\n{synopsis}\n```\n");
        }
        rendered = true;
    }
    if !rendered {
        return None;
    }
    text.push_str("\nSource candidates are conditional; function lookup, operand evaluation and observers remain unresolved.\n");
    if selected.occurrence.has_earlier_unknown_mutations() {
        text.push_str("Earlier source effects may change these candidates.\n");
    }
    if selected.declarations.is_none() {
        text.push_str("The source declaration inventory or lookup path is unavailable.\n");
    }
    Some(Hover::markdown(text))
}

/// Render only metadata independently selected by the original expression
/// function owner. Fixed tables never claim an existing command wrapper.
fn original_math_function_hover(
    selected: &crate::math_function_symbol::OriginalMathFunctionSelection<'_>,
) -> Option<Hover> {
    use std::fmt::Write;
    if let Some(declaration) = selected.procedure_metadata() {
        return Some(Hover::markdown(proc_hover_text(declaration.metadata())));
    }
    let spec = selected.builtin_spec()?;
    let hover = spec.hover.as_ref()?;
    let occurrence = selected.occurrence();
    let mut text = format!("**`{}`** — `expr` math function\n", occurrence.function());
    if !hover.summary.is_empty() {
        let _ = write!(text, "\n{}\n", hover.summary);
    }
    match occurrence.dispatch() {
        tcl_registry::mathfunc::NativeMathFunctionDispatch::FixedTable => {
            text.push_str("\nSelected interpreter function table entry.\n");
        }
        tcl_registry::mathfunc::NativeMathFunctionDispatch::CommandTable => {
            if let Some(synopsis) = hover.synopsis.first() {
                let _ = write!(text, "\n```tcl\n{synopsis}\n```\n");
            }
            let _ = write!(
                text,
                "\nCommand-table lookup: `{}`.\n",
                occurrence.registry_identity()?
            );
        }
    }
    Some(Hover::markdown(text))
}

/// [`hover`] resolving prefix-abbreviated subcommands and option / special-
/// variable availability against a specific dialect profile, so e.g.
/// `info class def` hovers `definition` under 8.6 but nothing (ambiguous
/// with `definitionnamespace`) under 9.0.
pub fn hover_with_profile(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Hover> {
    hover_in_program(source, line, character, analysis, registry, profile, None)
}

fn source_role_hover(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
    cursor_offset: u32,
    annotate: bool,
) -> std::ops::ControlFlow<Option<Hover>> {
    if let std::ops::ControlFlow::Break(selected) =
        crate::vendor_declaration::select_at_offset(source, analysis, cursor_offset)
    {
        return std::ops::ControlFlow::Break(selected.and_then(|selected| {
            let label =
                crate::vendor_declaration::source_label(selected.input(), selected.purpose())?;
            let category = if selected.procedure_metadata().is_some() {
                "Procedure"
            } else if selected.class_metadata().is_some() {
                "Class"
            } else {
                selected.symbol_metadata()?.kind.label()
            };
            let mut text = format!("**{category} source declaration** `{label}`");
            if let Some(procedure) = selected.procedure_metadata()
                && !procedure.doc.is_empty()
            {
                text.push_str("\n\n");
                text.push_str(&format_docstring(&procedure.doc));
            }
            if selected.input().literal_units(selected.purpose()).is_none() {
                text.push_str(
                    "\n\nRuntime name materialisation is unavailable for this source spelling.",
                );
            }
            Some(Hover::markdown(text))
        }));
    }
    // Everything the cursor's *position* decides, before any word-based
    // resolver runs. `Some` is definitive, including `Some(None)`.
    if let std::ops::ControlFlow::Break(selected) =
        crate::namespace_symbol::select_at_offset(source, analysis, cursor_offset)
    {
        return std::ops::ControlFlow::Break(selected.and_then(|symbol| {
            crate::namespace_symbol::original_namespace_hover_text(analysis, &symbol)
                .map(Hover::markdown)
        }));
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::variable_symbol::select_navigation(source, analysis, line, character, ctx)
    {
        return std::ops::ControlFlow::Break(selected.and_then(|selection| {
            navigation_variable_hover(source, analysis, &selection, cursor_offset, ctx, annotate)
        }));
    }
    match crate::method_symbol::local_candidate(source, analysis, line, character) {
        std::ops::ControlFlow::Break(Some(candidate)) => {
            return std::ops::ControlFlow::Break(
                declared_member_hover(source, analysis, &candidate).or_else(|| candidate.hover()),
            );
        }
        std::ops::ControlFlow::Break(None) => {
            // A unique written declaration supplies syntax, not a resolved method.
            return std::ops::ControlFlow::Break(
                counted_initial_method_hover(source, analysis, cursor_offset)
                    .or_else(|| {
                        original_registry::conditional_hover(source, analysis, cursor_offset)
                    })
                    .or_else(|| {
                        registry_pattern_format_hover(
                            source,
                            line,
                            character,
                            analysis,
                            ctx.registry?,
                        )
                    })
                    .or_else(|| declared_class_hover(source, analysis, cursor_offset)),
            );
        }
        std::ops::ControlFlow::Continue(()) => {}
    }
    if let Some(hover) = counted_initial_method_hover(source, analysis, cursor_offset) {
        return std::ops::ControlFlow::Break(Some(hover));
    }
    if let std::ops::ControlFlow::Break(selected) =
        original_property_source_hover(source, analysis, cursor_offset)
    {
        return std::ops::ControlFlow::Break(selected);
    }
    std::ops::ControlFlow::Continue(())
}

fn class_and_remaining_hover(
    source: &str,
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
    cursor_offset: u32,
    word: &str,
) -> Option<Hover> {
    if analysis.allows_retained_logical_declaration_advice() {
        let config = analysis.body_lexer_config?;
        let image = tcl_lexer::SourceImage::document(source);
        if analysis.matches_original_source_image(&image, config)
            && let Some(class) = analysis.logical_source_class_at(&image, config, cursor_offset)
            && analysis.logical_source_class_declaration(class).is_some()
        {
            let declaration = analysis.logical_source_class_declaration(class)?;
            let metadata = declaration.metadata();
            let mut text = format!(
                "```tcl\n{} create {}\n```",
                metadata.metaclass, metadata.qualified_name
            );
            if !metadata.doc.is_empty() {
                text.push_str("\n\n");
                text.push_str(&metadata.doc);
            }
            return Some(Hover::markdown(text));
        }
    } else {
        match crate::original_oo::class_at_cursor(analysis, source, cursor_offset) {
            std::ops::ControlFlow::Break(Some(record)) => {
                return Some(Hover::markdown(class_hover_text(
                    analysis,
                    record.metadata(),
                )));
            }
            std::ops::ControlFlow::Break(None) => {}
            std::ops::ControlFlow::Continue(()) => {
                if let Some((_, class_def)) = crate::definition::resolve_class_target_at(
                    analysis,
                    source,
                    ctx,
                    cursor_offset,
                    word,
                ) {
                    return Some(Hover::markdown(class_hover_text(analysis, class_def)));
                }
            }
        }
    }

    // Class-member hover — same dispatch as
    // [`crate::definition::lookup_class_member`], rendered as
    // a one-line method / property summary.  Fires when the
    // cursor sits inside a class body and `word` matches one
    // of that class's members.
    if let Some(text) = class_member_hover_text(source, analysis, word, cursor_offset) {
        return Some(Hover::markdown(text));
    }
    if let Some(answer) = original_registry::hover(source, analysis, cursor_offset) {
        return Some(answer);
    }

    if let Some(text) = ip_address_hover_text(word) {
        return Some(Hover::markdown(text));
    }

    None
}

/// The answer under a native recipe: the proof answer, an address literal, then
/// the document's declared procedures as a marked display answer.
fn native_hover(
    source: &str,
    (line, character): (u32, u32),
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
    word: Option<&str>,
) -> Option<Hover> {
    let cursor_offset =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)?
            .offset();
    // Registry operands may have their own genuine expansion/list-child
    // geometry. A standalone static-word label is not their prerequisite.
    original_hover(source, (line, character), analysis, ctx)
        .or_else(|| word.and_then(ip_address_hover_text).map(Hover::markdown))
        .or_else(|| lenient_declared_procedure_hover(analysis, cursor_offset, word?, ctx))
}

/// The body of [`hover_with_profile`] / [`hover_in_program`], stated once over
/// the [`crate::definition::CallResolution`] both build.
/// Hovers decided by the exact cursor position before any word lookup: a
/// proved registered instance's method word (a Tk widget call), or an
/// operator or grouping parenthesis inside a braced expression.
fn positional_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Hover> {
    registered_instance::method_hover(source, analysis, cursor_offset, profile).or_else(|| {
        expression_precedence::expression_precedence_hover(source, analysis, cursor_offset)
    })
}

fn hover_impl(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
    profile: &'static tcl_dialect::DialectProfile,
    annotate: bool,
) -> Option<Hover> {
    let registry = ctx.registry;
    // One index shared by the position conversions below.
    let line_index = tcl_lexer::LineIndex::new_lsp(source);
    let cursor_offset =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)?
            .offset();
    if let Some(hover) = positional_hover(source, analysis, cursor_offset, profile) {
        return Some(hover);
    }
    if let std::ops::ControlFlow::Break(selected) = source_role_hover(
        source,
        line,
        character,
        analysis,
        ctx,
        cursor_offset,
        annotate,
    ) {
        return selected;
    }
    if let Some(hover) = conditional_native_formal_hover(source, analysis, cursor_offset) {
        return Some(hover);
    }
    match variable_position_hover(
        source,
        (line, character),
        &line_index,
        analysis,
        ctx,
        profile,
        annotate,
    ) {
        PositionHover::Answer(hover) => return Some(hover),
        PositionHover::Abstain => return None,
        PositionHover::FallThrough => {}
    }

    if let std::ops::ControlFlow::Break(selected) =
        crate::math_function_symbol::select_at_offset(source, analysis, cursor_offset)
    {
        return selected.and_then(|selected| original_math_function_hover(&selected));
    }

    // `expr` math-function hover — asked before every
    // remaining path: inside an expression a `NAME(` word is a function-call
    // production, not a command lookup, so nothing else may claim it.  See
    // `math_function_hover`.
    if analysis
        .logical_math_function_at(source, cursor_offset)
        .is_some()
        || (analysis.allows_lexical_declaration_advice()
            && crate::expr_context::mathfunc_call_at(analysis, cursor_offset).is_some())
    {
        // An unavailable Logical receipt cannot borrow an ordinary head or the
        // analyser's resolved display. Native selection above remains terminal.
        return math_function_hover(source, analysis, cursor_offset);
    }

    // A word inside an enclosing proc/method's own *literal* parameter list is
    // pure data — a parameter name (already answered by `variable_hover`) or a
    // default value — never a command reference (both the parameter name
    // `destroy` and the default-value literal `destroy` in
    // `proc ::tk::RestoreFocusGrab {grab focus {destroy destroy}}` rendered
    // Tk's `destroy` *command* documentation).  A *computed* parameter list
    // (`proc p [makeargs] {…}`) holds live code and stays navigable.
    if crate::definition::offset_is_in_parameter_list(analysis, source, cursor_offset) {
        return None;
    }

    // A word naming a namespace — span-precise, so it is asked
    // before every word-based resolver AND it is **definitive**: once the
    // cursor is provably inside a registry-declared `ArgRole::NamespaceName`
    // argument, a proc, class, or built-in command of the same spelling is
    // not what it refers to.  Tcl keeps namespaces in their own symbol
    // space, so falling through on an empty local answer produced *command*
    // documentation for `namespace exists string` whenever `::string` was
    // declared in a sibling document — and, because that is a `Some`, the
    // server returned it and never consulted the cross-document namespace
    // tier at all.  `None` here means "no
    // local answer", which is exactly the signal that tier needs.
    if let Some(cell) =
        crate::namespace_symbol::namespace_cell_at(source, analysis, line, character)
    {
        return crate::namespace_symbol::namespace_hover_text(analysis, &cell).map(Hover::markdown);
    }

    let hover_registry = registry.unwrap_or_else(|| crate::registry_for_dialect_profile(profile));
    if let Some(hover) =
        registry_pattern_format_hover(source, line, character, analysis, hover_registry)
    {
        return Some(hover);
    }

    if let Some(hover) = positioned_class_hover(source, analysis, cursor_offset) {
        return Some(hover);
    }

    let word =
        crate::source_structure::original_complete_cursor_word(source, analysis, cursor_offset)
            .and_then(|word| word.static_unicode_value());
    if let Some(word) = word.as_deref()
        && let Some(text) = original_linked_member_hover_text(source, analysis, word, cursor_offset)
    {
        return Some(Hover::markdown(text));
    }
    if word.is_some()
        && let Some(hover) = source_constructor_hover(source, analysis, cursor_offset)
    {
        return Some(hover);
    }
    if !analysis.allows_lexical_declaration_advice() {
        return native_hover(source, (line, character), analysis, ctx, word.as_deref());
    }
    let word = word?;

    // `$obj m` / `my m` method-dispatch hover, including the per-object
    // visibility mask — `Break(None)` is a definitive
    // no-hover.
    if let std::ops::ControlFlow::Break(answer) =
        method_dispatch_hover(source, line, character, cursor_offset, analysis)
    {
        return answer;
    }

    // `<ensemble> <subcommand>` hover — see
    // `ensemble_subcommand_hover`'s doc for the full rationale; must run
    // before the generic proc lookup below for the same reason as the
    // `$obj method` check above: `make` is never independently a command,
    // only the pair `widget make` dispatches.
    if let Some(hover) = ensemble_subcommand_hover(source, line, character, analysis, ctx) {
        return Some(hover);
    }

    // The alias recipe belongs to the actual post-argument lookup snapshot.
    // Missing bytes cannot borrow a reporting-map target from another phase.
    if let std::ops::ControlFlow::Break(answer) =
        alias_hover_at(source, analysis, cursor_offset, &word)
    {
        return answer.map(Hover::markdown);
    }

    // Proc hover — namespace-aware, following C Tcl's command resolution
    // (`Tcl_FindCommand`, `tclNamesp.c`): the cursor's namespace first, then
    // the global namespace (absolute `::`-prefixed words exact), so a
    // namespace proc shadows a same-named builtin *inside its namespace
    // only*.  When no candidate is defined and the word names a registry
    // builtin, this yields nothing and the registry hover below wins — a
    // proc in an unrelated namespace must not hijack builtin hover.  A
    // non-builtin word keeps the lenient (deterministic) tail fallback.
    if let Some(hover) = proc_hover_at(analysis, source, cursor_offset, &word, ctx) {
        return Some(hover);
    }

    class_and_remaining_hover(source, analysis, ctx, cursor_offset, &word)
}

/// Format hover markdown for an IPv4 / IPv6 literal at the
/// cursor's word.
///
/// Returns `None` when `word` isn't a valid IP literal.  An
/// optional `/prefix` suffix is supported; the prefix is
/// rendered as a CIDR network in the result.
fn ip_address_hover_text(word: &str) -> Option<String> {
    use std::fmt::Write;
    if !word.contains('.') && !word.contains(':') {
        return None;
    }
    // Strip the optional `/prefix` suffix before parsing.
    let (addr, prefix) = match word.split_once('/') {
        Some((a, p)) => (a, p.parse::<u8>().ok()),
        None => (word, None),
    };
    if let Ok(v4) = addr.parse::<std::net::Ipv4Addr>() {
        let class = classify_ipv4(v4);
        let mut out = format!("**IPv4 address** `{addr}`\n\n* Classification: {class}\n");
        if let Some(p) = prefix
            && p <= 32
        {
            let _ = writeln!(out, "* CIDR network: `{addr}/{p}`");
        }
        return Some(out);
    }
    if let Ok(v6) = addr.parse::<std::net::Ipv6Addr>() {
        let class = classify_ipv6(v6);
        let mut out = format!("**IPv6 address** `{addr}`\n\n* Classification: {class}\n");
        if let Some(p) = prefix
            && p <= 128
        {
            let _ = writeln!(out, "* CIDR network: `{addr}/{p}`");
        }
        // IPv4-mapped form (`::ffff:x.x.x.x`).
        if let Some(mapped) = v6.to_ipv4_mapped() {
            let _ = writeln!(out, "* IPv4-mapped form: `{mapped}`");
        }
        return Some(out);
    }
    None
}

/// Classify an IPv4 address by RFC category — loopback,
/// private, multicast, broadcast, link-local, unspecified,
/// or public.
fn classify_ipv4(addr: std::net::Ipv4Addr) -> &'static str {
    if addr.is_unspecified() {
        "Unspecified (`0.0.0.0`)"
    } else if addr.is_loopback() {
        "Loopback (RFC 1122)"
    } else if addr.is_private() {
        "Private (RFC 1918)"
    } else if addr.is_link_local() {
        "Link-local (RFC 3927)"
    } else if addr.is_multicast() {
        "Multicast (RFC 5771)"
    } else if addr.is_broadcast() {
        "Broadcast"
    } else if addr.is_documentation() {
        "Documentation (RFC 5737)"
    } else {
        "Public / global"
    }
}

/// Classify an IPv6 address by RFC category.
fn classify_ipv6(addr: std::net::Ipv6Addr) -> &'static str {
    if addr.is_unspecified() {
        "Unspecified (`::`)"
    } else if addr.is_loopback() {
        "Loopback (`::1`)"
    } else if addr.is_multicast() {
        "Multicast (RFC 4291)"
    } else if addr.to_ipv4_mapped().is_some() {
        "IPv4-mapped (RFC 4291)"
    } else if addr.segments()[0] & 0xffc0 == 0xfe80 {
        "Link-local (RFC 4291)"
    } else if addr.segments()[0] & 0xfe00 == 0xfc00 {
        "Unique local (RFC 4193)"
    } else {
        "Global unicast"
    }
}

/// The `[start, end)` char-index bounds of the bare Tcl word around `col`
/// in `chars` (one source line), using [`WORD_DELIMS`].
///
/// Test-only method-name bounds using the standalone cursor presentation
/// rule. This supplies no original source, command identity or editable span.
/// Bare words are bounded by whitespace and the structural
/// characters `;{}[]"$` — rather than a programming-language identifier
/// rule.  Tcl puts no character restriction on a command or method name, so
/// `with-dash`, `a.b`, and TIP 558's generated property accessors
/// `<ReadProp-x>` / `<WriteProp-x>` are all one word (verified against
/// tclsh 8.6.14 and 9.0.4).
///
/// Returns `None` when the cursor sits on a delimiter run (empty word).
///
/// A single `:` is *not* a delimiter, and must not become one: in Tcl only
/// `::` is a namespace separator and a lone colon is an ordinary name
/// character, so `a:b`, `p:q`, `arr(k:1)`, and a dict key `x:y` are each one
/// name (verified against tclsh 8.6 and 9.0 — `set a:b 42`, `proc p:q {x} …`,
/// `namespace eval ns { proc c:d {} … }` all work).  What *is* handled is the
/// residual-`::` case: when the left scan stops on a substitution closer
/// (`}` / `]`), a leading `::` in the word is the tail of a **computed** name
/// (`${ns}::setdef`), not an absolute one — see [`word_char_bounds_kinded`]'s return
/// value.
///
/// Deliberately *not* handled: a word split across lines by a
/// backslash-newline continuation, and the interior structure of a word
/// that mixes literal text with substitutions (`pre[cmd]post`) — the
/// caller sees the literal slice only.
#[cfg(test)]
pub(crate) fn word_char_bounds(chars: &[char], col: usize) -> Option<(usize, usize)> {
    word_char_bounds_kinded(chars, col).map(|(start, end, _)| (start, end))
}

/// Standalone lexical bounds, also reporting whether the word is the **residual
/// tail of a computed name** — the literal fragment that follows a `${var}` /
/// `[cmd]` substitution inside the same word (`${ns}::setdef` → `::setdef`).
///
/// The scan cannot see the substitution (it stops at its closer, which is a
/// delimiter), so without this flag `::setdef` is indistinguishable from a
/// genuinely absolute `::setdef` and resolution looks for a global proc of
/// that literal name — finding nothing, and reporting no definition for a
/// call that statically resolves fine (`${ns}::setdef` with `ns ==
/// "::ticklecharts"`). The flag separates standalone absolute spellings from
/// computed tails; it supplies no lookup identity.
pub(crate) fn word_char_bounds_kinded(chars: &[char], col: usize) -> Option<(usize, usize, bool)> {
    let mut start = col.min(chars.len());
    while start > 0 && !WORD_DELIMS.contains(&chars[start - 1]) {
        start -= 1;
    }
    let mut end = col.min(chars.len());
    while end < chars.len() && !WORD_DELIMS.contains(&chars[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    // A `}` / `]` immediately left of the word closes a `${…}` / `[…]`
    // substitution that is part of this same word: whatever follows is a
    // *fragment*, not a standalone name.
    let after_substitution = start > 0 && matches!(chars[start - 1], '}' | ']');
    let residual = after_substitution && chars[start..end].starts_with(&[':', ':']);
    Some((start, end, residual))
}

/// Find the word and its `[start, end)` columns at the given
/// position, using Tcl's word delimiters.
///
/// Returns `None` when
/// `line` / `character` is out of bounds or the cursor sits on a
/// delimiter run.
///
/// A residual literal tail after a `${var}` / `[cmd]` substitution, such as
/// `setdef` in `${ns}::setdef`, excludes the preceding `::` from its reported
/// name and span. Rename can then preserve the computed namespace prefix.
/// This lexical tail alone supplies no command identity.
#[must_use]
pub fn find_word_span_at_position(
    source: &str,
    line: u32,
    character: u32,
) -> Option<(String, u32, u32)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character);
    if col >= chars.len() {
        return None;
    }
    let (mut start, end, residual) = word_char_bounds_kinded(&chars, col)?;
    if residual && col >= start + 2 {
        start += 2;
        if start >= end {
            return None;
        }
    }
    let word: String = chars[start..end].iter().collect();
    let prefix: String = chars[..start].iter().collect();
    let start_u32 = crate::definition::utf16_len(&prefix);
    let end_u32 = start_u32.saturating_add(crate::definition::utf16_len(&word));
    Some((word, start_u32, end_u32))
}

/// Standalone substitution syntax lookup with the compatibility grammar.
/// It supplies a source label only. Editor consumers with an analysis use the
/// retained-input cursor facade rather than selecting this nominal grammar.
#[must_use]
pub fn find_var_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    // dialect-drift-ok: explicit compatibility utility without AnalysisResult.
    find_var_at_position_with_config(source, line, character, tcl_lexer::LexerConfig::default())
}

/// Standalone substitution syntax lookup under explicitly selected grammar.
/// No source label proves variable resolution, a read or an editable reference.
#[must_use]
pub fn find_var_at_position_with_config(
    source: &str,
    line: u32,
    character: u32,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let index = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&index, source, line, character);
    crate::source_structure::template_variable_reference_at(source, config, cursor)
        .map(|reference| reference.root.to_owned())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_alias_hover_tests;

#[cfg(test)]
mod original_math_hover_tests;

#[cfg(test)]
mod original_vendor_hover_tests;

#[cfg(test)]
mod original_pattern_hover_tests;

#[cfg(test)]
mod original_special_variable_context_tests;

#[cfg(test)]
mod logical_math_source_hover_tests;

#[cfg(test)]
mod original_hover_independent_registry_tests;

#[cfg(test)]
mod local_logical_hover_source_owner_tests;

#[cfg(test)]
mod source_documentation_tests;
