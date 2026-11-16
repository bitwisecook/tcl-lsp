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
use crate::references::{MemberSel, resolve_member_span};

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
    let (word, word_start, word_end) = find_word_span_at_position(source, line, character)?;
    let word_off = crate::definition::byte_offset_at(line_index, source, line, character);
    if let Some(preparation) = prepare_list_built_self_method_rename(
        source,
        line,
        (word_start, word_end),
        analysis,
        &word,
        word_off,
    ) {
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

/// A variable a `TclOO` class declares with `variable NAME` is linked into
/// every method body (tclsh `8.6`/`9.0`: `oo::class create C {variable n; method
/// get {} {return $n}; method set {x} {set n $x}}` shares one `n` per object).
/// The original variable symbol of a method-body read does not yet unify the
/// class declaration and the other methods' uses, so renaming it would edit
/// one occurrence and break the program; the rename is refused instead.
fn class_variable_rename_refusal(
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

/// The other declarations of the instance member family selected at the
/// cursor, which an original member rename edits together.
fn member_family_spans(
    source: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    (line, character): (u32, u32),
) -> Vec<tcl_lexer::Span> {
    if analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let cursor = crate::definition::byte_offset_at(line_index, source, line, character);
    let std::ops::ControlFlow::Break(Some(identity)) =
        crate::original_declaration::select_at_offset("", source, analysis, cursor)
    else {
        return Vec::new();
    };
    crate::original_member_rename::original_member_family_spans(
        crate::original_declaration::OriginalDeclarationDocument {
            uri: "",
            source,
            analysis,
        },
        &identity,
    )
    .into_iter()
    .filter(|span| *span != identity.span())
    .collect()
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

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
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
    Some(edits.unwrap_or_else(|| Err(crate::rename_safety::RenameRefusal::at(
        "cannot rename this metadata operand: its original method declaration is unavailable for a complete edit.".to_owned(),
        source, line_index, Some(reference.name_span()),
    ))))
}

fn prepare_list_built_self_method_rename(
    source: &str,
    _line: u32,
    _word_range: (u32, u32),
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<PrepareRename> {
    let selected = crate::receiver_identity::method_at_cursor(analysis, source, cursor_offset)?;
    if selected.method.name != word {
        return None;
    }
    let selector = selected.editable_selector()?;
    Some(PrepareRename {
        range: span_to_range(source, &LineIndex::new(source), selector),
        placeholder: selected.method.name.clone(),
    })
}

fn rename_list_built_self_method(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    names: (&str, &str),
    cursor_offset: u32,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let (word, new_name) = names;
    let (provider, is_classmethod) = crate::references::list_built_self_method_target_at_cursor(
        source,
        dialect,
        analysis,
        word,
        cursor_offset,
    )?;
    rename_method_in_class(
        source,
        dialect,
        (&provider, word, is_classmethod),
        new_name,
        analysis,
        line_index,
    )
}

fn prepare_named_command_rename(
    source: &str,
    line_index: &LineIndex,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    target: (&str, u32),
) -> Option<PrepareRename> {
    let (word, cursor_offset) = target;
    if let Some((_, proc_def)) =
        crate::definition::resolve_proc_target_at(analysis, source, cursor_offset, word, resolution)
    {
        return Some(PrepareRename {
            range: span_to_range(source, line_index, proc_def.name_span),
            placeholder: proc_def.name.clone(),
        });
    }
    let (_, class_def) = crate::definition::resolve_class_target_at(
        analysis,
        source,
        resolution,
        cursor_offset,
        word,
    )?;
    Some(PrepareRename {
        range: span_to_range(source, line_index, class_def.name_span),
        placeholder: class_def.name.clone(),
    })
}

fn rename_named_command(
    source: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    line_index: &LineIndex,
    names: (&str, &str),
    cursor_offset: u32,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let (word, new_name) = names;
    rename_proc(
        source,
        word,
        cursor_offset,
        new_name,
        analysis,
        resolution,
        line_index,
    )
    .or_else(|| {
        rename_class(
            source,
            word,
            cursor_offset,
            new_name,
            analysis,
            resolution,
            line_index,
        )
    })
}

/// The **untargeted** member-rename tier, behind the safety gate — the last
/// tier [`rename_with_diagnosis`] tries, and the one that answers every
/// cursor position no trusted dispatch tier claims.
///
/// `Err` is a refusal, `Ok(None)` means the cursor names no member here.
///
/// The gate runs *here* rather than in [`pre_edit_refusal`] so the tier order
/// is preserved exactly: a cursor an earlier tier claims (a local variable, a
/// proc, a class) is renamed by that tier and never consulted about a
/// same-spelled member. And it runs at all because `pre_edit_refusal` can
/// only speak for a cursor [`method_rename_target`] resolves — the member's
/// own declaration name, a `my` dispatch, a tracked `$obj` dispatch. The
/// positions this tier owns (the method word inside an untracked `[$other
/// X]`, the bareword in `export … X …`) are precisely the ones from which the
/// hazard emits a declaration-only edit set that breaks the
/// program once applied. Asking the same question about the same member here
/// makes the verdict independent of *which occurrence* the rename was
/// triggered from.
fn gated_untargeted_member_rename(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    (word, new_name): (&str, &str),
    analysis: &AnalysisResult,
    cursor_offset: u32,
    line_index: &LineIndex,
) -> Result<Option<Vec<TextEdit>>, crate::rename_safety::RenameRefusal> {
    if let Some((seed_class, method, is_classmethod)) =
        untargeted_member_rename_target(analysis, word, cursor_offset)
        && let Some(refusal) = member_rename_hazard(
            source,
            dialect,
            analysis,
            (&seed_class, &method, is_classmethod),
            new_name,
            line_index,
        )
    {
        return Err(refusal);
    }
    rename_method(
        source,
        dialect,
        word,
        new_name,
        analysis,
        cursor_offset,
        line_index,
    )
    .transpose()
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

/// The safety refusal for a `TclOO` member rename anchored at the cursor, if
/// any — the single in-document entry point to [`crate::rename_safety`].
///
/// Resolves the cursor to a member and its whole override family exactly the
/// way the rename itself does ([`method_rename_target`] + [`override_family`]),
/// so the gate is asked about precisely the declarations the edit set would
/// rewrite — never a wider or narrower set.
fn method_rename_refusal(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<crate::rename_safety::RenameRefusal> {
    let (seed_class, method, is_classmethod) =
        method_rename_target(source, line, character, analysis)?;
    member_rename_hazard(
        source,
        dialect,
        analysis,
        (&seed_class, &method, is_classmethod),
        new_name,
        line_index,
    )
}

/// The safety verdict for renaming `method` on `seed_class`'s whole override
/// family — the one call both member-rename gates make, so a hazard is a
/// property of *which member the rename rewrites*, never of how the cursor
/// happened to name it.
///
/// Split out of [`method_rename_refusal`] because that entry point can only
/// speak for cursors a *trusted dispatch tier* resolves
/// ([`method_rename_target`]: the member's own declaration name, a `my`
/// dispatch, a tracked `$obj` / class-command dispatch). Every other position
/// that still reaches a member — the untracked call site the gate exists to
/// protect, an `export`/`unexport` bareword — is resolved by
/// [`untargeted_member_rename_target`] instead and asked the same question
/// here.
fn member_rename_hazard(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    (seed_class, method, is_classmethod): (&str, &str, bool),
    new_name: &str,
    line_index: &LineIndex,
) -> Option<crate::rename_safety::RenameRefusal> {
    let family = override_family(analysis, seed_class, method);
    if family.is_empty() {
        return None;
    }
    crate::rename_safety::method_rename_hazard(
        source,
        dialect,
        analysis,
        crate::rename_safety::MethodRenameTarget {
            family: &family,
            method,
            is_classmethod,
            new_name,
        },
        line_index,
    )
}

/// The member the **untargeted** member-rename tier ([`rename_method`]) would
/// rewrite for a cursor at `cursor_offset`, or `None` when that tier declines
/// the cursor.
///
/// This is deliberately the *same* resolution `rename_method` performs —
/// "which class body encloses the cursor, and does `word` name one of its
/// members" ([`crate::definition::enclosing_class_at`] +
/// [`resolve_member_span`]) — and not a re-derivation. That tier answers
/// exactly the cursor positions no trusted dispatch tier claims: the method
/// name inside `[$untracked X]`, and the bareword in an `export … X …` list.
/// Without the check they would reach `rename_method` ungated, so the very
/// shape [`crate::rename_safety`] refuses when the cursor sits on the
/// declaration would silently emit a declaration-only edit set from one line
/// away — applying that and running it under tclsh 9.0.4 / 8.6.16 gives
/// `unknown method "X"` at the untouched dispatch.
///
/// Restricted to `method` / `classmethod`: a *property* has no dispatch
/// (see [`rename_method`]'s own property branch), so it has no dispatch
/// hazard to check.
#[must_use]
pub fn untargeted_member_rename_target(
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<(String, String, bool)> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let class_q = crate::definition::enclosing_class_at(analysis, cursor_offset)?;
    let class_def = analysis.all_classes.get(class_q)?;
    let (kind, _span) = resolve_member_span(class_def, word, cursor_offset)?;
    match kind {
        MemberSel::Method => Some((class_def.qualified_name.clone(), word.to_owned(), false)),
        MemberSel::ClassMethod => Some((class_def.qualified_name.clone(), word.to_owned(), true)),
        MemberSel::Property => None,
    }
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
fn rename_variable_at(
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

/// Rename `method` across the class identified by `class_q` **and every
/// other class in its override family** — rewriting each class's
/// declaration name span, its intra-class `my method` call sites, and its
/// external `$obj method` call sites.  Returns `None` when neither
/// `class_q` nor any ancestor provides `method` (nothing to rename).
///
/// `target` is `(class_q, method, is_classmethod)` — bundled to keep the
/// parameter count under the lint budget; `is_classmethod` reflects which
/// [`crate::definition::MethodBucket`] the caller's own receiver resolved
/// to (`$obj method` vs. a bare `ClassName method` classmethod dispatch),
/// not something re-derivable from `class_q`/`method` alone.
///
/// A `TclOO` method that is (re)defined by a super- or sub-class is a
/// single polymorphic name: `$obj method` dispatch can reach any
/// definition along the chain, so renaming only the class under the
/// cursor would silently break the override relationship.  See
/// [`override_family`].  Shared by the in-class-body and external
/// `$obj method` rename entry points.
/// Rename an [incr Tcl] class-scoped `proc` from a `Factory::make` call site
/// or its declaration.
///
/// Tried after the proc / class rename paths so an ordinary qualified proc
/// call of the same spelling keeps priority.  The edits come from the shared
/// member-reference collector, whose itcl call sites cover only the final
/// `::`-segment — so the as-written qualifier survives the rewrite.
///
/// Single-document: a `Factory::make` call in a sibling file is not
/// rewritten, because the cross-file rename layer still carries only the
/// two-word `Class method` dispatch shape.
fn rename_itcl_class_proc(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    cursor: (u32, u32),
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let (line, character) = cursor;
    let (class_q, member) =
        crate::definition::itcl_class_proc_target_at(source, dialect, line, character, analysis)?;
    rename_method_in_class(
        source,
        dialect,
        (&class_q, &member, true),
        new_name,
        analysis,
        line_index,
    )
}

fn rename_method_in_class(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    target: (&str, &str, bool),
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let (class_q, method, is_classmethod) = target;
    let family = override_family(analysis, class_q, method);
    if family.is_empty() {
        return None;
    }
    let mut edits = Vec::new();
    for member in &family {
        // Reached via an external call site — either `$obj method` (always
        // instance-context) or a bare `ClassName method` classmethod
        // dispatch — so `is_classmethod` is fixed by
        // the caller's own receiver resolution, never re-derived per family
        // member (a method and a classmethod occupy separate dispatch
        // tables; the receiver picks exactly one for the whole rename).
        let Some((decl_span, call_spans)) = crate::references::method_references_for_class(
            source,
            dialect,
            analysis,
            member,
            method,
            is_classmethod,
        ) else {
            continue;
        };
        edits.push(PendingRenameEdit {
            span: decl_span,
            new_text: new_name.to_owned(),
        });
        for span in call_spans {
            edits.push(PendingRenameEdit {
                span,
                new_text: new_name.to_owned(),
            });
        }
    }
    if edits.is_empty() {
        return None;
    }
    Some(finish_rename_edits(source, line_index, edits))
}

/// When the cursor sits on a renameable `TclOO` **method**, return the
/// `(class_qualified_name, method_name)` it targets — the seed the server
/// uses to compute the cross-file override family.  Covers both entry
/// points the in-document rename recognises: an external `$obj method`
/// call site (class resolved via `instance_classes`) and a cursor inside a
/// class body on a method / classmethod name (declaration or `my method`
/// call).  Returns `None` for non-method targets (vars, procs, classes,
/// properties), which stay on the single-document rename path.
#[must_use]
pub fn method_rename_target(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<(String, String, bool)> {
    method_target_with_access(source, line, character, analysis)
        .map(|(c, m, is_cm, _)| (c, m, is_cm))
}

/// Like [`method_rename_target`] but also reports the **access context**
/// of the cursor's call shape: an external
/// `$obj m` dispatches through exported methods only, while a `my m`
/// call or a declaration-side cursor inside the class body reaches
/// unexported methods too.
#[must_use]
pub fn method_target_with_access(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<(String, String, bool, crate::workspace_index::MethodAccess)> {
    method_target_with_access_in_workspace(source, line, character, analysis, None)
}

fn callback_prefix_target_in_workspace(
    source: &str,
    analysis: &AnalysisResult,
    word: &str,
    cursor: u32,
    index: Option<(&crate::workspace_index::WorkspaceIndex, &str)>,
) -> Option<(String, String, bool, crate::workspace_index::MethodAccess)> {
    use crate::workspace_index::MethodAccess;

    let (receiver, is_classmethod, external) =
        crate::references::callback_prefix_method_receiver_at_cursor(
            source,
            crate::profile_for_analysis(analysis),
            analysis,
            word,
            cursor,
        )?;
    let (workspace, _uri) = index?;
    let access = if external {
        MethodAccess::External
    } else {
        MethodAccess::Internal
    };
    // The chain is the *gate*, not the answer: an empty one means this
    // receiver cannot dispatch the member at all (a `[self]` capture of a name
    // the receiver has unexported), so the position resolves to nothing.  What
    // is returned is the **receiver**, exactly as the bare `$obj method` arms
    // below return theirs — the family, definition and rename tiers each
    // resolve the provider from it through this same chain, and handing them a
    // provider instead would drop the receiver's own `export` / `unexport`
    // stub on the way: when `Base` unexports `tock` and `Child` exports the
    // inherited name, only a chain rooted at `Child` still reaches `Base`'s
    // body.
    let chain = if is_classmethod {
        workspace.class_method_dispatch_chain(&receiver, word, access)
    } else {
        workspace.method_dispatch_chain(&receiver, word, access)
    };
    chain.first()?;
    Some((receiver, word.to_owned(), is_classmethod, access))
}

/// [`method_target_with_access`] with the **workspace tier** available for
/// receiver classification.
///
/// The local analysis classifies every receiver this document knows about. A
/// *pure consumer* file — one holding only `C cm`, declaring no part of `C` —
/// knows none, and the server's workspace-class oracle reanalysis does not
/// close that gap: it supplies class **names** only, never their member
/// tables. So a class-side call from an ordinary consumer fell through both
/// receiver branches and definition / references / rename all answer nothing.
///
/// Passing `index` lets the classmethod arm ask the workspace what it cannot
/// see locally, through the same dispatch chain the resolvers use. The
/// instance arm needs no equivalent: it already crosses files, because the
/// analyser's own class oracle is enough to type `[Cls new]` and populate
/// `instance_classes`.
#[must_use]
pub fn method_target_with_access_in_workspace(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    index: Option<(&crate::workspace_index::WorkspaceIndex, &str)>,
) -> Option<(String, String, bool, crate::workspace_index::MethodAccess)> {
    use crate::workspace_index::MethodAccess;
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let line_index = LineIndex::new(source);
    let (word, _s, _e) = find_word_span_at_position(source, line, character)?;
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
    // Only the retained single-field link form couples command and member edits.
    if let Some(linked) =
        tcl_compiler::analyser::types::original_source_linked_member_at(analysis, source, cursor)
        && linked.has_coupled_name()
        && linked.editable_target_span(source).is_some()
        && linked.alias_input().bytes() == word.as_bytes()
        && linked.target_input().bytes() == word.as_bytes()
    {
        return Some((
            linked.class_declaration().metadata().qualified_name.clone(),
            linked.target().metadata().name.clone(),
            linked.target().side() == tcl_compiler::analyser::MemberSide::ClassObject,
            MethodAccess::Internal,
        ));
    }
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    if crate::receiver_identity::definition_reference_at_cursor(analysis, source, cursor).is_some()
    {
        let selected = crate::receiver_identity::method_at_cursor(analysis, source, cursor)?;
        let current = analysis.all_classes.get(&selected.class.qualified_name)?;
        return (current.name_span == selected.class.name_span
            && selected.editable_selector().is_some())
        .then(|| {
            (
                selected.class.qualified_name.clone(),
                selected.method.name.clone(),
                selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class,
                MethodAccess::Internal,
            )
        });
    }
    if let Some(target) =
        callback_prefix_target_in_workspace(source, analysis, &word, cursor, index)
    {
        return Some(target);
    }
    if let Some((provider, is_classmethod)) =
        crate::references::list_built_self_method_target_at_cursor(
            source,
            crate::profile_for_analysis(analysis),
            analysis,
            &word,
            cursor,
        )
    {
        return Some((provider, word, is_classmethod, MethodAccess::External));
    }
    if let Some((_, method, _)) =
        crate::definition::original_instance_method_at_cursor(source, analysis, line, character)
        && method == word
    {
        if let Some(selected) = crate::receiver_identity::method_at_cursor(analysis, source, cursor)
        {
            return Some((
                selected.class.qualified_name.clone(),
                selected.method.name.clone(),
                selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class,
                MethodAccess::External,
            ));
        }
        return None;
    }
    // Inside a class body on one of its method / classmethod names — the
    // declaration side, an internal context.
    if let Some(class_def) = crate::definition::enclosing_class_at(analysis, cursor)
        .and_then(|q| analysis.all_classes.get(q))
    {
        let method_decl = class_def.methods.get(&word);
        let classmethod_decl = class_def.class_methods.get(&word);
        let on_declaration = |m: &tcl_compiler::analyser::MethodDef| {
            m.name_span.start() <= cursor && cursor < m.name_span.end()
        };
        let cursor_on_method_decl = method_decl.is_some_and(on_declaration);
        let cursor_on_classmethod_decl = classmethod_decl.is_some_and(on_declaration);
        if (method_decl.is_some() || classmethod_decl.is_some())
            && (cursor_on_method_decl || cursor_on_classmethod_decl)
        {
            // A `method` and a `classmethod` of the same name are distinct
            // members; unambiguous when only one exists, otherwise prefer
            // whichever one's declaration the cursor is actually on.
            let is_classmethod =
                classmethod_decl.is_some() && (method_decl.is_none() || cursor_on_classmethod_decl);
            return Some((
                class_def.qualified_name.clone(),
                word,
                is_classmethod,
                MethodAccess::Internal,
            ));
        }
    }
    None
}

/// Every span (declaration + intra-class `my method` calls + external
/// `$obj method` sites) that renaming `method` on `class_q` must rewrite
/// **within `source`**.  Empty when `class_q` is not defined in this
/// document.  The server calls this per family-member document to assemble
/// a cross-file method rename.
#[must_use]
pub fn method_spans_in_document(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
) -> Vec<tcl_lexer::Span> {
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    match crate::references::method_references_for_class(
        source,
        dialect,
        analysis,
        class_q,
        method,
        is_classmethod,
    ) {
        Some((decl, calls)) => {
            let mut spans = Vec::with_capacity(1 + calls.len());
            spans.push(decl);
            spans.extend(calls);
            spans
        }
        None => Vec::new(),
    }
}

/// Every span that renaming `method` must rewrite **within `source`** for a
/// class `class_q` that *inherits* `method` (does not declare it): its
/// intra-class `my method` calls and external `$obj method` sites, with no
/// declaration span.  Empty when `class_q` is not defined in this document.
///
/// The server calls this for documents that contain a purely-inheriting
/// subclass of a rename family member but no definer of their own — the
/// cross-file complement to [`method_spans_in_document`], so an
/// inherited-method rename reaches `my method` / `$obj method` sites that
/// live in a subclass-only file.
///
/// `workspace` carries the two facts a subclass-only document cannot derive
/// from its own (definer-less) `all_classes` — see
/// [`crate::references::InheritedReceiverFacts`].
#[must_use]
pub fn inherited_method_spans_in_document(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    is_classmethod: bool,
    workspace: crate::references::InheritedReceiverFacts<'_>,
) -> Vec<tcl_lexer::Span> {
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    crate::references::inherited_method_call_sites(
        source,
        dialect,
        analysis,
        class_q,
        method,
        is_classmethod,
        workspace,
    )
}

/// The set of classes whose definition of `method` must be renamed
/// together with `seed_class`'s: every class that **directly defines**
/// `method` and sits in the same override-connected component as
/// `seed_class` (or the class that provides `method` to it when
/// `seed_class` only inherits it).
///
/// Two definitions belong to one component when their classes are related
/// by the (mixin-aware) subtype relation — directly, or transitively via
/// another definer.  So a base method plus every subclass override, and
/// sibling overrides of a common base, all rename as one unit; unrelated
/// same-named methods in disjoint hierarchies stay separate (never
/// over-renamed).  The returned vector always contains the seed and is
/// empty only when `method` is neither defined nor inherited by
/// `seed_class`.
fn override_family(analysis: &AnalysisResult, seed_class: &str, method: &str) -> Vec<String> {
    let hierarchy = analysis.class_hierarchy();
    // Classes that *directly* define `method` (own body, any visibility) —
    // constructors/destructors aren't in `methods`/`class_methods` so are
    // naturally excluded.
    let definers: Vec<&String> = analysis
        .all_classes
        .iter()
        .filter(|(_, cd)| cd.methods.contains_key(method) || cd.class_methods.contains_key(method))
        .map(|(q, _)| q)
        .collect();
    // Seed: the class under the cursor when it defines `method`, else the
    // class that provides it (an ancestor, via the MRO).  When neither, the
    // family is empty.
    let seed = if definers.iter().any(|d| d.as_str() == seed_class) {
        seed_class.to_string()
    } else if let Some(provider) = hierarchy.method_target(seed_class, method) {
        provider.to_string()
    } else {
        return Vec::new();
    };
    let connected =
        |a: &str, b: &str| a == b || hierarchy.is_subtype(a, b) || hierarchy.is_subtype(b, a);
    // Grow the weakly-connected component of definers containing `seed` to a
    // fixed point (siblings attach via a shared base already in the family).
    let mut family = vec![seed];
    let mut changed = true;
    while changed {
        changed = false;
        for d in &definers {
            if family.iter().any(|f| f == d.as_str()) {
                continue;
            }
            if family.iter().any(|f| connected(f, d)) {
                family.push((*d).clone());
                changed = true;
            }
        }
    }
    family
}

/// Variable-rename path — declaration span + every read site,
/// with brace-ref escaping (`$x` / `${x}` / `$ns::x` /
/// `${ns::x}` keep their leader characters and any namespace
/// qualifier).
fn rename_var(
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

/// Whether this call site is an ensemble **dispatch word whose spelling the
/// rename must not touch** — the `show` of `::app::widget show` when the
/// ensemble bound it with `-map {show ::app::widget::Show}`.
///
/// A `-map` key is an arbitrary name with no required relationship to the
/// target it dispatches to (tclsh 8.6.14 / 9.0.4: with that map,
/// `::app::widget show` runs the proc and `::app::widget Show` is `unknown or
/// ambiguous subcommand "Show": must be show`).  So renaming
/// `::app::widget::Show` → `Display` must rewrite the declaration and the
/// `-map` *value* — a separate reference this same loop rewrites, because its
/// span really does carry the target's name — while leaving the dispatch word
/// alone.  Rewriting it is the corruption the issue reports: the map still
/// says `show`, and the edited call names a subcommand that does not exist.
///
/// A `-subcommands {alpha}` entry is the opposite case and is deliberately
/// **not** gated: there the subcommand word *is* the target's tail, the
/// ensemble derives `::app::widget::alpha` from it, and leaving it behind
/// gives `invalid command name "alpha"` — so it must be rewritten with the
/// declaration, which is what the ungated path already does.
///
/// This reads the per-site provenance the analyser recorded, not a name
/// comparison against `ensemble_subcommand_targets`: a `-map {Show
/// ::app::widget::Show}` whose key happens to equal the target's tail is
/// textually indistinguishable from an ordinary bare call to the target, and
/// only the recording site knows which one this span is.
///
/// Find-references, code-lens counts and call-hierarchy are untouched by this
/// — a dispatch site is a genuine reference under either provenance, and the
/// gate lives here (in the two rename loops) rather than in
/// [`crate::references::invocation_references_proc`], which all of them
/// share.
#[must_use]
fn ensemble_map_dispatch_word(
    ensemble_dispatch: Option<tcl_compiler::signature_scan::types::EnsembleSubcommandProvenance>,
) -> bool {
    matches!(
        ensemble_dispatch,
        Some(tcl_compiler::signature_scan::types::EnsembleSubcommandProvenance::Map)
    )
}

/// Proc-rename path — declaration name span + every matching
/// call site.  Namespace-aware: the declaration keeps its
/// prefix; call sites pick the rewrite that matches the form
/// the source wrote (qualified ↔ short).  `Some(edits)` when
/// a proc matched `word` (even if the edit set ended up empty
/// from a safety gate); `None` when no proc matched.
fn rename_proc(
    source: &str,
    word: &str,
    cursor_off: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    line_index: &LineIndex,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let registry = resolution.registry;
    // Resolve the target the same way go-to-definition does: the proc whose
    // declaration covers the cursor, else C Tcl's namespace-aware call-site
    // resolution — never a namespace-blind `p.name == word` scan, which let a
    // rename from a bareword call site pick an arbitrary same-named proc in
    // another namespace and rewrite the wrong definition (matches
    // `references::references`).
    let (qname, proc_def) =
        crate::definition::resolve_proc_target_at(analysis, source, cursor_off, word, resolution)?;
    if let Some(registry) = registry
        && is_builtin_command_name(new_name, registry)
    {
        return Some(Ok(Vec::new()));
    }
    let namespace_prefix = namespace_prefix_of(&proc_def.qualified_name);
    let (new_qualified, qualified_decl) = qualified_and_decl_text(namespace_prefix, new_name);
    // The declaration rewrite follows the *form the source wrote* — a proc
    // declared short inside a `namespace eval` (`proc helper`) stays short
    // (`assist`); one declared qualified (`proc ::ns::greet`) stays qualified.
    let decl_was_qualified = source
        .get(proc_def.name_span.start() as usize..proc_def.name_span.end() as usize)
        .is_some_and(|t| tcl_syntax::naming::is_qualified(t.as_bytes()));
    let new_decl_text = if decl_was_qualified {
        qualified_decl
    } else {
        new_name.to_owned()
    };
    // Collision gate: renaming onto an existing proc of the same qualified
    // name would shadow it — refuse.  Compare names normalised to the
    // leading-`::` form.
    if analysis
        .all_procs
        .keys()
        .any(|qn| qn != &proc_def.qualified_name && qn == &new_qualified)
    {
        return Some(Ok(Vec::new()));
    }
    // Provenance gate: an indirect dispatch of this
    // proc whose contributing constants are not all source-writable cannot
    // be kept alive by any edit set — refuse the whole rename rather than
    // emit edits that leave the dispatch running the old name.
    if analysis.command_invocations.iter().any(|inv| {
        !inv.rename_safe
            && crate::references::invocation_references_proc(analysis, inv, qname, proc_def, source)
    }) {
        return Some(Ok(Vec::new()));
    }
    let mut edits = vec![PendingRenameEdit {
        span: proc_def.name_span,
        new_text: new_decl_text,
    }];
    for inv in &analysis.command_invocations {
        // An indirect site's span does not carry the written name (a constant
        // `$cmd` head) — rewriting it would splice the new name over
        // unrelated text.  References still report it; rename must not.
        if inv.indirect {
            continue;
        }
        // A `-map` key is an arbitrary name the target's rename does not
        // change — see [`ensemble_map_dispatch_word`].
        if ensemble_map_dispatch_word(inv.ensemble_dispatch) {
            continue;
        }
        // Use the *same* invocation-matching rule as Find-All-References so a
        // rename never rewrites a call the reference finder wouldn't report —
        // in particular the namespace gate that keeps a bare `helper` call in
        // `namespace eval ::b` from matching `::a::helper`.
        if !crate::references::invocation_references_proc(analysis, inv, qname, proc_def, source) {
            continue;
        }
        let replacement =
            invocation_replacement(namespace_prefix, &new_qualified, new_name, &inv.name);
        edits.push(PendingRenameEdit {
            span: inv.range,
            new_text: replacement,
        });
    }
    Some(finish_rename_edits(source, line_index, edits))
}

/// Class-rename path — `oo::class create ClassName`
/// declaration plus every `ClassName new` / `ClassName create
/// instance` invocation head.  Same shape as [`rename_proc`]:
/// prefers the class whose declaration name span covers the cursor (so
/// renaming from a same-named class's own decl in another namespace
/// resolves to *that* class, not the first one found), else the first
/// class matching the word.
fn rename_class(
    source: &str,
    word: &str,
    cursor_off: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    line_index: &LineIndex,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    let registry = resolution.registry;
    // Declaration under the cursor, else namespace-aware resolution — never a
    // namespace-blind `c.name == word` scan (which from a call site could
    // rename the wrong same-named class in another namespace).
    let (qname, class_def) =
        crate::definition::resolve_class_target_at(analysis, source, resolution, cursor_off, word)?;
    if let Some(registry) = registry
        && is_builtin_command_name(new_name, registry)
    {
        return Some(Ok(Vec::new()));
    }
    let namespace_prefix = namespace_prefix_of(&class_def.qualified_name);
    let (new_qualified, qualified_decl) = qualified_and_decl_text(namespace_prefix, new_name);
    // The declaration rewrite follows the *form the source wrote* — a class
    // declared short inside a `namespace eval` (`oo::class create Widget`)
    // stays short (`Panel`), one declared qualified
    // (`oo::class create ::ns::Widget`) stays qualified. Mirrors
    // `rename_proc`'s identical `decl_was_qualified` handling.
    let decl_was_qualified = source
        .get(class_def.name_span.start() as usize..class_def.name_span.end() as usize)
        .is_some_and(|t| tcl_syntax::naming::is_qualified(t.as_bytes()));
    let new_decl_text = if decl_was_qualified {
        qualified_decl
    } else {
        new_name.to_owned()
    };
    // Provenance gate — mirrors `rename_proc`: an
    // indirect dispatch with unwritable contributing constants cannot
    // follow the rename, so refuse it outright.
    if analysis.command_invocations.iter().any(|inv| {
        !inv.rename_safe
            && crate::references::invocation_references_class(
                analysis, inv, qname, class_def, source,
            )
    }) {
        return Some(Ok(Vec::new()));
    }
    let mut edits = vec![PendingRenameEdit {
        span: class_def.name_span,
        new_text: new_decl_text,
    }];
    for inv in &analysis.command_invocations {
        // Indirect sites are references, never rename targets — the span
        // does not carry the written name.
        if inv.indirect {
            continue;
        }
        // A `-map` key is an arbitrary name the target's rename does not
        // change — see [`ensemble_map_dispatch_word`].
        if ensemble_map_dispatch_word(inv.ensemble_dispatch) {
            continue;
        }
        // Use the *same* invocation-matching rule as Find-All-References so a
        // rename never rewrites a call the reference finder wouldn't report —
        // in particular the namespace gate that keeps a bare `ClassName new`
        // call in a *different* namespace from being rewritten.
        if !crate::references::invocation_references_class(analysis, inv, qname, class_def, source)
        {
            continue;
        }
        // Skip an invocation whose range contains the class
        // declaration site (defence in depth — the analyser's
        // recording shape doesn't usually file this here).
        if inv.range.start() <= class_def.name_span.start()
            && class_def.name_span.end() <= inv.range.end()
        {
            continue;
        }
        let replacement =
            invocation_replacement(namespace_prefix, &new_qualified, new_name, &inv.name);
        edits.push(PendingRenameEdit {
            span: inv.range,
            new_text: replacement,
        });
    }
    Some(finish_rename_edits(source, line_index, edits))
}

/// Method / property rename — locate the class whose body
/// contains the cursor, match `word` against its members,
/// then rewrite the declaration's name span plus every call
/// site inside any sibling method body that targets the same
/// name.
///
/// The analyser's `command_invocations` collection records
/// invocations at the top level only; method bodies aren't
/// walked.  To find call sites we re-segment each sibling
/// method body via [`tcl_compiler::segmenter::segment_commands_with_offset`]
/// and match on the segmented command's head token.
///
/// Returns `Some(edits)` when a class member matched `word`,
/// `None` otherwise.  `cursor_offset` is the byte offset for
/// containment filtering.
fn rename_method(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    word: &str,
    new_name: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
    line_index: &LineIndex,
) -> Option<Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal>> {
    use tcl_lexer::Span;

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
    let (selected_kind, name_span) = resolve_member_span(class_def, word, cursor_offset)?;
    let is_classmethod = selected_kind == MemberSel::ClassMethod;
    let mut edits = vec![PendingRenameEdit {
        span: name_span,
        new_text: new_name.to_owned(),
    }];
    // `command_invocations` records top-level invocations only; method
    // bodies aren't walked there.  `body_spans` (every method /
    // classmethod / constructor / destructor body) is the re-segmentable
    // material both branches below scan via the shared `my`-aware
    // matcher — a `TclOO` member is never a bare-callable command (a
    // bare `word` errors "invalid command name" at runtime; only `my
    // word` dispatches), so there is no separate bare-head scan here.
    let body_spans: Vec<Span> = crate::references::collect_member_bodies(class_def);
    // Methods / classmethods (not properties — those aren't dispatched
    // through the MRO) are one polymorphic name across the whole override
    // family: a method (re)defined by a super- or sub-class renames as a
    // unit.  For every family member — **including the class under the
    // cursor** — pull its declaration, intra-class `my method` sites (its
    // own bodies *and* any purely-inheriting subclass's), and external
    // `$obj method` sites via the shared resolver.  Routing the cursor
    // class through it too (rather than an ad-hoc self-only scan) is what
    // catches an inheriting subclass's `my method` / `$obj method` sites
    // when renaming from the base declaration.
    if matches!(selected_kind, MemberSel::Method | MemberSel::ClassMethod) {
        for member in override_family(analysis, &class_def.qualified_name, word) {
            let Some((decl_span, call_spans)) = crate::references::method_references_for_class(
                source,
                dialect,
                analysis,
                &member,
                word,
                is_classmethod,
            ) else {
                continue;
            };
            // The cursor class's own declaration edit is already queued.
            if member != class_def.qualified_name {
                edits.push(PendingRenameEdit {
                    span: decl_span,
                    new_text: new_name.to_owned(),
                });
            }
            for span in call_spans {
                edits.push(PendingRenameEdit {
                    span,
                    new_text: new_name.to_owned(),
                });
            }
        }
    } else {
        // Properties have no `$obj prop` dispatch and no inheritance model
        // (see `references::find_class_member_references`), so a
        // class-local `my <prop>` scan is the whole story.
        for span in
            crate::references::scan_my_method_sites(source, analysis, &body_spans, word, None)
        {
            edits.push(PendingRenameEdit {
                span,
                new_text: new_name.to_owned(),
            });
        }
    }
    Some(finish_rename_edits(source, line_index, edits))
}

/// Compute the qualified rewrite (`prefix::new`) and the
/// declaration-site rewrite (qualified when the original lived
/// in a namespace, short at the top level).  Shared by the
/// proc and class rename paths.
fn qualified_and_decl_text(namespace_prefix: &str, new_name: &str) -> (String, String) {
    let new_qualified = tcl_syntax::naming::qualify(namespace_prefix, new_name);
    let new_decl_text = if namespace_prefix.is_empty() {
        new_name.to_owned()
    } else {
        new_qualified.clone()
    };
    (new_qualified, new_decl_text)
}

/// Pick the rewrite shape for a single call/invocation site —
/// short form at the top level, qualified form when the call
/// itself was qualified.  Shared by the proc and class rename
/// paths.
fn invocation_replacement(
    _namespace_prefix: &str,
    _new_qualified: &str,
    new_name: &str,
    inv_name: &str,
) -> String {
    // Preserve the call's *as-written* qualifier: a qualified call replaces
    // only its final `::`-segment (`utils::helper` → `utils::assist`,
    // `::myns::greet` → `::myns::hello`); a short call becomes the short name.
    format!("{}{new_name}", source_name_qualifier(inv_name))
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

/// Compute the cross-document rename edits for a proc / class
/// identified by `(simple_name, qualified_name)` renamed to
/// `new_name`.  Walks the workspace index for every invocation
/// site (and definition site) in documents *other than*
/// `current_uri`, applying the same namespace-aware
/// replacement shape the in-document proc rename uses.
///
/// The caller (server) is responsible for the current document
/// — this returns only the sibling-document edits — and for
/// resolving each [`WorkspaceTextEdit`] span to an LSP range
/// against the target document's source.
#[must_use]
pub fn cross_document_symbol_edits(
    qualified_name: &str,
    new_name: &str,
    index: &crate::workspace_index::WorkspaceIndex,
    current_uri: &str,
) -> Vec<WorkspaceTextEdit> {
    if !index.allows_lexical_rename_advice() || !is_safe_symbol_name(new_name) {
        return Vec::new();
    }
    let namespace_prefix = namespace_prefix_of(qualified_name);
    let (new_qualified, new_decl_text) = qualified_and_decl_text(namespace_prefix, new_name);
    let mut edits = Vec::new();
    // Call sites.
    for inv in index.invocations_of(qualified_name, current_uri) {
        // Indirect sites (constant `$cmd` heads) are references, never
        // rename targets — their span is not the written name.
        if inv.indirect {
            continue;
        }
        // The cross-document half of the same `-map` gate the in-document
        // rename applies — see [`ensemble_map_dispatch_word`].  An ensemble
        // declared in one document and dispatched from another is exactly the
        // shape the recording covers.
        if ensemble_map_dispatch_word(inv.ensemble_dispatch) {
            continue;
        }
        let replacement =
            invocation_replacement(namespace_prefix, &new_qualified, new_name, &inv.name);
        edits.push(WorkspaceTextEdit {
            uri: inv.uri.clone(),
            span: inv.range,
            new_text: replacement,
        });
    }
    // Name-link declarations that reference the command by qualified name —
    // the `rename OLD NEW` `OLD` word, the `namespace import` pattern.  Each
    // names the renamed command and must follow it to stay bound; the new
    // fully-qualified name is always a valid replacement for a qualified
    // reference.  (The `interp alias` `TARGET` word is already a call site
    // rewritten above, so it is not among these spans.)  The local imported /
    // aliased *usages* are deliberately left alone — they name the local
    // command, which keeps its own name.
    for (link_uri, span) in index.link_target_spans(qualified_name, current_uri) {
        edits.push(WorkspaceTextEdit {
            uri: link_uri,
            span,
            new_text: new_qualified.clone(),
        });
    }
    // Definition sites (proc + class) in other documents — matched by
    // *qualified* name so a same-simple-name proc in a different namespace is
    // not rewritten (and moved into the target's namespace).
    for p in index.proc_definitions_qualified(qualified_name, current_uri) {
        edits.push(WorkspaceTextEdit {
            uri: p.uri.clone(),
            span: p.name_span,
            new_text: new_decl_text.clone(),
        });
    }
    for c in index.class_definitions_qualified(qualified_name, current_uri) {
        edits.push(WorkspaceTextEdit {
            uri: c.uri.clone(),
            span: c.name_span,
            new_text: new_decl_text.clone(),
        });
    }
    edits
}

/// Compute the **whole-workspace** edit set for renaming the command whose
/// qualified name is `qualified_name` — every call site, name-link word, and
/// definition site the index knows, the current document included.
///
/// This is the consumer-document rename path: the cursor sits on a call
/// whose definition lives in another document (a workspace sibling, or a
/// library file the autoload tier merged), so the in-document rename has no
/// local definition to resolve against and produces nothing.  The index
/// supplies everything instead.  Returns `None` when the rename would shadow
/// an existing workspace command — the same collision discipline as the
/// in-document rename's local gate.
#[must_use]
pub fn workspace_symbol_rename_edits(
    qualified_name: &str,
    new_name: &str,
    index: &crate::workspace_index::WorkspaceIndex,
) -> Option<Vec<WorkspaceTextEdit>> {
    if !index.allows_lexical_rename_advice() || !is_safe_symbol_name(new_name) {
        return None;
    }
    let namespace_prefix = namespace_prefix_of(qualified_name);
    let (new_qualified, _) = qualified_and_decl_text(namespace_prefix, new_name);
    if index.workspace_command_exists(&new_qualified) {
        return None;
    }
    // Provenance gate: an indirect dispatch of this
    // command anywhere in the workspace whose contributing constants are
    // not all source-writable cannot follow any edit set — abstain.
    if index.rename_blocked(qualified_name) {
        return None;
    }
    Some(cross_document_symbol_edits(
        qualified_name,
        new_name,
        index,
        "",
    ))
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
fn var_ref_edit_span(source: &str, span: tcl_lexer::Span) -> tcl_lexer::Span {
    tcl_lexer::word_span_at(source, span)
}

/// Selected whole-reference source projection. The caller's independent
/// Logical rename gates supply edit authority; this helper supplies syntax
/// and refuses malformed, compound or unavailable source geometry.
#[cfg(test)]
fn build_var_ref_replacement(
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
fn build_var_replacement_in_analysis(
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

fn source_name_qualifier(name: &str) -> &str {
    // Original written bytes, not a constructed holder or native identity.
    // The shared tail consumes every colon in its final separator run.
    let tail = tcl_syntax::naming::written_command_tail(name.as_bytes());
    &name[..name.len() - tail.len()]
}

/// Closed literal element partition shared with variable naming. An unmatched
/// open or trailing text belongs to the scalar name and has no index suffix.
#[cfg(test)]
fn split_array_suffix(name: &str) -> (&str, &str) {
    tcl_syntax::naming::split_element_ref(name)
        .map_or((name, ""), |(root, _)| (root, &name[root.len()..]))
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

#[cfg(test)]
mod conditional_source_alpha_tests;
