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

//! Cards for user commands: procedures with their docstrings, ensemble subcommands and aliases.

use super::Hover;
use tcl_compiler::analyser::{AnalysisResult, ProcDef};

/// `<ensemble> <subcommand>` hover — a static `namespace ensemble create
/// -map`/`-subcommands` mapping, the hover twin of
/// `definition()`'s identical check. Extracted from
/// [`hover_with_profile`] to stay within the line budget.
pub(super) fn ensemble_subcommand_hover(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    ctx: crate::definition::CallResolution<'_>,
) -> Option<Hover> {
    // Logical source inventories do not select static ensemble map entries.
    // The report-map target is unavailable as a substitute for that binding.
    if analysis.allows_retained_logical_declaration_advice() {
        return None;
    }
    let (head, sub, is_dollar) =
        crate::definition::original_instance_method_at_cursor(source, analysis, line, character)?;
    if is_dollar {
        return None;
    }
    let line_index = tcl_lexer::LineIndex::new(source);
    let cursor_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    let proc_def = crate::definition::ensemble_called_procedure(
        analysis,
        source,
        (&head, &sub),
        cursor_offset,
        ctx,
    )?;
    let text = format!(
        "**Ensemble subcommand** of `{head}`\n\n{}",
        proc_hover_text(proc_def)
    );
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
pub(super) fn proc_hover_at(
    analysis: &AnalysisResult,
    source: &str,
    cursor_offset: u32,
    word: &str,
    ctx: crate::definition::CallResolution<'_>,
) -> Option<Hover> {
    if analysis.allows_retained_logical_declaration_advice() {
        let config = analysis.body_lexer_config?;
        let image = tcl_lexer::SourceImage::document(source);
        if !analysis.matches_original_source_image(&image, config) {
            return None;
        }
        if let Some(declarations) =
            analysis.checked_logical_source_procedure_declarations(&image, config)
        {
            let mut selected = declarations.into_iter().filter(|declaration| {
                let span = declaration.metadata().name_span;
                span.start() <= cursor_offset && cursor_offset < span.end()
            });
            if let Some(declaration) = selected.next() {
                return selected
                    .next()
                    .is_none()
                    .then(|| Hover::markdown(proc_hover_text(declaration.metadata())));
            }
        }
        let call = tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_for_cursor(
            source,
            analysis,
            cursor_offset,
        )?;
        let declaration = call.declaration(analysis)?;
        return Some(Hover::markdown(format!(
            "**Possible procedure**\n\n{}",
            proc_hover_text(declaration.metadata())
        )));
    }
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

/// Alias hover at a genuine command head. Original aliases require their
/// post-argv publication and implementation receipt; lexical-only profiles
/// retain a separate declaration-advice presentation path.
pub(super) fn alias_hover_at(
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
    let allow_legacy_alias_labels = analysis.allows_lexical_declaration_advice()
        && !analysis.allows_retained_logical_declaration_advice();
    let Some(reference) = invocation.resolved_command_reference.as_ref() else {
        if allow_legacy_alias_labels {
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
    if answer.is_none() && allow_legacy_alias_labels {
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

pub(super) fn proc_hover_text(proc_def: &ProcDef) -> String {
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
pub(super) fn format_docstring(text: &str) -> String {
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
                .map_or_default(|s| s.trim_start_matches(['-', ' ']).to_string());
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
