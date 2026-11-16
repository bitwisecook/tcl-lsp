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

//! Completion candidates from the current source and selected Registry.
//!
//! Original variable, argument, member and workspace owners retain source
//! identity, frame and naming policy. Hosted source advice and explicitly
//! selected lexical compatibility have separate entry points. Missing or stale
//! original inputs cannot be reconstructed from displayed names.
//!
//! Registry roles select contextual commands, options and argument values.
//! Prefix filtering and bounded fuzzy fallback share [`filter_candidates`];
//! command-position fallback ranks candidates only when the whole response is
//! empty. Candidates provide source insertions, not live lookup or execution.
//!
//! The server owns caching and asynchronous scheduling; this provider performs
//! synchronous computation over the supplied source and analysis.

#[cfg(test)]
use rustc_hash::FxHashMap;
use tcl_compiler::analyser::AnalysisResult;
use tcl_registry::CommandRegistry;

mod original_context;
mod original_opaque;
mod original_properties;
mod original_variables;
mod original_workspace;

use crate::definition::utf16_col_to_char_col;
#[cfg(test)]
use tcl_dialect::model::SpecSurface;

mod argument_values;
mod commands;
mod methods;
mod scoped_environments;
mod variables;

#[cfg(test)]
use argument_values::subcommand_completions;
use argument_values::{
    arg_value_completions_from, arg_value_completions_from_subcommands, boolean_value_completions,
    event_name_completions, sub_subcommand_completions, switch_completions,
    switch_partial_at_position,
};
#[cfg(test)]
use commands::command_detail;
#[cfg(test)]
use commands::usage_bucket;
use commands::{
    builtin_completions, document_usage_counts, extend_builtin_completion_items,
    fuzzy_command_fallback, invoked_proc_completions, math_function_completions, proc_completions,
    proc_signature_str,
};
#[cfg(test)]
use methods::{
    generated_property_method_items, original_method_candidate_items, source_method_name_word,
};
use methods::{
    method_completions, original_instance_method_candidate_items_at,
    original_method_candidate_items_at, registry_method_items,
};
#[cfg(test)]
use methods::{method_items, original_class_method_items};
#[cfg(test)]
use scoped_environments::scoped_command_completions;
#[cfg(test)]
use scoped_environments::{definition_grammar_at_position, grammar_member_completions};
use scoped_environments::{
    definition_grammar_completion_items, extend_scoped_completion_items, scoped_env_at,
    scoped_op_completions,
};
#[cfg(test)]
use variables::variable_completions;
use variables::{
    VarEdit, push_local_var_items, variable_completions_in_source, visible_variable_names,
};

/// LSP completion-item kinds emitted by this provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompletionKind {
    /// Tcl variable.
    Variable,
    /// User-defined proc.
    #[default]
    Function,
    /// Enumerable argument value (e.g. a `string is <class>`
    /// character class).
    EnumValue,
    /// A context-aware code snippet.
    Snippet,
}

/// A single completion suggestion.
///
/// Presentation and insertion fields passed to the editor. Optional detail
/// provides a short synopsis independently of full documentation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompletionItem {
    /// Label shown in the completion list — for variables,
    /// `$name` / `${name}`; for procs, the proc's qualified or
    /// simple name.
    pub label: String,
    /// Text to insert when the item is accepted.
    pub insert_text: String,
    /// LSP completion kind.
    pub kind: CompletionKind,
    /// Optional detail text — typically a parameter-list
    /// summary for procs / a synopsis line for built-in
    /// commands.  `None` for items without extra detail.
    pub detail: Option<String>,
    /// Optional sort key.  When `Some`, the editor uses this
    /// string instead of the label for ordering completion
    /// results.  The usage-bucket scheme:
    /// `A<tier><usage>_<name>` for user procs, `B<usage>_<name>`
    /// for built-in commands.  Lower buckets sort first.
    pub sort_text: Option<String>,
    /// When `true`, `insert_text` is a VS Code snippet (tabstops
    /// `${1:…}` / `$0`) and the server emits
    /// `InsertTextFormat.Snippet`.  Plain items leave it
    /// `false`.
    pub is_snippet: bool,
    /// Optional `filterText` — what the editor matches the typed
    /// prefix against (snippets filter on their `tcl-…` prefix,
    /// not their human label).  `None` falls back to the label.
    pub filter_text: Option<String>,
    /// Optional explicit replacement edit.  When `Some`, the editor
    /// applies `new_text` over the given single-line range (UTF-16
    /// columns on the cursor line) verbatim instead of its own
    /// word-based replacement — required so a `$`/`-` prefix isn't
    /// duplicated or dropped on accept (var / switch / array
    /// completion).
    pub text_edit: Option<CompletionEdit>,
    /// Optional documentation (rendered in the editor's completion
    /// detail pane).  Distinct from `detail` (the one-line synopsis):
    /// switches carry their option help here, built-in commands their
    /// summary.  `None` for items with no extra docs.
    pub documentation: Option<String>,
}

/// A single-line replacement edit attached to a [`CompletionItem`].
/// `start_char` / `end_char` are UTF-16 columns on the cursor's line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionEdit {
    /// Inclusive start column (UTF-16) of the replaced range.
    pub start_char: u32,
    /// Exclusive end column (UTF-16) of the replaced range.
    pub end_char: u32,
    /// Replacement text.
    pub new_text: String,
}

/// Minimum typed-fragment length (in characters) for the fuzzy
/// completion fallback.  Zero- and one-character fragments sit within
/// one edit of nearly everything, so falling back there would be
/// noise, not typo correction.
const MIN_FUZZY_FRAGMENT_CHARS: usize = 2;

/// Cap on the fuzzy-fallback suggestion list.
const MAX_FUZZY_SUGGESTIONS: usize = 8;

/// Outcome of [`filter_candidates`]: the surviving candidates, and
/// whether they came from the fuzzy fallback (fuzzy items need
/// [`decorate_fuzzy_items`] before they reach an editor).
struct FilteredCandidates<T> {
    candidates: Vec<T>,
    fuzzy: bool,
}

/// The one shared candidate filter behind every completion list.
///
/// **Prefix path** — when `partial` is empty or prefix-matches at
/// least one candidate name — keeps exactly the prefix matches, in
/// input order.
///
/// **Fuzzy fallback** — when nothing prefix-matches and the typed
/// fragment is at least [`MIN_FUZZY_FRAGMENT_CHARS`] characters —
/// returns the candidates within the shared "did you mean…?" edit
/// budget ([`tcl_compiler::text::scaled_max_distance`]), ranked by
/// `(distance, name)` via [`tcl_compiler::text::rank_suggestions`] and
/// capped at [`MAX_FUZZY_SUGGESTIONS`], so a typo'd fragment
/// (`lsaerch`) still offers its target (`lsearch`) instead of an
/// empty list.
fn filter_candidates<T>(
    partial: &str,
    candidates: Vec<T>,
    name_of: impl Fn(&T) -> &str,
) -> FilteredCandidates<T> {
    if partial.is_empty()
        || candidates
            .iter()
            .any(|candidate| name_of(candidate).starts_with(partial))
    {
        let mut kept = candidates;
        kept.retain(|candidate| name_of(candidate).starts_with(partial));
        return FilteredCandidates {
            candidates: kept,
            fuzzy: false,
        };
    }
    if partial.chars().count() < MIN_FUZZY_FRAGMENT_CHARS {
        return FilteredCandidates {
            candidates: Vec::new(),
            fuzzy: false,
        };
    }
    let budget = tcl_compiler::text::scaled_max_distance(partial);
    let scored: Vec<(usize, T)> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let distance = tcl_compiler::text::edit_distance(partial, name_of(&candidate));
            (distance <= budget).then_some((distance, candidate))
        })
        .collect();
    let ranked = tcl_compiler::text::rank_suggestions(scored, MAX_FUZZY_SUGGESTIONS, &name_of);
    FilteredCandidates {
        fuzzy: !ranked.is_empty(),
        candidates: ranked,
    }
}

/// Decorate fuzzy-fallback items so editors keep and rank them.
///
/// Editors re-filter returned items against the typed word
/// (`filterText`, falling back to the label) with a subsequence
/// matcher that hides a fuzzy match — `lsaerch` is not a subsequence
/// of `lsearch` — so `filter_text` is pinned to the exact fragment the
/// user typed.  `sort_text` encodes the fallback rank (`F00_…`,
/// `F01_…`), preserving the `(distance, name)` order in editors that
/// re-sort: after real symbols (`A…`/`B…`/`C…`) and before snippets
/// (`Z0_…`).
fn decorate_fuzzy_items(items: &mut [CompletionItem], typed: &str) {
    for (rank, item) in items.iter_mut().enumerate() {
        item.filter_text = Some(typed.to_string());
        item.sort_text = Some(format!("F{rank:02}_{}", item.label));
    }
}

/// Match colon-led fragments against inserted qualification and other
/// fragments against labels, so a procedure's simple label stays usable.
fn item_match_name<'a>(item: &'a CompletionItem, partial: &str) -> &'a str {
    if partial.starts_with(':') {
        &item.insert_text
    } else {
        &item.label
    }
}

/// Convert a codepoint column on `line_text` to a UTF-16 column (for
/// LSP ranges).  The inverse of [`utf16_col_to_char_col`].
fn char_col_to_utf16(line_text: &str, char_col: usize) -> u32 {
    line_text
        .chars()
        .take(char_col)
        .map(|c| u32::try_from(c.len_utf16()).unwrap_or(1))
        .sum()
}

/// Registry-driven, context-aware completion: switch / event-name /
/// user-proc / subcommand / arg-value suggestions resolved from the
/// surrounding command's [`CommandRegistry`] spec.  Returns `None` when the
/// cursor isn't inside a recognised command context (so the caller falls
/// through to plain command + proc completion).
/// The `(receiver, is_dollar)` pair a command head presents to the
/// method-completion gate: a `$var` instance variable, or a plain bareword
/// instance-command name (a Tk widget path, or `CLASS create NAME`).
/// Substituted / decorated heads are neither.
fn dispatch_receiver_of(word: &tcl_lexer::NativeWord) -> Option<(String, bool)> {
    if word.group().expand {
        return None;
    }
    let written = word.image().try_text().ok()?.get(word.span().as_range())?;
    if let Some(name) = strip_instance_var(written, word.config()) {
        return Some((name, true));
    }
    let value = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
    if value.contains(&0) {
        return None;
    }
    let name = String::from_utf8(value).ok()?;
    (!name.is_empty()).then_some((name, false))
}

/// Variable receivers use source-scoped suggestions, independently of physical reads.
fn logical_completion_receiver_candidate<'a>(
    analysis: &'a AnalysisResult,
    registry: &CommandRegistry,
    head: &tcl_lexer::NativeWord,
    receiver: &str,
    variable: bool,
) -> Option<(&'a String, crate::definition::MethodBucket)> {
    if variable {
        let candidates = analysis.logical_source_receiver_class_candidates(head, registry)?;
        let mut classes = candidates.classes();
        let class = classes.next()?;
        return classes
            .next()
            .is_none()
            .then_some((class, crate::definition::MethodBucket::Instance));
    }
    crate::definition::logical_receiver_candidate_at(analysis, receiver, false, head.span().start())
}

fn context_aware_completions(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    partial: &str,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Vec<CompletionItem>> {
    let cursor = crate::definition::byte_offset_at(
        &tcl_lexer::LineIndex::new(source),
        source,
        line,
        character,
    );
    if let Some(items) = original_properties::items(source, cursor, line, analysis, partial) {
        return Some(items);
    }
    let original_class_context =
        crate::original_oo::class_completion_context(analysis, source, cursor);
    if let std::ops::ControlFlow::Break(Some(record)) = original_class_context {
        return Some(original_method_candidate_items_at(
            analysis,
            record,
            partial,
            tcl_compiler::analyser::types::MemberSide::ClassObject,
            Some((source, cursor, line)),
        ));
    }
    if let Some(record) =
        crate::original_oo::instance_method_completion_record(analysis, source, cursor)
    {
        return Some(original_instance_method_candidate_items_at(
            analysis,
            record,
            source,
            cursor,
            partial,
            Some((source, cursor, line)),
        ));
    }
    if !analysis.allows_lexical_declaration_advice() {
        return nonlexical_context_completions(source, line, character, cursor, analysis, partial);
    }
    let command = command_context_on_line(source, line, character, analysis)?;
    let cmd = command.head_text()?;
    let word_idx = command.word_index;

    // `$obj <method>` / `.w <subcommand>` — when the command head is an
    // instance variable *or* a bareword instance-command name (a Tk widget
    // path, or `CLASS create NAME`) whose class is known, complete the
    // methods/subcommands callable on it.  Checked before the registry
    // lookup because neither `$obj` nor a bareword widget path is itself a
    // registered command. These are advisory candidates, independent of the
    // temporal receipt required for navigation and editable references. A bare
    // name only resolves when it was actually bound by a create call
    // (`created_instance_commands`), not merely because some unrelated
    // variable of the same name happens to hold an object elsewhere.
    let line_index = tcl_lexer::LineIndex::new(source);
    if word_idx == 1
        && let Some((recv, is_dollar)) = dispatch_receiver_of(&command.head)
        && let Some((class_q, bucket)) = logical_completion_receiver_candidate(
            analysis,
            registry,
            &command.head,
            &recv,
            is_dollar,
        )
    {
        // Per-object member state layers over the class chain in real
        // dispatch order: per-object members offered first,
        // an unexport masks, a later export revives.
        let object_state =
            crate::definition::object_member_state_at(analysis, source, &recv, line, character);
        let items = if matches!(original_class_context, std::ops::ControlFlow::Continue(())) {
            method_completions(
                analysis,
                registry,
                class_q,
                bucket,
                partial,
                object_state,
                profile,
            )
        } else {
            // Registry object specifications supply authored candidate advice;
            // a missing source class cannot borrow String-keyed source members.
            registry_method_items(analysis, registry, class_q, profile).map(|items| {
                items
                    .into_iter()
                    .filter(|item| item.label.starts_with(partial))
                    .collect()
            })
        };
        if let Some(items) = items {
            return Some(items);
        }
    }

    // Inside a scoped command environment (a `report::defstyle` style script):
    // the ensemble operations of a scoped command (`top ` → `set`/`get`/`enable`
    // /…) complete at word index 1.  Checked before the registry lookup because
    // a scoped head (`top`) is not a registered command.
    if word_idx == 1
        && let Some(env) = scoped_env_at(analysis, source, line, character, &line_index)
        && let Some(scoped) = env.command(&cmd)
        && !scoped.subcommands.is_empty()
    {
        return Some(scoped_op_completions(
            scoped,
            partial,
            analysis.resolved_input.as_ref()?.lexer_config(),
        ));
    }

    original_context::items(source, cursor, line, character, analysis, partial)
}

/// [`completions`]'s `$var` / `${var}` trigger branch, split out so the
/// parent function stays under the line-count lint. `None` when the cursor
/// isn't on a variable trigger — the caller falls through to the rest of the
/// completion pipeline.
fn variable_trigger_completions(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<Vec<CompletionItem>> {
    let index = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&index, source, line, character);
    let position = index.position_at_utf16(cursor, source);
    // LSP: a character past the end of its line defaults back to the line
    // length, so only a position that lands inside a different place is
    // rejected.
    if position.line != line || position.character.get() > character {
        return Some(Vec::new());
    }
    let prefix =
        match crate::source_structure::original_variable_cursor_prefix(source, analysis, cursor) {
            Ok(Some(prefix)) => prefix,
            Ok(None) => return None,
            Err(_) => return Some(Vec::new()),
        };
    if let Some(items) = original_variables::items(source, line, character, analysis, &prefix) {
        return Some(items);
    }
    if !analysis.allows_retained_logical_declaration_advice() {
        return Some(Vec::new());
    }
    Some(variable_completions_in_source(
        source, line, character, analysis, &prefix,
    ))
}

/// Compute completions for a position in `source`.
///
/// `analysis` is the pre-computed analyser result; the caller
/// (server) is expected to cache it.  `registry`, when `Some`, opts into
/// the builtin command list at command position; the advice itself comes from
/// the retained document Registry and full availability context.  Returns an
/// empty vector when there is no useful suggestion (delimiter
/// run, EOF, etc.).
///
/// The trigger contexts checked in order:
///
/// 1. **Variable trigger** (`$prefix` / `${prefix}`) — short-
///    circuits to global-scope variable names.
/// 2. **Switch completion** — when the partial starts with `-`
///    and the surrounding command's spec declares matching
///    options.  Requires `registry`.
/// 3. **Subcommand completion** — when the cursor is at word
///    index 1 (i.e. the first argument of a command) and the
///    surrounding command's spec declares subcommands.
///    Requires `registry`.
/// 4. **Command + proc completion** — default fallback:
///    user-defined procs from `analysis.all_procs`, plus all
///    built-in commands the `registry` knows about.
#[must_use]
pub fn completions(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    caller_registry: Option<&CommandRegistry>,
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
    _dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<CompletionItem> {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    if let Some(items) = original_opaque::items(
        source,
        line,
        character,
        analysis,
        caller_registry.is_some(),
        &current,
    ) {
        return items;
    }
    if let Some(items) = variable_trigger_completions(source, line, character, analysis) {
        return items;
    }
    let line_index = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some(items) =
        expression_completion_items(source, (line, character, &line_index), analysis, &current)
    {
        return items;
    }
    let Some(prefix) = word_prefix_at_position(source, line, character, analysis) else {
        return Vec::new();
    };
    let Some(partial) = tcl_syntax::word_rules::original_word_cursor_literal_prefix(&prefix) else {
        return Vec::new();
    };
    // The current source owner retains the profile and actual command store.
    let profile = current.profile();
    let registry = Some(current.registry());

    // Per-point schema advice preserves effective operand origins, known
    // command barriers and the retained availability context.
    if let Some(registry) = registry
        && let Some(items) = context_aware_completions(
            source, line, character, analysis, registry, &partial, profile,
        )
    {
        return original_word_completion_edits(source, line, &prefix, items);
    }
    if prefix.word_index() != 0 {
        return noncommand_fuzzy_completion_items(
            source,
            (line, character, &line_index),
            analysis,
            registry,
            profile,
            &prefix,
        );
    }

    // Definition bodies and document grammars have a closed member vocabulary;
    // the selected Registry owns both nesting and the permitted heads.
    if let Some(items) = definition_grammar_completion_items(
        source,
        (line, character, &line_index),
        analysis,
        registry,
        &current,
        &partial,
        &prefix,
    ) {
        return items;
    }

    let usage = document_usage_counts(analysis);
    let mut items = proc_completions(analysis, &partial, &usage);
    // The retained Registry selects the builtin advice, but a caller that
    // supplies no registry has asked for document symbols only; the builtin
    // list is never widened beyond what the caller opted into.
    extend_builtin_completion_items(
        analysis,
        registry.filter(|_| caller_registry.is_some()),
        profile,
        (source, line, character, &line_index),
        &partial,
        &usage,
        &mut items,
    );
    // Inside a scoped command environment (a `report::defstyle` style script),
    // offer its command heads (`top`, `data`, `columns`, …) alongside the
    // ordinary command/proc set — a style body uses both scoped and core
    // commands.  Deduped by label so a scoped name never doubles a core one.
    extend_scoped_completion_items(
        analysis,
        (source, line, character, &line_index),
        &partial,
        &current,
        &mut items,
    );
    if let Some(index) = workspace {
        items.extend(original_workspace::items(
            source, cursor, analysis, index, &partial,
        ));
    }

    extend_context_snippets(
        &mut items,
        (source, line, character),
        analysis,
        cursor,
        &current,
    );
    // Preserve prefix matches before considering response-wide fuzzy candidates.
    fill_empty_completion_items(
        &mut items,
        (source, line, character, &line_index),
        analysis,
        registry,
        profile,
    );
    original_word_completion_edits(source, line, &prefix, items)
}

fn extend_context_snippets(
    items: &mut Vec<CompletionItem>,
    position: (&str, u32, u32),
    analysis: &AnalysisResult,
    cursor: u32,
    current: &crate::original_context::CurrentSourceContext<'_>,
) {
    let (source, line, character) = position;
    let profile = current.profile();
    // Context-aware snippet templates (`tcl-proc`, `tcl-if`,
    // …).  Only the command-position fallback reaches here (the
    // variable / switch / subcommand contexts returned earlier), so
    // snippets never pollute `$var` or `-option` completion.  Their
    // `Z0_…` sort key keeps them below real symbols, and the prefix
    // filter means they only surface once the user types `tcl-…`.
    let variable_references = original_variables::source_references(source, analysis, cursor)
        .unwrap_or_else(|| {
            fallback_completion_variable_references(source, analysis, cursor, current)
        });
    let snippet_partial = snippet_partial_at_position(source, line, character);
    // `current_event` / `file_events` drive the iRules event templates'
    // top-level guard and duplicate-event decline. Only the canonical iRules
    // profile carries those templates, including its accepted aliases.
    let (current_event, file_events) = completion_event_names(source, line, analysis, profile);
    let at_top_level = completion_is_top_level(source, analysis, cursor, current_event.as_ref());
    items.extend(crate::snippets::snippet_completions(
        &crate::snippets::SnippetContext {
            profile,
            indent_unit: "    ",
            variable_references: &variable_references,
            partial: &snippet_partial,
            at_top_level,
            file_events: &file_events,
        },
    ));
}

/// Variable-trigger detection — `$prefix` or `${prefix}`.
///
/// Returns `Some((trigger_kind, partial))` where `trigger_kind`
/// is `'$'` for plain `$` triggers and `'{'` for `${name}`
/// braced triggers.  Returns `None` when the cursor is not in
/// a variable-completion context.
///
/// Test convenience for source-only trigger fixtures; production retains the
/// analysis's original image, full configuration and enclosing script phase.
#[cfg(test)]
fn variable_trigger(source: &str, line: u32, character: u32) -> Option<(char, String)> {
    let index = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&index, source, line, character);
    let position = index.position_at_utf16(cursor, source);
    if position.line != line || position.character.get() != character {
        return None;
    }
    let image = tcl_lexer::SourceImage::document(source);
    let prefix = tcl_lexer::word_parts::variable_cursor_prefix_in(
        &image,
        tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?),
        cursor,
        // dialect-drift-ok: a #[cfg(test)] helper
        tcl_lexer::LexerConfig::default(),
    )
    .ok()??;
    Some((
        if prefix.is_braced() { '{' } else { '$' },
        source.get(prefix.partial_span().as_range())?.to_owned(),
    ))
}

/// Fixed expression-function trigger text, independently admitted by the
/// selected expression source phase. Command names use original word geometry.
fn expression_function_partial_at_position(source: &str, line: u32, character: u32) -> String {
    let Some(line_text) = source.split('\n').nth(line as usize) else {
        return String::new();
    };
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut start = col;
    while start > 0
        && (chars[start - 1].is_alphanumeric()
            || chars[start - 1] == '_'
            || chars[start - 1] == ':')
    {
        start -= 1;
    }
    chars[start..col].iter().collect()
}

fn word_prefix_at_position(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<tcl_lexer::word_parts::WordCursorPrefix> {
    let index = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&index, source, line, character);
    let position = index.position_at_utf16(cursor, source);
    if position.line != line || position.character.get() != character {
        return None;
    }
    crate::source_structure::original_word_cursor_prefix(source, analysis, cursor).ok()?
}

fn original_word_completion_edits(
    source: &str,
    line: u32,
    prefix: &tcl_lexer::word_parts::WordCursorPrefix,
    items: Vec<CompletionItem>,
) -> Vec<CompletionItem> {
    if !prefix.matches_source(&tcl_lexer::SourceImage::document(source), prefix.config()) {
        return Vec::new();
    }
    let index = tcl_lexer::LineIndex::new(source);
    let start = index.position_at_utf16(prefix.container_span().start(), source);
    let end = index.position_at_utf16(prefix.container_span().end(), source);
    if start.line != line || end.line != line {
        return Vec::new();
    }
    items
        .into_iter()
        .filter_map(|mut item| {
            if item.text_edit.is_none() {
                if !item.is_snippet {
                    let image = tcl_lexer::SourceImage::document(&item.insert_text);
                    let plan = tcl_lexer::native_script_words_in(
                        image.clone(),
                        tcl_lexer::Span::new(0, u32::try_from(image.len()).ok()?),
                        prefix.config(),
                    )
                    .ok()?;
                    let [command] = plan.commands.as_slice() else {
                        return None;
                    };
                    let [word] = command.words.as_slice() else {
                        return None;
                    };
                    if plan.fatal_tail.is_some()
                        || word.span() != tcl_lexer::Span::new(0, u32::try_from(image.len()).ok()?)
                        || tcl_syntax::word_rules::original_static_word_unicode_value(word)
                            .is_none()
                    {
                        return None;
                    }
                }
                item.text_edit = Some(CompletionEdit {
                    start_char: start.character.get(),
                    end_char: end.character.get(),
                    new_text: item.insert_text.clone(),
                });
            }
            Some(item)
        })
        .collect()
}

/// Fixed snippet trigger domain; `-` belongs to the authored `tcl-…` triggers.
fn snippet_partial_at_position(source: &str, line: u32, character: u32) -> String {
    let Some(line_text) = source.split('\n').nth(line as usize) else {
        return String::new();
    };
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut start = col;
    while start > 0 && (chars[start - 1].is_alphanumeric() || matches!(chars[start - 1], '_' | '-'))
    {
        start -= 1;
    }
    chars[start..col].iter().collect()
}

/// Original whole head and the cursor's zero-based argument position.
struct CompletionCommandContext {
    head: tcl_lexer::NativeWord,
    word_index: usize,
}

impl CompletionCommandContext {
    fn head_text(&self) -> Option<String> {
        if let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(&self.head)
        {
            return String::from_utf8(value).ok();
        }
        Some(
            self.head
                .image()
                .try_text()
                .ok()?
                .get(self.head.span().as_range())?
                .to_owned(),
        )
    }
}

/// Whole command and operand geometry in the current line prefix. This retains
/// lexical candidate context; receiver binding and Registry selection are separate.
fn command_context_on_line(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<CompletionCommandContext> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let image = analysis.retained_command_realm()?.original_source_image()?;
    let index = tcl_lexer::LineIndex::new(source);
    let start = crate::definition::byte_offset_at(&index, source, line, 0);
    let cursor = crate::definition::byte_offset_at(&index, source, line, character);
    let prefix = tcl_lexer::Span::new(start, cursor);
    let plan = tcl_lexer::native_script_words_in(image.clone(), prefix, current.config()).ok()?;
    if plan.fatal_tail.is_some() {
        return None;
    }
    let command = plan.commands.last()?;
    if command.source_extent?.end() != cursor {
        return None;
    }
    let head = command.words.first()?.clone();
    if head.group().expand {
        return None;
    }
    let last = command.words.last()?;
    let word_index = if last.span().end() < cursor {
        command.words.len()
    } else {
        command.words.len().checked_sub(1)?
    };
    Some(CompletionCommandContext { head, word_index })
}

/// The instance-variable name in a `$obj` / `${obj}` command head, or
/// `None` when the head is not a single bare variable reference.
fn strip_instance_var(cmd: &str, config: tcl_lexer::LexerConfig) -> Option<String> {
    let reference = tcl_lexer::word_parts::whole_var_ref(cmd.as_bytes(), config).ok()??;
    // Completion candidates use scalar handles. A closed combined element name
    // and an independently scanned index keep their separate array purpose.
    if reference.index.is_some()
        || tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
    {
        return None;
    }
    let name = std::str::from_utf8(reference.name).ok()?;
    (!name.is_empty()).then(|| name.to_owned())
}

/// The guaranteed-available version floor for a command's owning package,
/// resolved from the document's `package require` statements.
///
/// The rule itself lives in [`crate::document_floor::DocumentFloor`], the one
/// place a request-time provider resolves a floor, so every consumer reads it
/// without copying it.
fn package_version_floor<'a>(
    analysis: &'a AnalysisResult,
    spec: &tcl_registry::CommandSpec,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<&'a str> {
    crate::document_floor::DocumentFloor::new(analysis, profile).for_spec(spec)
}

#[cfg(test)]
mod source_catalogue_word_tests;

#[cfg(test)]
mod original_command_cursor_tests;

fn nonlexical_context_completions(
    source: &str,
    line: u32,
    character: u32,
    cursor: u32,
    analysis: &AnalysisResult,
    partial: &str,
) -> Option<Vec<CompletionItem>> {
    // A scoped body's operations (`top ` → `set`/`get`/…) under its
    // retained original body receipt; a scoped head is not a registered
    // command, so the registry selection below cannot reach it.
    if let Some(command) = command_context_on_line(source, line, character, analysis)
        && command.word_index == 1
        && let Some(cmd) = command.head_text()
        && let Some(env) = scoped_env_at(
            analysis,
            source,
            line,
            character,
            &tcl_lexer::LineIndex::new(source),
        )
        && let Some(scoped) = env.command(&cmd)
        && !scoped.subcommands.is_empty()
    {
        return Some(scoped_op_completions(
            scoped,
            partial,
            analysis.resolved_input.as_ref()?.lexer_config(),
        ));
    }
    original_context::items(source, cursor, line, character, analysis, partial)
}

fn fallback_completion_variable_references(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    current: &crate::original_context::CurrentSourceContext<'_>,
) -> Vec<String> {
    let image = tcl_lexer::SourceImage::document(source);
    let Some(advice) = analysis.logical_variable_scope_advice(&image, current.config(), cursor)
    else {
        return Vec::new();
    };
    let qualify_globals = crate::definition::global_vars_needing_qualification(&advice);
    let mut candidates = Vec::new();
    push_local_var_items(
        &mut candidates,
        visible_variable_names(&advice, ""),
        qualify_globals.as_ref(),
        VarEdit {
            start: None,
            end: 0,
            has_open_brace: false,
            config: current.config(),
        },
    );
    candidates
        .into_iter()
        .map(|item| item.insert_text)
        .collect()
}

fn completion_event_names(
    source: &str,
    line: u32,
    analysis: &AnalysisResult,
    profile: &'static tcl_dialect::DialectProfile,
) -> (Option<String>, Vec<String>) {
    if profile.is_irules() {
        let facts = crate::irules_context::EventHandlerFacts::from_analysis(source, analysis);
        facts.map_or_else(
            || (None, Vec::new()),
            |facts| (facts.enclosing_event(line), facts.file_events()),
        )
    } else {
        (None, Vec::new())
    }
}

fn completion_is_top_level(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    current_event: Option<&String>,
) -> bool {
    if analysis.allows_lexical_declaration_advice() {
        current_event.is_none()
    } else {
        analysis
            .body_lexer_config
            .and_then(|config| {
                crate::source_structure::SourceStructure::capture(source, Some(analysis), config)
            })
            .and_then(|structure| {
                structure
                    .scripts
                    .into_iter()
                    .filter(|(span, _)| span.start() <= cursor && cursor <= span.end())
                    .min_by_key(|(span, _)| span.end() - span.start())
            })
            .is_some_and(|(_, depth)| depth == 0)
    }
}

fn expression_completion_items(
    source: &str,
    position: (u32, u32, &tcl_lexer::LineIndex),
    analysis: &AnalysisResult,
    current: &crate::original_context::CurrentSourceContext<'_>,
) -> Option<Vec<CompletionItem>> {
    let (line, character, line_index) = position;
    if crate::expr_context::source_expr_arg_context_at(
        source, analysis, line, character, line_index,
    ) {
        return Some(math_function_completions(
            current.registry(),
            current.profile(),
            &expression_function_partial_at_position(source, line, character),
        ));
    }
    None
}

fn noncommand_fuzzy_completion_items(
    source: &str,
    position: (u32, u32, &tcl_lexer::LineIndex),
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    prefix: &tcl_lexer::word_parts::WordCursorPrefix,
) -> Vec<CompletionItem> {
    let (line, character, line_index) = position;
    original_word_completion_edits(
        source,
        line,
        prefix,
        fuzzy_command_fallback(
            source, line, character, analysis, registry, profile, line_index,
        ),
    )
}

/// The spelling a completion inserts for a native value: the bare word when
/// the value is ASCII and the bare word alone reads back as exactly that
/// value under the document's grammar, otherwise the checked quoted word.
fn insertable_source_word(quoted: String, value: &[u8], config: tcl_lexer::LexerConfig) -> String {
    match std::str::from_utf8(value) {
        Ok(text)
            if text.is_ascii()
                && tcl_syntax::word_rules::lexical_unicode_source_word(text, config).as_deref()
                    == Some(text) =>
        {
            text.to_owned()
        }
        _ => quoted,
    }
}

fn fill_empty_completion_items(
    items: &mut Vec<CompletionItem>,
    position: (&str, u32, u32, &tcl_lexer::LineIndex),
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
) {
    let (source, line, character, line_index) = position;
    if items.is_empty() {
        *items = fuzzy_command_fallback(
            source, line, character, analysis, registry, profile, line_index,
        );
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_proc_completion_tests;

#[cfg(test)]
mod original_oo_completion_tests;

#[cfg(test)]
mod original_inherited_completion_tests;

#[cfg(test)]
mod original_property_completion_tests;

#[cfg(test)]
mod original_own_object_completion_tests;

#[cfg(test)]
mod original_expression_completion_tests;

#[cfg(test)]
mod original_document_grammar_tests;

#[cfg(test)]
mod original_scoped_completion_tests;

#[cfg(test)]
mod original_snippet_context_tests;

#[cfg(test)]
mod original_package_completion_tests;

#[cfg(test)]
mod original_receiver_domain_tests;

#[cfg(test)]
mod original_source_instance_completion_tests;

#[cfg(test)]
mod configured_receiver_completion_tests;

#[cfg(test)]
mod selected_catalogue_provider_tests;

#[cfg(test)]
mod inherited_registry_completion_provider_tests;

#[cfg(test)]
mod inherited_registry_roster_completion_tests;

#[cfg(test)]
mod logical_scoped_environment_tests;

#[cfg(test)]
mod property_producer_completion_tests;
