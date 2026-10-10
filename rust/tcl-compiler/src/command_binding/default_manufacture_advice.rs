// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original default manufacture shape, independent of allocation completion.

use super::{
    BindingKind, CommandAllocationSite, SourceCommandDefinition, SourceCommandDefinitionKind,
    SourceCommandTarget, SourceInvocationBinding, SourceMethodReceiver, SourceRuntimeReachability,
};
use crate::ir::CommandTokens;
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_registry::{CommandRegistry, definer::MemberVisibility};

/// One actual original class invocation whose selected default manufacturer
/// has no represented constructor, destructor, filter or initializer body.
/// This supplies source argument-shape advice only. It proves no allocation,
/// Normal completion, native preparation or editable source coverage.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalDefaultManufactureShape {
    definition: SourceCommandDefinition,
    target: SourceCommandTarget,
    site: CommandAllocationSite,
    config: tcl_lexer::LexerConfig,
    inputs: Vec<SignatureSourceNameInput>,
}

impl OriginalDefaultManufactureShape {
    /// Independent current source class implementation selected at this point.
    #[must_use]
    pub const fn class_definition(&self) -> &SourceCommandDefinition {
        &self.definition
    }
}

impl SourceInvocationBinding {
    /// Source shape of a directly selected ordinary default manufacturer.
    /// The complete actual original vector, current class and independently
    /// absent lifecycle handlers are required. Future success remains separate.
    #[must_use]
    pub fn original_default_manufacture_shape(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<OriginalDefaultManufactureShape> {
        if self.runtime_reachability() != SourceRuntimeReachability::Reached
            || tokens.source_binding.as_ref() != Some(self)
            || self.entered_execution_observer.observed()
        {
            return None;
        }
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let head = self.original_head_name_input(tokens)?;
        let inputs = (0..tokens.words().len())
            .map(|ordinal| self.original_written_name_input(tokens, ordinal))
            .collect::<Option<Vec<_>>>()?;
        let policy = head.policy();
        if inputs.first() != Some(&head)
            || inputs.iter().any(|input| {
                input.original_word_key().is_none()
                    || input.policy() != policy
                    || !input.is_current(&self.variable_context)
            })
        {
            return None;
        }
        let state = &self.lookup_state.as_ref()?.state;
        let site = self.invocation_site()?;
        if state.current_source_origin.as_ref() != Some(&site.source)
            || state.baseline.native_entry.is_some()
            || state.baseline.unknown_entry
            || state.has_opaque_domain()
            || state.source_step_observed()
            || !state.command_observers.is_quiet()
            || state.tainted_object_dispatch.contains("*")
            || state.baseline.registry_snapshot.as_ref()
                != Some(&registry.snapshot().semantic_key())
            || state.baseline.execution_name_policy?.native_recipe()? != policy
        {
            return None;
        }
        state.original_namespace_geometry(&self.lookup_namespace_key, policy)?;
        let target = self.proved_execution_target()?;
        if target.kind != BindingKind::Class
            || target.registry_backed
            || !target.prepended.is_empty()
            || !state.retained_target_is_current(target)
            || state.source_execution_observed(target.identity.as_ref())
            || state.runtime_execution_observed(target.identity.as_ref())
            || state.tainted_object_dispatch.contains(&target.command)
        {
            return None;
        }
        let class = state.class_definitions.get(target.identity.as_ref()?)?;
        if class.implementation_generation != target.implementation_generation
            || class.dispatcher.is_some()
            || !class.inherited_classes.is_empty()
            || !class.lifecycle_entries_closed
            || class.constructor_entry.is_some()
            || class.constructor_provider.is_some()
            || class.destructor_entry.is_some()
            || class.destructor_provider.is_some()
            || class.instance_methods.is_none()
            || class
                .receiver_method_entries
                .values()
                .any(|entry| entry.receiver() == SourceMethodReceiver::Class)
            || !state.class_definition_dependencies_hold(class)
        {
            return None;
        }
        let factory = class.original_factory.as_ref()?;
        let dialect = state.baseline.dialect?;
        let recipe = factory.recipe();
        if recipe.support().is_some()
            || recipe.version() != dialect.tcl_version?
            || !factory.is_current(state)
            || config.grammar_over(dialect.lexer_grammar) != dialect.lexer_grammar
        {
            return None;
        }
        let grammar = recipe.grammar();
        grammar.native_constructor_boundary(dialect)?;
        if !state.default_construction_dependencies_hold(grammar) {
            return None;
        }
        let manufacturer =
            grammar.manufacturer(std::str::from_utf8(inputs.get(1)?.bytes()).ok()?)?;
        if manufacturer.visibility != MemberVisibility::Exported
            || inputs.len().checked_sub(1)? < usize::from(manufacturer.constructor_args_from)
        {
            return None;
        }
        let definition = self.original_default_manufacture_definition(target, site)?;
        Some(OriginalDefaultManufactureShape {
            definition,
            target: target.clone(),
            site: site.clone(),
            config,
            inputs,
        })
    }
    fn original_default_manufacture_definition(
        &self,
        target: &SourceCommandTarget,
        site: &CommandAllocationSite,
    ) -> Option<SourceCommandDefinition> {
        let definition = self
            .original_evaluated_command_reference()?
            .definition()?
            .clone();
        if definition.kind() != SourceCommandDefinitionKind::Class
            || Some(definition.allocation()) != target.implementation_allocation.as_ref()
            || definition.allocation().site.source != site.source
        {
            return None;
        }
        Some(definition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn original_tokens(
        bindings: &super::super::SourceCommandBindings,
        source: &str,
    ) -> CommandTokens {
        let offset = u32::try_from(source.rfind("C create").unwrap()).unwrap();
        let point = bindings.invocation_at_source("C", offset);
        let mut tokens = point
            .original_recorded_command_tokens()
            .expect("genuine original vector");
        bindings.stamp_original_tokens(&mut tokens);
        tokens
    }

    #[test]
    fn original_default_manufacture_shape_requires_current_class_and_absent_handlers() {
        // Implementation contract: naming.tcloo.original-default-manufacture-source-shape
        // docs/design/analysis/name-resolution-proofs/tcloo-original-default-manufacture-source-shape.md
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let registry =
                tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
            let source = "oo::class create C {method ping {} {return YES}}; C create object";
            let bindings = analyse(source, version);
            let tokens = original_tokens(&bindings, source);
            let point = tokens.source_binding.as_ref().unwrap();
            let shape = point
                .original_default_manufacture_shape(&tokens, registry)
                .expect("current ordinary default manufacture source shape");
            assert_eq!(
                Some(shape.class_definition()),
                point
                    .original_evaluated_command_reference()
                    .unwrap()
                    .definition(),
            );
            let foreign_registry =
                tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
            assert!(
                point
                    .original_default_manufacture_shape(&tokens, foreign_registry)
                    .is_none()
            );
            let mut synthetic = tokens.clone();
            synthetic.word_exprs.pop();
            assert!(
                point
                    .original_default_manufacture_shape(&synthetic, registry)
                    .is_none()
            );
            for source in [
                "oo::class create C {constructor {} {return}}; C create object",
                "oo::class create C {method observer args {}; filter observer}; C create object",
                "oo::class create C {self method create args {return}}; C create object",
                "oo::class create C {}; C create $unknown",
            ] {
                let bindings = analyse(source, version);
                let tokens = original_tokens(&bindings, source);
                assert!(
                    tokens
                        .source_binding
                        .as_ref()
                        .unwrap()
                        .original_default_manufacture_shape(&tokens, registry)
                        .is_none(),
                    "{version:?}: {source}"
                );
            }
        }
    }
}
