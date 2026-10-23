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
            && invocation.effective.origins.len() == invocation.arguments.len().saturating_add(1)
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
            dialect.and_then(tcl_registry::InvocationDialect::authoring_query),
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

/// Literal receiver names under the actual selected argument role. A list
/// role parses the decoded word value; a scalar role preserves that whole value.
pub(crate) fn original_name_role_values(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    argument: usize,
    role: tcl_registry::ArgRole,
) -> Option<Vec<String>> {
    let source = original_source_name_roles(registry, tokens)?;
    (source.roles_complete()
        && source.direct_written_arguments()
        && source.roles().contains(&(argument, role)))
    .then_some(())?;
    let value = source.argument_literal(argument)?;
    match role {
        tcl_registry::ArgRole::VarWrite => Some(vec![value]),
        tcl_registry::ArgRole::LoopVarList => Some(
            tcl_syntax::word_rules::WordValueRules::from_config(&source.config())
                .split_list(&value)
                .ok()?
                .into_iter()
                .map(std::borrow::Cow::into_owned)
                .collect(),
        ),
        _ => None,
    }
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

/// Conditional source metadata under the actual supplied input. Existing
/// declaration/body/compiler purposes keep terminal ownership of their route.
/// Only an independently authenticated unresolved point can select the separate
/// written-vocabulary view, retaining an unknown residual in every case.
/// No runtime target, namespace, value, frame or executable fact follows.
pub(crate) fn original_source_metadata_name_roles(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<SourceNameRoleInventory> {
    if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    binding.original_invocation_metadata_for_input(tokens, input, registry)?;
    if input.has_logical_source_name_context()
        || input.has_hosted_source_name_context()
        || binding.has_nonpoint_original_source_name_role_owner()
        || !binding.execution_is_unknown()
        || binding.execution_targets().next().is_some()
        || binding.proved_handler_target().is_some()
    {
        return original_source_name_roles(registry, tokens);
    }
    unresolved_point_source_name_roles(registry, tokens, input)
}

fn unresolved_point_source_name_roles(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<SourceNameRoleInventory> {
    let binding = tokens.source_binding.as_ref()?;
    binding.original_invocation_metadata_for_input(tokens, input, registry)?;
    let site = binding.invocation_site()?;
    let words = super::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        binding.original_lexer_config_for_tokens(tokens)?,
    )?;
    source_name_roles_from_point_words(registry, tokens, input, &words)
}

fn source_name_roles_from_point_words(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    input: &crate::analyser::ResolvedAnalysisInput,
    words: &[tcl_lexer::NativeWord],
) -> Option<SourceNameRoleInventory> {
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_input(tokens, input, registry)?;
    let retained_input = metadata.source_analysis_input()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    if retained_input.lexer_config() != config {
        return None;
    }
    let realm = binding.original_source_realm_for_input(tokens, input, registry)?;
    let site = binding.invocation_site()?;
    let (command, recorded) = binding.original_recorded_command()?;
    if recorded.words() != tokens.words() || command.argv.first()?.span.start() != site.offset {
        return None;
    }
    let original = super::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        config,
    )?;
    if original.as_slice() != words {
        return None;
    }
    let values = words
        .iter()
        .map(|word| {
            String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
                word,
            )?)
            .ok()
        })
        .collect::<Vec<_>>();
    if values.iter().flatten().any(|value| value.contains('\0')) {
        return None;
    }
    let head = values.first()?.as_deref()?;
    let arguments = words
        .iter()
        .zip(&values)
        .skip(1)
        .map(|(word, value)| {
            if word.group().expand {
                tcl_registry::InvocationWord::Expanded
            } else {
                value.as_deref().map_or(
                    tcl_registry::InvocationWord::Dynamic,
                    tcl_registry::InvocationWord::Literal,
                )
            }
        })
        .collect::<Vec<_>>();
    let mut dialect = tcl_registry::InvocationDialect::of_profile(retained_input.unit_profile());
    dialect.lexer_grammar = config.grammar_over(dialect.lexer_grammar);
    dialect.word_values = tcl_syntax::word_rules::WordValueRules::from_config(&config);
    let context = retained_input.borrowed_context_registry();
    let schema = tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
        registry,
        Some(context.context()),
        tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(head),
            &arguments,
        )
        .with_dialect(dialect),
        realm,
    )
    .resolved()?;
    let facts = Box::new(schema.facts());
    let (roles, roles_complete) = schema.authored_source_argument_roles();
    let roles = roles
        .into_iter()
        .map(|(argument, role)| (facts.argument_offset + usize::from(argument), role))
        .collect();
    Some(SourceNameRoleInventory {
        facts,
        values: values.into_iter().skip(1).collect(),
        unknown_residual: true,
        roles,
        roles_complete,
        direct_written_arguments: !words.iter().any(|word| word.group().expand),
        config,
    })
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
                .original_authored_scalar_body_scope(tokens, registry)?
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
        if let Some(context) = metadata {
            let input = context.source_analysis_input()?;
            let realm = binding.original_source_realm_for_input(tokens, input, registry)?;
            invocation.with_metadata_schema(registry, context, realm, |schema| {
                (schema.facts() == *invocation.facts).then_some(())
            })?;
        }
        return Some(SourceNameRoleInventory::from_selected(
            invocation,
            false,
            binding.original_lexer_config_for_tokens(tokens)?,
        ));
    }
    if let Some(context) = metadata {
        let input = context.source_analysis_input()?;
        if !input.has_logical_source_name_context() && !input.has_hosted_source_name_context() {
            // A present child owner keeps its own terminal source obligations.
            // Root compiler syntax has a distinct source-only metadata purpose.
            if binding.has_original_native_source_command() {
                let words = binding.original_native_source_words(tokens, input, registry)?;
                return from_source_words(&words, input);
            }
            let words = binding
                .original_compiler_point_words_for_source_metadata(tokens, input, registry)?;
            return source_name_roles_from_point_words(registry, tokens, input, &words);
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
            == words.arguments().len().saturating_add(1)
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
    fn original_loop_module(
        source: &str,
        dialect: &str,
    ) -> crate::provider_fixtures::OriginalNativeModuleFixture {
        crate::provider_fixtures::original_native_module(source, dialect)
    }

    #[test]
    fn loop_names_follow_decoded_word_then_the_selected_list_role() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            for (word, expected) in [
                ("{$literal}", vec!["$literal"]),
                ("{[literal]}", vec!["[literal]"]),
                ("{{}}", vec![""]),
                ("{{a b}}", vec!["a b"]),
                (r"a\ b", vec!["a", "b"]),
                (r"a\x62", vec!["ab"]),
                ("{a($key)}", vec!["a($key)"]),
            ] {
                let source = format!("foreach {word} {{X Y}} {{set result 1}}");
                let fixture = original_loop_module(&source, dialect);
                let (context, module) = &*fixture;
                let statement = &module.top_level.statements[0];
                let crate::ir::Statement::Foreach { iterators, .. } = statement else {
                    panic!("{dialect}: {source}: actual static list receiver");
                };
                assert_eq!(iterators[0].vars, expected, "{dialect}: {source}");
                let tokens = module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
                    .unwrap();
                let names = original_name_role_values(
                    context.commands(),
                    tokens,
                    0,
                    tcl_registry::ArgRole::LoopVarList,
                )
                .unwrap();
                assert_eq!(names, expected, "{dialect}: {source}");
                assert!(
                    original_name_role_values(
                        context.commands(),
                        tokens,
                        0,
                        tcl_registry::ArgRole::VarWrite
                    )
                    .is_none()
                );
                let mut detached = tokens.clone();
                detached.source_binding = None;
                assert!(
                    original_name_role_values(
                        context.commands(),
                        &detached,
                        0,
                        tcl_registry::ArgRole::LoopVarList
                    )
                    .is_none()
                );
                let mut changed = tokens.clone();
                changed.argv_texts[1].push('!');
                assert!(
                    original_name_role_values(
                        context.commands(),
                        &changed,
                        0,
                        tcl_registry::ArgRole::LoopVarList
                    )
                    .is_none()
                );
            }
            for word in ["$names", "[names]", "a($key)"] {
                let source = format!("foreach {word} {{X}} {{set result 1}}");
                let fixture = original_loop_module(&source, dialect);
                let module = &fixture.1;
                assert!(
                    !matches!(
                        module.top_level.statements[0],
                        crate::ir::Statement::Foreach { .. }
                    ),
                    "{dialect}: {source}"
                );
            }
        }
    }

    #[test]
    fn file_line_receiver_role_keeps_one_complete_name() {
        for dialect in ["tcl9.0", "tcl9.1"] {
            for expected in ["a b", "$literal", "[literal]", "", "a($key)"] {
                let source = format!("foreachLine {{{expected}}} input.txt {{set result 1}}");
                let fixture = original_loop_module(&source, dialect);
                let (context, module) = &*fixture;
                let statement = &module.top_level.statements[0];
                let crate::ir::Statement::Foreach { iterators, .. } = statement else {
                    panic!("{dialect}: {source}: actual scalar receiver");
                };
                assert_eq!(iterators[0].vars, vec![expected]);
                let tokens = module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
                    .unwrap();
                assert_eq!(
                    original_name_role_values(
                        context.commands(),
                        tokens,
                        0,
                        tcl_registry::ArgRole::VarWrite
                    )
                    .unwrap(),
                    vec![expected]
                );
                assert!(
                    original_name_role_values(
                        context.commands(),
                        tokens,
                        0,
                        tcl_registry::ArgRole::LoopVarList
                    )
                    .is_none()
                );
            }
            let fixture =
                original_loop_module("foreachLine $name input.txt {set result 1}", dialect);
            let module = &fixture.1;
            assert!(!matches!(
                module.top_level.statements[0],
                crate::ir::Statement::Foreach { .. }
            ));
        }
    }

    #[test]
    fn file_line_original_source_role_does_not_require_typed_execution() {
        for dialect in ["tcl9.0", "tcl9.1"] {
            for expected in ["a b", "$literal", "[literal]", "", "a($key)"] {
                let source = format!(
                    "proc read_lines {{}} {{foreachLine {{{expected}}} input.txt {{set result 1}}}}"
                );
                let environment = tcl_registry::model::ingress::resolve_known_environment(dialect)
                    .expect("actual source-role environment");
                let context = environment.default_context_registry();
                let profile = environment.unit_profile();
                let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
                let input = crate::analyser::ResolvedAnalysisInput::new(
                    profile,
                    profile,
                    std::sync::Arc::clone(&context),
                    config,
                );
                // Unentered native declaration bodies need explicit unknown history.
                let module = crate::lowering::lower_to_ir_with(
                    crate::lowering::Lowerer::with_config(context.commands(), config)
                        .with_resolved_analysis_input(input)
                        .with_explicit_unknown_entry(),
                    &source,
                );
                assert!(module.source_entry.native_entry.is_none());
                assert_eq!(module.original_native_declaration_body_units.len(), 1);
                let (id, card) = module
                    .original_native_declaration_body_units
                    .iter()
                    .next()
                    .unwrap();
                let procedure = module.body_units.get(id).unwrap();
                assert!(card.matches_procedure(&module, procedure));
                assert!(procedure.body.executed_source.is_none());
                assert!(procedure.body.native_compilation_admission.is_none());
                let statement = &procedure.body.statements[0];
                let tokens = procedure
                    .body
                    .retained_source_tokens_for_statement(statement)
                    .expect("actual original file-line source words");
                assert!(
                    tokens
                        .source_binding
                        .as_ref()
                        .unwrap()
                        .original_native_source_owner_for_body(
                            tokens,
                            &module,
                            *id,
                            context.commands()
                        )
                );
                let inventory = original_source_name_roles(context.commands(), tokens)
                    .expect("readonly original source name-role inventory");
                assert!(inventory.roles_complete());
                assert!(inventory.direct_written_arguments());
                assert!(
                    inventory
                        .roles()
                        .contains(&(0, tcl_registry::ArgRole::VarWrite))
                );
                assert_eq!(
                    original_name_role_values(
                        context.commands(),
                        tokens,
                        0,
                        tcl_registry::ArgRole::VarWrite
                    )
                    .unwrap(),
                    vec![expected]
                );
                assert!(
                    original_name_role_values(
                        context.commands(),
                        tokens,
                        0,
                        tcl_registry::ArgRole::LoopVarList
                    )
                    .is_none()
                );
                let mut detached = tokens.clone();
                detached.source_binding = None;
                assert!(
                    original_name_role_values(
                        context.commands(),
                        &detached,
                        0,
                        tcl_registry::ArgRole::VarWrite
                    )
                    .is_none()
                );
                let mut changed = tokens.clone();
                changed.argv_texts[1].push('!');
                assert!(
                    original_name_role_values(
                        context.commands(),
                        &changed,
                        0,
                        tcl_registry::ArgRole::VarWrite
                    )
                    .is_none()
                );
            }
        }
    }

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
    fn scalar_source_name_roles_keep_original_declaration_scope_and_currency() {
        let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
        let context = environment.default_context_registry();
        let registry = context.commands();
        let source = "proc p {used spare} {set local 1; return $used}";
        let config = tcl_lexer::LexerConfig::for_file_grammar(environment.unit_profile().grammar);
        let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
            source, registry, false, config,
        );
        let module = &unit.ir_module;
        let (&id, _) = module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let procedure = unit.original_procedure_for_body(id).unwrap();
        assert_eq!(procedure.body.statements.len(), 2);
        for statement in &procedure.body.statements {
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(binding.has_original_source_layout_owner());
            assert!(
                binding
                    .original_authored_source_body_scope_for_name_roles(tokens, registry)
                    .is_none()
            );
            let scope = binding
                .original_authored_scalar_body_scope(tokens, registry)
                .unwrap();
            assert!(super::super::original_authored_scalar_invocation(tokens, registry).is_some());
            let selected = original_source_name_roles(registry, tokens)
                .expect("authenticated scalar source scope");
            assert_eq!(selected.config(), scope.config());
            assert_eq!(selected.config(), config.nested());
            assert!(selected.roles_complete());
            assert!(selected.has_unknown_residual());
            assert!(selected.direct_written_arguments());
            assert_eq!(
                selected.roles(),
                super::super::original_authored_scalar_invocation(tokens, registry)
                    .unwrap()
                    .written_roles()
            );
            assert!(
                super::super::normal_representation_invocation_with_metadata_context(
                    registry, None, tokens
                )
                .is_none()
            );
            assert!(
                super::super::admitted_native_compiler_invocation(registry, None, tokens).is_none()
            );
            let mut changed = tokens.clone();
            changed.argv_texts[0].push('_');
            assert!(original_source_name_roles(registry, &changed).is_none());
            let mut withdrawn = tokens.clone();
            withdrawn.source_binding = None;
            assert!(original_source_name_roles(registry, &withdrawn).is_none());
            let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0");
            assert!(original_source_name_roles(foreign.commands(), tokens).is_none());
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

#[cfg(test)]
mod unresolved_point_metadata_tests {
    use super::*;
    use std::sync::Arc;

    fn module(source: &str) -> (crate::ir::Module, crate::analyser::ResolvedAnalysisInput) {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
        let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        let context = input.context_registry();
        let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
            .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        (lowerer.finish_module(source), input)
    }

    fn tokens(module: &crate::ir::Module) -> &CommandTokens {
        module
            .top_level
            .statements
            .iter()
            .find_map(|statement| {
                module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
            })
            .unwrap()
    }

    #[test]
    fn unresolved_point_source_metadata_keeps_unknown_execution_and_written_values() {
        for (source, expected) in [
            ("puts VALUE", Some("VALUE")),
            ("puts {$literal}", Some("$literal")),
            ("puts $computed", None),
        ] {
            let (module, input) = module(source);
            let registry = input.borrowed_context_registry().commands();
            let tokens = tokens(&module);
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(module.source_entry.native_entry.is_none());
            assert!(binding.execution_is_unknown(), "{source}");
            assert!(binding.execution_targets().next().is_none(), "{source}");
            assert!(binding.proved_handler_target().is_none(), "{source}");
            assert!(
                !binding.has_nonpoint_original_source_name_role_owner(),
                "{source}"
            );
            let inventory = original_source_metadata_name_roles(registry, tokens, &input)
                .expect("actual supplied unresolved point retains conditional vocabulary");
            assert_eq!(inventory.facts().descriptor_key().label(), "puts");
            assert_eq!(inventory.argument_literal(0).as_deref(), expected);
            assert!(inventory.has_unknown_residual());
            assert!(inventory.direct_written_arguments());
            assert_eq!(inventory.config(), input.lexer_config());
            assert!(binding.execution_is_unknown());
            assert!(binding.execution_targets().next().is_none());
        }
    }

    #[test]
    fn unresolved_point_source_metadata_refuses_changed_owners_and_stronger_carrier_fallbacks() {
        let (module, input) = module("puts VALUE");
        let registry = input.borrowed_context_registry().commands();
        let original = tokens(&module);
        assert!(original_source_metadata_name_roles(registry, original, &input).is_some());
        let mut changed = original.clone();
        changed.argv_texts[1].push('!');
        assert!(original_source_metadata_name_roles(registry, &changed, &input).is_none());
        let mut detached = original.clone();
        detached.source_binding = None;
        assert!(original_source_metadata_name_roles(registry, &detached, &input).is_none());
        let mut missing = original.clone();
        missing.source_binding.as_mut().unwrap().lookup_state = None;
        assert!(original_source_metadata_name_roles(registry, &missing, &input).is_none());
        let mut stronger = original.clone();
        let binding = stronger.source_binding.as_mut().unwrap();
        binding.compiler_lookup_state = binding.lookup_state.clone();
        assert!(binding.has_nonpoint_original_source_name_role_owner());
        assert!(original_source_metadata_name_roles(registry, &stronger, &input).is_none());
        let mut config = input.lexer_config();
        config.strict_quoting ^= true;
        let changed_input = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        assert_ne!(changed_input, input);
        assert!(original_source_metadata_name_roles(registry, original, &changed_input).is_none());
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(input.context_registry().commands())),
        );
        let changed_input = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        );
        assert_ne!(changed_input, input);
        assert!(original_source_metadata_name_roles(registry, original, &changed_input).is_none());
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
        assert!(
            original_source_metadata_name_roles(foreign.commands(), original, &input).is_none()
        );
    }
    #[test]
    fn selected_source_name_roles_recheck_actual_effective_argv_availability() {
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_registry::model::resolve_environment("tcl8.6").unit_profile());
        let mut puts = registry.get("puts").unwrap().clone();
        puts.surface = registry.get("dict").unwrap().surface;
        registry.insert(puts);
        // Insertion retains shipped alternatives. Project the authored override
        // after insertion so this fixture has only the intended 8.5+ puts row.
        let registry = registry
            .project_for_profile(tcl_registry::model::resolve_environment("tcl8.6").unit_profile());
        let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.commands())),
        );
        assert!(Arc::ptr_eq(current.commands(), older.commands()));
        assert!(
            current
                .context()
                .resolve_spec(current.commands(), "puts")
                .is_some()
        );
        assert!(
            older
                .context()
                .resolve_spec(older.commands(), "puts")
                .is_none()
        );
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let mut config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        config.braced_var = tcl_dialect::BracedVarStyle::FirstClose;
        for (context, available) in [(current, true), (older, false)] {
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::clone(&context),
                config,
            );
            assert!(input.has_logical_source_name_context());
            let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
                "puts VALUE",
                crate::compilation_unit::UnitBuildOptions {
                    registry: context.commands(),
                    defer_top_level: false,
                    config,
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                None,
                &input,
            );
            assert!(unit.ir_module.source_entry.native_entry.is_none());
            let original = tokens(&unit.ir_module);
            let inventory =
                original_source_metadata_name_roles(context.commands(), original, &input);
            assert_eq!(inventory.is_some(), available);
            if let Some(inventory) = inventory {
                assert_eq!(inventory.facts().descriptor_key().label(), "puts");
                assert_eq!(inventory.argument_literal(0).as_deref(), Some("VALUE"));
            }
            let mut changed = original.clone();
            changed.argv_texts[1].push('!');
            assert!(
                original_source_metadata_name_roles(context.commands(), &changed, &input).is_none()
            );
        }
    }
}
