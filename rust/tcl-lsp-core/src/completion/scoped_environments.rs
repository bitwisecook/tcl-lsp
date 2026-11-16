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

//! Candidates inside a scoped command environment or a definition-body grammar.

use super::commands::command_detail;
use super::{
    CompletionItem, CompletionKind, FilteredCandidates, decorate_fuzzy_items, filter_candidates,
    original_word_completion_edits,
};
use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_registry::CommandRegistry;

/// The scoped command environment active at the cursor, if the position falls
/// inside a recorded scoped body (a `report::defstyle` style script).
pub(super) fn scoped_env_at(
    analysis: &AnalysisResult,
    source: &str,
    line: u32,
    character: u32,
    _line_index: &tcl_lexer::LineIndex,
) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
    let position =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)?;
    let offset = position.offset();
    if !analysis.allows_lexical_declaration_advice() {
        let body = analysis.original_scoped_body_in_source(
            &tcl_lexer::SourceImage::document(source),
            analysis.body_lexer_config?,
            offset,
        )?;
        return Some(body.environment());
    }
    let inventory = crate::source_structure::SourceSyntaxCoverage::capture(source, analysis)?;
    let scoped = inventory.logical_scoped_environment_at(source, analysis, offset)?;
    scoped.environment(&analysis.resolved_input.as_ref()?.context_registry())
}

/// Current authored vocabulary at a strict original source position.
/// Ordinary Tcl definition scripts keep their open command completion purpose.
/// A declaration document separately retains selected nested grammar, explicit
/// root authoring vocabulary/debt, ordinary script context or unavailable source.
pub(super) fn definition_grammar_at_position(
    source: &str,
    line: u32,
    character: u32,
    _line_index: &tcl_lexer::LineIndex,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> crate::source_structure::SourceDefinitionGrammar {
    use crate::source_structure::{DefinitionGrammarUnavailable, SourceDefinitionGrammar};
    // Ordinary Tcl definition scripts remain open completion positions. This
    // consumer closes only explicit standalone authoring vocabularies.
    if registry.document_grammar().is_none() {
        return SourceDefinitionGrammar::OrdinaryScript;
    }
    let Some(position) =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)
    else {
        return SourceDefinitionGrammar::Unavailable(DefinitionGrammarUnavailable::CurrentSource);
    };
    if !std::ptr::eq(position.context().registry(), registry) {
        return SourceDefinitionGrammar::Unavailable(DefinitionGrammarUnavailable::CurrentSource);
    }
    crate::oo_body::definition_grammar_from_analysis(source, analysis, position.offset())
}

/// Completion items for the member words of a definition-body grammar — the
/// declarations legal at this position and nothing else.
///
/// A member's detail and documentation come from its own [`CommandSpec`] when
/// it has one (every `SslicTcl` and `SpecTcl` word does), so a grammar-driven
/// item is as informative as an ordinary command item; a member that is only a
/// grammar row still completes, with no detail.
pub(super) fn grammar_member_completions(
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    partial: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ResolvedContext,
) -> Vec<CompletionItem> {
    // naming.consumer.selected-catalogue-completion-provider
    // docs/design/analysis/name-resolution-proofs/selected-catalogue-completion-provider.md
    let query = context.authoring_query();
    grammar
        .members
        .iter()
        .filter(|member| {
            (partial.is_empty() || member.keyword.starts_with(partial))
                && grammar
                    .source_member_in(member.keyword.as_bytes(), Some(query))
                    .is_some_and(|actual| std::ptr::eq(actual, *member))
        })
        .filter_map(|member| {
            let insert_text =
                tcl_syntax::word_rules::lexical_unicode_source_word(member.keyword, config)?;
            let spec = context.resolve_spec(registry, member.keyword);
            Some(CompletionItem {
                label: member.keyword.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: spec.map(|s| command_detail(s, registry)),
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: spec
                    .and_then(|s| s.hover.as_ref())
                    .map(|h| h.summary.to_owned()),
            })
        })
        .collect()
}

/// Completion items for the command heads of a scoped environment matching
/// `partial` (`top`, `data`, `columns`, … inside a `report::defstyle` body).
pub(super) fn scoped_command_completions(
    env: &'static tcl_registry::scoped::ScopedCommandEnv,
    partial: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<CompletionItem> {
    env.commands
        .iter()
        .filter(|c| partial.is_empty() || c.name.starts_with(partial))
        .filter_map(|c| {
            let insert_text = tcl_syntax::word_rules::lexical_unicode_source_word(c.name, config)?;
            Some(CompletionItem {
                label: c.name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: (!c.detail.is_empty()).then(|| c.detail.to_owned()),
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            })
        })
        .collect()
}

/// Completion items for the ensemble operations of a scoped command
/// (`set` / `get` / `enable` / … after `top ` in a `report::defstyle` body).
pub(super) fn scoped_op_completions(
    cmd: &'static tcl_registry::scoped::ScopedCommand,
    partial: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<CompletionItem> {
    let ops: Vec<&tcl_registry::SubCommand> = cmd.subcommands.iter().collect();
    let FilteredCandidates {
        candidates: ops,
        fuzzy,
    } = filter_candidates(partial, ops, |op| op.name);
    let mut items: Vec<CompletionItem> = ops
        .into_iter()
        .filter_map(|s| {
            let insert_text = tcl_syntax::word_rules::lexical_unicode_source_word(s.name, config)?;
            Some(CompletionItem {
                label: s.name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: (!s.detail.is_empty()).then(|| s.detail.to_owned()),
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            })
        })
        .collect();
    if fuzzy {
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}

pub(super) fn extend_scoped_completion_items(
    analysis: &AnalysisResult,
    position: (&str, u32, u32, &tcl_lexer::LineIndex),
    partial: &str,
    current: &crate::original_context::CurrentSourceContext<'_>,
    items: &mut Vec<CompletionItem>,
) {
    let (source, line, character, line_index) = position;
    if let Some(env) = scoped_env_at(analysis, source, line, character, line_index) {
        let present: FxHashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        for item in scoped_command_completions(env, partial, current.config()) {
            if !present.contains(&item.label) {
                items.push(item);
            }
        }
    }
}

pub(super) fn definition_grammar_completion_items(
    source: &str,
    position: (u32, u32, &tcl_lexer::LineIndex),
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    current: &crate::original_context::CurrentSourceContext<'_>,
    partial: &str,
    prefix: &tcl_lexer::word_parts::WordCursorPrefix,
) -> Option<Vec<CompletionItem>> {
    let (line, character, line_index) = position;
    if let Some(registry) = registry {
        let vocabulary =
            definition_grammar_at_position(source, line, character, line_index, analysis, registry);
        if crate::source_structure::SourceDefinitionGrammar::unavailable(&vocabulary).is_some() {
            return Some(Vec::new());
        }
        // A standalone root authoring projection may retain unavailable child
        // traversal. It is not a selected nested body or an edit vocabulary.
        if let Some(grammar) = crate::source_structure::SourceDefinitionGrammar::for_authoring(
            &vocabulary,
            source,
            analysis,
        ) {
            return Some(original_word_completion_edits(
                source,
                line,
                prefix,
                grammar_member_completions(
                    grammar,
                    partial,
                    registry,
                    current.config(),
                    current.context().context(),
                ),
            ));
        }
    }

    None
}
