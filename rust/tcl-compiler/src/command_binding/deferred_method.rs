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

//! Lexical receiver bodies of proved native class declarations.

use super::{
    Arc, CommandAllocationSite, ExecutedScriptSource, InvocationFacts, ModuleCommandBindings,
    SourceCommandBindings, SourceExecutionContext, SourceNativeInvocation,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_core_types::NameBytes;

mod class_definitions;
mod closed_members;
mod definition_references;
mod retained_entries;

use closed_members::class_definition_block;
pub(super) use closed_members::{
    closed_definition_member, original_definition_member, original_definition_member_target,
};
use definition_references::{
    capture_definition_method_references, capture_standalone_definition_references,
    standalone_definition_class,
};
use retained_entries::{
    closed_bounded_method_segments, deferred_method_body, retained_class_entries,
    select_bounded_method_definition,
};
pub(super) use retained_entries::{original_member_input, retained_method_entry};

/// Receiver selected by an original, validated method declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceMethodReceiver {
    /// A method declared on manufactured instances.
    Instance,
    /// A class-facing declaration on its own table or native classmethod delegate.
    Class,
}

/// Temporal method declaration inventory. Original source, frame and selected
/// visibility remain separate from current receiver execution and result proof.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceReceiverMethodEntry {
    name: String,
    original_name: SignatureSourceNameInput,
    name_source: tcl_source::words::SourceSite,
    receiver: SourceMethodReceiver,
    exported: bool,
    declaring_class: Option<super::SourceCommandTarget>,
    declaring_object: Option<super::SourceObjectAllocation>,
    object_body_context: Option<super::SourceCommandTarget>,
    class_delegate: bool,
    delegate_allocation: Option<Arc<SourceClassDelegateAllocation>>,
    entry: super::SourceConstructorEntry,
}

/// One native delegate allocated by an original class factory, identified by
/// its owning class allocation rather than the generated namespace's bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SourceClassDelegateAllocation {
    owner: super::SourceCommandTarget,
    factory: super::SourceCommandTarget,
    worker: super::SourceCommandTarget,
    generation: u64,
}

/// Declared methods keyed by their original receiver table and frozen name.
pub type SourceReceiverMethodEntries =
    super::BTreeMap<(SourceMethodReceiver, NameBytes), SourceReceiverMethodEntry>;

#[derive(Clone, Copy)]
pub(super) struct MethodParameterSource {
    pub(super) parameters: tcl_dialect::ParameterGrammar,
    pub(super) policy: tcl_syntax::naming::NamePolicyProtocol,
}

struct BoundedMethodDefinition {
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    original_factory: super::registered_class_factory::OriginalClassFactoryState,
    definition: ExecutedScriptSource,
    parameters: tcl_dialect::ParameterGrammar,
}

impl SourceReceiverMethodEntry {
    /// Frozen declared method name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Actual original declaration name, independent of reporting text.
    #[must_use]
    pub fn original_name_input(&self) -> &SignatureSourceNameInput {
        &self.original_name
    }
    /// Select one exact original own-table name. The caller independently
    /// supplies the current receiver/table and any dispatch or visibility proof.
    #[must_use]
    pub fn for_original_input<'a>(
        entries: &'a SourceReceiverMethodEntries,
        receiver: SourceMethodReceiver,
        input: &SignatureSourceNameInput,
    ) -> Option<&'a Self> {
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        input
            .policy()
            .recipe()
            .oo_method_input(input.bytes())
            .ok()?;
        let entry = entries.get(&(receiver, NameBytes::from(input.bytes())))?;
        (entry.original_name.policy() == input.policy()).then_some(entry)
    }
    /// Exact original name-word site, including its source provenance.
    #[must_use]
    pub fn name_source(&self) -> &tcl_source::words::SourceSite {
        &self.name_source
    }
    /// Instance or class-object declaration receiver.
    #[must_use]
    pub fn receiver(&self) -> SourceMethodReceiver {
        self.receiver
    }
    /// Visibility at this retained dispatch generation. This does not prove
    /// that the receiver or dispatcher is still current.
    #[must_use]
    pub fn is_exported(&self) -> bool {
        self.exported
    }
    /// Original declaring class, separately from the current receiver class.
    /// Inherited entries retain this exact implementation allocation.
    #[must_use]
    pub fn declaring_class(&self) -> Option<&super::SourceCommandTarget> {
        self.declaring_class.as_ref()
    }
    /// Actual own-object implementation owner, independent of its class.
    #[must_use]
    pub fn declaring_object(&self) -> Option<&super::SourceObjectAllocation> {
        self.declaring_object.as_ref()
    }
    pub(super) fn body_context_provider(&self) -> Option<&super::SourceCommandTarget> {
        self.declaring_class
            .as_ref()
            .or(self.object_body_context.as_ref())
    }
    pub(super) fn with_own_object_provider(
        mut self,
        receiver: &super::SourceObjectInstanceProof,
    ) -> Self {
        self.declaring_class = None;
        self.declaring_object = Some(receiver.allocation().clone());
        self.object_body_context = Some(receiver.class_target().clone());
        self.receiver = SourceMethodReceiver::Class;
        self
    }
    /// Native delegate methods inherit through the original class allocation;
    /// an ordinary self method never acquires that inheritance policy.
    #[must_use]
    pub fn is_native_class_delegate(&self) -> bool {
        self.class_delegate
    }
    /// Actual source instance and declaration offset.
    #[must_use]
    pub fn declaration(&self) -> &CommandAllocationSite {
        self.entry.declaration()
    }
    /// Original body carrier of this method incarnation.
    #[must_use]
    pub fn body(&self) -> &ExecutedScriptSource {
        self.entry.body()
    }
    /// Native formal names and defaults retained at definition.
    #[must_use]
    pub fn formals(&self) -> &[(String, Option<String>)] {
        self.entry.formals()
    }
    /// Original receiver-method activation.
    #[must_use]
    pub fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        self.entry.frame()
    }
}

impl SourceCommandBindings {
    pub(super) fn retain_standalone_definition_references(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        let Some((class, grammar)) = standalone_definition_class(native, facts, state, context)
        else {
            return;
        };
        let Some(origin) = state.current_source_origin.as_ref() else {
            return;
        };
        let phases = capture_standalone_definition_references(
            native, facts, grammar, &class, state, context,
        );
        self.record_standalone_definition_method_references(
            &class,
            CommandAllocationSite {
                source: Arc::clone(origin),
                offset: native.segment.span.start(),
            },
            phases,
        );
    }

    pub(super) fn register_bounded_methods(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
    ) -> bool {
        // naming.tcloo.original-lexical-member-context
        // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
        let Some(BoundedMethodDefinition {
            grammar,
            original_factory,
            definition,
            parameters,
        }) = select_bounded_method_definition(native, state, context, facts)
        else {
            return false;
        };
        let map =
            tcl_lexer::SourceMap::from_image(&definition.text).with_base(definition.base(), 0, 0);
        let Some(segments) = tcl_source::segment::segment_commands_image_with_offset_and_config(
            &definition.text,
            definition.base(),
            context.config,
        ) else {
            return false;
        };
        if !closed_bounded_method_segments(grammar, &definition, &map, &segments, state, context) {
            return false;
        }
        let Some(policy) = state
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        else {
            return false;
        };
        // Closed declaration workers retain source scopes independently of
        // the later inherited dispatch-chain and allocation checks.
        self.register_original_method_bodies(
            grammar,
            &definition,
            state.baseline.dialect,
            parameters,
            context,
        );
        let mut entries = retained_class_entries(
            grammar,
            &definition,
            &map,
            &segments,
            state.baseline.dialect,
            MethodParameterSource { parameters, policy },
            context,
        );
        if !state.inherit_class_entries(grammar, &map, &segments, context, &mut entries) {
            return false;
        }
        let references = capture_definition_method_references(
            grammar,
            &definition,
            &map,
            &segments,
            state,
            DefinitionReferenceScope {
                parameters,
                receiver: SourceMethodReceiver::Instance,
            },
            context,
        );
        if original_factory.recipe().support().is_some()
            && let Some(names) = &mut entries.instance_methods
        {
            Arc::make_mut(names).extend(
                grammar
                    .property_accessor_methods
                    .iter()
                    .map(|name| NameBytes::from(name.as_bytes())),
            );
        }
        let created =
            state.record_class_definition(facts, native, context, entries, original_factory);
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_OO_REGISTRATION offset={} installed={}",
                native.segment.span.start(),
                created.is_some()
            );
        }
        if let Some(created) = created {
            self.record_definition_method_references(&created, references);
        }
        true
    }
    /// The methods a closed class configuration declares, recorded as that
    /// configuration's observation of the class, as a creation's body is.
    pub(super) fn record_configured_class_method_references(
        &mut self,
        configuration: &super::closed_class_configuration::ClosedClassConfiguration,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        let definition = &configuration.body;
        let map =
            tcl_lexer::SourceMap::from_image(&definition.text).with_base(definition.base(), 0, 0);
        let phases = tcl_source::segment::segment_commands_image_with_offset_and_config(
            &definition.text,
            definition.base(),
            context.config,
        )
        .and_then(|segments| {
            capture_definition_method_references(
                configuration.grammar,
                definition,
                &map,
                &segments,
                state,
                DefinitionReferenceScope {
                    parameters: configuration.parameters,
                    receiver: SourceMethodReceiver::Instance,
                },
                context,
            )
        });
        self.replace_standalone_definition_method_references(
            &configuration.class,
            configuration.site.clone(),
            phases,
        );
    }

    pub(super) fn register_original_method_bodies(
        &mut self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        definition: &ExecutedScriptSource,
        dialect: Option<tcl_native_model::InvocationDialect>,
        parameters: tcl_dialect::ParameterGrammar,
        context: SourceExecutionContext<'_>,
    ) {
        let map =
            tcl_lexer::SourceMap::from_image(&definition.text).with_base(definition.base(), 0, 0);
        let Some(segments) = tcl_source::segment::segment_commands_image_with_offset_and_config(
            &definition.text,
            definition.base(),
            context.config,
        ) else {
            return;
        };
        for segment in segments {
            if let Some(block) =
                class_definition_block(grammar, definition, &map, &segment, dialect, context)
            {
                self.register_original_method_bodies(grammar, &block, dialect, parameters, context);
            } else if let Some(body) = deferred_method_body(
                grammar, definition, &map, &segment, dialect, parameters, context,
            ) {
                self.deferred.insert(body.implementation_id(), body);
            }
        }
    }
}

struct RetainedClassEntries {
    instance_methods: Option<Arc<super::BTreeSet<NameBytes>>>,
    instance_variables:
        Option<Arc<super::receiver_variable_inventory::OriginalReceiverVariableInventory>>,
    constructor_entry: Option<Arc<super::SourceConstructorEntry>>,
    constructor_provider: Option<super::SourceCommandTarget>,
    destructor_entry: Option<Arc<super::SourceConstructorEntry>>,
    destructor_provider: Option<super::SourceCommandTarget>,
    lifecycle_entries_closed: bool,
    receiver_method_entries: Arc<SourceReceiverMethodEntries>,
    forward_method_entries: Arc<super::deferred_forward::OriginalForwardEntries>,
    inherited_classes: Arc<Vec<super::SourceCommandTarget>>,
    single_superclass: bool,
}

#[derive(Clone, Copy)]
struct DefinitionReferenceScope {
    parameters: tcl_dialect::ParameterGrammar,
    receiver: SourceMethodReceiver,
}

#[cfg(test)]
mod tests;
