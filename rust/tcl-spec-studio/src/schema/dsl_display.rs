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

//! Read-only DSL display metadata: the source image with its registered vocabulary and the statement projection.

use super::{command_field, subcommand_field};

/// Original complete DSL image and its actual registered vocabulary context.
/// Unsupported major versions retain source geometry but withhold vocabulary.
#[derive(Clone)]
pub struct DslDisplayContext {
    image: tcl_lexer::SourceImage,
    vocabulary: Option<String>,
    registry: Option<&'static tcl_registry::CommandRegistry>,
}

impl DslDisplayContext {
    /// Retain the supplied authored image and the loader's vocabulary decision.
    #[must_use]
    pub fn for_source(source: &str) -> Self {
        let image = tcl_lexer::SourceImage::document(source);
        let vocabulary = dsl_source_vocabulary(&image);
        let supported = vocabulary.as_ref().is_some_and(|version| {
            version.as_deref().is_none_or(|version| {
                tcl_compiler::parsing::spectcl_source::declared_pack_major(version).is_some()
                    && !tcl_compiler::parsing::spectcl_source::unsupported_pack_major(version)
            })
        });
        Self {
            image,
            vocabulary: vocabulary.flatten(),
            registry: supported.then(|| crate::environment::store_for_dialect("spectcl")),
        }
    }
    /// Exact original full image; nested display never creates a new source.
    #[must_use]
    pub fn image(&self) -> &tcl_lexer::SourceImage {
        &self.image
    }
    /// Actual authored version when present; a fragment invents no version.
    #[must_use]
    pub fn vocabulary(&self) -> Option<&str> {
        self.vocabulary.as_deref()
    }
}

// Outer `None`: the source text is unavailable; inner `None`: no vocabulary.
#[allow(clippy::option_option)]
fn dsl_source_vocabulary(image: &tcl_lexer::SourceImage) -> Option<Option<String>> {
    let source = image.try_text().ok()?;
    let Some((range, _)) = tcl_spectcl::speclib_version_span(source) else {
        return Some(None);
    };
    let config = tcl_spectcl::loader::pack_source_lexer_config(
        tcl_lexer::SourceSegmentationPurpose::WholeInput,
    );
    let region = tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?);
    let tokens = tcl_lexer::source_region_tokens_in(image, region, config).ok()?;
    for command in tcl_lexer::group_commands_bytes(&tokens, image.bytes(), config) {
        for group in command.words {
            let Ok(word) =
                tcl_lexer::NativeWord::from_group(image.clone(), config, &tokens, &group)
            else {
                continue;
            };
            if word
                .content_span()
                .is_ok_and(|content| content.as_range() == range)
            {
                return Some(Some(dsl_word_value(&word)?));
            }
        }
    }
    None
}

/// Actual static definition-body descriptor, retained only for source display.
#[derive(Debug, Clone, Copy)]
pub struct DslDisplayGrammar(&'static tcl_registry::definer::DefinitionBodyGrammar);

impl PartialEq for DslDisplayGrammar {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for DslDisplayGrammar {}

/// Readonly DSL statement projection from the actual registered descriptor.
/// Source words retain their original whole image and selected loader grammar.
#[derive(Debug, Clone)]
pub struct DslStatementDisplay {
    /// Original static statement value, used only for DSL metadata lookup.
    pub head: String,
    /// Exact original operands the registered statement declares as DSL bodies.
    pub bodies: Vec<tcl_lexer::NativeWord>,
    /// Actual declared grammar to retain when projecting the selected body.
    pub grammar: Option<DslDisplayGrammar>,
}

impl DslDisplayContext {
    /// Project body roles from the actual `SpecTcl` registry, without a second
    /// keyword/alias list or a Native command-selection claim.
    #[must_use]
    pub fn statement(
        &self,
        words: &[tcl_lexer::NativeWord],
        parent: Option<DslDisplayGrammar>,
    ) -> Option<DslStatementDisplay> {
        let first = words.first()?;
        (first.image() == &self.image
            && first.config().nested()
                == tcl_spectcl::loader::pack_source_lexer_config(
                    tcl_lexer::SourceSegmentationPurpose::WholeInput,
                )
                .nested())
        .then_some(())?;
        if first.group().expand
            || words
                .iter()
                .any(|word| word.image() != first.image() || word.config() != first.config())
        {
            return None;
        }
        let head = String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
            first,
        )?)
        .ok()?;
        if let Some(parent) = parent {
            parent.0.member(&head)?;
        }
        let registry = self.registry?;
        let spec = registry.get(&head)?;
        let mut bodies = Vec::new();
        if spec.definition_body.is_some() {
            let values = words[1..]
                .iter()
                .map(|word| {
                    tcl_syntax::word_rules::original_static_word_unicode_value(word)
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                })
                .collect::<Vec<_>>();
            let arguments = words[1..]
                .iter()
                .zip(&values)
                .map(|(word, value)| {
                    if word.group().expand {
                        tcl_registry::InvocationWord::Expanded
                    } else {
                        value.as_deref().map_or(
                            tcl_registry::InvocationWord::Dynamic,
                            tcl_registry::InvocationWord::Literal,
                        )
                    }
                })
                .collect::<Vec<_>>();
            let dialect = tcl_registry::InvocationDialect::of_profile(
                crate::environment::profile_for_dialect("spectcl"),
            );
            let args =
                tcl_registry::InvocationArguments::structured(&arguments).with_dialect(dialect);
            for ordinal in registry.arg_indices_for_role_words(
                &head,
                args,
                tcl_registry::arg_role::ArgRole::Body,
            )? {
                bodies.push(words.get(ordinal.checked_add(1)?)?.clone());
            }
        }
        Some(DslStatementDisplay {
            head,
            bodies,
            grammar: spec.definition_body.map(DslDisplayGrammar),
        })
    }

    /// Help for the exact selected DSL statement or one of its declared flags.
    /// Field help uses exact keys; singular/plural and lifecycle suffixes are not
    /// invented. Those source spellings are documented by their actual registry row.
    #[must_use]
    pub fn help(
        &self,
        head: &str,
        word: &str,
        parent: Option<DslDisplayGrammar>,
    ) -> Option<(String, String)> {
        if let Some(parent) = parent {
            parent.0.member(head)?;
        }
        let spec = self.registry?.get(head)?;
        if head == word {
            if let Some(field) = command_field(word).or_else(|| subcommand_field(word)) {
                return Some((
                    field.label.to_owned(),
                    crate::help::field_help(field.key)
                        .unwrap_or(field.doc)
                        .to_owned(),
                ));
            }
            let hover = spec.hover.as_ref()?;
            return Some((
                format!("{word} — {}", hover.summary),
                hover.snippet.to_owned(),
            ));
        }
        let option = spec
            .options
            .iter()
            .find(|option| option.name == word || option.aliases.contains(&word))?;
        Some((word.to_owned(), option.detail.to_owned()))
    }
}

/// Original static Unicode word value for readonly DSL display metadata.
/// Dynamic/expanded operands never become a label-derived selected keyword.
#[must_use]
pub fn dsl_word_value(word: &tcl_lexer::NativeWord) -> Option<String> {
    (!word.group().expand).then_some(())?;
    String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
        word,
    )?)
    .ok()
}
