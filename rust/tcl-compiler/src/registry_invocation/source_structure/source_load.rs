// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional current-interpreter source paths from actual original schemas.

use super::{OriginalRegistryWords, source_registry_words_at};
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};

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
    let input = analysis.resolved_input.as_ref()?;
    let image = tcl_lexer::SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, input.lexer_config()) {
        return None;
    }
    let words = source_registry_words_at(source, analysis, offset)?;
    let context = input.context_registry();
    let path_at = words.with_source_schema(&context, |schema| {
        if schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current {
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
        let first = paths.next()?;
        paths.all(|path| path == first).then_some(first)
    })??;
    let load = OriginalProjectSourceLoad {
        input: input.clone(),
        words,
        path_at,
    };
    load.matches_source(&image, input).then_some(load)
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
