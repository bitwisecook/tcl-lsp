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

//! Package references in original source and where a `package require` may be inserted.

use super::source_registry_words::source_registry_words;
use super::{OriginalRegistryWords, OriginalSourcePackageReference};
use crate::analyser::AnalysisResult;

fn selected_source_package_reference(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::source_navigation::SourcePackageReference> {
    schema.authored_source_package_reference()
}

/// Current root insertion geometry shared by missing-package diagnostics and
/// reviewed package suggestions. Only selected leading root requirements are
/// header commands; nested scripts, data and prefix-like text cannot supply
/// this position. The full original input and availability remain owners.
#[must_use]
pub fn original_package_require_insert_offset(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<u32> {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    use tcl_registry::source_navigation::SourcePackageReferenceKind as Kind;
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let source_end = u32::try_from(source.len()).ok()?;
    let original_commands = tcl_lexer::native_script_words_in(
        image.clone(),
        tcl_lexer::Span::new(0, source_end),
        config,
    )
    .ok()?;
    let mut end =
        if config.leading_bom == tcl_lexer::LeadingBom::Skip && source.starts_with('\u{feff}') {
            3
        } else {
            0
        };
    if source.get(end..)?.starts_with("#!") {
        end = source
            .get(end..)?
            .find('\n')
            .map_or(source.len(), |newline| end + newline + 1);
    }
    for (command_index, segment) in
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .into_iter()
            .enumerate()
    {
        let Some(words) = source_registry_words(source, analysis, &segment) else {
            break;
        };
        let head = words.head_source()?.word()?;
        let start = usize::try_from(head.span().start()).ok()?;
        let original = original_commands.commands.get(command_index)?;
        let source_extent = original.source_extent?;
        if original.span.start() != segment.span.start()
            || original.span.end() < segment.execution_span(source).end()
            || source_extent.start() != segment.span.start()
        {
            return None;
        }
        let insertion_end = match original.terminator {
            Some(terminator) if terminator.start() == source_extent.end() => terminator.end(),
            None if source_extent.end() == source_end => source_extent.end(),
            Some(_) | None => return None,
        };
        if start < end {
            continue;
        }
        if !source.get(end..start)?.trim().is_empty()
            || words
                .package_reference(&context)
                .is_none_or(|reference| reference.kind() != Kind::Require)
        {
            break;
        }
        end = usize::try_from(insertion_end).ok()?;
    }
    u32::try_from(end).ok()
}

impl OriginalSourcePackageReference {
    /// Same selected whole invocation, including its conditional obligations.
    #[must_use]
    pub fn words(&self) -> &OriginalRegistryWords {
        &self.words
    }
    /// Requirement or actual version-supplying source provision.
    #[must_use]
    pub const fn kind(&self) -> tcl_registry::source_navigation::SourcePackageReferenceKind {
        self.reference.kind
    }
    /// Original effective name ordinal; captured arguments gain no call anchor.
    #[must_use]
    pub const fn argument(&self) -> usize {
        self.reference.argument
    }
    /// Compare one explicitly intended Unicode source value with the same
    /// genuine written package word under its complete source configuration.
    /// This purpose does not compare Native package keys or imply installation.
    /// Missing word/configuration/Unicode correspondence is unavailable.
    #[must_use]
    pub fn matches_unicode_source_value(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        value: &str,
    ) -> Option<bool> {
        self.words.matches_source(image, config).then_some(())?;
        let word = self.words.original_argument_word(self.reference.argument)?;
        let original = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
        Some(original.as_slice() == value.as_bytes())
    }

    /// Match a registry package descriptor, reflecting declared provider aliases.
    #[must_use]
    pub fn matches_package_descriptor(&self, metadata: &str) -> bool {
        let name = self.native_key.as_ref().map_or(
            self.literal.as_slice(),
            tcl_registry::native_package::NativePackageNameKey::bytes,
        );
        tcl_registry::native_package::matches_package_descriptor(name, metadata)
    }

    /// Match fixed ASCII metadata under the original package purpose.
    #[must_use]
    pub fn matches_ascii(&self, metadata: &str) -> bool {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        self.native_key.as_ref().map_or_else(
            || metadata.is_ascii() && self.literal == metadata.as_bytes(),
            |key| key.matches_ascii(metadata),
        )
    }
}

/// Actual selected package reference or explicit source-coverage uncertainty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalSourcePackageReferenceSelection {
    /// Genuine original name under the selected require/provide operation.
    Reference(OriginalSourcePackageReference),
    /// Complete selected operation contributes no package reference.
    NotReference,
    /// Missing schema, dynamic name or unsupported original operand.
    Unavailable,
}

impl OriginalRegistryWords {
    /// Retain the distinction between missing source representation and an
    /// actual selected non-package operation. No package-table absence follows.
    #[must_use]
    pub fn package_reference_selection(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> OriginalSourcePackageReferenceSelection {
        use tcl_registry::source_navigation::SourcePackageReferenceSelection as Selection;
        let Some(selection) = self.with_source_schema(context, |schema| {
            schema.authored_source_package_reference_selection()
        }) else {
            return OriginalSourcePackageReferenceSelection::Unavailable;
        };
        match selection {
            Selection::NotReference => OriginalSourcePackageReferenceSelection::NotReference,
            Selection::Unavailable => OriginalSourcePackageReferenceSelection::Unavailable,
            Selection::Reference(_) => self.package_reference(context).map_or(
                OriginalSourcePackageReferenceSelection::Unavailable,
                OriginalSourcePackageReferenceSelection::Reference,
            ),
        }
    }

    /// Project a package source reference using the same complete actual
    /// availability context, selected grammar and original effective operand.
    #[must_use]
    pub fn package_reference(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<OriginalSourcePackageReference> {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        let reference = self.with_source_schema(context, selected_source_package_reference)??;
        let literal = self
            .arguments
            .get(reference.argument)?
            .literal_bytes()?
            .to_vec();
        let native_key = self
            .original_argument_value_input(reference.argument)
            .map(|input| {
                tcl_registry::native_package::NativePackageNameKey::from_native_units(
                    input.bytes(),
                    input.policy(),
                )
            });
        Some(OriginalSourcePackageReference {
            words: std::sync::Arc::new(self.clone()),
            reference,
            literal,
            native_key,
        })
    }
}
