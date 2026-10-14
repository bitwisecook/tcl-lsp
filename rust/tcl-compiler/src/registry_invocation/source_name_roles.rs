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

//! Original name-role metadata, with execution and source purposes kept separate.

use super::{CommandRegistry, CommandTokens, InvocationFacts, ResolvedStatementInvocation};

/// Possible name operands from a complete actual original source owner.
/// This result grants no Normal completion, entered frame or cell identity.
pub(crate) struct SourceNameRoleInventory {
    facts: Box<InvocationFacts>,
    values: Vec<Option<String>>,
    unknown_residual: bool,
    roles: Vec<(usize, tcl_registry::ArgRole)>,
    roles_complete: bool,
    config: tcl_lexer::LexerConfig,
}
impl SourceNameRoleInventory {
    pub(crate) fn config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
    pub(crate) fn facts(&self) -> &InvocationFacts {
        &self.facts
    }
    pub(crate) fn argument_literal(&self, argument: usize) -> Option<String> {
        self.values.get(argument)?.clone()
    }
    pub(crate) fn roles(&self) -> &[(usize, tcl_registry::ArgRole)] {
        &self.roles
    }
    pub(crate) fn roles_complete(&self) -> bool {
        self.roles_complete
    }
    pub(crate) fn has_unknown_residual(&self) -> bool {
        self.unknown_residual
    }

    fn from_selected(
        invocation: ResolvedStatementInvocation,
        unknown_residual: bool,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        let values = (0..invocation.arguments.len())
            .map(|argument| invocation.argument_literal(argument))
            .collect();
        let roles = invocation
            .facts
            .arg_roles
            .iter()
            .map(|&(ordinal, role)| {
                (
                    invocation.facts.argument_offset + usize::from(ordinal),
                    role,
                )
            })
            .collect();
        let roles_complete = invocation.facts.arg_roles_complete;
        Self {
            facts: invocation.facts,
            values,
            unknown_residual: unknown_residual || !roles_complete,
            roles,
            roles_complete,
            config,
        }
    }
}

pub(crate) fn original_source_name_roles(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<SourceNameRoleInventory> {
    if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    if !binding.has_original_source_layout_owner() {
        if binding.has_original_logical_source_name_role_owner() {
            let input = binding.original_logical_source_input_for_name_roles(tokens, registry)?;
            let words = binding.original_logical_source_words(tokens, input, registry)?;
            return from_source_words(&words, input);
        }
        if binding.has_original_authored_source_body_name_role_owner() {
            let scope =
                binding.original_authored_source_body_scope_for_name_roles(tokens, registry)?;
            return from_authored_body_scope(scope, tokens, registry);
        }
        let input = binding.original_native_source_input_for_name_roles(tokens, registry)?;
        let words = binding.original_native_source_words(tokens, input, registry)?;
        return from_source_words(&words, input);
    }
    let metadata = binding
        .original_source_metadata_for_advice(tokens, registry)
        .ok()?;
    if let Some(invocation) =
        super::resolved_tokens_invocation_with_metadata_context(registry, metadata, tokens)
    {
        return Some(SourceNameRoleInventory::from_selected(
            invocation,
            false,
            binding.original_lexer_config_for_tokens(tokens)?,
        ));
    }
    if let Some(context) = metadata {
        let input = context.source_analysis_input()?;
        if !input.has_logical_source_name_context() && !input.has_hosted_source_name_context() {
            let words = binding.original_native_source_words(tokens, input, registry)?;
            return from_source_words(&words, input);
        }
        let invocation = super::original_logical_operation_invocation_with_metadata_context(
            registry, context, tokens,
        )?;
        let advice = binding.original_conditional_operand_layout_advice(tokens, registry)?;
        return Some(SourceNameRoleInventory::from_selected(
            invocation,
            !advice.closed_logical_source_lookup(),
            binding.original_lexer_config_for_tokens(tokens)?,
        ));
    }
    let source = super::original_authored_source_structure(tokens, registry)?;
    let count = source.facts().frozen_argument_count?;
    Some(SourceNameRoleInventory {
        facts: Box::new(source.facts().clone()),
        values: (0..count)
            .map(|argument| source.argument_literal(argument))
            .collect(),
        unknown_residual: source.has_unknown_residual() || !source.facts().arg_roles_complete,
        roles: source
            .facts()
            .arg_roles
            .iter()
            .map(|&(ordinal, role)| (source.facts().argument_offset + usize::from(ordinal), role))
            .collect(),
        roles_complete: source.facts().arg_roles_complete,
        config: source.config(),
    })
}

pub(crate) fn original_authored_substitution_name_roles(
    receipt: &crate::word_subst::OriginalAuthoredSourceSubstitutionWords,
    registry: &CommandRegistry,
) -> Option<SourceNameRoleInventory> {
    receipt.with_source_schema(registry, |schema| {
        let facts = Box::new(schema.facts());
        let (roles, roles_complete) = schema.authored_source_argument_roles();
        let roles = roles
            .into_iter()
            .map(|(ordinal, role)| (facts.argument_offset + usize::from(ordinal), role))
            .collect();
        let words = receipt.original_words();
        let values = words
            .iter()
            .skip(1)
            .map(|word| {
                String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
                    word,
                )?)
                .ok()
            })
            .collect();
        Some(SourceNameRoleInventory {
            facts,
            values,
            roles,
            roles_complete,
            config: words.first()?.config(),
            unknown_residual: true,
        })
    })
}

fn from_authored_body_scope(
    scope: &super::OriginalAuthoredSourceBodyScope,
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<SourceNameRoleInventory> {
    let original = scope.original_words(tokens, registry)?;
    scope.with_source_schema(tokens, registry, |schema| {
        let facts = Box::new(schema.facts());
        let (roles, roles_complete) = schema.authored_source_argument_roles();
        let roles = roles
            .into_iter()
            .map(|(ordinal, role)| (facts.argument_offset + usize::from(ordinal), role))
            .collect();
        let values = original
            .iter()
            .skip(1)
            .map(|word| {
                String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
                    word,
                )?)
                .ok()
            })
            .collect();
        Some(SourceNameRoleInventory {
            facts,
            values,
            roles,
            roles_complete,
            config: scope.config(),
            unknown_residual: true,
        })
    })
}

fn from_source_words(
    words: &super::source_structure::OriginalRegistryWords,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<SourceNameRoleInventory> {
    let (facts, (roles, roles_complete)) =
        words.with_source_schema(input.borrowed_context_registry(), |schema| {
            let roles = if input.has_logical_source_name_context() {
                schema.authored_logical_source_argument_roles()
            } else {
                schema.authored_source_argument_roles()
            };
            let facts = schema.facts();
            let (roles, complete) = roles;
            let roles = roles
                .into_iter()
                .map(|(argument, role)| (facts.argument_offset + usize::from(argument), role))
                .collect();
            (Box::new(facts), (roles, complete))
        })?;
    let unknown_residual = !roles_complete
        || match words.source() {
            super::source_structure::OriginalRegistrySource::SourceTransitions(advice) => {
                !advice.obligations().is_empty() || !advice.uncertain_operations().is_empty()
            }
            _ => true,
        };
    let values = words
        .arguments()
        .iter()
        .map(|word| {
            std::str::from_utf8(word.literal_bytes()?)
                .ok()
                .map(str::to_owned)
        })
        .collect();
    Some(SourceNameRoleInventory {
        facts,
        values,
        unknown_residual,
        roles,
        roles_complete,
        config: input.lexer_config(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_dynamic_role_and_expansion_metadata_retain_unknown_name_hazards() {
        fn roles(_: &[&str]) -> Vec<(u8, tcl_registry::ArgRole)> {
            vec![(1, tcl_registry::ArgRole::VarRead)]
        }
        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "name_role_owner",
            arity: tcl_registry::Arity::any(),
            arg_roles: &[(0, tcl_registry::ArgRole::VarWrite)],
            arg_role_resolver: Some(roles),
            arg_role_resolver_roles: &[tcl_registry::ArgRole::VarRead],
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let arguments = [
            tcl_registry::InvocationWord::Literal("$literal"),
            tcl_registry::InvocationWord::Dynamic,
        ];
        let selected = registry.resolve_structured_invocation(
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal("name_role_owner"),
                &arguments,
            ),
            None,
        );
        assert!(!selected.resolved().unwrap().facts().arg_roles_complete);
        for source in [
            "name_role_owner {$literal} $selector",
            "info exists {*}$names",
        ] {
            let module = crate::lowering::lower_to_ir(source, &registry);
            let script = &module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(&script.statements[0])
                .unwrap();
            if let Some(inventory) = original_source_name_roles(&registry, tokens) {
                assert!(inventory.has_unknown_residual());
            }
        }
    }

    #[test]
    fn source_name_roles_keep_actual_authored_values_and_terminal_owner_refusals() {
        let registry = CommandRegistry::build_default();
        for expected in ["", "$literal", "[literal]"] {
            let source = format!("info exists {{{expected}}}");
            let module = crate::lowering::lower_to_ir(&source, &registry);
            assert!(module.source_entry.native_entry.is_none());
            let script = &module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(&script.statements[0])
                .unwrap();
            let roles =
                original_source_name_roles(&registry, tokens).expect("actual source name owner");
            assert!(roles.roles().contains(&(1, tcl_registry::ArgRole::VarRead)));
            assert_eq!(roles.argument_literal(1).as_deref(), Some(expected));
            let wrong = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
            assert!(original_source_name_roles(wrong, tokens).is_none());
            let mut changed = tokens.clone();
            changed.argv_texts[2] = "changed".to_owned();
            assert!(original_source_name_roles(&registry, &changed).is_none());
        }
    }
}
