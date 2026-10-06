// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Navigation projection of the called slot, independently of target execution.

use super::{
    BindingKind, CommandAllocation, CommandIdentity, MayBinding, ModuleCommandBindings,
    ResolvedCommandTarget, SourceInvocationBinding,
};

/// Category proved by the selected source definition owner, independently of
/// a document's assistance records and the shared class/instance binding kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceCommandDefinitionKind {
    /// Native procedure declaration with a retained implementation allocation.
    Procedure,
    /// Genuine class definition, excluding ordinary object instances.
    Class,
}

/// Navigation receipt for one current source implementation. Only positioned
/// command lookup mints this carrier; coordinates or a `QName` cannot mint it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceCommandDefinition {
    kind: SourceCommandDefinitionKind,
    allocation: CommandAllocation,
}

impl SourceCommandDefinition {
    pub(super) fn original_declared_procedure(target: &super::SourceCommandTarget) -> Option<Self> {
        if target.kind != BindingKind::Proc || target.registry_backed {
            return None;
        }
        Some(Self {
            kind: SourceCommandDefinitionKind::Procedure,
            allocation: target.implementation_allocation.clone()?,
        })
    }

    /// Retained declaration category; no execution or signature licence.
    #[must_use]
    pub fn kind(&self) -> SourceCommandDefinitionKind {
        self.kind
    }

    /// Current implementation's full source allocation, distinct from its token.
    #[must_use]
    pub fn allocation(&self) -> &CommandAllocation {
        &self.allocation
    }
}

/// The binding through which a written command name denotes a navigation target.
/// An import retains the origin token; it does not allocate an interpreter alias.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceCommandReferenceBinding {
    /// A command occupying this slot, including moved commands and aliases.
    Direct {
        /// Kind of the command occupying the called slot.
        kind: BindingKind,
        /// Retained identity of the called command, independently of its target.
        identity: CommandIdentity,
    },
    /// A namespace import referring to its retained origin token.
    Imported {
        /// Origin token retained by the namespace import.
        origin: CommandIdentity,
        /// Unanimous category of the retained origin implementations.
        kind: BindingKind,
    },
}

/// A unanimously selected command slot at a positioned interpreter snapshot.
/// This grants navigation only: it supplies no implementation, argument-layout,
/// normal-completion, native-compilation, or object-class proof.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceCommandReference {
    slot: String,
    binding: SourceCommandReferenceBinding,
    definition: Option<SourceCommandDefinition>,
    linked_definition: Option<SourceCommandDefinition>,
}

impl SourceCommandReference {
    /// Called slot, before interpreter-alias target traversal.
    #[must_use]
    pub fn slot(&self) -> &str {
        &self.slot
    }

    /// Direct versus imported binding retained by this lookup.
    #[must_use]
    pub fn binding(&self) -> &SourceCommandReferenceBinding {
        &self.binding
    }

    /// Allocation of the currently selected source definition. This is the
    /// implementation allocation, not the surviving command token's original
    /// declaration. Imported definitions retain their actual origin allocation.
    /// No runtime-entry, alias-wrapper or instance allocation grants this fact.
    #[must_use]
    pub fn definition(&self) -> Option<&SourceCommandDefinition> {
        self.definition.as_ref()
    }

    /// Terminal declaration reached through this actual alias/import lookup.
    /// This separate projection rejects observed, opaque and missing edges;
    /// the called slot and its direct-definition category remain unchanged.
    /// An absent linked receipt cannot be replaced by final name/link maps.
    #[must_use]
    pub fn linked_definition(&self) -> Option<&SourceCommandDefinition> {
        self.linked_definition.as_ref()
    }

    /// Whether this slot navigates to a source-owned command rather than a builtin.
    #[must_use]
    pub fn is_user_command(&self) -> bool {
        let kind = match self.binding {
            SourceCommandReferenceBinding::Direct { kind, .. }
            | SourceCommandReferenceBinding::Imported { kind, .. } => kind,
        };
        matches!(
            kind,
            BindingKind::Proc | BindingKind::Class | BindingKind::Alias | BindingKind::Command
        )
    }

    /// A direct procedure or class definition, excluding alias/import wrappers.
    #[must_use]
    pub fn is_direct_definition(&self) -> bool {
        self.definition.is_some()
            && matches!(self.binding, SourceCommandReferenceBinding::Direct { .. })
    }
}

impl SourceInvocationBinding {
    /// Source-owned procedure candidates retained at this original lookup.
    /// Unknown and missing alternatives remain possible. These candidates
    /// identify declarations for result advice, never a call or normal outcome.
    #[must_use]
    pub(crate) fn declared_procedure_candidates(&self) -> Vec<&super::SourceCommandTarget> {
        let Some(snapshot) = self.lookup_state.as_ref() else {
            return Vec::new();
        };
        if snapshot.state.opaque_domain || snapshot.state.source_step_observed() {
            return Vec::new();
        }
        self.targets
            .iter()
            .filter(|target| {
                target.kind == BindingKind::Proc
                    && !target.registry_backed
                    && target.implementation_allocation.is_some()
                    && !snapshot
                        .state
                        .source_execution_observed(target.identity.as_ref())
            })
            .collect()
    }

    /// Original declared procedure candidates for conditional result advice.
    /// The declaration/compiler table is independent of a reached invocation;
    /// missing allocations and replacement targets never acquire callee advice.
    pub(crate) fn declared_procedure_result_candidates(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Vec<super::SourceCommandTarget> {
        let current = self.declared_procedure_candidates();
        if !current.is_empty() {
            return current.into_iter().cloned().collect();
        }
        let Some(advice) = self
            .declaration_operand_layout_advice(tokens)
            .or_else(|| self.original_compilation_lookup_advice(tokens))
        else {
            return Vec::new();
        };
        advice
            .targets()
            .iter()
            .filter(|target| {
                target.kind == BindingKind::Proc
                    && !target.registry_backed
                    && target.implementation_allocation.is_some()
            })
            .cloned()
            .collect()
    }

    /// Navigation receipt for the command word actually frozen at this
    /// invocation, in its retained post-argument lookup world. This proves no
    /// writable source spelling or contributor provenance for a computed head.
    #[must_use]
    pub fn evaluated_command_reference(&self) -> Option<SourceCommandReference> {
        self.command_reference(self.evaluated_command_word()?)
    }

    /// Resolve a navigation name in this exact post-argument lookup world.
    /// This does not claim that the candidate value was actually evaluated or
    /// that calling the selected alias/import will complete successfully.
    #[must_use]
    pub fn command_reference(&self, word: &str) -> Option<SourceCommandReference> {
        let snapshot = self.lookup_state.as_ref()?;
        let state = &snapshot.state;
        if !state.source_lookup_is_closed(word, &self.lookup_namespace_key) {
            return None;
        }
        let keys = state.source_keys(word, &self.lookup_namespace_key);
        let [slot] = keys.as_slice() else {
            return None;
        };
        let bindings = state.bindings.get(slot).cloned().unwrap_or_else(|| {
            ModuleCommandBindings::unmodified_bindings(
                slot,
                state.baseline.semantics.binding_names(),
            )
        });
        if bindings.len() != 1 {
            return None;
        }
        let (binding, implementation) = match bindings.first()? {
            MayBinding::Target(target) => (
                SourceCommandReferenceBinding::Direct {
                    kind: target.kind,
                    identity: target.token.clone()?,
                },
                target,
            ),
            MayBinding::Imported(origin) => {
                let implementations = state.objects.get(&origin.origin)?;
                let mut kinds = implementations
                    .iter()
                    .map(|implementation| match implementation {
                        MayBinding::Target(target) => Some(target.kind),
                        MayBinding::Imported(_) | MayBinding::Missing | MayBinding::Unknown => None,
                    });
                let kind = kinds.next()??;
                if !kinds.all(|candidate| candidate == Some(kind)) {
                    return None;
                }
                if implementations.len() != 1 {
                    return None;
                }
                let MayBinding::Target(implementation) = implementations.first()? else {
                    return None;
                };
                (
                    SourceCommandReferenceBinding::Imported {
                        origin: origin.origin.clone(),
                        kind,
                    },
                    implementation,
                )
            }
            MayBinding::Missing | MayBinding::Unknown => return None,
        };
        let definition = state.source_definition_for_implementation(implementation);
        let linked_definition = self.linked_definition(word);
        Some(SourceCommandReference {
            slot: state.callable_spelling_for_key(slot)?,
            binding,
            definition,
            linked_definition,
        })
    }
}

impl ModuleCommandBindings {
    /// Project a retained current implementation into declaration navigation.
    /// Binding categories and surviving tokens alone grant no class definition.
    pub(super) fn source_definition_for_implementation(
        &self,
        implementation: &ResolvedCommandTarget,
    ) -> Option<SourceCommandDefinition> {
        match implementation.kind {
            BindingKind::Proc => Some(SourceCommandDefinitionKind::Procedure),
            BindingKind::Class | BindingKind::Command
                if implementation.token.as_ref().is_some_and(|identity| {
                    self.class_definitions.get(identity).is_some_and(|receipt| {
                        receipt.implementation_generation
                            == implementation.implementation_generation
                    })
                }) =>
            {
                Some(SourceCommandDefinitionKind::Class)
            }
            BindingKind::Bottom
            | BindingKind::Builtin
            | BindingKind::Class
            | BindingKind::Command
            | BindingKind::Alias
            | BindingKind::Opaque
            | BindingKind::Unknown => None,
        }
        .and_then(|kind| {
            Some(SourceCommandDefinition {
                kind,
                allocation: implementation.implementation_allocation.clone()?,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(source: &str) -> SourceInvocationBinding {
        let bindings = super::super::SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
        );
        bindings.invocation_at_source(
            "set",
            u32::try_from(source.rfind("set checkpoint").unwrap()).unwrap(),
        )
    }

    #[test]
    fn navigation_retains_called_alias_even_when_terminal_is_absent() {
        let point = at("interp alias {} short {} missing; set checkpoint READY");
        let reference = point.command_reference("short").unwrap();
        assert_eq!(reference.slot(), "::short");
        assert!(reference.is_user_command());
        assert!(!reference.is_direct_definition());
        assert!(matches!(
            reference.binding(),
            SourceCommandReferenceBinding::Direct {
                kind: BindingKind::Alias,
                ..
            }
        ));
        assert!(point.command_reference("missing").is_none());
        let execution = point.lookup_command_word("short");
        assert_ne!(
            execution
                .proved_target()
                .map(|target| target.command.as_str()),
            Some(reference.slot()),
            "missing-target fallback cannot replace the called alias's navigation identity",
        );
    }

    #[test]
    fn navigation_retains_import_origin_and_rename_lifetime() {
        let point = at(
            "namespace eval n {proc target {} {}; namespace export target}; namespace import ::n::target; rename ::n::target ::n::moved; set checkpoint READY",
        );
        let reference = point.command_reference("target").unwrap();
        assert_eq!(reference.slot(), "::target");
        assert!(
            matches!(reference.binding(), SourceCommandReferenceBinding::Imported { origin, kind: BindingKind::Proc } if origin.origin == "::n::target")
        );
        assert!(point.command_reference("::n::target").is_none());
        assert_eq!(
            point.command_reference("::n::moved").unwrap().slot(),
            "::n::moved"
        );
        let deleted = at(
            "namespace eval n {proc target {} {}; namespace export target}; namespace import ::n::target; rename ::n::target {}; set checkpoint READY",
        );
        assert!(deleted.command_reference("target").is_none());
    }

    #[test]
    fn navigation_keeps_current_definition_separate_from_surviving_token() {
        let source = "proc p {} {return FIRST}; proc p {x} {return SECOND}; set checkpoint READY";
        let reference = at(source).command_reference("p").unwrap();
        let SourceCommandReferenceBinding::Direct { identity, .. } = reference.binding() else {
            panic!("direct procedure slot expected");
        };
        let allocation = reference.definition().unwrap().allocation();
        assert_eq!(
            allocation.site.offset as usize,
            source.find("proc p {x}").unwrap()
        );
        assert_ne!(identity.allocation.as_ref(), Some(allocation));
        assert!(reference.is_direct_definition());
    }

    #[test]
    fn navigation_does_not_treat_instance_allocations_as_class_definitions() {
        let source = "oo::class create C {}; C create instance; set checkpoint READY";
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let bindings = super::super::SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::for_profile(context.commands().profile()),
            context.commands(),
        );
        let point = bindings.invocation_at_source(
            "set",
            u32::try_from(source.rfind("set checkpoint").unwrap()).unwrap(),
        );
        assert!(point.command_reference("C").unwrap().is_direct_definition());
        let instance = point.command_reference("instance").unwrap();
        assert!(instance.is_user_command());
        assert!(instance.definition().is_none());
        assert!(!instance.is_direct_definition());
    }

    #[test]
    fn navigation_treats_evaluated_command_bytes_as_names() {
        let point =
            at("proc {$target} {} {}; proc {has space} {} {}; proc {} {} {}; set checkpoint READY");
        for (word, slot) in [
            ("$target", "::$target"),
            ("has space", "::has space"),
            ("", "::"),
        ] {
            assert_eq!(point.command_reference(word).unwrap().slot(), slot);
        }
    }

    #[test]
    fn navigation_does_not_search_unrelated_tails_or_unknown_paths() {
        let point = at("namespace eval unrelated {proc target {} {}}; set checkpoint READY");
        assert!(point.command_reference("target").is_none());
        let mut unknown = point.clone();
        let snapshot = unknown.lookup_state.as_ref().unwrap();
        let mut state = snapshot.state.clone();
        std::sync::Arc::make_mut(&mut state.unknown_lookup_namespaces).insert("::".to_owned());
        unknown.lookup_state = Some(std::sync::Arc::new(
            super::super::SourceLookupSnapshot::in_realm(state, snapshot.realm),
        ));
        assert!(unknown.command_reference("target").is_none());
        assert_eq!(
            unknown
                .command_reference("::unrelated::target")
                .unwrap()
                .slot(),
            "::unrelated::target"
        );
    }

    #[test]
    fn indirect_alias_reference_keeps_the_new_called_slot_after_class_rename() {
        let source = "oo::class create C {}; rename C savedClass; interp alias {} C {} list ALIAS; set ns ::; ${ns}C";
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let bindings = super::super::SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::for_profile(context.commands().profile()),
            context.commands(),
        );
        let offset = u32::try_from(source.find("${ns}C").unwrap()).unwrap();
        let point = bindings.invocation_at_source("::C", offset);
        let reference = point.command_reference("::C");
        assert!(
            matches!(
                reference.as_ref().map(SourceCommandReference::binding),
                Some(SourceCommandReferenceBinding::Direct {
                    kind: BindingKind::Alias,
                    ..
                })
            ),
            "reach={:?} head={:?} targets={:?} unknown={} snapshot={:?}",
            point.runtime_reachability(),
            point.lookup_word,
            point
                .targets
                .iter()
                .map(|target| (&target.command, target.kind))
                .collect::<Vec<_>>(),
            point.unknown,
            point.lookup_state.as_ref().map(|snapshot| (
                snapshot.state.has_opaque_domain(),
                snapshot.state.source_variables.namespace_known,
                snapshot.state.bindings.get("::C")
            ))
        );
        let reference = reference.unwrap();
        assert!(reference.definition().is_none());
        assert!(reference.linked_definition().is_none());
    }
}
