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

//! Bounded class constructions: recorded definitions, constructors and manufacture slots.

use super::super::binding_projection::source_binding;
use super::super::command_identity::CommandIdentity;
use super::super::source_walk::source_effective_words;
use super::super::{
    BindingKind, MayBinding, ModuleCommandBindings, ResolvedCommandTarget, SourceCommandTarget,
    SourceExecutionContext, deferred_method,
};
use std::hash::Hash;
use std::sync::Arc;
use tcl_registry::{CommandRegistry, StateTransition};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in super::super) struct BoundedClassConstruction {
    pub(in super::super) factory: String,
    pub(in super::super) implementation_generation: u32,
}

impl ModuleCommandBindings {
    pub(in super::super) fn default_construction_dependencies_hold(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    ) -> bool {
        let Some(root) = self.source_root_namespace_key() else {
            return false;
        };
        let Some(policy) = self
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        else {
            return false;
        };
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_OO_DEPENDENCIES root_known={} geometry_policy={} taint_all={} opaque={} step={} execution={}",
                self.namespaces.contains(&root),
                (*self.original_command_world)
                    .clone()
                    .select_policy(self)
                    .is_some(),
                self.tainted_object_dispatch.contains("*"),
                self.has_opaque_domain(),
                self.source_step_observed(),
                self.source_execution_observed(None)
            );
        }
        !self.tainted_object_dispatch.contains("*")
            && grammar
                .default_construction_lookup_dependencies()
                .is_some_and(|dependencies| {
                    dependencies.iter().all(|command| {
                        let selected = self.original_registry_metadata_target(&root, command, policy);
                        #[cfg(test)]
                        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
                            eprintln!("ORIGINAL_OO_DEPENDENCY command={command} tainted={} paths={} selected={} generation={:?}",
                                self.tainted_object_dispatch.contains(*command), self.original_registry_command_paths(&root, command, policy).is_some(),
                                selected.is_some(), selected.as_ref().map(|target| target.implementation_generation));
                        }
                        !self.tainted_object_dispatch.contains(*command)
                            && selected.is_some_and(|target| {
                                    target.registry_backed
                                        && target.prepended.is_empty()
                                        && tcl_registry::command_descriptor_labels_match(&target.command, command)
                                        && target.implementation_generation == 0
                                })
                    })
                })
    }

    pub(in super::super) fn record_bounded_class(
        &mut self,
        facts: &tcl_registry::InvocationFacts,
        words: &[tcl_source::words::WordExpr],
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        let Some(dialect) = self.baseline.dialect else {
            return false;
        };
        let Some(grammar) = context.registry.native_default_construction_grammar(
            &target.command,
            dialect,
            context.realm(),
        ) else {
            return false;
        };
        if !self.default_construction_dependencies_hold(grammar) {
            return false;
        }
        let Some(name) = facts.state_transitions.declared().and_then(|transitions| {
            transitions
                .facts()
                .iter()
                .find_map(|fact| match &fact.transition {
                    StateTransition::ObjectDispatch(
                        tcl_registry::ObjectDispatchTransition::Create {
                            target: tcl_registry::ObjectDispatchTarget::Named(subject),
                            kind: tcl_registry::ObjectDispatchKind::Class,
                            ..
                        },
                    ) => subject.literal(),
                    _ => None,
                })
        }) else {
            return false;
        };
        let source = facts.arg_roles.iter().find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::Body)
                .then_some(facts.argument_offset + usize::from(*index))
        });
        if let Some(index) = source {
            let Some(index) = index.checked_sub(target.prepended.len()) else {
                return false;
            };
            if let Some(word) = words.get(index + 1) {
                let effective = source_effective_words(
                    std::slice::from_ref(word),
                    self.baseline.dialect,
                    Some((&self.source_variables, context.registry)),
                );
                let Some(body) = effective[0].as_registry_word().literal() else {
                    return false;
                };
                if !self.definition_has_only_deferred_members(body, grammar, context) {
                    return false;
                }
            }
        }
        let proof = source_binding(self, name, &context.namespace_identity());
        let Some(created) = proof.proved_target() else {
            return false;
        };
        let Some(identity) = &created.identity else {
            return false;
        };
        Arc::make_mut(&mut self.bounded_constructions).insert(
            identity.clone(),
            BoundedClassConstruction {
                factory: target.command.clone(),
                implementation_generation: created.implementation_generation,
            },
        );
        true
    }

    pub(in super::super) fn definition_has_only_deferred_members(
        &self,
        body: &str,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        definition_body_command_tokens(body, context.config).all(|words| {
                let effective = source_effective_words(
                    words.words(),
                    self.baseline.dialect,
                    Some((&self.source_variables, context.registry)),
                );
                let Some(head) = effective
                    .first()
                    .and_then(|word| word.as_registry_word().literal())
                else {
                    return false;
                };
                let Some(member) = grammar.member(head) else {
                    return false;
                };
                if member.visibility_effect.is_some()
                    && grammar.preserves_local_constructor_entry(member)
                    && grammar.manufacture_member_effect(member)
                        == tcl_registry::definer::MemberManufactureDispatchEffect::PreservesNativeFactory
                {
                    return deferred_method::original_definition_member(
                        grammar,
                        member,
                        tcl_registry::definer::DefinitionReceiver::Instance,
                        self,
                        context,
                    ) && effective
                        .iter()
                        .all(|word| word.as_registry_word().literal().is_some());
                }
                let query = self.baseline.dialect
                    .and_then(tcl_native_model::InvocationDialect::authoring_query)
                    .map(|query| query.with_realm(context.realm()));
                let delegate_preserves_manufacturers = member.arg_roles.iter()
                    .find_map(|(index, role)| (*role == tcl_registry::ArgRole::Name)
                        .then_some(usize::from(*index) + 1))
                    .and_then(|index| effective.get(index))
                    .and_then(|word| word.as_registry_word().literal())
                    .is_some_and(|name| grammar.native_classmethod_preserves_default_manufacturers(member, name, query));
                if grammar.construction_member_effect(member)
                    != tcl_registry::definer::MemberConstructionEffect::DeferredInstanceMethod
                    && !delegate_preserves_manufacturers
                {
                    return self
                        .closed_inherited_member(grammar, member, &effective, context)
                        .is_some();
                }
                let Some(lookup) =
                    grammar.definition_member_lookup(member, query)
                else {
                    return false;
                };
                if !source_binding(self, head, lookup.namespace)
                    .proved_target()
                    .is_some_and(|target| {
                        target.registry_backed
                            && target.prepended.is_empty()
                            && target.command == lookup.implementation
                            && target.implementation_generation == 0
                    })
                {
                    return false;
                }
                let expected = member
                    .arg_roles
                    .iter()
                    .map(|(index, _)| usize::from(*index) + 2)
                    .max()
                    .unwrap_or(1);
                effective.len() == expected
                    && effective
                        .iter()
                        .all(|word| word.as_registry_word().literal().is_some())
                    && member.arg_roles.iter().all(|(index, role)| {
                        if *role != tcl_registry::ArgRole::ParamList {
                            return true;
                        }
                        effective
                            .get(usize::from(*index) + 1)
                            .and_then(|word| word.as_registry_word().literal())
                            .is_some_and(|parameters| {
                                self.baseline.dialect
                                    .and_then(tcl_native_model::InvocationDialect::parameter_grammar)
                                    .is_some_and(|grammar| {
                                        tcl_syntax::formal_params::parse_formal_parameters_in(
                                            parameters, grammar,
                                        ).is_ok()
                                    })
                            })
                    })
            })
    }

    pub(in super::super) fn apply_bounded_constructor(
        &mut self,
        target: &SourceCommandTarget,
        words: &[crate::registry_invocation::EffectiveInvocationWord],
        registry: &CommandRegistry,
        namespace: &str,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> bool {
        let Some(identity) = &target.identity else {
            return false;
        };
        let Some(construction) = self.bounded_constructions.get(identity) else {
            return false;
        };
        if target.implementation_generation != construction.implementation_generation {
            return false;
        }
        if self
            .class_definitions
            .get(identity)
            .is_none_or(|definition| !self.class_definition_dependencies_hold(definition))
        {
            return false;
        }
        let Some(grammar) = self.baseline.dialect.and_then(|dialect| {
            registry.native_default_construction_grammar(&construction.factory, dialect, realm)
        }) else {
            return false;
        };
        if !self.default_construction_dependencies_hold(grammar) {
            return false;
        }
        let Some(method) = words
            .first()
            .and_then(|word| word.as_registry_word().literal())
            .and_then(|method| grammar.manufacturer(method))
        else {
            return false;
        };
        if method.visibility != tcl_registry::definer::MemberVisibility::Exported
            || words.len() != usize::from(method.constructor_args_from)
            || words.iter().any(|word| {
                matches!(
                    word,
                    crate::registry_invocation::EffectiveInvocationWord::Expanded
                        | crate::registry_invocation::EffectiveInvocationWord::Opaque
                )
            })
        {
            return false;
        }
        self.install_manufacture_slot(target, method, words, namespace)
    }

    fn install_manufacture_slot(
        &mut self,
        target: &SourceCommandTarget,
        method: &tcl_registry::definer::ManufacturerMethod,
        words: &[crate::registry_invocation::EffectiveInvocationWord],
        namespace: &str,
    ) -> bool {
        if let Some(index) = method.names_instance_at {
            let Some(name) = words
                .get(usize::from(index))
                .and_then(|word| word.as_registry_word().literal())
                .filter(|name| !name.is_empty())
            else {
                return false;
            };
            if !tcl_syntax::naming::is_absolute(name) && !self.source_variables.namespace_known {
                return false;
            }
            let name = tcl_syntax::naming::qualify(namespace, name);
            if !self.definitely_absent(&name, namespace) {
                return false;
            }
            // A proved fresh root slot cannot replace an existing object's
            // private dispatcher. Other namespace routes keep that dependency
            // conservative until their actual namespace object is retained.
            let preserves_dispatcher = tcl_syntax::naming::key_holder_and_tail(&name).0 == "::";
            let dispatcher_generation = self.object_instances.receiver_dispatcher_generation;
            let receivers = preserves_dispatcher.then(|| self.object_instances.receivers.clone());
            self.install(
                name.clone(),
                MayBinding::Target(ResolvedCommandTarget {
                    command: name.clone(),
                    prepended: Vec::new(),
                    registry_backed: false,
                    kind: BindingKind::Command,
                    implementation_generation: 0,
                    terminal: true,
                    target_lookup: tcl_registry::AliasTargetLookup::Global,
                    token: Some(CommandIdentity {
                        runtime: None,
                        origin: name,
                        declaration: target.implementation_generation,
                        allocation: None,
                    }),
                    implementation_allocation: None,
                }),
            );
            if let Some(receivers) = receivers {
                let objects = Arc::make_mut(&mut self.object_instances);
                objects.receiver_dispatcher_generation = dispatcher_generation;
                objects.receivers = receivers;
            }
        }
        true
    }
}

pub(in super::super) fn definition_body_command_tokens(
    body: &str,
    parent_config: tcl_lexer::LexerConfig,
) -> impl Iterator<Item = crate::ir::CommandTokens> + '_ {
    let config = parent_config.nested();
    let map = tcl_lexer::SourceMap::new(body);
    tcl_source::segment::segment_commands_with_offset_and_config(body, 0, config)
        .into_iter()
        .map(move |segment| crate::ir::CommandTokens::from_segmented(&map, config, &segment))
}
