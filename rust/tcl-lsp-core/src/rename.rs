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

//! Rename plans for current original symbols and explicit lexical compatibility.
//!
//! Native namespace, variable, command and member selection uses the shared
//! typed owners. Plans require current source/configuration, complete occurrence
//! coverage, unoccupied replacements and independently validated edit geometry.
//! Missing identity or coverage refuses the entire edit set.
//!
//! Lexical compatibility uses separate ASCII-name and builtin-collision gates.
//! The server validates every participating document before combining shared
//! plans; an empty result and an explicit safety refusal remain distinct.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;
use tcl_registry::CommandRegistry;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;
use crate::references::resolve_member_span;

mod commands;
mod members;
mod variables;

pub use commands::{cross_document_symbol_edits, workspace_symbol_rename_edits};
#[cfg(test)]
use commands::{invocation_replacement, qualified_and_decl_text};
use commands::{prepare_named_command_rename, rename_named_command};
use members::{
    gated_untargeted_member_rename, member_family_spans, method_rename_refusal,
    prepare_list_built_self_method_rename, rename_itcl_class_proc, rename_list_built_self_method,
    rename_method_in_class,
};
pub use members::{
    inherited_method_spans_in_document, method_rename_target, method_spans_in_document,
    method_target_with_access, method_target_with_access_in_workspace,
    untargeted_member_rename_target,
};
#[cfg(test)]
use variables::{
    build_var_ref_replacement, build_var_replacement_in_analysis, rename_var, split_array_suffix,
    var_ref_edit_span,
};
use variables::{class_variable_rename_refusal, rename_variable_at};
pub use variables::{
    conditional_variable_rename_with_diagnosis, namespace_variable_rename_edits,
    prepare_conditional_variable_rename,
};

/// One text edit in a rename — span plus replacement text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextEdit {
    /// Range to replace (byte spans translated to LSP
    /// line/character coordinates).
    pub range: LspRange,
    /// Replacement text.
    pub new_text: String,
}

/// `true` when `name` matches the safe-symbol shape
/// `^[A-Za-z_][A-Za-z0-9_]*$`.
///
/// Used by [`rename`] to reject new names that contain
/// whitespace, punctuation, leading digits, or other characters
/// that would produce invalid Tcl symbol references.
#[must_use]
pub fn is_safe_symbol_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `true` when `name` is registered as a built-in command in
/// `registry`.
///
/// Used by [`rename`] to refuse proc renames that would
/// overwrite a built-in command name (the editor would then
/// silently dispatch the user's calls to the built-in instead
/// of the renamed proc).
#[must_use]
pub fn is_builtin_command_name(name: &str, registry: &CommandRegistry) -> bool {
    registry.command_names().any(|n| n == name)
}

/// Result of [`prepare_rename`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareRename {
    /// Range of the symbol the rename will affect.
    pub range: LspRange,
    /// Suggested placeholder (the symbol's current tail name)
    /// the editor pre-fills in its rename input box.
    pub placeholder: String,
}

/// The cursor word's own range + text as a [`PrepareRename`] — the
/// consumer-document fall-through.  [`prepare_rename`] anchors on a
/// *local* declaration; when the document has none (the symbol is defined in
/// a sibling / autoloaded library file), the server may still accept the
/// rename after resolving the word through the workspace index — the range
/// the editor highlights is then the call-site word itself.
#[must_use]
pub fn word_prepare_at(source: &str, line: u32, character: u32) -> Option<PrepareRename> {
    let (word, start, end) = find_word_span_at_position(source, line, character)?;
    Some(PrepareRename {
        range: crate::definition::LspRange {
            start_line: line,
            start_character: start,
            end_line: line,
            end_character: end,
        },
        placeholder: word,
    })
}

/// Presentation range for a workspace call site, using the original configured
/// word. The host separately resolves the symbol before presenting a rename.
#[must_use]
pub fn original_word_prepare_at(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<PrepareRename> {
    let cursor =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)?
            .offset();
    let word = crate::source_structure::original_complete_cursor_word(source, analysis, cursor)?;
    Some(PrepareRename {
        range: span_to_range(source, &LineIndex::new(source), word.selection_span(cursor)),
        placeholder: word.static_unicode_value()?,
    })
}

/// Validate that the cursor sits on a renameable symbol and
/// return the symbol's range + placeholder text.  Editors
/// call this before `rename` to determine whether to show the
/// rename UI.
#[must_use]
pub fn prepare_rename(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<PrepareRename> {
    prepare_rename_in_program(
        source,
        line,
        character,
        analysis,
        crate::definition::CallResolution::document_only(),
    )
}

/// [`prepare_rename`] with the caller's whole-program export view attached —
/// the entry point a host with a workspace index should call.
///
/// A `namespace import -force` whose covering `namespace export` lives in
/// another file changes *which* definition a bare call site denotes, so a
/// rename started from that call must retarget with it.
/// When the reached definition is not in this document the resolver answers
/// `None` and the rename abstains — the safe outcome, and far better than
/// rewriting the unrelated local proc the shadow deleted.
#[must_use]
pub fn prepare_rename_in_program(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Option<PrepareRename> {
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let line_index = LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
    if crate::definition::program_forced_import_abstains(analysis, resolution.program, cursor) {
        return None;
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::namespace_symbol::select_at_offset(source, analysis, cursor)
    {
        selected?;
        let span = crate::namespace_rename::original_namespace_rename_span_at_offset(
            source, analysis, cursor,
        )?;
        return Some(PrepareRename {
            range: span_to_range(source, &line_index, span),
            placeholder: source.get(span.as_range())?.to_owned(),
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::variable_symbol::select(source, analysis, line, character)
    {
        let occurrence = selected?;
        if !analysis.original_variable_rename_is_complete(occurrence.symbol()) {
            return None;
        }
        let span = occurrence.rename_span()?;
        return Some(PrepareRename {
            range: span_to_range(source, &line_index, span),
            placeholder: source.get(span.as_range())?.to_owned(),
        });
    }
    if !analysis.allows_lexical_declaration_advice() {
        let std::ops::ControlFlow::Break(Some(identity)) =
            crate::original_declaration::select_at_offset("", source, analysis, cursor)
        else {
            return None;
        };
        let document = crate::original_declaration::OriginalDeclarationDocument {
            uri: "",
            source,
            analysis,
        };
        let span = if matches!(
            identity.role(),
            crate::original_declaration::OriginalDeclarationRole::Method(_)
        ) {
            crate::original_member_rename::original_member_prepare_span(document, &identity, cursor)
                .ok()?
        } else {
            crate::original_command_rename::original_command_prepare_span(
                document, &identity, cursor,
            )
            .ok()?
        };
        return Some(PrepareRename {
            range: span_to_range(source, &line_index, span),
            placeholder: source.get(span.as_range())?.to_owned(),
        });
    }
    if crate::receiver_identity::definition_reference_at_cursor(analysis, source, cursor).is_some()
    {
        let selected = crate::receiver_identity::method_at_cursor(analysis, source, cursor)?;
        let span = selected.editable_selector()?;
        return Some(PrepareRename {
            range: span_to_range(source, &line_index, span),
            placeholder: selected.method.name.clone(),
        });
    }
    prepare_lexical_rename(source, &line_index, (line, character), analysis, resolution)
}

fn prepare_lexical_rename(
    source: &str,
    line_index: &LineIndex,
    position: (u32, u32),
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Option<PrepareRename> {
    let (line, character) = position;
    // A namespace name is renameable, and the check must come
    // first: the word resolvers below would otherwise anchor prepare on a
    // same-spelled proc or class in a completely different place in the file.
    //
    // The range offered is the **final segment** of the namespace inside the
    // word under the cursor, because that is the only byte range the rename
    // rewrites — offering the whole `::app::old` word would have the editor
    // pre-fill a placeholder the rename never replaces.  A word that does not
    // spell that segment (a relative `sub` inside the namespace itself) is not
    // an anchor for it, so prepare declines and the editor offers no UI there.
    if let Some(cell) =
        crate::namespace_symbol::namespace_cell_at(source, analysis, line, character)
    {
        let cursor = crate::definition::byte_offset_at(line_index, source, line, character);
        return crate::namespace_rename::namespace_segment_at(source, analysis, cursor, &cell).map(
            |span| PrepareRename {
                range: span_to_range(source, line_index, span),
                placeholder: source[span.start() as usize..span.end() as usize].to_owned(),
            },
        );
    }
    // Variable?  Shared `$ref` gate: a `$name`-shaped substring in a comment
    // or a data brace is not a reference, so it is not renameable either —
    // and neither is the `$n` inside a brace-quoted *name* word.
    let byte_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    if let Some(var_name) = crate::definition::substituting_var_at_position(
        source,
        analysis,
        line,
        character,
        byte_offset,
    ) && let Some(var_def) =
        crate::definition::lookup_var_read_at(analysis, source, byte_offset, &var_name)
    {
        return Some(PrepareRename {
            range: span_to_range(source, line_index, var_def.definition_span),
            placeholder: var_def.name.clone(),
        });
    }
    // Variable definition / same-cell write site (`set x` / `variable x` /
    // a proc-param / a `catch` result-var) — no `$`, so resolve directly
    // via `var_def_at_declaration_offset`'s byte-offset span search (see
    // its own doc for why the ordinary scope-chain walk can't reach a
    // parameter's own declaring token).
    let def_byte = crate::definition::byte_offset_at(line_index, source, line, character);
    if let Some(var_def) =
        crate::definition::var_def_at_declaration_offset(&analysis.global_scope, def_byte)
    {
        return Some(PrepareRename {
            range: span_to_range(source, line_index, var_def.definition_span),
            placeholder: var_def.name.clone(),
        });
    }
    // Proc?  Declaration-span hit, else the namespace-aware candidate
    // resolution — never a namespace-blind `p.name == word` first-hit scan
    // (which could seed the rename with an arbitrary same-named proc).
    let word_off = crate::definition::byte_offset_at(line_index, source, line, character);
    let word = crate::source_structure::original_cursor_word_value(source, analysis, word_off)?;
    if let Some(preparation) =
        prepare_list_built_self_method_rename(source, analysis, &word, word_off)
    {
        return Some(preparation);
    }
    if let Some(preparation) =
        prepare_named_command_rename(source, line_index, analysis, resolution, (&word, word_off))
    {
        return Some(preparation);
    }
    // Method / classmethod / property inside a class body?
    let cursor_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    if let Some(class_def) = crate::definition::enclosing_class_at(analysis, cursor_offset)
        .and_then(|q| analysis.all_classes.get(q))
        && let Some((_, span)) = resolve_member_span(class_def, &word, cursor_offset)
    {
        return Some(PrepareRename {
            range: span_to_range(source, line_index, span),
            placeholder: word.clone(),
        });
    }
    if let Some(selected) =
        crate::receiver_identity::method_at_cursor(analysis, source, cursor_offset)
        && let Some(selector) = selected.editable_selector()
    {
        return Some(PrepareRename {
            range: span_to_range(source, line_index, selector),
            placeholder: selected.method.name.clone(),
        });
    }
    None
}

/// Compute rename text edits.
///
/// Explicit safety refusals become an empty edit set; use
/// [`rename_with_diagnosis`] when the caller needs the refusal reason.
#[must_use]
pub fn rename(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> Vec<TextEdit> {
    rename_with_diagnosis(
        source, dialect, line, character, new_name, analysis, registry,
    )
    .unwrap_or_default()
}

/// Explain why the current cursor cannot prepare a complete rename.
#[must_use]
pub fn prepare_rename_refusal(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> crate::rename_safety::RenameRefusal {
    let reason = if crate::original_context::CurrentSourceContext::capture(source, analysis)
        .is_none()
    {
        "cannot prove a complete current source context for this rename"
    } else if crate::variable_symbol::select(source, analysis, line, character).is_break() {
        "cannot account for every original variable name, alias target and read in this document"
    } else {
        "cannot prove a unique editable symbol and complete references at this position"
    };
    crate::rename_safety::RenameRefusal {
        reason: reason.to_owned(),
        range: None,
    }
}

/// Rename with a visible refusal for both unsafe and inapplicable requests.
pub fn rename_outcome(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> tcl_core_types::EditorOutcome<Vec<TextEdit>, LspRange> {
    let edits = rename_with_diagnosis(
        source, dialect, line, character, new_name, analysis, registry,
    )?;
    if edits.is_empty() {
        let mut refusal = prepare_rename_refusal(source, line, character, analysis);
        if !is_safe_symbol_name(new_name) {
            "the requested name cannot be written safely by this rename"
                .clone_into(&mut refusal.reason);
        }
        return Err(refusal);
    }
    Ok(edits)
}

/// Every refusal that must be decided **before** any edit is built, in order.
///
/// Grouped because they share that one property: a hazard here can never be
/// allowed to leak a partial edit set, so all of them run ahead of the first
/// `rename_*` tier rather than being interleaved with them.
fn pre_edit_refusal(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<crate::rename_safety::RenameRefusal> {
    // When the cursor names a `TclOO` member: refuse outright if any dispatch
    // of it in this document cannot be proved safe to rewrite, or if the
    // *requested name* would turn a `renamemethod` into one that
    // aborts the whole class definition.
    method_rename_refusal(
        source, dialect, line, character, new_name, analysis, line_index,
    )
    // A namespace name is answered by its own tier
    // ([`crate::namespace_rename`]), and a refusal from it must
    // stay a refusal: `Ok(vec![])` falls through to the server's
    // workspace-resolved rename branch, which resolves the word as a
    // *command* and would rewrite a same-spelled proc instead.  The cursor is
    // provably on a registry-declared
    // `ArgRole::NamespaceName` argument, so it names a namespace and nothing
    // else.
    .or_else(|| namespace_rename_refusal(source, dialect, line, character, new_name, analysis))
    // A variable whose *name* can only be written quoted (`set {$n} 1`) cannot
    // be renamed by substituting into the recorded spans — see the gate's own
    // doc.  Decided before `rename_variable_at`, whose
    // `$`-reference cursor search would otherwise read the `$n` *inside* the
    // braces as a reference to the unrelated variable `n` and rename that one.
    .or_else(|| {
        crate::rename_safety::literal_name_variable_rename_refusal(
            source, line, character, analysis, line_index,
        )
    })
}

/// [`rename`], but reporting *why* a rename was refused.
///
/// `Err(refusal)` means the cursor names a real, resolvable symbol whose
/// rename cannot be completed soundly — the safety gate
/// ([`crate::rename_safety`]) proved the edit set would change what the
/// program does.  The server turns that into an LSP error response carrying
/// the reason, so the editor tells the user instead of silently applying a
/// partial edit set.  `Ok(vec![])` keeps its existing meaning: no renameable
/// symbol here, or a shape / collision gate said no.
///
/// The distinction matters because the two answers must not be conflated: an
/// empty edit set falls through to the server's cross-document and
/// workspace-resolved rename branches, and a *refusal* must not.
pub fn rename_with_diagnosis(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal> {
    rename_in_program(
        source,
        dialect,
        line,
        character,
        new_name,
        analysis,
        crate::definition::CallResolution {
            registry,
            program: None,
        },
    )
}

/// [`rename_with_diagnosis`] with the caller's whole-program export view
/// attached — the entry point a host with a workspace index should call.
///
/// A `namespace import -force` whose covering `namespace export` lives in
/// another file changes *which* definition a bare call site denotes, so a
/// rename started from that call must retarget with it.
/// When the reached definition is not in this document the resolver answers
/// `None` and the rename abstains — the safe outcome, and far better than
/// rewriting the unrelated local proc the shadow deleted.
///
/// `resolution` carries the registry and the oracle together, which also keeps
/// this entry point at [`rename_with_diagnosis`]'s arity.
///
/// A call whose `-force` shadow only the whole-program view decides is never
/// edited (user decision 2026-10-22T07:18Z, `force_import_shadow_direction`):
/// [`references_in_program`](crate::references::references_in_program) omits
/// it, so a rename whose edit set would touch it is refused as a whole.
pub fn rename_in_program(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal> {
    let edits = rename_in_program_unchecked(
        source, dialect, line, character, new_name, analysis, resolution,
    )?;
    let line_index = LineIndex::new(source);
    if resolution.program.is_some()
        && let Some(edit) = edits.iter().find(|edit| {
            crate::definition::program_forced_import_abstains(
                analysis,
                resolution.program,
                crate::definition::byte_offset_at(
                    &line_index,
                    source,
                    edit.range.start_line,
                    edit.range.start_character,
                ),
            )
        })
    {
        return Err(crate::rename_safety::RenameRefusal {
            reason: FORCE_IMPORT_CONDITIONAL.to_owned(),
            range: Some(edit.range),
        });
    }
    // One settlement for both providers (review N05): a rename never edits an
    // occurrence that find-references from the same cursor does not report as
    // the same binding — e.g. a call a later `namespace import -force`
    // retargets. Any such edit refuses the whole rename.
    if !edits.is_empty() {
        let mut references = crate::references::references_in_program(
            source,
            dialect,
            line,
            character,
            analysis,
            true,
            resolution.program,
        );
        // A member family is renamed as one: each declaration contributes
        // the occurrences find-references reports for it.
        for span in member_family_spans(source, analysis, &line_index, (line, character)) {
            let start = span_to_range(source, &line_index, span);
            references.extend(crate::references::references_in_program(
                source,
                dialect,
                start.start_line,
                start.start_character,
                analysis,
                true,
                resolution.program,
            ));
        }
        let offset = |line: u32, character: u32| {
            crate::definition::byte_offset_at(&line_index, source, line, character)
        };
        let covered = |edit: &TextEdit| {
            let start = offset(edit.range.start_line, edit.range.start_character);
            let end = offset(edit.range.end_line, edit.range.end_character);
            // The same occurrence: the edit and a reported reference overlap
            // (a variable edit may also cover its `$`).
            references.iter().any(|reference| {
                let reference_start = offset(reference.start_line, reference.start_character);
                let reference_end = offset(reference.end_line, reference.end_character);
                start < reference_end.max(reference_start + 1)
                    && reference_start < end.max(start + 1)
            })
        };
        if let Some(edit) = edits.iter().find(|edit| !covered(edit)) {
            return Err(crate::rename_safety::RenameRefusal {
                reason: "unreferenced-edit: an edit falls outside the occurrences \
                         find-references reports for this binding"
                    .to_owned(),
                range: Some(edit.range),
            });
        }
    }
    Ok(edits)
}

const FORCE_IMPORT_CONDITIONAL: &str = "force-import-shadow-conditional: a sibling document's \
     export decides this call only under a cross-document execution order that is not proven";

fn rename_in_program_unchecked(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Err(crate::rename_safety::RenameRefusal {
            reason: "source analysis does not match the current document and input".to_owned(),
            range: None,
        });
    }
    let line_index = LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
    if crate::definition::program_forced_import_abstains(analysis, resolution.program, cursor) {
        return Err(crate::rename_safety::RenameRefusal {
            reason: FORCE_IMPORT_CONDITIONAL.to_owned(),
            range: None,
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::namespace_symbol::select_at_offset(source, analysis, cursor)
    {
        let symbol = selected.ok_or_else(|| crate::rename_safety::RenameRefusal {
            reason: "cannot prove the original namespace selected by this occurrence".to_owned(),
            range: None,
        })?;
        return crate::namespace_rename::original_namespace_rename_edits(
            source, dialect, analysis, &symbol, new_name,
        );
    }
    // The new-name shape gate applies to every variable and command tier,
    // including the original-input ones below; only a namespace path keeps
    // its own `::`-separated grammar above.
    if !is_safe_symbol_name(new_name) {
        return Ok(Vec::new());
    }
    // A variable whose name can only be written quoted (`set {$n} 1`) is
    // refused before any variable tier: its occurrences carry delimiters the
    // new name may not need, so no edit set is guessed.
    if let Some(refusal) = crate::rename_safety::literal_name_variable_rename_refusal(
        source,
        line,
        character,
        analysis,
        &line_index,
    ) {
        return Err(refusal);
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::variable_symbol::select(source, analysis, line, character)
    {
        let occurrence = selected.ok_or_else(|| crate::rename_safety::RenameRefusal {
            reason: "cannot prove the original variable selected by this occurrence".to_owned(),
            range: None,
        })?;
        if let Some(refusal) =
            class_variable_rename_refusal(source, analysis, occurrence, &line_index)
        {
            return Err(refusal);
        }
        return crate::variable_symbol::original_variable_rename_edits(
            source,
            analysis,
            occurrence.symbol(),
            new_name,
        );
    }
    if let Some(refusal) = readonly_math_rename_refusal(source, analysis, cursor) {
        return Err(refusal);
    }
    if !analysis.allows_lexical_declaration_advice() {
        return rename_original_declaration(source, analysis, cursor, &line_index, new_name);
    }
    if let Some(refusal) = pre_edit_refusal(
        source,
        dialect,
        line,
        character,
        new_name,
        analysis,
        &line_index,
    ) {
        return Err(refusal);
    }
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some(result) =
        rename_definition_operand(source, dialect, analysis, &line_index, cursor, new_name)
    {
        return result;
    }

    rename_lexical_tiers(
        source,
        dialect,
        (line, character),
        new_name,
        analysis,
        resolution,
        &line_index,
    )
}

fn rename_lexical_tiers(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    position: (u32, u32),
    new_name: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    line_index: &LineIndex,
) -> Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal> {
    let (line, character) = position;
    // Namespace name first: the position is *definitive* (a proc or class of
    // the same spelling is not what it names), and the gate above already
    // cleared this document.
    if let Some(edits) = rename_namespace_at(source, dialect, line, character, new_name, analysis) {
        return Ok(edits);
    }
    if let Some(edits) = rename_variable_at(source, line, character, new_name, analysis, line_index)
    {
        return Ok(edits);
    }
    let def_byte = crate::definition::byte_offset_at(line_index, source, line, character);

    let Some(word) =
        crate::source_structure::original_cursor_word_value(source, analysis, def_byte)
    else {
        return Ok(Vec::new());
    };

    if let Some(edits) = rename_list_built_self_method(
        source,
        dialect,
        analysis,
        line_index,
        (&word, new_name),
        def_byte,
    ) {
        return edits;
    }

    if let Some(edits) = rename_named_command(
        source,
        analysis,
        resolution,
        line_index,
        (&word, new_name),
        def_byte,
    ) {
        return edits;
    }
    if let Some(edits) = rename_itcl_class_proc(
        source,
        dialect,
        (line, character),
        new_name,
        analysis,
        line_index,
    ) {
        return edits;
    }
    let cursor_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    if let Some(selected) =
        crate::receiver_identity::method_at_cursor(analysis, source, cursor_offset)
        && selected.method.name == word
        && selected.editable_selector().is_some()
        && analysis
            .all_classes
            .get(&selected.class.qualified_name)
            .is_some_and(|current| current.name_span == selected.class.name_span)
        && let Some(edits) = rename_method_in_class(
            source,
            dialect,
            (
                &selected.class.qualified_name,
                &selected.method.name,
                selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class,
            ),
            new_name,
            analysis,
            line_index,
        )
    {
        return edits;
    }
    // Method rename — match `word` against any class's methods
    // / classmethods / properties at the cursor's byte offset.
    let cursor_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    gated_untargeted_member_rename(
        source,
        dialect,
        (&word, new_name),
        analysis,
        cursor_offset,
        line_index,
    )
    .map(Option::unwrap_or_default)
}

fn rename_original_declaration(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    line_index: &LineIndex,
    new_name: &str,
) -> Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal> {
    let std::ops::ControlFlow::Break(Some(identity)) =
        crate::original_declaration::select_at_offset("", source, analysis, cursor)
    else {
        return Err(crate::rename_safety::RenameRefusal {
            reason: "the original command selected at this cursor is unavailable; source-only vendor names and unknown native inputs cannot use lexical name edits".to_owned(),
            range: None,
        });
    };
    let document = crate::original_declaration::OriginalDeclarationDocument {
        uri: "",
        source,
        analysis,
    };
    let edits = if matches!(
        identity.role(),
        crate::original_declaration::OriginalDeclarationRole::Method(_)
    ) {
        crate::original_member_rename::original_member_rename_edits(
            &[document],
            &identity,
            new_name,
        )
        .map_err(|refusal| crate::rename_safety::RenameRefusal {
            reason: format!("{}: {}", refusal.code(), refusal.reason()),
            range: Some(span_to_range(source, line_index, identity.span())),
        })?
    } else {
        crate::original_command_rename::original_command_rename_edits(
            &[document],
            &identity,
            new_name,
        )
        .map_err(|refusal| crate::rename_safety::RenameRefusal {
            reason: format!("{}: {}", refusal.code(), refusal.reason()),
            range: Some(span_to_range(source, line_index, identity.span())),
        })?
    };
    Ok(edits
        .into_iter()
        .map(|edit| TextEdit {
            range: span_to_range(source, line_index, edit.span),
            new_text: edit.new_text,
        })
        .collect())
}

fn readonly_math_rename_refusal(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<crate::rename_safety::RenameRefusal> {
    if analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let refuse = || {
        crate::rename_safety::RenameRefusal {
        reason: "cannot rename this procedure atomically: its expression identifier references have no separately authorised edit owner".to_owned(),
        range: None,
    }
    };
    if let std::ops::ControlFlow::Break(Some(_)) =
        crate::math_function_symbol::select_at_offset(source, analysis, cursor)
    {
        return Some(refuse());
    }
    let line_index = LineIndex::new(source);
    let position = line_index.position_at_utf16(cursor, source);
    let std::ops::ControlFlow::Break(Some(identity)) = crate::original_declaration::select(
        "",
        source,
        analysis,
        position.line,
        position.character.get(),
    ) else {
        return None;
    };
    let declaration = identity.procedure_metadata(analysis)?;
    match crate::math_function_symbol::procedure_references_in(
        source,
        analysis,
        source,
        analysis,
        declaration,
        true,
    ) {
        Some(references) if references.is_empty() => None,
        Some(_) | None => Some(refuse()),
    }
}

fn rename_definition_operand(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    cursor: u32,
    new_name: &str,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let reference =
        crate::receiver_identity::definition_reference_at_cursor(analysis, source, cursor)?;
    let selected = crate::receiver_identity::method_at_cursor(analysis, source, cursor);
    let edits = selected
        .filter(|selected| {
            selected.editable_selector().is_some()
                && analysis
                    .all_classes
                    .get(&selected.class.qualified_name)
                    .is_some_and(|current| current.name_span == selected.class.name_span)
        })
        .and_then(|selected| {
            rename_method_in_class(
                source,
                dialect,
                (
                    &selected.class.qualified_name,
                    &selected.method.name,
                    selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class,
                ),
                new_name,
                analysis,
                line_index,
            )
        });
    Some(edits.unwrap_or_else(|| Err(crate::rename_safety::refusal_at(
        "cannot rename this metadata operand: its original method declaration is unavailable for a complete edit.".to_owned(),
        source, line_index, Some(reference.name_span()),
    ))))
}

/// The refusal for a cursor sitting on a **namespace name** — a word the
/// registry marks [`tcl_registry::ArgRole::NamespaceName`] —
/// when [`crate::namespace_rename`]'s tier cannot prove the edit set
/// complete.
///
/// It has to be a refusal rather than an empty edit set for the reason
/// [`rename_with_diagnosis`]'s own doc gives: an empty set falls through to
/// the server's cross-document and workspace-resolved branches, which resolve
/// the cursor **word** as a command — so `namespace children widget` beside a
/// `proc widget` offered to rename the proc, pointing the editor's highlight
/// at the proc's declaration on another line.
fn namespace_rename_refusal(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
) -> Option<crate::rename_safety::RenameRefusal> {
    let cell = crate::namespace_symbol::namespace_cell_at(source, analysis, line, character)?;
    crate::namespace_rename::namespace_rename_edits(source, dialect, analysis, &cell, new_name)
        .err()
}

/// The **in-document** namespace rename: every written spelling of the
/// namespace the cursor names, rewritten to `new_name`.
///
/// `None` when the cursor names no namespace.  A refusal has already been
/// decided by [`namespace_rename_refusal`] in [`pre_edit_refusal`], so this
/// tier only ever runs on a document the gate cleared — the two call the same
/// [`crate::namespace_rename::namespace_rename_edits`], so they cannot
/// disagree about which occurrences exist.
///
/// The server drives the *workspace* version of this: a namespace is spelled
/// in every file that reaches into it, so a rename that stopped at one
/// document would be exactly the partial edit set the safety bar forbids.
fn rename_namespace_at(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<TextEdit>> {
    let cell = crate::namespace_symbol::namespace_cell_at(source, analysis, line, character)?;
    crate::namespace_rename::namespace_rename_edits(source, dialect, analysis, &cell, new_name).ok()
}

/// A rename edit targeting a specific document, expressed as a
/// byte span (the caller resolves it to an LSP range against
/// that document's source).  Used for cross-document rename,
/// where the core provider can't see sibling documents'
/// sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceTextEdit {
    /// Document the edit applies to.
    pub uri: String,
    /// Byte span in `uri`'s source to replace.
    pub span: tcl_lexer::Span,
    /// Replacement text.
    pub new_text: String,
}

/// Return the namespace prefix of a qualified name — everything
/// before the final `::`.  `"::myns::greet"` → `"::myns"`;
/// `"::greet"` → `""` (proc lives at global scope, no enclosing
/// namespace).  `"greet"` → `""` likewise.
fn namespace_prefix_of(qualified: &str) -> &str {
    // `qualified` is a constructed key: its holder comes from the
    // analytical holder projection — a repeated-strip + `rfind` would
    // misread a lone-colon segment.  The root holder maps to `""` (a global
    // proc has no enclosing-namespace prefix).
    let (holder, _tail) = tcl_syntax::naming::key_holder_and_tail(qualified);
    if holder == "::" { "" } else { holder }
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

/// Compatibility edits retain their actual producer byte geometry until the
/// complete plan has passed conflict checks. This does not select symbols or
/// strengthen any source applicability receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingRenameEdit {
    span: tcl_lexer::Span,
    new_text: String,
}

fn finish_rename_edits(
    source: &str,
    line_index: &LineIndex,
    edits: Vec<PendingRenameEdit>,
) -> Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal> {
    if edits
        .iter()
        .any(|edit| edit.span.is_empty() || source.get(edit.span.as_range()).is_none())
    {
        return Err(crate::rename_safety::RenameRefusal {
            reason: "cannot rename: edit ranges are invalid or conflicting".to_owned(),
            range: None,
        });
    }
    // Check source order without changing the established publication order.
    // Only identical edits collapse; equal ranges with different text conflict.
    let mut order: Vec<usize> = (0..edits.len()).collect();
    order.sort_by_key(|&index| (edits[index].span.start(), edits[index].span.end()));
    let mut keep = vec![true; edits.len()];
    for pair in order.windows(2) {
        let (left, right) = (&edits[pair[0]], &edits[pair[1]]);
        if left == right {
            keep[pair[1]] = false;
        } else if left.span.end() > right.span.start() {
            return Err(crate::rename_safety::RenameRefusal {
                reason: "cannot rename: edit ranges are invalid or conflicting".to_owned(),
                range: Some(span_to_range(source, line_index, right.span)),
            });
        }
    }
    Ok(edits
        .into_iter()
        .enumerate()
        .filter(|(index, _)| keep[*index])
        .map(|(_, edit)| TextEdit {
            range: span_to_range(source, line_index, edit.span),
            new_text: edit.new_text,
        })
        .collect())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod selected_replacement_qualification_tests;

#[cfg(test)]
mod selected_reference_tail_proposal_tests;

#[cfg(test)]
mod source_declaration_rename_tests;

#[cfg(test)]
mod typed_namespace_variable_rename_tests;

#[cfg(test)]
mod conditional_source_alpha_tests;
