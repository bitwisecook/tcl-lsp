// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original nested invocation ownership for alternate-static diagnostics only.

use super::{Arc, OriginalProcedureStaticReadAlternatives};

macro_rules! child_read_guard {
    ($stage:literal, $value:expr) => {
        match $value {
            Some(value) => value,
            None => return super::trace_static_read_refusal($stage, None),
        }
    };
}

impl OriginalProcedureStaticReadAlternatives {
    pub(super) fn original_child_source_read(
        &self,
        tokens: &crate::ir::CommandTokens,
        original: &[tcl_lexer::NativeWord],
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        name: &str,
        span: tcl_lexer::Span,
    ) -> Option<()> {
        let plan = child_read_guard!("child.body_plan", self.original_body_plan());
        if plan.fatal_tail.is_some()
            || !plan
                .commands
                .iter()
                .any(|command| command.words.as_slice() == original)
        {
            return super::trace_static_read_refusal("child.original_geometry", None);
        }
        let registry = self.input.borrowed_context_registry().commands();
        let calls = child_read_guard!(
            "child.inventory",
            crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                tokens,
                self.input.lexer_config(),
                registry,
                metadata,
            )
        );
        let mut found = false;
        for call in &calls {
            found |= child_read_guard!(
                "child.read",
                self.original_child_read(call, metadata, name, span)
            );
        }
        child_read_guard!("child.no_matching_read", found.then_some(()));
        Some(())
    }
    fn original_child_read(
        &self,
        call: &crate::word_subst::LiftedCall,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        name: &str,
        span: tcl_lexer::Span,
    ) -> Option<bool> {
        let registry = self.input.borrowed_context_registry().commands();
        let child = child_read_guard!("child.tokens", call.tokens.as_ref());
        let binding = child_read_guard!("child.binding", child.source_binding.as_ref());
        if !Arc::ptr_eq(
            child_read_guard!(
                "child.analysis_attachment",
                binding.original_analysis_attachment()
            ),
            &self.owner,
        ) {
            return super::trace_static_read_refusal("child.analysis_owner", None);
        }
        let child_metadata = child_read_guard!(
            "child.invocation_metadata",
            binding.original_invocation_metadata_for_input(child, &self.input, registry)
        );
        if child_metadata.source_analysis_input() != metadata.source_analysis_input() {
            return super::trace_static_read_refusal("child.input", None);
        }
        let advice = child_read_guard!(
            "child.operand_advice",
            binding.original_conditional_operand_layout_advice(child, registry)
        );
        child_read_guard!(
            "child.scope",
            self.trace_original_child_scope(child, binding, &advice)
        );
        let child_context = child_read_guard!(
            "child.declaration_metadata",
            child_read_guard!(
                "child.declaration_metadata_query",
                advice
                    .original_declaration_metadata_context(registry, child)
                    .ok()
            )
        );
        if child_context.source_analysis_input() != metadata.source_analysis_input() {
            return super::trace_static_read_refusal("child.declaration_input", None);
        }
        let selected = child_read_guard!(
            "child.invocation_flow",
            crate::registry_invocation::declaration_invocation_flow(registry, child, &advice)
        );
        child_read_guard!(
            "child.effects",
            Self::trace_original_child_effects(&selected, name)
        );
        let found = child_read_guard!(
            "child.named_read",
            self.original_child_named_read(child, name)
        );
        if found && (call.span.start() < span.start() || call.span.end() > span.end()) {
            return super::trace_static_read_refusal("child.container", None);
        }
        Some(found)
    }
    fn trace_original_child_effects(
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        name: &str,
    ) -> Option<()> {
        if !selected.effects.lookup_stable {
            return super::trace_static_read_refusal("child.lookup_stable", None);
        }
        if selected.effects.frame_reachable() {
            return super::trace_static_read_refusal("child.frame_reachable", None);
        }
        if selected.effects.unknown_reads {
            return super::trace_static_read_refusal("child.unknown_reads", None);
        }
        if selected.effects.unknown_writes {
            return super::trace_static_read_refusal("child.unknown_writes", None);
        }
        if !selected.aliases.is_empty() {
            return super::trace_static_read_refusal("child.aliases", None);
        }
        if !selected.caller_frame_aliases.is_empty() {
            return super::trace_static_read_refusal("child.caller_aliases", None);
        }
        if !matches!(
            selected.flow,
            tcl_registry::script_body_flow::ScriptBodyFlow::None
        ) {
            return super::trace_static_read_refusal("child.script_flow", None);
        }
        if selected
            .reads
            .iter()
            .chain(&selected.writes)
            .chain(&selected.possible_output_writes)
            .any(|variable| variable != name)
        {
            return super::trace_static_read_refusal("child.other_variable", None);
        }
        if !selected.removes.is_empty() {
            return super::trace_static_read_refusal("child.removes", None);
        }
        if !selected.loop_bindings.is_empty() {
            return super::trace_static_read_refusal("child.loop_bindings", None);
        }
        if !selected.name_queries.is_empty() {
            return super::trace_static_read_refusal("child.name_queries", None);
        }
        Some(())
    }

    fn trace_original_child_scope(
        &self,
        child: &crate::ir::CommandTokens,
        binding: &super::super::SourceInvocationBinding,
        advice: &super::super::OriginalCompilationLookupAdvice,
    ) -> Option<()> {
        let state = &advice.snapshot.state;
        let header = &self.declaration.header;
        if advice.has_unclosed_source_coverage() {
            return super::trace_static_read_refusal("child.coverage", None);
        }
        if !Arc::ptr_eq(&advice.site.source, &header.declaration.source) {
            return super::trace_static_read_refusal("child.source_origin", None);
        }
        let frame = child_read_guard!(
            "child.body_frame_owner",
            self.original_static_child_frame(binding, advice)
        );
        if state.variable_frame != *frame {
            return super::trace_static_read_refusal("child.snapshot_frame", None);
        }
        let binding_frame = &binding.variable_frame;
        if binding_frame != frame
            && self
                .original_known_child_binding_scope(child, binding, advice)
                .is_none()
        {
            let label = child_binding_frame_refusal(binding_frame, binding.lookup_state.is_some());
            return super::trace_static_read_refusal(label, None);
        }
        if !super::super::SourceCommandBindings::context_owns_frame(
            &binding.variable_context,
            binding_frame,
            Some(&header.namespace),
        ) {
            return super::trace_static_read_refusal("child.binding_context", None);
        }
        if !super::super::SourceCommandBindings::context_owns_frame(
            &state.source_variables,
            frame,
            Some(&header.namespace),
        ) {
            return super::trace_static_read_refusal("child.snapshot_context", None);
        }
        Some(())
    }

    /// The declaration-header frame and installed-body recipe have separate
    /// issuers. Only an original body allocation can join their source purpose;
    /// this does not identify either recipe with an entered activation.
    fn original_static_child_frame<'a>(
        &'a self,
        binding: &'a super::super::SourceInvocationBinding,
        advice: &super::super::OriginalCompilationLookupAdvice,
    ) -> Option<&'a crate::var_resolve::VariableExecutionFrame> {
        let header = &self.declaration.header;
        if !Arc::ptr_eq(&advice.site.source, &header.declaration.source) {
            return None;
        }
        if advice.snapshot.state.variable_frame == header.frame {
            return Some(&header.frame);
        }
        self.original_static_child_body(binding, advice)
            .map(|body| body.frame())
    }

    fn original_static_child_body<'a>(
        &'a self,
        binding: &'a super::super::SourceInvocationBinding,
        advice: &super::super::OriginalCompilationLookupAdvice,
    ) -> Option<&'a Arc<super::super::SourceConditionalBodyEntry>> {
        let header = &self.declaration.header;
        let rows = super::super::declaration_layout::original_declaration_layouts(
            binding.declaration_layout_observations.as_deref()?,
        )?;
        let row = rows
            .clone()
            .find(|row| Arc::ptr_eq(&row.snapshot, &advice.snapshot))?;
        let super::super::declaration_layout::OriginalDiagnosticFrameEntry::Body(body) =
            row.entry.as_ref()
        else {
            return None;
        };
        let allocation = body.allocation();
        let source = body.source();
        let span = self.declaration.read.original_body_span()?;
        let super::super::ExecutedScriptMapping::Contiguous { base } = source.mapping else {
            return None;
        };
        if allocation.incarnation != super::super::AllocationIncarnation::First
            || allocation.site.offset != header.declaration.offset
            || !Arc::ptr_eq(&allocation.site.source, &header.declaration.source)
            || allocation.namespace != header.namespace
            || !Arc::ptr_eq(&source.origin, &header.declaration.source)
            || base != span.start()
            || source.text.bytes() != self.declaration.read.original_body_text()?.as_bytes()
            || !body.owns_source(&advice.site.source, advice.site.offset)
            || body.namespace_context() != Some(&header.namespace)
            || body.parameters() != header.parameters.as_slice()
            || body.original_formal_topology().is_none()
            || body.original_formal_topology() != header.original_parameters.as_ref()
            || row.config != self.input.lexer_config()
            || row.namespace != header.namespace
            || row.words.as_ref() != advice.words.as_ref()
            || row.snapshot.state.variable_frame != *body.frame()
            || !row
                .entry
                .owns_original_context(&row.snapshot.state.source_variables)
        {
            return None;
        }
        Some(body)
    }

    /// Authenticate a known projection with its exact live producer before
    /// relating its frame to the independent original body recipe.
    fn original_known_child_binding_scope(
        &self,
        child: &crate::ir::CommandTokens,
        binding: &super::super::SourceInvocationBinding,
        advice: &super::super::OriginalCompilationLookupAdvice,
    ) -> Option<()> {
        if !matches!(
            binding.variable_frame,
            crate::var_resolve::VariableExecutionFrame::Procedure { .. }
        ) {
            return None;
        }
        let lookup = binding.lookup_state.as_ref()?;
        if lookup.state.variable_frame != binding.variable_frame {
            return None;
        }
        let body = self.original_static_child_body(binding, advice)?;
        let current = self.source_analysis.upgrade()?;
        if current
            .original_analysis_owner
            .as_ref()
            .is_none_or(|owner| !Arc::ptr_eq(owner, &self.owner))
            || current.lexer_config != Some(self.input.lexer_config())
            || current
                .root_origin
                .as_ref()
                .is_none_or(|origin| !Arc::ptr_eq(origin, &advice.site.source))
        {
            return None;
        }
        let mut produced = child.clone();
        current.stamp_original_tokens(&mut produced);
        if produced.source_binding.as_ref() != Some(binding) {
            return None;
        }
        let header = &self.declaration.header;
        let header_owned = binding.variable_frame == header.frame
            && super::super::SourceCommandBindings::context_owns_frame(
                &binding.variable_context,
                &header.frame,
                Some(&header.namespace),
            );
        (header_owned
            || (current.original_body_owns_frame(body, &binding.variable_frame)
                && current.original_body_owns_context(body, &binding.variable_context)))
        .then_some(())
    }

    /// Match each selected variable role to its own original written operand.
    /// Bound prefixes and unrelated equal words cannot provide that receipt.
    fn original_child_named_read(
        &self,
        child: &crate::ir::CommandTokens,
        name: &str,
    ) -> Option<bool> {
        let registry = self.input.borrowed_context_registry().commands();
        let binding = child_read_guard!("child.binding", child.source_binding.as_ref());
        let source = child_read_guard!(
            "child.source_operand_advice",
            binding.declaration_source_operand_layout_advice(registry, child)
        );
        Some(child_read_guard!(
                "child.role_matrix",
                source
                    .with_conditional_registry_facts(
                        registry,
                        child,
                        |facts, effective, _values| {
                            if !facts.arg_roles_complete {
                                return super::trace_static_read_refusal(
                                    "child.roles_complete",
                                    None,
                                );
                            }
                            let mut roles = Vec::new();
                            for &(position, role) in &facts.arg_roles {
                                let argument = child_read_guard!(
                                    "child.role_index",
                                    facts.argument_offset.checked_add(usize::from(position))
                                );
                                let read = role == tcl_registry::ArgRole::VarRead
                                    && crate::existence_query::operand(facts)
                                        .is_none_or(|(_, position)| position != argument);
                                let write = role == tcl_registry::ArgRole::VarWrite;
                                if !read && !write {
                                    continue;
                                }
                                let crate::registry_invocation::InvocationWordOrigin::Written(
                                    written,
                                ) = child_read_guard!(
                                    "child.argument_origin",
                                    effective.origins.get(child_read_guard!(
                                        "child.argument_index",
                                        argument.checked_add(1)
                                    ))
                                )
                                else {
                                    return super::trace_static_read_refusal(
                                        "child.written_origin",
                                        None,
                                    );
                                };
                                let word = child_read_guard!(
                                    "child.written_word",
                                    child.words().get(*written)
                                );
                                let receipt = child_read_guard!(
                                    "child.scalar_operand",
                                    binding.declaration_source_variable_operand_advice(
                                        registry, child, word
                                    )
                                );
                                if !receipt.matches_original_scalar_read(name) {
                                    return super::trace_static_read_refusal(
                                        "child.scalar_correspondence",
                                        None,
                                    );
                                }
                                let reads = read
                                    || (write
                                        && facts
                                            .traits
                                            .contains(tcl_registry::Traits::READS_BEFORE_WRITE));
                                roles.push((*written, reads, write));
                            }
                            Some(roles)
                        }
                    )
                    .map(|roles| roles.iter().any(|(_, reads, _)| *reads))
            ))
    }
}

fn child_binding_frame_refusal(
    frame: &crate::var_resolve::VariableExecutionFrame,
    has_lookup_snapshot: bool,
) -> &'static str {
    use crate::var_resolve::VariableExecutionFrame as Frame;
    match (frame, has_lookup_snapshot) {
        (Frame::Global, true) => "child.binding_frame.Global.lookup",
        (Frame::Global, false) => "child.binding_frame.Global.no_lookup",
        (Frame::Namespace(_), true) => "child.binding_frame.Namespace.lookup",
        (Frame::Namespace(_), false) => "child.binding_frame.Namespace.no_lookup",
        (Frame::NamespaceActivation { .. }, true) => {
            "child.binding_frame.NamespaceActivation.lookup"
        }
        (Frame::NamespaceActivation { .. }, false) => {
            "child.binding_frame.NamespaceActivation.no_lookup"
        }
        (Frame::Procedure { .. }, true) => "child.binding_frame.Procedure.lookup",
        (Frame::Procedure { .. }, false) => "child.binding_frame.Procedure.no_lookup",
        (Frame::ReceiverMethod { .. }, true) => "child.binding_frame.ReceiverMethod.lookup",
        (Frame::ReceiverMethod { .. }, false) => "child.binding_frame.ReceiverMethod.no_lookup",
        (Frame::Selected { .. }, true) => "child.binding_frame.Selected.lookup",
        (Frame::Selected { .. }, false) => "child.binding_frame.Selected.no_lookup",
        (Frame::NamespaceIdentity { .. }, true) => "child.binding_frame.NamespaceIdentity.lookup",
        (Frame::NamespaceIdentity { .. }, false) => {
            "child.binding_frame.NamespaceIdentity.no_lookup"
        }
        (Frame::Unknown, true) => "child.binding_frame.Unknown.lookup",
        (Frame::Unknown, false) => "child.binding_frame.Unknown.no_lookup",
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
    fn body_frame_evidence(
        current: &CompilationUnit,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> (
        super::OriginalProcedureStaticReadAlternatives,
        crate::ir::CommandTokens,
        crate::command_binding::OriginalCompilationLookupAdvice,
    ) {
        let registry = input.borrowed_context_registry().commands();
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
        let (view, _, _) = child_view(function).unwrap();
        let tokens = view.source_tokens().unwrap();
        let metadata = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .original_invocation_metadata_for_input(tokens, input, registry)
            .unwrap();
        let child = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            tokens,
            input.lexer_config(),
            registry,
            metadata,
        )
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .tokens
        .unwrap();
        let advice = child
            .source_binding
            .as_ref()
            .unwrap()
            .original_conditional_operand_layout_advice(&child, registry)
            .unwrap();
        (receipt, child, advice)
    }

    #[test]
    fn original_child_body_keeps_its_issued_frame_distinct_from_header_layout() {
        let input = input();
        let current = unit(SOURCE, &input);
        let (receipt, child, advice) = body_frame_evidence(&current, &input);
        let frame = receipt
            .original_static_child_frame(child.source_binding.as_ref().unwrap(), &advice)
            .unwrap();
        assert_ne!(frame, &receipt.declaration.header.frame);
        assert_eq!(frame, &advice.snapshot.state.variable_frame);
        assert!(accepts(&current, &input));
    }

    #[test]
    fn original_child_body_frame_refuses_missing_reissued_and_foreign_rows() {
        let input = input();
        let current = unit(SOURCE, &input);
        let (receipt, child, advice) = body_frame_evidence(&current, &input);
        for replaced_snapshot in [false, true] {
            let mut changed = child.clone();
            let binding = changed.source_binding.as_mut().unwrap();
            let mut rows = binding
                .declaration_layout_observations
                .as_deref()
                .unwrap()
                .to_vec();
            for row in &mut rows {
                if replaced_snapshot {
                    row.snapshot = std::sync::Arc::new((*row.snapshot).clone());
                } else {
                    row.issuer = crate::command_binding::declaration_layout::DeclarationLayoutIssuer::EnteredActivation;
                }
            }
            binding.declaration_layout_observations = Some(rows.into());
            assert!(
                receipt
                    .original_static_child_frame(binding, &advice)
                    .is_none()
            );
        }
        let foreign = unit(SOURCE, &input);
        let (_, foreign_child, foreign_advice) = body_frame_evidence(&foreign, &input);
        assert!(
            receipt
                .original_static_child_frame(
                    foreign_child.source_binding.as_ref().unwrap(),
                    &foreign_advice
                )
                .is_none()
        );
    }

    #[test]
    fn original_child_body_frame_refuses_different_allocation_and_missing_formals() {
        let input = input();
        let current = unit(SOURCE, &input);
        for change in [false, true] {
            let (mut receipt, child, advice) = body_frame_evidence(&current, &input);
            if change {
                receipt.declaration.header.declaration.offset += 1;
            } else {
                receipt.declaration.header.original_parameters = None;
            }
            assert!(
                receipt
                    .original_static_child_frame(child.source_binding.as_ref().unwrap(), &advice)
                    .is_none()
            );
        }
    }
    #[test]
    fn known_child_projection_uses_live_producer_and_separate_body_scope() {
        let input = input();
        let current = unit(SOURCE, &input);
        let (receipt, child, advice) = body_frame_evidence(&current, &input);
        let binding = child.source_binding.as_ref().unwrap();
        assert!(matches!(
            binding.variable_frame,
            crate::var_resolve::VariableExecutionFrame::Procedure { .. }
        ));
        assert!(binding.lookup_state.is_some());
        assert!(binding.original_source_operand_attachment.is_none());
        let live = receipt.source_analysis.upgrade().unwrap();
        let mut produced = child.clone();
        live.stamp_original_tokens(&mut produced);
        let produced_binding = produced.source_binding.as_ref().unwrap();
        assert!(
            produced_binding
                .original_source_operand_attachment
                .is_none()
        );
        assert_eq!(produced_binding, binding);
        assert_eq!(
            receipt.original_known_child_binding_scope(&child, binding, &advice),
            Some(())
        );
        assert_ne!(
            receipt.declaration.header.frame,
            advice.snapshot.state.variable_frame
        );
        assert!(accepts(&current, &input));
    }

    #[test]
    fn known_child_projection_refuses_consistently_replaced_producer_fields() {
        let input = input();
        let current = unit(SOURCE, &input);
        let (receipt, child, advice) = body_frame_evidence(&current, &input);
        for axis in 0..5 {
            let mut changed = child.clone();
            let binding = changed.source_binding.as_mut().unwrap();
            if axis == 4 {
                binding.lookup_state = None;
            } else {
                match axis {
                    0 => binding.unknown = !binding.unknown,
                    1 => {
                        let variables = std::sync::Arc::make_mut(&mut binding.variable_context);
                        variables.dynamic_bindings = !variables.dynamic_bindings;
                    }
                    2 => {
                        binding.variable_frame =
                            crate::var_resolve::VariableExecutionFrame::Procedure {
                                namespace: "::".to_owned(),
                                identity: "foreign-original-activation".to_owned(),
                            };
                        binding.variable_context = std::sync::Arc::new(
                            binding.variable_context.in_frame(&binding.variable_frame),
                        );
                    }
                    _ => {
                        binding.variable_frame =
                            crate::var_resolve::VariableExecutionFrame::Unknown;
                        binding.variable_context = std::sync::Arc::new(
                            binding.variable_context.in_frame(&binding.variable_frame),
                        );
                    }
                }
                let frame = binding.variable_frame.clone();
                let variables = std::sync::Arc::clone(&binding.variable_context);
                let snapshot = std::sync::Arc::make_mut(binding.lookup_state.as_mut().unwrap());
                snapshot.state.variable_frame = frame;
                snapshot.state.source_variables = variables;
                if axis == 0 {
                    snapshot.state.unclosed_command_world = !snapshot.state.unclosed_command_world;
                }
                snapshot.fingerprint = std::sync::OnceLock::new();
                snapshot.compiler_name_effects = crate::command_binding::original_compiler_effects::SourceCompilerNameEffectsCache::default();
            }
            assert_eq!(
                receipt.original_known_child_binding_scope(
                    &changed,
                    changed.source_binding.as_ref().unwrap(),
                    &advice,
                ),
                None,
                "altered producer axis {axis}",
            );
        }
    }

    #[test]
    fn known_child_projection_refuses_foreign_and_retired_source_owner() {
        let input = input();
        let registry = input.borrowed_context_registry().commands();
        let current = unit(SOURCE, &input);
        let (mut receipt, child, advice) = body_frame_evidence(&current, &input);
        let foreign = unit(SOURCE, &input);
        let foreign_owner = foreign
            .ir_module
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .positioned_owner_for_module(&foreign.ir_module, registry)
            .unwrap();
        receipt.source_analysis = std::sync::Arc::downgrade(foreign_owner);
        assert_eq!(
            receipt.original_known_child_binding_scope(
                &child,
                child.source_binding.as_ref().unwrap(),
                &advice,
            ),
            None,
        );
        let (retired, child, advice) = {
            let retired_unit = unit(SOURCE, &input);
            body_frame_evidence(&retired_unit, &input)
        };
        assert!(retired.source_analysis.upgrade().is_none());
        assert_eq!(
            retired.original_known_child_binding_scope(
                &child,
                child.source_binding.as_ref().unwrap(),
                &advice,
            ),
            None,
        );
    }
}
