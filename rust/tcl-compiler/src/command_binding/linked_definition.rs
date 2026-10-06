// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Terminal navigation after an actual, unobserved alias/import lookup.

use super::{
    CommandIdentity, MayBinding, ModuleCommandBindings, ResolvedCommandTarget,
    SourceCommandDefinition, SourceInvocationBinding, SourceRuntimeReachability,
};
use std::collections::BTreeSet;

impl SourceInvocationBinding {
    /// Definition selected by this actual evaluated head after arguments.
    /// Called alias/import identity remains available through `command_reference`.
    /// Missing targets, fallback handlers and observed wrapper/target dispatch
    /// cannot mint a linked definition from a hypothetical pre-wrapper lookup.
    #[must_use]
    pub fn linked_definition(&self, word: &str) -> Option<SourceCommandDefinition> {
        if self.runtime_reachability() != SourceRuntimeReachability::Reached
            || self.evaluated_command_word() != Some(word)
            || self.entered_execution_observer.observed()
        {
            return None;
        }
        let state = &self.lookup_state.as_ref()?.state;
        let implementation =
            state.linked_definition_implementation(word, &self.lookup_namespace_key)?;
        state.source_definition_for_implementation(implementation)
    }
}

impl ModuleCommandBindings {
    fn linked_definition_implementation<'a>(
        &'a self,
        word: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> Option<&'a ResolvedCommandTarget> {
        let mut word = word.to_owned();
        let mut namespace = super::NamespaceKeyQuery::namespace_key(namespace).into_owned();
        let mut visiting = BTreeSet::<CommandIdentity>::new();
        loop {
            if !self.source_lookup_is_closed(&word, &namespace) || self.source_step_observed() {
                return None;
            }
            let keys = self.source_keys(&word, &namespace);
            let [slot] = keys.as_slice() else {
                return None;
            };
            // Every edge must have an actual singleton slot; catalogue-only
            // entries carry no source declaration allocation.
            let bindings = self.bindings.get(slot)?;
            if bindings.len() != 1 {
                return None;
            }
            let target = match bindings.first()? {
                MayBinding::Target(target) => target,
                MayBinding::Imported(identity) => {
                    let implementations = self.objects.get(&identity.origin)?;
                    if implementations.len() != 1 {
                        return None;
                    }
                    let MayBinding::Target(target) = implementations.first()? else {
                        return None;
                    };
                    target
                }
                MayBinding::Missing | MayBinding::Unknown => return None,
            };
            let identity = target.token.as_ref()?;
            if self.source_execution_observed(Some(identity))
                || self.runtime_execution_observed(Some(identity))
                || !visiting.insert(identity.clone())
            {
                return None;
            }
            if target.terminal {
                return Some(target);
            }
            namespace = self.alias_target_namespace_key(target, &namespace);
            target.command.clone_into(&mut word);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn final_invocation(source: &str) -> SourceInvocationBinding {
        let registry = tcl_registry::CommandRegistry::build_default();
        let bindings = super::super::SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::default(),
            &registry,
        );
        let offset = u32::try_from(source.rfind("bridge VALUE").unwrap()).unwrap();
        bindings.invocation_at_source("", offset)
    }

    #[test]
    fn frozen_alias_prefix_links_to_the_actual_source_procedure() {
        let source = "proc target {fixed value} {return RESULT}; interp alias {} bridge {} target FIXED; bridge VALUE";
        let binding = final_invocation(source);
        let reference = binding.command_reference("bridge").unwrap();
        assert!(reference.definition().is_none());
        let definition = binding.linked_definition("bridge").unwrap();
        assert_eq!(
            definition.kind(),
            super::super::SourceCommandDefinitionKind::Procedure
        );
        assert_eq!(definition.allocation().site.offset, 0);
        assert!(binding.linked_definition("target").is_none());
    }

    #[test]
    fn long_retained_alias_chain_uses_constant_stack() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let analysed = super::super::SourceCommandBindings::analyse(
            "proc target args {return RESULT}; interp alias {} bridge {} target",
            tcl_lexer::LexerConfig::default(),
            &registry,
        );
        let mut state = analysed.final_state.as_ref().clone();
        let MayBinding::Target(prototype) = state.bindings["::bridge"].first().unwrap() else {
            panic!("actual installed alias");
        };
        let prototype = prototype.clone();
        let bindings = super::super::Arc::make_mut(&mut state.bindings);
        for index in 0..4096 {
            let slot = format!("::chain{index}");
            let mut alias = prototype.clone();
            alias.token.as_mut().unwrap().origin.clone_from(&slot);
            alias.command = if index == 0 {
                "::target".to_owned()
            } else {
                format!("::chain{}", index - 1)
            };
            bindings.insert(slot, BTreeSet::from([MayBinding::Target(alias)]));
        }
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(move || {
                let target = state
                    .linked_definition_implementation("::chain4095", "::")
                    .unwrap();
                assert_eq!(target.command, "::target");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn missing_or_observed_alias_targets_cannot_borrow_a_linked_definition() {
        for source in [
            "proc unknown args {return CUSTOM}; interp alias {} bridge {} missing; bridge VALUE",
            "proc target args {return OLD}; proc mutate args {rename target {}}; interp alias {} bridge {} target; trace add execution target enter mutate; bridge VALUE",
            "proc target args {return OLD}; proc mutate args {rename target {}}; interp alias {} bridge {} target; trace add execution bridge enter mutate; bridge VALUE",
            "proc target args {return OLD}; interp alias {} bridge {} target; rename target {}; bridge VALUE",
        ] {
            assert!(
                final_invocation(source)
                    .linked_definition("bridge")
                    .is_none(),
                "{source}"
            );
        }
    }
}
