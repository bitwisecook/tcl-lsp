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

//! Complete-word metadata for explicitly selected Logical authoring passes.

use super::{MAX_MINIFY_DEPTH, MinifyEnv};
use tcl_lexer::{NativeScriptCommandWords, NativeWord, SourceImage};
use tcl_registry::ResolvedInvocation;

/// Same complete words and original before-argv Realm, without Native grants.
pub(super) fn with_command_schema<T>(
    env: MinifyEnv<'_>,
    command: &NativeScriptCommandWords,
    project: impl FnOnce(&ResolvedInvocation<'_, '_>) -> T,
) -> Option<T> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let words = tcl_compiler::registry_invocation::source_structure::original_logical_registry_words_for_command(
        env.identities, env.input, command,
    )?;
    // The present lexical editing loops address complete written ordinals.
    // Captured alias prefixes require their own effective-origin mapping.
    if words.origins().iter().enumerate().any(|(ordinal, origin)| {
        origin != &tcl_compiler::registry_invocation::InvocationWordOrigin::Written(ordinal)
    }) {
        return None;
    }
    words.with_source_schema(env.context, project)
}

/// Editing needs its own closed source-transition applicability. Readonly
/// roles remain available when an earlier callback or operation is unknown.
pub(super) fn with_editable_command_schema<T>(
    env: MinifyEnv<'_>,
    command: &NativeScriptCommandWords,
    project: impl FnOnce(&ResolvedInvocation<'_, '_>) -> T,
) -> Option<T> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let words = tcl_compiler::registry_invocation::source_structure::original_logical_registry_words_for_command(
        env.identities, env.input, command,
    )?;
    let tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    if advice.logical_source_input() != Some(env.input)
        || advice.obligations().contains(
            &tcl_compiler::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
        )
        || !advice.uncertain_operations().is_empty()
    {
        return None;
    }
    with_command_schema(env, command, project)
}

/// Tight original name fields from the actual Logical procedure header/body.
/// The caller's rename barriers, reference coverage and collision checks retain
/// the transformation purpose; these readonly rows grant geometry only.
pub(super) fn parameter_name_sources(
    source: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    procedure: &tcl_compiler::analyser::ProcDef,
) -> Option<Vec<tcl_compiler::signature_scan::formal_parameters::OriginalSourceFormalName>> {
    let input = analysis.resolved_input.as_ref()?;
    let config = analysis.body_lexer_config?;
    let image = SourceImage::document(source);
    let registry = analysis.resolved_registry()?;
    let realm = analysis.retained_command_realm()?;
    if !analysis.allows_lexical_declaration_advice()
        || config != input.lexer_config()
        || config != tcl_lexer::LexerConfig::for_profile(Some(input.analyser_profile()))
        || !analysis.matches_original_source_image(&image, config)
        || !realm.matches_resolved_analysis_input(input)
        || registry.snapshot().semantic_key()
            != input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
        || procedure.params_computed
    {
        return None;
    }
    // A reporting body span cannot replace the retained declaration/body card.
    analysis
        .logical_source_procedure_declaration(procedure)?
        .original_body_word(&image, input)?;
    let mut declarations = analysis
        .original_procedure_declarations()
        .filter(|declaration| declaration.metadata() == procedure);
    let declaration = declarations.next()?;
    declarations.next().is_none().then_some(())?;
    let names = analysis.original_procedure_formal_name_sources(declaration, registry)?;
    if names.len() != procedure.params.len() {
        return None;
    }
    let mut seen = rustc_hash::FxHashSet::default();
    let mut previous_end = procedure.name_span.end();
    for (name, parameter) in names.iter().zip(&procedure.params) {
        let span = name.source_span()?;
        if name.name() != parameter.name.as_bytes()
            || !seen.insert(name.name())
            || span.is_empty()
            || span.start() < previous_end
            || span.end() > procedure.body_span.start()
            || image.bytes().get(span.as_range()) != Some(name.name())
        {
            return None;
        }
        previous_end = span.end();
    }
    Some(names)
}

pub(super) fn ascii_value(word: &NativeWord) -> Option<String> {
    String::from_utf8(tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?).ok()
}

/// Original selected source commands with independent completeness. Observed
/// candidates are useful on partial input; absence/barrier users need complete.
pub(super) struct CommandCensus<'a> {
    analysis: &'a tcl_compiler::analyser::AnalysisResult,
    coverage: crate::SourceSyntaxCoverage,
    commands: Vec<NativeScriptCommandWords>,
    original_vectors_complete: bool,
}

impl CommandCensus<'_> {
    pub(super) fn observed_commands(&self) -> &[NativeScriptCommandWords] {
        &self.commands
    }

    pub(super) fn is_complete(
        &self,
        source: &str,
        analysis: &tcl_compiler::analyser::AnalysisResult,
    ) -> bool {
        std::ptr::eq(self.analysis, analysis)
            && self.original_vectors_complete
            && self.coverage.applicability(source, analysis)
                == Some(crate::SourceSyntaxCoverageApplicability::Complete)
    }
}

/// Reborrow the shared configured syntax census and each actual Logical whole
/// vector. Missing schema/body/vector ownership remains explicit incomplete
/// coverage; no local body/case/Expr/lambda walker infers missing regions.
pub(super) fn command_census<'a>(source: &str, env: MinifyEnv<'a>) -> Option<CommandCensus<'a>> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let analysis = env.analysis;
    let image = SourceImage::document(source);
    if !analysis.allows_lexical_declaration_advice()
        || source != env.original_source
        || analysis.resolved_input.as_ref() != Some(env.input)
        || analysis.body_lexer_config != Some(env.config)
        || env.config != env.input.lexer_config()
        || !std::ptr::eq(analysis.retained_command_realm()?, env.identities)
        || !std::ptr::eq(env.input.borrowed_context_registry(), env.context)
        || env.registry.snapshot().semantic_key()
            != env.context.commands().snapshot().semantic_key()
        || env.dialect.cache_key() != env.input.unit_profile().cache_key()
    {
        return None;
    }
    let coverage = crate::SourceSyntaxCoverage::capture(source, analysis)?;
    if coverage
        .script_regions(source, analysis)?
        .iter()
        .any(|(_, depth)| MAX_MINIFY_DEPTH.exceeded(*depth))
    {
        return None;
    }
    let selected = coverage.commands(source, analysis)?;
    let mut commands = Vec::with_capacity(selected.len());
    let mut original_vectors_complete = true;
    for command in selected {
        let Some(original) = command.argv.first().and_then(|head| {
            tcl_compiler::registry_invocation::source_structure::original_logical_source_words_at(
                source,
                analysis,
                head.span.start(),
            )
        }) else {
            original_vectors_complete = false;
            continue;
        };
        if original.is_empty()
            || original
                .iter()
                .any(|word| word.config() != env.config || word.image() != &image)
        {
            original_vectors_complete = false;
            continue;
        }
        commands.push(NativeScriptCommandWords {
            span: command.span,
            source_extent: None,
            terminator: None,
            words: original,
        });
    }
    Some(CommandCensus {
        analysis,
        coverage,
        commands,
        original_vectors_complete,
    })
}

/// Positive-only compatibility view used by existing source geometry controls.
/// This wrapper supplies no complete inventory or absence/barrier permission.
#[cfg(test)]
pub(super) fn commands(source: &str, env: MinifyEnv<'_>) -> Option<Vec<NativeScriptCommandWords>> {
    Some(command_census(source, env)?.commands)
}

#[cfg(test)]
mod source_census_tests {
    use super::*;
    use crate::minify::{MinifyEnv, original};
    use tcl_compiler::analyser::{AnalysisResult, ResolvedAnalysisInput};

    fn input() -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        )
    }

    fn with_analysis<T>(source: &str, project: impl FnOnce(MinifyEnv<'_>) -> T) -> T {
        let input = input();
        let analysis = original::analysis_with_input(source, &input);
        let context = input.context_registry();
        project(MinifyEnv {
            original_source: source,
            analysis: &analysis,
            input: &input,
            dialect: input.unit_profile(),
            config: input.lexer_config(),
            context: &context,
            registry: context.commands(),
            identities: analysis.retained_command_realm().unwrap(),
        })
    }

    #[test]
    fn minifier_source_census_owns_selected_full_vectors_and_complete_regions() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        for (source, count) in [
            ("list {string equal -nocase A a}", 1),
            ("if {1} {string equal -nocase A a}", 2),
            ("list [string equal -nocase A a]", 2),
        ] {
            with_analysis(source, |env| {
                let census = command_census(source, env).unwrap();
                assert!(census.is_complete(source, env.analysis), "{source}");
                assert_eq!(census.observed_commands().len(), count, "{source}");
                assert!(census.observed_commands().iter().all(|command| {
                    command.words.iter().all(|word| {
                        word.image().bytes() == source.as_bytes() && word.config() == env.config
                    })
                }));
                assert!(
                    census
                        .coverage
                        .script_regions(source, env.analysis)
                        .is_some()
                );
                assert!(!census.is_complete(&format!("#{source}"), env.analysis));
                assert!(
                    census
                        .coverage
                        .script_regions(&format!("#{source}"), env.analysis)
                        .is_none()
                );
            });
        }
    }

    #[test]
    fn minifier_source_census_retains_partial_candidates_without_barrier_absence() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let source = "string equal -nocase A a; if {1} $body";
        with_analysis(source, |env| {
            let census = command_census(source, env).unwrap();
            assert!(!census.is_complete(source, env.analysis));
            assert!(census.observed_commands().iter().any(|command| {
                command
                    .words
                    .first()
                    .is_some_and(|word| ascii_value(word).as_deref() == Some("string"))
            }));
            let (output, count) = crate::minify::lexical_keywords::abbreviate(source, env);
            assert!(count > 0, "{output}");
            assert!(output.starts_with("string "), "{output}");
            assert!(output.ends_with("if {1} $body"), "{output}");
        });
        let source = "proc p {{longName default}} {return VALUE}; if {1} $body";
        let (output, mapping) = crate::minify::compact_names(source, &input(), false);
        assert_eq!(output, source);
        assert!(mapping.variables.is_empty());
        assert!(mapping.procs.is_empty());
    }

    #[test]
    fn minifier_source_census_keeps_unrepresented_or_unknown_regions_incomplete() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        for source in [
            "mystery",
            "if {1} $body",
            "apply {{} {string equal -nocase A a}}",
            "list {unterminated",
        ] {
            with_analysis(source, |env| {
                let census = command_census(source, env).unwrap();
                assert!(!census.is_complete(source, env.analysis), "{source}");
            });
        }
    }

    #[test]
    fn minifier_source_census_refuses_foreign_analysis_realm_and_full_config() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let source = "string equal -nocase A a";
        with_analysis(source, |env| {
            assert!(
                command_census(source, env)
                    .unwrap()
                    .is_complete(source, env.analysis)
            );
            assert!(command_census("string equal -nocase B b", env).is_none());
            let config = tcl_lexer::LexerConfig {
                leading_bom: match env.config.leading_bom {
                    tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
                    tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
                },
                ..env.config
            };
            assert!(command_census(source, MinifyEnv { config, ..env }).is_none());
            let foreign: AnalysisResult = original::analysis_with_input(source, env.input);
            assert!(
                !command_census(source, env)
                    .unwrap()
                    .is_complete(source, &foreign)
            );
            assert!(
                command_census(
                    source,
                    MinifyEnv {
                        analysis: &foreign,
                        ..env
                    }
                )
                .is_none()
            );
        });
    }
}
