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

//! Whether control reaches a statement: how one invocation completes and whether the statements before a point fall through.

use super::super::super::state::Analyser;
use super::{ControlArm, MAX_UNKNOWN_BODY_DEPTH};
use crate::signature_scan::params::bind_proc_formals;
use tcl_lexer::Span;
use tcl_source::segment::SegmentedCommand;

impl Analyser {
    fn logical_segment_invocation_completion(
        registry: &tcl_registry::CommandRegistry,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        tokens: &crate::ir::CommandTokens,
        #[cfg(test)] trace: &impl Fn(&str, bool),
    ) -> Option<(
        crate::registry_invocation::ResolvedStatementInvocation,
        tcl_registry::registry::InvocationCompletion,
    )> {
        let invocation =
            crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                registry, metadata, tokens,
            );
        #[cfg(test)]
        trace("operation.selected", invocation.is_some());
        let invocation = invocation?;
        let realm = tokens.source_binding.as_ref().and_then(|binding| {
            binding.original_source_realm_for_input(
                tokens,
                metadata.source_analysis_input()?,
                registry,
            )
        });
        #[cfg(test)]
        trace("realm.original", realm.is_some());
        let realm = realm?;
        let completion = invocation.with_metadata_schema(registry, metadata, realm, |selected| {
            Some(selected.authored_source_invocation_completion())
        });
        #[cfg(test)]
        trace("completion.schema", completion.is_some());
        #[cfg(test)]
        trace(
            "completion.known",
            completion.is_some_and(|completion| {
                completion != tcl_registry::registry::InvocationCompletion::Unknown
            }),
        );
        let completion = completion?;
        Some((invocation, completion))
    }

    fn written_segment_completion(
        effective: &crate::registry_invocation::EffectiveCommandWords,
        completion: tcl_registry::registry::InvocationCompletion,
    ) -> tcl_registry::registry::InvocationCompletion {
        use crate::registry_invocation::InvocationWordOrigin;
        use tcl_registry::registry::InvocationCompletion;
        match completion {
            InvocationCompletion::ReturnsResult(Some(index)) => {
                match index
                    .checked_add(1)
                    .and_then(|index| effective.origins.get(index))
                {
                    Some(InvocationWordOrigin::Written(index)) => index
                        .checked_sub(1)
                        .map_or(InvocationCompletion::Unknown, |index| {
                            InvocationCompletion::ReturnsResult(Some(index))
                        }),
                    _ => InvocationCompletion::Unknown,
                }
            }
            completion => completion,
        }
    }

    #[cfg(test)]
    fn trace_segment_source_guards(
        &self,
        logical_source: bool,
        retained: Option<&crate::analyser::ResolvedAnalysisInput>,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        trace: &impl Fn(&str, bool),
    ) {
        trace("source.logical", logical_source);
        trace("source.retained", retained.is_some());
        trace("metadata.request", metadata.is_some());
        trace("tokens.binding", tokens.source_binding.is_some());
        let binding = tokens.source_binding.as_ref();
        trace(
            "tokens.config",
            binding.and_then(|binding| binding.original_lexer_config_for_tokens(tokens))
                == Some(self.file_lexer_config()),
        );
        trace(
            "metadata.original",
            binding.zip(retained).is_some_and(|(binding, input)| {
                binding
                    .original_invocation_metadata_for_input(tokens, input, registry)
                    .is_some()
            }),
        );
        let advice = binding.and_then(|binding| binding.declaration_operand_layout_advice(tokens));
        trace("declaration.layout", advice.is_some());
        trace(
            "declaration.closed",
            advice.is_some_and(|advice| advice.closed_logical_source_lookup()),
        );
    }

    /// Completion follows the actual selected implementation and frozen argv.
    /// A captured alias operand has no original segment index to return.
    pub(in super::super) fn segment_invocation_completion(
        &self,
        seg: &SegmentedCommand,
    ) -> tcl_registry::registry::InvocationCompletion {
        use tcl_registry::registry::InvocationCompletion;
        let context = self.analysis_context();
        let registry = context.commands();
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &self.cached_source_map(),
            self.file_lexer_config(),
            seg,
        );
        self.head_identities
            .source_bindings_ref()
            .stamp_original_tokens(&mut tokens);
        let retained = self
            .resolved_input
            .as_ref()
            .or(self.result.resolved_input.as_ref());

        #[cfg(test)]
        let trace_enabled =
            tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some();
        #[cfg(test)]
        let trace = |label: &str, passed: bool| {
            if trace_enabled {
                eprintln!("ORIGINAL_COMPLETION_GUARD label={label} passed={passed}");
            }
        };
        let metadata = retained.and_then(|input| {
            crate::registry_invocation::InvocationMetadataContext::for_source_input(
                registry,
                input,
                self.file_lexer_config(),
                Some(input.unit_profile()),
            )
        });
        let purpose = metadata.and_then(|metadata| {
            tokens
                .source_binding
                .as_ref()?
                .original_source_advice_purpose(&tokens, metadata, registry)
        });
        let Some(purpose) = purpose else {
            return InvocationCompletion::Unknown;
        };
        let logical_source =
            purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
        #[cfg(test)]
        if trace_enabled {
            self.trace_segment_source_guards(
                logical_source,
                retained,
                metadata,
                &tokens,
                registry,
                &trace,
            );
        }
        let selected = if logical_source {
            let Some(metadata) = metadata else {
                return InvocationCompletion::Unknown;
            };
            let Some(selected) = Self::logical_segment_invocation_completion(
                registry,
                metadata,
                &tokens,
                #[cfg(test)]
                &trace,
            ) else {
                return InvocationCompletion::Unknown;
            };
            selected
        } else {
            // Native completion retains its independently proved handler path.
            let Some(invocation) =
                crate::registry_invocation::resolved_tokens_invocation_in_context(
                    &context, &tokens,
                )
            else {
                return InvocationCompletion::Unknown;
            };
            let completion = invocation.with_argument_words(|words| {
                registry.invocation_completion_words(
                    &invocation.facts.canonical_command,
                    words.arguments(),
                    invocation
                        .dialect
                        .and_then(tcl_native_model::InvocationDialect::authoring_query),
                )
            });
            (invocation, completion)
        };
        let (invocation, completion) = selected;
        Self::written_segment_completion(&invocation.effective, completion)
    }

    /// Whether the statements of `body_span` before `stop_off` all fall
    /// through, so control really reaches `stop_off`.
    ///
    /// `depth` is the nesting of *control bodies* this walk has descended
    /// into — [`Self::static_statement_falls_through`] re-enters here for each
    /// arm whose body may run, so nested control flow recurses one native
    /// frame per level and has to be bounded like every other recursive walk
    /// in this file ([`MAX_UNKNOWN_BODY_DEPTH`]). Past
    /// the cap the walk abstains, which is the direction it already takes for
    /// anything it cannot read. Entry points that are not themselves inside a
    /// body pass `0`.
    pub(super) fn static_prefix_falls_through(
        &mut self,
        body_span: Span,
        stop_off: u32,
        depth: u32,
    ) -> bool {
        if self.retained_static_source_context().is_none() {
            return false;
        }
        let Some(statements) = self.direct_statements_in_span(body_span) else {
            return false;
        };
        for seg in statements
            .into_iter()
            .take_while(|seg| seg.span.start() < stop_off)
        {
            if !self.static_statement_falls_through(&seg, depth) {
                return false;
            }
        }
        true
    }

    /// Prove that a statically declared procedure call completes normally.
    /// `return` is normal completion at the caller, while registry-declared
    /// non-normal completions and unresolved statements make the proof
    /// abstain. Calls through a recorded class-factory oracle are ordinary
    /// fallthrough statements when their exported manufacturer layout is
    /// valid; no factory command spelling is known here.
    fn static_user_proc_call_falls_through(
        &mut self,
        caller: &str,
        seg: &SegmentedCommand,
        depth: u32,
        stack: &mut Vec<String>,
    ) -> Option<bool> {
        if depth >= MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let proc_def = if self.result.allows_lexical_declaration_advice() {
            let qname = self.resolve_static_proc_name(caller, seg.name())?;
            self.result.all_procs.get(&qname)?.clone()
        } else {
            self.original_static_procedure_at(seg)?
        };
        let qname = proc_def.qualified_name.clone();
        if stack.iter().any(|active| active == &qname) {
            return None;
        }
        if proc_def.params_computed {
            return None;
        }
        let actuals: Vec<Option<String>> = seg.args().iter().cloned().map(Some).collect();
        bind_proc_formals(&proc_def.params, &actuals, self.word_rules())?;
        stack.push(qname.clone());
        let result = self.static_proc_body_falls_through(&qname, proc_def.body_span, depth, stack);
        stack.pop();
        result
    }

    pub(in super::super) fn static_proc_body_falls_through(
        &mut self,
        qname: &str,
        body_span: Span,
        depth: u32,
        stack: &mut Vec<String>,
    ) -> Option<bool> {
        let context = self.retained_static_source_context()?;
        let registry = context.commands();
        for seg in self.direct_statements_in_span(body_span)? {
            match self.segment_invocation_completion(&seg) {
                tcl_registry::registry::InvocationCompletion::ReturnsResult(_) => {
                    return Some(true);
                }
                tcl_registry::registry::InvocationCompletion::Terminates => return Some(false),
                tcl_registry::registry::InvocationCompletion::Unknown => {
                    if registry.get(seg.name()).is_some() {
                        return None;
                    }
                    if self.static_user_proc_call_falls_through(qname, &seg, depth + 1, stack)? {
                        continue;
                    }
                    return Some(false);
                }
                tcl_registry::registry::InvocationCompletion::FallsThrough => {
                    if !self.static_statement_falls_through(&seg, depth + 1) {
                        return None;
                    }
                }
            }
        }
        Some(true)
    }

    pub(in super::super) fn static_statement_falls_through(
        &mut self,
        seg: &SegmentedCommand,
        depth: u32,
    ) -> bool {
        if depth >= MAX_UNKNOWN_BODY_DEPTH {
            return false;
        }
        let Some(context) = self.retained_static_source_context() else {
            return false;
        };
        let registry = context.commands().as_ref();
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        match self.segment_invocation_completion(seg) {
            tcl_registry::registry::InvocationCompletion::FallsThrough => {}
            tcl_registry::registry::InvocationCompletion::Unknown
                if registry.get(seg.name()).is_none() =>
            {
                return self
                    .static_user_proc_call_falls_through("", seg, depth + 1, &mut Vec::new())
                    .unwrap_or(false);
            }
            _ => return false,
        }
        let is_control = registry
            .invocation_traits(seg.name(), &args, Some(context.context().authoring_query()))
            .contains(tcl_registry::Traits::CONTROL_FLOW);
        if is_control
            && registry.control_invocation_valid(
                seg.name(),
                &args,
                Some(context.context().authoring_query()),
            ) != Some(true)
        {
            return false;
        }
        let controls = self.control_arms_for_segment(seg);
        if !controls.complete {
            return false;
        }
        let arms = controls.arms;
        if arms.is_empty() {
            return true;
        }

        let mut selected = Vec::new();
        let mut saw_typed_arm = false;
        let mut saw_propagating_arm = false;
        for arm in arms {
            let Some(semantics) = self.control_arm_semantics_for(&arm) else {
                if is_control {
                    return false;
                }
                // Untyped is not the same as irrelevant. The
                // registry types the arms whose *selection* it models; a body
                // it does not type is still a body, and unless the command
                // declared that it stores one (`DEFERS_BODY`), it runs it
                // here — so `uplevel 1 {error stop}`, `eval {error stop}` and
                // `oo::define C {error stop}` cannot be walked past on the
                // strength of having no descriptor.
                let body_runs_now = self
                    .control_arm_body_index(&arm)
                    .is_none_or(|index| self.body_runs_now(&arm.controller, index));
                if body_runs_now && self.untyped_body_provably_blocks(arm.body_span, depth + 1) {
                    return false;
                }
                continue;
            };
            saw_typed_arm = true;
            match semantics {
                tcl_registry::registry::ControlArmSemantics::Always
                | tcl_registry::registry::ControlArmSemantics::FrameBoundary => {
                    saw_propagating_arm = true;
                    if !self.static_prefix_falls_through(
                        arm.body_span,
                        arm.body_span.end(),
                        depth + 1,
                    ) {
                        return false;
                    }
                }
                tcl_registry::registry::ControlArmSemantics::Selected => selected.push(arm),
                tcl_registry::registry::ControlArmSemantics::CompletionBoundary => {}
                // A collection loop runs its body a finite number of times, so
                // control reaches the next statement exactly when the body
                // falls through — the same question, and the same answer, as
                // an `if` arm whose selection is unknown (below). Nothing is
                // *read* out of the loop: it stays unselectable, and the
                // provenance walk still clears every fact the iteration
                // variables or the body could write.
                tcl_registry::registry::ControlArmSemantics::BoundedIteration => {
                    if !self.static_prefix_falls_through(
                        arm.body_span,
                        arm.body_span.end(),
                        depth + 1,
                    ) {
                        return false;
                    }
                }
                // A condition loop's trip count is not bounded by anything
                // readable here — it may run forever, so no claim is made
                // about the statement after it.
                tcl_registry::registry::ControlArmSemantics::Uncertain => return false,
            }
        }
        if !saw_typed_arm || selected.is_empty() {
            return true;
        }
        if saw_propagating_arm {
            return false;
        }

        self.selected_arms_fall_through(&selected, depth)
    }

    /// The `Selected`-arm half of [`Self::static_statement_falls_through`]:
    /// at most one arm of an `if` / `switch` runs, so control reaches the
    /// next statement when *that* arm does. An arm whose selection cannot be
    /// decided is treated as one that may run, which is the same requirement
    /// a [`ControlArmSemantics::BoundedIteration`] body carries.
    ///
    /// [`ControlArmSemantics::BoundedIteration`]: tcl_registry::registry::ControlArmSemantics::BoundedIteration
    fn selected_arms_fall_through(&mut self, selected: &[ControlArm], depth: u32) -> bool {
        let env = std::collections::HashMap::new();
        let mut selected_body = None;
        let mut selection_unknown = false;
        for arm in selected {
            match self.control_body_is_selected("", arm, &env, depth + 1) {
                Some(true) if selected_body.is_none() => selected_body = Some(arm.body_span),
                Some(true) => return false,
                Some(false) => {}
                None => {
                    selection_unknown = true;
                    if !self.static_prefix_falls_through(
                        arm.body_span,
                        arm.body_span.end(),
                        depth + 1,
                    ) {
                        return false;
                    }
                }
            }
        }
        if selection_unknown {
            return true;
        }
        selected_body
            .is_none_or(|span| self.static_prefix_falls_through(span, span.end(), depth + 1))
    }
}
