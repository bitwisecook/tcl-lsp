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

pub(in crate::package_resolver) struct IndexLoader<'a> {
    source: &'a str,
    config: tcl_lexer::LexerConfig,
    input: LoaderInput<'a>,
}

enum LoaderInput<'a> {
    Original(&'a AnalysisResult),
    Standalone,
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
            .then_some(Self {
                source,
                config: input.lexer_config(),
                input: LoaderInput::Original(analysis),
            })
    }

    /// Explicit compatibility source data, without an analysis or Native issuer.
    pub(in crate::package_resolver) fn standalone(
        source: &'a str,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        Self {
            source,
            config,
            input: LoaderInput::Standalone,
        }
    }

    pub(in crate::package_resolver) fn filename(
        &self,
        loader: &NativeWord,
        directory: &Path,
    ) -> Option<PathBuf> {
        if loader.image().bytes() != self.source.as_bytes()
            || loader.config() != self.config
            || loader.group().expand
        {
            return None;
        }
        if let Some(value) = self.loader_value(loader, directory) {
            let length = u32::try_from(value.len()).ok()?;
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::from_bytes(value, loader.image().channel()),
                Span::new(0, length),
                self.config.nested(),
            )
            .ok()?;
            let [command] = plan.commands.as_slice() else {
                return None;
            };
            return self.source_filename(&command.words, WordOrigin::LoaderValue, directory);
        }
        let child = original_single_source_command_substitution(loader)?;
        let (words, origin) = match self.input {
            LoaderInput::Original(analysis) => {
                let schema = source_registry_words_at(
                    self.source,
                    analysis,
                    child.command().words.first()?.span().start(),
                )?;
                let input = analysis.resolved_input.as_ref()?;
                let context = input.context_registry();
                let range = schema.with_source_schema(&context, |selected| {
                    selected.authored_source_list_arguments()
                })??;
                let words = range
                    .map(|argument| {
                        matches!(
                            schema.origins().get(argument.checked_add(1)?),
                            Some(
                                tcl_compiler::registry_invocation::InvocationWordOrigin::Written(_)
                            )
                        )
                        .then_some(())?;
                        schema.original_argument_word(argument).cloned()
                    })
                    .collect::<Option<Vec<_>>>()?;
                (words, WordOrigin::Original)
            }
            LoaderInput::Standalone => {
                let words = &child.command().words;
                let range = self.with_schema(words, WordOrigin::LoaderValue, |selected| {
                    selected.authored_source_list_arguments()
                })??;
                let elements = range
                    .map(|argument| words.get(argument.checked_add(1)?).cloned())
                    .collect::<Option<Vec<_>>>()?;
                (elements, WordOrigin::LoaderValue)
            }
        };
        self.source_filename(&words, origin, directory)
    }

    fn loader_value(&self, loader: &NativeWord, directory: &Path) -> Option<Vec<u8>> {
        literal(loader).or_else(|| {
            matches!(self.input, LoaderInput::Standalone)
                .then(|| directory_text_hint(loader, directory))
                .flatten()
        })
    }

    /// Traverse only explicit standalone syntax/data; no entered script follows.
    pub(in crate::package_resolver) fn collect_word(
        &self,
        word: &NativeWord,
        directory: &Path,
        out: &mut Vec<PathBuf>,
        depth: u32,
    ) {
        if !matches!(self.input, LoaderInput::Standalone)
            || word.image().bytes() != self.source.as_bytes()
            || word.config() != self.config
            || super::super::MAX_SOURCE_TARGET_SCAN_DEPTH.exceeded(depth)
            || word.group().expand
        {
            return;
        }
        if word.group().kind != tcl_lexer::WordKind::Bare {
            if let Some(value) = self.loader_value(word, directory) {
                let Ok(length) = u32::try_from(value.len()) else {
                    return;
                };
                self.collect_script(
                    SourceImage::from_bytes(value, word.image().channel()),
                    Span::new(0, length),
                    directory,
                    out,
                    depth + 1,
                );
                return;
            }
        }
        if let Some(file) = self.filename(word, directory) {
            out.push(file);
        }
        for part in word.executable_parts().all_parts() {
            if let ExecutablePart::Command { body } = part.part {
                self.collect_script(word.image().clone(), body, directory, out, depth + 1);
            }
        }
    }

    fn collect_script(
        &self,
        image: SourceImage,
        extent: Span,
        directory: &Path,
        out: &mut Vec<PathBuf>,
        depth: u32,
    ) {
        if super::super::MAX_SOURCE_TARGET_SCAN_DEPTH.exceeded(depth) {
            return;
        }
        let Ok(plan) = tcl_lexer::native_script_words_in(image, extent, self.config.nested())
        else {
            return;
        };
        for command in plan.commands {
            if let Some(file) =
                self.source_filename(&command.words, WordOrigin::LoaderValue, directory)
            {
                out.push(file);
            }
            let source = command
                .words
                .first()
                .and_then(|word| word.image().try_text().ok());
            let Some(source) = source else {
                continue;
            };
            let nested = IndexLoader::standalone(source, self.config.nested());
            for word in &command.words {
                nested.collect_word(word, directory, out, depth);
            }
        }
    }

    fn source_filename(
        &self,
        words: &[NativeWord],
        origin: WordOrigin,
        directory: &Path,
    ) -> Option<PathBuf> {
        // naming.package.original-file-candidate-provenance
        // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
        // These words are data for a possible future invocation. Their schema
        // is conditional catalogue advice, never a reached Source command.
        let argument = self.with_schema(words, WordOrigin::LoaderValue, |selected| {
            selected
                .semantics
                .arity
                .accepts(selected.argument_count_for_arity()?)
                .then_some(())?;
            let argument = selected.authored_source_file_argument()?;
            let arguments = selected.words.arguments();
            let first = arguments.get(0).and_then(|word| match word {
                InvocationWord::KnownBytes(bytes) => Some(bytes),
                InvocationWord::Literal(value) => Some(value.as_bytes()),
                _ => None,
            });
            let grammar = match self.input {
                LoaderInput::Original(_) => arguments.dialect()?.source_file_grammar()?,
                // This API's explicit compatibility grammar retains encoding
                // syntax; it does not select an actual runtime release.
                LoaderInput::Standalone => tcl_registry::source_file::SourceFileGrammar::Encoding,
            };
            let layout = grammar.select_original(arguments.exact_argv_len()?, first);
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
        let context = match self.input {
            LoaderInput::Original(analysis) => analysis
                .resolved_input
                .as_ref()?
                .borrowed_context_registry(),
            LoaderInput::Standalone => crate::context_for_dialect("tcl"),
        };
        if matches!(origin, WordOrigin::Original) {
            let LoaderInput::Original(analysis) = self.input else {
                return None;
            };
            let schema =
                source_registry_words_at(self.source, analysis, words.first()?.span().start())?;
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
            return schema.with_source_schema(context, project);
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
                    // These are decoded Unicode source values, rather than
                    // independently produced native byte operands. Conditional
                    // head/member/option selection needs their literal facet.
                    value
                        .as_deref()
                        .and_then(|bytes| std::str::from_utf8(bytes).ok())
                        .map_or(InvocationWord::Dynamic, InvocationWord::Literal)
                }
            })
            .collect::<Vec<_>>();
        let (&head, arguments) = values.split_first()?;
        let profile = match self.input {
            LoaderInput::Original(analysis) => analysis.resolved_input.as_ref()?.unit_profile(),
            LoaderInput::Standalone => tcl_dialect::DialectProfile::plain_tcl(),
        };
        let mut dialect = InvocationDialect::of_profile(profile);
        dialect.lexer_grammar = self.config.grammar_over(dialect.lexer_grammar);
        dialect.word_values = tcl_syntax::word_rules::WordValueRules::from_config(&self.config);
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

    #[test]
    fn standalone_filename_geometry_requires_actual_source_and_full_configuration() {
        let source = "set loader [list source [file join $dir p.tcl]]";
        let config = tcl_lexer::LexerConfig::default();
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = &plan.commands[0].words[2];
        let changed_source = format!("{source} # changed");
        let mut candidates = Vec::new();
        IndexLoader::standalone(source, config).collect_word(
            word,
            Path::new("/library"),
            &mut candidates,
            0,
        );
        assert_eq!(candidates, vec![Path::new("/library/p.tcl").to_path_buf()]);
        candidates.clear();
        IndexLoader::standalone(&changed_source, config).collect_word(
            word,
            Path::new("/library"),
            &mut candidates,
            0,
        );
        assert!(candidates.is_empty());
        let changed_config = tcl_lexer::LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        };
        IndexLoader::standalone(source, changed_config).collect_word(
            word,
            Path::new("/library"),
            &mut candidates,
            0,
        );
        assert!(candidates.is_empty());
    }

    #[test]
    fn loader_value_schemas_keep_unicode_literals_and_unknown_operand_kinds() {
        let source = r"sour\u0063e [file join $dir { café.tcl }]";
        let config = tcl_lexer::LexerConfig::default();
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let loader = IndexLoader::standalone(source, config);
        let words = &plan.commands[0].words;
        assert_eq!(
            loader.with_schema(words, WordOrigin::LoaderValue, |selected| {
                (
                    selected.words.head_literal() == Some("source"),
                    selected.authored_source_navigation_operand(),
                )
            }),
            Some((
                true,
                Some(
                    tcl_registry::source_navigation::SourceNavigationOperand::File { argument: 0 }
                ),
            )),
        );
        let child = original_single_source_command_substitution(&words[1]).unwrap();
        assert_eq!(
            loader.with_schema(
                &child.command().words,
                WordOrigin::LoaderValue,
                |selected| {
                    (
                        selected.words.head_literal() == Some("file"),
                        selected.words.arguments().get(1) == Some(InvocationWord::Dynamic),
                        selected.words.arguments().literal_at(2) == Some(" café.tcl "),
                        selected.authored_source_path_operation(),
                    )
                }
            ),
            Some((
                true,
                true,
                true,
                Some(tcl_registry::source_path::SourcePathInvocation {
                    operation: tcl_registry::SourcePathOperation::Join,
                    arguments: 1..3,
                }),
            )),
        );
        assert_eq!(
            loader.source_filename(words, WordOrigin::LoaderValue, Path::new("/library")),
            Some(Path::new("/library/ café.tcl ").to_path_buf()),
        );
        for source in ["$source path.tcl", "source {*}$paths", "source $unknown"] {
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            assert!(
                IndexLoader::standalone(source, config)
                    .source_filename(
                        &plan.commands[0].words,
                        WordOrigin::LoaderValue,
                        Path::new("/library"),
                    )
                    .is_none(),
                "{source}",
            );
        }
    }
}

#[cfg(test)]
mod selected_source_form_tests {
    use super::*;

    #[test]
    fn index_filename_uses_selected_source_form_before_computed_path_value() {
        // naming.package.original-file-candidate-provenance
        // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
        let directory = Path::new("/library");
        let source = "source -encoding utf-8 [file join $dir {-literal path.tcl}]";
        let config = tcl_lexer::LexerConfig::default();
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let words = &plan.commands[0].words;
        let loader = IndexLoader::standalone(source, config);
        assert_eq!(
            loader.with_schema(words, WordOrigin::LoaderValue, |selected| {
                (
                    selected.authored_source_file_argument(),
                    selected.words.arguments().get(2) == Some(InvocationWord::Dynamic),
                )
            }),
            Some((Some(2), true))
        );
        assert_eq!(
            loader.source_filename(words, WordOrigin::LoaderValue, directory),
            Some(directory.join("-literal path.tcl"))
        );
        // Authored source metadata is not an accepted native handler. Keep
        // the original arity refusal even for a SourceFile-tagged descriptor.
        let custom_source = "load-file p.tcl";
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "load-file",
            arity: tcl_registry::Arity::exact(0),
            traits: tcl_registry::Traits::SOURCES_FILE,
            body_execution: Some(tcl_registry::body_execution::BodyExecutionSpec::SourceFile),
            analyser_hook: Some(tcl_registry::hooks::AnalyserHookId::Source),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let profile = crate::profile_for_dialect("tcl8.6");
        let analysis =
            crate::source_structure::analyse_document(custom_source, profile, &registry, config);
        let custom_loader = IndexLoader::new(custom_source, &analysis).unwrap();
        let custom_plan = tcl_lexer::native_script_words_in(
            SourceImage::document(custom_source),
            Span::new(0, u32::try_from(custom_source.len()).unwrap()),
            config,
        )
        .unwrap();
        let custom_words = &custom_plan.commands[0].words;
        assert_eq!(
            custom_loader.with_schema(custom_words, WordOrigin::LoaderValue, |selected| {
                selected
                    .argument_count_for_arity()
                    .map(|count| selected.semantics.arity.accepts(count))
            }),
            Some(Some(false))
        );
        assert!(
            custom_loader
                .source_filename(custom_words, WordOrigin::LoaderValue, directory)
                .is_none()
        );
        for source in [
            "source -enc utf-8 [file join $dir p.tcl]",
            "source $option utf-8 [file join $dir p.tcl]",
            "source -nopkg [file join $dir p.tcl]",
            "source -encoding utf-8 {*}$paths",
            "puts -encoding utf-8 [file join $dir p.tcl]",
        ] {
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            assert!(
                IndexLoader::standalone(source, config)
                    .source_filename(&plan.commands[0].words, WordOrigin::LoaderValue, directory)
                    .is_none(),
                "{source}"
            );
        }
    }
}
