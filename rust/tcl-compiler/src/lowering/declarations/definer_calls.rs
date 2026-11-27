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

//! Classifying a call that defines a class or its members, and whether its words supply a body.

use super::super::{Lowerer, MAX_LOWER_NEST_DEPTH, word_is_static_braced};
use super::DefinerCall;
use crate::ir::{CommandTokens, Script, Statement};
use tcl_registry::ArgRole;
use tcl_registry::hooks::LoweringHookId;
use tcl_source::segment::SegmentedCommand;

impl Lowerer<'_> {
    /// Read an exact definition layout from the selected original invocation.
    /// These source units do not establish class presence or entered execution.
    pub(in super::super) fn classify_definer_call(
        &self,
        tokens: &CommandTokens,
    ) -> Option<DefinerCall> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context = self.invocation_metadata_context()?;
        if let Some(binding) = tokens.source_binding.as_ref()
            && binding.has_original_logical_source_name_role_owner()
            && !binding.has_original_source_layout_owner()
        {
            let selected = crate::registry_invocation::original_logical_source_definition_layout(
                self.registry,
                context.source_analysis_input()?,
                tokens,
            )?;
            return Some(DefinerCall {
                definition_scope: None,
                descriptor: selected.descriptor,
                grammar: selected.grammar,
                name_idx: selected.name_word,
                body_idx: selected.body_word,
                per_object: selected.per_object,
            });
        }
        let invocation = self.logical_structured_invocation(tokens)?;
        let mut bodies = invocation
            .written_roles()
            .into_iter()
            .filter_map(|(index, role)| (role == ArgRole::Body).then_some(index + 1));
        let body_idx = bodies.next()?;
        if bodies.next().is_some()
            || !word_is_static_braced(&tokens.argv_kinds, &tokens.single_token_word, body_idx)
        {
            return None;
        }
        let realm = tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
            .unwrap_or(self.invocation_realm);
        invocation.with_metadata_schema(self.registry, context, realm, |selected| {
            let grammar = selected.authored_source_definition_body_grammar()?;
            if !grammar.profile.manufactures_runtime_commands()
                || !selected
                    .authored_source_script_arguments()?
                    .contains(&(body_idx - 1))
            {
                return None;
            }
            let name_arg = selected
                .state_transitions()
                .facts()
                .iter()
                .find_map(|fact| match &fact.transition {
                    tcl_registry::StateTransition::ObjectDispatch(
                        tcl_registry::ObjectDispatchTransition::Configure { target, .. },
                    ) => target.argument_index(),
                    _ => None,
                })
                .or_else(|| {
                    selected
                        .authored_source_command_publication()
                        .map(|name| name.argument)
                })?;
            let name_idx = invocation.written_argument(name_arg)?.checked_add(1)?;
            (name_idx < tokens.argv.len()).then_some(DefinerCall {
                definition_scope: None,
                descriptor: selected.descriptor_key(),
                grammar,
                name_idx,
                body_idx,
                per_object: selected.authored_source_descriptors().command.analyser_hook
                    == Some(tcl_registry::hooks::AnalyserHookId::OoObjdefine),
            })
        })
    }

    /// Retain possible definition roots from original candidate identities.
    /// Unknown availability or lookup withdraws completeness, never supplies
    /// a nominal grammar or a class declaration from the written command name.
    pub(super) fn definer_invocation_may_supply_body(&self, tokens: &CommandTokens) -> bool {
        let Some(context) = self
            .invocation_metadata_context()
            .filter(|context| context.matches_registry(self.registry))
        else {
            return true;
        };
        if tokens
            .source_binding
            .as_ref()
            .is_some_and(invokes_only_retained_source_commands)
        {
            return false;
        }
        let Some(advice) = crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            self.registry, Some(context), tokens,
        ) else { return true; };
        if advice.unknown_residual || advice.candidates.is_empty() {
            return true;
        }
        let realm = tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
            .unwrap_or(self.invocation_realm);
        advice.candidates.iter().any(|candidate| {
            let Some(spec) =
                context
                    .context()
                    .resolve_spec_in_realm(self.registry, &candidate.command, realm)
            else {
                return true;
            };
            spec.definition_body.is_some_and(|grammar| {
                grammar.profile.manufactures_runtime_commands()
                    && (!candidate.roles_complete
                        || candidate
                            .roles
                            .iter()
                            .any(|&(_, role)| role == ArgRole::Body))
            })
        })
    }

    /// A selected namespace wrapper with unchanged original source operands.
    pub(super) fn is_namespace_eval_shape(&self, tokens: &CommandTokens) -> bool {
        tokens.argv.len() >= 4
            && word_is_static_braced(&tokens.argv_kinds, &tokens.single_token_word, 3)
            && self
                .logical_structured_invocation(tokens)
                .is_some_and(|invocation| {
                    invocation.lowering_hook() == Some(LoweringHookId::NamespaceEval)
                        && invocation.written_roles().contains(&(2, ArgRole::Body))
                })
    }

    pub(super) fn source_definition_member(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        keyword: &str,
    ) -> Option<&'static tcl_registry::definer::MemberSpec> {
        let context = self
            .invocation_metadata_context()
            .filter(|context| context.matches_registry(self.registry))?;
        grammar.source_member_in(
            keyword.as_bytes(),
            Some(context.context().authoring_query()),
        )
    }

    /// Shared vocabulary and layout over original semantic source words.
    /// Dynamic control words cannot become literal optional member selectors.
    pub(super) fn source_member_body_indices(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        keyword: &str,
        segment: &SegmentedCommand,
        base: usize,
    ) -> Option<Vec<usize>> {
        let context = self
            .invocation_metadata_context()
            .filter(|context| context.matches_registry(self.registry))?;
        let dialect = self.invocation_dialect?;
        let tokens = self.cmd_tokens(segment);
        let source = tokens.words().get(base..)?;
        let values: Vec<_> = source
            .iter()
            .map(|word| {
                crate::registry_invocation::effective_invocation_word(
                    word,
                    self.config.escapes,
                    dialect.word_values,
                )
            })
            .collect();
        let words: Vec<_> = source
            .iter()
            .zip(&values)
            .map(|(source, value)| {
                crate::registry_invocation::invocation_word_with_source(
                    source,
                    value,
                    self.config.escapes,
                )
            })
            .collect();
        grammar.source_member_body_indices_in(
            keyword.as_bytes(),
            tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect),
            Some(context.context().authoring_query()),
        )
    }

    pub(super) fn member_invocation_supplies_body(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        segment: &SegmentedCommand,
    ) -> bool {
        self.source_member_body_indices(grammar, &segment.texts[0], segment, 1)
            .is_none_or(|indices| !indices.is_empty())
    }

    /// A definition created only when this method runs is not part of the
    /// post-pass's source-root walk. Detect it recursively and retain explicit
    /// module evidence rather than pretending its callable bodies do not exist.
    pub(super) fn method_body_may_install_definition(&self, script: &Script) -> bool {
        fn walk(lowerer: &Lowerer<'_>, script: &Script, depth: u32) -> bool {
            if MAX_LOWER_NEST_DEPTH.exceeded(depth) {
                return true;
            }
            script.statements.iter().any(|stmt| {
                let installs = matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. })
                    && stmt
                        .tokens()
                        .is_none_or(|tokens| lowerer.definer_invocation_may_supply_body(tokens));
                installs
                    || crate::ir_helpers::nested_bodies(stmt)
                        .into_iter()
                        .any(|body| walk(lowerer, body, depth + 1))
            })
        }

        walk(self, script, 0)
    }
}

/// A proved call of a document procedure or class/object command runs a body
/// the module already retains, so it supplies no unretained executable root.
fn invokes_only_retained_source_commands(
    binding: &crate::command_binding::SourceInvocationBinding,
) -> bool {
    use tcl_names::command::BindingKind;
    if binding.execution_is_unknown() {
        return false;
    }
    let mut targets = binding.execution_targets().peekable();
    targets.peek().is_some()
        && targets.all(|target| {
            !target.registry_backed && matches!(target.kind, BindingKind::Proc | BindingKind::Class)
        })
}
