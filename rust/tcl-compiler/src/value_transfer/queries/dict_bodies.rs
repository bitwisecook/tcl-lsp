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

//! What a dictionary body command binds on entry: the dictionary operand, its binders and their key paths.

use super::super::argument_words::view_of;
use super::{Prior, StructureInputs};
use crate::ir::CommandTokens;
#[cfg(test)]
use crate::value_transfer::word_of;
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::{
    AnalysisContext, BinderName, InvocationLayout, OperandId, PlanAnswer,
};
#[cfg(test)]
use tcl_registry::{InvocationWord, InvocationWords};
use tcl_syntax::word_rules::WordValueRules;

/// One variable a dictionary body binds on entry ([`dict_body`]).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DictBinder {
    /// A key the dictionary holds, bound to the variable of its name, with
    /// the value it binds.
    Key {
        /// The key and variable name.
        name: String,
        /// The key's value.
        value: String,
    },
    /// The variable operand at `variable`, bound to the value of the key
    /// operand before it when the dictionary holds that key.
    Variable {
        /// The variable operand's argument index.
        variable: usize,
        /// Whether the dictionary holds the key, so the variable is bound on
        /// entry.
        bound: bool,
    },
}

/// The dictionary operand, the binders and the key path of the plan `head
/// args…` declares, when it binds a dictionary's keys into its body and
/// writes them back (`dict with`, `dict update`, under whichever spelling
/// the registry resolves), asked over `prior`. The key path is the exact
/// words the plan read on its way to the dictionary's keys.
#[cfg(test)]
fn dict_body_plan(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
    prior: Prior<'_>,
) -> Option<(
    OperandId,
    Vec<tcl_registry::value_transfer::Binder>,
    Vec<String>,
)> {
    let texts: Vec<&str> = args.iter().map(String::as_str).collect();
    let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()?;
    let semantics = resolved.semantics.value.semantics()?;
    let context = AnalysisContext::detached(registry.profile());
    let mut inputs = StructureInputs::new(
        view_of(&resolved, &texts, &words, InvocationLayout::Source),
        &context,
    );
    inputs.prior = prior;
    let PlanAnswer::Body {
        binders,
        reconcile: tcl_registry::value_transfer::Reconcile::WriteBackKeys(dict),
        ..
    } = semantics.structure(&inputs)
    else {
        return None;
    };
    let path = inputs
        .exact_read_texts()
        .into_iter()
        .map(str::to_owned)
        .collect();
    Some((dict, binders, path))
}

/// The argument index of the dictionary variable when the plan `head
/// args…` declares binds a dictionary's keys into its body and writes them
/// back — whatever the dictionary holds, so a call whose keys the analysis
/// cannot name is still found.
#[cfg(test)]
pub(crate) fn dict_body_operand(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
) -> Option<usize> {
    dict_body_plan(registry, head, args, Prior::KeyPath).map(|(dict, ..)| dict.0)
}

/// What the dictionary body `head args…` binds on entry when its
/// dictionary holds `dictionary`: each key the plan declares, with its
/// value at the plan's key path, and each variable operand, with whether
/// the dictionary holds its key. `None` when the call is no dictionary
/// body, or its dictionary does not hold the key path (the command's
/// error).
#[cfg(test)]
pub(crate) fn dict_body(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
    dictionary: &str,
) -> Option<Vec<DictBinder>> {
    let (_, binders, path) = dict_body_plan(registry, head, args, Prior::Known(dictionary))?;
    let rules = WordValueRules::of_profile(registry.profile());
    let level = path.iter().try_fold(dictionary.to_owned(), |level, key| {
        dict_value_at(rules, &level, key)
    })?;
    Some(
        binders
            .into_iter()
            .filter_map(|binder| match binder.name {
                BinderName::Declared(name) => {
                    let value = dict_value_at(rules, &level, &name)?;
                    Some(DictBinder::Key { name, value })
                }
                // `dict update d k v …`: the variable takes the value of the
                // key word before it, which must be literal to be looked up.
                BinderName::Operand(id) => id.0.checked_sub(1).map(|key| DictBinder::Variable {
                    variable: id.0,
                    bound: args.get(key).is_some_and(|word| {
                        matches!(word_of(word), InvocationWord::Literal(_))
                            && dict_value_at(rules, &level, word).is_some()
                    }),
                }),
            })
            .collect(),
    )
}

/// A dictionary body's lexical binders under the exact retained source
/// invocation. These are conditional diagnostic hints, not executed stores.
fn dict_body_plan_with_metadata_context(
    registry: &CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    prior: Prior<'_>,
) -> Option<(
    OperandId,
    Vec<tcl_registry::value_transfer::Binder>,
    Vec<String>,
)> {
    let input = metadata.source_analysis_input()?;
    let binding = tokens.source_binding.as_ref()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    if config.normalized() != input.lexer_config().normalized() {
        return None;
    }
    let invocation =
        crate::registry_invocation::original_callback_invocation_with_metadata_context(
            registry, metadata, tokens,
        )?;
    let realm = binding.invocation_realm().or_else(|| {
        binding
            .declaration_operand_layout_advice(tokens)
            .map(|advice| advice.realm())
    })?;
    let mut context = AnalysisContext::detached(Some(input.unit_profile()));
    context.grammar = config.grammar_over(input.unit_profile().grammar);
    context.script_encoding = input.script_encoding();
    context.registry_generation = registry.generation();
    context.overlay_generation = registry.overlay_generation();
    invocation.with_metadata_schema(registry, metadata, realm, |resolved| {
        let semantics = resolved.semantics.value.semantics()?;
        let values: Vec<_> = (0..invocation.arguments.len())
            .map(|index| invocation.argument_word(index))
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
            .collect();
        let texts: Vec<_> = (0..invocation.arguments.len())
            .map(|index| invocation.argument_literal(index).unwrap_or_default())
            .collect();
        let borrowed: Vec<_> = texts.iter().map(String::as_str).collect();
        let mut inputs = StructureInputs::new(
            view_of(resolved, &borrowed, &words, InvocationLayout::Source),
            &context,
        );
        inputs.prior = prior;
        let PlanAnswer::Body {
            binders,
            reconcile: tcl_registry::value_transfer::Reconcile::WriteBackKeys(dict),
            ..
        } = semantics.structure(&inputs)
        else {
            return None;
        };
        Some((
            dict,
            binders,
            inputs
                .exact_read_texts()
                .into_iter()
                .map(str::to_owned)
                .collect(),
        ))
    })
}

/// Select the effective dictionary operand through original lookup and supplied
/// availability. Captured aliases retain their effective operand coordinates.
pub(crate) fn dict_body_operand_with_metadata_context(
    registry: &CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<(usize, String)> {
    let (dict, ..) =
        dict_body_plan_with_metadata_context(registry, metadata, tokens, Prior::KeyPath)?;
    let invocation =
        crate::registry_invocation::original_callback_invocation_with_metadata_context(
            registry, metadata, tokens,
        )?;
    Some((dict.0, invocation.argument_literal(dict.0)?))
}

/// Conditional dictionary-body binder values, with the original selected
/// grammar and effective alias operands. This does not prove body entry.
pub(crate) fn dict_body_with_metadata_context(
    registry: &CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    dictionary: &str,
) -> Option<Vec<DictBinder>> {
    let (_, binders, path) =
        dict_body_plan_with_metadata_context(registry, metadata, tokens, Prior::Known(dictionary))?;
    let invocation =
        crate::registry_invocation::original_callback_invocation_with_metadata_context(
            registry, metadata, tokens,
        )?;
    let rules = WordValueRules::from_config(
        &tokens
            .source_binding
            .as_ref()?
            .original_lexer_config_for_tokens(tokens)?,
    );
    let level = path.iter().try_fold(dictionary.to_owned(), |level, key| {
        dict_value_at(rules, &level, key)
    })?;
    Some(
        binders
            .into_iter()
            .filter_map(|binder| match binder.name {
                BinderName::Declared(name) => Some(DictBinder::Key {
                    value: dict_value_at(rules, &level, &name)?,
                    name,
                }),
                BinderName::Operand(id) => id.0.checked_sub(1).map(|key| DictBinder::Variable {
                    variable: id.0,
                    bound: invocation
                        .argument_literal(key)
                        .is_some_and(|key| dict_value_at(rules, &level, &key).is_some()),
                }),
            })
            .collect(),
    )
}

/// The value `key` maps to in the dictionary `text`, split as the
/// dialect's `rules` split a list: the last pair's, as `dict get` reads a
/// list with a repeated key.
fn dict_value_at(rules: WordValueRules, text: &str, key: &str) -> Option<String> {
    let elements = rules.split_list(text).ok()?;
    if !elements.len().is_multiple_of(2) {
        return None;
    }
    elements
        .as_chunks::<2>()
        .0
        .iter()
        .rev()
        .find(|[held, _]| held.as_ref() == key)
        .map(|[_, value]| value.to_string())
}
