// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Execute captured array teardown phases without choosing a portable member order.

use super::{
    Arc, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceOutcomes,
};
use crate::array_destruction::ArrayDestruction;
use tcl_registry::TraceOperation;

impl SourceCommandBindings {
    pub(super) fn walk_staged_array_destruction(
        &mut self,
        site: u32,
        name: &str,
        plan: &ArrayDestruction,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut outcomes = self.walk_selected_variable_observers(
            (site, &plan.original, Some(name), TraceOperation::Unset),
            &plan.root_callbacks,
            state,
            context,
        );
        let Some(after_root) = outcomes.normal.take() else {
            finish_all(&mut outcomes, plan);
            outcomes.publish(state);
            return outcomes;
        };
        for order in member_orders(plan.members.len()) {
            *state = (*after_root).clone();
            let mut branch = SourceOutcomes::normal(state);
            for index in order {
                let Some(continuing) = branch.normal.take() else {
                    break;
                };
                *state = *continuing;
                let member = &plan.members[index];
                // Earlier root/member callbacks may mutate this old member's
                // registrations through an existing direct alias. Capture the
                // list at this member's retirement, using its original receipt.
                let callbacks = state.source_variables.variable_observers_on_cell(
                    &member.receiver,
                    TraceOperation::Unset,
                    context.registry,
                );
                Arc::make_mut(&mut state.source_variables).retire_array_member(
                    &member.receiver,
                    site,
                    context.registry,
                );
                let delivered = self.walk_selected_variable_observers(
                    (site, &member.receiver, Some(name), TraceOperation::Unset),
                    &callbacks,
                    state,
                    context,
                );
                branch.join(&delivered);
            }
            finish_all(&mut branch, plan);
            outcomes.join(&branch);
        }
        finish_all(&mut outcomes, plan);
        outcomes.publish(state);
        outcomes
    }
}

fn finish_all(outcomes: &mut SourceOutcomes, plan: &ArrayDestruction) {
    if let Some(normal) = &mut outcomes.normal {
        Arc::make_mut(&mut normal.source_variables).finish_array_destruction(&plan.retained);
    }
    for (_, state) in &mut outcomes.abrupt {
        Arc::make_mut(&mut state.source_variables).finish_array_destruction(&plan.retained);
    }
}

/// The physical owner bounded the complete inventory to four members. Enumerate
/// every order rather than letting map order become a native execution proof.
fn member_orders(count: usize) -> Vec<Vec<usize>> {
    fn extend(count: usize, prefix: &mut Vec<usize>, orders: &mut Vec<Vec<usize>>) {
        if prefix.len() == count {
            orders.push(prefix.clone());
            return;
        }
        for index in 0..count {
            if !prefix.contains(&index) {
                prefix.push(index);
                extend(count, prefix, orders);
                prefix.pop();
            }
        }
    }
    let mut orders = Vec::new();
    extend(count, &mut Vec::new(), &mut orders);
    orders
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::SourceAnalysisOptions;

    struct OriginalWorker {
        identity: String,
        token: crate::command_binding::RuntimeCommandTokenIdentity,
        generation: u64,
    }

    fn assert_original_worker(
        binding: &crate::command_binding::SourceInvocationBinding,
        expected: &OriginalWorker,
    ) {
        let target = binding
            .proved_handler_target()
            .expect("the original native worker remains proved after its callback rename");
        assert!(target.registry_backed);
        assert_eq!(target.kind, crate::command_binding::BindingKind::Builtin);
        assert_eq!(target.command, expected.identity);
        assert_eq!(
            target
                .identity
                .as_ref()
                .and_then(|identity| identity.runtime),
            Some(expected.token)
        );
        assert_eq!(
            target.runtime_implementation_generation,
            Some(expected.generation)
        );
    }

    fn analyse(source: &str) -> (SourceCommandBindings, OriginalWorker) {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let worker = entry
            .commands
            .iter()
            .find(|command| {
                command.namespace_token == entry.current_namespace
                    && command.slot.simple.as_bytes() == b"llength"
            })
            .expect("independently captured original root llength binding");
        let tcl_runtime_api::native_compilation::NativeCommandImplementation::Registry {
            identity,
            ..
        } = &worker.implementation
        else {
            panic!("the original llength binding is not a native registry worker");
        };
        let original_worker = OriginalWorker {
            identity: identity.clone(),
            token: crate::command_binding::RuntimeCommandTokenIdentity {
                interpreter: entry.interpreter,
                token: worker.token,
            },
            generation: worker.implementation_generation,
        };
        let namespace = entry
            .retained_namespace_context(entry.current_namespace)
            .unwrap();
        let tables = entry
            .namespace_variable_tables
            .as_ref()
            .expect("independently captured actual native variable tables");
        let table = tables
            .iter()
            .find(|table| table.namespace == namespace)
            .expect("actual original root namespace table");
        assert!(
            !table.roots.iter().any(|name| name.as_bytes() == b"a"),
            "the original array root is absent before its source store"
        );
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                native_entry: Some(&entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let unset = u32::try_from(source.find("unset a;").unwrap()).unwrap();
        let before = bindings.invocation_at_source("unset", unset);
        let root = crate::var_resolve::resolve_literal_place(
            "a",
            &before.variable_context,
            false,
            &registry,
        );
        let root = crate::var_resolve::physical_array_key(&root)
            .expect("same original namespace array slot");
        assert!(
            before.variable_context.closed_array_roots.contains(&root),
            "original successful literal stores preserve the fresh array inventory"
        );
        (bindings, original_worker)
    }

    #[test]
    fn root_unset_callback_reads_the_retained_old_element_alias() {
        let source = "set a(x) X; upvar 0 a(x) ex; proc root args {set ::seen $::ex}; trace add variable a unset root; unset a; llength stable";
        let (bindings, _) = analyse(source);
        let final_call = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        assert_eq!(
            final_call.variable_context.literal_value(
                "::seen",
                tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            ),
            Some("X")
        );
        assert!(final_call.proved_handler_target().is_some());
    }

    #[test]
    fn captured_member_callback_survives_root_recreation_and_new_registration() {
        let source = "set a(x) X; proc old args {rename llength saved}; proc new args {}; proc root args {set ::a(x) NEW; trace add variable ::a(x) unset new}; trace add variable a(x) unset old; trace add variable a unset root; unset a; saved stable";
        let (bindings, original_worker) = analyse(source);
        let final_call = bindings.invocation_at_source(
            "saved",
            u32::try_from(source.rfind("saved").unwrap()).unwrap(),
        );
        assert_original_worker(&final_call, &original_worker);
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        assert_eq!(
            final_call.variable_context.literal_value("a(x)", registry),
            Some("NEW")
        );
    }

    #[test]
    fn member_order_inventory_contains_every_order_and_no_duplicates() {
        let orders = member_orders(4);
        assert_eq!(orders.len(), 24);
        let unique = orders.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(unique.len(), 24);
        for mut order in orders {
            order.sort_unstable();
            assert_eq!(order, vec![0, 1, 2, 3]);
        }
    }

    #[test]
    fn old_element_alias_can_change_a_later_member_callback_before_retirement() {
        let source = "set a(x) X; upvar 0 a(x) ex; proc old args {rename llength lost}; proc fresh args {rename llength saved}; proc root args {trace remove variable ::ex unset old; trace add variable ::ex unset fresh}; trace add variable a(x) unset old; trace add variable a unset root; unset a; saved stable";
        let (bindings, original_worker) = analyse(source);
        let final_call = bindings.invocation_at_source(
            "saved",
            u32::try_from(source.rfind("saved").unwrap()).unwrap(),
        );
        if final_call.proved_handler_target().is_none() {
            for (offset, _) in source.match_indices("trace ") {
                let point = bindings.invocation_at_source("trace", u32::try_from(offset).unwrap());
                eprintln!(
                    "trace@{offset}: target={:?}, registrations={:?}, receivers={:?}, ex={:?}",
                    point.proved_handler_target().map(|target| &target.command),
                    point.variable_context.trace_registrations,
                    point.variable_context.trace_registration_receivers,
                    crate::var_resolve::resolve_literal_place(
                        "::ex",
                        &point.variable_context,
                        false,
                        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
                    ),
                );
            }
        }
        assert_original_worker(&final_call, &original_worker);
    }
}
