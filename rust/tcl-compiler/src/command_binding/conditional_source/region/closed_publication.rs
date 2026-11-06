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

//! One child publication under an exact completed source cut, never an entry.

use super::{ConditionalOutcomeKind, OriginalSingleSourceCutSurvey};
use crate::analyser::{AnalysisResult, ProcDef};
use crate::command_binding::{
    OriginalCommandPublication, OriginalCommandPublicationKind, OriginalCompletedCommandWorld,
};
use crate::signature_scan::original_name::SourceDeclarationMetadata;
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::{LexerConfig, SourceImage};
use tcl_syntax::naming::NamePolicyProtocol;

/// Modeled child-return publication and occupancy under one genuine completed
/// root cut. The ordinary child ledger remains absent. This grants no Native
/// entry, entered frame, compiler admission, execution or source edit.
pub struct OriginalClosedSourceCutPublication<'a> {
    prefix: &'a AnalysisResult,
    child: &'a AnalysisResult,
    outcome: &'a super::OriginalSourceConditionalOutcome,
    child_world: OriginalCompletedCommandWorld,
    publication: OriginalCommandPublication,
}

impl OriginalClosedSourceCutPublication<'_> {
    /// Exact inherited analysis attachment, not another equal-source analysis.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &AnalysisResult) -> bool {
        std::ptr::eq(self.child, analysis)
            && self.outcome.matches_prefix_analysis(self.prefix)
            && self.child.retained_command_realm().is_some_and(|realm| {
                realm
                    .source_bindings_ref()
                    .original_inherited_source_entry()
                    .is_some_and(|entry| {
                        entry.matches_parent_analysis(self.prefix)
                            && entry.matches_child_analysis(analysis)
                    })
            })
    }

    /// Child source extent retained independently of the root's outcome image.
    #[must_use]
    pub fn source_image(&self) -> &SourceImage {
        self.child_world.source_image()
    }

    /// Actual complete child lexer configuration.
    #[must_use]
    pub const fn lexer_config(&self) -> LexerConfig {
        self.child_world.lexer_config()
    }

    /// Original allocation surviving both child return and the root's Normal route.
    /// A consumer still owns all static-container, reference and editing checks.
    #[must_use]
    pub const fn publication(&self) -> &OriginalCommandPublication {
        &self.publication
    }

    /// Occupancy at the authenticated child return only. Root and peer worlds
    /// must still participate independently in collision checking.
    #[must_use]
    pub fn command_slot_occupied(
        &self,
        slot: &ByteCommandSlot,
        policy: NamePolicyProtocol,
    ) -> Option<bool> {
        self.child_world.command_slot_occupied(slot, policy)
    }
}

impl<'a> OriginalSingleSourceCutSurvey<'a> {
    /// Join one actual child procedure declaration to the same complete cut.
    /// Neither a conditional root world alone nor matching source bytes issue it.
    #[must_use]
    pub fn closed_child_procedure_publication(
        &self,
        declaration: &SourceDeclarationMetadata<ProcDef>,
    ) -> Option<OriginalClosedSourceCutPublication<'a>> {
        let ConditionalOutcomeKind::Regions(set) = &self.outcome.0 else {
            return None;
        };
        self.point
            .matches_parent_analysis(self.prefix)
            .then_some(())?;
        self.point
            .matches_child_analysis(self.child)
            .then_some(())?;
        self.conditional_body_projection(self.child)?;
        self.child
            .original_procedure_declarations()
            .any(|row| std::ptr::eq(row, declaration))
            .then_some(())?;
        let input = self.child.resolved_input.as_ref()?;
        if input.has_logical_source_name_context() || input.has_hosted_source_name_context() {
            return None;
        }
        let bindings = self.child.retained_command_realm()?.source_bindings_ref();
        let outcomes = &set.continuation.child_outcomes;
        if outcomes.normal_completion.is_none() || !outcomes.abrupt.is_empty() {
            return None;
        }
        let state = outcomes.normal.as_ref()?;
        let origin = bindings.source_origin()?;
        if state
            .current_source_origin
            .as_ref()
            .is_none_or(|current| !std::sync::Arc::ptr_eq(current, origin))
            || state.variable_frame != *self.point.frame()
        {
            return None;
        }
        // This is a private cut-scoped query surface, not an assignment to the
        // independently withheld ordinary root ledger or a Native capability.
        let child_world = bindings
            .completed_original_world_from_normal_state(std::sync::Arc::new((**state).clone()))?;
        let root_world = self.completed_body_world()?;
        let slot = declaration.name().slot();
        let policy = declaration.name().policy();
        let child_publication = child_world.declaration_at(slot, policy)?;
        let publication = root_world.declaration_at(slot, policy)?;
        if publication.kind() != OriginalCommandPublicationKind::Procedure
            || child_publication != publication
            || publication.declaration_site() != declaration.declaration_site()
            || publication.name_input()
                != &SignatureSourceNameInput::OriginalWord(declaration.name_input().clone())
        {
            return None;
        }
        let scope = self
            .child
            .original_namespace_scope_at(declaration.declaration_site().offset)?;
        let selected = policy
            .recipe()
            .command_publication_slot(scope.context()?, declaration.name_input().bytes())
            .ok()?;
        (selected == *slot).then_some(())?;
        Some(OriginalClosedSourceCutPublication {
            prefix: self.prefix,
            child: self.child,
            outcome: self.outcome,
            child_world,
            publication: publication.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    fn with_cut(
        source: &str,
        child_source: &str,
        check: impl FnOnce(&AnalysisResult, &AnalysisResult, &OriginalSingleSourceCutSurvey<'_>),
    ) {
        let prefix = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("source child").unwrap()).unwrap();
        let load = crate::registry_invocation::source_structure::original_project_source_load_at(
            source, &prefix, offset,
        )
        .unwrap();
        let point = load.source_point_entry(&prefix).unwrap();
        let child = Analyser::new()
            .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
            .unwrap();
        let regions = load
            .conditional_continuation(&prefix, &child)
            .unwrap()
            .source_regions(&prefix, &child)
            .unwrap();
        let suffix = Analyser::new()
            .analyse_source_region(&regions.regions()[0], &prefix, &child)
            .unwrap();
        let outcome = regions.normal_outcome().unwrap();
        let survey = outcome
            .single_source_cut_survey(&prefix, &[&child, &suffix])
            .unwrap();
        check(&prefix, &child, &survey);
    }

    #[test]
    fn closed_cut_publication_joins_real_child_and_deferred_parent_reference() {
        let source = "source child.tcl; proc run {} {helper}";
        let child_source = "proc helper {} {return VALUE}";
        with_cut(source, child_source, |prefix, child, survey| {
            assert!(child.original_completed_command_world().is_none());
            let declaration = child.original_procedure_declarations().next().unwrap();
            let publication = survey
                .closed_child_procedure_publication(declaration)
                .expect("actual child return and whole-root Normal publication");
            assert!(publication.is_for_analysis(child));
            assert!(!publication.is_for_analysis(prefix));
            assert!(!publication.is_for_analysis(&child.clone()));
            assert_eq!(publication.source_image().try_text().unwrap(), child_source);
            assert_eq!(
                publication.publication().declaration_site(),
                declaration.declaration_site()
            );
            assert_eq!(
                publication
                    .command_slot_occupied(declaration.name().slot(), declaration.name().policy(),),
                Some(true)
            );
            assert!(child.original_completed_command_world().is_none());
        });
    }

    #[test]
    fn closed_cut_publication_refuses_foreign_equal_source_declarations() {
        let child_source = "proc helper {} {return VALUE}";
        with_cut(
            "source child.tcl; proc run {} {helper}",
            child_source,
            |_, child, survey| {
                let declaration = child.original_procedure_declarations().next().unwrap();
                assert!(
                    survey
                        .closed_child_procedure_publication(declaration)
                        .is_some()
                );
                let ordinary = Analyser::new().analyse(child_source, "tcl8.6");
                let foreign = ordinary.original_procedure_declarations().next().unwrap();
                assert!(survey.closed_child_procedure_publication(foreign).is_none());
                let cloned = child.clone();
                let foreign = cloned.original_procedure_declarations().next().unwrap();
                assert!(survey.closed_child_procedure_publication(foreign).is_none());
            },
        );
    }

    #[test]
    fn closed_cut_publication_refuses_child_allocation_replaced_in_suffix() {
        with_cut(
            "source child.tcl; proc helper {} {return REPLACED}",
            "proc helper {} {return VALUE}",
            |_, child, survey| {
                let declaration = child.original_procedure_declarations().next().unwrap();
                assert!(
                    survey
                        .closed_child_procedure_publication(declaration)
                        .is_none()
                );
                assert!(child.original_completed_command_world().is_none());
            },
        );
    }

    #[test]
    fn closed_cut_survey_keeps_exact_child_owner_and_unique_membership() {
        let source = "source child.tcl; proc run {} {helper}";
        let child_source = "proc helper {} {return VALUE}";
        with_cut(source, child_source, |prefix, child, survey| {
            let load =
                crate::registry_invocation::source_structure::original_project_source_load_at(
                    source, prefix, 0,
                )
                .unwrap();
            let continuation = load.conditional_continuation(prefix, child).unwrap();
            let foreign = Analyser::new()
                .analyse_with_source_point_entry(child_source, "tcl8.6", survey.point)
                .unwrap();
            assert!(survey.point.matches_child_analysis(&foreign));
            assert!(continuation.matches_child_analysis(child));
            assert!(!continuation.matches_child_analysis(&foreign));
            assert_eq!(child.resolved_input, foreign.resolved_input);
            assert_eq!(child.body_lexer_config, foreign.body_lexer_config);
            assert!(child.original_completed_command_world().is_none());
            assert!(foreign.original_completed_command_world().is_none());
            let captured = survey
                .outcome
                .single_source_cut_survey(prefix, &[child, &foreign, survey.suffix_analysis()])
                .expect("foreign point-compatible input cannot replace the retained child");
            assert!(std::ptr::eq(captured.child_analysis(), child));
            assert!(std::ptr::eq(
                captured.suffix_analysis(),
                survey.suffix_analysis()
            ));
            let declaration = child.original_procedure_declarations().next().unwrap();
            let publication = captured
                .closed_child_procedure_publication(declaration)
                .expect("same retained child declaration under the genuine Normal outcome");
            assert!(publication.is_for_analysis(child));
            assert!(!publication.is_for_analysis(&foreign));
            assert!(
                survey
                    .outcome
                    .single_source_cut_survey(prefix, &[&foreign, survey.suffix_analysis()],)
                    .is_none(),
                "point compatibility alone cannot fill the missing child owner",
            );
            let retained = child.clone();
            assert!(!std::ptr::eq(child, &raw const retained));
            assert!(continuation.matches_child_analysis(&retained));
            let child_owner = child
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .original_analysis_owner()
                .unwrap();
            let retained_owner = retained
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .original_analysis_owner()
                .unwrap();
            let foreign_owner = foreign
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .original_analysis_owner()
                .unwrap();
            assert!(std::sync::Arc::ptr_eq(child_owner, retained_owner));
            assert!(!std::sync::Arc::ptr_eq(child_owner, foreign_owner));
            assert!(
                survey.outcome.single_source_cut_survey(
                    prefix, &[child, &retained, survey.suffix_analysis()],
                ).is_none(),
                "two eligible retained captures remain ambiguous membership",
            );
        });
    }

    #[test]
    fn direct_source_procedure_calls_do_not_gain_a_closed_cut() {
        let source = "source child.tcl; helper";
        let child_source = "proc helper {} {return VALUE}; helper";
        let prefix = Analyser::new().analyse(source, "tcl8.6");
        let load = crate::registry_invocation::source_structure::original_project_source_load_at(
            source, &prefix, 0,
        )
        .unwrap();
        let point = load.source_point_entry(&prefix).unwrap();
        let child = Analyser::new()
            .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
            .unwrap();
        assert!(child.original_completed_command_world().is_none());
        assert!(load.conditional_continuation(&prefix, &child).is_none());
    }
}
