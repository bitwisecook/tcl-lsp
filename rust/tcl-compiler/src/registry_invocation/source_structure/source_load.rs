// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional current-interpreter source paths from actual original schemas.

mod frontier;
pub use frontier::{
    OriginalProjectSourceAssociation, OriginalProjectSourceCurrentCut,
    OriginalProjectSourceFrontier,
};

use super::{OriginalRegistryWords, source_registry_words_at};
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use crate::registry_invocation::{load_trace_flag, load_trace_option};

/// One original source-load path under a retained current-interpreter schema.
/// This is project association advice, without file existence, working-directory
/// knowledge, actual loading, interpreter entry or normal completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalProjectSourceLoad {
    input: ResolvedAnalysisInput,
    words: OriginalRegistryWords,
    path_at: usize,
}

impl OriginalProjectSourceLoad {
    /// Join the actual parent continuation and the same source-only child normal
    /// outcome. This supplies no physical load, Native completion or compiler visit.
    #[must_use]
    pub fn conditional_continuation(
        &self,
        parent: &crate::analyser::AnalysisResult,
        child: &crate::analyser::AnalysisResult,
    ) -> Option<crate::command_binding::OriginalSourceConditionalContinuation> {
        let point = self.source_point_entry(parent)?;
        parent
            .retained_command_realm()?
            .source_bindings_ref()
            .conditional_source_continuation(point, parent, child)
    }

    /// Continue using the same child's completed conditional region chain.
    /// The original prefix analysis is retained; no fresh analysis renews it.
    #[must_use]
    pub fn conditional_continuation_with_outcome(
        &self,
        parent: &AnalysisResult,
        child_prefix: &AnalysisResult,
        outcome: &crate::command_binding::OriginalSourceConditionalOutcome,
    ) -> Option<crate::command_binding::OriginalSourceConditionalContinuation> {
        let point = self.source_point_entry(parent)?;
        parent
            .retained_command_realm()?
            .source_bindings_ref()
            .conditional_source_continuation_with_outcome(
                point,
                parent,
                child_prefix,
                Some(outcome),
            )
    }

    /// Rejoin this actual load with its parent's retained pre-handler point.
    /// The host separately owns physical path selection and publication currency.
    #[must_use]
    pub fn source_point_entry(
        &self,
        parent: &AnalysisResult,
    ) -> Option<crate::command_binding::OriginalSourcePointEntry> {
        // Optional/dynamic decoding requires a separately owned source ingress.
        // A path association alone cannot attest the host's decoded child image.
        if self.path_at != 0 || self.words.arguments().len() != 1 {
            return None;
        }
        let input = parent.resolved_input.as_ref()?;
        let realm = parent.retained_command_realm()?;
        let image = realm.original_source_image()?;
        if !self.matches_source(image, input)
            || !parent.matches_original_source_image(image, input.lexer_config())
            || !realm.matches_resolved_analysis_input(input)
        {
            return None;
        }
        let head = self.words.head_source()?.word()?;
        // The load card cannot donate its old selected hook to a different
        // current source world with identical bytes. Reselect through this
        // parent's authentic Registry/source owner before capturing its state.
        let selected = capture_original_project_source_load_at(
            image.try_text().ok()?,
            parent,
            head.span().start(),
        )?;
        if selected.path_at != self.path_at || selected.words != self.words {
            return None;
        }
        let (actual_input, actual_realm, original) = super::registered_original_words_at(
            image.try_text().ok()?,
            parent,
            head.span().start(),
        )?;
        if original.len() != 2
            || actual_input != input
            || !std::ptr::eq(actual_realm, realm)
            || self.path_word()? != original.get(self.path_at.checked_add(1)?)?
        {
            return None;
        }
        let entry = realm.source_bindings_ref().original_source_point_entry(
            input,
            &original,
            self.path_at,
        )?;
        entry.matches_parent_analysis(parent).then_some(entry)
    }

    /// Actual selected post-head path ordinal, independent of reporting text.
    #[must_use]
    pub const fn path_argument(&self) -> usize {
        self.path_at
    }

    /// Original whole path operand, retaining its source and complete grammar.
    #[must_use]
    pub fn path_word(&self) -> Option<&tcl_lexer::NativeWord> {
        self.words.original_argument_word(self.path_at)
    }

    /// Static original Unicode path only. Folded report values cannot supply it.
    #[must_use]
    pub fn literal_path(&self) -> Option<String> {
        String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
            self.path_word()?,
        )?)
        .ok()
    }

    /// Same whole source, complete immutable input and selected source schema.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        &self.input == input
            && self.words.matches_source(image, input.lexer_config())
            && self
                .words
                .matches_registry(input.borrowed_context_registry().commands())
            && self.words.context() == Some(input.borrowed_context_registry().context())
            && self
                .path_word()
                .is_some_and(|path| path.image() == image && path.config() == input.lexer_config())
    }
}

/// Current retained project path at the exact original source invocation.
/// Missing catalogue ownership never manufactures a current-interpreter scope.
#[must_use]
pub fn original_project_source_load_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<OriginalProjectSourceLoad> {
    let input = analysis.resolved_input.as_ref()?;
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, input.lexer_config())
        .then_some(())?;
    let mut candidates = analysis.source_targets.iter().filter_map(|target| {
        let load = target.original_project_source_load.as_ref()?;
        (target.original_interpreter_source_load.is_none()
            && load.words.head_source()?.span().start() == offset
            && load.path_word()?.span() == target.range
            && load.matches_source(&image, input))
        .then_some(load.as_ref())
    });
    let first = candidates.next()?;
    candidates
        .all(|candidate| candidate == first)
        .then(|| first.clone())
}

/// Capture at the analyser's independently selected current-interpreter hook.
/// Child-interpreter visibility has its own independent load owner.
#[must_use]
pub(crate) fn capture_original_project_source_load_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<OriginalProjectSourceLoad> {
    let input = load_trace_option("input", Some(offset), analysis.resolved_input.as_ref())?;
    let image = tcl_lexer::SourceImage::document(source);
    if !load_trace_flag(
        "image",
        Some(offset),
        analysis.matches_original_source_image(&image, input.lexer_config()),
    ) {
        return None;
    }
    let words = load_trace_option(
        "words",
        Some(offset),
        source_registry_words_at(source, analysis, offset),
    )?;
    let context = input.context_registry();
    let path_at = load_trace_option(
        "schema",
        Some(offset),
        words.with_source_schema(&context, |schema| {
            if load_trace_flag(
                "schema.not-current-interpreter",
                Some(offset),
                schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current,
            ) {
                return None;
            }
            let transitions = schema.state_transitions();
            let mut paths = transitions
                .facts()
                .iter()
                .filter_map(|fact| match &fact.transition {
                    tcl_registry::StateTransition::Package(
                        tcl_registry::model::binding::PackageTransition::SourceLoad { path },
                    ) => path.argument_index(),
                    _ => None,
                });
            let first = load_trace_option("path.first", Some(offset), paths.next())?;
            load_trace_flag(
                "path.unanimity",
                Some(offset),
                paths.all(|path| path == first),
            )
            .then_some(first)
        }),
    )??;
    let load = OriginalProjectSourceLoad {
        input: input.clone(),
        words,
        path_at,
    };
    load_trace_flag(
        "final-source",
        Some(offset),
        load.matches_source(&image, input),
    )
    .then_some(load)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn project_source_paths_keep_original_grammar_and_decline_reporting_values() {
        for (source, dialect, literal) in [
            ("source {a $b [c].tcl}", "tcl8.6", Some("a $b [c].tcl")),
            ("source -encoding utf-8 lib.tcl", "tcl8.6", Some("lib.tcl")),
            ("source -nopkg lib.tcl", "tcl9.0", Some("lib.tcl")),
            ("source $path", "tcl8.6", None),
        ] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            let load = original_project_source_load_at(source, &analysis, 0).expect(source);
            assert_eq!(load.literal_path().as_deref(), literal, "{source}");
            let input = analysis.resolved_input.as_ref().unwrap();
            assert!(load.matches_source(&tcl_lexer::SourceImage::document(source), input));
            assert!(!load.matches_source(
                &tcl_lexer::SourceImage::document(&format!("{source} ")),
                input
            ));
            for target in &mut analysis.source_targets {
                target.raw_path = "forged.tcl".to_owned();
                target.is_literal = true;
            }
            assert_eq!(
                original_project_source_load_at(source, &analysis, 0)
                    .unwrap()
                    .literal_path()
                    .as_deref(),
                literal
            );
            analysis.resolved_input = None;
            assert!(original_project_source_load_at(source, &analysis, 0).is_none());
        }
        let child_source = "interp create child; child eval {source library.tcl}";
        let child = Analyser::new().analyse(child_source, "tcl8.6");
        assert!(
            original_project_source_load_at(
                child_source,
                &child,
                u32::try_from(child_source.find("source").unwrap()).unwrap()
            )
            .is_none()
        );
        for source in ["source {*}$args", "proc source {x} {}; source lib.tcl"] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind("source").unwrap()).unwrap();
            assert!(
                original_project_source_load_at(source, &analysis, offset).is_none(),
                "{source}"
            );
        }
    }
}

#[cfg(test)]
mod source_point_tests {
    use super::*;
    use crate::analyser::Analyser;

    fn parent() -> (&'static str, AnalysisResult, OriginalProjectSourceLoad) {
        let source = "proc helper {} {return ROOT}; namespace eval lib {proc helper {} {return GOOD}}; namespace eval app {namespace path ::lib; source child.tcl}";
        let result = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("source child.tcl").unwrap()).unwrap();
        let load = original_project_source_load_at(source, &result, offset).unwrap();
        (source, result, load)
    }

    #[test]
    fn source_point_receipt_retains_the_actual_parent_attachment() {
        let (source, a, load_a) = parent();
        let a_entry = load_a
            .source_point_entry(&a)
            .expect("exact pre-handler source point");
        assert!(a_entry.matches_parent_analysis(&a));
        assert!(a_entry.matches_parent_analysis(&a.clone()));
        assert_eq!(Some(a_entry.clone()), load_a.source_point_entry(&a));
        let b = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("source child.tcl").unwrap()).unwrap();
        let load_b = original_project_source_load_at(source, &b, offset).unwrap();
        let b_entry = load_b
            .source_point_entry(&b)
            .expect("new source point owner");
        assert_ne!(a_entry, b_entry);
        assert!(!a_entry.matches_parent_analysis(&b));
        assert!(!b_entry.matches_parent_analysis(&a));
        let mut wrong_body_config = a.clone();
        wrong_body_config.body_lexer_config = None;
        assert!(!a_entry.matches_parent_analysis(&wrong_body_config));
        assert!(load_a.source_point_entry(&wrong_body_config).is_none());
        let mut unavailable = a.clone();
        unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
            environment: "tcl8.6".to_owned(),
            overlay: 1,
        });
        assert!(!a_entry.matches_parent_analysis(&unavailable));
        assert!(load_a.source_point_entry(&unavailable).is_none());
        let mut unavailable = a.clone();
        unavailable.resolved_input = None;
        assert!(!a_entry.matches_parent_analysis(&unavailable));
        assert!(load_a.source_point_entry(&unavailable).is_none());
    }

    #[test]
    fn source_point_unavailable_state_never_becomes_a_namespace_seed() {
        for source in [
            "unknown_effect; source child.tcl",
            "trace add execution source enter callback; source child.tcl",
            "proc source args {}; source child.tcl",
        ] {
            let result = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind("source child.tcl").unwrap()).unwrap();
            if let Some(load) = original_project_source_load_at(source, &result, offset) {
                assert!(load.source_point_entry(&result).is_none(), "{source}");
            }
        }
    }
    #[test]
    fn inherited_source_point_keeps_path_and_actual_ancestor_without_native_grants() {
        let (parent_source, parent_a, load_a) = parent();
        let entry_a = load_a.source_point_entry(&parent_a).unwrap();
        let child_source = "proc run {} {helper}";
        let offset = u32::try_from(child_source.find("helper").unwrap()).unwrap();
        let mut analyser = Analyser::new();
        let child = analyser
            .analyse_with_source_point_entry(child_source, "tcl8.6", &entry_a)
            .expect("authentic inherited source view");
        assert!(entry_a.matches_child_analysis(&child));
        assert!(!entry_a.matches_child_analysis(&parent_a));
        assert!(child.original_completed_command_world().is_none());
        let call = super::super::source_original_procedure_call_at(child_source, &child, offset)
            .expect("conditional path-selected parent helper");
        assert!(call.declaration(&child).is_none());
        assert_eq!(
            call.declaration_in_parent(&child, &parent_a)
                .unwrap()
                .name_input()
                .bytes(),
            b"helper"
        );
        let header = call.declaration_in_parent(&child, &parent_a).unwrap();
        assert_eq!(
            &parent_source[header.metadata().body_span.as_range()],
            "return GOOD"
        );
        let declarations = child.original_procedure_declarations().collect::<Vec<_>>();
        assert_eq!(declarations.len(), 1);
        assert!(matches!(
            declarations[0].declaration_site().source.kind(),
            crate::command_binding::SourceOriginKind::InheritedPoint { .. }
        ));
        let (_, parent_b, load_b) = parent();
        let entry_b = load_b.source_point_entry(&parent_b).unwrap();
        assert_ne!(entry_a, entry_b);
        assert!(call.declaration_in_parent(&child, &parent_b).is_none());
        let second = Analyser::new()
            .analyse_with_source_point_entry(child_source, "tcl8.6", &entry_b)
            .unwrap();
        assert!(call.declaration_in_parent(&second, &parent_a).is_none());
        assert!(!entry_a.matches_child_analysis(&second));
        assert!(entry_b.matches_child_analysis(&second));
        let mut unavailable_child = child.clone();
        unavailable_child.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
            environment: "tcl8.6".to_owned(),
            overlay: 1,
        });
        assert!(!entry_a.matches_child_analysis(&unavailable_child));
        let mut wrong_config_child = child.clone();
        wrong_config_child.body_lexer_config = None;
        assert!(!entry_a.matches_child_analysis(&wrong_config_child));
        let input = child.resolved_input.as_ref().unwrap();
        let unit = child
            .original_optimiser_source_unit(
                child_source,
                input.borrowed_context_registry().commands(),
                None,
            )
            .expect("same retained child entry CU");
        assert_eq!(
            unit.ir_module.source_entry.source_point_entry.as_ref(),
            Some(&entry_a)
        );
        assert!(unit.ir_module.source_entry.native_entry.is_none());
        assert!(
            unit.ir_module
                .source_entry
                .compiled_variable_provider
                .is_none()
        );
        assert_eq!(
            unit.ir_module.source_entry.native_compilation,
            tcl_registry::native_compilation::NativeCompilationContext::default()
        );
        assert_eq!(
            unit.ir_module.original_native_declaration_body_units.len(),
            0
        );
        assert!(
            child
                .original_optimiser_source_unit(
                    "proc run {} {WRONG}",
                    input.borrowed_context_registry().commands(),
                    None
                )
                .is_none()
        );
        let fresh = analyser.analyse(child_source, "tcl8.6");
        assert!(
            super::super::source_original_procedure_call_at(child_source, &fresh, offset).is_none()
        );
        assert!(
            Analyser::new()
                .analyse_with_source_point_entry(child_source, "tcl9.0", &entry_a)
                .is_none()
        );
    }

    #[test]
    fn inherited_source_point_rejoins_transitive_actual_ancestors() {
        let (_, root_parent, load) = parent();
        let entry = load.source_point_entry(&root_parent).unwrap();
        let middle_source = "source leaf.tcl";
        let middle = Analyser::new()
            .analyse_with_source_point_entry(middle_source, "tcl8.6", &entry)
            .unwrap();
        let middle_load = original_project_source_load_at(middle_source, &middle, 0).unwrap();
        let middle_entry = middle_load.source_point_entry(&middle).unwrap();
        let leaf_source = "proc run {} {helper}";
        let leaf = Analyser::new()
            .analyse_with_source_point_entry(leaf_source, "tcl8.6", &middle_entry)
            .unwrap();
        let call = super::super::source_original_procedure_call_at(
            leaf_source,
            &leaf,
            u32::try_from(leaf_source.find("helper").unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            call.declaration_in_parent(&leaf, &root_parent)
                .unwrap()
                .name_input()
                .bytes(),
            b"helper"
        );
        assert!(call.declaration_in_parent(&leaf, &middle).is_none());
        let (_, foreign, _) = parent();
        assert!(call.declaration_in_parent(&leaf, &foreign).is_none());
    }

    #[test]
    fn carried_load_card_cannot_select_a_different_inherited_hook() {
        let outer_a = "namespace eval app {::source parent.tcl}";
        let outer_b = "namespace eval app {proc source args {}; ::source parent.tcl}";
        let make_entry = |source: &str| {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.find("::source parent.tcl").unwrap()).unwrap();
            original_project_source_load_at(source, &analysis, offset)
                .unwrap()
                .source_point_entry(&analysis)
                .unwrap()
        };
        let entry_a = make_entry(outer_a);
        let entry_b = make_entry(outer_b);
        let source = "source child.tcl";
        let a = Analyser::new()
            .analyse_with_source_point_entry(source, "tcl8.6", &entry_a)
            .unwrap();
        let b = Analyser::new()
            .analyse_with_source_point_entry(source, "tcl8.6", &entry_b)
            .unwrap();
        let load = original_project_source_load_at(source, &a, 0).unwrap();
        assert!(load.source_point_entry(&a).is_some());
        assert!(load.source_point_entry(&b).is_none());
    }
    #[test]
    fn global_source_point_remains_inherited_and_local_publication_is_typed() {
        let parent_source = "source child.tcl";
        let parent = Analyser::new().analyse(parent_source, "tcl8.6");
        let load = original_project_source_load_at(parent_source, &parent, 0).unwrap();
        let point = load.source_point_entry(&parent).unwrap();
        let source = "proc local {} {}; local";
        let child = Analyser::new()
            .analyse_with_source_point_entry(source, "tcl8.6", &point)
            .unwrap();
        let call = child
            .command_invocations
            .iter()
            .find(|invocation| {
                invocation.range.start() == u32::try_from(source.rfind("local").unwrap()).unwrap()
            })
            .unwrap();
        let definition = call
            .resolved_definition
            .as_ref()
            .expect("conditional local declaration reference");
        assert!(matches!(
            definition.allocation().site.source.kind(),
            crate::command_binding::SourceOriginKind::InheritedPoint { .. }
        ));
        assert!(child.proc_for_definition(definition, source).is_some());
        assert!(
            !definition
                .allocation()
                .matches_source_declaration(source, 0)
        );
        let independently_authored = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            independently_authored
                .proc_for_definition(definition, source)
                .is_none()
        );
    }
    #[test]
    fn source_point_decode_options_remain_unavailable_without_an_ingress_owner() {
        for source in [
            "source -encoding utf-8 child.tcl",
            "source -encoding $encoding child.tcl",
            "source -nopkg child.tcl",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl9.0");
            if let Some(load) = original_project_source_load_at(source, &analysis, 0) {
                assert!(load.source_point_entry(&analysis).is_none(), "{source}");
            }
        }
        let (_, parent, load) = parent();
        let entry = load.source_point_entry(&parent).unwrap();
        assert!(
            entry.obligations().contains(
                &crate::command_binding::OriginalSourcePointObligation::FileReadAndDecode
            )
        );
    }
}
