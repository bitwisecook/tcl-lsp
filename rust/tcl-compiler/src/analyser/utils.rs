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

//! Pure analyser helpers.
//!
//! Pure free functions used by handlers and diagnostic emitters
//! across the analyser. `parse_param_list` is re-exported from
//! `signature_scan::params` so the analyser can keep its imports
//! flat.

#[cfg(test)]
use std::collections::HashMap;
use std::collections::HashSet;

use tcl_lexer::{SourceMap, Span, Token};
#[cfg(test)]
use tcl_registry::ArgRole;
use tcl_registry::{CommandRegistry, Traits};

use crate::ir::Statement;

pub use crate::signature_scan::params::{
    param_name_spans, param_name_spans_for_token, parse_param_list,
};

mod directives;
mod known_commands;
mod script_comments;
mod stub_directives;

#[cfg(test)]
pub use directives::parse_noqa_line_suppressions;
pub use directives::{
    FILE_SUPPRESS_KEY, apply_preceding_noqa, line_suppressed, parse_file_suppression,
    parse_noqa_line_suppressions_for_dialect, parse_noqa_line_suppressions_from_analysis,
    parse_noqa_marker, parse_provides_directives, parse_supports_directives,
};
pub use known_commands::{
    RecoveryKnownCommands, SharedNameSet, insert_qualified_and_tail, proc_shadows_call,
    recovery_known_commands,
};
pub use script_comments::{
    ScriptCommentFact, original_script_command_comment_facts, original_script_comment_facts,
    script_command_comment_facts_from_analysis, script_comment_facts,
    script_comment_facts_from_analysis,
};
pub use stub_directives::{
    document_declared_surface, document_stub_declarations, document_stub_declarations_with_outside,
    has_sidecar_stubs, outside_stub_diagnostics, scan_sidecar_stubs, scan_source_for_stubs,
    scan_source_for_stubs_with_outside, scan_stub_command_names, workspace_procedure_declarations,
};
#[cfg(test)]
use stub_directives::{parse_command_stub, parse_expr_stub, strip_tcl_lsp_prefix};

/// Decoration-character set for the body-docstring scanner
/// (``.-=*~#``).
const DECORATION_CHARS: &[char] = &['.', '-', '=', '*', '~', '#'];

/// Extract a leading comment block from a proc body — the
/// fallback that fires when there's no preceding-comment harvest
/// from the segmenter.
///
/// Lines containing only decoration characters
/// (``.-=*~#``) are skipped; remaining ``#``-prefixed lines have
/// the leading hash + whitespace stripped, then accumulated.
/// The first non-comment / non-blank line ends the block.
#[must_use]
pub fn extract_body_docstring(body: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for raw in body.lines() {
        let stripped = raw.trim();
        if stripped.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        if !stripped.starts_with('#') {
            break;
        }
        let text = stripped.trim_start_matches('#').trim().to_string();
        // Skip pure-hash decoration lines (``####`` etc.).
        if text.is_empty() && stripped.chars().all(|c| c == '#') {
            continue;
        }
        // Skip lines made entirely of decoration characters.
        if !text.is_empty() && text.chars().all(|c| DECORATION_CHARS.contains(&c)) {
            continue;
        }
        lines.push(text);
    }
    lines.join("\n")
}

/// Format a literal value for inclusion in a diagnostic message.
///
/// Replaces newlines with `\n` (literal two characters), and
/// truncates strings longer than 40 characters to `…[37 chars]…`.
#[must_use]
pub fn format_literal_for_message(value: &str) -> String {
    let display = value.replace('\n', "\\n");
    if display.chars().count() > 40 {
        // Slice at character (not byte) boundaries — Tcl source is
        // usually ASCII but multibyte diagnostic messages are
        // valid.
        let truncated: String = display.chars().take(37).collect();
        format!("{truncated}...")
    } else {
        display
    }
}

/// Return `(variable, static_value)` for assignments worth the
/// W214 paste-error heuristic check.
///
/// Only [`Statement::AssignConst`] and [`Statement::AssignValue`]
/// shapes qualify; `AssignValue` rejects values containing
/// `$` / `[` / `]` (interpolation / command substitution
/// disqualify the fingerprint).
#[must_use]
pub fn possible_paste_fingerprint(stmt: &Statement) -> Option<(String, String)> {
    match stmt {
        Statement::AssignConst { name, value, .. } => {
            Some((name.clone(), value.trim().to_string()))
        }
        Statement::AssignValue { name, value, .. } => {
            let value = value.trim();
            if value.is_empty() {
                return None;
            }
            if value.contains('$') || value.contains('[') || value.contains(']') {
                return None;
            }
            Some((name.clone(), value.to_string()))
        }
        _ => None,
    }
}

/// Return argv tokens widened to each Tcl word's full token span.
///
/// The segmenter already returns argv tokens with word-wide spans
/// (see `rust/tcl-compiler/src/segmenter.rs`), so this is the
/// identity function. The helper is kept as a transparent
/// passthrough so the indirection layer at the call sites stays
/// intact.
#[must_use]
pub fn argv_with_word_spans(argv: Vec<Token>, _all_tokens: &[Token]) -> Vec<Token> {
    argv
}

/// Full source span of one Tcl word, as defined by the lexer.
///
/// [`tcl_lexer::word_span`] is the authoritative delimiter policy: it handles
/// braces, brackets, quotes, empty words, escaped closers, and recovery
/// tokens without each analyser diagnostic reimplementing byte arithmetic.
///
/// Builds a [`SourceMap`] per call, and [`SourceMap::new`] rescans the whole
/// document to construct its line index. That is fine for a one-off caller,
/// but a caller that runs **per command or per word** must hoist the map and
/// use [`full_word_span_in`] instead — see its doc comment.
#[must_use]
pub fn full_word_span(token: Token, source: &str) -> Span {
    full_word_span_in(&SourceMap::new(source), token)
}

/// [`full_word_span`] against an already-built [`SourceMap`].
///
/// Same answer, without the rebuild. `SourceMap::new` scans the entire
/// document to populate its line index, so calling [`full_word_span`] once per
/// *word* makes span resolution `O(document size × word count)` — the same
/// quadratic that [`crate::analyser::state::Analyser::cached_line_index`] was
/// introduced to remove for the per-command `SourceMap::new(&self.source)`
/// call sites. On a ~5k-line file those rebuilds dominate whole-file analysis.
///
/// Callers inside the analyser should build the map once with
/// `Analyser::source_map(&self.source, &self.cached_line_index,
/// self.cached_line_index_source_len)`, hoisted out of any loop over words.
#[must_use]
pub fn full_word_span_in(source_map: &SourceMap<'_>, token: Token) -> Span {
    tcl_lexer::word_span(source_map, token)
}

/// The set of iRules commands that may only appear at the top
/// level of an event handler, computed fresh from the supplied
/// `registry`.
///
/// Reads the `IRULES_TOP_LEVEL_ONLY` trait from `registry` and
/// returns the matching command names.
///
/// **Not cached.** Caching the first-call result in a static
/// `OnceLock` silently returns stale data once a non-default
/// `CommandRegistry` is passed on a later call. The trait scan is `O(n)` over the registry's
/// command specs (~150 entries) and the registry itself is
/// already cached at the call sites that need this — so per-call
/// recomputation is cheap in practice and removes a correctness
/// trap. Callers that want caching should cache at their own
/// layer, keyed by whatever identity makes sense for them.
#[must_use]
pub fn irules_top_level_only(registry: &CommandRegistry) -> HashSet<String> {
    registry
        .commands_with_trait(Traits::IRULES_TOP_LEVEL_ONLY)
        .into_iter()
        .map(String::from)
        .collect()
}

/// The exact source window a [`Traits::SCRIPT_CONCATENATES_ARGS`] tail
/// evaluates — every content byte at its true offset, every structural
/// byte blanked to a space — or `None` when the script is not statically
/// knowable.
///
/// `Tcl_ConcatObj` (`generic/tclUtil.c`) is the one rule the whole eval
/// family shares: take each word's **string representation**, trim ASCII
/// whitespace from both ends of it, drop the ones that end up empty, and
/// join the rest with a single space. A braced word contributes its
/// *contents* — Tcl's word parsing has already removed the outer braces
/// before `Tcl_ConcatObj` ever sees the value — so braced and bare words
/// concatenate identically and a command may span word boundaries.
///
/// Joining the trimmed texts into a fresh string reproduces those bytes
/// but loses their *positions*: stripped delimiters, dropped words, and
/// collapsed whitespace shift every byte after the first divergence, so a
/// span recorded while walking the joined text can land on the wrong
/// source bytes (in `eval {} foo` the reconstructed `foo` starts at the
/// empty word's closing brace, and a rename would edit that delimiter
/// instead of the command). So this builds the equivalent script *in
/// place* instead: a buffer covering the words' whole span in `source`,
/// holding each word's content bytes at their real offsets and a space
/// everywhere else (delimiters, inter-word gaps). Script parsing treats
/// any whitespace run as one separator, so the window parses into exactly
/// the words the trimmed-and-joined text would — `Tcl_ConcatObj`'s trim,
/// drop, and single-space steps all degenerate to "more whitespace" —
/// while every span the walk records maps to the source bytes it
/// describes.
///
/// Pinned against tclsh8.6.14 and tclsh9.0.4:
///
/// ```text
/// eval set l2 hello        ≡ set l2 hello
/// eval {set l2} hello      ≡ set l2 hello   (braces gone, contents kept)
/// eval "set q" 7           ≡ set q 7        (quotes gone; a space inside
///                                            a quoted word separates —
///                                            the re-parse splits it just
///                                            as the join would)
/// eval {set l2} {$value}   ≡ set l2 $value  (braces blocked the outer
///                                            substitution, so the script
///                                            text is static; `eval`
///                                            itself resolves `$value`
///                                            when the script runs)
/// eval {} foo              ≡ foo            (an empty word is just more
///                                            whitespace)
/// ```
///
/// Returns `None` when any word is not statically-known script text
/// ([`super::diagnostics::helpers::word_is_static_script_text`]: a
/// substitution or a decode runs before concatenation, so the real script
/// is unknowable), when a word's text does not sit verbatim at its
/// token's content offset (bytes that cannot be mapped must not be
/// analysed), or when the window holds no content at all. Callers must
/// then consume the command without walking it rather than guess.
///
/// This is **not** the rule for `namespace inscope`, whose tail is appended
/// as list elements — see [`Traits::SCRIPT_APPENDS_LIST_ARGS`].
#[must_use]
pub fn concat_script_window(
    words: &[String],
    tokens: &[Token],
    source: &str,
) -> Option<(String, tcl_lexer::Span)> {
    let (first, last) = (tokens.first()?, tokens.last()?);
    if words.len() != tokens.len() {
        return None;
    }
    let start = first.span.start() as usize;
    let end = last.span.end() as usize;
    if end <= start || source.len() < end {
        return None;
    }
    let mut window = vec![b' '; end - start];
    for (word, tok) in words.iter().zip(tokens) {
        if !crate::analyser::diagnostics::helpers::word_is_static_script_text(word, tok.kind) {
            return None;
        }
        let content_start = tok.span.start() as usize + usize::from(tok.content_offset);
        // The word's text must sit verbatim at its content offset — any
        // divergence (an escape decode, a token shape this fn does not
        // model) makes the bytes unmappable, and unmappable bytes must
        // not be walked.
        if source.get(content_start..content_start + word.len()) != Some(word.as_str()) {
            return None;
        }
        let off = content_start.checked_sub(start)?;
        window
            .get_mut(off..off + word.len())?
            .copy_from_slice(word.as_bytes());
    }
    if window.iter().all(u8::is_ascii_whitespace) {
        return None;
    }
    let window = String::from_utf8(window).ok()?;
    Some((
        window,
        tcl_lexer::Span::new(u32::try_from(start).ok()?, u32::try_from(end).ok()?),
    ))
}

#[cfg(test)]
mod concat_script_window_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_configured_recovery_tests;
