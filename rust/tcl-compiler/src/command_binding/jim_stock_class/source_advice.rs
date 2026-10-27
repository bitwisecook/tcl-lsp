// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-advice custody from a completed modeled stock class publication.

use super::{Arc, OriginalJimStockClass, SourceCommandBindings, SourceJimClassHelper};
use crate::command_binding::source_command_world::OriginalCommandPublication;
use crate::command_binding::{BindingBaseline, OriginalSourceAnalysisOwner};

#[derive(Clone)]
pub(in crate::command_binding) struct OriginalJimStockSourceEffects {
    class: Arc<OriginalJimStockClass>,
    owner: Arc<OriginalSourceAnalysisOwner>,
    baseline: Arc<BindingBaseline>,
    publications: Vec<Arc<OriginalCommandPublication>>,
}
impl std::fmt::Debug for OriginalJimStockSourceEffects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalJimStockSourceEffects")
            .field("publications", &self.publications.len())
            .finish()
    }
}
impl PartialEq for OriginalJimStockSourceEffects {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.class, &other.class)
            && Arc::ptr_eq(&self.owner, &other.owner)
            && Arc::ptr_eq(&self.baseline, &other.baseline)
            && self.publications == other.publications
    }
}
impl Eq for OriginalJimStockSourceEffects {}

impl super::OriginalJimStockName {
    pub(in crate::command_binding) fn is_source_class_dispatcher(&self) -> bool {
        self.helper == SourceJimClassHelper::Dispatcher
            && self.method.is_none()
            && self.owner.base.is_none()
    }
    pub(in crate::command_binding) fn same_source_class_helper(
        &self,
        other: &Self,
        helper: SourceJimClassHelper,
    ) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && other.method.is_none() && other.helper == helper
    }
    pub(in crate::command_binding) fn matches_class_continuation(
        &self,
        point: &crate::command_binding::SourceBindingPoint,
    ) -> bool {
        self.is_source_class_dispatcher()
            && !point.dispatch
            && !point.declaration_preview
            && point.realm == tcl_dialect::model::InvocationRealm::RuleLoader
            && point.continuation_of.as_ref().is_some_and(|site| {
                site.offset == self.owner.site.offset
                    && Arc::ptr_eq(&site.source, &self.owner.site.source)
            })
    }
    pub(in crate::command_binding) fn source_class_effects(
        &self,
        owner: &Arc<OriginalSourceAnalysisOwner>,
        bindings: &SourceCommandBindings,
        point_index: usize,
        publications: Vec<Arc<OriginalCommandPublication>>,
    ) -> Option<OriginalJimStockSourceEffects> {
        let point = bindings.points.get(point_index)?;
        let state = &point.state;
        let input = state.baseline.metadata_context.source_analysis_input()?;
        (Arc::ptr_eq(owner, bindings.original_analysis_owner()?) && owner.matches(bindings, input))
            .then_some(())?;
        self.matches_class_continuation(point).then_some(())?;
        self.matches_publication_state(state).then_some(())?;
        for (helper, publication) in self.owner.recipe.publications().iter().zip(&publications) {
            let crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(value) =
                publication.name_input()
            else {
                return None;
            };
            self.same_source_class_helper(value.jim_stock_name()?, *helper)
                .then_some(())?;
            let key =
                state.original_command_key_for_slot(publication.slot(), publication.policy())?;
            let bindings = state.original_bindings_for_key(&key)?;
            let super::MayBinding::Target(target) = bindings.first()? else {
                return None;
            };
            (bindings.len() == 1
                && publication
                    .has_implementation_allocation(target.implementation_allocation.as_ref()?))
            .then_some(())?;
        }
        (self.owner.recipe.publications().len() == publications.len()).then_some(())?;
        Some(OriginalJimStockSourceEffects {
            class: Arc::clone(&self.owner),
            owner: Arc::clone(owner),
            baseline: Arc::clone(&state.baseline),
            publications,
        })
    }
}
impl OriginalJimStockSourceEffects {
    pub(in crate::command_binding) fn matches_seed(
        &self,
        owner: Option<&Arc<OriginalSourceAnalysisOwner>>,
        origin: &Arc<crate::command_binding::SourceOriginId>,
        baseline: &Arc<BindingBaseline>,
    ) -> bool {
        owner.is_some_and(|owner| Arc::ptr_eq(owner, &self.owner))
            && Arc::ptr_eq(baseline, &self.baseline)
            && Arc::ptr_eq(origin, &self.class.site.source)
    }
    pub(in crate::command_binding) fn original(&self) -> &[tcl_lexer::NativeWord] {
        &self.class.original
    }
    pub(in crate::command_binding) fn publications(
        &self,
        original: &[tcl_lexer::NativeWord],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<&[Arc<OriginalCommandPublication>]> {
        (self.class.original == original
            && self.class.policy == policy
            && self.class.base.is_none())
        .then_some(self.publications.as_slice())
    }
}
impl SourceCommandBindings {
    pub(in crate::command_binding) fn original_jim_stock_source_effects(
        &self,
    ) -> Vec<OriginalJimStockSourceEffects> {
        let Some(owner) = self.original_analysis_owner() else {
            return Vec::new();
        };
        let Some(origin) = self.source_origin() else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        for (point_index, point) in self.points.iter().enumerate() {
            if point.declaration_preview
                || point.dispatch
                || point
                    .continuation_of
                    .as_ref()
                    .is_none_or(|site| !Arc::ptr_eq(&site.source, origin))
                || !Arc::ptr_eq(&point.state.baseline, &self.final_state.baseline)
            {
                continue;
            }
            for effect in point.state.original_command_world.jim_stock_source_effects(
                self,
                point_index,
                owner,
            ) {
                if !effects.contains(&effect) {
                    effects.push(effect);
                }
            }
        }
        effects
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::analyser::{Analyser, AnalysisResult};
    use crate::command_binding::source_transition_advice::{
        OriginalSourceTransitionSeed, SourceCommandTransitionObligation,
    };

    fn analysed(source: &str) -> AnalysisResult {
        tcl_spectcl::core_surfaces::ensure();
        Analyser::new().analyse(source, "jimtcl")
    }
    fn seed(analysis: &AnalysisResult) -> OriginalSourceTransitionSeed {
        analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref()
            .original_source_transition_seed()
            .unwrap()
    }
    fn member(
        source: &str,
        analysis: &AnalysisResult,
    ) -> crate::command_binding::source_transition_advice::OriginalAuthoredJimProcedureMember {
        crate::registry_invocation::source_structure::source_authored_jim_procedure_members_at(
            source,
            analysis,
            u32::try_from(source.find("proc {").unwrap()).unwrap(),
        )
        .unwrap()
        .pop()
        .unwrap()
    }

    fn assert_creator_rejects_partial_and_foreign(
        analysis: &AnalysisResult,
        effect: &OriginalJimStockSourceEffects,
        foreign: &OriginalSourceTransitionSeed,
    ) {
        let realm = analysis.retained_command_realm().unwrap();
        let bindings = realm.source_bindings_ref();
        let owner = bindings.original_analysis_owner().unwrap();
        let crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(value) =
            effect.publications[0].name_input()
        else {
            panic!("original stock publication");
        };
        let dispatcher = value.jim_stock_name().unwrap();
        let (index, _) = bindings
            .points
            .iter()
            .enumerate()
            .find(|(_, point)| dispatcher.matches_class_continuation(point))
            .unwrap();
        let issue = |owner: &Arc<OriginalSourceAnalysisOwner>, publications| {
            dispatcher.source_class_effects(owner, bindings, index, publications)
        };
        assert!(issue(owner, effect.publications.clone()).is_some());
        let mut partial = effect.publications.clone();
        partial.pop();
        assert!(issue(owner, partial).is_none());
        let mut duplicate = effect.publications.clone();
        duplicate[1] = Arc::clone(&duplicate[0]);
        assert!(issue(owner, duplicate).is_none());
        let mut reordered = effect.publications.clone();
        reordered.swap(0, 1);
        assert!(issue(owner, reordered).is_none());
        assert!(
            issue(
                foreign.analysis_owner.as_ref().unwrap(),
                effect.publications.clone()
            )
            .is_none()
        );
        let wrong = bindings
            .points
            .iter()
            .position(|point| point.dispatch)
            .unwrap();
        assert!(
            dispatcher
                .source_class_effects(owner, bindings, wrong, effect.publications.clone())
                .is_none()
        );
    }

    #[test]
    fn stock_source_effects_need_the_actual_analysis_owner_and_complete_publications() {
        let source = "class Point {x 0}\nproc {Point move} {dx} {return $dx}\n";
        let analysis = analysed(source);
        let retained = seed(&analysis);
        assert_eq!(retained.stock_source_effects.len(), 1);
        let effect = &retained.stock_source_effects[0];
        assert_eq!(
            effect.publications.len(),
            effect.class.recipe.publications().len()
        );
        assert!(effect.matches_seed(
            retained.analysis_owner.as_ref(),
            &retained.origin,
            &retained.baseline
        ));
        assert!(!effect.matches_seed(None, &retained.origin, &retained.baseline));
        let foreign = seed(&analysed(source));
        assert!(!effect.matches_seed(
            foreign.analysis_owner.as_ref(),
            &retained.origin,
            &retained.baseline
        ));
        assert!(retained != foreign);
        assert_creator_rejects_partial_and_foreign(&analysis, effect, &foreign);
        let own = member(source, &analysis);
        assert!(own.supports_member_association(&analysis));
        assert!(
            !own.obligations()
                .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
        );
        let mut withdrawn = retained.clone();
        withdrawn.stock_source_effects.clear();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let tape = withdrawn.prepare(&context).unwrap();
        let candidates = tape.authored_jim_procedure_members(
            own.procedure_advice().original_words(),
            withdrawn.analysis_owner.as_ref().unwrap(),
        );
        assert_eq!(candidates.len(), 1);
        assert!(
            candidates[0]
                .obligations()
                .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
        );
    }

    #[test]
    fn stock_source_effects_keep_unknown_operations_and_changed_dependencies_terminal() {
        let source = "class Point {x 0}\nputs OPAQUE\nproc {Point move} {dx} {return $dx}\n";
        let analysis = analysed(source);
        let receipt = member(source, &analysis);
        assert!(!receipt.supports_member_association(&analysis));
        assert!(
            receipt
                .obligations()
                .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
        );
        let earlier = receipt.procedure_advice().uncertain_operations();
        assert!(
            earlier
                .iter()
                .any(|words| words.first().is_some_and(|head| head.bytes() == b"puts"))
        );
        assert!(
            !earlier
                .iter()
                .any(|words| words.first().is_some_and(|head| head.bytes() == b"class"))
        );
        let changed = "proc alias {args} {return CLOSED}\nclass Point {x 0}\nproc {Point move} {dx} {return $dx}\n";
        let changed_analysis = analysed(changed);
        assert!(seed(&changed_analysis).stock_source_effects.is_empty());
        assert!(!member(changed, &changed_analysis).supports_member_association(&changed_analysis));
    }
}
