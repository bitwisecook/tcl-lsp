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

//! Argument evaluation coordinates declared by the selected registry descriptor.
//!
//! These facts describe where a value would be interpreted. They establish
//! neither that the argument executes nor that a runtime implementation was selected.

use crate::hover::ScriptTiming;
use crate::state_transition::TransitionSubject;
use crate::{ArgRole, BodyInterpreter, InterpreterScope, InvocationArguments, ResolvedInvocation};

/// The activation used to interpret a script or resolve an identity operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationFrame {
    /// The receiving invocation's activation.
    Invoking,
    /// The selected interpreter's global activation at entry.
    Global,
    /// The activation whose operation triggers a callback.
    Trigger,
    /// Namespace activation selected by this descriptor-local argument.
    NamespaceArgument(u8),
    /// A fixed namespace supplied by the selected standard definition profile.
    NamedNamespace(&'static str),
    /// The namespace of the invocation, independently of its local variable table.
    CurrentNamespace,
    /// The namespace of a lambda list, selected by this argument.
    LambdaArgument(u8),
    /// The existing frame-effect descriptor selects the activation.
    LevelSelected,
    /// The object-system profile supplies the receiver and helper environment.
    ObjectReceiver,
    /// The invocation creates a procedure activation when the body is called.
    Procedure,
    /// No activation contract has been established.
    Unknown,
}

impl EvaluationFrame {
    /// Default standard definition namespace supplied by an authored object profile.
    /// Missing/custom worker coordinates remain unknown.
    #[must_use]
    pub const fn for_definition_profile(profile: &crate::definer::ObjectSystemProfile) -> Self {
        match profile.native_definition_namespace {
            Some(namespace) => Self::NamedNamespace(namespace),
            None => Self::Unknown,
        }
    }
}

/// Timing, activation and interpreter of one argument's interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvaluationContext {
    /// Interpreter selector in descriptor-local argument coordinates.
    pub interpreter: BodyInterpreter,
    /// Activation coordinate, independent of callback timing.
    pub frame: EvaluationFrame,
    /// Whether interpretation happens now, later, or only names a reference.
    pub timing: ScriptTiming,
}

impl EvaluationContext {
    /// Immediate interpretation in the invoking activation.
    pub const INVOKING: Self = Self {
        interpreter: BodyInterpreter::Current,
        frame: EvaluationFrame::Invoking,
        timing: ScriptTiming::SameInvocation,
    };
    /// A deferred callback entered in the selected interpreter's global frame.
    pub const DEFERRED_GLOBAL: Self = Self {
        frame: EvaluationFrame::Global,
        timing: ScriptTiming::Deferred,
        ..Self::INVOKING
    };
    /// An identity is referenced without being invoked.
    pub const REFERENCE: Self = Self {
        timing: ScriptTiming::ReferenceOnly,
        ..Self::INVOKING
    };
}

/// Shared context for every selected argument carrying one of these roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleEvaluationContext {
    /// Roles this rule describes; it does not assign roles to any word.
    pub roles: &'static [ArgRole],
    /// Context of interpretation for each selected role.
    pub context: EvaluationContext,
}

/// Invocation-sensitive contexts, retaining unknown and expanded words.
pub type EvaluationContextResolver =
    for<'w> fn(InvocationArguments<'w>, usize) -> Option<EvaluationContext>;

/// Authored contexts of a command, subcommand or invocation form.
#[derive(Debug, Clone, Copy)]
pub struct EvaluationContextSpec {
    /// Positional overrides, in descriptor-local coordinates.
    pub arguments: &'static [(u8, EvaluationContext)],
    /// Role defaults after positional overrides.
    pub roles: &'static [RoleEvaluationContext],
    /// Closed inherited role defaults retained when a descriptor overrides `roles`.
    pub fallback_roles: &'static [RoleEvaluationContext],
    /// Conditional grammar consulted before static rows.
    pub resolver: Option<EvaluationContextResolver>,
    /// Roles whose conditional contexts the resolver completely owns.
    /// Every other role keeps its static rows or remains unknown.
    pub resolver_roles: &'static [ArgRole],
}

const CALLER_ROLE_CONTEXTS: &[RoleEvaluationContext] = &[
    RoleEvaluationContext {
        roles: &[ArgRole::Body, ArgRole::Expr, ArgRole::CommandPrefix],
        context: EvaluationContext::INVOKING,
    },
    RoleEvaluationContext {
        roles: &[
            ArgRole::VarRead,
            ArgRole::VarWrite,
            ArgRole::VarCell,
            ArgRole::Name,
            ArgRole::CommandName,
            ArgRole::CommandNameProbe,
            ArgRole::NamespaceName,
            ArgRole::LoopVarList,
        ],
        context: EvaluationContext::REFERENCE,
    },
];

impl EvaluationContextSpec {
    /// An explicitly empty declaration; arguments remain undescribed.
    pub const EMPTY: Self = Self {
        arguments: &[],
        roles: &[],
        fallback_roles: &[],
        resolver: None,
        resolver_roles: &[],
    };
    /// Current-frame script interpretation and ordinary current-frame references.
    pub const CALLER: Self = Self {
        fallback_roles: CALLER_ROLE_CONTEXTS,
        ..Self::EMPTY
    };
    /// Stored scripts and prefixes entered globally, with ordinary identity references.
    pub const DEFERRED_GLOBAL: Self = Self {
        roles: &[RoleEvaluationContext {
            roles: &[ArgRole::Body, ArgRole::CommandPrefix],
            context: EvaluationContext::DEFERRED_GLOBAL,
        }],
        ..Self::EMPTY
    };

    /// Read a context without assigning a role or inventing a value for a word.
    #[must_use]
    pub fn context_at(
        self,
        arguments: InvocationArguments<'_>,
        index: usize,
        role: ArgRole,
    ) -> Option<EvaluationContext> {
        arguments.exact_argv_len()?;
        arguments.get(index)?;
        if self.resolver_roles.contains(&role)
            && let Some(resolve) = self.resolver
        {
            return resolve(arguments, index);
        }
        self.arguments
            .iter()
            .find(|(at, _)| usize::from(*at) == index)
            .map(|(_, context)| *context)
            .or_else(|| {
                self.roles
                    .iter()
                    .chain(self.fallback_roles.iter())
                    .find(|row| row.roles.contains(&role))
                    .map(|row| row.context)
            })
    }
}

/// One selected argument's context, with its interpreter path resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentEvaluationContext {
    /// Position after the command head, including any subcommand selector.
    pub argument: usize,
    /// The selected argument role for which this answer applies.
    pub role: ArgRole,
    /// Authored symbolic activation and timing. `None` means no descriptor.
    pub context: Option<EvaluationContext>,
    /// Selected interpreter; unknown paths remain `Any`, never current by default.
    pub interpreter: InterpreterScope,
    /// Namespace/lambda operand when the frame selects one; unknown stays typed.
    pub frame_subject: Option<TransitionSubject>,
}

impl ResolvedInvocation<'_, '_> {
    /// Context of a selected argument, including explicit context-only positions.
    /// This reads authored coordinates without assigning a new role to the word.
    #[must_use]
    pub fn argument_evaluation_context(&self, argument: usize) -> Option<EvaluationContext> {
        let roles = self.arg_roles().unwrap_or_default();
        let role = roles
            .iter()
            .find(|(at, _)| *at == argument)
            .map_or(ArgRole::Value, |(_, role)| *role);
        self.argument_evaluation_context_as(argument, role)
    }

    /// [`Self::argument_evaluation_context`] for an argument already known to
    /// carry `role`, such as one the successful-edge role layout selected.
    fn argument_evaluation_context_as(
        &self,
        argument: usize,
        role: ArgRole,
    ) -> Option<EvaluationContext> {
        let descriptor = self
            .selected
            .form
            .and_then(|form| form.evaluation_contexts)
            .or_else(|| self.selected.sub.and_then(|sub| sub.evaluation_contexts))
            .or_else(|| {
                (!self.selected.instance)
                    .then_some(self.selected.spec.evaluation_contexts)
                    .flatten()
            })?;
        let local = argument.checked_sub(self.semantics.argument_offset)?;
        descriptor
            .context_at(
                self.words
                    .arguments()
                    .slice_from(self.semantics.argument_offset),
                local,
                role,
            )
            .map(|context| reference_timing(self, argument, context))
    }

    /// Contexts for every selected scope-sensitive argument, in post-head coordinates.
    /// Missing descriptors remain explicit and grant no execution permission.
    #[must_use]
    pub fn argument_evaluation_contexts(&self) -> Option<Vec<ArgumentEvaluationContext>> {
        let offset = self.semantics.argument_offset;
        let descriptor = self
            .selected
            .form
            .and_then(|form| form.evaluation_contexts)
            .or_else(|| self.selected.sub.and_then(|sub| sub.evaluation_contexts))
            .or_else(|| {
                (!self.selected.instance)
                    .then_some(self.selected.spec.evaluation_contexts)
                    .flatten()
            });
        let arguments = self.words.arguments().slice_from(offset);
        Some(
            self.arg_roles()?
                .into_iter()
                .filter(|(_, role)| role.needs_evaluation_context())
                .filter_map(|(index, role)| {
                    let local = index.checked_sub(offset)?;
                    let context = descriptor
                        .and_then(|spec| spec.context_at(arguments, local, role))
                        .map(|context| reference_timing(self, index, context));
                    let interpreter = context.map_or(InterpreterScope::Any, |context| {
                        context.interpreter.resolve(arguments)
                    });
                    let frame_subject = context.and_then(|context| match context.frame {
                        EvaluationFrame::NamespaceArgument(at)
                        | EvaluationFrame::LambdaArgument(at) => TransitionSubject::from_argument(
                            self.words.arguments(),
                            offset + usize::from(at),
                        ),
                        _ => None,
                    });
                    Some(ArgumentEvaluationContext {
                        argument: index,
                        role,
                        context,
                        interpreter,
                        frame_subject,
                    })
                })
                .collect(),
        )
    }
}

/// The frame a script argument runs in, counted from the invoking activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScriptFrame {
    /// `N` activations up from the invoking one; `Relative(0)` is the invoking
    /// activation itself (`eval`, `catch`, `if`), `Relative(1)` its caller
    /// (`uplevel 1`).
    Relative(u32),
    /// The interpreter's global activation (`uplevel #0`, `after`).
    Global,
    /// A run-time level, a namespace or another activation the descriptor
    /// does not place relative to the invoking one.
    Unknown,
}

/// Where and when one selected script argument runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptArgumentContext {
    /// Activation, counted from the invoking one.
    pub frame: ScriptFrame,
    /// Whether the script runs before the invocation returns or later.
    pub timing: ScriptTiming,
}

impl ResolvedInvocation<'_, '_> {
    /// Frame and timing of the script a `Body` argument runs, read from the
    /// selected descriptor's evaluation context and, for a level-selected
    /// activation, its frame-effect grammar on the successful path. `None`
    /// when no descriptor describes the argument or it is only referenced.
    #[must_use]
    pub fn script_argument_context(&self, argument: usize) -> Option<ScriptArgumentContext> {
        use crate::frame_effect::{FrameArgumentResolution, FrameLevel};
        let context = self.argument_evaluation_context_as(argument, ArgRole::Body)?;
        if context.timing == ScriptTiming::ReferenceOnly
            || context.interpreter != BodyInterpreter::Current
        {
            return None;
        }
        let frame = match context.frame {
            EvaluationFrame::Invoking => ScriptFrame::Relative(0),
            EvaluationFrame::Global => ScriptFrame::Global,
            EvaluationFrame::LevelSelected => match self
                .semantics
                .frame_effect
                .map(|effect| effect.successful_layout(self.words.arguments()).layout)
            {
                Some(FrameArgumentResolution::Valid {
                    level: FrameLevel::Relative(level),
                    ..
                }) => ScriptFrame::Relative(level),
                Some(FrameArgumentResolution::Valid {
                    level: FrameLevel::Absolute(0),
                    ..
                }) => ScriptFrame::Global,
                _ => ScriptFrame::Unknown,
            },
            _ => ScriptFrame::Unknown,
        };
        Some(ScriptArgumentContext {
            frame,
            timing: context.timing,
        })
    }
}

fn reference_timing(
    invocation: &ResolvedInvocation<'_, '_>,
    argument: usize,
    mut context: EvaluationContext,
) -> EvaluationContext {
    if let Some(scope) = invocation.authored_source_option_variable_scope_at(argument) {
        context.frame = match scope {
            crate::VariableScope::CurrentFrame => EvaluationFrame::Invoking,
            crate::VariableScope::Global => EvaluationFrame::Global,
        };
    }
    if invocation.authored_source_script_timing_at(argument) == Some(ScriptTiming::ReferenceOnly) {
        context.timing = ScriptTiming::ReferenceOnly;
    }
    context
}

impl ArgRole {
    /// Whether the role interprets code or names an identity in an activation.
    #[must_use]
    pub const fn needs_evaluation_context(self) -> bool {
        matches!(
            self,
            Self::Body
                | Self::Expr
                | Self::CommandPrefix
                | Self::LambdaLiteral
                | Self::Name
                | Self::CommandName
                | Self::CommandNameProbe
                | Self::NamespaceName
                | Self::VarRead
                | Self::VarWrite
                | Self::VarCell
                | Self::LoopVarList
                | Self::ParamList
                | Self::StaticVarList
        )
    }
}

#[cfg(test)]
mod tests;

/// Naming coordinates projected from the existing variable-alias target owner.
/// A missing transition supplies no frame fact and never falls back to caller scope.
#[must_use]
pub fn variable_alias_target_context(
    transitions: &crate::StateTransitions,
    argument: usize,
) -> Option<EvaluationContext> {
    use crate::state_transition::{StateTransition, VariableAliasTarget};
    transitions.facts().iter().find_map(|fact| {
        let StateTransition::VariableCellAlias(alias) = &fact.transition else {
            return None;
        };
        if alias.words.target != argument {
            return None;
        }
        let frame = match &alias.target {
            VariableAliasTarget::Global { .. } => EvaluationFrame::Global,
            VariableAliasTarget::CurrentNamespace { .. } => EvaluationFrame::CurrentNamespace,
            VariableAliasTarget::CallerSelectedFrame { .. } => EvaluationFrame::LevelSelected,
            VariableAliasTarget::Namespace { namespace, .. } => {
                EvaluationFrame::NamespaceArgument(u8::try_from(namespace.argument_index()?).ok()?)
            }
        };
        Some(EvaluationContext {
            frame,
            ..EvaluationContext::REFERENCE
        })
    })
}
