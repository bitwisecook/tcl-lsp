// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native iterator entry precedes the abstract repetition join.

use super::{
    Arc, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceLoopOperands,
    SourceOutcomes, SourceScriptOperands, select_contents_write_source, source_operand_truth,
};
use tcl_registry::{
    completion::CompletionCode as Code, completion_route::InvocationCompletionRoute as Route,
    iteration_entry::IterationEntry,
};

impl SourceCommandBindings {
    pub(super) fn walk_source_loop(
        &mut self,
        operands: SourceLoopOperands<'_>,
        scripts: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut outcomes = Box::<SourceOutcomes>::default();
        self.prepare_source_loop(operands, scripts, state, context, &mut outcomes);
        let Some(mut head) = outcomes.normal.take() else {
            return finish_source_loop(&mut outcomes, state);
        };
        if let Some(finite) = operands.finite {
            self.walk_finite_source_iterations(
                finite,
                operands,
                scripts,
                &mut head,
                context,
                &mut outcomes,
            );
            return finish_source_loop(&mut outcomes, state);
        }
        match operands.entry {
            IterationEntry::Empty => join_source_loop_exit(&mut outcomes, &head),
            IterationEntry::Invalid => outcomes.add_abrupt(Route::Tcl(Code::Error), &head),
            IterationEntry::Required | IterationEntry::Unknown => {
                let mut required = operands.entry == IterationEntry::Required;
                let mut initial = true;
                loop {
                    let before = head.clone();
                    let Some((mut iteration, truth)) = self.prepare_source_loop_iteration(
                        operands,
                        scripts,
                        &mut head,
                        context,
                        &mut outcomes,
                    ) else {
                        break;
                    };
                    if !required && truth != Some(true) {
                        join_source_loop_exit(&mut outcomes, &iteration);
                    }
                    if truth == Some(false) {
                        break;
                    }
                    let next = self.walk_source_loop_iteration(
                        operands,
                        scripts,
                        &mut iteration,
                        context,
                        &mut outcomes,
                    );
                    if required || (initial && truth == Some(true)) {
                        let Some(next) = next else { break };
                        // Only a completed first iteration can seed repetition.
                        // Joining the initial state here would invent a skip path.
                        head = next;
                        // The mandatory iteration has already executed. If
                        // its complete successor equals its input, replaying
                        // that same abstract body cannot add a new state. A
                        // finite native iterator can terminate here; a proved
                        // true while/for condition still supplies no exit.
                        if head.same_state(&before) {
                            if required {
                                join_source_loop_exit(&mut outcomes, &head);
                            }
                            break;
                        }
                        required = false;
                        initial = false;
                    } else {
                        initial = false;
                        if let Some(next) = next {
                            head.join(&next);
                        }
                        if head.same_state(&before) {
                            break;
                        }
                    }
                }
            }
        }
        finish_source_loop(&mut outcomes, state)
    }

    fn walk_finite_source_iterations(
        &mut self,
        finite: &tcl_registry::iteration_entry::FiniteIteration,
        operands: SourceLoopOperands<'_>,
        scripts: SourceScriptOperands<'_>,
        head: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) {
        use crate::variable_bindings::SourceIterationBindingOutcome;
        for iteration in 0..finite.iterations() {
            let Some(values) = finite.bindings(iteration) else {
                head.mark_opaque_binding_mutation();
                outcomes.add_abrupt(Route::Unknown, head);
                return;
            };
            select_contents_write_source(head);
            match crate::variable_bindings::transfer_finite_iteration_bindings(
                Arc::make_mut(&mut head.source_variables),
                &values,
                context.registry,
                context.invocation_offset,
            ) {
                SourceIterationBindingOutcome::Entered => {}
                SourceIterationBindingOutcome::StoreError => {
                    outcomes.add_abrupt(Route::Tcl(Code::Error), head);
                    return;
                }
                SourceIterationBindingOutcome::Unknown => {
                    head.mark_opaque_binding_mutation();
                    outcomes.add_abrupt(Route::Unknown, head);
                }
            }
            let executed = self.walk_body_sequence(operands.repeated, scripts, head, context);
            let Some(next) =
                self.settle_source_loop_iteration(executed, operands, scripts, context, outcomes)
            else {
                return;
            };
            *head = *next;
        }
        join_source_loop_exit(outcomes, head);
    }

    /// Keep initial and completed outcome construction outside the frame that
    /// remains live while a repeated body descends into another source loop.
    #[inline(never)]
    fn prepare_source_loop(
        &mut self,
        operands: SourceLoopOperands<'_>,
        scripts: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) {
        *outcomes = self.walk_body_sequence(operands.initial, scripts, state, context);
    }

    /// Finish condition evaluation before retaining the repeated-body frame.
    #[inline(never)]
    fn prepare_source_loop_iteration(
        &mut self,
        operands: SourceLoopOperands<'_>,
        scripts: SourceScriptOperands<'_>,
        head: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) -> Option<(Box<ModuleCommandBindings>, Option<bool>)> {
        let evaluated = self.walk_expression_sequence(operands.conditions, scripts, head, *context);
        for (route, state) in &evaluated.abrupt {
            outcomes.add_abrupt(*route, state);
        }
        let iteration = evaluated.normal?;
        let truth = operands.conditions.last().and_then(|condition| {
            source_operand_truth(*condition, scripts, &iteration, context.registry)
        });
        Some((iteration, truth))
    }

    fn walk_source_loop_iteration(
        &mut self,
        operands: SourceLoopOperands<'_>,
        scripts: SourceScriptOperands<'_>,
        iteration: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) -> Option<Box<ModuleCommandBindings>> {
        use crate::variable_bindings::SourceIterationBindingOutcome;
        select_contents_write_source(iteration);
        match crate::variable_bindings::transfer_source_iteration_bindings(
            Arc::make_mut(&mut iteration.source_variables),
            operands.variable_lists,
            *scripts.arguments,
            context.registry,
            context.invocation_offset,
        ) {
            SourceIterationBindingOutcome::Entered => {}
            SourceIterationBindingOutcome::StoreError => {
                outcomes.add_abrupt(Route::Tcl(Code::Error), iteration);
                return None;
            }
            SourceIterationBindingOutcome::Unknown => {
                iteration.mark_opaque_binding_mutation();
                outcomes.add_abrupt(Route::Unknown, iteration);
            }
        }
        let executed = self.walk_body_sequence(operands.repeated, scripts, iteration, context);
        self.settle_source_loop_iteration(executed, operands, scripts, context, outcomes)
    }

    /// Process completed body routes outside the frame for its descent. The
    /// continue step may recurse independently, without charging those owned
    /// outcome temporaries to every ordinary repeated-body entry.
    #[inline(never)]
    fn settle_source_loop_iteration(
        &mut self,
        executed: SourceOutcomes,
        operands: SourceLoopOperands<'_>,
        scripts: SourceScriptOperands<'_>,
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) -> Option<Box<ModuleCommandBindings>> {
        let mut back_edge = SourceOutcomes {
            normal: executed.normal,
            ..SourceOutcomes::default()
        };
        for (route, mut state) in executed.abrupt {
            match route {
                Route::Tcl(Code::Break) => outcomes.join(&SourceOutcomes::normal(&state)),
                Route::Tcl(Code::Continue) => {
                    let advanced =
                        self.walk_body_sequence(operands.continued, scripts, &mut state, context);
                    if let Some(normal) = advanced.normal {
                        back_edge.join(&SourceOutcomes::normal(&normal));
                    }
                    for (route, abrupt) in advanced.abrupt {
                        outcomes.add_abrupt(route, &abrupt);
                    }
                }
                _ => outcomes.add_abrupt(route, &state),
            }
        }
        back_edge.normal
    }
}

#[inline(never)]
fn join_source_loop_exit(outcomes: &mut SourceOutcomes, state: &ModuleCommandBindings) {
    outcomes.join(&SourceOutcomes::normal(state));
}

#[inline(never)]
fn finish_source_loop(
    outcomes: &mut SourceOutcomes,
    state: &mut ModuleCommandBindings,
) -> SourceOutcomes {
    outcomes.publish(state);
    std::mem::take(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_original_iterations_preserve_command_values_and_body_order() {
        let source = "proc first {} {set ::seen FIRST}; proc second {} {set ::seen SECOND}; foreach command {first second} {$command}; set result $::seen";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let bindings = SourceCommandBindings::analyse(source, config, registry);
        let body = u32::try_from(source.find("$command}").unwrap()).unwrap();
        let repeated = bindings.invocation_at_source("", body);
        for command in ["first", "second"] {
            assert_eq!(
                repeated
                    .lookup_command_word(command)
                    .selected_slot_presence(),
                super::super::SourceCommandSlotPresence::Present
            );
        }
        let after = u32::try_from(source.find("set result").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("set", after);
        assert_eq!(
            binding.variable_context.literal_value("::seen", registry),
            Some("SECOND")
        );
    }

    #[test]
    fn finite_original_iteration_padding_break_and_store_error_keep_native_order() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        for (source, expected) in [
            (
                "foreach {x y} {a b c} z {1 2 3} {set selected $z}; set result $selected",
                "3",
            ),
            (
                "foreach value {one two} {set selected $value; break}; set result $selected",
                "one",
            ),
        ] {
            let bindings = SourceCommandBindings::analyse(source, config, registry);
            let after = u32::try_from(source.find("set result").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("set", after);
            assert_eq!(
                binding.variable_context.literal_value("selected", registry),
                Some(expected),
                "{source}"
            );
        }
        let source =
            "array set value {}; foreach value {one two} {set selected ENTERED}; set result LATE";
        let bindings = SourceCommandBindings::analyse(source, config, registry);
        let after = u32::try_from(source.find("set result").unwrap()).unwrap();
        assert!(
            bindings
                .invocation_at_source("set", after)
                .invocation_site()
                .is_none()
        );
    }

    fn analyse(source: &str) -> (SourceCommandBindings, tcl_registry::CommandRegistry) {
        analyse_in(source, "tcl8.6")
    }

    fn analyse_in(
        source: &str,
        dialect: &str,
    ) -> (SourceCommandBindings, tcl_registry::CommandRegistry) {
        let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        (bindings, registry)
    }

    #[test]
    fn unknown_iteration_input_keeps_conversion_callback_effects_open() {
        // Unknown outer objects can run updater/free hooks before a zero-trip
        // exit on C8 and Jim too. Native paired controls are retained in
        // tcl-syntax/tests/data/native_list_methods/string-callbacks/README.md.
        for (dialect, expected, residual) in [
            (
                "tcl8.6",
                None,
                super::super::SourceVariableReadResidual::Unknown,
            ),
            (
                "tcl9.0",
                None,
                super::super::SourceVariableReadResidual::Unknown,
            ),
        ] {
            let source = "proc f {items} {foreach i $items {lappend r $i}; puts READY; return $r}";
            let (bindings, registry) = analyse_in(source, dialect);
            let site = u32::try_from(source.rfind("return").unwrap()).unwrap();
            let reads = bindings.variable_accesses_for_invocation_args(site);
            let read = reads
                .iter()
                .find(|read| read.original_spelling == "$r")
                .expect("the final original read is retained before its completion");
            let variables = &read.variable_context;
            let place = crate::var_resolve::resolve_literal_access(
                "r",
                variables,
                false,
                &registry,
                tcl_registry::TraceOperation::Read,
            );
            assert_eq!(
                variables.closed_contents_presence(&place),
                expected,
                "{dialect}: world={:?}, dynamic={}, residual={:?}, presence={:?}",
                variables.contents_world,
                variables.dynamic_bindings,
                read.context_residual(),
                variables.contents_presence(&place),
            );
            assert_eq!(read.context_residual(), residual, "{dialect}");
            assert!(
                !variables.read_produces_value(&place, &registry),
                "{dialect}"
            );
        }
    }

    #[test]
    fn possibly_empty_iteration_retains_the_unwritten_successor() {
        // Both paths manufacture ordinary lists in this same physical frame.
        // The native no-argument rand call chooses a path without a foreign
        // input object; the lists genuinely have zero or one element.
        let source = "set constructor list; if {[expr {rand() > 0.5}]} {set items [$constructor]} else {set items [$constructor ITEM]}; foreach i $items {lappend r $i}; set result $r";
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (bindings, registry) = analyse_in(source, dialect);
            let site = u32::try_from(source.rfind("set result").unwrap()).unwrap();
            let reads = bindings.variable_accesses_for_invocation_args(site);
            let read = reads
                .iter()
                .find(|read| read.original_spelling == "$r")
                .expect("the final original read is retained before its completion");
            assert_eq!(
                read.context_residual(),
                super::super::SourceVariableReadResidual::Closed,
                "{dialect}"
            );
            assert!(!read.context_alternatives().is_empty(), "{dialect}");
            for variables in read.context_alternatives() {
                let place = read.place_in_context(variables, &registry);
                assert_eq!(
                    variables.closed_contents_presence(&place),
                    Some(crate::var_resolve::ContentsPresence::DefinedOrUndefined),
                    "{dialect}: {:?}",
                    variables.contents_presence(&place)
                );
                assert!(
                    !variables.read_produces_value(&place, &registry),
                    "{dialect}"
                );
            }
        }
    }

    #[test]
    fn pooled_empty_iteration_keeps_unproved_root_effects_open() {
        // Inline pooled empty objects have no generic manufacturer receipt.
        // Their bytes alone cannot establish current object-method provenance.
        let source = "if {[expr {rand() > 0.5}]} {set items [list]} else {set items [list ITEM]}; foreach i $items {lappend r $i}; set result $r";
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (bindings, _) = analyse_in(source, dialect);
            let site = u32::try_from(source.rfind("set result").unwrap()).unwrap();
            let reads = bindings.variable_accesses_for_invocation_args(site);
            let read = reads
                .iter()
                .find(|read| read.original_spelling == "$r")
                .unwrap();
            assert_eq!(
                read.context_residual(),
                super::super::SourceVariableReadResidual::Unknown,
                "{dialect}"
            );
        }
    }

    #[test]
    fn nonempty_iteration_has_no_initial_skip_successor() {
        for body in [
            "set made YES",
            "set made YES; continue",
            "set made YES; break",
        ] {
            let source = format!("foreach value {{one}} {{{body}}}; puts DONE");
            let (bindings, registry) = analyse(&source);
            let offset = u32::try_from(source.find("puts DONE").unwrap()).unwrap();
            let invocation = bindings.invocation_at_source("puts", offset);
            assert_eq!(
                invocation.variable_context.literal_value("made", &registry),
                Some("YES"),
                "{source}",
            );
        }
    }

    #[test]
    fn proved_initial_condition_seeds_only_completed_first_iteration() {
        for source in [
            "set ready 1; while {$ready} {set ready 0; set made YES}; puts DONE",
            "for {set ready 1} {$ready} {set ready 0} {set made YES}; puts DONE",
        ] {
            let (bindings, registry) = analyse(source);
            let offset = u32::try_from(source.find("puts DONE").unwrap()).unwrap();
            assert_eq!(
                bindings
                    .invocation_at_source("puts", offset)
                    .variable_context
                    .literal_value("made", &registry),
                Some("YES"),
                "{source}",
            );
        }
        for source in [
            "while {0} {set made YES}; puts DONE",
            "while {$unknown} {set made YES; break}; puts DONE",
        ] {
            let (bindings, registry) = analyse(source);
            let offset = u32::try_from(source.find("puts DONE").unwrap()).unwrap();
            assert_ne!(
                bindings
                    .invocation_at_source("puts", offset)
                    .variable_context
                    .literal_value("made", &registry),
                Some("YES"),
                "{source}",
            );
        }
    }

    #[test]
    fn empty_or_failing_iterator_cannot_enter_its_body() {
        for source in [
            "foreach value {} {set made YES}; puts DONE",
            "foreach {} {one} {set made YES}; puts DONE",
            "foreach value \"{\" {set made YES}; puts DONE",
            "array set value {}; foreach value {one} {set made YES}; puts DONE",
            "foreach value {one} {break; set made YES}; puts DONE",
        ] {
            let (bindings, _) = analyse(source);
            let body = u32::try_from(source.find("set made YES").unwrap()).unwrap();
            assert_ne!(
                bindings
                    .invocation_at_source("set", body)
                    .runtime_reachability(),
                super::super::SourceRuntimeReachability::Reached,
                "{source}",
            );
        }
    }
}
