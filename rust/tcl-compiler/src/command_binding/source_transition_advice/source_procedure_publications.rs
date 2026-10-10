// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Final conditional procedure source slots keep their original declarations.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocationContext, OriginalSourceCommandTransition,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    SourceAdviceNameInput, SourceCommandTransitionObligation,
};
use crate::analyser::{AnalysisResult, ProcDef};
use crate::signature_scan::original_name::SourceDeclarationMetadata;
use crate::signature_scan::scope::SignatureSourceCommand;
use std::sync::Arc;
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::model::{ContextRegistry, ResolvedContext};
use tcl_syntax::naming::NamePolicyProtocol;

/// An original root procedure declaration surviving the conditional source graph.
/// Its current source slot is distinct from its canonical declaration name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceProcedurePublication {
    procedure: Arc<super::source_procedure::OriginalSourceProcedureBody>,
    policy: NamePolicyProtocol,
    obligations: Vec<SourceCommandTransitionObligation>,
    uncertain_operations: Vec<Arc<[NativeWord]>>,
}
impl OriginalSourceProcedurePublication {
    /// Original selected procedure definer and complete source operands.
    #[must_use]
    pub fn declaration(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.procedure.declaration
    }
    /// Canonical original naming operand, independent of current source spelling.
    #[must_use]
    pub fn declaration_name_input(&self) -> &SourceAdviceNameInput {
        &self.procedure.name
    }
    /// Authentic original body, without procedure entry or an execution frame.
    #[must_use]
    pub fn body(&self) -> &crate::registry_invocation::OriginalSourceScriptBody {
        &self.procedure.body
    }
    /// Conditional current source slot, with no installed command claim.
    #[must_use]
    pub fn source_slot(&self) -> &ByteCommandSlot {
        &self.procedure.slot
    }
    /// Independently selected source naming policy.
    #[must_use]
    pub const fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }
    /// Original definer and move lineage; no held alias target is followed.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.procedure.lineage
    }
    /// Unresolved declaration, transition and earlier mutation premises.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Exact original source operations whose mutation effects remain unknown.
    #[must_use]
    pub fn uncertain_operations(&self) -> &[Arc<[NativeWord]>] {
        &self.uncertain_operations
    }
    /// Canonical retained declaration joined without a report-name lookup.
    #[must_use]
    pub fn source_procedure<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a SourceDeclarationMetadata<ProcDef>> {
        self.procedure.source_procedure(analysis)
    }
    /// Readonly source publication metadata, without runtime publication authority.
    #[must_use]
    pub fn source_name(&self) -> SignatureSourceCommand {
        SignatureSourceCommand::procedure_from_original_source_publication(self)
    }
}

/// One source suggestion and the independent publication receipt, when present.
/// An unrepresented declaration remains a source card, not a current slot fact.
#[derive(Debug, Clone, Copy)]
pub struct OriginalSourceProcedureCandidate<'a> {
    declaration: &'a SourceDeclarationMetadata<ProcDef>,
    publication: Option<&'a OriginalSourceProcedurePublication>,
}
impl<'a> OriginalSourceProcedureCandidate<'a> {
    /// Original canonical declaration metadata.
    #[must_use]
    pub const fn declaration(&self) -> &'a SourceDeclarationMetadata<ProcDef> {
        self.declaration
    }
    /// Conditional source publication, absent for an unrepresented source card.
    #[must_use]
    pub const fn publication(&self) -> Option<&'a OriginalSourceProcedurePublication> {
        self.publication
    }
    /// Current conditional slot or the independent declaration card's original slot.
    #[must_use]
    pub fn source_slot(&self) -> &'a ByteCommandSlot {
        self.publication.map_or_else(
            || self.declaration.name().slot(),
            OriginalSourceProcedurePublication::source_slot,
        )
    }
    /// Genuine declaration naming policy, distinct from an engine activation.
    #[must_use]
    pub fn policy(&self) -> NamePolicyProtocol {
        self.declaration.name().policy()
    }
    /// Purpose-specific readonly source publication metadata.
    #[must_use]
    pub fn source_name(&self) -> SignatureSourceCommand {
        self.publication.map_or_else(
            || self.declaration.name().clone(),
            OriginalSourceProcedurePublication::source_name,
        )
    }
}

/// Authentic final root source graph and its represented procedure headers.
/// Known terminal cells withdraw represented suggestions; this does not prove
/// absence from a running interpreter or publish speculative body effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceProcedurePublications {
    declarations: Vec<Arc<super::source_procedure::OriginalSourceProcedureBody>>,
    publications: Vec<OriginalSourceProcedurePublication>,
    image: SourceImage,
    config: LexerConfig,
    context: ResolvedContext,
    registry: tcl_registry::RegistrySemanticKey,
    policy: NamePolicyProtocol,
}
fn matches_declaration(
    procedure: &super::source_procedure::OriginalSourceProcedureBody,
    record: &SourceDeclarationMetadata<ProcDef>,
) -> bool {
    record.declaration_site() == procedure.declaration.site()
        && procedure
            .name
            .original_word()
            .is_some_and(|word| record.name_input().original_word() == word)
        && record.metadata().source_name.as_ref() == Some(record.name())
}
impl OriginalSourceProcedurePublications {
    /// Surviving direct procedure cells, independent of aliases and runtime state.
    #[must_use]
    pub fn publications(&self) -> &[OriginalSourceProcedurePublication] {
        &self.publications
    }
    /// Full original image, lexical configuration and Registry context agree.
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
    /// Shared readonly candidates. Represented headers use current source cells;
    /// known deletion/replacement does not fall back to their original spelling.
    /// Other genuine declarations retain their separate source-card purpose.
    #[must_use]
    pub fn candidates<'a>(
        &'a self,
        records: impl IntoIterator<Item = &'a SourceDeclarationMetadata<ProcDef>>,
    ) -> Vec<OriginalSourceProcedureCandidate<'a>> {
        // naming.source.original-procedure-publications
        // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
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
                .any(|procedure| matches_declaration(procedure, record))
            {
                candidates.extend(self.publications.iter().filter_map(|publication| {
                    matches_declaration(&publication.procedure, record).then_some(
                        OriginalSourceProcedureCandidate {
                            declaration: record,
                            publication: Some(publication),
                        },
                    )
                }));
            } else {
                candidates.push(OriginalSourceProcedureCandidate {
                    declaration: record,
                    publication: None,
                });
            }
        }
        candidates
    }
}
impl AdviceInvocationContext<'_> {
    pub(super) fn retain_final_original_procedure_publications(
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
                let AdviceCell::SourceProcedure(procedure) = cell else {
                    return None;
                };
                if slot != &procedure.slot {
                    return None;
                }
                let mut obligations = procedure.declaration.obligations().to_vec();
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
                Some(OriginalSourceProcedurePublication {
                    procedure: Arc::clone(procedure),
                    policy,
                    obligations,
                    uncertain_operations: graph.uncertain_operations.clone(),
                })
            })
            .collect::<Vec<_>>();
        publications.sort_by_key(|publication| {
            tcl_syntax::naming::native_command_full_name_bytes(publication.source_slot())
        });
        tape.procedure_publications = Some(OriginalSourceProcedurePublications {
            declarations: graph.procedure_declarations.clone(),
            publications,
            image: self.origin.source_image().clone(),
            config: self.config,
            context: self.context.context().clone(),
            registry: self.context.commands().snapshot().semantic_key(),
            policy,
        });
    }
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn procedure_publications(&self) -> Option<&OriginalSourceProcedurePublications> {
        self.procedure_publications.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::source_procedure_publications;

    #[test]
    fn original_procedure_publications_keep_current_slots_and_canonical_headers() {
        // naming.source.original-procedure-publications
        // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
        let source = "proc old {value} {}; proc deleted {} {}; rename old moved; rename deleted {}";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            assert!(analysis.original_completed_command_world().is_none());
            analysis.all_procs.clear();
            analysis.superseded_procs.clear();
            let inventory = source_procedure_publications(source, &analysis).expect(dialect);
            assert_eq!(inventory.publications().len(), 1, "{dialect}");
            let publication = &inventory.publications()[0];
            assert_eq!(publication.source_slot().simple.as_bytes(), b"moved");
            assert_eq!(publication.declaration_name_input().bytes(), b"old");
            assert_eq!(
                publication
                    .source_procedure(&analysis)
                    .unwrap()
                    .name_input()
                    .bytes(),
                b"old"
            );
            assert_eq!(publication.lineage().len(), 1);
            assert_eq!(
                publication.lineage()[0].original_words()[0].span().start(),
                u32::try_from(source.find("rename old").unwrap()).unwrap()
            );
            assert!(
                publication
                    .obligations()
                    .contains(&SourceCommandTransitionObligation::UnavailableActualLookup)
            );
            assert!(
                publication
                    .obligations()
                    .contains(&SourceCommandTransitionObligation::WrittenTransitionApplicability)
            );
            let candidates = inventory.candidates(analysis.original_procedure_declarations());
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].source_slot(), publication.source_slot());
            assert_eq!(
                candidates[0].source_name().slot(),
                publication.source_slot()
            );
        }
    }

    #[test]
    fn original_procedure_publications_keep_unknown_mutations_and_known_terminals_distinct() {
        // naming.source.original-procedure-publications
        // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
        for (source, expected) in [
            (
                "proc old {} {}; unknown_effect; rename old moved",
                b"moved".as_slice(),
            ),
            (
                "proc old {first} {}; proc old {replacement} {}",
                b"old".as_slice(),
            ),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let inventory = source_procedure_publications(source, &analysis).unwrap();
            let candidates = inventory.candidates(analysis.original_procedure_declarations());
            assert_eq!(candidates.len(), 1, "{source}");
            let publication = candidates[0].publication().unwrap();
            assert_eq!(publication.source_slot().simple.as_bytes(), expected);
            if source.contains("unknown_effect") {
                assert!(
                    publication
                        .obligations()
                        .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
                );
                assert_eq!(publication.uncertain_operations().len(), 1);
                assert_eq!(
                    publication.uncertain_operations()[0][0].span().start(),
                    u32::try_from(source.find("unknown_effect").unwrap()).unwrap()
                );
            } else {
                assert_eq!(
                    candidates[0].declaration().metadata().params[0].name,
                    "replacement"
                );
            }
        }
        for source in [
            "proc old {} {}; rename old {}",
            "proc old {} {}; interp alias {} old {} external",
            "proc old {} {}; oo::class create old {}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let inventory = source_procedure_publications(source, &analysis).unwrap();
            assert!(
                inventory
                    .candidates(analysis.original_procedure_declarations())
                    .is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_procedure_publications_require_full_source_input_and_keep_body_cards_separate() {
        // naming.source.original-procedure-publications
        // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
        let source = "proc owner {} {proc inner {} {}}; rename owner held";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let inventory = source_procedure_publications(source, &analysis).unwrap();
        let candidates = inventory.candidates(analysis.original_procedure_declarations());
        let held = candidates
            .iter()
            .find(|row| row.source_slot().simple.as_bytes() == b"held")
            .unwrap();
        assert!(held.publication().is_some());
        let inner = candidates
            .iter()
            .find(|row| row.declaration().name_input().bytes() == b"inner")
            .unwrap();
        assert!(inner.publication().is_none());
        assert!(
            source_procedure_publications(&source.replace("held", "other"), &analysis).is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(source_procedure_publications(source, &missing).is_none());
        let mut changed = analysis.clone();
        let mut config = changed.body_lexer_config.unwrap();
        config.strict_quoting = !config.strict_quoting;
        changed.body_lexer_config = Some(config);
        assert!(source_procedure_publications(source, &changed).is_none());
        assert!(source_procedure_publications("", &AnalysisResult::default()).is_none());
    }
}
