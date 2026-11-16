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

//! Renaming variables: declarations and reads, namespace variables, reference spelling and the conditional source rename.

use super::{PrepareRename, TextEdit, is_safe_symbol_name, span_to_range};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

/// A variable a `TclOO` class declares with `variable NAME` is linked into
/// every method body (tclsh `8.6`/`9.0`: `oo::class create C {variable n; method
/// get {} {return $n}; method set {x} {set n $x}}` shares one `n` per object).
/// The original variable symbol of a method-body read does not yet unify the
/// class declaration and the other methods' uses, so renaming it would edit
/// one occurrence and break the program; the rename is refused instead.
pub(super) fn class_variable_rename_refusal(
    source: &str,
    analysis: &AnalysisResult,
    occurrence: &tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
    line_index: &LineIndex,
) -> Option<crate::rename_safety::RenameRefusal> {
    let span = occurrence.span();
    let name = source.get(span.as_range())?;
    let name = name.strip_prefix('$').unwrap_or(name);
    let name = name
        .strip_prefix('{')
        .and_then(|inner| inner.strip_suffix('}'))
        .unwrap_or(name);
    // Inside the class body every method shares the declared variable, and
    // the `variable NAME` declaration itself is one of its occurrences.
    let declared = analysis.all_classes.values().any(|class| {
        class.variables.iter().any(|variable| variable == name)
            && class.body_span.start() <= span.start()
            && span.end() <= class.body_span.end()
    });
    declared.then(|| crate::rename_safety::RenameRefusal {
        reason: format!(
            "cannot rename `{name}`: it is a class-declared variable linked into every method, \
             and its declaration and other method uses are not yet one symbol"
        ),
        range: Some(span_to_range(source, line_index, span)),
    })
}

/// The variable-rename edits for a cursor that names one, or `None` when it
/// names no variable at all.
///
/// Two cursor shapes, both variables: a `$ref` occurrence, and a *declaration
/// / same-cell write* token with no `$` (a `set x` / `variable x`, a
/// proc/method parameter, a `catch` result-var).  The second needs
/// `var_def_at_declaration_offset`'s byte-offset span search — a parameter's
/// own declaring token sits textually *before* its scope's body span even
/// starts, so the ordinary scope-chain walk never reaches it (see that
/// function's own doc).
pub(super) fn rename_variable_at(
    source: &str,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Vec<TextEdit>> {
    let def_byte = crate::definition::byte_offset_at(line_index, source, line, character);
    // Shared `$ref` gate — see `substituting_var_at_position`.  A cursor
    // inside a brace-quoted name word falls through to the declaration-span
    // search, so it names the literal cell; the refusal gate in
    // `rename_with_diagnosis` has already stopped it before this point, but
    // resolving it correctly here keeps the two from disagreeing.
    let name = if let Some(var_name) =
        crate::definition::substituting_var_at_position(source, analysis, line, character, def_byte)
    {
        var_name
    } else {
        crate::definition::var_def_at_declaration_offset(&analysis.global_scope, def_byte)?
            .name
            .clone()
    };
    Some(rename_var(
        source, line, character, new_name, analysis, line_index, &name,
    ))
}

/// Variable-rename path — declaration span + every read site,
/// with brace-ref escaping (`$x` / `${x}` / `$ns::x` /
/// `${ns::x}` keep their leader characters and any namespace
/// qualifier).
pub(super) fn rename_var(
    source: &str,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    var_name: &str,
) -> Vec<TextEdit> {
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    if !analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config) {
        return Vec::new();
    }
    let byte_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    // The ordinary scope-chain lookup resolves a `$ref` cursor (and a
    // definition-site cursor that happens to sit inside its own scope's
    // body span); a proc/method parameter's own declaring token sits
    // *before* its scope's body span even starts, so fall back to the
    // byte-offset span search there (see `var_def_at_declaration_offset`'s
    // own doc).
    let Some(var_def) =
        crate::definition::lookup_var_in_scope_chain(analysis, source, byte_offset, var_name)
            .or_else(|| {
                crate::definition::var_def_at_declaration_offset(
                    &analysis.global_scope,
                    byte_offset,
                )
                .filter(|v| v.name == var_name)
            })
    else {
        return Vec::new();
    };
    // Collision gate: refuse when `new_name` already resolves to a *different*
    // variable visible at the cursor (e.g. a sibling `set y` in the same proc
    // scope) — renaming would merge two distinct variables.
    if let Some(existing) =
        crate::definition::lookup_var_in_scope_chain(analysis, source, byte_offset, new_name)
        && existing.definition_span != var_def.definition_span
    {
        return Vec::new();
    }
    let Some(declaration) =
        crate::variable_symbol::declaration_at_cursor(source, analysis, var_def, byte_offset)
    else {
        return Vec::new();
    };
    // A conditional alias supplies readonly navigation, never an editable
    // reference set. The original constructor separately checks source coverage,
    // collisions and every original receiver/container before producing edits.
    if declaration.original_occurrence().is_none() {
        return Vec::new();
    }
    crate::variable_symbol::original_variable_rename_edits(
        source,
        analysis,
        declaration.symbol(),
        new_name,
    )
    .unwrap_or_default()
}

/// Source edits for an independently retained namespace symbol matching one
/// checked compatibility coordinate. The caller string cannot manufacture the
/// symbol, and alias reports supply no reference-set or edit authority.
#[must_use]
pub fn namespace_variable_rename_edits(
    source: &str,
    analysis: &AnalysisResult,
    cell: &str,
    new_name: &str,
) -> Vec<TextEdit> {
    if !is_safe_symbol_name(new_name) {
        return Vec::new();
    }
    let Some(symbol) =
        crate::variable_symbol::namespace_symbol_for_compatibility_key(source, analysis, cell)
    else {
        return Vec::new();
    };
    crate::variable_symbol::original_variable_rename_edits(source, analysis, symbol, new_name)
        .unwrap_or_default()
}

/// Token-only compatibility geometry controls. Actual rename consumers use
/// the original selected component's whole span rather than widening tokens.
#[cfg(test)]
pub(super) fn var_ref_edit_span(source: &str, span: tcl_lexer::Span) -> tcl_lexer::Span {
    tcl_lexer::word_span_at(source, span)
}

/// Selected whole-reference source projection. The caller's independent
/// Logical rename gates supply edit authority; this helper supplies syntax
/// and refuses malformed, compound or unavailable source geometry.
#[cfg(test)]
pub(super) fn build_var_ref_replacement(
    source: &str,
    span: tcl_lexer::Span,
    new_tail: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    if source.as_bytes().get(span.start() as usize) != Some(&b'$') {
        return None;
    }
    let original =
        tcl_lexer::scan_var_ref(source.as_bytes(), span.start() as usize, config).ok()??;
    let whole = tcl_lexer::Span::new(span.start(), u32::try_from(original.next).ok()?);
    if span != whole
        && Some(span) != original.source_span(source.as_bytes(), span.start() as usize, 0)
    {
        return None;
    }
    let text = source.get(whole.as_range())?;
    tcl_syntax::naming::variable_source_reference_with_tail(text, new_tail, config)
}

#[cfg(test)]
pub(super) fn build_var_replacement_in_analysis(
    source: &str,
    analysis: &AnalysisResult,
    span: tcl_lexer::Span,
    new_tail: &str,
) -> Option<(tcl_lexer::Span, String)> {
    let config = analysis.body_lexer_config?;
    if !analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config) {
        return None;
    }
    if let Some(reference) =
        crate::source_structure::original_variable_reference_at(source, analysis, span.start())
    {
        if reference.token_span != span && reference.whole_span != span {
            return None;
        }
        return Some((
            reference.whole_span,
            build_var_ref_replacement(source, span, new_tail, config)?,
        ));
    }
    let text = source.get(span.as_range())?;
    // A sigil in non-data source must have a genuine selected variable
    // component. A literal name word can itself contain dollar bytes.
    if text.starts_with('$') && !crate::definition::offset_is_inert(source, analysis, span.start())?
    {
        return None;
    }
    let (name, suffix) = split_array_suffix(text);
    let qualifier = source_name_qualifier(name);
    Some((span, format!("{qualifier}{new_tail}{suffix}")))
}

pub(super) fn source_name_qualifier(name: &str) -> &str {
    // Original written bytes, not a constructed holder or native identity.
    // The shared tail consumes every colon in its final separator run.
    let tail = tcl_syntax::naming::written_command_tail(name.as_bytes());
    &name[..name.len() - tail.len()]
}

/// Closed literal element partition shared with variable naming. An unmatched
/// open or trailing text belongs to the scalar name and has no index suffix.
#[cfg(test)]
pub(super) fn split_array_suffix(name: &str) -> (&str, &str) {
    tcl_syntax::naming::split_element_ref(name)
        .map_or((name, ""), |(root, _)| (root, &name[root.len()..]))
}

/// Plan a conditional, single-document source alpha rename. The caller must
/// present the separate source-only conditions before accepting the edit.
///
/// # Errors
/// Refuses when the source, selected occurrence or proposed name cannot support
/// a conditional source-only proposal.
pub fn conditional_variable_rename_with_diagnosis(
    source: &str,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
) -> Result<
    crate::variable_symbol::ConditionalVariableRenameEdits,
    crate::rename_safety::RenameRefusal,
> {
    let refuse = |reason: &str| crate::rename_safety::RenameRefusal {
        reason: reason.to_owned(),
        range: None,
    };
    crate::original_context::CurrentSourceContext::capture(source, analysis)
        .ok_or_else(|| refuse("source analysis does not match the current document and input"))?;
    if !is_safe_symbol_name(new_name) {
        return Err(refuse("the replacement is not a safe variable name"));
    }
    let index = LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&index, source, line, character);
    if crate::namespace_symbol::select_at_offset(source, analysis, cursor).is_break() {
        return Err(refuse("the selected occurrence is not a variable"));
    }
    let std::ops::ControlFlow::Break(Some(occurrence)) =
        crate::variable_symbol::select(source, analysis, line, character)
    else {
        return Err(refuse("cannot prove the original source variable"));
    };
    if let Some(refusal) = class_variable_rename_refusal(source, analysis, occurrence, &index) {
        return Err(refusal);
    }
    // Keep the complete Native route authoritative when it is available.
    if analysis.original_variable_rename_is_complete(occurrence.symbol()) {
        return Err(refuse("the normal rename route owns this variable"));
    }
    crate::variable_symbol::conditional_variable_rename_edits(
        source,
        analysis,
        occurrence.symbol(),
        new_name,
    )
}

/// Prepare only when a complete conditional proposal can be formed. The real
/// name still requires a fresh certificate in the rename request.
#[must_use]
pub fn prepare_conditional_variable_rename(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<PrepareRename> {
    let std::ops::ControlFlow::Break(Some(occurrence)) =
        crate::variable_symbol::select(source, analysis, line, character)
    else {
        return None;
    };
    let mut admitted = false;
    for candidate in ["__tcl_lsp_source_alpha_1", "__tcl_lsp_source_alpha_2"] {
        if conditional_variable_rename_with_diagnosis(source, line, character, candidate, analysis)
            .is_ok_and(|proposal| !proposal.edits.is_empty())
        {
            admitted = true;
            break;
        }
    }
    if !admitted {
        return None;
    }
    let span = occurrence.rename_span()?;
    Some(PrepareRename {
        range: span_to_range(source, &LineIndex::new(source), span),
        placeholder: source.get(span.as_range())?.to_owned(),
    })
}
