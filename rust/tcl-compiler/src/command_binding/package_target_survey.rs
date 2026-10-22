// SPDX-License-Identifier: AGPL-3.0-or-later
//! Existing Logical source-point history for an explicitly hypothetical target survey,
//! and the survey-scoped Logical conditional source control that produces it.
use super::*;

/// Immutable source-model premise, never a package database/provision result.
#[derive(Debug, Clone)]
pub(crate) struct OriginalPackageIndexSourceModel {
    origin: Arc<SourceOriginId>,
    input: crate::analyser::ResolvedAnalysisInput,
    _words: Arc<[tcl_lexer::NativeWord]>,
    namespaces: Vec<SourceNamespaceKey>,
    subject: Option<String>,
    _advice: Arc<OriginalSourceCommandTransitionAdvice>,
    _layouts: Vec<Arc<OriginalCompilationLookupAdvice>>,
}

impl OriginalPackageIndexSourceModel {
    pub(crate) fn same_source_context(&self, parent: &Self) -> bool {
        self.origin == parent.origin
            && self.input == parent.input
            && self.subject == parent.subject
            && self.namespaces == parent.namespaces
    }
}

impl SourceCommandBindings {
    /// Reborrow actual Logical dispatch points and original whole vectors.
    /// This models the declared initial source world only, with all prior
    /// opaque/source/package effects retained. No Native acceptance is changed.
    pub(crate) fn original_package_index_model_at(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        command: &tcl_lexer::NativeScriptCommandWords,
        subject: Option<&str>,
        advice: &Arc<OriginalSourceCommandTransitionAdvice>,
    ) -> Option<OriginalPackageIndexSourceModel> {
        if self.original_logical_source_name_advice_input() != Some(input)
            || !input.has_logical_source_name_context()
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let origin = self.root_origin.as_ref()?;
        let offset = command.words.first()?.span().start();
        if advice.logical_source_input() != Some(input)
            || !advice.matches_source(image, input.lexer_config())
            || !advice.matches_context(input.borrowed_context_registry())
            || advice.original_words() != command.words.as_slice()
            || advice.original_head().logical_input().is_none()
            || advice
                .obligations()
                .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
            || !advice.uncertain_operations().is_empty()
        {
            return None;
        }
        let source = image.try_text().ok()?;
        let segments = crate::segmenter::segment_commands_with_offset_and_config(
            source.get(command.span.as_range())?,
            command.span.start(),
            input.lexer_config().nested(),
        );
        let [segment] = segments.as_slice() else {
            return None;
        };
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(image),
            input.lexer_config(),
            segment,
        );
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset,
        };
        let registry = input.borrowed_context_registry().commands();
        let registry_key = registry.snapshot().semantic_key();
        let points = self
            .dispatch_points_at(offset)
            .filter(|point| !point.declaration_preview)
            .collect::<Vec<_>>();
        if points.is_empty() {
            return None;
        }
        let mut namespaces = Vec::new();
        let mut layouts = Vec::new();
        for point in points {
            let state = &point.state;
            let binding = point.original_dispatch_binding();
            // Existing shared Logical layout projection from this exact point,
            // not a Native declaration row or a fabricated observation.
            let layout = original_site_operand_layout_advice(
                &site,
                &tokens,
                binding.lookup_state.as_ref()?,
                &point.namespace_key,
                input.lexer_config(),
            )?;
            if !layout.closed_logical_source_lookup()
                || layout.targets() != binding.targets.as_slice()
            {
                return None;
            }

            if binding.original_logical_prefix_source_input() != Some(input)
                || state.current_source_origin.as_ref() != Some(origin)
                || state.baseline.registry_snapshot.as_ref() != Some(&registry_key)
                || state.has_opaque_domain()
                || state.source_step_observed()
                || state.opaque_binding_mutation
                || state.unnameable_rebinding_subject
                || !state.command_observers.is_quiet()
                || state.loader_handler_unknown
                || state.tainted_provider_state.contains("*")
                || binding.targets.is_empty()
                || binding.targets.iter().any(|target| {
                    !target.registry_backed
                        || !target.prepended.is_empty()
                        || target.implementation_generation != 0
                        || !tcl_registry::command_descriptor_labels_match(
                            &target.command,
                            advice.command(),
                        )
                        || state.source_execution_observed(target.identity.as_ref())
                        || state.runtime_execution_observed(target.identity.as_ref())
                })
            {
                return None;
            }
            if let Some(subject) = subject
                && (state.packages.contains_key(subject)
                    || state.revoked_loaders.contains(subject)
                    || state.tainted_provider_state.contains(subject))
            {
                return None;
            }
            namespaces.push(point.namespace_key.clone());
            layouts.push(Arc::new(layout));
        }
        namespaces.sort();
        namespaces.dedup();
        Some(OriginalPackageIndexSourceModel {
            origin: Arc::clone(origin),
            input: input.clone(),
            _words: Arc::from(command.words.clone()),
            namespaces: namespaces,
            subject: subject.map(str::to_owned),
            _advice: Arc::clone(advice),
            _layouts: layouts,
        })
    }
}

impl SourceCommandBindings {
    /// Survey-scoped Logical conditional source control. Only an
    /// interpretation built by `analyse_package_index_survey` reaches it, and
    /// only for an invocation without compile error or unknown compiler.
    ///
    /// A selected stock first-match conditional with a complete clause plan
    /// enters its condition and body operands as C Tcl evaluates them, one
    /// body level deeper under the shared source nesting cap. A selected stock
    /// operation whose descriptor is closed and quiet
    /// (`logical_survey_quiet_operation`) continues with its own completion
    /// route and an unchanged source model. Anything else declines to the
    /// ordinary Logical path.
    ///
    /// This frame stays on the recursive source walk: descriptor resolution,
    /// flow selection and outcome construction run in nonrecursive leaves, and
    /// only their heap results are live during descent.
    #[inline(never)]
    pub(super) fn walk_logical_survey_command(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        compiled: &compiled_invocation::CompiledInvocationSelection,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Option<Box<SourceOutcomes>> {
        if !self.logical_conditional_source_control
            || self.declaration_preview_depth != 0
            || compiled.compile_error
            || compiled.unknown
        {
            return None;
        }
        let target = logical_survey_target(words, segment.span.start(), state, *context)?;
        let dispatch = logical_survey_dispatch(&target, effective, state, *context)?;
        let Some(branches) = &dispatch.branches else {
            return Some(logical_survey_quiet_outcome(state, dispatch.prepared.route));
        };
        // The bodies of a current-frame conditional run one level deeper,
        // exactly as the Native selected-body walk counts them.
        let body_context = source_body_context_boxed(context, context.namespace, context.frame);
        let invocation = dispatch.prepared.invocation_boxed(&target.command);
        let operands = SourceScriptOperands {
            compilation_spec: dispatch.native_compilation.as_ref(),
            compilation_selection:
                &tcl_registry::native_compilation::NativeCompilationSelection::Generic,
            words,
            written_arguments: context.written_arguments,
            target: &target,
            arguments: invocation.arguments_ref(),
        };
        Some(Box::new(self.walk_conditional(
            branches,
            operands,
            state,
            *body_context,
        )))
    }
}

/// The selected survey operation, resolved outside the recursive frame.
/// `branches` is `Some` for a complete first-match conditional and `None`
/// for a closed, quiet continuation.
struct LogicalSurveyDispatch<'a> {
    prepared: Box<PreparedSourceNativeInvocation<'a>>,
    native_compilation: Option<tcl_registry::native_compilation::NativeCompilationSpec>,
    branches: Option<Vec<(Option<usize>, usize)>>,
}

/// The singleton stock target an original Logical invocation selects, with
/// no captured prefix. Unknown, possibly absent, procedure, alias and
/// redefined selections decline.
#[inline(never)]
fn logical_survey_target(
    words: &[crate::ir::WordExpr],
    offset: u32,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Box<SourceCommandTarget>> {
    let original =
        super::logical_definition::original_invocation_words(words, offset, state, context)?;
    let head = super::logical_definition::static_value(original.first()?)?;
    let selected = source_binding_projection(state, &head, &context.namespace_identity());
    if selected.unknown || selected.may_be_absent {
        return None;
    }
    let [target] = selected.targets.as_slice() else {
        return None;
    };
    (target.registry_backed
        && target.prepended.is_empty()
        && matches!(target.kind, BindingKind::Builtin))
    .then(|| Box::new(target.clone()))
}

/// Resolve the ordinary (not success-edge) facts and the body flow of one
/// selected survey invocation. A conditional flow needs the selected clause
/// grammar's complete plan: C Tcl checks the whole `if` chain before it runs
/// any clause, so a malformed chain enters no body here either.
#[inline(never)]
fn logical_survey_dispatch<'a>(
    target: &'a SourceCommandTarget,
    effective: &'a [crate::registry_invocation::EffectiveInvocationWord],
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Box<LogicalSurveyDispatch<'a>>> {
    let prepared =
        prepare_source_native_invocation(target, effective, state, context.registry, context.realm)
            .ok()?;
    let (native_compilation, branches) = {
        let invocation = prepared.invocation(&target.command);
        let resolution = tcl_registry::model::semantic::resolve_structured_invocation_in_realm(
            context.registry,
            context
                .registry
                .profile()
                .map(tcl_registry::model::semantic::SemanticContext::for_profile),
            invocation,
            context.realm,
        );
        let schema = resolution.resolved()?;
        let facts = schema.facts();
        if facts.frame_effect.is_some() || !facts.arg_roles_complete {
            return None;
        }
        let branches = match tcl_registry::case_bodies::script_body_flow_in_registry(
            context.registry,
            &facts,
            invocation.arguments(),
        ) {
            tcl_registry::script_body_flow::ScriptBodyFlow::Conditional(branches) => {
                schema.clause_plan()?.defect.is_none().then_some(())?;
                Some(branches)
            }
            _ => {
                logical_survey_quiet_operation(&facts).then_some(())?;
                None
            }
        };
        (facts.native_compilation, branches)
    };
    Some(Box::new(LogicalSurveyDispatch {
        prepared,
        native_compilation,
        branches,
    }))
}

#[inline(never)]
fn logical_survey_quiet_outcome(
    state: &ModuleCommandBindings,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
) -> Box<SourceOutcomes> {
    Box::new(SourceOutcomes::invocation(state, route))
}

/// A closed, quiet selected descriptor: declared empty transitions, no
/// callback, re-entry, frame or body execution, no variable or script
/// operand, and only current-interpreter reads besides the invocation's own
/// result and completion state, which its completion route models.
fn logical_survey_quiet_operation(facts: &tcl_registry::InvocationFacts) -> bool {
    use tcl_registry::world_effect::{
        CallbackKinds, EffectAccessMode, InterpreterScope, Reentrancy, WorldStateDomain,
    };
    let callback = facts.effects.callback();
    callback.kinds == CallbackKinds::NONE
        && callback.reentrancy == Reentrancy::Never
        && facts.frame_effect.is_none()
        && facts.body_execution.is_none()
        && facts.arg_roles_complete
        && !facts
            .arg_roles
            .iter()
            .any(|(_, role)| role.carries_script() || role.names_variable())
        && facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| transitions.facts().is_empty())
        && facts.effects.accesses().iter().all(|access| {
            access.interpreter == InterpreterScope::Current
                && match access.domain {
                    WorldStateDomain::InterpreterResult | WorldStateDomain::CompletionState => {
                        access.mode != EffectAccessMode::Clobber
                    }
                    _ => access.mode == EffectAccessMode::Read,
                }
        })
}
