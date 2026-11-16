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

//! Shared helpers for the analyser diagnostics families.
//!
//! Free functions and small types used by more than one diagnostic family
//! (or by both the per-function dispatcher in the module root and a family):
//! source-slice extraction, dotted-quad scanning shared between the
//! subnet-mask and invalid-IP checks, the substitution predicates shared by source usage checks, the
//! defined-variable / existence-guard / globals-written collectors consumed
//! by the read-before-set machinery, and the [`UndefSuppression`] context
//! plus its phi-undef index that the dataflow read-before-set emitters
//! consult.

#[cfg(test)]
use std::collections::HashSet;

#[cfg(test)]
use rustc_hash::FxHashSet;

mod existence_guards;
mod hidden_writes;
mod original_cells;
mod phi_undef;
mod undef_suppression;

pub(super) use existence_guards::{
    ExistenceGuard, block_dominated_by, collect_condition_writes, collect_existence_guards,
    original_condition_read_is_guarded,
};
#[cfg(test)]
use hidden_writes::{collect_expr_cmd_sub_writes, collect_script_concat_writes};
pub(super) use original_cells::{
    DiagnosticCellFacts, collect_defined_vars, definition_has_cell_fact, globals_read_by_procs,
    globals_written_by_procs, original_array_scalar_read_at_span, original_definition_places,
    original_read_cell, original_read_cells_at, original_read_is_live_array_scalar,
    original_read_occurrence_at_span, read_has_cell_fact, special_variable_definition,
    special_variable_definition_at_span,
};
pub(super) use phi_undef::{
    MayDefMap, PhiBlockMap, PhiDefMap, PhiUndefCtx, PhiUndefMemo, UndefIndexMaps,
    build_phi_undef_index, phi_can_undef, startup_cell_binding,
};
#[cfg(test)]
pub(super) use phi_undef::{call_steps, has_global_startup_binding};
pub(super) use undef_suppression::{
    UndefSuppression, UndefSuppressionSemantics, build_undef_suppression,
};
#[cfg(test)]
use undef_suppression::{collect_qualified_variable_alias_tails, harvest_dict_with_suppression};

/// Find a case-insensitive match for `variable` in `defined_vars`.
///
/// The source text covered by `span`, or `None` when the span is out
/// of bounds / not on char boundaries.
pub(super) fn source_slice(source: &str, span: tcl_lexer::Span) -> Option<String> {
    let start = span.start() as usize;
    let end = span.end() as usize;
    if start <= end && end <= source.len() {
        source.get(start..end).map(str::to_owned)
    } else {
        None
    }
}

/// A `\w` byte: ASCII alphanumeric or underscore (word boundary basis).
pub(super) fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// One dotted-quad match found in a value: the four octet substrings and
/// the byte offset where it begins (for context checks like a preceding
/// `/`).
pub(super) struct DottedQuad<'a> {
    pub(super) octets: [&'a str; 4],
    pub(super) start: usize,
    /// Byte offset just past the final octet (the regex `m.end()`).
    pub(super) end: usize,
}

/// Find every `\b\d{1,N}.\d{1,N}.\d{1,N}.\d{1,N}\b` dotted quad in
/// `text` (non-overlapping, left-to-right), replacing the regex scan.
/// `max_digits` caps each octet's digit count (`3` for the subnet-mask
/// check, `4` for the invalid-IP one).  Each octet starts at a word
/// boundary, so a longer digit run (a 4th/5th digit) simply fails to
/// align with the following `.` and is skipped — matching the regex.
pub(super) fn find_dotted_quads(text: &str, max_digits: usize) -> Vec<DottedQuad<'_>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let boundary_before = i == 0 || !is_word_byte(bytes[i - 1]);
        if boundary_before && let Some((octets, end)) = match_dotted_quad(text, i, max_digits) {
            out.push(DottedQuad {
                octets,
                start: i,
                end,
            });
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// Match a dotted quad starting at byte `start` (a word boundary), each
/// octet `1..=max_digits` digits separated by `.`, requiring a trailing
/// word boundary.  Returns the octet substrings and the end offset.
fn match_dotted_quad(text: &str, start: usize, max_digits: usize) -> Option<([&str; 4], usize)> {
    let bytes = text.as_bytes();
    let mut pos = start;
    let mut octets: [&str; 4] = [""; 4];
    for (k, slot) in octets.iter_mut().enumerate() {
        let run_start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        let len = pos - run_start;
        if len == 0 || len > max_digits {
            return None;
        }
        *slot = &text[run_start..pos];
        if k < 3 {
            if bytes.get(pos) != Some(&b'.') {
                return None;
            }
            pos += 1; // consume the dot
        }
    }
    // Trailing `\b`: end of string or a non-word byte.
    if pos < bytes.len() && is_word_byte(bytes[pos]) {
        return None;
    }
    Some((octets, pos))
}

/// True when `text` carries a substitution (`$` / `[`) or `tok` is a
/// `Var` / `Cmd` token.
pub(in crate::analyser) fn has_substitution(text: &str, tok: &tcl_lexer::Token) -> bool {
    has_substitution_of_kind(text, tok.kind)
}

/// [`has_substitution`] for a consumer that holds the word's token *kind*
/// without the token itself — the IR's `CommandTokens` records `argv_kinds`
/// alongside `argv_texts`, with no `Token` to hand.  The one predicate both
/// spellings share, so a change to what counts as a substitution reaches
/// every static-word check at once.
pub(in crate::analyser) fn has_substitution_of_kind(
    text: &str,
    kind: tcl_lexer::TokenType,
) -> bool {
    text.contains('$')
        || text.contains('[')
        || matches!(kind, tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd)
}

/// The safety class of a "wrap this word in braces" quick-fix (W100's
/// unbraced expression, W105's unbraced code block).
///
/// Bracing is the recommended form because it stops Tcl substituting the
/// word *before* the command sees it.  That is exactly why it cannot be
/// classified once for the whole diagnostic code: where the
/// written word carries no substitution, brace-quoting reaches the command
/// with byte-identical text and nothing observable changes; where it does,
/// the fix deliberately removes a round of substitution and a program that
/// depended on it changes behaviour.  C Tcl 9.0.3:
///
/// ```tcl
/// set a {$x}; set x 3; set b 2
/// puts [expr $a + $b]      ;# 5  — `$a` substitutes to `$x`, then expr
/// puts [expr {$a + $b}]    ;# error: `$a` is the string `$x`
/// ```
///
/// Equivalence therefore requires the written word to be substitution-free
/// on *every* mechanism the outer parse applies:
///
/// * no `$` / `[` and no whole-word `Var` / `Cmd` token — the caller passes
///   this as *`has_substitution`*, since W100 and W105 each already compute
///   it (W100 over a joined argument run, W105 over one body word);
/// * no backslash — the outer parse decodes `\n`, `\t`, `\x41`, and a
///   line continuation, so the braced text is not the text that reached
///   the command before;
/// * no `"` — a quoted word is stripped of its quotes by the outer parse,
///   so brace-quoting it either keeps them as literal characters or (where
///   the emitter strips them) depends on the word being exactly one quoted
///   run, which a `"a" eq "b"` argument list is not.
pub(in crate::analyser) fn brace_wrap_fix_safety(
    text: &str,
    has_substitution: bool,
) -> crate::analyser::types::FixSafety {
    use crate::analyser::types::FixSafety;
    if has_substitution || text.contains('\\') || text.contains('"') {
        FixSafety::BehaviourHardening
    } else {
        FixSafety::SemanticsEquivalent
    }
}

/// Whether one word of a [`tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS`]
/// tail contributes *statically-known script text* to the `Tcl_ConcatObj`
/// join — the one predicate every eval-family static-tail check shares
/// (`utils::concat_script_window` and source-owned diagnostic tails), so what counts
/// as static moves everywhere at once.
///
/// A braced (`Str`) word always does: the braces blocked every outer
/// substitution, so its contents reach the join byte-for-byte — a `$` or
/// `[` inside is script *text* that the eval-family command itself
/// resolves when the joined script runs, not a value consumed before it
/// (tclsh8.6.14 / tclsh9.0.4: `set value 5; eval {set l2} {$value};
/// puts $l2` → `5`).
///
/// Any other word is static only when nothing runs before the join: no
/// `$`/`[` substitution ([`has_substitution_of_kind`], which also rejects
/// whole-word `Var`/`Cmd` tokens), no backslash (the outer parse decodes
/// it, so the joined text is not the written text), and no quote byte
/// (quoting is consumed by the outer parse, never carried into the join).
/// A `{*}` expansion prefix restructures the words entirely and is never
/// static.
pub(in crate::analyser) fn word_is_static_script_text(
    text: &str,
    kind: tcl_lexer::TokenType,
) -> bool {
    match kind {
        tcl_lexer::TokenType::Str => true,
        tcl_lexer::TokenType::Expand => false,
        _ => !has_substitution_of_kind(text, kind) && !text.contains(['\\', '"']),
    }
}

/// An identifier-continuation byte: ASCII alphanumeric, `_`, or `:` (the
/// namespace-separator byte).
pub(super) fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b':'
}

#[cfg(test)]
mod exact_cell_diagnostic_tests;

#[cfg(test)]
mod original_resolved_variable_advice_tests;
