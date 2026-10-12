// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original declaration lifecycle semantics without entered execution authority.

use super::{
    CommandAllocationSite, SourceBodyPhase, SourceBodyPhaseReachability, SourceCommandBindings,
};
use crate::ir::CommandTokens;

pub(crate) struct SourceScopedLifecycleAdvice {
    pub(crate) invocation: crate::registry_invocation::DeclarationLifecycleInvocation,
    pub(crate) loader: super::TrustedPackageLoader,
    site: CommandAllocationSite,
    frame: crate::var_resolve::VariableExecutionFrame,
    namespace: super::SourceNamespaceKey,
}

impl SourceCommandBindings {
    pub(crate) fn scoped_lifecycle_advice(
        tokens: &CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<SourceScopedLifecycleAdvice> {
        let binding = tokens.source_binding.as_ref()?;
        if binding.runtime_reachability != super::SourceRuntimeReachability::Conditional {
            return None;
        }
        let site = binding.invocation_site()?;
        crate::registry_invocation::native_compiler_replay_source(tokens, site)?;
        let observations = super::declaration_layout::original_declaration_layouts(
            binding.declaration_layout_observations.as_deref()?,
        )?;
        let first = observations.clone().next()?;
        let super::declaration_layout::OriginalDiagnosticFrameEntry::Body(entry) =
            first.entry.as_ref()
        else {
            return None;
        };
        let namespace = entry.namespace_context()?;
        if !entry.owns_invocation(binding)
            || observations.clone().any(|observation| {
                observation.entry != first.entry
                    || &observation.namespace != namespace
                    || !observation.entry.owns_source(&site.source, site.offset)
            })
        {
            return None;
        }
        let state = &binding.lookup_state.as_ref()?.state;
        if state.has_opaque_domain()
            || state.source_step_observed()
            || state.source_execution_observed(None)
            || binding.entered_execution_observer.observed()
        {
            return None;
        }
        let invocation =
            crate::registry_invocation::declaration_lifecycle_invocation(registry, tokens)?;
        let provider = invocation.contract.provider;
        let loader = state.loaded_provider(provider.package)?;
        if loader.implementation_id != provider.implementation_id
            || !state.provider_surface_is_live(loader)
        {
            return None;
        }
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(
            &tcl_lexer::LexerConfig::from_grammar(invocation.dialect.lexer_grammar),
        );
        let values = invocation.arguments.get(invocation.argument_offset..)?;
        if values
            .get(invocation.contract.leading_arguments)
            .and_then(Option::as_deref)
            .is_some_and(|first| first.starts_with('-'))
        {
            for index in (invocation.contract.leading_arguments..values.len()).step_by(2) {
                let option = values[index].as_deref()?;
                if !invocation
                    .contract
                    .supports_option(loader.version.as_deref(), option)
                    || (invocation.contract.list_valued_options.contains(&option)
                        && values
                            .get(index + 1)
                            .and_then(Option::as_deref)
                            .is_none_or(|value| rules.split_list(value).is_err()))
                {
                    return None;
                }
            }
        }
        let hooks = invocation
            .contract
            .required_absent_hooks(loader.version.as_deref());
        if !hooks
            .iter()
            .all(|hook| state.definitely_absent(hook, namespace))
        {
            return None;
        }
        Some(SourceScopedLifecycleAdvice {
            invocation,
            loader: loader.clone(),
            site: site.clone(),
            frame: entry.frame().clone(),
            namespace: namespace.clone(),
        })
    }
}

pub(crate) enum ScopedLifecycleBodySource {
    Absent,
    Source(super::ExecutedScriptSource, super::SourceNamespaceKey),
}

impl ScopedLifecycleBodySource {
    pub(crate) fn into_source(
        self,
    ) -> Option<(super::ExecutedScriptSource, super::SourceNamespaceKey)> {
        match self {
            Self::Absent => None,
            Self::Source(source, namespace) => Some((source, namespace)),
        }
    }
}

impl SourceScopedLifecycleAdvice {
    pub(crate) fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.frame
    }

    pub(crate) fn reachability(
        &self,
        bindings: &SourceCommandBindings,
        phase: SourceBodyPhase,
    ) -> SourceBodyPhaseReachability {
        let Some(coverage) = bindings.lifecycle_coverage.get(&self.site) else {
            return SourceBodyPhaseReachability::Unknown;
        };
        let entries = coverage
            .iter()
            .filter(|entry| {
                entry.declaration_preview
                    && entry.frame == self.frame
                    && entry.namespace == self.namespace
            })
            .collect::<Vec<_>>();
        if entries.is_empty() || entries.iter().any(|entry| entry.entered.is_none()) {
            return SourceBodyPhaseReachability::Unknown;
        }
        let index = match phase {
            SourceBodyPhase::Setup => 0,
            SourceBodyPhase::Body => 1,
            SourceBodyPhase::Cleanup => 2,
        };
        if entries
            .iter()
            .filter_map(|entry| entry.entered)
            .any(|entered| entered[index])
        {
            SourceBodyPhaseReachability::MayEntered
        } else {
            SourceBodyPhaseReachability::NotEntered
        }
    }

    fn points<'a>(
        &'a self,
        bindings: &'a SourceCommandBindings,
        phase: SourceBodyPhase,
    ) -> impl Iterator<Item = &'a super::SourceBindingPoint> {
        bindings
            .phases
            .iter()
            .filter(move |point| {
                point.site == self.site.offset
                    && point.phase == phase
                    && point.point.declaration_preview
                    && point.point.state.current_source_origin.as_ref() == Some(&self.site.source)
                    && point.point.state.variable_frame == self.frame
                    && point.point.namespace_key == self.namespace
            })
            .map(|point| &point.point)
    }

    pub(crate) fn phase_binding(
        &self,
        bindings: &SourceCommandBindings,
        phase: SourceBodyPhase,
        command: &str,
    ) -> Option<super::SourceInvocationBinding> {
        let points = self.points(bindings, phase).collect::<Vec<_>>();
        let hooks = self
            .invocation
            .contract
            .required_absent_hooks(self.loader.version.as_deref());
        if points.is_empty()
            || points.iter().any(|point| {
                point.state.has_opaque_domain()
                    || !point.state.provider_surface_is_live(&self.loader)
                    || point.state.source_step_observed()
                    || point.state.source_execution_observed(None)
                    || !hooks
                        .iter()
                        .all(|hook| point.state.definitely_absent(hook, &point.namespace_key))
            })
        {
            return None;
        }
        // These are independently validated original declaration phase
        // points, not a command dispatch. Retain the conditional purpose
        // without granting argument completion or an entered target.
        let mut binding = SourceCommandBindings::query_points(command, points.into_iter());
        binding.runtime_reachability = super::SourceRuntimeReachability::Conditional;
        Some(binding)
    }

    pub(crate) fn body_source(
        &self,
        bindings: &SourceCommandBindings,
        operand: Option<tcl_registry::body_execution::BodyOperand>,
    ) -> Option<ScopedLifecycleBodySource> {
        let Some(operand) = operand else {
            return Some(ScopedLifecycleBodySource::Absent);
        };
        if operand.list_element.is_some() {
            return None;
        }
        let argument = operand
            .argument
            .checked_add(self.invocation.argument_offset)?;
        let observations = bindings.entered_scripts.get(&self.site)?.get(&argument)?;
        let mut selected = None;
        for observation in observations {
            if observation.frame.as_ref() != Some(&self.frame) {
                continue;
            }
            if selected
                .as_ref()
                .is_some_and(|source| source != &observation.source)
            {
                return None;
            }
            selected = Some(observation.source.clone());
        }
        Some(ScopedLifecycleBodySource::Source(
            selected?,
            self.namespace.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inventory(
        suffix: &str,
    ) -> (
        SourceCommandBindings,
        CommandTokens,
        tcl_registry::CommandRegistry,
    ) {
        inventory_with_provider(suffix, true)
    }

    fn inventory_with_provider(
        suffix: &str,
        retain_provider: bool,
    ) -> (
        SourceCommandBindings,
        CommandTokens,
        tcl_registry::CommandRegistry,
    ) {
        let registry = tcl_registry::CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let loader = crate::lowering::stock_body_provider_loader(
            tcl_registry::body_execution::TCLTEST_STOCK_PROVIDER,
            None,
        )
        .unwrap();
        let wrapper = "tcltest::test n d -setup {set x 1} -body {puts $x} -cleanup {unset x}";
        let source = format!("package require tcltest\nproc p {{}} {{{wrapper}}}{suffix}");
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let bindings = SourceCommandBindings::analyse_with_options(
            &source,
            config,
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile),
                trusted_package_loaders: if retain_provider {
                    std::slice::from_ref(&loader)
                } else {
                    &[]
                },
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find(wrapper).unwrap()).unwrap();
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(wrapper, offset, config)
                .remove(0);
        let mut tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceImage::document(&source).source_map(),
            config,
            &command,
        );
        bindings.stamp_original_tokens(&mut tokens);
        (bindings, tokens, registry)
    }

    #[test]
    fn scoped_lifecycle_coverage_keeps_declared_issuer_frame_and_original_source() {
        let (bindings, tokens, registry) = inventory("");
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.proved_execution_target().is_none());
        let advice = SourceCommandBindings::scoped_lifecycle_advice(&tokens, &registry)
            .expect("original scoped provider");
        for phase in [
            SourceBodyPhase::Setup,
            SourceBodyPhase::Body,
            SourceBodyPhase::Cleanup,
        ] {
            assert_eq!(
                advice.reachability(&bindings, phase),
                SourceBodyPhaseReachability::MayEntered
            );
            assert_eq!(
                bindings.lifecycle_phase_reachability_at(binding.invocation_site().unwrap(), phase),
                SourceBodyPhaseReachability::Unknown
            );
            let phase_binding = advice
                .phase_binding(&bindings, phase, &advice.invocation.command)
                .unwrap();
            assert_eq!(
                phase_binding.runtime_reachability(),
                super::super::SourceRuntimeReachability::Conditional
            );
            assert!(phase_binding.proved_execution_target().is_none());
        }
        let body = advice
            .body_source(&bindings, advice.invocation.selection.body)
            .unwrap()
            .into_source()
            .unwrap();
        assert_eq!(body.0.try_text().unwrap(), "puts $x");
        let mut changed = tokens.clone();
        changed.source_binding.as_mut().unwrap().variable_frame =
            crate::var_resolve::VariableExecutionFrame::Global;
        assert!(SourceCommandBindings::scoped_lifecycle_advice(&changed, &registry).is_none());
        changed = tokens.clone();
        changed.word_exprs.clear();
        assert!(SourceCommandBindings::scoped_lifecycle_advice(&changed, &registry).is_none());
    }

    #[test]
    fn lifecycle_repeat_join_preserves_entry_and_withdraws_unbounded_issuance() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut head = super::super::ModuleCommandBindings::initial(&registry);
        let initial = head.clone();
        let mut transfer = head.clone();
        transfer.mark_opaque_binding_mutation();
        assert_ne!(
            transfer.source_variables.representation_epoch,
            initial.source_variables.representation_epoch
        );
        head.join(&transfer);
        assert_eq!(initial.source_variables.representation_epoch, Some(0));
        assert_eq!(head.source_variables.representation_epoch, None);
        assert_eq!(head.source_variables.representation_epoch_high_water, None);
        let fixed = head.clone();
        let mut next = head.clone();
        next.mark_opaque_binding_mutation();
        head.join(&next);
        assert!(head.same_state(&fixed));
        head.join(&fixed);
        assert!(head.same_state(&fixed), "repeat joins remain idempotent");
        let mut fresh = initial.clone();
        fresh.join(&initial);
        assert_eq!(fresh.source_variables.representation_epoch, Some(0));
        fresh.mark_opaque_binding_mutation();
        assert_eq!(
            fresh.source_variables.representation_epoch,
            Some(1),
            "independent fresh issuance remains available"
        );
    }

    #[test]
    fn lifecycle_repeat_join_withdraws_conflicting_cell_generations() {
        use crate::place::CellGeneration;
        use crate::var_resolve::{ResolveContext, VariableCellKey};
        let key = VariableCellKey::Authored("x".to_owned());
        let mut head = ResolveContext::default();
        head.generations
            .insert(key.clone(), CellGeneration::After(10));
        let mut next = head.clone();
        next.generations
            .insert(key.clone(), CellGeneration::After(20));
        head.join(&next);
        assert_eq!(head.generations.get(&key), Some(&CellGeneration::Unknown));
        let fixed = head.clone();
        next.generations
            .insert(key.clone(), CellGeneration::After(30));
        head.join(&next);
        assert_eq!(head, fixed, "later lifetimes cannot restore a joined owner");
        head.join(&fixed);
        assert_eq!(head, fixed);
    }

    #[test]
    fn missing_authored_lifecycle_provider_retains_unknown_phase_coverage() {
        let (bindings, tokens, registry) = inventory_with_provider("", false);
        assert!(SourceCommandBindings::scoped_lifecycle_advice(&tokens, &registry).is_none());
        if let Some(site) = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.invocation_site())
        {
            assert_eq!(
                bindings.lifecycle_phase_reachability_at(site, SourceBodyPhase::Body),
                SourceBodyPhaseReachability::Unknown
            );
        }
    }

    #[test]
    fn actual_lifecycle_coverage_and_unknown_world_cannot_donate_scoped_advice() {
        let (bindings, tokens, registry) = inventory("; p");
        assert!(SourceCommandBindings::scoped_lifecycle_advice(&tokens, &registry).is_none());
        assert_eq!(
            bindings.lifecycle_phase_reachability_at(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .invocation_site()
                    .unwrap(),
                SourceBodyPhase::Body,
            ),
            SourceBodyPhaseReachability::MayEntered
        );
        for suffix in [
            "; unknown_future_entry",
            "; proc ::tcltest::EvalTest args {}",
            "; proc ::tcltest::SetupTest args {}",
        ] {
            let (_bindings, tokens, registry) = inventory(suffix);
            assert!(
                SourceCommandBindings::scoped_lifecycle_advice(&tokens, &registry).is_none(),
                "{suffix}"
            );
        }
    }
}
