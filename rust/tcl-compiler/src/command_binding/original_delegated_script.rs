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

//! A script-evaluating handler given one braced literal script.
//!
//! `eval {script}` and `uplevel ?level? {script}` do nothing of their own
//! beyond running that script in the selected frame, so the handler's
//! footprint is the script's, which the common body walker accounts for
//! command by command. A computed script or level keeps the conservative
//! footprint: nothing here admits it.

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandTarget, SourceExecutionContext,
    SourceNamespaceKey, SourceNativeInvocation,
};

/// The exact literal script handed to the body walker and the frame it ran
/// in. Completion is checked only after the walk and parent restoration.
pub(super) struct OriginalDelegatedScript {
    site: CommandAllocationSite,
    words: Vec<crate::ir::WordExpr>,
    config: tcl_lexer::LexerConfig,
    handler: SourceCommandTarget,
    body: super::ExecutedScriptSource,
    argument: usize,
    namespace: SourceNamespaceKey,
    entered: crate::var_resolve::VariableExecutionFrame,
    parent: crate::var_resolve::VariableExecutionFrame,
}

/// The script operand and its retained text, when the selected handler is
/// the stock script evaluator and its one script word is a braced literal.
fn literal_script(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<(usize, super::ExecutedScriptSource)> {
    use tcl_registry::script_body_flow::ScriptBodyFlow;
    if !super::original_namespace_body::quiet(state)
        || facts.arity_accepts_frozen_arguments() != Some(true)
        || !facts.traits.contains(tcl_registry::Traits::EVALUATES_CODE)
        // A namespace body has its own receipt, with its Ensure geometry.
        || facts.analyser_hook == Some(tcl_registry::hooks::AnalyserHookId::NamespaceEval)
        || !native.target.registry_backed
        || native.target.kind != super::BindingKind::Builtin
        || native.target.implementation_generation != 0
        || native.target.runtime_implementation_generation.is_some()
        || !native.target.prepended.is_empty()
        || !state.retained_target_is_current(native.target)
    {
        return None;
    }
    let ScriptBodyFlow::ConcatenatedScript { argument_offset } =
        tcl_registry::case_bodies::script_body_flow_in_registry(
            context.registry,
            facts,
            native.invocation.arguments(),
        )
    else {
        return None;
    };
    let arguments = native.invocation.arguments();
    if arguments.exact_argv_len()? != argument_offset.checked_add(1)? {
        return None;
    }
    // Every word before the script (a frame level) is a written literal.
    if (0..argument_offset).any(|index| arguments.literal_at(index).is_none())
        || (argument_offset != 0 && facts.frame_effect.is_none())
    {
        return None;
    }
    let crate::ir::WordExpr::BracedLiteral { .. } =
        native.script_operands().written_word(argument_offset)?
    else {
        return None;
    };
    let body = super::retained_script_operand(
        argument_offset,
        native.script_operands(),
        state,
        native.segment.span.start(),
        context.config,
    )?;
    Some((argument_offset, body))
}

/// Whether the handler's own footprint is exactly its literal script's.
pub(super) fn has_delegated_literal_script(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    literal_script(native, facts, state, context).is_some()
}

/// Whether the normal return of a delegated literal script leaves a closed
/// command world. A possible error inside the script is a separate path; it
/// never clears a world the script itself left unclosed.
pub(super) fn world_closed_on_normal_return(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    incoming: &ModuleCommandBindings,
    outcomes: &super::SourceOutcomes,
    context: SourceExecutionContext<'_>,
) -> bool {
    use tcl_registry::completion_route::InvocationCompletionRoute as Route;
    if super::requires_reentry_guard(facts)
        || !has_delegated_literal_script(native, facts, incoming, context)
        || !outcomes.abrupt.iter().all(|(route, _)| {
            matches!(
                route,
                Route::Tcl(tcl_registry::completion::CompletionCode::Error)
            )
        })
    {
        return false;
    }
    outcomes.normal.as_deref().is_some_and(|normal| {
        !normal.unclosed_command_world
            && super::original_namespace_body::quiet(normal)
            && !normal.original_command_world.is_withdrawn()
            && normal.unknown_handler_is_original()
    })
}

impl OriginalDelegatedScript {
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        prepared: &super::PreparedSourceNativeBody,
    ) -> Option<Self> {
        if state.baseline.source_point_entry.is_some() {
            return None;
        }
        let (argument, body) = literal_script(native, facts, state, context)?;
        Some(Self {
            site: CommandAllocationSite {
                source: Arc::clone(state.current_source_origin.as_ref()?),
                offset: native.segment.span.start(),
            },
            words: native.words.to_vec(),
            config: context.config,
            handler: native.target.clone(),
            body,
            argument,
            namespace: prepared.selection.namespace_key.clone(),
            entered: prepared.selection.frame.clone(),
            parent: prepared.parent_frame.clone(),
        })
    }

    pub(super) fn completed(
        &self,
        native: SourceNativeInvocation<'_>,
        outcomes: &super::SourceOutcomes,
        context: SourceExecutionContext<'_>,
        prepared: &super::PreparedSourceNativeBody,
    ) -> bool {
        let Some(normal) = outcomes
            .normal
            .as_deref()
            .filter(|_| outcomes.normal_completion.is_some() && outcomes.abrupt.is_empty())
        else {
            return false;
        };
        super::original_namespace_body::quiet(normal)
            && self.site.offset == native.segment.span.start()
            && normal.current_source_origin.as_ref() == Some(&self.site.source)
            && self.words == native.words
            && self.config == context.config
            && self.handler == *native.target
            && normal.retained_target_is_current(native.target)
            && self.entered == prepared.selection.frame
            && self.namespace == prepared.selection.namespace_key
            && self.parent == prepared.parent_frame
            && normal.variable_frame == self.parent
            && normal.source_variables.namespace_identity
                == prepared.parent_variables.namespace_identity
            && super::retained_script_operand(
                self.argument,
                native.script_operands(),
                normal,
                native.segment.span.start(),
                context.config,
            )
            .as_ref()
                == Some(&self.body)
    }
}
