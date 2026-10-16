// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional source selectors joined to original member declarations.

use super::original_instance_metadata_order_for_inventory;
use crate::analyser::types::{MemberSide, OriginalSourceMethodMetadata};
use crate::analyser::{AnalysisResult, ClassDef};
use crate::command_binding::OriginalSourceCommandTransitionAdvice;
use crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody;
use crate::signature_scan::original_name::{SignatureSourceNameKey, SourceDeclarationMetadata};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_lexer::{SourceImage, Span};
use tcl_registry::definer::{
    CallableRole, DefinitionMemberNamePurpose, MemberEffect, MemberReceiver,
};

/// Independent applicability debts of a readonly source correspondence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalSourceSelfMethodReferenceObligation {
    /// Source declarations have not been proved successfully installed.
    DeclarationApplicability,
    /// The original member body has not been proved entered.
    MemberBodyEntry,
    /// No current receiver allocation or method frame is supplied.
    ReceiverApplicability,
    /// The selected source helper does not prove actual method dispatch.
    MethodLookupApplicability,
    /// Source roles do not close command, execution or variable observers.
    ObserverApplicability,
    /// Declared source relations do not prove a runtime inheritance graph.
    InheritanceApplicability,
}

/// One possible direct self selector under its original member-body issuer.
/// This retains canonical source correspondence, never a receiver entry,
/// invocation frame, complete occurrence census or replacement authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceSelfMethodReference {
    declaring_class: SourceDeclarationMetadata<ClassDef>,
    method: OriginalSourceMethodMetadata,
    body_class: SourceDeclarationMetadata<ClassDef>,
    body_method: OriginalSourceMethodMetadata,
    body: OriginalSourceDefinitionMemberScriptBody,
    body_correspondence:
        crate::registry_invocation::OriginalSourceDefinitionMemberBodyCorrespondence,
    selector: SignatureSourceNameInput,
    selector_span: Span,
    source_schema: OriginalSourceCommandTransitionAdvice,
    obligations: Vec<OriginalSourceSelfMethodReferenceObligation>,
}

impl OriginalSourceSelfMethodReference {
    /// Exact canonical source class publishing the selected declaration.
    #[must_use]
    pub const fn declaring_class(&self) -> &SourceDeclarationMetadata<ClassDef> {
        &self.declaring_class
    }
    /// Canonical member input, declaration site, formals and body.
    #[must_use]
    pub const fn method(&self) -> &OriginalSourceMethodMetadata {
        &self.method
    }
    /// Source class whose original callable body contains the selector.
    #[must_use]
    pub const fn body_class(&self) -> &SourceDeclarationMetadata<ClassDef> {
        &self.body_class
    }
    /// Exact original caller member, including its formals and definition parent.
    #[must_use]
    pub const fn body_method(&self) -> &OriginalSourceMethodMetadata {
        &self.body_method
    }
    /// Genuine member script issuer; it supplies no entered method frame.
    #[must_use]
    pub const fn body(&self) -> &OriginalSourceDefinitionMemberScriptBody {
        &self.body
    }
    /// Separate conditional catalogue issuer, with its applicability intact.
    #[must_use]
    pub fn catalogue_body(&self) -> &OriginalSourceDefinitionMemberScriptBody {
        crate::registry_invocation::OriginalSourceDefinitionMemberBodyCorrespondence::catalogue_body(
            &self.body_correspondence,
        )
    }
    /// Original selector naming input; reported strings cannot replace it.
    #[must_use]
    pub const fn selector_input(&self) -> &SignatureSourceNameInput {
        &self.selector
    }
    /// Original readonly selector extent; this is not an edit recipe.
    #[must_use]
    pub const fn selector_span(&self) -> Span {
        self.selector_span
    }
    /// The original command keeps its selected schema and unknown-source debts.
    #[must_use]
    pub const fn source_schema(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.source_schema
    }
    /// Successful declarations, entry, receiver, lookup and observers remain open.
    #[must_use]
    pub fn obligations(&self) -> &[OriginalSourceSelfMethodReferenceObligation] {
        &self.obligations
    }
}

/// Partial positive source occurrences from authentic member-body catalogues.
/// Missing whole-source/input ownership returns None. A present vector still
/// contains partial positives only; missing or conflicting correspondence yields
/// no row, and an empty vector supplies no census, absence or dispatch conclusion.
#[must_use]
pub fn original_source_self_method_references(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<OriginalSourceSelfMethodReference>> {
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let image = SourceImage::document(source);
    let realm = analysis.retained_command_realm()?;
    if analysis.body_lexer_config != Some(config)
        || !analysis.matches_original_source_image(&image, config)
        || !realm.matches_resolved_analysis_input(input)
    {
        return None;
    }
    let context = input.context_registry();
    let tape = realm
        .source_bindings_ref()
        .original_source_transition_advice_tape(&context)?;
    let records = analysis.original_class_declarations().collect::<Vec<_>>();
    let occupied = analysis
        .original_procedure_declarations()
        .map(SourceDeclarationMetadata::name)
        .collect::<Vec<_>>();
    let mut result = tape
        .original_definition_member_source_catalogue()
        .filter_map(|advice| capture(source, analysis, &records, &occupied, advice))
        .collect::<Vec<_>>();
    result.sort_by_key(|reference| {
        (
            reference.selector_span.start(),
            reference.selector_span.end(),
        )
    });
    result.dedup();
    Some(result)
}

fn capture(
    source: &str,
    analysis: &AnalysisResult,
    records: &[&SourceDeclarationMetadata<ClassDef>],
    occupied: &[&crate::signature_scan::scope::SignatureSourceCommand],
    advice: &OriginalSourceCommandTransitionAdvice,
) -> Option<OriginalSourceSelfMethodReference> {
    let (body, selector_word) = checked_member_source_schema(source, analysis, advice)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let (body_index, body_method, body_correspondence) =
        original_body_owner(&context, &body, advice, records)?;
    let body = crate::registry_invocation::OriginalSourceDefinitionMemberBodyCorrespondence::declaration_body(&body_correspondence).clone();
    let key = SignatureSourceNameKey::from_original_native_word(
        &selector_word,
        tcl_syntax::word_rules::WordValueRules::from_config(&selector_word.config()),
        body_method.original_name_input().policy(),
    )?;
    let selector = SignatureSourceNameInput::OriginalWord(key);
    let order = original_instance_metadata_order_for_inventory(records, body_index, occupied)?;
    for index in order {
        let class = *records.get(index)?;
        let mut methods = class
            .metadata()
            .original_members
            .methods(MemberSide::Instance)?
            .into_iter()
            .filter(|method| {
                method.name_purpose() == DefinitionMemberNamePurpose::TclOoMethod
                    && method.original_name_input().policy() == selector.policy()
                    && method.original_name_input().bytes() == selector.bytes()
                    && method
                        .name_purpose()
                        .admits(selector.policy().recipe(), selector.bytes())
            });
        let Some(method) = methods.next() else {
            continue;
        };
        if methods.next().is_some() {
            return None;
        }
        return Some(OriginalSourceSelfMethodReference {
            declaring_class: (*class).clone(),
            method,
            body_class: (*records[body_index]).clone(),
            body_method: body_method.clone(),
            body,
            body_correspondence,
            selector,
            selector_span: selector_word.content_span().ok()?,
            source_schema: advice.clone(),
            obligations: vec![
                OriginalSourceSelfMethodReferenceObligation::DeclarationApplicability,
                OriginalSourceSelfMethodReferenceObligation::MemberBodyEntry,
                OriginalSourceSelfMethodReferenceObligation::ReceiverApplicability,
                OriginalSourceSelfMethodReferenceObligation::MethodLookupApplicability,
                OriginalSourceSelfMethodReferenceObligation::ObserverApplicability,
                OriginalSourceSelfMethodReferenceObligation::InheritanceApplicability,
            ],
        });
    }
    None
}

fn checked_member_source_schema(
    source: &str,
    analysis: &AnalysisResult,
    advice: &OriginalSourceCommandTransitionAdvice,
) -> Option<(
    OriginalSourceDefinitionMemberScriptBody,
    tcl_lexer::NativeWord,
)> {
    use crate::registry_invocation::source_structure::source_transition_words_from_advice;
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let image = SourceImage::document(source);
    let context = input.context_registry();
    let realm = analysis.retained_command_realm()?;
    let body = advice.original_definition_member_source_body()?;
    if !advice.matches_source(&image, config)
        || !advice.matches_context(&context)
        || !body.matches_source(&image, config)
        || !body.matches_context(&context)
        || body.receiver_grammar(&context)?.family != tcl_registry::definer::DefinerFamily::TclOo
        || !matches!(
            body.member_row()?.effect,
            MemberEffect::Callable {
                role: CallableRole::Method,
                ..
            }
        )
        || body.member_row()?.receiver != MemberReceiver::Instance
    {
        return None;
    }
    let source_words =
        source_transition_words_from_advice(source, config, &context, advice.clone())?;
    if source_words.with_source_schema(&context, |schema| {
        schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
            && schema.facts().arity_accepts_frozen_arguments() == Some(true)
    }) != Some(true)
    {
        return None;
    }
    let checked =
        realm.original_member_reference_source_catalogue_advice(input, advice.original_words())?;
    if &checked != advice {
        return None;
    }
    let crate::registry_invocation::InvocationWordOrigin::Written(written) =
        source_words.origins().get(1)?
    else {
        return None;
    };
    let selector = advice.original_words().get(*written)?;
    if advice.arguments().first()?.original != *selector || selector.group().expand {
        return None;
    }
    Some((body.clone(), selector.clone()))
}

fn original_body_owner<'a>(
    context: &tcl_registry::model::ContextRegistry,
    body: &OriginalSourceDefinitionMemberScriptBody,
    advice: &OriginalSourceCommandTransitionAdvice,
    records: &[&'a SourceDeclarationMetadata<ClassDef>],
) -> Option<(
    usize,
    &'a OriginalSourceMethodMetadata,
    crate::registry_invocation::OriginalSourceDefinitionMemberBodyCorrespondence,
)> {
    let member_region = body
        .original_parent_body()
        .definition_member_region(context, body.original_parent_body().content_span())?;
    let original =
        member_region.original_command_words_at(body.original_words().first()?.span().start())?;
    if original != body.original_words() {
        return None;
    }
    let parameters =
        member_region.argument_words_for_role(original, tcl_registry::ArgRole::ParamList)?;
    let [parameters] = parameters.as_slice() else {
        return None;
    };
    let mut owners = records
        .iter()
        .copied()
        .enumerate()
        .flat_map(|(index, class)| {
            class
                .metadata()
                .original_members
                .declarations()
                .filter_map(move |method| {
                    (method.side() == MemberSide::Instance
                        && method.name_purpose() == DefinitionMemberNamePurpose::TclOoMethod
                        && method.body_word() == Some(body.original_container())
                        && method.parameters_word() == Some(*parameters)
                        && method.declaration().site().offset == original.first()?.span().start()
                        && method.declaration().site().source == class.declaration_site().source
                        && method.declaration().site().source == advice.site().source
                        && !method.metadata().params_computed
                        && !method.native_class_delegate()
                        && method.forward_prefix().is_none())
                    .then_some(())?;
                    let correspondence = body.original_source_correspondence_in_parent(
                        context,
                        method.definition_parent()?,
                    )?;
                    Some((index, method, correspondence))
                })
        });
    let first = owners.next()?;
    owners.next().is_none().then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn readonly_self_selectors_retain_own_and_inherited_source_members() {
        // naming.editor.original-method-candidates
        // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
        for dialect in ["tcl8.6", "tcl9.0"] {
            for (source, expected) in [
                (
                    "oo::class create C {method ping {} {}; method body {} {my ping; my ping}}",
                    2,
                ),
                (
                    "oo::class create Base {method ping {} {}}; oo::class create Child {superclass Base; method body {} {my ping}}",
                    1,
                ),
            ] {
                let analysis = Analyser::new().analyse(source, dialect);
                let references = original_source_self_method_references(source, &analysis)
                    .expect("current partial source catalogue");
                assert_eq!(references.len(), expected, "{dialect}: {source}");
                for reference in references {
                    assert_eq!(&source[reference.selector_span().as_range()], "ping");
                    assert_eq!(reference.method().original_name_input().bytes(), b"ping");
                    assert_ne!(
                        reference.method().declaration().site(),
                        reference.body_method.declaration().site()
                    );
                    assert!(reference.source_schema().logical_source_input().is_none());
                    assert!(reference.source_schema().obligations().contains(
                        &crate::command_binding::SourceCommandTransitionObligation::ConditionalMemberBodyCatalogueApplicability
                    ));
                    assert!(reference.obligations().contains(
                        &OriginalSourceSelfMethodReferenceObligation::ObserverApplicability
                    ));
                    assert!(reference.obligations().contains(
                        &OriginalSourceSelfMethodReferenceObligation::InheritanceApplicability
                    ));
                    assert_eq!(
                        reference.body_method.body_word(),
                        Some(reference.body.original_container())
                    );
                    assert_eq!(
                        reference.body_method.definition_parent(),
                        Some(reference.body.original_parent_body())
                    );
                }
            }
        }
    }

    #[test]
    fn readonly_self_selectors_refuse_unowned_wrong_context_and_ambiguous_sources() {
        // naming.editor.original-method-candidates
        // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
        for source in [
            "oo::class create C {method ping {} {}; method body {} {ping}}",
            "oo::class create C {method ping {} {}; method body {} {::my ping}}",
            "oo::class create C {method ping {} {}; method body {} {my $unknown}}",
            "oo::class create C {method ping {} {}; method body {} {proc nested {} {my ping}}}",
            "oo::class create C {method ping {} {}}; proc outside {} {my ping}",
            "oo::class create Child {superclass $unknown; method body {} {my ping}}",
            "oo::class create Base {method ping {} {}}; oo::class create Child {superclass Missing; method body {} {my ping}}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            assert!(
                original_source_self_method_references(source, &analysis)
                    .is_none_or(|references| references.is_empty()),
                "{source}"
            );
        }
        let source = "oo::class create C {method ping {} {}; method body {} {my ping}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(
            original_source_self_method_references(source, &analysis)
                .unwrap()
                .len(),
            1
        );
        assert!(
            original_source_self_method_references(&source.replace("ping", "pong"), &analysis)
                .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(original_source_self_method_references(source, &missing).is_none());
        let mut foreign = analysis.clone();
        foreign.resolved_input = Analyser::new()
            .analyse("puts foreign", "tcl9.0")
            .resolved_input;
        assert!(original_source_self_method_references(source, &foreign).is_none());
        let mut stale = analysis.clone();
        stale.body_lexer_config.as_mut().unwrap().leading_bom = tcl_lexer::LeadingBom::Skip;
        assert!(original_source_self_method_references(source, &stale).is_none());
        let mut unowned = analysis.clone();
        for class in unowned.all_classes.values_mut() {
            class.name_span = Span::new(0, 0);
        }
        assert_eq!(
            original_source_self_method_references(source, &unowned)
                .unwrap()
                .len(),
            1,
            "report-map corruption cannot alter independently retained original correspondence"
        );
    }
}

#[cfg(test)]
mod original_source_correspondence_tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn readonly_member_correspondence_retains_both_issuers_and_strong_query_result() {
        // naming.editor.original-method-candidates
        // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
        let source = "oo::class create C {method ping {} {}; method body {} {my ping}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let input = analysis.resolved_input.as_ref().unwrap();
        let realm = analysis.retained_command_realm().unwrap();
        let offset = source.find("my ping").unwrap() as u32;
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            &source[offset as usize..offset as usize + 7],
            offset,
            input.lexer_config(),
        )
        .pop()
        .unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &SourceImage::document(source).source_map(),
            input.lexer_config(),
            &segment,
        );
        realm.stamp_original_tokens(&mut tokens);
        let stronger_before = realm.original_member_source_catalogue_advice(input, &tokens);
        let references = original_source_self_method_references(source, &analysis).unwrap();
        let [reference] = references.as_slice() else {
            panic!("one actual readonly source selector")
        };
        assert_eq!(
            reference.body_method().definition_parent(),
            Some(reference.body().original_parent_body())
        );
        assert_eq!(
            reference.catalogue_body(),
            reference
                .source_schema()
                .original_definition_member_source_body()
                .unwrap()
        );
        assert!(reference.source_schema().obligations().contains(&crate::command_binding::SourceCommandTransitionObligation::ConditionalMemberBodyCatalogueApplicability));
        assert_eq!(
            realm.original_member_source_catalogue_advice(input, &tokens),
            stronger_before
        );
        let reborrowed = realm
            .original_member_reference_source_catalogue_advice(
                input,
                reference.source_schema().original_words(),
            )
            .unwrap();
        assert_eq!(&reborrowed, reference.source_schema());
        let foreign = Analyser::new().analyse(source, "tcl9.0");
        assert!(
            realm
                .original_member_reference_source_catalogue_advice(
                    foreign.resolved_input.as_ref().unwrap(),
                    reference.source_schema().original_words()
                )
                .is_none()
        );
        let foreign_source = source.replace("C", "Other");
        let foreign = Analyser::new().analyse(&foreign_source, "tcl8.6");
        let foreign_references =
            original_source_self_method_references(&foreign_source, &foreign).unwrap();
        let foreign_reference = foreign_references.first().unwrap();
        assert!(
            reference
                .catalogue_body()
                .original_source_correspondence_in_parent(
                    &input.context_registry(),
                    foreign_reference.body().original_parent_body()
                )
                .is_none()
        );
    }
}
