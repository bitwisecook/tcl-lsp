// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Command mutation receivers captured before their trace callbacks.

use super::{
    Arc, BTreeSet, CommandAllocationSite, CommandIdentity, MayBinding, ModuleCommandBindings,
    ResolvedCommandTarget, SourceCommandBindings, SourceExecutionContext, SourceOutcomes,
};
use tcl_registry::{CommandBindingTransition, TraceOperation};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SourceCommandMutationReceipt {
    pub(super) site: CommandAllocationSite,
    actions: Vec<CapturedCommandMutation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CapturedCommandMutation {
    subject: String,
    slot: super::SourceCommandKey,
    original: ResolvedCommandTarget,
    kind: CapturedMutationKind,
    certainty: CapturedCommandCertainty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CapturedCommandCertainty {
    Unique,
    AllocationFamily,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CapturedMutationKind {
    Delete,
    Move {
        destination: super::SourceCommandKey,
    },
}

impl ModuleCommandBindings {
    pub(super) fn closed_rename_receiver(
        &self,
        facts: &tcl_registry::InvocationFacts,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        if self.has_opaque_domain()
            || facts.analyser_hook != Some(tcl_registry::hooks::AnalyserHookId::Rename)
        {
            return false;
        }
        let Some(transitions) = facts.state_transitions.declared() else {
            return false;
        };
        let mut mutations = transitions.command_bindings();
        let Some(mutation) = mutations.next() else {
            return false;
        };
        if mutations.next().is_some() {
            return false;
        }
        let (subject, destination) = match mutation {
            CommandBindingTransition::Delete {
                interpreter: None,
                name,
            } => (name.literal(), None),
            CommandBindingTransition::Move { from, to } => (from.literal(), to.literal()),
            _ => return false,
        };
        let Some(subject) = subject else {
            return false;
        };
        let retained = self
            .command_observers
            .pending_mutations
            .iter()
            .any(|receipt| {
                self.current_source_origin.as_ref() == Some(&receipt.site.source)
                    && receipt.site.offset == context.invocation_offset
                    && receipt
                        .actions
                        .iter()
                        .any(|action| action.subject == subject)
            });
        if !retained
            && self
                .captured_command_at(subject, &context.namespace_identity())
                .is_none()
        {
            return false;
        }
        destination.is_none_or(|destination| {
            let Some(destination) = self.publication_key_at(&context.namespace_identity(), destination, super::namespace_slots::PublicationPurpose::Rename) else { return false; };
            if !self.namespaces.contains(destination.holder().as_ref())
                || self.unknown_lookup_namespaces.contains(destination.holder().as_ref()) { return false; }
            self.bindings.get(&destination).map_or_else(
                || !destination.authored_spelling().is_some_and(|spelling| self.baseline.semantics.binding_names().contains(spelling)),
                |bindings| bindings == &BTreeSet::from([MayBinding::Missing]),
            ) || self.command_observers.pending_mutations.iter().any(|receipt| {
                self.current_source_origin.as_ref() == Some(&receipt.site.source)
                    && receipt.site.offset == context.invocation_offset
                    && receipt.actions.iter().any(|action| {
                        action.subject == subject
                            && matches!(&action.kind, CapturedMutationKind::Move { destination: moved } if moved == &destination)
                            && self.bindings.get(&destination).is_some_and(|bindings| {
                                bindings.iter().all(|binding| matches!(binding,
                                    MayBinding::Target(target) if target.token == action.original.token))
                            })
                    })
            })
        })
    }

    fn captured_command_at(
        &self,
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> Option<(super::SourceCommandKey, ResolvedCommandTarget)> {
        if self.has_opaque_domain()
            || (!head.starts_with("::")
                && super::NamespaceKeyQuery::namespace_key(namespace)
                    .native_context()
                    .is_none()
                && !self.source_variables.namespace_known)
        {
            return None;
        }
        for slot in self.source_keys(head, namespace) {
            let alternatives = self.binding_alternatives(&slot);
            if alternatives.len() != 1 {
                return None;
            }
            match alternatives.first()? {
                MayBinding::Missing => {}
                MayBinding::Target(target) if target.token.is_some() => {
                    return Some((slot, target.clone()));
                }
                _ => return None,
            }
        }
        None
    }

    pub(super) fn command_token_replaced_on_install(
        &self,
        token: &CommandIdentity,
        slot: &(impl super::namespace_context::CommandKeyQuery + ?Sized),
    ) -> bool {
        let slot = super::namespace_context::CommandKeyQuery::command_key(slot);
        self.command_observers
            .pending_mutations
            .iter()
            .any(|receipt| {
                receipt.actions.iter().any(|action| {
                    action.original.token.as_ref() == Some(token)
                        && match &action.kind {
                            CapturedMutationKind::Delete => true,
                            CapturedMutationKind::Move { destination } => {
                                destination == slot.as_ref()
                            }
                        }
                })
            })
    }

    /// Retire one object; callbacks may already have published another object
    /// under the same lookup spelling or moved their replacement elsewhere.
    fn retire_command_instance(&mut self, token: &CommandIdentity) {
        Arc::make_mut(&mut self.objects).remove(token);
        let changed = self
            .bindings
            .iter()
            .filter_map(|(slot, alternatives)| {
                let mut retained = alternatives.clone();
                retained.retain(|binding| match binding {
                    MayBinding::Target(target) => target.token.as_ref() != Some(token),
                    MayBinding::Imported(imported) => {
                        &imported.origin != token && imported.compiler.as_ref() != Some(token)
                    }
                    MayBinding::Missing | MayBinding::Unknown => true,
                });
                if retained == *alternatives {
                    return None;
                }
                if retained.is_empty() {
                    retained.insert(MayBinding::Missing);
                }
                Some((slot.clone(), retained))
            })
            .collect::<Vec<_>>();
        for (slot, alternatives) in changed {
            self.rebound_names.insert(slot.clone());
            self.replace(slot, alternatives);
        }
        self.remove_command_observers_for_token(token);
    }

    // A repeated source allocation represents multiple physical command tokens.
    // Retirement is conditional for that family, including retained imports.
    fn retire_command_allocation_family(&mut self, token: &CommandIdentity) {
        if let Some(objects) = Arc::make_mut(&mut self.objects).get_mut(token) {
            objects.insert(MayBinding::Missing);
        }
        let slots = self
            .bindings
            .iter()
            .filter(|(_, alternatives)| {
                alternatives.iter().any(|binding| match binding {
                    MayBinding::Target(target) => target.token.as_ref() == Some(token),
                    MayBinding::Imported(imported) => {
                        &imported.origin == token || imported.compiler.as_ref() == Some(token)
                    }
                    MayBinding::Missing | MayBinding::Unknown => false,
                })
            })
            .map(|(slot, _)| slot.clone())
            .collect::<Vec<_>>();
        for slot in slots {
            let mut alternatives = self.bindings[&slot].clone();
            alternatives.insert(MayBinding::Missing);
            self.rebound_names.insert(slot.clone());
            self.replace(slot, alternatives);
        }
    }

    pub(super) fn remove_command_observers_for_token(&mut self, token: &CommandIdentity) {
        Arc::make_mut(&mut self.command_observers)
            .registrations
            .retain(|observer| observer.token.as_ref() != Some(token));
    }

    pub(super) fn apply_captured_command_mutation(
        &mut self,
        transition: &CommandBindingTransition,
        declaration: u32,
    ) -> bool {
        let Some(origin) = self.current_source_origin.as_ref() else {
            return false;
        };
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: declaration,
        };
        let subject = match transition {
            CommandBindingTransition::Delete { name, .. } => name.literal(),
            CommandBindingTransition::Move { from, .. } => from.literal(),
            _ => return false,
        };
        let action = self
            .command_observers
            .pending_mutations
            .iter()
            .find(|receipt| receipt.site == site)
            .and_then(|receipt| {
                receipt.actions.iter().find(|action| {
                    Some(action.subject.as_str()) == subject
                        && matches!(
                            (&action.kind, transition),
                            (
                                CapturedMutationKind::Delete,
                                CommandBindingTransition::Delete { .. }
                            ) | (
                                CapturedMutationKind::Move { .. },
                                CommandBindingTransition::Move { .. }
                            )
                        )
                })
            })
            .cloned();
        let Some(action) = action else {
            return false;
        };
        if action.certainty == CapturedCommandCertainty::AllocationFamily {
            self.retire_command_allocation_family(
                action.original.token.as_ref().expect("captured token"),
            );
            return true;
        }
        match action.kind {
            CapturedMutationKind::Delete => {
                self.retire_command_instance(
                    action.original.token.as_ref().expect("captured token"),
                );
            }
            CapturedMutationKind::Move { destination } => {
                // Tcl publishes the destination before Rename traces and
                // removes the captured source hash entry after they finish.
                let token = action.original.token.as_ref().expect("captured token");
                let replaced = self.objects.get(token).is_none_or(|implementations| {
                    implementations.iter().any(|binding| match binding {
                        MayBinding::Target(target) => {
                            target.implementation_generation
                                != action.original.implementation_generation
                                || target.implementation_allocation
                                    != action.original.implementation_allocation
                        }
                        _ => true,
                    })
                });
                if replaced {
                    self.retire_command_instance(token);
                } else {
                    self.rebound_names.insert(action.slot.clone());
                    self.replace(action.slot, BTreeSet::from([MayBinding::Missing]));
                }
                self.rebound_names.insert(destination);
            }
        }
        true
    }

    pub(super) fn captured_move_destination_exists(
        &self,
        facts: &tcl_registry::InvocationFacts,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| {
                transitions.command_bindings().any(|transition| {
                    let tcl_registry::CommandBindingTransition::Move { to, .. } = transition else {
                        return false;
                    };
                    to.literal().is_some_and(|destination| {
                        self.captured_command_at(destination, &context.namespace_identity())
                            .is_some()
                    })
                })
            })
    }

    pub(super) fn begin_command_mutation_receipt(
        &mut self,
        facts: &tcl_registry::InvocationFacts,
        context: SourceExecutionContext<'_>,
    ) -> Option<Arc<SourceCommandMutationReceipt>> {
        let site = CommandAllocationSite {
            source: Arc::clone(self.current_source_origin.as_ref()?),
            offset: context.invocation_offset,
        };
        let mut actions = Vec::new();
        for transition in facts.state_transitions.declared()?.command_bindings() {
            let (subject, namespace, kind, operation) = match transition {
                CommandBindingTransition::Delete { interpreter, name }
                    if interpreter
                        .as_ref()
                        .is_none_or(|path| path.literal() == Some("")) =>
                {
                    (
                        name.literal()?,
                        if interpreter.is_some() {
                            self.native_root_namespace_key()
                                .unwrap_or_else(|| super::SourceNamespaceKey::authored("::"))
                        } else {
                            context.namespace_identity()
                        },
                        CapturedMutationKind::Delete,
                        TraceOperation::Delete,
                    )
                }
                CommandBindingTransition::Move { from, to } => {
                    let destination = self.publication_key_at(
                        &context.namespace_identity(),
                        to.literal()?,
                        super::namespace_slots::PublicationPurpose::Rename,
                    )?;
                    (
                        from.literal()?,
                        context.namespace_identity(),
                        CapturedMutationKind::Move { destination },
                        TraceOperation::Rename,
                    )
                }
                _ => continue,
            };
            let observers = SourceCommandBindings::command_observers_for_event(
                self,
                Some(subject),
                &namespace,
                operation,
            );
            if observers.is_empty() {
                continue;
            }
            if let CapturedMutationKind::Move { destination } = &kind {
                let alternatives = self.binding_alternatives(destination);
                if alternatives != BTreeSet::from([MayBinding::Missing]) {
                    continue;
                }
            }
            let (slot, original) = self.captured_command_at(subject, &namespace)?;
            let token = original.token.as_ref().expect("captured token");
            let certainty = if token.runtime.is_none()
                && token.allocation.as_ref().is_some_and(|allocation| {
                    allocation.incarnation == super::AllocationIncarnation::RepeatedFresh
                }) {
                CapturedCommandCertainty::AllocationFamily
            } else {
                CapturedCommandCertainty::Unique
            };
            actions.push(CapturedCommandMutation {
                certainty,
                subject: subject.to_owned(),
                slot,
                original,
                kind,
            });
        }
        if actions.is_empty() {
            return None;
        }
        let receipt = Arc::new(SourceCommandMutationReceipt { site, actions });
        Arc::make_mut(&mut self.command_observers)
            .pending_mutations
            .push(Arc::clone(&receipt));
        for action in &receipt.actions {
            if let CapturedMutationKind::Move { destination } = &action.kind {
                let mut moved = action.original.clone();
                if !moved.registry_backed
                    && moved.terminal
                    && let Some(spelling) = self.callable_spelling_for_key(destination)
                {
                    moved.command = spelling;
                }
                let binding = MayBinding::Target(moved);
                self.replace(destination.clone(), BTreeSet::from([binding.clone()]));
                self.replace(action.slot.clone(), BTreeSet::from([binding.clone()]));
                if let Some(token) = &action.original.token {
                    Arc::make_mut(&mut self.objects)
                        .insert(token.clone(), BTreeSet::from([binding]));
                }
            }
        }
        Some(receipt)
    }
}

impl SourceCommandBindings {
    pub(super) fn finish_command_mutation_receipt(
        outcomes: &mut SourceOutcomes,
        receipt: Option<&Arc<SourceCommandMutationReceipt>>,
    ) {
        let Some(receipt) = receipt else {
            return;
        };
        for branch in outcomes
            .normal
            .iter_mut()
            .chain(outcomes.abrupt.iter_mut().map(|(_, state)| state))
        {
            Arc::make_mut(&mut branch.command_observers)
                .pending_mutations
                .retain(|active| active != receipt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SourceCommandBindings;
    use crate::command_binding::{SourceAnalysisOptions, SourceInvocationBinding};
    use tcl_registry::native_compilation::{NativeCompilationContext, NativeCompilationMode};

    fn analyse(source: &str) -> SourceCommandBindings {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    fn last_call(
        bindings: &SourceCommandBindings,
        source: &str,
        head: &str,
    ) -> SourceInvocationBinding {
        bindings.invocation_at_source(head, u32::try_from(source.rfind(head).unwrap()).unwrap())
    }

    #[test]
    fn repeated_factory_replacement_is_not_strongly_retired() {
        for source in [
            "proc install {} {proc victim {} {return NEW}}; install; install; install; proc cb args {install}; trace add command victim delete cb; rename victim {}; victim",
            "proc install {} {proc victim {} {return NEW}}; install; rename victim {}; install; rename victim {}; install; proc cb args {install}; trace add command victim delete cb; rename victim {}; victim",
        ] {
            let bindings = analyse(source);
            let replacement = last_call(&bindings, source, "victim");
            assert!(
                replacement.unknown || !replacement.targets.is_empty(),
                "{replacement:#?}"
            );
            assert!(!bindings.final_state.definitely_absent("victim", "::"));
        }
    }

    #[test]
    fn deletion_retires_the_captured_object_and_preserves_callback_replacement() {
        let source = "proc victim {} {return OLD}; victim; proc cb args {proc victim {} {return NEW}}; trace add command victim delete cb; rename victim {}; victim";
        let bindings = analyse(source);
        let before = bindings.invocation_at_source(
            "victim",
            u32::try_from(source.find("; victim;").unwrap() + 2).unwrap(),
        );
        let after = last_call(&bindings, source, "victim");
        let original = before.proved_handler_target().unwrap();
        let replacement = after.proved_handler_target().unwrap();
        assert_ne!(original.identity, replacement.identity);
        assert_ne!(
            original.implementation_allocation,
            replacement.implementation_allocation
        );
    }

    #[test]
    fn deletion_does_not_retire_a_replacement_moved_by_the_callback() {
        let source = "proc victim {} {return OLD}; proc cb args {proc victim {} {return NEW}; rename victim kept}; trace add command victim delete cb; rename victim {}; kept";
        let bindings = analyse(source);
        let binding = last_call(&bindings, source, "kept");
        if binding.proved_handler_target().is_none() {
            for point in &bindings.points {
                eprintln!(
                    "site={} head={:?} opaque={} victim={:?} kept={:?}",
                    point.offset,
                    point.head,
                    point.state.has_opaque_domain(),
                    point.state.bindings.get("::victim"),
                    point.state.bindings.get("::kept")
                );
            }
        }
        assert!(
            binding.proved_handler_target().is_some(),
            "lookup_unknown={} absent={} targets={:?}",
            binding.unknown,
            binding.may_be_absent,
            binding.targets
        );
    }

    #[test]
    fn replacement_has_an_independent_delete_registration() {
        let source = "proc victim {} {return OLD}; proc later args {rename llength savedLength}; proc cb args {proc victim {} {return NEW}; trace add command victim delete later}; trace add command victim delete cb; rename victim {}; llength {a}; rename victim {}; llength {a}";
        let bindings = analyse(source);
        let before = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.find("; llength {a}").unwrap() + 2).unwrap(),
        );
        assert!(before.proved_handler_target().is_some());
        assert!(
            last_call(&bindings, source, "llength")
                .proved_handler_target()
                .is_none_or(|target| target.command != "::llength")
        );
    }

    #[test]
    fn rename_publishes_the_captured_destination_before_callbacks() {
        let source = "proc victim {} {return OLD}; proc cb args {moved}; trace add command victim rename cb; rename victim moved; moved";
        let bindings = analyse(source);
        let callback = bindings.invocation_at_source(
            "moved",
            u32::try_from(source.find("{moved}").unwrap() + 1).unwrap(),
        );
        assert!(callback.proved_handler_target().is_some());
        assert!(
            last_call(&bindings, source, "moved")
                .proved_handler_target()
                .is_some()
        );
    }

    #[test]
    fn rename_source_replacement_retires_both_captured_lookup_entries() {
        let source = "proc victim {} {return OLD}; proc cb args {proc victim {} {return NEW}}; trace add command victim rename cb; rename victim moved; moved";
        let bindings = analyse(source);
        assert!(
            last_call(&bindings, source, "moved")
                .proved_handler_target()
                .is_none()
        );
    }

    #[test]
    fn existing_destination_rejects_rename_before_its_callback() {
        let source = "proc victim {} {}; proc moved {} {}; proc cb args {rename llength savedLength}; trace add command victim rename cb; catch {rename victim moved}; llength {a}";
        let bindings = analyse(source);
        assert!(
            last_call(&bindings, source, "llength")
                .proved_handler_target()
                .is_some()
        );
    }
}
