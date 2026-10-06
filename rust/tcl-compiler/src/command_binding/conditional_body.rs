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

//! Conditional declaration entry, independently of actual script execution.

use super::{Arc, CommandAllocation, ExecutedScriptSource, SourceCommandBindings};

/// A native receiver-body declaration's original local scope. Its source
/// recipe and formals do not allocate a method implementation or receiver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDeclaredReceiverBodyEntry {
    declaration: super::CommandAllocationSite,
    source: ExecutedScriptSource,
    declaration_namespace: super::SourceNamespaceKey,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    preview_frame: crate::var_resolve::VariableExecutionFrame,
}

impl SourceDeclaredReceiverBodyEntry {
    /// Exact original declaration allocation that owns this receiver-body scope.
    #[must_use]
    pub fn declaration(&self) -> &super::CommandAllocationSite {
        &self.declaration
    }

    /// Retained original body source and its mapping to the declaration image.
    #[must_use]
    pub fn source(&self) -> &ExecutedScriptSource {
        &self.source
    }

    pub(super) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        &self.parameters
    }

    pub(super) fn preview_frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.preview_frame
    }

    /// Original declaration context, independently of the unknown namespace
    /// of a future runtime receiver. This supplies no receiver or method entry.
    #[must_use]
    pub fn declaration_namespace_context(&self) -> &super::SourceNamespaceKey {
        &self.declaration_namespace
    }
}

/// An original call's scoped caller-name instantiation. This is navigation
/// evidence only: a write in the callee template proves no completed store,
/// allocated caller cell, value or absence of observers.
#[derive(Debug, Clone)]
pub struct SourceCallerFrameInvocationTemplate {
    call: super::CommandAllocationSite,
    frame: Option<crate::var_resolve::VariableExecutionFrame>,
    namespace_context: Option<super::SourceNamespaceKey>,
    declared_receiver: Option<Arc<SourceDeclaredReceiverBodyEntry>>,
    procedure: Option<super::SourceCommandReference>,
    declared_procedure: Option<super::SourceCommandDefinition>,
    method: Option<super::SourceReceiverMethodEntry>,
    parameter_arguments: Vec<Option<(usize, String)>>,
}

impl SourceCallerFrameInvocationTemplate {
    /// Original call allocation; an unpositioned or final-name lookup cannot
    /// construct this receipt.
    #[must_use]
    pub fn call(&self) -> &super::CommandAllocationSite {
        &self.call
    }

    /// Original procedure-frame recipe or retained entered frame. A recipe
    /// supplies no physical caller cells. Declaration-only receiver scopes
    /// supply no method frame or allocated receiver through this getter.
    #[must_use]
    pub fn frame(&self) -> Option<&crate::var_resolve::VariableExecutionFrame> {
        self.frame.as_ref()
    }

    /// Original procedure or retained actual caller namespace. An unentered
    /// receiver declaration exposes only its separate declaration context.
    #[must_use]
    pub fn namespace_context(&self) -> Option<&super::SourceNamespaceKey> {
        self.namespace_context.as_ref()
    }

    /// Original declared receiver-body scope for conditional caller navigation.
    /// This scope supplies neither an allocated receiver nor actual method entry.
    #[must_use]
    pub fn declared_receiver_body(&self) -> Option<&SourceDeclaredReceiverBodyEntry> {
        self.declared_receiver.as_deref()
    }

    /// Selected procedure declaration, including separately retained alias or
    /// import traversal. The implementation allocation is validated too.
    #[must_use]
    pub fn procedure(&self) -> Option<&super::SourceCommandReference> {
        self.procedure.as_ref()
    }

    /// Original procedure allocation selected for this conditional navigation
    /// template. It grants neither actual dispatch nor a completed caller write.
    #[must_use]
    pub fn procedure_definition(&self) -> Option<&super::SourceCommandDefinition> {
        self.procedure
            .as_ref()
            .and_then(|reference| {
                reference
                    .linked_definition()
                    .or_else(|| reference.definition())
            })
            .or(self.declared_procedure.as_ref())
    }

    /// Original method selected through the retained receiver dispatcher.
    #[must_use]
    pub fn method(&self) -> Option<&super::SourceReceiverMethodEntry> {
        self.method.as_ref()
    }

    /// A formal's literal original operand and its post-head source index.
    /// Captured prefixes, dynamic values and expansions have no editable
    /// literal call-site operand and therefore withdraw this projection.
    #[must_use]
    pub fn literal_parameter_argument(&self, parameter: usize) -> Option<(usize, &str)> {
        self.parameter_arguments
            .get(parameter)?
            .as_ref()
            .map(|(index, value)| (*index, value.as_str()))
    }
}

// The effective-word owner supplies all prefix, selector and expansion
// mappings. A template argument additionally requires an original literal.
fn literal_parameter_arguments(
    tokens: &crate::ir::CommandTokens,
    binding: &super::SourceInvocationBinding,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    selectors: usize,
    parameters: &[tcl_syntax::formal_params::FormalParameter],
) -> Option<Vec<Option<(usize, String)>>> {
    let dialect = binding.variable_context.invocation_dialect?;
    let mut originals =
        original_literal_parameter_arguments(tokens, effective, selectors, parameters, dialect)?;
    for operand in &mut originals {
        if operand.as_ref().is_some_and(|(written, value)| {
            binding.evaluated_argument_words.get(*written)
                != Some(
                    &crate::registry_invocation::EffectiveInvocationWord::Literal(value.clone()),
                )
        }) {
            *operand = None;
        }
    }
    Some(originals)
}

fn original_literal_parameter_arguments(
    tokens: &crate::ir::CommandTokens,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    selectors: usize,
    parameters: &[tcl_syntax::formal_params::FormalParameter],
    dialect: tcl_registry::InvocationDialect,
) -> Option<Vec<Option<(usize, String)>>> {
    use tcl_syntax::formal_params::FormalArgumentBinding;
    let count = effective.words.len().checked_sub(1 + selectors)?;
    let plan = tcl_syntax::formal_params::bind_formal_arguments(
        parameters,
        count,
        dialect.parameter_grammar()?,
    )
    .ok()?;
    let mut originals = vec![None; parameters.len()];
    for formal in plan {
        let FormalArgumentBinding::Value {
            parameter,
            argument,
        } = formal
        else {
            continue;
        };
        let Some(written) = effective.written_argument(selectors + argument) else {
            continue;
        };
        let word = tokens.words().get(written + 1)?;
        let crate::registry_invocation::EffectiveInvocationWord::Literal(value) =
            crate::registry_invocation::effective_invocation_word(
                word,
                dialect.lexer_grammar.escapes,
                dialect.word_values,
            )
        else {
            continue;
        };
        originals[parameter] = Some((written, value));
    }
    Some(originals)
}

/// A declaration's own activation recipe. This proves neither that a caller
/// reached the body nor any caller frame, argument value or normal outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceConditionalBodyEntry {
    allocation: CommandAllocation,
    source: ExecutedScriptSource,
    namespace_key: super::SourceNamespaceKey,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    frame: crate::var_resolve::VariableExecutionFrame,
}

impl SourceConditionalBodyEntry {
    pub(super) fn allocation(&self) -> &CommandAllocation {
        &self.allocation
    }

    pub(super) fn source(&self) -> &ExecutedScriptSource {
        &self.source
    }

    pub(super) fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.frame
    }

    pub(crate) fn namespace_context(&self) -> Option<&super::SourceNamespaceKey> {
        matches!(
            self.frame.layout(),
            crate::var_resolve::VariableExecutionFrame::Procedure { .. }
        )
        .then_some(&self.namespace_key)
    }

    pub(super) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        &self.parameters
    }

    pub(crate) fn owns_source(&self, origin: &Arc<super::SourceOriginId>, offset: u32) -> bool {
        &self.source.origin == origin
            && offset >= self.source.base()
            && u64::from(offset) < u64::from(self.source.base()) + self.source.text.len() as u64
    }

    pub(crate) fn matches_parameters(&self, names: &[&str]) -> bool {
        self.parameters.len() == names.len()
            && self
                .parameters
                .iter()
                .all(|formal| names.contains(&formal.name.as_str()))
    }

    pub(crate) fn owns_invocation(&self, binding: &super::SourceInvocationBinding) -> bool {
        binding.variable_frame == self.frame
    }
}

impl SourceCommandBindings {
    /// A declaration frame and a retained entered frame are independent owners
    /// of the same original body. Entered ownership requires the definition
    /// allocation, original body image and recorded native-bound frame together.
    pub(super) fn original_body_owns_frame(
        &self,
        entry: &SourceConditionalBodyEntry,
        frame: &crate::var_resolve::VariableExecutionFrame,
    ) -> bool {
        frame == entry.frame()
            || self
                .entered_scripts
                .get(&entry.allocation().site)
                .into_iter()
                .flat_map(|arguments| arguments.values())
                .flatten()
                .any(|observation| {
                    observation.source == *entry.source()
                        && observation.frame.as_ref() == Some(frame)
                })
    }

    pub(super) fn original_body_owns_context(
        &self,
        entry: &SourceConditionalBodyEntry,
        context: &crate::var_resolve::ResolveContext,
    ) -> bool {
        let owns = |frame: &crate::var_resolve::VariableExecutionFrame| {
            Self::context_owns_frame(context, frame, frame.namespace_identity())
        };
        Self::declared_body_owns_context(entry, context)
            || self
                .entered_scripts
                .get(&entry.allocation().site)
                .into_iter()
                .flat_map(|arguments| arguments.values())
                .flatten()
                .filter(|observation| observation.source == *entry.source())
                .filter_map(|observation| observation.frame.as_ref())
                .any(owns)
    }

    pub(super) fn declared_body_owns_context(
        entry: &SourceConditionalBodyEntry,
        context: &crate::var_resolve::ResolveContext,
    ) -> bool {
        Self::context_owns_frame(context, entry.frame(), entry.namespace_context())
    }

    pub(super) fn context_owns_frame(
        context: &crate::var_resolve::ResolveContext,
        frame: &crate::var_resolve::VariableExecutionFrame,
        namespace: Option<&super::SourceNamespaceKey>,
    ) -> bool {
        let mut expected = crate::var_resolve::ResolveContext::default().in_frame(frame);
        if expected.namespace_known
            && let Some(namespace) = namespace
        {
            expected = expected.with_namespace_identity(namespace.clone());
        }
        context.activation == expected.activation
            && context.frame_kind == expected.frame_kind
            && context.selected_frame == expected.selected_frame
            && context.namespace_identity == expected.namespace_identity
            && context.namespace_known == expected.namespace_known
            && (!expected.namespace_known || context.namespace == expected.namespace)
    }

    /// Query the declaration's independently owned local frame. Ordinary
    /// invocation queries still join every actual and conditional observation.
    pub(crate) fn conditional_invocation_at_source(
        &self,
        entry: &SourceConditionalBodyEntry,
        offset: u32,
    ) -> super::SourceInvocationBinding {
        if self
            .root_origin
            .as_ref()
            .is_none_or(|origin| !entry.owns_source(origin, offset))
        {
            return super::SourceInvocationBinding::unknown();
        }
        Self::query_dispatch_points(
            self.dispatch_points_at(offset)
                .filter(|point| point.state.variable_frame == entry.frame),
        )
    }

    /// Instantiate only the original selected call in the read's own scoped
    /// activation. Conditional declaration ownership grants symbolic identity;
    /// it never supplies runtime entry or a physical caller-cell write.
    pub(crate) fn caller_frame_invocation_template_at(
        &self,
        tokens: &crate::ir::CommandTokens,
        read_offset: u32,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<SourceCallerFrameInvocationTemplate> {
        self.actual_caller_frame_invocation_template_at(tokens, read_offset, registry)
            .or_else(|| self.declared_caller_frame_invocation_template_at(tokens, read_offset))
    }

    fn declared_caller_frame_invocation_template_at(
        &self,
        tokens: &crate::ir::CommandTokens,
        read_offset: u32,
    ) -> Option<SourceCallerFrameInvocationTemplate> {
        let offset = tokens.words().first()?.source().span.start();
        let origin = self.root_origin.as_ref()?;
        let (frame, namespace_context, declared_receiver) =
            if let Some(entry) = self.conditional_body_entry_at(origin, offset) {
                if self
                    .conditional_body_entry_at(origin, read_offset)?
                    .as_ref()
                    != entry.as_ref()
                {
                    return None;
                }
                (
                    Some(entry.frame().clone()),
                    entry.namespace_context().cloned(),
                    None,
                )
            } else {
                let entry = self.declared_receiver_body_entry_at(origin, offset)?;
                if self
                    .declared_receiver_body_entry_at(origin, read_offset)?
                    .as_ref()
                    != entry.as_ref()
                {
                    return None;
                }
                (None, None, Some(entry))
            };
        let mut original = tokens.clone();
        self.stamp_original_tokens(&mut original);
        let advice = self.declaration_call_layout_advice(&original)?;
        let target = advice.targets().first()?;
        if advice.targets().iter().any(|candidate| candidate != target)
            || target.implementation_allocation.as_ref()?.incarnation
                == super::AllocationIncarnation::RepeatedFresh
        {
            return None;
        }
        let definition = super::SourceCommandDefinition::original_declared_procedure(target)?;
        let parameters = self.caller_template_parameters(definition.allocation())?;
        let effective = crate::registry_invocation::effective_words_for_target(&original, target)?;
        let parameter_arguments = original_literal_parameter_arguments(
            tokens,
            &effective,
            0,
            &parameters,
            advice.dialect(),
        )?;
        let call = super::CommandAllocationSite {
            source: Arc::clone(origin),
            offset,
        };
        crate::registry_invocation::native_compiler_replay_source(tokens, &call)?;
        Some(SourceCallerFrameInvocationTemplate {
            call,
            frame,
            namespace_context,
            declared_receiver,
            procedure: None,
            declared_procedure: Some(definition),
            method: None,
            parameter_arguments,
        })
    }

    fn actual_caller_frame_invocation_template_at(
        &self,
        tokens: &crate::ir::CommandTokens,
        read_offset: u32,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<SourceCallerFrameInvocationTemplate> {
        let offset = tokens.words().first()?.source().span.start();
        let head = tokens.argv_texts.first()?;
        let binding = self.caller_template_binding(head, offset, read_offset)?;
        let call = binding.invocation_site()?.clone();
        crate::registry_invocation::native_compiler_replay_source(tokens, &call)?;
        if call.offset != offset || self.root_origin.as_ref() != Some(&call.source) {
            return None;
        }
        if matches!(
            binding.variable_frame,
            crate::var_resolve::VariableExecutionFrame::Unknown
                | crate::var_resolve::VariableExecutionFrame::Selected { .. }
        ) || binding.variable_context.dynamic_bindings
        {
            return None;
        }
        let state = &binding.lookup_state.as_ref()?.state;
        if binding.entered_execution_observer.observed() || state.source_step_observed() {
            return None;
        }
        if !self.caller_template_scope_matches(&binding, offset, read_offset) {
            return None;
        }
        let mut stamped = tokens.clone();
        stamped.source_binding = Some(binding.clone());
        let (procedure, method, effective, selectors, parameters) = if let Some((_, entry, _)) =
            binding.receiver_self_method_entry(registry)
        {
            (
                None,
                Some(entry.clone()),
                crate::registry_invocation::compose_original_effective_words(&stamped, head, &[])?,
                1,
                entry
                    .formals()
                    .iter()
                    .map(
                        |(name, default)| tcl_syntax::formal_params::FormalParameter {
                            name: name.clone(),
                            default: default.clone(),
                        },
                    )
                    .collect::<Vec<_>>(),
            )
        } else {
            let target = binding.proved_execution_target()?;
            if target.registry_backed || target.kind != super::BindingKind::Proc {
                return None;
            }
            if state.source_execution_observed(Some(target.identity.as_ref()?)) {
                return None;
            }
            let allocation = target.implementation_allocation.as_ref()?;
            if allocation.incarnation == super::AllocationIncarnation::RepeatedFresh {
                return None;
            }
            let reference = binding.evaluated_command_reference()?;
            let definition = reference
                .linked_definition()
                .or_else(|| reference.definition())?;
            if definition.allocation() != allocation {
                return None;
            }
            (
                Some(reference),
                None,
                crate::registry_invocation::effective_words_for_target(&stamped, target)?,
                0,
                self.caller_template_parameters(allocation)?,
            )
        };
        let parameter_arguments =
            literal_parameter_arguments(tokens, &binding, &effective, selectors, &parameters)?;
        Some(SourceCallerFrameInvocationTemplate {
            call,
            namespace_context: binding
                .variable_context
                .namespace_identity
                .clone()
                .or_else(|| {
                    (self.final_state.baseline.native_entry.is_none()
                        && binding.variable_context.namespace_known)
                        .then(|| {
                            super::SourceNamespaceKey::authored(&binding.variable_context.namespace)
                        })
                }),
            frame: Some(binding.variable_frame),
            declared_receiver: None,
            procedure,
            declared_procedure: None,
            method,
            parameter_arguments,
        })
    }

    fn caller_template_parameters(
        &self,
        allocation: &super::CommandAllocation,
    ) -> Option<Vec<tcl_syntax::formal_params::FormalParameter>> {
        let mut bodies = self
            .deferred
            .values()
            .filter(|body| body.implementation_allocation.as_ref() == Some(allocation));
        let first = bodies.next()?;
        bodies
            .all(|body| body.parameters == first.parameters)
            .then(|| first.parameters.clone())
    }

    // A joined query retains all runtime alternatives. A declaration-owned
    // frame may recover symbolic scope only when every retained alternative
    // still selects the same procedure allocation and argument prefix.
    fn caller_template_binding(
        &self,
        head: &str,
        offset: u32,
        read_offset: u32,
    ) -> Option<super::SourceInvocationBinding> {
        let joined = self.invocation_at_source(head, offset);
        if joined.variable_frame != crate::var_resolve::VariableExecutionFrame::Unknown {
            return Some(joined);
        }
        let call = joined.invocation_site()?;
        let entry = self.conditional_body_entry_at(&call.source, offset)?;
        if self
            .conditional_body_entry_at(&call.source, read_offset)?
            .as_ref()
            != entry.as_ref()
        {
            return None;
        }
        let scoped = self.conditional_invocation_at_source(&entry, offset);
        let target = joined.proved_execution_target()?;
        let state = &joined.lookup_state.as_ref()?.state;
        if target.registry_backed
            || target.kind != super::BindingKind::Proc
            || scoped.proved_execution_target() != Some(target)
            || joined.entered_execution_observer.observed()
            || state.source_step_observed()
            || state.source_execution_observed(Some(target.identity.as_ref()?))
            || joined.variable_context.dynamic_bindings
        {
            return None;
        }
        Some(scoped)
    }

    fn caller_template_scope_matches(
        &self,
        binding: &super::SourceInvocationBinding,
        offset: u32,
        read_offset: u32,
    ) -> bool {
        let Some(call) = binding.invocation_site() else {
            return false;
        };
        let same_declared_frame = self
            .conditional_body_entry_at(&call.source, offset)
            .filter(|entry| entry.owns_invocation(binding))
            .is_some_and(|entry| {
                self.conditional_body_entry_at(&call.source, read_offset)
                    .is_some_and(|read| entry == read)
            });
        let original_call_operand = self.dispatch_points_at(offset).any(|point| {
            point.offset <= read_offset
                && read_offset < point.end
                && point.state.variable_frame == binding.variable_frame
        });
        let same_observed_frame = || {
            let reads = self
                .variable_accesses
                .values()
                .flatten()
                .filter(|access| {
                    access.source.span.start() <= read_offset
                        && read_offset < access.source.span.end()
                })
                .collect::<Vec<_>>();
            !reads.is_empty()
                && reads.iter().all(|read| {
                    !read.context_alternatives().is_empty()
                        && read.context_alternatives().iter().all(|context| {
                            !context.dynamic_bindings
                                && context.interpreter == binding.variable_context.interpreter
                                && context.execution == binding.variable_context.execution
                                && context.frame_kind == binding.variable_context.frame_kind
                                && context.activation == binding.variable_context.activation
                                && context.namespace == binding.variable_context.namespace
                                && context.selected_frame == binding.variable_context.selected_frame
                        })
                })
        };
        same_declared_frame || original_call_operand || same_observed_frame()
    }

    /// Actual procedure activations for this exact source. Declaration-only
    /// observations never establish a caller frame, even with the same bytes.
    #[cfg(test)]
    pub(crate) fn has_actual_procedure_entry(&self, source: &ExecutedScriptSource) -> bool {
        let mut observed = false;
        for observation in self
            .entered_scripts
            .values()
            .flat_map(|arguments| arguments.values())
            .flatten()
        {
            if &observation.source != source {
                continue;
            }
            let Some(frame) = &observation.frame else {
                continue;
            };
            observed = true;
            if !matches!(
                frame.layout(),
                crate::var_resolve::VariableExecutionFrame::Procedure { .. }
            ) {
                return false;
            }
        }
        observed
    }

    /// Retain the innermost declaration recipe for this original occurrence.
    /// Conflicting allocations, formal recipes or namespaces withdraw it.
    /// Events and future caller frames require their own protocols. Receiver
    /// methods retain only their independently allocated local-frame recipe.
    pub(crate) fn conditional_body_entry_at(
        &self,
        origin: &Arc<super::SourceOriginId>,
        offset: u32,
    ) -> Option<Arc<SourceConditionalBodyEntry>> {
        let mut selected: Option<SourceConditionalBodyEntry> = None;
        let candidates = || {
            self.deferred.values().filter(|body| {
                body.source_origin.as_ref() == Some(origin)
                    && body.offset <= offset
                    && u64::from(offset) < u64::from(body.offset) + body.source.len() as u64
            })
        };
        let length = candidates().map(|body| body.source.len()).min()?;
        for body in candidates().filter(|body| body.source.len() == length) {
            if body.event.is_some() || body.future_frame.is_some() {
                return None;
            }
            let source = body.executed_script.as_ref()?.2.clone();
            let recipe = SourceConditionalBodyEntry {
                allocation: body.implementation_allocation.clone()?,
                source,
                namespace_key: body.namespace_key.clone(),
                parameters: body.parameters.clone(),
                frame: if body.receiver_method {
                    crate::var_resolve::VariableExecutionFrame::ReceiverMethod {
                        identity: super::source_activation_name(
                            body.source_origin.as_ref(),
                            &body.identity,
                            body.implementation_generation,
                        ),
                    }
                } else {
                    crate::var_resolve::VariableExecutionFrame::Procedure {
                        namespace: body.namespace.clone(),
                        identity: super::source_activation_name(
                            body.source_origin.as_ref(),
                            &body.identity,
                            body.implementation_generation,
                        ),
                    }
                    .with_namespace_identity(body.namespace_key.clone())
                },
            };
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &recipe)
            {
                return None;
            }
            selected = Some(recipe);
        }
        selected.map(Arc::new)
    }

    /// The innermost native receiver-body declaration retains its source
    /// owner even when no implementation or receiver has been allocated.
    pub(super) fn declared_receiver_body_entry_at(
        &self,
        origin: &Arc<super::SourceOriginId>,
        offset: u32,
    ) -> Option<Arc<SourceDeclaredReceiverBodyEntry>> {
        let candidates = || {
            self.deferred.values().filter(|body| {
                body.source_origin.as_ref() == Some(origin)
                    && body.offset <= offset
                    && u64::from(offset) < u64::from(body.offset) + body.source.len() as u64
            })
        };
        let length = candidates().map(|body| body.source.len()).min()?;
        let mut agreed = None;
        for body in candidates().filter(|body| body.source.len() == length) {
            if !body.receiver_method || body.event.is_some() || body.future_frame.is_some() {
                return None;
            }
            let (declaration, _, source) = body.executed_script.as_ref()?;
            if &source.origin != origin
                || source.base() != body.offset
                || source.try_text().ok()? != body.source
            {
                return None;
            }
            let entry = SourceDeclaredReceiverBodyEntry {
                declaration: declaration.clone(),
                source: source.clone(),
                declaration_namespace: body.namespace_key.clone(),
                parameters: body.parameters.clone(),
                preview_frame: crate::var_resolve::VariableExecutionFrame::ReceiverMethod {
                    identity: super::source_activation_name(
                        body.source_origin.as_ref(),
                        &body.identity,
                        body.implementation_generation,
                    ),
                },
            };
            if agreed.as_ref().is_some_and(|previous| previous != &entry) {
                return None;
            }
            agreed = Some(entry);
        }
        agreed.map(Arc::new)
    }
}

impl SourceCommandBindings {
    /// Attach independent declared assistance through the same immutable lookup
    /// worlds. Supplied metadata cannot restore a replaced or opaque slot.
    pub(crate) fn attach_declared_body_assistance(
        &self,
        tokens: &mut crate::ir::CommandTokens,
        declared: &tcl_registry::model::DeclaredSurface,
    ) {
        let Some(head) = tokens.argv_texts.first() else {
            return;
        };
        let declarations = declared
            .iter()
            .map(|(name, _)| (super::nqn(name), name.to_owned()))
            .collect::<super::BTreeMap<_, _>>();
        let Some(binding) = tokens.source_binding.as_ref() else {
            return;
        };
        let candidate = if let Some(site) = binding.invocation_site() {
            if self.root_origin.as_ref() != Some(&site.source) {
                return;
            }
            let mut points = self.dispatch_points_at(site.offset);
            let Some(first) = points.next() else {
                return;
            };
            let candidate =
                first
                    .state
                    .declared_candidate_from(head, &first.namespace, &declarations);
            if points.any(|point| {
                point
                    .state
                    .declared_candidate_from(head, &point.namespace, &declarations)
                    != candidate
            }) {
                return;
            }
            candidate
        } else {
            let candidate = self
                .final_state
                .declared_candidate_from(head, "::", &declarations);
            if self.points.iter().any(|point| {
                point
                    .state
                    .declared_candidate_from(head, &point.namespace, &declarations)
                    != candidate
            }) {
                return;
            }
            candidate
        };
        if let Some(binding) = tokens.source_binding.as_mut() {
            binding.declared_command = candidate;
        }
    }
}

#[cfg(test)]
mod original_body_frame_tests {
    #[test]
    fn declared_caller_navigation_keeps_definition_without_an_actual_dispatch() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let source = "proc setter {target} {upvar 1 $target local; set local VALUE}\noo::class create C {constructor {} {setter name; puts $name}; method other {} {puts $elsewhere}}";
        let bindings = super::SourceCommandBindings::analyse(source, config, registry);
        let offset = u32::try_from(source.find("setter name").unwrap()).unwrap();
        let read = u32::try_from(source.find("$name").unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            "setter name",
            offset,
            config,
        )
        .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .proved_execution_target()
                .is_none()
        );
        let template = bindings
            .caller_frame_invocation_template_at(&tokens, read + 1, registry)
            .expect("original constructor owns a conditional caller template");
        assert!(template.procedure().is_none());
        assert!(template.frame().is_none());
        let declared_receiver = template
            .declared_receiver_body()
            .expect("an original receiver body has a declaration scope, not a physical receiver");
        assert_eq!(
            declared_receiver.source().try_text().unwrap(),
            "setter name; puts $name"
        );
        assert_eq!(
            template
                .procedure_definition()
                .unwrap()
                .allocation()
                .site
                .offset,
            0
        );
        assert_eq!(template.literal_parameter_argument(0), Some((0, "name")));
        assert!(
            bindings
                .caller_frame_invocation_template_at(
                    &tokens,
                    u32::try_from(source.find("$elsewhere").unwrap()).unwrap() + 1,
                    registry
                )
                .is_none()
        );
    }

    #[test]
    fn original_multiline_caller_navigation_retains_the_declared_procedure() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let source = "proc NameProcess {arguments object} {\n    upvar name name\n    set name W1\n}\nproc build {} {\n    NameProcess x obj\n    set out $name\n}\n";
        let bindings = super::SourceCommandBindings::analyse(source, config, registry);
        let offset = u32::try_from(source.find("NameProcess x obj").unwrap()).unwrap();
        let read = u32::try_from(source.find("$name").unwrap()).unwrap() + 1;
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            "NameProcess x obj",
            offset,
            config,
        )
        .remove(0);
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        let template = bindings
            .caller_frame_invocation_template_at(&tokens, read, registry)
            .expect("the original caller and callee allocations retain navigation");
        assert_eq!(
            template
                .procedure_definition()
                .unwrap()
                .allocation()
                .site
                .offset,
            0
        );
        assert!(template.declared_receiver_body().is_none());
        assert!(template.frame().is_some());
    }

    #[test]
    fn entered_original_body_frames_keep_declaration_and_allocation_independent() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let source = "proc p {x} {set y $x}; proc q {x} {set y $x}; p 1; q 2";
        let bindings = super::SourceCommandBindings::analyse(source, config, registry);
        let origin = bindings.root_origin.as_ref().unwrap();
        let first = u32::try_from(source.find("set y $x").unwrap()).unwrap();
        let second = u32::try_from(source.rfind("set y $x").unwrap()).unwrap();
        let declaration = bindings.conditional_body_entry_at(origin, first).unwrap();
        let p = bindings.invocation_at_source("set", first);
        let q = bindings.invocation_at_source("set", second);
        assert!(!declaration.owns_invocation(&p));
        assert!(bindings.original_body_owns_frame(&declaration, &p.variable_frame));
        assert!(bindings.original_body_owns_context(&declaration, &p.variable_context));
        assert!(!bindings.original_body_owns_frame(&declaration, &q.variable_frame));
        assert!(!bindings.original_body_owns_context(&declaration, &q.variable_context));
        assert!(bindings.original_body_owns_frame(&declaration, declaration.frame()));
    }

    #[test]
    fn rejected_formal_binding_cannot_supply_an_entered_original_body_frame() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let source = "proc p {x} {set y $x}; p";
        let bindings = super::SourceCommandBindings::analyse(source, config, registry);
        let origin = bindings.root_origin.as_ref().unwrap();
        let offset = u32::try_from(source.find("set y $x").unwrap()).unwrap();
        let declaration = bindings.conditional_body_entry_at(origin, offset).unwrap();
        assert!(
            bindings
                .entered_scripts
                .get(&declaration.allocation().site)
                .into_iter()
                .flat_map(|arguments| arguments.values())
                .flatten()
                .all(|observation| observation.frame.is_none())
        );
        assert!(!bindings.original_body_owns_frame(
            &declaration,
            &crate::var_resolve::VariableExecutionFrame::Global
        ));
    }
}

#[cfg(test)]
mod native_context_tests;
