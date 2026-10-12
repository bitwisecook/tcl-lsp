// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Final conditional class slots retain their original factory declarations.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocationContext, OriginalSourceClassDeclaration,
    OriginalSourceCommandTransition, OriginalSourceTransitionAdviceTape,
    SourceCommandTransitionObligation,
};
use crate::analyser::{AnalysisResult, ClassDef, ResolvedAnalysisInput};
use crate::signature_scan::original_name::SourceDeclarationMetadata;
use crate::signature_scan::scope::SignatureSourceCommand;
use std::sync::Arc;
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::model::{ContextRegistry, ResolvedContext};
use tcl_syntax::naming::NamePolicyProtocol;

/// A genuine class factory surviving in the conditional root source graph.
/// Its current source slot supplies no allocated class or installed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassPublication {
    declaration: Arc<OriginalSourceClassDeclaration>,
    policy: NamePolicyProtocol,
    obligations: Vec<SourceCommandTransitionObligation>,
    uncertain_operations: Vec<Arc<[NativeWord]>>,
}
impl OriginalSourceClassPublication {
    /// Canonical original factory and name operand, preserving move lineage.
    #[must_use]
    pub fn class_declaration(&self) -> &OriginalSourceClassDeclaration {
        &self.declaration
    }
    /// Conditional current source slot, independently of allocation identity.
    #[must_use]
    pub fn source_slot(&self) -> &ByteCommandSlot {
        self.declaration.source_slot()
    }
    /// Independently selected authored naming policy.
    #[must_use]
    pub const fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }
    /// Actual original transitions selecting the current source slot.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        self.declaration.lineage()
    }
    /// Factory applicability and unresolved execution or earlier mutation.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Whole original source operations with unresolved mutation effects.
    #[must_use]
    pub fn uncertain_operations(&self) -> &[Arc<[NativeWord]>] {
        &self.uncertain_operations
    }
    /// Genuine canonical declaration metadata, without a reported-name lookup.
    #[must_use]
    pub fn source_class<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a SourceDeclarationMetadata<ClassDef>> {
        self.declaration.source_class(analysis)
    }
    pub(crate) fn matches_declaration(&self, record: &SourceDeclarationMetadata<ClassDef>) -> bool {
        self.declaration.matches_source_declaration(record)
    }
}

/// Readonly canonical source card with its independent current-slot receipt.
/// An unrepresented declaration remains a card, never a current slot fact.
#[derive(Debug, Clone, Copy)]
pub struct OriginalSourceClassCandidate<'a> {
    declaration: &'a SourceDeclarationMetadata<ClassDef>,
    publication: Option<&'a OriginalSourceClassPublication>,
}
impl<'a> OriginalSourceClassCandidate<'a> {
    /// Genuine canonical factory declaration metadata.
    #[must_use]
    pub const fn declaration(&self) -> &'a SourceDeclarationMetadata<ClassDef> {
        self.declaration
    }
    /// Conditional current publication, absent for an unrepresented source card.
    #[must_use]
    pub const fn publication(&self) -> Option<&'a OriginalSourceClassPublication> {
        self.publication
    }
    /// Current source slot or the independent card's canonical original slot.
    #[must_use]
    pub fn source_slot(&self) -> &'a ByteCommandSlot {
        self.publication.map_or_else(
            || self.declaration.name().slot(),
            OriginalSourceClassPublication::source_slot,
        )
    }
    /// Original selected declaration policy, distinct from engine activation.
    #[must_use]
    pub fn policy(&self) -> NamePolicyProtocol {
        self.declaration.name().policy()
    }
    /// Conditional publication premises. An unrepresented source card grants
    /// no current slot and carries no publication receipt.
    #[must_use]
    pub fn obligations(&self) -> &'a [SourceCommandTransitionObligation] {
        self.publication
            .map_or(&[], OriginalSourceClassPublication::obligations)
    }

    /// Current readonly source metadata with the original publication purpose.
    #[must_use]
    pub fn source_name(&self) -> Option<SignatureSourceCommand> {
        self.publication.map_or_else(
            || Some(self.declaration.name().clone()),
            |publication| {
                SignatureSourceCommand::class_from_original_source_publication(
                    publication,
                    self.declaration,
                )
            },
        )
    }
}

/// Final authentic root source graph and separately represented factory ancestry.
/// Known terminal transitions withdraw represented source suggestions; unknown
/// effects retain premises and supply no running interpreter absence verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassPublications {
    declarations: Vec<Arc<OriginalSourceClassDeclaration>>,
    publications: Vec<OriginalSourceClassPublication>,
    image: SourceImage,
    config: LexerConfig,
    context: ResolvedContext,
    registry: tcl_registry::RegistrySemanticKey,
    policy: NamePolicyProtocol,
    input: Option<ResolvedAnalysisInput>,
}
impl OriginalSourceClassPublications {
    /// Surviving direct conditional class cells, independent of aliases.
    #[must_use]
    pub fn publications(&self) -> &[OriginalSourceClassPublication] {
        &self.publications
    }
    /// Complete original image, lexical configuration and Registry context.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.image == *image
            && self.config == config
            && self.context == *context.context()
            && self.registry == context.commands().snapshot().semantic_key()
    }
    /// The realm has independently checked this exact retained input before
    /// exposing its cached inventory. No profile reconstruction is accepted.
    pub(crate) fn with_retained_input(&self, input: &ResolvedAnalysisInput) -> Self {
        let mut retained = self.clone();
        retained.input = Some(input.clone());
        retained
    }
    /// Shared current source candidates. Represented factories use only their
    /// surviving cells; known deletion or replacement cannot revive a card.
    /// Unrepresented body declarations keep their separate source-card purpose.
    #[must_use]
    pub fn candidates<'a>(
        &'a self,
        analysis: &'a AnalysisResult,
    ) -> Vec<OriginalSourceClassCandidate<'a>> {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        if !analysis.matches_original_source_image(&self.image, self.config) {
            return Vec::new();
        }
        self.candidates_from_retained_records(
            analysis.resolved_input.as_ref(),
            analysis.original_class_declarations(),
        )
    }
    /// Current indexed canonical records under the exact input retained by
    /// their producer. Missing or foreign input withdraws every candidate;
    /// source labels alone cannot replace the sealed final graph.
    #[must_use]
    pub fn candidates_from_retained_records<'a>(
        &'a self,
        input: Option<&ResolvedAnalysisInput>,
        records: impl IntoIterator<Item = &'a SourceDeclarationMetadata<ClassDef>>,
    ) -> Vec<OriginalSourceClassCandidate<'a>> {
        let Some(input) = input else {
            return Vec::new();
        };
        if self.input.as_ref() != Some(input)
            || input.lexer_config() != self.config
            || !self.matches_source_context(&self.image, self.config, &input.context_registry())
        {
            return Vec::new();
        }
        let mut candidates = Vec::new();
        for record in records {
            if record.name_input().source_image() != &self.image
                || record.name_input().lexer_config() != self.config
                || record.name().policy() != self.policy
                || record.name_input().policy() != self.policy
                || record.metadata().source_name.as_ref() != Some(record.name())
            {
                continue;
            }
            if self
                .declarations
                .iter()
                .any(|declaration| declaration.matches_source_declaration(record))
            {
                candidates.extend(self.publications.iter().filter_map(|publication| {
                    publication.matches_declaration(record).then_some(
                        OriginalSourceClassCandidate {
                            declaration: record,
                            publication: Some(publication),
                        },
                    )
                }));
            } else {
                candidates.push(OriginalSourceClassCandidate {
                    declaration: record,
                    publication: None,
                });
            }
        }
        candidates
    }
}
impl AdviceInvocationContext<'_> {
    pub(super) fn retain_final_original_class_publications(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
    ) {
        let Some(policy) = self.policy.native() else {
            return;
        };
        let mut publications = graph
            .cells
            .iter()
            .filter_map(|(slot, cell)| {
                let AdviceCell::SourceClass(declaration) = cell else {
                    return None;
                };
                if slot != declaration.source_slot() {
                    return None;
                }
                let mut obligations = declaration.factory().obligations().to_vec();
                if !obligations
                    .contains(&SourceCommandTransitionObligation::WrittenTransitionApplicability)
                {
                    obligations
                        .push(SourceCommandTransitionObligation::WrittenTransitionApplicability);
                }
                if !graph.uncertain_operations.is_empty()
                    && !obligations
                        .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
                {
                    obligations.push(SourceCommandTransitionObligation::UnknownEarlierMutation);
                }
                Some(OriginalSourceClassPublication {
                    declaration: Arc::clone(declaration),
                    policy,
                    obligations,
                    uncertain_operations: graph.uncertain_operations.clone(),
                })
            })
            .collect::<Vec<_>>();
        publications.sort_by_key(|publication| {
            tcl_syntax::naming::native_command_full_name_bytes(publication.source_slot())
        });
        tape.class_publications = Some(OriginalSourceClassPublications {
            declarations: graph.class_declarations.clone(),
            publications,
            image: self.origin.source_image().clone(),
            config: self.config,
            context: self.context.context().clone(),
            registry: self.context.commands().snapshot().semantic_key(),
            policy,
            input: None,
        });
    }
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn class_publications(&self) -> Option<&OriginalSourceClassPublications> {
        self.class_publications.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::source_class_publications;

    #[test]
    fn original_class_publications_keep_current_slots_and_canonical_factories() {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let source = "oo::class create Old {}; oo::class create Removed {}; rename Old New; rename Removed {}";
            let mut analysis = Analyser::new().analyse(source, dialect);
            assert!(analysis.original_completed_command_world().is_none());
            analysis.all_classes.clear();
            let inventory = source_class_publications(source, &analysis).expect(dialect);
            let candidates = inventory.candidates(&analysis);
            assert_eq!(candidates.len(), 1, "{dialect}");
            let candidate = &candidates[0];
            let publication = candidate.publication().unwrap();
            assert_eq!(candidate.source_slot().simple.as_bytes(), b"New");
            assert_eq!(candidate.declaration().name_input().bytes(), b"Old");
            assert_eq!(
                publication.source_class(&analysis),
                Some(candidate.declaration())
            );
            assert_eq!(publication.lineage().len(), 1);
            assert_eq!(
                candidate.source_name().unwrap().publication(),
                candidate.declaration().name().publication()
            );
            assert_eq!(
                candidate.source_name().unwrap().slot(),
                publication.source_slot()
            );
            assert!(
                publication
                    .obligations()
                    .contains(&SourceCommandTransitionObligation::UnavailableActualLookup)
            );
            assert!(
                publication
                    .obligations()
                    .contains(&SourceCommandTransitionObligation::RegisteredFactoryApplicability)
            );
        }
    }

    #[test]
    fn original_class_publications_keep_unknown_mutations_and_known_terminals_distinct() {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        let source = "oo::class create Old {}; unknown_effect; rename Old New";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let inventory = source_class_publications(source, &analysis).unwrap();
        let candidates = inventory.candidates(&analysis);
        assert_eq!(candidates.len(), 1);
        let publication = candidates[0].publication().unwrap();
        assert!(
            publication
                .obligations()
                .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
        );
        assert!(publication.uncertain_operations().iter().any(|words| {
            words[0].span().start()
                == u32::try_from(source.find("unknown_effect").unwrap()).unwrap()
        }));
        for source in [
            "oo::class create Old {}; rename Old {}",
            "oo::class create Old {}; proc Old {} {}",
            "oo::class create Old {}; interp alias {} Old {} external",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let inventory = source_class_publications(source, &analysis).unwrap();
            assert!(inventory.candidates(&analysis).is_empty(), "{source}");
        }
    }

    #[test]
    fn original_class_publications_require_retained_input_and_actual_availability() {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        let source = "oo::class create C {}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let inventory = source_class_publications(source, &analysis).unwrap();
        assert_eq!(inventory.candidates(&analysis).len(), 1);
        let indexed = analysis
            .original_class_declarations()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            inventory
                .candidates_from_retained_records(analysis.resolved_input.as_ref(), indexed.iter(),)
                .len(),
            1
        );
        assert!(
            inventory
                .candidates_from_retained_records(None, indexed.iter())
                .is_empty()
        );
        assert!(source_class_publications(&source.replace("C", "D"), &analysis).is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(source_class_publications(source, &missing).is_none());
        assert!(inventory.candidates(&missing).is_empty());
        let mut wrong_config = analysis.clone();
        wrong_config
            .body_lexer_config
            .as_mut()
            .unwrap()
            .strict_quoting = !analysis.body_lexer_config.unwrap().strict_quoting;
        assert!(source_class_publications(source, &wrong_config).is_none());
        assert!(inventory.candidates(&wrong_config).is_empty());
        let input = analysis.resolved_input.as_ref().unwrap();
        let generation = input.context_registry();
        let foreign = Arc::new(
            generation.with_command_store(generation.commands().snapshot().shared_registry()),
        );
        let mut foreign_analysis = analysis.clone();
        foreign_analysis.resolved_input = Some(ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            foreign,
            input.lexer_config(),
        ));
        assert!(source_class_publications(source, &foreign_analysis).is_none());
        assert!(inventory.candidates(&foreign_analysis).is_empty());
        assert!(
            inventory
                .candidates_from_retained_records(
                    foreign_analysis.resolved_input.as_ref(),
                    indexed.iter(),
                )
                .is_empty()
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(generation.commands())),
        );
        let supplied = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        );
        let mut changed = analysis.clone();
        changed.resolved_input = Some(supplied.clone());
        assert!(source_class_publications(source, &changed).is_none());
        assert!(inventory.candidates(&changed).is_empty());
        let unavailable = Analyser::new()
            .with_resolved_input(supplied)
            .analyse(source, "tcl8.6");
        let unavailable_inventory = source_class_publications(source, &unavailable).unwrap();
        assert!(unavailable_inventory.publications().is_empty());
        assert!(unavailable_inventory.candidates(&unavailable).is_empty());
        assert!(unavailable.original_class_declarations().next().is_none());
        assert!(source_class_publications("", &AnalysisResult::default()).is_none());
    }
}
