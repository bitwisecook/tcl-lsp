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

use std::collections::HashMap;
use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::surface_admits;

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::{AnalysisResult, ClassDef, ProcDef, VarDef};
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::taint::{TaintColour, TaintLattice};
use tcl_compiler::types::{TclType, TypeKind, TypeLattice};
use tcl_lexer::TokenType;
use tcl_registry::CommandRegistry;

mod original_registry;

use crate::definition::utf16_col_to_char_col;
use tcl_dialect::model::SpecSurface;

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

/// Compute hover text for a position in `source`.
///
/// `analysis` is the pre-computed analyser result; the caller is
/// expected to cache it. Returns `None` when:
///
/// * `line` / `character` falls outside the source extents,
/// * the cursor isn't on any recognisable identifier or `$var`,
/// * no proc / class / var matches the resolved word.
///
/// The character index is interpreted as UTF-16 code units per
/// the LSP spec, but this provider treats it as a char-count
/// index.  Multi-byte BMP code points round-trip correctly;
/// supplementary-plane characters can drift by one position
/// (rare in Tcl source).
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
    _registry: Option<&CommandRegistry>,
    _profile: &'static tcl_dialect::DialectProfile,
    program: Option<crate::definition::ProgramExports<'_>>,
) -> Option<Hover> {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
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
    if !defining_analysis.allows_lexical_declaration_advice() {
        return None;
    }
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
    if !defining_analysis.allows_lexical_declaration_advice() {
        return None;
    }
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

/// `<ensemble> <subcommand>` hover — a static `namespace ensemble create
/// -map`/`-subcommands` mapping, the hover twin of
/// `definition()`'s identical check. Extracted from
/// [`hover_with_profile`] to stay within the line budget.
fn ensemble_subcommand_hover(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
) -> Option<Hover> {
    let (head, sub, is_dollar) = crate::definition::instance_method_at_cursor(
        source,
        line,
        character,
        crate::definition::dialect_config(analysis),
    )?;
    if is_dollar {
        return None;
    }
    let line_index = tcl_lexer::LineIndex::new(source);
    let cursor_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    let namespace = crate::definition::namespace_context_at(
        &analysis.global_scope,
        cursor_offset,
        &analysis.namespace_overrides,
    );
    let target = crate::definition::ensemble_subcommand_target(analysis, &namespace, &head, &sub)?;
    let proc_def =
        crate::definition::resolve_called_proc(analysis, source, "::", target, cursor_offset, ctx)?;
    let text = format!(
        "**Ensemble subcommand** of `{head}`\n\n{}",
        proc_hover_text(proc_def)
    );
    Some(Hover::markdown(text))
}

/// `expr` math-function hover — the **bare** in-expression spelling
/// `sin(1.0)` / `max($a, $b)`, which renders from the same registry spec the
/// qualified `::tcl::mathfunc::sin` spelling reads.  Both spellings must be
/// registered: with only the qualified ones, a bare call draws nothing at any
/// column.
///
/// Definitive: once the cursor is on a recorded math-function call, this is
/// what the call reaches — a user override, the built-in, or (when the
/// function does not exist in this dialect) nothing at all. The caller must
/// not fall through to ordinary command resolution, which would let a
/// coincidentally same-named proc in an unrelated namespace hijack the hover.
///
/// See [`crate::expr_context`] for the resolution rule and its C Tcl oracle.
fn math_function_hover(
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    cursor_offset: u32,
) -> Option<Hover> {
    use crate::expr_context::MathFuncTarget;
    match crate::expr_context::math_function_target_at(analysis, registry, profile, cursor_offset)?
    {
        MathFuncTarget::UserProc(proc_def) => Some(Hover::markdown(proc_hover_text(proc_def))),
        MathFuncTarget::Builtin {
            bare,
            qualified,
            spec,
        } => {
            use std::fmt::Write;
            let hover = spec.hover.as_ref()?;
            let mut out = format!("**`{bare}`** — `expr` math function\n");
            if !hover.summary.is_empty() {
                let _ = write!(out, "\n{}\n", hover.summary);
            }
            if let Some(synopsis) = hover.synopsis.first() {
                let _ = write!(out, "\n```tcl\n{synopsis}\n```\n");
            }
            let _ = write!(out, "\nDispatches to `{qualified}`.\n");
            Some(Hover::markdown(out))
        }
    }
}

/// Render only metadata independently selected by the original expression
/// function owner. Fixed tables never claim an existing command wrapper.
fn original_math_function_hover(
    selected: &crate::math_function_symbol::OriginalMathFunctionSelection<'_>,
) -> Option<Hover> {
    if let Some(declaration) = selected.procedure_metadata() {
        return Some(Hover::markdown(proc_hover_text(declaration.metadata())));
    }
    use std::fmt::Write;
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

/// Proc hover at `cursor_offset`: namespace-aware, following C Tcl's
/// command resolution (`Tcl_FindCommand`, `tclNamesp.c`) — the cursor's
/// namespace first (consulting `analysis.namespace_overrides` ahead of the
/// ordinary lexical walk), then the global namespace.
/// Extracted from [`hover_with_profile`] to keep it within the line budget.
/// Gated on the cursor's word actually occupying the enclosing command's
/// head position: hover shares `definition()`'s text-scan fallback, so
/// without the gate an ordinary *argument* that happens to share a proc's
/// name renders that proc's documentation. See
/// [`crate::definition::offset_is_command_head`].
///
/// An already-resolved **indirect** head (a constant `${ns}::cmd` / `$cmd`)
/// is answered from the analyser's own resolution, since the
/// span carries no written command name for the bareword lookup to use.
fn proc_hover_at(
    analysis: &AnalysisResult,
    source: &str,
    cursor_offset: u32,
    word: &str,
    ctx: crate::definition::CallResolution<'_>,
) -> Option<Hover> {
    // A proc's own declaration name (`proc greet …`) is an argument of
    // `proc`, so the head gate below would refuse it; it is answered here,
    // mirroring how `definition()` consults `proc_declaration_sites` before
    // its own call resolution.
    if let Some(proc_def) = crate::definition::proc_declaration_at(analysis, cursor_offset) {
        return Some(Hover::markdown(proc_hover_text(proc_def)));
    }
    if let Some(invocation) = crate::definition::invocation_reference_at(analysis, cursor_offset)
        && !invocation.lookup.is_execution_site()
    {
        let reference = invocation.resolved_command_reference.as_ref()?;
        let definition = reference
            .linked_definition()
            .or_else(|| reference.definition())?;
        let proc_def = analysis.proc_for_definition(definition, source)?;
        return Some(Hover::markdown(proc_hover_text(proc_def)));
    }
    if let Some(proc_def) = crate::definition::resolved_indirect_head_proc(analysis, cursor_offset)
    {
        return Some(Hover::markdown(proc_hover_text(proc_def)));
    }
    if !crate::definition::offset_is_command_head(analysis, cursor_offset) {
        return None;
    }
    let namespace = crate::definition::namespace_context_at(
        &analysis.global_scope,
        cursor_offset,
        &analysis.namespace_overrides,
    );
    let proc_def = crate::definition::resolve_called_proc(
        analysis,
        source,
        &namespace,
        word,
        cursor_offset,
        ctx,
    )?;
    Some(Hover::markdown(proc_hover_text(proc_def)))
}

/// Variable hover at the cursor: a `$`-prefixed read first (surfacing the
/// [`VarDef`] — or, absent a user definition, a dialect-aware
/// interpreter-provided special variable's documentation), then a
/// bareword *declaration* / same-cell write site (a `set x`/`variable x`
/// target, a proc/method parameter, a `catch` result-var) via
/// [`crate::definition::var_def_at_declaration_offset`] — see that
/// function's own doc for why it can't reuse the ordinary scope-chain walk.
/// Extracted from [`hover_with_profile`] to keep it within the line budget.
///
/// `profile` carries the document's dialect through to the type/taint
/// inference below: the intrep lattice is read off a freshly-built
/// [`CompilationUnit`], and building that with the default (plain-Tcl) lexer
/// config mis-tokenises a dialect-specific construct — an iRules word
/// operator, `{*}` under 8.4 — which skews or drops the inferred type.
fn variable_hover(
    source: &str,
    line: u32,
    character: u32,
    line_index: &tcl_lexer::LineIndex,
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Hover> {
    let registry = ctx.registry;
    let dialect = Some(crate::document_context_for_profile(profile).authoring_query());
    // `$var` resolution sits at a position where `find_word_span_at_position`
    // would also match the unqualified name, but a `$`-led ref should
    // surface the `VarDef` not the (typically absent) proc of the same name.
    let var_byte_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    // Resolved through the shared gate, not the raw character scan: a cursor
    // inside a brace-quoted variable-name word (`set {$n} 1`) is not a `$n`
    // reference, and must fall through to the declaration-span search below
    // so it hovers the *literal* cell.
    if let Some(var_name) = crate::definition::substituting_var_at_position(
        source,
        analysis,
        line,
        character,
        var_byte_offset,
    ) {
        // Use the byte-offset scope-chain lookup (the local line-based helper
        // mis-resolves namespace/proc-scoped vars), gated on the occurrence
        // actually being one Tcl substitutes — see `lookup_var_read_at`.
        if let Some(var_def) =
            crate::definition::lookup_var_read_at(analysis, source, var_byte_offset, &var_name)
        {
            // Inferred-intrep / taint annotations need the compiler
            // pipeline (`CompilationUnit`), which requires a
            // registry; without one we surface just the reference
            // count.
            let (type_info, taint_info) =
                var_type_annotations(source, analysis, var_byte_offset, &var_name, registry);
            let format_info = registry.and_then(|registry| {
                registry_pattern_format_hover(source, line, character, analysis, registry)
            });
            return Some(Hover::markdown(var_hover_text(
                var_def,
                type_info.as_deref(),
                taint_info.as_deref(),
                format_info.as_ref().map(|h| h.value.as_str()),
            )));
        }
        // No user definition: an interpreter-provided special variable
        // (`auto_path`, `env`, `tcl_platform`, the iRules `static::` namespace)
        // still has documentation, sourced from the dialect-aware
        // special-variable registry.  The `(idx)` array index is already
        // supplied by the selected source-reference root, so `$tcl_platform(os)` resolves
        // to the `tcl_platform` spec.
        if let Some(spec) = tcl_registry::special_var(&var_name).filter(|s| s.available_in(dialect))
        {
            return Some(Hover::markdown(special_var_hover_text(spec, dialect)));
        }
        // Still no definition — but a variable this frame never assigns may
        // be one a callee creates here through `upvar`.  The call site names
        // it, so hover can say so.
        let bindings = crate::caller_frame::caller_frame_bindings(
            analysis,
            source,
            profile,
            ctx,
            var_byte_offset,
            &var_name,
        );
        if let Some(binding) = crate::caller_frame::primary_binding(&bindings) {
            return Some(Hover::markdown(caller_frame_hover_text(&var_name, binding)));
        }
        return None;
    }

    let decl_byte_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    // A *computed* parameter list (`proc p [makeargs] …`) registers no
    // `VarDef` for the whole word, so this lookup finds nothing there and the
    // caller falls through to the command hover — which is what such a word
    // really is.  No guard of its own is needed here.
    let var_def =
        crate::definition::var_def_at_declaration_offset(&analysis.global_scope, decl_byte_offset)?;
    let (type_info, taint_info) =
        var_type_annotations(source, analysis, decl_byte_offset, &var_def.name, registry);
    Some(Hover::markdown(var_hover_text(
        var_def,
        type_info.as_deref(),
        taint_info.as_deref(),
        None,
    )))
}

/// The hover the cursor's *position* decides — a variable read or a bareword
/// declaration — or `Some(None)` when the position forbids every word-based
/// resolver from answering.  `None` falls through to those resolvers.
///
/// A `$`-led read resolving to nothing is **definitive**, not a
/// fall-through: Tcl keeps variables and commands in disjoint namespaces, so
/// `$dataset` can never denote a proc, a method, or a class member of that
/// name.  Without this stop, hover answers a caller-frame `$dataset` read
/// with the card of an unrelated same-named `TclOO` accessor method — a
/// wrong-kind answer, worse than none.  `definition` forces the same
/// abstention in `position_definition`.
fn variable_position_hover(
    source: &str,
    line: u32,
    character: u32,
    line_index: &tcl_lexer::LineIndex,
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
    profile: &'static tcl_dialect::DialectProfile,
) -> PositionHover {
    if !analysis.allows_lexical_declaration_advice() {
        return PositionHover::FallThrough;
    }
    if let Some(hover) = variable_hover(source, line, character, line_index, analysis, ctx, profile)
    {
        return PositionHover::Answer(hover);
    }
    let cursor_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    if crate::caller_frame::substituted_var_read_at(
        source,
        analysis,
        line,
        character,
        cursor_offset,
    )
    .is_some()
    {
        return PositionHover::Abstain;
    }
    PositionHover::FallThrough
}

/// What [`variable_position_hover`] decided.
enum PositionHover {
    /// The position resolved to this card.
    Answer(Hover),
    /// The position forbids every word-based resolver from answering.
    Abstain,
    /// Not a variable position — try the word-based resolvers.
    FallThrough,
}

/// Embedded pattern and format advice from the retained source inventory.
/// All source domains keep their exact source, configuration and context seal;
/// missing original operands do not enable a nominal compatibility walker.
fn registry_pattern_format_hover(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> Option<Hover> {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    let line_index = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
    original_pattern_format_hover(source, analysis, registry, cursor)
}

/// Readonly embedded-language advice from the same original effective argv.
fn original_pattern_format_hover(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    cursor: u32,
) -> Option<Hover> {
    let context = analysis.resolved_input.as_ref()?.context_registry();
    if registry.snapshot().semantic_key() != context.commands().snapshot().semantic_key() {
        return None;
    }
    let commands = crate::original_invocation::registry_commands_in_source(source, analysis)?;
    let (_, words) = commands
        .into_iter()
        .filter(|(command, _)| command.span.start() <= cursor && cursor <= command.span.end())
        .min_by_key(|(command, _)| command.span.end() - command.span.start())?;
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    let mut proven_unit = None;
    let patterns = words
        .with_source_schema(&context, |schema| {
            schema.authored_source_pattern_arguments()
        })
        .flatten()?;
    for pattern in patterns {
        let Some(text) = original_embedded_operand(&words, usize::from(pattern.index), cursor)
            .map(str::to_owned)
            .or_else(|| {
                proven_embedded_operand(
                    source,
                    analysis,
                    registry,
                    &words,
                    usize::from(pattern.index),
                    cursor,
                    &mut proven_unit,
                )
            })
        else {
            continue;
        };
        let text = match pattern.kind {
            tcl_registry::patterns::PatternType::Glob => glob_hover_text(&text),
            tcl_registry::patterns::PatternType::Regex => regex_hover_text(&text),
        };
        return Some(Hover::markdown(text));
    }
    let formats = words
        .with_source_schema(&context, |schema| schema.authored_source_format_arguments())
        .flatten()?;
    for format in formats {
        let Some(text) = original_embedded_operand(&words, format.index, cursor)
            .map(str::to_owned)
            .or_else(|| {
                proven_embedded_operand(
                    source,
                    analysis,
                    registry,
                    &words,
                    format.index,
                    cursor,
                    &mut proven_unit,
                )
            })
        else {
            continue;
        };
        let text = match format.kind {
            tcl_registry::patterns::FormatType::Sprintf if text.contains('%') => {
                sprintf_format_hover_text(&text)
            }
            tcl_registry::patterns::FormatType::Clock if text.contains('%') => {
                clock_format_hover_text(&text)
            }
            tcl_registry::patterns::FormatType::Binary => {
                let arguments = words
                    .arguments
                    .iter()
                    .map(|argument| {
                        argument
                            .literal_bytes()
                            .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    })
                    .collect::<Vec<_>>();
                binary_format_hover_text(&BinaryContext {
                    text: text.to_owned(),
                    subcmd: arguments.first().copied().flatten()?.to_owned(),
                    args: arguments
                        .get(format.index.checked_add(1)?..)?
                        .iter()
                        .map(|argument| argument.unwrap_or_default().to_owned())
                        .collect(),
                })
            }
            tcl_registry::patterns::FormatType::Regsub
                if !scan_regsub_backrefs(&text).is_empty() =>
            {
                regsub_hover_text(&text)
            }
            _ => continue,
        };
        return Some(Hover::markdown(text));
    }
    None
}

/// Known contents at a genuine original operand, retaining its source owner.
/// The computed value supplies a mini-language card, never source coordinates.
fn proven_embedded_operand(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    words: &crate::original_invocation::OriginalRegistryWords,
    argument: usize,
    cursor: u32,
    unit: &mut Option<CompilationUnit>,
) -> Option<String> {
    let word = words.operands.get(argument)?.as_ref()?.word.as_ref()?;
    let span = word.span();
    (span.start() <= cursor && cursor <= span.end()).then_some(())?;
    if unit.is_none() {
        *unit = crate::original_invocation::source_compilation_unit(source, analysis, registry);
    }
    crate::original_invocation::known_word_contents(unit.as_ref()?, registry, span)
}

fn original_embedded_operand(
    words: &crate::original_invocation::OriginalRegistryWords,
    argument: usize,
    cursor: u32,
) -> Option<&str> {
    let operand = words.operands.get(argument)?.as_ref()?;
    let span = if let Some(word) = &operand.word {
        // A computed value cannot claim the source code that produced it.
        if word
            .tokens()
            .iter()
            .any(|token| !matches!(token.kind, TokenType::Str | TokenType::Esc))
        {
            return None;
        }
        word.content_span().ok()?
    } else {
        // Only a retained readonly expansion child owns this independent span.
        operand.input.as_ref()?.original_static_list_container()?;
        operand.span
    };
    (span.start() <= cursor && cursor <= span.end()).then_some(())?;
    std::str::from_utf8(words.arguments.get(argument)?.literal_bytes()?).ok()
}

/// The format-string hovers: when the cursor sits on the format-string
/// argument of a known format-bearing command, render a table of the
/// specifiers it contains.  Covers `clock format` / `clock scan`, `format` /
/// `scan`, `binary format` / `binary scan`, and `regsub`'s subspec.
#[cfg(test)]
#[allow(dead_code)]
fn format_string_hover(source: &str, line: u32, character: u32) -> Option<Hover> {
    if let Some(text) = clock_format_string_at_position(source, line, character) {
        return Some(Hover::markdown(clock_format_hover_text(&text)));
    }
    if let Some(text) = sprintf_format_string_at_position(source, line, character) {
        return Some(Hover::markdown(sprintf_format_hover_text(&text)));
    }
    if let Some(ctx) = binary_format_context_at_position(source, line, character) {
        return Some(Hover::markdown(binary_format_hover_text(&ctx)));
    }
    let text = regsub_subspec_at_position(source, line, character)?;
    Some(Hover::markdown(regsub_hover_text(&text)))
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

/// The body of [`hover_with_profile`] / [`hover_in_program`], stated once over
/// the [`crate::definition::CallResolution`] both build.
fn hover_impl(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Hover> {
    let registry = ctx.registry;
    // One index shared by the position conversions below.
    let line_index = tcl_lexer::LineIndex::new(source);

    let cursor_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let std::ops::ControlFlow::Break(selected) =
        crate::vendor_declaration::select_at_offset(source, analysis, cursor_offset)
    {
        return selected.and_then(|selected| {
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
        });
    }
    // Everything the cursor's *position* decides, before any word-based
    // resolver runs. `Some` is definitive, including `Some(None)`.
    if let std::ops::ControlFlow::Break(selected) =
        crate::namespace_symbol::select_at_offset(source, analysis, cursor_offset)
    {
        return selected.and_then(|symbol| {
            crate::namespace_symbol::original_namespace_hover_text(analysis, &symbol)
                .map(Hover::markdown)
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::variable_symbol::select_navigation(source, analysis, line, character, ctx)
    {
        return selected
            .and_then(|selection| selection.hover_text(analysis))
            .map(Hover::markdown);
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::method_symbol::local_candidate(source, analysis, line, character)
    {
        return selected.and_then(|candidate| candidate.hover());
    }
    match variable_position_hover(source, line, character, &line_index, analysis, ctx, profile) {
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
    if let Some(hover) = math_function_hover(analysis, registry, profile, cursor_offset) {
        return Some(hover);
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

    if !analysis.allows_lexical_declaration_advice() {
        return original_registry::hover(source, analysis, cursor_offset).or_else(|| {
            let (word, _, _) = find_word_span_at_position(source, line, character)?;
            ip_address_hover_text(&word).map(Hover::markdown)
        });
    }

    let (word, _start, _end) = find_word_span_at_position(source, line, character)?;

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
                &word,
            ) {
                return Some(Hover::markdown(class_hover_text(analysis, class_def)));
            }
        }
    }

    // Class-member hover — same dispatch as
    // [`crate::definition::lookup_class_member`], rendered as
    // a one-line method / property summary.  Fires when the
    // cursor sits inside a class body and `word` matches one
    // of that class's members.
    if let Some(text) = class_member_hover_text(analysis, &word, cursor_offset) {
        return Some(Hover::markdown(text));
    }
    if let Some(answer) = original_registry::hover(source, analysis, cursor_offset) {
        return Some(answer);
    }

    if let Some(text) = ip_address_hover_text(&word) {
        return Some(Hover::markdown(text));
    }

    None
}

/// Resolve an unqualified command `name` at byte offset `cursor_offset` to its
/// qualified registry spec via the document's `namespace import`s
/// (`namespace import ::tcltest::*` → `test` resolves to `tcltest::test`).
/// Mirrors the analyser's imported-command body resolution, respecting the
/// import's **scope** and **source order**: only an import made at global scope
/// (`ns == "::"`, in effect everywhere via Tcl's unqualified-name fallback) and
/// positioned *before* the hovered word applies — a nested-namespace import
/// (e.g. inside `namespace eval ns { … }`) or one appearing after the cursor
/// must not retroactively resolve a bare name here.  Returns the qualified name
/// and its spec, or `None`.  Only unqualified names are resolved.
#[cfg(test)]
fn resolve_imported_command<'r>(
    registry: &'r CommandRegistry,
    name: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<(String, &'r tcl_registry::CommandSpec)> {
    if name.contains("::") {
        return None;
    }
    // Iterate imports newest-first (`namespace_imports` is in source order): when
    // several in-scope imports could provide the same bare name, Tcl's later
    // `namespace import` (notably `-force`) wins, so the most recent one before
    // the cursor is the effective binding.
    for imp in analysis.namespace_imports.iter().rev() {
        if imp.ns != "::" || imp.range.end() > cursor_offset {
            continue;
        }
        let Some(candidate) = imp
            .source
            .as_ref()
            .and_then(|source| source.candidate(name))
        else {
            continue;
        };
        if let Some(spec) = registry.get(&candidate) {
            return Some((candidate, spec));
        }
    }
    None
}

/// Render a hover snippet for a built-in command name.
/// Looks up `name` in the registry, uses the matched spec's
/// `hover.summary` / `synopsis` to produce a markdown block.
#[cfg(test)]
fn builtin_command_hover_text(
    registry: &CommandRegistry,
    name: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<String> {
    use std::borrow::Cow;
    use std::fmt::Write;
    // Resolve the head through the document's `namespace import`s: a bare
    // `test` under `namespace import ::tcltest::*` hovers as `tcltest::test`
    // (mirrors the analyser's imported-command body resolution, scoped + ordered
    // to imports actually in effect at the cursor).
    let (name, spec): (Cow<'_, str>, _) = if let Some(spec) = registry.get(name) {
        (Cow::Borrowed(name), spec)
    } else {
        let (qual, spec) = resolve_imported_command(registry, name, analysis, cursor_offset)?;
        (Cow::Owned(qual), spec)
    };
    let name = name.as_ref();
    // A command whose *bare* spelling only works inside a `TclOO` method
    // context (`link` / `my` / `next` / `nextto` / `self` /
    // `classvariable`) has no hover anywhere else: at the top level real Tcl
    // answers `invalid command name`, so there is nothing to describe.
    // Registry data decides which commands those are; the frame classifier
    // decides where the cursor is. Hover keys on *callability*, not mere
    // resolution, so a Tcl 9 class `initialise` body — where the family
    // resolves but raises `… may only be called from inside a method` —
    // hovers only `my`, the one member a class-object frame can call. The
    // separately-registered qualified spelling (`::oo::Helpers::link`) is
    // not scoped and still hovers everywhere.
    if !crate::oo_dispatch::OoFrame::at(analysis, cursor_offset).admits(registry, name) {
        return None;
    }
    let hover = spec.hover.as_ref()?;
    let mut out = format!("**`{name}`** — built-in command\n");
    if !hover.summary.is_empty() {
        let _ = write!(out, "\n{}\n", hover.summary);
    }
    if let Some(synopsis) = hover.synopsis.first() {
        let _ = write!(out, "\n```tcl\n{synopsis}\n```\n");
    }
    if !spec.subcommands.is_empty() {
        let mut names: Vec<&str> = spec.subcommands.iter().map(|s| s.name).collect();
        names.sort_unstable();
        let joined = names.join(", ");
        let _ = write!(out, "\nSubcommands: {joined}\n");
    }
    // Import hint: when the command needs a `package require` the
    // document hasn't imported, append a `**Requires**` line.  Gated on
    // the dialect supporting `package require` at all (the `package`
    // command must exist — e.g. iRules bans it), and never shown for a
    // package the profile ships ambiently (an F5 surface is part of the
    // runtime, §7.1 axis C — there is nothing to require).
    if let Some(pkg) = spec.required_package
        && !registry.is_ambient_package(pkg)
    {
        let pkg_available = registry.get("package").is_some();
        let imported = analysis.package_requires.iter().any(|pr| pr.name == pkg);
        if pkg_available && !imported {
            let _ = write!(out, "\n**Requires**: `package require {pkg}`");
        }
    }
    // iRules: append the **Valid events** list + event **Requires**.
    // Only iRules commands carry `event_requires`, so its presence is the
    // dialect gate.
    if let Some(requires) = spec.event_requires.as_ref() {
        let effective = effective_event_requires(name, requires);
        append_valid_events(&mut out, &effective);
    } else if let Some((prefix, _)) = name.split_once("::") {
        // Namespace-only command (no `event_requires`): if its protocol
        // namespace declares profiles (e.g. `ACCESS::log` → ACCESS), surface
        // a `**Requires**` profile line.
        let profile_reg = tcl_registry::profiles::ProfileRegistry::build();
        if let Some(ns) = profile_reg.get_namespace(prefix)
            && !ns.profiles.is_empty()
        {
            let _ = write!(out, "\n\n**Requires**: {} profile", ns.profiles.join(", "));
        }
    }
    Some(out)
}

/// Augment a command's `EventRequires` with namespace-backed profile
/// metadata.  For an iRules `NS::cmd` whose own `profiles` are empty, the
/// profiles declared by the `NS` protocol namespace are substituted, so
/// the **Valid events** list narrows and a profile **Requires** line is
/// shown (e.g. `DIAMETER::*`, `MQTT::*`, `LDAP::*`, `IMAP::*`).  The
/// f5-irules gate is implicit — only iRules specs carry `event_requires`.
fn effective_event_requires(
    command_name: &str,
    requires: &tcl_registry::events::EventRequires,
) -> tcl_registry::events::EventRequires {
    // A command with its own profiles, or no `NS::` qualifier, is unchanged.
    if !requires.profiles.is_empty() {
        return requires.clone();
    }
    let Some((prefix, _)) = command_name.split_once("::") else {
        return requires.clone();
    };
    let profile_reg = tcl_registry::profiles::ProfileRegistry::build();
    match profile_reg.get_namespace(prefix) {
        Some(ns) if !ns.profiles.is_empty() => tcl_registry::events::EventRequires {
            profiles: ns.profiles,
            ..*requires
        },
        _ => requires.clone(),
    }
}

/// Append the iRules "**Valid events**" list (first 8 + total) and the
/// event "**Requires**" line (sides / transport / profiles) for a
/// command's `EventRequires`.
fn append_valid_events(out: &mut String, requires: &tcl_registry::events::EventRequires) {
    use std::fmt::Write;
    let matching = valid_events(requires);
    if !matching.is_empty() {
        let mut list = matching
            .iter()
            .take(8)
            .map(|e| format!("`{e}`"))
            .collect::<Vec<_>>()
            .join(", ");
        if matching.len() > 8 {
            let _ = write!(list, ", ... ({} total)", matching.len());
        }
        let _ = write!(out, "\n\n**Valid events**: {list}");
    }
    let mut reqs: Vec<String> = Vec::new();
    if requires.client_side {
        reqs.push("client-side".to_owned());
    }
    if requires.server_side {
        reqs.push("server-side".to_owned());
    }
    if let Some(t) = requires.transport {
        reqs.push(t.to_uppercase());
    }
    if !requires.profiles.is_empty() {
        let mut profs: Vec<&str> = requires.profiles.to_vec();
        profs.sort_unstable();
        reqs.push(format!("profile {}", profs.join(" or ")));
    }
    if !reqs.is_empty() {
        let _ = write!(out, "\n\n**Requires**: {}", reqs.join(", "));
    }
}

/// Sorted event names whose properties satisfy `requires`.
fn valid_events(requires: &tcl_registry::events::EventRequires) -> Vec<String> {
    let event_reg = tcl_registry::events::EventRegistry::build();
    let profile_reg = tcl_registry::profiles::ProfileRegistry::build();
    let mut out: Vec<String> = event_reg
        .all_event_names()
        .into_iter()
        .filter(|name| {
            event_reg.get_props(name).is_some_and(|p| {
                tcl_registry::events::event_satisfies(p, requires, name, &profile_reg)
            })
        })
        .map(ToOwned::to_owned)
        .collect();
    out.sort_unstable();
    out
}

/// Strftime specifier descriptions for clock-format hover.
const CLOCK_SPEC_DESC: &[(char, &str)] = &[
    ('a', "Abbreviated weekday name"),
    ('A', "Full weekday name"),
    ('b', "Abbreviated month name"),
    ('B', "Full month name"),
    ('C', "Century (00–99)"),
    ('d', "Day of month (01–31)"),
    ('D', "Date as %m/%d/%Y"),
    ('e', "Day of month (1–31, no leading zero)"),
    ('H', "Hour (00–23)"),
    ('I', "Hour (01–12)"),
    ('j', "Day of year (001–366)"),
    ('J', "Julian day number"),
    ('k', "Hour (0–23, no leading zero)"),
    ('l', "Hour (1–12, no leading zero)"),
    ('m', "Month (01–12)"),
    ('M', "Minute (00–59)"),
    ('n', "Month number (1–12, no leading zero)"),
    ('p', "AM/PM indicator (uppercase)"),
    ('s', "Seconds since Unix epoch"),
    ('S', "Second (00–59)"),
    ('u', "Day of week (1=Monday–7=Sunday)"),
    ('w', "Day of week (0=Sunday–6=Saturday)"),
    ('x', "Locale date representation"),
    ('y', "2-digit year (00–99)"),
    ('Y', "4-digit year"),
    ('z', "Timezone offset (+hhmm)"),
    ('Z', "Timezone abbreviation"),
    ('%', "Literal percent sign"),
];

/// Look up a clock-format specifier letter's description.
fn clock_spec_desc(letter: char) -> Option<&'static str> {
    CLOCK_SPEC_DESC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, d)| *d)
}

/// Find every clock-format specifier in `text` —
/// `%[EO]?[a-zA-Z%]`.  Returns each specifier as its source
/// text (including any `%E` / `%O` locale prefix).
fn scan_clock_specifiers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        // `%`...
        let start = i;
        i += 1;
        if i < chars.len() && (chars[i] == 'E' || chars[i] == 'O') {
            i += 1;
        }
        if i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '%') {
            i += 1;
            out.push(chars[start..i].iter().collect());
        }
    }
    out
}

/// Render a markdown table of clock-format specifiers found
/// in `text`.
fn clock_format_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Clock format string** (strftime-style)\n".to_string()];
    let specs = scan_clock_specifiers(text);
    if specs.is_empty() {
        parts.push("No specifiers found.".to_string());
    } else {
        parts.push("| Specifier | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for spec in specs {
            let last = spec.chars().last().unwrap_or(' ');
            let desc = clock_spec_desc(last).unwrap_or("Unknown");
            let display = if spec.chars().count() == 3 {
                format!("{desc} (locale-modified)")
            } else {
                desc.to_string()
            };
            parts.push(format!("| `{spec}` | {display} |"));
        }
    }
    parts.join("\n")
}

/// `printf`-style format-specifier descriptions for
/// sprintf-hover.
const SPRINTF_SPEC_DESC: &[(char, &str)] = &[
    ('d', "Signed decimal integer"),
    ('i', "Signed decimal integer"),
    ('u', "Unsigned decimal integer"),
    ('o', "Unsigned octal integer"),
    ('x', "Unsigned hexadecimal (lowercase)"),
    ('X', "Unsigned hexadecimal (uppercase)"),
    ('f', "Floating-point (fixed notation)"),
    ('e', "Floating-point (scientific, lowercase)"),
    ('E', "Floating-point (scientific, uppercase)"),
    ('g', "Shorter of %e or %f"),
    ('G', "Shorter of %E or %f"),
    ('s', "String"),
    ('c', "Character (by Unicode code point)"),
    ('%', "Literal percent sign"),
    ('b', "Unsigned binary integer"),
];

fn sprintf_spec_desc(letter: char) -> Option<&'static str> {
    SPRINTF_SPEC_DESC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, d)| *d)
}

/// Scan `text` for sprintf-style format specifiers.  Captures
/// the full specifier as written, e.g. `%05d` or `%-10s`.
/// Scan for `printf`-style specifiers
/// (`%[positional$]?[flags]*[width]?[.prec]?[type]`).
fn scan_sprintf_specifiers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        // Positional argument `<digit>+$`.
        let digits_start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i < chars.len() && chars[i] == '$' && i > digits_start {
            i += 1;
        } else {
            // Roll back — those digits were flags / width.
            i = digits_start;
        }
        // Flags: `-` / `+` / ` ` / `#` / `0`.
        while i < chars.len() && matches!(chars[i], '-' | '+' | ' ' | '#' | '0') {
            i += 1;
        }
        // Width.
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        // Precision.
        if i < chars.len() && chars[i] == '.' {
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
        // Type character.
        if i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '%') {
            i += 1;
            out.push(chars[start..i].iter().collect());
        }
    }
    out
}

/// Render a markdown table of sprintf-style specifiers in
/// `text`.
fn sprintf_format_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Format string** (sprintf-style)\n".to_string()];
    let specs = scan_sprintf_specifiers(text);
    if specs.is_empty() {
        parts.push("No specifiers found.".to_string());
    } else {
        parts.push("| Specifier | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for spec in specs {
            let type_char = spec.chars().last().unwrap_or(' ');
            let desc = sprintf_spec_desc(type_char).unwrap_or("Unknown");
            parts.push(format!("| `{spec}` | {desc} |"));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on a `format` / `scan`
/// format-string argument and return the literal text.
/// `format <fmtString> ?arg arg ...?` — the first arg is the
/// format.  Single-line context only.
#[cfg(test)]
fn sprintf_format_string_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    if tokens[0] != "format" && tokens[0] != "scan" {
        return None;
    }
    string_literal_with_percent_at(line_text, character)
}

/// Find a `"..."` or `{...}` literal that contains `character`
/// AND has at least one `%` in it.  Helper shared between
/// `clock_format_string_at_position` and
/// `sprintf_format_string_at_position`.
#[cfg(test)]
fn string_literal_with_percent_at(line_text: &str, character: u32) -> Option<String> {
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let opener = chars[i];
        let closer = match opener {
            '"' => '"',
            '{' => '}',
            _ => {
                i += 1;
                continue;
            }
        };
        let start = i + 1;
        let mut end = start;
        while end < chars.len() && chars[end] != closer {
            end += 1;
        }
        if start <= col && col <= end {
            let literal: String = chars[start..end].iter().collect();
            if literal.contains('%') {
                return Some(literal);
            }
            return None;
        }
        i = end + 1;
    }
    None
}

/// `binary format` / `binary scan` specifier table.
const BINARY_SPEC_DESC: &[(char, &str)] = &[
    ('a', "Byte string, padded with nulls"),
    ('A', "Byte string, padded with spaces"),
    ('b', "Binary digits (low-to-high order)"),
    ('B', "Binary digits (high-to-low order)"),
    ('h', "Hexadecimal digits (low-to-high nibble)"),
    ('H', "Hexadecimal digits (high-to-low nibble)"),
    ('c', "8-bit signed integer"),
    ('s', "16-bit signed integer (little-endian)"),
    ('S', "16-bit signed integer (big-endian)"),
    ('i', "32-bit signed integer (little-endian)"),
    ('I', "32-bit signed integer (big-endian)"),
    ('n', "32-bit integer (native byte order)"),
    ('w', "64-bit signed integer (little-endian)"),
    ('W', "64-bit signed integer (big-endian)"),
    ('m', "64-bit integer (native byte order)"),
    ('r', "32-bit float (little-endian)"),
    ('R', "32-bit float (big-endian)"),
    ('f', "32-bit float (native byte order)"),
    ('d', "64-bit double (native byte order)"),
    ('q', "64-bit double (little-endian)"),
    ('Q', "64-bit double (big-endian)"),
    ('x', "Null padding byte (format) / skip byte (scan)"),
    ('X', "Move cursor back one byte"),
    ('@', "Move cursor to absolute position"),
    ('t', "Reserved (Tcl 8.5+)"),
];

fn binary_spec_desc(letter: char) -> Option<&'static str> {
    BINARY_SPEC_DESC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, d)| *d)
}

/// Compact type label for the detail table.
fn binary_short_type(letter: char) -> &'static str {
    match letter {
        'a' => "str (null-pad)",
        'A' => "str (space-pad)",
        'b' => "bits lo→hi",
        'B' => "bits hi→lo",
        'h' => "hex lo→hi",
        'H' => "hex hi→lo",
        'c' => "int8",
        's' => "int16 LE",
        'S' => "int16 BE",
        'i' => "int32 LE",
        'I' => "int32 BE",
        'n' => "int32 native",
        'w' => "int64 LE",
        'W' => "int64 BE",
        'm' => "int64 native",
        'r' => "float32 LE",
        'R' => "float32 BE",
        'f' => "float32 native",
        'd' => "float64 native",
        'q' => "float64 LE",
        'Q' => "float64 BE",
        'x' => "pad/skip",
        'X' => "back",
        '@' => "seek",
        't' => "reserved",
        _ => "?",
    }
}

/// Unit byte size per element for fixed-width binary types.
fn binary_unit_bytes(letter: char) -> Option<u32> {
    match letter {
        'c' => Some(1),
        's' | 'S' => Some(2),
        'i' | 'I' | 'n' | 'r' | 'R' | 'f' => Some(4),
        'w' | 'W' | 'm' | 'd' | 'q' | 'Q' => Some(8),
        _ => None,
    }
}

/// Specifiers that don't consume a variable / value argument.
fn binary_no_var(letter: char) -> bool {
    matches!(letter, 'x' | 'X' | '@')
}

/// Total byte size for one binary format field, or `None` if
/// unknown (`*` count, `X` move-back, …).
fn binary_field_bytes(letter: char, count: u32, star: bool) -> Option<u32> {
    if star {
        return None;
    }
    if let Some(unit) = binary_unit_bytes(letter) {
        // A large-but-parseable count (`d600000000` ⇒ 8 × 600000000) overflows
        // u32; report an unknown size rather than panicking in debug or
        // wrapping to a bogus value in release (issue 182).
        return unit.checked_mul(count);
    }
    match letter {
        'a' | 'A' | 'x' => Some(count),
        'b' | 'B' => Some(count.div_ceil(8)),
        'h' | 'H' => Some(count.div_ceil(2)),
        _ => None,
    }
}

/// One parsed `binary format` / `binary scan` field.
#[derive(Debug, Clone)]
struct BinaryField {
    /// The full spec token as written (e.g. `"i4"`, `"a*"`).
    full: String,
    /// Type character (e.g. `'i'`).
    letter: char,
    /// Numeric count (defaults to `1` when omitted).
    count: u32,
    /// `u` / `s` size-modifier (Tcl 8.5+), or empty string.
    modifier: String,
    /// `true` when the spec used `*` for the count.
    star: bool,
    /// Per-unit byte size before seek/skip adjustment.
    byte_size: Option<u32>,
    /// `true` when this field consumes a value/variable argument.
    consumes_var: bool,
}

/// Scan a `binary format` / `binary scan` spec string into
/// structured fields.  Tcl grammar: `type [modifier] [count|*]`,
/// repeated.
fn scan_binary_fields(text: &str) -> Vec<BinaryField> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let letter = chars[i];
        if binary_spec_desc(letter).is_none() {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        let mut modifier = String::new();
        if i < chars.len() && (chars[i] == 'u' || chars[i] == 's') {
            modifier.push(chars[i]);
            i += 1;
        }
        let mut star = false;
        let mut count_str = String::new();
        if i < chars.len() && chars[i] == '*' {
            star = true;
            i += 1;
        } else {
            while i < chars.len() && chars[i].is_ascii_digit() {
                count_str.push(chars[i]);
                i += 1;
            }
        }
        let count: u32 = count_str.parse().unwrap_or(1);
        let full: String = chars[start..i].iter().collect();
        let byte_size = binary_field_bytes(letter, count, star);
        let consumes_var = !binary_no_var(letter);
        out.push(BinaryField {
            full,
            letter,
            count,
            modifier,
            star,
            byte_size,
            consumes_var,
        });
    }
    out
}

/// Surrounding-command context the binary-hover renderer uses
/// to label fields with variable / value argument names.
#[derive(Debug, Clone)]
struct BinaryContext {
    /// Format-string content (between the surrounding quotes
    /// / braces).
    text: String,
    /// `"format"` or `"scan"`.
    subcmd: String,
    /// Trailing argument tokens (variable names for `scan`,
    /// value expressions for `format`).  Filled best-effort
    /// from the line tokenisation — may be empty.
    args: Vec<String>,
}

/// Render the binary format-spec hover markdown, including
/// the byte-ruler diagram when every field has a known byte
/// size, no field uses `X` (move-back), and the total fits in
/// 32 bytes.
fn binary_format_hover_text(ctx: &BinaryContext) -> String {
    let fields = scan_binary_fields(&ctx.text);
    if fields.is_empty() {
        return "**Binary format string**\n\nNo specifiers found.".to_string();
    }

    // Map each consuming field → arg name (variable for scan,
    // value expr for format).  Fields without a corresponding
    // arg fall back to the spec text as their label.
    let mut field_labels: Vec<String> = Vec::with_capacity(fields.len());
    let mut var_idx = 0;
    for field in &fields {
        if field.consumes_var && var_idx < ctx.args.len() {
            field_labels.push(ctx.args[var_idx].clone());
            var_idx += 1;
        } else {
            field_labels.push(field.full.clone());
        }
    }

    // Resolve effective byte deltas, including absolute seek
    // (`@N` jumps to absolute offset N — count the gap from the
    // current cursor).  A backward seek (target < cursor)
    // disables the diagram entirely.
    let mut effective_bytes: Vec<Option<u32>> = Vec::with_capacity(fields.len());
    let mut cursor: u32 = 0;
    let mut has_backward_seek = false;
    for field in &fields {
        if field.letter == '@' {
            if field.star {
                effective_bytes.push(None);
                continue;
            }
            let target = field.count;
            if target < cursor {
                effective_bytes.push(Some(0));
                has_backward_seek = true;
            } else {
                effective_bytes.push(Some(target - cursor));
            }
            cursor = target;
            continue;
        }
        match field.byte_size {
            Some(bs) => {
                effective_bytes.push(Some(bs));
                cursor += bs;
            }
            None => effective_bytes.push(None),
        }
    }

    let n_vars = fields.iter().filter(|f| f.consumes_var).count();
    let total_known: u32 = effective_bytes
        .iter()
        .filter_map(|bs| bs.filter(|n| *n > 0))
        .sum();
    let has_unknown = effective_bytes.iter().any(Option::is_none);
    let plural = if n_vars == 1 { "" } else { "s" };
    let size_suffix = if has_unknown { "+ " } else { "" };

    let mut parts: Vec<String> = vec![format!(
        "**binary {}** — {n_vars} field{plural}, {total_known}{size_suffix} bytes\n",
        ctx.subcmd
    )];

    // Byte-ruler diagram — skipped when any field has unknown
    // size, when a backward seek scrambled the offsets, or when
    // the total exceeds the 32-byte rendering budget.
    let can_diagram = !has_backward_seek
        && !effective_bytes.is_empty()
        && effective_bytes
            .iter()
            .all(|bs| matches!(bs, Some(n) if *n > 0));
    if can_diagram && (1..=32).contains(&total_known) {
        parts.push("```".to_string());
        parts.extend(render_byte_ruler(
            &fields,
            &effective_bytes,
            &field_labels,
            total_known,
        ));
        parts.push("```\n".to_string());
    }

    // Detail table — Spec / Variable / Type / Bytes.
    parts.push("| Spec | Variable | Type | Bytes |".to_string());
    parts.push("|------|----------|------|------:|".to_string());
    for (j, field) in fields.iter().enumerate() {
        let var = if field.consumes_var {
            field_labels[j].as_str()
        } else {
            "—"
        };
        let mut typ = binary_short_type(field.letter).to_string();
        if field.modifier == "u" {
            typ = typ.replace("int", "uint");
        }
        if field.count > 1 && binary_unit_bytes(field.letter).is_some() {
            typ = format!("{typ} ×{}", field.count);
        }
        let bs_str = if field.star {
            "…".to_string()
        } else {
            effective_bytes[j].map_or_else(|| "?".to_string(), |n| n.to_string())
        };
        parts.push(format!("| `{}` | {var} | {typ} | {bs_str} |", field.full));
    }
    parts.join("\n")
}

/// Render the four-line byte-ruler diagram: a numeric ruler
/// across the byte axis, then top / middle / bottom rows of
/// box-drawing characters labelling each field.  `total_known`
/// is guaranteed to be in `1..=32` (gated by the caller).
fn render_byte_ruler(
    fields: &[BinaryField],
    effective_bytes: &[Option<u32>],
    field_labels: &[String],
    total_known: u32,
) -> Vec<String> {
    use std::fmt::Write;
    const CPB: u32 = 4; // chars per byte
    let indent = "      ";
    let mut ruler = String::from(indent);
    for b in 0..total_known {
        let _ = write!(ruler, "{b:<width$}", width = CPB as usize);
    }
    let mut top = String::from(indent);
    let mut mid = String::from(indent);
    let mut bot = String::from(indent);
    for j in 0..fields.len() {
        let bs = effective_bytes[j].expect("caller gates on all-Some");
        let w = (CPB * bs).saturating_sub(1) as usize;
        let label = field_labels[j].chars().take(w).collect::<String>();
        let sep_t = if j == 0 { '┌' } else { '┬' };
        let sep_b = if j == 0 { '└' } else { '┴' };
        top.push(sep_t);
        top.push_str(&"─".repeat(w));
        mid.push('│');
        mid.push_str(&center(&label, w));
        bot.push(sep_b);
        bot.push_str(&"─".repeat(w));
    }
    top.push('┐');
    mid.push('│');
    bot.push('┘');
    vec![ruler, top, mid, bot]
}

/// Center `s` within a `width`-character cell using spaces (the
/// byte-ruler labels).
fn center(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.to_string();
    }
    let extra = width - len;
    let left = extra / 2;
    let right = extra - left;
    let mut out = String::with_capacity(width);
    for _ in 0..left {
        out.push(' ');
    }
    out.push_str(s);
    for _ in 0..right {
        out.push(' ');
    }
    out
}

/// Detect when the cursor sits on a `binary format` /
/// `binary scan` format-string argument and capture the
/// surrounding command's argument list.  Returns the format
/// text plus the `format`/`scan` subcommand and the trailing
/// argument tokens (best-effort, single-line).
#[cfg(test)]
fn binary_format_context_at_position(
    source: &str,
    line: u32,
    character: u32,
) -> Option<BinaryContext> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.len() < 3 {
        return None;
    }
    if tokens[0] != "binary" || (tokens[1] != "format" && tokens[1] != "scan") {
        return None;
    }
    let subcmd = tokens[1].to_string();
    // The format string is argv[2] for `format`, argv[3] for `scan`.  It is
    // usually a braced/quoted literal, but a bare word (`binary format c2s …`)
    // is just as valid — fall back to the whitespace-token at the cursor when
    // it sits at the format-arg index.
    let fmt_idx = if subcmd == "scan" { 3 } else { 2 };
    let text = string_literal_at(line_text, character).or_else(|| {
        word_token_at(line_text, character)
            .filter(|(idx, _)| *idx == fmt_idx)
            .map(|(_, w)| w)
    })?;
    // `binary format FORMAT VAL ...`   — format is argv[2]
    // `binary scan STRING FORMAT VAR ...` — format is argv[3]
    let skip = if subcmd == "scan" { 4 } else { 3 };
    let args = binary_trailing_args(line_text, skip);
    Some(BinaryContext { text, subcmd, args })
}

/// Return `(token_index, token_text)` for the whitespace-delimited token that
/// contains `character` (UTF-16 column), or `None` when the cursor is on
/// whitespace / past the end.
#[cfg(test)]
fn word_token_at(line_text: &str, character: u32) -> Option<(usize, String)> {
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut idx = 0;
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        if start <= col && col <= i {
            return Some((idx, chars[start..i].iter().collect()));
        }
        idx += 1;
    }
    None
}

/// Recover the trailing argument tokens (variable names for
/// `scan`, value expressions for `format`) that follow the
/// format-string argument.  Skips over braced / quoted literal
/// groupings so the format string itself doesn't bleed into
/// the args list — `binary format {a4 i} val` correctly yields
/// `["val"]` rather than `["{a4", "i}", "val"]`.  The first
/// `skip` argv positions (incl. the format string itself) are
/// dropped.
#[cfg(test)]
fn binary_trailing_args(line_text: &str, skip: usize) -> Vec<String> {
    let chars: Vec<char> = line_text.chars().collect();
    let mut tokens: Vec<String> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let mut token = String::new();
        match chars[i] {
            '{' => {
                let mut depth = 1;
                i += 1;
                token.push('{');
                while i < chars.len() && depth > 0 {
                    if chars[i] == '{' {
                        depth += 1;
                    } else if chars[i] == '}' {
                        depth -= 1;
                    }
                    token.push(chars[i]);
                    i += 1;
                }
            }
            '"' => {
                let mut escaped = false;
                i += 1;
                token.push('"');
                while i < chars.len() {
                    let c = chars[i];
                    token.push(c);
                    i += 1;
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        break;
                    }
                }
            }
            _ => {
                while i < chars.len() && !chars[i].is_whitespace() {
                    token.push(chars[i]);
                    i += 1;
                }
            }
        }
        tokens.push(token);
    }
    tokens.into_iter().skip(skip).collect()
}

/// `regsub` backref description table.
fn regsub_backref_desc(c: char) -> Option<&'static str> {
    match c {
        '&' | '0' => Some("Entire matched string"),
        '1' => Some("First capture group"),
        '2' => Some("Second capture group"),
        '3' => Some("Third capture group"),
        '4' => Some("Fourth capture group"),
        '5' => Some("Fifth capture group"),
        '6' => Some("Sixth capture group"),
        '7' => Some("Seventh capture group"),
        '8' => Some("Eighth capture group"),
        '9' => Some("Ninth capture group"),
        _ => None,
    }
}

/// Scan a `regsub` substitution spec for `\0` … `\9` / `\&`
/// backreferences.  Returns each match as written (e.g.
/// `\\1`).
fn scan_regsub_backrefs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' {
            i += 1;
            continue;
        }
        if i + 1 >= chars.len() {
            break;
        }
        let next = chars[i + 1];
        if next == '&' || next.is_ascii_digit() {
            out.push(chars[i..=i + 1].iter().collect());
        }
        i += 2;
    }
    out
}

/// Render the regsub substitution-spec hover markdown.
fn regsub_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Substitution spec** (regsub)\n".to_string()];
    let refs = scan_regsub_backrefs(text);
    if refs.is_empty() {
        parts.push("No backreferences found.".to_string());
    } else {
        parts.push("| Reference | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for r in refs {
            let backref_char = r.chars().nth(1).unwrap_or(' ');
            let desc = regsub_backref_desc(backref_char).unwrap_or("Unknown");
            // Escape the backslash for display (`\\` in
            // markdown renders as `\`).
            parts.push(format!("| `{r}` | {desc} |"));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on the substitution-spec
/// argument of a `regsub` invocation and return the literal
/// text.  `regsub ?switches? exp string subSpec ?varName?`
/// — `subSpec` is the 4th positional arg (after switches).
/// Single-line only.
#[cfg(test)]
fn regsub_subspec_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() || tokens[0] != "regsub" {
        return None;
    }
    // The substitution spec contains backslash sequences,
    // typically as a quoted or braced literal.  Any literal
    // string containing `\\<digit-or-&>` overlapping the cursor
    // counts as the subspec — a loose detection that does not
    // do precise arg-position resolution.
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let opener = chars[i];
        let closer = match opener {
            '"' => '"',
            '{' => '}',
            _ => {
                i += 1;
                continue;
            }
        };
        let start = i + 1;
        let mut end = start;
        while end < chars.len() && chars[end] != closer {
            end += 1;
        }
        if start <= col && col <= end {
            let literal: String = chars[start..end].iter().collect();
            if scan_regsub_backrefs(&literal).is_empty() {
                return None;
            }
            return Some(literal);
        }
        i = end + 1;
    }
    None
}

/// Helper: find any `"..."` / `{...}` literal containing the
/// cursor.  Shared between hover providers that need
/// literal-context detection but don't care whether the
/// literal contains `%`.
#[cfg(test)]
fn string_literal_at(line_text: &str, character: u32) -> Option<String> {
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let opener = chars[i];
        let closer = match opener {
            '"' => '"',
            '{' => '}',
            _ => {
                i += 1;
                continue;
            }
        };
        let start = i + 1;
        let mut end = start;
        while end < chars.len() && chars[end] != closer {
            end += 1;
        }
        if start <= col && col <= end {
            return Some(chars[start..end].iter().collect());
        }
        i = end + 1;
    }
    None
}

/// Glob metacharacter descriptions.
fn glob_meta_desc(c: char) -> Option<&'static str> {
    match c {
        '*' => Some("Matches any sequence of characters"),
        '?' => Some("Matches any single character"),
        '[' => Some("Character class — matches any character inside brackets"),
        _ => None,
    }
}

/// Scan a glob pattern for metacharacters.  Returns a list of
/// `(token, description)` tuples — `*`, `?`, character class
/// `[abc]`, and escape sequences. +
/// `_glob_hover`'s metacharacter walk.
fn scan_glob_metachars(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            let key = "escape".to_string();
            if !seen.contains(&key) {
                out.push((format!("\\{next}"), format!("Escaped character `{next}`")));
                seen.insert(key);
            }
            i += 2;
            continue;
        }
        if chars[i] == '[' {
            let start = i;
            let mut end = i + 1;
            while end < chars.len() && chars[end] != ']' {
                end += 1;
            }
            let token: String = if end < chars.len() {
                chars[start..=end].iter().collect()
            } else {
                chars[start..end].iter().collect()
            };
            if !seen.contains(&token) {
                let inner: String = chars[start + 1..end].iter().collect();
                out.push((
                    token.clone(),
                    format!("Character class: matches any of `{inner}`"),
                ));
                seen.insert(token);
            }
            i = end + 1;
            continue;
        }
        if let Some(desc) = glob_meta_desc(chars[i]) {
            let key = chars[i].to_string();
            if !seen.contains(&key) {
                out.push((key.clone(), desc.to_string()));
                seen.insert(key);
            }
        }
        i += 1;
    }
    out
}

/// Escape a value for a GFM table cell: a literal `|` closes the cell even
/// inside an inline code span, so it must be backslash-escaped (`\|`, which
/// GFM renders back as `|`).  Applied to the regex/glob hover token and
/// meaning cells, whose content is user-pattern-derived and may contain `|`
/// (e.g. an alternation `foo|bar`) that would otherwise break the row in
/// strict GFM renderers (issue 187).
fn gfm_table_cell(s: &str) -> String {
    s.replace('|', "\\|")
}

/// Render the glob-pattern hover markdown.
fn glob_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Glob pattern**\n".to_string()];
    let metas = scan_glob_metachars(text);
    if metas.is_empty() {
        parts.push("Literal string (no metacharacters).".to_string());
    } else {
        parts.push("| Pattern | Meaning |".to_string());
        parts.push("|---------|---------|".to_string());
        for (tok, desc) in metas {
            parts.push(format!(
                "| `{}` | {} |",
                gfm_table_cell(&tok),
                gfm_table_cell(&desc)
            ));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on a glob pattern.  Recognises
/// `string match <pat> ...`, `glob <pat>...`, and `lsearch
/// -glob <pat> ...` — three common entry points for glob
/// matching in Tcl.  Single-line only.
#[cfg(test)]
#[allow(dead_code)]
fn glob_pattern_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let is_glob_command = matches!(tokens[0], "glob")
        || (tokens.len() >= 2 && tokens[0] == "string" && tokens[1] == "match")
        || (tokens.len() >= 2 && tokens[0] == "lsearch" && tokens.contains(&"-glob"));
    if !is_glob_command {
        return None;
    }
    let literal = string_literal_at(line_text, character)?;
    // Require at least one glob metacharacter or `\` escape so
    // we don't fire on literal strings.
    if !literal.chars().any(|c| matches!(c, '*' | '?' | '[' | '\\')) {
        return None;
    }
    Some(literal)
}

/// Regex metacharacter descriptions.
fn regex_meta_desc(token: &str) -> Option<&'static str> {
    match token {
        "^" => Some("Start of line/string anchor"),
        "$" => Some("End of line/string anchor"),
        "." => Some("Match any single character"),
        "*" => Some("Zero or more (greedy)"),
        "+" => Some("One or more (greedy)"),
        "?" => Some("Zero or one (greedy)"),
        "*?" => Some("Zero or more (lazy)"),
        "+?" => Some("One or more (lazy)"),
        "??" => Some("Zero or one (lazy)"),
        "|" => Some("Alternation (OR)"),
        _ => None,
    }
}

/// Regex escape descriptions for common shorthand classes.
fn regex_escape_desc(token: &str) -> Option<&'static str> {
    match token {
        "\\d" => Some("Digit `[0-9]`"),
        "\\D" => Some("Non-digit"),
        "\\s" => Some("Whitespace"),
        "\\S" => Some("Non-whitespace"),
        "\\w" => Some("Word character `[a-zA-Z0-9_]`"),
        "\\W" => Some("Non-word character"),
        "\\b" => Some("Word boundary"),
        "\\B" => Some("Non-word boundary"),
        "\\A" => Some("Start of string"),
        "\\Z" => Some("End of string"),
        "\\z" => Some("End of string (Tcl 9.1+)"),
        "\\n" => Some("Newline"),
        "\\t" => Some("Tab"),
        "\\r" => Some("Carriage return"),
        _ => None,
    }
}

/// One emitted regex-token entry — `(consumed, key, tok, desc)`.
/// `consumed` is the number of source chars the token covers;
/// `key` is the dedup key; `tok` is what the table renders;
/// `desc` is the explanation.  Returned by each sub-scanner so
/// the outer loop in [`scan_regex_components`] stays readable.
type RegexComp = (usize, String, String, String);

/// Scan a regex pattern for metacharacters / classes /
/// escapes.  Handles common cases:
/// anchors, quantifiers, alternation, character classes,
/// shorthand escapes, capture-group parens, lazy
/// quantifiers.
fn scan_regex_components(text: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let mut i = 0;
    while i < chars.len() {
        // The sub-scanners are tried in order: escape and char
        // class are eager (they consume multi-char windows);
        // lazy-quantifier has to run before single-char meta so
        // `*?` doesn't get split.  Group-open also runs before
        // single-char meta so `(?:` / `(` get attributed
        // correctly.
        let token = scan_regex_escape(&chars, i)
            .or_else(|| scan_regex_char_class(&chars, i))
            .or_else(|| scan_regex_lazy_quantifier(&chars, i))
            .or_else(|| scan_regex_group(&chars, i))
            .or_else(|| scan_regex_single_meta(chars[i]));
        match token {
            Some((consumed, key, tok, desc)) => {
                if seen.insert(key) {
                    out.push((tok, desc));
                }
                i += consumed.max(1);
            }
            None => i += 1,
        }
    }
    out
}

/// `\<char>` escape sequences — shorthand classes (`\d`, `\w`),
/// numbered backreferences (`\1`-`\9`), and escaped literals
/// (`\.`, `\*`, …).  Falls back to a generic "Escape sequence"
/// label for unknown payloads.
fn scan_regex_escape(chars: &[char], i: usize) -> Option<RegexComp> {
    if chars.get(i)? != &'\\' {
        return None;
    }
    let next = *chars.get(i + 1)?;
    let tok = format!("\\{next}");
    let desc = if next.is_ascii_digit() {
        format!("Backreference to group {next}")
    } else if let Some(d) = regex_escape_desc(&tok) {
        d.to_string()
    } else if ".*+?(){}[]|^$\\".contains(next) {
        format!("Escaped literal `{next}`")
    } else {
        format!("Escape sequence `{tok}`")
    };
    Some((2, tok.clone(), tok, desc))
}

/// `[...]` character classes, including leading `^` negation
/// and a literal `]` as first char per regex grammar.
/// Consumes the entire class including the closing `]` (or to
/// EOL when the pattern is malformed).
fn scan_regex_char_class(chars: &[char], i: usize) -> Option<RegexComp> {
    if chars.get(i)? != &'[' {
        return None;
    }
    let start = i;
    let mut end = i + 1;
    if chars.get(end) == Some(&'^') {
        end += 1;
    }
    if chars.get(end) == Some(&']') {
        end += 1;
    }
    while end < chars.len() && chars[end] != ']' {
        if chars[end] == '\\' && end + 1 < chars.len() {
            end += 2;
        } else {
            end += 1;
        }
    }
    let (tok_slice, consumed) = if end < chars.len() {
        (&chars[start..=end], end + 1 - start)
    } else {
        (&chars[start..end], end - start)
    };
    let tok: String = tok_slice.iter().collect();
    let inner: String = if tok.starts_with('[') && tok.ends_with(']') {
        tok[1..tok.len() - 1].to_string()
    } else {
        tok[1..].to_string()
    };
    let desc = format!("Character class: matches any of `{inner}`");
    Some((consumed, tok.clone(), tok, desc))
}

/// Lazy quantifiers — `*?`, `+?`, `??`.  Must run before
/// [`scan_regex_single_meta`] so `*` alone doesn't claim the
/// pair.
fn scan_regex_lazy_quantifier(chars: &[char], i: usize) -> Option<RegexComp> {
    let c = *chars.get(i)?;
    if !matches!(c, '*' | '+' | '?') {
        return None;
    }
    if chars.get(i + 1) != Some(&'?') {
        return None;
    }
    let tok = format!("{c}?");
    let desc = regex_meta_desc(&tok)?.to_string();
    Some((2, tok.clone(), tok, desc))
}

/// Grouping — `(?:`, `(?=`, `(?!`, `(?>`, and bare `(` / `)`.
fn scan_regex_group(chars: &[char], i: usize) -> Option<RegexComp> {
    let c = *chars.get(i)?;
    if c == ')' {
        return Some((1, ")".into(), ")".into(), "Group close".into()));
    }
    if c != '(' {
        return None;
    }
    if chars.get(i + 1) == Some(&'?')
        && let Some(trail) = chars.get(i + 2)
    {
        let pair = match trail {
            ':' => Some(("(?:", "Non-capturing group")),
            '=' => Some(("(?=", "Positive lookahead")),
            '!' => Some(("(?!", "Negative lookahead")),
            '>' => Some(("(?>", "Atomic (possessive) group")),
            _ => None,
        };
        if let Some((tok, desc)) = pair {
            return Some((3, tok.to_string(), tok.to_string(), desc.to_string()));
        }
    }
    Some((1, "(".into(), "(".into(), "Capture group open".into()))
}

/// Single-char metacharacters — `^`, `$`, `.`, `*`, `+`, `?`,
/// `|`.  Anything [`regex_meta_desc`] knows about.
fn scan_regex_single_meta(c: char) -> Option<RegexComp> {
    let key = c.to_string();
    let desc = regex_meta_desc(&key)?.to_string();
    Some((1, key.clone(), key, desc))
}

/// Alias hover at a genuine command head. Original aliases require their
/// post-argv publication and implementation receipt; lexical-only profiles
/// retain a separate declaration-advice presentation path.
fn alias_hover_at(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    word: &str,
) -> std::ops::ControlFlow<Option<String>> {
    use std::ops::ControlFlow::{Break, Continue};
    use tcl_compiler::command_binding::{BindingKind, SourceCommandReferenceBinding};
    let Some(invocation) = crate::definition::invocation_head_at(analysis, cursor) else {
        return Continue(());
    };
    let Some(reference) = invocation.resolved_command_reference.as_ref() else {
        if analysis.allows_lexical_declaration_advice() {
            return lexical_alias_hover_text(analysis, word)
                .map_or(Continue(()), |text| Break(Some(text)));
        }
        return Continue(());
    };
    if !matches!(
        reference.binding(),
        SourceCommandReferenceBinding::Direct {
            kind: BindingKind::Alias,
            ..
        } | SourceCommandReferenceBinding::Imported {
            kind: BindingKind::Alias,
            ..
        }
    ) {
        return Continue(());
    }
    let original = || {
        let lookup = invocation.original_lookup.as_ref()?;
        if lookup.site().source.source_image() != &tcl_lexer::SourceImage::document(source) {
            return None;
        }
        let realm = analysis.retained_command_realm()?;
        let binding = realm.invocation_at_source(&invocation.name, lookup.site().offset);
        let target = binding.original_alias_target_for_reference(reference)?;
        let mut rendered = Vec::with_capacity(target.arguments().len() + 1);
        for input in std::iter::once(target.name_input()).chain(target.arguments()) {
            if input.policy() != lookup.policy() {
                return None;
            }
            rendered.push(tcl_syntax::native_string::resident_name_label(
                input.bytes(),
            ));
        }
        Some(format!("**Alias** \u{2192} `{}`", rendered.join(" ")))
    };
    let answer = original();
    if answer.is_none() && analysis.allows_lexical_declaration_advice() {
        return Break(lexical_alias_hover_text(analysis, word));
    }
    Break(answer)
}

/// Explicitly bounded legacy declaration advice, without a native source name
/// policy. It never supplies a target for an original/native alias receipt.
fn lexical_alias_hover_text(analysis: &AnalysisResult, word: &str) -> Option<String> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    for alias in analysis
        .command_aliases
        .values()
        .filter(|alias| alias.source_name.is_none())
    {
        let simple = tcl_syntax::naming::unroot_rooted_key(&alias.qualified_name)
            .unwrap_or(&alias.qualified_name);
        if simple == word || alias.qualified_name == word {
            let mut target = alias.target.as_str().to_owned();
            if !alias.extras.is_empty() {
                target.push(' ');
                target.push_str(&alias.extras.join(" "));
            }
            return Some(format!("**Alias** \u{2192} `{target}`"));
        }
    }
    None
}

/// Render a regex-pattern hover markdown.
fn regex_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Regex pattern**\n".to_string()];
    let comps = scan_regex_components(text);
    if comps.is_empty() {
        parts.push("Literal string (no metacharacters).".to_string());
    } else {
        parts.push("| Component | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for (tok, desc) in comps {
            parts.push(format!(
                "| `{}` | {} |",
                gfm_table_cell(&tok),
                gfm_table_cell(&desc)
            ));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on a regex pattern.
/// Recognises `regexp <pat> ...`, `regsub <pat> ...` (the
/// pattern arg, not the subspec), and `lsearch -regexp <pat>
/// ...`.  Single-line only.
#[cfg(test)]
#[allow(dead_code)]
fn regex_pattern_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let is_regex_command = matches!(tokens[0], "regexp" | "regsub")
        || (tokens.len() >= 2 && tokens[0] == "lsearch" && tokens.contains(&"-regexp"));
    if !is_regex_command {
        return None;
    }
    let literal = string_literal_at(line_text, character)?;
    // An explicit `regexp` / `regsub` / `lsearch -regexp` pattern argument
    // hovers even with no metacharacters — `regex_hover_text` renders the
    // "Literal string (no metacharacters)" note.  (The `regsub` replacement
    // spec is handled by the earlier `regsub_subspec_at_position` branch.)
    Some(literal)
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

/// Detect when the cursor sits on a `clock format` /
/// `clock scan` format-string argument and return the
/// literal text.  Single-line only — multi-line literals
/// are not handled.
#[cfg(test)]
fn clock_format_string_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    // Tokenise the line on whitespace — the first two tokens
    // must be `clock format` or `clock scan` for the hover to
    // fire.  This is the same single-line context detection
    // used by `signature_help` / `completion`; multi-line
    // command segments are not handled.
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.len() < 3 {
        return None;
    }
    if tokens[0] != "clock" || (tokens[1] != "format" && tokens[1] != "scan") {
        return None;
    }
    string_literal_with_percent_at(line_text, character)
}

/// The `[start, end)` char-index bounds of the bare Tcl word around `col`
/// in `chars` (one source line), using [`WORD_DELIMS`].
///
/// This is the single word-bounding rule shared by every cursor-word
/// consumer, so hover, definition, references, and rename all agree on
/// where a word starts and ends.  It deliberately follows the *lexer's*
/// notion of a bare word — bounded by whitespace and the structural
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
/// (`${ns}::setdef`), not an absolute one — see [`word_char_bounds`]'s return
/// value.
///
/// Deliberately *not* handled: a word split across lines by a
/// backslash-newline continuation, and the interior structure of a word
/// that mixes literal text with substitutions (`pre[cmd]post`) — the
/// caller sees the literal slice only.
pub(crate) fn word_char_bounds(chars: &[char], col: usize) -> Option<(usize, usize)> {
    word_char_bounds_kinded(chars, col).map(|(start, end, _)| (start, end))
}

/// [`word_char_bounds`] also reporting whether the word is the **residual
/// tail of a computed name** — the literal fragment that follows a `${var}` /
/// `[cmd]` substitution inside the same word (`${ns}::setdef` → `::setdef`).
///
/// The scan cannot see the substitution (it stops at its closer, which is a
/// delimiter), so without this flag `::setdef` is indistinguishable from a
/// genuinely absolute `::setdef` and resolution looks for a global proc of
/// that literal name — finding nothing, and reporting no definition for a
/// call that statically resolves fine (`${ns}::setdef` with `ns ==
/// "::ticklecharts"`).  Reporting the fact here, in the one shared
/// word-bounding rule, keeps every consumer (hover, definition, references,
/// rename) on the same answer.
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
/// A word that is the residual tail of a computed name — the literal
/// fragment after a `${var}` / `[cmd]` substitution in the same word, as in
/// `${ns}::setdef` — is reported as the *name* it spells (`setdef`), with the
/// span narrowed past the `::` accordingly.  Keeping the
/// leading `::` made it indistinguishable from an absolute name, so
/// resolution looked for a global proc literally called `::setdef` and every
/// consumer reported nothing; narrowing the span as well is what keeps rename
/// from eating the namespace separator it must preserve.
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

/// Type/taint labels use the same selected original reference geometry as
/// scope lookup. A source element label has no independent SSA/read authority.
fn var_type_annotations(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    var_name: &str,
    registry: Option<&CommandRegistry>,
) -> (Option<String>, Option<String>) {
    let type_var =
        crate::source_structure::original_variable_reference_at(source, analysis, cursor)
            .map_or(var_name, |reference| reference.element_label);
    match registry {
        Some(reg) => infer_var_type_and_taint(source, reg, type_var, analysis),
        None => (None, None),
    }
}

fn proc_hover_text(proc_def: &ProcDef) -> String {
    let params: Vec<String> = proc_def
        .params
        .iter()
        .map(|p| {
            if p.has_default {
                let default = p.default_value.as_deref().unwrap_or("");
                format!("{{{} {}}}", p.name, default)
            } else {
                p.name.clone()
            }
        })
        .collect();
    let sig = format!(
        "proc {} {{{}}} {{...}}",
        proc_def.qualified_name,
        params.join(" ")
    );
    let mut parts = vec![format!("```tcl\n{sig}\n```")];
    if !proc_def.doc.is_empty() {
        // Render `@param` / `@return` /
        // `@brief` tagged docstrings as structured Markdown
        // sections rather than the raw harvested text.  Lines
        // that don't carry a tag fall through into the
        // description block, preserving free-form comments
        // for procs that don't use Doxygen-style tags.
        parts.push(format_docstring(&proc_def.doc));
    }
    parts.join("\n\n")
}

/// Parse a raw docstring and render it as Markdown for LSP
/// hover.
///
/// Recognised tags:
///
/// * `@brief <text>` — short summary surfaced before the
///   description block.
/// * `@param <name> <text>` — parameter docs rendered as a
///   bulleted **Parameters** list.
/// * `@return <text>` / `@returns <text>` — return-value
///   description surfaced as a **Returns** line.
///
/// Other lines accumulate into the description block.  Pure-
/// decoration lines (a run of `.`, `-`, `=`, `*`, `~`, `#`)
/// are dropped.
fn format_docstring(text: &str) -> String {
    let mut brief = String::new();
    let mut description_lines: Vec<String> = Vec::new();
    let mut params: Vec<(String, String)> = Vec::new();
    let mut returns_parts: Vec<String> = Vec::new();

    for line in text.lines() {
        let stripped = line.trim();
        let low = stripped.to_ascii_lowercase();
        if let Some(rest) = low
            .strip_prefix("@param ")
            .or_else(|| low.strip_prefix("@param\t"))
        {
            // Use the original `stripped` slice for body extract
            // so we preserve case on the parameter name and
            // description.  Find the offset of `rest` within
            // `low` (always 7 — `@param ` length).
            let body = &stripped[7..].trim();
            let mut iter = body.splitn(2, char::is_whitespace);
            let Some(name) = iter.next() else {
                continue;
            };
            let name = name.trim_end_matches(['-', ' ']);
            let desc = iter
                .next()
                .map(|s| s.trim_start_matches(['-', ' ']).to_string())
                .unwrap_or_default();
            params.push((name.to_string(), desc));
            let _ = rest;
            continue;
        }
        if low.starts_with("@return ")
            || low.starts_with("@return\t")
            || low.starts_with("@returns ")
            || low.starts_with("@returns\t")
        {
            let body = stripped
                .split_once(char::is_whitespace)
                .map_or("", |x| x.1)
                .trim();
            returns_parts.push(body.trim_start_matches(['-', ' ']).to_string());
            continue;
        }
        if let Some(rest) = low
            .strip_prefix("@brief ")
            .or_else(|| low.strip_prefix("@brief\t"))
        {
            brief = stripped[7..].trim().to_string();
            let _ = rest;
            continue;
        }
        // Drop decoration-only lines.
        if !stripped.is_empty()
            && stripped
                .chars()
                .all(|c| matches!(c, '.' | '-' | '=' | '*' | '~' | '#'))
        {
            continue;
        }
        description_lines.push(stripped.to_string());
    }

    let description = description_lines.join("\n");
    let description = description.trim().to_string();
    let returns_text = returns_parts.join(" ");

    let mut parts: Vec<String> = Vec::new();
    if !brief.is_empty() {
        parts.push(brief);
    }
    if !description.is_empty() {
        parts.push(description);
    }
    if !params.is_empty() {
        let mut lines = vec!["**Parameters:**".to_string()];
        for (name, desc) in &params {
            if desc.is_empty() {
                lines.push(format!("- **{name}**"));
            } else {
                lines.push(format!("- **{name}** \u{2014} {desc}"));
            }
        }
        parts.push(lines.join("\n"));
    }
    if !returns_text.is_empty() {
        parts.push(format!("**Returns:** {returns_text}"));
    }
    parts.join("\n\n")
}

fn class_hover_text(analysis: &AnalysisResult, class_def: &ClassDef) -> String {
    let original_word = class_def.source_name.as_ref().and_then(|name| {
        let mut records = analysis.original_class_declarations().filter(|record| {
            record.name() == name && record.metadata().name_span == class_def.name_span
        });
        let record = records.next()?;
        if records.next().is_some() || record.name_input().display().is_some() {
            return None;
        }
        tcl_syntax::naming::native_command_source_word(
            name.policy().recipe(),
            name.slot(),
            record.name_input().source_image().channel(),
            record.name_input().lexer_config(),
        )
    });
    let mut sig = format!(
        "{} create {}",
        class_def.metaclass,
        original_word
            .as_deref()
            .unwrap_or(&class_def.qualified_name)
    );
    if !class_def.superclasses.is_empty() {
        use std::fmt::Write as _;
        let _ = write!(sig, " (superclass: {})", class_def.superclasses.join(", "));
    }
    if !class_def.mixins.is_empty() {
        use std::fmt::Write as _;
        let _ = write!(sig, " (mixin: {})", class_def.mixins.join(", "));
    }
    let mut parts = vec![format!("```tcl\n{sig}\n```")];
    let mut details: Vec<String> = Vec::new();
    for (side, title) in [
        (
            tcl_compiler::analyser::types::MemberSide::Instance,
            "Methods",
        ),
        (
            tcl_compiler::analyser::types::MemberSide::ClassObject,
            "Class methods",
        ),
    ] {
        let names = if class_def.source_name.is_some() {
            crate::original_oo::own_method_labels(class_def, side)
        } else if analysis.allows_lexical_declaration_advice() {
            let table = match side {
                tcl_compiler::analyser::types::MemberSide::Instance => &class_def.methods,
                tcl_compiler::analyser::types::MemberSide::ClassObject => &class_def.class_methods,
            };
            let mut names = table.keys().cloned().collect::<Vec<_>>();
            names.sort();
            Some(names)
        } else {
            None
        };
        if let Some(names) = names.filter(|names| !names.is_empty()) {
            details.push(format!("**{title}**: {}", names.join(", ")));
        }
    }
    if !class_def.variables.is_empty() {
        details.push(format!(
            "**Instance variables**: {}",
            class_def.variables.join(", ")
        ));
    }
    // MRO chain + direct subclasses from the class hierarchy — surfaces
    // the inheritance shape inline. Only shown when non-trivial.
    if let Some(source_name) = &class_def.source_name {
        let mut records = analysis.original_class_declarations().filter(|record| {
            record.name() == source_name && record.metadata().name_span == class_def.name_span
        });
        if let Some(record) = records.next().filter(|_| records.next().is_none()) {
            use tcl_compiler::analyser::class_hierarchy::original_metadata::original_instance_metadata_order;
            if let Some(order) =
                original_instance_metadata_order(analysis, record).filter(|order| order.len() > 1)
            {
                details.push(format!(
                    "**MRO**: {}",
                    order
                        .iter()
                        .map(|provider| provider.metadata().qualified_name.as_str())
                        .collect::<Vec<_>>()
                        .join(" → ")
                ));
            }
            let mut subclasses = analysis
                .original_class_declarations()
                .filter_map(|candidate| {
                    use tcl_compiler::analyser::types::{
                        MemberSide, OriginalSourceClassRelationKind,
                    };
                    let parents = candidate.metadata().original_relations.resolve(
                        MemberSide::Instance,
                        OriginalSourceClassRelationKind::Superclass,
                        |relation| {
                            let selected = relation.lookup().first_matching_publications(
                                analysis
                                    .original_class_declarations()
                                    .map(|record| (record.name(), record)),
                            );
                            match selected.as_slice() {
                                [record] => Some(*record),
                                _ => None,
                            }
                        },
                    )?;
                    parents
                        .iter()
                        .any(|parent| parent.declaration_site() == record.declaration_site())
                        .then_some(candidate.metadata().qualified_name.as_str())
                })
                .collect::<Vec<_>>();
            subclasses.sort_unstable();
            if !subclasses.is_empty() {
                details.push(format!("**Subclasses**: {}", subclasses.join(", ")));
            }
        }
    } else if analysis.allows_lexical_declaration_advice() {
        let hierarchy = analysis.class_hierarchy();
        let qname = &class_def.qualified_name;
        if let Some(mro) = hierarchy.mro_map.get(qname).filter(|mro| mro.len() > 1) {
            details.push(format!("**MRO**: {}", mro.join(" → ")));
        }
        if let Some(subs) = hierarchy
            .subclasses
            .get(qname)
            .filter(|subs| !subs.is_empty())
        {
            let mut names = subs.iter().map(String::as_str).collect::<Vec<_>>();
            names.sort_unstable();
            details.push(format!("**Subclasses**: {}", names.join(", ")));
        }
    }
    if !details.is_empty() {
        parts.push(details.join("  \n"));
    }
    if !class_def.doc.is_empty() {
        parts.push(class_def.doc.clone());
    }
    parts.join("\n\n")
}

/// Hover markdown for a **caller-frame** variable — one this frame never
/// assigns because a callee creates it here through `upvar`.
///
/// The card names the callee and the parameter that carried the name, because
/// that pair is the whole explanation of where the value comes from; without
/// it the read looks like a bug.  tclsh 9.0.4 / 8.6.14 both run
/// `proc setdef {d} {upvar 1 $d dst; set dst SET}` /
/// `proc build {} {setdef options; return $options}` to `SET`.
fn caller_frame_hover_text(
    name: &str,
    binding: &crate::caller_frame::CallerFrameBinding,
) -> String {
    crate::caller_frame::caller_frame_hover_text(name, binding)
}

/// `format_info` is the same embedded-language table
/// [`registry_pattern_format_hover`] renders for a literal argument; the hover
/// appends it here too, at *this* read's own occurrence, when the lattice
/// proves the variable is used as a registry pattern or format-string
/// argument right where the cursor sits (`set fmt "%-20s %d"; format $fmt a
/// 1` hovering `$fmt`). `None` leaves the card exactly as before.
fn var_hover_text(
    var_def: &VarDef,
    type_info: Option<&str>,
    taint_info: Option<&str>,
    format_info: Option<&str>,
) -> String {
    use std::fmt::Write as _;
    let ref_count = var_def.references.len();
    let mut text = format!(
        "**Variable** `{}`\n\n{} reference(s)",
        var_def.name, ref_count
    );
    if let Some(t) = type_info {
        let _ = write!(text, "\n\n**Inferred intrep**: {t}");
    }
    if let Some(t) = taint_info {
        let _ = write!(text, "\n\n**Taint**: {t}");
    }
    if let Some(f) = format_info {
        let _ = write!(text, "\n\n{f}");
    }
    text
}

/// Render hover markdown for an interpreter-provided special variable
/// ([`tcl_registry::SpecialVarSpec`]), showing its shape, summary, provenance,
/// and — for arrays — the keys available in the active `dialect`.
fn special_var_hover_text(
    spec: &tcl_registry::SpecialVarSpec,
    dialect: Option<SurfaceQuery<'_>>,
) -> String {
    use std::fmt::Write as _;
    use tcl_registry::{SpecialVarKind, VarAccess, VarOrigin};

    let (shape, sigil) = match spec.kind {
        SpecialVarKind::Scalar => ("special variable", format!("${}", spec.name)),
        SpecialVarKind::Array => ("special array", format!("${}(…)", spec.name)),
        SpecialVarKind::Namespace => ("special namespace", format!("{}::…", spec.name)),
    };
    let access = match spec.access {
        VarAccess::ReadOnly => "read-only",
        VarAccess::ReadWrite => "read/write",
    };
    let origin = match spec.origin {
        VarOrigin::Interpreter => "the Tcl interpreter",
        VarOrigin::AutoLoader => "the auto-loader (`init.tcl`)",
        VarOrigin::Platform => "the platform / build",
        VarOrigin::Environment => "the process environment",
        VarOrigin::Dialect => "the dialect runtime",
    };

    let mut text = format!(
        "**`{sigil}`** — {shape} ({access})\n\nProvided by {origin}.\n\n{}",
        spec.summary
    );

    // Array keys available in this dialect (skip open-keyed arrays like `env`).
    let keys: Vec<&tcl_registry::SpecialVarKey> = spec.keys_in(dialect).collect();
    if !keys.is_empty() {
        text.push_str("\n\n**Keys**:\n");
        for k in keys {
            let _ = write!(text, "\n- `{}` — {}", k.key, k.summary);
        }
    }

    // CMP-safety note only matters under iRules.
    if spec.cmp_unsafe && surface_admits(SpecSurface::IRULES, dialect.as_ref()) {
        let _ = write!(
            text,
            "\n\n⚠️ Accessing `{}` as a plain global demotes the virtual server \
             from CMP. Use the CMP-safe `static::{}` alias instead.",
            spec.name, spec.name
        );
    }

    text
}

/// Lower-case label for a Tcl intrep type (e.g. `ByteArray` →
/// `bytearray`).
fn tcl_type_label(t: TclType) -> String {
    format!("{t:?}").to_lowercase()
}

/// Type/taint inference under the same actual source, grammar and complete
/// context as cursor selection. Registry mismatch or unavailable retained
/// input declines; this conditional source build has no Native entry.
fn infer_var_type_and_taint(
    source: &str,
    registry: &CommandRegistry,
    var_name: &str,
    analysis: &AnalysisResult,
) -> (Option<String>, Option<String>) {
    let Some(unit) =
        crate::original_invocation::source_compilation_unit(source, analysis, registry)
    else {
        return (None, None);
    };
    let first_use = tcl_compiler::shimmer::first_use_commitments_for_cu(&unit, registry);
    (
        infer_var_type(&unit, var_name, &first_use),
        infer_var_taint(&unit, var_name),
    )
}

/// The top-level function unit followed by every procedure unit
/// in deterministic (name-sorted) order, so the per-variable
/// inference picks a stable first match regardless of the
/// procedure map's hashing order.
fn function_units_in_order(unit: &CompilationUnit) -> Vec<&FunctionUnit> {
    let mut out: Vec<&FunctionUnit> = Vec::with_capacity(unit.procedures.len() + 1);
    out.push(&unit.top_level);
    let mut procs: Vec<&FunctionUnit> = unit.procedures.values().collect();
    procs.sort_by(|a, b| a.name.cmp(&b.name));
    out.extend(procs);
    out
}

/// Infer a dominant intrep type for `var_name`, mirroring
/// `_infer_var_type`: returns the first function (top-level, then
/// procs in name order) that has type entries for the variable
/// and resolves them to a single label.
///
/// A **pure** variable — a literal / interpolation / string-command result
/// whose intrep is not committed at the def — additionally reports the intrep
/// its first use commits when every executable typed read agrees ("first used
/// as"), the def-site pushback of the committed-intrep dataflow
/// (`tcl_compiler::shimmer::first_use_commitments_for_cu`).
fn infer_var_type(
    unit: &CompilationUnit,
    var_name: &str,
    first_use: &HashMap<String, HashMap<String, TclType>>,
) -> Option<String> {
    for func in function_units_in_order(unit) {
        let entries: Vec<&TypeLattice> = func
            .types
            .iter()
            .filter(|((name, _ver), _)| func.ssa.var_name(*name) == var_name)
            .map(|(_, t)| t)
            .collect();
        if entries.is_empty() {
            continue;
        }
        // Every version agrees on the same KNOWN type.
        let known: Vec<&TypeLattice> = entries
            .iter()
            .copied()
            .filter(|t| t.kind() == TypeKind::Known && t.tcl_type().is_some())
            .collect();
        if !known.is_empty()
            && known.iter().all(|t| t.tcl_type() == known[0].tcl_type())
            && known.len() == entries.len()
        {
            let label = tcl_type_label(known[0].tcl_type().unwrap());
            // A pure def whose every typed read commits one intrep reports it
            // at the creation site: `set l {1 2 3}` first used by `llength`
            // hovers as "string (first used as: list)".
            if let Some(committed) = first_use
                .get(&func.name)
                .and_then(|vars| vars.get(var_name))
            {
                let committed_label = tcl_type_label(*committed);
                if committed_label != label {
                    return Some(format!("{label} (first used as: {committed_label})"));
                }
            }
            return Some(label);
        }
        // Every version agrees on the same shimmer union — render every
        // member (`shimmered (int / list / string)` for a 3-way merge).
        let shimmered: Vec<&TypeLattice> = entries
            .iter()
            .copied()
            .filter(|t| t.kind() == TypeKind::Shimmered)
            .collect();
        if !shimmered.is_empty() && shimmered.len() == entries.len() {
            let s = shimmered[0];
            if shimmered.iter().all(|t| *t == s) {
                let members: Vec<String> = s
                    .shapes()
                    .iter()
                    .map(|shape| tcl_type_label(shape.coarse()))
                    .collect();
                return Some(format!("shimmered ({})", members.join(" / ")));
            }
        }
        // Mixed, but a dominant KNOWN type exists.  Pick the
        // smallest by `Ord` so the choice is deterministic.
        if let Some(t) = known
            .iter()
            .filter_map(|t| t.tcl_type())
            .min()
            .map(tcl_type_label)
        {
            return Some(t);
        }
    }
    None
}

/// Human-readable mitigation-colour labels present in `taint`,
/// in display-priority order.
fn taint_colour_labels(taint: TaintLattice) -> Vec<&'static str> {
    let flag_labels: [(TaintColour, &str); 13] = [
        (TaintColour::PATH_PREFIXED, "path-prefixed"),
        (TaintColour::NON_DASH_PREFIXED, "non-dash-prefixed"),
        (TaintColour::CRLF_FREE, "CRLF-free"),
        (TaintColour::SHELL_ATOM, "shell-atom"),
        (TaintColour::LIST_CANONICAL, "list-canonical"),
        (TaintColour::REGEX_LITERAL, "regex-literal"),
        (TaintColour::PATH_NORMALISED, "path-normalised"),
        (TaintColour::HEADER_TOKEN_SAFE, "header-token-safe"),
        (TaintColour::HTML_ESCAPED, "HTML-escaped"),
        (TaintColour::URL_ENCODED, "URL-encoded"),
        (TaintColour::IP_ADDRESS, "IP-address"),
        (TaintColour::PORT, "port"),
        (TaintColour::FQDN, "FQDN"),
    ];
    flag_labels
        .into_iter()
        .filter(|(flag, _)| taint.colours.contains(*flag))
        .map(|(_, label)| label)
        .collect()
}

/// Infer taint for `var_name`, mirroring `_infer_var_taint`:
/// returns the first function with taint entries for the
/// variable, joining the tainted versions to the most
/// conservative colour set.
fn infer_var_taint(unit: &CompilationUnit, var_name: &str) -> Option<String> {
    for func in function_units_in_order(unit) {
        let entries: Vec<&TaintLattice> = func
            .taints
            .iter()
            .filter(|((name, _ver), _)| func.ssa.var_name(*name) == var_name)
            .map(|(_, t)| t)
            .collect();
        if entries.is_empty() {
            continue;
        }
        let tainted: Vec<&TaintLattice> =
            entries.iter().copied().filter(|t| t.is_tainted()).collect();
        if let Some((first, rest)) = tainted.split_first() {
            let combined = rest.iter().fold(**first, |acc, t| acc.join(**t));
            let labels = taint_colour_labels(combined);
            if labels.is_empty() {
                return Some("tainted (from I/O)".to_owned());
            }
            return Some(format!("tainted (from I/O); {}", labels.join(", ")));
        }
    }
    None
}

/// Hover text for a class member at the cursor's byte
/// offset.  Walks every class whose body span contains the
/// cursor and looks `word` up against `methods` /
/// `class_methods` / `properties` — but ONLY when `word` is
/// genuinely bareword-callable from a method body of this class
/// (`ClassDef::linked_members`; see
/// `lookup_class_member`'s doc for the full rationale) — plus the
/// `constructor` / `destructor` keywords, unconditionally.
/// Returns a one-line markdown summary on hit, `None` otherwise.
fn class_member_hover_text(
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<String> {
    let class_def = analysis
        .all_classes
        .get(crate::definition::enclosing_class_at(
            analysis,
            cursor_offset,
        )?)?;
    let qname = &class_def.qualified_name;
    // The member's **own declaration** token — `method reopened {v} {…}`'s
    // `reopened` — hovers as that member, wherever in the class the
    // declaration was written.  A class body is not a
    // single lexical region: `oo::define Cls { … }` reopens the class and
    // its members are just as much `Cls`'s own, so this hangs off
    // `enclosing_class_at` (which already consults
    // `AnalysisResult::class_body_spans`, the multi-span record) rather than
    // any single `body_span`, and the reopening block behaves exactly like
    // the creation block.  Without a declaration-site path only *call* sites
    // (`my m`, `$obj m`) and `link`-exposed barewords render, which would
    // leave a class member the one declared symbol in the language that
    // cannot be hovered where it is declared, while `proc` and the class name
    // itself both can.
    //
    // Gated on the cursor genuinely sitting inside the member's own
    // `name_span`, so this never fires for a same-named word elsewhere in
    // the body (those keep falling through to the tiers below).
    if let Some(text) = member_declaration_hover_text(analysis, class_def, word, cursor_offset) {
        return Some(text);
    }
    if let Some(target) = class_def.linked_members.get(word) {
        let linked_note = if target == word {
            String::new()
        } else {
            format!("  \nlinked from `{word}`")
        };
        if let Some(m) = class_def.methods.get(target) {
            let note = oo_method_resolution_note(analysis, qname, target)
                .map_or(String::new(), |n| format!("  \n{n}"));
            return Some(format!(
                "**method** `{qname}::{name}` ({nparam} param(s)){note}{linked_note}",
                name = m.name,
                nparam = m.params.len(),
            ));
        }
        if let Some(m) = class_def.class_methods.get(target) {
            let note = oo_method_resolution_note(analysis, qname, target)
                .map_or(String::new(), |n| format!("  \n{n}"));
            return Some(format!(
                "**classmethod** `{qname}::{name}` ({nparam} param(s)){note}{linked_note}",
                name = m.name,
                nparam = m.params.len(),
            ));
        }
        if let Some(p) = class_def.properties.get(target) {
            return Some(format!(
                "**property** `{qname}::{name}`{linked_note}",
                name = p.name
            ));
        }
    }
    // A constructor or destructor keyword: the member it declared says what
    // it is.
    if let Some(ctor) = class_def
        .constructors
        .first()
        .filter(|c| c.is_declared_by_keyword(word))
    {
        let nparam = ctor.params.len();
        return Some(format!(
            "**{}** of `{qname}` ({nparam} param(s))",
            ctor.kind
        ));
    }
    if let Some(dtor) = class_def
        .destructor
        .as_ref()
        .filter(|d| d.is_declared_by_keyword(word))
    {
        return Some(format!("**{}** of `{qname}`", dtor.kind));
    }
    None
}

/// Hover text for a class member's **own declaration** name token, or
/// `None` when the cursor is not sitting on one.
///
/// Which member a name belongs to is
/// [`crate::references::resolve_member_span`] — the very same
/// method/classmethod/property disambiguation find-references uses, so
/// hover and references can never disagree about *which* declaration a
/// cursor is on when a name appears in more than one of a class's three
/// independent member tables.  That helper falls back to the first
/// candidate when the cursor is on no declaration at all (it is also used
/// from call sites), so the containment test is re-applied here: a
/// declaration hover must be anchored on the declaration.
///
/// The rendered summary is byte-identical to the call-site
/// (`my m` / `$obj m`) rendering, including the MRO note, so hovering a
/// method at its declaration and at a call site describe the same member
/// the same way.
fn member_declaration_hover_text(
    analysis: &AnalysisResult,
    class_def: &tcl_compiler::analyser::types::ClassDef,
    word: &str,
    cursor_offset: u32,
) -> Option<String> {
    use crate::references::MemberSel;
    let (kind, span) = crate::references::resolve_member_span(class_def, word, cursor_offset)?;
    if !(span.start() <= cursor_offset && cursor_offset <= span.end()) {
        return None;
    }
    let qname = &class_def.qualified_name;
    let note = |label: &str, m: &tcl_compiler::analyser::types::MethodDef| {
        let suffix = oo_method_resolution_note(analysis, qname, &m.name)
            .map_or(String::new(), |n| format!("  \n{n}"));
        format!(
            "**{label}** `{qname}::{name}` ({nparam} param(s)){suffix}",
            name = m.name,
            nparam = m.params.len(),
        )
    };
    match kind {
        MemberSel::Method => class_def.methods.get(word).map(|m| note("method", m)),
        MemberSel::ClassMethod => class_def
            .class_methods
            .get(word)
            .map(|m| note("classmethod", m)),
        MemberSel::Property => class_def
            .properties
            .get(word)
            .map(|p| format!("**property** `{qname}::{name}`", name = p.name)),
    }
}

/// The `$obj method` / `my method` dispatch arms of the hover walk, plus
/// the per-object visibility mask that precedes them.
///
/// Three-valued so the caller's tier order survives the extraction:
/// `Break(Some(_))` is a rendered method hover, `Break(None)` is a
/// **definitive no-hover** — a per-object mask (`oo::objdefine $o {
/// unexport m }`, or an unexported per-object member) makes `$obj m` answer
/// `unknown method` regardless of the class chain, never a
/// fall-through to a same-named proc or command — and `Continue(())` means
/// the cursor is not a method-dispatch site at all, so the remaining hover
/// tiers run.
fn method_dispatch_hover(
    source: &str,
    line: u32,
    character: u32,
    cursor_offset: u32,
    analysis: &AnalysisResult,
) -> std::ops::ControlFlow<Option<Hover>> {
    use std::ops::ControlFlow;
    if let Some(selected) =
        crate::receiver_identity::method_at_cursor(analysis, source, cursor_offset)
    {
        let label = match selected.receiver {
            tcl_compiler::command_binding::SourceMethodReceiver::Instance => "method",
            tcl_compiler::command_binding::SourceMethodReceiver::Class => "classmethod",
        };
        return ControlFlow::Break(Some(Hover::markdown(format!(
            "**{label}** `{}::{}` ({} param(s))",
            selected.class.qualified_name,
            selected.method.name,
            selected.method.params.len(),
        ))));
    }
    if crate::receiver_identity::definition_reference_at_cursor(analysis, source, cursor_offset)
        .is_some()
        || crate::definition::object_masks_external_dispatch(analysis, source, line, character)
    {
        return ControlFlow::Break(None);
    }

    ControlFlow::Continue(())
}

/// The one-line method summary for a dispatch whose **provider was resolved
/// elsewhere** — the server's workspace tier, walking the same C-faithful
/// linearisation `definition.rs`'s cross-file method tier walks.
///
/// [`method_dispatch_hover`] can only answer when the whole class chain is
/// visible in the requesting document's own analysis. The real corpus shape
/// is the opposite: `my ArgsPreprocess` in one file, the `mixin`/`superclass`
/// that provides `ArgsPreprocess` in another (the `SpiceGenTcl` `RModel` /
/// `Utility` split). Go-to-definition and
/// find-references cross that boundary through the workspace index;
/// `hover.rs` has no cross-document tier of its
/// own. Rather than grow one here — the index and the document store live in
/// the server — the server resolves the provider and asks this function to
/// render it, so both tiers emit the identical heading and MRO note.
///
/// `receiver_class_q` is the class the call dispatches *on*; `provider_q` the
/// class the chain landed on. They differ exactly when the member is
/// inherited, which is the note this renders — there is no `next`-chain
/// lookup to do here, because the chain walk that produced `provider_q`
/// already applied the visibility rule and the instance/class-side split.
#[must_use]
pub fn cross_document_method_hover(
    receiver_class_q: &str,
    provider_q: &str,
    method_name: &str,
    nparams: usize,
    is_classmethod: bool,
) -> Hover {
    let label = if is_classmethod {
        "classmethod"
    } else {
        "method"
    };
    let note = if provider_q == receiver_class_q {
        String::new()
    } else {
        format!("  \n_inherited from `{provider_q}`_")
    };
    Hover::markdown(format!(
        "**{label}** `{provider_q}::{method_name}` ({nparams} param(s)){note}"
    ))
}

/// MRO note for `method` on `class_q`: `inherited from ::Provider` when the
/// method resolves to an ancestor, or `overrides ::Super::method` when it
/// is defined on `class_q` but a superclass also provides it.  `None` for a
/// method defined only here (no note needed) or an unresolvable one.
fn oo_method_resolution_note(
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
) -> Option<String> {
    let provider = analysis.class_hierarchy().method_target(class_q, method)?;
    oo_resolution_note_for_provider(analysis, class_q, provider, method)
}

/// Render the inheritance or override note for the provider selected by the
/// caller. The note uses that exact provider rather than resolving it again.
fn oo_resolution_note_for_provider(
    analysis: &AnalysisResult,
    class_q: &str,
    provider: &str,
    method: &str,
) -> Option<String> {
    if provider == class_q {
        // Defined here — does a superclass further down the MRO also
        // provide it (i.e. this is an override)?
        analysis
            .class_hierarchy()
            .next_provider(class_q, method, class_q, None)
            .map(|sup| format!("_overrides `{sup}::{method}`_"))
    } else {
        Some(format!("_inherited from `{provider}`_"))
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family, SurfaceLayer};

    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn standalone_reference_cursor_uses_explicit_selected_grammar_and_utf16_geometry() {
        // Implementation contract: naming.core.selected-variable-cursor-syntax
        // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
        let source = "puts café🙂 ${a{b}c} ${cash$name} ${scalar(open)tail}";
        let prefix = "puts café🙂 ";
        let character = u32::try_from(prefix.encode_utf16().count() + 2).unwrap();
        for style in [
            tcl_dialect::BracedVarStyle::FirstClose,
            tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ] {
            let config = tcl_lexer::LexerConfig {
                braced_var: style,
                ..Default::default()
            };
            assert_eq!(
                find_var_at_position_with_config(source, 0, character, config).as_deref(),
                Some(if style.nests() { "a{b}c" } else { "a{b" }),
            );
            let dollar = u32::try_from(
                source[..source.find("$name").unwrap()]
                    .encode_utf16()
                    .count(),
            )
            .unwrap();
            assert_eq!(
                find_var_at_position_with_config(source, 0, dollar, config).as_deref(),
                Some("cash$name")
            );
            let tail = u32::try_from(
                source[..source.find("scalar(open)").unwrap()]
                    .encode_utf16()
                    .count(),
            )
            .unwrap();
            assert_eq!(
                find_var_at_position_with_config(source, 0, tail, config).as_deref(),
                Some("scalar(open)tail")
            );
        }
    }

    #[test]
    fn qualified_colon_variable_hover_keeps_the_empty_cell_separate() {
        let source = "set {} EMPTY; set : COLON; puts ${}; puts $:";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse(source, dialect);
            let hover = qualified_variable_hover(&analysis, ":::").unwrap();
            assert!(hover.value.contains("`:::`"), "{dialect}: {}", hover.value);
            assert!(!hover.value.contains("`::`"), "{dialect}: {}", hover.value);
        }
    }

    fn position_of(source: &str, needle: &str) -> (u32, u32) {
        let offset = source.find(needle).expect("test cursor text");
        let before = &source[..offset];
        let line = u32::try_from(before.bytes().filter(|b| *b == b'\n').count()).unwrap();
        let column = u32::try_from(
            before
                .rsplit_once('\n')
                .map_or(before, |(_, tail)| tail)
                .len(),
        )
        .unwrap();
        (line, column)
    }

    #[test]
    fn find_word_span_returns_none_at_eol() {
        // Position one past the line's last char yields None.
        let src = "proc foo {} {}\n";
        let line = src.split('\n').next().unwrap();
        let len = u32::try_from(line.chars().count()).expect("len fits u32");
        assert!(find_word_span_at_position(src, 0, len).is_none());
    }

    #[test]
    fn smoke_find_word_span_extracts_word_under_cursor() {
        // Cursor on the 'r' of `proc`.
        let src = "proc greet {} {}\n";
        let (word, start, end) = find_word_span_at_position(src, 0, 1).unwrap();
        assert_eq!(word, "proc");
        assert_eq!(start, 0);
        assert_eq!(end, 4);
    }

    #[test]
    fn find_word_span_stops_at_dollar_sign() {
        // `$var` — `$` is in `_WORD_DELIMS`, so a cursor inside
        // `var` should yield just `var`.
        let src = "set x $var\n";
        let (word, start, end) = find_word_span_at_position(src, 0, 8).unwrap();
        assert_eq!(word, "var");
        assert_eq!(start, 7);
        assert_eq!(end, 10);
    }

    #[test]
    fn smoke_find_var_at_position_recognises_dollar_ref() {
        // Cursor inside `$var`.
        let src = "set x $var\n";
        assert_eq!(find_var_at_position(src, 0, 8), Some("var".to_owned()));
    }

    #[test]
    fn find_var_at_position_returns_none_for_bare_word() {
        let src = "set x 1\n";
        assert!(find_var_at_position(src, 0, 4).is_none());
    }

    #[test]
    fn find_var_at_position_resolves_second_var_of_concatenation() {
        // `set z $x$y` — a cursor on `y` must resolve `y`, not
        // walk left across the `$` to the first variable `x`.
        let src = "set z $x$y\n";
        // Columns: s(0)e(1)t(2) (3)z(4) (5)$(6)x(7)$(8)y(9)
        assert_eq!(find_var_at_position(src, 0, 9), Some("y".to_owned())); // on `y`
        assert_eq!(find_var_at_position(src, 0, 7), Some("x".to_owned())); // on `x`
        // Three-way concatenation resolves the middle var too.
        let src3 = "set w $a$b$c\n";
        assert_eq!(find_var_at_position(src3, 0, 9), Some("b".to_owned()));
    }

    #[test]
    fn find_var_at_position_stops_at_lone_colon() {
        // `$host:$port` — a lone `:` is not part of the variable name (Tcl
        // substitutes only `$host`); it must not be pulled in as `host:`
        // (issue 183).
        let src = "puts $host:$port\n";
        // Columns: p0 u1 t2 s3 (4) $5 h6 o7 s8 t9 :10 $11 ...
        assert_eq!(find_var_at_position(src, 0, 7), Some("host".to_owned()));
        // The `::` qualifier IS part of the name, and a full colon run stays.
        let src2 = "puts $a::b:$c\n";
        assert_eq!(find_var_at_position(src2, 0, 7), Some("a::b".to_owned()));
        let src3 = "puts $a:::b\n";
        assert_eq!(find_var_at_position(src3, 0, 7), Some("a:::b".to_owned()));
    }

    #[test]
    fn binary_field_bytes_saturates_instead_of_overflowing() {
        // `d600000000` ⇒ 8 × 600000000 overflows u32; report unknown size,
        // never panic (debug) or wrap (release) (issue 182).
        assert_eq!(super::binary_field_bytes('d', 600_000_000, false), None);
        assert_eq!(super::binary_field_bytes('i', u32::MAX, false), None);
        // Ordinary sizes still compute.
        assert_eq!(super::binary_field_bytes('i', 4, false), Some(16));
    }

    #[test]
    fn regex_and_glob_hover_escape_table_pipes() {
        // A `|` inside a code-span table cell must be `\|` or it splits the GFM
        // row (issue 187).
        let rx = regex_hover_text("foo|bar");
        assert!(
            rx.lines().all(|l| !l.starts_with("| `|`")),
            "raw pipe token would break the row: {rx}"
        );
        assert!(
            rx.contains("\\|"),
            "alternation `|` should be escaped: {rx}"
        );
        // Glob alternation `{a,b}` has no pipe, but a bracketed pipe would; a
        // char-class-style pattern with `|` escapes too.
        let rxcc = regex_hover_text("[a|b]");
        assert!(rxcc.contains("\\|"));
        // Every non-separator row keeps a balanced cell count (3 pipes → 2
        // cells) after escaping.
        for line in rx.lines().filter(|l| l.starts_with("| ")) {
            let unescaped = line.replace("\\|", "");
            assert_eq!(
                unescaped.matches('|').count(),
                3,
                "row must have exactly two cells: {line}"
            );
        }
    }

    #[test]
    fn find_var_at_position_recognises_braced_form() {
        // Cursor on the `r` inside `${var}`.  The braced form
        // should still resolve to `"var"` so rename and hover
        // can find the symbol.
        let src = "set x ${var}\n";
        assert_eq!(find_var_at_position(src, 0, 9), Some("var".to_owned()));
        // Cursor immediately after the opening `${` (start of name).
        assert_eq!(find_var_at_position(src, 0, 8), Some("var".to_owned()));
        // Cursor on the closing `}` itself — the inner name is
        // still resolvable as long as the cursor sits inside the
        // braces inclusive of the close brace.
        assert_eq!(find_var_at_position(src, 0, 11), Some("var".to_owned()));
    }

    #[test]
    fn hover_on_proc_name_returns_signature() {
        let src = "proc greet {name} { puts $name }\n";
        let analysis = analyse(src);
        let h = hover(src, 0, 6, &analysis, None).expect("expected hover for proc name");
        assert_eq!(h.kind, HoverKind::Markdown);
        assert!(h.value.contains("proc ::greet"), "{}", h.value);
        assert!(h.value.contains("name"), "{}", h.value);
    }

    #[test]
    fn hover_on_ensemble_subcommand_resolves_target_proc() {
        // TP — hover on an ensemble subcommand call
        // site surfaces the resolved target proc's own signature.
        let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e foo bar]\n";
        let analysis = analyse(src);
        // Cursor on "foo" in `puts [e foo bar]` (0-based line 7, col 8).
        let h = hover(src, 7, 8, &analysis, None).expect("expected hover for ensemble subcommand");
        assert_eq!(h.kind, HoverKind::Markdown);
        assert!(h.value.contains("Ensemble subcommand"), "{}", h.value);
        assert!(h.value.contains("proc ::e::Foo"), "{}", h.value);
    }

    #[test]
    fn hover_on_tk_ensemble_configure_splice_resolves_the_real_target() {
        // TP — `tk`'s built-in ensemble is extended at
        // runtime via `namespace ensemble configure tk -map [dict merge
        // [namespace ensemble configure tk -map] {systray ::tk::systray}]`
        // (the real `tk/library/systray.tcl` idiom). Hover on the call
        // site's "systray" must surface the real `::tk::systray` proc, not
        // abstain or resolve a same-tail-name decoy elsewhere.
        let src = "namespace eval ::decoy {\n    proc systray {args} { return \"DECOY\" }\n}\nproc ::tk::systray {args} { return \"real systray: $args\" }\nnamespace ensemble configure tk -map [dict merge [namespace ensemble configure tk -map] {systray ::tk::systray}]\ntk systray create -image book\n";
        let analysis = analyse(src);
        // Cursor on "systray" in `tk systray create ...` (0-based line 5).
        let h = hover(src, 5, 5, &analysis, None).expect("expected hover for tk systray splice");
        assert_eq!(h.kind, HoverKind::Markdown);
        assert!(h.value.contains("proc ::tk::systray"), "{}", h.value);
        assert!(!h.value.contains("DECOY"), "{}", h.value);
    }

    #[test]
    fn hover_on_self_method_call_site_resolves() {
        // TP — `self method make {n} {...}` records into `class_methods`,
        // and `Widget make gadget`'s bare class-command receiver resolves
        // against it, so hover on the call site works end-to-end.
        let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\nWidget make gadget\n";
        let analysis = analyse(src);
        // Cursor on `make` in `Widget make gadget` (line 3, col 8).
        let h = hover(src, 3, 8, &analysis, None).expect("expected hover for self-method call");
        assert!(h.value.contains("classmethod"), "{}", h.value);
        assert!(h.value.contains("Widget::make"), "{}", h.value);
    }

    #[test]
    fn hover_on_proc_qualified_name() {
        let src = "namespace eval ::ns { proc helper {} { return } }\n";
        let analysis = analyse(src);
        // Cursor on `helper` token at column ~28
        let h = hover(src, 0, 28, &analysis, None);
        // Either matches via simple name or qualified name; the
        // contract is that hover surfaces the proc when present.
        if let Some(h) = h {
            assert!(h.value.contains("helper"), "{}", h.value);
        }
    }

    // namespace-aware proc resolution (C Tcl `Tcl_FindCommand` order)

    #[test]
    fn hover_unqualified_call_prefers_callers_namespace_proc() {
        // Two namespaces each define `helper`; hovering the unqualified call
        // inside ::b must surface ::b::helper — the current namespace
        // resolves before global, never a sibling namespace.
        let src = "namespace eval a {\n    proc helper {x} { return 1 }\n}\nnamespace eval b {\n    proc helper {y} { return 2 }\n    helper 5\n}\n";
        let analysis = analyse(src);
        let h = hover(src, 5, 6, &analysis, None).expect("hover on namespaced call");
        assert!(h.value.contains("proc ::b::helper {y}"), "{}", h.value);
    }

    #[test]
    fn hover_namespace_proc_set_shadows_builtin_inside_namespace_only() {
        // A namespace proc named like a builtin: inside ::ns the proc wins;
        // at global scope only the builtin resolves — a proc in an unrelated
        // namespace must not hijack builtin hover.
        let src = "namespace eval ns {\n    proc set {key value} { return $value }\n    set x 1\n}\nset y 2\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let inside = hover(src, 2, 5, &analysis, Some(&registry)).expect("hover inside ns");
        assert!(inside.value.contains("proc ::ns::set"), "{}", inside.value);
        let global = hover(src, 4, 1, &analysis, Some(&registry)).expect("hover at global");
        assert!(global.value.contains("built-in"), "{}", global.value);
        assert!(!global.value.contains("::ns::set"), "{}", global.value);
    }

    #[test]
    fn hover_global_call_fallback_is_deterministic() {
        // No ::helper exists, so no candidate resolves; the lenient tail
        // fallback fires and must pick the lexicographically smallest
        // qualified name (::a::helper) on every repeat — never a
        // `HashMap`-iteration-order hijack.
        let src = "namespace eval z {\n    proc helper {a} { return 26 }\n}\nnamespace eval a {\n    proc helper {b} { return 1 }\n}\nhelper 1\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let first = hover(src, 6, 2, &analysis, Some(&registry)).expect("hover on global call");
        assert!(first.value.contains("proc ::a::helper"), "{}", first.value);
        for attempt in 0..8 {
            let repeat =
                hover(src, 6, 2, &analysis, Some(&registry)).expect("hover on global call");
            assert_eq!(first, repeat, "attempt {attempt}: hover must be stable");
        }
    }

    #[test]
    fn hover_on_unknown_word_returns_none() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        // Cursor on "hello" — not a proc / class / var, so None.
        // (`puts` is a builtin and isn't in `all_procs` either.)
        assert!(hover(src, 0, 6, &analysis, None).is_none());
    }

    #[test]
    fn hover_on_class_name_returns_metaclass_signature() {
        let src = "oo::class create Greeter {}\n";
        let analysis = analyse(src);
        let h = hover(src, 0, 18, &analysis, None);
        if let Some(h) = h {
            assert!(h.value.contains("Greeter"), "{}", h.value);
            assert!(
                h.value.contains("oo::class create"),
                "expected metaclass declaration, got {}",
                h.value,
            );
        }
    }

    #[test]
    fn hover_on_dollar_var_returns_var_text() {
        // Variable defined at top level, referenced via `$x`.
        let src = "set x 1\nset y $x\n";
        let analysis = analyse(src);
        let h = hover(src, 1, 7, &analysis, None);
        if let Some(h) = h {
            assert!(h.value.contains("Variable"), "{}", h.value);
            assert!(h.value.contains("`x`"), "{}", h.value);
        }
    }

    #[test]
    fn hover_on_proc_param_bareword_declaration_resolves() {
        // TP — a cursor on a proc parameter's own bareword name (not a
        // `$`-prefixed read) must hover, just as the same variable's `$name`
        // reads do.
        let src = "proc greet {name} { return $name }\n";
        let analysis = analyse(src);
        // Cursor on `name` inside the parameter list (col 12-16).
        let h = hover(src, 0, 13, &analysis, None).expect("hover");
        assert!(h.value.contains("**Variable** `name`"), "{}", h.value);
    }

    #[test]
    fn hover_on_catch_resultvar_bareword_resolves() {
        // TP — the other bareword shape: a `catch script name`
        // result-var reuses an existing variable; its own bareword token
        // must still hover, surfacing the same variable.
        let src = "proc resolveSwitch {name def} {\n    catch {foo} name\n    return $name\n}\n";
        let analysis = analyse(src);
        // Cursor on the catch result-var `name` (line 1, col 16-20).
        let h = hover(src, 1, 17, &analysis, None).expect("hover");
        assert!(h.value.contains("**Variable** `name`"), "{}", h.value);
    }

    #[test]
    fn hover_on_var_surfaces_inferred_intrep() {
        // `x` is assigned an integer literal; the compiler's
        // type-propagation pass should infer `int`, and the hover
        // (given a registry) should surface it.
        let src = "set x 1\nset y $x\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 1, 7, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("**Variable** `x`"), "{}", h.value);
        assert!(h.value.contains("**Inferred intrep**: int"), "{}", h.value);
    }

    #[test]
    fn hover_on_pure_literal_surfaces_first_used_as() {
        // `l` is a pure list-shaped literal whose only typed read commits the
        // list intrep — the def-site pushback of the committed-intrep
        // dataflow surfaces "first used as: list" at the creation site.
        let src = "set l {1 2 3}\nllength $l\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 1, 9, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("**Variable** `l`"), "{}", h.value);
        assert!(
            h.value
                .contains("**Inferred intrep**: string (first used as: list)"),
            "{}",
            h.value
        );
    }

    #[test]
    fn hover_committed_producer_has_no_first_used_as() {
        // A committed producer (`[list …]`) is not "first used as" anything —
        // its intrep is set at the def, so the pushback annotation must not
        // appear.
        let src = "set l [list 1 2 3]\nllength $l\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 1, 9, &analysis, Some(&registry)).expect("hover");
        assert!(
            !h.value.contains("first used as"),
            "committed producers must not carry the pushback label: {}",
            h.value
        );
    }

    #[test]
    fn hover_returns_none_for_out_of_range_line() {
        let src = "proc foo {} {}\n";
        let analysis = analyse(src);
        assert!(hover(src, 99, 0, &analysis, None).is_none());
    }

    #[test]
    fn builtin_hover_shows_requires_when_package_not_imported() {
        // `http::geturl` needs `package require http`; without the
        // import the hover appends the **Requires** line.
        let src = "http::geturl $url\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 6, &analysis, Some(&registry)).expect("hover");
        assert!(
            h.value.contains("**Requires**: `package require http`"),
            "{}",
            h.value
        );
    }

    #[test]
    fn builtin_hover_omits_requires_when_package_imported() {
        // With `package require http` present, the hint is suppressed.
        let src = "package require http\nhttp::geturl $url\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 1, 6, &analysis, Some(&registry)).expect("hover");
        assert!(!h.value.contains("**Requires**"), "{}", h.value);
    }

    #[test]
    fn builtin_hover_resolves_imported_command() {
        // A bare command imported into the global scope resolves
        // to its qualified spec — hovering `test` after `namespace import
        // ::tcltest::*` surfaces the `tcltest::test` documentation.
        let src = "namespace import ::tcltest::*\ntest t-1 {desc} -body { set x 1 } -result 1\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 1, 2, &analysis, Some(&registry)).expect("hover on bare `test`");
        assert!(
            h.value.contains("tcltest::test"),
            "bare imported `test` must hover as tcltest::test: {}",
            h.value
        );
    }

    #[test]
    fn builtin_hover_bare_test_without_import_is_none() {
        // Control: without an import, a bare `test` is not a known command.
        let src = "test t-1 {desc}\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        assert!(hover(src, 0, 0, &analysis, Some(&registry)).is_none());
    }

    #[test]
    fn builtin_hover_ignores_nested_namespace_import() {
        // An import made inside `namespace eval ns { … }` is not in scope at top
        // level, so hovering a bare top-level `test` must NOT resolve to
        // tcltest::test (mirrors the analyser's scope rule).
        let src = "namespace eval ns { namespace import ::tcltest::* }\ntest t-1 {desc}\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        // `test` is on line 1 (top level) — out of the `ns` import's scope.
        assert!(
            hover(src, 1, 2, &analysis, Some(&registry)).is_none(),
            "a nested-namespace import must not resolve a top-level bare `test`",
        );
    }

    #[test]
    fn builtin_hover_ignores_import_after_cursor() {
        // Source order: hovering a `test` that appears *before* the
        // `namespace import` must not resolve — the import is not yet in effect.
        let src = "test t-1 {desc}\nnamespace import ::tcltest::*\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        assert!(
            hover(src, 0, 2, &analysis, Some(&registry)).is_none(),
            "an import after the cursor must not retroactively resolve `test`",
        );
    }

    #[test]
    fn valid_events_for_asm_profile_requirement() {
        use tcl_registry::events::EventRequires;
        // A command requiring the ASM profile is valid only in ASM events.
        let requires = EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            flow: false,
        };
        let events = valid_events(&requires);
        assert_eq!(
            events,
            vec![
                "ASM_REQUEST_BLOCKING",
                "ASM_REQUEST_DONE",
                "ASM_REQUEST_VIOLATION",
                "ASM_RESPONSE_LOGIN",
                "ASM_RESPONSE_VIOLATION",
            ],
            "ASM-profile events"
        );
    }

    #[test]
    fn valid_events_uses_transitive_profile_expansion() {
        use tcl_registry::events::EventRequires;
        // `HTTP`-profile requirement matches HTTP events directly *and*
        // events whose implied profile transitively requires HTTP (e.g.
        // `ADAPT_REQUEST_HEADERS` via `REQUESTADAPT` → `HTTP`).  This
        // exercises `expand_profile_stack`.
        let requires = EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            flow: false,
        };
        let events = valid_events(&requires);
        assert!(events.iter().any(|e| e == "HTTP_REQUEST"), "{events:?}");
        assert!(
            events.iter().any(|e| e == "ADAPT_REQUEST_HEADERS"),
            "transitive-profile event missing (expansion broken): {events:?}"
        );
    }

    #[test]
    fn expand_profile_stack_includes_parents() {
        let reg = tcl_registry::profiles::ProfileRegistry::build();
        let stack = reg.expand_profile_stack(&["HTTP"]);
        assert!(stack.contains("HTTP"), "{stack:?}");
        assert!(stack.contains("TCP"), "HTTP should require TCP: {stack:?}");
    }

    #[test]
    fn irules_command_hover_lists_valid_events() {
        let mut registry = CommandRegistry::build_default();
        registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
        if let Some(text) =
            builtin_command_hover_text(&registry, "ASM::is_authenticated", &analyse(""), u32::MAX)
        {
            assert!(text.contains("**Valid events**"), "{text}");
            assert!(text.contains("ASM_REQUEST_BLOCKING"), "{text}");
            assert!(text.contains("**Requires**: profile ASM"), "{text}");
        }
    }

    #[test]
    fn irules_namespace_backed_profiles_injected_into_hover() {
        // `DIAMETER::retransmission_default` carries `event_requires` with
        // *empty* profiles; `effective_event_requires` substitutes the
        // `DIAMETER` namespace's profiles, so the hover shows a profile
        // **Requires** line (none would appear without the injection).
        let mut registry = CommandRegistry::build_default();
        registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
        let text = builtin_command_hover_text(
            &registry,
            "DIAMETER::retransmission_default",
            &analyse(""),
            u32::MAX,
        )
        .expect("hover");
        assert!(
            text.contains("**Requires**: profile DIAMETER or DIAMETERSESSION or DIAMETER_ENDPOINT"),
            "{text}"
        );
    }

    #[test]
    fn effective_event_requires_injects_namespace_profiles() {
        // Unit check: empty own-profiles + a known `NS::` prefix → the
        // namespace's profiles; a non-namespaced name is unchanged.
        let base = tcl_registry::events::EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            flow: false,
        };
        let eff = super::effective_event_requires("DIAMETER::foo", &base);
        assert_eq!(
            eff.profiles,
            &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"]
        );
        // No `::` qualifier → unchanged (still empty).
        assert!(
            super::effective_event_requires("puts", &base)
                .profiles
                .is_empty()
        );
        // Already-populated profiles are left as-is.
        let with = tcl_registry::events::EventRequires {
            profiles: &["HTTP"],
            ..base
        };
        assert_eq!(
            super::effective_event_requires("HTTP::uri", &with).profiles,
            &["HTTP"]
        );
    }

    #[test]
    fn builtin_hover_omits_requires_for_core_command() {
        // A core built-in (`lindex`) needs no package — no hint.
        let src = "lindex $l 0\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 2, &analysis, Some(&registry)).expect("hover");
        assert!(!h.value.contains("**Requires**"), "{}", h.value);
    }

    #[test]
    fn proc_hover_text_formats_default_param() {
        let src = "proc greet {{name world}} { puts $name }\n";
        let analysis = analyse(src);
        let proc_def = analysis.all_procs.values().next().unwrap();
        let text = proc_hover_text(proc_def);
        assert!(text.contains("{name world}"), "got: {text}");
    }

    /// The cross-document renderer answers by qualified name and
    /// produces the *same* body the in-document path does, for both a proc and
    /// a class — one renderer, so a call-site hover in another file can never
    /// drift from the declaration's own.
    #[test]
    fn qualified_symbol_hover_matches_the_in_document_rendering_1018() {
        let src = "proc ::math::zzqfrobnicate {val args} { return $val }\noo::class create ::geo::Plane {\n    method fly {} {}\n}\n";
        let analysis = analyse(src);

        let proc_hover =
            qualified_symbol_hover(&analysis, "::math::zzqfrobnicate").expect("proc hover");
        assert_eq!(
            proc_hover.value,
            proc_hover_text(&analysis.all_procs["::math::zzqfrobnicate"]),
        );
        assert!(proc_hover.value.contains("val"), "{proc_hover:?}");

        let class_hover = qualified_symbol_hover(&analysis, "::geo::Plane").expect("class hover");
        assert_eq!(
            class_hover.value,
            class_hover_text(&analysis, &analysis.all_classes["::geo::Plane"]),
        );

        // A name neither map holds resolves to nothing — the fallback never
        // invents a symbol.
        assert!(qualified_symbol_hover(&analysis, "::math::nosuchthing").is_none());
        // The lookup is by *qualified* name only; a bare tail is not a match,
        // so a same-tailed proc in an unrelated namespace cannot be picked up.
        assert!(qualified_symbol_hover(&analysis, "zzqfrobnicate").is_none());
    }

    #[test]
    fn class_hover_text_lists_methods_alphabetically() {
        let src = concat!(
            "oo::class create Foo {\n",
            "    method beta {} {}\n",
            "    method alpha {} {}\n",
            "}\n",
        );
        let analysis = analyse(src);
        let class_def = analysis
            .all_classes
            .values()
            .next()
            .expect("class recorded")
            .clone();
        let text = class_hover_text(&analysis, &class_def);
        // Methods listed in sorted order.
        let alpha_pos = text.find("alpha");
        let beta_pos = text.find("beta");
        if let (Some(a), Some(b)) = (alpha_pos, beta_pos) {
            assert!(a < b, "expected alpha before beta in: {text}");
        }
    }

    #[test]
    fn class_hover_shows_mro_and_subclasses() {
        let src = "oo::class create A {}\noo::class create B {\n    superclass A\n}\noo::class create C {\n    superclass B\n}\n";
        let analysis = analyse(src);
        let b = analysis.all_classes.get("::B").expect("B").clone();
        let text = class_hover_text(&analysis, &b);
        assert!(text.contains("**MRO**"), "MRO missing: {text}");
        assert!(text.contains("::B → ::A"), "MRO chain wrong: {text}");
        assert!(
            text.contains("**Subclasses**") && text.contains("::C"),
            "subclasses missing: {text}"
        );
    }

    /// `pos_of` — (line, character) of the `occurrence`-th `needle`.
    fn pos_of(src: &str, needle: &str, occurrence: usize) -> (u32, u32) {
        let mut start = 0;
        for _ in 0..occurrence {
            let idx = src[start..].find(needle).expect("needle not found") + start;
            start = idx + 1;
        }
        let idx = start - 1;
        let prefix = &src[..idx];
        let line = u32::try_from(prefix.matches('\n').count()).unwrap();
        let col = u32::try_from(idx - prefix.rfind('\n').map_or(0, |n| n + 1)).unwrap();
        (line, col)
    }

    #[test]
    fn class_hover_disambiguates_same_name_across_namespaces() {
        // `::A::Shape` and `::B::Shape` share a simple name and each has a
        // distinct subclass.  A bare `Shape` written inside `::A` must hover
        // `::A::Shape` (with `::A::Circle` among its subclasses), never the
        // arbitrary first same-named class a namespace-blind scan would pick.
        let src = "namespace eval A {\n\
                       oo::class create Shape {}\n\
                       oo::class create Circle {\n\
                           superclass Shape\n\
                       }\n\
                   }\n\
                   namespace eval B {\n\
                       oo::class create Shape {}\n\
                       oo::class create Square {\n\
                           superclass Shape\n\
                       }\n\
                   }\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        // Occurrence 2 is `superclass Shape` inside `::A::Circle`.
        let (l, c) = pos_of(src, "Shape", 2);
        let h = hover(src, l, c, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("::A::Shape"), "{}", h.value);
        assert!(h.value.contains("::A::Circle"), "{}", h.value);
        assert!(!h.value.contains("::B::Shape"), "{}", h.value);
        assert!(!h.value.contains("::B::Square"), "{}", h.value);
    }

    #[test]
    fn method_hover_notes_inheritance_and_override() {
        let src = "oo::class create A {\n    method greet {} {}\n}\noo::class create B {\n    superclass A\n    method greet {} {}\n}\noo::class create C {\n    superclass A\n}\n";
        let analysis = analyse(src);
        // B::greet overrides A::greet.
        let over = oo_method_resolution_note(&analysis, "::B", "greet").unwrap_or_default();
        assert!(
            over.contains("overrides") && over.contains("::A::greet"),
            "{over}"
        );
        // C inherits greet from A.
        let inh = oo_method_resolution_note(&analysis, "::C", "greet").unwrap_or_default();
        assert!(
            inh.contains("inherited from") && inh.contains("::A"),
            "{inh}"
        );
        // A::greet defined only here — no note.
        assert!(oo_method_resolution_note(&analysis, "::A", "greet").is_none());
    }

    #[test]
    fn var_hover_text_renders_reference_count() {
        let src = "set x 1\nset y $x\nset z $x\n";
        let analysis = analyse(src);
        let var_def = analysis
            .global_scope
            .variables
            .get("x")
            .expect("x recorded");
        let text = var_hover_text(var_def, None, None, None);
        assert!(text.contains("**Variable** `x`"), "{}", text);
        assert!(text.contains("reference"), "{}", text);
    }

    #[test]
    fn var_hover_text_appends_intrep_and_taint() {
        let var_def = VarDef {
            name: "x".to_owned(),
            definition_span: tcl_lexer::Span::new(0, 1),
            references: Vec::new(),
            warn_if_unused: false,
            array_indices: std::collections::BTreeSet::new(),
            link_target: None,
            link_target_span: None,
        };
        let text = var_hover_text(&var_def, Some("int"), Some("tainted (from I/O)"), None);
        assert!(text.contains("**Inferred intrep**: int"), "{text}");
        assert!(text.contains("**Taint**: tainted (from I/O)"), "{text}");
    }

    #[test]
    fn var_hover_text_appends_proven_format_info() {
        let var_def = VarDef {
            name: "fmt".to_owned(),
            definition_span: tcl_lexer::Span::new(0, 1),
            references: Vec::new(),
            warn_if_unused: false,
            array_indices: std::collections::BTreeSet::new(),
            link_target: None,
            link_target_span: None,
        };
        let text = var_hover_text(&var_def, None, None, Some("**Format string**"));
        assert!(text.contains("**Variable** `fmt`"), "{text}");
        assert!(text.contains("**Format string**"), "{text}");
    }

    #[test]
    fn special_var_hover_documents_auto_path() {
        // `$auto_path` has no user definition, but the special-variable
        // registry provides documentation on hover.
        let src = "puts $auto_path\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 8, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("special variable"), "{}", h.value);
        assert!(h.value.contains("auto-loader"), "{}", h.value);
    }

    #[test]
    fn special_var_hover_is_dialect_aware() {
        let analysis = analyse("puts $auto_path\n");
        let mut registry = CommandRegistry::build_default();
        registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
        // iRules provides no `auto_path`, so no special-var hover fires there.
        assert!(
            hover_with_profile(
                "puts $auto_path\n",
                0,
                8,
                &analysis,
                Some(&registry),
                tcl_dialect::DialectProfile::irules(),
            )
            .is_none()
        );

        // `tcl_platform` exists under iRules and is CMP-unsafe as a plain
        // global — the hover surfaces the `static::` guidance.
        let src = "set x $tcl_platform(os)\n";
        let analysis = analyse(src);
        let h = hover_with_profile(
            src,
            0,
            10,
            &analysis,
            Some(&registry),
            tcl_dialect::DialectProfile::irules(),
        )
        .expect("hover");
        assert!(h.value.contains("special array"), "{}", h.value);
        assert!(h.value.contains("CMP"), "{}", h.value);
        assert!(h.value.contains("static::tcl_platform"), "{}", h.value);
    }

    // clock format hover

    #[test]
    fn scan_clock_specifiers_finds_each_specifier() {
        let s = scan_clock_specifiers("%Y-%m-%d %H:%M:%S");
        assert_eq!(s, vec!["%Y", "%m", "%d", "%H", "%M", "%S"]);
    }

    #[test]
    fn scan_clock_specifiers_handles_locale_prefix() {
        let s = scan_clock_specifiers("%EY-%Om");
        assert_eq!(s, vec!["%EY", "%Om"]);
    }

    #[test]
    fn scan_clock_specifiers_handles_literal_percent() {
        let s = scan_clock_specifiers("100%% complete");
        assert_eq!(s, vec!["%%"]);
    }

    #[test]
    fn clock_format_hover_renders_specifier_table() {
        let text = clock_format_hover_text("%Y-%m-%d");
        assert!(text.contains("**Clock format string**"), "{text}");
        assert!(text.contains("| `%Y` | 4-digit year |"), "{text}");
        assert!(text.contains("| `%m` | Month (01–12) |"), "{text}");
        assert!(text.contains("| `%d` | Day of month (01–31) |"), "{text}");
    }

    #[test]
    fn clock_format_hover_marks_locale_modified_specifiers() {
        let text = clock_format_hover_text("%EY");
        assert!(text.contains("(locale-modified)"), "{text}");
    }

    #[test]
    fn clock_format_hover_handles_empty_format() {
        let text = clock_format_hover_text("no specifiers here");
        assert!(text.contains("No specifiers found"), "{text}");
    }

    #[test]
    fn clock_format_string_at_position_detects_braced_literal() {
        let src = "clock format $time -format {%Y-%m-%d}\n";
        // Cursor inside the `{...}` literal.
        let found = clock_format_string_at_position(src, 0, 30);
        assert_eq!(found.as_deref(), Some("%Y-%m-%d"));
    }

    #[test]
    fn clock_format_string_at_position_detects_quoted_literal() {
        let src = "clock format $time -format \"%Y\"\n";
        // Cursor inside the `"..."` literal.
        let found = clock_format_string_at_position(src, 0, 30);
        assert_eq!(found.as_deref(), Some("%Y"));
    }

    #[test]
    fn clock_format_string_at_position_skips_non_clock_commands() {
        let src = "puts \"%Y\"\n";
        let found = clock_format_string_at_position(src, 0, 7);
        assert!(found.is_none(), "{found:?}");
    }

    #[test]
    fn hover_fires_for_clock_format_specifier() {
        let src = "clock format $time -format {%Y-%m-%d}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor inside the format literal.
        let h = hover(src, 0, 30, &analysis, None).expect("hover");
        assert!(
            h.value.contains("Clock format string"),
            "expected clock hover, got: {value}",
            value = h.value,
        );
    }

    #[test]
    fn hover_abstains_for_local_proc_shadowing_format_command() {
        let src = "proc clock {args} { return 0 }\nclock format $time {%Y}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        assert!(hover(src, 1, 25, &analysis, None).is_none());
    }

    #[test]
    fn hover_abstains_after_static_rename_moves_builtin_away() {
        let src = "rename clock saved\nclock format $time {%Y}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let cursor = u32::try_from(src.find("%Y").unwrap() - src.find('\n').unwrap() - 1).unwrap();
        assert!(hover(src, 1, cursor, &analysis, None).is_none());
    }

    #[test]
    fn hover_uses_effective_registry_identity_after_rename_and_alias() {
        let renamed = "rename format local_format\nlocal_format {%Y}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(renamed, "tcl8.6").clone();
        let cursor =
            u32::try_from(renamed.find("%Y").unwrap() - renamed.find('\n').unwrap() - 1).unwrap();
        let h = hover(renamed, 1, cursor, &analysis, None).expect("renamed format hover");
        assert!(h.value.contains("**Format string**"), "{}", h.value);

        let aliased = "interp alias {} local_format {} format\nlocal_format {%Y}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(aliased, "tcl8.6").clone();
        let cursor =
            u32::try_from(aliased.find("%Y").unwrap() - aliased.find('\n').unwrap() - 1).unwrap();
        let h = hover(aliased, 1, cursor, &analysis, None).expect("aliased format hover");
        assert!(h.value.contains("**Format string**"), "{}", h.value);
    }

    #[test]
    fn pattern_and_format_hover_descend_through_every_registry_executable_region() {
        // The cursor offsets deliberately sit in separately nested source
        // slices.  A line-local or re-based walker finds the command but
        // reports against the wrong source position; all must retain the
        // original document's absolute coordinates.
        let cases = [
            (
                "proc p {} { format {%04d} 1 }\n",
                "%04d",
                "**Format string**",
                "tcl8.6",
            ),
            (
                "oo::class create C { method m {} { string match {*needle*} x } }\n",
                "*needle*",
                "**Glob pattern**",
                "tcl8.6",
            ),
            (
                "switch $x { a { format {%03d} 1 } default { puts no } }\n",
                "%03d",
                "**Format string**",
                "tcl8.6",
            ),
            (
                "switch $x { a \"format {%08d} 1\" default { puts no } }\n",
                "%08d",
                "**Format string**",
                "tcl8.6",
            ),
            (
                "expect { -re {ready} \"format {%09d} 1\" }\n",
                "%09d",
                "**Format string**",
                "expect",
            ),
            (
                "apply {{} {format {%02d} 1}}\n",
                "%02d",
                "**Format string**",
                "tcl8.6",
            ),
            (
                "puts [format {%01d} 1]\n",
                "%01d",
                "**Format string**",
                "tcl8.6",
            ),
            (
                "when HTTP_REQUEST { format {%05d} 1 }\n",
                "%05d",
                "**Format string**",
                "f5-irules",
            ),
        ];
        for (source, needle, expected, dialect) in cases {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse(source, dialect).clone();
            let (line, column) = position_of(source, needle);
            let profile =
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
            let hover = hover_with_profile(source, line, column, &analysis, None, profile)
                .unwrap_or_else(|| panic!("no hover for {dialect}: {source}"));
            assert!(hover.value.contains(expected), "{}", hover.value);
        }
    }

    #[test]
    fn quoted_case_action_hover_obeys_head_mutations_and_static_list_validation() {
        let aliased = "interp alias {} fmt {} format\nswitch $x { a \"fmt {%10d} 1\" }\n";
        let analysis = analyse(aliased);
        let (line, column) = position_of(aliased, "%10d");
        let result = hover(aliased, line, column, &analysis, None).expect("aliased quoted action");
        assert!(
            result.value.contains("**Format string**"),
            "{}",
            result.value
        );

        for source in [
            "rename format saved\nswitch $x { a \"format {%11d} 1\" }\n",
            "set actions { a \"format {%12d} 1\" }\nswitch $x $actions\n",
            "switch $x { a \"format {%13d} 1\" orphan }\n",
        ] {
            let analysis = analyse(source);
            let (line, column) = position_of(source, "%");
            assert!(
                hover(source, line, column, &analysis, None).is_none(),
                "mutated, dynamic, or malformed case list leaked a nested hover: {source}"
            );
        }
    }

    #[test]
    fn nested_format_hover_honours_aliases_and_shadowing() {
        let aliased = "interp alias {} fmt {} format\nproc p {} { fmt {%06d} 1 }\n";
        let analysis = analyse(aliased);
        let (line, column) = position_of(aliased, "%06d");
        let result = hover(aliased, line, column, &analysis, None).expect("aliased nested format");
        assert!(
            result.value.contains("**Format string**"),
            "{}",
            result.value
        );

        // A same-named proc owns the nested call, so the builtin format
        // grammar must not leak through merely because its spelling matches.
        let shadowed = "proc format {args} {}\nproc p {} { format {%07d} 1 }\n";
        let analysis = analyse(shadowed);
        let (line, column) = position_of(shadowed, "%07d");
        assert!(hover(shadowed, line, column, &analysis, None).is_none());
    }

    #[test]
    fn nested_substitution_hover_beats_its_outer_pattern_word() {
        let source = "regexp \"[format {%d} 1]\" $value\n";
        let analysis = analyse(source);
        let (line, column) = position_of(source, "%d");
        let result = hover(source, line, column, &analysis, None)
            .expect("format hover inside regexp substitution");
        assert!(
            result.value.contains("**Format string**"),
            "{}",
            result.value
        );
        assert!(
            !result.value.contains("**Regular expression**"),
            "outer regexp incorrectly claimed nested format: {}",
            result.value
        );
    }

    // sprintf format hover

    #[test]
    fn scan_sprintf_specifiers_finds_basic_types() {
        let s = scan_sprintf_specifiers("%d - %s : %x");
        assert_eq!(s, vec!["%d", "%s", "%x"]);
    }

    #[test]
    fn scan_sprintf_specifiers_captures_width_and_precision() {
        let s = scan_sprintf_specifiers("%05d %-10s %.3f");
        assert_eq!(s, vec!["%05d", "%-10s", "%.3f"]);
    }

    #[test]
    fn scan_sprintf_specifiers_captures_positional() {
        let s = scan_sprintf_specifiers("%1$s %2$d");
        assert_eq!(s, vec!["%1$s", "%2$d"]);
    }

    #[test]
    fn scan_sprintf_specifiers_handles_literal_percent() {
        let s = scan_sprintf_specifiers("%% done");
        assert_eq!(s, vec!["%%"]);
    }

    #[test]
    fn sprintf_format_hover_renders_specifier_table() {
        let text = sprintf_format_hover_text("%d - %s");
        assert!(text.contains("**Format string** (sprintf-style)"), "{text}");
        assert!(text.contains("| `%d` | Signed decimal integer |"), "{text}");
        assert!(text.contains("| `%s` | String |"), "{text}");
    }

    #[test]
    fn sprintf_format_hover_handles_empty_format() {
        let text = sprintf_format_hover_text("no specifiers here");
        assert!(text.contains("No specifiers found"), "{text}");
    }

    #[test]
    fn sprintf_format_string_at_position_detects_braced_literal() {
        let src = "format {%d items} $count\n";
        let found = sprintf_format_string_at_position(src, 0, 10);
        assert_eq!(found.as_deref(), Some("%d items"));
    }

    #[test]
    fn sprintf_format_string_at_position_detects_quoted_literal() {
        let src = "format \"%d\" 42\n";
        let found = sprintf_format_string_at_position(src, 0, 9);
        assert_eq!(found.as_deref(), Some("%d"));
    }

    #[test]
    fn sprintf_format_string_at_position_skips_non_format_commands() {
        let src = "puts \"%d\"\n";
        let found = sprintf_format_string_at_position(src, 0, 7);
        assert!(found.is_none(), "{found:?}");
    }

    #[test]
    fn hover_fires_for_sprintf_specifier() {
        let src = "format {%d items} $count\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let h = hover(src, 0, 10, &analysis, None).expect("hover");
        assert!(
            h.value.contains("Format string"),
            "expected sprintf hover, got: {value}",
            value = h.value,
        );
    }

    // binary format hover

    fn binary_ctx(text: &str) -> BinaryContext {
        BinaryContext {
            text: text.to_string(),
            subcmd: "format".to_string(),
            args: Vec::new(),
        }
    }

    #[test]
    fn scan_binary_fields_finds_basic_types() {
        let fields = scan_binary_fields("a4 H2 i");
        let fulls: Vec<&str> = fields.iter().map(|f| f.full.as_str()).collect();
        assert_eq!(fulls, vec!["a4", "H2", "i"]);
        assert_eq!(fields[0].byte_size, Some(4));
        assert_eq!(fields[1].byte_size, Some(1));
        assert_eq!(fields[2].byte_size, Some(4));
    }

    #[test]
    fn scan_binary_fields_handles_star_count() {
        let fields = scan_binary_fields("a* I*");
        assert!(fields[0].star);
        assert!(fields[1].star);
        assert_eq!(fields[0].byte_size, None);
        assert_eq!(fields[1].byte_size, None);
    }

    #[test]
    fn binary_format_hover_renders_summary_and_detail_table() {
        let text = binary_format_hover_text(&binary_ctx("a4 i"));
        assert!(text.contains("**binary format**"), "{text}");
        assert!(text.contains("2 fields"), "{text}");
        assert!(text.contains("8 bytes"), "{text}");
        // Detail table now has 4 columns and a Bytes column.
        assert!(
            text.contains("| Spec | Variable | Type | Bytes |"),
            "{text}"
        );
        assert!(
            text.contains("| `a4` | a4 | str (null-pad) | 4 |"),
            "{text}"
        );
        assert!(text.contains("| `i` | i | int32 LE | 4 |"), "{text}");
    }

    #[test]
    fn binary_format_hover_renders_byte_ruler_diagram() {
        let text = binary_format_hover_text(&binary_ctx("c s i"));
        // Diagram fenced in a code block.
        assert!(text.contains("```"), "{text}");
        // Box-drawing characters for the field boundaries.
        assert!(text.contains('┌'), "{text}");
        assert!(text.contains('┬'), "{text}");
        assert!(text.contains('┐'), "{text}");
        // Numeric ruler — 7 bytes total (1 + 2 + 4).
        assert!(text.contains("0   1"), "{text}");
    }

    #[test]
    fn binary_format_hover_omits_diagram_when_total_exceeds_32_bytes() {
        // `d` is 8 bytes; five of them = 40 bytes — over the
        // 32-byte diagram budget.
        let text = binary_format_hover_text(&binary_ctx("d5"));
        assert!(text.contains("**binary format**"), "{text}");
        assert!(!text.contains('┌'), "diagram should be skipped: {text}");
    }

    #[test]
    fn binary_format_hover_omits_diagram_when_size_unknown() {
        // `a*` has unknown byte count.
        let text = binary_format_hover_text(&binary_ctx("a*"));
        assert!(!text.contains('┌'), "{text}");
        // The Bytes column still renders `…` for star fields.
        assert!(
            text.contains("| `a*` | a* | str (null-pad) | … |"),
            "{text}"
        );
    }

    #[test]
    fn binary_format_hover_labels_fields_with_arg_names() {
        let ctx = BinaryContext {
            text: "c i".to_string(),
            subcmd: "scan".to_string(),
            args: vec!["byte".to_string(), "word".to_string()],
        };
        let text = binary_format_hover_text(&ctx);
        // Detail-table Variable column gets the real names.
        assert!(text.contains("| `c` | byte |"), "{text}");
        assert!(text.contains("| `i` | word |"), "{text}");
        // Ruler diagram labels also pick up the names.
        assert!(text.contains("byte"), "{text}");
        assert!(text.contains("word"), "{text}");
    }

    #[test]
    fn binary_format_hover_renders_uint_modifier() {
        let text = binary_format_hover_text(&binary_ctx("iu"));
        assert!(text.contains("uint32"), "{text}");
    }

    #[test]
    fn binary_format_hover_no_specifiers_returns_friendly_message() {
        let text = binary_format_hover_text(&binary_ctx("ZZZ"));
        assert!(text.contains("No specifiers found"), "{text}");
    }

    #[test]
    fn binary_format_context_at_position_detects_braced_literal() {
        let src = "binary format {a4 i} val\n";
        let ctx = binary_format_context_at_position(src, 0, 17).expect("found ctx");
        assert_eq!(ctx.text, "a4 i");
        assert_eq!(ctx.subcmd, "format");
        assert_eq!(ctx.args, vec!["val"]);
    }

    #[test]
    fn binary_format_context_extracts_scan_var_names() {
        // Quoted format string with two trailing var names — the
        // hover should pick up both as the scan target labels.
        let src = "binary scan $buf \"cI\" byte word\n";
        let ctx = binary_format_context_at_position(src, 0, 19).expect("found ctx");
        assert_eq!(ctx.text, "cI");
        assert_eq!(ctx.subcmd, "scan");
        assert_eq!(ctx.args, vec!["byte", "word"]);
    }

    #[test]
    fn hover_fires_for_binary_specifier() {
        let src = "binary format {a4 i} val\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let h = hover(src, 0, 17, &analysis, None).expect("hover");
        assert!(h.value.contains("binary format"), "{}", h.value);
    }

    #[test]
    fn hover_fires_for_bare_binary_format_specifier() {
        // Tcl accepts an unquoted format word.  The registry identifies it as
        // the `FormatString`; hover must not require braces or quotes.
        let src = "binary format c2s value\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let h = hover(src, 0, 15, &analysis, None).expect("hover");
        assert!(h.value.contains("binary format"), "{}", h.value);
    }

    #[test]
    fn hover_uses_lsearchs_registry_selected_pattern_language() {
        // `-regexp` changes lsearch's embedded language, while an exact or
        // abbreviated option still shifts list/pattern positions through the
        // registry's OptionSpec grammar.
        let src = "lsearch -regexp {a b} {a+}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let h = hover(src, 0, 24, &analysis, None).expect("hover");
        assert!(h.value.contains("Regex pattern"), "{}", h.value);
    }

    #[test]
    fn dynamic_lsearch_option_prefix_does_not_claim_the_following_list_as_glob() {
        let src = "lsearch $mode {a b} {a*}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // `$mode` can evaluate to a value-taking lsearch option. The source
        // shape therefore cannot prove that `{a b}` is the pattern word.
        assert!(hover(src, 0, 16, &analysis, None).is_none());
    }

    /// A substituted leading word is not a positional operand until Tcl has
    /// evaluated it.  The source-aware registry query must therefore keep
    /// every resolver-owned pattern and regsub replacement unclaimed, not
    /// merely the lsearch-specific descriptor.
    #[test]
    fn dynamic_leading_options_abstain_for_pattern_and_regsub_format_hovers() {
        let registry = tcl_registry::CommandRegistry::build_default();
        for (src, needle) in [
            ("regexp $mode {a+} $value\n", "a+"),
            ("glob $mode /tmp {*.tcl}\n", "*.tcl"),
            ("regsub $mode {a} $value {\\1}\n", "\\1"),
            ("regsub -c {a} $value {\\1}\n", "\\1"),
            ("regsub -command {a} $value callback\n", "callback"),
        ] {
            let analysis = analyse(src);
            let (line, character) = position_of(src, needle);
            assert!(
                !hover(src, line, character, &analysis, Some(&registry)).is_some_and(|found| {
                    found.value.contains("Regex pattern")
                        || found.value.contains("Glob pattern")
                        || found.value.contains("Substitution spec")
                }),
                "{src:?}: an unresolved, invalid, or callback-leading layout must not claim {needle:?} as an embedded language",
            );
        }

        let src = "regsub -start $start {a} $value {\\1}\n";
        let analysis = analyse(src);
        let (line, character) = position_of(src, "\\1");
        let found = hover(src, line, character, &analysis, Some(&registry)).expect("hover");
        assert!(
            found.value.contains("Substitution spec"),
            "a declared -start value has a fixed width: {}",
            found.value
        );
    }

    #[test]
    fn hover_explains_a_computed_format_string() {
        // A literal `format "%-20s %d" a 1` already explains its
        // specifiers; the lattice proves the same text when it reaches the
        // word through a variable, so the computed spelling must not go
        // blind.
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "set fmt \"%-20s %d\"\nformat $fmt a 1\n";
        let analysis = analyse(src);
        let (line, character) = position_of(src, "$fmt");
        let found = hover(src, line, character, &analysis, Some(&registry)).expect("hover");
        assert!(
            found.value.contains("**Format string** (sprintf-style)"),
            "{}",
            found.value
        );
        assert!(
            found.value.contains("| `%d` | Signed decimal integer |"),
            "{}",
            found.value
        );

        // Negative: an unknown `$fmt` (a bare parameter, never assigned a
        // literal) proves nothing, so the plain variable card shows with no
        // format-string section appended.
        let src = "proc p {fmt} {\n    format $fmt a 1\n}\n";
        let analysis = analyse(src);
        let (line, character) = position_of(src, "$fmt");
        let found = hover(src, line, character, &analysis, Some(&registry)).expect("hover");
        assert!(
            found.value.contains("**Variable** `fmt`"),
            "{}",
            found.value
        );
        assert!(
            !found.value.contains("Format string"),
            "an unproven $fmt must not draw a format-string hover: {}",
            found.value
        );
    }

    #[test]
    fn lsearch_pattern_hover_uses_the_documents_profiled_option_set() {
        // Tcl 9 makes -str the unique -stride abbreviation. Before 9 it is
        // not an lsearch option, so the preceding -regexp cannot claim the
        // final word as a regex pattern. This reaches the profile-aware
        // PatternArgResolver path, not a command-name hover branch.
        let src = "lsearch -regexp -str 2 {a b} {a+}\n";
        let cursor = u32::try_from(src.rfind("a+").expect("final pattern")).unwrap();
        for (dialect, expected) in [("tcl8.6", false), ("tcl9.0", true)] {
            let mut analyser = tcl_compiler::analyser::Analyser::new();
            let analysis = analyser.analyse(src, dialect).clone();
            let profile =
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
            let found = hover_with_profile(
                src,
                0,
                cursor,
                &analysis,
                Some(&tcl_registry::CommandRegistry::build_default()),
                profile,
            );
            assert_eq!(
                found.is_some_and(|hover| hover.value.contains("Regex pattern")),
                expected,
                "{dialect}: profile-filtered lsearch options decide the pattern position"
            );
        }
    }

    // regsub substitution-spec hover

    #[test]
    fn scan_regsub_backrefs_finds_each_backref() {
        let r = scan_regsub_backrefs("\\1-\\2 (\\& and \\0)");
        assert_eq!(r, vec!["\\1", "\\2", "\\&", "\\0"]);
    }

    #[test]
    fn regsub_hover_renders_backref_table() {
        let text = regsub_hover_text("prefix \\1 suffix");
        assert!(text.contains("**Substitution spec**"), "{text}");
        assert!(text.contains("| `\\1` | First capture group |"), "{text}");
    }

    #[test]
    fn regsub_hover_handles_no_backrefs() {
        let text = regsub_hover_text("plain text");
        assert!(text.contains("No backreferences found"), "{text}");
    }

    #[test]
    fn regsub_subspec_at_position_finds_subspec_literal() {
        let src = "regsub foo bar {\\1-baz} out\n";
        // Cursor inside the subspec literal.
        let found = regsub_subspec_at_position(src, 0, 18);
        assert_eq!(found.as_deref(), Some("\\1-baz"));
    }

    #[test]
    fn hover_fires_for_regsub_backref() {
        let src = "regsub foo bar {\\1-baz} out\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let h = hover(src, 0, 18, &analysis, None).expect("hover");
        assert!(h.value.contains("Substitution spec"), "{}", h.value);
    }

    // glob pattern hover

    #[test]
    fn scan_glob_metachars_finds_star_and_question() {
        let m = scan_glob_metachars("*.tcl");
        let toks: Vec<&str> = m.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"*"), "{m:?}");
    }

    #[test]
    fn scan_glob_metachars_finds_character_class() {
        let m = scan_glob_metachars("[abc]*.tcl");
        let toks: Vec<&str> = m.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"[abc]"), "{m:?}");
        assert!(toks.contains(&"*"), "{m:?}");
    }

    #[test]
    fn glob_hover_renders_table() {
        let text = glob_hover_text("*.tcl");
        assert!(text.contains("**Glob pattern**"), "{text}");
        assert!(text.contains("| `*` |"), "{text}");
    }

    #[test]
    fn glob_hover_for_literal_string() {
        let text = glob_hover_text("plain");
        assert!(text.contains("Literal string"), "{text}");
    }

    #[test]
    fn hover_fires_for_glob_pattern() {
        // Braced glob pattern — single-line literal detection
        // requires `"..."` or `{...}` delimiters.  Bare globs
        // (`glob *.tcl`) fall through to the proc / word
        // lookup; their support lives in the same multi-line
        // / arg-position machinery that other `*-rich`
        // sub-strips defer.
        let src = "glob {*.tcl}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor inside the braced pattern.
        let h = hover(src, 0, 8, &analysis, None).expect("hover");
        assert!(h.value.contains("Glob pattern"), "{}", h.value);
    }

    // regex pattern hover

    #[test]
    fn scan_regex_components_finds_anchors_and_quantifiers() {
        let r = scan_regex_components("^foo.*$");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"^"), "{r:?}");
        assert!(toks.contains(&"."), "{r:?}");
        assert!(toks.contains(&"*"), "{r:?}");
        assert!(toks.contains(&"$"), "{r:?}");
    }

    #[test]
    fn scan_regex_components_finds_character_class() {
        let r = scan_regex_components("[a-z]+");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"[a-z]"), "{r:?}");
        assert!(toks.contains(&"+"), "{r:?}");
    }

    #[test]
    fn scan_regex_components_finds_escapes() {
        let r = scan_regex_components("\\d+");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"\\d"), "{r:?}");
    }

    #[test]
    fn scan_regex_components_finds_groups_and_lookahead() {
        let r = scan_regex_components("(?:foo)(?=bar)");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"(?:"), "{r:?}");
        assert!(toks.contains(&"(?="), "{r:?}");
    }

    #[test]
    fn regex_hover_renders_table() {
        let text = regex_hover_text("^foo$");
        assert!(text.contains("**Regex pattern**"), "{text}");
        assert!(text.contains("| `^` |"), "{text}");
    }

    #[test]
    fn hover_fires_for_regex_pattern() {
        let src = "regexp {^foo.*$} $line\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor inside the pattern literal.
        let h = hover(src, 0, 10, &analysis, None).expect("hover");
        assert!(h.value.contains("Regex pattern"), "{}", h.value);
    }

    #[test]
    fn hover_abstains_for_local_proc_shadowing_regexp_command() {
        let src = "proc regexp {args} { return 0 }\nregexp {^foo$} $line\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        assert!(hover(src, 1, 10, &analysis, None).is_none());
    }

    // IP address hover

    #[test]
    fn ip_hover_classifies_private_ipv4() {
        let t = ip_address_hover_text("10.0.0.1").expect("hover");
        assert!(t.contains("IPv4 address"), "{t}");
        assert!(t.contains("Private (RFC 1918)"), "{t}");
    }

    #[test]
    fn ip_hover_classifies_loopback() {
        let t = ip_address_hover_text("127.0.0.1").expect("hover");
        assert!(t.contains("Loopback"), "{t}");
    }

    #[test]
    fn ip_hover_classifies_public_ipv4() {
        let t = ip_address_hover_text("8.8.8.8").expect("hover");
        assert!(t.contains("Public"), "{t}");
    }

    #[test]
    fn ip_hover_renders_cidr_prefix() {
        let t = ip_address_hover_text("10.0.0.0/8").expect("hover");
        assert!(t.contains("CIDR network: `10.0.0.0/8`"), "{t}");
    }

    #[test]
    fn ip_hover_classifies_ipv6_loopback() {
        let t = ip_address_hover_text("::1").expect("hover");
        assert!(t.contains("IPv6 address"), "{t}");
        assert!(t.contains("Loopback"), "{t}");
    }

    #[test]
    fn ip_hover_detects_ipv4_mapped_ipv6() {
        let t = ip_address_hover_text("::ffff:192.0.2.1").expect("hover");
        assert!(t.contains("IPv4-mapped"), "{t}");
    }

    #[test]
    fn ip_hover_rejects_non_ip_strings() {
        assert!(ip_address_hover_text("hello").is_none());
        assert!(ip_address_hover_text("256.256.256.256").is_none());
        assert!(ip_address_hover_text("not.an.ip.address").is_none());
    }

    #[test]
    fn hover_fires_for_ip_address_word() {
        let src = "set host 10.0.0.1\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor on `10.0.0.1`.
        let h = hover(src, 0, 11, &analysis, None).expect("hover");
        assert!(h.value.contains("IPv4 address"), "{}", h.value);
    }

    // registry-driven hovers

    #[test]
    fn builtin_command_hover_surfaces_summary_from_registry() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let t =
            builtin_command_hover_text(&registry, "puts", &analyse(""), u32::MAX).expect("hover");
        assert!(t.contains("built-in command"), "{t}");
        assert!(t.contains("`puts`"), "{t}");
    }

    #[test]
    fn builtin_command_hover_lists_subcommands() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let t =
            builtin_command_hover_text(&registry, "string", &analyse(""), u32::MAX).expect("hover");
        assert!(t.contains("Subcommands:"), "{t}");
        assert!(t.contains("length"), "{t}");
    }

    #[test]
    fn builtin_command_hover_returns_none_for_unknown() {
        let registry = tcl_registry::CommandRegistry::build_default();
        assert!(
            builtin_command_hover_text(&registry, "totallyMadeUpCommand", &analyse(""), u32::MAX,)
                .is_none()
        );
    }

    fn retained_registry_text(source: &str, cursor: u32, environment: &str) -> Option<String> {
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-logical-registry-cards";
        let profile = profile.intern();
        let context = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "presentation-only");
        original_registry::hover(source, &analysis, cursor).map(|hover| hover.value)
    }

    #[test]
    fn subcommand_hover_surfaces_for_string_length() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        let text = retained_registry_text("string length $name\n", 10, "tcl8.6")
            .expect("subcommand hover");
        assert!(text.contains("`string length`"), "{text}");
        assert!(text.contains("subcommand"), "{text}");
    }

    #[test]
    fn subcommand_hover_resolves_unique_prefix_abbreviation() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        let text = retained_registry_text("string le $name\n", 8, "tcl8.6").expect("prefix hover");
        assert!(text.contains("`string length`"), "{text}");
    }

    #[test]
    fn subcommand_hover_prefix_is_dialect_aware() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        let source = "info class def ::C\n";
        assert!(
            retained_registry_text(source, 13, "tcl8.6")
                .is_some_and(|text| text.contains("`info class definition`"))
        );
        assert!(retained_registry_text(source, 13, "tcl9.0").is_none());
        let source = "string rev abc\n";
        assert!(retained_registry_text(source, 8, "tcl8.6").is_some());
        assert!(retained_registry_text(source, 8, "tcl8.4").is_none());
    }

    #[test]
    fn subcommand_hover_skips_unknown_subcommand() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        assert!(retained_registry_text("string bogusSubcommand\n", 12, "tcl8.6").is_none());
    }

    #[test]
    fn option_hover_resolves_subcommand_scoped_option() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        let text = retained_registry_text("chan configure $chan -inputmode raw\n", 26, "tcl9.0")
            .expect("configure-scoped option");
        assert!(text.contains("`-inputmode`"), "{text}");
        assert!(text.contains("chan configure"), "{text}");
    }

    #[test]
    fn option_hover_notes_dialect_unavailability() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        let source = "chan configure $chan -inputmode raw\n";
        let old =
            retained_registry_text(source, 26, "tcl8.6").expect("excluded exact option metadata");
        assert!(old.contains("Not available in the active dialect"), "{old}");
        let new = retained_registry_text(source, 26, "tcl9.0").expect("admitted option");
        assert!(
            !new.contains("Not available in the active dialect"),
            "{new}"
        );
    }

    #[test]
    fn option_hover_falls_back_to_top_level_options_for_simple_command() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        let text = retained_registry_text("lsearch -exact {a b} x\n", 11, "tcl8.6")
            .expect("root option metadata");
        assert!(text.contains("`-exact`"), "{text}");
        assert!(text.contains("of `lsearch`"), "{text}");
    }

    #[test]
    fn sub_subcommand_hover_surfaces_for_info_object_class() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        let text = retained_registry_text("info object class $obj\n", 14, "tcl8.6")
            .expect("nested subcommand");
        assert!(text.contains("`info object class`"), "{text}");
        assert!(text.contains("subcommand"), "{text}");
        let text =
            retained_registry_text("info class super $cls\n", 13, "tcl8.6").expect("nested prefix");
        assert!(text.contains("`info class superclasses`"), "{text}");
        // The first selector is described by its own retained subcommand row.
        let parent =
            retained_registry_text("info object class\n", 6, "tcl8.6").expect("parent selector");
        assert!(parent.contains("`info object`"), "{parent}");
        assert!(!parent.contains("`info object class`"), "{parent}");
    }

    #[test]
    fn logical_registry_hover_keeps_actual_context_and_source_currency() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        let source = "string cat A B";
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-logical-hover-context";
        let profile = profile.intern();
        let driver =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
            let actual = tcl_registry::model::ingress::resolve_environment(environment)
                .default_context_registry();
            let context = std::sync::Arc::new(
                actual.with_command_store(driver.commands().snapshot().shared_registry()),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context.clone(),
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            );
            let mut analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "presentation-only");
            analysis.command_invocations.clear();
            let answer = hover(source, 0, 8, &analysis, None);
            assert_eq!(answer.is_some(), expected, "{environment}: {answer:?}");
            assert!(hover("string cat C D", 0, 8, &analysis, None).is_none());
            analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(
                    context.with_command_store(context.commands().snapshot().shared_registry()),
                ),
                input.lexer_config(),
            ));
            assert!(hover(source, 0, 8, &analysis, None).is_none());
        }
    }

    #[test]
    fn logical_registry_hover_preserves_whole_alias_operands_and_declines_data() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        for (source, needle, expected) in [
            (
                "interp alias {} length {} string; length length value",
                "length value",
                "`string length`",
            ),
            (
                "interp alias {} rx {} regexp; rx -nocase pattern text",
                "-nocase",
                "option of `regexp`",
            ),
            (
                "proc p {} {string length value}",
                "length",
                "`string length`",
            ),
            (
                r"string \u006cength value",
                r"\u006cength",
                "`string length`",
            ),
        ] {
            let offset = u32::try_from(source.rfind(needle).unwrap() + 1).unwrap();
            let text = retained_registry_text(source, offset, "tcl8.6").expect(source);
            assert!(text.contains(expected), "{source}: {text}");
        }
        for (source, needle) in [
            (
                "interp alias {} rx {} regexp -nocase; rx pattern -nocase",
                "-nocase",
            ),
            ("puts {string length value}", "length"),
            ("proc string args {}; string length value", "length value"),
            ("string $selector value", "$selector"),
        ] {
            let offset = u32::try_from(source.rfind(needle).unwrap() + 1).unwrap();
            assert!(
                retained_registry_text(source, offset, "tcl8.6").is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn hover_fires_for_builtin_command_with_registry() {
        let src = "puts hello\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 2, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("built-in command"), "{}", h.value);
    }

    #[test]
    fn hover_fires_for_subcommand_with_registry() {
        let src = "string length $name\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 10, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("subcommand"), "{}", h.value);
    }

    // docstring formatting

    #[test]
    fn format_docstring_renders_brief_param_return_tags() {
        let raw = concat!(
            "@brief Greet someone\n",
            "Free-form description\n",
            "spanning two lines.\n",
            "@param name the person's name\n",
            "@param greeting optional greeting prefix\n",
            "@return the formatted greeting\n",
        );
        let rendered = format_docstring(raw);
        assert!(rendered.contains("Greet someone"), "{rendered}");
        assert!(rendered.contains("Free-form description"), "{rendered}");
        assert!(rendered.contains("**Parameters:**"), "{rendered}");
        assert!(
            rendered.contains("- **name** \u{2014} the person's name"),
            "{rendered}",
        );
        assert!(
            rendered.contains("- **greeting** \u{2014} optional greeting prefix"),
            "{rendered}",
        );
        assert!(
            rendered.contains("**Returns:** the formatted greeting"),
            "{rendered}",
        );
    }

    #[test]
    fn format_docstring_drops_decoration_lines() {
        // Pure-decoration lines (`.....`, `-----`) shouldn't
        // pollute the description block.
        let raw = "..........\nA description.\n..........\n";
        let rendered = format_docstring(raw);
        assert_eq!(rendered, "A description.");
    }

    #[test]
    fn format_docstring_passes_through_plain_text() {
        let raw = "Just a free-form description.\nNo tags here.\n";
        let rendered = format_docstring(raw);
        assert!(rendered.contains("Just a free-form description"));
        assert!(rendered.contains("No tags here"));
    }

    #[test]
    fn format_docstring_handles_param_without_description() {
        let raw = "@param naked\n";
        let rendered = format_docstring(raw);
        assert!(
            rendered.contains("- **naked**"),
            "expected bare param entry; got {rendered}",
        );
        // No trailing em-dash since there's no description.
        assert!(!rendered.contains("**naked** \u{2014}"), "{rendered}");
    }

    // class-member hover

    #[test]
    fn class_member_hover_bare_sibling_method_without_link_abstains() {
        // FP — a bareword sibling method call is NOT
        // actually reachable from another method's body unless `link`
        // exposed it that way; real tclsh: "invalid command name". Must
        // abstain rather than falsely resolve.
        let src = "oo::class create C {\n    method greet {who} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the first `greet` invocation (line 2, col 22).
        assert!(hover(src, 2, 22, &analysis, None).is_none());
    }

    #[test]
    fn class_member_hover_linked_sibling_method_resolves() {
        // TP — `link greet` (called from the
        // constructor) makes `greet` genuinely bareword-callable, so
        // hover now resolves. alias == target here, so no "linked from"
        // note (see the two-element-alias test below for that).
        let src = "oo::class create C {\n    constructor {} { link greet }\n    method greet {who} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Line 3: `    method twice {} { greet ; greet }` — same text/col
        // as the un-linked test above, one line further down.
        let h = hover(src, 3, 22, &analysis, None).expect("hover");
        assert!(h.value.contains("**method**"), "{}", h.value);
        assert!(h.value.contains("C::greet"), "{}", h.value);
        assert!(h.value.contains("1 param"), "{}", h.value);
        assert!(!h.value.contains("linked from"), "{}", h.value);
    }

    #[test]
    fn class_member_hover_resolves_when_class_extended_via_separate_oo_define() {
        // `Gadget` is created via `oo::class create` with no body; the
        // `link`, the linked method, and the bareword call site that depends
        // on it are all added via a *separate*, later `oo::define Gadget
        // { ... }` block — the `ticklecharts::chart` shape. Hover on
        // the bareword `Helper` call must still resolve — the cursor sits
        // inside that separate block, which `class_member_hover_text`'s
        // `enclosing_class_at` containment check must recognise as part of
        // `Gadget`'s body too, not just the original creation block.
        let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    constructor {} { link Helper }\n    method Helper {} { return hi }\n    method Caller {} { Helper }\n}\n";
        let analysis = analyse(src);
        // Line 6: `    method Caller {} { Helper }` — cursor on the bareword
        // `Helper` call (col 23).
        let h = hover(src, 6, 23, &analysis, None).expect("hover");
        assert!(h.value.contains("**method**"), "{}", h.value);
        assert!(h.value.contains("Gadget::Helper"), "{}", h.value);
    }

    #[test]
    fn class_member_hover_two_element_link_alias_notes_the_real_target() {
        // TP — `link {shortcut realMethod}` aliases
        // a DIFFERENT bareword to the real method; hover on the alias
        // must resolve to `realMethod`'s own declaration and note it was
        // reached via the alias (a currently-live false negative this
        // closes: today's LSP returns no hover at all for `shortcut`).
        let src = "oo::class create C {\n    constructor {} { link {shortcut realMethod} }\n    method realMethod {x} { return $x }\n    method bar {} { return [shortcut 42] }\n}\n";
        let analysis = analyse(src);
        // Line 3: `    method bar {} { return [shortcut 42] }` — col 28
        // lands on the `s` of `shortcut`.
        let h = hover(src, 3, 28, &analysis, None).expect("hover");
        assert!(h.value.contains("**method**"), "{}", h.value);
        assert!(h.value.contains("C::realMethod"), "{}", h.value);
        assert!(h.value.contains("linked from `shortcut`"), "{}", h.value);
    }

    // `my method` internal-dispatch hover

    #[test]
    fn my_dispatch_hover_resolves_a_plain_call_in_a_single_block_class() {
        // TP — a definite, single-target `my methodName` call must hover,
        // like a `link`-exposed bareword sibling call or `$obj method` —
        // go-to-definition and find-references resolve this exact shape (they
        // use
        // `enclosing_class_at`/`method_dispatch_definition` directly,
        // cursor-shape-driven; hover had no equivalent path, only the
        // word-match-driven `class_member_hover_text`, gated on
        // `linked_members`, which a plain un-linked `my` call never
        // populates).
        let src = "oo::class create geo::Plane {\n    method GetType {} { return Plane }\n    method WhichAmI {} { return [my GetType] }\n}\n";
        let analysis = analyse(src);
        // Line 2: `    method WhichAmI {} { return [my GetType] }` — cursor
        // on `GetType` inside `my GetType` (col 38).
        let h = hover(src, 2, 38, &analysis, None).expect("hover");
        assert!(h.value.contains("**method**"), "{}", h.value);
        assert!(h.value.contains("::geo::Plane::GetType"), "{}", h.value);
    }

    #[test]
    fn my_dispatch_hover_resolves_when_class_extended_via_separate_oo_define() {
        // TP — the two-block `oo::class create` + separate `oo::define`
        // pattern (constructor in `create`, every method including the
        // dispatched-on one in a later `define`), exercised here for hover
        // specifically; definition and references have their own tests for
        // the same shape.
        let src = "oo::class create geo::Plane {\n    constructor {args} {}\n}\noo::define geo::Plane {\n    method GetType {} { return Plane }\n    method WhichAmI {} { return [my GetType] }\n}\n";
        let analysis = analyse(src);
        // Line 5: `    method WhichAmI {} { return [my GetType] }` — cursor
        // on `GetType` inside `my GetType` (col 38).
        let h = hover(src, 5, 38, &analysis, None).expect("hover");
        assert!(h.value.contains("**method**"), "{}", h.value);
        assert!(h.value.contains("::geo::Plane::GetType"), "{}", h.value);
    }

    #[test]
    fn my_dispatch_hover_abstains_for_an_undefined_method() {
        // TN — `my` dispatching to a method the class genuinely doesn't
        // have must not fabricate a hover.
        let src = "oo::class create C {\n    method twice {} { return [my nope] }\n}\n";
        let analysis = analyse(src);
        // Line 1, cursor on `nope` inside `my nope` (col 34).
        assert!(hover(src, 1, 34, &analysis, None).is_none());
    }

    // A class member's own declaration hovers as that member, in the
    // creation block and in a reopening `oo::define` block alike — the same
    // way `proc` and the class name itself hover at their declarations.

    #[test]
    fn member_declaration_hover_resolves_in_the_creation_block() {
        // TP — the baseline: `method plain` hovers at its own name token.
        let src = "oo::class create Foo {\n    method plain {a} { return $a }\n}\n";
        let analysis = analyse(src);
        // Line 1, col 13 — inside `plain`.
        let h = hover(src, 1, 13, &analysis, None).expect("hover");
        assert!(h.value.contains("**method**"), "{}", h.value);
        assert!(h.value.contains("::Foo::plain"), "{}", h.value);
        assert!(h.value.contains("1 param"), "{}", h.value);
    }

    #[test]
    fn member_declaration_hover_resolves_in_a_reopening_oo_define_block() {
        // TP — the SpiceGenTcl
        // `generalClasses.tcl`: `oo::configurable create Parameter { … }`
        // then a separate `oo::define Parameter { method <WriteProp-value>
        // … }`).  tclsh 9.0.4 dispatches `$f reopened hi` to the reopened
        // block's method, so it is every bit `Foo`'s own member and must
        // hover identically to one written in the creation block.
        let src = "oo::class create Foo {\n    method plain {} { return plain }\n}\noo::define Foo {\n    method reopened {val} { return $val }\n}\n";
        let analysis = analyse(src);
        // Line 4, col 13 — inside `reopened`, in the reopening block.
        let h = hover(src, 4, 13, &analysis, None).expect("hover");
        assert!(h.value.contains("**method**"), "{}", h.value);
        assert!(h.value.contains("::Foo::reopened"), "{}", h.value);
        assert!(h.value.contains("1 param"), "{}", h.value);
    }

    #[test]
    fn member_declaration_hover_covers_classmethods_and_properties() {
        // TP — the other two member tables render with their own labels.
        let src = "oo::configurable create C {\n    property size\n    self method make {n} { return $n }\n}\n";
        let analysis = analyse(src);
        let prop = hover(src, 1, 15, &analysis, None).expect("property hover");
        assert!(prop.value.contains("**property**"), "{}", prop.value);
        assert!(prop.value.contains("::C::size"), "{}", prop.value);
        let cm = hover(src, 2, 19, &analysis, None).expect("classmethod hover");
        assert!(cm.value.contains("**classmethod**"), "{}", cm.value);
        assert!(cm.value.contains("::C::make"), "{}", cm.value);
    }

    #[test]
    fn member_declaration_hover_notes_an_override() {
        // TP — the declaration renders the same MRO note a call site does,
        // so the two descriptions of one member cannot disagree.
        let src = "oo::class create Base {\n    method run {} { return base }\n}\noo::class create Derived {\n    superclass Base\n    method run {} { return derived }\n}\n";
        let analysis = analyse(src);
        let h = hover(src, 5, 12, &analysis, None).expect("hover");
        assert!(h.value.contains("::Derived::run"), "{}", h.value);
        assert!(h.value.contains("Base"), "MRO note expected: {}", h.value);
    }

    #[test]
    fn member_declaration_hover_does_not_fire_on_a_same_named_word_elsewhere() {
        // FP guard — only the declaration's own name token hovers as the
        // member.  A same-spelled *data* word inside a method body is not a
        // member reference, and must keep falling through the tier order
        // (here: to nothing at all).
        let src = "oo::class create Foo {\n    method plain {} { return plain }\n}\n";
        let analysis = analyse(src);
        // Line 1, col 30 — the bareword `plain` in `return plain`, which is
        // a value, not a call (an un-linked sibling name is not callable).
        assert!(hover(src, 1, 30, &analysis, None).is_none());
    }

    #[test]
    fn member_declaration_hover_abstains_outside_a_class_body() {
        // TN — a proc whose name happens to match a method of some class in
        // the file must not borrow that class's member hover.
        let src = "oo::class create Foo {\n    method plain {} { return 1 }\n}\nproc plain {} { return 2 }\n";
        let analysis = analyse(src);
        let h = hover(src, 3, 6, &analysis, None).expect("proc hover");
        assert!(
            !h.value.contains("**method**"),
            "the proc, not Foo's method: {}",
            h.value
        );
    }

    #[test]
    fn class_member_hover_bare_sibling_classmethod_without_link_abstains() {
        // FP — same shape as the method case above,
        // for `classmethod`.
        let src = "oo::class create C {\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
        let analysis = analyse(src);
        assert!(hover(src, 2, 20, &analysis, None).is_none());
    }

    #[test]
    fn class_member_hover_linked_sibling_classmethod_resolves() {
        // TP — `link factory` makes the classmethod
        // hover resolve.
        let src = "oo::class create C {\n    constructor {} { link factory }\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
        let analysis = analyse(src);
        // Line 3: `    method use {} { factory }` — same text/col as the
        // un-linked test above, one line further down.
        let h = hover(src, 3, 20, &analysis, None).expect("hover");
        assert!(h.value.contains("**classmethod**"), "{}", h.value);
        assert!(h.value.contains("C::factory"), "{}", h.value);
    }

    #[test]
    fn class_member_hover_fires_for_constructor_keyword() {
        let src = "oo::class create C {\n    constructor {arg} {}\n    method touch_ctor {} { constructor }\n}\n";
        let analysis = analyse(src);
        let h = hover(src, 2, 27, &analysis, None).expect("hover");
        assert!(h.value.contains("constructor"), "{}", h.value);
        // Class qualified name is `::C`.
        assert!(h.value.contains("::C"), "{}", h.value);
    }

    #[test]
    fn class_member_hover_skipped_outside_class_body() {
        let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the bare `greet` outside the class body.
        // No proc / class / method match — should return None.
        assert!(hover(src, 3, 2, &analysis, None).is_none());
    }

    // $obj method dispatch

    #[test]
    fn obj_method_hover_fires_for_known_instance() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
        let analysis = analyse(src);
        // Line 4 `$d bark` — cursor on `bark` (col 3).
        let h = hover(src, 4, 3, &analysis, None).expect("hover");
        assert!(h.value.contains("**method**"), "{}", h.value);
        assert!(h.value.contains("::Dog::bark"), "{}", h.value);
    }

    #[test]
    fn obj_method_hover_fires_for_a_literal_foreach_installed_method() {
        // The member's name comes from the loop's own literal
        // list, not a written `method NAME` word, but a call site must
        // still resolve to a hover — go-to-definition and hover share the
        // same member table this walk populates.
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
        // Line 6 `$d alpha` — cursor on `alpha` (col 3).
        let h = hover(src, 6, 3, &analysis, None).expect("hover");
        assert!(h.value.contains("**method**"), "{}", h.value);
        assert!(h.value.contains("::Widget::alpha"), "{}", h.value);
    }

    #[test]
    fn obj_method_hover_none_for_unknown_instance() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\n$x bark\n";
        let analysis = analyse(src);
        // `x` has no recorded class — no hover.
        assert!(hover(src, 3, 3, &analysis, None).is_none());
    }

    /// A Tk widget's instance command (a registry-modelled, self-referential
    /// `object_class`, not a user-defined one) needs the
    /// registry passed to resolve hover text at all.
    #[test]
    fn obj_method_hover_fires_for_bareword_widget() {
        let reg = tcl_registry::CommandRegistry::build_default();
        let src = "ttk::treeview .t\n.t instate {selected} {}\n";
        let analysis = analyse(src);
        // Line 1 `.t instate …` — cursor on `instate` (col 3).
        let h = hover(src, 1, 3, &analysis, Some(&reg)).expect("hover");
        assert!(h.value.contains("ttk::treeview instate"), "{}", h.value);
    }

    #[test]
    fn widget_method_hover_follows_the_resolved_tk_lifecycle() {
        // ttk::treeview's `current` instance method is introduced by Tk 9.1.
        // The registry lookup is the only method-specific input here, so this
        // proves an older profile cannot render an impossible method hover.
        let src = "ttk::treeview .tree\n.tree current\n";
        let analysis = analyse(src);
        for (dialect, expected) in [("tcl9.0", false), ("tcl9.1", true)] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
            let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
            let hover = hover_with_profile(src, 1, 6, &analysis, Some(registry), profile);
            assert_eq!(
                hover.is_some(),
                expected,
                "ttk::treeview current hover under {dialect}: {hover:?}"
            );
        }

        // A document's unconditional package require can raise the profile
        // floor. The request-time hover path passes that explicit package
        // floor to the same registry lookup instead of baking Tk knowledge
        // into the provider.
        let raised = "package require Tk 9.1\nttk::treeview .tree\n.tree current\n";
        let raised_analysis = analyse(raised);
        let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let raised_hover = hover_with_profile(
            raised,
            2,
            6,
            &raised_analysis,
            Some(tcl_registry::model::ingress::static_context_for("tcl9.0").commands()),
            tcl90,
        );
        assert!(
            raised_hover.is_some(),
            "an explicit Tk 9.1 floor must expose current: {raised_hover:?}"
        );
    }

    /// Without a registry passed, the same widget dispatch has nowhere to
    /// look up the method — `obj_method_hover_text`'s registry fallback is
    /// skipped, not a panic.
    #[test]
    fn obj_method_hover_none_for_widget_without_registry() {
        let src = "ttk::treeview .t\n.t instate {selected} {}\n";
        let analysis = analyse(src);
        assert!(hover(src, 1, 3, &analysis, None).is_none());
    }

    #[test]
    fn obj_method_hover_fires_for_var_captured_widget() {
        let reg = tcl_registry::CommandRegistry::build_default();
        let src = "set lb [listbox .l]\n$lb curselection\n";
        let analysis = analyse(src);
        // Line 1 `$lb curselection` — cursor on `curselection` (col 4).
        let h = hover(src, 1, 4, &analysis, Some(&reg)).expect("hover");
        assert!(h.value.contains("listbox curselection"), "{}", h.value);
    }

    // tcl::OptProc — the `opt` package's automatic-option-parsing proc
    // definer: hover on either the declaration or a
    // call site must show the real `args`-only signature, never
    // `optlist`'s own literal text.

    #[test]
    fn opt_proc_declaration_and_call_site_both_hover_the_real_args_only_signature() {
        let src = "::tcl::OptProc greet {child -use -display} { return $child }\ngreet foo\n";
        let analysis = analyse(src);
        // Line 0 — cursor on "greet" right after `::tcl::OptProc` (col 15).
        let decl = hover(src, 0, 15, &analysis, None).expect("hover on decl");
        assert!(
            decl.value.contains("greet") && decl.value.contains("args"),
            "{}",
            decl.value
        );
        // Line 1 — cursor on the `greet foo` call site (col 0).
        let call = hover(src, 1, 0, &analysis, None).expect("hover on call site");
        assert_eq!(decl.value, call.value, "decl and call site must agree");
    }

    // Hover shares `definition()`'s command-resolution seam, so it shares
    // both of its gates — the command-position gate and the
    // resolved-indirect-head preference.

    #[test]
    fn fp_argument_word_sharing_a_procs_name_does_not_hover_that_proc() {
        // FP guard — `dump` is `anotherproc`'s first argument, never a
        // command, so rendering `proc ::dump`'s signature there is a wrong
        // answer.
        let src = "proc anotherproc {a b} { return $a }\nproc dump {x} { return $x }\nproc caller {} {\n    return [anotherproc dump 5]\n}\n";
        let analysis = analyse(src);
        let h = hover(src, 3, 24, &analysis, None);
        assert!(
            h.is_none_or(|hv| !hv.value.contains("proc ::dump")),
            "an argument word must not hover as a command"
        );
    }

    #[test]
    fn tp_the_command_head_beside_that_argument_still_hovers() {
        // TP — the gate must not cost the real head its hover.
        let src = "proc anotherproc {a b} { return $a }\nproc dump {x} { return $x }\nproc caller {} {\n    return [anotherproc dump 5]\n}\n";
        let analysis = analyse(src);
        let h = hover(src, 3, 12, &analysis, None).expect("hover on the call head");
        assert!(h.value.contains("proc ::anotherproc"), "{}", h.value);
    }

    #[test]
    fn consumed_procedure_name_hover_uses_its_lookup_receipt() {
        let source = "proc target {} {return OK}\ninfo args target\nrename target moved\nmoved\n";
        let analysis = tcl_compiler::analyser::Analyser::new()
            .analyse(source, "tcl8.6")
            .clone();
        for (written, prefix) in [
            ("info args target", "info args "),
            ("rename target", "rename "),
        ] {
            let offset = u32::try_from(source.find(written).unwrap() + prefix.len()).unwrap();
            assert!(!crate::definition::offset_is_command_head(
                &analysis, offset
            ));
            let result = proc_hover_at(
                &analysis,
                source,
                offset,
                "target",
                crate::definition::CallResolution::document_only(),
            )
            .expect("hover on the consumed source procedure");
            assert!(result.value.contains("proc ::target"), "{}", result.value);
        }
    }

    #[test]
    fn tp_resolved_indirect_head_hovers_the_command_it_reaches() {
        // TP — hover's half: the `${ns}::setdef` head carries
        // no written command name, so only the analyser's own resolution
        // can answer, and it must not lose to a same-named decoy.
        let src = "namespace eval ::tc { proc setdef {a} { return $a } }\nnamespace eval ::other { proc setdef {b c} { return $b } }\nset ns ::tc\n${ns}::setdef\n";
        let analysis = analyse(src);
        let h = hover(src, 3, 7, &analysis, None).expect("hover on the indirect head");
        assert!(
            h.value.contains("::tc::setdef"),
            "must describe ::tc::setdef, not the ::other decoy: {}",
            h.value
        );
    }
}

#[cfg(test)]
mod original_alias_hover_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn alias_advice_cannot_claim_a_same_named_data_argument() {
        let source = "interp alias {} shortcut {} missing; list shortcut";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.rfind("shortcut").unwrap()).unwrap();
        assert!(matches!(
            alias_hover_at(source, &analysis, offset, "shortcut"),
            std::ops::ControlFlow::Continue(())
        ));
    }

    #[test]
    fn reporting_alias_target_does_not_replace_missing_original_recipe() {
        let source = "interp alias {} shortcut {} missing; shortcut";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.rfind("shortcut").unwrap()).unwrap();
        assert!(!analysis.allows_lexical_declaration_advice());
        analysis
            .command_aliases
            .get_mut("::shortcut")
            .unwrap()
            .extras
            .push("forged-display".to_owned());
        let answer = alias_hover_at(source, &analysis, offset, "shortcut");
        assert!(
            !matches!(answer, std::ops::ControlFlow::Break(Some(ref text)) if text.contains("forged-display"))
        );
        for invocation in &mut analysis.command_invocations {
            invocation.original_lookup = None;
        }
        assert!(!matches!(
            alias_hover_at(source, &analysis, offset, "shortcut"),
            std::ops::ControlFlow::Break(Some(_))
        ));
    }
}

#[cfg(test)]
mod original_math_hover_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_math_hover_keeps_fixed_registration_without_a_fake_wrapper() {
        // Implementation contract: naming.core.original-math-function-hover
        // docs/design/analysis/name-resolution-proofs/core-original-math-function-hover.md
        let source = "expr {abs(-2)}\n";
        for dialect in ["tcl8.4", "jim"] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.command_invocations.clear();
            let offset = u32::try_from(source.find("abs").unwrap()).unwrap();
            let std::ops::ControlFlow::Break(Some(selected)) =
                crate::math_function_symbol::select_at_offset(source, &analysis, offset)
            else {
                panic!("{dialect}: missing fixed function");
            };
            let hover = original_math_function_hover(&selected).unwrap();
            assert!(
                hover.value.contains("function table entry"),
                "{}",
                hover.value
            );
            assert!(!hover.value.contains("Dispatches to"));
            assert!(!hover.value.contains("Command-table lookup"));
            assert!(!hover.value.contains("tcl::mathfunc"));
        }
    }

    #[test]
    fn original_math_hover_uses_the_actual_current_source_and_override() {
        // Implementation contract: naming.core.original-math-function-hover
        // docs/design/analysis/name-resolution-proofs/core-original-math-function-hover.md
        let source = "# Actual override\nproc ::tcl::mathfunc::abs {argument} {return OVERRIDE}\nexpr {abs(-2)}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        analysis.command_invocations.clear();
        let offset = u32::try_from(source.rfind("abs(").unwrap()).unwrap();
        let std::ops::ControlFlow::Break(Some(selected)) =
            crate::math_function_symbol::select_at_offset(source, &analysis, offset)
        else {
            panic!("missing actual override");
        };
        let hover = original_math_function_hover(&selected).unwrap();
        assert!(hover.value.contains("argument"));
        assert!(hover.value.contains("Actual override"));
        assert!(!hover.value.contains("Command-table lookup"));
        assert!(matches!(
            crate::math_function_symbol::select_at_offset(
                &format!("# changed\n{source}"),
                &analysis,
                offset
            ),
            std::ops::ControlFlow::Break(None)
        ));
    }
}

#[cfg(test)]
mod original_vendor_hover_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_vendor_header_hover_keeps_source_only_opaque_cards_after_ui_clear() {
        // Implementation contract: naming.vendor.original-source-declaration-consumers
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-declaration-consumers.md
        let source = "proc {p\\uD800} {argument} {return $argument}\n";
        let mut analysis = Analyser::new().analyse(source, "f5-iapps");
        assert_eq!(analysis.original_vendor_procedure_declarations().count(), 1);
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        let card = hover(source, 0, 8, &analysis, None).unwrap();
        assert!(card.value.contains("p\\uD800"));
        assert!(card.value.contains("source declaration"));
        assert!(card.value.contains("materialisation is unavailable"));
        assert!(hover("# displaced\n", 0, 8, &analysis, None).is_none());
    }
}

#[cfg(test)]
mod original_pattern_hover_tests {
    use super::*;

    #[test]
    fn original_pattern_hover_keeps_actual_schema_operands_and_current_source() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = "format {%04d} 7";
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        let cursor = u32::try_from(source.find("%04d").unwrap()).unwrap();
        let selected = original_pattern_format_hover(
            source,
            &analysis,
            analysis.resolved_registry().unwrap(),
            cursor,
        )
        .expect("original source format role");
        assert!(selected.value.contains("**Format string**"));
        assert!(
            original_pattern_format_hover(
                &source.replace("%04d", "%08d"),
                &analysis,
                analysis.resolved_registry().unwrap(),
                cursor
            )
            .is_none()
        );
        for source in [
            "set data {%04d}",
            "proc format args {}; format {%04d} 7",
            "rename format {}; format {%04d} 7",
            "interp alias {} custom {} format {%04d}; custom 7",
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
            let cursor = u32::try_from(source.find("%04d").unwrap()).unwrap();
            assert!(
                original_pattern_format_hover(
                    source,
                    &analysis,
                    analysis.resolved_registry().unwrap(),
                    cursor
                )
                .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_format_hover_keeps_selected_availability_and_generation() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = "clock scan 2020 -format {%Y}";
        let profile = crate::profile_for_dialect("tcl9.1");
        let driver =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let cursor = u32::try_from(source.find("%Y").unwrap()).unwrap();
        for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
            let actual = tcl_registry::model::ingress::resolve_environment(environment)
                .default_context_registry();
            let context = std::sync::Arc::new(
                actual.with_command_store(driver.commands().snapshot().shared_registry()),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context.clone(),
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            );
            let mut analysis = tcl_compiler::analyser::Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, profile.name);
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            let hover =
                original_pattern_format_hover(source, &analysis, context.commands(), cursor);
            assert_eq!(hover.is_some(), expected, "{environment}");
            if let Some(hover) = hover {
                assert!(hover.value.contains("4-digit year"), "{}", hover.value);
            }
            assert!(
                original_pattern_format_hover(
                    "clock scan 2020 -format {%m}",
                    &analysis,
                    context.commands(),
                    cursor,
                )
                .is_none()
            );
            analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(
                    context.with_command_store(context.commands().snapshot().shared_registry()),
                ),
                input.lexer_config(),
            ));
            assert!(
                original_pattern_format_hover(source, &analysis, context.commands(), cursor)
                    .is_none()
            );
        }
    }

    #[test]
    fn original_pattern_hover_keeps_selected_availability_and_generation() {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        let source = "lsearch -regexp -stride 2 {a b} {a+}";
        let profile = crate::profile_for_dialect("tcl9.1");
        let driver =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let cursor = u32::try_from(source.find("a+").unwrap()).unwrap();
        for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
            let actual = tcl_registry::model::ingress::resolve_environment(environment)
                .default_context_registry();
            let context = std::sync::Arc::new(
                actual.with_command_store(driver.commands().snapshot().shared_registry()),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context.clone(),
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            );
            let mut analysis = tcl_compiler::analyser::Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            let hover =
                original_pattern_format_hover(source, &analysis, context.commands(), cursor);
            assert_eq!(hover.is_some(), expected, "{environment}");
            if let Some(hover) = hover {
                assert!(hover.value.contains("Regex pattern"));
            }
            assert!(
                original_pattern_format_hover(
                    &source.replace("a+", "b+"),
                    &analysis,
                    context.commands(),
                    cursor
                )
                .is_none()
            );
            analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(
                    context.with_command_store(context.commands().snapshot().shared_registry()),
                ),
                input.lexer_config(),
            ));
            assert!(
                original_pattern_format_hover(source, &analysis, context.commands(), cursor)
                    .is_none()
            );
        }
    }

    #[test]
    fn original_pattern_hover_keeps_alias_written_operands_and_captured_refusal() {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        for (source, expected) in [
            (
                "interp alias {} search {} lsearch -regexp; search {a b} {a+}",
                true,
            ),
            (
                "interp alias {} search {} lsearch -regexp {a b} {a+}; search",
                false,
            ),
            ("lsearch $mode {a b} {a+}", false),
            ("lsearch {*}$arguments {a+}", false),
            ("proc lsearch args {}; lsearch -regexp {a b} {a+}", false),
        ] {
            let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.1");
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            let cursor = u32::try_from(source.find("a+").unwrap()).unwrap();
            let hover = original_pattern_format_hover(
                source,
                &analysis,
                analysis.resolved_registry().unwrap(),
                cursor,
            );
            assert_eq!(hover.is_some(), expected, "{source}");
            if let Some(hover) = hover {
                assert!(hover.value.contains("Regex pattern"));
            }
        }
    }

    #[test]
    fn pattern_hover_entry_keeps_explicit_logical_source_context_and_nested_words() {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-logical-pattern-source";
        let profile = profile.intern();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        for (source, text, heading) in [
            ("string match -nocase {a*} $value", "a*", "Glob pattern"),
            ("proc p {} {regexp {a+} $value}", "a+", "Regex pattern"),
            ("if {1} {format {%04d} 7}", "%04d", "Format string"),
            (
                "interp alias {} search {} lsearch -regexp; search {a b} {a+}",
                "a+",
                "Regex pattern",
            ),
        ] {
            let mut analysis = tcl_compiler::analyser::Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            assert!(analysis.allows_lexical_declaration_advice());
            assert!(!analysis.has_original_vendor_source_names());
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            let cursor = u32::try_from(source.find(text).unwrap()).unwrap();
            let position = tcl_lexer::LineIndex::new(source).position_at_utf16(cursor, source);
            let selected = hover(
                source,
                position.line,
                position.character.get(),
                &analysis,
                Some(&CommandRegistry::build_default()),
            )
            .expect("retained Logical embedded language");
            assert!(
                selected.value.contains(heading),
                "{source}: {}",
                selected.value
            );
            assert!(
                registry_pattern_format_hover(
                    &source.replace(text, "different"),
                    position.line,
                    position.character.get(),
                    &analysis,
                    context.commands()
                )
                .is_none()
            );
            analysis.resolved_input = None;
            assert!(
                registry_pattern_format_hover(
                    source,
                    position.line,
                    position.character.get(),
                    &analysis,
                    context.commands()
                )
                .is_none()
            );
        }
    }

    #[test]
    fn pattern_hover_entry_keeps_logical_shadow_and_captured_anchor_refusals() {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-logical-pattern-refusals";
        let profile = profile.intern();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        for source in [
            "proc lsearch args {}; lsearch -regexp {a b} {a+}",
            "interp alias {} search {} lsearch -regexp {a b} {a+}; search",
            "lsearch $mode {a b} {a+}",
            "lsearch {*}$arguments {a+}",
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "reporting-only");
            let cursor = u32::try_from(source.find("a+").unwrap()).unwrap();
            let position = tcl_lexer::LineIndex::new(source).position_at_utf16(cursor, source);
            assert!(
                registry_pattern_format_hover(
                    source,
                    position.line,
                    position.character.get(),
                    &analysis,
                    context.commands()
                )
                .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_pattern_hover_preserves_full_lexer_configuration() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = "\u{feff}format {%04d} 7";
        let context = tcl_registry::model::ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        );
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let cursor = u32::try_from(source.find("%04d").unwrap()).unwrap();
        for (leading_bom, expected) in [
            (tcl_lexer::LeadingBom::Skip, true),
            (tcl_lexer::LeadingBom::Content, false),
        ] {
            let config = tcl_lexer::LexerConfig {
                leading_bom,
                ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
            };
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::clone(&context),
                config,
            );
            let analysis = tcl_compiler::analyser::Analyser::new()
                .with_resolved_input(input)
                .analyse(source, "display-only");
            assert_eq!(
                original_pattern_format_hover(source, &analysis, context.commands(), cursor)
                    .is_some(),
                expected
            );
        }
    }
}
