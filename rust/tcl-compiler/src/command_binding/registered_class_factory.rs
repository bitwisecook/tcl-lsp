// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Selected stock factory/support allocations, independent of source bodies.

use super::{
    ModuleCommandBindings, SourceCommandTarget, SourceExecutionContext, SourceNativeInvocation,
};
use tcl_registry::native_tcloo_bootstrap::NativeClassFactoryRecipe;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalClassFactoryState {
    recipe: NativeClassFactoryRecipe,
    factory: SourceCommandTarget,
    support: Option<SourceCommandTarget>,
    native: Option<super::native_class_factory_roles::NativeClassFactoryState>,
}

impl OriginalClassFactoryState {
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        let factory = native.target;
        if state.baseline.unknown_entry
            || state.has_opaque_domain()
            || !factory.registry_backed
            || factory.kind != super::BindingKind::Builtin
            || !factory.prepended.is_empty()
            || factory.implementation_generation != 0
            || factory.runtime_implementation_generation
                != state.runtime_implementation_generation(factory.identity.as_ref())
            || !state.retained_target_is_current(factory)
            || state.tainted_object_dispatch.contains("*")
            || state.tainted_object_dispatch.contains(&factory.command)
        {
            return None;
        }
        let dialect = state.baseline.dialect?;
        let recipe = context.registry.native_class_factory_recipe(
            factory.registry_identity()?,
            dialect,
            context.realm,
        )?;
        if !state.default_construction_dependencies_hold(recipe.grammar()) {
            return None;
        }
        let (native_state, support) = if state.baseline.native_entry.is_some() {
            let selector = native
                .original_variable_operands
                .input(0, &state.source_variables)?;
            let (native, support) =
                super::native_class_factory_roles::NativeClassFactoryState::capture(
                    factory, selector, recipe, state,
                )?;
            (Some(native), support)
        } else {
            (
                None,
                match recipe.support() {
                    Some(support) => {
                        let root = state.source_root_namespace_key()?;
                        let policy = state.baseline.execution_name_policy?.native_recipe()?;
                        let raw = state.original_registry_metadata_target(
                            &root,
                            support.command(),
                            policy,
                        )?;
                        let target = SourceCommandTarget {
                            runtime_implementation_generation: state
                                .runtime_implementation_generation(raw.token.as_ref()),
                            command: raw.command,
                            prepended: raw.prepended,
                            original_prepended: None,
                            registry_backed: raw.registry_backed,
                            kind: raw.kind,
                            identity: raw.token,
                            implementation_generation: raw.implementation_generation,
                            implementation_allocation: raw.implementation_allocation,
                        };
                        if !(target.kind == super::BindingKind::Builtin
                            && target.registry_backed
                            && target.prepended.is_empty()
                            && target.implementation_generation == 0
                            && target.runtime_implementation_generation.is_none()
                            && target.registry_identity() == Some(support.command())
                            && state.retained_target_is_current(&target)
                            && !state.tainted_object_dispatch.contains(&target.command))
                        {
                            return None;
                        }
                        Some(target)
                    }
                    None => None,
                },
            )
        };
        Some(Self {
            recipe,
            factory: factory.clone(),
            support,
            native: native_state,
        })
    }

    pub(super) fn recipe(&self) -> NativeClassFactoryRecipe {
        self.recipe
    }

    pub(super) fn is_current(&self, state: &ModuleCommandBindings) -> bool {
        !state.has_opaque_domain()
            && self.native.is_some() == state.baseline.native_entry.is_some()
            && self
                .native
                .as_ref()
                .is_none_or(|native| native.is_current(state))
            && !state.tainted_object_dispatch.contains("*")
            && state.retained_target_is_current(&self.factory)
            && !state
                .tainted_object_dispatch
                .contains(&self.factory.command)
            && self.support.as_ref().is_none_or(|support| {
                state.retained_target_is_current(support)
                    && !state.tainted_object_dispatch.contains(&support.command)
                    && self.recipe.support().is_some_and(|recipe| {
                        recipe.own_constructor_absent() && recipe.own_destructor_absent()
                    })
            })
    }
}

#[cfg(test)]
mod tests {
    fn analyse(
        source: &str,
        version: tcl_dialect::TclVersion,
    ) -> super::super::SourceCommandBindings {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
        super::super::SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
        )
    }

    #[test]
    fn original_configurable_class_retains_the_selected_support_allocation() {
        // Implementation contract: naming.tcloo.original-configurable-factory-support
        // docs/design/analysis/name-resolution-proofs/tcloo-original-configurable-factory-support.md
        for version in [tcl_dialect::TclVersion::V9_0, tcl_dialect::TclVersion::V9_1] {
            let source = "oo::configurable create C {property readable -kind readable writable -kind writable}; C create object; object configure";
            let bindings = analyse(source, version);
            let point = bindings.invocation_at_source(
                "object",
                u32::try_from(source.rfind("object configure").unwrap()).unwrap(),
            );
            let receiver = point
                .named_object_instance_at_dispatch()
                .expect("actual source manufacture receipt");
            let state = &point.lookup_state.as_ref().unwrap().state;
            let definition = state
                .class_definitions
                .get(receiver.class_target().identity.as_ref().unwrap())
                .unwrap();
            let factory = definition.original_factory.as_ref().unwrap();
            assert!(factory.is_current(state));
            let support = factory.support.as_ref().unwrap();
            assert_eq!(
                support.registry_identity(),
                Some("::oo::configuresupport::configurable")
            );
            assert!(
                definition.constructor_entry.is_none() && definition.destructor_entry.is_none()
            );
            assert!(factory.recipe().support().unwrap().own_constructor_absent());
            assert!(
                definition
                    .instance_methods
                    .as_ref()
                    .unwrap()
                    .contains(&tcl_core_types::NameBytes::from(b"configure".as_slice()))
            );
        }
    }

    #[test]
    fn original_configurable_existing_classes_keep_moved_support_but_new_factories_reselect() {
        // Implementation contract: naming.tcloo.original-configurable-factory-support
        // docs/design/analysis/name-resolution-proofs/tcloo-original-configurable-factory-support.md
        for version in [tcl_dialect::TclVersion::V9_0, tcl_dialect::TclVersion::V9_1] {
            let source = "oo::configurable create C {}; rename ::oo::configuresupport::configurable ::HeldSupport; C create object; object configure; oo::configurable create Later {}";
            let bindings = analyse(source, version);
            let point = bindings.invocation_at_source(
                "object",
                u32::try_from(source.find("object configure").unwrap()).unwrap(),
            );
            let receiver = point
                .named_object_instance_at_dispatch()
                .expect("retained support allocation survives its name move");
            let state = &point.lookup_state.as_ref().unwrap().state;
            let definition = state
                .class_definitions
                .get(receiver.class_target().identity.as_ref().unwrap())
                .unwrap();
            assert!(
                definition
                    .original_factory
                    .as_ref()
                    .unwrap()
                    .is_current(state)
            );
            let root = state.source_root_namespace_key().unwrap();
            let policy = state
                .baseline
                .execution_name_policy
                .unwrap()
                .native_recipe()
                .unwrap();
            assert!(
                state
                    .original_registry_metadata_target(
                        &root,
                        "::oo::configuresupport::configurable",
                        policy
                    )
                    .is_none()
            );
        }
    }
}
