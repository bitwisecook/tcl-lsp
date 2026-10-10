// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independently attested loader transfer before unresolved callback widening.

use super::{
    Arc, BTreeMap, InvocationFacts, ModuleCommandBindings, SourceCommandKey,
    SourceExecutionContext, SourceNativeInvocation, SourceOutcomes, StateTransition,
    TrustedPackageLoader,
};
use tcl_registry::model::binding::PackageTransition;

/// A complete selected provider transfer in the original source model.
/// Its retained table is separate from physical interpreter and CPP receipts.
pub(super) struct OriginalTrustedPackageTransfer {
    state: Box<ModuleCommandBindings>,
}

impl OriginalTrustedPackageTransfer {
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        // Proof: naming.package.attested-source-loader-transfer
        // docs/design/analysis/name-resolution-proofs/package-attested-source-loader-transfer.md
        if state.has_opaque_domain()
            || state.baseline.unknown_entry
            || state.baseline.native_entry.is_some()
            || state.current_source_origin.is_none()
            || state.source_step_observed()
            || state.source_execution_observed(None)
            || state.source_variables.dynamic_traces
            || !state.source_variables.traced.is_empty()
            || !state.source_variables.untracked_traces.is_empty()
            || !state
                .source_variables
                .possible_trace_registrations
                .is_empty()
            || !facts.arg_roles_complete
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || native.invocation.arguments().exact_argv_len().is_none()
            || native.invocation.arguments().dialect() != state.baseline.dialect
            || Some(context.registry.snapshot().semantic_key()) != state.baseline.registry_snapshot
        {
            return None;
        }
        let mut requires = facts
            .state_transitions
            .declared()?
            .facts()
            .iter()
            .filter_map(|fact| match &fact.transition {
                StateTransition::Package(transition @ PackageTransition::Require { .. }) => {
                    Some(transition)
                }
                _ => None,
            });
        let transition = requires.next()?;
        if requires.next().is_some() {
            return None;
        }
        let PackageTransition::Require {
            package,
            requirements,
            exact,
        } = transition
        else {
            return None;
        };
        let package = package.literal()?;
        let loader = state.baseline.trusted_loaders.get(package)?;
        if loader.required_core_family != Some(tcl_dialect::model::Family::Tcl)
            || state
                .packages
                .get(package)
                .is_some_and(|provisions| provisions.iter().any(|provision| provision.present))
            || !super::trusted_loader_is_selected(state, loader, requirements, *exact)
        {
            return None;
        }
        let mut selected = super::boxed_source_branch(state);
        super::apply_package_transition(&mut selected, transition);
        if selected.loaded_provider(package) != Some(loader)
            || !selected.provider_surface_is_live(loader)
            || selected.has_opaque_domain()
        {
            return None;
        }
        let variables = Arc::make_mut(&mut selected.source_variables);
        variables.namespace_identities = selected.namespaces.iter().cloned().collect();
        variables.known_namespaces = selected
            .namespaces
            .iter()
            .filter_map(super::SourceNamespaceKey::advisory_key)
            .collect();
        // Installed provider state is independent of incoming variable values.
        // It supplies neither a captured variable receiver nor a literal value.
        variables.widen_deferred_namespace_inputs();
        Some(Self { state: selected })
    }

    pub(super) fn finish(
        self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        // Proof: naming.package.attested-source-loader-transfer
        // docs/design/analysis/name-resolution-proofs/package-attested-source-loader-transfer.md
        *state = *self.state;
        let mut outcomes = SourceOutcomes::normal(state);
        outcomes.retain_closed_native_handler_completion(native, facts, context);
        outcomes
    }
}

fn retain_provider_geometry(
    state: &mut ModuleCommandBindings,
    loader: &TrustedPackageLoader,
) -> Option<()> {
    let mut world = (*state.original_command_world).clone();
    world.retain_trusted_provider_namespaces(state, loader)?;
    state.original_command_world = Arc::new(world);
    Some(())
}

pub(super) fn preload_dependencies_hold(
    state: &ModuleCommandBindings,
    loader: &TrustedPackageLoader,
) -> bool {
    // Proof: naming.package.attested-source-loader-transfer
    // docs/design/analysis/name-resolution-proofs/package-attested-source-loader-transfer.md
    if state
        .baseline
        .execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        .is_none()
    {
        return state.provider_lookup_dependencies_are_live(loader.lookup_dependencies.iter());
    }
    let mut activation = super::boxed_source_branch(state);
    retain_provider_geometry(&mut activation, loader).is_some()
        && activation.provider_lookup_dependencies_are_live(loader.lookup_dependencies.iter())
}

pub(super) fn publication_keys(
    state: &mut ModuleCommandBindings,
    loader: &TrustedPackageLoader,
) -> Option<BTreeMap<String, SourceCommandKey>> {
    // Proof: naming.package.attested-source-loader-transfer
    // docs/design/analysis/name-resolution-proofs/package-attested-source-loader-transfer.md
    let commands = loader
        .command_surface
        .iter()
        .chain(&loader.optional_command_surface);
    let Some(policy) = state
        .baseline
        .execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
    else {
        return Some(
            commands
                .map(|command| (command.clone(), super::nqn(command).into()))
                .collect(),
        );
    };
    retain_provider_geometry(state, loader)?;
    commands
        .map(|command| {
            let slot = policy
                .recipe()
                .command_c_api_publication_slot(
                    tcl_syntax::naming::NativeNameContext::root(),
                    command.as_bytes(),
                )
                .ok()?;
            Some((
                command.clone(),
                state.original_command_key_for_slot(&slot, policy)?,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};

    fn inventory(source: &str, profile: &str, retain: bool) -> SourceCommandBindings {
        let context = tcl_registry::model::ingress::static_context_for(profile);
        let registry = context.commands();
        let loader = crate::lowering::stock_body_provider_loader(
            tcl_registry::body_execution::TCLTEST_STOCK_PROVIDER,
            None,
        )
        .unwrap();
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry,
            SourceAnalysisOptions {
                invocation_dialect: registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile),
                trusted_package_loaders: if retain {
                    std::slice::from_ref(&loader)
                } else {
                    &[]
                },
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        )
    }

    #[test]
    fn attested_provider_entry_keeps_original_qualified_and_imported_slots() {
        // Implementation contract: naming.package.attested-source-loader-transfer
        // docs/design/analysis/name-resolution-proofs/package-attested-source-loader-transfer.md
        let source = "package require tcltest; namespace import ::tcltest::test; list done";
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            // The selected package transfer certifies this completed prefix.
            // A subsequent normal branch needs its own completion receipt.
            let completed = inventory("package require tcltest", profile, true);
            assert!(
                completed.original_completed_command_world().is_some(),
                "{profile}"
            );
            let bindings = inventory(source, profile, true);
            let offset = u32::try_from(source.rfind("list").unwrap()).unwrap();
            let point = bindings.invocation_at_source("list", offset);
            assert!(
                point.lookup_command_word("list").proved_target().is_some(),
                "{profile}: {point:#?}"
            );
            // An import retains an abrupt alternative. Its authentic Normal-path
            // name receipts do not certify completion of the whole root script.
            assert!(
                bindings.original_completed_command_world().is_none(),
                "{profile}"
            );
            for head in ["::tcltest::test", "test"] {
                let selected = point.lookup_command_word(head);
                assert!(
                    selected.proved_target().is_some_and(
                        |target| target.registry_backed && target.command == "::tcltest::test"
                    ),
                    "{profile}/{head}: {selected:#?}"
                );
            }
        }
    }

    #[test]
    fn attested_provider_transfer_declines_unknown_revoked_and_shadowed_preludes() {
        // Implementation contract: naming.package.attested-source-loader-transfer
        // docs/design/analysis/name-resolution-proofs/package-attested-source-loader-transfer.md
        for prefix in [
            "unknown_before; ",
            "proc ::catch args {}; ",
            "namespace eval ::tcltest {proc catch args {}}; ",
            "package ifneeded tcltest 2.5 {}; ",
            "package provide tcltest 2.5; ",
            "package unknown changed_loader; ",
        ] {
            let source = format!("{prefix}package require tcltest; list done");
            let bindings = inventory(&source, "tcl8.6", true);
            assert!(
                bindings.original_completed_command_world().is_none(),
                "{prefix}"
            );
            assert!(
                bindings
                    .loaded_implementation_at(
                        "tcltest",
                        u32::try_from(source.rfind("list").unwrap()).unwrap()
                    )
                    .is_none(),
                "{prefix}"
            );
        }
        // Missing attestation cannot certify even the package-only prefix.
        let source = "package require tcltest";
        let bindings = inventory(source, "tcl8.6", false);
        assert!(bindings.original_completed_command_world().is_none());
    }
}
