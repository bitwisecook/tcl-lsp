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

//! Cards for variables: reads and declarations, special variables, inferred type and taint.

use super::Hover;
use super::embedded_operands::registry_pattern_format_hover;
use tcl_compiler::analyser::{AnalysisResult, VarDef};
use tcl_compiler::compilation_unit::FunctionUnit;
use tcl_compiler::taint::{TaintColour, TaintLattice};
use tcl_compiler::types::{TclType, TypeKind};
use tcl_dialect::model::{SpecSurface, SurfaceQuery, surface_admits};
use tcl_registry::CommandRegistry;

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
pub(super) fn variable_hover(
    source: &str,
    (line, character): (u32, u32),
    line_index: &tcl_lexer::LineIndex,
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
    profile: &'static tcl_dialect::DialectProfile,
    annotate: bool,
) -> Option<Hover> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let context = current.context();
    let registry = Some(current.registry());
    let annotations = registry.filter(|_| annotate);
    let dialect = Some(context.context().authoring_query());
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
                var_type_annotations(source, analysis, var_byte_offset, annotations);
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
        if let Some(spec) = current
            .registry()
            .special_var_in_dialect(&var_name, dialect)
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
        var_type_annotations(source, analysis, decl_byte_offset, annotations);
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
pub(super) fn variable_position_hover(
    source: &str,
    (line, character): (u32, u32),
    line_index: &tcl_lexer::LineIndex,
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
    profile: &'static tcl_dialect::DialectProfile,
    annotate: bool,
) -> PositionHover {
    if !analysis.allows_lexical_declaration_advice() {
        return PositionHover::FallThrough;
    }
    if let Some(hover) = variable_hover(
        source,
        (line, character),
        line_index,
        analysis,
        ctx,
        profile,
        annotate,
    ) {
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
pub(super) enum PositionHover {
    /// The position resolved to this card.
    Answer(Hover),
    /// The position forbids every word-based resolver from answering.
    Abstain,
    /// Not a variable position — try the word-based resolvers.
    FallThrough,
}

// An unresolved variable position remains terminal. The special-variable card
// needs a genuine selected original root and an independently current Registry;
// it does not turn an unavailable variable or a copied label into a lookup.
pub(super) fn current_special_variable_source_hover(
    source: &str,
    analysis: &AnalysisResult,
    occurrence: &tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
    cursor: u32,
) -> Option<Hover> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    selected_root_special_variable_source_hover(source, analysis, occurrence, cursor)
}

pub(super) fn selected_root_special_variable_source_hover(
    source: &str,
    analysis: &AnalysisResult,
    occurrence: &tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
    cursor: u32,
) -> Option<Hover> {
    use tcl_compiler::signature_scan::scope::SignatureSourceNameInput;
    use tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    if !crate::variable_symbol::declaration_spans(analysis, occurrence.symbol()).is_empty() {
        return None;
    }
    let SignatureSourceNameInput::OriginalVariableRoot(root) = occurrence.original_name_input()
    else {
        return None;
    };
    let reference =
        crate::source_structure::original_variable_reference_at(source, analysis, cursor)?;
    if root.name_span()? != reference.name_span || root.part_span() != reference.whole_span {
        return None;
    }
    let name = match occurrence.symbol().slot() {
        SignatureSourceVariableSlot::C { namespace, simple } if namespace.is_root() => {
            std::str::from_utf8(simple.as_bytes()).ok()?
        }
        SignatureSourceVariableSlot::Jim(key) => std::str::from_utf8(key.as_bytes()).ok()?,
        SignatureSourceVariableSlot::C { .. } | SignatureSourceVariableSlot::Local { .. } => {
            return None;
        }
    };
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let context = current.context();
    let dialect = Some(context.context().authoring_query());
    let spec = current.registry().special_var_in_dialect(name, dialect)?;
    Some(Hover::markdown(special_var_hover_text(spec, dialect)))
}

pub(super) fn navigation_variable_hover(
    source: &str,
    analysis: &AnalysisResult,
    selection: &crate::variable_symbol::NavigationVariable<'_>,
    cursor_offset: u32,
    ctx: crate::definition::CallResolution<'_>,
    annotate: bool,
) -> Option<Hover> {
    use crate::variable_symbol::NavigationVariable;
    match selection {
        NavigationVariable::NativeRootSourceReference(reference) => {
            return selected_root_special_variable_source_hover(
                source,
                analysis,
                reference.occurrence(),
                cursor_offset,
            );
        }
        NavigationVariable::LogicalRootSourceReference(reference) => {
            return current_special_variable_source_hover(
                source,
                analysis,
                reference.occurrence(),
                cursor_offset,
            );
        }
        NavigationVariable::Original(occurrence)
            if crate::variable_symbol::declaration_spans(analysis, occurrence.symbol())
                .is_empty() =>
        {
            return current_special_variable_source_hover(
                source,
                analysis,
                occurrence,
                cursor_offset,
            );
        }
        _ => {}
    }
    let mut text = selection.hover_text(analysis)?;
    if matches!(selection, NavigationVariable::Original(_)) {
        let (type_info, taint_info) = var_type_annotations(
            source,
            analysis,
            cursor_offset,
            ctx.registry.filter(|_| annotate),
        );
        append_var_annotations(&mut text, type_info.as_deref(), taint_info.as_deref());
    }
    Some(Hover::markdown(text))
}

/// Annotation queries retain the selected source occurrence and current input.
fn var_type_annotations(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    registry: Option<&CommandRegistry>,
) -> (Option<String>, Option<String>) {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return (None, None);
    };
    // The annotations come from the compiler pipeline, which a caller opts
    // into by supplying its registry; it must be the analysis's own.
    let Some(registry) = registry else {
        return (None, None);
    };
    if registry.snapshot().semantic_key() != current.registry().snapshot().semantic_key() {
        return (None, None);
    }
    infer_var_type_and_taint(source, current.registry(), cursor, analysis)
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
pub(super) fn var_hover_text(
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
    append_var_annotations(&mut text, type_info, taint_info);
    if let Some(f) = format_info {
        let _ = write!(text, "\n\n{f}");
    }
    text
}

fn append_var_annotations(text: &mut String, type_info: Option<&str>, taint_info: Option<&str>) {
    use std::fmt::Write as _;
    if let Some(value) = type_info {
        let _ = write!(text, "\n\n**Inferred intrep**: {value}");
    }
    if let Some(value) = taint_info {
        let _ = write!(text, "\n\n**Taint**: {value}");
    }
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
    cursor: u32,
    analysis: &AnalysisResult,
) -> (Option<String>, Option<String>) {
    let Some(unit) =
        crate::original_invocation::source_compilation_unit(source, analysis, registry)
    else {
        return (None, None);
    };
    let selected = if let Some(reference) =
        crate::source_structure::original_variable_reference_at(source, analysis, cursor)
    {
        tcl_compiler::ssa::original_source_read_at_unit(&unit, reference.whole_span, registry)
            .and_then(|read| Some((read.function(), (read.symbol(), read.version()?))))
    } else {
        let declaration = analysis
            .original_variable_symbol_at_offset(cursor)
            .filter(|occurrence| occurrence.is_declaration())
            .map(tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence::span)
            .or_else(|| {
                crate::definition::var_def_at_declaration_offset(&analysis.global_scope, cursor)
                    .map(|definition| definition.definition_span)
            });
        declaration
            .and_then(|span| {
                tcl_compiler::ssa::original_source_definition_at_unit(&unit, span, registry)
            })
            .map(|definition| {
                (
                    definition.function(),
                    (definition.symbol(), definition.version()),
                )
            })
    };
    let Some((function, value)) = selected else {
        return (None, None);
    };
    let first_use =
        tcl_compiler::shimmer::first_use_value_commitments_for_function(function, registry);
    (
        infer_var_type(function, value, first_use.get(&value).copied()),
        infer_var_taint(function, value),
    )
}

/// Type display from the selected represented SSA contents version.
fn infer_var_type(
    function: &FunctionUnit,
    value: tcl_compiler::ssa::ValueKey,
    first_use: Option<TclType>,
) -> Option<String> {
    let lattice = function.types.get(&value)?;
    match lattice.kind() {
        TypeKind::Known => {
            let label = tcl_type_label(lattice.tcl_type()?);
            if let Some(committed) = first_use {
                let committed = tcl_type_label(committed);
                if committed != label {
                    return Some(format!("{label} (first used as: {committed})"));
                }
            }
            Some(label)
        }
        TypeKind::Shimmered => {
            let members = lattice
                .shapes()
                .iter()
                .map(|shape| tcl_type_label(shape.coarse()))
                .collect::<Vec<_>>();
            Some(format!("shimmered ({})", members.join(" / ")))
        }
        _ => None,
    }
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

/// Taint display from the same represented version as the source read.
fn infer_var_taint(function: &FunctionUnit, value: tcl_compiler::ssa::ValueKey) -> Option<String> {
    let taint = *function.taints.get(&value)?;
    if !taint.is_tainted() {
        return None;
    }
    let labels = taint_colour_labels(taint);
    if labels.is_empty() {
        Some("tainted (from I/O)".to_owned())
    } else {
        Some(format!("tainted (from I/O); {}", labels.join(", ")))
    }
}
