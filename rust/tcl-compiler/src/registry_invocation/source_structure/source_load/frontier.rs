// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source cuts and conditional associations retain different kernel issuers.

use super::{AnalysisResult, OriginalProjectSourceLoad, capture_original_project_source_load_at};
use crate::command_binding::{OriginalSourceCommandTransitionAdvice, OriginalSourcePointEntry};
use crate::registry_invocation::source_structure::OriginalRegistrySource;
use std::sync::Arc;

/// A source association cannot supply a current cut or prove body inactivity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalProjectSourceFrontier {
    /// Actual finite original continuation and its pre-handler source point.
    CurrentCut(OriginalProjectSourceCurrentCut),
    /// Conditional declaration/deferred-body association with retained debts.
    ConditionalAssociation(OriginalProjectSourceAssociation),
    /// This view cannot issue either receipt. The host retains local debt.
    Unknown,
}

/// A finite source cut in the exact still-current parent interpretation.
/// Physical publication, path selection and child input remain host obligations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalProjectSourceCurrentCut {
    load: OriginalProjectSourceLoad,
    kernel: crate::command_binding::OriginalSourceFiniteCut,
    region: Option<crate::command_binding::OriginalSourceConditionalRegion>,
}
impl OriginalProjectSourceCurrentCut {
    /// The actual original path/schema card, without physical loading proof.
    #[must_use]
    pub fn source_load(&self) -> &OriginalProjectSourceLoad {
        &self.load
    }
    /// The same genuine source-only child ingress, never a Native entry.
    #[must_use]
    pub fn point(&self) -> &OriginalSourcePointEntry {
        self.kernel.point()
    }
    /// Raw path assignments before this cut's actual handler head. This omits
    /// the current SourceLoad barrier, never a preceding opaque/write barrier.
    /// Physical filename and genuine imported global homes remain host-owned;
    /// no runtime value, local-cell import or child-return continuity is granted.
    #[must_use]
    pub fn path_advice_before_handler(
        &self,
        parent: &AnalysisResult,
    ) -> Option<crate::auto_path_eval::PathConstantAssignments> {
        self.matches_parent_analysis(parent).then_some(())?;
        let head = self.load.words.head_source()?.word()?;
        crate::auto_path_eval::path_constant_prefix_for_source_analysis(
            head.image().try_text().ok()?,
            parent,
            head.span().start(),
        )
    }

    /// Rejoin this exact parent view; physical publication is independent.
    #[must_use]
    pub fn matches_parent_analysis(&self, parent: &AnalysisResult) -> bool {
        self.region.as_ref() == parent.original_source_region.as_ref()
            && self.kernel.matches_parent_analysis(parent)
            && self.load.authenticated_frontier_words(parent).is_some()
    }
}

/// A selected original body and load retain applicability and execution debts.
/// This grants no body inactivity, active cut, frame, Normal or child analysis.
#[derive(Clone)]
pub struct OriginalProjectSourceAssociation(Arc<AssociationPayload>);
struct AssociationPayload {
    load: OriginalProjectSourceLoad,
    owner: Arc<crate::command_binding::OriginalSourceAnalysisOwner>,
    body: Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    advice: Arc<OriginalSourceCommandTransitionAdvice>,
    region: Option<crate::command_binding::OriginalSourceConditionalRegion>,
}
impl std::fmt::Debug for OriginalProjectSourceAssociation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalProjectSourceAssociation")
            .field("body", &self.0.body.content_span())
            .finish_non_exhaustive()
    }
}
impl PartialEq for OriginalProjectSourceAssociation {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0.owner, &other.0.owner)
            && self.0.load == other.0.load
            && self.0.body == other.0.body
            && self.0.advice == other.0.advice
            && self.0.region == other.0.region
    }
}
impl Eq for OriginalProjectSourceAssociation {}
impl OriginalProjectSourceAssociation {
    /// Actual retained path/schema card; a dynamic operand cannot attest a URI.
    #[must_use]
    pub fn source_load(&self) -> &OriginalProjectSourceLoad {
        &self.0.load
    }
    /// Positive selected body lineage, independently of entered execution.
    #[must_use]
    pub fn source_body(&self) -> &crate::registry_invocation::OriginalSourceScriptBody {
        &self.0.body
    }
    /// Original applicability/lookup obligations remain attached, unflattened.
    #[must_use]
    pub fn source_advice(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.0.advice
    }
    /// Same actual current parent attachment, including any partial region.
    /// Byte-equal foreign parents cannot renew this attachment. The host
    /// separately revalidates physical publication and retirement.
    #[must_use]
    pub fn matches_parent_analysis(&self, parent: &AnalysisResult) -> bool {
        self.0.load.capture_source_association(parent).as_ref() == Some(self)
    }
}

impl crate::command_binding::OriginalSourceConditionalRegion {
    /// Transfer one retained prefix association/debt only through this region's
    /// actual replay of the same complete original command vector. The returned
    /// load/frontier pair belongs to the exact region view. Even Unknown retains
    /// that reissued load for a later exact replay; missing replay leaves the
    /// old debt unresolved. Physical publication remains independently checked.
    #[must_use]
    pub fn reissued_source_frontier(
        &self,
        prefix: &AnalysisResult,
        original_load: &OriginalProjectSourceLoad,
        region_analysis: &AnalysisResult,
    ) -> Option<(OriginalProjectSourceLoad, OriginalProjectSourceFrontier)> {
        let original = original_load.authenticated_frontier_words(prefix)?;
        self.reissues_original_source_command(prefix, region_analysis, &original)
            .then_some(())?;
        let image = self.source_image();
        let head = original.first()?;
        let current = capture_original_project_source_load_at(
            image.try_text().ok()?,
            region_analysis,
            head.span().start(),
        )?;
        (current.authenticated_frontier_words(region_analysis)? == original).then_some(())?;
        let frontier = current.frontier(region_analysis);
        Some((current, frontier))
    }
}

impl OriginalProjectSourceLoad {
    /// Classify only from the actual finite kernel or selected body issuer.
    /// Unknown is terminal local debt. Missing points and compiler reachability
    /// never prove an inactive body or a standalone child interpretation.
    #[must_use]
    pub fn frontier(&self, parent: &AnalysisResult) -> OriginalProjectSourceFrontier {
        if self.authenticated_frontier_words(parent).is_none() {
            return OriginalProjectSourceFrontier::Unknown;
        }
        if let Some(point) = self.source_point_entry(parent)
            && let Some(kernel) = parent.retained_command_realm().and_then(|realm| {
                realm
                    .source_bindings_ref()
                    .original_source_finite_cut(point, parent)
            })
        {
            return OriginalProjectSourceFrontier::CurrentCut(OriginalProjectSourceCurrentCut {
                load: self.clone(),
                kernel,
                region: parent.original_source_region.clone(),
            });
        }
        self.capture_source_association(parent).map_or(
            OriginalProjectSourceFrontier::Unknown,
            OriginalProjectSourceFrontier::ConditionalAssociation,
        )
    }

    fn authenticated_frontier_words(
        &self,
        parent: &AnalysisResult,
    ) -> Option<Vec<tcl_lexer::NativeWord>> {
        // A selected encoding needs its own child decode obligation/issuer.
        if self.path_at != 0 || self.words.arguments().len() != 1 {
            return None;
        }
        let input = parent.resolved_input.as_ref()?;
        let realm = parent.retained_command_realm()?;
        let image = realm.original_source_image()?;
        if !self.matches_source(image, input)
            || !parent.matches_original_source_image(image, input.lexer_config())
            || !realm.matches_resolved_analysis_input(input)
            || !realm
                .source_bindings_ref()
                .source_frontier_parent_view_matches(parent)
            || parent
                .original_source_region
                .as_ref()
                .is_some_and(|region| !region.matches_region_analysis(parent))
        {
            return None;
        }
        let offset = self.words.head_source()?.word()?.span().start();
        let selected =
            capture_original_project_source_load_at(image.try_text().ok()?, parent, offset)?;
        if selected != *self {
            return None;
        }
        let (actual_input, actual_realm, original) =
            super::super::registered_original_words_at(image.try_text().ok()?, parent, offset)?;
        (actual_input == input
            && std::ptr::eq(actual_realm, realm)
            && original.len() == 2
            && original.iter().all(|word| !word.group().expand)
            && self.words.head_source()?.word()? == &original[0]
            && self.path_word()? == &original[1])
            .then_some(original)
    }

    fn capture_source_association(
        &self,
        parent: &AnalysisResult,
    ) -> Option<OriginalProjectSourceAssociation> {
        let original = self.authenticated_frontier_words(parent)?;
        let input = parent.resolved_input.as_ref()?;
        let context = input.context_registry();
        let realm = parent.retained_command_realm()?;
        let bindings = realm.source_bindings_ref();
        let owner = bindings.original_analysis_owner()?;
        owner.matches(bindings, input).then_some(())?;
        let OriginalRegistrySource::SourceTransitions(advice) = self.words.source() else {
            return None;
        };
        let body = advice.original_source_load_association_body()?;
        if !advice.matches_source(original[0].image(), input.lexer_config())
            || !advice.matches_context(&context)
            || advice.original_words() != original
            || !body.matches_source(original[0].image(), input.lexer_config())
            || !body.matches_context(&context)
            || !body
                .original_commands(&context)?
                .iter()
                .any(|command| command.words == original)
        {
            return None;
        }
        let body_words = body.source_words();
        let OriginalRegistrySource::SourceTransitions(declaration) = body_words.source() else {
            return None;
        };
        if !declaration.matches_source(original[0].image(), input.lexer_config())
            || !declaration.matches_context(&context)
            || !declaration.uncertain_operations().is_empty()
            || advice
                .uncertain_operations()
                .iter()
                .any(|operation| operation.as_ref() != declaration.original_words())
        {
            return None;
        }
        let mut arguments = (0..body_words.arguments().len()).filter(|&argument| {
            body_words.original_argument_word(argument) == Some(body.original_container())
        });
        let argument = arguments.next()?;
        arguments.next().is_none().then_some(())?;
        body_words.with_source_schema(&context, |schema| {
            (schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
                && schema.authored_source_script_timing_at(argument)
                    == Some(tcl_registry::ScriptTiming::Deferred))
            .then_some(())
        })??;
        if bindings.source_association_has_conflicting_entry(declaration.site(), argument, &body) {
            return None;
        }
        Some(OriginalProjectSourceAssociation(Arc::new(
            AssociationPayload {
                load: self.clone(),
                owner: Arc::clone(owner),
                body,
                advice: Arc::clone(advice),
                region: parent.original_source_region.clone(),
            },
        )))
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::original_project_source_load_at;

    fn load(source: &str, parent: &AnalysisResult, text: &str) -> OriginalProjectSourceLoad {
        original_project_source_load_at(
            source,
            parent,
            u32::try_from(source.find(text).unwrap()).unwrap(),
        )
        .unwrap()
    }
    fn expect_association(
        frontier: OriginalProjectSourceFrontier,
    ) -> OriginalProjectSourceAssociation {
        let OriginalProjectSourceFrontier::ConditionalAssociation(association) = frontier else {
            panic!("genuine declared/deferred body association required: {frontier:?}");
        };
        association
    }
    fn expect_cut(frontier: OriginalProjectSourceFrontier) -> OriginalProjectSourceCurrentCut {
        let OriginalProjectSourceFrontier::CurrentCut(cut) = frontier else {
            panic!("genuine finite kernel cut required: {frontier:?}");
        };
        cut
    }

    #[test]
    fn path_prefix_is_cut_before_the_actual_head_and_refuses_foreign_owner() {
        let source = "set ::sourceDir /LIB; source child.tcl; set ::late LATE";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let load = load(source, &parent, "source child");
        let cut = expect_cut(load.frontier(&parent));
        let prefix = cut.path_advice_before_handler(&parent).unwrap();
        let folded = crate::auto_path_eval::fold_constant_assignments(&prefix, None);
        let at = load.path_word().unwrap().span().start();
        assert_eq!(folded.lookup_at("::sourceDir", at).as_deref(), Some("/LIB"));
        assert!(folded.lookup_at("::late", at).is_none());
        assert_ne!(folded.exported().len(), 0);
        let too_late = crate::auto_path_eval::fold_constant_assignments(
            &parent.path_constant_assignments.before(at),
            None,
        );
        assert_eq!(
            too_late.exported().len(),
            0,
            "operand offset includes own SourceLoad barrier"
        );
        let foreign = Analyser::new().analyse(source, "tcl8.6");
        assert!(cut.path_advice_before_handler(&foreign).is_none());
        let mut changed = parent.clone();
        changed.body_lexer_config = None;
        assert!(cut.path_advice_before_handler(&changed).is_none());
    }

    #[test]
    fn region_path_inventory_uses_only_its_actual_remaining_commands() {
        let source =
            "set ::before ROOT; source first.tcl; set ::regionValue SUFFIX; source second.tcl";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let first = load(source, &parent, "source first");
        let cut = expect_cut(first.frontier(&parent));
        let child = Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", cut.point())
            .unwrap();
        let continuation = first.conditional_continuation(&parent, &child).unwrap();
        let chain = continuation.source_regions(&parent, &child).unwrap();
        let region = chain.regions().first().unwrap();
        let view = Analyser::new()
            .analyse_source_region(region, &parent, &child)
            .unwrap();
        let inventory = region.path_inventory(&view).unwrap();
        let folded = crate::auto_path_eval::fold_constant_assignments(&inventory, None);
        let at = u32::try_from(source.find("source second").unwrap()).unwrap() - 1;
        assert_eq!(
            folded.lookup_at("::regionValue", at).as_deref(),
            Some("SUFFIX")
        );
        assert!(folded.lookup_at("::before", at).is_none());
        assert!(region.path_inventory(&parent).is_none());
        let mut stripped = view.clone();
        stripped.original_source_region = None;
        assert!(region.path_inventory(&stripped).is_none());
        assert!(crate::source_graph::current_analysis(source, &view).is_none());
    }

    #[test]
    fn inherited_path_prefix_retains_exact_source_namespace_home() {
        let source = "namespace eval app {source child.tcl}";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let first = load(source, &parent, "source child");
        let cut = expect_cut(first.frontier(&parent));
        let child_source = "set local LOCAL; set ::global GLOBAL; source leaf.tcl";
        let child = Analyser::new()
            .analyse_with_source_point_entry(child_source, "tcl8.6", cut.point())
            .unwrap();
        let leaf = load(child_source, &child, "source leaf");
        let leaf_cut = expect_cut(leaf.frontier(&child));
        let prefix = leaf_cut.path_advice_before_handler(&child).unwrap();
        let folded = crate::auto_path_eval::fold_constant_assignments(&prefix, None);
        let at = leaf.path_word().unwrap().span().start();
        assert_eq!(folded.lookup_at("local", at).as_deref(), Some("LOCAL"));
        assert_eq!(
            folded.lookup_at("::app::local", at).as_deref(),
            Some("LOCAL")
        );
        assert!(folded.lookup_at("::local", at).is_none());
        assert_eq!(folded.lookup_at("::global", at).as_deref(), Some("GLOBAL"));
    }

    #[test]
    fn finite_root_and_namespace_cuts_retain_actual_kernel_plans() {
        for source in [
            "source child.tcl\n",
            "namespace eval ::app {source child.tcl}\n",
        ] {
            let parent = Analyser::new().analyse(source, "tcl8.6");
            let load = load(source, &parent, "source child");
            let cut = expect_cut(load.frontier(&parent));
            assert!(cut.matches_parent_analysis(&parent));
            assert!(cut.point().matches_parent_analysis(&parent));
            assert_eq!(cut.source_load(), &load);
            let mut unavailable = parent.clone();
            unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
                environment: "tcl8.6".to_owned(),
                overlay: 1,
            });
            assert!(!cut.matches_parent_analysis(&unavailable));
            assert!(matches!(
                load.frontier(&unavailable),
                OriginalProjectSourceFrontier::Unknown
            ));
            let mut changed = parent.clone();
            changed.body_lexer_config = None;
            assert!(!cut.matches_parent_analysis(&changed));
            changed = parent.clone();
            changed.resolved_input = None;
            assert!(!cut.matches_parent_analysis(&changed));
            assert_eq!(
                load.frontier(&parent),
                OriginalProjectSourceFrontier::CurrentCut(cut)
            );
        }
    }

    #[test]
    fn declared_procedure_and_deferred_package_body_keep_conditional_association_debts() {
        for source in [
            "proc load_optional {} {source optional.tcl}\nproc local {} {}\nlocal\n",
            "package ifneeded Optional 1.0 {source optional.tcl}\nproc local {} {}\nlocal\n",
        ] {
            let parent = Analyser::new().analyse(source, "tcl8.6");
            let load = load(source, &parent, "source optional");
            let association = expect_association(load.frontier(&parent));
            assert!(association.matches_parent_analysis(&parent));
            assert_eq!(association.source_load(), &load);
            assert_eq!(
                association.source_load().literal_path().as_deref(),
                Some("optional.tcl")
            );
            assert_ne!(association.source_advice().obligations().len(), 0);
            assert_ne!(
                association
                    .source_body()
                    .original_commands(&parent.resolved_input.as_ref().unwrap().context_registry())
                    .unwrap()
                    .len(),
                0
            );
            assert_eq!(
                load.frontier(&parent),
                OriginalProjectSourceFrontier::ConditionalAssociation(association)
            );
        }
    }

    #[test]
    fn association_receipts_refuse_foreign_parent_and_unavailable_or_changed_input() {
        let source = "proc load_optional {} {source optional.tcl}\n";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let load = load(source, &parent, "source optional");
        let association = expect_association(load.frontier(&parent));
        let foreign = Analyser::new().analyse(source, "tcl8.6");
        let foreign_load = original_project_source_load_at(
            source,
            &foreign,
            u32::try_from(source.find("source optional").unwrap()).unwrap(),
        )
        .unwrap();
        let foreign_receipt = expect_association(foreign_load.frontier(&foreign));
        assert!(foreign_receipt.matches_parent_analysis(&foreign));
        assert!(!association.matches_parent_analysis(&foreign));
        assert!(!foreign_receipt.matches_parent_analysis(&parent));
        assert_ne!(association, foreign_receipt);
        let mut unavailable = parent.clone();
        unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
            environment: "tcl8.6".to_owned(),
            overlay: 1,
        });
        assert!(!association.matches_parent_analysis(&unavailable));
        assert!(matches!(
            load.frontier(&unavailable),
            OriginalProjectSourceFrontier::Unknown
        ));
        let mut changed = parent.clone();
        changed.body_lexer_config = None;
        assert!(!association.matches_parent_analysis(&changed));
        changed = parent.clone();
        changed.resolved_input = None;
        assert!(!association.matches_parent_analysis(&changed));
    }

    #[test]
    fn unsupported_decode_dynamic_path_and_later_held_load_remain_unknown() {
        for (source, target) in [
            ("source -encoding utf-8 child.tcl\n", "source -encoding"),
            ("source $runtimePath\n", "source $runtimePath"),
            ("source first.tcl\nsource second.tcl\n", "source second"),
        ] {
            let parent = Analyser::new().analyse(source, "tcl8.6");
            let load = load(source, &parent, target);
            assert!(
                matches!(
                    load.frontier(&parent),
                    OriginalProjectSourceFrontier::Unknown
                ),
                "{source}"
            );
        }
    }

    #[test]
    fn independently_selected_enclosing_body_is_not_an_association_only_frontier() {
        let source = "proc load_optional {} {source optional.tcl}\nload_optional\n";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let load = load(source, &parent, "source optional");
        assert!(matches!(
            load.frontier(&parent),
            OriginalProjectSourceFrontier::Unknown
        ));
    }

    #[test]
    fn whole_scope_join_refuses_a_stripped_region_but_keeps_full_child_views() {
        let source = "source first.tcl; source second.tcl";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        assert!(crate::source_graph::current_analysis(source, &parent).is_some());
        let first = load(source, &parent, "source first");
        let cut = expect_cut(first.frontier(&parent));
        let child = Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", cut.point())
            .unwrap();
        assert!(crate::source_graph::current_analysis("", &child).is_some());
        let continuation = first.conditional_continuation(&parent, &child).unwrap();
        let chain = continuation.source_regions(&parent, &child).unwrap();
        let region = chain.regions().first().unwrap();
        let view = Analyser::new()
            .analyse_source_region(region, &parent, &child)
            .unwrap();
        assert!(region.matches_region_analysis(&view));
        assert!(crate::source_graph::current_analysis(source, &view).is_none());
        let mut stripped = view.clone();
        stripped.original_source_region = None;
        // Image matching remains available to genuine region-specific APIs.
        let input = stripped.resolved_input.as_ref().unwrap();
        assert!(stripped.matches_original_source_image(
            &tcl_lexer::SourceImage::document(source),
            input.lexer_config()
        ));
        assert!(crate::source_graph::current_analysis(source, &stripped).is_none());
    }

    #[test]
    fn replayed_frontier_transfers_only_the_exact_prefix_command_debt() {
        for source in [
            "source first.tcl; source second.tcl; source third.tcl",
            "namespace eval ::app {source first.tcl; source second.tcl; source third.tcl}",
        ] {
            let parent = Analyser::new().analyse(source, "tcl8.6");
            let first = load(source, &parent, "source first");
            let later = load(source, &parent, "source second");
            assert!(matches!(
                later.frontier(&parent),
                OriginalProjectSourceFrontier::Unknown
            ));
            let first_cut = expect_cut(first.frontier(&parent));
            let child = Analyser::new()
                .analyse_with_source_point_entry("", "tcl8.6", first_cut.point())
                .unwrap();
            let continuation = first.conditional_continuation(&parent, &child).unwrap();
            let chain = continuation.source_regions(&parent, &child).unwrap();
            let region = chain.regions().first().unwrap();
            let view = Analyser::new()
                .analyse_source_region(region, &parent, &child)
                .unwrap();
            let (reissued_load, replayed) = region
                .reissued_source_frontier(&parent, &later, &view)
                .unwrap();
            let replayed = expect_cut(replayed);
            assert_eq!(replayed.source_load(), &reissued_load);
            let third = load(source, &parent, "source third");
            let (held_third, held_frontier) = region
                .reissued_source_frontier(&parent, &third, &view)
                .unwrap();
            assert!(matches!(
                held_frontier,
                OriginalProjectSourceFrontier::Unknown
            ));
            let second_child = Analyser::new()
                .analyse_with_source_point_entry("", "tcl8.6", replayed.point())
                .unwrap();
            let second_continuation = reissued_load
                .conditional_continuation(&view, &second_child)
                .unwrap();
            let second_chain = second_continuation
                .source_regions(&view, &second_child)
                .unwrap();
            let second_region = second_chain.regions().first().unwrap();
            let second_view = Analyser::new()
                .analyse_source_region(second_region, &view, &second_child)
                .unwrap();
            let (_, third_frontier) = second_region
                .reissued_source_frontier(&view, &held_third, &second_view)
                .unwrap();
            assert!(expect_cut(third_frontier).matches_parent_analysis(&second_view));
            // An ancestor prefix cannot bypass the genuine intermediate owner.
            assert!(
                second_region
                    .reissued_source_frontier(&parent, &third, &second_view)
                    .is_none()
            );
            assert!(replayed.matches_parent_analysis(&view));
            assert_eq!(
                replayed.source_load().literal_path().as_deref(),
                Some("second.tcl")
            );
            assert!(
                region
                    .reissued_source_frontier(&parent, &first, &view)
                    .is_none()
            );
            let foreign = Analyser::new().analyse(source, "tcl8.6");
            assert!(
                region
                    .reissued_source_frontier(&foreign, &later, &view)
                    .is_none()
            );
            let mut unavailable = parent.clone();
            unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
                environment: "tcl8.6".to_owned(),
                overlay: 1,
            });
            assert!(
                region
                    .reissued_source_frontier(&unavailable, &later, &view)
                    .is_none()
            );
            let mut unavailable_view = view.clone();
            unavailable_view.analysis_context_unavailable =
                unavailable.analysis_context_unavailable.clone();
            assert!(
                region
                    .reissued_source_frontier(&parent, &later, &unavailable_view)
                    .is_none()
            );
            unavailable_view = view.clone();
            unavailable_view.original_source_region = None;
            assert!(
                region
                    .reissued_source_frontier(&parent, &later, &unavailable_view)
                    .is_none()
            );
            let foreign_child = Analyser::new()
                .analyse_with_source_point_entry("", "tcl8.6", first_cut.point())
                .unwrap();
            let foreign_continuation = first
                .conditional_continuation(&parent, &foreign_child)
                .unwrap();
            let foreign_chain = foreign_continuation
                .source_regions(&parent, &foreign_child)
                .unwrap();
            let foreign_region = foreign_chain.regions().first().unwrap();
            let foreign_view = Analyser::new()
                .analyse_source_region(foreign_region, &parent, &foreign_child)
                .unwrap();
            assert!(
                foreign_region
                    .reissued_source_frontier(&parent, &later, &foreign_view)
                    .is_some()
            );
            assert!(
                region
                    .reissued_source_frontier(&parent, &later, &foreign_view)
                    .is_none()
            );
        }
    }

    #[test]
    fn region_cut_rejoins_the_exact_view_and_refuses_equal_byte_foreign_region() {
        let source = "source first.tcl; source second.tcl";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let first = load(source, &parent, "source first");
        let first_cut = expect_cut(first.frontier(&parent));
        let child = Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", first_cut.point())
            .unwrap();
        let continuation = first.conditional_continuation(&parent, &child).unwrap();
        let chain = continuation.source_regions(&parent, &child).unwrap();
        assert_eq!(chain.regions().len(), 1);
        let region = &chain.regions()[0];
        let view = Analyser::new()
            .analyse_source_region(region, &parent, &child)
            .unwrap();
        let second = load(source, &view, "source second");
        let second_cut = expect_cut(second.frontier(&view));
        assert!(second_cut.matches_parent_analysis(&view));
        let mut stripped = view.clone();
        stripped.original_source_region = None;
        assert!(!second_cut.matches_parent_analysis(&stripped));
        assert!(matches!(
            second.frontier(&stripped),
            OriginalProjectSourceFrontier::Unknown
        ));
        assert!(!second_cut.matches_parent_analysis(&parent));
        assert!(matches!(
            first.frontier(&view),
            OriginalProjectSourceFrontier::Unknown
        ));
        let foreign_child = Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", first_cut.point())
            .unwrap();
        let foreign_continuation = first
            .conditional_continuation(&parent, &foreign_child)
            .unwrap();
        let foreign_chain = foreign_continuation
            .source_regions(&parent, &foreign_child)
            .unwrap();
        let foreign_region = &foreign_chain.regions()[0];
        let foreign_view = Analyser::new()
            .analyse_source_region(foreign_region, &parent, &foreign_child)
            .unwrap();
        assert!(foreign_region.matches_region_analysis(&foreign_view));
        assert!(!second_cut.matches_parent_analysis(&foreign_view));
    }
}
