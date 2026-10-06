// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional provider-defined dispatcher installation, without object proofs.

use super::{
    Arc, BindingKind, ClassDefinitionReceipt, MayBinding, ModuleCommandBindings,
    ResolvedCommandTarget, SourceCommandBindings, SourceCommandTarget, SourceExecutionContext,
    SourceInvocationBinding, SourceNativeInvocation, SourceOutcomes, qualify_execution_name,
    retained_script_operand, source_effective_words,
};
use tcl_registry::definer::{DefinitionBodyGrammar, DefinitionDispatcher};

impl SourceCommandBindings {
    /// A bounded declaration can install its actual provider dispatcher. Its
    /// constructor and method bodies remain deferred, independently opaque.
    #[inline(never)]
    pub(super) fn walk_snit_definition(
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        let query = state
            .baseline
            .dialect?
            .authoring_query()?
            .with_realm(context.realm);
        let spec = context
            .registry
            .get_for_surface(&native.target.command, Some(query))?;
        let grammar = spec.definition_body?;
        let installation = grammar.command_installation()?;
        let loader = state.loaded_provider(spec.owning_package()?)?;
        if !definition_installation_is_live(
            state,
            loader,
            &native.target.command,
            installation.dispatcher,
        ) {
            return None;
        }
        let arguments = native.invocation.arguments();
        let name = arguments.literal_at(usize::from(installation.name_at))?;
        if installation.dispatcher == DefinitionDispatcher::ItclClass
            && (name.is_empty() || name.contains('.') || name.ends_with(':'))
        {
            return None;
        }
        let namespace = if state.source_variables.namespace_known {
            crate::ir::ExecutionNamespace::exact(context.namespace)
        } else {
            crate::ir::ExecutionNamespace::RuntimeSelected
        };
        let name = qualify_execution_name(&namespace, name)?;
        if name.is_empty()
            || !state.definitely_absent(&name, "::")
            || state.namespaces.contains(&name)
        {
            return None;
        }
        if installation.dispatcher == DefinitionDispatcher::ItclClass
            && !state
                .namespaces
                .contains(tcl_syntax::naming::key_holder_and_tail(&name).0)
        {
            return None;
        }
        let definition = retained_script_operand(
            usize::from(installation.body_at),
            native.script_operands(),
            state,
            native.segment.span.start(),
            context.config,
        )?;
        let dispatcher_methods =
            deferred_definition_methods(grammar, definition.try_text().ok()?, state, context)?;
        install_snit_dispatcher(
            &name,
            &native.target.command,
            native.segment.span.start(),
            installation.dispatcher,
            dispatcher_methods,
            state,
        )
    }
}

fn definition_installation_is_live(
    state: &ModuleCommandBindings,
    loader: &super::TrustedPackageLoader,
    factory: &str,
    dispatcher: DefinitionDispatcher,
) -> bool {
    loader
        .definition_dispatchers
        .iter()
        .any(|(name, recipe)| super::nqn(name) == factory && *recipe == dispatcher)
        && !loader.lookup_dependencies.is_empty()
        && state.provider_surface_is_live(loader)
        && !state.has_opaque_domain()
        && !state.source_variables.dynamic_traces
        && state.source_variables.traced.is_empty()
        && state
            .source_variables
            .possible_trace_registrations
            .is_empty()
        && provider_definition_observers_closed(state, loader)
        && state
            .baseline
            .dialect
            .is_some_and(|dialect| dispatcher.native_installation_is_audited(dialect))
}

fn provider_definition_observers_closed(
    state: &ModuleCommandBindings,
    loader: &super::TrustedPackageLoader,
) -> bool {
    !state.source_step_observed()
        && loader
            .lookup_dependencies
            .iter()
            .chain(&loader.installed_lookup_dependencies)
            .all(|dependency| {
                let binding = super::source_binding(state, &dependency.head, &dependency.namespace);
                !state.source_execution_observed(
                    binding
                        .proved_target()
                        .and_then(|target| target.identity.as_ref()),
                )
            })
}

fn install_snit_dispatcher(
    name: &str,
    factory: &str,
    generation: u32,
    dispatcher: DefinitionDispatcher,
    dispatcher_methods: super::BTreeSet<String>,
    state: &mut ModuleCommandBindings,
) -> Option<SourceOutcomes> {
    state.install(
        name.to_owned(),
        MayBinding::Target(ResolvedCommandTarget {
            command: name.to_owned(),
            prepended: Vec::new(),
            registry_backed: false,
            kind: BindingKind::Command,
            implementation_generation: generation,
            implementation_allocation: None,
            terminal: true,
            target_lookup: tcl_registry::AliasTargetLookup::Global,
            token: None,
        }),
    );
    Arc::make_mut(&mut state.namespaces).insert(name.to_owned());
    if dispatcher == DefinitionDispatcher::ItclClass {
        for method in &dispatcher_methods {
            state.install(
                format!("{name}::{method}"),
                MayBinding::Target(ResolvedCommandTarget {
                    command: format!("{name}::{method}"),
                    prepended: Vec::new(),
                    registry_backed: false,
                    kind: BindingKind::Command,
                    implementation_generation: generation,
                    implementation_allocation: None,
                    terminal: true,
                    target_lookup: tcl_registry::AliasTargetLookup::Global,
                    token: None,
                }),
            );
        }
    }
    // Installation owns this fresh slot. Execution proof is unavailable
    // until its compiler-presence receipt has been installed below.
    let installed = state.bindings.get(name)?;
    if installed.len() != 1 {
        return None;
    }
    let MayBinding::Target(target) = installed.first()? else {
        return None;
    };
    Arc::make_mut(&mut state.class_definitions).insert(
        target.token.clone()?,
        ClassDefinitionReceipt {
            factory: factory.to_owned(),
            implementation_generation: generation,
            dispatcher: Some(dispatcher),
            dispatcher_methods: Arc::new(dispatcher_methods),
            instance_methods: None,
            instance_variables: None,
            constructor_entry: None,
            destructor_entry: None,
            lifecycle_entries_closed: false,
            receiver_method_entries: Arc::default(),
            inherited_classes: Arc::default(),
        },
    );
    Some(SourceOutcomes::normal(state))
}

fn deferred_definition_methods(
    grammar: &DefinitionBodyGrammar,
    source: &str,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<super::BTreeSet<String>> {
    let parameters = state
        .baseline
        .dialect
        .and_then(tcl_registry::InvocationDialect::parameter_grammar)?;
    let mut methods = super::BTreeSet::new();
    let mut declarations = super::BTreeSet::new();
    let map = tcl_lexer::SourceMap::new(source);
    let valid =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, context.config)
            .into_iter()
            .all(|segment| {
                let tokens =
                    crate::ir::CommandTokens::from_segmented(&map, context.config, &segment);
                let values = source_effective_words(tokens.words(), state.baseline.dialect, None);
                let Some(member) = values
                    .first()
                    .and_then(|word| word.as_registry_word().literal())
                    .and_then(|head| grammar.member(head))
                else {
                    return false;
                };
                if !grammar.installation_member_is_deferred(member) {
                    return false;
                }
                let expected = member
                    .arg_roles
                    .iter()
                    .map(|(index, _)| usize::from(*index) + 2)
                    .max()
                    .unwrap_or(1);
                let valid = values.len() == expected
                    && values
                        .iter()
                        .all(|word| word.as_registry_word().literal().is_some())
                    && member.arg_roles.iter().all(|(index, role)| {
                        *role != tcl_registry::ArgRole::ParamList
                            || values
                                .get(usize::from(*index) + 1)
                                .and_then(|word| word.as_registry_word().literal())
                                .is_some_and(|text| {
                                    tcl_syntax::formal_params::parse_formal_parameters_in(
                                        text, parameters,
                                    )
                                    .is_ok()
                                })
                    });
                if !valid {
                    return false;
                }
                if !bounded_installation_member_name(grammar, member, &values, &mut declarations) {
                    return false;
                }
                if let Some(index) = grammar.installation_dispatch_method_name_at(member) {
                    let Some(text) = values
                        .get(usize::from(index) + 1)
                        .and_then(|word| word.as_registry_word().literal())
                    else {
                        return false;
                    };
                    let Ok(names) = tcl_syntax::list::split_list(text) else {
                        return false;
                    };
                    let Some(first) = names.first() else {
                        return false;
                    };
                    methods.insert(first.to_string());
                }
                true
            });
    valid.then_some(methods)
}

fn bounded_installation_member_name(
    grammar: &DefinitionBodyGrammar,
    member: &tcl_registry::definer::MemberSpec,
    values: &[crate::registry_invocation::EffectiveInvocationWord],
    declarations: &mut super::BTreeSet<String>,
) -> bool {
    if grammar.family != tcl_registry::definer::DefinerFamily::Itcl {
        return true;
    }
    let name = if matches!(member.keyword, "method" | "proc") {
        values
            .get(1)
            .and_then(|word| word.as_registry_word().literal())
            .unwrap_or("")
    } else {
        member.keyword
    };
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && declarations.insert(name.to_owned())
}

impl ModuleCommandBindings {
    pub(super) fn definition_dispatcher_receipt_is_live(
        &self,
        receipt: &ClassDefinitionReceipt,
    ) -> bool {
        self.baseline.trusted_loaders.values().any(|loader| {
            self.loaded_provider(&loader.package)
                .is_some_and(|selected| {
                    selected
                        .definition_dispatchers
                        .iter()
                        .any(|(factory, dispatcher)| {
                            super::nqn(factory) == receipt.factory
                                && Some(*dispatcher) == receipt.dispatcher
                        })
                        && self.provider_surface_is_live(selected)
                        && provider_definition_observers_closed(self, selected)
                })
        })
    }

    pub(super) fn installed_definition_dispatcher(
        &self,
        target: &SourceCommandTarget,
    ) -> Option<DefinitionDispatcher> {
        if self.has_opaque_domain() {
            return None;
        }
        let receipt = self.class_definitions.get(target.identity.as_ref()?)?;
        (receipt.implementation_generation == target.implementation_generation
            && self.definition_dispatcher_receipt_is_live(receipt))
        .then_some(receipt.dispatcher)
        .flatten()
    }
}

impl SourceInvocationBinding {
    /// Conditional returned-name protocol of an actual installed dispatcher.
    /// This is an advisory callable-name fact on normal completion. It proves
    /// no constructor closure, current object existence, physical class,
    /// method availability, native opcode or executable specialization.
    #[must_use]
    pub fn nominal_definition_name_result(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<DefinitionDispatcher> {
        let target = self.proved_target()?;
        let snapshot = &self.lookup_state.as_ref()?.state;
        if snapshot.source_execution_observed(target.identity.as_ref()) {
            return None;
        }
        let dispatcher = snapshot.installed_definition_dispatcher(target)?;
        let receipt = snapshot.class_definitions.get(target.identity.as_ref()?)?;
        let grammar = registry
            .get_for_surface(
                &receipt.factory,
                snapshot.baseline.dialect?.authoring_query(),
            )?
            .definition_body?;
        let effective = target
            .prepended
            .iter()
            .chain(self.evaluated_argument_words.iter())
            .collect::<Vec<_>>();
        let first = effective.first()?.as_registry_word().literal()?;
        let name_at = grammar.conditional_construction_name_at(
            first,
            receipt.dispatcher_methods.iter().map(String::as_str),
        )?;
        let name = effective.get(name_at)?.as_registry_word().literal()?;
        (!name.is_empty()).then_some(dispatcher)
    }
}
