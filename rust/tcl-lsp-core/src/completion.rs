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

//! Completion provider.
//!
//! Wires the LSP completion surface for the three Tcl
//! completion contexts the analyser surfaces today:
//!
//! * **Variable completion** — when the cursor sits immediately
//!   after `$` (or inside a `${name}` braced reference), suggest
//!   every variable name visible in the global scope.
//! * **Proc-name completion** — at any other cursor position,
//!   suggest every user-defined `proc` name that the analyser
//!   recorded for the document.
//! * **Built-in command completion** — at the same cursor
//!   contexts as proc-name completion, also suggest every
//!   command registered in the caller-provided
//!   [`tcl_registry::CommandRegistry`].  The caller (server)
//!   threads its already-built per-dialect registry through.
//!   When no registry is provided, the completion surface
//!   degrades cleanly to the proc-only set.
//!
//! Subcommand-scoped argument-value completion: when a
//! subcommand declares `arg_values` for a positional slot
//! (e.g. `string is <class>`), the matching character classes
//! complete at that argument with `EnumValue` kind.
//!
//! Workspace-wide proc enumeration: when the caller threads a
//! [`crate::workspace_index::WorkspaceIndex`], procs defined in
//! *other* analysed documents surface in the command/proc-completion
//! fallback (deduped by label against the local set, sorted after
//! local procs / built-ins via a `C0_…` sort key, detail tagged
//! `(workspace)`).
//!
//! Trait-driven dialect arg rules: any command carrying
//! `Traits::IS_EVENT_HANDLER` triggers event-name completion
//! at word-index 1 (iRules `when EVENT`), and any command
//! carrying `Traits::INVOKES_USER_PROC` surfaces user-defined
//! proc names at word-index 1 (iRules `call PROC_NAME`).
//!
//! **Fuzzy fallback.**  Candidate filtering goes through one shared
//! helper ([`filter_candidates`]): prefix matches keep the historical
//! behaviour byte-for-byte, and when a typed fragment of two-plus
//! characters prefix-matches nothing, the closest candidates by edit
//! distance (budget and ranking shared with the analyser's "did you
//! mean…?" emitters via `tcl_compiler::text`) are offered instead —
//! capped, ranked by `(distance, name)`, and decorated
//! ([`decorate_fuzzy_items`]) so editors neither hide nor re-order
//! them.  Context-scoped lists (switches, subcommands, argument
//! values, events, scoped ops, `call` procs, variables, array
//! elements) fall back per list; the command-position response falls
//! back as a whole ([`fuzzy_command_fallback`]) only when procs,
//! built-ins, scoped heads, workspace procs, and snippets all
//! produced nothing, so a fragment that matches anything today keeps
//! today's response exactly.
//!
//! Cache + `spawn_blocking` + cached-analysis read-out ride on
//! top of this provider in `tcl-lsp-server::Backend::completion`;
//! this module is the pure-CPU computation, no I/O, no async.

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::{AnalysisResult, ProcDef, Scope};
use tcl_registry::{CommandRegistry, ProfileQueries};

use crate::definition::utf16_col_to_char_col;

/// LSP completion-item kind for our surface.  Keep narrow —
/// extend as richer completion is added.
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
/// The subset of the LSP `CompletionItem` fields we
/// emit today: label, insert-text, kind, and an optional
/// detail line.  The detail is what the editor shows in the
/// right-hand column of the completion list (typically a
/// parameter-list summary for procs).
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
/// input order: byte-identical to the per-site `starts_with` filters
/// it replaced.
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

/// Candidate name of an assembled completion item for fragment
/// matching: a `::`-led fragment matches the inserted (qualified)
/// name, anything else matches the label.  Procs label their simple
/// name but insert the qualified one; for every other item kind the
/// two agree, so this reproduces the historical
/// name-or-qualified-name prefix test.
fn item_match_name<'a>(item: &'a CompletionItem, partial: &str) -> &'a str {
    if partial.starts_with(':') {
        &item.insert_text
    } else {
        &item.label
    }
}

/// `true` when `$name` lexes as a single bare variable token (so it
/// needs no `${…}` braces). Delegates to the shared, ASCII-correct rule
/// (`TclIsBareword` accepts only ASCII alphanumerics — a Unicode-permissive
/// copy here would offer brace-free completions that change which variable
/// is read).
fn is_bare_var_name(name: &str) -> bool {
    tcl_syntax::naming::is_bare_var_name(name)
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

/// Cursor context threaded into [`switch_completion_items`] — bundled so
/// the helper stays under the argument-count budget.
#[derive(Clone, Copy)]
struct SwitchCompletionCtx<'a> {
    spec: &'a tcl_registry::CommandSpec,
    source: &'a str,
    line: u32,
    character: u32,
    word_idx: usize,
    analysis: &'a AnalysisResult,
    profile: &'static tcl_dialect::DialectProfile,
}

/// Option-flag completion for a `-<cursor>` position: resolves the
/// subcommand-scoped option table (`chan configure -<cursor>`) before
/// falling back to the command's own top-level table — an ensemble's real
/// options live on the subcommand (`SubCommand::options`), and only that
/// table is dialect-correct for a subcommand-specific option (e.g. `chan
/// configure -inputmode`, 9.0+, absent from `chan`'s own top-level table).
/// Mirrors the sub-arg-value resolution in [`context_aware_completions`].
/// Returns `None` when the resolved table has no options at all (so the
/// caller falls through to the next completion context).
fn switch_completion_items(
    ctx: &SwitchCompletionCtx<'_>,
    switch_partial: &str,
) -> Option<Vec<CompletionItem>> {
    let SwitchCompletionCtx {
        spec,
        source,
        line,
        character,
        word_idx,
        analysis,
        profile,
    } = *ctx;
    let sub = (word_idx >= 2)
        .then(|| nth_word_on_line(source, line, 1))
        .flatten()
        .and_then(|sub_name| {
            spec.resolve_subcommand_for_dialect(&sub_name, profile.availability_mask)
        });
    let (options, parent_dialects) = match sub {
        Some(sub) => (sub.options, sub.dialects.or(spec.dialects)),
        None => (spec.options, spec.dialects),
    };
    if options.is_empty() {
        return None;
    }
    // Replacement range spans the `-partial` already typed (dash column →
    // cursor) so the dash isn't duplicated.
    let line_text = source.split('\n').nth(line as usize).unwrap_or("");
    let cursor_col = utf16_col_to_char_col(line_text, character).min(line_text.chars().count());
    let dash_col = cursor_col.saturating_sub(switch_partial.chars().count());
    let edit = (
        char_col_to_utf16(line_text, dash_col),
        char_col_to_utf16(line_text, cursor_col),
    );
    let floor = package_version_floor(analysis, spec, profile);
    Some(switch_completions(
        options,
        profile,
        parent_dialects,
        switch_partial,
        edit,
        floor,
    ))
}

/// Registry-driven, context-aware completion: switch / event-name /
/// user-proc / subcommand / arg-value suggestions resolved from the
/// surrounding command's [`CommandRegistry`] spec.  Returns `None` when the
/// cursor isn't inside a recognised command context (so the caller falls
/// through to plain command + proc completion).
fn context_aware_completions(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    partial: &str,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Vec<CompletionItem>> {
    let (cmd, word_idx) = command_context_on_line(source, line, character)?;

    // `$obj <method>` / `.w <subcommand>` — when the command head is an
    // instance variable *or* a bareword instance-command name (a Tk widget
    // path, or `CLASS create NAME`) whose class is known, complete the
    // methods/subcommands callable on it.  Checked before the registry
    // lookup because neither `$obj` nor a bareword widget path is itself a
    // registered command.  `receiver_instance_class` applies the same
    // bareword-vs-`$var` gate go-to-definition/hover already use — a bare
    // name only resolves when it was actually bound by a create call
    // (`created_instance_commands`), not merely because some unrelated
    // variable of the same name happens to hold an object elsewhere
    // (issue #927).
    if word_idx == 1 {
        let receiver = strip_instance_var(&cmd).map(|v| (v, true)).or_else(|| {
            (!cmd.is_empty() && !cmd.contains(['$', '['])).then(|| (cmd.clone(), false))
        });
        if let Some((recv, is_dollar)) = receiver
            && let Some(class_q) =
                crate::definition::receiver_instance_class(analysis, &recv, is_dollar)
            && let Some(items) = method_completions(analysis, registry, class_q, partial)
        {
            return Some(items);
        }
    }

    // Inside a scoped command environment (a `report::defstyle` style script):
    // the ensemble operations of a scoped command (`top ` → `set`/`get`/`enable`
    // /…) complete at word index 1.  Checked before the registry lookup because
    // a scoped head (`top`) is not a registered command.
    if word_idx == 1
        && let Some(env) = scoped_env_at(analysis, source, line, character)
        && let Some(scoped) = env.command(&cmd)
        && !scoped.subcommands.is_empty()
    {
        return Some(scoped_op_completions(scoped, partial));
    }

    let spec = registry.get(&cmd)?;

    // Switch completion fires when the identifier
    // partial is preceded by a literal `-` on the
    // line.  `word_partial_at_position` stops at the
    // dash (it's not an identifier char), so detect
    // the dash here and rebuild the switch partial.
    if let Some(switch_partial) = switch_partial_at_position(source, line, character, partial)
        && let Some(items) = switch_completion_items(
            &SwitchCompletionCtx {
                spec,
                source,
                line,
                character,
                word_idx,
                analysis,
                profile,
            },
            &switch_partial,
        )
    {
        return Some(items);
    }
    // iRules `when EVENT { body }`: when the cursor is
    // typing the first argument of an event-handler
    // command, enumerate the known event names from the
    // shared event registry.
    if word_idx == 1 && spec.traits.contains(tcl_registry::Traits::IS_EVENT_HANDLER) {
        return Some(event_name_completions(partial));
    }
    // iRules `call PROC_NAME ?ARGS?`: when the cursor
    // is typing the first argument of an
    // `INVOKES_USER_PROC` command (today only `call`
    // in iRules), surface user-defined proc names —
    // and only those, not built-in commands.
    if word_idx == 1
        && spec
            .traits
            .contains(tcl_registry::Traits::INVOKES_USER_PROC)
    {
        return Some(invoked_proc_completions(analysis, partial));
    }
    if word_idx == 1 && !spec.subcommands.is_empty() {
        return Some(subcommand_completions(spec, profile, partial));
    }
    // Second-level subcommand completion — the word after a two-level
    // ensemble's first-level subcommand (`info object <op>`, `info class <op>`,
    // issue #798).  The first-level word is at index 1; offer its declared
    // `sub_subcommands` at index 2.
    if word_idx == 2
        && let Some(sub_name) = nth_word_on_line(source, line, 1)
        && let Some(sub) = spec.resolve_subcommand(&sub_name)
        && !sub.sub_subcommands.is_empty()
    {
        return Some(sub_subcommand_completions(sub, partial));
    }
    // Subcommand argument-value completion — e.g.
    // `string is <class>`.  When the cursor is at
    // word-index ≥ 2 of a command whose subcommand
    // (the word at index 1) declares enumerable
    // values for that sub-arg position, list them.
    if word_idx >= 2
        && let Some(sub_name) = nth_word_on_line(source, line, 1)
        && let Some(sub) = spec.resolve_subcommand(&sub_name)
    {
        let sub_arg_idx = u8::try_from(word_idx - 2).unwrap_or(u8::MAX);
        let values = sub.arg_values_at(sub_arg_idx);
        if !values.is_empty() {
            return Some(arg_value_completions(values, partial));
        }
    }
    // Option-value completion — when the word immediately before the cursor is
    // a value-taking option that declares an enumerable value set, offer those
    // values (e.g. `button .b -relief <cursor>` → flat|raised|…).  Matches by
    // name or alias; arity-`One` covered (the value follows the switch).
    if word_idx >= 2
        && let Some(prev) = nth_word_on_line(source, line, word_idx - 1)
        && prev.starts_with('-')
        && let Some(opt) = spec
            .options
            .iter()
            .chain(spec.command_forms.iter().flat_map(|f| f.options.iter()))
            .find(|o| o.matches(prev.as_str()))
    {
        let values = opt.value_values();
        if !values.is_empty() {
            return Some(arg_value_completions(values, partial));
        }
    }
    // Command-level positional arg-value completion — the
    // bareword value sets declared directly on the command
    // (not a subcommand).  Covers iRules `when EVENT timing
    // enable|disable` and `HTTP::respond <status>
    // content|noserver|version`.  The argument index is the
    // 0-based position after the command name (`word_idx - 1`).
    if word_idx >= 1 {
        let arg_idx = u8::try_from(word_idx - 1).unwrap_or(u8::MAX);
        let mut values = spec.arg_values_at(arg_idx);
        // `when`'s keyword tail carries an enumerable value
        // slot only after the `timing` keyword (the `priority`
        // keyword takes a numeric argument).  Even-index
        // value slots are gated on the preceding literal being
        // `timing`.
        if spec.traits.contains(tcl_registry::Traits::IS_EVENT_HANDLER)
            && arg_idx >= 2
            && nth_word_on_line(source, line, word_idx - 1).as_deref() != Some("timing")
        {
            values = &[];
        }
        if !values.is_empty() {
            return Some(arg_value_completions(values, partial));
        }
    }
    None
}

/// Compute completions for a position in `source`.
///
/// `analysis` is the pre-computed analyser result; the caller
/// (server) is expected to cache it.  `registry`, when `Some`,
/// extends proc-name completion with built-in command names —
/// every command registered in the caller's dialect-aware
/// registry surfaces at the same cursor contexts.  Returns an
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
    registry: Option<&CommandRegistry>,
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
    dialect: &str,
) -> Vec<CompletionItem> {
    if let Some((trigger, partial)) = variable_trigger(source, line, character) {
        return variable_completions(
            source,
            line,
            character,
            &analysis.global_scope,
            &partial,
            trigger,
            analysis.ns_var_global_fallback(),
        );
    }
    let partial = word_partial_at_position(source, line, character);

    // Context-aware completions — switch + subcommand + event-name.
    // All three require the caller-provided registry to look up
    // the surrounding command's spec.  Without a registry we
    // can't tell which switches / subcommands / events are valid,
    // so fall through to plain command + proc completion.
    if let Some(registry) = registry
        && let Some(items) = context_aware_completions(
            source,
            line,
            character,
            analysis,
            registry,
            &partial,
            tcl_dialect::DialectProfile::by_name(dialect),
        )
    {
        return items;
    }

    let usage = document_usage_counts(analysis);
    let mut items = proc_completions(analysis, &partial, &usage);
    if let Some(registry) = registry {
        // Tk commands are dialect-gated to Tcl/`tk` already, but they are also
        // only *present* once the Tk package is loaded — the `tk` dialect (a
        // `wish` document) or a `package require Tk` in this file.  Without
        // that, a plain `.tcl` script must not be offered `button`/`pack`/… .
        let tk_loaded =
            dialect == "tk" || analysis.package_requires.iter().any(|req| req.name == "Tk");
        items.extend(builtin_completions(
            registry, dialect, &partial, &usage, tk_loaded, analysis,
        ));
    }
    // Inside a scoped command environment (a `report::defstyle` style script),
    // offer its command heads (`top`, `data`, `columns`, …) alongside the
    // ordinary command/proc set — a style body uses both scoped and core
    // commands.  Deduped by label so a scoped name never doubles a core one.
    if let Some(env) = scoped_env_at(analysis, source, line, character) {
        let present: FxHashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        for item in scoped_command_completions(env, &partial) {
            if !present.contains(&item.label) {
                items.push(item);
            }
        }
    }
    // Workspace-wide proc enumeration: surface procs defined in
    // *other* analysed documents that aren't already in the
    // result.  Deduped by label against the current set so the
    // current document's procs (already present above) and
    // any same-named workspace proc don't double up.
    if let Some(index) = workspace {
        let present: FxHashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        let mut ws: Vec<&crate::workspace_index::WorkspaceProc> =
            index.procs_matching(&partial, "");
        // Stable, name-sorted order so cross-doc results don't
        // jitter between requests.
        ws.sort_unstable_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
        let mut seen_ws: FxHashSet<String> = FxHashSet::default();
        for proc in ws {
            if present.contains(&proc.name) || !seen_ws.insert(proc.name.clone()) {
                continue;
            }
            items.push(CompletionItem {
                label: proc.name.clone(),
                insert_text: proc.qualified_name.clone(),
                kind: CompletionKind::Function,
                detail: Some(workspace_proc_detail(proc)),
                // Sort cross-document procs after local procs
                // (`A…`) and built-ins (`B…`): `C0_<name>`.
                sort_text: Some(format!("C0_{}", proc.name)),
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            });
        }
    }

    // Context-aware snippet templates (`tcl-proc`, `tcl-if`,
    // …).  Only the command-position fallback reaches here (the
    // variable / switch / subcommand contexts returned earlier), so
    // snippets never pollute `$var` or `-option` completion.  Their
    // `Z0_…` sort key keeps them below real symbols, and the prefix
    // filter means they only surface once the user types `tcl-…`.
    let scope_vars: Vec<String> = analysis.global_scope.variables.keys().cloned().collect();
    let snippet_partial = snippet_partial_at_position(source, line, character);
    // `current_event` / `file_events` drive the iRules event templates'
    // top-level guard and duplicate-event decline.  Only `f5-irules`
    // carries those templates, so skip the segmentation otherwise.
    let (current_event, file_events) = if dialect == "f5-irules" {
        (
            crate::irules_context::find_enclosing_when_event(source, line, dialect),
            crate::irules_context::scan_file_events(source, dialect),
        )
    } else {
        (None, Vec::new())
    };
    items.extend(crate::snippets::snippet_completions(
        &crate::snippets::SnippetContext {
            dialect,
            indent_unit: "    ",
            scope_vars: &scope_vars,
            partial: &snippet_partial,
            current_event: current_event.as_deref(),
            file_events: &file_events,
        },
    ));
    // Response-level fuzzy fallback — only when nothing at all matched
    // the fragment, so any fragment that matches something today keeps
    // today's response byte-for-byte.
    if items.is_empty() {
        items = fuzzy_command_fallback(source, line, character, analysis, registry, dialect);
    }
    items
}

/// Detail line for a workspace (cross-document) proc
/// completion — a param-count summary plus a marker so the
/// user can tell it comes from another file.
fn workspace_proc_detail(proc: &crate::workspace_index::WorkspaceProc) -> String {
    let params = if proc.param_count == 1 {
        "1 param".to_string()
    } else {
        format!("{} params", proc.param_count)
    };
    format!("{params} (workspace)")
}

/// Variable-trigger detection — `$prefix` or `${prefix}`.
///
/// Returns `Some((trigger_kind, partial))` where `trigger_kind`
/// is `'$'` for plain `$` triggers and `'{'` for `${name}`
/// braced triggers.  Returns `None` when the cursor is not in
/// a variable-completion context.
///
/// Walks left from the cursor over identifier-continuation
/// characters (alphanumeric, `_`, `:`), then checks whether the
/// character preceding that run forms a recognised trigger:
/// `$` for plain triggers, `${` for braced triggers.
fn variable_trigger(source: &str, line: u32, character: u32) -> Option<(char, String)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());

    // Run of identifier-continuation chars immediately before the cursor —
    // that is the partial.  `(` is included so an array reference (`$arr(na`)
    // is captured whole; the array branch in `variable_completions` then
    // splits on it.
    let mut start = col;
    while start > 0
        && (chars[start - 1].is_alphanumeric()
            || chars[start - 1] == '_'
            || chars[start - 1] == ':'
            || chars[start - 1] == '(')
    {
        start -= 1;
    }
    let partial: String = chars[start..col].iter().collect();

    // What sits immediately before the partial run?
    if start == 0 {
        return None;
    }
    if chars[start - 1] == '$' {
        return Some(('$', partial));
    }
    if start >= 2 && chars[start - 2] == '$' && chars[start - 1] == '{' {
        return Some(('{', partial));
    }
    None
}

/// Word-partial extraction for proc-name completion.
///
/// Returns the run of identifier-like chars immediately to the
/// left of the cursor (the "partial" the user has typed so far).
fn word_partial_at_position(source: &str, line: u32, character: u32) -> String {
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

/// Like [`word_partial_at_position`] but also treats `-` as a word
/// character, so a `tcl-…` snippet prefix is captured whole.
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

/// `true` when offering `$<name>` / `${<name>}` would round-trip back to
/// the runtime variable — i.e. the raw scope key carries no `}` / `\` /
/// newline that the brace parser couldn't reproduce.  The full
/// backslash analysis isn't needed: a name with a `\` or `}` is dropped
/// from the suggestion set, which is what the
/// `omits_unsubstitutable_brace_names` case requires.
fn var_is_substitutable(name: &str) -> bool {
    !name.contains('}') && !name.contains('\\') && !name.contains('\n')
}

/// `::`-qualify a global variable name reached from a local (proc / nested
/// namespace) context; leave already-qualified names and locals untouched.
/// `qualify_globals` is the set of root-scope names that need it (`None` at
/// global scope, where bare names are correct).
fn qualified_var_name(name: &str, qualify_globals: Option<&FxHashSet<String>>) -> String {
    match qualify_globals {
        Some(globals) if globals.contains(name) && !name.starts_with("::") => format!("::{name}"),
        _ => name.to_owned(),
    }
}

fn variable_completions(
    source: &str,
    line: u32,
    character: u32,
    scope: &Scope,
    partial: &str,
    trigger: char,
    ns_global_fallback: bool,
) -> Vec<CompletionItem> {
    let line_text = source.split('\n').nth(line as usize).unwrap_or("");
    let chars: Vec<char> = line_text.chars().collect();
    let line_len = chars.len();
    let cursor_col = utf16_col_to_char_col(line_text, character).min(line_len);

    // Locate the `$` that opens this reference (scan left from the cursor).
    let dollar = chars[..cursor_col].iter().rposition(|&c| c == '$');
    let has_open_brace =
        trigger == '{' || dollar.is_some_and(|d| d + 1 < line_len && chars[d + 1] == '{');

    // Scan forward to the end of the existing reference so the edit replaces
    // the whole token (brace form tracks `{}` depth and
    // `\X` pairs; bare form takes alnum / `_` / `::`).
    let mut end = cursor_col;
    if has_open_brace {
        let mut depth: i32 = 0;
        while end < line_len {
            match chars[end] {
                '}' if depth == 0 => {
                    end += 1;
                    break;
                }
                '{' => {
                    depth += 1;
                    end += 1;
                }
                '}' => {
                    depth -= 1;
                    end += 1;
                }
                '\\' => {
                    end += 1;
                    if end < line_len {
                        end += 1;
                    }
                }
                _ => end += 1,
            }
        }
    } else {
        while end < line_len {
            let ch = chars[end];
            if ch.is_alphanumeric() || ch == '_' {
                end += 1;
            } else if ch == ':' && end + 1 < line_len && chars[end + 1] == ':' {
                end += 2;
            } else {
                break;
            }
        }
    }
    let edit_start = dollar.map(|d| char_col_to_utf16(line_text, d));
    let edit_end = char_col_to_utf16(line_text, end);

    let line_index = tcl_lexer::LineIndex::new(source);
    let byte_offset = crate::definition::byte_offset_at(&line_index, source, line, character);

    // Array-element completion: `$arr(` / `$arr(prefix` — offer the recorded
    // indices of `arr` as `$arr(index)`.  A `(` in the partial always
    // resolves here (to suggestions or an empty list); it never falls
    // through to plain-name completion.
    if partial.contains('(') {
        return array_element_completions(
            line_text,
            scope,
            byte_offset,
            partial,
            end,
            edit_start,
            ns_global_fallback,
        );
    }

    // Scope-aware: union of variables visible at the cursor (innermost scope
    // first, then enclosing scopes up to the global root).
    let names = visible_substitutable_names(scope, byte_offset, partial);

    // Inside a proc / nested namespace, a global variable is only reachable
    // via its `::`-qualified name (a bare `$foo` there is a local / namespace
    // lookup), so qualify global-origin names — `foo-bar` → `::foo-bar`.
    let qualify_globals = crate::definition::global_vars_needing_qualification(
        scope,
        byte_offset,
        ns_global_fallback,
    );

    let edit = VarEdit {
        start: edit_start,
        end: edit_end,
        has_open_brace,
    };
    let mut items = Vec::new();
    push_local_var_items(&mut items, names, qualify_globals.as_ref(), edit);
    // Cross-namespace candidates — variables in *other* namespaces, offered in
    // fully-qualified `::ns::var` form (vars in the cursor's own namespace
    // chain are already above as bare names).
    push_cross_namespace_vars(&mut items, scope, byte_offset, partial, edit);
    if !items.is_empty() {
        return items;
    }

    // Fuzzy fallback for the whole variable response: nothing
    // prefix-matched the fragment, so re-enumerate every candidate
    // (local and cross-namespace) and offer the closest names.
    let mut universe = Vec::new();
    push_local_var_items(
        &mut universe,
        visible_substitutable_names(scope, byte_offset, ""),
        qualify_globals.as_ref(),
        edit,
    );
    push_cross_namespace_vars(&mut universe, scope, byte_offset, "", edit);
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

/// Sorted, deduplicated variable names visible at `byte_offset` that
/// start with `partial` and survive the substitutability check.
fn visible_substitutable_names(scope: &Scope, byte_offset: u32, partial: &str) -> Vec<String> {
    let mut names: Vec<String> = crate::definition::visible_variable_names(scope, byte_offset)
        .into_iter()
        .filter(|n| n.starts_with(partial) && var_is_substitutable(n))
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
        let use_brace = edit.has_open_brace || !is_bare_var_name(&qname);
        let new_text = if use_brace {
            format!("${{{qname}}}")
        } else {
            format!("${qname}")
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
}

/// Append fully-qualified `::ns::var` candidates from namespaces outside the
/// cursor's own lexical chain, deduped against the labels already in `items`.
fn push_cross_namespace_vars(
    items: &mut Vec<CompletionItem>,
    scope: &Scope,
    byte_offset: u32,
    partial: &str,
    edit: VarEdit,
) {
    let mut seen: FxHashSet<String> = items.iter().map(|i| i.label.clone()).collect();
    let chain = crate::definition::lexical_namespace_chain(scope, byte_offset);
    let mut qnames = crate::definition::cross_namespace_qualified_vars(scope, &chain);
    qnames.sort_unstable();
    qnames.dedup();
    for qname in qnames {
        if !qname.starts_with(partial) || !var_is_substitutable(&qname) {
            continue;
        }
        let label = format!("${qname}");
        if !seen.insert(label.clone()) {
            continue;
        }
        let use_brace = edit.has_open_brace || !is_bare_var_name(&qname);
        let new_text = if use_brace {
            format!("${{{qname}}}")
        } else {
            format!("${qname}")
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
/// matching the original short-circuit semantics for array-element context.
fn array_element_completions(
    line_text: &str,
    scope: &Scope,
    byte_offset: u32,
    partial: &str,
    end: usize,
    edit_start: Option<u32>,
    ns_global_fallback: bool,
) -> Vec<CompletionItem> {
    // The caller dispatches here exactly when `partial` contains a `(`.
    let Some(paren) = partial.find('(') else {
        return Vec::new();
    };
    let arr_name = partial[..paren].trim_start_matches('{');
    let elem_prefix = &partial[paren + 1..];
    let Some(arr_def) = crate::definition::lookup_var_in_scope_chain(
        scope,
        byte_offset,
        arr_name,
        ns_global_fallback,
    ) else {
        return Vec::new();
    };
    if arr_def.array_indices.is_empty() || !is_bare_var_name(arr_name) {
        return Vec::new();
    }
    // Extend the replace range to swallow an existing `)` that closes this
    // array reference — but only if one actually exists on the line. When the
    // reference is unclosed (`$arr(k more stuff`), the old unbounded walk ran
    // to end-of-line, so accepting a completion deleted the trailing text;
    // leave the range at the cursor instead (issue 181).
    let line_chars: Vec<char> = line_text.chars().collect();
    let arr_end = match line_chars[end.min(line_chars.len())..]
        .iter()
        .position(|&c| c == ')')
    {
        Some(rel) => end + rel + 1,
        None => end,
    };
    let arr_edit_end = char_col_to_utf16(line_text, arr_end);
    let valid: Vec<&String> = arr_def
        .array_indices
        .iter()
        .filter(|elem| !elem.is_empty() && !elem.contains(')'))
        .collect();
    let FilteredCandidates {
        candidates: elems,
        fuzzy,
    } = filter_candidates(elem_prefix, valid, |elem| elem.as_str());
    let mut items = Vec::new();
    for elem in elems {
        let new_text = format!("${arr_name}({elem})");
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

/// Determine the surrounding command's name and the cursor's
/// word index on the current line.  Returns `(command,
/// word_index)` where `word_index` is the 0-based position of
/// the cursor (0 = typing the command name, 1 = typing the
/// first argument, etc.).
///
/// **Single-line context only.**  Continuation lines, embedded
/// `[…]` / `{…}` token nesting, and `;` command separators are
/// not handled here.  The single-line approach covers the common
/// editor cases (cursor on the same logical line as the
/// command head) and shares its shape with the single-line
/// helper in [`crate::signature_help`].
fn command_context_on_line(source: &str, line: u32, character: u32) -> Option<(String, usize)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let prefix: String = chars[..col].iter().collect();

    // Tokenise on whitespace.  The first token is the command;
    // the count of subsequent tokens (adjusted for whether the
    // cursor sits mid-token) gives the word index.
    let tokens: Vec<&str> = prefix.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let command = tokens[0].to_owned();

    // If the prefix ends with whitespace the cursor is *between*
    // tokens — at the start of a fresh word.  Otherwise the
    // cursor is inside the last token.
    let ends_in_space = prefix.ends_with(|c: char| c.is_whitespace());
    let word_index = if ends_in_space {
        tokens.len()
    } else {
        tokens.len().saturating_sub(1)
    };
    Some((command, word_index))
}

/// The instance-variable name in a `$obj` / `${obj}` command head, or
/// `None` when the head is not a single bare variable reference.
fn strip_instance_var(cmd: &str) -> Option<String> {
    let rest = cmd.strip_prefix('$')?;
    let inner = rest
        .strip_prefix('{')
        .and_then(|r| r.strip_suffix('}'))
        .unwrap_or(rest);
    (!inner.is_empty()
        && inner
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':'))
    .then(|| inner.to_string())
}

/// Complete the methods/subcommands callable on a receiver whose class
/// (`class_q`) is already resolved — a user-defined class walks the whole
/// MRO, a registry-modelled one reads its instance table (see
/// [`method_items`]).  Returns `None` when the class is unknown to both or
/// nothing prefix-matches, so the caller falls through to plain command
/// completion — a typo'd fragment is deliberately *not* fuzzy-matched
/// here, because the command/proc path this falls through to still offers
/// its own prefix matches; the response-level [`fuzzy_command_fallback`]
/// re-ranks these methods together with command candidates only once the
/// whole response would otherwise be empty.
fn method_completions(
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    class_q: &str,
    partial: &str,
) -> Option<Vec<CompletionItem>> {
    let all = method_items(analysis, Some(registry), class_q)?;
    let FilteredCandidates {
        candidates: items,
        fuzzy,
    } = filter_candidates(partial, all, |item| item.label.as_str());
    (!fuzzy && !items.is_empty()).then_some(items)
}

/// Every method item callable on an instance of `class_q` (label-sorted) —
/// gathered across the whole MRO so **inherited** methods appear, not just
/// the receiver class's own.  Overridden methods appear once (the
/// most-derived provider wins).  Only public methods plus the universal
/// `destroy` are offered (an external `$obj method` dispatch cannot reach
/// private / unexported methods).  A class unknown to the analysis falls
/// through to the registry (`ObjectClassSpec` / a self-referential Tk
/// widget spec — issue #927).  The candidate universe behind
/// [`method_completions`] and [`fuzzy_command_fallback`].
fn method_items(
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    class_q: &str,
) -> Option<Vec<CompletionItem>> {
    if !analysis.all_classes.contains_key(class_q) {
        return registry_method_items(registry?, class_q);
    }
    let hierarchy = analysis.class_hierarchy();
    let mro = hierarchy
        .mro_map
        .get(class_q)
        .cloned()
        .unwrap_or_else(|| vec![class_q.to_string()]);
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let mut items: Vec<CompletionItem> = Vec::new();
    for cls in &mro {
        let Some(cd) = analysis.all_classes.get(cls) else {
            continue;
        };
        // Instance dispatch (`$obj method`) reaches *instance* methods only.
        // Class-side methods (`self method` / classmethod) are callable on
        // the class command, not the instance command — so they are excluded
        // here to avoid suggesting methods that would error on `$obj`.
        let mut methods: Vec<(&String, &str)> = cd
            .methods
            .iter()
            .filter(|(_, m)| m.visibility == "public")
            .map(|(n, _)| (n, cls.as_str()))
            .collect();
        methods.sort_by(|a, b| a.0.cmp(b.0));
        for (name, provider) in methods {
            if !seen.insert(name.clone()) {
                continue;
            }
            let detail = if provider == class_q {
                format!("method — {class_q}")
            } else {
                format!("method — inherited from {provider}")
            };
            items.push(CompletionItem {
                label: name.clone(),
                insert_text: name.clone(),
                kind: CompletionKind::Function,
                detail: Some(detail),
                ..CompletionItem::default()
            });
        }
    }
    // The universal object method (present on every object).
    if seen.insert("destroy".to_string()) {
        items.push(CompletionItem {
            label: "destroy".to_string(),
            insert_text: "destroy".to_string(),
            kind: CompletionKind::Function,
            detail: Some("method — oo::object builtin".to_string()),
            ..CompletionItem::default()
        });
    }
    items.sort_by(|a, b| a.label.cmp(&b.label));
    Some(items)
}

/// Every instance method/subcommand item for a *registry*-modelled class:
/// a tcllib-style factory (`report::report`, `struct::graph` — a distinct
/// `ObjectClassSpec`) or a Tk/ttk widget (self-referential — `class_q`
/// names its own `CommandSpec`, whose `subcommands` *is* its instance
/// dispatch table).  Tries the dedicated object class first since that is
/// the precise, intended shape; the self-referential widget case falls
/// back to the class's own `CommandSpec` when it has no separate
/// `ObjectClassSpec` (matching `CommandRegistry::instance_method`'s own
/// `object_class` lookup, `registry.rs:413-415`).  No MRO walk — neither
/// shape currently declares `superclasses` (issue #927).  Unfiltered:
/// fragment matching happens in [`method_completions`].
fn registry_method_items(registry: &CommandRegistry, class_q: &str) -> Option<Vec<CompletionItem>> {
    let methods: &[tcl_registry::SubCommand] = registry
        .object_class(class_q)
        .map(|oc| oc.instance_methods)
        .or_else(|| registry.get(class_q).map(|spec| spec.subcommands))?;
    let mut items: Vec<CompletionItem> = methods
        .iter()
        .map(|m| CompletionItem {
            label: m.name.to_owned(),
            insert_text: m.name.to_owned(),
            kind: CompletionKind::Function,
            detail: Some(format!("method — {class_q}")),
            documentation: (!m.detail.is_empty()).then(|| m.detail.to_owned()),
            ..CompletionItem::default()
        })
        .collect();
    if items.is_empty() {
        return None;
    }
    items.sort_by(|a, b| a.label.cmp(&b.label));
    Some(items)
}

/// Detect a `-switch` partial at the cursor.  Returns
/// `Some(switch_partial)` (including the leading dash) when
/// the character immediately preceding the identifier run
/// before the cursor is a `-`; returns `None` otherwise.
///
/// `partial` is the identifier-only partial computed by
/// [`word_partial_at_position`]; the helper reconstructs the
/// switch-aware partial by prepending the dash without
/// re-walking the line.
/// The guaranteed-available version floor for a command's owning package,
/// resolved from the document's `package require` statements.
///
/// When several `package require <pkg> <req>` lines name the same package, the
/// most restrictive (highest) lower bound wins.  Returns `None` when the
/// command is not package-gated or the package was required without a version
/// (permissive — every option surfaces).
fn package_version_floor<'a>(
    analysis: &'a AnalysisResult,
    spec: &tcl_registry::CommandSpec,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<&'a str> {
    let pkg = spec.owning_package()?;
    let require_floor = analysis
        .package_requires
        .iter()
        // Only *unconditional* requires guarantee the version; an optional
        // probe (`catch {package require Tk 8.7}`, or a `require` inside an
        // `if` arm) must not raise the floor and hide a gated option/command.
        .filter(|req| req.name == pkg && !req.conditional)
        .filter_map(|req| req.version.as_deref())
        .map(tcl_registry::version::requirement_lower_bound)
        .max_by(|a, b| tcl_registry::version::compare(a, b));
    // The profile's library pin supplies the base floor (§7.1: the shipped
    // Tk on a plain Tcl base, a keyed vendor surface at its D5
    // oldest-supported default); an explicit require can only raise it.
    let pin_floor = profile.library_floor_default(pkg);
    match (pin_floor, require_floor) {
        (Some(pin), Some(req)) => {
            if tcl_registry::version::compare(req, pin).is_gt() {
                Some(req)
            } else {
                Some(pin)
            }
        }
        (pin, require) => pin.or(require),
    }
}

fn switch_partial_at_position(
    source: &str,
    line: u32,
    character: u32,
    partial: &str,
) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let start = col.checked_sub(partial.chars().count())?;
    if start == 0 {
        return None;
    }
    if chars[start - 1] != '-' {
        return None;
    }
    Some(format!("-{partial}"))
}

fn switch_completions(
    options: &[tcl_registry::hover::OptionSpec],
    profile: &'static tcl_dialect::DialectProfile,
    parent_dialects: Option<tcl_dialect::DialectSet>,
    partial: &str,
    edit: (u32, u32),
    package_version: Option<&str>,
) -> Vec<CompletionItem> {
    let mut opts: Vec<_> = options
        .iter()
        .filter(|opt| {
            opt.available_for_version(package_version)
                && profile.is_option_available(opt, parent_dialects)
        })
        .collect();
    opts.sort_unstable_by_key(|opt| opt.name);
    let FilteredCandidates {
        candidates: opts,
        fuzzy,
    } = filter_candidates(partial, opts, |opt| opt.name);
    let mut items: Vec<CompletionItem> = opts
        .into_iter()
        .map(|opt| {
            let doc = (!opt.detail.is_empty()).then(|| opt.detail.to_owned());
            CompletionItem {
                label: opt.name.to_owned(),
                insert_text: opt.name.to_owned(),
                kind: CompletionKind::Function,
                detail: doc.clone(),
                documentation: doc,
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: Some(CompletionEdit {
                    start_char: edit.0,
                    end_char: edit.1,
                    new_text: opt.name.to_owned(),
                }),
            }
        })
        .collect();
    if fuzzy {
        // `partial` carries the leading dash, matching the on-screen
        // fragment the replace edit covers.
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}

/// Compute iRules event-name completions for `when EVENT { body }`.
/// Returns one [`CompletionItem`] per event registered in the
/// shared [`tcl_registry::events::EventRegistry`] whose name starts
/// with `partial` (case-sensitive — event names are all-uppercase
/// snake-case by convention).
///
/// The registry is built once per call.  `EventRegistry::build`
/// materialises a small static table at runtime; for completion
/// (one call per user keystroke at a `when ` site) that's
/// negligible.  Threading a cached registry through the public
/// `completions()` signature would force every caller to plumb it
/// even when iRules isn't in scope, so we keep it local instead.
fn event_name_completions(partial: &str) -> Vec<CompletionItem> {
    let reg = tcl_registry::events::EventRegistry::build();
    let mut names: Vec<&str> = reg.all_event_names().into_iter().collect();
    names.sort_unstable();
    let FilteredCandidates {
        candidates: names,
        fuzzy,
    } = filter_candidates(partial, names, |name| *name);
    let mut items: Vec<CompletionItem> = names
        .into_iter()
        .map(|name| {
            // Describe the event from its registry props (sides / transport /
            // implied profiles) so the item carries documentation.
            let doc = reg.get_props(name).map(|p| {
                let mut parts = vec![format!("F5 iRules event `{name}`")];
                if !p.implied_profiles.is_empty() {
                    parts.push(format!("Profiles: {}", p.implied_profiles.join(", ")));
                }
                if p.deprecated {
                    parts.push("Deprecated".to_string());
                }
                parts.join("\n\n")
            });
            CompletionItem {
                label: name.to_owned(),
                insert_text: name.to_owned(),
                kind: CompletionKind::Function,
                detail: Some("F5 iRules event".to_string()),
                sort_text: None,
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: doc.or_else(|| Some(format!("F5 iRules event `{name}`"))),
            }
        })
        .collect();
    if fuzzy {
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}

fn subcommand_completions(
    spec: &tcl_registry::CommandSpec,
    profile: &'static tcl_dialect::DialectProfile,
    partial: &str,
) -> Vec<CompletionItem> {
    // Only subcommands available under the active profile are offered —
    // the §5.1 `available_subcommands` gap: `dict getwithdefault` (9.0+)
    // must not be offered in an 8.6 buffer, and an iRules-only subcommand
    // must not surface in plain Tcl.
    let mut subs: Vec<&tcl_registry::SubCommand> = profile.available_subcommands(spec);
    subs.sort_unstable_by_key(|sub| sub.name);
    let FilteredCandidates {
        candidates: subs,
        fuzzy,
    } = filter_candidates(partial, subs, |sub| sub.name);
    let mut items: Vec<CompletionItem> = subs
        .into_iter()
        .map(|sub| CompletionItem {
            label: sub.name.to_owned(),
            insert_text: sub.name.to_owned(),
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
) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
    let line_index = tcl_lexer::LineIndex::new(source);
    let offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    analysis
        .scoped_command_regions
        .iter()
        .find(|r| r.contains(offset))
        .map(|r| r.env)
}

/// Completion items for the command heads of a scoped environment matching
/// `partial` (`top`, `data`, `columns`, … inside a `report::defstyle` body).
fn scoped_command_completions(
    env: &'static tcl_registry::scoped::ScopedCommandEnv,
    partial: &str,
) -> Vec<CompletionItem> {
    env.commands
        .iter()
        .filter(|c| partial.is_empty() || c.name.starts_with(partial))
        .map(|c| CompletionItem {
            label: c.name.to_owned(),
            insert_text: c.name.to_owned(),
            kind: CompletionKind::Function,
            detail: (!c.detail.is_empty()).then(|| c.detail.to_owned()),
            sort_text: None,
            is_snippet: false,
            filter_text: None,
            text_edit: None,
            documentation: None,
        })
        .collect()
}

/// Completion items for the ensemble operations of a scoped command
/// (`set` / `get` / `enable` / … after `top ` in a `report::defstyle` body).
fn scoped_op_completions(
    cmd: &'static tcl_registry::scoped::ScopedCommand,
    partial: &str,
) -> Vec<CompletionItem> {
    let ops: Vec<&tcl_registry::SubCommand> = cmd.subcommands.iter().collect();
    let FilteredCandidates {
        candidates: ops,
        fuzzy,
    } = filter_candidates(partial, ops, |op| op.name);
    let mut items: Vec<CompletionItem> = ops
        .into_iter()
        .map(|s| CompletionItem {
            label: s.name.to_owned(),
            insert_text: s.name.to_owned(),
            kind: CompletionKind::Function,
            detail: (!s.detail.is_empty()).then(|| s.detail.to_owned()),
            sort_text: None,
            is_snippet: false,
            filter_text: None,
            text_edit: None,
            documentation: None,
        })
        .collect();
    if fuzzy {
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}

/// Build completions for the second-level subcommands of a two-level ensemble
/// (`info object <op>` / `info class <op>`), filtered by `partial`.  Each
/// item's detail is the operation's one-line description (issue #798).
fn sub_subcommand_completions(
    sub: &tcl_registry::SubCommand,
    partial: &str,
) -> Vec<CompletionItem> {
    let mut subs: Vec<&tcl_registry::SubSubCommand> = sub.sub_subcommands.iter().collect();
    subs.sort_unstable_by_key(|s| s.name);
    let FilteredCandidates {
        candidates: subs,
        fuzzy,
    } = filter_candidates(partial, subs, |s| s.name);
    let mut items: Vec<CompletionItem> = subs
        .into_iter()
        .map(|s| CompletionItem {
            label: s.name.to_owned(),
            insert_text: s.name.to_owned(),
            kind: CompletionKind::Function,
            detail: (!s.detail.is_empty()).then(|| s.detail.to_owned()),
            sort_text: None,
            is_snippet: false,
            filter_text: None,
            text_edit: None,
            documentation: None,
        })
        .collect();
    if fuzzy {
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}

/// Return the `n`-th whitespace-delimited word on `line` of
/// `source` (0-based), if present.  Used to recover the
/// subcommand keyword (word index 1) for argument-value
/// completion.
fn nth_word_on_line(source: &str, line: u32, n: usize) -> Option<String> {
    source
        .split('\n')
        .nth(line as usize)?
        .split_whitespace()
        .nth(n)
        .map(str::to_owned)
}

/// Build completions for a fixed set of enumerable argument
/// values (e.g. `string is <class>`), filtered by `partial`.
fn arg_value_completions(values: &[tcl_registry::ArgValue], partial: &str) -> Vec<CompletionItem> {
    let mut values: Vec<&tcl_registry::ArgValue> = values.iter().collect();
    // Pre-sort by value so the prefix path's output matches the
    // historical post-build label sort, while the fuzzy path keeps its
    // `(distance, name)` ranking.
    values.sort_unstable_by_key(|v| v.value);
    let FilteredCandidates {
        candidates: values,
        fuzzy,
    } = filter_candidates(partial, values, |v| v.value);
    let mut items: Vec<CompletionItem> = values
        .into_iter()
        .map(|v| CompletionItem {
            label: v.value.to_owned(),
            insert_text: v.value.to_owned(),
            kind: CompletionKind::EnumValue,
            detail: (!v.detail.is_empty()).then(|| v.detail.to_owned()),
            sort_text: None,
            is_snippet: false,
            filter_text: None,
            text_edit: None,
            documentation: None,
        })
        .collect();
    if fuzzy {
        decorate_fuzzy_items(&mut items, partial);
    }
    items
}

/// Math operators that the registry registers as commands
/// (Tcl 9's `tcl::mathop` exposes them as commands) but that
/// don't make sense as completion items at a command position.
const SKIP_BUILTIN_NAMES: &[&str] = &["+", "-", "*", "/", ">", ">=", "<", "<=", "==", "!="];

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

fn builtin_sort_text(name: &str, usage: usize) -> String {
    // `B<usage>_<name>` — built-ins sort after user procs (`A…`)
    // but before any item with no `sort_text`.  The two-digit
    // bucket prevents lexicographic confusion between 1 and 10.
    let rank = usage_bucket(usage);
    format!("B{rank:02}_{name}")
}

fn builtin_completions(
    registry: &CommandRegistry,
    dialect: &str,
    partial: &str,
    usage: &FxHashMap<String, usize>,
    tk_loaded: bool,
    analysis: &AnalysisResult,
) -> Vec<CompletionItem> {
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
    // (custom / non-Tcl) resolves to the permissive fallback profile.
    let profile = tcl_dialect::DialectProfile::by_name(dialect);
    let mut names: Vec<&str> = registry
        .command_names()
        .filter(|n| partial.is_empty() || n.starts_with(partial))
        .filter(|n| !SKIP_BUILTIN_NAMES.iter().any(|skip| skip == n))
        .filter(|n| profile.resolve_command(registry, n).is_some())
        // Tk commands (`required_package == "Tk"`) are only offered once Tk
        // is loaded — see the `tk_loaded` computation in `completions` — and
        // never inside a vendor shell: an F5 / EDA / bpf profile is a closed
        // world where a desktop library cannot be `package require`d, even
        // if the source says so (dialect-profile-model.md §7.2; Tk hosting
        // becomes a first-class library pin on the versioned-library axis).
        .filter(|n| {
            (tk_loaded && profile.vendor_bit.is_none())
                || registry
                    .get(n)
                    .is_none_or(|spec| spec.required_package != Some("Tk"))
        })
        // Package-version gate: a command introduced in a later package release
        // (`ttk::*` needs Tk 8.5) must not be offered when this file's
        // `package require <pkg> <req>` guarantees only an older version — the
        // same floor W135 checks.  A package required without a version, or not
        // required at all, yields no floor and stays permissive.
        .filter(|n| {
            registry.get(n).is_none_or(|spec| {
                let floor = package_version_floor(analysis, spec, profile);
                // On a keyed ambient axis (the F5 surfaces) the declared
                // range applies: explicit introduction or the 15.0
                // baseline, plus any removal release.
                match (profile.keyed_version_range(spec), floor) {
                    (Some((min, max)), Some(floor)) => {
                        tcl_registry::version::within_range(floor, min, max)
                    }
                    _ => spec.available_for_version(floor),
                }
            })
        })
        .collect();
    names.sort_unstable();
    names
        .into_iter()
        .map(|name| {
            let count = usage.get(name).copied().unwrap_or(0);
            let spec = registry.get(name);
            CompletionItem {
                label: name.to_owned(),
                insert_text: name.to_owned(),
                kind: CompletionKind::Function,
                detail: spec.map(|s| command_detail(s, profile)),
                sort_text: Some(builtin_sort_text(name, count)),
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
    profile: &tcl_dialect::DialectProfile,
) -> String {
    if let Some(pkg) = spec.tcllib_package {
        format!("tcllib ({pkg})")
    } else if let Some(pkg) = spec.required_package
        && !profile.is_ambient_package(pkg)
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
    let mut items = Vec::new();
    let mut names: Vec<(&str, &ProcDef)> = analysis
        .all_procs
        .iter()
        .filter_map(|(qname, proc_def)| {
            // Match either the simple name or the qualified
            // name — the user may have typed the leading `::`.
            if proc_def.name.starts_with(partial) || qname.starts_with(partial) {
                Some((qname.as_str(), proc_def))
            } else {
                None
            }
        })
        .collect();
    names.sort_unstable_by_key(|(qname, _)| *qname);
    for (qname, proc_def) in names {
        let count = usage
            .get(proc_def.name.as_str())
            .copied()
            .max(usage.get(qname).copied())
            .unwrap_or(0);
        items.push(CompletionItem {
            label: proc_def.name.clone(),
            insert_text: qname.to_owned(),
            kind: CompletionKind::Function,
            detail: Some(proc_signature_str(proc_def)),
            sort_text: Some(proc_sort_text(&proc_def.name, count)),
            is_snippet: false,
            filter_text: None,
            text_edit: None,
            documentation: None,
        });
    }
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
    dialect: &str,
) -> Vec<CompletionItem> {
    let partial = word_partial_at_position(source, line, character);
    if partial.chars().count() < MIN_FUZZY_FRAGMENT_CHARS {
        return Vec::new();
    }
    let usage = document_usage_counts(analysis);
    let mut universe: Vec<CompletionItem> = Vec::new();
    // Receiver-method context — the method universe the instance branch of
    // `context_aware_completions` declined to fuzzy-match (see
    // `method_completions`) joins the ranking here, resolved with the same
    // `$var`-vs-bareword gate that branch applies (issue #927).
    if let Some((cmd, word_idx)) = command_context_on_line(source, line, character)
        && word_idx == 1
    {
        let receiver = strip_instance_var(&cmd).map(|v| (v, true)).or_else(|| {
            (!cmd.is_empty() && !cmd.contains(['$', '['])).then(|| (cmd.clone(), false))
        });
        if let Some((recv, is_dollar)) = receiver
            && let Some(class_q) =
                crate::definition::receiver_instance_class(analysis, &recv, is_dollar)
            && let Some(methods) = method_items(analysis, registry, class_q)
        {
            universe.extend(methods);
        }
    }
    universe.extend(proc_completions(analysis, "", &usage));
    if let Some(registry) = registry {
        let tk_loaded =
            dialect == "tk" || analysis.package_requires.iter().any(|req| req.name == "Tk");
        universe.extend(builtin_completions(
            registry, dialect, "", &usage, tk_loaded, analysis,
        ));
    }
    if let Some(env) = scoped_env_at(analysis, source, line, character) {
        let present: FxHashSet<String> = universe.iter().map(|i| i.label.clone()).collect();
        universe.extend(
            scoped_command_completions(env, "")
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
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn array_element_completion_does_not_eat_trailing_text_when_unclosed() {
        // `$arr(k more stuff` with no `)` — accepting `$arr(key)` must replace
        // only up to the cursor, not to end-of-line, so ` more stuff` survives
        // (issue 181).
        let src = "set arr(key) 1\nset v $arr(k more stuff\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        // Line 1, cursor just after `k`: `set v $arr(k` → char col 12.
        let items = completions(src, 1, 12, &analysis, Some(&registry), None, "tcl8.6");
        let key = items
            .iter()
            .find(|i| i.label == "$arr(key)")
            .expect("array-element completion offered");
        let edit = key.text_edit.as_ref().expect("has a replace edit");
        assert_eq!(
            edit.end_char, 12,
            "replace range must stop at the cursor, not run to EOL"
        );
    }

    #[test]
    fn array_element_completion_swallows_existing_close_paren() {
        // With a closing `)`, the replace range extends through it so the old
        // index is fully replaced.
        let src = "set arr(key) 1\nset v $arr(k)\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 1, 12, &analysis, Some(&registry), None, "tcl8.6");
        let key = items
            .iter()
            .find(|i| i.label == "$arr(key)")
            .expect("array-element completion offered");
        let edit = key.text_edit.as_ref().expect("has a replace edit");
        assert_eq!(edit.end_char, 13, "replace range must cover the `)`");
    }

    #[test]
    fn obj_method_completion_includes_inherited() {
        let src = "oo::class create Animal {\n    method eat {} {}\n}\noo::class create Dog {\n    superclass Animal\n    method bark {} {}\n}\nset d [Dog new]\n$d \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        // Cursor after `$d ` on line 8.
        let items = completions(src, 8, 3, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"bark"), "own method missing: {labels:?}");
        assert!(
            labels.contains(&"eat"),
            "inherited method missing: {labels:?}"
        );
        assert!(labels.contains(&"destroy"), "builtin missing: {labels:?}");
        // The inherited one is labelled as such.
        let eat = items.iter().find(|i| i.label == "eat").unwrap();
        assert!(
            eat.detail
                .as_deref()
                .unwrap_or("")
                .contains("inherited from ::Animal"),
            "{:?}",
            eat.detail
        );
    }

    #[test]
    fn obj_method_completion_excludes_class_side_methods() {
        // `classmethod build` is callable on the *class* command, not on an
        // instance.  `$obj ` completion must offer the instance method
        // (`bark`) but not the class-side `build`.
        let src = "oo::class create Dog {\n    method bark {} {}\n    classmethod build {} {}\n}\nset d [Dog new]\n$d \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        // Cursor after `$d ` on line 5.
        let items = completions(src, 5, 3, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"bark"),
            "instance method missing: {labels:?}"
        );
        assert!(
            !labels.contains(&"build"),
            "class-side method must not appear on an instance: {labels:?}",
        );
    }

    #[test]
    fn obj_method_completion_filters_by_partial() {
        let src = "oo::class create Dog {\n    method bark {} {}\n    method beg {} {}\n    method sit {} {}\n}\nset d [Dog new]\n$d b\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 6, 4, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"bark") && labels.contains(&"beg"),
            "{labels:?}"
        );
        assert!(
            !labels.contains(&"sit"),
            "partial `b` should exclude sit: {labels:?}"
        );
    }

    /// A Tk widget's bareword instance path completes its own subcommands
    /// (a self-referential registry `object_class`, not a user class —
    /// issue #927), same as `$var`-receiver completion above.
    #[test]
    fn widget_bareword_completion_offers_subcommands() {
        let src = "ttk::treeview .t\n.t i\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        // Cursor after `.t i` on line 1.
        let items = completions(src, 1, 4, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"instate")
                && labels.contains(&"identify")
                && labels.contains(&"insert"),
            "{labels:?}"
        );
        assert!(
            !labels.contains(&"curselection"),
            "must not offer an unrelated widget's subcommand: {labels:?}"
        );
    }

    /// The `set lb [listbox .l]` return-value-capture shape completes too.
    #[test]
    fn widget_var_captured_completion_offers_subcommands() {
        let src = "set lb [listbox .l]\n$lb cu\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 1, 6, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"curselection"), "{labels:?}");
    }

    /// A bareword that merely shares a name with an unrelated tracked
    /// variable must not offer method completion — only a name genuinely
    /// bound by a create call qualifies (mirrors
    /// `receiver_instance_class_gates_bare_on_created_commands` in
    /// definition.rs).
    #[test]
    fn bareword_completion_does_not_leak_unrelated_variable_class() {
        let src = "oo::class create Bar {\n    method get {} {}\n}\nset b [Bar new]\nb \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 4, 2, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            !labels.contains(&"get"),
            "bareword `b` was never created — must not complete as `Bar`: {labels:?}"
        );
    }

    #[test]
    fn dollar_trigger_at_eol_returns_partial() {
        let (trigger, partial) = variable_trigger("set x $", 0, 7).unwrap();
        assert_eq!(trigger, '$');
        assert_eq!(partial, "");
    }

    #[test]
    fn dollar_trigger_with_partial_name() {
        let (trigger, partial) = variable_trigger("set y $foo", 0, 10).unwrap();
        assert_eq!(trigger, '$');
        assert_eq!(partial, "foo");
    }

    #[test]
    fn brace_trigger_recognised() {
        let (trigger, partial) = variable_trigger("set y ${ba", 0, 10).unwrap();
        assert_eq!(trigger, '{');
        assert_eq!(partial, "ba");
    }

    #[test]
    fn space_resets_trigger_context() {
        // The `$` is followed by a space + new identifier — no
        // variable trigger active at the cursor.
        assert!(variable_trigger("set $ ab", 0, 8).is_none());
    }

    #[test]
    fn variable_completion_lists_globals_alphabetically() {
        let src = "set apple 1\nset banana 2\nset $\n";
        let analysis = analyse(src);
        // Cursor after `$` on the third line.
        let items = completions(src, 2, 5, &analysis, None, None, "tcl8.6");
        assert!(!items.is_empty(), "expected variable completions");
        assert_eq!(items[0].kind, CompletionKind::Variable);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // Alphabetical order.
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        assert_eq!(labels, sorted);
        assert!(labels.contains(&"$apple"));
        assert!(labels.contains(&"$banana"));
    }

    #[test]
    fn variable_completion_filters_by_partial() {
        let src = "set apple 1\nset banana 2\nset $b\n";
        let analysis = analyse(src);
        let items = completions(src, 2, 6, &analysis, None, None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["$banana"]);
    }

    #[test]
    fn proc_completion_lists_user_defined_procs() {
        let src = "proc greet {} {}\nproc shout {} {}\ng\n";
        let analysis = analyse(src);
        // Cursor right after `g` on third line.
        let items = completions(src, 2, 1, &analysis, None, None, "tcl8.6");
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].kind, CompletionKind::Function);
        assert_eq!(items[0].label, "greet");
    }

    // usage-bucket sort-text

    #[test]
    fn usage_bucket_thresholds() {
        // Exhaustive bucket-table check across every threshold.
        assert_eq!(usage_bucket(0), 5);
        assert_eq!(usage_bucket(1), 4);
        assert_eq!(usage_bucket(2), 4);
        assert_eq!(usage_bucket(3), 3);
        assert_eq!(usage_bucket(7), 3);
        assert_eq!(usage_bucket(8), 2);
        assert_eq!(usage_bucket(19), 2);
        assert_eq!(usage_bucket(20), 1);
        assert_eq!(usage_bucket(49), 1);
        assert_eq!(usage_bucket(50), 0);
        assert_eq!(usage_bucket(1000), 0);
    }

    #[test]
    fn proc_sort_text_has_lower_bucket_for_used_procs() {
        // Three calls to `greet` and zero calls to `shout`.
        // The completion-list partial is empty (cursor on
        // line 6 col 0), so both procs surface — `greet` in
        // bucket 3 (≥3 calls) and `shout` in bucket 5 (no
        // calls).
        let src = "proc greet {} {}\nproc shout {} {}\ngreet\ngreet\ngreet\n\n";
        let analysis = analyse(src);
        let items = completions(src, 5, 0, &analysis, None, None, "tcl8.6");
        let greet = items
            .iter()
            .find(|i| i.label == "greet")
            .unwrap_or_else(|| panic!("greet missing from {items:?}"));
        let greet_sort = greet.sort_text.as_deref().unwrap_or("");
        assert!(
            greet_sort.starts_with("A003_"),
            "greet sort_text {greet_sort:?} should land in bucket 3",
        );
        let shout = items
            .iter()
            .find(|i| i.label == "shout")
            .unwrap_or_else(|| panic!("shout missing from {items:?}"));
        let shout_sort = shout.sort_text.as_deref().unwrap_or("");
        assert!(
            shout_sort.starts_with("A005_"),
            "shout sort_text {shout_sort:?} should land in bucket 5",
        );
        // Lower bucket sorts first.
        assert!(greet_sort < shout_sort);
    }

    #[test]
    fn empty_partial_lists_all_procs() {
        let src = "proc alpha {} {}\nproc beta {} {}\n\n";
        let analysis = analyse(src);
        let items = completions(src, 2, 0, &analysis, None, None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"alpha"));
        assert!(labels.contains(&"beta"));
    }

    // built-in command completion
    //
    // Tests pin the contract that a non-`None` registry parameter
    // extends proc-name completion with every command the
    // registry knows about, filtered by the partial typed at the
    // cursor.

    #[test]
    fn builtin_completion_lists_registry_commands_at_command_position() {
        // No user-defined procs, partial `pu` → `puts` (and any
        // other registered command beginning with `pu`) should
        // surface.
        let src = "pu\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 2, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"puts"),
            "expected `puts` from registry; got {labels:?}",
        );
    }

    #[test]
    fn builtin_completion_detail_shows_provenance() {
        // A core built-in shows `built-in`; a stdlib command shows its
        // `stdlib (PKG)` provenance from the registry `required_package`.
        let src = "pu\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 2, &analysis, Some(&registry), None, "tcl8.6");
        let puts = items.iter().find(|i| i.label == "puts").expect("puts");
        assert_eq!(puts.detail.as_deref(), Some("built-in"), "{puts:?}");

        // `http::geturl` is a stdlib command requiring the `http` package.
        let src = "http::ge\n";
        let analysis = analyse(src);
        let items = completions(src, 0, 8, &analysis, Some(&registry), None, "tcl8.6");
        if let Some(geturl) = items.iter().find(|i| i.label == "http::geturl") {
            assert_eq!(
                geturl.detail.as_deref(),
                Some("stdlib (http)"),
                "{geturl:?}"
            );
        }
    }

    #[test]
    fn command_detail_formats_each_provenance() {
        use tcl_registry::CommandRegistry;
        let reg = CommandRegistry::build_default();
        let tcl86 = tcl_dialect::DialectProfile::by_name("tcl8.6");
        // built-in: no package.
        assert_eq!(command_detail(reg.get("puts").unwrap(), tcl86), "built-in");
        // stdlib: required_package set.
        if let Some(spec) = reg.get("http::geturl") {
            assert_eq!(command_detail(spec, tcl86), "stdlib (http)");
        }
        // An ambient vendor surface reads as part of the runtime, never as
        // a require-gated stdlib package (§7.1 axis C).
        let ireg = tcl_registry::registry_for_dialect("f5-irules");
        let http2 = ireg.get("HTTP2::header").expect("HTTP2::header spec");
        assert_eq!(
            command_detail(http2, tcl_dialect::DialectProfile::irules()),
            "built-in"
        );
    }

    #[test]
    fn builtin_completion_filters_by_partial() {
        // Partial `whi` should yield `while` but not unrelated
        // commands like `puts`.
        let src = "whi\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 3, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"while"),
            "expected `while`; got {labels:?}"
        );
        assert!(
            !labels.contains(&"puts"),
            "should NOT contain `puts` for partial `whi`; got {labels:?}",
        );
    }

    #[test]
    fn builtin_completion_skips_math_operators() {
        // The registry registers `+` / `-` / `*` / etc. as
        // commands (Tcl 9 `tcl::mathop`), but they don't make
        // sense as completion items at a command position.
        let src = "+\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 1, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        for op in &["+", "-", "*", "/", ">", ">=", "<", "<=", "==", "!="] {
            assert!(
                !labels.contains(op),
                "math operator `{op}` should be filtered out; got {labels:?}",
            );
        }
    }

    #[test]
    fn builtin_completion_none_registry_keeps_minimal_behaviour() {
        // Passing `None` for the registry must not regress the
        // minimal port's behaviour — proc-name completion alone.
        let src = "proc helper {} {}\nhe\n";
        let analysis = analyse(src);
        let items_no_registry = completions(src, 1, 2, &analysis, None, None, "tcl8.6");
        let labels_no_registry: Vec<&str> =
            items_no_registry.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels_no_registry.contains(&"helper"),
            "expected `helper` proc; got {labels_no_registry:?}",
        );
        // No registry commands surface without the registry.
        assert_eq!(items_no_registry.len(), 1, "{items_no_registry:?}");
    }

    #[test]
    fn builtin_completion_merges_procs_and_registry() {
        // Both a user-defined proc and built-in commands should
        // surface when both apply.
        let src = "proc parade {} {}\npar\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 1, 3, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"parade"),
            "expected user proc `parade`; got {labels:?}",
        );
        // `parray` is a stdlib command starting with `par`.
        assert!(
            labels.contains(&"parray"),
            "expected built-in `parray`; got {labels:?}",
        );
    }

    #[test]
    fn builtin_completion_skipped_inside_variable_trigger() {
        // Variable trigger (`$par`) must take precedence and
        // suppress built-in command completions even when a
        // registry is supplied, via the `variable_completions`
        // short-circuit at the top of `completions`.
        let src = "set apple 1\nset $par\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 1, 8, &analysis, Some(&registry), None, "tcl8.6");
        // Only variable completions allowed here.
        for it in &items {
            assert_eq!(
                it.kind,
                CompletionKind::Variable,
                "variable trigger should suppress built-ins; got {it:?}",
            );
        }
    }

    // subcommand completion
    //
    // When the cursor sits at word-index 1 of a known command
    // whose spec declares non-empty `subcommands`, the
    // completion surface lists subcommand names (not user procs
    // or unrelated built-ins).

    #[test]
    fn subcommand_completion_surfaces_subcommands_at_word_index_1() {
        // `string l` — partial `l`, cursor at word-index 1 of
        // `string`.  The registry declares many `string`
        // subcommands; expect `length` and similar `l*` ones.
        let src = "string l\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 8, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"length"),
            "expected `length` subcommand; got {labels:?}",
        );
        // No proc / unrelated commands should leak through.
        assert!(
            !labels.contains(&"puts"),
            "subcommand context should not include `puts`; got {labels:?}",
        );
    }

    #[test]
    fn sub_subcommand_completion_surfaces_at_word_index_2() {
        // Issue #798 fix 3: after `info object ` (word-index 2), offer the
        // OBJECT INTROSPECTION operations with their descriptions.
        let src = "info object \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 12, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"class"),
            "expected `class`; got {labels:?}"
        );
        assert!(
            labels.contains(&"methods"),
            "expected `methods`; got {labels:?}"
        );
        // Descriptions are surfaced as detail.
        assert!(
            items
                .iter()
                .find(|i| i.label == "class")
                .and_then(|i| i.detail.as_deref())
                .is_some_and(|d| !d.is_empty()),
            "expected a detail on the `class` op",
        );
        // Partial filters: `info class super` → `superclasses`.
        let src = "info class super\n";
        let analysis = analyse(src);
        let items = completions(src, 0, 16, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(
            labels,
            vec!["superclasses"],
            "prefix should filter; got {labels:?}"
        );
    }

    #[test]
    fn subcommand_completion_lists_all_subcommands_with_empty_partial() {
        // `string ` (cursor just past the space) — empty partial,
        // word-index 1 of `string`.  Should list every `string`
        // subcommand, alphabetically.
        let src = "string \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 7, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // `string` has at minimum `length`, `match`, `range`,
        // `tolower`, `toupper` across all dialects.
        assert!(labels.contains(&"length"), "{labels:?}");
        assert!(labels.contains(&"match"), "{labels:?}");
        // Should be sorted.
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        assert_eq!(labels, sorted, "expected sorted subcommands");
    }

    #[test]
    fn subcommand_completion_falls_through_when_word_index_not_1() {
        // `string length f` — cursor at word-index 2 (after
        // `length` argument).  Subcommand completion should NOT
        // fire; we fall through to command + proc completion
        // (which would surface built-ins like `foreach`,
        // `format`, etc.).
        let src = "string length f\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 15, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // Some f-prefixed command should surface (foreach,
        // format, file…), confirming we fell through.
        assert!(
            labels.iter().any(|l| l.starts_with('f')),
            "expected at least one f-prefixed command via fallback; got {labels:?}",
        );
        // String's `length` subcommand should NOT be in the
        // result — we're past word-index 1.
        assert!(
            !labels.contains(&"length"),
            "should not surface `length` subcommand at word-index 2; got {labels:?}",
        );
    }

    #[test]
    fn subcommand_completion_skipped_when_command_has_no_subcommands() {
        // `puts hel` — `puts` declares no subcommands, so we
        // should fall through to command + proc completion (and
        // ideally find no commands starting with `hel`).
        let src = "proc helper {} {}\nputs hel\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 1, 8, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // The user proc `helper` should surface (fallback path).
        assert!(
            labels.contains(&"helper"),
            "expected `helper` proc via fallback; got {labels:?}",
        );
    }

    // switch completion
    //
    // When the partial starts with `-` and the surrounding
    // command's spec declares matching options, the completion
    // surface lists option names (not subcommands or built-ins).

    #[test]
    fn switch_completion_surfaces_options_for_dash_partial() {
        // `lsearch -n` — partial `-n`.  The `lsearch` spec
        // declares many `-...` options; expect at least one
        // starting with `-n` (`-nocase`).
        let src = "lsearch -n\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 10, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.iter().any(|l| l.starts_with("-n")),
            "expected at least one `-n*` switch; got {labels:?}",
        );
        // No unrelated commands / procs.
        assert!(
            !labels.iter().any(|l| !l.starts_with('-')),
            "all switch-completion items must start with '-'; got {labels:?}",
        );
    }

    #[test]
    fn switch_completion_lists_all_options_for_bare_dash() {
        // `lsearch -` — partial `-`, every option should
        // surface.
        let src = "lsearch -\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 9, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // Every result must start with `-`.
        for l in &labels {
            assert!(l.starts_with('-'), "non-switch in result: {l}");
        }
        // Sorted.
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        assert_eq!(labels, sorted);
    }

    #[test]
    fn switch_completion_gates_options_by_package_version() {
        // `entry -placeholder` was introduced in Tk 8.7.  Completion must gate
        // it on the version resolved from `package require Tk <req>`.
        let registry = CommandRegistry::build_default();
        let older = "package require Tk 8.6\nentry .e -p\n";
        let a1 = analyse(older);
        let l1: Vec<String> = completions(older, 1, 11, &a1, Some(&registry), None, "tcl8.6")
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert!(
            !l1.iter().any(|l| l == "-placeholder"),
            "Tk 8.6 must not offer -placeholder: {l1:?}",
        );

        let newer = "package require Tk 8.7\nentry .e -p\n";
        let a2 = analyse(newer);
        let l2: Vec<String> = completions(newer, 1, 11, &a2, Some(&registry), None, "tcl8.6")
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert!(
            l2.iter().any(|l| l == "-placeholder"),
            "Tk 8.7 must offer -placeholder: {l2:?}",
        );
    }

    #[test]
    fn switch_completion_gates_options_by_dialect() {
        // `lsearch -stride` is Tcl 9.0+ (see the W004 diagnostic for the same
        // gate).  Completion must not suggest it under an older dialect, and
        // must once the document's dialect supports it.
        let registry = CommandRegistry::build_default();
        let src = "lsearch -s {a b} x\n";
        let a = analyse(src);
        let old: Vec<String> = completions(src, 0, 10, &a, Some(&registry), None, "tcl8.6")
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert!(
            !old.iter().any(|l| l == "-stride"),
            "tcl8.6 must not offer -stride: {old:?}",
        );
        let new: Vec<String> = completions(src, 0, 10, &a, Some(&registry), None, "tcl9.0")
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert!(
            new.iter().any(|l| l == "-stride"),
            "tcl9.0 must offer -stride: {new:?}",
        );
    }

    #[test]
    fn switch_completion_resolves_subcommand_scoped_options() {
        // `chan configure -inputmode` is a subcommand-scoped, Tcl 9.0+
        // option that lives only on the `configure` SubCommand's own table —
        // absent from `chan`'s top-level option table entirely.  Completion
        // must resolve the typed subcommand to find it, and still gate it by
        // dialect exactly like W004 does.
        let registry = CommandRegistry::build_default();
        let src = "chan configure $chan -i\n";
        let a = analyse(src);
        let old: Vec<String> = completions(src, 0, 23, &a, Some(&registry), None, "tcl8.6")
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert!(
            !old.iter().any(|l| l == "-inputmode"),
            "tcl8.6 must not offer -inputmode: {old:?}",
        );
        let new: Vec<String> = completions(src, 0, 23, &a, Some(&registry), None, "tcl9.0")
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert!(
            new.iter().any(|l| l == "-inputmode"),
            "tcl9.0 must offer -inputmode: {new:?}",
        );
    }

    #[test]
    fn command_completion_gates_commands_by_package_version() {
        // `ttk::button` needs Tk 8.5. On a tcl8.4 host the shipped Tk is
        // 8.4 (the §7.1 TracksBase pin) and a require cannot raise it —
        // not offered. On a tcl8.6 host the shipped Tk is 8.6 even when
        // the file writes `package require Tk 8.4` (a minimum, not a
        // downgrade) — offered; the old require-only floor wrongly hid it.
        let registry = CommandRegistry::build_default();
        let older = "package require Tk 8.4\nttk::b\n";
        let a1 = analyse(older);
        let l1: Vec<String> = completions(older, 1, 6, &a1, Some(&registry), None, "tcl8.4")
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert!(
            !l1.iter().any(|l| l == "ttk::button"),
            "a tcl8.4 host (Tk 8.4) must not offer ttk::button: {l1:?}",
        );

        let l1_86: Vec<String> = completions(older, 1, 6, &a1, Some(&registry), None, "tcl8.6")
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert!(
            l1_86.iter().any(|l| l == "ttk::button"),
            "a tcl8.6 host ships Tk 8.6; `require Tk 8.4` does not downgrade it: {l1_86:?}",
        );

        let newer = "package require Tk 8.5\nttk::b\n";
        let a2 = analyse(newer);
        let l2: Vec<String> = completions(newer, 1, 6, &a2, Some(&registry), None, "tcl8.6")
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert!(
            l2.iter().any(|l| l == "ttk::button"),
            "Tk 8.5 must offer ttk::button: {l2:?}",
        );
    }

    // iRules event-name completion
    //
    // When the cursor sits at word-index 1 of an event-handler
    // command (the `when` iRules keyword carries
    // `Traits::IS_EVENT_HANDLER`), the completion surface lists
    // event names from the shared `EventRegistry`.

    fn irules_registry() -> CommandRegistry {
        let mut r = CommandRegistry::build_default();
        r.load_dialect(tcl_dialect::DialectSet::IRULES);
        r
    }

    #[test]
    fn event_completion_surfaces_when_handler_first_arg() {
        // `when HT` inside iRules — should list every event
        // starting with `HT` (HTTP_REQUEST, HTTP_RESPONSE, …).
        let src = "when HT\n";
        let analysis = analyse(src);
        let registry = irules_registry();
        let items = completions(src, 0, 7, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"HTTP_REQUEST"),
            "expected `HTTP_REQUEST`; got {labels:?}",
        );
        assert!(
            labels.iter().all(|l| l.starts_with("HT")),
            "all results should start with `HT`; got {labels:?}",
        );
    }

    #[test]
    fn event_completion_marks_detail_as_irules_event() {
        let src = "when CL\n";
        let analysis = analyse(src);
        let registry = irules_registry();
        let items = completions(src, 0, 7, &analysis, Some(&registry), None, "tcl8.6");
        assert!(
            items
                .iter()
                .all(|i| i.detail.as_deref() == Some("F5 iRules event")),
            "every event completion should be labelled as an iRules event; got {items:?}",
        );
    }

    #[test]
    fn event_completion_does_not_fire_in_plain_tcl_dialect() {
        // The default registry doesn't carry the `when` spec —
        // completion should fall through to plain command + proc
        // completion (which won't surface `HTTP_REQUEST`).
        let src = "when HT\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 7, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            !labels.contains(&"HTTP_REQUEST"),
            "event completion should not fire outside iRules; got {labels:?}",
        );
    }

    #[test]
    fn event_completion_skipped_at_word_index_other_than_1() {
        // `when HTTP_REQUEST f` — word-index 2 (after the event
        // name).  Event completion must not fire.
        let src = "when HTTP_REQUEST f\n";
        let analysis = analyse(src);
        let registry = irules_registry();
        let items = completions(src, 0, 19, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            !labels.iter().any(|l| l.contains("HTTP_REQUEST")),
            "event completion should not fire at word-index 2; got {labels:?}",
        );
    }

    // iRules `call PROC_NAME`
    //
    // When the cursor sits at word-index 1 of a command carrying
    // `Traits::INVOKES_USER_PROC` (today only the iRules `call`
    // command), the completion surface lists user-defined proc
    // names and excludes built-in commands.

    #[test]
    fn call_completion_surfaces_user_procs_at_word_index_1() {
        // Two user procs starting with `he`, plus one that
        // doesn't.  Built-in `puts` starts with `p` so won't
        // surface against `he` either way — but we use it as
        // additional cover.
        let src = "proc helper {} {}\nproc help_inner {} {}\nproc unrelated {} {}\ncall he\n";
        let analysis = analyse(src);
        let registry = irules_registry();
        let items = completions(src, 3, 7, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"helper") && labels.contains(&"help_inner"),
            "expected user procs `helper` and `help_inner`; got {labels:?}",
        );
        assert!(
            !labels.contains(&"unrelated"),
            "should filter on partial `he`; got {labels:?}",
        );
        // Every result must be a user proc — built-in commands
        // are excluded from the `call` context.
        for it in &items {
            assert_eq!(
                it.kind,
                CompletionKind::Function,
                "every call-completion item should be a Function; got {it:?}",
            );
        }
    }

    #[test]
    fn call_completion_does_not_fire_in_plain_tcl_dialect() {
        // Plain Tcl registry has no `call` spec — completion
        // should fall through to built-in + proc completion.
        let src = "proc helper {} {}\ncall help\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 1, 9, &analysis, Some(&registry), None, "tcl8.6");
        // The user proc still appears via the fallback path.
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"helper"),
            "fallback should still surface `helper`; got {labels:?}",
        );
    }

    #[test]
    fn call_completion_excludes_builtin_commands() {
        // Built-in `puts` starts with `p`.  In a `call p`
        // context, the completion list should contain user
        // procs starting with `p` but never `puts`.
        let src = "proc parade {} {}\ncall p\n";
        let analysis = analyse(src);
        let registry = irules_registry();
        let items = completions(src, 1, 6, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"parade"), "{labels:?}");
        assert!(
            !labels.contains(&"puts"),
            "`puts` is a built-in; should not surface in `call` context: {labels:?}",
        );
    }

    #[test]
    fn tk_commands_only_offered_when_tk_is_loaded() {
        let registry = CommandRegistry::build_default();

        // Plain `.tcl` (tcl8.6) with no `package require Tk`: Tk widget
        // commands must NOT be offered, even though the registry knows them.
        let src = "butt\n";
        let analysis = analyse(src);
        let items = completions(src, 0, 4, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            !labels.contains(&"button"),
            "Tk `button` must not surface without `package require Tk`: {labels:?}",
        );

        // Once `package require Tk` is declared, `button` becomes available.
        let src = "package require Tk\nbutt\n";
        let analysis = analyse(src);
        let items = completions(src, 1, 4, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"button"),
            "Tk `button` should surface after `package require Tk`: {labels:?}",
        );
    }

    #[test]
    fn tk_commands_offered_in_wish_tk_dialect() {
        // A `wish`-labelled document (`tk` dialect) is implicitly Tk-loaded.
        let registry = CommandRegistry::build_default();
        let src = "butt\n";
        let analysis = analyse(src);
        let items = completions(src, 0, 4, &analysis, Some(&registry), None, "tk");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"button"),
            "Tk `button` should surface in the `tk` dialect: {labels:?}",
        );
    }

    #[test]
    fn tk_commands_never_offered_in_irules() {
        // Even a stray `package require Tk` cannot make Tk valid in iRules —
        // the command is dialect-gated out.
        let registry = CommandRegistry::build_default();
        let src = "package require Tk\nbutt\n";
        let analysis = analyse(src);
        let items = completions(src, 1, 4, &analysis, Some(&registry), None, "f5-irules");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            !labels.contains(&"button"),
            "Tk `button` must never surface in iRules: {labels:?}",
        );
    }

    #[test]
    fn call_completion_skipped_at_word_index_other_than_1() {
        // `call helper extra` — word-index 2.  Completion
        // should fall through to plain command + proc
        // completion (built-ins included).
        let src = "proc helper {} {}\ncall helper e\n";
        let analysis = analyse(src);
        let registry = irules_registry();
        let items = completions(src, 1, 14, &analysis, Some(&registry), None, "tcl8.6");
        // Plain fallback fires — any e-prefixed builtin
        // (`eval`, `exec`, `expr`, `error`…) should surface.
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.iter().any(|l| l.starts_with('e') && *l != "helper"),
            "fallback should surface some e-prefixed built-in at word-index 2; got {labels:?}",
        );
    }

    // -- subcommand argument-value completion ----

    #[test]
    fn string_is_completes_character_classes() {
        // `string is a` — cursor at the class arg (word 2);
        // expect the `a*` classes (alnum, alpha, ascii).
        let src = "string is a\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 11, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"alnum"), "{labels:?}");
        assert!(labels.contains(&"alpha"), "{labels:?}");
        assert!(labels.contains(&"ascii"), "{labels:?}");
        // Every result is an enum value, not a proc / command.
        for it in &items {
            assert_eq!(it.kind, CompletionKind::EnumValue, "{it:?}");
        }
        // Non-`a` classes filtered out.
        assert!(!labels.contains(&"digit"), "{labels:?}");
    }

    #[test]
    fn string_is_lists_all_classes_with_empty_partial() {
        // `string is ` — cursor just past the space, empty
        // partial → all 22 character classes.
        let src = "string is \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 10, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"boolean"), "{labels:?}");
        assert!(labels.contains(&"wordchar"), "{labels:?}");
        assert!(
            labels.len() >= 20,
            "expected the full class set; got {labels:?}"
        );
        // Sorted.
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        assert_eq!(labels, sorted);
    }

    #[test]
    fn string_is_class_detail_is_surfaced() {
        let src = "string is alnum\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 15, &analysis, Some(&registry), None, "tcl8.6");
        let alnum = items.iter().find(|i| i.label == "alnum").expect("alnum");
        assert!(
            alnum
                .detail
                .as_deref()
                .unwrap_or("")
                .contains("alphabet or digit"),
            "{alnum:?}",
        );
    }

    #[test]
    fn subcommand_without_arg_values_falls_through() {
        // `string length x` — `length` has no arg_values, so
        // word-index 2 falls through to plain completion (no
        // enum-value items).
        let src = "string length x\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 15, &analysis, Some(&registry), None, "tcl8.6");
        assert!(
            items.iter().all(|i| i.kind != CompletionKind::EnumValue),
            "no enum-value completions expected; got {items:?}",
        );
    }

    // workspace-index: cross-document proc completion

    #[test]
    fn workspace_procs_surface_in_completion() {
        use crate::workspace_index::WorkspaceIndex;
        // Current doc defines `local_proc`; a sibling doc
        // defines `shared_helper`.  Completing `s` should
        // surface the cross-document proc.
        let cur_src = "proc local_proc {} {}\ns\n";
        let cur = analyse(cur_src);
        let other = analyse("proc shared_helper {} {}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///cur.tcl", &cur),
            ("file:///other.tcl", &other),
        ]);
        let items = completions(cur_src, 1, 1, &cur, None, Some(&index), "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"shared_helper"),
            "expected cross-doc proc; got {labels:?}",
        );
        // It's tagged as a workspace proc in the detail.
        let helper = items.iter().find(|i| i.label == "shared_helper").unwrap();
        assert!(
            helper.detail.as_deref().unwrap_or("").contains("workspace"),
            "{helper:?}",
        );
        assert!(helper.sort_text.as_deref().unwrap_or("").starts_with("C0_"));
    }

    #[test]
    fn workspace_procs_do_not_duplicate_local_procs() {
        use crate::workspace_index::WorkspaceIndex;
        // Both the current doc and the index contain `greet`.
        // The result must list `greet` once (the local entry).
        let cur_src = "proc greet {} {}\ngr\n";
        let cur = analyse(cur_src);
        let index = WorkspaceIndex::from_documents([("file:///cur.tcl", &cur)]);
        let items = completions(cur_src, 1, 2, &cur, None, Some(&index), "tcl8.6");
        let count = items.iter().filter(|i| i.label == "greet").count();
        assert_eq!(count, 1, "{items:?}");
    }

    #[test]
    fn option_enum_value_completion_offers_members() {
        // `button .b -relief ra` — cursor on the value word after a closed-set
        // option offers the relief members, filtered by the partial (Phase 5).
        let src = "button .b -relief ra";
        let cur = analyse(src);
        let registry = CommandRegistry::build_default();
        let col = u32::try_from(src.len()).unwrap();
        let items = completions(src, 0, col, &cur, Some(&registry), None, "tk");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"raised"),
            "expected `raised` among relief completions; got {labels:?}"
        );
    }

    #[test]
    fn workspace_none_keeps_single_doc_behaviour() {
        let cur_src = "proc greet {} {}\ngr\n";
        let cur = analyse(cur_src);
        let items = completions(cur_src, 1, 2, &cur, None, None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["greet"], "{labels:?}");
    }

    /// iRule event snippet labels surfaced at a cursor.
    fn irule_snippet_labels(src: &str, line: u32, character: u32, dialect: &str) -> Vec<String> {
        let analysis = analyse(src);
        let registry = irules_registry();
        completions(
            src,
            line,
            character,
            &analysis,
            Some(&registry),
            None,
            dialect,
        )
        .into_iter()
        .filter(|i| i.label.starts_with("iRule"))
        .map(|i| i.label)
        .collect()
    }

    #[test]
    fn irules_event_snippets_surface_at_top_level() {
        // `irule` partial at a top-level command position in the iRules
        // dialect offers every event template.
        let labels = irule_snippet_labels("irule", 0, 5, "f5-irules");
        assert!(
            labels.iter().any(|l| l == "iRule RULE_INIT"),
            "expected RULE_INIT; got {labels:?}",
        );
        assert!(
            labels.iter().any(|l| l == "iRule HTTP_REQUEST"),
            "expected HTTP_REQUEST; got {labels:?}",
        );
    }

    #[test]
    fn irules_snippets_decline_for_already_declared_event() {
        // `when RULE_INIT { }` already declared → the RULE_INIT template
        // drops out, but other event templates remain.
        let src = "when RULE_INIT {\n}\nirule";
        let labels = irule_snippet_labels(src, 2, 5, "f5-irules");
        assert!(
            !labels.iter().any(|l| l == "iRule RULE_INIT"),
            "RULE_INIT already declared, should decline; got {labels:?}",
        );
        assert!(
            labels.iter().any(|l| l == "iRule HTTP_REQUEST"),
            "HTTP_REQUEST not declared, should remain; got {labels:?}",
        );
    }

    #[test]
    fn irules_event_snippets_suppressed_inside_when_body() {
        // Cursor inside a `when HTTP_REQUEST { }` body → the top-level
        // guard suppresses every event template.
        let src = "when HTTP_REQUEST {\n    irule\n}\n";
        let labels = irule_snippet_labels(src, 1, 9, "f5-irules");
        assert!(
            labels.is_empty(),
            "event templates must not offer inside a when block; got {labels:?}",
        );
    }

    #[test]
    fn irules_snippets_hidden_in_plain_tcl_dialect() {
        // Same source, but the `tcl8.6` dialect carries no iRule
        // templates at all.
        let labels = irule_snippet_labels("irule", 0, 5, "tcl8.6");
        assert!(labels.is_empty(), "got {labels:?}");
    }

    // The shared candidate filter (prefix path + fuzzy fallback).

    #[test]
    fn filter_candidates_prefix_path_keeps_input_order() {
        let FilteredCandidates { candidates, fuzzy } =
            filter_candidates("alpha", vec!["delta", "alpha", "beta", "alphabet"], |n| *n);
        assert!(!fuzzy);
        assert_eq!(candidates, vec!["alpha", "alphabet"]);
    }

    #[test]
    fn filter_candidates_empty_partial_keeps_everything() {
        let FilteredCandidates { candidates, fuzzy } =
            filter_candidates("", vec!["b", "a"], |n| *n);
        assert!(!fuzzy);
        assert_eq!(candidates, vec!["b", "a"]);
    }

    #[test]
    fn filter_candidates_no_fallback_for_one_char_fragment() {
        let FilteredCandidates { candidates, fuzzy } =
            filter_candidates("q", vec!["set", "puts"], |n| *n);
        assert!(!fuzzy);
        assert!(candidates.is_empty());
    }

    #[test]
    fn filter_candidates_fuzzy_ranks_by_distance_then_name() {
        // No candidate prefix-matches `abcdef`; `zbcdef` is one edit
        // away, `abcdxy` two — distance outranks the name order.
        let FilteredCandidates { candidates, fuzzy } =
            filter_candidates("abcdef", vec!["abcdxy", "zbcdef", "qqqqqq"], |n| *n);
        assert!(fuzzy);
        assert_eq!(candidates, vec!["zbcdef", "abcdxy"]);
    }

    #[test]
    fn filter_candidates_fuzzy_cap_respected() {
        // Nine candidates all one edit from `az` — the fallback caps
        // at MAX_FUZZY_SUGGESTIONS, keeping the name-order head.
        let names = vec!["aa", "ab", "ac", "ad", "ae", "af", "ag", "ah", "ai"];
        let FilteredCandidates { candidates, fuzzy } = filter_candidates("az", names, |n| *n);
        assert!(fuzzy);
        assert_eq!(candidates.len(), MAX_FUZZY_SUGGESTIONS);
        assert_eq!(candidates[0], "aa");
        assert!(!candidates.contains(&"ai"), "cap should drop the tail");
    }

    #[test]
    fn filter_candidates_fuzzy_respects_distance_budget() {
        // `xyzzy` is beyond the scaled edit budget of every candidate.
        let FilteredCandidates { candidates, fuzzy } =
            filter_candidates("xyzzy", vec!["set", "puts"], |n| *n);
        assert!(!fuzzy);
        assert!(candidates.is_empty());
    }

    // Pinned prefix lists — the byte-identical contract for fragments
    // that match something.

    #[test]
    fn prefix_proc_list_pinned() {
        let src = "proc alpha {} {}\nproc beta {} {}\nal\n";
        let analysis = analyse(src);
        let items = completions(src, 2, 2, &analysis, None, None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["alpha"]);
        assert_eq!(items[0].insert_text, "::alpha");
        assert_eq!(
            items[0].filter_text, None,
            "prefix matches stay undecorated"
        );
    }

    #[test]
    fn prefix_subcommand_list_pinned() {
        let src = "string to\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 9, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["tolower", "totitle", "toupper"]);
        assert!(items.iter().all(|i| i.filter_text.is_none()));
    }

    // Fuzzy fallback behaviour through the public entry point.

    #[test]
    fn fuzzy_fallback_offers_lsearch_for_lsaerch() {
        let src = "lsaerch\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 7, &analysis, Some(&registry), None, "tcl8.6");
        let lsearch = items
            .iter()
            .find(|i| i.label == "lsearch")
            .unwrap_or_else(|| panic!("expected fuzzy lsearch, got {items:?}"));
        // Decorated so editors neither hide nor re-order the item.
        assert_eq!(lsearch.filter_text.as_deref(), Some("lsaerch"));
        assert_eq!(lsearch.sort_text.as_deref(), Some("F00_lsearch"));
    }

    #[test]
    fn fuzzy_fallback_skips_one_char_fragment() {
        // `q` prefix-matches no command and is below the fragment
        // minimum — the response stays empty rather than fuzzing.
        let src = "q\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 1, &analysis, Some(&registry), None, "tcl8.6");
        assert!(items.is_empty(), "got {items:?}");
    }

    #[test]
    fn fuzzy_subcommand_offers_length_for_lenght() {
        let src = "string lenght\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 13, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["length"], "typo'd subcommand fragment");
        assert_eq!(items[0].filter_text.as_deref(), Some("lenght"));
    }

    #[test]
    fn fuzzy_switch_offers_nocase_for_ncoase() {
        let src = "lsort -ncoase\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 13, &analysis, Some(&registry), None, "tcl8.6");
        let nocase = items
            .iter()
            .find(|i| i.label == "-nocase")
            .unwrap_or_else(|| panic!("expected fuzzy -nocase, got {items:?}"));
        // The filter text carries the dash, matching the replaced range.
        assert_eq!(nocase.filter_text.as_deref(), Some("-ncoase"));
        let edit = nocase.text_edit.as_ref().expect("switch replace edit");
        assert_eq!(edit.new_text, "-nocase");
    }

    #[test]
    fn fuzzy_variable_offers_banana_for_bnaana() {
        let src = "set apple 1\nset banana 2\nputs $bnaana\n";
        let analysis = analyse(src);
        let items = completions(src, 2, 12, &analysis, None, None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["$banana"], "typo'd variable fragment");
        // Sigil included, matching the replaced `$…` range.
        assert_eq!(items[0].filter_text.as_deref(), Some("$bnaana"));
        let edit = items[0].text_edit.as_ref().expect("variable replace edit");
        assert_eq!(edit.new_text, "$banana");
    }

    #[test]
    fn fuzzy_array_element_offers_key_for_kye() {
        let src = "set arr(key) 1\nset v $arr(kye\n";
        let analysis = analyse(src);
        let items = completions(src, 1, 14, &analysis, None, None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["$arr(key)"], "typo'd array index");
        assert_eq!(items[0].filter_text.as_deref(), Some("$arr(kye"));
    }

    #[test]
    fn fuzzy_method_surfaces_when_response_would_be_empty() {
        // `$d brk` — no method prefix-matches and no command comes
        // within the edit budget, so the response-level fallback offers
        // the instance's `bark`.
        let src = "oo::class create Dog {\n    method bark {} {}\n    method beg {} {}\n}\nset d [Dog new]\n$d brk\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 5, 6, &analysis, Some(&registry), None, "tcl8.6");
        let bark = items
            .iter()
            .find(|i| i.label == "bark")
            .unwrap_or_else(|| panic!("expected fuzzy bark, got {items:?}"));
        assert!(
            bark.detail.as_deref().unwrap_or("").starts_with("method"),
            "method item expected, got {:?}",
            bark.detail,
        );
    }

    #[test]
    fn method_typo_does_not_hijack_command_prefix_matches() {
        // `$d xy` — no method prefix-matches, but the proc `xyz` does;
        // the fall-through command/proc list must stay exactly as
        // before (no fuzzy methods creep in alongside it).
        let src = "oo::class create Dog {\n    method xx {} {}\n}\nproc xyz {} {}\nset d [Dog new]\n$d xy\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 5, 5, &analysis, Some(&registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"xyz"), "prefix proc expected: {labels:?}");
        assert!(
            !labels.contains(&"xx"),
            "fuzzy method must not pad a prefix-matching response: {labels:?}",
        );
        assert!(items.iter().all(|i| i.filter_text.is_none()));
    }

    #[test]
    fn invoked_proc_completions_fuzzy_falls_back() {
        let src = "proc greet {} {}\nproc shout {} {}\n";
        let analysis = analyse(src);
        // Prefix path unchanged…
        let items = invoked_proc_completions(&analysis, "gre");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["greet"]);
        assert!(items[0].filter_text.is_none());
        // …and the typo'd fragment falls back to the closest proc.
        let items = invoked_proc_completions(&analysis, "gret");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["greet"]);
        assert_eq!(items[0].filter_text.as_deref(), Some("gret"));
    }

    // Dialect-profile availability (dialect-profile-model.md, Milestone 4):
    // the §5.1 available_subcommands gap and the §5.2 option semantics in
    // completion.

    #[test]
    fn subcommand_completion_is_version_gated_by_the_profile() {
        let src = "dict \n";
        let analysis = analyse(src);
        let registry = tcl_registry::registry_for_dialect("tcl8.6");
        // 8.6 buffer: 9.0-only subcommands are not offered.
        let items = completions(src, 0, 5, &analysis, Some(registry), None, "tcl8.6");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"get"), "{labels:?}");
        assert!(
            !labels.contains(&"getwithdefault"),
            "dict getwithdefault is 9.0+ and must not be offered under tcl8.6: {labels:?}"
        );
        // 9.0 buffer: it is offered.
        let registry90 = tcl_registry::registry_for_dialect("tcl9.0");
        let items90 = completions(src, 0, 5, &analysis, Some(registry90), None, "tcl9.0");
        let labels90: Vec<&str> = items90.iter().map(|i| i.label.as_str()).collect();
        assert!(labels90.contains(&"getwithdefault"), "{labels90:?}");
    }

    #[test]
    fn command_completion_uses_profile_availability() {
        let src = "dic\n";
        let analysis = analyse(src);
        // iApps embed Tcl 8.5: dict IS offered (the composed-mask fix)…
        let registry = tcl_registry::registry_for_dialect("f5-iapps");
        let items = completions(src, 0, 3, &analysis, Some(registry), None, "f5-iapps");
        assert!(
            items.iter().any(|i| i.label == "dict"),
            "dict must be offered under f5-iapps (Tcl 8.5.13 host)"
        );
        // …while the banned iRules commands never surface there.
        let irules_reg = tcl_registry::registry_for_dialect("f5-irules");
        let exec_src = "exe\n";
        let exec_analysis = analyse(exec_src);
        let irules_items = completions(
            exec_src,
            0,
            3,
            &exec_analysis,
            Some(irules_reg),
            None,
            "f5-irules",
        );
        assert!(
            !irules_items.iter().any(|i| i.label == "exec"),
            "exec is banned in iRules and must not be completed"
        );
    }

    #[test]
    fn switch_option_completion_respects_profile_gating() {
        // Option completion under f5-iapps offers the 8.5+ -nocase (the old
        // contains rule dropped every version-gated option under a composed
        // mask) but not the 9.0-only regsub -command.
        let src = "switch -\n";
        let analysis = analyse(src);
        let registry = tcl_registry::registry_for_dialect("f5-iapps");
        let items = completions(src, 0, 8, &analysis, Some(registry), None, "f5-iapps");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"-nocase"),
            "switch -nocase is 8.5+ core, offered under f5-iapps: {labels:?}"
        );

        let src9 = "regsub -\n";
        let analysis9 = analyse(src9);
        let items9 = completions(src9, 0, 8, &analysis9, Some(registry), None, "f5-iapps");
        let labels9: Vec<&str> = items9.iter().map(|i| i.label.as_str()).collect();
        assert!(
            !labels9.contains(&"-command"),
            "regsub -command is 9.0-only, hidden under f5-iapps: {labels9:?}"
        );
    }
}
