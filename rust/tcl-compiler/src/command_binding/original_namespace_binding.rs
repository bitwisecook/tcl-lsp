// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Canonical namespace pattern transfers over exact source command cells.

use super::{
    Arc, BTreeSet, MayBinding, ModuleCommandBindings, SourceCommandKey, SourceNamespaceKey,
};
use crate::signature_scan::scope::SignatureNamespaceScope;
use tcl_core_types::NameBytes;
use tcl_syntax::{
    naming::{NamePolicyProtocol, NativeNamePurpose, NativeNamespacePatternSource},
    native_glob::{NativeGlobProtocol, NativeNameGlobPurpose},
};

fn policy(state: &ModuleCommandBindings) -> Option<NamePolicyProtocol> {
    if state.has_opaque_domain() || state.baseline.unknown_entry {
        return None;
    }
    state.baseline.execution_name_policy?.native_recipe()
}

fn pattern_source(
    state: &ModuleCommandBindings,
    destination: &SourceNamespaceKey,
    original: &[u8],
    purpose: NativeNamePurpose,
    policy: NamePolicyProtocol,
) -> Option<(Option<SourceNamespaceKey>, NameBytes)> {
    let scope = state.original_namespace_geometry(destination, policy)?;
    let parts = policy
        .recipe()
        .namespace_pattern_parts(scope.context()?, original, purpose)
        .ok()?;
    let source = match parts.source {
        Some(source) => {
            let scope = match source {
                NativeNamespacePatternSource::C(path) => SignatureNamespaceScope::C(path),
                NativeNamespacePatternSource::Jim(object) => SignatureNamespaceScope::Jim(object),
            };
            let mut selected = state.namespaces.iter().filter(|namespace| {
                state
                    .original_namespace_geometry(namespace, policy)
                    .as_ref()
                    == Some(&scope)
            });
            let namespace = selected.next()?.clone();
            if selected.next().is_some() {
                return None;
            }
            Some(namespace)
        }
        None => None,
    };
    Some((source, parts.tail))
}

fn matches(
    policy: NamePolicyProtocol,
    purpose: NativeNameGlobPurpose,
    pattern: &[u8],
    name: &[u8],
) -> Option<bool> {
    NativeGlobProtocol::from_name_policy(policy)
        .match_name_pattern(purpose, pattern, name)
        .ok()
}

/// Enumerate canonical source cells and independently selected fixed Registry
/// metadata. A native entry's absent rows are never filled by the Registry.
fn keys(
    state: &ModuleCommandBindings,
    policy: NamePolicyProtocol,
) -> Option<BTreeSet<SourceCommandKey>> {
    let mut keys = state
        .bindings
        .keys()
        .filter(|key| matches!(key, SourceCommandKey::Slot { .. }))
        .cloned()
        .collect::<BTreeSet<_>>();
    if state.baseline.native_entry.is_none() {
        let root = state.source_root_namespace_key()?;
        for name in state.baseline.semantics.binding_names() {
            let paths = state.original_registry_command_paths(&root, name, policy)?;
            for path in paths {
                keys.extend(path);
            }
        }
    }
    Some(keys)
}

pub(super) fn apply_exports(
    state: &mut ModuleCommandBindings,
    namespace: &SourceNamespaceKey,
    inputs: &[&[u8]],
) -> Option<()> {
    let policy = policy(state)?;
    if policy.recipe().is_jim084() {
        // The selected Jim export command ignores all operands.
        return Some(());
    }
    let (clear, inputs) =
        tcl_registry::NamespaceTransition::export_pattern_byte_operands(inputs, policy)?;
    for input in inputs {
        if pattern_source(
            state,
            namespace,
            input,
            NativeNamePurpose::NamespaceExportPattern,
            policy,
        )?
        .0
        .is_some()
        {
            return None;
        }
    }
    let selected_patterns = inputs
        .iter()
        .map(|input| {
            policy
                .recipe()
                .namespace_pattern_input(input, NativeNamePurpose::NamespaceExportPattern)
                .ok()
                .map(|input| NameBytes::from(input.selected()))
        })
        .collect::<Option<Vec<_>>>()?;
    let previous = state
        .namespace_exports
        .get(namespace)
        .cloned()
        .unwrap_or_else(|| BTreeSet::from([Vec::new()]));
    let alternatives = previous
        .into_iter()
        .map(|mut patterns| {
            if clear {
                patterns.clear();
            }
            for value in &selected_patterns {
                if !patterns.contains(value) {
                    patterns.push(value.clone());
                }
            }
            patterns
        })
        .collect();
    Arc::make_mut(&mut state.namespace_exports).insert(namespace.clone(), alternatives);
    if clear {
        Arc::make_mut(&mut state.unknown_export_namespaces).remove(namespace);
    }
    Some(())
}

pub(super) fn apply_import(
    state: &mut ModuleCommandBindings,
    destination: &SourceNamespaceKey,
    original: &[u8],
    force: Option<bool>,
) -> Option<()> {
    let policy = policy(state)?;
    if policy.recipe().is_jim084() {
        // Jim namespace import is an independently selected source-name helper.
        // Its flat-name enumeration and callback transfer require that owner.
        return None;
    }
    let name_binding =
        state.baseline.import_binding == Some(tcl_dialect::NamespaceImportBinding::SourceName);
    let force = if name_binding { Some(true) } else { force };
    let (source, pattern) = pattern_source(
        state,
        destination,
        original,
        NativeNamePurpose::NamespaceImportPattern,
        policy,
    )?;
    let Some(source) = source else {
        return name_binding.then_some(());
    };
    if &source == destination {
        return None;
    }
    if state.unknown_lookup_namespaces.contains(&source)
        || state.unknown_export_namespaces.contains(&source)
    {
        Arc::make_mut(&mut state.unknown_lookup_namespaces).insert(destination.clone());
        return None;
    }
    let exports = state
        .namespace_exports
        .get(&source)
        .cloned()
        .unwrap_or_else(|| BTreeSet::from([Vec::new()]));
    let scope = state.original_namespace_geometry(destination, policy)?;
    for key in keys(state, policy)? {
        if key.holder().as_ref() != &source {
            continue;
        }
        let SourceCommandKey::Slot { simple, .. } = &key else {
            return None;
        };
        if !matches(
            policy,
            NativeNameGlobPurpose::ImportSearch,
            pattern.as_bytes(),
            simple.as_bytes(),
        )? {
            continue;
        }
        let mut exported = 0;
        for patterns in &exports {
            if name_binding
                || patterns
                    .iter()
                    .map(|pattern| {
                        matches(
                            policy,
                            NativeNameGlobPurpose::ExportFilter,
                            pattern.as_bytes(),
                            simple.as_bytes(),
                        )
                    })
                    .collect::<Option<Vec<_>>>()?
                    .into_iter()
                    .any(|matched| matched)
            {
                exported += 1;
            }
        }
        if exported == 0 {
            continue;
        }
        let implementations = state.original_bindings_for_key(&key)?;
        if implementations == BTreeSet::from([MayBinding::Missing]) {
            continue;
        }
        let destination_slot = policy
            .recipe()
            .command_publication_slot(scope.context()?, simple.as_bytes())
            .ok()?;
        let imported = state.original_command_key_for_slot(&destination_slot, policy)?;
        let prior = state.original_bindings_for_key(&imported)?;
        if force == Some(false) && !prior.contains(&MayBinding::Missing) {
            return None;
        }
        let (mut links, source_bindings) =
            super::imported_binding_links(state, &key, implementations, name_binding);
        state.replace(key, source_bindings);
        if exported != exports.len() || force.is_none() || (force == Some(false) && prior.len() > 1)
        {
            links.extend(prior);
        }
        state.replace(imported, links);
    }
    Some(())
}

pub(super) fn apply_forget(
    state: &mut ModuleCommandBindings,
    destination: &SourceNamespaceKey,
    original: &[u8],
) -> Option<()> {
    let policy = policy(state)?;
    let (source, pattern) = pattern_source(
        state,
        destination,
        original,
        NativeNamePurpose::NamespaceForgetPattern,
        policy,
    )?;
    let mut replacements = Vec::new();
    for (key, alternatives) in state.bindings.iter() {
        if key.holder().as_ref() != destination {
            continue;
        }
        let SourceCommandKey::Slot { simple, .. } = key else {
            return None;
        };
        let mut changed = false;
        let mut bindings = BTreeSet::new();
        for binding in alternatives {
            let matched = if let MayBinding::Imported(token) = binding {
                if let Some(source) = &source {
                    let mut origins = state.bindings.iter().filter(|(_, implementations)| implementations.iter().any(|implementation| matches!(implementation, MayBinding::Target(target) if target.token.as_ref() == Some(&token.origin))));
                    let (origin, _) = origins.next()?;
                    if origins.next().is_some() {
                        return None;
                    }
                    let SourceCommandKey::Slot { namespace, simple } = origin else {
                        return None;
                    };
                    namespace == source
                        && matches(
                            policy,
                            NativeNameGlobPurpose::ForgetOriginFilter,
                            pattern.as_bytes(),
                            simple.as_bytes(),
                        )?
                } else {
                    matches(
                        policy,
                        NativeNameGlobPurpose::ForgetOwnSearch,
                        pattern.as_bytes(),
                        simple.as_bytes(),
                    )?
                }
            } else {
                false
            };
            changed |= matched;
            bindings.insert(if matched {
                MayBinding::Missing
            } else {
                binding.clone()
            });
        }
        if changed {
            replacements.push((key.clone(), bindings));
        }
    }
    state.replace_bindings(replacements);
    Some(())
}

/// Independent quiet intrinsic pattern-operation premises and expected table
/// transfer. Publication geometry and an `OnOk` transition cannot issue this seal.
#[derive(Clone)]
pub(super) struct OriginalNamespaceBinding {
    site: super::CommandAllocationSite,
    words: Vec<crate::ir::WordExpr>,
    config: tcl_lexer::LexerConfig,
    handler: super::SourceCommandTarget,
    namespace: SourceNamespaceKey,
    inputs: Vec<crate::signature_scan::scope::SignatureSourceNameInput>,
    expected: Box<ModuleCommandBindings>,
}

fn quiet(state: &ModuleCommandBindings) -> bool {
    !state.has_opaque_domain()
        && state.baseline.native_entry.is_none()
        && !state.baseline.unknown_entry
        && !state.source_step_observed()
        && !state.source_execution_observed(None)
        && state.command_observers.is_quiet()
}

#[cfg(debug_assertions)]
fn trace_capture(
    native: super::SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
) {
    use tcl_registry::{NamespaceTransition, StateTransition};
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some()
        && facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| {
                transitions.facts().iter().any(|fact| {
                    matches!(
                        &fact.transition,
                        StateTransition::Namespace(
                            NamespaceTransition::Export { .. }
                                | NamespaceTransition::Import { .. }
                                | NamespaceTransition::Forget { .. }
                        )
                    )
                })
            })
    {
        let count = native.invocation.arguments().exact_argv_len();
        let inputs = count.map(|count| {
            (0..count)
                .map(|index| {
                    native
                        .original_variable_operands
                        .input(index, &state.source_variables)
                        .map(|input| {
                            (
                                input.original_word_key().is_some(),
                                input.is_current(&state.source_variables),
                            )
                        })
                })
                .collect::<Vec<_>>()
        });
        eprintln!(
            "ORIGINAL_NAMESPACE_BINDING_CAPTURE site={} hook={:?} quiet={} current={} kind={:?} registry={} generation={} runtime_generation={} prefixes={} arity={:?} callbacks={:?} argc={count:?} inputs={inputs:?} context={:?}",
            native.segment.span.start(),
            facts.analyser_hook,
            quiet(state),
            state.retained_target_is_current(native.target),
            native.target.kind,
            native.target.registry_backed,
            native.target.implementation_generation,
            native.target.runtime_implementation_generation.is_some(),
            native.target.prepended.len(),
            facts.arity_accepts_frozen_arguments(),
            facts.effects.callback().kinds,
            context.namespace_identity()
        );
    }
}

fn expected_pattern_transfer(
    native: super::SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    namespace: &SourceNamespaceKey,
    policy: NamePolicyProtocol,
) -> Option<Box<ModuleCommandBindings>> {
    use tcl_registry::{
        NamespaceTransition, NamespaceTransitionTarget, StateTransition, hooks::AnalyserHookId,
    };
    let transitions = facts.state_transitions.declared()?;
    let mut expected = Box::new(state.clone());
    for fact in transitions.facts() {
        match (&fact.transition, facts.analyser_hook) {
            (
                StateTransition::Namespace(NamespaceTransition::Export {
                    namespace: NamespaceTransitionTarget::Current,
                    patterns,
                }),
                Some(AnalyserHookId::NamespaceExport),
            ) => {
                let operands = patterns
                    .iter()
                    .map(|subject| {
                        super::original_command_table::original_operand(
                            native, subject, state, context,
                        )
                    })
                    .collect::<Option<Vec<_>>>()?;
                let inputs = operands
                    .iter()
                    .map(crate::signature_scan::scope::SignatureSourceNameInput::bytes)
                    .collect::<Vec<_>>();
                apply_exports(&mut expected, namespace, &inputs)?;
            }
            (
                StateTransition::Namespace(NamespaceTransition::Import {
                    namespace: NamespaceTransitionTarget::Current,
                    force: Some(force),
                    patterns,
                }),
                Some(AnalyserHookId::NamespaceImport),
            ) => {
                for subject in patterns {
                    let input = super::original_command_table::original_operand(
                        native, subject, state, context,
                    )?;
                    // The narrow fresh-destination envelope invokes no
                    // replacement cleanup or imported-alias loop callback.
                    import_is_fresh_and_closed(&expected, namespace, input.bytes(), policy)?;
                    apply_import(&mut expected, namespace, input.bytes(), Some(*force))?;
                }
            }
            (
                StateTransition::Namespace(NamespaceTransition::Forget {
                    namespace: NamespaceTransitionTarget::Current,
                    patterns,
                }),
                Some(AnalyserHookId::NamespaceForget),
            ) => {
                // A removed import must not be the last surviving owner
                // of an unrepresented command cleanup/destructor.
                import_origins_are_live(&expected, namespace)?;
                for subject in patterns {
                    let input = super::original_command_table::original_operand(
                        native, subject, state, context,
                    )?;
                    apply_forget(&mut expected, namespace, input.bytes())?;
                }
            }
            _ => return None,
        }
    }
    Some(expected)
}

impl OriginalNamespaceBinding {
    pub(super) fn capture(
        native: super::SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &ModuleCommandBindings,
        context: super::SourceExecutionContext<'_>,
    ) -> Option<Self> {
        use tcl_registry::hooks::AnalyserHookId;
        #[cfg(debug_assertions)]
        trace_capture(native, facts, state, context);
        if !quiet(state)
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || !native.target.registry_backed
            || native.target.kind != super::BindingKind::Builtin
            || native.target.implementation_generation != 0
            || native.target.runtime_implementation_generation.is_some()
            || !native.target.prepended.is_empty()
            || !state.retained_target_is_current(native.target)
            || facts.effects.callback().kinds != tcl_registry::world_effect::CallbackKinds::NONE
            || !matches!(
                facts.analyser_hook,
                Some(
                    AnalyserHookId::NamespaceExport
                        | AnalyserHookId::NamespaceImport
                        | AnalyserHookId::NamespaceForget
                )
            )
        {
            return None;
        }
        let policy = policy(state)?;
        let tcl_syntax::naming::NativeNameProtocol::C(version) = policy.recipe() else {
            return None;
        };
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let spec = context.registry.native_registration_source_descriptor(
            native.target.registry_identity()?,
            dialect,
            context.realm,
        )?;
        if tcl_registry::registry::spec_pack_of(spec) != Some("tcl") {
            return None;
        }
        let count = native.invocation.arguments().exact_argv_len()?;
        let inputs = (0..count)
            .map(|index| {
                native
                    .original_variable_operands
                    .input(index, &state.source_variables)
                    .cloned()
            })
            .collect::<Option<Vec<_>>>()?;
        if inputs
            .iter()
            .any(|input| input.policy() != policy || input.original_word_key().is_none())
        {
            return None;
        }
        let namespace = context.namespace_identity();
        state.original_namespace_geometry(&namespace, policy)?;
        if state.unknown_lookup_namespaces.contains(&namespace) {
            return None;
        }
        let expected =
            expected_pattern_transfer(native, facts, state, context, &namespace, policy)?;
        if !quiet(&expected) {
            return None;
        }
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_NAMESPACE_BINDING_CAPTURED site={} namespace={namespace:?} exports={} bindings={} transitions={}",
                native.segment.span.start(),
                expected.namespace_exports.keys().count(),
                expected.bindings.iter().count(),
                facts.state_transitions.declared()?.facts().len()
            );
        }
        Some(Self {
            site: super::CommandAllocationSite {
                source: Arc::clone(state.current_source_origin.as_ref()?),
                offset: native.segment.span.start(),
            },
            words: native.words.to_vec(),
            config: context.config,
            handler: native.target.clone(),
            namespace,
            inputs,
            expected,
        })
    }

    pub(super) fn completed(
        &self,
        native: super::SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        context: super::SourceExecutionContext<'_>,
    ) -> bool {
        let completed = quiet(state)
            && context.config == self.config
            && native.words == self.words
            && *native.target == self.handler
            && state.retained_target_is_current(&self.handler)
            && native.segment.span.start() == self.site.offset
            && state.current_source_origin.as_ref() == Some(&self.site.source)
            && context.namespace_identity() == self.namespace
            && self
                .inputs
                .iter()
                .all(|input| input.is_current(&state.source_variables))
            && state.bindings == self.expected.bindings
            && state.objects == self.expected.objects
            && state.namespace_exports == self.expected.namespace_exports
            && state.namespaces == self.expected.namespaces
            && state.unknown_lookup_namespaces == self.expected.unknown_lookup_namespaces
            && state.unknown_export_namespaces == self.expected.unknown_export_namespaces;
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_NAMESPACE_BINDING_COMPLETED site={} complete={completed} quiet={} handler={} source={} context={} inputs={} bindings={} objects={} exports={} namespaces={} unknown_lookup={} unknown_exports={}",
                native.segment.span.start(),
                quiet(state),
                *native.target == self.handler,
                state.current_source_origin.as_ref() == Some(&self.site.source),
                context.namespace_identity() == self.namespace,
                self.inputs
                    .iter()
                    .all(|input| input.is_current(&state.source_variables)),
                state.bindings == self.expected.bindings,
                state.objects == self.expected.objects,
                state.namespace_exports == self.expected.namespace_exports,
                state.namespaces == self.expected.namespaces,
                state.unknown_lookup_namespaces == self.expected.unknown_lookup_namespaces,
                state.unknown_export_namespaces == self.expected.unknown_export_namespaces
            );
        }
        completed
    }
}

fn import_origins_are_live(
    state: &ModuleCommandBindings,
    namespace: &SourceNamespaceKey,
) -> Option<()> {
    for (key, bindings) in state.bindings.iter() {
        if key.holder().as_ref() != namespace {
            continue;
        }
        for binding in bindings {
            let MayBinding::Imported(imported) = binding else {
                continue;
            };
            let mut origins = state.bindings.values().filter_map(|bindings| {
                if bindings.len() != 1 {
                    return None;
                }
                let MayBinding::Target(target) = bindings.first()? else {
                    return None;
                };
                (target.token.as_ref() == Some(&imported.origin)
                    && target.terminal
                    && target.prepended.is_empty()
                    && matches!(
                        target.kind,
                        super::BindingKind::Proc
                            | super::BindingKind::Builtin
                            | super::BindingKind::Class
                    ))
                .then_some(target.token.as_ref()?.clone())
            });
            origins.next()?;
            if origins.next().is_some() {
                return None;
            }
        }
    }
    Some(())
}

fn import_is_fresh_and_closed(
    state: &ModuleCommandBindings,
    destination: &SourceNamespaceKey,
    original: &[u8],
    policy: NamePolicyProtocol,
) -> Option<()> {
    let preload = tcl_registry::NamespaceTransition::import_preload_command(policy)?;
    let root = state.source_root_namespace_key()?;
    for path in state.original_registry_command_paths(&root, preload, policy)? {
        for key in path {
            if state.original_bindings_for_key(&key)? != BTreeSet::from([MayBinding::Missing]) {
                return None;
            }
        }
    }
    let (source, pattern) = pattern_source(
        state,
        destination,
        original,
        NativeNamePurpose::NamespaceImportPattern,
        policy,
    )?;
    let source = source?;
    if &source == destination
        || state.unknown_lookup_namespaces.contains(&source)
        || state.unknown_export_namespaces.contains(&source)
    {
        return None;
    }
    let exports = state
        .namespace_exports
        .get(&source)
        .cloned()
        .unwrap_or_else(|| BTreeSet::from([Vec::new()]));
    if exports.len() != 1 {
        return None;
    }
    let scope = state.original_namespace_geometry(destination, policy)?;
    for key in keys(state, policy)? {
        if key.holder().as_ref() != &source {
            continue;
        }
        let SourceCommandKey::Slot { simple, .. } = &key else {
            return None;
        };
        if !matches(
            policy,
            NativeNameGlobPurpose::ImportSearch,
            pattern.as_bytes(),
            simple.as_bytes(),
        )? {
            continue;
        }
        let exported = exports
            .first()?
            .iter()
            .map(|pattern| {
                matches(
                    policy,
                    NativeNameGlobPurpose::ExportFilter,
                    pattern.as_bytes(),
                    simple.as_bytes(),
                )
            })
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .any(|matched| matched);
        if !exported {
            continue;
        }
        let bindings = state.original_bindings_for_key(&key)?;
        if bindings == BTreeSet::from([MayBinding::Missing]) {
            continue;
        }
        let alternatives = bindings.iter().collect::<Vec<_>>();
        let [MayBinding::Target(target)] = alternatives.as_slice() else {
            return None;
        };
        if !target.terminal
            || !target.prepended.is_empty()
            || !matches!(
                target.kind,
                super::BindingKind::Proc | super::BindingKind::Builtin | super::BindingKind::Class
            )
            || target.token.is_none()
        {
            return None;
        }
        let slot = policy
            .recipe()
            .command_publication_slot(scope.context()?, simple.as_bytes())
            .ok()?;
        let key = state.original_command_key_for_slot(&slot, policy)?;
        if state.original_bindings_for_key(&key)? != BTreeSet::from([MayBinding::Missing]) {
            return None;
        }
    }
    Some(())
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
    fn original_namespace_export_storage_keeps_exact_opaque_units() {
        // Implementation contract: naming.namespace.original-byte-pattern-transfers
        // docs/design/analysis/name-resolution-proofs/namespace-original-byte-pattern-transfers.md
        // Native question: naming.namespace.source-export-filter-units
        // docs/design/analysis/name-resolution-proofs/namespace-source-export-filter-units.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = r"namespace eval A {proc p\uD800 {} {}; proc p\uD801 {} {}; namespace export p\uD800}; set checkpoint READY";
            let bindings = analyse(source, version);
            let point = bindings.invocation_at_source(
                "set",
                u32::try_from(source.find("set checkpoint").unwrap()).unwrap(),
            );
            let state = &point
                .lookup_state
                .as_ref()
                .unwrap_or_else(|| panic!("{}", version.dialect_name()))
                .state;
            let namespace = super::super::SourceNamespaceKey::authored("::A");
            assert_eq!(
                state.namespace_exports.get(&namespace),
                Some(&super::BTreeSet::from([vec![
                    tcl_core_types::NameBytes::from(b"p\xed\xa0\x80")
                ]])),
                "{version:?}"
            );
        }
    }

    #[test]
    fn original_namespace_import_and_forget_keep_actual_origin_with_closed_prelude() {
        // Implementation contract: naming.namespace.original-quiet-pattern-completion
        // docs/design/analysis/name-resolution-proofs/namespace-original-quiet-pattern-completion.md
        // Native question: naming.namespace.source-import-token-lifetime
        // docs/design/analysis/name-resolution-proofs/namespace-source-import-token-lifetime.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = r"rename auto_import savedPrelude; namespace eval A {proc p\uD800 {} {}; proc p\uD801 {} {}; namespace export p\uD800 p\uD801}; namespace eval B {namespace import ::A::*; rename p\uD800 moved; namespace forget ::A::p\uD800}; set checkpoint READY";
            let bindings = analyse(source, version);
            let world = bindings
                .original_completed_command_world()
                .unwrap_or_else(|| panic!("{}", version.dialect_name()));
            let publications = world.declarations().collect::<Vec<_>>();
            let imports = publications
                .iter()
                .filter(|publication| {
                    publication.kind() == super::super::OriginalCommandPublicationKind::Imported
                })
                .collect::<Vec<_>>();
            let [imported] = imports.as_slice() else {
                panic!("{version:?}: one surviving distinct import")
            };
            assert_eq!(imported.slot().simple.as_bytes(), b"p\xed\xa0\x81");
            let origin = imported.imported_origin().unwrap();
            assert!(
                publications
                    .iter()
                    .any(|source| source.declaration_site() == origin
                        && source.slot().simple.as_bytes() == b"p\xed\xa0\x81")
            );
        }
    }

    #[test]
    fn original_namespace_import_completion_requires_closed_prelude_and_destinations() {
        // Implementation contract: naming.namespace.original-quiet-pattern-completion
        // docs/design/analysis/name-resolution-proofs/namespace-original-quiet-pattern-completion.md
        // Native question: naming.namespace.source-pattern-options-and-errors
        // docs/design/analysis/name-resolution-proofs/namespace-source-pattern-options-and-errors.md
        for version in tcl_dialect::TclVersion::ALL {
            for source in [
                "namespace eval A {proc p {} {}; namespace export p}; namespace eval B {namespace import ::A::p}",
                "rename auto_import savedPrelude; namespace eval A {proc p {} {}; namespace export p}; namespace eval B {proc p {} {}; namespace import -force ::A::p}",
                "rename auto_import savedPrelude; namespace eval A {namespace import ::A::p}",
                "namespace export ::A::p",
            ] {
                assert!(
                    analyse(source, version)
                        .original_completed_command_world()
                        .is_none(),
                    "{version:?}: {source}"
                );
            }
        }
    }
}
