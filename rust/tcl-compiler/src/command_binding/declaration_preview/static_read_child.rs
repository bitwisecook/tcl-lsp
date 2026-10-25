// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original nested invocation ownership for alternate-static diagnostics only.

use super::{Arc, OriginalProcedureStaticReadAlternatives};

impl OriginalProcedureStaticReadAlternatives {
    pub(super) fn original_child_source_read(
        &self,
        tokens: &crate::ir::CommandTokens,
        original: &[tcl_lexer::NativeWord],
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        name: &str,
        span: tcl_lexer::Span,
    ) -> Option<()> {
        let plan = self.original_body_plan()?;
        if plan.fatal_tail.is_some()
            || !plan
                .commands
                .iter()
                .any(|command| command.words.as_slice() == original)
        {
            return None;
        }
        let registry = self.input.borrowed_context_registry().commands();
        let calls = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            tokens,
            self.input.lexer_config(),
            registry,
            metadata,
        )?;
        let mut found = false;
        for call in &calls {
            found |= self.original_child_read(call, metadata, name, span)?;
        }
        found.then_some(())
    }
    fn original_child_read(
        &self,
        call: &crate::word_subst::LiftedCall,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        name: &str,
        span: tcl_lexer::Span,
    ) -> Option<bool> {
        let registry = self.input.borrowed_context_registry().commands();
        let child = call.tokens.as_ref()?;
        let binding = child.source_binding.as_ref()?;
        if !Arc::ptr_eq(binding.original_analysis_attachment()?, &self.owner) {
            return None;
        }
        let child_metadata =
            binding.original_invocation_metadata_for_input(child, &self.input, registry)?;
        if child_metadata.source_analysis_input() != metadata.source_analysis_input() {
            return None;
        }
        let advice = binding.original_conditional_operand_layout_advice(child, registry)?;
        let state = &advice.snapshot.state;
        let header = &self.declaration.header;
        if advice.has_unclosed_source_coverage()
            || !Arc::ptr_eq(&advice.site.source, &header.declaration.source)
            || state.variable_frame != header.frame
            || binding.variable_frame != header.frame
            || !super::super::SourceCommandBindings::context_owns_frame(
                &binding.variable_context,
                &header.frame,
                Some(&header.namespace),
            )
            || !super::super::SourceCommandBindings::context_owns_frame(
                &state.source_variables,
                &header.frame,
                Some(&header.namespace),
            )
        {
            return None;
        }
        let child_context = advice
            .original_declaration_metadata_context(registry, child)
            .ok()??;
        if child_context.source_analysis_input() != metadata.source_analysis_input() {
            return None;
        }
        let selected =
            crate::registry_invocation::declaration_invocation_flow(registry, child, &advice)?;
        if !selected.effects.lookup_stable
            || selected.effects.frame_reachable()
            || selected.effects.unknown_reads
            || selected.effects.unknown_writes
            || !selected.aliases.is_empty()
            || !selected.caller_frame_aliases.is_empty()
            || !matches!(
                selected.flow,
                tcl_registry::script_body_flow::ScriptBodyFlow::None
            )
        {
            return None;
        }
        if selected
            .reads
            .iter()
            .chain(&selected.writes)
            .chain(&selected.possible_output_writes)
            .any(|variable| variable != name)
            || !selected.removes.is_empty()
            || !selected.loop_bindings.is_empty()
            || !selected.name_queries.is_empty()
        {
            return None;
        }
        let found = self.original_child_named_read(child, name)?;
        if found && (call.span.start() < span.start() || call.span.end() > span.end()) {
            return None;
        }
        Some(found)
    }
    /// Match each selected variable role to its own original written operand.
    /// Bound prefixes and unrelated equal words cannot provide that receipt.
    fn original_child_named_read(
        &self,
        child: &crate::ir::CommandTokens,
        name: &str,
    ) -> Option<bool> {
        let registry = self.input.borrowed_context_registry().commands();
        let binding = child.source_binding.as_ref()?;
        let source = binding.declaration_source_operand_layout_advice(registry, child)?;
        source
            .with_conditional_registry_facts(registry, child, |facts, effective, _values| {
                if !facts.arg_roles_complete {
                    return None;
                }
                let mut roles = Vec::new();
                for &(position, role) in &facts.arg_roles {
                    let argument = facts.argument_offset.checked_add(usize::from(position))?;
                    let read = role == tcl_registry::ArgRole::VarRead
                        && crate::existence_query::operand(facts)
                            .is_none_or(|(_, position)| position != argument);
                    let write = role == tcl_registry::ArgRole::VarWrite;
                    if !read && !write {
                        continue;
                    }
                    let crate::registry_invocation::InvocationWordOrigin::Written(written) =
                        effective.origins.get(argument.checked_add(1)?)?
                    else {
                        return None;
                    };
                    let word = child.words().get(*written)?;
                    let receipt = binding
                        .declaration_source_variable_operand_advice(registry, child, word)?;
                    if !receipt.matches_original_scalar_read(name) {
                        return None;
                    }
                    let reads = read
                        || (write
                            && facts
                                .traits
                                .contains(tcl_registry::Traits::READS_BEFORE_WRITE));
                    roles.push((*written, reads, write));
                }
                Some(roles)
            })
            .map(|roles| roles.iter().any(|(_, reads, _)| *reads))
    }
}

#[cfg(test)]
mod tests {
    use crate::compilation_unit::{CompilationUnit, FunctionUnit, UnitBuildOptions};

    const SOURCE: &str = "proc counter {} {{n 0}} { return [incr n] }; counter";

    fn input() -> crate::analyser::ResolvedAnalysisInput {
        tcl_spectcl::core_surfaces::ensure();
        crate::analyser::Analyser::new()
            .prepare_analysis_input("jim")
            .unwrap()
    }

    fn unit(source: &str, input: &crate::analyser::ResolvedAnalysisInput) -> CompilationUnit {
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: input.borrowed_context_registry().commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        )
    }

    fn child_view(
        function: &FunctionUnit,
    ) -> Option<(
        crate::ssa::SsaSourceView<'_>,
        crate::ssa::Symbol,
        tcl_lexer::Span,
    )> {
        for (block, body) in &function.ssa.blocks {
            for index in 0..=body.statements.len() {
                let view = if index == body.statements.len() {
                    crate::ssa::SsaSourceView::at_terminator(&function.ssa, *block)
                } else {
                    crate::ssa::SsaSourceView::at_statement(&function.ssa, *block, index)
                };
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                if tokens.nested_bindings.is_empty() {
                    continue;
                }
                let Some(symbol) = view
                    .source_names()
                    .and_then(|names| names.get("n"))
                    .copied()
                else {
                    continue;
                };
                let start = tokens.words().first()?.source().span.start();
                let end = tokens.words().last()?.source().span.end();
                return Some((view, symbol, tcl_lexer::Span::new(start, end)));
            }
        }
        None
    }

    fn accepts(unit: &CompilationUnit, input: &crate::analyser::ResolvedAnalysisInput) -> bool {
        let registry = input.borrowed_context_registry().commands();
        unit.body_units.values().any(|function| {
            let Some(receipt) =
                unit.original_native_declaration_static_read_alternatives(function, registry)
            else {
                return false;
            };
            let Some((view, symbol, span)) = child_view(function) else {
                return false;
            };
            receipt.may_select_direct_source_read(view, symbol, "n", span)
        })
    }

    #[test]
    fn original_nested_named_static_read_keeps_original_positive_expectations() {
        let input = input();
        assert!(accepts(&unit(SOURCE, &input), &input));
        for per_item in [false, true] {
            let mut analyser = crate::analyser::Analyser::new();
            let result = if per_item {
                analyser.analyse_per_item(SOURCE, "jim")
            } else {
                analyser.analyse(SOURCE, "jim")
            };
            assert!(
                result
                    .diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.code != tcl_core_types::DiagCode::W210),
                "per_item={per_item}: {:?}",
                result.diagnostics
            );
        }
    }

    #[test]
    fn original_nested_named_read_refuses_alias_rebinding_callee_and_opaque_children() {
        let input = input();
        for source in [
            "proc counter {} {{n 0}} {upvar 1 other n; return [incr n]}; counter",
            "proc incr {name} {return 1}; proc counter {} {{n 0}} {return [incr n]}; counter",
            "proc child {} {incr n}; proc counter {} {{n 0}} {return [child]}; counter",
            "proc counter {} {{n 0}} {return [unknown_command [incr n]]}; counter",
            "interp alias {} readN {} incr n; proc counter {} {{n 0}} {return [readN n]}; counter",
            "proc counter {} {{n 0}} {return [incr n(x) n]}; counter",
            "proc counter {} {{n 0}} {return [set ::outside 1][incr n]}; counter",
        ] {
            assert!(!accepts(&unit(source, &input), &input), "{source}");
        }
    }

    #[test]
    fn original_nested_named_read_refuses_equal_foreign_and_stale_source_owners() {
        let input = input();
        let registry = input.borrowed_context_registry().commands();
        let current = unit(SOURCE, &input);
        let function = current
            .ir_module
            .original_native_declaration_body_units
            .keys()
            .filter_map(|id| current.body_units.get(id))
            .find(|function| child_view(function).is_some())
            .unwrap();
        let receipt = current
            .original_native_declaration_static_read_alternatives(function, registry)
            .unwrap();
        for source in [
            SOURCE,
            "proc counter {} {{n 0}} { return [incr n 2] }; counter",
        ] {
            let foreign = unit(source, &input);
            let foreign_function = foreign
                .ir_module
                .original_native_declaration_body_units
                .keys()
                .filter_map(|id| foreign.body_units.get(id))
                .find(|function| child_view(function).is_some())
                .unwrap();
            let (view, symbol, span) = child_view(foreign_function).unwrap();
            assert!(!receipt.may_select_direct_source_read(view, symbol, "n", span));
        }
    }
    #[test]
    fn original_nested_read_rejects_a_child_with_a_foreign_frame() {
        let input = input();
        let registry = input.borrowed_context_registry().commands();
        let current = unit(SOURCE, &input);
        let function = current
            .ir_module
            .original_native_declaration_body_units
            .keys()
            .filter_map(|id| current.body_units.get(id))
            .find(|function| child_view(function).is_some())
            .unwrap();
        let receipt = current
            .original_native_declaration_static_read_alternatives(function, registry)
            .unwrap();
        let (view, _, span) = child_view(function).unwrap();
        let tokens = view.source_tokens().unwrap();
        let metadata = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .original_invocation_metadata_for_input(tokens, &input, registry)
            .unwrap();
        let mut calls = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            tokens,
            input.lexer_config(),
            registry,
            metadata,
        )
        .unwrap();
        assert_eq!(calls.len(), 1);
        let call = calls.first_mut().unwrap();
        assert_eq!(
            receipt.original_child_read(call, metadata, "n", span),
            Some(true)
        );
        call.tokens
            .as_mut()
            .unwrap()
            .source_binding
            .as_mut()
            .unwrap()
            .variable_frame = crate::var_resolve::VariableExecutionFrame::Unknown;
        assert_eq!(receipt.original_child_read(call, metadata, "n", span), None);
    }
}
