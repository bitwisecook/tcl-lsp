// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original receiver declarations retained at an actual source read or dispatch.
//! These views grant navigation only, independently of method execution,
//! visibility, result, purity and nominal completion candidates.

use tcl_compiler::analyser::{AnalysisResult, ClassDef, MethodDef};
use tcl_compiler::command_binding::{
    SourceCommandTarget, SourceDefinitionMethodReference, SourceMethodReceiver, SourceOriginKind,
    SourceReceiverMethodEntry,
};
use tcl_compiler::ir::CommandTokens;
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_lexer::{SourceMap, Span};

/// Original class and member selected by a retained temporal receiver receipt.
pub(crate) struct RetainedReceiverMethod<'a> {
    pub(crate) class: &'a ClassDef,
    pub(crate) receiver_class: &'a ClassDef,
    pub(crate) method: &'a MethodDef,
    pub(crate) metadata: &'a tcl_compiler::analyser::types::OriginalSourceMethodMetadata,
    pub(crate) own_entry: Option<(SourceReceiverMethodEntry, u64)>,
    pub(crate) receiver: SourceMethodReceiver,
    pub(crate) selector: Span,
    editable_selector: bool,
    selector_input: tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
}

impl RetainedReceiverMethod<'_> {
    /// An original static written selector, independently of dispatch identity.
    /// Computed and expanded operands retain navigation but cannot be replaced.
    /// Immutable original queried selector, independent of the canonical body
    /// name and the separate editable geometry receipt.
    pub(crate) fn original_selector_input(
        &self,
    ) -> &tcl_compiler::signature_scan::scope::SignatureSourceNameInput {
        &self.selector_input
    }

    pub(crate) fn editable_selector(&self) -> Option<Span> {
        self.editable_selector.then_some(self.selector)
    }
}

/// Match the original class allocation, including overwritten declarations.
/// Neither the current slot nor a nominal class name can substitute for it.
pub(crate) fn original_class<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    target: &SourceCommandTarget,
) -> Option<&'a ClassDef> {
    let allocation = target.implementation_allocation.as_ref()?;
    if !matches!(allocation.site.source.kind(), SourceOriginKind::Authored(bytes) if bytes.as_ref() == source.as_bytes())
    {
        return None;
    }
    let mut declarations = analysis
        .original_class_declarations()
        .filter(|declaration| declaration.declaration_site() == &allocation.site);
    let declaration = declarations.next()?;
    declarations
        .next()
        .is_none()
        .then_some(declaration.metadata())
}

/// Original instance class at the authentic post-argument command head,
/// independently of whether its written selector names an existing method.
pub(crate) fn class_at_command_head<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    command: &SegmentedCommand,
) -> Option<&'a ClassDef> {
    let realm = analysis.retained_command_realm()?;
    let config = analysis.body_lexer_config?;
    let tokens = CommandTokens::from_segmented(&SourceMap::new(source), config, command);
    let head = tokens.word_exprs.first()?;
    let offset = head.source().span.start();
    let binding = realm.invocation_at_source(command.name(), offset);
    if !matches!(binding.source_origin()?.kind(), SourceOriginKind::Authored(bytes)
        if bytes.as_ref() == source.as_bytes())
    {
        return None;
    }
    if head.sole_variable_substitution().is_some() {
        let accesses = realm.variable_accesses_for_invocation_args(offset);
        let mut classes = accesses
            .iter()
            .filter(|access| {
                head.sole_variable_substitution()
                    .is_some_and(|(_, site)| site == &access.source)
            })
            .filter(|access| binding.retains_object_instance_at_dispatch(access))
            .filter_map(|access| access.proved_object_instance())
            .filter_map(|proof| original_class(analysis, source, proof.class_target()));
        let first = classes.next()?;
        return classes
            .all(|class| class.name_span == first.name_span)
            .then_some(first);
    }
    let proof = binding
        .frozen_object_instance_at_dispatch(head)
        .or_else(|| binding.named_object_instance_at_dispatch())?;
    original_class(analysis, source, proof.class_target())
}

fn original_method<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    receiver_target: &SourceCommandTarget,
    entry: &SourceReceiverMethodEntry,
    selection: (
        Span,
        &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
        bool,
    ),
    generation: Option<u64>,
) -> Option<RetainedReceiverMethod<'a>> {
    let (selector, selector_input, editable_selector) = selection;
    if let Some(allocation) = entry.declaring_object() {
        let receiver_class = original_class(analysis, source, receiver_target)?;
        let metadata = original_own_method_metadata(analysis, source, allocation, entry)?;
        let method = metadata.metadata();
        return Some(RetainedReceiverMethod {
            class: receiver_class,
            receiver_class,
            method,
            metadata,
            own_entry: Some((entry.clone(), generation?)),
            receiver: entry.receiver(),
            selector,
            selector_input: selector_input.clone(),
            editable_selector,
        });
    }
    original_method_in_class(
        analysis,
        source,
        (receiver_target, entry.declaring_class()?),
        entry,
        selector,
        selector_input,
        editable_selector,
    )
}

/// Reborrow the canonical declaration of one independently retained own-table
/// entry. Current receiver/table currency and visibility remain caller duties.
pub(crate) fn original_own_method_metadata<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    allocation: &tcl_compiler::command_binding::SourceObjectAllocation,
    entry: &SourceReceiverMethodEntry,
) -> Option<&'a tcl_compiler::analyser::types::OriginalSourceMethodMetadata> {
    if entry.declaring_object() != Some(allocation)
        || !matches!(entry.declaration().source.kind(), SourceOriginKind::Authored(bytes)
            if bytes.as_ref() == source.as_bytes())
        || !matches!(
            entry.frame(),
            tcl_compiler::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
        )
    {
        return None;
    }
    let mut declarations = analysis
        .original_object_configurations()
        .filter(|configuration| configuration.target().instance().allocation() == allocation)
        .flat_map(|configuration| configuration.members().declarations())
        .filter(|declaration| {
            declaration.side() == tcl_compiler::analyser::types::MemberSide::ClassObject
                && declaration.declaration().site() == entry.declaration()
                && declaration.metadata().name_span == entry.name_source().span
        });
    let first = declarations.next()?;
    declarations.next().is_none().then_some(first)
}

fn original_method_in_class<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    targets: (&SourceCommandTarget, &SourceCommandTarget),
    entry: &SourceReceiverMethodEntry,
    selector: Span,
    selector_input: &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
    editable_selector: bool,
) -> Option<RetainedReceiverMethod<'a>> {
    if !matches!(entry.declaration().source.kind(), SourceOriginKind::Authored(bytes) if bytes.as_ref() == source.as_bytes())
        || !matches!(
            entry.frame(),
            tcl_compiler::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
        )
    {
        return None;
    }
    let class = original_class(analysis, source, targets.1)?;
    let receiver_class = original_class(analysis, source, targets.0)?;
    let side = match entry.receiver() {
        SourceMethodReceiver::Instance => tcl_compiler::analyser::types::MemberSide::Instance,
        SourceMethodReceiver::Class => tcl_compiler::analyser::types::MemberSide::ClassObject,
    };
    let mut declarations = class.original_members.declarations().filter(|declaration| {
        declaration.side() == side
            && declaration.declaration().site() == entry.declaration()
            && declaration.metadata().name_span == entry.name_source().span
    });
    let metadata = declarations.next()?;
    let method = metadata.metadata();
    declarations
        .next()
        .is_none()
        .then_some(RetainedReceiverMethod {
            class,
            receiver_class,
            method,
            metadata,
            own_entry: None,
            receiver: entry.receiver(),
            selector,
            selector_input: selector_input.clone(),
            editable_selector,
        })
}

/// Resolve the original method declaration at this actual command, after argv.
pub(crate) fn method_at_command<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    command: &SegmentedCommand,
) -> Option<RetainedReceiverMethod<'a>> {
    let realm = analysis.retained_command_realm()?;
    let config = analysis.body_lexer_config?;
    let tokens = CommandTokens::from_segmented(&SourceMap::new(source), config, command);
    let head = tokens.word_exprs.first()?;
    let selector = command.argv.get(1)?.span;
    let offset = head.source().span.start();
    let binding = realm.invocation_at_source(command.name(), offset);
    if !matches!(binding.source_origin()?.kind(), SourceOriginKind::Authored(bytes) if bytes.as_ref() == source.as_bytes())
    {
        return None;
    }
    let method = binding.original_written_name_input(&tokens, 1)?;
    let editable_selector = tokens.word_exprs.get(1).is_some_and(|word| {
        use tcl_compiler::ir::{WordExpr, WordPart};
        let static_word = match word {
            WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => true,
            WordExpr::Template { parts, .. } => parts
                .iter()
                .all(|part| matches!(part, WordPart::Text { .. })),
            _ => false,
        };
        static_word
            && method.original_word_key().is_some()
            && source.as_bytes().get(selector.as_range()) == Some(method.bytes())
    });
    if let Some((class, entry, generation)) =
        binding.receiver_self_method_entry(analysis.resolved_registry()?)
    {
        return original_method(
            analysis,
            source,
            class,
            entry,
            (selector, &method, editable_selector),
            Some(generation),
        );
    }
    if head.sole_variable_substitution().is_some() {
        return realm
            .variable_accesses_for_invocation_args(offset)
            .iter()
            .filter_map(|access| binding.object_receiver_method_entry(access, head))
            .filter(|(_, entry, _)| entry.is_exported())
            .find_map(|(class, entry, generation)| {
                original_method(
                    analysis,
                    source,
                    class,
                    entry,
                    (selector, &method, editable_selector),
                    Some(generation),
                )
            });
    }
    if let Some((class, entry, generation)) = binding.frozen_object_receiver_method_entry(head) {
        if !entry.is_exported() {
            return None;
        }
        return original_method(
            analysis,
            source,
            class,
            entry,
            (selector, &method, editable_selector),
            Some(generation),
        );
    }
    if let Some((class, entry, generation)) = binding.named_object_receiver_method_entry() {
        if !entry.is_exported() {
            return None;
        }
        return original_method(
            analysis,
            source,
            class,
            entry,
            (selector, &method, editable_selector),
            Some(generation),
        );
    }
    let (class, entry) = binding.class_definition_method_entry()?;
    if !class.prepended.is_empty() {
        return None;
    }
    if !entry.is_exported() {
        return None;
    }
    original_method(
        analysis,
        source,
        class,
        entry,
        (selector, &method, editable_selector),
        None,
    )
}

/// Captured prefix names in independently selected deferred operands. The
/// original capture supplies declaration identity; registration supplies only
/// its role, never proof that a future callback executes this method.
pub(crate) fn captured_methods_at_command<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    command: &SegmentedCommand,
) -> Vec<RetainedReceiverMethod<'a>> {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    let Some(realm) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    let config = current.config();
    let mut tokens = CommandTokens::from_segmented(&SourceMap::new(source), config, command);
    let Some(head) = tokens.word_exprs.first() else {
        return Vec::new();
    };
    let binding = realm.invocation_at_source(command.name(), head.source().span.start());
    if !matches!(binding.source_origin().map(|origin| origin.kind()),
        Some(SourceOriginKind::Authored(bytes)) if bytes.as_ref() == source.as_bytes())
    {
        return Vec::new();
    }
    tokens.source_binding = Some(binding.clone());
    let Some(metadata) =
        tcl_compiler::registry_invocation::InvocationMetadataContext::for_analysis_input(
            current.registry(),
            analysis.resolved_input.as_ref(),
        )
    else {
        return Vec::new();
    };
    let Some(normal) =
        tcl_compiler::registry_invocation::normal_representation_invocation_with_metadata_context(
            current.registry(),
            Some(metadata),
            &tokens,
        )
    else {
        return Vec::new();
    };
    let Some(deferred) = normal.deferred_script_source_argument_indices() else {
        return Vec::new();
    };
    binding
        .captured_method_prefix_arguments()
        .filter_map(|(argument, prefix)| {
            if !deferred.contains(&argument)
                || !matches!(prefix.capture_site().source.kind(),
                SourceOriginKind::Authored(bytes) if bytes.as_ref() == source.as_bytes())
            {
                return None;
            }
            let entry = prefix.method_entry();
            let input = prefix.original_selector_input();
            let editable = input
                .original_word_key()
                .and_then(|key| key.display())
                .and_then(|name| {
                    let base =
                        tcl_compiler::command_binding::ExecutedScriptSource::literal_word_base(
                            source,
                            prefix.selector(),
                            name,
                            config,
                        )?;
                    Some(Span::new(
                        base,
                        base.checked_add(u32::try_from(name.len()).ok()?)?,
                    ))
                });
            original_method(
                analysis,
                source,
                prefix.receiver().class_target(),
                entry,
                (
                    editable.unwrap_or(prefix.selector().source().span),
                    input,
                    editable.is_some(),
                ),
                Some(prefix.receiver().dispatch_generation()),
            )
        })
        .collect()
}

/// Locate a written method selector with the shared executable-region walker.
/// The walker supplies source shape; the temporal receipt supplies identity.
pub(crate) fn method_at_cursor<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    offset: u32,
) -> Option<RetainedReceiverMethod<'a>> {
    if let Some(reference) = definition_reference_at_cursor(analysis, source, offset) {
        return original_definition_reference(analysis, source, reference);
    }
    let mut selected = None;
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if let Some(captured) = captured_methods_at_command(analysis, source, command)
                .into_iter()
                .find(|selected| selected.selector.as_range().contains(&(offset as usize)))
            {
                selected = Some(captured);
                return true;
            }
            if command
                .argv
                .get(1)
                .is_some_and(|word| word.span.start() <= offset && offset < word.span.end())
            {
                selected = method_at_command(analysis, source, command);
                return selected.is_some();
            }
            false
        },
    );
    selected
}

/// The original phase consumed this written metadata name. A known operand
/// with no method entry must not fall back to the document's later class map.
pub(crate) fn definition_reference_at_cursor<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    offset: u32,
) -> Option<&'a SourceDefinitionMethodReference> {
    let mut references = analysis
        .retained_command_realm()?
        .definition_method_reference_inventories()
        .filter_map(|(_, references)| references)
        .flatten()
        .filter(|reference| {
            matches!(reference.invocation().source.kind(), SourceOriginKind::Authored(bytes) if bytes.as_ref() == source.as_bytes())
                && reference.name_span().as_range().contains(&(offset as usize))
        });
    let first = references.next()?;
    references.next().is_none().then_some(first)
}

fn original_definition_reference<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    reference: &SourceDefinitionMethodReference,
) -> Option<RetainedReceiverMethod<'a>> {
    if !matches!(reference.invocation().source.kind(), SourceOriginKind::Authored(bytes) if bytes.as_ref() == source.as_bytes())
    {
        return None;
    }
    original_method_in_class(
        analysis,
        source,
        (reference.class(), reference.declaring_class()),
        reference.method_entry()?,
        reference.name_span(),
        reference.original_name_input(),
        reference.editable_name_span() == Some(reference.name_span()),
    )
}

/// Definition-phase uses belonging to one original method. Its own declaration
/// remains available to cursor navigation and is added by declaration consumers.
pub(crate) fn definition_method_reference_spans(
    analysis: &AnalysisResult,
    source: &str,
    class: &ClassDef,
    method: &MethodDef,
    receiver: SourceMethodReceiver,
) -> Vec<Span> {
    let Some(realm) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    realm
        .definition_method_reference_inventories()
        .filter_map(|(_, references)| references)
        .flatten()
        .filter(|reference| {
            reference
                .method_entry()
                .is_none_or(|entry| entry.declaration() != reference.invocation())
        })
        .filter_map(|reference| original_definition_reference(analysis, source, reference))
        .filter(|selected| {
            selected.class.name_span == class.name_span
                && selected.method.name_span == method.name_span
                && selected.receiver == receiver
        })
        .filter_map(|selected| selected.editable_selector())
        .collect()
}

/// Editable executed selectors belonging to one original declaration.
/// Receiver names and the document's final class map never select this set.
pub(crate) fn method_call_spans(
    analysis: &AnalysisResult,
    source: &str,
    class: &ClassDef,
    method: &MethodDef,
    receiver: SourceMethodReceiver,
) -> Vec<Span> {
    selected_method_reference_spans(
        analysis,
        source,
        MethodReferencePurpose::Dispatch,
        |selected| {
            selected.class.name_span == class.name_span
                && selected.method.name_span == method.name_span
                && selected.receiver == receiver
        },
    )
}

/// Calls on one original receiver class, including inherited member entries.
/// A workspace class-name candidate cannot replace this local allocation.
pub(crate) fn method_calls_on_receiver(
    analysis: &AnalysisResult,
    source: &str,
    class: &ClassDef,
    method: &str,
    receiver: SourceMethodReceiver,
) -> Vec<Span> {
    selected_method_reference_spans(
        analysis,
        source,
        MethodReferencePurpose::Dispatch,
        |selected| {
            selected.receiver_class.name_span == class.name_span
                && selected.method.name == method
                && selected.receiver == receiver
        },
    )
}

#[derive(Clone, Copy)]
enum MethodReferencePurpose {
    Dispatch,
    CapturedPrefix,
}

/// Original names frozen for deferred registration, independently of calls.
pub(crate) fn captured_method_reference_spans(
    analysis: &AnalysisResult,
    source: &str,
    class: &ClassDef,
    method: &MethodDef,
    receiver: SourceMethodReceiver,
) -> Vec<Span> {
    selected_method_reference_spans(
        analysis,
        source,
        MethodReferencePurpose::CapturedPrefix,
        |selected| {
            selected.class.name_span == class.name_span
                && selected.method.name_span == method.name_span
                && selected.receiver == receiver
        },
    )
}

fn selected_method_reference_spans(
    analysis: &AnalysisResult,
    source: &str,
    purpose: MethodReferencePurpose,
    accepts: impl Fn(&RetainedReceiverMethod<'_>) -> bool,
) -> Vec<Span> {
    let mut spans = Vec::new();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            let selected = match purpose {
                MethodReferencePurpose::Dispatch => method_at_command(analysis, source, command)
                    .into_iter()
                    .collect::<Vec<_>>(),
                MethodReferencePurpose::CapturedPrefix => {
                    captured_methods_at_command(analysis, source, command)
                }
            };
            for selected in selected {
                if accepts(&selected)
                    && let Some(span) = selected.editable_selector()
                {
                    spans.push(span);
                }
            }
            false
        },
    );
    spans.sort_unstable_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    spans
}

/// A selected method whose original selector lacks an editable literal.
/// This supplies a whole-rename hazard, never a replacement source span.
pub(crate) fn uneditable_method_selector(
    analysis: &AnalysisResult,
    source: &str,
    accepts: impl Fn(&RetainedReceiverMethod<'_>) -> bool,
) -> Option<Span> {
    let mut found = None;
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if let Some(selected) = method_at_command(analysis, source, command)
                && selected.editable_selector().is_none()
                && accepts(&selected)
            {
                found = Some(selected.selector);
                return true;
            }
            false
        },
    );
    found
}

/// Type navigation at the original object read, independently of later argv.
pub(crate) fn class_at_read<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    offset: u32,
) -> Option<&'a ClassDef> {
    let realm = analysis.retained_command_realm()?;
    let mut selected = None;
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            let Some(head) = command.argv.first() else {
                return false;
            };
            for access in realm.variable_accesses_for_invocation_args(head.span.start()) {
                if access.source.span.start() <= offset && offset < access.source.span.end() {
                    selected = access
                        .proved_object_instance()
                        .and_then(|proof| original_class(analysis, source, proof.class_target()));
                    return true;
                }
            }
            false
        },
    );
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        Analyser::new().analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn original_own_object_navigation_joins_the_actual_configuration_allocation() {
        // Implementation contract: naming.tcloo.original-own-object-method-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-own-object-method-transfer.md
        let source = r"oo::class create C {method shared {} {}}; C create object; oo::objdefine object {method shared {} {}; method p\uD800 {} {}; method p\uD801 {} {}}; object p\uD800; object p\uD801";
        let mut analysis = analyse(source);
        analysis.all_classes.clear();
        for spelling in [r"p\uD800", r"p\uD801"] {
            let offset =
                u32::try_from(source.rfind(&format!("object {spelling}")).unwrap()).unwrap();
            let command = tcl_compiler::segmenter::segment_commands(source)
                .into_iter()
                .find(|command| command.span.start() == offset)
                .unwrap();
            let selected = method_at_command(&analysis, source, &command).expect(spelling);
            assert_eq!(
                selected.original_selector_input().bytes(),
                if spelling == r"p\uD800" {
                    b"p\xed\xa0\x80"
                } else {
                    b"p\xed\xa0\x81"
                }
            );
            assert!(selected.method.name_span.start() < offset);
            assert_eq!(selected.receiver, SourceMethodReceiver::Class);
        }
    }

    #[test]
    fn original_receiver_navigation_joins_allocations_without_reporting_maps() {
        for class in ["C", r"C\uD800", r"C\uD801"] {
            let source = format!(
                "oo::class create {class} {{self method pick {{}} {{return ORIGINAL}}}}; {class} pick"
            );
            let mut analysis = analyse(&source);
            assert_eq!(analysis.original_class_declarations().count(), 1);
            analysis.all_classes.clear();
            analysis.superseded_classes.clear();
            let cursor = u32::try_from(source.rfind("pick").unwrap()).unwrap();
            let selected = method_at_cursor(&analysis, &source, cursor)
                .expect("an actual method entry must join its original class allocation");
            assert_eq!(
                selected.class.name_span,
                analysis
                    .original_class_declarations()
                    .next()
                    .unwrap()
                    .metadata()
                    .name_span
            );
            assert_eq!(
                selected.method.name_span.start(),
                u32::try_from(source.find("pick").unwrap()).unwrap()
            );
        }
    }

    #[test]
    fn captured_prefix_references_require_actual_capture_and_deferred_registration() {
        for builder in ["[list [self] tick]", "[namespace code [list my tick]]"] {
            let source = format!(
                "oo::class create C {{method tick {{}} {{return OK}}; method setup {{}} {{set cb {builder}; after 0 $cb}}}}; C create obj; obj setup"
            );
            let analysis = analyse(&source);
            let cursor = u32::try_from(source.find("tick]").unwrap()).unwrap();
            let selected = method_at_cursor(&analysis, &source, cursor).unwrap();
            assert_eq!(selected.method.name, "tick");
            assert_eq!(
                selected.editable_selector(),
                Some(Span::new(cursor, cursor + 4))
            );
            assert_eq!(
                captured_method_reference_spans(
                    &analysis,
                    &source,
                    selected.class,
                    selected.method,
                    selected.receiver
                ),
                vec![Span::new(cursor, cursor + 4)]
            );
            assert!(
                method_call_spans(
                    &analysis,
                    &source,
                    selected.class,
                    selected.method,
                    selected.receiver
                )
                .is_empty(),
                "registration supplies no future dispatch"
            );
            let index = tcl_lexer::LineIndex::new(&source);
            assert_eq!(
                crate::definition::definition(&source, 0, cursor, &analysis),
                vec![crate::definition::span_to_range(
                    &source,
                    &index,
                    selected.method.name_span
                )]
            );
            assert!(crate::rename::prepare_rename(&source, 0, cursor, &analysis).is_some());
            let edits = crate::rename::rename(
                &source,
                analysis.resolved_profile().unwrap(),
                0,
                cursor,
                "tock",
                &analysis,
                None,
            );
            assert!(!edits.is_empty(), "{source}");
        }
        for (registration, reached) in [
            ("after 0 $cb", false),
            ("puts $cb", true),
            ("set cb {OTHER tick}; after 0 $cb", true),
            (
                "oo::define C method tick {} {return NEW}; after 0 $cb",
                true,
            ),
            ("unknownOperation; after 0 $cb", true),
        ] {
            let source = format!(
                "oo::class create C {{method tick {{}} {{}}; method setup {{}} {{set cb [list [self] tick]; {registration}}}}}; {}",
                if reached {
                    "C create obj; obj setup"
                } else {
                    ""
                }
            );
            let analysis = analyse(&source);
            let cursor = u32::try_from(source.find("tick]").unwrap()).unwrap();
            assert!(
                method_at_cursor(&analysis, &source, cursor).is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn external_navigation_respects_the_retained_visibility_phase() {
        for (directive, visible) in [("", false), ("export Ping", true)] {
            let source = format!(
                "oo::class create C {{method Ping {{}} {{return ORIGINAL}}; {directive}}}; C create obj; obj Ping"
            );
            let analysis = analyse(&source);
            let cursor = u32::try_from(source.rfind("Ping").unwrap()).unwrap();
            assert_eq!(
                method_at_cursor(&analysis, &source, cursor).is_some(),
                visible
            );
        }
        let source = "oo::class create C {method Ping {} {return ORIGINAL}; method run {} {my Ping}}; C create obj; obj run";
        let analysis = analyse(source);
        let cursor = u32::try_from(source.find("my Ping").unwrap() + 3).unwrap();
        assert!(method_at_cursor(&analysis, source, cursor).is_some());
    }

    #[test]
    fn definition_metadata_names_use_the_original_phase_for_all_consumers() {
        let source = "oo::class create C {method Ping {} {return ORIGINAL}; export Ping}; C create obj; obj Ping";
        let analysis = analyse(source);
        let cursor = u32::try_from(source.find("export Ping").unwrap() + 7).unwrap();
        let reference = definition_reference_at_cursor(&analysis, source, cursor).unwrap();
        assert!(reference.method_entry().is_some());
        let selected = method_at_cursor(&analysis, source, cursor).unwrap();
        let line_index = tcl_lexer::LineIndex::new(source);
        assert_eq!(
            crate::definition::definition(source, 0, cursor, &analysis),
            vec![crate::definition::span_to_range(
                source,
                &line_index,
                selected.method.name_span
            )]
        );
        assert!(
            crate::hover::hover(source, 0, cursor, &analysis, None)
                .unwrap()
                .value
                .contains("::C::Ping")
        );
        let refs = crate::references::references(
            source,
            analysis.resolved_profile().unwrap(),
            0,
            cursor,
            &analysis,
            true,
        );
        assert_eq!(refs.len(), 3, "{refs:?}");
        assert_eq!(
            crate::references::document_highlights(
                source,
                analysis.resolved_profile().unwrap(),
                0,
                cursor,
                &analysis
            )
            .len(),
            3
        );
        assert_eq!(
            method_call_spans(
                &analysis,
                source,
                selected.class,
                selected.method,
                selected.receiver
            )
            .len(),
            1,
            "metadata is not a reached method call"
        );
        assert!(crate::rename::prepare_rename(source, 0, cursor, &analysis).is_some());
        let edits = crate::rename::rename(
            source,
            analysis.resolved_profile().unwrap(),
            0,
            cursor,
            "Pong",
            &analysis,
            None,
        );
        assert_eq!(edits.len(), 3, "{edits:?}");
    }

    #[test]
    fn export_before_declaration_cannot_borrow_the_later_method() {
        let source = "proc Ping {} {return DECOY}; oo::class create C {export Ping; method Ping {} {return ORIGINAL}}";
        let analysis = analyse(source);
        let cursor = u32::try_from(source.find("export Ping").unwrap() + 7).unwrap();
        let reference = definition_reference_at_cursor(&analysis, source, cursor).unwrap();
        assert!(reference.method_entry().is_none());
        assert!(method_at_cursor(&analysis, source, cursor).is_none());
        assert!(crate::definition::definition(source, 0, cursor, &analysis).is_empty());
        assert!(
            crate::references::references(
                source,
                analysis.resolved_profile().unwrap(),
                0,
                cursor,
                &analysis,
                true
            )
            .is_empty()
        );
        assert!(crate::hover::hover(source, 0, cursor, &analysis, None).is_none());
        assert!(crate::rename::prepare_rename(source, 0, cursor, &analysis).is_none());
        assert!(
            crate::rename::rename_with_diagnosis(
                source,
                analysis.resolved_profile().unwrap(),
                0,
                cursor,
                "Pong",
                &analysis,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn original_receiver_entry_is_shared_by_navigation_hover_and_rename() {
        for (source, receiver) in [
            (
                "oo::class create C {self method ping {} {return CLASS}}; C ping",
                SourceMethodReceiver::Class,
            ),
            (
                "oo::class create C {method ping {} {return INSTANCE}}; set obj [C new]; $obj ping",
                SourceMethodReceiver::Instance,
            ),
            (
                "oo::class create C {method ping {} {return INSTANCE}}; C create obj; obj ping",
                SourceMethodReceiver::Instance,
            ),
        ] {
            let analysis = analyse(source);
            let cursor = u32::try_from(source.rfind("ping").unwrap()).unwrap();
            let selected = method_at_cursor(&analysis, source, cursor).unwrap();
            assert_eq!(selected.receiver, receiver);
            let expected = crate::definition::span_to_range(
                source,
                &tcl_lexer::LineIndex::new(source),
                selected.method.name_span,
            );
            assert_eq!(
                crate::definition::definition(source, 0, cursor, &analysis),
                vec![expected]
            );
            let hover = crate::hover::hover(source, 0, cursor, &analysis, None).unwrap();
            assert!(hover.value.contains("::C::ping"), "{}", hover.value);
            let edits = crate::rename::rename(
                source,
                analysis.resolved_profile().unwrap(),
                0,
                cursor,
                "pong",
                &analysis,
                None,
            );
            assert_eq!(edits.len(), 2, "{edits:?}");
            assert!(edits.iter().all(|edit| edit.new_text == "pong"));
        }
    }

    #[test]
    fn computed_method_selectors_report_identity_without_partial_rename_edits() {
        let source = "oo::class create C {method ping {} {return INSTANCE}}; set obj [C new]; set selected ping; $obj $selected";
        let analysis = analyse(source);
        let cursor = u32::try_from(source.rfind("$selected").unwrap()).unwrap();
        let selected = method_at_cursor(&analysis, source, cursor).unwrap();
        assert_eq!(selected.method.name, "ping");
        assert!(selected.editable_selector().is_none());
        let declaration = selected.method.name_span.start();
        assert!(
            crate::rename::rename_with_diagnosis(
                source,
                analysis.resolved_profile().unwrap(),
                0,
                declaration,
                "pong",
                &analysis,
                None,
            )
            .is_err(),
            "a computed selector cannot be omitted from a supposedly complete method rename",
        );
    }

    #[test]
    fn internal_receiver_navigation_requires_the_actual_receiver_and_dispatch_epoch() {
        let source = "oo::class create Base {method ping {} {return BASE}}; oo::class create Child {superclass Base; method run {} {my ping}}; Child create obj; obj run";
        let analysis = analyse(source);
        let cursor = u32::try_from(source.find("my ping").unwrap() + 3).unwrap();
        let selected = method_at_cursor(&analysis, source, cursor).unwrap();
        assert_eq!(selected.class.qualified_name, "::Base");
        assert_eq!(selected.method.name, "ping");
        assert!(selected.editable_selector().is_some());
        assert_eq!(
            crate::definition::definition(source, 0, cursor, &analysis),
            vec![crate::definition::span_to_range(
                source,
                &tcl_lexer::LineIndex::new(source),
                selected.method.name_span,
            )],
        );
        let deferred = "oo::class create C {method ping {} {return C}; method run {} {my ping}}";
        let analysis = analyse(deferred);
        let cursor = u32::try_from(deferred.find("my ping").unwrap() + 3).unwrap();
        assert!(method_at_cursor(&analysis, deferred, cursor).is_none());
        assert!(crate::definition::definition(deferred, 0, cursor, &analysis).is_empty());
    }

    #[test]
    fn frozen_self_heads_use_the_reached_receiver_instead_of_the_lexical_class() {
        let source = "oo::class create Base {method ping {} {return BASE}}; oo::class create Child {superclass Base; method run {} {[self] ping}}; Child create obj; obj run";
        let analysis = analyse(source);
        let cursor = u32::try_from(source.find("[self] ping").unwrap() + 7).unwrap();
        let selected = method_at_cursor(&analysis, source, cursor).unwrap();
        assert_eq!(selected.class.qualified_name, "::Base");
        assert_eq!(selected.method.name, "ping");
        assert!(selected.editable_selector().is_some());
        assert_eq!(
            method_call_spans(
                &analysis,
                source,
                selected.class,
                selected.method,
                selected.receiver
            ),
            vec![selected.selector]
        );
        let deferred =
            "oo::class create C {method ping {} {return C}; method run {} {[self] ping}}";
        let analysis = analyse(deferred);
        let cursor = u32::try_from(deferred.find("[self] ping").unwrap() + 7).unwrap();
        assert!(method_at_cursor(&analysis, deferred, cursor).is_none());
        for source in [
            "oo::class create C {method ping {x} {return C}; method run {} {[self] ping [oo::define C method ping {x} {return REPLACED}]}}; C create obj; obj run",
            "oo::class create C {method ping {} {return C}; method run {} {rename my retired; proc my args {return REPLACED}; [self] ping}}; C create obj; obj run",
        ] {
            let analysis = analyse(source);
            let cursor = u32::try_from(source.find("[self] ping").unwrap() + 7).unwrap();
            assert!(method_at_cursor(&analysis, source, cursor).is_none());
        }
    }

    #[test]
    fn original_method_reference_sets_do_not_merge_reused_class_names() {
        let source = "oo::class create C {method ping {} {return FIRST}}; C create first; first ping; rename C old; oo::class create C {method ping {} {return SECOND}}; C create second; second ping";
        let analysis = analyse(source);
        let first = u32::try_from(source.find("first ping").unwrap() + 6).unwrap();
        let second = u32::try_from(source.find("second ping").unwrap() + 7).unwrap();
        let original = method_at_cursor(&analysis, source, first).unwrap_or_else(|| {
            let binding = analysis
                .retained_command_realm()
                .unwrap()
                .invocation_at_source("first", first - 6);
            eprintln!(
                "historical class spans={:?} superseded={:?} head={:?} named_method={:?}",
                analysis.all_classes.get("::C").map(|class| class.name_span),
                analysis.superseded_classes.get("::C").map(|classes| classes
                    .iter()
                    .map(|class| class.name_span)
                    .collect::<Vec<_>>()),
                binding.evaluated_command_word(),
                binding
                    .named_object_receiver_method_entry()
                    .map(|(class, entry, generation)| (
                        &class.command,
                        entry.name_source(),
                        entry.declaring_class(),
                        generation
                    )),
            );
            panic!("the historical method must retain its original declaration");
        });
        let replacement = method_at_cursor(&analysis, source, second).unwrap();
        assert_ne!(original.class.name_span, replacement.class.name_span);
        assert_ne!(original.method.name_span, replacement.method.name_span);
        for selected in [original, replacement] {
            assert_eq!(
                method_call_spans(
                    &analysis,
                    source,
                    selected.class,
                    selected.method,
                    selected.receiver
                ),
                vec![selected.selector]
            );
            let (declaration, calls) = crate::references::method_references_for_declaration(
                source,
                analysis.resolved_profile().unwrap(),
                &analysis,
                selected.class,
                selected.method,
                false,
            );
            assert_eq!(declaration, selected.method.name_span);
            assert_eq!(calls, vec![selected.selector]);
        }
    }

    #[test]
    fn argument_mutation_withdraws_all_external_member_identity() {
        for source in [
            "oo::class create C {self method ping {x} {return CLASS}}; C ping [oo::objdefine C method ping {x} {return REPLACED}]",
            "oo::class create C {method ping {x} {return INSTANCE}}; set obj [C new]; $obj ping [oo::define C method ping {x} {return REPLACED}]",
            "oo::class create C {method ping {x} {return INSTANCE}}; C create obj; obj ping [oo::define C method ping {x} {return REPLACED}]",
        ] {
            let analysis = analyse(source);
            let cursor = u32::try_from(source.find("ping [").unwrap()).unwrap();
            assert!(method_at_cursor(&analysis, source, cursor).is_none());
            assert_eq!(
                crate::definition::definition(source, 0, cursor, &analysis).len(),
                0
            );
            assert_eq!(
                crate::rename::rename(
                    source,
                    analysis.resolved_profile().unwrap(),
                    0,
                    cursor,
                    "pong",
                    &analysis,
                    None,
                )
                .len(),
                0
            );
        }
    }

    #[test]
    fn object_type_navigation_uses_the_original_read_allocation() {
        let source = "oo::class create C {method ping {x} {return INSTANCE}}; set obj [C new]; $obj ping [oo::define C method ping {x} {return REPLACED}]";
        let analysis = analyse(source);
        let cursor = u32::try_from(source.find("$obj").unwrap() + 1).unwrap();
        let class = class_at_read(&analysis, source, cursor).unwrap();
        let expected = crate::definition::span_to_range(
            source,
            &tcl_lexer::LineIndex::new(source),
            class.name_span,
        );
        assert_eq!(
            crate::type_definition::type_definition(source, 0, cursor, &analysis),
            vec![expected]
        );
        let selector = u32::try_from(source.find("ping [").unwrap()).unwrap();
        assert!(method_at_cursor(&analysis, source, selector).is_none());
    }

    #[test]
    fn nominal_class_names_and_display_dialect_cannot_replace_actual_receipts() {
        let source = "oo::class create C {self method ping {} {return CLASS}}; rename C old; proc C args {return DECOY}; C ping";
        let mut analysis = analyse(source);
        analysis.dialect.clear();
        let cursor = u32::try_from(source.rfind("ping").unwrap()).unwrap();
        assert!(method_at_cursor(&analysis, source, cursor).is_none());
        assert_eq!(
            crate::definition::definition(source, 0, cursor, &analysis).len(),
            0
        );
    }
    #[test]
    fn captured_method_registration_keeps_actual_metadata_and_source_currency() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        let source = "oo::class create C {method tick {} {}; method setup {} {set cb [list [self] tick]; after 0 $cb}}; C create obj; obj setup";
        let baseline = analyse(source);
        let config = baseline.body_lexer_config.unwrap();
        let offset = u32::try_from(source.find("after 0 $cb").unwrap()).unwrap();
        let command = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            "after 0 $cb",
            offset,
            config,
        )
        .remove(0);
        assert_eq!(
            captured_methods_at_command(&baseline, source, &command).len(),
            1
        );
        let mut missing = baseline.clone();
        missing.resolved_input = None;
        assert!(captured_methods_at_command(&missing, source, &command).is_empty());
        assert!(captured_methods_at_command(&baseline, &format!("{source} "), &command).is_empty());
        let mut changed = baseline.clone();
        changed.body_lexer_config = Some(tcl_lexer::LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        });
        assert!(captured_methods_at_command(&changed, source, &command).is_empty());
        let input = baseline.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let mut foreign = baseline.clone();
        foreign.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            std::sync::Arc::new(context.with_command_store(std::sync::Arc::new(
                tcl_registry::CommandRegistry::build_default(),
            ))),
            config,
        ));
        assert!(captured_methods_at_command(&foreign, source, &command).is_empty());
        let mut availability = baseline.clone();
        availability.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry(),
            config,
        ));
        assert!(captured_methods_at_command(&availability, source, &command).is_empty());
    }
}
