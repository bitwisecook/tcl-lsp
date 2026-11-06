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

//! Conditional autoload script data, independent of future loader execution.

use std::path::{Path, PathBuf};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::registry_invocation::source_structure::{
    OriginalSourceRegistryContext, OriginalStandaloneListProjection,
    OriginalStandaloneSchemaAbstention, OriginalStandaloneSchemaAbstentions,
};
use tcl_compiler::registry_invocation::source_structure::{
    original_single_source_command_substitution,
    original_single_source_command_substitution_in_plan, source_registry_words_at,
};
use tcl_lexer::{ExecutablePart, NativeWord, SourceImage, Span};
use tcl_registry::{InvocationDialect, InvocationWord, InvocationWords, ResolvedInvocation};

pub(in crate::package_resolver) struct IndexLoader<'a> {
    source: &'a str,
    config: tcl_lexer::LexerConfig,
    input: LoaderInput<'a>,
    abstentions: std::cell::OnceCell<Option<OriginalStandaloneSchemaAbstentions<'a>>>,
    missing_schema: std::cell::RefCell<Option<OriginalStandaloneSchemaAbstention<'a>>>,
}

pub(super) enum IndexFilenameSurvey<'a> {
    Filename(PathBuf),
    NoContribution(Box<IndexFilenameNoContribution<'a>>),
    Unavailable,
}

/// Original filename query, independently of future loader execution.
#[derive(Debug)]
pub enum OriginalIndexFilenameSurvey<'a> {
    /// Filename retained by the existing original source consumer.
    Filename(PathBuf),
    /// The full consumer lacks a required original source schema.
    NoContribution(Box<IndexFilenameNoContribution<'a>>),
    /// The full consumer cannot select the authenticated source-file layout.
    InvalidLayout(Box<OriginalIndexFilenameInvalidLayout<'a>>),
    /// Required original source, schema, grammar or path ownership is missing.
    Unavailable,
}

/// Missing original schema bound to one complete loader and real directory.
#[derive(Debug)]
pub struct IndexFilenameNoContribution<'a> {
    schema: OriginalStandaloneSchemaAbstention<'a>,
    source: &'a str,
    context: &'a OriginalSourceRegistryContext,
    loader: NativeWord,
    directory: PathBuf,
}

impl IndexFilenameNoContribution<'_> {
    /// This exact original loader and directory still own the query.
    #[must_use]
    pub fn matches_current_query(&self, loader: &NativeWord, directory: &Path) -> bool {
        self.loader == *loader
            && self.directory == directory
            && self
                .schema
                .matches_source_context(self.source, self.context)
    }
}

/// Invalid future file-source layout, retaining original list projection and
/// the actual authored grammar. This grants no runtime error or file effect.
#[derive(Debug)]
pub struct OriginalIndexFilenameInvalidLayout<'a> {
    projection: OriginalStandaloneListProjection<'a>,
    layout: tcl_registry::source_file::AuthoredSourceFileLayoutAbstention,
    source: &'a str,
    context: &'a OriginalSourceRegistryContext,
    loader: NativeWord,
    directory: PathBuf,
}

impl OriginalIndexFilenameInvalidLayout<'_> {
    /// Same original loader, directory and grammar under the captured input.
    /// Current document/provider custody remains the caller's responsibility.
    #[must_use]
    pub fn matches_current_query(&self, loader: &NativeWord, directory: &Path) -> bool {
        if self.loader != *loader
            || self.directory != directory
            || !self
                .projection
                .matches_source_context(self.source, self.context)
        {
            return false;
        }
        let Some(child) = original_single_source_command_substitution(loader) else {
            return false;
        };
        if child.command().words.as_slice() != self.projection.original_words() {
            return false;
        }
        IndexLoader::from_source_context(self.source, self.context).and_then(|query| {
            query.with_schema(
                self.projection.elements_if_projected(),
                WordOrigin::LoaderValue,
                |selected| self.layout.matches_invocation(selected),
            )
        }) == Some(true)
    }
}

/// Survey one original filename with independently typed Invalid source-layout
/// evidence. Unknown schema/grammar/path proofs retain Unavailable. Existing
/// package extraction reuses its captured loader and the same proof checks.
#[must_use]
pub fn survey_original_index_filename<'a>(
    source: &'a str,
    context: &'a OriginalSourceRegistryContext,
    loader: &NativeWord,
    directory: &Path,
) -> OriginalIndexFilenameSurvey<'a> {
    let Some(query) = IndexLoader::from_source_context(source, context) else {
        return OriginalIndexFilenameSurvey::Unavailable;
    };
    query.survey_filename_with_layout_proof(loader, directory)
}

enum LoaderInput<'a> {
    Original(&'a AnalysisResult),
    OriginalSource(
        &'a tcl_compiler::registry_invocation::source_structure::OriginalSourceRegistryContext,
    ),
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
                abstentions: std::cell::OnceCell::new(),
                missing_schema: std::cell::RefCell::new(None),
            })
    }

    pub(super) fn from_source_context(
        source: &'a str,
        context: &'a tcl_compiler::registry_invocation::source_structure::OriginalSourceRegistryContext,
    ) -> Option<Self> {
        (context.source_image() == &SourceImage::document(source)).then_some(Self {
            source,
            config: context.editing_input().lexer_config(),
            input: LoaderInput::OriginalSource(context),
            abstentions: std::cell::OnceCell::new(),
            missing_schema: std::cell::RefCell::new(None),
        })
    }

    fn original_input(&self) -> Option<&tcl_compiler::analyser::ResolvedAnalysisInput> {
        match self.input {
            LoaderInput::Original(analysis) => analysis.resolved_input.as_ref(),
            LoaderInput::OriginalSource(source) => Some(source.editing_input()),
            LoaderInput::Standalone => None,
        }
    }

    fn source_words_at(
        &self,
        offset: u32,
    ) -> Option<tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords> {
        match self.input {
            LoaderInput::Original(analysis) => {
                source_registry_words_at(self.source, analysis, offset)
            }
            LoaderInput::OriginalSource(source) => {
                let words = source.registration_words_at(offset);
                if words.is_none()
                    && let Some(survey) = self
                        .abstentions
                        .get_or_init(|| source.standalone_schema_abstentions())
                {
                    self.missing_schema.replace(survey.at(offset));
                }
                words
            }
            LoaderInput::Standalone => None,
        }
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
            abstentions: std::cell::OnceCell::new(),
            missing_schema: std::cell::RefCell::new(None),
        }
    }

    pub(super) fn registration_abstention(
        &self,
        words: &[NativeWord],
    ) -> Option<OriginalStandaloneSchemaAbstention<'a>> {
        let LoaderInput::OriginalSource(source) = self.input else {
            return None;
        };
        let survey = self
            .abstentions
            .get_or_init(|| source.standalone_schema_abstentions())
            .as_ref()?;
        let certificate = survey.at(words.first()?.span().start())?;
        (certificate.matches_source_context(self.source, source)
            && certificate.original_words() == words)
            .then_some(certificate)
    }

    /// The source schema certificate is bound to this exact loader/directory
    /// query. Other missing path/layout/grammar proofs retain the general path.
    pub(super) fn survey_filename(
        &self,
        loader: &NativeWord,
        directory: &Path,
    ) -> IndexFilenameSurvey<'a> {
        self.missing_schema.borrow_mut().take();
        if let Some(file) = self.filename(loader, directory) {
            return IndexFilenameSurvey::Filename(file);
        }
        let Some(schema) = self.missing_schema.borrow_mut().take() else {
            return IndexFilenameSurvey::Unavailable;
        };
        let LoaderInput::OriginalSource(context) = self.input else {
            return IndexFilenameSurvey::Unavailable;
        };
        let Some(first) = schema.original_words().first() else {
            return IndexFilenameSurvey::Unavailable;
        };
        let Some(last) = schema.original_words().last() else {
            return IndexFilenameSurvey::Unavailable;
        };
        if !schema.matches_source_context(self.source, context)
            || first.span().start() <= loader.span().start()
            || last.span().end() > loader.span().end()
        {
            return IndexFilenameSurvey::Unavailable;
        }
        IndexFilenameSurvey::NoContribution(Box::new(IndexFilenameNoContribution {
            schema,
            source: self.source,
            context,
            loader: loader.clone(),
            directory: directory.to_path_buf(),
        }))
    }

    pub(super) fn survey_filename_with_layout_proof(
        &self,
        loader: &NativeWord,
        directory: &Path,
    ) -> OriginalIndexFilenameSurvey<'a> {
        match self.survey_filename(loader, directory) {
            IndexFilenameSurvey::Filename(file) => OriginalIndexFilenameSurvey::Filename(file),
            IndexFilenameSurvey::NoContribution(certificate) => {
                OriginalIndexFilenameSurvey::NoContribution(certificate)
            }
            IndexFilenameSurvey::Unavailable => self
                .invalid_layout_abstention(loader, directory)
                .map_or(OriginalIndexFilenameSurvey::Unavailable, |certificate| {
                    OriginalIndexFilenameSurvey::InvalidLayout(Box::new(certificate))
                }),
        }
    }

    fn invalid_layout_abstention(
        &self,
        loader: &NativeWord,
        directory: &Path,
    ) -> Option<OriginalIndexFilenameInvalidLayout<'a>> {
        let LoaderInput::OriginalSource(context) = self.input else {
            return None;
        };
        if loader.image().bytes() != self.source.as_bytes()
            || loader.image().channel() != tcl_lexer::SourceChannel::Document
            || loader.config() != self.config
            || loader.group().expand
            || self.loader_value(loader, directory).is_some()
        {
            return None;
        }
        let child = original_single_source_command_substitution(loader)?;
        let survey = self
            .abstentions
            .get_or_init(|| context.standalone_schema_abstentions())
            .as_ref()?;
        let projection =
            survey.list_projection_at(child.command().words.first()?.span().start())?;
        if !projection.matches_source_context(self.source, context)
            || projection.original_words() != child.command().words.as_slice()
        {
            return None;
        }
        let layout = self.with_schema(
            projection.elements_if_projected(),
            WordOrigin::LoaderValue,
            authored_invalid_file_layout,
        )??;
        Some(OriginalIndexFilenameInvalidLayout {
            projection,
            layout,
            source: self.source,
            context,
            loader: loader.clone(),
            directory: directory.to_path_buf(),
        })
    }

    pub(in crate::package_resolver) fn filename(
        &self,
        loader: &NativeWord,
        directory: &Path,
    ) -> Option<PathBuf> {
        self.filename_with_plan(loader, directory, None)
    }

    fn filename_with_plan(
        &self,
        loader: &NativeWord,
        directory: &Path,
        checked_parent: Option<&tcl_lexer::NativeScriptWordsPlan>,
    ) -> Option<PathBuf> {
        if loader.image().bytes() != self.source.as_bytes()
            || (!matches!(self.input, LoaderInput::Standalone)
                && loader.image().channel() != tcl_lexer::SourceChannel::Document)
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
        let child = match checked_parent {
            Some(plan) => original_single_source_command_substitution_in_plan(plan, loader)?,
            None => original_single_source_command_substitution(loader)?,
        };
        let (words, origin) = match self.input {
            LoaderInput::Original(_) | LoaderInput::OriginalSource(_) => {
                let schema = self.source_words_at(child.command().words.first()?.span().start())?;
                let input = self.original_input()?;
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
    #[cfg(test)]
    pub(in crate::package_resolver) fn collect_word(
        &self,
        word: &NativeWord,
        directory: &Path,
        out: &mut Vec<PathBuf>,
        depth: u32,
    ) {
        self.collect_word_with_plan(word, None, directory, out, depth);
    }

    /// Same explicit standalone traversal using the genuine parent's recursive
    /// syntax proof for its actual Command parts. Literal/data bodies still
    /// require their independent full parse.
    pub(in crate::package_resolver) fn collect_word_in_plan(
        &self,
        plan: &tcl_lexer::NativeScriptWordsPlan,
        word: &NativeWord,
        directory: &Path,
        out: &mut Vec<PathBuf>,
        depth: u32,
    ) {
        self.collect_word_with_plan(word, Some(plan), directory, out, depth);
    }

    fn collect_word_with_plan(
        &self,
        word: &NativeWord,
        checked_parent: Option<&tcl_lexer::NativeScriptWordsPlan>,
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
        if word.group().kind != tcl_lexer::WordKind::Bare
            && let Some(value) = self.loader_value(word, directory)
        {
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
        if let Some(file) = self.filename_with_plan(word, directory, checked_parent) {
            out.push(file);
        }
        for part in word.executable_parts().all_parts() {
            if let ExecutablePart::Command { body } = part.part {
                if super::super::MAX_SOURCE_TARGET_SCAN_DEPTH.exceeded(depth + 1) {
                    continue;
                }
                if let Some(parent) = checked_parent {
                    if let Ok(child) = parent.reborrow_command_substitution_plan(word, body) {
                        self.collect_plan(&child, directory, out, depth + 1);
                    }
                } else {
                    self.collect_script(word.image().clone(), body, directory, out, depth + 1);
                }
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
        self.collect_plan(&plan, directory, out, depth);
    }

    fn collect_plan(
        &self,
        plan: &tcl_lexer::NativeScriptWordsPlan,
        directory: &Path,
        out: &mut Vec<PathBuf>,
        depth: u32,
    ) {
        if super::super::MAX_SOURCE_TARGET_SCAN_DEPTH.exceeded(depth) {
            return;
        }
        for command in &plan.commands {
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
            let Some(config) = command.words.first().map(NativeWord::config) else {
                continue;
            };
            let nested = IndexLoader::standalone(source, config);
            for word in &command.words {
                nested.collect_word_in_plan(plan, word, directory, out, depth);
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
                LoaderInput::Original(_) | LoaderInput::OriginalSource(_) => {
                    arguments.dialect()?.source_file_grammar()?
                }
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
        let joined = tcl_cmd_core::path::join(&parts, tcl_cmd_core::path::PathReading::Tcl9);
        Some(PathBuf::from(std::str::from_utf8(&joined).ok()?))
    }

    fn with_schema<T>(
        &self,
        words: &[NativeWord],
        origin: WordOrigin,
        project: impl FnOnce(&ResolvedInvocation<'_, '_>) -> T,
    ) -> Option<T> {
        let context = match self.input {
            LoaderInput::Original(_) | LoaderInput::OriginalSource(_) => {
                self.original_input()?.borrowed_context_registry()
            }
            LoaderInput::Standalone => crate::context_for_dialect("tcl"),
        };
        if matches!(origin, WordOrigin::Original) {
            let schema = self.source_words_at(words.first()?.span().start())?;
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
            LoaderInput::Original(_) | LoaderInput::OriginalSource(_) => {
                self.original_input()?.unit_profile()
            }
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

// Keep the invocation borrow independent of its context and word lifetimes.
fn authored_invalid_file_layout(
    invocation: &ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::source_file::AuthoredSourceFileLayoutAbstention> {
    invocation.authored_source_file_layout_abstention()
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
    fn fresh_source_loader_keeps_complete_image_channel_and_configuration() {
        let source = "set auto_index(p) [list source [file join $dir p.tcl]]";
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let generation = crate::context_for_dialect_profile(profile);
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                generation.with_command_store(generation.commands().snapshot().shared_registry()),
            ),
            config,
        );
        let context = tcl_compiler::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(source, input);
        let loader = IndexLoader::from_source_context(source, &context).unwrap();
        assert!(
            IndexLoader::from_source_context(&format!("{source}\n# changed"), &context).is_none()
        );
        let plan = |image, config| {
            tcl_lexer::native_script_words_in(
                image,
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap()
        };
        let actual = plan(SourceImage::document(source), config);
        assert_eq!(
            loader.filename(&actual.commands[0].words[2], Path::new("/library")),
            Some(PathBuf::from("/library/p.tcl"))
        );
        let native = plan(
            SourceImage::from_bytes(source.as_bytes(), tcl_lexer::SourceChannel::NativeValue),
            config,
        );
        assert!(
            loader
                .filename(&native.commands[0].words[2], Path::new("/library"))
                .is_none()
        );
        let changed = plan(
            SourceImage::document(source),
            tcl_lexer::LexerConfig {
                strict_quoting: !config.strict_quoting,
                ..config
            },
        );
        assert!(
            loader
                .filename(&changed.commands[0].words[2], Path::new("/library"))
                .is_none()
        );
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
        assert_eq!(candidates, [] as [std::path::PathBuf; 0]);
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
        assert_eq!(candidates, [] as [std::path::PathBuf; 0]);
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

#[cfg(test)]
mod abstention_tests {
    use super::*;

    fn source_context(
        source: &str,
        version: tcl_dialect::TclVersion,
    ) -> OriginalSourceRegistryContext {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let generation = crate::context_for_dialect_profile(profile);
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                generation.with_command_store(registry.snapshot().shared_registry()),
            ),
            config,
        );
        OriginalSourceRegistryContext::capture_standalone_structure(source, input)
    }

    fn original_loader(source: &str, config: tcl_lexer::LexerConfig, row: usize) -> NativeWord {
        tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap()
        .commands[row]
            .words[2]
            .clone()
    }

    #[test]
    fn reused_root_abstention_keeps_the_complete_current_word_vector() {
        let source = "set auto_index(a) [list source [file join $dir a.tcl]]\nset auto_index(b) [list source [file join $dir b.tcl]]\nset auto_index(c) [list source [file join $dir c.tcl]]\n";
        let context = source_context(source, tcl_dialect::TclVersion::V8_6);
        let config = context.editing_input().lexer_config();
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let query = IndexLoader::from_source_context(source, &context).unwrap();
        let root = &plan.commands[2].words;
        let offset = root[0].span().start();
        assert!(context.registration_words_at(offset).is_none());
        let full = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(context.editing_input().clone())
            .structure_only()
            .analyse(source, "tcl8.6");
        assert!(source_registry_words_at(source, &full, offset).is_none());
        for root in &plan.commands[..2] {
            let offset = root.words[0].span().start();
            assert!(context.registration_words_at(offset).is_some());
            assert!(source_registry_words_at(source, &full, offset).is_some());
            assert!(query.registration_abstention(&root.words).is_none());
        }
        for _ in 0..2 {
            let receipt = query
                .registration_abstention(root)
                .expect("same immutable missing-root prerequisite");
            assert_eq!(receipt.original_words(), root.as_slice());
            assert!(receipt.matches_source_context(source, &context));
        }
        let mut changed = root.clone();
        changed[2] = plan.commands[0].words[2].clone();
        assert_eq!(changed[0], root[0]);
        assert!(query.registration_abstention(&changed).is_none());
        assert!(query.registration_abstention(&root[..2]).is_none());
        let altered = format!("{source}# changed");
        let mut changed_config = config;
        changed_config.strict_quoting = !changed_config.strict_quoting;
        for (image, config) in [
            (SourceImage::document(&altered), config),
            (SourceImage::document(source), changed_config),
            (SourceImage::native(source.as_bytes()), config),
        ] {
            let changed = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                config,
            )
            .unwrap();
            assert!(
                query
                    .registration_abstention(&changed.commands[2].words)
                    .is_none()
            );
        }
        assert!(query.registration_abstention(root).is_some());
    }

    #[test]
    fn invalid_layout_receipt_keeps_original_loader_and_directory() {
        let source = "set auto_index(a) [list source -encoding utf-8 [file join $dir a.tcl]]\nset auto_index(b) [list source -encoding utf-8 [file join $dir b.tcl]]\n";
        let context = source_context(source, tcl_dialect::TclVersion::V8_4);
        let config = context.editing_input().lexer_config();
        let first = original_loader(source, config, 0);
        let second = original_loader(source, config, 1);
        let directory = Path::new("/library");
        let OriginalIndexFilenameSurvey::InvalidLayout(receipt) =
            survey_original_index_filename(source, &context, &first, directory)
        else {
            panic!("authenticated invalid file-layout prerequisite")
        };
        assert_eq!(
            receipt.layout.grammar(),
            tcl_registry::source_file::SourceFileGrammar::Filename
        );
        assert!(receipt.matches_current_query(&first, directory));
        assert!(!receipt.matches_current_query(&second, directory));
        assert!(!receipt.matches_current_query(&first, Path::new("/other")));
        let native = tcl_lexer::native_script_words_in(
            SourceImage::from_bytes(source.as_bytes(), tcl_lexer::SourceChannel::NativeValue),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        assert!(!receipt.matches_current_query(&native.commands[0].words[2], directory));
        let changed_config = tcl_lexer::LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        };
        let changed_word = original_loader(source, changed_config, 0);
        assert!(!receipt.matches_current_query(&changed_word, directory));
        assert!(matches!(
            survey_original_index_filename(source, &context, &changed_word, directory),
            OriginalIndexFilenameSurvey::Unavailable
        ));
        let general =
            OriginalSourceRegistryContext::capture(source, context.editing_input().clone());
        assert!(matches!(
            survey_original_index_filename(source, &general, &first, directory),
            OriginalIndexFilenameSurvey::Unavailable
        ));
        let changed = format!("{source}# changed");
        assert!(matches!(
            survey_original_index_filename(&changed, &context, &first, directory),
            OriginalIndexFilenameSurvey::Unavailable
        ));
    }

    #[test]
    fn invalid_layout_survey_preserves_unknown_path_and_constructor_refusals() {
        for source in [
            "set auto_index(a) [list source $missing]",
            "set auto_index(a) [list source [file dirname $dir]]",
            "set auto_index(a) {source -encoding utf-8 a.tcl}",
            "set auto_index(a) [list source -encoding utf-8 a.tcl; list]",
            "set auto_index(a) \"prefix[list source -encoding utf-8 a.tcl]\"",
            "unavailable; set auto_index(a) [list source -encoding utf-8 a.tcl]",
            "rename list saved; set auto_index(a) [list source -encoding utf-8 a.tcl]",
            "interp alias {} make {} list; set auto_index(a) [make source -encoding utf-8 a.tcl]",
        ] {
            let context = source_context(source, tcl_dialect::TclVersion::V8_4);
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                context.editing_input().lexer_config(),
            )
            .unwrap();
            let word = plan.commands.last().unwrap().words[2].clone();
            assert!(
                matches!(
                    survey_original_index_filename(source, &context, &word, Path::new("/library")),
                    OriginalIndexFilenameSurvey::Unavailable
                ),
                "{source}"
            );
        }
        for source in [
            "set auto_index(a) [list source $option utf-8 a.tcl]",
            "set auto_index(a) [list source -encoding utf-8 $missing]",
            "set auto_index(a) [list source {*}$unknown]",
        ] {
            let context = source_context(source, tcl_dialect::TclVersion::V8_5);
            let word = original_loader(source, context.editing_input().lexer_config(), 0);
            assert!(
                matches!(
                    survey_original_index_filename(source, &context, &word, Path::new("/library")),
                    OriginalIndexFilenameSurvey::Unavailable
                ),
                "{source}"
            );
        }
    }

    #[test]
    fn source_filename_abstention_keeps_exact_query_and_directory() {
        let source = "set auto_index(a) [list source [file join $dir a.tcl]]\nset auto_index(b) [list source [file join $dir b.tcl]]\n";
        for version in tcl_dialect::TclVersion::ALL {
            let profile = tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap();
            let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
            let generation = crate::context_for_dialect_profile(profile);
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(
                    generation
                        .with_command_store(generation.commands().snapshot().shared_registry()),
                ),
                config,
            );
            let context =
                OriginalSourceRegistryContext::capture_standalone_structure(source, input);
            let loader = IndexLoader::from_source_context(source, &context).unwrap();
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let directory = Path::new("/library");
            let first = &plan.commands[0].words[2];
            let second = &plan.commands[1].words[2];
            assert!(
                matches!(
                    loader.survey_filename(first, directory),
                    IndexFilenameSurvey::Filename(_)
                ),
                "{version:?}"
            );
            let IndexFilenameSurvey::NoContribution(certificate) =
                loader.survey_filename(second, directory)
            else {
                panic!("exact source abstention prerequisite: {version:?}")
            };
            assert!(certificate.matches_current_query(second, directory));
            assert!(!certificate.matches_current_query(first, directory));
            assert!(!certificate.matches_current_query(second, Path::new("/other")));
            assert!(
                matches!(
                    loader.survey_filename(&plan.commands[1].words[1], directory),
                    IndexFilenameSurvey::Unavailable
                ),
                "a failed literal loader query cannot inherit a prior certificate"
            );
            assert!(
                matches!(
                    loader.survey_filename(first, directory),
                    IndexFilenameSurvey::Filename(_)
                ),
                "an earlier abstention cannot suppress a valid loader"
            );
            let native = tcl_lexer::native_script_words_in(
                SourceImage::from_bytes(source.as_bytes(), tcl_lexer::SourceChannel::NativeValue),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            assert!(
                matches!(
                    loader.survey_filename(&native.commands[1].words[2], directory),
                    IndexFilenameSurvey::Unavailable
                ),
                "a previous query cannot donate its certificate to another channel"
            );
        }
    }
}
