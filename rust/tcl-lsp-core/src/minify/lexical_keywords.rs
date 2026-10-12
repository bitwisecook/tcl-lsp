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

//! Keyword edits from complete Logical argv and selected availability.

use super::{Edit, MinifyEnv, apply_edits, lexical_metadata};
use tcl_lexer::{NativeScriptCommandWords, NativeWord, WordKind};
use tcl_registry::abbrev::{KeywordTable, PrefixMatching};
use tcl_registry::{InvocationWord, InvocationWords, ResolvedInvocation};

pub(super) fn abbreviate(source: &str, env: MinifyEnv<'_>) -> (String, usize) {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let Some(commands) = lexical_metadata::commands(source, env) else {
        return (source.to_owned(), 0);
    };
    let mut edits = Vec::new();
    for command in commands {
        // Before-argv metadata cannot prove stability across dynamic argument
        // effects. The entire selected argv must be static original text.
        if command.words.iter().any(|word| {
            lexical_metadata::ascii_value(word).is_none()
                || word
                    .executable_parts()
                    .all_parts()
                    .any(|part| !matches!(part.part, tcl_lexer::ExecutablePart::Text(_)))
        }) {
            continue;
        }
        lexical_metadata::with_editable_command_schema(env, &command, |schema| {
            abbreviate_command(env, &command, schema, &mut edits);
        });
    }
    let count = edits.len();
    (apply_edits(source, edits), count)
}

fn abbreviate_command(
    env: MinifyEnv<'_>,
    command: &NativeScriptCommandWords,
    schema: &ResolvedInvocation<'_, '_>,
    edits: &mut Vec<Edit>,
) {
    if schema.semantics.argument_offset == 1
        && let Some(subcommand) = schema.subcommand.resolved()
        && let Some(word) = command.words.get(1)
    {
        let tables = keyword_tables(env, schema, subcommand_table);
        push_word_edit(word, subcommand.canonical_name, &tables, edits);
    }
    let options = schema.semantics.options;
    let Some(occurrences) = options.prefix_occurrences(
        schema
            .words
            .arguments()
            .slice_from(schema.semantics.argument_offset),
    ) else {
        return;
    };
    let tables = keyword_tables(env, schema, |_, schema| {
        schema.semantics.options.keyword_table()
    });
    for occurrence in occurrences {
        let Some(option) = occurrence.option else {
            continue;
        };
        let Some(index) = schema
            .semantics
            .argument_offset
            .checked_add(occurrence.argument_index)
            .and_then(|index| index.checked_add(1))
        else {
            continue;
        };
        if let Some(word) = command.words.get(index) {
            push_word_edit(word, option.name, &tables, edits);
        }
    }
}

fn subcommand_table(
    context: &tcl_registry::model::ContextRegistry,
    schema: &ResolvedInvocation<'_, '_>,
) -> KeywordTable<'static> {
    context
        .context()
        .resolve_spec_in_realm(
            context.commands(),
            schema.canonical_command,
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .map_or_else(empty_table, |spec| {
            spec.subcommand_table(
                schema.semantics.options.availability.query,
                schema.semantics.options.availability.package_version,
                None,
            )
        })
}

fn empty_table() -> KeywordTable<'static> {
    KeywordTable::new(std::iter::empty(), PrefixMatching::Enabled)
}

/// Each release comparison selects its actual authoring context; it is only
/// counterfactual keyword metadata, never a native execution/name receipt.
fn keyword_tables(
    env: MinifyEnv<'_>,
    schema: &ResolvedInvocation<'_, '_>,
    project: impl Fn(
        &tcl_registry::model::ContextRegistry,
        &ResolvedInvocation<'_, '_>,
    ) -> KeywordTable<'static>,
) -> Vec<KeywordTable<'static>> {
    let mut tables = vec![project(env.context, schema)];
    for name in tcl_registry::version_range::forward_range(env.dialect.name)
        .iter()
        .skip(1)
    {
        let context = tcl_registry::model::ingress::static_context_for(name);
        let mut words = InvocationWords::from_arguments(
            InvocationWord::Literal(schema.canonical_command),
            schema.words.arguments(),
        );
        if let Some(profile) = context.commands().profile() {
            words = words.with_dialect(tcl_registry::InvocationDialect::of_profile(profile));
        }
        let selected =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                words,
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        tables.push(
            selected
                .resolved()
                .as_ref()
                .map_or_else(empty_table, |schema| project(context, schema)),
        );
    }
    tables
}

fn push_word_edit(
    word: &NativeWord,
    canonical: &'static str,
    tables: &[KeywordTable<'static>],
    edits: &mut Vec<Edit>,
) {
    let Some(value) = lexical_metadata::ascii_value(word) else {
        return;
    };
    // Escaped, quoted, compound and expanded spellings are not edit sites.
    if word.group().kind != WordKind::Bare
        || word.group().expand
        || word.written_bytes() != value.as_bytes()
    {
        return;
    }
    let Some(short) = shortest_spelling(tables, canonical) else {
        return;
    };
    if short.len() < value.len() {
        edits.push((
            word.span().start() as usize,
            word.span().len() as usize,
            short.to_owned(),
        ));
    }
}

fn shortest_spelling(
    tables: &[KeywordTable<'static>],
    canonical: &'static str,
) -> Option<&'static str> {
    let short = tables.first()?.minimal_unique_prefix(canonical)?;
    tables[1..]
        .iter()
        .all(|table| table.resolve(short).unique() == Some(canonical))
        .then_some(short)
}
