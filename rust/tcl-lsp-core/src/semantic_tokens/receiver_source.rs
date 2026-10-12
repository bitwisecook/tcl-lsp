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
use tcl_lexer::{ExecutablePart, NativeWord};
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
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
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
        return Ok(ReceiverSourceCandidates::ClassNames(
            collection_classes(ctx, &words).unwrap_or_default(),
        ));
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

fn collection_classes(ctx: ScriptCtx<'_>, words: &OriginalRegistryWords) -> Option<Vec<String>> {
    let ordinal = words.with_source_schema(ctx.generation, |schema| {
        let tcl_registry::types::ReturnElements::ElementOf { container_arg } =
            schema.semantics.return_elements?
        else {
            return None;
        };
        let ordinal = schema.semantics.argument_offset + usize::from(container_arg);
        // Multiple indices can select an inner collection rather than one element.
        (schema.words.arguments().exact_argv_len() == Some(ordinal + 2)).then_some(ordinal)
    })??;
    let container = scalar_reference_name(words.original_argument_word(ordinal)?)?;
    ctx.object_collections
        .get(container)
        .map(|classes| classes.iter().cloned().collect())
}

fn scalar_reference_name(word: &NativeWord) -> Option<&str> {
    if word.group().expand {
        return None;
    }
    let arena = word.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    let ExecutablePart::Variable { name, index: None } = part.part else {
        return None;
    };
    std::str::from_utf8(arena.bytes(name)?).ok()
}

#[cfg(test)]
mod tests;
