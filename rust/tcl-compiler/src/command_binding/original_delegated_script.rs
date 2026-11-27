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

//! A script-evaluating handler given one exact script.
//!
//! `eval {script}` and `uplevel ?level? {script}` do nothing of their own
//! beyond running that script in the selected frame, so the handler's
//! footprint is the script's, which the common body walker accounts for
//! command by command. A script built by one substitution of literal words
//! (`eval [list set a 1]`) is admitted the same way once its value is exact.
//! Other frozen script operands require an independently closed stock world
//! and the exact original operand bytes.

use super::{
    Arc, ModuleCommandBindings, SourceCommandTarget, SourceExecutionContext, SourceNativeInvocation,
};
use tcl_names::command::CommandAllocationSite;
use tcl_names::namespace::SourceNamespaceKey;

/// The exact script handed to the body walker and the frame it ran
/// in. Completion is checked only after the walk and parent restoration.
pub(super) struct OriginalDelegatedScript {
    site: CommandAllocationSite,
    words: Vec<tcl_source::words::WordExpr>,
    config: tcl_lexer::LexerConfig,
    handler: SourceCommandTarget,
    body: tcl_names::origin::ExecutedScriptSource,
    argument: usize,
    list_tail: bool,
    namespace: SourceNamespaceKey,
    entered: tcl_names::variable::VariableExecutionFrame,
    parent: tcl_names::variable::VariableExecutionFrame,
}

/// Selected existing namespace in the stock C command world. The effective
/// operand owns the counted address; this operation never creates a holder.
pub(super) fn existing_namespace(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<SourceNamespaceKey> {
    let (argument, body) = facts.existing_namespace_script?;
    if !super::original_namespace_body::closed_original_world(state)
        || !native.target.registry_backed
        || native.target.kind != tcl_names::command::BindingKind::Builtin
        || native.target.implementation_generation != 0
        || !state.retained_target_is_current(native.target)
        || facts.arity_accepts_frozen_arguments() != Some(true)
        || !matches!(
            native
                .invocation
                .arguments()
                .dialect()?
                .native_string_protocol()?,
            tcl_syntax::native_string::NativeStringProtocol::C(_)
        )
    {
        return None;
    }
    retained_body(native, state, context, body, true)?;
    let input = native
        .original_variable_operands
        .input(argument, &state.source_variables)?;
    state.original_namespace_key_for_input(&context.namespace_identity(), input)
}

/// Assemble a selected C namespace script with extra words as list elements.
/// An exact one-word script instead retains its original form and mapping.
pub(super) fn append_list_tail(values: &[&str]) -> Option<String> {
    let (script, tail) = values.split_first()?;
    if values
        .iter()
        .any(|text| !text.is_ascii() || text.as_bytes().contains(&0))
    {
        return None;
    }
    let extra = tcl_syntax::list::join_list(tail.iter().copied());
    Some(tcl_syntax::list::concat_values([*script, extra.as_str()]))
}

fn retained_body(
    native: SourceNativeInvocation<'_>,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    argument: usize,
    list_tail: bool,
) -> Option<tcl_names::origin::ExecutedScriptSource> {
    let arguments = native.invocation.arguments();
    let count = arguments.exact_argv_len()?;
    if !list_tail || count == argument.checked_add(1)? {
        let script = super::retained_script_operand(
            argument,
            native.script_operands(),
            state,
            native.segment.span.start(),
            context.config,
        )?;
        if list_tail
            && native
                .original_variable_operands
                .input(argument, &state.source_variables)?
                .bytes()
                != script.text.bytes()
        {
            return None;
        }
        return Some(script);
    }
    let values = (argument..count)
        .map(|index| {
            let input = native
                .original_variable_operands
                .input(index, &state.source_variables)?;
            let value = arguments.literal_at(index)?;
            (input.bytes() == value.as_bytes()).then_some(value)
        })
        .collect::<Option<Vec<_>>>()?;
    let script = append_list_tail(&values)?;
    let parent = CommandAllocationSite {
        source: Arc::clone(state.current_source_origin.as_ref()?),
        offset: native.segment.span.start(),
    };
    Some(tcl_names::origin::ExecutedScriptSource::materialised(
        parent,
        (argument..count).collect(),
        &script,
    ))
}

/// The script operand and its retained text, when the selected handler is
/// the stock script evaluator and its one script operand is closed.
fn literal_script(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<(usize, tcl_names::origin::ExecutedScriptSource)> {
    use tcl_registry::script_body_flow::ScriptBodyFlow;
    if !super::original_namespace_body::quiet(state)
        || facts.arity_accepts_frozen_arguments() != Some(true)
        || !facts.traits.contains(tcl_registry::Traits::EVALUATES_CODE)
        // A namespace body has its own receipt, with its Ensure geometry.
        || (facts.analyser_hook == Some(tcl_registry::hooks::AnalyserHookId::NamespaceEval)
            && facts.existing_namespace_script.is_none())
        || !native.target.registry_backed
        || native.target.kind != tcl_names::command::BindingKind::Builtin
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
    if facts.existing_namespace_script.is_some() {
        existing_namespace(native, facts, state, context)?;
        let body = retained_body(native, state, context, argument_offset, true)?;
        return Some((argument_offset, body));
    }
    if arguments.exact_argv_len()? != argument_offset.checked_add(1)? {
        return None;
    }
    // Every word before the script (a frame level) is a written literal.
    if (0..argument_offset).any(|index| arguments.literal_at(index).is_none())
        || (argument_offset != 0 && facts.frame_effect.is_none())
    {
        return None;
    }
    let written = native.script_operands().written_word(argument_offset)?;
    let body = super::retained_script_operand(
        argument_offset,
        native.script_operands(),
        state,
        native.segment.span.start(),
        context.config,
    )?;
    let literal = matches!(written, tcl_source::words::WordExpr::BracedLiteral { .. })
        || super::literal_word_script::is_exact_literal_word_script(
            native,
            argument_offset,
            context.config,
        );
    if !literal {
        // The closed stock world excludes object methods that can invoke Tcl
        // while the selected evaluator prepares the frozen script object.
        if !super::original_namespace_body::closed_original_world(state)
            || !matches!(
                arguments.dialect()?.native_string_protocol()?,
                tcl_syntax::native_string::NativeStringProtocol::C(_)
            )
        {
            return None;
        }
        let input = native
            .original_variable_operands
            .input(argument_offset, &state.source_variables)?;
        if input.bytes() != body.text.bytes() {
            return None;
        }
    }
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
            list_tail: facts.existing_namespace_script.is_some(),
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
            && retained_body(native, normal, context, self.argument, self.list_tail).as_ref()
                == Some(&self.body)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn namespace_script_appends_tail_as_list_elements() {
        assert_eq!(
            super::append_list_tail(&["  helper ", "a b", "$notSubstituted"]).as_deref(),
            Some("helper {a b} {$notSubstituted}"),
        );
    }
}
