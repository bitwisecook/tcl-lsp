// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conditional Normal string-method closure from selected original producers.
//! This is no cache, contents, numeric, completion, or runtime getter authority.

use super::{
    Arc, ModuleCommandBindings, SourceExecutionContext, SourceNativeInvocation, SourceOutcomes,
};
use crate::ir::WordPart;

/// Effect-only result class. Constructors remain in the actual producer module;
/// ownership/currentness stays in the frozen outcome and exact stored cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceNativeStringAccess {
    owners: Vec<super::literal_object_pool::SourceOrdinaryLiteralPool>,
}

impl SourceNativeStringAccess {
    fn from_normal(normal: &ModuleCommandBindings, inputs: &[Arc<Self>]) -> Option<Arc<Self>> {
        let owner = normal.ordinary_literal_pool.as_ref()?;
        if !normal
            .current_source_origin
            .as_ref()
            .is_some_and(|source| owner.effects_current_for(source))
        {
            return None;
        }
        if inputs.iter().any(|input| !input.is_current()) {
            return None;
        }
        let mut owners = Vec::new();
        for source in
            std::iter::once(owner).chain(inputs.iter().flat_map(|input| input.owners.iter()))
        {
            if !owners.iter().any(|existing| existing == source) {
                owners.push(source.clone());
            }
        }
        Some(Arc::new(Self { owners }))
    }

    pub(crate) fn is_current(&self) -> bool {
        !self.owners.is_empty()
            && self
                .owners
                .iter()
                .all(super::literal_object_pool::SourceOrdinaryLiteralPool::effects_current)
    }
}

pub(super) struct TemplateInputs {
    epoch: Option<u64>,
    closed: bool,
    inputs: Vec<Arc<SourceNativeStringAccess>>,
}
impl TemplateInputs {
    pub(super) fn new(state: &ModuleCommandBindings) -> Self {
        Self {
            epoch: state.source_variables.representation_epoch,
            closed: true,
            inputs: Vec::new(),
        }
    }
    pub(super) fn observe(
        &mut self,
        part: &WordPart,
        next: &SourceOutcomes,
        state: &ModuleCommandBindings,
    ) {
        self.closed &= template_part_closed(part, next, state)
            && next
                .normal
                .as_ref()
                .is_some_and(|normal| normal.source_variables.representation_epoch == self.epoch);
        if self.closed
            && let Some(receipt) = next
                .normal_representation
                .as_ref()
                .and_then(|frozen| frozen.native_string_effect_receipt())
        {
            self.inputs.push(Arc::clone(receipt));
        }
    }
    pub(super) fn finish(
        self,
        outcomes: &SourceOutcomes,
        original_word: bool,
    ) -> Option<Arc<super::source_representation::FrozenSourceRepresentation>> {
        template_result(
            outcomes,
            original_word,
            self.closed,
            self.epoch,
            &self.inputs,
        )
    }
}

pub(super) fn template_part_closed(
    part: &WordPart,
    next: &SourceOutcomes,
    state: &ModuleCommandBindings,
) -> bool {
    match part {
        WordPart::Text { .. } => state
            .ordinary_literal_pool
            .as_ref()
            .is_some_and(super::literal_object_pool::SourceOrdinaryLiteralPool::effects_current),
        WordPart::Variable { .. } | WordPart::CommandSubstitution { .. } => next
            .normal
            .as_ref()
            .zip(next.normal_representation.as_ref())
            .is_some_and(|(normal, receipt)| {
                receipt.native_string_result_input_is_closed(&normal.source_variables)
            }),
        WordPart::Opaque { .. } => false,
    }
}

fn template_result(
    outcomes: &SourceOutcomes,
    original_word: bool,
    inputs_closed: bool,
    epoch: Option<u64>,
    inputs: &[Arc<SourceNativeStringAccess>],
) -> Option<Arc<super::source_representation::FrozenSourceRepresentation>> {
    let normal = outcomes.normal.as_ref()?;
    if !original_word
        || !inputs_closed
        || epoch.is_none()
        || normal.source_variables.representation_epoch != epoch
        || normal
            .baseline
            .dialect
            .and_then(|dialect| dialect.native_string_protocol())
            .and_then(|protocol| protocol.tcl_version())
            .is_none()
        || normal
            .ordinary_literal_pool
            .as_ref()
            .is_none_or(|pool| !pool.effects_current())
    {
        return None;
    }
    // The original concat may retain any original header on an empty/sole-input
    // branch. Every such input is closed; no fresh-header/type claim is made.
    let receipt = SourceNativeStringAccess::from_normal(normal, inputs)?;
    super::source_representation::FrozenSourceRepresentation::capture_native_string(
        receipt,
        &normal.source_variables,
    )
    .map(Arc::new)
}

fn append_handler_selected(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    before: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    use tcl_registry::native_compilation::{
        NativeAppendKind, NativeCompilationGrammar, NativeCompilationMode,
        NativeCompilationSelection, SuccessfulHandlerSpec,
    };
    let Some(spec) = native.compilation_spec else {
        return false;
    };
    if !native.target.registry_backed
        || !native.target.prepended.is_empty()
        || native.target.kind != super::BindingKind::Builtin
        || facts.successful_handler != Some(SuccessfulHandlerSpec::VariableStringAppend)
        || spec.grammar != NativeCompilationGrammar::VariableAppend(NativeAppendKind::String)
        || spec.operation != facts.operation
        || before
            .baseline
            .dialect
            .and_then(|dialect| dialect.native_object_append_protocol(None))
            .is_none()
    {
        return false;
    }
    match native.compilation_selection {
        NativeCompilationSelection::Inline { operation, .. } => *operation == facts.operation,
        NativeCompilationSelection::Generic => {
            context.compilation().mode == NativeCompilationMode::Direct
        }
        _ => false,
    }
}

pub(super) fn retain_append(
    outcomes: &mut SourceOutcomes,
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    before: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) {
    if !append_handler_selected(native, facts, before, context) {
        return;
    }
    if super::native_result::numeric_call_arguments(context, native.target).is_none() {
        return;
    }
    let arguments = native.invocation.arguments();
    let Some(count) = arguments.exact_argv_len() else {
        return;
    };
    let from = facts.argument_offset;
    if count < from + 2 || native.words.len() != count + 1 {
        return;
    }
    let Some(name) = arguments.literal_at(from) else {
        return;
    };
    let receiver = crate::var_resolve::resolve_literal_access(
        name,
        &before.source_variables,
        false,
        context.registry,
        tcl_registry::TraceOperation::Read,
    );
    let writer = crate::var_resolve::resolve_literal_access(
        name,
        &before.source_variables,
        false,
        context.registry,
        tcl_registry::TraceOperation::Write,
    );
    if receiver.dynamic
        || receiver.observed
        || writer.dynamic
        || writer.observed
        || receiver.cell != writer.cell
        || receiver.index != writer.index
        || !before
            .source_variables
            .contents_native_string_result_input_closed_at(&receiver, context.registry)
    {
        return;
    }
    let Some(representations) = context.written_representations else {
        return;
    };
    if !(from + 1..count).all(|argument| {
        representations
            .get(argument + 1)
            .and_then(Option::as_ref)
            .is_some_and(|receipt| {
                receipt.native_string_result_input_is_closed(&before.source_variables)
            })
    }) {
        return;
    }
    let mut inputs = Vec::new();
    if let Some(receipt) = before
        .source_variables
        .contents_native_string_access_receipt_at(&receiver, context.registry)
    {
        inputs.push(Arc::clone(receipt));
    }
    for argument in from + 1..count {
        if let Some(receipt) = representations[argument + 1]
            .as_ref()
            .and_then(|frozen| frozen.native_string_effect_receipt())
        {
            inputs.push(Arc::clone(receipt));
        }
    }
    publish_append(outcomes, before, context, name, &writer, &inputs);
}

fn publish_append(
    outcomes: &mut SourceOutcomes,
    before: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    name: &str,
    writer: &crate::place::Place,
    inputs: &[Arc<SourceNativeStringAccess>],
) {
    let Some(normal) = outcomes.normal.as_mut() else {
        return;
    };
    let after = crate::var_resolve::resolve_literal_access(
        name,
        &normal.source_variables,
        false,
        context.registry,
        tcl_registry::TraceOperation::Write,
    );
    if after.dynamic
        || after.observed
        || after.cell != writer.cell
        || after.index != writer.index
        || before.source_variables.representation_epoch.is_none()
        || normal.source_variables.representation_epoch
            != before.source_variables.representation_epoch
        || normal.current_source_origin != before.current_source_origin
        || normal.source_variables.contents_presence(&after)
            != crate::var_resolve::ContentsPresence::Defined
        || normal.source_variables.contents_origin(&after)
            != crate::var_resolve::ContentsOrigin::WrittenAt(context.invocation_offset)
        || normal
            .current_source_origin
            .as_ref()
            .is_none_or(|source| !normal.source_variables.contents_have_source(&after, source))
    {
        return;
    }
    let Some(key) = crate::var_resolve::canonical_binding_value_key(&after) else {
        return;
    };
    let Some(receipt) = SourceNativeStringAccess::from_normal(normal, inputs) else {
        return;
    };
    let Some(frozen) =
        super::source_representation::FrozenSourceRepresentation::capture_native_string(
            Arc::clone(&receipt),
            &normal.source_variables,
        )
    else {
        return;
    };
    Arc::make_mut(&mut normal.source_variables)
        .value_representations
        .insert(
            key,
            crate::native_numeric::StoredNativeRepresentation::NativeStringAccess(receipt),
        );
    outcomes.normal_representation = Some(Arc::new(frozen));
}

#[cfg(test)]
mod tests;
