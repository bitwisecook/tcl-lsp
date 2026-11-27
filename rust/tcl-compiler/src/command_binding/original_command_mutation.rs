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

//! Complete unobserved intrinsic command moves at their actual occupied cells.

use super::{
    Arc, BTreeSet, BindingKind, CommandAllocationSite, MayBinding, ModuleCommandBindings,
    ResolvedCommandTarget, SourceCommandKey, SourceExecutionContext, SourceNativeInvocation,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_registry::{CommandBindingTransition, InvocationFacts, StateTransition};

/// Independent intrinsic move premises, captured before the operation. A
/// table/publication row alone grants neither successful completion nor native
/// interpreter execution. Destruction and custom cleanup are separate owners.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalCommandMove {
    site: CommandAllocationSite,
    words: Vec<tcl_source::words::WordExpr>,
    config: tcl_lexer::LexerConfig,
    handler: super::SourceCommandTarget,
    from: SignatureSourceNameInput,
    /// `None` for the delete form.
    to: Option<SignatureSourceNameInput>,
    source: SourceCommandKey,
    destination: Option<SourceCommandKey>,
    binding: MayBinding,
    target: ResolvedCommandTarget,
    imported_origin: Option<SourceCommandKey>,
}

impl OriginalCommandMove {
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        Self::capture_in_view(native, facts, state, context, false)
    }

    /// The delete form for the diagnostic view, which also admits a class or
    /// any builtin. Deleting either may run cleanup that removes further
    /// commands (a class's instances and subclasses, a builtin's own
    /// delete callback), which this receipt does not model: their slots stay
    /// as they were, and cleanup never adds a command, so the view can only
    /// claim the deleted name itself absent. Not a completion receipt.
    pub(super) fn capture_deletion_for_diagnostics(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        Self::capture_in_view(native, facts, state, context, true)
            .filter(|receipt| receipt.destination.is_none())
    }

    fn capture_in_view(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        diagnostic: bool,
    ) -> Option<Self> {
        if state.has_opaque_domain()
            || state.baseline.native_entry.is_some()
            || state.source_step_observed()
            || state.source_execution_observed(None)
            || !state.command_observers.registrations.is_empty()
            || !state.command_observers.pending_mutations.is_empty()
            || facts.analyser_hook != Some(tcl_registry::hooks::AnalyserHookId::Rename)
            || !native.target.registry_backed
            || native.target.kind != BindingKind::Builtin
            || native.target.implementation_generation != 0
            || !native.target.prepended.is_empty()
            || !state.retained_target_is_current(native.target)
            || native.invocation.arguments().exact_argv_len() != Some(2)
            || facts
                .effects
                .accesses()
                .iter()
                .any(|access| access.mode != tcl_registry::world_effect::EffectAccessMode::Read)
        {
            return None;
        }
        let (from, to) = Self::declared_operands(facts)?;
        let from = super::original_command_table::original_operand(native, from, state, context)?;
        if !matches!(
            from.policy().recipe(),
            tcl_syntax::naming::NativeNameProtocol::C(_)
        ) {
            return None;
        }
        let current = context.namespace_identity();
        let (source, occupied) = state.original_occupied_command_for_input(&current, &from)?;
        let (binding, target, imported_origin) =
            Self::intrinsic_occupied_target(state, &source, &occupied, diagnostic)?;
        let to = match to {
            Some(to) => {
                let to =
                    super::original_command_table::original_operand(native, to, state, context)?;
                if from.policy() != to.policy() {
                    return None;
                }
                let scope = state.original_namespace_geometry(&current, to.policy())?;
                let selected = to
                    .policy()
                    .recipe()
                    .rename_destination_input(scope.context()?, to.bytes())
                    .ok()?;
                (!selected.selected().is_empty()).then_some(to)
            }
            None => None,
        };
        let destination = match &to {
            None => {
                let modelled =
                    diagnostic && matches!(target.kind, BindingKind::Class | BindingKind::Builtin);
                if !(modelled
                    || Self::deletion_runs_no_cleanup(state, &current, &source, &binding, &target))
                {
                    return None;
                }
                None
            }
            Some(to) => {
                let destination = state.original_publication_key(
                    &current,
                    to,
                    super::namespace_slots::PublicationPurpose::Rename,
                )?;
                if source == destination
                    || !state.namespaces.contains(destination.holder().as_ref())
                    || state
                        .unknown_lookup_namespaces
                        .contains(destination.holder().as_ref())
                    || state.original_bindings_for_key(&destination)?
                        != BTreeSet::from([MayBinding::Missing])
                {
                    return None;
                }
                Some(destination)
            }
        };
        Some(Self {
            site: CommandAllocationSite {
                source: Arc::clone(state.current_source_origin.as_ref()?),
                offset: native.segment.span.start(),
            },
            words: native.words.to_vec(),
            config: context.config,
            handler: native.target.clone(),
            from,
            to,
            source,
            destination,
            binding: binding.clone(),
            target,
            imported_origin,
        })
    }

    /// The one declared command-binding transition's operands. The registry
    /// states the delete form (`rename NAME {}`) as its own transition; the
    /// move form names a destination operand.
    fn declared_operands(
        facts: &InvocationFacts,
    ) -> Option<(
        &tcl_registry::TransitionSubject,
        Option<&tcl_registry::TransitionSubject>,
    )> {
        let transitions = facts.state_transitions.declared()?;
        let mut bindings = transitions.command_bindings();
        let first = bindings.next()?;
        if bindings.next().is_some() {
            return None;
        }
        let (from, to) = match first {
            CommandBindingTransition::Move { from, to } => (from, Some(to)),
            CommandBindingTransition::Delete {
                interpreter: None,
                name,
            } => (name, None),
            _ => return None,
        };
        if transitions
            .facts()
            .iter()
            .any(|fact| match &fact.transition {
                StateTransition::CommandBinding(transition) => transition != first,
                StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
                    namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
                }) => to.is_none_or(|to| subject.argument_index() != to.argument_index()),
                _ => true,
            })
        {
            return None;
        }
        Some((from, to))
    }

    /// Deleting a procedure's own slot runs no delete callback and leaves
    /// every other slot as it was, provided no import link refers to it: Tcl
    /// removes those links with their origin. The release's unresolved-command
    /// handler is a library procedure the registry describes as a builtin.
    /// Other builtins, objects and imported wrappers keep their own cleanup
    /// owners.
    fn deletion_runs_no_cleanup(
        state: &ModuleCommandBindings,
        current: &super::SourceNamespaceKey,
        source: &SourceCommandKey,
        binding: &MayBinding,
        target: &ResolvedCommandTarget,
    ) -> bool {
        let Some(identity) = target.token.as_ref() else {
            return false;
        };
        let procedure = match target.kind {
            BindingKind::Proc => true,
            BindingKind::Builtin => state
                .baseline
                .dialect
                .and_then(|dialect| {
                    tcl_registry::command_lookup::native_lookup_fallback_policy(
                        dialect,
                        tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
                    )
                })
                .is_some_and(|policy| {
                    state
                        .source_keys(policy.default_handler, current)
                        .contains(source)
                }),
            _ => false,
        };
        procedure
            && matches!(binding, MayBinding::Target(_))
            && !state.bindings.values().any(|alternatives| {
                alternatives.iter().any(|alternative| {
                    matches!(alternative, MayBinding::Imported(imported) if &imported.origin == identity)
                })
            })
    }

    fn intrinsic_occupied_target(
        state: &ModuleCommandBindings,
        source: &SourceCommandKey,
        occupied: &BTreeSet<MayBinding>,
        diagnostic: bool,
    ) -> Option<(MayBinding, ResolvedCommandTarget, Option<SourceCommandKey>)> {
        let targets = occupied.iter().cloned().collect::<Vec<_>>();
        let [binding] = targets.as_slice() else {
            return None;
        };
        let (target, imported_origin) = match binding {
            MayBinding::Target(target) => (target.clone(), None),
            MayBinding::Imported(imported) => {
                // An imported wrapper retains its real command independently.
                // A direct, unique origin closes this bounded graph without
                // resolving an interp alias or borrowing a publication row.
                let implementations = state.objects.get(&imported.origin)?;
                let mut alternatives = implementations.iter();
                let MayBinding::Target(target) = alternatives.next()? else {
                    return None;
                };
                if alternatives.next().is_some() || target.token.as_ref() != Some(&imported.origin)
                {
                    return None;
                }
                let mut origins = state.bindings.iter().filter(|(key, alternatives)| {
                    *key != source && *alternatives == implementations
                });
                let (origin, _) = origins.next()?;
                if origins.next().is_some()
                    || !state.namespaces.contains(origin.holder().as_ref())
                    || state
                        .unknown_lookup_namespaces
                        .contains(origin.holder().as_ref())
                    || state.original_bindings_for_key(origin).as_ref() != Some(implementations)
                {
                    return None;
                }
                (target.clone(), Some(origin.clone()))
            }
            MayBinding::Missing | MayBinding::Unknown => return None,
        };
        // Moving these intrinsic objects does not invoke a delete callback.
        // Interp alias loop checks, object destruction and arbitrary extension
        // command cleanup remain outside this closed handler envelope. An
        // imported wrapper is admitted only with the direct origin above.
        if !target.terminal
            || !target.prepended.is_empty()
            || !(matches!(target.kind, BindingKind::Proc | BindingKind::Builtin)
                || (diagnostic && target.kind == BindingKind::Class))
        {
            return None;
        }
        let identity = target.token.as_ref()?;
        if identity.runtime.is_some()
            || identity.allocation.as_ref().is_some_and(|allocation| {
                allocation.incarnation == super::AllocationIncarnation::RepeatedFresh
            })
        {
            return None;
        }
        if target.kind == BindingKind::Proc {
            let allocation = target.implementation_allocation.as_ref()?;
            if identity.allocation.as_ref() != Some(allocation)
                || state.objects.get(identity).is_some_and(|implementations| {
                    implementations != &BTreeSet::from([MayBinding::Target(target.clone())])
                })
            {
                return None;
            }
        }
        Some((binding.clone(), target, imported_origin))
    }

    pub(super) fn completed(
        &self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        if state.has_opaque_domain()
            || state.baseline.native_entry.is_some()
            || self.config != context.config
            || self.words != native.words
            || self.handler != *native.target
            || self.site.offset != native.segment.span.start()
            || !self.from.is_current(&state.source_variables)
            || self
                .to
                .as_ref()
                .is_some_and(|to| !to.is_current(&state.source_variables))
            || state.current_source_origin.as_ref() != Some(&self.site.source)
            || facts.analyser_hook != Some(tcl_registry::hooks::AnalyserHookId::Rename)
            || state.source_step_observed()
            || state.source_execution_observed(None)
            || !state.command_observers.registrations.is_empty()
            || !state.command_observers.pending_mutations.is_empty()
        {
            return false;
        }
        let expected_target = BTreeSet::from([MayBinding::Target(self.target.clone())]);
        let origin_current =
            if let Some(origin) = &self.imported_origin {
                state.namespaces.contains(origin.holder().as_ref())
                    && !state
                        .unknown_lookup_namespaces
                        .contains(origin.holder().as_ref())
                    && state.original_bindings_for_key(origin).as_ref() == Some(&expected_target)
                    && self.target.token.as_ref().is_some_and(|identity| {
                        state.objects.get(identity) == Some(&expected_target)
                    })
            } else {
                self.target.kind != BindingKind::Proc
                    || self.target.token.as_ref().is_some_and(|identity| {
                        state
                            .objects
                            .get(identity)
                            .is_none_or(|implementations| implementations == &expected_target)
                    })
            };
        state.original_bindings_for_key(&self.source) == Some(BTreeSet::from([MayBinding::Missing]))
            && self.destination.as_ref().is_none_or(|destination| {
                state.original_bindings_for_key(destination)
                    == Some(BTreeSet::from([self.binding.clone()]))
            })
            && (self.destination.is_none() || origin_current)
    }
}

#[cfg(test)]
mod tests {

    fn analyse(
        source: &str,
        version: tcl_dialect::TclVersion,
    ) -> super::super::SourceCommandBindings {
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_profile_name())
                .commands();
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        super::super::SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    #[test]
    fn original_command_move_retains_the_allocation_in_a_complete_normal_world() {
        // Implementation contract: naming.command.original-occupied-mutation-transfer
        // docs/design/analysis/name-resolution-proofs/original-occupied-mutation-transfer.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = "proc P {} {}; rename P Q";
            let bindings = analyse(source, version);
            let completed = bindings
                .original_completed_command_world()
                .unwrap_or_else(|| panic!("{}", version.dialect_profile_name()));
            let entries = completed.declarations().collect::<Vec<_>>();
            let [entry] = entries.as_slice() else {
                panic!("one moved source publication")
            };
            assert_eq!(entry.slot().simple.as_bytes(), b"Q");
            assert_eq!(entry.declaration_site().offset, 0);
            assert_eq!(entry.definition().unwrap().allocation().site.offset, 0);
        }
    }

    #[test]
    fn original_byte_command_move_keeps_the_original_definition_and_complete_world() {
        // Implementation contract: naming.command.original-byte-rename-transition
        // docs/design/analysis/name-resolution-proofs/command-original-byte-rename-transition.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = r"proc P\uD800 {} {}; rename P\uD800 Q\uD801";
            let bindings = analyse(source, version);
            let completed = bindings
                .original_completed_command_world()
                .unwrap_or_else(|| panic!("{}", version.dialect_profile_name()));
            let entries = completed.declarations().collect::<Vec<_>>();
            let [entry] = entries.as_slice() else {
                panic!("one moved opaque publication")
            };
            assert_eq!(entry.slot().simple.as_bytes(), b"Q\xed\xa0\x81");
            assert_eq!(entry.declaration_site().offset, 0);
            assert_eq!(entry.definition().unwrap().allocation().site.offset, 0);
            assert_eq!(entry.name_input().bytes(), b"Q\xed\xa0\x81");
        }
    }

    #[test]
    fn original_imported_command_move_keeps_the_wrapper_and_direct_origin() {
        // Implementation contract: naming.command.original-occupied-mutation-transfer
        // docs/design/analysis/name-resolution-proofs/original-occupied-mutation-transfer.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = r"rename auto_import savedPrelude; namespace eval A {proc p\uD800 {} {}; namespace export p\uD800}; namespace eval B {namespace import ::A::*; rename p\uD800 moved}; set checkpoint READY";
            let bindings = analyse(source, version);
            let completed = bindings
                .original_completed_command_world()
                .unwrap_or_else(|| panic!("{}", version.dialect_profile_name()));
            let publications = completed.declarations().collect::<Vec<_>>();
            let imports = publications
                .iter()
                .filter(|publication| {
                    publication.kind() == super::super::OriginalCommandPublicationKind::Imported
                })
                .collect::<Vec<_>>();
            let [imported] = imports.as_slice() else {
                panic!("one moved import");
            };
            assert_eq!(imported.slot().simple.as_bytes(), b"moved");
            assert_eq!(
                imported
                    .slot()
                    .namespace
                    .as_segments()
                    .iter()
                    .map(tcl_core_types::NameBytes::as_bytes)
                    .collect::<Vec<_>>(),
                vec![b"B".as_slice()]
            );
            let origin = imported.imported_origin().unwrap();
            let source = publications
                .iter()
                .find(|publication| {
                    publication.kind() == super::super::OriginalCommandPublicationKind::Procedure
                        && publication.declaration_site() == origin
                })
                .unwrap();
            assert_eq!(source.slot().simple.as_bytes(), b"p\xed\xa0\x80");
            assert_eq!(imported.definition().unwrap(), source.definition().unwrap());
        }
    }

    #[test]
    fn original_imported_command_move_keeps_origin_and_observer_barriers() {
        // Implementation contract: naming.command.original-occupied-mutation-transfer
        // docs/design/analysis/name-resolution-proofs/original-occupied-mutation-transfer.md
        for version in tcl_dialect::TclVersion::ALL {
            for body in [
                r"proc moved {} {}; rename p\uD800 moved",
                r"trace add command p\uD800 rename {list}; rename p\uD800 moved",
                r"rename p\uD800 {}",
            ] {
                let source = format!(
                    r"rename auto_import savedPrelude; namespace eval A {{proc p\uD800 {{}} {{}}; namespace export p\uD800}}; namespace eval B {{namespace import ::A::*; {body}}}"
                );
                assert!(
                    analyse(&source, version)
                        .original_completed_command_world()
                        .is_none(),
                    "{}: {body}",
                    version.dialect_profile_name()
                );
            }
        }
    }

    #[test]
    fn deleting_an_unobserved_procedure_keeps_the_complete_normal_world() {
        // Every release: `proc P {} {}; rename P {}; P` raises
        // `invalid command name "P"`, and no other command changes.
        for version in tcl_dialect::TclVersion::ALL {
            let bindings = analyse("proc P {} {}; proc Q {} {}; rename P {}", version);
            let completed = bindings
                .original_completed_command_world()
                .unwrap_or_else(|| panic!("{}", version.dialect_profile_name()));
            let names = completed
                .declarations()
                .map(|entry| entry.slot().simple.as_bytes().to_vec())
                .collect::<Vec<_>>();
            assert_eq!(names, [b"Q".to_vec()], "{}", version.dialect_profile_name());
        }
    }

    #[test]
    fn original_command_move_does_not_close_missing_occupied_or_observed_targets() {
        // Implementation contract: naming.command.original-occupied-mutation-transfer
        // docs/design/analysis/name-resolution-proofs/original-occupied-mutation-transfer.md
        for version in tcl_dialect::TclVersion::ALL {
            for source in [
                "rename absent Q",
                "proc P {} {}; proc Q {} {}; rename P Q",
                "proc P {} {}; trace add command P rename {list}; rename P Q",
                "proc P {} {}; trace add command P delete {list}; rename P {}",
                "rename set {}",
            ] {
                let bindings = analyse(source, version);
                assert!(
                    bindings.original_completed_command_world().is_none(),
                    "{}: {source}",
                    version.dialect_profile_name()
                );
            }
        }
    }

    #[test]
    fn a_deleted_builtin_is_absent_for_diagnostics_only() {
        // tclsh 8.4-9.1: `rename puts {}; puts x` raises
        // `invalid command name "puts"`.
        for version in tcl_dialect::TclVersion::ALL {
            let source = "rename puts {}\nputs x\n";
            let bindings = analyse(source, version);
            let at = u32::try_from(source.find("\nputs").unwrap() + 1).unwrap();
            assert_eq!(
                bindings.diagnostic_slot_presence_at(at),
                super::super::SourceCommandSlotPresence::absent(),
                "{}",
                version.dialect_profile_name()
            );
            assert!(bindings.original_completed_command_world().is_none());
        }
    }
}
