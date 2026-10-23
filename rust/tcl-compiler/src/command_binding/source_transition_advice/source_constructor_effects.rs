// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Explicit original constructor-body coverage, independent of call effects.

use super::OriginalSourceClassDeclaration;
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use crate::registry_invocation::{
    OriginalSourceDefinitionMemberScriptBody, OriginalSourceScriptBody,
};
use std::sync::Arc;
use tcl_lexer::NativeWord;
use tcl_registry::definer::{
    DefinerFamily, DefinitionSpecialMemberKind, MemberConstructionEffect, MemberKind,
    NativeLifecycleBodyDisposition, SourceFormalValidationApplicability,
};

/// What the genuine counted constructor body contributes to source coverage.
/// Neither variant closes an inherited/default constructor chain or dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalSourceConstructorBodyDisposition {
    /// A nonempty counted body has a complete original plan with no commands.
    /// This covers that body only; it grants no lookup or completion result.
    CommandFreeRetainedBody,
    /// An explicitly empty counted body selects the Registry's removal syntax.
    /// Missing/default/inherited constructor effects remain unknown.
    ExplicitlyRemoved,
}

/// Sealed complete own-definition coverage and one explicit constructor body.
/// The nominal source recipe is separate from Native worker identity, runtime
/// configuration, inherited constructor effects and operand/read observers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceConstructorBodyCoverage {
    class: OriginalSourceClassDeclaration,
    input: ResolvedAnalysisInput,
    realm: Arc<crate::realm::CommandBindingRealm>,
    definition: OriginalSourceScriptBody,
    members: Vec<Vec<NativeWord>>,
    constructor: OriginalSourceDefinitionMemberScriptBody,
    disposition: OriginalSourceConstructorBodyDisposition,
}
impl OriginalSourceConstructorBodyCoverage {
    /// Exact retained original class incarnation; route moves are independent.
    #[must_use]
    pub const fn class_declaration(&self) -> &OriginalSourceClassDeclaration {
        &self.class
    }
    /// Genuine whole class definition body and its source schema.
    #[must_use]
    pub const fn definition_body(&self) -> &OriginalSourceScriptBody {
        &self.definition
    }
    /// Complete original member vectors covered by this own-definition view.
    #[must_use]
    pub fn original_members(&self) -> &[Vec<NativeWord>] {
        &self.members
    }
    /// Actual selected explicit constructor member/body source.
    #[must_use]
    pub const fn constructor_body(&self) -> &OriginalSourceDefinitionMemberScriptBody {
        &self.constructor
    }
    /// Original counted-body disposition, without constructor-chain closure.
    #[must_use]
    pub const fn disposition(&self) -> OriginalSourceConstructorBodyDisposition {
        self.disposition
    }
    /// Full declaring analysis input, independent of any project's caller input.
    #[must_use]
    pub const fn declaring_input(&self) -> &ResolvedAnalysisInput {
        &self.input
    }

    pub(crate) fn for_source_attempt(
        &self,
        analysis: &AnalysisResult,
        original: &[NativeWord],
    ) -> Option<OriginalSourceConstructorBodyAttempt> {
        let input = analysis.resolved_input.as_ref()?;
        let realm = analysis.retained_command_realm_arc()?;
        let image = realm.original_source_image()?;
        (self.realm.matches_resolved_analysis_input(&self.input)
            && self.realm.matches_original_source_image(
                self.definition.original_container().image(),
                self.input.lexer_config(),
            )
            && realm.matches_resolved_analysis_input(input)
            && analysis.matches_original_source_image(image, input.lexer_config())
            && !original.is_empty()
            && original.iter().all(|word| {
                word.image() == image
                    && word.config() == input.lexer_config()
                    && !word.group().expand
            }))
        .then_some(())?;
        Some(OriginalSourceConstructorBodyAttempt {
            coverage: self.clone(),
            input: input.clone(),
            realm: Arc::clone(realm),
            original: Arc::from(original),
        })
    }
}

/// Actual selected project source construction syntax carrying independent
/// declaring-body coverage. This is a conditional attempt, never a reached
/// constructor, unchanged command lookup, allocated instance or Normal grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceConstructorBodyAttempt {
    coverage: OriginalSourceConstructorBodyCoverage,
    input: ResolvedAnalysisInput,
    realm: Arc<crate::realm::CommandBindingRealm>,
    original: Arc<[NativeWord]>,
}
impl OriginalSourceConstructorBodyAttempt {
    /// Actual constructor-body source coverage selected by the project owner.
    #[must_use]
    pub const fn body_coverage(&self) -> &OriginalSourceConstructorBodyCoverage {
        &self.coverage
    }
    /// Full original constructor argv at the actual caller source site.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Actual caller input, retained separately from the declaring input.
    #[must_use]
    pub const fn caller_input(&self) -> &ResolvedAnalysisInput {
        &self.input
    }
}

fn capture_constructor_member(
    class: &OriginalSourceClassDeclaration,
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    region: &crate::registry_invocation::OriginalSourceDefinitionMemberRegion,
    words: &[NativeWord],
    constructor: &mut Option<(
        OriginalSourceDefinitionMemberScriptBody,
        OriginalSourceConstructorBodyDisposition,
    )>,
) -> Option<()> {
    let member = region.member_keyword(words)?;
    // Only Registry-classified ordinary deferred methods and one actual
    // own constructor are covered. Relations, wrappers, initialisers and
    // all other effects need independent MRO/configuration coverage.
    let is_constructor = grammar.source_special_member_kind(member)
        == Some(DefinitionSpecialMemberKind::Constructor);
    if !is_constructor
        && grammar.construction_member_effect(member)
            != MemberConstructionEffect::DeferredInstanceMethod
    {
        return None;
    }
    if member.kind != MemberKind::Flat || member.optional_argument.is_some() {
        return None;
    }
    let expected = member
        .arg_roles
        .iter()
        .map(|(index, _)| usize::from(*index) + 2)
        .max()?;
    if words.len() != expected || words.iter().any(|word| word.group().expand) {
        return None;
    }
    let values = words
        .iter()
        .map(tcl_syntax::word_rules::original_static_word_unicode_value)
        .collect::<Option<Vec<_>>>()?;
    let mut bodies = region.script_bodies(words)?.into_iter();
    let body = bodies.next()?;
    bodies.next().is_none().then_some(())?;
    let row = body.member_row()?;
    if row.receiver != tcl_registry::definer::MemberReceiver::Instance {
        return None;
    }
    let body_value = values.get(body.argument().checked_add(1)?)?;
    match grammar.source_member_formal_validation_applicability(member, Some(body_value))? {
        SourceFormalValidationApplicability::Required => {
            for (ordinal, role) in member.arg_roles {
                if *role == tcl_registry::ArgRole::ParamList {
                    let parameters =
                        std::str::from_utf8(values.get(usize::from(*ordinal) + 1)?).ok()?;
                    tcl_syntax::formal_params::parse_formal_parameters_in(
                        parameters,
                        class.factory().dialect().parameter_grammar()?,
                    )
                    .ok()?;
                }
            }
        }
        SourceFormalValidationApplicability::SkippedRemovedLifecycle => {}
    }
    if is_constructor {
        if constructor.is_some() || !body.complete_original_commands()?.is_empty() {
            return None;
        }
        let disposition = match grammar.native_lifecycle_body_disposition(member, body_value)? {
            NativeLifecycleBodyDisposition::Removed => {
                OriginalSourceConstructorBodyDisposition::ExplicitlyRemoved
            }
            NativeLifecycleBodyDisposition::Retained => {
                OriginalSourceConstructorBodyDisposition::CommandFreeRetainedBody
            }
        };
        *constructor = Some((body, disposition));
    }
    Some(())
}

pub(super) fn capture(
    class: &OriginalSourceClassDeclaration,
    analysis: &AnalysisResult,
) -> Option<OriginalSourceConstructorBodyCoverage> {
    let input = analysis.resolved_input.as_ref()?;
    let realm = analysis.retained_command_realm_arc()?;
    let image = realm.original_source_image()?;
    let context = input.context_registry();
    (realm.matches_resolved_analysis_input(input)
        && analysis.matches_original_source_image(image, input.lexer_config())
        && class.factory().uncertain_operations().is_empty()
        && realm
            .original_class_display_publications(input)?
            .has_complete_class_source_coverage(class))
    .then_some(())?;
    // No unrepresented final configuration is repaired from report metadata.
    // This first own-definition facet refuses even an unrelated retained delta.
    if analysis.original_class_configurations().next().is_some()
        || analysis.original_object_configurations().next().is_some()
    {
        return None;
    }
    let metadata = if input.has_logical_source_name_context() {
        class.logical_source_class(analysis)?
    } else {
        class.source_class(analysis)?.metadata()
    };
    let grammar = class.grammar(&context)?;
    (grammar.family == DefinerFamily::TclOo).then_some(())?;
    let source = std::str::from_utf8(image.bytes()).ok()?;
    let factory = crate::registry_invocation::source_structure::source_registry_words_at(
        source,
        analysis,
        class.factory().site().offset,
    )?;
    let mut definitions = factory
        .source_script_bodies(&context)
        .into_iter()
        .filter(|body| {
            body.original_container()
                .tokens()
                .first()
                .is_some_and(|token| token.span == metadata.body_span)
        });
    let definition = definitions.next()?;
    definitions.next().is_none().then_some(())?;
    let region = definition.definition_member_region(&context, definition.content_span())?;
    let members = region.complete_original_commands()?;
    let mut constructor = None;
    for words in &members {
        capture_constructor_member(class, grammar, &region, words, &mut constructor)?;
    }
    // Positive complete own-member coverage cannot supply an inherited or
    // default body when no explicit constructor was present.
    let (constructor, disposition) = constructor?;
    Some(OriginalSourceConstructorBodyCoverage {
        class: class.clone(),
        input: input.clone(),
        realm: Arc::clone(realm),
        definition,
        members,
        constructor,
        disposition,
    })
}
