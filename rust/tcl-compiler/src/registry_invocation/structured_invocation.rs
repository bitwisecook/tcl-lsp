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

//! Structured source invocations: Logical roles, simulation support and declared layouts.

use super::body_candidates::{body_assistance_context, owned_body_layout_invocation};
use super::handler_invocation::resolved_handler_invocation_with_metadata_context;
use super::metadata_context::InvocationMetadataContext;
use super::source_model_operations::resolve_original_declared_layout;
#[cfg(debug_assertions)]
use super::trace_jim_structure_fields;
use super::{
    AuthoredSourceCaseBodySelection, InvocationWordOrigin, ResolvedStatementInvocation,
    host_structure_observer, source_structure,
};
use crate::ir::CommandTokens;
use tcl_registry::CommandRegistry;
use tcl_source::words::WordExpr;
use tcl_source::words::WordPart;

/// Structured semantics of one exact source invocation for analysis.
///
/// Conditional declarations describe their own frame only. This projection
/// grants no native compilation admission, executed activation, successful
/// store, caller identity or outward effect.
pub(crate) struct LogicalStructuredInvocation {
    pub(super) invocation: ResolvedStatementInvocation,
    pub(super) source_roles: Option<StructuredSourceRoles>,
    source_purpose: crate::command_binding::OriginalInvocationSourcePurpose,
}

/// Distinct source purposes retain their own registry-issued role owners.
/// The Native case projection needs an actual closed compiler observation;
/// neither purpose grants a handler, entered arm or completed operation.
pub(super) enum StructuredSourceRoles {
    Logical(tcl_registry::resolved_invocation::AuthoredLogicalSourceRoleProjection),
    NativeCase(tcl_registry::case_bodies::CaseBodyOperands),
    AuthoredSimulationCase(tcl_registry::case_bodies::CaseBodyOperands),
}

impl StructuredSourceRoles {
    pub(super) fn selection_unknown(&self) -> bool {
        match self {
            Self::Logical(roles) => roles.selection_unknown(),
            Self::NativeCase(roles) | Self::AuthoredSimulationCase(roles) => {
                roles.selection_unknown
            }
        }
    }

    pub(super) fn argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        match self {
            Self::Logical(roles) => roles.argument_roles().to_vec(),
            Self::NativeCase(roles) | Self::AuthoredSimulationCase(roles) => roles
                .bodies
                .iter()
                .map(|body| (body.argument, tcl_registry::ArgRole::Body))
                .collect(),
        }
    }
}

impl LogicalStructuredInvocation {
    pub(crate) fn is_authored_simulation(&self) -> bool {
        self.source_purpose
            == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    }

    pub(crate) fn canonical_command(&self) -> &str {
        &self.invocation.facts.canonical_command
    }

    pub(crate) fn descriptor_key(&self) -> tcl_registry::CanonicalCommandDescriptorKey<'_> {
        self.invocation.facts.descriptor_key()
    }

    /// Borrow selected source descriptors under the genuine supplied generation.
    /// Original direct-word correspondence remains part of this issuer.
    pub(crate) fn with_metadata_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        realm: tcl_dialect::model::InvocationRealm,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        self.invocation
            .with_metadata_schema(registry, context, realm, apply)
    }

    pub(crate) fn lowering_hook(&self) -> Option<tcl_registry::hooks::LoweringHookId> {
        self.invocation.facts.lowering_hook
    }

    pub(crate) fn written_argument(&self, effective: usize) -> Option<usize> {
        self.invocation.effective.written_argument(effective)
    }

    pub(crate) fn written_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        self.source_roles.as_ref().map_or_else(
            || self.invocation.written_argument_roles(),
            |roles| {
                roles
                    .argument_roles()
                    .iter()
                    .filter_map(|&(argument, role)| {
                        self.invocation
                            .effective
                            .written_argument(argument)
                            .map(|written| (written, role))
                    })
                    .collect()
            },
        )
    }

    /// Conditional source body positions from the exact retained Logical
    /// descriptor. These roles grant no handler, entered frame or Normal path.
    pub(crate) fn conditional_logical_body_indices(&self) -> Option<Vec<usize>> {
        if self.source_purpose != crate::command_binding::OriginalInvocationSourcePurpose::Logical {
            return None;
        }
        let StructuredSourceRoles::Logical(roles) = self.source_roles.as_ref()? else {
            return None;
        };
        if roles.selection_unknown()
            || roles.case_body_operands().is_some()
            || self.invocation.facts.operation
                != tcl_registry::SemanticOperationId::StructuredLowering(
                    tcl_registry::hooks::LoweringHookId::If,
                )
            || self.invocation.facts.body_kind != tcl_registry::BodyKind::Plain
            || self.invocation.facts.body_interpreter != tcl_registry::BodyInterpreter::Current
            || self.invocation.facts.frame_effect.is_some()
            || self.invocation.facts.body_execution.is_some()
            || self
                .invocation
                .facts
                .traits
                .contains(tcl_registry::Traits::DEFERS_BODY)
        {
            return None;
        }
        Some(
            self.written_roles()
                .into_iter()
                .filter_map(|(index, role)| (role == tcl_registry::ArgRole::Body).then_some(index))
                .collect(),
        )
    }

    /// Literal name at one genuine written variable-role operand. Captured
    /// prefix words and ambiguous effective correspondence have no occurrence.
    pub(crate) fn written_variable_name_operand(&self, written: usize) -> Option<String> {
        self.written_roles()
            .iter()
            .any(|&(argument, role)| {
                argument == written
                    && matches!(
                        role,
                        tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                    )
            })
            .then_some(())?;
        let mut arguments = (0..self.invocation.arguments.len()).filter(|&argument| {
            self.invocation.effective.written_argument(argument) == Some(written)
        });
        let argument = arguments.next()?;
        if arguments.next().is_some() {
            return None;
        }
        self.invocation.argument_literal(argument)
    }

    /// Literal payload under the retained grammar; direct written slots were
    /// checked by the source issuer. This grants no evaluated Native operand.
    pub(crate) fn argument_literal(&self, argument: usize) -> Option<String> {
        self.invocation.argument_literal(argument)
    }

    pub(crate) fn loop_bindings_conditional(&self) -> bool {
        self.invocation.facts.repeated_args.iter().any(|layout| {
            layout.conditional_binding && layout.role == tcl_registry::ArgRole::LoopVarList
        })
    }

    pub(crate) fn traits(&self) -> tcl_registry::Traits {
        self.invocation.facts.traits
    }

    pub(crate) fn body_scope<'a>(
        &self,
        registry: &CommandRegistry,
        context: impl Into<InvocationMetadataContext<'a>>,
    ) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
        let context = context.into();
        if !context.matches_registry(registry) {
            return None;
        }
        context
            .context()
            .resolve_spec(registry, self.canonical_command())?
            .body_scope
    }

    /// Abstract routing inside this original conditional body. This cannot
    /// certify an actual completion, handler effect or native instruction.
    pub(crate) fn conditional_completion_route(
        &self,
        registry: &CommandRegistry,
    ) -> tcl_registry::completion_route::InvocationCompletionRoute {
        self.invocation.completion_route(registry)
    }

    pub(crate) fn error_context(&self) -> Option<tcl_registry::InlineBodyErrorContext> {
        self.invocation.facts.operation.inline_body_error_context()
    }
}

/// Resolve logical structure through the selected handler and original words.
/// Native compilation and runtime execution consume their separate receipts.
pub(crate) fn logical_structured_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    bindings: Option<&crate::command_binding::SourceCommandBindings>,
) -> Option<LogicalStructuredInvocation> {
    let context = body_assistance_context(registry, tokens)?;
    logical_structured_invocation_with_metadata_context(registry, context.into(), tokens, bindings)
}

fn trace_original_structure(stage: &'static str, tokens: &CommandTokens, passed: bool) {
    host_structure_observer::flag(stage, passed);
    let _ = tokens;
}

fn original_structured_source_selection(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: Option<&crate::command_binding::SourceCommandBindings>,
) -> Option<ResolvedStatementInvocation> {
    let selected =
        resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens);
    trace_original_structure("resolved_handler", tokens, selected.is_some());
    let selected = selected.or_else(|| {
        let selected = bindings.and_then(|bindings| {
            owned_body_layout_invocation(registry, context, tokens, bindings)
                .map(|owned| owned.invocation)
        });
        trace_original_structure("owned_body_layout", tokens, selected.is_some());
        selected
    });
    let selected = selected.or_else(|| {
        let selected = original_declared_structured_invocation(registry, context, tokens);
        trace_original_structure("original_declaration", tokens, selected.is_some());
        selected
    });
    let selected = selected.or_else(|| {
        let selected = original_inline_control_structured_invocation(registry, context, tokens);
        trace_original_structure("original_inline_control", tokens, selected.is_some());
        selected
    })?;
    Some(selected)
}

/// A word a simulated lookup reads without evaluating anything: a literal
/// (bare, braced, or quoted or escaped text without substitutions), or one
/// whole scalar variable reference.
fn simulation_static_word(word: &WordExpr, config: tcl_lexer::LexerConfig) -> bool {
    match word {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => true,
        WordExpr::Template {
            parts,
            rejected: None,
            ..
        } => parts
            .iter()
            .all(|part| matches!(part, WordPart::Text { .. })),
        WordExpr::Variable { .. } => {
            word.sole_variable_substitution()
                .is_some_and(|(spelling, _)| {
                    tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)
                        .is_some()
                })
        }
        _ => false,
    }
}

fn authored_simulation_structure_is_supported(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    selected: &ResolvedStatementInvocation,
    source_roles: Option<&StructuredSourceRoles>,
) -> Option<()> {
    let binding = tokens.source_binding.as_ref()?;
    // A reached point whose registered handler is proved in its own closed
    // world needs no simulated declaration lookup; only bodies need modelling.
    let reached = binding.reached_in_closed_world()
        && binding
            .proved_handler_target()
            .is_some_and(|target| target.registry_backed);
    let advice = binding
        .declaration_operand_layout_advice(tokens)
        .or_else(|| binding.original_compilation_lookup_advice(tokens));
    let realm = match &advice {
        Some(advice) => {
            if !advice.closed_authored_simulation_source_lookup(tokens, context, registry) {
                return None;
            }
            advice.realm()
        }
        None if reached => binding.invocation_realm()?,
        None => return None,
    };
    let config = context.source_analysis_input()?.lexer_config();
    let roles = source_roles.map_or_else(
        || selected.written_argument_roles(),
        StructuredSourceRoles::argument_roles,
    );
    let bodiless = roles.iter().all(|(_, role)| {
        !matches!(
            role,
            tcl_registry::ArgRole::Body
                | tcl_registry::ArgRole::OpaqueScript
                | tcl_registry::ArgRole::LoopVarList
                | tcl_registry::ArgRole::ParamList
                | tcl_registry::ArgRole::StaticVarList
        )
    });
    // Simulated words must be static. A reached bodiless command's values
    // were evaluated at its point; only its head and role words must be.
    // The returned value is a value like any other operand, not a name.
    let simulated = |index: usize| {
        !(reached && bodiless)
            || index == 0
            || roles.iter().any(|(argument, role)| {
                argument + 1 == index && *role != tcl_registry::ArgRole::Result
            })
    };
    if tokens
        .words()
        .iter()
        .enumerate()
        .any(|(index, word)| simulated(index) && !simulation_static_word(word, config))
    {
        return None;
    }
    selected.with_metadata_schema(registry, context, realm, |schema| {
        authored_simulation_schema_is_supported(schema, tokens, &roles, bodiless)
    })?;
    Some(())
}

/// The schema half of [`authored_simulation_structure_is_supported`]: which
/// lowering structures the authored walk models, given the call's static words.
fn authored_simulation_schema_is_supported(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    tokens: &CommandTokens,
    roles: &[(usize, tcl_registry::ArgRole)],
    bodiless: bool,
) -> Option<()> {
    use tcl_registry::hooks::LoweringHookId;
    // A class or object definition, like a procedure, stores its body for
    // later dispatch; its static words above are all that is modelled.
    if schema
        .authored_source_definition_body_grammar()
        .is_some_and(|grammar| grammar.profile.manufactures_runtime_commands())
    {
        return Some(());
    }
    let hook = schema.facts().lowering_hook;
    // A definition stores its body for later calls in the procedure's own
    // frame, so only a fully static name, parameter list and body is modelled.
    let all_literal = tokens.words().iter().all(|word| {
        matches!(
            word,
            WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
        )
    });
    if hook == Some(LoweringHookId::Proc) {
        return (schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
            && all_literal)
            .then_some(());
    }
    // A static `namespace eval` runs its body now, in the named namespace's
    // frame; the walk models that frame for each command it contains.
    if hook == Some(LoweringHookId::NamespaceEval) {
        return (schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
            && all_literal
            && roles.iter().all(|&(argument, role)| {
                role != tcl_registry::ArgRole::Body
                    || schema.authored_source_script_timing_at(argument)
                        == Some(tcl_registry::ScriptTiming::SameInvocation)
            }))
        .then_some(());
    }
    let static_body_evaluator =
        hook.is_some_and(|hook| static_body_evaluator_is_supported(schema, hook, roles));
    if !static_body_evaluator
        && (!(bodiless
            || matches!(
                hook,
                Some(
                    LoweringHookId::If
                        | LoweringHookId::Switch
                        | LoweringHookId::While
                        | LoweringHookId::For
                        | LoweringHookId::Foreach
                        | LoweringHookId::Lmap
                        | LoweringHookId::Catch
                        | LoweringHookId::Try
                )
            ))
            || schema.semantics.frame_effect.is_some()
            || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
            || schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current)
    {
        return None;
    }
    for &(argument, role) in roles {
        if role == tcl_registry::ArgRole::Body
            && schema.authored_source_script_timing_at(argument)?
                != tcl_registry::ScriptTiming::SameInvocation
        {
            return None;
        }
    }
    Some(())
}

/// A selected builtin that runs its body word exactly as written: `eval` in
/// the current frame, `uplevel` in the frame its level word names, and the
/// loops `dict for`/`dict map`, `array for` and `foreachLine`. The lowerer
/// still declines a body or level word it cannot read statically.
pub(super) fn static_body_evaluator_is_supported(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    hook: tcl_registry::hooks::LoweringHookId,
    roles: &[(usize, tcl_registry::ArgRole)],
) -> bool {
    use tcl_registry::frame_effect::FrameArgLayout;
    use tcl_registry::hooks::LoweringHookId;
    let semantics = &schema.semantics;
    let layout = semantics.frame_effect.map(|effect| effect.layout);
    let plain = semantics.body_kind == tcl_registry::BodyKind::Plain;
    let loop_body = roles
        .iter()
        .any(|&(_, role)| role == tcl_registry::ArgRole::LoopVarList);
    semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
        && match hook {
            LoweringHookId::Eval => plain && layout == Some(FrameArgLayout::ScriptInCurrentFrame),
            LoweringHookId::Uplevel => layout == Some(FrameArgLayout::ScriptInSelectedFrame),
            LoweringHookId::Dict | LoweringHookId::ArrayFor => {
                plain && layout.is_none() && loop_body
            }
            LoweringHookId::ForeachLine => plain && layout.is_none(),
            _ => false,
        }
}

pub(super) fn original_structured_operands_are_supported(
    registry: &CommandRegistry,
    selected: &ResolvedStatementInvocation,
    source_roles: Option<&StructuredSourceRoles>,
    tokens: &CommandTokens,
) -> Option<()> {
    let result = original_structured_operands_are_supported_untraced(
        registry,
        selected,
        source_roles,
        tokens,
    );
    #[cfg(debug_assertions)]
    trace_jim_structure_fields("operands", selected, source_roles);
    crate::registry_invocation::source_structure::trace_jim_source_option(
        "jim.body.operands_supported",
        result,
    )
}

fn original_structured_operands_are_supported_untraced(
    registry: &CommandRegistry,
    selected: &ResolvedStatementInvocation,
    source_roles: Option<&StructuredSourceRoles>,
    tokens: &CommandTokens,
) -> Option<()> {
    // A possible case body is readonly advice. Unreadable option selection
    // retains generic IR instead of borrowing a definitive Switch topology.
    if source_roles.is_some_and(StructuredSourceRoles::selection_unknown) {
        #[cfg(test)]
        trace_original_structure("source_roles", tokens, false);
        host_structure_observer::flag("operands.source_roles", false);
        return None;
    }
    // Source-shape lowering requires each operand to retain its original slot.
    // Alias prefixes, selector rewriting and expanded elements use generic IR
    // until a lowerer explicitly accepts their composed operand projection.
    if (!selected.facts.arg_roles_complete && source_roles.is_none())
        || selected.facts.arity_accepts_frozen_arguments() != Some(true)
        || selected.effective.words.len() != tokens.words().len()
        || !selected
            .effective
            .origins
            .iter()
            .enumerate()
            .skip(1)
            .all(|(index, origin)| *origin == InvocationWordOrigin::Written(index))
    {
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            trace_original_structure(
                "roles_complete",
                tokens,
                selected.facts.arg_roles_complete || source_roles.is_some(),
            );
            trace_original_structure(
                "arity",
                tokens,
                selected.facts.arity_accepts_frozen_arguments() == Some(true),
            );
            trace_original_structure(
                "word_count",
                tokens,
                selected.effective.words.len() == tokens.words().len(),
            );
            trace_original_structure(
                "written_origins",
                tokens,
                selected
                    .effective
                    .origins
                    .iter()
                    .enumerate()
                    .skip(1)
                    .all(|(index, origin)| *origin == InvocationWordOrigin::Written(index)),
            );
        }
        host_structure_observer::flag("operands.shape", false);
        return None;
    }
    if selected.facts.successful_handler
        == Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::ExpressionArguments)
        && tokens
            .source_binding
            .as_ref()?
            .conditional_expression_evaluation(registry, tokens)
            .is_none_or(|expression| !expression.semantic_lookup_closed())
    {
        #[cfg(test)]
        trace_original_structure("expression_lookup", tokens, false);
        host_structure_observer::flag("operands.expression_lookup", false);
        return None;
    }
    #[cfg(test)]
    trace_original_structure("complete", tokens, true);
    Some(())
}

fn traced_original_source_purpose(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<crate::command_binding::OriginalInvocationSourcePurpose> {
    source_structure::trace_jim_source_option(
        "jim.body.purpose",
        host_structure_observer::option(
            "source.purpose",
            host_structure_observer::option("source.binding", tokens.source_binding.as_ref())?
                .original_source_advice_purpose(tokens, context, registry),
        ),
    )
    .inspect(|purpose| {
        use crate::command_binding::OriginalInvocationSourcePurpose as Purpose;
        for (stage, expected) in [
            ("purpose.execution", Purpose::Execution),
            ("purpose.logical", Purpose::Logical),
            ("purpose.standalone", Purpose::Standalone),
            ("purpose.authored_simulation", Purpose::AuthoredSimulation),
        ] {
            host_structure_observer::flag(stage, *purpose == expected);
        }
    })
}

/// Select source structure using the caller's complete immutable context.
/// Missing or foreign context refuses; this supplies no executed body/frame.
pub(crate) fn logical_structured_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: Option<&crate::command_binding::SourceCommandBindings>,
) -> Option<LogicalStructuredInvocation> {
    let source_purpose = traced_original_source_purpose(registry, context, tokens)?;
    let logical_source =
        source_purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) {
        trace_original_structure("registry", tokens, false);
        return None;
    }
    let context = if logical_source {
        source_structure::trace_jim_source_option(
            "jim.body.metadata",
            tokens
                .source_binding
                .as_ref()?
                .original_invocation_metadata_for_input(
                    tokens,
                    host_structure_observer::option(
                        "metadata.input",
                        context.source_analysis_input(),
                    )?,
                    registry,
                ),
        )?
    } else {
        context
    };
    if bindings.is_some_and(|bindings| bindings.original_arguments_rejected_before_handler(tokens))
    {
        trace_original_structure("argument_entry", tokens, false);
        return None;
    }
    let selected = original_structured_source_selection(registry, context, tokens, bindings)?;
    let source_roles = if logical_source {
        let binding = tokens.source_binding.as_ref()?;
        let realm = binding.original_source_realm_for_input(
            tokens,
            host_structure_observer::option("metadata.input", context.source_analysis_input())?,
            registry,
        )?;
        selected
            .with_metadata_schema(registry, context, realm, |schema| {
                schema.authored_logical_source_role_projection()
            })
            .map(StructuredSourceRoles::Logical)
    } else if !selected.facts.arg_roles_complete {
        // Case syntax retains its independently selected source purpose.
        // Native lookup requires its original compiler observation; authored
        // simulation retains its source namespace without a physical frame.
        let binding = tokens.source_binding.as_ref()?;
        let authored_simulation = source_purpose
            == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation;
        let lookup = if authored_simulation {
            binding
                .declaration_operand_layout_advice(tokens)
                .or_else(|| binding.original_compilation_lookup_advice(tokens))?
        } else {
            binding.original_compilation_lookup_advice(tokens)?
        };
        if if authored_simulation {
            !lookup.closed_authored_simulation_source_lookup(tokens, context, registry)
        } else {
            lookup.has_unclosed_source_coverage()
        } {
            return None;
        }
        match selected.authored_case_body_selection.as_ref()? {
            AuthoredSourceCaseBodySelection::Selected(roles)
                if !roles.selection_unknown
                    && roles
                        .bodies
                        .iter()
                        .all(|body| selected.argument_literal(body.argument).is_some()) =>
            {
                Some(if authored_simulation {
                    StructuredSourceRoles::AuthoredSimulationCase(roles.clone())
                } else {
                    StructuredSourceRoles::NativeCase(roles.clone())
                })
            }
            _ => None,
        }
    } else {
        None
    };
    if source_purpose == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    {
        authored_simulation_structure_is_supported(
            registry,
            context,
            tokens,
            &selected,
            source_roles.as_ref(),
        )?;
    }
    original_structured_operands_are_supported(registry, &selected, source_roles.as_ref(), tokens)?;
    Some(LogicalStructuredInvocation {
        invocation: selected,
        source_roles,
        source_purpose,
    })
}

/// Conditional structure of one unchanged original source invocation.
/// The shared issuer requires closed original lookup, complete roles, accepted
/// arity and direct written operand correspondence. It can describe an
/// unentered declaration body; it grants no Normal completion, native compiler
/// admission, physical frame or executed store.
#[must_use]
pub fn original_structured_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    logical_structured_invocation(registry, tokens, None).map(|selected| selected.invocation)
}

/// Select unchanged original source structure under the supplied metadata.
/// Complete source consumers validate their input with
/// [`InvocationMetadataContext::for_source_input`]. Missing ownership refuses;
/// direct operand correspondence grants no entered frame or Normal completion.
#[must_use]
pub fn original_structured_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    logical_structured_invocation_with_metadata_context(registry, context, tokens, None)
        .map(|selected| selected.invocation)
}

/// Original declaration grammar, without a completed handler or CPP receipt.
pub(super) fn original_declared_structured_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let context = context.into();
    let source_purpose = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?;
    let logical_source =
        source_purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) {
        trace_original_structure("declaration_registry", tokens, false);
        return None;
    }
    let Some(binding) = tokens.source_binding.as_ref() else {
        trace_original_structure("declaration_binding", tokens, false);
        return None;
    };
    let context = if logical_source {
        binding.original_invocation_metadata_for_input(
            tokens,
            host_structure_observer::option("metadata.input", context.source_analysis_input())?,
            registry,
        )?
    } else {
        context
    };
    if let Some(input) = context.source_analysis_input()
        && host_structure_observer::option(
            "declaration.lexer_config",
            binding.original_lexer_config_for_tokens(tokens),
        )?
        .normalized()
            != input.lexer_config().normalized()
    {
        trace_original_structure("declaration_config", tokens, false);
        return None;
    }
    let advice = if logical_source {
        binding.original_conditional_operand_layout_advice(tokens, registry)
    } else if source_purpose
        == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    {
        binding
            .declaration_operand_layout_advice(tokens)
            .or_else(|| binding.original_compilation_lookup_advice(tokens))
    } else {
        binding
            .declaration_operand_layout_advice(tokens)
            .or_else(|| {
                binding
                    .original_compilation_lookup_advice(tokens)
                    .filter(|advice| !advice.has_unclosed_source_coverage())
            })
    };
    trace_original_structure("declaration_advice", tokens, advice.is_some());
    let advice = advice?;
    // This is conditional source structure. A positively retained Logical
    // command world needs no physical variable frame; Native lookup keeps its
    // independent frame gate and cannot borrow that source-only closure.
    let closed = if logical_source {
        advice.closed_logical_source_lookup()
    } else if source_purpose
        == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    {
        advice.closed_authored_simulation_source_lookup(tokens, context, registry)
    } else {
        advice.closed_lookup()
    };
    trace_original_structure("declaration_closed", tokens, closed);
    if !closed {
        return None;
    }
    let selected = resolve_original_declared_layout(registry, context, tokens, &advice);
    trace_original_structure("declaration_facts", tokens, selected.is_some());
    selected
}

/// Conditional source structure retained by a genuine Inline Control compiler.
/// This purpose does not repair an unavailable runtime declaration or handler.
fn original_inline_control_structured_invocation(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let source_purpose = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?;
    let logical_source =
        source_purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) || logical_source {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_inline_control_metadata_for_input(
        tokens,
        host_structure_observer::option("metadata.input", context.source_analysis_input())?,
        registry,
    )?;
    let preparation = binding.original_inline_control_compilation(tokens)?;
    let advice = binding.original_compilation_lookup_advice(tokens)?;
    if !advice.closed_lookup()
        || advice.has_opaque_handler_alternatives()
        || advice.namespace() != &preparation.dependency().namespace_key
        || !advice
            .targets()
            .iter()
            .all(|target| target == &preparation.dependency().target)
    {
        return None;
    }
    resolve_original_declared_layout(registry, metadata, tokens, &advice)
}
