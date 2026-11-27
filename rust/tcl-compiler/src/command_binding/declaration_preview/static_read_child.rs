// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
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
    ) -> Option<&'a tcl_names::variable::VariableExecutionFrame> {
        let header = &self.declaration.header;
        if binding
            .original_analysis_owner
            .as_ref()
            .is_none_or(|owner| !Arc::ptr_eq(owner, &self.owner))
        {
            return None;
        }
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
        let tcl_names::origin::ExecutedScriptMapping::Contiguous { base } = source.mapping else {
            return None;
        };
        if allocation.incarnation != tcl_names::command::AllocationIncarnation::First
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
            tcl_names::variable::VariableExecutionFrame::Procedure { .. }
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
    frame: &tcl_names::variable::VariableExecutionFrame,
    has_lookup_snapshot: bool,
) -> &'static str {
    use tcl_names::variable::VariableExecutionFrame as Frame;
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
