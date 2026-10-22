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

//! Explicit standalone package-registration source data, without loading authority.

#[cfg(test)]
use tcl_lexer::Span;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Token};
use tcl_registry::CommandRegistry;
#[cfg(test)]
use tcl_registry::{InvocationDialect, InvocationWord, InvocationWords};

pub(super) struct Registration {
    name: Option<String>,
    version: Option<String>,
    body: usize,
    image: SourceImage,
    config: LexerConfig,
    registry: tcl_registry::RegistrySemanticKey,
    words: Vec<NativeWord>,
    local: Option<(String, Vec<Vec<Token>>)>,
    _premise: Option<
        std::sync::Arc<
            tcl_compiler::registry_invocation::source_structure::OriginalPackageIndexAdvertisement,
        >,
    >,
}

impl Registration {
    /// Reborrow this selected survey result only at its complete original script
    /// and word vector. It supplies no reached lookup or package installation.
    pub(super) fn advertisement(
        &self,
        source: &str,
        tokens: &[Vec<Token>],
        registry: &CommandRegistry,
        config: LexerConfig,
    ) -> Option<(&str, &str, usize)> {
        if let Some((text, shown)) = &self.local {
            if text != source
                || shown != tokens
                || self.config != config
                || self.registry != registry.snapshot().semantic_key()
            {
                return None;
            }
            return Some((self.name.as_deref()?, self.version.as_deref()?, self.body));
        }
        if self.image.bytes() != source.as_bytes()
            || self.config != config
            || self.registry != registry.snapshot().semantic_key()
            || self.words.len() != tokens.len()
            || self.words.iter().zip(tokens).any(|(word, tokens)| {
                word.image() != &self.image
                    || word.config() != config
                    || word.tokens() != tokens.as_slice()
            })
        {
            return None;
        }
        Some((self.name.as_deref()?, self.version.as_deref()?, self.body))
    }
}

/// Test convenience for the same complete-plan issuer used by the live walker.
#[cfg(test)]
pub(super) fn select(
    source: &str,
    tokens: &[Vec<Token>],
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<Registration> {
    let end = u32::try_from(source.len()).ok()?;
    let plan =
        tcl_lexer::native_script_words_in(SourceImage::document(source), Span::new(0, end), config)
            .ok()?;
    select_in_plan(source, tokens, registry, config, &plan)
}

#[cfg(test)]
pub(super) fn select_in_plan(
    source: &str,
    tokens: &[Vec<Token>],
    registry: &CommandRegistry,
    config: LexerConfig,
    plan: &tcl_lexer::NativeScriptWordsPlan,
) -> Option<Registration> {
    if plan.fatal_tail.is_some() {
        return None;
    }
    let mut matches = plan.commands.iter().filter(|command| {
        command.words.len() == tokens.len()
            && command
                .words
                .iter()
                .zip(tokens)
                .all(|(word, tokens)| word.tokens() == tokens.as_slice())
    });
    let command = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    if command
        .words
        .iter()
        .any(|word| word.image().bytes() != source.as_bytes() || word.config() != config)
    {
        return None;
    }
    let values = command
        .words
        .iter()
        .map(|word| {
            let value = String::from_utf8(
                tcl_syntax::word_rules::original_static_word_unicode_value(word)?,
            )
            .ok()?;
            (!value.contains('\0')).then_some(value)
        })
        .collect::<Vec<_>>();
    let head = values.first()?.as_deref()?;
    // The index survey's existing authored command convention, not runtime lookup.
    let head = tcl_syntax::naming::canonical_written_command(head);
    let head = tcl_syntax::naming::unroot_rooted_key(&head).unwrap_or(&head);
    let arguments = command
        .words
        .iter()
        .zip(&values)
        .skip(1)
        .map(|(word, value)| {
            if word.group().expand {
                InvocationWord::Expanded
            } else {
                value
                    .as_deref()
                    .map_or(InvocationWord::Dynamic, InvocationWord::Literal)
            }
        })
        .collect::<Vec<_>>();
    if command.words.first()?.group().expand {
        return None;
    }
    // The public standalone survey explicitly uses Tcl compatibility source
    // vocabulary; the actual registry and full lexical grammar remain supplied.
    let profile = registry
        .profile()
        .unwrap_or(tcl_dialect::DialectProfile::plain_tcl());
    let mut dialect = InvocationDialect::of_profile(profile);
    dialect.lexer_grammar = config.grammar_over(dialect.lexer_grammar);
    dialect.word_values = tcl_syntax::word_rules::WordValueRules::from_config(&config);
    let schema = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &arguments)
                .with_dialect(dialect),
            None,
        )
        .resolved()?;
    let (name, version, body) = schema.authored_source_package_registration()?;
    Some(Registration {
        name: values.get(name.checked_add(1)?)?.clone(),
        version: values.get(version.checked_add(1)?)?.clone(),
        body: body.checked_add(1)?,
        image: command.words.first()?.image().clone(),
        config,
        registry: registry.snapshot().semantic_key(),
        words: command.words.clone(),
        local: None,
        _premise: None,
    })
}

/// Join callback presentation coordinates to the actual retained original vector.
/// No Document reconstruction, source-channel conversion or inverse identity.
pub(super) fn select_original_advertisement(
    source: &str,
    tokens: &[Vec<Token>],
    image: &SourceImage,
    region: tcl_lexer::Span,
    registry: &CommandRegistry,
    config: LexerConfig,
    premise: std::sync::Arc<
        tcl_compiler::registry_invocation::source_structure::OriginalPackageIndexAdvertisement,
    >,
) -> Option<Registration> {
    use tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource;
    let OriginalRegistrySource::SourceTransitions(advice) = premise.words().source() else {
        return None;
    };
    let original = advice.original_words();
    let source_end = u32::try_from(source.len()).ok()?;
    if image.try_text().ok()?.get(region.as_range())? != source
        || original.len() != tokens.len()
        || original.iter().zip(tokens).any(|(word, shown)| {
            word.image() != image
                || word.config() != config
                || word.span().start() < region.start()
                || word.span().end() > region.end()
                || word.tokens().len() != shown.len()
                || word.tokens().iter().zip(shown).any(|(actual, shown)| {
                    shown.span.end() > source_end
                        || actual.kind != shown.kind
                        || actual.content_offset != shown.content_offset
                        || Some(actual.span.start())
                            != region.start().checked_add(shown.span.start())
                        || Some(actual.span.end()) != region.start().checked_add(shown.span.end())
                })
        })
    {
        return None;
    }
    Some(Registration {
        name: premise.name().filter(|value| !value.contains('\0')),
        version: premise.version().filter(|value| !value.contains('\0')),
        body: premise.body_argument().checked_add(1)?,
        image: image.clone(),
        config,
        registry: registry.snapshot().semantic_key(),
        words: original.to_vec(),
        local: Some((source.to_owned(), tokens.to_vec())),
        _premise: Some(premise),
    })
}
