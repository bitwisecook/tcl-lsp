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

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_registry::CommandRegistry;

mod original_context;
mod original_opaque;
mod original_properties;
mod original_variables;
mod original_workspace;

use crate::definition::utf16_col_to_char_col;
use tcl_dialect::model::SpecSurface;

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
fn variable_completions(
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

fn variable_completions_in_source(
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
fn visible_variable_names(
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
fn push_local_var_items(
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
struct VarEdit {
    start: Option<u32>,
    end: u32,
    has_open_brace: bool,
    config: tcl_lexer::LexerConfig,
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

/// Class-command declaration candidates. This does not select an executed
/// method provider or model a maker/default delegate's native attachment.
fn original_class_method_items(
    analysis: &AnalysisResult,
    root: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    partial: &str,
) -> Vec<CompletionItem> {
    original_method_candidate_items(
        analysis,
        root,
        partial,
        tcl_compiler::analyser::types::MemberSide::ClassObject,
    )
}

fn original_method_candidate_items(
    analysis: &AnalysisResult,
    root: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    partial: &str,
    side: tcl_compiler::analyser::types::MemberSide,
) -> Vec<CompletionItem> {
    original_method_candidate_items_at(analysis, root, partial, side, None)
}

fn original_instance_method_candidate_items_at(
    analysis: &AnalysisResult,
    root: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    source: &str,
    cursor: u32,
    partial: &str,
    edit_context: Option<(&str, u32, u32)>,
) -> Vec<CompletionItem> {
    let own = match crate::original_oo::own_object_completion_methods(analysis, source, cursor) {
        std::ops::ControlFlow::Break(Some(own)) => own,
        std::ops::ControlFlow::Break(None) => return Vec::new(),
        std::ops::ControlFlow::Continue(()) => Vec::new(),
    };
    let receiver = edit_context
        .and_then(|_| crate::original_oo::instance_completion_source(analysis, source, cursor));
    if receiver
        .as_ref()
        .is_some_and(|receiver| receiver.class.declaration_site() != root.declaration_site())
    {
        return Vec::new();
    }
    rank_original_method_candidates(
        partial,
        original_method_candidate_items_with_own_at(
            analysis,
            root,
            "",
            tcl_compiler::analyser::types::MemberSide::Instance,
            OriginalMethodEditContext {
                position: edit_context,
                receiver: receiver.as_ref(),
            },
            &own,
        ),
    )
}

fn original_method_candidate_items_at(
    analysis: &AnalysisResult,
    root: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    partial: &str,
    side: tcl_compiler::analyser::types::MemberSide,
    edit_context: Option<(&str, u32, u32)>,
) -> Vec<CompletionItem> {
    rank_original_method_candidates(
        partial,
        original_method_candidate_items_with_own_at(
            analysis,
            root,
            "",
            side,
            OriginalMethodEditContext {
                position: edit_context,
                receiver: None,
            },
            &[],
        ),
    )
}

/// Rank only the authenticated source method universe. Matching insertion
/// spellings retains escaped-name prefixes; shared fuzzy ranking is UI advice.
fn rank_original_method_candidates(
    partial: &str,
    items: Vec<CompletionItem>,
) -> Vec<CompletionItem> {
    let FilteredCandidates {
        mut candidates,
        fuzzy,
    } = filter_candidates(partial, items, |item| {
        if item.insert_text.starts_with(partial) {
            item.insert_text.as_str()
        } else {
            item.label.as_str()
        }
    });
    if fuzzy {
        decorate_fuzzy_items(&mut candidates, partial);
    }
    candidates
}

#[derive(Clone, Copy)]
struct OriginalMethodEditContext<'a, 'b> {
    position: Option<(&'a str, u32, u32)>,
    receiver: Option<&'a crate::original_oo::instance_completion::InstanceCompletionSource<'b>>,
}

fn original_method_candidate_items_with_own_at(
    analysis: &AnalysisResult,
    root: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    partial: &str,
    side: tcl_compiler::analyser::types::MemberSide,
    edit_context: OriginalMethodEditContext<'_, '_>,
    own: &[crate::original_oo::OwnObjectMethodCandidate<'_>],
) -> Vec<CompletionItem> {
    use tcl_compiler::analyser::class_hierarchy::original_metadata::original_instance_metadata_order;
    use tcl_compiler::analyser::types::MemberSide;
    let providers = original_instance_metadata_order(analysis, root).unwrap_or_else(|| vec![root]);
    if side == MemberSide::Instance
        && providers.iter().any(|provider| {
            !provider
                .metadata()
                .original_properties
                .side_matches_producer_input(side, analysis)
        })
    {
        return Vec::new();
    }
    let mut names = Vec::<(
        tcl_core_types::NameBytes,
        tcl_syntax::naming::NamePolicyProtocol,
    )>::new();
    let mut items = Vec::new();
    extend_original_own_method_items(analysis, partial, edit_context, own, &mut names, &mut items);
    for provider in providers {
        for method in provider
            .metadata()
            .original_members
            .methods(side)
            .into_iter()
            .flatten()
        {
            if !method.exported()
                || (side == MemberSide::ClassObject
                    && provider.declaration_site() != root.declaration_site()
                    && match method.name_purpose() {
                        tcl_registry::definer::DefinitionMemberNamePurpose::TclOoMethod => {
                            !method.native_class_delegate()
                        }
                        tcl_registry::definer::DefinitionMemberNamePurpose::SourceValue => {
                            method.metadata().is_self_method
                        }
                    })
            {
                continue;
            }
            let input = method.original_name_input();
            let key = (
                tcl_core_types::NameBytes::from(input.bytes()),
                input.policy(),
            );
            if names.contains(&key) {
                continue;
            }
            let Some(spelling) = crate::original_oo::method_word(&method) else {
                continue;
            };
            let label = crate::original_oo::method_label(&method);
            if !label.starts_with(partial) && !spelling.starts_with(partial) {
                continue;
            }
            let Some(item) = original_method_candidate_item(
                analysis,
                input.bytes(),
                input.policy(),
                label,
                spelling,
                edit_context,
                "method — source declaration candidate",
            ) else {
                continue;
            };
            names.push(key);
            items.push(item);
        }
        extend_original_property_method_items(
            analysis,
            provider,
            side,
            partial,
            edit_context,
            &mut names,
            &mut items,
        );
    }
    items.sort_by(|left, right| left.label.cmp(&right.label));
    items
}

fn original_method_candidate_item(
    analysis: &AnalysisResult,
    bytes: &[u8],
    policy: tcl_syntax::naming::NamePolicyProtocol,
    label: String,
    spelling: String,
    edit_context: OriginalMethodEditContext<'_, '_>,
    detail: &str,
) -> Option<CompletionItem> {
    let text_edit = if let Some((source, cursor, line)) = edit_context.position {
        let (span, new_text) = if let Some(receiver) = edit_context.receiver {
            receiver.selector_replacement(source, cursor, bytes, policy)?
        } else {
            crate::original_oo::method_selector_replacement(
                analysis, source, cursor, bytes, policy,
            )?
        };
        let index = tcl_lexer::LineIndex::new(source);
        let start = index.position_at_utf16(span.start(), source);
        let end = index.position_at_utf16(span.end(), source);
        if start.line != line || end.line != line {
            return None;
        }
        Some(CompletionEdit {
            start_char: start.character.get(),
            end_char: end.character.get(),
            new_text,
        })
    } else {
        None
    };
    Some(CompletionItem {
        label,
        insert_text: spelling,
        text_edit,
        kind: CompletionKind::Function,
        detail: Some(detail.to_owned()),
        ..CompletionItem::default()
    })
}

fn method_completions(
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    class_q: &str,
    bucket: crate::definition::MethodBucket,
    partial: &str,
    object_state: Option<&tcl_compiler::analyser::ObjectMemberState>,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Vec<CompletionItem>> {
    let all = method_items(
        analysis,
        Some(registry),
        class_q,
        bucket,
        object_state,
        profile,
    )?;
    let FilteredCandidates {
        candidates: items,
        fuzzy,
    } = filter_candidates(partial, all, |item| item.label.as_str());
    (!fuzzy && !items.is_empty()).then_some(items)
}

/// Readonly method suggestions preserve source side, provider order and visibility.
/// Named source members require their current selected original name operand;
/// registry, generated and per-object suggestions retain separate producers.
fn method_items(
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    class_q: &str,
    bucket: crate::definition::MethodBucket,
    object_state: Option<&tcl_compiler::analyser::ObjectMemberState>,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Vec<CompletionItem>> {
    use crate::definition::MethodBucket;
    let object_members = if bucket == MethodBucket::Instance {
        match object_state {
            Some(state) => Some(analysis.logical_source_object_member_state(state)?),
            None => None,
        }
    } else {
        None
    };
    if !analysis.all_classes.contains_key(class_q) {
        // An external catalogue cannot supply the source object's own masks.
        if object_members.is_some() {
            return None;
        }
        return registry_method_items(analysis, registry?, class_q, profile);
    }
    let exports = object_members
        .as_ref()
        .into_iter()
        .flat_map(tcl_compiler::analyser::LogicalSourceObjectMembers::export_names)
        .collect::<FxHashSet<_>>();
    let unexports = object_members
        .as_ref()
        .into_iter()
        .flat_map(tcl_compiler::analyser::LogicalSourceObjectMembers::unexport_names)
        .collect::<FxHashSet<_>>();
    let hierarchy = analysis.class_hierarchy();
    let mro = hierarchy
        .mro_map
        .get(class_q)
        .cloned()
        .unwrap_or_else(|| vec![class_q.to_string()]);
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let mut items: Vec<CompletionItem> = Vec::new();
    // Per-object members first — `TclOO` layers an `oo::objdefine`d method
    // ahead of the class chain, so the receiver's own members claim their
    // names before any class provider.
    if bucket == MethodBucket::Instance
        && let (Some(state), Some(members)) = (object_state, object_members.as_ref())
    {
        layer_per_object_members(state, members, &mut seen, &mut items)?;
    }
    for cls in &mro {
        let Some(cd) = analysis.all_classes.get(cls) else {
            continue;
        };
        let bucketed: Vec<(&String, &tcl_compiler::analyser::MethodDef)> = match bucket {
            MethodBucket::Instance => cd.methods.iter().collect(),
            MethodBucket::Class => cd
                .class_methods
                .iter()
                .filter(|(_, m)| cls.as_str() == class_q || !m.is_self_method)
                .collect(),
        };
        // Per-object visibility flips override the declared state for this
        // receiver: an `oo::objdefine … export` revives an unexported class
        // member, an `… unexport` masks an exported one.
        let flipped = |name: &str, declared_public: bool| {
            if exports.contains(name) {
                true
            } else if unexports.contains(name) {
                false
            } else {
                declared_public
            }
        };
        let side = match bucket {
            MethodBucket::Instance => tcl_compiler::analyser::MemberSide::Instance,
            MethodBucket::Class => tcl_compiler::analyser::MemberSide::ClassObject,
        };
        let mut methods = bucketed
            .into_iter()
            .filter(|(name, method)| flipped(name, method.visibility == "public"))
            .filter_map(|(_, method)| analysis.logical_source_member_insertion(cd, side, method))
            .collect::<Vec<_>>();
        methods.sort_by(|left, right| left.name().cmp(right.name()));
        for method in methods {
            let name = method.name();
            let provider = cls.as_str();
            if !seen.insert(name.to_owned()) {
                continue;
            }
            let detail = if provider == class_q {
                format!("method — {class_q}")
            } else {
                format!("method — inherited from {provider}")
            };
            items.push(CompletionItem {
                label: name.to_owned(),
                insert_text: method.source_word().to_owned(),
                kind: CompletionKind::Function,
                detail: Some(detail),
                ..CompletionItem::default()
            });
        }
    }
    extend_generated_method_items(analysis, registry, class_q, bucket, &mut seen, &mut items);
    items.sort_by(|a, b| a.label.cmp(&b.label));
    Some(items)
}

/// Own names and visibility masks require the same current original source state.
fn layer_per_object_members(
    state: &tcl_compiler::analyser::ObjectMemberState,
    members: &tcl_compiler::analyser::LogicalSourceObjectMembers<'_>,
    seen: &mut FxHashSet<String>,
    items: &mut Vec<CompletionItem>,
) -> Option<()> {
    let mut methods = state.methods.values().collect::<Vec<_>>();
    methods.sort_by(|left, right| left.name.cmp(&right.name));
    for method in methods {
        let spelling = members.source_word(method)?;
        if method.name.contains('\0') {
            return None;
        }
        if !seen.insert(method.name.clone()) || method.visibility != "public" {
            continue;
        }
        items.push(CompletionItem {
            label: method.name.clone(),
            insert_text: spelling,
            kind: CompletionKind::Function,
            detail: Some("method — per-object (oo::objdefine)".to_owned()),
            ..CompletionItem::default()
        });
    }
    Some(())
}

/// Catalogue name data uses the actual retained source grammar, not a runtime name recipe.
fn source_method_name_word(analysis: &AnalysisResult, name: &str) -> Option<String> {
    let input = analysis.resolved_input.as_ref()?;
    if analysis.body_lexer_config != Some(input.lexer_config()) || name.contains('\0') {
        return None;
    }
    tcl_syntax::word_rules::lexical_unicode_source_word(name, input.lexer_config())
}

fn generated_property_method_items(
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    class_q: &str,
) -> Vec<CompletionItem> {
    let Some(registry) = registry else {
        return Vec::new();
    };
    if analysis
        .resolved_registry()
        .is_none_or(|actual| actual.snapshot().semantic_key() != registry.snapshot().semantic_key())
    {
        return Vec::new();
    }
    let Some(class) = analysis.all_classes.get(class_q) else {
        return Vec::new();
    };
    if analysis.logical_source_class_declaration(class).is_none() {
        return Vec::new();
    }
    let hierarchy = analysis.class_hierarchy();
    let providers = hierarchy
        .mro_map
        .get(class_q)
        .cloned()
        .unwrap_or_else(|| vec![class_q.to_owned()]);
    if providers
        .iter()
        .filter_map(|name| analysis.all_classes.get(name))
        .any(|provider| {
            analysis
                .logical_source_class_declaration(provider)
                .is_none()
        })
    {
        return Vec::new();
    }
    hierarchy
        .property_accessor_methods(Some(registry), class_q)
        .into_iter()
        .filter_map(|name| {
            let insert_text = source_method_name_word(analysis, name)?;
            Some(CompletionItem {
                label: name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: Some("method — generated property accessor".to_owned()),
                ..CompletionItem::default()
            })
        })
        .collect()
}

/// Every instance method/subcommand item for a *registry*-modelled class:
/// a tcllib-style factory (`report::report`, `struct::graph` — a distinct
/// `ObjectClassSpec`) or a Tk/ttk widget (self-referential — `class_q`
/// names its own `CommandSpec`, whose `subcommands` *is* its instance
/// dispatch table).  Tries the dedicated object class first since that is
/// the precise, intended shape; the self-referential widget case falls
/// back to the class's own `CommandSpec` when it has no separate
/// `ObjectClassSpec` (matching [`CommandRegistry::instance_methods`]). The
/// registry also applies the owning package's lifecycle floor before returning
/// a row, so Tk 9.1-only methods cannot leak into an older resolved profile.
/// Fragment matching happens in [`method_completions`].
fn registry_method_items(
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    class_q: &str,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Vec<CompletionItem>> {
    // naming.consumer.original-receiver-completion-domain
    // docs/design/analysis/name-resolution-proofs/original-receiver-completion-domain.md
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let floor = crate::document_floor::DocumentFloor::new(analysis, profile);
    let context = floor.context()?;
    let spec = context.resolve_spec(registry, class_q)?;
    let methods = CommandRegistry::instance_method_providers_for_descriptor_at(
        registry,
        spec,
        Some(context.authoring_query()),
        |owner| {
            context
                .resolve_spec(registry, owner.name)
                .filter(|selected| std::ptr::eq(*selected, owner))
                .map(|selected| floor.for_spec(selected))
        },
    );
    let mut items: Vec<CompletionItem> = methods
        .into_iter()
        .filter_map(|(_owner, method)| {
            let insert_text = source_method_name_word(analysis, method.name)?;
            Some(CompletionItem {
                label: method.name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: Some(format!("method — {class_q}")),
                documentation: (!method.detail.is_empty()).then(|| method.detail.to_owned()),
                ..CompletionItem::default()
            })
        })
        .collect();
    if items.is_empty() {
        return None;
    }
    items.sort_by(|a, b| a.label.cmp(&b.label));
    Some(items)
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

fn switch_partial_at_position(
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

fn switch_completions(
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
fn event_name_completions(partial: &str, analysis: &AnalysisResult) -> Vec<CompletionItem> {
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
fn subcommand_completions(
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

fn arg_value_completions_from_subcommands(
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

/// The scoped command environment active at the cursor, if the position falls
/// inside a recorded scoped body (a `report::defstyle` style script).
fn scoped_env_at(
    analysis: &AnalysisResult,
    source: &str,
    line: u32,
    character: u32,
    _line_index: &tcl_lexer::LineIndex,
) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
    let position =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)?;
    let offset = position.offset();
    if !analysis.allows_lexical_declaration_advice() {
        let body = analysis.original_scoped_body_in_source(
            &tcl_lexer::SourceImage::document(source),
            analysis.body_lexer_config?,
            offset,
        )?;
        return Some(body.environment());
    }
    let inventory = crate::source_structure::SourceSyntaxCoverage::capture(source, analysis)?;
    let scoped = inventory.logical_scoped_environment_at(source, analysis, offset)?;
    scoped.environment(&analysis.resolved_input.as_ref()?.context_registry())
}

/// Current authored vocabulary at a strict original source position.
/// Ordinary Tcl definition scripts keep their open command completion purpose.
/// A declaration document separately retains selected nested grammar, explicit
/// root authoring vocabulary/debt, ordinary script context or unavailable source.
fn definition_grammar_at_position(
    source: &str,
    line: u32,
    character: u32,
    _line_index: &tcl_lexer::LineIndex,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> crate::source_structure::SourceDefinitionGrammar {
    use crate::source_structure::{DefinitionGrammarUnavailable, SourceDefinitionGrammar};
    // Ordinary Tcl definition scripts remain open completion positions. This
    // consumer closes only explicit standalone authoring vocabularies.
    if registry.document_grammar().is_none() {
        return SourceDefinitionGrammar::OrdinaryScript;
    }
    let Some(position) =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)
    else {
        return SourceDefinitionGrammar::Unavailable(DefinitionGrammarUnavailable::CurrentSource);
    };
    if !std::ptr::eq(position.context().registry(), registry) {
        return SourceDefinitionGrammar::Unavailable(DefinitionGrammarUnavailable::CurrentSource);
    }
    crate::oo_body::definition_grammar_from_analysis(source, analysis, position.offset())
}

/// Completion items for the member words of a definition-body grammar — the
/// declarations legal at this position and nothing else.
///
/// A member's detail and documentation come from its own [`CommandSpec`] when
/// it has one (every `SslicTcl` and `SpecTcl` word does), so a grammar-driven
/// item is as informative as an ordinary command item; a member that is only a
/// grammar row still completes, with no detail.
fn grammar_member_completions(
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    partial: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ResolvedContext,
) -> Vec<CompletionItem> {
    // naming.consumer.selected-catalogue-completion-provider
    // docs/design/analysis/name-resolution-proofs/selected-catalogue-completion-provider.md
    let query = context.authoring_query();
    grammar
        .members
        .iter()
        .filter(|member| {
            (partial.is_empty() || member.keyword.starts_with(partial))
                && grammar
                    .source_member_in(member.keyword.as_bytes(), Some(query))
                    .is_some_and(|actual| std::ptr::eq(actual, *member))
        })
        .filter_map(|member| {
            let insert_text =
                tcl_syntax::word_rules::lexical_unicode_source_word(member.keyword, config)?;
            let spec = context.resolve_spec(registry, member.keyword);
            Some(CompletionItem {
                label: member.keyword.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: spec.map(|s| command_detail(s, registry)),
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: spec
                    .and_then(|s| s.hover.as_ref())
                    .map(|h| h.summary.to_owned()),
            })
        })
        .collect()
}

/// Completion items for the command heads of a scoped environment matching
/// `partial` (`top`, `data`, `columns`, … inside a `report::defstyle` body).
fn scoped_command_completions(
    env: &'static tcl_registry::scoped::ScopedCommandEnv,
    partial: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<CompletionItem> {
    env.commands
        .iter()
        .filter(|c| partial.is_empty() || c.name.starts_with(partial))
        .filter_map(|c| {
            let insert_text = tcl_syntax::word_rules::lexical_unicode_source_word(c.name, config)?;
            Some(CompletionItem {
                label: c.name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: (!c.detail.is_empty()).then(|| c.detail.to_owned()),
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            })
        })
        .collect()
}

/// Completion items for the ensemble operations of a scoped command
/// (`set` / `get` / `enable` / … after `top ` in a `report::defstyle` body).
fn scoped_op_completions(
    cmd: &'static tcl_registry::scoped::ScopedCommand,
    partial: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<CompletionItem> {
    let ops: Vec<&tcl_registry::SubCommand> = cmd.subcommands.iter().collect();
    let FilteredCandidates {
        candidates: ops,
        fuzzy,
    } = filter_candidates(partial, ops, |op| op.name);
    let mut items: Vec<CompletionItem> = ops
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

/// Build completions for the second-level subcommands of a two-level ensemble
/// (`info object <op>` / `info class <op>`), filtered by `partial`.  Each
/// item's detail is the operation's one-line description.
fn sub_subcommand_completions(
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
fn boolean_value_completions(partial: &str, config: tcl_lexer::LexerConfig) -> Vec<CompletionItem> {
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

fn arg_value_completions_from(
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

/// Map a usage count to its sort bucket (lower is better).
fn usage_bucket(count: usize) -> u8 {
    match count {
        c if c >= 50 => 0,
        c if c >= 20 => 1,
        c if c >= 8 => 2,
        c if c >= 3 => 3,
        c if c >= 1 => 4,
        _ => 5,
    }
}

/// Build a `HashMap<command_name, usage_count>` from the
/// analyser's per-document `command_invocations`.  Used as a
/// best-effort, document-local proxy for workspace-wide usage
/// counts.
fn document_usage_counts(analysis: &AnalysisResult) -> FxHashMap<String, usize> {
    let mut counts: FxHashMap<String, usize> = FxHashMap::default();
    for inv in &analysis.command_invocations {
        *counts.entry(inv.name.clone()).or_insert(0) += 1;
        if let Some(q) = inv.resolved_qualified_name.as_deref()
            && q != inv.name
        {
            *counts.entry(q.to_owned()).or_insert(0) += 1;
        }
    }
    counts
}

/// What kind of `TclOO` frame the completion cursor sits in — which decides
/// whether the `oo::Helpers` family's bare spellings may be offered.
///
/// Thin position-to-offset wrapper over the crate's single frame
/// classifier, so completion, hover, and the analyser's W123 emitter all
/// answer from the same scope walk.
fn oo_frame_at(
    analysis: &AnalysisResult,
    source: &str,
    line: u32,
    character: u32,
    line_index: &tcl_lexer::LineIndex,
) -> crate::oo_dispatch::OoFrame {
    let offset = crate::definition::byte_offset_at(line_index, source, line, character);
    crate::oo_dispatch::OoFrame::at(analysis, offset)
}

fn builtin_sort_text(name: &str, usage: usize) -> String {
    // `B<usage>_<name>` — built-ins sort after user procs (`A…`)
    // but before any item with no `sort_text`.  The two-digit
    // bucket prevents lexicographic confusion between 1 and 10.
    let rank = usage_bucket(usage);
    format!("B{rank:02}_{name}")
}

fn builtin_completions(
    registry: &CommandRegistry,
    dialect: &'static tcl_dialect::DialectProfile,
    partial: &str,
    usage: &FxHashMap<String, usize>,
    tk_advice: bool,
    analysis: &AnalysisResult,
    oo_frame: crate::oo_dispatch::OoFrame,
) -> Vec<CompletionItem> {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let config = input.lexer_config();
    if analysis.body_lexer_config != Some(config) {
        return Vec::new();
    }
    // Availability-gate the command list through the dialect profile: a
    // command whose spec restricts itself to later dialects (`try` is Tcl
    // 8.6+, `lseq` is 9.0+, …) must not be offered in an earlier-dialect
    // buffer, a vendor profile's composed mask admits its embedded Tcl core
    // (8.5 `dict` under f5-iapps), and the subtractive iRules disable list
    // filters the banned commands (§9 of the dialect-profile model).
    // `load_dialect` only *adds* dialect command sets (iRules / Tk / …); it
    // never removes a version-gated core command, so `command_names()`
    // still lists `try` under `tcl8.4`/`tcl8.5` — filter here via the same
    // profile resolution the analyser's W123 uses. An unknown dialect
    // (custom / non-Tcl) resolves to the permissive fallback environment.
    let profile = dialect;
    let Some(context) = crate::document_floor::DocumentFloor::new(analysis, profile).context()
    else {
        return Vec::new();
    };
    // naming.consumer.selected-catalogue-completion-provider
    // docs/design/analysis/name-resolution-proofs/selected-catalogue-completion-provider.md
    // Retain one descriptor from the actual document availability context.
    // A same-name row from another provider cannot decide any later filter.
    let mut candidates: Vec<_> = registry
        .command_names()
        .filter(|name| partial.is_empty() || name.starts_with(partial))
        .filter_map(|name| {
            context
                .resolve_spec(registry, name)
                .map(|spec| (name, spec))
        })
        .filter(|(_, spec)| {
            !spec.traits.contains(tcl_registry::Traits::OPERATOR_COMMAND)
                && !spec
                    .traits
                    .contains(tcl_registry::Traits::DEFINITION_BODY_MEMBER_ONLY)
        })
        .filter(|(_, spec)| oo_frame.admits_selected_spec(spec))
        .filter(|(_, spec)| {
            (tk_advice && context.can_host_package("Tk")) || spec.required_package != Some("Tk")
        })
        .filter(|(_, spec)| {
            let floor = package_version_floor(analysis, spec, profile);
            match (context.keyed_version_range(spec), floor) {
                (Some((min, max)), Some(floor)) => {
                    tcl_registry::version::within_range(floor, min, max)
                }
                _ => spec.available_for_version(floor),
            }
        })
        .collect();
    candidates.sort_unstable_by_key(|(name, _)| *name);
    candidates
        .into_iter()
        .filter_map(|(name, spec)| {
            let insert_text = tcl_syntax::word_rules::lexical_unicode_source_word(name, config)?;
            let count = usage.get(name).copied().unwrap_or(0);
            Some(CompletionItem {
                label: name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: Some(command_detail(spec, registry)),
                sort_text: Some(builtin_sort_text(name, count)),
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: spec.hover.as_ref().map(|hover| hover.summary.to_owned()),
            })
        })
        .collect()
}

/// `expr` math-function completions — the bare names
/// (`sin`, `max`, …) that are callable *only* inside an expression.
///
/// Sourced from the registry's own `::tcl::mathfunc::*` specs
/// ([`CommandRegistry::math_function_names`] / `math_function_spec`), which
/// are themselves derived from `tcl_syntax::expr::mathfunc` — one canonical
/// table, no second list here — and gated on the profile's `expr`-grammar
/// version, so `max` never appears under `tcl8.4` and `gamma` only under
/// `tcl9.1`.
///
/// Ranked in the local-symbol tier (`A000_…`, alongside the most-used local
/// procs and ahead of the `B…` built-ins) because at this position they are
/// the candidates that can actually be called: an ordinary command is
/// reachable inside an expression only through a `[…]` substitution, which is
/// a different cursor context.
fn math_function_completions(
    registry: &CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
    partial: &str,
) -> Vec<CompletionItem> {
    registry
        .math_function_names(profile)
        .into_iter()
        .filter(|name| partial.is_empty() || name.starts_with(partial))
        .map(|name| {
            let spec = registry.math_function_spec(name, profile);
            CompletionItem {
                label: name.to_owned(),
                insert_text: name.to_owned(),
                kind: CompletionKind::Function,
                detail: Some("expr math function".to_owned()),
                sort_text: Some(format!("A000_{name}")),
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: spec
                    .and_then(|s| s.hover.as_ref())
                    .map(|h| h.summary.to_owned()),
            }
        })
        .collect()
}

/// Completion-detail provenance string for a built-in command:
/// `tcllib (PKG)` / `stdlib (PKG)` / `Tk` / `built-in`.  Tcllib takes
/// precedence over a plain `required_package`; a package the profile
/// ships ambiently (an F5 surface — §7.1 axis C) is part of the runtime
/// and reads `built-in`, not like a require-gated stdlib package.
fn command_detail(
    spec: &tcl_registry::CommandSpec,
    registry: &tcl_registry::registry::CommandRegistry,
) -> String {
    if let Some(pkg) = spec.tcllib_package {
        format!("tcllib ({pkg})")
    } else if let Some(pkg) = spec.required_package
        && !registry.is_ambient_package(pkg)
    {
        if pkg == "Tk" {
            "Tk".to_string()
        } else {
            format!("stdlib ({pkg})")
        }
    } else {
        "built-in".to_string()
    }
}

/// Render a parameter-list summary for a proc completion's
/// `detail` field.
/// Returns `"(no args)"` for paramless procs, otherwise a
/// space-separated list with `{name default}` for optional
/// params.
fn proc_signature_str(proc_def: &ProcDef) -> String {
    if proc_def.params.is_empty() {
        return "(no args)".to_string();
    }
    proc_def
        .params
        .iter()
        .map(|p| {
            if p.has_default {
                let default = p.default_value.as_deref().unwrap_or("");
                format!("{{{} {}}}", p.name, default)
            } else {
                p.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn proc_sort_text(name: &str, usage: usize) -> String {
    // `A<tier><usage>_<name>` — `tier = 0` reserved for
    // single-document user procs; `tier = 1` differentiates
    // same-file procs from workspace ones.
    let rank = usage_bucket(usage);
    format!("A0{rank:02}_{name}")
}

fn proc_completions(
    analysis: &AnalysisResult,
    partial: &str,
    usage: &FxHashMap<String, usize>,
) -> Vec<CompletionItem> {
    // naming.source.original-procedure-publications
    // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
    let source_inventory = analysis
        .retained_command_realm()
        .and_then(|realm| realm.original_source_image())
        .and_then(|image| image.try_text().ok())
        .and_then(|source| {
            tcl_compiler::registry_invocation::source_structure::source_procedure_publications(
                source, analysis,
            )
        });
    let mut names = Vec::new();
    if let Some(world) = analysis.original_completed_command_world() {
        // Current publication is independent of canonical declaration metadata.
        // A move changes the candidate spelling; deletion removes it here.
        for publication in world.declarations() {
            for declaration in analysis.original_procedure_declarations() {
                if publication.declaration_site() == declaration.declaration_site()
                    && publication.policy() == declaration.name().policy()
                {
                    names.push((declaration, publication.slot(), publication.policy()));
                }
            }
        }
    } else if let Some(inventory) = source_inventory.as_ref() {
        names.extend(
            inventory
                .candidates(analysis.original_procedure_declarations())
                .into_iter()
                .map(|candidate| {
                    (
                        candidate.declaration(),
                        candidate.source_slot(),
                        candidate.policy(),
                    )
                }),
        );
    } else {
        // Genuine declaration cards remain independent of a represented graph.
        names.extend(
            analysis
                .original_procedure_declarations()
                .map(|declaration| {
                    (
                        declaration,
                        declaration.name().slot(),
                        declaration.name().policy(),
                    )
                }),
        );
    }
    let mut items = Vec::new();
    for (declaration, slot, policy) in names {
        push_native_proc_candidate(declaration, slot, policy, partial, usage, &mut items);
    }
    extend_authored_proc_items(analysis, partial, usage, &mut items);
    items.sort_unstable_by(|left, right| {
        left.label
            .cmp(&right.label)
            .then_with(|| left.insert_text.cmp(&right.insert_text))
    });
    items.dedup_by(|left, right| {
        left.insert_text == right.insert_text && left.detail == right.detail
    });
    items
}

/// Proc-name completions for an `INVOKES_USER_PROC` argument (iRules
/// `call PROC_NAME`), with the fuzzy fallback: this list is the whole
/// response for that cursor context, so when no proc prefix-matches a
/// two-plus-character fragment the closest proc names are offered
/// instead of an empty list.
fn invoked_proc_completions(analysis: &AnalysisResult, partial: &str) -> Vec<CompletionItem> {
    let usage = document_usage_counts(analysis);
    let items = proc_completions(analysis, partial, &usage);
    if !items.is_empty() || partial.is_empty() {
        return items;
    }
    let FilteredCandidates {
        candidates: mut fallback,
        fuzzy,
    } = filter_candidates(partial, proc_completions(analysis, "", &usage), |item| {
        item_match_name(item, partial)
    });
    if fuzzy {
        decorate_fuzzy_items(&mut fallback, partial);
    }
    fallback
}

/// Response-level fuzzy fallback for the command-position path: called
/// by [`completions`] only when procs, built-ins, scoped heads,
/// workspace procs, and snippets all produced nothing for the typed
/// fragment.  Re-enumerates the same candidate universe the prefix
/// path consults — `$obj` methods when the command head is a known
/// instance, then user procs, built-ins, and scoped command heads —
/// and offers the closest names via [`filter_candidates`].
fn fuzzy_command_fallback(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    dialect: &'static tcl_dialect::DialectProfile,
    line_index: &tcl_lexer::LineIndex,
) -> Vec<CompletionItem> {
    let Some(prefix) = word_prefix_at_position(source, line, character, analysis) else {
        return Vec::new();
    };
    let command_head = prefix.word_index() == 0;
    let Some(partial) = tcl_syntax::word_rules::original_word_cursor_literal_prefix(&prefix) else {
        return Vec::new();
    };
    if partial.chars().count() < MIN_FUZZY_FRAGMENT_CHARS {
        return Vec::new();
    }
    let usage = document_usage_counts(analysis);
    let mut universe: Vec<CompletionItem> = Vec::new();
    // Receiver-method context — the method universe the instance branch of
    // `context_aware_completions` declined to fuzzy-match (see
    // `method_completions`) joins the ranking here, resolved with the same
    // `$var`-vs-bareword gate that branch applies. This candidate universe
    // grants no method navigation or rename identity.
    let cursor = crate::definition::byte_offset_at(line_index, source, line, character);
    let original_context = crate::original_oo::class_completion_context(analysis, source, cursor);
    if let std::ops::ControlFlow::Break(Some(record)) = original_context {
        universe.extend(original_class_method_items(analysis, record, ""));
    } else if let Some(record) =
        crate::original_oo::instance_method_completion_record(analysis, source, cursor)
    {
        universe.extend(original_instance_method_candidate_items_at(
            analysis, record, source, cursor, "", None,
        ));
    }
    if analysis.allows_lexical_declaration_advice()
        && let Some(command) = command_context_on_line(source, line, character, analysis)
        && command.word_index == 1
        && let Some((recv, is_dollar)) = dispatch_receiver_of(&command.head)
        && let Some(registry) = registry
        && let Some((class_q, bucket)) = logical_completion_receiver_candidate(
            analysis,
            registry,
            &command.head,
            &recv,
            is_dollar,
        )
    {
        let object_state =
            crate::definition::object_member_state_at(analysis, source, &recv, line, character);
        let methods = method_items(
            analysis,
            Some(registry),
            class_q,
            bucket,
            object_state,
            dialect,
        );
        if let Some(methods) = methods {
            universe.extend(methods);
        }
    }
    if command_head {
        universe.extend(proc_completions(analysis, "", &usage));
    }
    if command_head && let Some(registry) = registry {
        // Both halves are placement facts — the
        // environment ships Tk ambient (`wish`), or this document required
        // it. No `SpecSurface` bit and no environment *name* is consulted.
        let tk_advice =
            crate::document_floor::DocumentFloor::new(analysis, dialect).advises_package("Tk");
        let oo_frame = oo_frame_at(analysis, source, line, character, line_index);
        universe.extend(builtin_completions(
            registry, dialect, "", &usage, tk_advice, analysis, oo_frame,
        ));
    }
    if command_head && let Some(env) = scoped_env_at(analysis, source, line, character, line_index)
    {
        let present: FxHashSet<String> = universe.iter().map(|i| i.label.clone()).collect();
        universe.extend(
            scoped_command_completions(env, "", prefix.config())
                .into_iter()
                .filter(|item| !present.contains(&item.label)),
        );
    }
    let FilteredCandidates {
        candidates: mut items,
        fuzzy,
    } = filter_candidates(&partial, universe, |item| item_match_name(item, &partial));
    if fuzzy {
        decorate_fuzzy_items(&mut items, &partial);
    }
    items
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

fn extend_original_own_method_items(
    analysis: &AnalysisResult,
    partial: &str,
    edit_context: OriginalMethodEditContext<'_, '_>,
    own: &[crate::original_oo::OwnObjectMethodCandidate<'_>],
    names: &mut Vec<(
        tcl_core_types::NameBytes,
        tcl_syntax::naming::NamePolicyProtocol,
    )>,
    items: &mut Vec<CompletionItem>,
) {
    for candidate in own {
        let input = &candidate.input;
        // A private own name blocks the identical class candidate, even when
        // its source reporting label or insertion spelling collides.
        names.push((
            tcl_core_types::NameBytes::from(input.bytes()),
            input.policy(),
        ));
        if !candidate.exported
            || !candidate
                .metadata
                .name_purpose()
                .admits(input.policy().recipe(), input.bytes())
        {
            continue;
        }
        let declaration = candidate.metadata.declaration().original_word();
        let Some(spelling) = tcl_syntax::backslash::native_literal_source_word(
            input.bytes(),
            declaration.image().channel(),
            declaration.config(),
            input.policy().string_protocol(),
        ) else {
            continue;
        };
        let spelling = insertable_source_word(spelling, input.bytes(), declaration.config());
        let label =
            std::str::from_utf8(input.bytes()).map_or_else(|_| spelling.clone(), str::to_owned);
        if !label.starts_with(partial) && !spelling.starts_with(partial) {
            continue;
        }
        if let Some(item) = original_method_candidate_item(
            analysis,
            input.bytes(),
            input.policy(),
            label,
            spelling,
            edit_context,
            "method — per-object declaration candidate",
        ) {
            items.push(item);
        }
    }
}

fn extend_generated_method_items(
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    class_q: &str,
    bucket: crate::definition::MethodBucket,
    seen: &mut FxHashSet<String>,
    items: &mut Vec<CompletionItem>,
) {
    use crate::definition::MethodBucket;
    if bucket == MethodBucket::Instance {
        for item in generated_property_method_items(analysis, registry, class_q) {
            if seen.insert(item.label.clone()) {
                items.push(item);
            }
        }
    }
    // The universal object method (present on every instance, never on a
    // class command).
    if bucket == MethodBucket::Instance && seen.insert("destroy".to_string()) {
        items.push(CompletionItem {
            label: "destroy".to_string(),
            insert_text: "destroy".to_string(),
            kind: CompletionKind::Function,
            detail: Some("method — oo::object builtin".to_string()),
            ..CompletionItem::default()
        });
    }
}

fn extend_authored_proc_items(
    analysis: &AnalysisResult,
    partial: &str,
    usage: &FxHashMap<String, usize>,
    items: &mut Vec<CompletionItem>,
) {
    if let Some(config) = analysis.body_lexer_config {
        for declaration in analysis.original_vendor_procedure_declarations() {
            let input = declaration.name_input();
            let Some(spelling) =
                crate::vendor_declaration::source_word(input, declaration.purpose(), config)
            else {
                continue;
            };
            let Some(label) = crate::vendor_declaration::source_label(input, declaration.purpose())
            else {
                continue;
            };
            if !label.starts_with(partial) && !spelling.starts_with(partial) {
                continue;
            }
            let count = usage.get(&label).copied().unwrap_or(0);
            items.push(CompletionItem {
                label: label.clone(),
                insert_text: spelling,
                kind: CompletionKind::Function,
                detail: Some(format!(
                    "Source declaration: {}",
                    proc_signature_str(declaration.metadata())
                )),
                sort_text: Some(proc_sort_text(&label, count)),
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            });
        }
    }
    if analysis.allows_lexical_declaration_advice()
        && let Some(config) = analysis.body_lexer_config
    {
        // Original Logical headers supply insertion data without Native publications.
        for proc_def in analysis
            .all_procs
            .values()
            .filter(|proc_def| proc_def.source_name.is_none())
        {
            let Some(declaration) = analysis.logical_source_procedure_declaration(proc_def) else {
                continue;
            };
            let Some(name) = declaration.name() else {
                continue;
            };
            let Some(spelling) = declaration.source_word(config) else {
                continue;
            };
            if !name.starts_with(partial) && !spelling.starts_with(partial) {
                continue;
            }
            let count = usage.get(name).copied().unwrap_or(0);
            items.push(CompletionItem {
                label: name.to_owned(),
                insert_text: spelling,
                kind: CompletionKind::Function,
                detail: Some(proc_signature_str(declaration.metadata())),
                sort_text: Some(proc_sort_text(name, count)),
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            });
        }
    }
}

fn extend_builtin_completion_items(
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    position: (&str, u32, u32, &tcl_lexer::LineIndex),
    partial: &str,
    usage: &FxHashMap<String, usize>,
    items: &mut Vec<CompletionItem>,
) {
    let (source, line, character, line_index) = position;
    if let Some(registry) = registry {
        // Tk commands are dialect-gated to Tcl/`tk` already, but they are also
        // offered as package advice from ambient placement — the `tk` dialect (a
        // `wish` document) or a `package require Tk` in this file.  Without
        // that, a plain `.tcl` script must not be offered `button`/`pack`/… .
        // Both halves are placement facts — the
        // environment ships Tk ambient (`wish`), or this document required
        // it. No `SpecSurface` bit and no environment *name* is consulted.
        let tk_advice =
            crate::document_floor::DocumentFloor::new(analysis, profile).advises_package("Tk");
        let oo_frame = oo_frame_at(analysis, source, line, character, line_index);
        items.extend(builtin_completions(
            registry, profile, partial, usage, tk_advice, analysis, oo_frame,
        ));
    }
}

fn extend_scoped_completion_items(
    analysis: &AnalysisResult,
    position: (&str, u32, u32, &tcl_lexer::LineIndex),
    partial: &str,
    current: &crate::original_context::CurrentSourceContext<'_>,
    items: &mut Vec<CompletionItem>,
) {
    let (source, line, character, line_index) = position;
    if let Some(env) = scoped_env_at(analysis, source, line, character, line_index) {
        let present: FxHashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        for item in scoped_command_completions(env, partial, current.config()) {
            if !present.contains(&item.label) {
                items.push(item);
            }
        }
    }
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

fn definition_grammar_completion_items(
    source: &str,
    position: (u32, u32, &tcl_lexer::LineIndex),
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    current: &crate::original_context::CurrentSourceContext<'_>,
    partial: &str,
    prefix: &tcl_lexer::word_parts::WordCursorPrefix,
) -> Option<Vec<CompletionItem>> {
    let (line, character, line_index) = position;
    if let Some(registry) = registry {
        let vocabulary =
            definition_grammar_at_position(source, line, character, line_index, analysis, registry);
        if crate::source_structure::SourceDefinitionGrammar::unavailable(&vocabulary).is_some() {
            return Some(Vec::new());
        }
        // A standalone root authoring projection may retain unavailable child
        // traversal. It is not a selected nested body or an edit vocabulary.
        if let Some(grammar) = crate::source_structure::SourceDefinitionGrammar::for_authoring(
            &vocabulary,
            source,
            analysis,
        ) {
            return Some(original_word_completion_edits(
                source,
                line,
                prefix,
                grammar_member_completions(
                    grammar,
                    partial,
                    registry,
                    current.config(),
                    current.context().context(),
                ),
            ));
        }
    }

    None
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

fn extend_original_property_method_items(
    analysis: &AnalysisResult,
    provider: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    side: tcl_compiler::analyser::types::MemberSide,
    partial: &str,
    edit_context: OriginalMethodEditContext<'_, '_>,
    names: &mut Vec<(
        tcl_core_types::NameBytes,
        tcl_syntax::naming::NamePolicyProtocol,
    )>,
    items: &mut Vec<CompletionItem>,
) {
    use tcl_compiler::analyser::types::MemberSide;
    if side == MemberSide::Instance
        && let Some(properties) = provider
            .metadata()
            .original_properties
            .properties_for_analysis(side, analysis)
    {
        for property in properties {
            let key = property.declaration().name_input();
            for name in property.accessor_methods() {
                // This is selected Registry-authored metadata, separate
                // from the property's counted name and native table state.
                let identity = (
                    tcl_core_types::NameBytes::from(name.as_bytes()),
                    key.policy(),
                );
                if names.contains(&identity) || !name.starts_with(partial) {
                    continue;
                }
                let Some(spelling) = tcl_syntax::backslash::native_literal_source_word(
                    name.as_bytes(),
                    key.source_image().channel(),
                    key.lexer_config(),
                    key.policy().string_protocol(),
                ) else {
                    continue;
                };
                let spelling =
                    insertable_source_word(spelling, name.as_bytes(), key.lexer_config());
                let Some(item) = original_method_candidate_item(
                    analysis,
                    name.as_bytes(),
                    key.policy(),
                    (*name).to_owned(),
                    spelling,
                    edit_context,
                    "method — property declaration candidate",
                ) else {
                    continue;
                };
                names.push(identity);
                items.push(item);
            }
        }
    }
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

fn push_native_proc_candidate(
    declaration: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ProcDef,
    >,
    slot: &tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    partial: &str,
    usage: &FxHashMap<String, usize>,
    items: &mut Vec<CompletionItem>,
) {
    let proc_def = declaration.metadata();
    let original = declaration.name_input();
    let Some(spelling) = tcl_syntax::naming::native_command_source_word(
        policy.recipe(),
        slot,
        original.source_image().channel(),
        original.lexer_config(),
    ) else {
        return;
    };
    let spelling = insertable_source_word(
        spelling,
        &tcl_syntax::naming::native_command_full_name_bytes(slot),
        original.lexer_config(),
    );
    let label = if slot.simple == declaration.name().slot().simple && original.display().is_some() {
        proc_def.name.clone()
    } else {
        slot.simple
            .try_utf8()
            .map_or_else(|_| spelling.clone(), str::to_owned)
    };
    let report = String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(slot)).ok();
    if !label.starts_with(partial)
        && !report
            .as_ref()
            .is_some_and(|name| name.starts_with(partial))
        && !spelling.starts_with(partial)
    {
        return;
    }
    let count = usage
        .get(&label)
        .copied()
        .max(report.as_ref().and_then(|name| usage.get(name)).copied())
        .unwrap_or(0);
    items.push(CompletionItem {
        label: label.clone(),
        insert_text: spelling,
        kind: CompletionKind::Function,
        detail: Some(proc_signature_str(proc_def)),
        sort_text: Some(proc_sort_text(&label, count)),
        is_snippet: false,
        filter_text: None,
        text_edit: None,
        documentation: None,
    });
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
