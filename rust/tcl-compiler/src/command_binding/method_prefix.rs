// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Method names frozen by an actual list builder, independently of callback entry.

use super::{
    Arc, ModuleCommandBindings, SourceCommandTarget, SourceExecutionContext,
    SourceInvocationBinding, SourceObjectInstanceProof, SourceOutcomes, SourceReceiverMethodEntry,
};

/// Original method selector captured with an actual object command prefix.
/// This supplies declaration navigation at registration, never callback execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceCapturedMethodPrefix {
    receiver: Arc<SourceObjectInstanceProof>,
    entry: SourceReceiverMethodEntry,
    selector: crate::ir::WordExpr,
    capture: super::CommandAllocationSite,
    builder: SourceCommandTarget,
    self_scope: Option<CapturedSelfScope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CapturedSelfScope {
    frame: crate::var_resolve::VariableExecutionFrame,
    dispatcher_generation: u64,
    wrapper: Option<(SourceCommandTarget, super::CommandAllocationSite)>,
}

impl SourceCapturedMethodPrefix {
    /// Original receiving allocation, separate from the declaring class.
    #[must_use]
    pub fn receiver(&self) -> &SourceObjectInstanceProof {
        &self.receiver
    }
    /// Original method entry selected at capture.
    #[must_use]
    pub fn method_entry(&self) -> &SourceReceiverMethodEntry {
        &self.entry
    }
    /// Original unchanged method-name word inside the builder.
    #[must_use]
    pub fn selector(&self) -> &crate::ir::WordExpr {
        &self.selector
    }
    /// Exact source instance and instruction of the native builder.
    #[must_use]
    pub fn capture_site(&self) -> &super::CommandAllocationSite {
        &self.capture
    }

    pub(super) fn is_current(&self, state: &ModuleCommandBindings) -> bool {
        state.receiver_allocation_is_current(&self.receiver)
            && self.self_scope.as_ref().is_none_or(|scope| {
                state.object_instances.receiver_dispatcher_generation
                    == Some(scope.dispatcher_generation)
                    && !state.source_execution_observed(None)
            })
            && self
                .receiver
                .class_target()
                .identity
                .as_ref()
                .and_then(|identity| state.class_definitions.get(identity))
                .and_then(|definition| {
                    definition.receiver_method_entries.get(&(
                        super::SourceMethodReceiver::Instance,
                        self.entry.name().to_owned(),
                    ))
                })
                == Some(&self.entry)
    }
}

impl SourceInvocationBinding {
    /// Captured selectors present in original written arguments at registration.
    /// Consumers must separately select the actual deferred prefix operand.
    /// Future method changes and callback entry are not proved by this inventory.
    pub fn captured_method_prefix_arguments(
        &self,
    ) -> impl Iterator<Item = (usize, &SourceCapturedMethodPrefix)> {
        self.method_prefix_arguments
            .iter()
            .filter_map(|(argument, prefix)| {
                let state = &self.lookup_state.as_ref()?.state;
                (!self.entered_execution_observer.observed()
                    && !state.source_step_observed()
                    && prefix
                        .self_scope
                        .as_ref()
                        .is_none_or(|scope| scope.wrapper.is_some())
                    && prefix.is_current(state))
                .then_some((*argument, prefix.as_ref()))
            })
    }
}

pub(super) fn retain_created_prefix(
    outcomes: &mut SourceOutcomes,
    native: super::SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    context: SourceExecutionContext<'_>,
) {
    if facts.native_result
        != Some(tcl_registry::native_result::NativeResultContract::ListArguments { from: 0 })
        || !native.target.prepended.is_empty()
        || !native.target.registry_backed
        || native.target.implementation_generation != 0
        || native.invocation.arguments().exact_argv_len() != Some(2)
        || context.written_representations.is_none()
        || super::native_result::numeric_call_arguments(context, native.target).is_none()
    {
        return;
    }
    let Some(normal) = &outcomes.normal else {
        return;
    };
    if normal.source_step_observed()
        || normal.source_execution_observed(native.target.identity.as_ref())
    {
        return;
    }
    let Some((receiver, self_scope)) = prefix_receiver(normal, native, context) else {
        return;
    };
    let Some(selector) = native.words.get(2) else {
        return;
    };
    let Some(name) = native.invocation.arguments().literal_at(1) else {
        return;
    };
    let Some(origin) = &normal.current_source_origin else {
        return;
    };
    let Some(source) = super::executed_script_source::source_text(origin) else {
        return;
    };
    if super::ExecutedScriptSource::literal_word_base(source, selector, name, context.config)
        .is_none()
    {
        return;
    }
    let Some(entry) = receiver
        .class_target()
        .identity
        .as_ref()
        .and_then(|identity| normal.class_definitions.get(identity))
        .and_then(|definition| {
            definition
                .receiver_method_entries
                .get(&(super::SourceMethodReceiver::Instance, name.to_owned()))
        })
    else {
        return;
    };
    outcomes.normal_method_prefix = Some(Arc::new(SourceCapturedMethodPrefix {
        receiver,
        entry: entry.clone(),
        selector: selector.clone(),
        capture: super::CommandAllocationSite {
            source: Arc::clone(origin),
            offset: native.segment.span.start(),
        },
        builder: native.target.clone(),
        self_scope,
    }));
}

fn prefix_receiver(
    normal: &ModuleCommandBindings,
    native: super::SourceNativeInvocation<'_>,
    context: SourceExecutionContext<'_>,
) -> Option<(Arc<SourceObjectInstanceProof>, Option<CapturedSelfScope>)> {
    if let Some(receiver) = context
        .written_objects?
        .get(1)?
        .as_ref()
        .filter(|receiver| normal.receiver_allocation_is_current(receiver))
    {
        return Some((Arc::clone(receiver), None));
    }
    let head = native.invocation.arguments().literal_at(0)?;
    if head.contains("::")
        || context.registry.method_dispatch_keyword(head)
            != Some(tcl_registry::MethodDispatchKind::SelfDispatch)
        || normal.source_execution_observed(None)
    {
        return None;
    }
    let receiver = normal.active_method_receiver(context.frame)?;
    let origin = normal.current_source_origin.as_ref()?;
    super::ExecutedScriptSource::literal_word_base(
        super::executed_script_source::source_text(origin)?,
        native.words.get(1)?,
        head,
        context.config,
    )?;
    Some((
        Arc::clone(receiver),
        Some(CapturedSelfScope {
            frame: context.frame.clone(),
            dispatcher_generation: receiver.receiver_dispatcher_generation?,
            wrapper: None,
        }),
    ))
}

pub(super) fn retain_scoped_prefix(
    outcomes: &mut SourceOutcomes,
    native: super::SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    context: SourceExecutionContext<'_>,
) {
    let Some(operand) = facts.native_result.and_then(|contract| {
        contract.normal_scoped_prefix_operand(native.invocation.arguments(), facts.argument_offset)
    }) else {
        return;
    };
    if !native.target.registry_backed
        || native.target.implementation_generation != 0
        || !native.target.prepended.is_empty()
        || super::native_result::numeric_call_arguments(context, native.target).is_none()
    {
        return;
    }
    let Some(normal) = outcomes.normal.as_ref() else {
        return;
    };
    let Some(prefix) = context
        .written_method_prefixes
        .and_then(|arguments| arguments.get(operand + 1))
        .and_then(Option::as_ref)
    else {
        return;
    };
    let Some(scope) = prefix.self_scope.as_ref() else {
        return;
    };
    if scope.wrapper.is_some()
        || scope.frame != *context.frame
        || !prefix.is_current(normal)
        || normal.active_method_receiver(context.frame) != Some(&prefix.receiver)
        || normal.source_step_observed()
        || normal.source_execution_observed(native.target.identity.as_ref())
    {
        return;
    }
    let Some(origin) = normal.current_source_origin.as_ref() else {
        return;
    };
    let mut captured = prefix.as_ref().clone();
    captured.self_scope.as_mut().unwrap().wrapper = Some((
        native.target.clone(),
        super::CommandAllocationSite {
            source: Arc::clone(origin),
            offset: native.segment.span.start(),
        },
    ));
    outcomes.normal_method_prefix = Some(Arc::new(captured));
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct MethodPrefixStore {
    pub(super) receiver: crate::place::Place,
    pub(super) source: u32,
    pub(super) origin: Arc<super::SourceOriginId>,
    pub(super) prefix: Arc<SourceCapturedMethodPrefix>,
}

pub(super) fn retain_prefix_store(
    outcomes: &mut SourceOutcomes,
    facts: &tcl_registry::InvocationFacts,
    target: &SourceCommandTarget,
    arguments: tcl_registry::InvocationArguments<'_>,
    context: SourceExecutionContext<'_>,
) {
    if facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Set,
        )
        || !target.prepended.is_empty()
        || arguments.exact_argv_len() != Some(2)
        || super::native_result::numeric_call_arguments(context, target).is_none()
    {
        return;
    }
    let Some(normal) = &mut outcomes.normal else {
        return;
    };
    let Some(name) = arguments.literal_at(0) else {
        return;
    };
    let receiver = crate::var_resolve::resolve_literal_access(
        name,
        &normal.source_variables,
        false,
        context.registry,
        tcl_registry::TraceOperation::Write,
    );
    let Some(key) = crate::var_resolve::canonical_binding_value_key(&receiver) else {
        return;
    };
    Arc::make_mut(&mut normal.object_instances)
        .prefix_stores
        .remove(&key);
    let Some(prefix) = context
        .written_method_prefixes
        .and_then(|words| words.get(2))
        .and_then(Option::as_ref)
        .filter(|prefix| prefix.is_current(normal))
    else {
        return;
    };
    if receiver.dynamic
        || receiver.observed
        || normal.opaque_domain
        || normal.source_variables.contents_presence(&receiver)
            != crate::var_resolve::ContentsPresence::Defined
        || normal
            .source_variables
            .read_contents_origin(&receiver, context.registry)
            != crate::var_resolve::ContentsOrigin::WrittenAt(context.invocation_offset)
    {
        return;
    }
    let Some(origin) = &normal.current_source_origin else {
        return;
    };
    Arc::make_mut(&mut normal.object_instances)
        .prefix_stores
        .insert(
            key,
            MethodPrefixStore {
                receiver,
                source: context.invocation_offset,
                origin: Arc::clone(origin),
                prefix: Arc::clone(prefix),
            },
        );
    outcomes.normal_method_prefix = Some(Arc::clone(prefix));
}

pub(super) fn read_prefix(
    state: &ModuleCommandBindings,
    place: &crate::place::Place,
    registry: &tcl_registry::CommandRegistry,
) -> Option<Arc<SourceCapturedMethodPrefix>> {
    if state.opaque_domain
        || place.dynamic
        || place.observed
        || !state.source_variables.read_produces_value(place, registry)
    {
        return None;
    }
    let key = crate::var_resolve::canonical_binding_value_key(place)?;
    let store = state.object_instances.prefix_stores.get(&key)?;
    (store.receiver == *place
        && store.prefix.is_current(state)
        && state.source_variables.read_contents_origin(place, registry)
            == crate::var_resolve::ContentsOrigin::WrittenAt(store.source)
        && state.source_variables.contents_source(place) == Some(&store.origin))
    .then(|| Arc::clone(&store.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn captured(source: &str) -> Vec<SourceCapturedMethodPrefix> {
        let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let bindings = super::super::SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find("after 0").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("after", offset);
        binding
            .captured_method_prefix_arguments()
            .map(|(argument, prefix)| {
                assert_eq!(argument, 1);
                prefix.clone()
            })
            .collect()
    }

    #[test]
    fn reached_stored_prefix_retains_the_actual_receiver_and_original_selector() {
        let source = "oo::class create C {method tick {} {return OK}; method setup {} {set cb [list [self] tick]; after 0 $cb}}; C create obj; obj setup";
        let prefixes = captured(source);
        assert_eq!(prefixes.len(), 1);
        let prefix = &prefixes[0];
        assert_eq!(prefix.method_entry().name(), "tick");
        assert_eq!(
            prefix.method_entry().body().text.try_text().unwrap(),
            "return OK"
        );
        assert_eq!(
            prefix.selector().source().span.start(),
            u32::try_from(source.find("tick];").unwrap()).unwrap()
        );
        assert_eq!(
            prefix.capture_site().offset,
            u32::try_from(source.find("list [self]").unwrap()).unwrap()
        );
        assert_eq!(
            prefix.receiver().allocation().site.offset,
            u32::try_from(source.find("C create obj").unwrap()).unwrap()
        );
    }

    #[test]
    fn deferred_receiver_or_changed_prefix_dependencies_supply_no_registration_receipt() {
        for source in [
            "oo::class create C {method tick {} {}; method setup {} {set cb [list [self] tick]; after 0 $cb}}",
            "oo::class create C {method tick {} {}; method setup {} {set cb [list [self] tick]; set cb {other tick}; after 0 $cb}}; C create obj; obj setup",
            "oo::class create C {method tick {} {}; method setup {} {set cb [list [self] tick]; oo::define C method tick {} {return NEW}; after 0 $cb}}; C create obj; obj setup",
            "oo::class create C {method tick {} {}; method setup {} {set cb [list [self] tick]; unresolvedOperation; after 0 $cb}}; C create obj; obj setup",
            "proc list args {return OTHER}; oo::class create C {method tick {} {}; method setup {} {set cb [list [self] tick]; after 0 $cb}}; C create obj; obj setup",
        ] {
            assert!(captured(source).is_empty(), "{source}");
        }
    }

    #[test]
    fn internal_prefix_requires_the_actual_native_namespace_capture() {
        let source = "oo::class create C {method tick {} {return OK}; method setup {} {set cb [namespace code [list my tick]]; after 0 $cb}}; C create obj; obj setup";
        let prefixes = captured(source);
        assert_eq!(prefixes.len(), 1);
        assert_eq!(prefixes[0].method_entry().name(), "tick");
        assert_eq!(
            prefixes[0].selector().source().span.start(),
            u32::try_from(source.find("tick]];").unwrap()).unwrap(),
        );
        for source in [
            "oo::class create C {method tick {} {}; method setup {} {set cb [list my tick]; after 0 $cb}}; C create obj; obj setup",
            "oo::class create C {method tick {} {}; method setup {} {set cb [namespace code [list my tick]]; after 0 $cb}}",
            "oo::class create C {method tick {} {}; method setup {} {proc my args {return OTHER}; set cb [namespace code [list my tick]]; after 0 $cb}}; C create obj; obj setup",
            "oo::class create C {method tick {} {}; method setup {} {set cb [namespace code [list my tick]]; proc my args {return OTHER}; after 0 $cb}}; C create obj; obj setup",
            "rename namespace saved_namespace; proc namespace args {return OTHER}; oo::class create C {method tick {} {}; method setup {} {set cb [namespace code [list my tick]]; after 0 $cb}}; C create obj; obj setup",
        ] {
            assert!(captured(source).is_empty(), "{source}");
        }
    }
}
