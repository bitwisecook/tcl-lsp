// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional receiver candidates from current original child commands.

use super::{ArgOverride, ScriptCtx};
use rustc_hash::FxHashMap;
use tcl_compiler::analyser::{ClassDef, ClassHierarchy};
use tcl_compiler::{
    registry_invocation::source_structure::{self, OriginalRegistryWords},
    segmenter::SegmentedCommand,
};
use tcl_registry::definer::{BuiltinMethodReceiver, DefinitionBodyGrammar, MethodReach};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReceiverSourceUnavailable {
    OriginalOperand,
    SingleChild,
    SelectedSchema,
    CanonicalClassReceipt,
    ClassShape,
}

pub(super) enum ReceiverSourceCandidates<'a> {
    ClassNames(Vec<String>),
    LogicalClass(OriginalLogicalReceiverClass<'a>),
    /// A whole selected element extraction. Its element classes come only from
    /// the retained original insertion/extraction occurrences, never a name map.
    CollectionElement,
}

/// A canonical same-source class metadata join and its actual selected grammar.
/// Its displayed name cannot enter the registry-class receiver branch.
pub(super) struct OriginalLogicalReceiverClass<'a> {
    metadata: &'a ClassDef,
    grammar: &'static DefinitionBodyGrammar,
}
impl OriginalLogicalReceiverClass<'_> {
    pub(super) fn insert_method(
        self,
        ctx: ScriptCtx<'_>,
        command: &SegmentedCommand,
        hierarchy: Option<&ClassHierarchy>,
        overrides: &mut FxHashMap<u32, ArgOverride>,
    ) {
        let Some(hierarchy) = hierarchy else {
            return;
        };
        let class = &self.metadata.qualified_name;
        if hierarchy.classes.get(class) != Some(self.metadata) {
            return;
        }
        // Reborrow the shared receiver/member owner at this exact original
        // command. Its full vector check and static operand requirement precede
        // presentation; the child constructor alone cannot select a method.
        let Some(analysis) = ctx.analysis else {
            return;
        };
        let Some(receiver) = source_structure::original_project_receiver_class_with_sources(
            ctx.full_source,
            analysis,
            command,
            &[],
            ctx.original_collections,
        ) else {
            return;
        };
        if receiver.metadata() != self.metadata || !receiver.provides_method() {
            return;
        }
        let Some(method) = command.texts.get(1) else {
            return;
        };
        let property = self
            .grammar
            .property_accessor_methods
            .contains(&method.as_str());
        if hierarchy.method_target(class, method).is_none()
            && self
                .grammar
                .builtin_object_method(method, MethodReach::ObjectCommand)
                .is_none_or(|method| method.receiver != BuiltinMethodReceiver::AnyObject)
            && !property
        {
            return;
        }
        super::mark_method_word(command, overrides);
        if property {
            super::insert_user_configure_options(
                command,
                hierarchy,
                ctx.registry,
                class,
                method,
                overrides,
            );
        }
    }
}

/// Source-only candidates. Reporting class names never select the constructor;
/// the canonical class receipt precedes the name used for presentation.
pub(super) fn candidates<'a>(
    ctx: ScriptCtx<'a>,
    command: &SegmentedCommand,
) -> Result<ReceiverSourceCandidates<'a>, ReceiverSourceUnavailable> {
    // naming.core.original-inlay-retained-context

    let analysis = ctx
        .analysis
        .ok_or(ReceiverSourceUnavailable::OriginalOperand)?;
    let offset = command
        .argv
        .first()
        .ok_or(ReceiverSourceUnavailable::OriginalOperand)?
        .span
        .start();
    let original =
        source_structure::original_logical_source_words_at(ctx.full_source, analysis, offset)
            .ok_or(ReceiverSourceUnavailable::OriginalOperand)?;
    let head = original
        .first()
        .ok_or(ReceiverSourceUnavailable::OriginalOperand)?;
    let child = source_structure::original_single_source_command_substitution(head)
        .ok_or(ReceiverSourceUnavailable::SingleChild)?;
    let child_head = child
        .command()
        .words
        .first()
        .ok_or(ReceiverSourceUnavailable::SingleChild)?;
    let child_offset = child_head.span().start();
    let words = source_structure::source_registry_words_at(ctx.full_source, analysis, child_offset);
    if let Some(words) = words {
        if words.head_source().and_then(|head| head.word()) != Some(child_head) {
            return Err(ReceiverSourceUnavailable::SelectedSchema);
        }
        if let Some(class) = words
            .with_source_schema(ctx.generation, |schema| {
                schema.authored_source_callable_factory_class()
            })
            .flatten()
        {
            return Ok(ReceiverSourceCandidates::ClassNames(vec![class.to_owned()]));
        }
        return Ok(if element_extraction(ctx, &words) {
            ReceiverSourceCandidates::CollectionElement
        } else {
            ReceiverSourceCandidates::ClassNames(Vec::new())
        });
    }
    let call =
        source_structure::source_constructor_call_at(ctx.full_source, analysis, child_offset)
            .ok_or(ReceiverSourceUnavailable::CanonicalClassReceipt)?;
    if call.original_words() != child.command().words.as_slice() {
        return Err(ReceiverSourceUnavailable::CanonicalClassReceipt);
    }
    call.logical_constructor_shape(analysis)
        .ok_or(ReceiverSourceUnavailable::ClassShape)?;
    let class = call
        .class_declaration()
        .logical_source_class(analysis)
        .ok_or(ReceiverSourceUnavailable::CanonicalClassReceipt)?;
    let grammar = call
        .class_declaration()
        .grammar(ctx.generation)
        .ok_or(ReceiverSourceUnavailable::ClassShape)?;
    Ok(ReceiverSourceCandidates::LogicalClass(
        OriginalLogicalReceiverClass {
            metadata: class,
            grammar,
        },
    ))
}

fn element_extraction(ctx: ScriptCtx<'_>, words: &OriginalRegistryWords) -> bool {
    words
        .with_source_schema(ctx.generation, |schema| {
            let tcl_registry::types::ReturnElements::ElementOf { container_arg } =
                schema.semantics.return_elements?
            else {
                return None;
            };
            let ordinal = schema.semantics.argument_offset + usize::from(container_arg);
            // Multiple indices can select an inner collection rather than one element.
            (schema.words.arguments().exact_argv_len() == Some(ordinal + 2)).then_some(())
        })
        .flatten()
        .is_some()
}

/// Element receiver class from the document's retained original collection
/// sources at this exact receiver occurrence. A missing compilation unit or an
/// unselected association supplies no element class. Project mode has already
/// asked the same owner with its foreign providers; that answer is terminal.
pub(super) fn insert_collection_method(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if ctx.project_classes.is_some() {
        return;
    }
    let (Some(analysis), Some(sources)) = (ctx.analysis, ctx.original_collections) else {
        return;
    };
    let Some(class) = source_structure::original_project_receiver_class_with_sources(
        ctx.full_source,
        analysis,
        command,
        &[],
        Some(sources),
    ) else {
        return;
    };
    if class.collection_source().is_none() || !class.provides_method() {
        return;
    }
    super::mark_method_word(command, overrides);
    if let Some(method) = command.texts.get(1) {
        super::insert_user_configure_options(
            command,
            class.hierarchy(),
            ctx.registry,
            &class.metadata().qualified_name,
            method,
            overrides,
        );
    }
}

#[cfg(test)]
mod tests;

/// Actual selected source class/member owner before any project retagging.
pub(super) fn insert_project_method(
    ctx: ScriptCtx<'_>,
    command: &SegmentedCommand,
    providers: &[source_structure::OriginalProjectClassCards],
    overrides: &mut FxHashMap<u32, ArgOverride>,
) -> bool {
    let Some(analysis) = ctx.analysis else {
        return false;
    };
    let Some(class) = source_structure::original_project_receiver_class_with_sources(
        ctx.full_source,
        analysis,
        command,
        providers,
        ctx.original_collections,
    ) else {
        return false;
    };
    if class.provides_method() {
        super::mark_method_word(command, overrides);
        if let Some(method) = command.texts.get(1) {
            super::insert_user_configure_options(
                command,
                class.hierarchy(),
                ctx.registry,
                &class.metadata().qualified_name,
                method,
                overrides,
            );
        }
    }
    true
}
