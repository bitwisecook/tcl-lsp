// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Package file advice retains actual source syntax separately from loader facts.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::registry_invocation::InvocationWordOrigin;
use tcl_compiler::registry_invocation::source_structure::{
    OriginalRegistryWords, source_registry_words,
};
use tcl_lexer::{ExecutablePart, ExecutableText, LexerConfig, NativeWord, SourceImage};
use tcl_registry::model::ContextRegistry;

/// Why a package index suggests a file. Every variant is advisory; none proves
/// future command lookup, loader execution, filesystem success or installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalPackageFileCandidateKind {
    /// Exact static filename at the shared authored Source operand ordinal.
    /// The retained source schema keeps conditional applicability explicit.
    SourceOperand,
    /// Conventional `$dir/path` source spelling at that genuine ordinal.
    /// The text does not prove what this variable will contain when loaded.
    DeferredTextHint,
    /// A caller-supplied directory entry, without a source operand receipt.
    DirectoryHint,
}

/// A sealed source-only file suggestion from one original package loader word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalPackageFileCandidate {
    path: PathBuf,
    kind: OriginalPackageFileCandidateKind,
    loader: NativeWord,
    operand: Option<NativeWord>,
    schema: Option<OriginalRegistryWords>,
}

impl OriginalPackageFileCandidate {
    /// Candidate path only; no source or load operation is claimed.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Separate original source, deferred spelling and directory provenance.
    #[must_use]
    pub const fn kind(&self) -> OriginalPackageFileCandidateKind {
        self.kind
    }

    /// Whole genuine word registered as the deferred package loader.
    #[must_use]
    pub const fn loader_word(&self) -> &NativeWord {
        &self.loader
    }

    /// Whole actual filename operand, absent for directory-only advice.
    #[must_use]
    pub fn original_operand(&self) -> Option<&NativeWord> {
        self.operand.as_ref()
    }

    /// Same shared original source schema and all conditional obligations.
    /// A directory candidate has no schema and cannot acquire source geometry.
    #[must_use]
    pub fn source_schema(&self) -> Option<&OriginalRegistryWords> {
        self.schema.as_ref()
    }

    /// Complete original image/channel and semantic lexer configuration.
    /// This checks source currency only, without any load or execution claim.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.loader.image() == image
            && self.loader.config() == config
            && self
                .operand
                .as_ref()
                .is_none_or(|word| word.image() == image && word.config() == config)
            && self
                .schema
                .as_ref()
                .is_none_or(|schema| schema.matches_source(image, config))
    }
}

pub(super) struct FileCandidateScanner<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    context: Arc<ContextRegistry>,
    structure: crate::source_structure::SourceStructure,
}

impl<'a> FileCandidateScanner<'a> {
    pub(super) fn new(source: &'a str, analysis: &'a AnalysisResult) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let image = SourceImage::document(source);
        if !analysis.matches_original_source_image(&image, config) {
            return None;
        }
        let context = analysis.resolved_input.as_ref()?.context_registry();
        if context.commands().snapshot().semantic_key()
            != analysis.resolved_registry()?.snapshot().semantic_key()
        {
            return None;
        }
        Some(Self {
            source,
            analysis,
            context,
            structure: crate::source_structure::SourceStructure::capture(
                source,
                Some(analysis),
                config,
            )?,
        })
    }

    pub(super) fn collect(
        &self,
        original: &[NativeWord],
        loader: &NativeWord,
        directory: &Path,
        exists: &dyn Fn(&Path) -> bool,
        list_files: &dyn Fn(&Path) -> Vec<PathBuf>,
    ) -> Vec<OriginalPackageFileCandidate> {
        // Implementation contract: naming.package.original-file-candidate-provenance
        // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
        let image = SourceImage::document(self.source);
        let Some(config) = self.analysis.body_lexer_config else {
            return Vec::new();
        };
        if loader.image() != &image
            || loader.config() != config
            || original
                .iter()
                .any(|word| word.image() != &image || word.config() != config)
            || !original.contains(loader)
        {
            return Vec::new();
        }
        let Some(head) = original.first() else {
            return Vec::new();
        };
        let regions = self
            .structure
            .commands
            .iter()
            .find(|command| command.span.start() == head.span().start())
            .and_then(|command| source_registry_words(self.source, self.analysis, command))
            .map(|words| {
                words
                    .source_script_bodies_for(
                        &self.context,
                        tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
                    )
                    .into_iter()
                    .filter(|body| {
                        body.original_container() == loader
                            && body.matches_source(&image, config)
                            && body.matches_context(&self.context)
                    })
                    .map(|body| body.content_span())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut files = Vec::new();
        for command in &self.structure.commands {
            if !regions.iter().any(|region| {
                region.start() <= command.span.start() && command.span.end() <= region.end()
            }) {
                continue;
            }
            let Some(schema) = source_registry_words(self.source, self.analysis, command) else {
                continue;
            };
            let source = schema
                .with_source_schema(&self.context, |selected| {
                    selected.semantics.analyser_hook
                        == Some(tcl_registry::hooks::AnalyserHookId::Source)
                        && selected
                            .words
                            .arguments()
                            .exact_argv_len()
                            .is_some_and(|count| {
                                u16::try_from(
                                    count.saturating_sub(selected.semantics.argument_offset),
                                )
                                .is_ok_and(|count| selected.semantics.arity.accepts(count))
                            })
                })
                .unwrap_or(false);
            if !source {
                continue;
            }
            let Some(grammar) = schema
                .dialect()
                .and_then(|dialect| dialect.source_file_grammar())
            else {
                continue;
            };
            let tcl_registry::source_file::SourceFileSelection::Selected(layout) = grammar
                .select_original(
                    schema.arguments().len(),
                    schema
                        .arguments()
                        .first()
                        .and_then(|word| word.literal_bytes()),
                )
            else {
                continue;
            };
            let argument = layout.path_at;
            // No captured alias prefix or expansion child can become a filename word.
            if !matches!(
                schema.origins().get(argument + 1),
                Some(InvocationWordOrigin::Written(_))
            ) {
                continue;
            }
            let Some(operand) = schema.operands().get(argument).and_then(Option::as_ref) else {
                continue;
            };
            let Some(word) = operand.word().filter(|word| {
                word.image() == &image && word.config() == config && !word.group().expand
            }) else {
                continue;
            };
            let filename = schema
                .arguments()
                .get(argument)
                .and_then(|argument| argument.literal_bytes())
                .filter(|bytes| !bytes.contains(&0))
                .and_then(|bytes| {
                    let input = operand.input()?;
                    (input.bytes() == bytes && input.original_word_key()?.original_word() == word)
                        .then_some(bytes)
                })
                .and_then(|bytes| std::str::from_utf8(bytes).ok());
            let (path, kind) = if let Some(filename) = filename {
                (
                    directory.join(filename),
                    OriginalPackageFileCandidateKind::SourceOperand,
                )
            } else if let Some(tail) = directory_text_hint(word) {
                (
                    directory.join(tail),
                    OriginalPackageFileCandidateKind::DeferredTextHint,
                )
            } else {
                continue;
            };
            if exists(&path)
                && !files
                    .iter()
                    .any(|file: &OriginalPackageFileCandidate| file.path == path)
            {
                files.push(OriginalPackageFileCandidate {
                    path,
                    kind,
                    loader: loader.clone(),
                    operand: Some(word.clone()),
                    schema: Some(schema),
                });
            }
        }
        if files.is_empty() {
            for path in list_files(directory) {
                if path.file_name().is_some_and(|name| name != "pkgIndex.tcl")
                    && !files.iter().any(|file| file.path == path)
                {
                    files.push(OriginalPackageFileCandidate {
                        path,
                        kind: OriginalPackageFileCandidateKind::DirectoryHint,
                        loader: loader.clone(),
                        operand: None,
                        schema: None,
                    });
                }
            }
        }
        files
    }
}

/// Preserve a conventional directory-relative spelling as text advice only.
/// The shared word scanner, rather than a parser over stripped String text,
/// owns the complete scalar reference and literal suffix. No `$dir` value or
/// command lookup is supplied by this convention.
fn directory_text_hint(word: &NativeWord) -> Option<&str> {
    let parts = word.executable_parts().all_parts().collect::<Vec<_>>();
    let [variable, suffix] = parts.as_slice() else {
        return None;
    };
    let ExecutablePart::Variable { name, index: None } = variable.part else {
        return None;
    };
    if word.image().bytes().get(name.as_range()) != Some(b"dir")
        || !matches!(suffix.part, ExecutablePart::Text(ExecutableText::Original))
    {
        return None;
    }
    let bytes = word
        .image()
        .bytes()
        .get(suffix.span.as_range())?
        .strip_prefix(b"/")?;
    if bytes.is_empty() || bytes.contains(&0) {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;

    fn analysis(source: &str, version: TclVersion) -> AnalysisResult {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap();
        let registry = tcl_registry::model::ingress::static_context_for(version.dialect_name());
        crate::source_structure::analyse_document(
            source,
            profile,
            registry.commands(),
            LexerConfig::for_file_grammar(profile.grammar),
        )
    }

    #[test]
    fn original_package_files_keep_source_ordinals_data_and_directory_hints_distinct() {
        // Implementation contract: naming.package.original-file-candidate-provenance
        // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
        for version in TclVersion::ALL {
            let source = "package ifneeded P 1.0 {source actual.tcl; set data {source inert.tcl}; puts source}\n";
            let entries = super::super::original_package_entries(
                source,
                Path::new("/library"),
                Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
                version,
            );
            let entry = entries.first().expect("original package declaration");
            assert_eq!(
                entry
                    .file_candidates()
                    .iter()
                    .map(|file| file.path())
                    .collect::<Vec<_>>(),
                [Path::new("/library/actual.tcl")],
                "{version:?}"
            );
            let file = &entry.file_candidates()[0];
            assert_eq!(file.kind(), OriginalPackageFileCandidateKind::SourceOperand);
            assert!(file.source_schema().is_some());
            assert!(file.original_operand().is_some());
            assert!(file.matches_source(
                &SourceImage::document(source),
                analysis(source, version).body_lexer_config.unwrap()
            ));

            let source = "package ifneeded P 1.0 {set data {source inert.tcl}; puts source}";
            let entries = super::super::original_package_entries(
                source,
                Path::new("/library"),
                Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| {
                    vec![
                        PathBuf::from("/library/pkgIndex.tcl"),
                        PathBuf::from("/library/advice.tcl"),
                    ]
                },
                version,
            );
            let files = entries[0].file_candidates();
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].path(), Path::new("/library/advice.tcl"));
            assert_eq!(
                files[0].kind(),
                OriginalPackageFileCandidateKind::DirectoryHint
            );
            assert!(files[0].source_schema().is_none() && files[0].original_operand().is_none());
        }
    }

    #[test]
    fn original_package_file_scanner_keeps_the_full_input_generation() {
        // naming.package.original-file-candidate-provenance
        // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
        let source = "package ifneeded P 1.0 {source actual.tcl}";
        let retained = analysis(source, TclVersion::V9_0);
        assert!(FileCandidateScanner::new(source, &retained).is_some());
        assert!(FileCandidateScanner::new(source, &retained.clone()).is_some());
        let original = retained.resolved_input.as_ref().unwrap();
        let generation = original.context_registry();
        let mut foreign = retained.clone();
        foreign.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            original.analyser_profile(),
            original.unit_profile(),
            Arc::new(
                generation.with_command_store(generation.commands().snapshot().shared_registry()),
            ),
            original.lexer_config(),
        ));
        assert!(FileCandidateScanner::new(source, &foreign).is_none());
    }

    #[test]
    fn original_package_file_currency_keeps_file_ingress_and_nested_bom_distinct() {
        // naming.package.original-file-candidate-provenance
        // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
        let source = "package ifneeded P 1.0 {source actual.tcl}";
        for version in TclVersion::ALL {
            let profile = tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap();
            let config = LexerConfig::for_file_grammar(profile.grammar);
            let entries = super::super::original_package_entries(
                source,
                Path::new("/library"),
                Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
                version,
            );
            let candidate = &entries[0].file_candidates()[0];
            let image = SourceImage::document(source);
            assert_eq!(candidate.loader_word().config(), config);
            assert_eq!(candidate.original_operand().unwrap().config(), config);
            assert!(
                candidate
                    .source_schema()
                    .unwrap()
                    .matches_source(&image, config)
            );
            assert!(candidate.matches_source(&image, config));
            let mut foreign = config;
            foreign.leading_bom = match config.leading_bom {
                tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
                tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
            };
            assert!(!candidate.matches_source(&image, foreign));
            assert!(!candidate.matches_source(
                &SourceImage::document(&source.replace("actual", "other")),
                config,
            ));

            let prefixed = format!("\u{feff}{source}");
            let entries = super::super::original_package_entries(
                &prefixed,
                Path::new("/library"),
                Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
                version,
            );
            assert_eq!(
                entries.len(),
                usize::from(config.leading_bom == tcl_lexer::LeadingBom::Skip)
            );
            if let Some(entry) = entries.first() {
                let candidate = &entry.file_candidates()[0];
                assert_eq!(candidate.path(), Path::new("/library/actual.tcl"));
                assert!(candidate.matches_source(&SourceImage::document(&prefixed), config));
            }

            let nested = "package ifneeded P 1.0 {\u{feff}source inert.tcl}";
            let entries = super::super::original_package_entries(
                nested,
                Path::new("/library"),
                Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| vec![PathBuf::from("/library/advice.tcl")],
                version,
            );
            let candidate = &entries[0].file_candidates()[0];
            assert_eq!(
                candidate.kind(),
                OriginalPackageFileCandidateKind::DirectoryHint
            );
            assert!(candidate.original_operand().is_none() && candidate.source_schema().is_none());
            assert!(candidate.matches_source(&SourceImage::document(nested), config));
        }
    }

    #[test]
    fn original_package_file_advice_keeps_future_values_shadow_arity_and_currency_separate() {
        // Implementation contract: naming.package.original-file-candidate-provenance
        // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
        let version = TclVersion::V8_6;
        let source = "package ifneeded P 1.0 {source $dir/path.tcl}";
        let entries = super::super::original_package_entries(
            source,
            Path::new("/library"),
            Path::new("/library/pkgIndex.tcl"),
            &|_| true,
            &|_| Vec::new(),
            version,
        );
        let candidate = &entries[0].file_candidates()[0];
        assert_eq!(
            candidate.kind(),
            OriginalPackageFileCandidateKind::DeferredTextHint
        );
        assert_eq!(candidate.path(), Path::new("/library/path.tcl"));
        assert!(candidate.source_schema().is_some());
        let retained = analysis(source, version);
        assert!(
            FileCandidateScanner::new(&source.replace("path.tcl", "else.tcl"), &retained).is_none()
        );
        let mut changed = retained.clone();
        let mut config = changed.body_lexer_config.unwrap();
        config.strict_quoting = !config.strict_quoting;
        changed.body_lexer_config = Some(config);
        assert!(FileCandidateScanner::new(source, &changed).is_none());
        assert!(!candidate.matches_source(&SourceImage::document(source), config));

        for source in [
            "proc source args {return custom}\npackage ifneeded P 1.0 {source wrong.tcl}",
            "rename source {}\npackage ifneeded P 1.0 {source wrong.tcl}",
            "package ifneeded P 1.0 {source -invalid wrong.tcl}",
            "package ifneeded P 1.0 {source {*}$words}",
            "package ifneeded P 1.0 [list source [file join $dir produced.tcl]]",
        ] {
            let entries = super::super::original_package_entries(
                source,
                Path::new("/library"),
                Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| vec![PathBuf::from("/library/advice.tcl")],
                version,
            );
            assert!(!entries.is_empty(), "package remains advisory: {source}");
            assert!(
                entries[0]
                    .file_candidates()
                    .iter()
                    .all(|candidate| candidate.kind()
                        == OriginalPackageFileCandidateKind::DirectoryHint),
                "{source}"
            );
        }
    }

    fn reference_only_script(
        _arguments: tcl_registry::InvocationArguments<'_>,
    ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
        vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
    }

    #[test]
    fn original_package_file_candidates_require_potential_loader_evaluation() {
        // naming.source.original-script-region-purpose
        // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
        // naming.package.original-file-candidate-provenance
        // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
        let source = "package {source selected.tcl}";
        for reference_only in [true, false] {
            let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
            let generation = tcl_registry::model::context_for_profile(profile);
            let mut registry = tcl_registry::CommandRegistry::build_default();
            registry.insert(tcl_registry::CommandSpec {
                name: "package",
                arity: tcl_registry::Arity::exact(1),
                arg_roles: &[(0, tcl_registry::ArgRole::Body)],
                arg_role_layout_resolver: None,
                arg_role_count_resolver: None,
                arg_role_resolver: None,
                arg_role_resolver_roles: &[],
                subcommands: &[],
                script_timing_resolver: reference_only.then_some(reference_only_script),
                ..generation
                    .context()
                    .resolve_spec(generation.commands(), "package")
                    .unwrap()
                    .clone()
            });
            let context = Arc::new(generation.with_command_store(Arc::new(registry)));
            let selected = context
                .context()
                .resolve_spec(context.commands(), "package")
                .unwrap();
            assert_eq!(selected.arg_roles, &[(0, tcl_registry::ArgRole::Body)]);
            assert!(selected.subcommands.is_empty());
            assert_eq!(selected.script_timing_resolver.is_some(), reference_only);
            let config = LexerConfig::for_file_grammar(profile.grammar);
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile, profile, context, config,
            );
            let analysis = tcl_compiler::analyser::Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name);
            let original = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap()
            .commands
            .remove(0)
            .words;
            let scanner = FileCandidateScanner::new(source, &analysis).unwrap();
            let candidates = scanner.collect(
                &original,
                &original[1],
                Path::new("/library"),
                &|_| true,
                &|_| Vec::new(),
            );
            assert_eq!(candidates.len(), usize::from(!reference_only));
            if let Some(candidate) = candidates.first() {
                assert_eq!(candidate.path(), Path::new("/library/selected.tcl"));
                assert_eq!(
                    candidate.kind(),
                    OriginalPackageFileCandidateKind::SourceOperand
                );
                assert!(
                    candidate.original_operand().is_some() && candidate.source_schema().is_some()
                );
            }
        }
    }
}
