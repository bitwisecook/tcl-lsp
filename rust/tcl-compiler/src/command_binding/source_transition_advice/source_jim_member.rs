// SPDX-License-Identifier: AGPL-3.0-or-later
//! Own Jim class/procedure source correspondence; no generic class identity.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocationContext, OriginalAuthoredJimClass,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    SourceCommandTransitionObligation, source_procedure::OriginalSourceProcedureBody,
};
use crate::{
    analyser::{AnalysisResult, ClassDef},
    signature_scan::{original_name::SourceDeclarationMetadata, scope::SignatureSourceNameInput},
};
use std::sync::{Arc, Weak};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AuthoredJimProcedureMemberRecipe {
    consumer: Arc<OriginalSourceCommandTransitionAdvice>,
    class: Arc<OriginalAuthoredJimClass>,
    procedure: Arc<OriginalSourceProcedureBody>,
    class_input: SignatureSourceNameInput,
    member: SignatureSourceNameInput,
    obligations: Vec<SourceCommandTransitionObligation>,
}

/// A readonly member relation from one authentic original Jim tape and owner.
/// Its applicability debts remain separate from entered or installed state.
#[derive(Debug, Clone)]
pub struct OriginalAuthoredJimProcedureMember {
    recipe: AuthoredJimProcedureMemberRecipe,
    owner: Weak<crate::command_binding::OriginalSourceAnalysisOwner>,
}
impl OriginalAuthoredJimProcedureMember {
    fn matches_analysis(&self, analysis: &AnalysisResult) -> Option<()> {
        let retained = self.owner.upgrade()?;
        let realm = analysis.retained_command_realm()?;
        let current = realm.source_bindings_ref().original_analysis_owner()?;
        Arc::ptr_eq(&retained, current).then_some(())?;
        let input = analysis.resolved_input.as_ref()?;
        realm
            .authenticates_authored_jim_procedure_member(input, self)
            .then_some(())?;
        let image = self.recipe.consumer.original_words().first()?.image();
        (analysis.matches_original_source_image(image, input.lexer_config())
            && self
                .recipe
                .consumer
                .matches_source(image, input.lexer_config())
            && self
                .recipe
                .consumer
                .matches_context(&input.context_registry())
            && self.recipe.class.matches_source_context(
                image,
                input.lexer_config(),
                &input.context_registry(),
            )
            && self
                .recipe
                .procedure
                .declaration
                .matches_source(image, input.lexer_config())
            && self
                .recipe
                .procedure
                .declaration
                .matches_context(&input.context_registry()))
        .then_some(())
    }
    pub(crate) fn class_metadata<'a>(&self, analysis: &'a AnalysisResult) -> Option<&'a ClassDef> {
        self.matches_analysis(analysis)?;
        self.recipe.class.metadata(analysis)
    }
    pub(crate) fn source_procedure<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a SourceDeclarationMetadata<crate::analyser::ProcDef>> {
        self.matches_analysis(analysis)?;
        self.recipe.procedure.source_procedure(analysis)
    }
    pub(crate) fn member(&self) -> &SignatureSourceNameInput {
        &self.recipe.member
    }
    pub(crate) fn class_input(&self) -> &SignatureSourceNameInput {
        &self.recipe.class_input
    }
    pub(crate) fn procedure_advice(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.recipe.procedure.declaration
    }
    pub(crate) fn class_grammar(
        &self,
        analysis: &AnalysisResult,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        self.matches_analysis(analysis)?;
        self.recipe
            .class
            .grammar(&analysis.resolved_input.as_ref()?.context_registry())
    }
    pub(crate) fn supports_member_association(&self, analysis: &AnalysisResult) -> bool {
        self.matches_analysis(analysis).is_some()
            && self.recipe.obligations.iter().all(|obligation| {
                matches!(
                    obligation,
                    SourceCommandTransitionObligation::UnavailableActualLookup
                        | SourceCommandTransitionObligation::WrittenTransitionApplicability
                )
            })
    }
    /// This relation never certifies quietness or successful publication.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.recipe.obligations
    }
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn authenticates_authored_jim_procedure_member(
        &self,
        receipt: &OriginalAuthoredJimProcedureMember,
    ) -> bool {
        self.authored_jim_procedure_members
            .get(&receipt.recipe.consumer.site().offset)
            .is_some_and(|rows| {
                rows.iter().any(|(_, candidate)| {
                    candidate.as_ref().is_some_and(|candidate| {
                        Arc::ptr_eq(
                            &candidate.consumer.site().source,
                            &receipt.recipe.consumer.site().source,
                        ) && Arc::ptr_eq(
                            &candidate.class.factory().site().source,
                            &receipt.recipe.class.factory().site().source,
                        ) && Arc::ptr_eq(
                            &candidate.procedure.declaration.site().source,
                            &receipt.recipe.procedure.declaration.site().source,
                        ) && candidate == &receipt.recipe
                    })
                })
            })
    }
    pub(crate) fn authored_jim_procedure_members(
        &self,
        original: &[tcl_lexer::NativeWord],
        owner: &Arc<crate::command_binding::OriginalSourceAnalysisOwner>,
    ) -> Vec<OriginalAuthoredJimProcedureMember> {
        let Some(head) = original.first() else {
            return Vec::new();
        };
        self.authored_jim_procedure_members
            .get(&head.span().start())
            .into_iter()
            .flatten()
            .filter_map(|(_, recipe)| {
                let recipe = recipe.as_ref()?;
                (recipe.consumer.original_words() == original).then(|| {
                    OriginalAuthoredJimProcedureMember {
                        recipe: recipe.clone(),
                        owner: Arc::downgrade(owner),
                    }
                })
            })
            .collect()
    }
    pub(super) fn merge_authored_jim_procedure_member(
        &mut self,
        offset: u32,
        input: SignatureSourceNameInput,
        recipe: Option<AuthoredJimProcedureMemberRecipe>,
    ) {
        let row = self
            .authored_jim_procedure_members
            .entry(offset)
            .or_default();
        if let Some((_, old)) = row.iter_mut().find(|(owned, _)| owned == &input) {
            if old != &recipe {
                *old = None;
            }
        } else {
            row.push((input, recipe));
        }
    }
    fn retain_authored_jim_procedure_member(&mut self, recipe: AuthoredJimProcedureMemberRecipe) {
        let Some(input) = recipe.procedure.name.native_input().cloned() else {
            return;
        };
        self.merge_authored_jim_procedure_member(
            recipe.consumer.site().offset,
            input,
            Some(recipe),
        );
    }
}
impl AdviceInvocationContext<'_> {
    pub(super) fn retain_original_class_operand_references(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        class: Option<&Arc<super::OriginalSourceClassDeclaration>>,
        procedure: Option<&Arc<OriginalSourceProcedureBody>>,
    ) {
        self.retain_source_class_operand_references(tape, graph, class, procedure);
        self.retain_authored_jim_procedure_reference(tape, graph, procedure);
    }
    pub(super) fn retain_authored_jim_procedure_reference(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        procedure: Option<&Arc<OriginalSourceProcedureBody>>,
    ) {
        let Some(procedure) = procedure else {
            return;
        };
        let Some((class_input, member)) = authored_jim_procedure_children(procedure) else {
            return;
        };
        let Some(policy) = graph.policy.native() else {
            return;
        };
        if class_input.policy() != policy {
            return;
        }
        let Some(key) = graph.selected_head_key(class_input.bytes()) else {
            return;
        };
        let Some(AdviceCell::AuthoredJimClass(class)) = graph.cells.get(&key) else {
            return;
        };
        if class.authored_policy() != Some(policy) {
            return;
        }
        if !authored_jim_member_publication_matches(graph, procedure, &class_input, &member) {
            return;
        }
        tape.retain_authored_jim_procedure_member(authored_jim_member_recipe(
            Arc::clone(&procedure.declaration),
            Arc::clone(class),
            Arc::clone(procedure),
            class_input,
            member,
        ));
    }
    pub(super) fn retain_authored_jim_procedure_adoptions(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        class: Option<&Arc<OriginalAuthoredJimClass>>,
    ) {
        let Some(class) = class else {
            return;
        };
        let Some(policy) = graph.policy.native() else {
            return;
        };
        if class.authored_policy() != Some(policy) {
            return;
        }
        let Some(grammar) = class.grammar(self.context) else {
            return;
        };
        let Some(initialisers) =
            grammar.source_two_word_factory_initialisers(class.factory().dialect())
        else {
            return;
        };
        let base_count = class.base_fields().map(|fields| fields.len());
        let Some(name) = class
            .factory()
            .arguments()
            .first()
            .and_then(|word| word.value.as_deref())
        else {
            return;
        };
        for initialiser in initialisers {
            if initialiser.presence.replaces_source_slot(base_count) == Some(false) {
                continue;
            }
            let Some(context) = graph.native_source_context() else {
                return;
            };
            if let Ok(slot) = policy.recipe().jim_two_word_member_publication_slot(
                context,
                name,
                initialiser.name.as_bytes(),
                initialiser.publication,
            ) {
                graph.cells.insert(slot, AdviceCell::Shadowed);
            }
        }
        if !class.statically_complete() {
            return;
        }
        for cell in graph.cells.values() {
            let AdviceCell::SourceProcedure(procedure) = cell else {
                continue;
            };
            let Some((input, member)) = authored_jim_procedure_children(procedure) else {
                continue;
            };
            if super::lookup_key(&graph.policy, procedure.name.bytes()).as_ref()
                != Some(&procedure.slot)
                || super::lookup_key(&graph.policy, input.bytes()).as_ref()
                    != Some(class.source_slot())
            {
                continue;
            }
            if !authored_jim_member_publication_matches(graph, procedure, &input, &member) {
                continue;
            }
            tape.retain_authored_jim_procedure_member(authored_jim_member_recipe(
                Arc::new(class.factory().clone()),
                Arc::clone(class),
                Arc::clone(procedure),
                input,
                member,
            ));
        }
    }
}
fn authored_jim_member_publication_matches(
    graph: &AdviceGraph,
    procedure: &OriginalSourceProcedureBody,
    class: &SignatureSourceNameInput,
    member: &SignatureSourceNameInput,
) -> bool {
    let Some(policy) = graph.policy.native() else {
        return false;
    };
    if class.policy() != policy || member.policy() != policy {
        return false;
    }
    let Some(context) = graph.native_source_context() else {
        return false;
    };
    policy
        .recipe()
        .jim_two_word_member_publication_slot(
            context,
            class.bytes(),
            member.bytes(),
            tcl_syntax::naming::NativeNamePurpose::CommandPublication,
        )
        .is_ok_and(|slot| slot == procedure.slot && slot == procedure.declaration_slot)
}
fn authored_jim_procedure_children(
    procedure: &OriginalSourceProcedureBody,
) -> Option<(SignatureSourceNameInput, SignatureSourceNameInput)> {
    let children = procedure.name.native_input()?.original_list_elements()?;
    let [class, member] = children.as_slice() else {
        return None;
    };
    Some((class.clone(), member.clone()))
}
fn authored_jim_member_recipe(
    consumer: Arc<OriginalSourceCommandTransitionAdvice>,
    class: Arc<OriginalAuthoredJimClass>,
    procedure: Arc<OriginalSourceProcedureBody>,
    class_input: SignatureSourceNameInput,
    member: SignatureSourceNameInput,
) -> AuthoredJimProcedureMemberRecipe {
    let mut obligations = consumer.obligations().to_vec();
    for obligation in class
        .factory()
        .obligations()
        .iter()
        .chain(procedure.declaration.obligations())
    {
        if !obligations.contains(obligation) {
            obligations.push(obligation.clone());
        }
    }
    AuthoredJimProcedureMemberRecipe {
        consumer,
        class,
        procedure,
        class_input,
        member,
        obligations,
    }
}

#[cfg(test)]
mod tests {
    use super::OriginalAuthoredJimProcedureMember;
    use crate::analyser::{Analyser, AnalysisResult};
    use crate::registry_invocation::source_structure::source_authored_jim_procedure_members_at;

    fn analysed(source: &str) -> AnalysisResult {
        tcl_spectcl::core_surfaces::ensure();
        let mut analyser = Analyser::new();
        analyser.analyse(source, "jimtcl")
    }
    fn member(
        source: &str,
        analyser: &AnalysisResult,
        marker: &str,
    ) -> OriginalAuthoredJimProcedureMember {
        let offset = u32::try_from(source.find(marker).unwrap()).unwrap();
        let mut receipts =
            source_authored_jim_procedure_members_at(source, &analyser, offset).unwrap();
        assert_eq!(receipts.len(), 1);
        receipts.pop().unwrap()
    }

    #[test]
    fn own_jim_member_keeps_before_and_after_factory_custody() {
        for (source, marker) in [
            (
                "class Point {x 0}\nproc {Point move} {dx} {return $dx}\n",
                "proc",
            ),
            (
                "proc {Point move} {dx} {return $dx}\nclass Point {x 0}\n",
                "class",
            ),
        ] {
            let analyser = analysed(source);
            let receipt = member(source, &analyser, marker);
            let class = receipt.class_metadata(&analyser).unwrap();
            assert!(class.source_name.is_none());
            assert!(class.methods.contains_key("move"));
            assert!(receipt.source_procedure(&analyser).is_some());
            assert_eq!(receipt.member().bytes(), b"move");
            assert!(!analyser.all_variables.contains_key("Point move::x"));
            assert!(!analyser.all_variables.contains_key("Point move::self"));
        }
    }

    #[test]
    fn own_jim_member_refuses_foreign_and_corrupt_recipes() {
        let source = "class Point {x 0}\nproc {Point move} {dx} {return $dx}\n";
        let analyser = analysed(source);
        let receipt = member(source, &analyser, "proc");
        assert!(receipt.class_metadata(&analyser).is_some());
        assert!(receipt.source_procedure(&analyser).is_some());
        let foreign = analysed(source);
        assert!(receipt.class_metadata(&foreign).is_none());
        assert!(receipt.source_procedure(&foreign).is_none());
        let shifted_source = format!("# distinct original input\n{source}");
        let shifted = analysed(&shifted_source);
        let foreign_receipt = member(&shifted_source, &shifted, "proc");
        let mut changed = receipt.clone();
        changed.recipe.class = foreign_receipt.recipe.class.clone();
        assert!(changed.class_metadata(&analyser).is_none());
        let mut changed = receipt.clone();
        changed.recipe.procedure = foreign_receipt.recipe.procedure.clone();
        assert!(changed.source_procedure(&analyser).is_none());
        let mut changed = receipt.clone();
        changed.recipe.member = changed.recipe.class_input.clone();
        assert!(changed.class_metadata(&analyser).is_none());
        let mut changed = receipt.clone();
        changed.recipe.class_input = changed.recipe.member.clone();
        assert!(changed.source_procedure(&analyser).is_none());
        let mut changed = receipt.clone();
        changed.recipe.consumer = std::sync::Arc::new(changed.recipe.class.factory().clone());
        assert!(changed.class_metadata(&analyser).is_none());
        let changed_source = format!("{source}# changed input\n");
        assert!(
            source_authored_jim_procedure_members_at(
                &changed_source,
                &analyser,
                u32::try_from(source.find("proc").unwrap()).unwrap()
            )
            .is_none()
        );
        let mut retired = receipt.clone();
        retired.owner = std::sync::Weak::new();
        assert!(retired.class_metadata(&analyser).is_none());
    }

    #[test]
    fn own_jim_member_refuses_rebound_class_and_replaced_initialisers() {
        for source in [
            "class Point {x 0}\nrename Point OldPoint\nproc {Point move} {dx} {return $dx}\n",
            "class Point {x 0}\nproc Point {} {return OTHER}\nproc {Point move} {dx} {return $dx}\n",
            "class Point {x 0}\nalias Alias Point\nproc {Alias move} {dx} {return $dx}\n",
            "class Point {x 0}\nalias Point puts\nproc {Point move} {dx} {return $dx}\n",
            "class Point {x 0}\nproc {{Point} move} {dx} {return $dx}\n",
        ] {
            let analyser = analysed(source);
            let offset = u32::try_from(source.rfind("proc {").unwrap()).unwrap();
            assert!(
                source_authored_jim_procedure_members_at(source, &analyser, offset)
                    .unwrap()
                    .is_empty()
            );
        }
        let source = "proc {Point new} {} {return OLD}\nclass Point {x 0}\n";
        let analyser = analysed(source);
        let offset = u32::try_from(source.find("class").unwrap()).unwrap();
        assert!(
            source_authored_jim_procedure_members_at(source, &analyser, offset)
                .unwrap()
                .is_empty()
        );
        let source = "class Point {x 0}\nputs OPAQUE\nproc {Point move} {dx} {return $dx}\n";
        let analyser = analysed(source);
        let receipt = member(source, &analyser, "proc");
        assert!(!receipt.supports_member_association(&analyser));
        let class = receipt.class_metadata(&analyser).unwrap();
        assert!(!class.methods.contains_key("move"));
        assert!(
            class
                .original_members
                .methods(crate::analyser::MemberSide::Instance)
                .is_none_or(|methods| methods
                    .iter()
                    .all(|method| method.metadata().name != "move"))
        );
        assert!(
            receipt
                .obligations()
                .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
        );
    }
}
