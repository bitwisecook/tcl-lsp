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

//! Which pattern or format language an argument word holds, and the card for it.

use super::Hover;
use super::format_specifiers::{
    BinaryContext, binary_format_hover_text, clock_format_hover_text, sprintf_format_hover_text,
};
use super::pattern_syntax::{
    glob_hover_text, regex_hover_text, regsub_hover_text, scan_regsub_backrefs,
};
use crate::original_invocation::schema_format_arguments;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_lexer::TokenType;
use tcl_registry::CommandRegistry;

/// Embedded pattern and format advice from the retained source inventory.
/// All source domains keep their exact source, configuration and context seal;
/// missing original operands do not enable a nominal compatibility walker.
/// The result describes a retained source operand, never a method or runtime
/// binding. Hosts must preserve their stronger workspace selection and leases.
#[must_use]
pub fn registry_pattern_format_hover(
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

/// Conditional embedded-language source advice with stronger indexed command
/// occupation preserved. The same retained catalogue owns the path alternatives,
/// residual conditions, selected schema and original operand; no runtime absence
/// or method binding follows from an empty indexed match.
#[must_use]
pub fn registry_pattern_format_hover_in_workspace(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    index: &crate::workspace_index::WorkspaceIndex,
) -> Option<Hover> {
    let line_index = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
    let mut hover = original_pattern_format_hover_with_workspace(
        source,
        analysis,
        registry,
        cursor,
        Some(index),
    )?;
    hover
        .value
        .push_str("\n\nPattern syntax applies if this is the expected command implementation.");
    Some(hover)
}

/// Readonly embedded-language advice from the same original effective argv.
pub(super) fn original_pattern_format_hover(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    cursor: u32,
) -> Option<Hover> {
    original_pattern_format_hover_with_workspace(source, analysis, registry, cursor, None)
}

fn original_pattern_workspace_allowed(
    words: &crate::original_invocation::OriginalRegistryWords,
    index: &crate::workspace_index::WorkspaceIndex,
) -> Option<()> {
    use tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource;
    let OriginalRegistrySource::Conditional(metadata) = &words.source else {
        return None;
    };
    let paths = metadata.possible_source_command_paths();
    if paths.is_empty() || paths.iter().any(Vec::is_empty) {
        return None;
    }
    let policy = metadata.original_head().name_input().policy();
    let occupied = index
        .original_command_publications()
        .map(|(_, publication)| (publication.slot(), publication.policy()))
        .chain(
            index
                .original_class_source_candidates()
                .map(|(_, _, slot, policy)| (slot, policy)),
        )
        .chain(
            index
                .original_procedure_source_candidates()
                .map(|(_, _, slot, policy)| (slot, policy)),
        )
        .any(|(slot, occupied_policy)| {
            occupied_policy == policy && paths.iter().any(|path| path.contains(slot))
        });
    if occupied {
        return None;
    }
    // The original catalogue has already refused unbounded namespace paths
    // and nonunanimous descriptor rows. All its applicability/holder/lookup
    // obligations remain on this same metadata; none is discharged here.
    Some(())
}

fn original_pattern_format_hover_with_workspace(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    cursor: u32,
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
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
    if let Some(index) = workspace {
        original_pattern_workspace_allowed(&words, index)?;
    }
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
        .with_source_schema(&context, schema_format_arguments)
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
                original_binary_format_hover_text(&words, format.index, &text)?
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

fn original_binary_format_hover_text(
    words: &crate::original_invocation::OriginalRegistryWords,
    index: usize,
    text: &str,
) -> Option<String> {
    let arguments = words
        .arguments
        .iter()
        .map(|argument| {
            argument
                .literal_bytes()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
        })
        .collect::<Vec<_>>();
    Some(binary_format_hover_text(&BinaryContext {
        text: text.to_owned(),
        subcmd: arguments.first().copied().flatten()?.to_owned(),
        args: arguments
            .get(index.checked_add(1)?..)?
            .iter()
            .map(|argument| argument.unwrap_or_default().to_owned())
            .collect(),
    }))
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
