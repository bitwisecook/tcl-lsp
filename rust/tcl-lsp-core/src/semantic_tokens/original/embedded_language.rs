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

//! Semantic tokens for retained embedded patterns and conversion languages.

use super::{ArgOverride, ScriptCtx, TokenKind, original_argument_token};
use crate::semantic_tokens::mark_literal_fragments;
use rustc_hash::FxHashMap;
use tcl_compiler::segmenter::SegmentedCommand;

/// Embedded conversion languages use the retained schema for every source
/// domain and only the exact original operand at each effective ordinal.
pub(in crate::semantic_tokens) fn insert_format_overrides(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let _ = words.with_source_schema(ctx.generation, |schema| {
        for found in schema
            .authored_source_format_arguments()
            .unwrap_or_default()
        {
            if let Some(token) = original_argument_token(command, words, found.index) {
                let kind = match found.kind {
                    tcl_registry::FormatType::Sprintf if found.scan => {
                        let Some(parts) = template_subtoken_spans(ctx, words, found) else {
                            continue;
                        };
                        ArgOverride::TemplateParts(TokenKind::String, parts)
                    }
                    tcl_registry::FormatType::Sprintf => ArgOverride::SprintfFormat,
                    tcl_registry::FormatType::Clock => ArgOverride::ClockFormat,
                    tcl_registry::FormatType::Binary => ArgOverride::BinaryFormat,
                    tcl_registry::FormatType::Regsub => {
                        let Some(captures) = template_subtoken_spans(ctx, words, found) else {
                            continue;
                        };
                        ArgOverride::TemplateParts(TokenKind::String, captures)
                    }
                };
                mark_literal_fragments(command, token.span, &kind, overrides);
            }
        }
    });
}

fn template_subtoken_spans(
    ctx: ScriptCtx<'_>,
    words: &crate::original_invocation::OriginalRegistryWords,
    format: tcl_registry::FormatStringArg,
) -> Option<Vec<(std::ops::Range<usize>, TokenKind)>> {
    let input = ctx.analysis?.resolved_input.as_ref()?;
    let operand = crate::format_template::FormatOperand::capture(words, input, format.index)?;
    let template = crate::format_template::FormatTemplate::from_operand(ctx.full_source, &operand)?;
    let mut parts = Vec::new();
    let mut push = |span: std::ops::Range<usize>, kind| {
        if !span.is_empty() {
            parts.push((
                template.original_offset(span.start)?..template.original_offset(span.end)?,
                kind,
            ));
        }
        Some(())
    };
    match format.kind {
        tcl_registry::FormatType::Sprintf if format.scan => {
            for field in tcl_syntax::scan::conversion_spans(&template.content, ctx.dialect) {
                push(field.percent, TokenKind::FormatPercent)?;
                if field.spec.xpg_index.is_some() {
                    let dollar = field.assignment.end - 1;
                    push(field.assignment.start..dollar, TokenKind::FormatWidth)?;
                    push(dollar..field.assignment.end, TokenKind::FormatPercent)?;
                } else {
                    push(field.assignment, TokenKind::FormatFlag)?;
                }
                push(field.width, TokenKind::FormatWidth)?;
                push(field.size, TokenKind::FormatFlag)?;
                push(field.conversion, TokenKind::FormatSpec)?;
            }
        }
        tcl_registry::FormatType::Regsub => {
            for (span, part) in tcl_cmd_core::regex::replacement_parts(template.content.as_bytes())
            {
                if let tcl_cmd_core::regex::ReplacementPart::Capture(_) = part {
                    let kind = if span.len() == 1 {
                        TokenKind::Operator
                    } else {
                        TokenKind::Number
                    };
                    push(span, kind)?;
                }
            }
        }
        _ => return None,
    }
    (!parts.is_empty()).then_some(parts)
}

/// Embedded patterns from the same retained schema and source-word geometry
/// for Native, hosted and explicitly Logical input. Missing receipts decline;
/// source text cannot reselect an embedded-language descriptor.
pub(in crate::semantic_tokens) fn insert_pattern_overrides(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    words: &crate::original_invocation::OriginalRegistryWords,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    let _ = words.with_source_schema(ctx.generation, |schema| {
        for pattern in schema
            .authored_source_pattern_arguments()
            .unwrap_or_default()
        {
            if pattern.kind == tcl_registry::patterns::PatternType::Regex
                && let Some(token) =
                    original_argument_token(command, words, usize::from(pattern.index))
            {
                let index = usize::from(pattern.index);
                let mapped = ctx
                    .analysis
                    .and_then(|analysis| analysis.resolved_input.as_ref())
                    .and_then(|input| {
                        crate::format_template::FormatOperand::capture(words, input, index)
                    })
                    .and_then(|operand| {
                        let template = crate::format_template::FormatTemplate::from_operand(
                            ctx.full_source,
                            &operand,
                        )?;
                        let roles = crate::pattern_components::components(
                            &template.content,
                            ctx.dialect,
                            crate::pattern_components::source_flags(schema),
                        )?;
                        Some(
                            crate::semantic_tokens::regex_subtokens::original_component_spans(
                                roles,
                                |offset| template.original_offset(offset),
                            ),
                        )
                    });
                let kind = mapped.map_or(ArgOverride::RegexPattern, |parts| {
                    ArgOverride::TemplateParts(TokenKind::Regexp, parts)
                });
                mark_literal_fragments(command, token.span, &kind, overrides);
            }
        }
    });
}
