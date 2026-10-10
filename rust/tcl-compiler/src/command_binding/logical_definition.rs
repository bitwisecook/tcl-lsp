// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A selected Logical procedure definition changes only the authored model.

use super::{
    Arc, BindingKind, CommandAllocationSite, MayBinding, ModuleCommandBindings,
    ResolvedCommandTarget, SourceCommandKey, SourceCommandTarget, SourceExecutionContext,
    SourceNamespaceKey, SourceOutcomes, source_binding_projection,
};
use crate::analyser::ResolvedAnalysisInput;
use tcl_lexer::{NativeWord, SourceChannel};

struct LogicalProcedureDefinition {
    input: ResolvedAnalysisInput,
    site: CommandAllocationSite,
    words: Vec<NativeWord>,
    namespace: SourceNamespaceKey,
    target: SourceCommandTarget,
    key: SourceCommandKey,
}

pub(super) fn transfer(
    words: &[crate::ir::WordExpr],
    target: &SourceCommandTarget,
    offset: u32,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<SourceOutcomes> {
    // naming.source.logical-procedure-definition-model
    // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
    if let Some(definition) =
        LogicalProcedureDefinition::capture(words, target, offset, state, context)
    {
        return definition.apply(state, context);
    }
    super::logical_operation::transfer(words, target, offset, state, context)
}

/// The prepared argv boundary may select an authored model handler without
/// a Native compiler or head input. Exact quiet source capture stays mandatory.
pub(super) fn transfer_selected(
    words: &[crate::ir::WordExpr],
    offset: u32,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<SourceOutcomes> {
    // naming.source.logical-procedure-definition-model
    // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
    let original = original_static_invocation_words(words, offset, state, context)?;
    let head = static_value(original.first()?)?;
    let selected = source_binding_projection(state, &head, &context.namespace_identity());
    if selected.unknown || selected.may_be_absent {
        return None;
    }
    let [target] = selected.targets.as_slice() else {
        return None;
    };
    transfer(words, target, offset, state, context)
}

impl LogicalProcedureDefinition {
    fn capture(
        words: &[crate::ir::WordExpr],
        target: &SourceCommandTarget,
        offset: u32,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let input = logical_entry(state, context)?;
        let origin = Arc::clone(state.current_source_origin.as_ref()?);
        if origin.source_image().channel() != SourceChannel::Document
            || offset != context.invocation_offset
        {
            return None;
        }
        let original = original_static_invocation_words(words, offset, state, context)?;
        let values = original
            .iter()
            .map(static_value)
            .collect::<Option<Vec<_>>>()?;
        let head = values.first()?;
        let namespace = context.namespace_identity();
        if !selected_target(state, head, &namespace, target) {
            return None;
        }
        let actual_context = input.context_registry();
        let mut effective_values = vec![target.command.clone()];
        effective_values.extend(
            target
                .prepended
                .iter()
                .map(|word| word.as_registry_word().literal().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?,
        );
        effective_values.extend(values.into_iter().skip(1));
        let arguments = effective_values
            .iter()
            .skip(1)
            .map(|value| tcl_registry::InvocationWord::Literal(value))
            .collect::<Vec<_>>();
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                actual_context.commands(),
                Some(actual_context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&target.command),
                    &arguments,
                )
                .with_dialect(super::source_analysis_entry::source_input_dialect(input)),
                context.realm,
            );
        let schema = resolution.resolved()?;
        let shape = schema.authored_source_procedure_arguments()?;
        static_formals(
            effective_values.get(shape.parameters + 1)?,
            context.config.list_parse,
        )?;
        let key = source_procedure_key(state, &namespace, effective_values.get(shape.name + 1)?)?;
        Some(Self {
            input: input.clone(),
            site: CommandAllocationSite {
                source: origin,
                offset,
            },
            words: original,
            namespace,
            target: target.clone(),
            key,
        })
    }

    fn apply(
        self,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        if logical_entry(state, context) != Some(&self.input)
            || state.current_source_origin.as_ref() != Some(&self.site.source)
            || context.invocation_offset != self.site.offset
            || context.namespace_identity() != self.namespace
            || self.words.iter().any(|word| {
                word.image() != self.site.source.source_image() || word.config() != context.config
            })
            || !selected_target(
                state,
                &static_value(self.words.first()?)?,
                &self.namespace,
                &self.target,
            )
            || !state.namespaces.contains(self.key.holder().as_ref())
        {
            return None;
        }
        let label = self.key.authored_spelling()?.to_owned();
        state.extend_procedure_bodies([label.clone()]);
        state.record_proc_rebound_candidates(
            &label,
            &crate::ir_helpers::ExecutionNamespace::SourceContext(self.namespace),
        );
        state.install(
            self.key,
            MayBinding::Target(ResolvedCommandTarget {
                command: label,
                prepended: Vec::new(),
                registry_backed: false,
                kind: BindingKind::Proc,
                implementation_generation: self.site.offset,
                implementation_allocation: None,
                terminal: true,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: None,
            }),
        );
        // This is a continuation of the positively selected authored model.
        // It carries neither a Native publication nor a Normal certificate.
        Some(SourceOutcomes::normal(state))
    }
}

pub(super) fn logical_entry<'a>(
    state: &'a ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<&'a ResolvedAnalysisInput> {
    let input = state.logical_source_name_advice_input()?;
    if !input.has_logical_source_name_context()
        || input.lexer_config() != context.config
        || state.baseline.native_entry.is_some()
        || state.baseline.unknown_entry
        || state.baseline.execution_name_policy.is_some()
        || state.baseline.hosted_execution_context.is_some()
        || state.has_opaque_domain()
        || !state.command_observers.is_quiet()
        || !state.source_variables.namespace_known
        || !matches!(
            context.namespace_identity(),
            SourceNamespaceKey::Authored(_)
        )
        || !state.namespaces.contains(&context.namespace_identity())
        || input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            != context.registry.snapshot().semantic_key()
        || state.baseline.dialect != Some(super::source_analysis_entry::source_input_dialect(input))
    {
        return None;
    }
    Some(input)
}

fn selected_target(
    state: &ModuleCommandBindings,
    head: &str,
    namespace: &SourceNamespaceKey,
    target: &SourceCommandTarget,
) -> bool {
    if !matches!(target.kind, BindingKind::Builtin | BindingKind::Alias) || !target.registry_backed
    {
        return false;
    }
    let selected = source_binding_projection(state, head, namespace);
    !selected.unknown
        && !selected.may_be_absent
        && selected.targets.as_slice() == std::slice::from_ref(target)
}

/// Capture one complete quiet original vector in the retained Logical domain.
/// Authored model operands grant neither Native values nor execution receipts.
pub(super) fn original_static_invocation_words(
    words: &[crate::ir::WordExpr],
    offset: u32,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Vec<NativeWord>> {
    logical_entry(state, context)?;
    let origin = state.current_source_origin.as_ref()?;
    if origin.source_image().channel() != SourceChannel::Document
        || !matches!(origin.kind(), super::SourceOriginKind::Authored(_))
        || offset != context.invocation_offset
        || state.variable_frame != *context.frame
        || state.source_variables.namespace != context.namespace
    {
        return None;
    }
    let original = crate::registry_invocation::original_native_compiler_words(
        origin.source_image(),
        words,
        offset,
        context.config,
    )?;
    original
        .iter()
        .all(|word| static_value(word).is_some())
        .then_some(original)
}

pub(super) fn static_value(word: &NativeWord) -> Option<String> {
    let bytes = tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?;
    if bytes.contains(&0) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

pub(super) fn static_formals(
    text: &str,
    syntax: tcl_dialect::ListParse,
) -> Option<Vec<tcl_syntax::formal_params::FormalParameter>> {
    // Proof: naming.source.logical-procedure-definition-model
    // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
    // The source model uses shared strict formal validity. The selected list
    // parser must agree at both levels; this is no Native parameter recipe.
    let parameters = tcl_syntax::formal_params::parse_formal_parameters(text).ok()?;
    matching_list_elements(text, syntax)?;
    for specifier in tcl_syntax::list::split_list(text).ok()? {
        matching_list_elements(&specifier, syntax)?;
    }
    Some(parameters)
}

/// Construct a conditional authored declaration slot in its retained namespace.
/// This does not invoke the native publication recipe or allocate a command.
pub(super) fn source_procedure_key(
    state: &ModuleCommandBindings,
    namespace: &SourceNamespaceKey,
    name: &str,
) -> Option<SourceCommandKey> {
    state.logical_source_name_advice_input()?;
    let SourceNamespaceKey::Authored(current) = namespace else {
        return None;
    };
    let key = SourceCommandKey::authored(tcl_syntax::naming::qualify(current, name));
    let (_, tail) = tcl_syntax::naming::key_holder_and_tail(key.authored_spelling()?);
    (!tail.is_empty() && state.namespaces.contains(key.holder().as_ref())).then_some(key)
}

fn matching_list_elements(text: &str, syntax: tcl_dialect::ListParse) -> Option<()> {
    let mut strict_next = 0;
    let mut selected_next = 0;
    loop {
        let strict = tcl_syntax::list::find_element(text, strict_next).ok()?;
        let selected =
            tcl_syntax::list::find_element_with_syntax(text, selected_next, syntax).ok()?;
        match (strict, selected) {
            (None, None) => return Some(()),
            (Some(strict), Some(selected))
                if strict.value == selected.value
                    && strict.literal == selected.literal
                    && strict.braced == selected.braced =>
            {
                strict_next = strict.next;
                selected_next = selected.next;
            }
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandSlotPresence};

    fn input() -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "logical-definition-model",
            &[],
            "Logical definition model",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )
        .intern();
        ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        )
    }

    fn final_presence(source: &str, input: &ResolvedAnalysisInput) -> SourceCommandSlotPresence {
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, input.analyser_profile().name);
        let offset = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .last()
        .unwrap()
        .span
        .start();
        let binding = analysis
            .retained_command_realm()
            .unwrap()
            .invocation_at_source("", offset);
        assert_eq!(binding.logical_source_name_advice_input(), Some(input));
        assert!(binding.original_recorded_head_name_input().is_none());
        binding.selected_slot_diagnostic_presence()
    }

    #[test]
    fn logical_definition_publishes_general_static_procedures_in_the_authored_model() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let input = input();
        for source in [
            "proc defined {} {}; defined",
            "proc defined {left {right default} args} {return $left}; defined",
            "proc first {value} {return $value}; proc second {a b} {return \"$a$b\"}; second",
            "proc ::defined {value} {not a valid body yet [}; ::defined",
        ] {
            assert_eq!(
                final_presence(source, &input),
                SourceCommandSlotPresence::Present,
                "{source}"
            );
        }
    }

    #[test]
    fn logical_definition_declines_unknown_rebound_dynamic_and_invalid_inputs() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let input = input();
        for source in [
            "mystery; proc defined {} {}; defined",
            "proc proc args {}; proc defined {} {}; defined",
            "proc $name {} {}; defined",
            "proc defined $formals {}; defined",
            "proc defined {} $body; defined",
            "proc defined {{value default extra}} {}; defined",
            "proc defined {{array(index)}} {}; defined",
            "proc missing::defined {} {}; missing::defined",
            "trace add execution proc enter observer; proc defined {} {}; defined",
        ] {
            assert_ne!(
                final_presence(source, &input),
                SourceCommandSlotPresence::Present,
                "{source}"
            );
        }
    }

    #[test]
    fn logical_model_roster_requires_the_complete_current_input_and_source_mode() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let input = input();
        let context = input.context_registry();
        let registry = context.commands();
        let options = SourceAnalysisOptions {
            logical_source_input: Some(&input),
            invocation_dialect: Some(super::super::source_analysis_entry::source_input_dialect(
                &input,
            )),
            ..SourceAnalysisOptions::default()
        };
        let initial = |options, config| {
            ModuleCommandBindings::initial_with_options(registry, options, Some(config))
        };
        let state = initial(options, input.lexer_config());
        assert_eq!(state.logical_source_name_advice_input(), Some(&input));
        assert!(state.baseline.semantics.binding_names().contains("::proc"));
        assert_eq!(state.baseline.dialect, options.invocation_dialect);
        assert!(state.baseline.execution_name_policy.is_none());
        assert!(state.baseline.native_entry.is_none());
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let stale_config = initial(options, config);
        assert!(stale_config.logical_source_name_advice_input().is_none());
        assert!(stale_config.baseline.semantics.binding_names().is_empty());
        let foreign_context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let foreign_input = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            foreign_context,
            input.lexer_config(),
        );
        let foreign = initial(
            SourceAnalysisOptions {
                logical_source_input: Some(&foreign_input),
                ..options
            },
            input.lexer_config(),
        );
        assert!(foreign.logical_source_name_advice_input().is_none());
        assert!(foreign.baseline.semantics.binding_names().is_empty());
        let native_profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let native_input = ResolvedAnalysisInput::new(
            native_profile,
            native_profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(native_profile)),
        );
        let native_options = SourceAnalysisOptions {
            logical_source_input: Some(&native_input),
            invocation_dialect: Some(super::super::source_analysis_entry::source_input_dialect(
                &native_input,
            )),
            ..SourceAnalysisOptions::default()
        };
        let native = initial(native_options, native_input.lexer_config());
        assert!(native.logical_source_name_advice_input().is_none());
        let expected_native = registry.effective_semantics_for_dialect_in_realm(
            native_options.invocation_dialect.unwrap(),
            tcl_dialect::model::InvocationRealm::InterpreterRuntime,
        );
        assert_eq!(
            native.baseline.semantics.binding_names(),
            expected_native.binding_names()
        );
    }

    #[test]
    fn logical_definition_receipt_rechecks_source_configuration_and_entry_without_native_normal() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let input = input();
        let context_registry = input.context_registry();
        let registry = context_registry.commands();
        let options = SourceAnalysisOptions {
            logical_source_input: Some(&input),
            invocation_dialect: Some(super::super::source_analysis_entry::source_input_dialect(
                &input,
            )),
            ..SourceAnalysisOptions::default()
        };
        let source = tcl_lexer::SourceImage::document("proc defined {value} {return $value}");
        let origin = Arc::new(super::super::SourceOriginId::authored_image(source.clone()));
        let mut state = ModuleCommandBindings::initial_with_options(
            registry,
            options,
            Some(input.lexer_config()),
        );
        state.current_source_origin = Some(Arc::clone(&origin));
        let namespace = SourceNamespaceKey::authored("::");
        let frame = crate::var_resolve::VariableExecutionFrame::Global;
        let context = super::super::root_source_execution_context(
            &frame,
            "::",
            &namespace,
            input.lexer_config(),
            registry,
            options,
        );
        let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
            &source,
            0,
            context.config,
        )
        .unwrap();
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&source),
            context.config,
            &segments[0],
        );
        let selected = source_binding_projection(&state, "proc", &namespace);
        let target = selected
            .targets
            .first()
            .expect("sealed Logical stock definer");
        let proof = || {
            LogicalProcedureDefinition::capture(tokens.words(), target, 0, &state, context).unwrap()
        };
        let mut foreign_source = state.clone();
        foreign_source.current_source_origin =
            Some(Arc::new(super::super::SourceOriginId::authored_image(
                tcl_lexer::SourceImage::document("proc other {value} {return $value}"),
            )));
        assert!(proof().apply(&mut foreign_source, context).is_none());
        let mut changed_config = context.config;
        changed_config.strict_quoting = !changed_config.strict_quoting;
        assert!(
            proof()
                .apply(
                    &mut state.clone(),
                    SourceExecutionContext {
                        config: changed_config,
                        ..context
                    }
                )
                .is_none()
        );
        let mut unknown = state.clone();
        unknown.mark_opaque_binding_mutation();
        assert!(proof().apply(&mut unknown, context).is_none());
        let mut normal_state = state.clone();
        let outcomes = proof().apply(&mut normal_state, context).unwrap();
        assert!(outcomes.normal.is_some());
        assert!(outcomes.normal_completion.is_none());
        assert!(outcomes.native_normal_completion.is_none());
        assert_eq!(
            normal_state.original_command_world,
            state.original_command_world
        );
        assert!(
            matches!(normal_state.binding_alternatives(&SourceCommandKey::authored("::defined")).iter().next(), Some(MayBinding::Target(target)) if target.kind == BindingKind::Proc && !target.registry_backed)
        );
        let mut unsealed = options;
        unsealed.logical_source_input = None;
        let state = ModuleCommandBindings::initial_with_options(
            registry,
            unsealed,
            Some(input.lexer_config()),
        );
        assert!(
            LogicalProcedureDefinition::capture(tokens.words(), target, 0, &state, context)
                .is_none()
        );
    }
}
