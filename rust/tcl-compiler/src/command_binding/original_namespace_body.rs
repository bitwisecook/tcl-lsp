// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact namespace script delegation and enclosing handler completion.

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandTarget, SourceExecutionContext,
    SourceNamespaceKey, SourceNativeInvocation,
};
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};

/// Exact original script handed to the common namespace body walker. This
/// receipt owns correspondence and Ensure geometry, without completion.
struct OriginalNamespaceBodyDelegation {
    site: CommandAllocationSite,
    words: Vec<crate::ir::WordExpr>,
    config: tcl_lexer::LexerConfig,
    handler: SourceCommandTarget,
    body: super::ExecutedScriptSource,
    argument: usize,
    namespace_scope: SignatureNamespaceScope,
}

/// Selected intrinsic handler and actually entered namespace/frame. Completion
/// is checked only after the common child walker and parent restoration.
pub(super) struct OriginalNamespaceBody {
    delegation: OriginalNamespaceBodyDelegation,
    namespace: SourceNamespaceKey,
    entered: crate::var_resolve::VariableExecutionFrame,
    parent: crate::var_resolve::VariableExecutionFrame,
}

fn quiet(state: &ModuleCommandBindings) -> bool {
    !state.has_opaque_domain()
        && state.baseline.native_entry.is_none()
        && !state.baseline.unknown_entry
        && !state.source_step_observed()
        && !state.source_execution_observed(None)
        && state.command_observers.is_quiet()
}

impl OriginalNamespaceBodyDelegation {
    fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        variables: &crate::var_resolve::ResolveContext,
    ) -> Option<Self> {
        use tcl_registry::{
            NamespaceTransition, NamespaceTransitionTarget, StateTransition,
            hooks::{AnalyserHookId, LoweringHookId},
            script_body_flow::ScriptBodyFlow,
            world_effect::{CallbackKinds, EffectAccessMode, Reentrancy, WorldStateDomain},
        };
        if !quiet(state)
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || facts.operation
                != tcl_registry::SemanticOperationId::StructuredLowering(
                    LoweringHookId::NamespaceEval,
                )
            || facts.analyser_hook != Some(AnalyserHookId::NamespaceEval)
            || !native.target.registry_backed
            || native.target.kind != super::BindingKind::Builtin
            || native.target.implementation_generation != 0
            || native.target.runtime_implementation_generation.is_some()
            || !native.target.prepended.is_empty()
            || !state.retained_target_is_current(native.target)
            || facts.effects.callback().kinds != CallbackKinds::SCRIPT
            || facts.effects.callback().reentrancy != Reentrancy::CurrentInterpreter
            || facts.effects.accesses().iter().any(|access| {
                matches!(
                    access.domain,
                    WorldStateDomain::CommandBindings | WorldStateDomain::NamespaceLookup
                ) && access.mode != EffectAccessMode::Read
            })
        {
            return None;
        }
        // The own namespace Ensure is accounted by the selected transition.
        // Any unexplained name write or additional callback remains terminal;
        // no descriptor-presence exemption subtracts an effect footprint.
        let ScriptBodyFlow::ConcatenatedScript { argument_offset } =
            tcl_registry::case_bodies::script_body_flow_in_registry(
                context.registry,
                facts,
                native.invocation.arguments(),
            )
        else {
            return None;
        };
        if native.invocation.arguments().exact_argv_len()? != argument_offset.checked_add(1)? {
            return None;
        }
        let body_input = native
            .original_variable_operands
            .input(argument_offset, variables)?;
        let SignatureSourceNameInput::OriginalWord(_) = body_input else {
            return None;
        };
        let body = super::retained_script_operand(
            argument_offset,
            native.script_operands(),
            state,
            native.segment.span.start(),
            context.config,
        )?;
        let transitions = facts.state_transitions.declared()?;
        let [fact] = transitions.facts() else {
            return None;
        };
        let StateTransition::Namespace(NamespaceTransition::Ensure {
            namespace: NamespaceTransitionTarget::Named(subject),
        }) = &fact.transition
        else {
            return None;
        };
        let input =
            super::original_command_table::original_operand(native, subject, state, context)?;
        let policy = input.policy();
        input.original_word_key()?;
        let tcl_syntax::naming::NativeNameProtocol::C(version) = policy.recipe() else {
            return None;
        };
        if body_input.policy() != policy {
            return None;
        }
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let spec = context.registry.get_for_surface(
            native.target.registry_identity()?,
            Some(dialect.authoring_query()?.with_realm(context.realm)),
        )?;
        if tcl_registry::registry::spec_pack_of(spec) != Some("tcl") {
            return None;
        }
        let scope = state.original_namespace_geometry(&context.namespace_identity(), policy)?;
        let path = policy
            .recipe()
            .namespace_address_path(scope.context()?, input.bytes())
            .ok()?;
        Some(Self {
            site: CommandAllocationSite {
                source: Arc::clone(state.current_source_origin.as_ref()?),
                offset: native.segment.span.start(),
            },
            words: native.words.to_vec(),
            config: context.config,
            handler: native.target.clone(),
            body,
            argument: argument_offset,
            namespace_scope: SignatureNamespaceScope::C(path),
        })
    }
}

/// The pre-transfer original body/Ensure owner. It supplies no Normal result
/// and does not suppress any remaining mutable-name footprint.
pub(super) fn has_original_namespace_body_delegation(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    OriginalNamespaceBodyDelegation::capture(native, facts, state, context, &state.source_variables)
        .is_some()
}

impl OriginalNamespaceBody {
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        prepared: &super::PreparedSourceNativeBody,
    ) -> Option<Self> {
        let delegation = OriginalNamespaceBodyDelegation::capture(
            native,
            facts,
            state,
            context,
            &prepared.parent_variables,
        );
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_NAMESPACE_BODY_CAPTURE site={} delegation={} quiet={} argc={:?} arity={:?} callbacks={:?} namespace_activation={} entered_frame={} namespace_identity={} namespace_known={}",
                native.segment.span.start(),
                delegation.is_some(),
                quiet(state),
                native.invocation.arguments().exact_argv_len(),
                facts.arity_accepts_frozen_arguments(),
                facts.effects.callback(),
                prepared.selection.namespace_activation,
                state.variable_frame == prepared.selection.frame,
                state.source_variables.namespace_identity.as_ref()
                    == Some(&prepared.selection.namespace_key),
                state.namespaces.contains(&prepared.selection.namespace_key),
            );
        }
        let delegation = delegation?;
        let policy = state.baseline.execution_name_policy?.native_recipe()?;
        if !prepared.selection.namespace_activation
            || state.variable_frame != prepared.selection.frame
            || state.source_variables.namespace_identity.as_ref()
                != Some(&prepared.selection.namespace_key)
            || state.original_namespace_geometry(&prepared.selection.namespace_key, policy)?
                != delegation.namespace_scope
            || !state.namespaces.contains(&prepared.selection.namespace_key)
            || state
                .unknown_lookup_namespaces
                .contains(&prepared.selection.namespace_key)
        {
            return None;
        }
        Some(Self {
            delegation,
            namespace: prepared.selection.namespace_key.clone(),
            entered: prepared.selection.frame.clone(),
            parent: prepared.parent_frame.clone(),
        })
    }

    pub(super) fn completed(
        &self,
        native: SourceNativeInvocation<'_>,
        outcomes: &super::SourceOutcomes,
        context: SourceExecutionContext<'_>,
        prepared: &super::PreparedSourceNativeBody,
    ) -> bool {
        let Some(normal) = outcomes
            .normal
            .as_deref()
            .filter(|_| outcomes.normal_completion.is_some() && outcomes.abrupt.is_empty())
        else {
            #[cfg(debug_assertions)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
                eprintln!(
                    "ORIGINAL_NAMESPACE_BODY_FINISH site={} normal={} child_complete={} abrupt={}",
                    native.segment.span.start(),
                    outcomes.normal.is_some(),
                    outcomes.normal_completion.is_some(),
                    outcomes.abrupt.len()
                );
            }
            return false;
        };
        let delegation = &self.delegation;
        let completed = quiet(normal)
            && delegation.site.offset == native.segment.span.start()
            && normal.current_source_origin.as_ref() == Some(&delegation.site.source)
            && delegation.words == native.words
            && delegation.config == context.config
            && delegation.handler == *native.target
            && normal.retained_target_is_current(native.target)
            && self.entered == prepared.selection.frame
            && self.namespace == prepared.selection.namespace_key
            && self.parent == prepared.parent_frame
            && normal.variable_frame == self.parent
            && normal.source_variables.namespace_identity
                == prepared.parent_variables.namespace_identity
            && normal.namespaces.contains(&self.namespace)
            && !normal.unknown_lookup_namespaces.contains(&self.namespace)
            && super::retained_script_operand(
                delegation.argument,
                native.script_operands(),
                normal,
                native.segment.span.start(),
                context.config,
            )
            .as_ref()
                == Some(&delegation.body);
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_NAMESPACE_BODY_FINISH site={} complete={} quiet={} parent_frame={} parent_namespace={}",
                native.segment.span.start(),
                completed,
                quiet(normal),
                normal.variable_frame == self.parent,
                normal.source_variables.namespace_identity
                    == prepared.parent_variables.namespace_identity
            );
        }
        completed
    }
}

#[cfg(test)]
mod tests {
    fn analyse(
        source: &str,
        version: tcl_dialect::TclVersion,
    ) -> super::super::SourceCommandBindings {
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
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
    fn original_namespace_body_keeps_completed_child_and_restores_parent() {
        // Implementation contract: naming.namespace.original-entered-body-completion
        // docs/design/analysis/name-resolution-proofs/namespace-original-entered-body-completion.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = "namespace eval A {proc p {} {}}; set checkpoint READY";
            let bindings = analyse(source, version);
            let world = bindings
                .original_completed_command_world()
                .unwrap_or_else(|| panic!("{}", version.dialect_name()));
            let entries = world.declarations().collect::<Vec<_>>();
            let [entry] = entries.as_slice() else {
                panic!("one nested declaration")
            };
            assert_eq!(entry.slot().simple.as_bytes(), b"p");
            let point = bindings.invocation_at_source(
                "set",
                u32::try_from(source.find("set checkpoint").unwrap()).unwrap(),
            );
            let state = &point.lookup_state.as_ref().unwrap().state;
            assert!(matches!(
                state.variable_frame.layout(),
                crate::var_resolve::VariableExecutionFrame::Global
            ));
            assert_eq!(
                state.source_variables.namespace_identity,
                Some(super::SourceNamespaceKey::authored("::"))
            );
        }
    }

    #[test]
    fn original_namespace_activation_retains_selected_source_identity_before_child_words() {
        // Implementation contract: naming.namespace.original-entered-body-completion
        // docs/design/analysis/name-resolution-proofs/namespace-original-entered-body-completion.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = "namespace eval A {proc p {} {}}; set checkpoint READY";
            let bindings = analyse(source, version);
            let child = bindings
                .invocation_at_source("proc", u32::try_from(source.find("proc").unwrap()).unwrap());
            assert_eq!(
                child.variable_context.namespace_identity,
                Some(super::super::SourceNamespaceKey::authored("::A")),
                "{version:?}"
            );
            assert!(matches!(
                child.variable_frame.layout(),
                crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
            ));
            let parent = bindings.invocation_at_source(
                "set",
                u32::try_from(source.find("set checkpoint").unwrap()).unwrap(),
            );
            assert_ne!(
                parent.variable_context.namespace_identity,
                child.variable_context.namespace_identity
            );
        }
    }

    #[test]
    fn original_namespace_body_refuses_unrepresented_and_abrupt_children() {
        // Implementation contract: naming.namespace.original-entered-body-completion
        // docs/design/analysis/name-resolution-proofs/namespace-original-entered-body-completion.md
        for version in tcl_dialect::TclVersion::ALL {
            for source in [
                "namespace eval A {unknownChild}",
                "namespace eval A {error BOOM}",
                "namespace eval A {return EARLY}",
                "namespace eval A $unknownBody",
            ] {
                assert!(
                    analyse(source, version)
                        .original_completed_command_world()
                        .is_none(),
                    "{version:?}: {source}"
                );
            }
            let source = "proc namespace args {}; namespace eval A {}; set checkpoint READY";
            let bindings = analyse(source, version);
            assert!(
                bindings.original_completed_command_world().is_some(),
                "{version:?}: complete replacement body"
            );
            let point = bindings.invocation_at_source(
                "set",
                u32::try_from(source.find("set checkpoint").unwrap()).unwrap(),
            );
            assert!(
                !point
                    .lookup_state
                    .as_ref()
                    .unwrap()
                    .state
                    .namespaces
                    .contains(&super::SourceNamespaceKey::authored("::A")),
                "replacement procedure cannot donate namespace creation"
            );
        }
    }
}
