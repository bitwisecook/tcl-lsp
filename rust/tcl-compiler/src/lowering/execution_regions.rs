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

//! Proof-gated attachment of registry-owned caller-frame execution contracts.

use super::Lowerer;
use crate::execution_region::{EvaluatedBodyRegion, ExecutionRegionDependency, RegionSelection};
use crate::ir::{CommandTokens, Script, SourceSite};
use crate::registry_invocation::effective_command_words;
use crate::segmenter::SegmentedCommand;
use tcl_registry::InvocationArguments;
use tcl_registry::body_execution::{BodyExecutionSelection, BodyExecutionSpec, BodyOperand};
use tcl_syntax::word_rules::WordValueRules;

/// Adapt explicitly selected stock distribution data to the shared source
/// kernel. `None` retains the audited intersection without guessing a version.
#[must_use]
pub fn stock_body_provider_loader(
    provider: tcl_registry::body_execution::StockBodyProvider,
    version: Option<&str>,
) -> Option<crate::command_binding::TrustedPackageLoader> {
    let (commands, exports) = if let Some(version) = version {
        let selected = provider
            .versions
            .iter()
            .find(|selected| selected.version == version)?;
        (
            selected.commands.split_whitespace().collect::<Vec<_>>(),
            selected.exports.split_whitespace().collect::<Vec<_>>(),
        )
    } else {
        (provider.common_commands(), provider.common_exports())
    };
    if commands.is_empty() {
        return None;
    }
    let namespace = provider.namespace.to_owned();
    let optional_commands = if version.is_none() {
        provider
            .possible_commands()
            .into_iter()
            .filter(|command| !commands.contains(command))
            .map(str::to_owned)
            .collect()
    } else {
        Vec::new()
    };
    let optional_exports = if version.is_none() {
        provider
            .possible_exports()
            .into_iter()
            .filter(|export| !exports.contains(export))
            .map(str::to_owned)
            .collect()
    } else {
        Vec::new()
    };
    let (core_heads, required_heads) = if let Some(version) = version {
        let heads = provider
            .versions
            .iter()
            .find(|selected| selected.version == version)?
            .core_lookups
            .to_vec();
        (heads.clone(), heads)
    } else {
        (
            provider.possible_core_lookups(),
            provider.common_core_lookups(),
        )
    };
    let lookup_dependencies = provider
        .core_lookup_namespaces
        .iter()
        .flat_map(|namespace| {
            core_heads
                .iter()
                .map(|head| crate::command_binding::TrustedCommandLookup {
                    head: (*head).into(),
                    namespace: (*namespace).into(),
                    registry_identity: format!("::{head}"),
                    required: required_heads.contains(head),
                })
        })
        .collect();
    Some(crate::command_binding::TrustedPackageLoader {
        required_core_family: Some(provider.required_core_family),
        package: provider.package.into(),
        version: version.map(str::to_owned),
        implementation_id: provider.implementation_id.into(),
        // The audited stock body provider surface consists of Tcl procedures.
        compiler_hooks: commands
            .iter()
            .map(|command| {
                (
                    (*command).to_owned(),
                    tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
                )
            })
            .collect(),
        command_surface: commands.into_iter().map(str::to_owned).collect(),
        definition_dispatchers: std::collections::BTreeSet::new(),
        optional_command_surface: optional_commands,
        optional_namespace_exports: [(namespace.clone(), optional_exports)]
            .into_iter()
            .collect(),
        namespace_exports: [(namespace, exports.into_iter().map(str::to_owned).collect())]
            .into_iter()
            .collect(),
        lookup_dependencies,
        installed_lookup_dependencies: Vec::new(),
        state_dependency_namespaces: provider
            .state_dependency_namespaces
            .iter()
            .map(|namespace| (*namespace).to_owned())
            .collect(),
        modelled_state_variables: provider
            .modelled_state_variables
            .iter()
            .map(|variable| (*variable).to_owned())
            .collect(),
    })
}

type BodyText = Option<(
    crate::command_binding::ExecutedScriptSource,
    Option<crate::command_binding::SourceNamespaceKey>,
)>;

struct SelectedLifecycle {
    setup: BodyText,
    body: BodyText,
    cleanup: BodyText,
    dependencies: Vec<ExecutionRegionDependency>,
    repetition: crate::execution_region::RegionRepetition,
}

struct SelectedProvider<'a> {
    contract: tcl_registry::body_execution::CapturedLifecycleSpec,
    loader: &'a crate::command_binding::TrustedPackageLoader,
    commands: Vec<&'static str>,
    argument_offset: usize,
}

fn hooks_are_absent(
    bindings: &crate::command_binding::SourceCommandBindings,
    hooks: &[&str],
    offset: u32,
) -> bool {
    hooks
        .iter()
        .all(|hook| bindings.command_is_absent_at(hook, offset))
}

fn direct_body_operand(
    bindings: &crate::command_binding::SourceCommandBindings,
    tokens: &CommandTokens,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    operand: Option<BodyOperand>,
    argument_offset: usize,
) -> Option<BodyText> {
    let Some(operand) = operand else {
        return Some(None);
    };
    let argument = operand.argument.checked_add(argument_offset)?;
    let written = effective.written_argument(argument)?;
    let word = tokens.words().get(written + 1)?;
    let script = if let Some(element) = operand.list_element {
        bindings.executed_script_for_list_element(word.source().span, element)
    } else {
        bindings.executed_script_for_word(word.source().span)
    }?;
    let namespace = bindings.executed_script_entry_namespace_key_at(
        tokens.source_binding.as_ref()?.invocation_site()?,
        argument,
        script,
    );
    Some(Some((script.clone(), namespace)))
}

fn lifecycle_site(
    seg: &SegmentedCommand,
    tokens: &CommandTokens,
) -> Option<crate::command_binding::CommandAllocationSite> {
    Some(crate::command_binding::CommandAllocationSite {
        source: tokens.source_binding.as_ref()?.source_origin()?.clone(),
        offset: seg.span.start(),
    })
}

fn lifecycle_body_operand(
    bindings: &crate::command_binding::SourceCommandBindings,
    site: &crate::command_binding::CommandAllocationSite,
    phase: crate::command_binding::SourceBodyPhase,
    tokens: &CommandTokens,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    operand: Option<BodyOperand>,
    argument_offset: usize,
) -> Option<BodyText> {
    use crate::command_binding::SourceBodyPhaseReachability;
    match bindings.lifecycle_phase_reachability_at(site, phase) {
        SourceBodyPhaseReachability::NotEntered => Some(None),
        SourceBodyPhaseReachability::MayEntered => {
            direct_body_operand(bindings, tokens, effective, operand, argument_offset)
        }
        SourceBodyPhaseReachability::Unknown => None,
    }
}

impl Lowerer<'_> {
    /// Attach only a proved invocation/provider/phase contract. Runtime argv
    /// remains original; unsupported wrapper effects retain opaque dispatch.
    pub(super) fn attach_evaluated_body(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: CommandTokens,
    ) -> CommandTokens {
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource
            && crate::registry_invocation::proved_native_admitted_inline_operation(&tokens)
                .is_none()
        {
            return tokens;
        }
        if let Some(region) = self.prove_dictionary_scope(seg, namespace, &tokens) {
            return tokens.with_evaluated_body(region);
        }
        let Some(selected) = self.prove_captured_lifecycle(seg, &tokens) else {
            return self
                .prove_possible_bodies(seg, namespace, &tokens)
                .map_or(tokens.clone(), |region| tokens.with_evaluated_body(region));
        };
        let lower = |this: &mut Self, source: BodyText| {
            let Some((source, namespace)) = source else {
                return Script::new();
            };
            // Declaration effects belong to phase entry, not module loading.
            let previous = std::mem::replace(&mut this.suppress_proc_register, true);
            let script = this.lower_original_body(source, namespace.as_ref());
            this.suppress_proc_register = previous;
            script
        };
        let setup = lower(self, selected.setup);
        let body = lower(self, selected.body);
        let cleanup = lower(self, selected.cleanup);
        tokens.with_evaluated_body(
            EvaluatedBodyRegion::captured_lifecycle(
                SourceSite::source(seg.span),
                setup,
                body,
                cleanup,
                RegionSelection::MaySkip,
                selected.dependencies,
            )
            .with_repetition(selected.repetition),
        )
    }

    fn prove_possible_bodies(
        &mut self,
        seg: &SegmentedCommand,
        _namespace: &str,
        tokens: &CommandTokens,
    ) -> Option<EvaluatedBodyRegion> {
        use tcl_registry::native_compilation::PossibleBodyTopology;
        let (selected, entered) = if let Some(selected) =
            crate::registry_invocation::possible_body_invocation(self.registry, None, tokens)
        {
            (selected, true)
        } else {
            (
                crate::registry_invocation::conditional_body_topology_advice(
                    self.registry,
                    tokens,
                    self.source_bindings.as_ref()?,
                )?,
                false,
            )
        };
        let operands = possible_body_operands(&selected.topology);
        let conditions = possible_body_conditions(selected.dialect, self.registry, &selected);
        let mut phases = Vec::new();
        let mut conditional_sources = Vec::new();
        for &operand in &operands {
            let argument = operand.argument;
            let written = selected.effective.written_argument(argument)?;
            let word = tokens.words().get(written + 1)?;
            conditional_sources.push(self.conditional_body_source(
                tokens,
                word,
                written,
                operand,
                selected.dialect,
            ));
            let source = self
                .source_bindings
                .as_ref()
                .filter(|_| entered)
                .and_then(|bindings| {
                    if let Some(element) = operand.list_element {
                        bindings.executed_script_for_list_element(word.source().span, element)
                    } else {
                        bindings.executed_script_for_word(word.source().span)
                    }
                })
                .cloned();
            let entered = source.and_then(|source| {
                let namespace = self.source_bindings.as_ref().and_then(|bindings| {
                    bindings.executed_script_entry_namespace_key_at(
                        tokens.source_binding.as_ref()?.invocation_site()?,
                        argument,
                        &source,
                    )
                })?;
                Some((source, namespace))
            });
            let script = if let Some((source, entered_namespace)) = entered {
                let previous = std::mem::replace(&mut self.suppress_proc_register, true);
                let script = self.lower_executed_script_in_context(source, &entered_namespace);
                self.suppress_proc_register = previous;
                script
            } else {
                // Conditional source advice is retained separately. An absent
                // entered inventory provides no executable statements.
                Script::new()
            };
            phases.push(crate::execution_region::ExecutionPhase {
                script,
                normal: crate::execution_region::RegionTarget::Exit,
                abrupt: if matches!(selected.topology, PossibleBodyTopology::Captured(_)) {
                    crate::execution_region::RegionTarget::Exit
                } else {
                    crate::execution_region::RegionTarget::Propagate
                },
            });
        }
        if phases.is_empty()
            || (conditions.is_empty()
                && conditional_sources.iter().all(Option::is_none)
                && phases
                    .iter()
                    .all(|phase| phase.script.statements.is_empty()))
        {
            return None;
        }
        let mut region = EvaluatedBodyRegion::transparent(
            SourceSite::source(seg.span),
            Script::new(),
            vec![ExecutionRegionDependency::Dispatch(Box::new(
                tokens.source_binding.as_ref()?.clone(),
            ))],
        );
        region.phases = phases;
        region.selection = RegionSelection::MaySkip;
        region.residual_effects = crate::execution_region::WrapperEffectProjection::OpaqueResidual;
        region.possible_bodies = Some(Box::new(crate::execution_region::PossibleBodyRegion {
            topology: selected.topology,
            arguments: operands.iter().map(|operand| operand.argument).collect(),
            conditional_sources,
            conditions,
        }));
        Some(region)
    }

    fn conditional_body_source(
        &self,
        tokens: &CommandTokens,
        word: &crate::ir::WordExpr,
        written: usize,
        operand: BodyOperand,
        dialect: Option<tcl_registry::InvocationDialect>,
    ) -> Option<crate::execution_region::ConditionalBodySource> {
        use crate::registry_invocation::{EffectiveInvocationWord, effective_invocation_word};
        let binding = tokens.source_binding.as_ref()?;
        let dialect = dialect?;
        let config = self.config.with_grammar(dialect.lexer_grammar);
        let EffectiveInvocationWord::Literal(value) =
            effective_invocation_word(word, config.escapes, dialect.word_values)
        else {
            return None;
        };
        let parent = binding.invocation_site()?.clone();
        let mut source = crate::command_binding::ExecutedScriptSource::from_word(
            parent.clone(),
            written,
            word,
            &value,
            config,
        );
        if let Some(element) = operand.list_element {
            source = source.list_element(parent, written, element, dialect.word_values)?;
        }
        crate::execution_region::ConditionalBodySource::new(
            source,
            binding.variable_frame.clone(),
            config,
        )
    }

    fn prove_dictionary_scope(
        &mut self,
        seg: &SegmentedCommand,
        _namespace: &str,
        tokens: &CommandTokens,
    ) -> Option<EvaluatedBodyRegion> {
        use tcl_registry::dictionary_scope::{
            DictionaryScopeImplementation, DictionaryScopeSelection,
        };
        let binding = tokens.source_binding.as_ref()?;
        if !binding.proved_execution_target()?.registry_backed {
            return None;
        }
        let invocation =
            crate::registry_invocation::resolved_tokens_invocation(self.registry, None, tokens)?;
        let BodyExecutionSpec::DictionaryScope(spec) = invocation.facts.body_execution? else {
            return None;
        };
        let DictionaryScopeSelection::Selected(plan) = invocation.with_argument_words(|words| {
            spec.select(words.arguments(), invocation.facts.argument_offset)
        }) else {
            return None;
        };
        // Scripted wrappers need independent provider/helper closure evidence.
        if plan.implementation != DictionaryScopeImplementation::NativePrimitive {
            return None;
        }
        if let Some(lookup) = invocation
            .facts
            .native_compilation?
            .implementation_lookup(plan.dialect)
            && !binding.proves_native_implementation_lookup(&lookup)
        {
            return None;
        }
        let effective = effective_command_words(tokens)?;
        let written = effective.written_argument(plan.body_argument)?;
        let word = tokens.words().get(written + 1)?;
        let source = self
            .source_bindings
            .as_ref()?
            .executed_script_for_word(word.source().span)?
            .clone();
        let mut entry = binding.variable_context.as_ref().clone();
        if !invocation.with_argument_words(|words| {
            matches!(
                crate::dictionary_bindings::enter_dictionary_scope(
                    &mut entry,
                    &plan,
                    words.arguments(),
                    self.registry,
                    seg.span.start()
                ),
                crate::dictionary_bindings::DictionaryScopeEntry::Entered(_)
            )
        }) {
            return None;
        }
        let previous = std::mem::replace(&mut self.suppress_proc_register, true);
        let namespace = self.original_body_namespace_context(tokens, plan.body_argument, &source);
        let script = self.lower_original_body(source, namespace.as_ref());
        self.suppress_proc_register = previous;
        let scope = crate::execution_region::DictionaryScopeRegion {
            plan,
            invocation: tokens.clone(),
            arguments: (0..invocation.arguments.len())
                .map(|index| invocation.argument_literal(index))
                .collect(),
        };
        Some(EvaluatedBodyRegion::dictionary_scope(
            SourceSite::source(seg.span),
            script,
            scope,
            vec![ExecutionRegionDependency::Dispatch(Box::new(
                binding.clone(),
            ))],
        ))
    }

    fn selected_body_provider(
        &self,
        seg: &SegmentedCommand,
        tokens: &CommandTokens,
    ) -> Option<SelectedProvider<'_>> {
        let target = tokens.source_binding.as_ref()?.proved_execution_target()?;
        if !target.registry_backed {
            return None;
        }
        let context = self.invocation_metadata_context()?;
        let crate::registry_invocation::RegistryInvocationResolution::Resolved(facts) =
            crate::registry_invocation::resolve_command_tokens_with_metadata_context(
                self.registry,
                Some(context),
                tokens,
            )
            .ok()?
        else {
            return None;
        };
        let BodyExecutionSpec::CapturedLifecycle(contract) = facts.body_execution? else {
            return None;
        };
        let bindings = self.source_bindings.as_ref()?;
        let loader =
            bindings.loaded_implementation_at(contract.provider.package, seg.span.start())?;
        if loader.implementation_id != contract.provider.implementation_id {
            return None;
        }
        let commands = if let Some(version) = loader.version.as_deref() {
            contract
                .provider
                .versions
                .iter()
                .find(|provider| provider.version == version)?
                .commands
                .split_whitespace()
                .collect()
        } else {
            contract.provider.common_commands()
        };
        // Helper-command closure is part of the stock implementation proof.
        if !bindings.provider_surface_untampered(
            contract.provider.package,
            contract.provider.implementation_id,
            seg.span.start(),
        ) {
            return None;
        }
        Some(SelectedProvider {
            contract: *contract,
            loader,
            commands,
            argument_offset: facts.argument_offset,
        })
    }

    fn selected_lifecycle_operands(
        &self,
        seg: &SegmentedCommand,
        tokens: &CommandTokens,
        provider: &SelectedProvider<'_>,
    ) -> Option<(BodyText, BodyText, BodyText)> {
        let contract = provider.contract;
        let effective = effective_command_words(tokens)?;
        let rules = WordValueRules::from_config(&self.config);
        let invocation =
            crate::registry_invocation::resolved_tokens_invocation(self.registry, None, tokens)?;
        let values = (provider.argument_offset..invocation.arguments.len())
            .map(|index| invocation.argument_literal(index))
            .collect::<Vec<_>>();
        let words = values
            .iter()
            .map(|value| {
                value.as_deref().map_or(
                    tcl_registry::InvocationWord::Dynamic,
                    tcl_registry::InvocationWord::Literal,
                )
            })
            .collect::<Vec<_>>();
        let arguments = InvocationArguments::structured(&words).with_dialect(invocation.dialect?);
        let BodyExecutionSelection::CapturedLifecycle(selection) = contract.select(arguments)
        else {
            return None;
        };
        if contract.has_unmodelled_pre_phase_effects(arguments) {
            return None;
        }
        if arguments
            .literal_at(contract.leading_arguments)
            .is_some_and(|first| first.starts_with('-'))
        {
            for index in (contract.leading_arguments..arguments.len()).step_by(2) {
                let option = arguments.literal_at(index)?;
                if !contract.supports_option(provider.loader.version.as_deref(), option) {
                    return None;
                }
                if contract.list_valued_options.contains(&option)
                    && arguments
                        .literal_at(index + 1)
                        .is_none_or(|value| rules.split_list(value).is_err())
                {
                    return None;
                }
            }
        }
        let bindings = self.source_bindings.as_ref()?;
        let site = lifecycle_site(seg, tokens)?;
        let body = |phase, operand| {
            lifecycle_body_operand(
                bindings,
                &site,
                phase,
                tokens,
                &effective,
                operand,
                provider.argument_offset,
            )
        };
        Some((
            body(
                crate::command_binding::SourceBodyPhase::Setup,
                selection.setup,
            )?,
            body(
                crate::command_binding::SourceBodyPhase::Body,
                selection.body,
            )?,
            body(
                crate::command_binding::SourceBodyPhase::Cleanup,
                selection.cleanup,
            )?,
        ))
    }

    fn prove_captured_lifecycle(
        &self,
        seg: &SegmentedCommand,
        tokens: &CommandTokens,
    ) -> Option<SelectedLifecycle> {
        if tokens.source_binding.as_ref()?.runtime_reachability()
            == crate::command_binding::SourceRuntimeReachability::Conditional
        {
            return self.prove_scoped_lifecycle(tokens);
        }
        let provider = self.selected_body_provider(seg, tokens)?;
        let (setup, body, cleanup) = self.selected_lifecycle_operands(seg, tokens, &provider)?;
        let bindings = self.source_bindings.as_ref()?;
        let hooks = provider
            .contract
            .required_absent_hooks(provider.loader.version.as_deref());
        if !hooks_are_absent(bindings, hooks, seg.span.start()) {
            return None;
        }
        let source_binding = tokens.source_binding.as_ref()?;
        let wrapper_head = &source_binding.proved_execution_target()?.command;
        let mut phase_dependencies = Vec::new();
        let site = lifecycle_site(seg, tokens)?;
        // The source owner records nested dispatch in semantic execution order.
        // Earlier phases may install a hook or replace a stock helper.
        for phase in [
            crate::command_binding::SourceBodyPhase::Setup,
            crate::command_binding::SourceBodyPhase::Body,
            crate::command_binding::SourceBodyPhase::Cleanup,
        ] {
            use crate::command_binding::SourceBodyPhaseReachability;
            match bindings.lifecycle_phase_reachability_at(&site, phase) {
                SourceBodyPhaseReachability::NotEntered => {
                    phase_dependencies.push(ExecutionRegionDependency::NotEnteredPhase(phase));
                    continue;
                }
                SourceBodyPhaseReachability::MayEntered => {}
                SourceBodyPhaseReachability::Unknown => return None,
            }
            if !bindings.provider_surface_untampered_at_phase(
                provider.contract.provider.package,
                provider.contract.provider.implementation_id,
                seg.span.start(),
                phase,
            ) || !hooks
                .iter()
                .all(|hook| bindings.command_is_absent_at_phase(hook, seg.span.start(), phase))
            {
                return None;
            }
            phase_dependencies.push(ExecutionRegionDependency::PhaseDispatch {
                phase,
                binding: Box::new(bindings.invocation_at_phase(
                    wrapper_head,
                    seg.span.start(),
                    phase,
                )),
            });
        }
        let mut dependencies = vec![
            ExecutionRegionDependency::Dispatch(Box::new(source_binding.clone())),
            ExecutionRegionDependency::Provider(Box::new(provider.loader.clone())),
            ExecutionRegionDependency::Implementation(provider.loader.implementation_id.clone()),
            ExecutionRegionDependency::Package {
                name: provider.contract.provider.package.into(),
                version: provider
                    .loader
                    .version
                    .as_deref()
                    .unwrap_or("audited-stock-intersection")
                    .into(),
            },
        ];
        dependencies.extend(phase_dependencies);
        dependencies.extend(
            provider
                .commands
                .iter()
                .map(|command| ExecutionRegionDependency::Implementation((*command).into())),
        );
        dependencies.extend(
            provider
                .loader
                .optional_command_surface
                .iter()
                .cloned()
                .map(ExecutionRegionDependency::OptionalImplementation),
        );
        dependencies.extend(
            hooks
                .iter()
                .map(|hook| ExecutionRegionDependency::AbsentCommand((*hook).into())),
        );
        Some(SelectedLifecycle {
            setup,
            body,
            cleanup,
            dependencies,
            repetition: provider
                .contract
                .repetition(provider.loader.version.as_deref()),
        })
    }

    fn prove_scoped_lifecycle(&self, tokens: &CommandTokens) -> Option<SelectedLifecycle> {
        use crate::command_binding::{SourceBodyPhase, SourceBodyPhaseReachability};
        let bindings = self.source_bindings.as_ref()?;
        let advice = crate::command_binding::SourceCommandBindings::scoped_lifecycle_advice(
            tokens,
            self.registry,
        )?;
        let invocation = &advice.invocation;
        let selected_body = |phase, operand| match advice.reachability(bindings, phase) {
            SourceBodyPhaseReachability::NotEntered => Some(None),
            SourceBodyPhaseReachability::MayEntered => Some(
                advice
                    .body_source(bindings, operand)?
                    .into_source()
                    .map(|(source, namespace)| (source, Some(namespace))),
            ),
            SourceBodyPhaseReachability::Unknown => None,
        };
        let setup = selected_body(SourceBodyPhase::Setup, invocation.selection.setup)?;
        let body = selected_body(SourceBodyPhase::Body, invocation.selection.body)?;
        let cleanup = selected_body(SourceBodyPhase::Cleanup, invocation.selection.cleanup)?;
        let mut dependencies = vec![
            ExecutionRegionDependency::ConditionalDispatch {
                binding: Box::new(tokens.source_binding.as_ref()?.clone()),
                frame: advice.frame().clone(),
            },
            ExecutionRegionDependency::Provider(Box::new(advice.loader.clone())),
            ExecutionRegionDependency::Implementation(advice.loader.implementation_id.clone()),
            ExecutionRegionDependency::Package {
                name: invocation.contract.provider.package.to_owned(),
                version: advice
                    .loader
                    .version
                    .as_deref()
                    .unwrap_or("audited-stock-intersection")
                    .to_owned(),
            },
        ];
        for phase in [
            SourceBodyPhase::Setup,
            SourceBodyPhase::Body,
            SourceBodyPhase::Cleanup,
        ] {
            match advice.reachability(bindings, phase) {
                SourceBodyPhaseReachability::NotEntered => {
                    dependencies.push(ExecutionRegionDependency::ConditionalNotEnteredPhase(phase));
                }
                SourceBodyPhaseReachability::MayEntered => {
                    dependencies.push(ExecutionRegionDependency::ConditionalPhaseDispatch {
                        phase,
                        binding: Box::new(advice.phase_binding(
                            bindings,
                            phase,
                            &invocation.command,
                        )?),
                        frame: advice.frame().clone(),
                    });
                }
                SourceBodyPhaseReachability::Unknown => return None,
            }
        }
        Some(SelectedLifecycle {
            setup,
            body,
            cleanup,
            dependencies,
            repetition: invocation
                .contract
                .repetition(advice.loader.version.as_deref()),
        })
    }
}

fn possible_body_conditions(
    dialect: Option<tcl_registry::InvocationDialect>,
    registry: &tcl_registry::CommandRegistry,
    selected: &crate::registry_invocation::PossibleBodyInvocation,
) -> Vec<crate::execution_region::PossibleBodyCondition> {
    selected
        .condition_values
        .iter()
        .filter_map(|(argument, value)| {
            let word = selected.effective.words.get(argument + 1)?;
            let expression = value.as_ref().and_then(|value| {
                dialect.map(|dialect| {
                    crate::expr_parser::parse_expr_with_syntax_context(
                        value,
                        &dialect.expression_parse_context(registry.profile()),
                    )
                })
            });
            let expression_base = match (word, value.as_deref()) {
                (crate::ir::WordExpr::BracedLiteral { text, source }, Some(value))
                    if text == value && source.provenance == crate::ir::Provenance::Source =>
                {
                    crate::lowering_hooks::word_content_base(source.span, true, text)
                }
                (crate::ir::WordExpr::Literal { text, source }, Some(value))
                    if text == value && source.provenance == crate::ir::Provenance::Source =>
                {
                    Some(source.span.start())
                }
                _ => None,
            };
            Some(crate::execution_region::PossibleBodyCondition {
                argument: *argument,
                expression,
                source: word.source().clone(),
                expression_base,
            })
        })
        .collect()
}

fn possible_body_operands(
    topology: &tcl_registry::native_compilation::PossibleBodyTopology,
) -> Vec<BodyOperand> {
    use tcl_registry::native_compilation::PossibleBodyTopology;
    match topology {
        PossibleBodyTopology::Captured(selected) => vec![BodyOperand {
            argument: selected.script_at,
            list_element: None,
        }],
        PossibleBodyTopology::Sequence(indices) | PossibleBodyTopology::Alternatives(indices) => {
            indices
                .iter()
                .map(|&argument| BodyOperand {
                    argument,
                    list_element: None,
                })
                .collect()
        }
        PossibleBodyTopology::Conditional(branches) => branches
            .iter()
            .map(|&(_, argument)| BodyOperand {
                argument,
                list_element: None,
            })
            .collect(),
        PossibleBodyTopology::CaseAlternatives(operands) => operands.clone(),
        PossibleBodyTopology::Loop {
            initial, repeated, ..
        } => initial
            .iter()
            .chain(repeated)
            .map(|&argument| BodyOperand {
                argument,
                list_element: None,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::SourceAnalysisOptions;
    use crate::ir::Statement;
    use crate::lowering::lower_to_ir_with;
    use tcl_registry::CommandRegistry;

    #[test]
    fn possible_f5_conditional_retains_original_expression_ownership() {
        let profile = tcl_dialect::DialectProfile::irules();
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let source = r#"if { [HTTP::uri] contains "&key=" } { log local0. x }"#;
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source, &registry, false, profile,
        );
        let function = unit.function("::top").unwrap();
        let branches: Vec<_> = function
            .cfg
            .blocks
            .iter()
            .filter_map(|(block, data)| match &data.terminator {
                Some(crate::cfg::Terminator::Branch { condition_base, .. }) => {
                    Some((*block, *condition_base))
                }
                _ => None,
            })
            .collect();
        assert_eq!(branches.len(), 1, "conditional entries: {branches:?}");
        let (block, base) = branches[0];
        assert_eq!(base, Some(4));
        assert!(
            matches!(
                function.cfg.blocks[&block].terminator.as_ref(),
                Some(crate::cfg::Terminator::Branch {
                    condition: crate::expr_ast::ExprNode::Binary {
                        op: crate::expr_ast::BinOp::Contains,
                        ..
                    },
                    ..
                })
            ),
            "the possible branch retains selected F5 operator grammar"
        );
        let tokens = function
            .cfg
            .source_tokens_at(block, usize::MAX)
            .expect("original conditional input carrier");
        let nested: Vec<_> = tokens
            .nested_bindings
            .iter()
            .map(|(offset, binding)| {
                (
                    *offset,
                    binding.unknown,
                    binding.may_be_absent,
                    binding
                        .targets
                        .iter()
                        .map(|target| target.command.as_str())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        assert!(
            nested.iter().any(|(offset, unknown, absent, targets)| {
                *offset == 6 && !unknown && !absent && targets.contains(&"::HTTP::uri")
            }),
            "retained expression calls: {nested:?}"
        );
    }

    fn lower(source: &str, trust: bool) -> crate::ir::Module {
        lower_stock_version(source, trust, "2.5.11")
    }

    fn lower_stock_version(source: &str, trust: bool, version: &str) -> crate::ir::Module {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("C Tcl profile");
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let provider = stock_body_provider_loader(
            tcl_registry::body_execution::TCLTEST_STOCK_PROVIDER,
            Some(version),
        )
        .unwrap();
        let providers = if trust { vec![provider] } else { Vec::new() };
        let mut lowerer = Lowerer::new(&registry).with_dialect(Some(profile));
        lowerer.set_source_analysis_options(SourceAnalysisOptions {
            trusted_package_loaders: &providers,
            unknown_entry: false,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..SourceAnalysisOptions::default()
        });
        lower_to_ir_with(lowerer, source)
    }

    fn region(module: &crate::ir::Module) -> Option<&EvaluatedBodyRegion> {
        module
            .procedures
            .get("::p")?
            .body
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Call {
                    tokens: Some(tokens),
                    ..
                }
                | Statement::Barrier {
                    tokens: Some(tokens),
                    ..
                } => tokens.evaluated_body(),
                _ => None,
            })
    }

    #[test]
    fn possible_handler_bodies_share_analysis_regions_without_opcode_permission() {
        use tcl_registry::native_compilation::{
            NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
        };
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let profile = registry.profile().expect("F5 context");
        let mut lowerer = Lowerer::new(registry).with_dialect(Some(profile));
        lowerer.set_source_analysis_options(SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ScriptCode,
                catch_depth: Some(0),
                loop_depth: 0,
            },
            ..SourceAnalysisOptions::default()
        });
        let module = lower_to_ir_with(lowerer, "if 1 {set flag 1}; list final");
        let first = module.top_level.statements.first().unwrap();
        let tokens = first.tokens().expect("original invocation");
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .proved_execution_target()
                .is_none()
        );
        let region = tokens.evaluated_body().expect("possible entered body");
        assert!(region.possible_bodies.is_some());
        assert_eq!(region.selection, RegionSelection::MaySkip);
        assert!(
            region
                .phases
                .iter()
                .any(|phase| !phase.script.statements.is_empty())
        );
        let cfg = crate::cfg_builder::build_cfg_with_registry(&module, false, registry);
        assert_ne!(
            cfg.top_level.analysis_edges,
            [] as [(crate::cfg::BlockId, crate::cfg::BlockId); 0]
        );
    }

    #[test]
    fn conditional_case_sources_do_not_claim_entered_bodies() {
        let source = "proc p {mode} {switch -- $mode {a {for {set j 0} {$j < 3} {incr j} {puts $j}} default {puts $missing}}}";
        let module = lower(source, false);
        if region(&module).is_none() {
            for statement in &module.procedures["::p"].body.statements {
                let Some(tokens) = statement.tokens() else {
                    continue;
                };
                let Some(binding) = &tokens.source_binding else {
                    continue;
                };
                let advice = binding.original_compilation_lookup_advice(tokens);
                let declaration = binding.declaration_operand_layout_advice(tokens);
                eprintln!(
                    "conditional topology: head={:?} offset={:?} runtime_unknown={} frame={:?} compiler_snapshot={} advice={} candidates={:?} declaration={:?}",
                    tokens.argv_texts.first(),
                    binding.invocation_site().map(|site| site.offset),
                    binding.execution_is_unknown(),
                    binding.variable_frame,
                    binding.compiler_lookup_state.is_some(),
                    advice.is_some(),
                    advice.as_ref().map(|advice| advice
                        .targets()
                        .iter()
                        .map(|target| &target.command)
                        .collect::<Vec<_>>()),
                    declaration.as_ref().map(|advice| (
                        advice.alias_frame(),
                        advice.dialect(),
                        advice
                            .targets()
                            .iter()
                            .map(|target| &target.command)
                            .collect::<Vec<_>>()
                    )),
                );
            }
        }
        let region = region(&module).expect("selected possible switch topology");
        assert_eq!(
            region.residual_effects,
            crate::execution_region::WrapperEffectProjection::OpaqueResidual
        );
        let selected = region.possible_bodies.as_ref().expect("possible bodies");
        assert_eq!(selected.conditional_sources.len(), 2);
        let first = selected.conditional_sources[0]
            .as_ref()
            .expect("unchanged first arm");
        assert_eq!(
            first.source().try_text().unwrap(),
            "for {set j 0} {$j < 3} {incr j} {puts $j}"
        );
        assert!(matches!(
            first.frame(),
            crate::var_resolve::VariableExecutionFrame::Procedure { .. }
        ));
        let base = first.source().base() as usize;
        assert_eq!(
            &source.as_bytes()[base..base + first.source().text.len()],
            first.source().text.bytes()
        );
        assert!(
            region
                .phases
                .iter()
                .all(|phase| phase.script.statements.is_empty()),
            "conditional source must not manufacture entered phase statements"
        );
        assert!(crate::script_binds::script_binds_name(
            first.source().try_text().unwrap(),
            "j",
            crate::script_binds::Ownership::Bindings,
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            first.config(),
        ));
        let last = selected.conditional_sources[1].as_ref().unwrap();
        assert!(!crate::script_binds::script_binds_name(
            last.source().try_text().unwrap(),
            "missing",
            crate::script_binds::Ownership::Bindings,
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            last.config(),
        ));
    }

    #[test]
    fn original_compilation_layout_advice_requires_its_retained_snapshot() {
        let source = "return done; switch -- $mode {a {puts A} default {puts D}}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let bindings = crate::command_binding::SourceCommandBindings::analyse_in_frame_with_options(
            source,
            &crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::".to_owned(),
                identity: "compiler-layout-test".to_owned(),
            },
            config,
            registry,
            crate::command_binding::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                    registry.profile().unwrap(),
                )),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let body = "switch -- $mode {a {puts A} default {puts D}}";
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            body,
            u32::try_from(source.find("switch").unwrap()).unwrap(),
            config,
        )
        .remove(0);
        let mut original = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceImage::document(source).source_map(),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut original);
        let tokens = &original;
        let binding = tokens
            .source_binding
            .as_ref()
            .expect("original source binding");
        let advice = binding.original_compilation_lookup_advice(tokens);
        if advice.is_none() {
            eprintln!(
                "compiler layout: snapshot={} runtime={:?} native_inline={:?} site={:?} replay={:?}",
                binding.compiler_lookup_state.is_some(),
                binding.runtime_reachability(),
                binding.admitted_inline_invocation(),
                binding.invocation_site().map(|site| site.offset),
                binding.invocation_site().and_then(|site| {
                    crate::registry_invocation::native_compiler_replay_source(tokens, site)
                }),
            );
        }
        let advice = advice.expect("pre-argument stock lookup can supply lexical layout");
        assert!(
            binding.invocation_site().is_none(),
            "this command never entered at runtime"
        );
        assert_eq!(
            binding.original_lexer_config_for_tokens(tokens),
            Some(config)
        );
        assert!(!advice.targets().is_empty());
        assert!(
            binding.execution_is_unknown(),
            "lexical lookup must retain opaque runtime dispatch"
        );
        let mut missing = binding.clone();
        missing.compiler_lookup_state = None;
        assert!(missing.original_compilation_lookup_advice(tokens).is_none());
        let mut synthetic = tokens.clone();
        synthetic.synthetic = Some(crate::ir::SyntheticMarker::IterationBindings(Some(
            tcl_registry::TclType::List,
        )));
        assert!(
            binding
                .original_compilation_lookup_advice(&synthetic)
                .is_none()
        );
    }

    #[test]
    fn conditional_case_sources_decline_replaced_handlers() {
        let module = lower(
            "proc switch args {}; proc p {mode} {switch -- $mode {a {set j 0}}}",
            false,
        );
        assert!(region(&module).is_none());
    }

    #[test]
    fn case_list_possible_regions_enter_bodies_as_list_elements() {
        use tcl_registry::native_compilation::{
            NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
            PossibleBodyTopology,
        };
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let profile = registry.profile().unwrap();
        let mut lowerer = Lowerer::new(registry).with_dialect(Some(profile));
        lowerer.set_source_analysis_options(SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ScriptCode,
                catch_depth: Some(0),
                loop_depth: 0,
            },
            ..SourceAnalysisOptions::default()
        });
        let module = lower_to_ir_with(
            lowerer,
            "set mode loud; switch $mode {loud {set debug 1} default {set debug 0}}",
        );
        let region = module
            .top_level
            .statements
            .iter()
            .filter_map(Statement::tokens)
            .find_map(CommandTokens::evaluated_body)
            .expect("possible case bodies");
        let selected = region.possible_bodies.as_ref().unwrap();
        assert!(
            matches!(&selected.topology, PossibleBodyTopology::CaseAlternatives(bodies)
            if bodies.len()==2 && bodies[0].list_element==Some(1)
                && bodies[1].list_element==Some(3))
        );
        assert_eq!(region.phases.len(), 2);
        assert!(selected.conditional_sources.iter().all(Option::is_some));
        for phase in &region.phases {
            assert_eq!(phase.script.statements.len(), 1);
            assert!(
                matches!(&phase.script.statements[0], Statement::Call { command, .. }
                if command=="set")
            );
            assert!(
                phase
                    .script
                    .source_edit_span(phase.script.statements[0].span())
                    .is_some()
            );
        }
        let cfg = crate::cfg_builder::build_cfg_with_registry(&module, false, registry);
        assert_ne!(
            cfg.top_level.analysis_edges,
            [] as [(crate::cfg::BlockId, crate::cfg::BlockId); 0]
        );
    }

    #[test]
    fn captured_provider_metadata_keeps_actual_availability_and_missing_owner_refusal() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Held original source binding and explicit stock loader; no physical
        // package load, execution or future lifecycle is observed by this test.
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut commands = baseline.commands().project_for_profile(profile);
        let mut test = commands.get("tcltest::test").unwrap().clone();
        test.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        commands.insert(test);
        let context =
            std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(commands)));
        let loaders = [stock_body_provider_loader(
            tcl_registry::body_execution::TCLTEST_STOCK_PROVIDER,
            Some("2.5.11"),
        )
        .unwrap()];
        let mut lowerer = Lowerer::with_config(
            context.commands(),
            tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        )
        .with_context_registry(std::sync::Arc::clone(&context));
        lowerer.set_source_analysis_options(SourceAnalysisOptions {
            trusted_package_loaders: &loaders,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..SourceAnalysisOptions::default()
        });
        let source = "package require tcltest; tcltest::test n d -body {return OK}";
        let tokens = lowerer
            .lower(source)
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap()
            .clone();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, lowerer.config)
                .pop()
                .unwrap();
        // This is the same completed source owner, restored for its read-only
        // selected-provider query after lower() has unwound its lexical walk.
        lowerer.source_bindings = lowerer
            .module_source_bindings
            .take()
            .map(|bindings| *bindings);
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .proved_execution_target()
                .is_some()
        );
        assert!(lowerer.selected_body_provider(&segment, &tokens).is_some());
        let older = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(std::sync::Arc::clone(context.commands())),
        );
        let foreign = tcl_registry::model::ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
        );
        for unavailable in [Some(older), Some(foreign), None] {
            lowerer.dialect_context = unavailable;
            assert!(lowerer.selected_body_provider(&segment, &tokens).is_none());
        }
        lowerer.dialect_context = Some(std::sync::Arc::clone(&context));
        assert!(lowerer.selected_body_provider(&segment, &tokens).is_some());
    }

    #[test]
    fn qualified_and_imported_wrappers_share_the_same_lifecycle() {
        for head in ["tcltest::test", "test"] {
            let module = lower(
                &format!(
                    "package require tcltest\nnamespace import ::tcltest::test\nproc p {{}} {{ {head} n d -body {{puts $x}} -cleanup {{unset x}} -setup {{set x 1}} }}"
                ),
                true,
            );
            let region = region(&module).expect("proved stock wrapper must expose phases");
            assert_eq!(region.phases.len(), 3);
            assert!(
                matches!(region.phases[0].script.statements.first(), Some(Statement::AssignConst { name, .. }) if name == "x")
            );
            assert_eq!(
                region.phases[0].normal,
                crate::execution_region::RegionTarget::Phase(1)
            );
            assert_eq!(
                region.phases[0].abrupt,
                crate::execution_region::RegionTarget::Phase(2)
            );
        }
    }

    #[test]
    fn package_advertisement_and_replaced_wrapper_do_not_license_expansion() {
        let source = "package require tcltest\nproc p {} {tcltest::test n d -setup {set x 1} -body {puts $x}}";
        assert!(region(&lower(source, false)).is_none());
        let replaced = "package require tcltest\nproc ::tcltest::test {args} {}\nproc p {} {tcltest::test n d -body {puts $missing}}";
        assert!(region(&lower(replaced, true)).is_none());
    }

    #[test]
    fn helper_replacement_and_phase_hook_installation_decline_the_plan() {
        for source in [
            "package require tcltest\nproc ::tcltest::Skipped {args} {return 0}\nproc p {} {tcltest::test n d -body {puts $missing}}",
            "package require tcltest\nproc p {} {tcltest::test n d -setup {proc ::tcltest::EvalTest {args} {}} -body {puts $missing}}",
            "package require tcltest\nproc p {} {tcltest::test n d -setup {rename ::catch ::oldCatch} -body {return ok}}",
            "package require tcltest\nproc p {} {tcltest::test n d -body {rename ::tcltest::RunTest ::oldRunTest} -cleanup {puts done}}",
            "package require tcltest\ntrace add variable ::tcltest::testLevel write {apply {{args} {proc ::tcltest::EvalTest {args} {}}}}\nproc p {} {tcltest::test n d -body {puts $missing}}",
        ] {
            assert!(region(&lower(source, true)).is_none());
        }
    }

    #[test]
    fn issue_2286_default_c_authoring_entry_uses_real_setup_definitions() {
        let registry = CommandRegistry::build_default();
        for head in ["tcltest::test", "test"] {
            let source = format!(
                "package require tcltest\nnamespace import ::tcltest::test\nproc p {{}} {{{head} n d -setup {{set x 1}} -body {{puts $x; puts $missing}} -cleanup {{unset x}}}}"
            );
            let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
                &source, &registry, false, "tcl8.6",
            );
            assert!(region(&unit.ir_module).is_some());
            let ssa = &unit.procedures["::p"].ssa;
            let read_versions = |name: &str| {
                let mut versions = Vec::new();
                for (block, body) in &ssa.blocks {
                    for (index, statement) in body.statements.iter().enumerate() {
                        if let Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. } =
                            &statement.statement
                            && tokens.as_ref().is_some_and(|tokens| {
                                tokens.argv_texts.first().is_some_and(|head| head == "puts")
                            })
                            && let Some(symbol) = ssa.var_symbol_at(*block, index, name)
                        {
                            versions.push(
                                *statement
                                    .uses
                                    .get(&symbol)
                                    .expect("real variable substitution remains visible"),
                            );
                        }
                    }
                }
                versions
            };
            let x_reads = read_versions("x");
            assert!(
                !x_reads.is_empty(),
                "real body read is missing from SSA: {ssa:#?}"
            );
            assert!(
                x_reads.iter().all(|version| *version != 0),
                "setup's normal completion must reach the body read"
            );
            assert!(read_versions("missing").contains(&0));
        }
    }

    #[test]
    fn scoped_provider_lifecycle_retains_conditional_dependencies() {
        let source = "package require tcltest\nproc p {} {tcltest::test n d -setup {set x 1} -body {puts $x} -cleanup {unset x}}";
        let module = lower(source, true);
        let region = region(&module).expect("scoped original lifecycle");
        assert!(region.dependencies.iter().any(|dependency| matches!(
            dependency, ExecutionRegionDependency::ConditionalDispatch { binding, .. }
                if binding.runtime_reachability() == crate::command_binding::SourceRuntimeReachability::Conditional
                    && binding.proved_execution_target().is_none()
        )));
        assert_eq!(region.dependencies.iter().filter(|dependency| matches!(
            dependency, ExecutionRegionDependency::ConditionalPhaseDispatch { binding, .. }
                if binding.runtime_reachability() == crate::command_binding::SourceRuntimeReachability::Conditional
                    && binding.proved_execution_target().is_none()
        )).count(), 3);
        assert!(region.dependencies.iter().all(|dependency| !matches!(
            dependency,
            ExecutionRegionDependency::Dispatch(_)
                | ExecutionRegionDependency::PhaseDispatch { .. }
        )));
    }

    #[test]
    fn core_and_provider_namespace_shadows_withdraw_stock_proof() {
        for source in [
            "package require tcltest\nproc ::tcltest::set {args} {return}\nproc p {} {tcltest::test n d -body {puts $missing}}",
            "proc ::puts {args} {}\npackage require tcltest\nproc p {} {tcltest::test n d -body {puts $missing}}",
            "package require tcltest\nproc ::puts {args} {}\nproc p {} {tcltest::test n d -body {puts $missing}}",
        ] {
            assert!(region(&lower(source, true)).is_none());
        }
    }

    #[test]
    fn unknown_version_optional_helpers_are_not_assumed_absent() {
        let provider =
            stock_body_provider_loader(tcl_registry::body_execution::TCLTEST_STOCK_PROVIDER, None)
                .unwrap();
        assert!(
            provider
                .optional_command_surface
                .iter()
                .any(|command| command == "::tcltest::TestOnce")
        );
        assert!(
            provider
                .lookup_dependencies
                .iter()
                .any(|dependency| dependency.head == "set" && dependency.namespace == "::tcltest")
        );
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("C Tcl profile");
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let providers = [provider];
        let mut lowerer = Lowerer::new(&registry).with_dialect(Some(profile));
        lowerer.set_source_analysis_options(SourceAnalysisOptions {
            trusted_package_loaders: &providers,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            ..SourceAnalysisOptions::default()
        });
        let module = lower_to_ir_with(
            lowerer,
            "package require tcltest\nproc ::tcltest::TestOnce {args} {}\nproc p {} {tcltest::test n d -body {puts $missing}}",
        );
        assert!(region(&module).is_none());
    }

    #[test]
    fn direct_framework_callback_state_replacement_withdraws_phase_proof() {
        for mutation in [
            "set ::tcltest::CustomMatch(exact) {apply {{args} {proc ::tcltest::CleanupTest {script} {}}}}",
            "tcltest::customMatch exact {apply {{args} {proc ::tcltest::CleanupTest {script} {}}}}",
        ] {
            let source = format!(
                "package require tcltest\n{mutation}\nproc p {{}} {{tcltest::test n d -setup {{set x 1}} -body {{puts $x}} -cleanup {{puts $missing}}}}"
            );
            assert!(region(&lower(&source, true)).is_none());
        }
    }

    #[test]
    fn repetition_value_contract_does_not_license_new_private_observers() {
        let source = "package require tcltest\ntrace add variable ::tcltest::Option(-iterations) read {apply {{args} {proc ::tcltest::EvalTest {args} {}}}}\nproc p {} {tcltest::test n d -setup {set x 1} -body {puts $x}}";
        assert!(region(&lower_stock_version(source, true, "2.6.0")).is_none());
    }
}
