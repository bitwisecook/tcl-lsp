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

//! Definitions in the authored source: procedure parameters, command publication, class lists, object factories and definition bodies.

use super::ResolvedInvocation;
use crate::arg_role::ArgRole;
use crate::arity::Arity;
use crate::traits::Traits;

/// Conditional list-operand geometry of the selected Jim class source form.
/// This retains the authored recipe only; no Native argument, class allocation,
/// handler entry, body admission or published command follows from it.
#[derive(Debug, Clone, Copy)]
pub struct AuthoredSourceJimClassLists {
    /// Exact selected family descriptor, independently of runtime occupation.
    pub grammar: &'static crate::definer::DefinitionBodyGrammar,
    /// Selected authored Jim simulation for Original source. Logical source
    /// retains its independent full input at the caller and issues no recipe.
    pub policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
    /// Effective original operand containing bases; absence means no base operand.
    pub bases: Option<usize>,
    /// Effective original operand containing the instance-variable dictionary.
    pub variables: usize,
}

impl AuthoredSourceJimClassLists {
    /// Finite stock source layout under this exact authored family recipe.
    /// Current provider, operands, publication and observer proofs remain separate.
    #[must_use]
    pub fn stock_source_recipe(&self) -> Option<crate::definer::SourceJimStockClassRecipe> {
        crate::definer::SourceJimStockClassRecipe::for_original_layout(self)
    }
}

/// Effective authored procedure declaration positions. This layout accepts
/// no native parameter grammar and proves no publication or entered body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredSourceProcedureArguments {
    /// Original effective post-head procedure-name ordinal.
    pub name: usize,
    /// Original effective post-head formal-list ordinal.
    pub parameters: usize,
    /// Original effective post-head deferred-body ordinal.
    pub body: usize,
}

/// Static-list source geometry, without native parsing or an entered frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoredSourceProcedureStaticList {
    /// The complete selected declaration roles contain no static-list operand.
    Absent,
    /// Effective post-head ordinal of the one selected static-list operand.
    Argument(usize),
}

/// Conditional authored publication layout, without command creation or lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoredSourceCommandPublicationKind {
    /// A descriptor names a future command, independently of its implementation.
    Command,
    /// A mandatory named factory describes a possible instance class.
    Instance {
        /// Descriptor's conditional instance class, without native allocation.
        class_name: &'static str,
    },
}

/// Effective naming operand selected by the same exact source invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredSourceCommandPublication {
    /// Effective post-head naming position, including any selected member.
    pub argument: usize,
    /// Source-only publication purpose; no successful creation follows.
    pub kind: AuthoredSourceCommandPublicationKind,
}

/// Authored class candidate and name operand of a selected naming factory.
/// This inert metadata cannot prove a created command, successful invocation,
/// source declaration or object-method dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedObjectFactory {
    pub(super) argument: u8,
    pub(super) class_name: &'static str,
}

impl NamedObjectFactory {
    /// Post-head argument naming the object command under this contract.
    #[must_use]
    pub fn argument(self) -> usize {
        usize::from(self.argument)
    }

    /// Authored nominal class candidate, independently of runtime identity.
    #[must_use]
    pub const fn class_name(self) -> &'static str {
        self.class_name
    }
}

impl ResolvedInvocation<'_, '_> {
    fn authored_source_procedure_arity(&self) -> Arity {
        if self.uses_authored_jim_core_layout() {
            self.selected_arity()
        } else {
            self.semantics.arity
        }
    }

    /// Source-only procedure declaration shape from the selected descriptor.
    /// Original argv cardinality and authored role consensus stay mandatory;
    /// native parameter acceptance and successful definition remain separate.
    /// Configured Proc hooks also require their exact selected publication
    /// transition and deferred source timing, without a native descriptor.
    #[must_use]
    pub fn authored_source_procedure_arguments(&self) -> Option<AuthoredSourceProcedureArguments> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let native_definition = self.semantics.procedure_definition.is_some();
        let authored_hook =
            self.semantics.analyser_hook == Some(crate::hooks::AnalyserHookId::Proc);
        if !native_definition && !authored_hook {
            return None;
        }
        if self.semantics.frame_effect.is_some()
            || (native_definition
                && !self
                    .semantics
                    .traits
                    .contains(Traits::DEFINES_PROCEDURE | Traits::DEFERS_BODY))
            || !self
                .authored_source_procedure_arity()
                .accepts(self.argument_count_for_arity()?)
        {
            return None;
        }
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        let ordinal = |wanted| {
            let mut positions = roles.iter().filter_map(|&(position, role)| {
                (role == wanted).then_some(self.semantics.argument_offset + usize::from(position))
            });
            let first = positions.next()?;
            (first < count && positions.next().is_none()).then_some(first)
        };
        let positions = AuthoredSourceProcedureArguments {
            name: ordinal(ArgRole::Name)?,
            parameters: ordinal(ArgRole::ParamList)?,
            body: ordinal(ArgRole::Body)?,
        };
        if !native_definition {
            use crate::state_transition::{
                CommandBindingDefinitionKind, CommandBindingTransition, StateTransitionDomain,
                TransitionSubject,
            };
            let offset = self.semantics.argument_offset;
            if count != offset.checked_add(3)?
                || positions.name != offset
                || positions.parameters != offset.checked_add(1)?
                || positions.body != offset.checked_add(2)?
            {
                return None;
            }
            let name = TransitionSubject::from_argument(self.words.arguments(), positions.name)?;
            let transitions = self.state_transitions();
            if transitions.widens(StateTransitionDomain::CommandBindings)
                || transitions.widens(StateTransitionDomain::CommandResolution)
            {
                return None;
            }
            let mut definitions = transitions.command_bindings();
            match (definitions.next(), definitions.next()) {
                (
                    Some(CommandBindingTransition::Define {
                        name: selected,
                        kind: CommandBindingDefinitionKind::Procedure,
                    }),
                    None,
                ) if *selected == name => {}
                _ => return None,
            }
            (self.semantics.script_metadata.timing_at_with_default(
                self,
                positions.body,
                crate::ScriptTiming::Deferred,
            ) == Some(crate::ScriptTiming::Deferred))
            .then_some(())?;
        }
        Some(positions)
    }

    /// Independently selected source static-list geometry. Unavailable or
    /// conflicting roles cannot be treated as an absent static operand.
    #[must_use]
    pub fn authored_source_procedure_static_list(
        &self,
    ) -> Option<AuthoredSourceProcedureStaticList> {
        let shape = self.authored_source_procedure_arguments()?;
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        complete.then_some(())?;
        let mut statics = roles.iter().filter_map(|&(position, role)| {
            (role == ArgRole::StaticVarList)
                .then_some(self.semantics.argument_offset + usize::from(position))
        });
        match (statics.next(), statics.next()) {
            (None, None) => Some(AuthoredSourceProcedureStaticList::Absent),
            (Some(at), None)
                if at < count && ![shape.name, shape.parameters, shape.body].contains(&at) =>
            {
                Some(AuthoredSourceProcedureStaticList::Argument(at))
            }
            _ => None,
        }
    }

    /// Exact authored naming layout from the selected descriptors and options.
    /// Unknown control words, expansion cardinality, and optional factory names
    /// decline. This supplies source metadata only, never a published command.
    #[must_use]
    pub fn authored_source_command_publication(&self) -> Option<AuthoredSourceCommandPublication> {
        // naming.source.original-command-name-publications
        // docs/design/analysis/name-resolution-proofs/original-command-name-publications.md
        let descriptors = self.authored_source_descriptors();
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let offset = self.semantics.argument_offset;
        let (relative, kind) = if let Some(sub) = descriptors.subcommand {
            (
                sub.defines_command_at?,
                AuthoredSourceCommandPublicationKind::Command,
            )
        } else if let Some(argument) = descriptors.command.creates_instance_at {
            // Optional naming/control layouts require their own authored recipe.
            if descriptors.command.arity.min == 0 {
                return None;
            }
            (
                argument,
                AuthoredSourceCommandPublicationKind::Instance {
                    class_name: descriptors
                        .command
                        .object_class
                        .map_or(descriptors.command.name, |class| class.class_name),
                },
            )
        } else {
            if descriptors.command.traits.contains(Traits::IS_OO_METACLASS)
                && descriptors.command.definition_body.is_some()
            {
                return None;
            }
            (
                descriptors.command.defines_command_at?,
                AuthoredSourceCommandPublicationKind::Command,
            )
        };
        let leading = self
            .semantics
            .options
            .leading_word_count(arguments.slice_from(offset))?;
        let argument = offset
            .checked_add(leading)?
            .checked_add(usize::from(relative))?;
        (argument < count).then_some(AuthoredSourceCommandPublication { argument, kind })
    }

    /// Original Jim class list layout under this selected source schema.
    /// Dynamic operands retain their positions; unknown argv cardinality and
    /// unsupported source protocols decline. Consumers must retain the actual
    /// original words and context rather than reparsing reporting strings.
    #[must_use]
    pub fn authored_source_jim_class_lists(
        &self,
        purpose: crate::source_name_ownership::SourceRolePurpose,
    ) -> Option<AuthoredSourceJimClassLists> {
        use tcl_syntax::naming::{NamePolicyAuthority, NativeNameProtocol};
        let selected = self.authored_source_descriptors();
        let grammar = selected.command.definition_body?;
        if selected.subcommand.is_some()
            || self.semantics.argument_offset != 0
            || grammar.family != crate::definer::DefinerFamily::JimClass
        {
            return None;
        }
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let arity = self.authored_source_arity()?;
        if !(2..=3).contains(&count)
            || arity.count.indeterminate
            || !arity.arity.accepts(arity.count.minimum)
        {
            return None;
        }
        let dialect = arguments.dialect()?;
        if dialect.family() != Some(tcl_dialect::model::Family::Jim)
            || dialect.lexer_grammar.escapes != tcl_dialect::EscapeSyntax::Jim
            || dialect.word_values != tcl_syntax::word_rules::WordValueRules::JIM
        {
            return None;
        }
        let policy = match purpose {
            crate::source_name_ownership::SourceRolePurpose::Original => {
                let policy = dialect.authored_name_policy()?;
                if policy.authority() != NamePolicyAuthority::AuthoredSimulation
                    || policy.recipe() != NativeNameProtocol::Jim084
                {
                    return None;
                }
                Some(policy)
            }
            crate::source_name_ownership::SourceRolePurpose::Logical => {
                // The caller must retain the positive Logical source owner.
                // An unavailable Native recipe alone supplies no such owner.
                dialect.native_name_protocol().is_none().then_some(())?;
                None
            }
        };
        Some(AuthoredSourceJimClassLists {
            grammar,
            policy,
            bases: grammar.source_base_class_list_argument(count),
            variables: count.checked_sub(1)?,
        })
    }

    /// Conditional callable class of this selected factory descriptor.
    /// Original operands and exported manufacturer visibility are retained;
    /// this does not prove object creation, completion or method dispatch.
    #[must_use]
    pub fn authored_source_callable_factory_class(&self) -> Option<&'static str> {
        let descriptors = self.authored_source_descriptors();
        if descriptors.subcommand.is_some() {
            return None;
        }
        if let Some(factory) = self.semantics.named_object_factory {
            return Some(factory.class_name());
        }
        let class = descriptors.command.object_class?;
        let method = crate::CommandRegistry::manufacturer_method_for_descriptor(
            descriptors.command,
            self.words.arguments().literal_at(0)?,
        )?;
        (method.visibility == crate::definer::MemberVisibility::Exported)
            .then_some(class.class_name)
    }

    /// Definition vocabulary of an actual authored script operand. Configure
    /// operations enter that vocabulary only for the body directly following
    /// their selected target; an inline method body remains ordinary Tcl.
    /// This projection supplies neither a runtime frame nor target presence.
    #[must_use]
    pub fn authored_source_definition_body_grammar(
        &self,
    ) -> Option<&'static crate::definer::DefinitionBodyGrammar> {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let grammar = self.authored_source_descriptors().command.definition_body?;
        let transitions = self.state_transitions();
        let target = transitions
            .facts()
            .iter()
            .find_map(|fact| match &fact.transition {
                crate::StateTransition::ObjectDispatch(
                    crate::ObjectDispatchTransition::Configure { target, .. },
                ) => target.argument_index(),
                _ => None,
            });
        if let Some(target) = target {
            let body = target.checked_add(1)?;
            let (roles, complete) = self.authored_source_argument_roles();
            return (complete
                && roles.iter().any(|&(index, role)| {
                    self.semantics
                        .argument_offset
                        .checked_add(usize::from(index))
                        == Some(body)
                        && role == ArgRole::Body
                }))
            .then_some(grammar);
        }
        Some(grammar)
    }
}
