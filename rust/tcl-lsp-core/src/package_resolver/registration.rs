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

use tcl_lexer::{LexerConfig, SourceImage, Span, Token};
use tcl_registry::{CommandRegistry, InvocationDialect, InvocationWord, InvocationWords};

pub(super) struct Registration {
    pub(super) name: Option<String>,
    pub(super) version: Option<String>,
    pub(super) body: usize,
}

/// Select only an exact original command under the held complete source grammar.
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
    })
}
