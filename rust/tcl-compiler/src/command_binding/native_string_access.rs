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
    fn current_owner<'a>(
        normal: &'a ModuleCommandBindings,
        inputs: &[Arc<Self>],
    ) -> Option<&'a super::literal_object_pool::SourceOrdinaryLiteralPool> {
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
        Some(owner)
    }

    fn from_normal(normal: &ModuleCommandBindings, inputs: &[Arc<Self>]) -> Option<Arc<Self>> {
        let owner = Self::current_owner(normal, inputs)?;
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
            .and_then(tcl_registry::InvocationDialect::native_string_protocol)
            .and_then(tcl_syntax::native_string::NativeStringProtocol::tcl_version)
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

/// Entered append-specific effect transport. All donor alternatives are
/// retained before the existing getter/coercion preparation withdraws caches.
/// The snapshot is Rust analysis ownership, not Native header ownership.
pub(super) struct CapturedAppend {
    before: ModuleCommandBindings,
    name: String,
    writer: crate::place::Place,
    inputs: Vec<Arc<SourceNativeStringAccess>>,
    prepared_epoch: Option<u64>,
    value: Option<append_value::CapturedAppendValue>,
}

fn append_observers_quiet(
    state: &ModuleCommandBindings,
    native: SourceNativeInvocation<'_>,
) -> bool {
    !state.has_opaque_domain()
        && !state.source_step_observed()
        && state.command_observers.is_quiet()
        && !state.source_execution_observed(native.target.identity.as_ref())
        && !state.runtime_execution_observed(native.target.identity.as_ref())
}

impl CapturedAppend {
    pub(super) fn after_preparation(
        &mut self,
        state: &ModuleCommandBindings,
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        context: SourceExecutionContext<'_>,
        object_protocol: super::object_callbacks::ObjectInputProtocol,
    ) {
        self.prepared_epoch = None;
        // This method is reached once, only after this selected stock handler's
        // unchanged preparation returned. Closed original input methods and
        // quiet observers exclude arbitrary reentry; representation coercion
        // still withdraws every cache exactly as before.
        if !matches!(
            object_protocol,
            super::object_callbacks::ObjectInputProtocol::Unknown
        ) && append_handler_selected(native, facts, state, context)
            && append_observers_quiet(state, native)
            && state.current_source_origin == self.before.current_source_origin
            && state.original_command_world == self.before.original_command_world
            && state.variable_frame == self.before.variable_frame
            && self.inputs.iter().all(|input| input.is_current())
            && SourceNativeStringAccess::current_owner(state, &self.inputs).is_some()
        {
            self.prepared_epoch = state.source_variables.representation_epoch;
            if self
                .value
                .as_ref()
                .is_some_and(|value| !value.before_write_is_current(state, context.registry))
            {
                self.value = None;
            }
        }
    }

    pub(super) fn finish(self, outcomes: &mut SourceOutcomes, context: SourceExecutionContext<'_>) {
        if let Some(epoch) = self.prepared_epoch {
            let after = publish_append(
                outcomes,
                &self.before,
                context,
                &self.name,
                &self.writer,
                &self.inputs,
                epoch,
            );
            if let (Some(value), Some(after)) = (self.value, after) {
                value.publish(outcomes, &self.before, context, &after);
            }
        }
    }
}

pub(super) fn capture_append(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    before: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<CapturedAppend> {
    if !append_handler_selected(native, facts, before, context)
        || !append_observers_quiet(before, native)
    {
        return None;
    }
    super::native_result::numeric_call_arguments(context, native.target)?;
    let arguments = native.invocation.arguments();
    let count = arguments.exact_argv_len()?;
    let from = facts.argument_offset;
    if count < from + 2 || native.words.len() != count + 1 {
        return None;
    }
    let name = arguments.literal_at(from)?;
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
        return None;
    }
    let representations = context.written_representations?;
    if !(from..count).all(|argument| {
        representations
            .get(argument + 1)
            .and_then(Option::as_ref)
            .is_some_and(|receipt| {
                receipt.native_string_result_input_is_closed(&before.source_variables)
            })
    }) {
        return None;
    }
    let mut inputs = Vec::new();
    if let Some(receipt) = before
        .source_variables
        .contents_native_string_access_receipt_at(&receiver, context.registry)
    {
        inputs.push(Arc::clone(receipt));
    }
    for argument in from..count {
        if let Some(receipt) = representations[argument + 1]
            .as_ref()
            .and_then(|frozen| frozen.native_string_effect_receipt())
        {
            inputs.push(Arc::clone(receipt));
        }
    }
    Some(CapturedAppend {
        before: before.clone(),
        name: name.to_owned(),
        value: append_value::CapturedAppendValue::capture(before, &receiver, context, from, count),
        writer,
        inputs,
        prepared_epoch: None,
    })
}

fn publish_append(
    outcomes: &mut SourceOutcomes,
    before: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    name: &str,
    writer: &crate::place::Place,
    inputs: &[Arc<SourceNativeStringAccess>],
    prepared_epoch: u64,
) -> Option<crate::place::Place> {
    let normal = outcomes.normal.as_mut()?;
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
        || normal.source_variables.representation_epoch != Some(prepared_epoch)
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
        return None;
    }
    let key = crate::var_resolve::canonical_binding_value_key(&after)?;
    let receipt = SourceNativeStringAccess::from_normal(normal, inputs)?;
    let frozen = super::source_representation::FrozenSourceRepresentation::capture_native_string(
        Arc::clone(&receipt),
        &normal.source_variables,
    )?;
    Arc::make_mut(&mut normal.source_variables)
        .value_representations
        .insert(
            key,
            crate::native_numeric::StoredNativeRepresentation::NativeStringAccess(receipt),
        );
    outcomes.normal_representation = Some(Arc::new(frozen));
    Some(after)
}

mod append_value;

#[cfg(test)]
mod tests;
