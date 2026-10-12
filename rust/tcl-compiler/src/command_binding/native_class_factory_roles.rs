// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual backend factory roles, independent of source transfer completion.

use super::{
    BindingKind, MayBinding, ModuleCommandBindings, SourceCommandTarget, SourceNamespaceKey,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_registry::native_tcloo_bootstrap::{
    NativeClassFactoryConstructor, NativeClassFactoryRecipe,
};
use tcl_runtime_api::native_oo::{
    NativeOoBootstrapRole as Role, NativeOoClassObservation, NativeOoIntrinsicMethod as Intrinsic,
    NativeOoLifecycleObservation as Lifecycle, NativeOoMethodObservation,
};

/// Same-entry observations and their independently current command allocations.
/// This closes a stock role/dispatch query, never manufacture Normal or a body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct NativeClassFactoryState {
    interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    epoch: u64,
    rows: Vec<(NativeOoClassObservation, SourceCommandTarget)>,
}

impl NativeClassFactoryState {
    pub(super) fn capture(
        factory: &SourceCommandTarget,
        selector: &SignatureSourceNameInput,
        recipe: NativeClassFactoryRecipe,
        state: &ModuleCommandBindings,
    ) -> Option<(Self, Option<SourceCommandTarget>)> {
        let entry = state.baseline.native_entry.as_ref()?;
        let runtime = factory.identity.as_ref()?.runtime?;
        let generation = factory.runtime_implementation_generation?;
        if runtime.interpreter != entry.interpreter
            || selector.policy().recipe()
                != tcl_syntax::naming::NativeNameProtocol::C(recipe.version())
            || !selector.is_current(&state.source_variables)
            || matches!(selector, SignatureSourceNameInput::OriginalVariableRoot(_))
        {
            return None;
        }
        let selected = entry.original_oo_class(runtime.token, generation)?;
        let expected_role = match recipe.constructor() {
            NativeClassFactoryConstructor::ClassDefinition => Role::ClassFactory,
            NativeClassFactoryConstructor::ConfigurableThenClassDefinition => {
                Role::ConfigurableFactory
            }
        };
        if selected.bootstrap_role != Some(expected_role) {
            return None;
        }
        #[cfg(debug_assertions)]
        trace_selected_factory_row(selected);
        let (class_root, object_root) =
            original_factory_roots(entry, selected, recipe, expected_role)?;
        // Both the original operand and actual method records use the selected
        // counted member-name purpose; no reporting label is a method key.
        let name = selector
            .policy()
            .recipe()
            .oo_method_input(selector.bytes())
            .ok()?;
        let manufacturer = recipe
            .grammar()
            .manufacturer(std::str::from_utf8(name.selected()).ok()?)?;
        let intrinsic = match (
            manufacturer.names_instance_at,
            manufacturer.constructor_args_from,
        ) {
            (Some(1), 2) => Intrinsic::Create,
            (None, 1) => Intrinsic::New,
            _ => return None,
        };
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_NATIVE_FACTORY_MANUFACTURER role={:?} intrinsic={intrinsic:?} current={}",
                selected.bootstrap_role,
                current_manufacturer(selected, class_root, selector, intrinsic).is_some()
            );
        }
        current_manufacturer(selected, class_root, selector, intrinsic)?;
        let mut rows = Vec::new();
        for row in [selected, class_root, object_root] {
            retain_row(&mut rows, row, state)?;
        }
        let support = if let Some(support_recipe) = recipe.support() {
            Some(capture_factory_support(
                support_recipe,
                state,
                entry,
                class_root,
                object_root,
                &mut rows,
            )?)
        } else {
            None
        };
        if !definition_namespace_is(
            state,
            selected.class_definition_namespace.as_ref(),
            recipe
                .stored_factory_definition_namespace(
                    tcl_registry::definer::DefinitionReceiver::Instance,
                )
                .unwrap_or_else(|| {
                    recipe.definition_namespace(tcl_registry::definer::DefinitionReceiver::Instance)
                }),
            recipe
                .stored_factory_definition_namespace(
                    tcl_registry::definer::DefinitionReceiver::Instance,
                )
                .is_some(),
        ) || !definition_namespace_is(
            state,
            selected.object_definition_namespace.as_ref(),
            recipe.definition_namespace(tcl_registry::definer::DefinitionReceiver::Class),
            false,
        ) {
            return None;
        }
        Some((
            Self {
                interpreter: entry.interpreter,
                epoch: entry.epoch,
                rows,
            },
            support,
        ))
    }

    pub(super) fn is_current(&self, state: &ModuleCommandBindings) -> bool {
        let Some(entry) = state.baseline.native_entry.as_ref() else {
            return false;
        };
        entry.interpreter == self.interpreter
            && entry.epoch == self.epoch
            && !state.tainted_object_dispatch.contains("*")
            && self.rows.iter().all(|(row, target)| {
                state.retained_target_is_current(target)
                    && !state.tainted_object_dispatch.contains(&target.command)
                    && target.runtime_implementation_generation
                        == state.runtime_implementation_generation(target.identity.as_ref())
                    && entry.original_oo_class(row.command_token, row.implementation_generation)
                        == Some(row)
            })
    }
}

#[cfg(debug_assertions)]
fn trace_selected_factory_row(selected: &NativeOoClassObservation) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        eprintln!(
            "ORIGINAL_NATIVE_FACTORY_ROW role={:?} maker={:?} object={:?} superclass={:?} class_mixins={} object_mixins={} class_filters={:?} object_filters={:?} class_namespace={} object_namespace={} ctor={:?} dtor={:?}",
            selected.bootstrap_role,
            selected.maker,
            selected.object,
            selected.superclasses,
            selected.class_mixins.len(),
            selected.object_mixins.len(),
            selected.class_filters.as_ref().map(Vec::len),
            selected.object_filters.as_ref().map(Vec::len),
            selected.class_definition_namespace.is_some(),
            selected.object_definition_namespace.is_some(),
            selected.constructor,
            selected.destructor
        );
    }
}

fn original_factory_roots<'a>(
    entry: &'a tcl_runtime_api::NativeCompilationEntry,
    selected: &NativeOoClassObservation,
    recipe: NativeClassFactoryRecipe,
    expected_role: Role,
) -> Option<(&'a NativeOoClassObservation, &'a NativeOoClassObservation)> {
    let class_root = role_row(entry, Role::ClassFactory)?;
    let object_root = role_row(entry, Role::ObjectRoot)?;
    if class_root.maker != class_root.object
        || class_root.superclasses != [object_root.object]
        || object_root.maker != class_root.object
        || !object_root.superclasses.is_empty()
        || !intrinsic_constructor(class_root, Intrinsic::ClassConstructor)
        || !matches!(object_root.constructor, Lifecycle::Absent)
    {
        return None;
    }
    if selected.maker != class_root.object
        || selected.foundation_epoch != class_root.foundation_epoch
        || object_root.foundation_epoch != class_root.foundation_epoch
        || (expected_role == Role::ConfigurableFactory
            && selected.superclasses != [class_root.object])
        || !intrinsic_constructor(
            selected,
            match recipe.constructor() {
                NativeClassFactoryConstructor::ClassDefinition => Intrinsic::ClassConstructor,
                NativeClassFactoryConstructor::ConfigurableThenClassDefinition => {
                    Intrinsic::ConfigurableConstructor
                }
            },
        )
    {
        return None;
    }
    Some((class_root, object_root))
}

fn capture_factory_support(
    support_recipe: tcl_registry::native_tcloo_bootstrap::NativeConfigurableSupportRecipe,
    state: &ModuleCommandBindings,
    entry: &tcl_runtime_api::NativeCompilationEntry,
    class_root: &NativeOoClassObservation,
    object_root: &NativeOoClassObservation,
    rows: &mut Vec<(NativeOoClassObservation, SourceCommandTarget)>,
) -> Option<SourceCommandTarget> {
    let root = state.source_root_namespace_key()?;
    let policy = state.baseline.execution_name_policy?.native_recipe()?;
    let (_, occupied) = state.original_occupied_registry_metadata_command(
        &root,
        support_recipe.command(),
        policy,
    )?;
    let alternatives = occupied.iter().collect::<Vec<_>>();
    let [MayBinding::Target(raw)] = alternatives.as_slice() else {
        return None;
    };
    let target = source_target(raw, state);
    let runtime = target.identity.as_ref()?.runtime?;
    if runtime.interpreter != entry.interpreter || !target.prepended.is_empty() {
        return None;
    }
    let support =
        entry.original_oo_class(runtime.token, target.runtime_implementation_generation?)?;
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        eprintln!(
            "ORIGINAL_NATIVE_FACTORY_SUPPORT role={:?} maker_match={} superclass_match={} epoch_match={} ctor_absent={} dtor_absent={} class_namespace={} object_namespace={}",
            support.bootstrap_role,
            support.maker == class_root.object,
            support.superclasses == [object_root.object],
            support.foundation_epoch == class_root.foundation_epoch,
            matches!(support.constructor, Lifecycle::Absent),
            matches!(support.destructor, Lifecycle::Absent),
            definition_namespace_is(
                state,
                support.class_definition_namespace.as_ref(),
                support_recipe
                    .definition_namespace(tcl_registry::definer::DefinitionReceiver::Instance),
                true
            ),
            definition_namespace_is(
                state,
                support.object_definition_namespace.as_ref(),
                support_recipe
                    .definition_namespace(tcl_registry::definer::DefinitionReceiver::Class),
                true
            )
        );
    }
    if support.bootstrap_role != Some(Role::ConfigurableSupport)
        || support.maker != class_root.object
        || support.superclasses != [object_root.object]
        || support.foundation_epoch != class_root.foundation_epoch
        || !matches!(support.constructor, Lifecycle::Absent)
        || !matches!(support.destructor, Lifecycle::Absent)
        || !definition_namespace_is(
            state,
            support.class_definition_namespace.as_ref(),
            support_recipe
                .definition_namespace(tcl_registry::definer::DefinitionReceiver::Instance),
            true,
        )
        || !definition_namespace_is(
            state,
            support.object_definition_namespace.as_ref(),
            support_recipe.definition_namespace(tcl_registry::definer::DefinitionReceiver::Class),
            true,
        )
    {
        return None;
    }
    retain_row(rows, support, state)?;
    Some(target)
}

fn role_row(
    entry: &tcl_runtime_api::NativeCompilationEntry,
    role: Role,
) -> Option<&NativeOoClassObservation> {
    let mut rows = entry
        .oo_classes
        .as_ref()?
        .classes
        .iter()
        .filter(|row| row.bootstrap_role == Some(role));
    let row = rows.next()?;
    if rows.next().is_some() {
        return None;
    }
    let retained = entry.original_oo_class(row.command_token, row.implementation_generation)?;
    (retained == row).then_some(row)
}

fn intrinsic_constructor(row: &NativeOoClassObservation, intrinsic: Intrinsic) -> bool {
    matches!(&row.constructor, Lifecycle::Present(method)
        if method.name.is_none() && !method.visibility_only && method.intrinsic == Some(intrinsic))
        && matches!(row.destructor, Lifecycle::Absent)
}

fn definition_namespace_is(
    state: &ModuleCommandBindings,
    observed: Option<&tcl_runtime_api::native_compilation::NativeNamespaceContext>,
    expected: &str,
    required: bool,
) -> bool {
    match observed {
        Some(observed) => {
            state.namespace_for_rooted_operand(expected).as_ref()
                == Some(&SourceNamespaceKey::Native(observed.clone()))
        }
        None => !required, // Backend capture explicitly observed an unset field.
    }
}

fn current_manufacturer(
    object: &NativeOoClassObservation,
    maker: &NativeOoClassObservation,
    selector: &SignatureSourceNameInput,
    intrinsic: Intrinsic,
) -> Option<()> {
    let selected =
        |methods: &[NativeOoMethodObservation]| -> Option<Option<NativeOoMethodObservation>> {
            let mut matched = methods.iter().filter(|method| {
                method
                    .name
                    .as_ref()
                    .is_some_and(|name| name_matches(selector, name.as_bytes()))
            });
            let method = matched.next().cloned();
            if matched.next().is_some() {
                return None;
            }
            Some(method)
        };
    let own = selected(&object.object_methods)?;
    if own.as_ref().is_some_and(|method| method.private) {
        return None;
    }
    let method = match own.as_ref() {
        Some(method) if !method.visibility_only => method.clone(),
        _ => selected(&maker.class_methods)??,
    };
    if method.visibility_only || method.intrinsic != Some(intrinsic) || method.private {
        return None;
    }
    let mut public = method.public;
    if let Some(own) = own {
        public = own.public;
    }
    for (exported, unexported) in [
        (&maker.class_exported, &maker.class_unexported),
        (&object.object_exported, &object.object_unexported),
    ] {
        let export = exported
            .iter()
            .any(|name| name_matches(selector, name.as_bytes()));
        let unexport = unexported
            .iter()
            .any(|name| name_matches(selector, name.as_bytes()));
        if export && unexport {
            return None;
        }
        if export {
            public = true;
        }
        if unexport {
            public = false;
        }
    }
    public.then_some(())
}

fn name_matches(selector: &SignatureSourceNameInput, name: &[u8]) -> bool {
    selector
        .policy()
        .recipe()
        .oo_method_input(selector.bytes())
        .ok()
        .zip(selector.policy().recipe().oo_method_input(name).ok())
        .is_some_and(|(selected, named)| selected.selected() == named.selected())
}

fn retain_row(
    rows: &mut Vec<(NativeOoClassObservation, SourceCommandTarget)>,
    row: &NativeOoClassObservation,
    state: &ModuleCommandBindings,
) -> Option<()> {
    if !row.class_mixins.is_empty()
        || !row.object_mixins.is_empty()
        || row
            .class_filters
            .as_ref()
            .is_none_or(|filters| !filters.is_empty())
        || row
            .object_filters
            .as_ref()
            .is_none_or(|filters| !filters.is_empty())
    {
        return None;
    }
    if rows.iter().any(|(known, _)| known.object == row.object) {
        return rows.iter().any(|(known, _)| known == row).then_some(());
    }
    let entry = state.baseline.native_entry.as_ref()?;
    let mut matching = state.objects.iter().filter(|(identity, _)| {
        identity.runtime.is_some_and(|runtime| {
            runtime.interpreter == entry.interpreter && runtime.token == row.command_token
        })
    });
    let (_, implementations) = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let alternatives = implementations.iter().collect::<Vec<_>>();
    let [MayBinding::Target(raw)] = alternatives.as_slice() else {
        return None;
    };
    let target = source_target(raw, state);
    if !raw.terminal
        || raw.kind == BindingKind::Alias
        || !target.prepended.is_empty()
        || target.implementation_generation != 0
        || target.runtime_implementation_generation != Some(row.implementation_generation)
        || !state.retained_target_is_current(&target)
        || state.tainted_object_dispatch.contains(&target.command)
    {
        return None;
    }
    rows.push((row.clone(), target));
    Some(())
}

fn source_target(
    raw: &super::ResolvedCommandTarget,
    state: &ModuleCommandBindings,
) -> SourceCommandTarget {
    SourceCommandTarget {
        runtime_implementation_generation: state
            .runtime_implementation_generation(raw.token.as_ref()),
        command: raw.command.clone(),
        prepended: raw.prepended.clone(),
        original_prepended: None,
        registry_backed: raw.registry_backed,
        kind: raw.kind,
        identity: raw.token.clone(),
        implementation_generation: raw.implementation_generation,
        implementation_allocation: raw.implementation_allocation.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;

    fn state(
        version: TclVersion,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> ModuleCommandBindings {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
        ModuleCommandBindings::initial_with_options(
            registry,
            super::super::SourceAnalysisOptions {
                native_entry: Some(entry),
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
            Some(tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar)),
        )
    }

    fn selector(
        version: TclVersion,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> SignatureSourceNameInput {
        let source = tcl_lexer::SourceImage::document("create");
        let config = tcl_lexer::LexerConfig::from_grammar(
            tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
        );
        let words =
            tcl_lexer::native_script_words_in(source, tcl_lexer::Span::new(0, 6), config).unwrap();
        let [command] = words.commands.as_slice() else {
            panic!("one original command")
        };
        let [word] = command.words.as_slice() else {
            panic!("one original word")
        };
        SignatureSourceNameInput::OriginalWord(
            crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                word,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                entry.command_name_policy().unwrap(),
            )
            .unwrap(),
        )
    }

    fn capture(
        version: TclVersion,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        command: &str,
    ) -> Option<(NativeClassFactoryState, Option<SourceCommandTarget>)> {
        let state = state(version, entry);
        let root = state.source_root_namespace_key()?;
        let policy = entry.command_name_policy()?;
        let raw = state.original_registry_metadata_target(&root, command, policy)?;
        let target = source_target(&raw, &state);
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
        let recipe = registry.native_class_factory_recipe(
            target.registry_identity()?,
            tcl_registry::InvocationDialect::for_version(version),
            tcl_dialect::model::InvocationRealm::default(),
        )?;
        NativeClassFactoryState::capture(&target, &selector(version, entry), recipe, &state)
    }

    #[test]
    fn original_native_class_factory_roles_require_actual_same_entry_allocations() {
        // Implementation contract: naming.tcloo.original-native-factory-role-selection
        // docs/design/analysis/name-resolution-proofs/tcloo-original-native-factory-role-selection.md
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let profile =
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
            let (owner, entry) =
                crate::environment_ingress::captured_native_entry_with_owner(profile);
            let (retained, support) = capture(version, &entry, "::oo::class")
                .expect("actual backend ordinary factory role and intrinsic Create");
            assert!(support.is_none(), "{version:?}");
            assert!(retained.is_current(&state(version, &entry)), "{version:?}");
            let configurable = capture(version, &entry, "::oo::configurable");
            assert_eq!(
                configurable.is_some(),
                version >= TclVersion::V9_0,
                "{version:?}"
            );
            if let Some((retained, support)) = configurable {
                let support = support.expect("actual independently allocated support class");
                assert!(support.identity.as_ref().unwrap().runtime.is_some());
                assert!(retained.is_current(&state(version, &entry)));
                assert!(retained.rows.iter().any(|(row, _)| {
                    row.bootstrap_role == Some(Role::ConfigurableSupport)
                        && matches!(row.constructor, Lifecycle::Absent)
                        && matches!(row.destructor, Lifecycle::Absent)
                }));
            }
            let mut missing = entry.clone();
            missing.oo_classes = None;
            assert!(capture(version, &missing, "::oo::class").is_none());
            drop(owner);
        }
    }

    #[test]
    fn original_native_class_factory_roles_decline_changed_dispatch_and_unknown_topology() {
        // Implementation contract: naming.tcloo.original-native-factory-role-selection
        // docs/design/analysis/name-resolution-proofs/tcloo-original-native-factory-role-selection.md
        let version = TclVersion::V9_0;
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let (owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let (retained, _) =
            capture(version, &entry, "::oo::class").expect("actual unchanged factory");
        let index = entry
            .oo_classes
            .as_ref()
            .unwrap()
            .classes
            .iter()
            .position(|row| row.bootstrap_role == Some(Role::ClassFactory))
            .unwrap();
        for change in 0..6 {
            let mut changed = entry.clone();
            let inventory = changed.oo_classes.as_mut().unwrap();
            let row = &mut inventory.classes[index];
            match change {
                0 => row.bootstrap_role = None,
                1 => row.constructor = Lifecycle::Absent,
                2 => row.class_filters = None,
                3 => row.object_mixins.push(row.object),
                4 => {
                    let create = row
                        .class_methods
                        .iter_mut()
                        .find(|method| {
                            method
                                .name
                                .as_ref()
                                .is_some_and(|name| name.as_bytes() == b"create")
                        })
                        .expect("actual intrinsic Create allocation");
                    create.intrinsic = None;
                }
                5 => row
                    .object_unexported
                    .push(tcl_core_types::NameBytes::from(b"create".as_slice())),
                _ => unreachable!(),
            }
            assert!(
                capture(version, &changed, "::oo::class").is_none(),
                "change={change}"
            );
        }
        let mut duplicate = entry.clone();
        let inventory = duplicate.oo_classes.as_mut().unwrap();
        inventory.classes.push(inventory.classes[index].clone());
        assert!(capture(version, &duplicate, "::oo::class").is_none());
        let mut changed = entry.clone();
        changed.oo_classes.as_mut().unwrap().classes[index].dispatch_epoch += 1;
        assert!(!retained.is_current(&state(version, &changed)));
        let mut changed = state(version, &entry);
        std::sync::Arc::make_mut(&mut changed.tainted_object_dispatch).insert("*".to_owned());
        assert!(!retained.is_current(&changed));
        let mut missing_support = entry.clone();
        let support = missing_support
            .oo_classes
            .as_mut()
            .unwrap()
            .classes
            .iter_mut()
            .find(|row| row.bootstrap_role == Some(Role::ConfigurableSupport))
            .unwrap();
        support.class_definition_namespace = None;
        assert!(capture(version, &missing_support, "::oo::configurable").is_none());
        drop(owner);
    }
}
