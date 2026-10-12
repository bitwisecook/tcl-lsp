// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original named instance cells at the actual constructor boundary.

use super::{
    Arc, BTreeSet, BindingKind, MayBinding, ModuleCommandBindings, SourceCommandKey,
    SourceExecutionContext, SourceObjectInstanceProof,
};
use crate::signature_scan::scope::SignatureSourceNameInput;

/// Genuine original name and a fresh current command cell. This pre-operation
/// selection grants no constructor completion or native object allocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalNamedManufacture {
    input: SignatureSourceNameInput,
    key: SourceCommandKey,
}

/// Historical full-name result issued by a complete source constructor at its
/// actual surviving named allocation. It grants no written word, current table
/// liveness or physical native object authority.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OriginalObjectCommandNameResult {
    allocation: super::SourceObjectAllocation,
    requested: SignatureSourceNameInput,
    slot: tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
}

impl OriginalObjectCommandNameResult {
    pub(crate) fn requested_name_input(&self) -> &SignatureSourceNameInput {
        &self.requested
    }
    pub(crate) fn slot(&self) -> &tcl_core_types::ByteCommandSlot {
        &self.slot
    }
    pub(crate) fn policy(&self) -> tcl_syntax::naming::NamePolicyProtocol {
        self.policy
    }
}

impl OriginalNamedManufacture {
    pub(super) fn capture(
        input: &SignatureSourceNameInput,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        if state.has_opaque_domain()
            || !input.is_current(&state.source_variables)
            || matches!(input, SignatureSourceNameInput::OriginalVariableRoot(_))
        {
            return None;
        }
        let key = state.original_publication_key(
            &context.namespace_identity(),
            input,
            super::namespace_slots::PublicationPurpose::Command,
        )?;
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_NAMED_CELL key={key:?} current={} namespace={} unknown_holder={} cell={:?}",
                input.is_current(&state.source_variables),
                state.namespaces.contains(key.holder().as_ref()),
                state
                    .unknown_lookup_namespaces
                    .contains(key.holder().as_ref()),
                state.original_bindings_for_key(&key)
            );
        }
        let SourceCommandKey::Slot { namespace, simple } = &key else {
            return None;
        };
        if simple.as_bytes().is_empty()
            || !state.namespaces.contains(namespace)
            || state.unknown_lookup_namespaces.contains(namespace)
            || state.original_bindings_for_key(&key)? != BTreeSet::from([MayBinding::Missing])
        {
            return None;
        }
        Some(Self {
            input: input.clone(),
            key,
        })
    }

    pub(super) fn result(
        &self,
        proof: &SourceObjectInstanceProof,
        outcomes: &super::SourceOutcomes,
    ) -> Option<OriginalObjectCommandNameResult> {
        if outcomes.normal_completion.is_none() || !outcomes.abrupt.is_empty() {
            return None;
        }
        let state = outcomes.normal.as_deref()?;
        if !state.receiver_allocation_is_current(proof) {
            return None;
        }
        let policy = self.input.policy();
        if state.baseline.execution_name_policy?.native_recipe()? != policy {
            return None;
        }
        let key = state.constructed_command_key(proof)?;
        Some(OriginalObjectCommandNameResult {
            allocation: proof.allocation().clone(),
            requested: self.input.clone(),
            slot: state.original_byte_slot_for_key(key, policy)?,
            policy,
        })
    }

    pub(super) fn install(&self, site: u32, state: &mut ModuleCommandBindings) -> Option<()> {
        if !self.input.is_current(&state.source_variables)
            || state.original_bindings_for_key(&self.key)? != BTreeSet::from([MayBinding::Missing])
        {
            return None;
        }
        let policy = self.input.policy();
        let slot = state.original_byte_slot_for_key(&self.key, policy)?;
        let preserve_dispatcher = slot.namespace.is_root();
        let dispatcher = state.object_instances.receiver_dispatcher_generation;
        let receivers = preserve_dispatcher.then(|| state.object_instances.receivers.clone());
        state.install(
            self.key.clone(),
            MayBinding::Target(super::ResolvedCommandTarget {
                command: ModuleCommandBindings::command_key_label(&self.key).unwrap_or_default(),
                prepended: Vec::new(),
                registry_backed: false,
                kind: BindingKind::Command,
                implementation_generation: site,
                terminal: true,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: None,
                implementation_allocation: None,
            }),
        );
        if let Some(receivers) = receivers {
            let objects = Arc::make_mut(&mut state.object_instances);
            objects.receiver_dispatcher_generation = dispatcher;
            objects.receivers = receivers;
        }
        Some(())
    }

    pub(super) fn retain(
        &self,
        proof: &mut Arc<SourceObjectInstanceProof>,
        state: &mut ModuleCommandBindings,
    ) -> Option<()> {
        let bindings = state.original_bindings_for_key(&self.key)?;
        let alternatives = bindings.iter().collect::<Vec<_>>();
        let [MayBinding::Target(target)] = alternatives.as_slice() else {
            return None;
        };
        if target.kind != BindingKind::Command
            || target.registry_backed
            || !target.terminal
            || !target.prepended.is_empty()
        {
            return None;
        }
        let allocation = target.implementation_allocation.as_ref()?;
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_NAMED_ALLOCATION site={} command_incarnation={:?} object_incarnation={:?} same_site={} same_namespace={} same_token={}",
                allocation.site.offset,
                allocation.incarnation,
                proof.allocation().incarnation,
                allocation.site == proof.allocation().site,
                allocation.namespace == self.key.holder().as_ref().clone(),
                target
                    .token
                    .as_ref()
                    .is_some_and(|token| token.allocation.as_ref() == Some(allocation))
            );
        }
        if allocation.site != proof.allocation().site
            || allocation.incarnation != proof.allocation().incarnation
            || allocation.namespace != self.key.holder().as_ref().clone()
            || target.token.as_ref()?.allocation.as_ref() != Some(allocation)
        {
            return None;
        }
        let identity = target.token.clone()?;
        Arc::make_mut(proof).receiver_dispatcher_generation =
            state.object_instances.receiver_dispatcher_generation;
        Arc::make_mut(&mut state.object_instances)
            .named
            .insert(identity, Arc::clone(proof));
        Some(())
    }
}

impl ModuleCommandBindings {
    /// The current exact cell of this actual named allocation. Reporting text
    /// cannot establish its presence or decide whether construction deleted it.
    pub(super) fn constructed_command_key(
        &self,
        proof: &SourceObjectInstanceProof,
    ) -> Option<&SourceCommandKey> {
        let mut slots = self.bindings.iter().filter_map(|(key, bindings)| {
            let alternatives = bindings.iter().collect::<Vec<_>>();
            let [MayBinding::Target(target)] = alternatives.as_slice() else {
                return None;
            };
            if !target.terminal || !target.prepended.is_empty() {
                return None;
            }
            let receiver = self.object_instances.named.get(target.token.as_ref()?)?;
            (receiver.allocation() == proof.allocation()).then_some(key)
        });
        let key = slots.next()?;
        slots.next().is_none().then_some(key)
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::TclVersion;

    fn analyse(source: &str, version: TclVersion) -> super::super::SourceCommandBindings {
        analyse_with_compilation(
            source,
            version,
            crate::environment_ingress::authoring_native_compilation(),
        )
    }

    fn analyse_with_compilation(
        source: &str,
        version: TclVersion,
        compilation: tcl_registry::native_compilation::NativeCompilationContext,
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
                native_compilation: compilation,
                ..Default::default()
            },
        )
    }

    #[test]
    fn original_named_manufacture_keeps_the_canonical_cell_and_complete_receiver_call() {
        // Implementation contract: naming.tcloo.original-named-manufacture-cell-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-named-manufacture-cell-transfer.md
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            for (name, method, expected) in [
                ("object", "pick", b"object".as_slice()),
                (
                    r"object\uD800",
                    r"m\uD800",
                    b"object\xed\xa0\x80".as_slice(),
                ),
            ] {
                let class = format!("oo::class create C {{method {method} {{}} {{return body}}}}");
                let manufacture = format!("{class}\nC create {name}");
                let complete = format!("{manufacture}\n{name} {method}\n");
                for (phase, source) in [
                    ("class", class.as_str()),
                    ("manufacture", manufacture.as_str()),
                    ("call", complete.as_str()),
                ] {
                    assert!(
                        analyse(source, version)
                            .original_completed_command_world()
                            .is_some(),
                        "{version:?} phase={phase}: {source}"
                    );
                }
                let bindings = analyse(&complete, version);
                let offset =
                    u32::try_from(complete.rfind(&format!("{name} {method}")).unwrap()).unwrap();
                let point = bindings.invocation_at_source(name, offset);
                let receiver = point
                    .named_object_instance_at_dispatch()
                    .expect("actual installed source instance");
                let state = &point.lookup_state.as_ref().unwrap().state;
                let key = state
                    .constructed_command_key(receiver)
                    .expect("exact surviving allocation cell");
                let policy = point
                    .original_retained_written_name_input(0)
                    .unwrap()
                    .policy();
                let slot = state.original_byte_slot_for_key(key, policy).unwrap();
                assert_eq!(slot.simple.as_bytes(), expected, "{version:?}");
                assert_eq!(
                    receiver.allocation().site.offset,
                    u32::try_from(complete.find("C create").unwrap()).unwrap()
                );
                let selector = point.original_retained_written_name_input(1).unwrap();
                assert!(
                    state
                        .retained_instance_method_entry(
                            receiver,
                            selector.bytes(),
                            selector.policy()
                        )
                        .is_some()
                );
            }
        }
    }

    #[test]
    fn original_named_manufacture_result_retains_opaque_units_without_a_written_word() {
        // Implementation contract: naming.tcloo.original-named-manufacture-cell-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-named-manufacture-cell-transfer.md
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let source = r"oo::class create C {method pick {} {return body}}; set held [C create object\uD800]; $held pick";
            let bindings = analyse(source, version);
            let offset = u32::try_from(source.find("$held pick").unwrap()).unwrap();
            let point = bindings.invocation_at_source("$held", offset);
            let name = point
                .original_retained_written_name_input(0)
                .expect("held actual constructor full-name result");
            assert!(name.original_word_key().is_none());
            assert_eq!(name.bytes(), b"::object\xed\xa0\x80", "{version:?}");
            assert!(
                point.named_object_instance_at_dispatch().is_some(),
                "{version:?}"
            );
            assert!(
                bindings.original_completed_command_world().is_some(),
                "{version:?}"
            );
        }
    }

    #[test]
    fn original_named_manufacture_result_does_not_close_an_unknown_setter_mode() {
        // Implementation contract: naming.tcloo.original-named-manufacture-cell-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-named-manufacture-cell-transfer.md
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let source = r"oo::class create C {method pick {} {return body}}; set held [C create object\uD800]; $held pick";
            let mut compilation = crate::environment_ingress::authoring_native_compilation();
            compilation.mode = tcl_registry::native_compilation::NativeCompilationMode::Unknown;
            let bindings = analyse_with_compilation(source, version, compilation);
            let offset = u32::try_from(source.find("$held pick").unwrap()).unwrap();
            let point = bindings.invocation_at_source("$held", offset);
            assert!(
                point.original_retained_written_name_input(0).is_none(),
                "{version:?}"
            );
            assert!(
                point.named_object_instance_at_dispatch().is_none(),
                "{version:?}"
            );
            assert!(
                bindings.original_completed_command_world().is_none(),
                "{version:?}"
            );
        }
    }

    #[test]
    fn original_named_manufacture_result_survives_a_genuine_called_scalar_frame() {
        // Implementation contract: naming.tcloo.original-named-manufacture-cell-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-named-manufacture-cell-transfer.md
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let source = r"oo::class create C {method pick {} {return body}}; proc make {} {set held [C create object\uD800]; $held pick}; make";
            let bindings = analyse(source, version);
            let offset = u32::try_from(source.find("$held pick").unwrap()).unwrap();
            let point = bindings.invocation_at_source("$held", offset);
            let name = point
                .original_retained_written_name_input(0)
                .expect("constructor result retained in the actual called scalar frame");
            assert!(name.original_word_key().is_none());
            assert_eq!(name.bytes(), b"::object\xed\xa0\x80", "{version:?}");
            assert!(
                point.named_object_instance_at_dispatch().is_some(),
                "{version:?}"
            );
            assert!(
                bindings.original_completed_command_world().is_some(),
                "{version:?}"
            );
        }
    }

    #[test]
    fn original_named_manufacture_declines_occupied_unknown_and_failed_construction() {
        // Implementation contract: naming.tcloo.original-named-manufacture-cell-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-named-manufacture-cell-transfer.md
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            for source in [
                "proc object {} {}; oo::class create C {}; C create object; object pick",
                "oo::class create C {}; C create $unknown; object pick",
                "oo::class create C {constructor {} {error BOOM}}; C create object; object pick",
                "oo::class create C {filter unresolved; method pick {} {return body}}; C create object; object pick",
            ] {
                let bindings = analyse(source, version);
                assert!(
                    bindings.original_completed_command_world().is_none(),
                    "{version:?}: {source}"
                );
                let offset = u32::try_from(source.rfind("object pick").unwrap()).unwrap();
                assert!(
                    bindings
                        .invocation_at_source("object", offset)
                        .named_object_instance_at_dispatch()
                        .is_none(),
                    "{version:?}: {source}"
                );
            }
        }
    }
}
