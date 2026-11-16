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

//! Candidates the registry gives an argument position: switches, event names, subcommands and enumerated values.

#[cfg(test)]
use super::package_version_floor;
use super::{
    CompletionEdit, CompletionItem, CompletionKind, FilteredCandidates, decorate_fuzzy_items,
    filter_candidates, word_prefix_at_position,
};
use tcl_compiler::analyser::AnalysisResult;
use tcl_dialect::model::SpecSurface;

pub(super) fn switch_partial_at_position(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<(String, (u32, u32))> {
    let prefix = word_prefix_at_position(source, line, character, analysis)?;
    let value = tcl_syntax::word_rules::original_word_cursor_literal_prefix(&prefix)?;
    if !value.starts_with('-') {
        return None;
    }
    let index = tcl_lexer::LineIndex::new(source);
    let start = index.position_at_utf16(prefix.container_span().start(), source);
    let end = index.position_at_utf16(prefix.container_span().end(), source);
    (start.line == line && end.line == line)
        .then_some((value, (start.character.get(), end.character.get())))
}

pub(super) fn switch_completions(
    options: &[tcl_registry::hover::OptionSpec],
    context: &tcl_registry::model::ResolvedContext,
    parent_surface: Option<&'static [SpecSurface]>,
    partial: &str,
    edit: (u32, u32),
    package_version: Option<&str>,
    config: tcl_lexer::LexerConfig,
) -> Vec<CompletionItem> {
    let mut opts: Vec<_> = options
        .iter()
        .filter(|opt| {
            opt.available_for_version(package_version)
                && context.option_available(opt, parent_surface)
        })
        .collect();
    opts.sort_unstable_by_key(|opt| opt.name);
    let FilteredCandidates {
        candidates: opts,
        fuzzy,
    } = filter_candidates(partial, opts, |opt| opt.name);
    let mut items: Vec<CompletionItem> = opts
        .into_iter()
        .filter_map(|opt| {
            let insert_text =
                tcl_syntax::word_rules::lexical_unicode_source_word(opt.name, config)?;
            let doc = (!opt.detail.is_empty()).then(|| opt.detail.to_owned());
            Some(CompletionItem {
                label: opt.name.to_owned(),
                insert_text: insert_text.clone(),
                kind: CompletionKind::Function,
                detail: doc.clone(),
                documentation: doc,
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: Some(CompletionEdit {
                    start_char: edit.0,
                    end_char: edit.1,
                    new_text: insert_text,
                }),
            })
        })
        .collect();
    if fuzzy {
        // `partial` carries the leading dash, matching the on-screen
        // fragment the replace edit covers.
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}

/// Event candidates retain the actual BIG-IP placement and source grammar.
pub(super) fn event_name_completions(
    partial: &str,
    analysis: &AnalysisResult,
) -> Vec<CompletionItem> {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let config = input.lexer_config();
    if analysis.body_lexer_config != Some(config) {
        return Vec::new();
    }
    let context = input.context_registry();
    let Some(target) = context.context().placement_floor("f5-irules-cmds") else {
        return Vec::new();
    };
    let reg = tcl_registry::events::EventRegistry::build();
    let mut names: Vec<&str> = reg
        .all_event_names()
        .into_iter()
        .filter(|name| reg.event_available_at(name, target.as_str()))
        .collect();
    names.sort_unstable();
    let FilteredCandidates {
        candidates: names,
        fuzzy,
    } = filter_candidates(partial, names, |name| *name);
    let mut items: Vec<CompletionItem> = names
        .into_iter()
        .filter_map(|name| {
            let insert_text = tcl_syntax::word_rules::lexical_unicode_source_word(name, config)?;
            // Describe the event from its registry props (sides / transport /
            // implied profiles) so the item carries documentation.
            let doc = reg.get_props(name).map(|p| {
                let mut parts = vec![format!("F5 iRules event `{name}`")];
                if !p.implied_profiles.is_empty() {
                    parts.push(format!("Profiles: {}", p.implied_profiles.join(", ")));
                }
                if let Some(release) = p.lifecycle().deprecated {
                    parts.push(format!("Deprecated as of BIG-IP {release}"));
                }
                parts.join("\n\n")
            });
            Some(CompletionItem {
                label: name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: Some("F5 iRules event".to_string()),
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: doc.or_else(|| Some(format!("F5 iRules event `{name}`"))),
            })
        })
        .collect();
    if fuzzy {
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}

#[cfg(test)]
pub(super) fn subcommand_completions(
    spec: &tcl_registry::CommandSpec,
    analysis: &AnalysisResult,
    profile: &'static tcl_dialect::DialectProfile,
    partial: &str,
) -> Vec<CompletionItem> {
    // Only subcommands available under the active profile are offered —
    // the §5.1 `available_subcommands` gap: `dict getwithdefault` (9.0+)
    // must not be offered in an 8.6 buffer, and an iRules-only subcommand
    // must not surface in plain Tcl.
    let floor = package_version_floor(analysis, spec, profile);
    let subs: Vec<&tcl_registry::SubCommand> = crate::document_context_for_profile(profile)
        .available_subcommands(spec)
        .into_iter()
        .filter(|sub| sub.available_for_version(floor))
        .collect();
    arg_value_completions_from_subcommands(
        subs,
        partial,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    )
}

pub(super) fn arg_value_completions_from_subcommands(
    mut subs: Vec<&tcl_registry::SubCommand>,
    partial: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<CompletionItem> {
    subs.sort_unstable_by_key(|sub| sub.name);
    let FilteredCandidates {
        candidates: subs,
        fuzzy,
    } = filter_candidates(partial, subs, |sub| sub.name);
    let mut items: Vec<CompletionItem> = subs
        .into_iter()
        .filter_map(|sub| {
            let insert_text =
                tcl_syntax::word_rules::lexical_unicode_source_word(sub.name, config)?;
            Some(CompletionItem {
                label: sub.name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                // Surface the registry's one-line description, exactly as the
                // sub-subcommand / scoped-op / arg-value completions already do
                // — `string <TAB>` shows what each operation does, not a bare
                // name list.
                detail: (!sub.detail.is_empty()).then(|| sub.detail.to_owned()),
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            })
        })
        .collect();
    if fuzzy {
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}

/// Build completions for the second-level subcommands of a two-level ensemble
/// (`info object <op>` / `info class <op>`), filtered by `partial`.  Each
/// item's detail is the operation's one-line description.
pub(super) fn sub_subcommand_completions(
    mut subs: Vec<&'static tcl_registry::SubSubCommand>,
    partial: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<CompletionItem> {
    subs.sort_unstable_by_key(|s| s.name);
    let FilteredCandidates {
        candidates: subs,
        fuzzy,
    } = filter_candidates(partial, subs, |s| s.name);
    let mut items: Vec<CompletionItem> = subs
        .into_iter()
        .filter_map(|s| {
            let insert_text = tcl_syntax::word_rules::lexical_unicode_source_word(s.name, config)?;
            Some(CompletionItem {
                label: s.name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: (!s.detail.is_empty()).then(|| s.detail.to_owned()),
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            })
        })
        .collect();
    if fuzzy {
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}

/// Build completions for a value position the registry declares boolean
/// (`ArgRole::Boolean`), filtered by `partial`.
///
/// The vocabulary is `tcl_registry::abbrev::BOOLEAN_KEYWORDS` — the same list
/// `resolve_boolean` accepts, so completion, the formatter's canonical-boolean
/// rewrite, and the analyser cannot disagree about what a boolean word is.
pub(super) fn boolean_value_completions(
    partial: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<CompletionItem> {
    let candidates: Vec<&&str> = tcl_registry::abbrev::BOOLEAN_KEYWORDS.iter().collect();
    let FilteredCandidates { candidates, fuzzy } = filter_candidates(partial, candidates, |v| **v);
    let mut items: Vec<CompletionItem> = candidates
        .into_iter()
        .filter_map(|value| {
            let insert_text = tcl_syntax::word_rules::lexical_unicode_source_word(value, config)?;
            Some(CompletionItem {
                label: (*value).to_owned(),
                insert_text,
                kind: CompletionKind::EnumValue,
                detail: Some("boolean".to_owned()),
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            })
        })
        .collect();
    if fuzzy {
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}

pub(super) fn arg_value_completions_from(
    mut values: Vec<&tcl_registry::ArgValue>,
    partial: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<CompletionItem> {
    // Prefix matches retain value order; fuzzy matches retain distance ranking.
    values.sort_unstable_by_key(|v| v.value);
    let FilteredCandidates {
        candidates: values,
        fuzzy,
    } = filter_candidates(partial, values, |v| v.value);
    let mut items: Vec<CompletionItem> = values
        .into_iter()
        .filter_map(|v| {
            let insert_text = tcl_syntax::word_rules::lexical_unicode_source_word(v.value, config)?;
            Some(CompletionItem {
                label: v.value.to_owned(),
                insert_text,
                kind: CompletionKind::EnumValue,
                detail: (!v.detail.is_empty()).then(|| v.detail.to_owned()),
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            })
        })
        .collect();
    if fuzzy {
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}
