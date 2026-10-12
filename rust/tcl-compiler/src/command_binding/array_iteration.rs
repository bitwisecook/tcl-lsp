// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual array lookup precedes caller-frame iterator bindings and body entry.

use super::{
    ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceLoopOperands,
    SourceNativeInvocation, SourceOutcomes,
};
use tcl_registry::array_iteration::ArrayIterationSelection;

impl SourceCommandBindings {
    pub(super) fn walk_array_entries_store(
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        use crate::array_destruction::ArrayEntriesStoreOutcome;
        use tcl_registry::completion::CompletionCode;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let tcl_registry::types::VarElementsEffect::SetsArrayElementsFromList { values_at } =
            facts.var_elements_effect?
        else {
            return None;
        };
        let arguments = native.invocation.arguments();
        let dialect = arguments.dialect()?;
        let values = arguments.literal_at(facts.argument_offset + usize::from(values_at))?;
        let [(name_at, tcl_registry::ArgRole::VarWrite)] = facts.arg_roles.as_slice() else {
            return None;
        };
        let name = arguments.literal_at(facts.argument_offset + usize::from(*name_at))?;
        let rules = tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
        let error = Route::Tcl(CompletionCode::Error);
        let values = match rules.split_list(values) {
            Ok(values) if values.len() % 2 == 0 => values,
            _ => return Some(SourceOutcomes::invocation(state, error)),
        };
        let entries = values
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| (pair[0].to_string(), pair[1].to_string()))
            .collect::<Vec<_>>();
        let root = crate::var_resolve::resolve_literal_access(
            name,
            &state.source_variables,
            true,
            context.registry,
            tcl_registry::TraceOperation::Write,
        );
        match super::Arc::make_mut(&mut state.source_variables).publish_array_entries(
            &root,
            &entries,
            native.segment.span.start(),
            context.registry,
        ) {
            ArrayEntriesStoreOutcome::Stored => Some(SourceOutcomes::normal(state)),
            ArrayEntriesStoreOutcome::Error => Some(SourceOutcomes::invocation(state, error)),
            ArrayEntriesStoreOutcome::Unknown => None,
        }
    }

    pub(super) fn walk_array_iteration(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use crate::array_destruction::ArrayContentsInventory;
        use tcl_registry::completion::CompletionCode;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let error = Route::Tcl(CompletionCode::Error);
        let plan = match tcl_registry::array_iteration::select(
            native.invocation.arguments(),
            facts.argument_offset,
        ) {
            ArrayIterationSelection::Valid(plan) => plan,
            ArrayIterationSelection::Invalid => return SourceOutcomes::invocation(state, error),
            ArrayIterationSelection::Unknown => return super::opaque_source_invocation(state),
        };
        let root = crate::var_resolve::resolve_literal_access(
            &plan.array_name,
            &state.source_variables,
            true,
            context.registry,
            tcl_registry::TraceOperation::Read,
        );
        let inventory = state.source_variables.array_contents_inventory(&root);
        match &inventory {
            ArrayContentsInventory::Error => return SourceOutcomes::invocation(state, error),
            ArrayContentsInventory::Known(members) if members.is_empty() => {
                return SourceOutcomes::normal(state);
            }
            _ => {}
        }
        let mut outcomes = SourceOutcomes::default();
        if let ArrayContentsInventory::Known(members) = &inventory
            && members.iter().any(|member| {
                let observers = state.source_variables.variable_observers_at(
                    member,
                    tcl_registry::TraceOperation::Read,
                    context.registry,
                );
                observers.unknown_residual
                    || !observers.callbacks.is_empty()
                    || !observers.possible_callbacks.is_empty()
            })
        {
            // A member read is a distinct native phase between iterator stores.
            // Until its selected callback sequence is closed, retain that phase's
            // world residual rather than treating enumeration as callback-free.
            state.mark_opaque_binding_mutation();
            outcomes.add_abrupt(Route::Unknown, state);
        }
        if matches!(inventory, ArrayContentsInventory::Unknown) {
            if root.observed || root.dynamic {
                state.mark_opaque_binding_mutation();
                outcomes.add_abrupt(Route::Unknown, state);
            } else {
                outcomes.add_abrupt(error, state);
            }
        }
        // The shared loop store owner resolves each destination sequentially,
        // preserving partial writes and observer uncertainty before body entry.
        // No member order or literal value is inferred from the key inventory.
        let entered = self.walk_source_loop(
            SourceLoopOperands {
                finite: None,
                entry: if matches!(inventory, ArrayContentsInventory::Known(_)) {
                    tcl_registry::iteration_entry::IterationEntry::Required
                } else {
                    tcl_registry::iteration_entry::IterationEntry::Unknown
                },
                variable_lists: &[plan.bindings_at],
                initial: &[],
                conditions: &[],
                repeated: &[plan.body_at],
                continued: &[],
            },
            native.script_operands(),
            state,
            &SourceExecutionContext {
                compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    loop_depth: context.compilation.loop_depth.saturating_add(1),
                    ..context.compilation
                },
                depth: context.depth + 1,
                ..context
            },
        );
        outcomes.join(&entered);
        outcomes.publish(state);
        outcomes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_lookup_controls_body_entry_before_iterator_stores() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        for (prefix, entered) in [
            ("", false),
            ("set a SCALAR; ", false),
            ("array set a {}; ", false),
            ("array set a {one VALUE}; ", true),
        ] {
            let source = format!("{prefix}array for {{k v}} a {{::set body YES}}");
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.find("::set body").unwrap()).unwrap();
            assert_eq!(
                bindings
                    .invocation_at_source("::set", offset)
                    .runtime_reachability()
                    == super::super::SourceRuntimeReachability::Reached,
                entered,
                "{source}",
            );
        }
    }
}
