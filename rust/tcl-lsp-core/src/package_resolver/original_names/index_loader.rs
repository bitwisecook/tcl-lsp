// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional autoload script data, independent of future loader execution.

use std::path::{Path, PathBuf};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::registry_invocation::source_structure::{
    original_single_source_command_substitution, source_registry_words_at,
};
use tcl_lexer::{ExecutablePart, NativeWord, SourceImage, Span};
use tcl_registry::{InvocationDialect, InvocationWord, InvocationWords, ResolvedInvocation};

pub(super) struct IndexLoader<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
}

#[derive(Clone, Copy)]
enum WordOrigin {
    Original,
    LoaderValue,
}

impl<'a> IndexLoader<'a> {
    pub(super) fn new(source: &'a str, analysis: &'a AnalysisResult) -> Option<Self> {
        crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let input = analysis.resolved_input.as_ref()?;
        analysis
            .matches_original_source_image(&SourceImage::document(source), input.lexer_config())
            .then_some(Self { source, analysis })
    }

    pub(super) fn filename(&self, loader: &NativeWord, directory: &Path) -> Option<PathBuf> {
        let input = self.analysis.resolved_input.as_ref()?;
        if loader.image().bytes() != self.source.as_bytes()
            || loader.config() != input.lexer_config()
            || loader.group().expand
        {
            return None;
        }
        if let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(loader) {
            let length = u32::try_from(value.len()).ok()?;
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::from_bytes(value, loader.image().channel()),
                Span::new(0, length),
                input.lexer_config().nested(),
            )
            .ok()?;
            let [command] = plan.commands.as_slice() else {
                return None;
            };
            return self.source_filename(&command.words, WordOrigin::LoaderValue, directory);
        }
        let child = original_single_source_command_substitution(loader)?;
        let schema = source_registry_words_at(
            self.source,
            self.analysis,
            child.command().words.first()?.span().start(),
        )?;
        let context = input.context_registry();
        let range = schema.with_source_schema(&context, |selected| {
            selected.authored_source_list_arguments()
        })??;
        let words = range
            .map(|argument| {
                matches!(
                    schema.origins().get(argument.checked_add(1)?),
                    Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(_))
                )
                .then_some(())?;
                schema.original_argument_word(argument).cloned()
            })
            .collect::<Option<Vec<_>>>()?;
        self.source_filename(&words, WordOrigin::Original, directory)
    }

    fn source_filename(
        &self,
        words: &[NativeWord],
        origin: WordOrigin,
        directory: &Path,
    ) -> Option<PathBuf> {
        // These words are data for a possible future invocation. Their schema
        // is conditional catalogue advice, never a reached Source command.
        let argument = self.with_schema(words, WordOrigin::LoaderValue, |selected| {
            let tcl_registry::source_navigation::SourceNavigationOperand::File { argument } =
                selected.authored_source_navigation_operand()?
            else {
                return None;
            };
            let arguments = selected.words.arguments();
            let first = arguments.get(0).and_then(|word| match word {
                InvocationWord::KnownBytes(bytes) => Some(bytes),
                InvocationWord::Literal(value) => Some(value.as_bytes()),
                _ => None,
            });
            let layout = arguments
                .dialect()?
                .source_file_grammar()?
                .select_original(arguments.exact_argv_len()?, first);
            let tcl_registry::source_file::SourceFileSelection::Selected(layout) = layout else {
                return None;
            };
            (layout.path_at == argument).then_some(argument)
        })??;
        self.path(words.get(argument.checked_add(1)?)?, origin, directory)
    }

    fn path(&self, word: &NativeWord, origin: WordOrigin, directory: &Path) -> Option<PathBuf> {
        if let Some(value) = literal(word) {
            return Some(directory.join(std::str::from_utf8(&value).ok()?));
        }
        if let Some(value) = directory_text_hint(word, directory) {
            return Some(PathBuf::from(std::str::from_utf8(&value).ok()?));
        }
        let child = original_single_source_command_substitution(word)?;
        let words = &child.command().words;
        let selected = self.with_schema(words, origin, |selected| {
            selected.authored_source_path_operation()
        })??;
        if selected.operation != tcl_registry::SourcePathOperation::Join {
            return None;
        }
        let mut arguments = selected.arguments;
        let first = words.get(arguments.next()?.checked_add(1)?)?;
        if !directory_reference(first) {
            return None;
        }
        let mut parts = vec![directory.to_str()?.as_bytes().to_vec()];
        for argument in arguments {
            parts.push(literal(words.get(argument.checked_add(1)?)?)?);
        }
        let joined = tcl_cmd_core::path::join(&parts);
        Some(PathBuf::from(std::str::from_utf8(&joined).ok()?))
    }

    fn with_schema<T>(
        &self,
        words: &[NativeWord],
        origin: WordOrigin,
        project: impl FnOnce(&ResolvedInvocation<'_, '_>) -> T,
    ) -> Option<T> {
        let input = self.analysis.resolved_input.as_ref()?;
        let context = input.context_registry();
        if matches!(origin, WordOrigin::Original) {
            let schema = source_registry_words_at(
                self.source,
                self.analysis,
                words.first()?.span().start(),
            )?;
            if schema.origins().len() != words.len()
                || schema
                    .origins()
                    .iter()
                    .enumerate()
                    .skip(1)
                    .any(|(index, origin)| {
                        *origin
                            != tcl_compiler::registry_invocation::InvocationWordOrigin::Written(
                                index,
                            )
                    })
            {
                return None;
            }
            return schema.with_source_schema(&context, project);
        }
        // Parsed immutable loader-value geometry is not a document entry.
        // Resolve only conditional Registry vocabulary under the retained input.
        let values = words.iter().map(literal).collect::<Vec<_>>();
        let values = words
            .iter()
            .zip(&values)
            .map(|(word, value)| {
                if word.group().expand {
                    InvocationWord::Expanded
                } else {
                    value
                        .as_deref()
                        .map_or(InvocationWord::Dynamic, InvocationWord::KnownBytes)
                }
            })
            .collect::<Vec<_>>();
        let (&head, arguments) = values.split_first()?;
        let mut dialect = InvocationDialect::of_profile(input.unit_profile());
        dialect.lexer_grammar = input.lexer_config().grammar_over(dialect.lexer_grammar);
        dialect.word_values =
            tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config());
        let selected =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                InvocationWords::structured(head, arguments).with_dialect(dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()?;
        Some(project(&selected))
    }
}

fn literal(word: &NativeWord) -> Option<Vec<u8>> {
    let value = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
    (!word.group().expand && !value.contains(&0)).then_some(value)
}

fn directory_reference(word: &NativeWord) -> bool {
    let parts = word.executable_parts();
    let mut root = parts.list(parts.root()).iter();
    let Some(part) = root.next() else {
        return false;
    };
    let ExecutablePart::Variable { name, index: None } = part.part else {
        return false;
    };
    root.next().is_none() && word.image().bytes().get(name.as_range()) == Some(b"dir")
}

fn directory_text_hint(word: &NativeWord, directory: &Path) -> Option<Vec<u8>> {
    let directory = directory.to_str()?.as_bytes();
    let mut selected = false;
    let value =
        tcl_syntax::word_rules::original_word_unicode_value_with_variables(word, |arena, part| {
            let ExecutablePart::Variable { name, index: None } = part.part else {
                return None;
            };
            (arena.bytes(name)? == b"dir").then_some(())?;
            selected = true;
            Some(directory.to_vec())
        })?;
    (selected && !value.contains(&0)).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_value_advice_requires_the_original_complete_source_and_input() {
        let source = "set auto_index(p) [list source [file join $dir p.tcl]]";
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let context = tcl_registry::model::ingress::static_context_for_profile(profile);
        let analysis = crate::source_structure::analyse_document(
            source,
            profile,
            context.commands(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        assert!(IndexLoader::new(source, &analysis).is_some());
        assert!(IndexLoader::new(&format!("{source}\n# changed"), &analysis).is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(IndexLoader::new(source, &missing).is_none());
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
        assert!(IndexLoader::new(source, &changed).is_none());
    }
}
