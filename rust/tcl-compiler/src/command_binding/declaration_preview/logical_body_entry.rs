// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compiler visits of exact installed authored procedure recipes.

use super::DeclaredProcedureBody;
use crate::command_binding::{
    AllocationIncarnation, Arc, BindingKind, CommandAllocationSite, MayBinding,
    ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext,
};

pub(in crate::command_binding) struct OriginalLogicalBodyEntry {
    body: Arc<DeclaredProcedureBody>,
    target: crate::command_binding::ResolvedCommandTarget,
    input: crate::analyser::ResolvedAnalysisInput,
    realm: tcl_dialect::model::InvocationRealm,
}

impl OriginalLogicalBodyEntry {
    fn capture(
        collector: &SourceCommandBindings,
        site: &CommandAllocationSite,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Self> {
        let input = incoming.logical_source_name_advice_input()?;
        if input.lexer_config() != config
            || incoming.baseline.native_entry.is_some()
            || incoming.baseline.unknown_entry
            || incoming.baseline.source_point_entry.is_some()
            || incoming.has_opaque_domain()
            || !incoming.command_observers.is_quiet()
        {
            return None;
        }
        let observations =
            crate::command_binding::declaration_layout::original_declaration_layouts(
                collector.declaration_layouts.get(site)?,
            )?;
        for observation in observations {
            if observation.snapshot.realm != incoming.baseline.invocation_realm
                || observation.config != config
                || observation
                    .snapshot
                    .state
                    .logical_source_name_advice_input()
                    != Some(input)
            {
                return None;
            }
        }
        let body = collector.original_declared_procedure_at(site, registry)?;
        if body.logical_name.is_none()
            || body.original_parameters.is_some()
            || !Arc::ptr_eq(&body.source.origin, &site.source)
            || !matches!(
                body.source.mapping,
                crate::command_binding::ExecutedScriptMapping::Contiguous { .. }
            )
            || !incoming.namespaces.contains(&body.namespace)
        {
            return None;
        }
        let alternatives = incoming.binding_alternatives(&body.slot);
        let mut alternatives = alternatives.iter();
        let MayBinding::Target(target) = alternatives.next()? else {
            return None;
        };
        if alternatives.next().is_some()
            || target.kind != BindingKind::Proc
            || !target.terminal
            || target.registry_backed
            || !target.prepended.is_empty()
            || target.implementation_generation != site.offset
        {
            return None;
        }
        let allocation = target.implementation_allocation.as_ref()?;
        if allocation.incarnation != AllocationIncarnation::First
            || allocation.site.offset != site.offset
            || !Arc::ptr_eq(&allocation.site.source, &site.source)
            || allocation.namespace != body.namespace
            || target.token.is_none()
        {
            return None;
        }
        Some(Self {
            body: Arc::new(body),
            target: target.clone(),
            input: input.clone(),
            realm: incoming.baseline.invocation_realm,
        })
    }
}

impl SourceCommandBindings {
    pub(in crate::command_binding) fn analyse_logical_declared_body_entries(
        &mut self,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self
            .original_completed_root_state
            .as_deref()
            .is_none_or(|world| !world.same_state(incoming))
        {
            return;
        }
        let entries = self
            .declaration_layouts
            .keys()
            .filter_map(|site| {
                OriginalLogicalBodyEntry::capture(self, site, incoming, config, registry)
            })
            .collect::<Vec<_>>();
        for entry in entries {
            self.walk_logical_declared_body_entry(&entry, incoming, config, registry);
        }
    }

    fn walk_logical_declared_body_entry(
        &mut self,
        entry: &OriginalLogicalBodyEntry,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let input = &entry.input;
        let Some(options) =
            crate::command_binding::SourceAnalysisOptions::for_logical_source(input)
        else {
            return;
        };
        let Ok(source) = entry.body.source.try_text() else {
            return;
        };
        let Some(namespace) = entry.body.namespace.display() else {
            return;
        };
        let implementation = crate::command_binding::DeferredImplementationId {
            command: entry
                .target
                .token
                .as_ref()
                .expect("captured actual token")
                .origin
                .clone(),
            generation: entry.target.implementation_generation,
            allocation: entry.target.implementation_allocation.clone(),
        };
        if !self.active_calls.insert(implementation.clone()) {
            return;
        }
        let mut branch = crate::command_binding::boxed_source_branch(incoming);
        branch.current_source_origin = Some(Arc::clone(&entry.body.source.origin));
        branch.variable_frame = entry.body.frame.clone();
        branch.source_variables = Arc::new(
            branch
                .source_variables
                .in_frame(&entry.body.frame)
                .with_namespace_identity(entry.body.namespace.clone()),
        );
        let variables = Arc::make_mut(&mut branch.source_variables);
        variables.widen_deferred_namespace_inputs();
        for parameter in &entry.body.parameters {
            variables.bind_unknown_incoming(&parameter.name, registry);
        }
        let mut context = crate::command_binding::root_source_execution_context(
            &entry.body.frame,
            &namespace,
            &entry.body.namespace,
            config,
            registry,
            options,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        );
        context.control = context.control.with_realm(entry.realm);
        context.original_logical_body = Some(entry);
        // Body completions remain separate from the document continuation.
        let _outcomes = self.walk_source(source, entry.body.source.base(), &mut branch, &context);
        self.active_calls.remove(&implementation);
    }
}

pub(in crate::command_binding) fn current_entry(
    state: &ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
    offset: u32,
) -> Option<Arc<DeclaredProcedureBody>> {
    let entry = context.original_logical_body?;
    let body = &entry.body;
    let input = state.logical_source_name_advice_input()?;
    if input != &entry.input
        || context.realm() != entry.realm
        || state.baseline.invocation_realm != entry.realm
        || context.body_entry() != crate::command_binding::SourceBodyEntryPurpose::Selected
        || state.baseline.native_entry.is_some()
        || input.lexer_config() != context.config
        || state.variable_frame != body.frame
        || context.frame != &body.frame
        || context.namespace_identity() != body.namespace
        || !SourceCommandBindings::context_owns_frame(
            &state.source_variables,
            &body.frame,
            Some(&body.namespace),
        )
    {
        return None;
    }
    let origin = state.current_source_origin.as_ref()?;
    let start = body.source.base();
    let end = start.checked_add(u32::try_from(body.source.text.bytes().len()).ok()?)?;
    (Arc::ptr_eq(origin, &body.source.origin) && start <= offset && offset < end)
        .then(|| Arc::clone(body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, SourceOriginId};

    fn analyse(source: &str) -> SourceCommandBindings {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
                .unwrap()
                .default_context_registry(),
            config,
        );
        SourceCommandBindings::analyse_with_options(
            source,
            config,
            input.borrowed_context_registry().commands(),
            SourceAnalysisOptions::for_logical_source(&input).unwrap(),
        )
    }

    fn body_dispatch_at(bindings: &SourceCommandBindings, offset: usize) -> bool {
        bindings
            .points
            .iter()
            .chain(bindings.origin_points.values().flatten())
            .any(|point| {
                point.dispatch
                    && usize::try_from(point.offset).unwrap() == offset
                    && matches!(
                        point.state.variable_frame.layout(),
                        crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                    )
            })
    }

    #[test]
    fn installed_logical_declaration_gets_a_real_body_visit_without_native_entry() {
        let source = "proc p {} {set anchor 1; puts literal}";
        let bindings = analyse(source);
        assert!(body_dispatch_at(&bindings, source.find("puts").unwrap()));
        let tail = u32::try_from(source.find("puts").unwrap()).unwrap();
        assert!(
            bindings
                .points
                .iter()
                .chain(bindings.origin_points.values().flatten())
                .any(|point| {
                    point.dispatch
                        && point.offset == tail
                        && (point.original_written_words.is_some()
                            || point.compiled_execution.original_words.is_some())
                })
        );
        assert!(
            bindings
                .points
                .iter()
                .chain(bindings.origin_points.values().flatten())
                .all(|point| point.state.baseline.native_entry.is_none())
        );
        assert!(matches!(
            bindings.final_state.variable_frame.layout(),
            crate::var_resolve::VariableExecutionFrame::Global
        ));
    }

    #[test]
    fn replacement_keeps_old_lexical_body_without_old_body_dispatch() {
        let source = "proc p {} {puts OLD}; proc p {} {puts NEW}";
        let bindings = analyse(source);
        assert!(!body_dispatch_at(
            &bindings,
            source.find("puts OLD").unwrap()
        ));
        assert!(body_dispatch_at(
            &bindings,
            source.find("puts NEW").unwrap()
        ));
    }

    #[test]
    fn body_unknown_effects_do_not_change_completed_root_world() {
        let source = "proc p {} {set anchor 1; mystery; puts NEVER}";
        let bindings = analyse(source);
        assert!(body_dispatch_at(&bindings, source.find("mystery").unwrap()));
        let tail = u32::try_from(source.find("puts NEVER").unwrap()).unwrap();
        assert!(
            bindings
                .points
                .iter()
                .chain(bindings.origin_points.values().flatten())
                .filter(|point| point.dispatch && point.offset == tail)
                .all(|point| point.state.has_opaque_domain())
        );
        assert!(
            bindings
                .original_completed_root_state
                .as_deref()
                .unwrap()
                .same_state(&bindings.final_state)
        );
        assert!(!bindings.final_state.has_opaque_domain());
    }

    #[test]
    fn scoped_body_entry_refuses_foreign_origin_and_changed_configuration() {
        let source = "proc p {} {puts literal}";
        let bindings = analyse(source);
        let incoming = &bindings.final_state;
        let input = incoming.logical_source_name_advice_input().unwrap();
        let registry = input.borrowed_context_registry().commands();
        let config = input.lexer_config();
        let site = CommandAllocationSite {
            source: Arc::clone(bindings.source_origin().unwrap()),
            offset: 0,
        };
        let entry = OriginalLogicalBodyEntry::capture(&bindings, &site, incoming, config, registry)
            .unwrap();
        let namespace = entry.body.namespace.display().unwrap();
        let mut state = incoming.as_ref().clone();
        state.variable_frame = entry.body.frame.clone();
        state.source_variables = Arc::new(
            state
                .source_variables
                .in_frame(&entry.body.frame)
                .with_namespace_identity(entry.body.namespace.clone()),
        );
        let mut context = crate::command_binding::root_source_execution_context(
            &entry.body.frame,
            &namespace,
            &entry.body.namespace,
            config,
            registry,
            SourceAnalysisOptions::for_logical_source(input).unwrap(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        );
        context.control = context.control.with_realm(entry.realm);
        context.original_logical_body = Some(&entry);
        assert!(current_entry(&state, &context, entry.body.source.base()).is_some());
        let mut foreign = state.clone();
        foreign.current_source_origin = Some(Arc::new(SourceOriginId::authored_image(
            tcl_lexer::SourceImage::document(source),
        )));
        assert!(current_entry(&foreign, &context, entry.body.source.base()).is_none());
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let other_input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_known_environment("tcl8.5")
                .unwrap()
                .default_context_registry(),
            config,
        );
        assert_eq!(other_input.lexer_config(), entry.input.lexer_config());
        assert_ne!(other_input, entry.input);
        let mut other_state = state.clone();
        Arc::make_mut(&mut other_state.baseline).logical_source_input = Some(other_input);
        assert!(other_state.logical_source_name_advice_input().is_some());
        assert!(current_entry(&other_state, &context, entry.body.source.base()).is_none());
        let mut other_realm = context;
        other_realm.control = other_realm.control.with_realm(
            if entry.realm == tcl_dialect::model::InvocationRealm::RuleLoader {
                tcl_dialect::model::InvocationRealm::InterpreterRuntime
            } else {
                tcl_dialect::model::InvocationRealm::RuleLoader
            },
        );
        assert!(current_entry(&state, &other_realm, entry.body.source.base()).is_none());
        context.config.strict_quoting = !context.config.strict_quoting;
        assert!(current_entry(&state, &context, entry.body.source.base()).is_none());
    }
    #[test]
    fn declaration_realm_must_match_the_completed_root_realm() {
        let source = "proc p {} {puts literal}";
        let bindings = analyse(source);
        let incoming = &bindings.final_state;
        let input = incoming.logical_source_name_advice_input().unwrap();
        let registry = input.borrowed_context_registry().commands();
        let config = input.lexer_config();
        let site = CommandAllocationSite {
            source: Arc::clone(bindings.source_origin().unwrap()),
            offset: 0,
        };
        assert!(
            OriginalLogicalBodyEntry::capture(&bindings, &site, incoming, config, registry)
                .is_some()
        );
        let wrong_realm = if incoming.baseline.invocation_realm
            == tcl_dialect::model::InvocationRealm::RuleLoader
        {
            tcl_dialect::model::InvocationRealm::InterpreterRuntime
        } else {
            tcl_dialect::model::InvocationRealm::RuleLoader
        };
        let mut changed = bindings.clone();
        for row in changed.declaration_layouts.get_mut(&site).unwrap() {
            row.snapshot = Arc::new(crate::command_binding::SourceLookupSnapshot::in_realm(
                row.snapshot.state.clone(),
                wrong_realm,
            ));
            assert_eq!(row.config, config);
            assert_eq!(
                row.snapshot.state.logical_source_name_advice_input(),
                Some(input)
            );
        }
        assert!(
            OriginalLogicalBodyEntry::capture(&changed, &site, incoming, config, registry)
                .is_none()
        );
    }
}
