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

pub(crate) struct OriginalVariableNameOperand(String);

impl OriginalVariableNameOperand {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl ResolvedStatementInvocation {
    pub(crate) fn original_variable_name_operand(
        &self,
        argument: usize,
    ) -> Option<OriginalVariableNameOperand> {
        self.facts
            .arg_roles
            .iter()
            .any(|&(ordinal, role)| {
                self.facts.argument_offset + usize::from(ordinal) == argument
                    && matches!(
                        role,
                        tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                    )
            })
            .then_some(())?;
        self.argument_literal(argument)
            .map(OriginalVariableNameOperand)
    }
}

/// Possible name operands from a complete actual original source owner.
/// This result grants no Normal completion, entered frame or cell identity.
pub(crate) struct SourceNameRoleInventory {
    facts: Box<InvocationFacts>,
    values: Vec<Option<String>>,
    unknown_residual: bool,
    roles: Vec<(usize, tcl_registry::ArgRole)>,
    roles_complete: bool,
    direct_written_arguments: bool,
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
    pub(crate) fn original_variable_name_operand(
        &self,
        argument: usize,
    ) -> Option<OriginalVariableNameOperand> {
        self.roles
            .iter()
            .any(|&(ordinal, role)| {
                ordinal == argument
                    && matches!(
                        role,
                        tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                    )
            })
            .then_some(())?;
        self.argument_literal(argument)
            .map(OriginalVariableNameOperand)
    }
    pub(crate) fn roles(&self) -> &[(usize, tcl_registry::ArgRole)] {
        &self.roles
    }
    pub(crate) fn roles_complete(&self) -> bool {
        self.roles_complete
    }
    pub(crate) fn direct_written_arguments(&self) -> bool {
        self.direct_written_arguments
    }
    pub(crate) fn has_unknown_residual(&self) -> bool {
        self.unknown_residual
    }

    fn from_selected(
        invocation: ResolvedStatementInvocation,
        unknown_residual: bool,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        let direct_written_arguments = invocation.effective.origins.len()
            == invocation.effective.words.len()
            && invocation.effective.origins.len()
                == invocation
                    .arguments
                    .len()
                    .checked_add(1)
                    .unwrap_or(usize::MAX)
            && invocation
                .effective
                .origins
                .iter()
                .enumerate()
                .skip(1)
                .all(|(index, origin)| *origin == super::InvocationWordOrigin::Written(index));
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
            direct_written_arguments,
            config,
        }
    }
}

/// Explicit Standalone syntax inventory under the caller's actual grammar.
/// These literal values supply possible names, never point or handler ownership.
pub(crate) fn standalone_written_source_name_roles(
    registry: &CommandRegistry,
    words: &[tcl_lexer::NativeWord],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Option<SourceNameRoleInventory> {
    let first = words.first()?;
    let config = first.config();
    if words
        .iter()
        .any(|word| word.image() != first.image() || word.config() != config || word.group().expand)
    {
        return None;
    }
    let plan = tcl_lexer::native_script_words_in_with_purpose(
        first.image().clone(),
        tcl_lexer::Span::new(first.word_span().start(), words.last()?.word_span().end()),
        config,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    if plan.fatal_tail.is_some() || command.words != words {
        return None;
    }
    let values: Vec<Option<String>> = words
        .iter()
        .map(|word| {
            String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
                word,
            )?)
            .ok()
        })
        .collect();
    let head = values.first()?.as_deref()?;
    if head.contains('\0') {
        return None;
    }
    let arguments: Vec<_> = values
        .iter()
        .skip(1)
        .map(|value| {
            value.as_deref().map_or(
                tcl_registry::InvocationWord::Dynamic,
                tcl_registry::InvocationWord::Literal,
            )
        })
        .collect();
    let invocation = registry
        .resolve_structured_invocation(
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal(head),
                &arguments,
            ),
            dialect.and_then(|dialect| dialect.authoring_query()),
        )
        .resolved()?;
    let facts = Box::new(invocation.facts());
    let (roles, roles_complete) = invocation.authored_source_argument_roles();
    let roles = roles
        .into_iter()
        .map(|(ordinal, role)| (facts.argument_offset + usize::from(ordinal), role))
        .collect();
    Some(SourceNameRoleInventory {
        facts,
        values: values.into_iter().skip(1).collect(),
        roles,
        roles_complete,
        direct_written_arguments: true,
        config,
        unknown_residual: !roles_complete,
    })
}

pub(crate) fn original_assignment_name_operand(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    statement_name: &str,
) -> Option<(OriginalVariableNameOperand, bool)> {
    let source = original_source_name_roles(registry, tokens)?;
    source
        .facts
        .traits
        .contains(tcl_registry::Traits::FIRST_ARG_VARNAME)
        .then_some(())?;
    let argument = source.facts.argument_offset;
    source
        .roles
        .contains(&(argument, tcl_registry::ArgRole::VarWrite))
        .then_some(())?;
    let name = source.original_variable_name_operand(argument)?;
    (name.as_str() == statement_name).then_some((name, source.has_unknown_residual()))
}

pub(crate) fn original_source_name_roles(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<SourceNameRoleInventory> {
    if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    if binding.has_original_authored_source_body_name_role_owner()
        && let Some(scalar) = super::original_authored_scalar_invocation(tokens, registry)
    {
        let roles = scalar.written_roles();
        let facts = scalar.with_source_schema(registry, |schema| Some(Box::new(schema.facts())))?;
        let values = (0..tokens.words().len().checked_sub(1)?)
            .map(|argument| scalar.argument_literal(argument))
            .collect();
        return Some(SourceNameRoleInventory {
            facts,
            values,
            unknown_residual: true,
            roles,
            roles_complete: true,
            direct_written_arguments: true,
            config: binding
                .original_authored_source_body_scope_for_name_roles(tokens, registry)?
                .config(),
        });
    }
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
        super::resolved_tokens_invocation_with_metadata_context(registry, metadata, tokens).or_else(
            || super::resolved_handler_invocation_with_metadata_context(registry, metadata, tokens),
        )
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
        direct_written_arguments: source.direct_written_arguments(),
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
            direct_written_arguments: true,
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
            direct_written_arguments: true,
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
        direct_written_arguments: words.origins().len()
            == words.arguments().len().checked_add(1).unwrap_or(usize::MAX)
            && words
                .origins()
                .iter()
                .enumerate()
                .skip(1)
                .all(|(index, origin)| *origin == super::InvocationWordOrigin::Written(index)),
        config: input.lexer_config(),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_scalar_name_projection_preserves_data_and_refuses_relabelled_fields() {
        let registry = tcl_registry::CommandRegistry::build_default();
        for (source, expected) in [
            ("set {} 1", ""),
            ("incr {}", ""),
            ("set {$literal} 1", "$literal"),
            ("incr {[literal]}", "[literal]"),
        ] {
            let module = crate::lowering::lower_to_ir(source, &registry);
            let tokens = module
                .top_level
                .retained_source_tokens_for_statement(&module.top_level.statements[0])
                .unwrap();
            let (name, residual) =
                super::original_assignment_name_operand(&registry, tokens, expected)
                    .expect("actual selected original scalar name role");
            assert_eq!(name.as_str(), expected);
            let source_roles = super::original_source_name_roles(&registry, tokens).unwrap();
            assert_eq!(residual, source_roles.has_unknown_residual());
            assert!(
                super::original_assignment_name_operand(&registry, tokens, "relabelled").is_none()
            );
            let mut changed = tokens.clone();
            changed.argv_texts[1].push_str(" changed");
            assert!(
                super::original_assignment_name_operand(&registry, &changed, expected).is_none()
            );
            let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
            assert!(super::original_assignment_name_operand(foreign, tokens, expected).is_none());
        }
    }

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

    #[test]
    fn standalone_written_roles_require_one_complete_original_command_vector() {
        let registry = CommandRegistry::build_default();
        let source = "info exists {$literal}; info exists {$other}";
        let image = tcl_lexer::SourceImage::document(source);
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .unwrap();
        let first = &plan.commands[0].words;
        let selected = standalone_written_source_name_roles(&registry, first, None)
            .expect("exact Standalone original word vector");
        assert_eq!(
            selected.original_variable_name_operand(1).unwrap().as_str(),
            "$literal"
        );
        let mut donated = first.clone();
        donated[2] = plan.commands[1].words[2].clone();
        assert!(standalone_written_source_name_roles(&registry, &donated, None).is_none());
    }
}
