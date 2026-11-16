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

//! Effective words of an invocation: alias prefixes, expansions and frozen argument values.

use super::word_resolution::{EffectiveInvocationWord, effective_invocation_word, invocation_word};
use super::{EffectiveCommandWords, InvocationWordOrigin};
use crate::ir::{CommandTokens, WordExpr};
use tcl_registry::InvocationWord;

/// Compose effective execution words exactly once from the shared point binding.
/// Native compiled selection can differ from the later live lookup target.
/// An explicitly uncertain binding never falls back to source spelling.
#[must_use]
pub fn effective_command_words(tokens: &CommandTokens) -> Option<EffectiveCommandWords> {
    let Some(binding) = &tokens.source_binding else {
        return Some(EffectiveCommandWords {
            words: tokens.words().to_vec(),
            origins: (0..tokens.words().len())
                .map(InvocationWordOrigin::Written)
                .collect(),
            binding_prefix: Vec::new(),
        });
    };
    effective_words_for_target(tokens, binding.proved_execution_target()?)
}

pub(super) fn original_logical_prefix_for_tokens<'a>(
    tokens: &CommandTokens,
    target: &'a crate::command_binding::SourceCommandTarget,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<&'a [crate::signature_scan::scope::SignatureSourceNameInput]> {
    let binding = tokens.source_binding.as_ref()?;
    target.original_logical_prepended_name_inputs(
        input,
        binding.invocation_site()?.source.source_image(),
        binding.original_lexer_config_for_tokens(tokens)?,
    )
}

/// Compose a single candidate using the same ordering as proved dispatch.
pub(crate) fn effective_words_for_target(
    tokens: &CommandTokens,
    target: &crate::command_binding::SourceCommandTarget,
) -> Option<EffectiveCommandWords> {
    if !target.prepended.is_empty()
        && let Some(input) = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_logical_prefix_source_input())
    {
        original_logical_prefix_for_tokens(tokens, target, input)?;
    }
    let mut effective = compose_effective_words(tokens, &target.command, &target.prepended)?;
    if let Some(named) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.named_invocation())
    {
        discard_named_selection_operands(&mut effective, named.arguments_from);
    }
    Some(effective)
}

pub(super) fn source_effective_words_for_target(
    tokens: &CommandTokens,
    target: &crate::command_binding::SourceCommandTarget,
) -> Option<EffectiveCommandWords> {
    if !target.prepended.is_empty()
        && let Some(input) = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_logical_prefix_source_input())
    {
        original_logical_prefix_for_tokens(tokens, target, input)?;
    }
    // Source descriptors retain their selectors; private handler argv does not.
    compose_effective_words(tokens, &target.command, &target.prepended)
}

pub(super) fn discard_named_selection_operands(
    effective: &mut EffectiveCommandWords,
    arguments_from: usize,
) {
    let mut index = 0;
    effective.words.retain(|_| {
        let retain = !matches!(effective.origins[index], InvocationWordOrigin::Written(written) if written <= arguments_from);
        index += 1;
        retain
    });
    effective.origins.retain(|origin| !matches!(origin, InvocationWordOrigin::Written(written) if *written <= arguments_from));
}

pub(super) fn compose_effective_words(
    tokens: &CommandTokens,
    command: &str,
    prepended: &[EffectiveInvocationWord],
) -> Option<EffectiveCommandWords> {
    let original = compose_original_effective_words(tokens, command, prepended)?;
    let Some(frozen) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.frozen_written_words())
    else {
        return Some(original);
    };
    if frozen.len() != tokens.words().len() {
        return None;
    }
    let mut effective = EffectiveCommandWords {
        words: Vec::new(),
        origins: Vec::new(),
        binding_prefix: original.binding_prefix.clone(),
    };
    for (word, origin) in original.words.into_iter().zip(original.origins) {
        if !matches!(origin, InvocationWordOrigin::Written(_)) {
            effective.words.push(word);
            effective.origins.push(origin);
        }
    }
    let mut needs_head = true;
    for (written, (word, frozen)) in tokens.words().iter().zip(frozen).enumerate() {
        if let Some(count) = frozen.expansion_len() {
            if count == 0 {
                continue;
            }
            append_expansion_elements(
                &mut effective,
                word,
                written,
                frozen,
                usize::from(needs_head),
            );
        } else if matches!(frozen, EffectiveInvocationWord::Expanded) {
            return None;
        } else if !needs_head {
            effective.words.push(word.clone());
            effective
                .origins
                .push(InvocationWordOrigin::Written(written));
        }
        needs_head = false;
    }
    if needs_head {
        return None;
    }
    Some(effective)
}

fn append_expansion_elements(
    effective: &mut EffectiveCommandWords,
    word: &WordExpr,
    written: usize,
    elements: &EffectiveInvocationWord,
    from: usize,
) {
    for element in from..elements.expansion_len().unwrap_or(0) {
        let text = elements
            .expansion_element(element)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .unwrap_or_default();
        effective.words.push(WordExpr::Opaque {
            text: text.to_owned(),
            source: crate::ir::SourceSite::opaque(word.source().span),
            reason: crate::ir::WordOpacity::LossySnapshot,
        });
        effective
            .origins
            .push(InvocationWordOrigin::ExpandedElement { written, element });
    }
}

pub(crate) fn compose_original_effective_words(
    tokens: &CommandTokens,
    command: &str,
    prepended: &[EffectiveInvocationWord],
) -> Option<EffectiveCommandWords> {
    let span = tokens.words().first()?.source().span;
    let mut words = vec![WordExpr::Literal {
        text: command.to_owned(),
        source: crate::ir::SourceSite::opaque(span),
    }];
    let mut origins = vec![InvocationWordOrigin::ResolvedHead];
    for (index, value) in prepended.iter().enumerate() {
        words.push(match value {
            EffectiveInvocationWord::Literal(text) => WordExpr::Literal {
                text: text.clone(),
                source: crate::ir::SourceSite::opaque(span),
            },
            _ => WordExpr::Opaque {
                text: String::new(),
                source: crate::ir::SourceSite::opaque(span),
                reason: crate::ir::WordOpacity::LossySnapshot,
            },
        });
        origins.push(InvocationWordOrigin::BindingPrefix(index));
    }
    for (index, word) in tokens.words().iter().enumerate().skip(1) {
        words.push(word.clone());
        origins.push(InvocationWordOrigin::Written(index));
    }
    Some(EffectiveCommandWords {
        words,
        origins,
        binding_prefix: prepended.to_vec(),
    })
}

/// Values retained before later argument substitutions change interpreter state.
/// Expansion and recovery words cannot acquire a fixed argument cardinality.
pub(super) fn frozen_argument_values(
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
) -> Vec<Option<String>> {
    effective
        .words
        .iter()
        .zip(&effective.origins)
        .skip(1)
        .map(|(word, origin)| {
            if let InvocationWordOrigin::ExpandedElement { written, element } = origin {
                let values = tokens
                    .source_binding
                    .as_ref()?
                    .frozen_written_words()?
                    .get(*written)?;
                return values
                    .expansion_element(*element)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    .map(str::to_owned);
            }
            if matches!(word, WordExpr::Expand { .. } | WordExpr::Opaque { .. }) {
                return None;
            }
            match origin {
                InvocationWordOrigin::BindingPrefix(_) => match word {
                    WordExpr::Literal { text, .. } => Some(text.clone()),
                    _ => None,
                },
                InvocationWordOrigin::Written(index) => tokens
                    .source_binding
                    .as_ref()?
                    .evaluated_argument_values
                    .get(index.checked_sub(1)?)
                    .cloned()
                    .flatten(),
                InvocationWordOrigin::ExpandedElement { .. }
                | InvocationWordOrigin::ResolvedHead => None,
            }
        })
        .collect()
}

/// Readonly evaluated arguments aligned with the supplied effective origins.
/// Captured alias operands and retained native expansion children preserve
/// their exact bytes; no value is re-evaluated and no source geometry, handler
/// execution or completion proof is issued by this projection.
#[must_use]
pub fn frozen_argument_words(
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
) -> Vec<EffectiveInvocationWord> {
    effective
        .words
        .iter()
        .zip(&effective.origins)
        .skip(1)
        .map(|(word, origin)| match origin {
            InvocationWordOrigin::Written(index) => tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.evaluated_argument_words.get(index - 1))
                .cloned()
                .unwrap_or_else(|| {
                    if let Some(dialect) = tokens
                        .source_binding
                        .as_ref()
                        .and_then(|binding| binding.variable_context.invocation_dialect)
                    {
                        let value = effective_invocation_word(
                            word,
                            dialect.lexer_grammar.escapes,
                            dialect.word_values,
                        );
                        return if value == EffectiveInvocationWord::Dynamic {
                            tokens
                                .source_binding
                                .as_ref()
                                .and_then(|binding| {
                                    binding.original_compound_array_argument(
                                        tokens,
                                        *index,
                                        dialect.lexer_grammar.escapes,
                                        dialect.word_values,
                                    )
                                })
                                .unwrap_or(value)
                        } else {
                            value
                        };
                    }
                    match invocation_word(word) {
                        InvocationWord::Literal(text) => {
                            EffectiveInvocationWord::Literal(text.to_owned())
                        }
                        InvocationWord::KnownBytes(value) => {
                            EffectiveInvocationWord::from_bytes(value)
                        }
                        InvocationWord::Expanded => EffectiveInvocationWord::Expanded,
                        InvocationWord::Opaque => EffectiveInvocationWord::Opaque,
                        _ => EffectiveInvocationWord::Dynamic,
                    }
                }),
            InvocationWordOrigin::ExpandedElement { written, element } => tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.frozen_written_words())
                .and_then(|words| words.get(*written))
                .and_then(|word| word.expansion_element(*element))
                .map_or(
                    EffectiveInvocationWord::Dynamic,
                    EffectiveInvocationWord::from_bytes,
                ),
            InvocationWordOrigin::BindingPrefix(index) => effective
                .binding_prefix
                .get(*index)
                .cloned()
                .unwrap_or(EffectiveInvocationWord::Dynamic),
            InvocationWordOrigin::ResolvedHead => EffectiveInvocationWord::Dynamic,
        })
        .collect()
}
