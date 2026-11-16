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

//! Variable-name candidates: visible locals, cross-namespace names and array elements.

use super::{
    CompletionEdit, CompletionItem, CompletionKind, FilteredCandidates, char_col_to_utf16,
    decorate_fuzzy_items, filter_candidates,
};
use crate::definition::utf16_col_to_char_col;
use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;

/// `::`-qualify a global variable name reached from a local (proc / nested
/// namespace) context; leave already-qualified names and locals untouched.
/// `qualify_globals` is the set of root-scope names that need it (`None` at
/// global scope, where bare names are correct).
fn qualified_var_name(name: &str, qualify_globals: Option<&FxHashSet<String>>) -> String {
    match qualify_globals {
        Some(globals)
            if globals.contains(name) && tcl_syntax::naming::unroot_rooted_key(name).is_none() =>
        {
            tcl_syntax::naming::root_unrooted_key(name)
        }
        _ => name.to_owned(),
    }
}

#[cfg(test)]
pub(super) fn variable_completions(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    partial: &str,
    trigger: char,
) -> Vec<CompletionItem> {
    let index = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&index, source, line, character);
    let Ok(Some(prefix)) =
        crate::source_structure::original_variable_cursor_prefix(source, analysis, cursor)
    else {
        return Vec::new();
    };
    if source.get(prefix.partial_span().as_range()) != Some(partial)
        || prefix.is_braced() != (trigger == '{')
    {
        return Vec::new();
    }
    variable_completions_in_source(source, line, character, analysis, &prefix)
}

pub(super) fn variable_completions_in_source(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    prefix: &tcl_lexer::word_parts::VariableCursorPrefix,
) -> Vec<CompletionItem> {
    let line_text = source.split('\n').nth(line as usize).unwrap_or("");
    let line_index = tcl_lexer::LineIndex::new(source);
    let byte_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    let Some(image) = analysis
        .retained_command_realm()
        .and_then(|realm| realm.original_source_image())
    else {
        return Vec::new();
    };
    if image.bytes() != source.as_bytes()
        || !analysis.matches_original_source_image(image, config)
        || !prefix.matches_source(image, config)
        || prefix.reference_span().end() != byte_offset
    {
        return Vec::new();
    }
    let Some(partial) = source.get(prefix.partial_span().as_range()) else {
        return Vec::new();
    };
    let begin = line_index.position_at_utf16(prefix.reference_span().start(), source);
    let finish = line_index.position_at_utf16(prefix.completion_replacement_span().end(), source);
    if begin.line != line || finish.line != line {
        return Vec::new();
    }
    let has_open_brace = prefix.is_braced();
    let edit_start = Some(begin.character.get());
    let edit_end = finish.character.get();
    let end = utf16_col_to_char_col(line_text, edit_end);

    let Some(advice) = analysis.logical_variable_scope_advice(image, config, byte_offset) else {
        return Vec::new();
    };

    // An explicit index is distinct from parentheses inside a braced name.
    if prefix.index_span().is_some() {
        return array_element_completions(
            line_text,
            &advice,
            partial,
            end,
            edit_start,
            has_open_brace,
        );
    }

    // Source visibility and rooted insertion share the checked scope selector.
    let names = visible_variable_names(&advice, partial);

    let qualify_globals = crate::definition::global_vars_needing_qualification(&advice);

    let edit = VarEdit {
        start: edit_start,
        end: edit_end,
        has_open_brace,
        config,
    };
    let mut items = Vec::new();
    push_local_var_items(&mut items, names, qualify_globals.as_ref(), edit);
    // Cross-namespace candidates — variables in *other* namespaces, offered in
    // fully-qualified `::ns::var` form (vars in the cursor's own namespace
    // chain are already above as bare names).
    push_cross_namespace_vars(&mut items, &advice, partial, edit);
    if !items.is_empty() {
        return items;
    }

    // Fuzzy fallback for the whole variable response: nothing
    // prefix-matched the fragment, so re-enumerate every candidate
    // (local and cross-namespace) and offer the closest names.
    let mut universe = Vec::new();
    push_local_var_items(
        &mut universe,
        visible_variable_names(&advice, ""),
        qualify_globals.as_ref(),
        edit,
    );
    push_cross_namespace_vars(&mut universe, &advice, "", edit);
    let FilteredCandidates {
        candidates: mut fallback,
        fuzzy,
    } = filter_candidates(partial, universe, |item| {
        item.label.strip_prefix('$').unwrap_or(&item.label)
    });
    if fuzzy {
        // The on-screen fragment (sigil included) — the text editors
        // match the replaced range against.
        let typed = if has_open_brace {
            format!("${{{partial}")
        } else {
            format!("${partial}")
        };
        decorate_fuzzy_items(&mut fallback, &typed);
    }
    fallback
}

/// Sorted current source names matching the fragment before insertion validation.
pub(super) fn visible_variable_names(
    advice: &tcl_compiler::analyser::scope::LogicalVariableScopeAdvice<'_>,
    partial: &str,
) -> Vec<String> {
    let mut names: Vec<String> = crate::definition::visible_variable_names(advice)
        .into_iter()
        .filter(|n| n.starts_with(partial))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// Append `$name` / `${name}` items for `names` (already sorted and
/// deduplicated), `::`-qualifying globals reached from a local
/// context and forcing the `${…}` form when the user already typed
/// `${` or the bare `$name` syntax can't carry the name (hyphens,
/// dots, …).
pub(super) fn push_local_var_items(
    items: &mut Vec<CompletionItem>,
    names: Vec<String>,
    qualify_globals: Option<&FxHashSet<String>>,
    edit: VarEdit,
) {
    for name in names {
        // `::`-qualify a global reached from a local context; leave already
        // qualified names and locals untouched.
        let qname = qualified_var_name(&name, qualify_globals);
        let Some(new_text) =
            tcl_syntax::naming::scalar_source_reference(&qname, edit.config, edit.has_open_brace)
        else {
            continue;
        };
        let text_edit = edit.start.map(|start_char| CompletionEdit {
            start_char,
            end_char: edit.end,
            new_text: new_text.clone(),
        });
        items.push(CompletionItem {
            label: format!("${qname}"),
            insert_text: new_text,
            kind: CompletionKind::Variable,
            detail: None,
            sort_text: None,
            is_snippet: false,
            filter_text: None,
            text_edit,
            documentation: None,
        });
    }
}

/// Edit-range + brace-form parameters shared by the variable-name builders.
#[derive(Clone, Copy)]
pub(super) struct VarEdit {
    pub(super) start: Option<u32>,
    pub(super) end: u32,
    pub(super) has_open_brace: bool,
    pub(super) config: tcl_lexer::LexerConfig,
}

/// Append fully-qualified `::ns::var` candidates from namespaces outside the
/// cursor's own lexical chain, deduped against the labels already in `items`.
fn push_cross_namespace_vars(
    items: &mut Vec<CompletionItem>,
    advice: &tcl_compiler::analyser::scope::LogicalVariableScopeAdvice<'_>,
    partial: &str,
    edit: VarEdit,
) {
    let mut seen: FxHashSet<String> = items.iter().map(|i| i.label.clone()).collect();
    for qname in advice.cross_namespace_variable_names() {
        if !qname.starts_with(partial) {
            continue;
        }
        let label = format!("${qname}");
        if !seen.insert(label.clone()) {
            continue;
        }
        let Some(new_text) =
            tcl_syntax::naming::scalar_source_reference(&qname, edit.config, edit.has_open_brace)
        else {
            continue;
        };
        let text_edit = edit.start.map(|start_char| CompletionEdit {
            start_char,
            end_char: edit.end,
            new_text: new_text.clone(),
        });
        items.push(CompletionItem {
            label,
            insert_text: new_text,
            kind: CompletionKind::Variable,
            detail: Some("namespace variable".to_string()),
            sort_text: None,
            is_snippet: false,
            filter_text: None,
            text_edit,
            documentation: None,
        });
    }
}

/// Build `$arr(index)` completions for the recorded indices of `arr` when
/// the partial contains `(`.  Always returns (suggestions or empty) —
/// preventing command completion from replacing an array-element response.
fn array_element_completions(
    line_text: &str,
    advice: &tcl_compiler::analyser::scope::LogicalVariableScopeAdvice<'_>,
    partial: &str,
    end: usize,
    edit_start: Option<u32>,
    has_open_brace: bool,
) -> Vec<CompletionItem> {
    // The caller dispatches here exactly when `partial` contains a `(`.
    let Some((arr_name, elem_prefix)) =
        tcl_syntax::naming::array_variable_completion_prefix(partial)
    else {
        return Vec::new();
    };
    let candidates = advice.array_index_candidates(arr_name);
    let arr_edit_end = char_col_to_utf16(line_text, end);
    let FilteredCandidates {
        candidates: elems,
        fuzzy,
    } = filter_candidates(elem_prefix, candidates, |candidate| candidate.index());
    let mut items = Vec::new();
    for candidate in elems {
        let Some(new_text) = candidate.source_reference(has_open_brace) else {
            continue;
        };
        let elem = candidate.index();
        let text_edit = edit_start.map(|start_char| CompletionEdit {
            start_char,
            end_char: arr_edit_end,
            new_text: new_text.clone(),
        });
        items.push(CompletionItem {
            label: format!("${arr_name}({elem})"),
            insert_text: new_text,
            kind: CompletionKind::Variable,
            detail: None,
            sort_text: None,
            is_snippet: false,
            filter_text: None,
            text_edit,
            documentation: None,
        });
    }
    if fuzzy {
        // The on-screen fragment runs from the `$` to the cursor (the
        // partial spans `arr(prefix`), sigil included.
        decorate_fuzzy_items(&mut items, &format!("${partial}"));
    } else {
        items.sort_by(|a, b| a.label.cmp(&b.label));
    }
    items
}
