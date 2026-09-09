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

//! Variable-reference scans over emitted or original source text.
//!
//! Two spellings name a variable, and a consumer that asks about one must
//! usually ask about the other too:
//!
//! - a substitution, `$var` / `${var}`, counted by [`count_var_refs`];
//! - a bare name in a command's variable-name position — `info exists x`,
//!   `[set x]`, `upvar … x` — counted by [`bareword_occurrences`].
//!
//! The registry answers the bare-name question exactly for a command the
//! lowerer resolved, through the `ArgRole::VarRead` / `VarWrite` positions it
//! records on `Statement::Call`. These scans are the conservative backstop
//! for the text the IR does not decompose into statements, chiefly a nested
//! command substitution inside an argument word. They over-report rather than
//! under-report, because every caller uses a positive answer to decline a
//! rewrite.

/// Number of `$var` / `${var}` references in `text` (word-bounded).
#[must_use]
pub fn count_var_refs(text: &str, var: &str) -> usize {
    let bytes = text.as_bytes();
    let dollar = format!("${var}");
    let mut n = 0;
    let mut from = 0;
    while let Some(rel) = text[from..].find(&dollar) {
        let pos = from + rel;
        let after = pos + dollar.len();
        // `${var}` form: a `{` immediately after `$`, matched separately below.
        let boundary = bytes
            .get(after)
            .is_none_or(|b| !(b.is_ascii_alphanumeric() || *b == b'_'));
        if boundary {
            n += 1;
        }
        from = pos + 1;
    }
    let braced = format!("${{{var}}}");
    n += text.matches(&braced).count();
    n
}

/// Count occurrences of `var` as a standalone bareword in `text` — word
/// boundaries on both sides, **not** a `$var` / `${var}` substitution, and
/// **not** inside a `"…"` quoted string or `{…}` braced literal. Used to
/// detect by-name reads the `$var` scan misses (`info exists x`, a `[set x]`
/// command substitution); occurrences of the name as literal text inside a
/// string (`"x=$x"`) or braces are not reads and must not count.
#[must_use]
pub fn bareword_occurrences(text: &str, var: &str) -> usize {
    let bytes = text.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut n = 0;
    let mut from = 0;
    while let Some(rel) = text[from..].find(var) {
        let pos = from + rel;
        from = pos + 1;
        // Word boundary after.
        let after = pos + var.len();
        if bytes.get(after).is_some_and(|b| is_word(*b)) {
            continue;
        }
        // Word boundary before, and not a `$`-substitution form.
        match pos.checked_sub(1).and_then(|i| bytes.get(i)).copied() {
            Some(b'$') => continue,                                       // `$var`
            Some(b'{') if pos >= 2 && bytes[pos - 2] == b'$' => continue, // `${var`
            Some(b) if is_word(b) => continue,                            // part of a longer word
            _ => {}
        }
        // Skip occurrences inside a `"…"` string or `{…}` braces — there the
        // name is literal text, not a variable read. (Command substitutions
        // `[…]` are *not* skipped: `[set x]` / `[info exists x]` read by name.)
        if in_string_or_braces(bytes, pos) {
            continue;
        }
        n += 1;
    }
    n
}

/// Whether byte offset `pos` in `text` lies inside a `"…"` quoted string or
/// `{…}` braces. A conservative scan from the start: an unescaped `"` toggles
/// quote state (only while not inside braces), and `{`/`}` track brace depth
/// (only while not inside a quote). Over-counting on pathological input keeps
/// the def (safe); the common cases (`"x=$x"`, `{$x}`) are handled.
fn in_string_or_braces(bytes: &[u8], pos: usize) -> bool {
    let mut in_quote = false;
    let mut brace_depth = 0u32;
    let mut i = 0;
    while i < pos {
        match bytes[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'"' if brace_depth == 0 => in_quote = !in_quote,
            b'{' if !in_quote => brace_depth += 1,
            b'}' if !in_quote && brace_depth > 0 => brace_depth -= 1,
            _ => {}
        }
        i += 1;
    }
    in_quote || brace_depth > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bareword_occurrences_skips_string_and_brace_literals() {
        // `info exists x` / bare `x` command word → counted (by-name read).
        assert_eq!(bareword_occurrences("info exists x", "x"), 1);
        // Literal text inside quotes / braces → not counted.
        assert_eq!(bareword_occurrences("puts \"x=$x\"", "x"), 0);
        assert_eq!(bareword_occurrences("puts {x marks}", "x"), 0);
        // A `[set x]` command substitution still counts (reads by name).
        assert_eq!(bareword_occurrences("puts [set x]", "x"), 1);
    }

    #[test]
    fn bareword_occurrences_needs_whole_word_boundaries() {
        assert_eq!(bareword_occurrences("info exists xy", "x"), 0);
        assert_eq!(bareword_occurrences("info exists ax", "x"), 0);
        assert_eq!(bareword_occurrences("puts $x", "x"), 0);
        assert_eq!(bareword_occurrences("puts ${x}", "x"), 0);
    }

    #[test]
    fn count_var_refs_counts_both_substitution_spellings() {
        assert_eq!(count_var_refs("puts $x", "x"), 1);
        assert_eq!(count_var_refs("puts ${x}", "x"), 1);
        assert_eq!(count_var_refs("puts \"$x $x\"", "x"), 2);
        // A longer name that merely starts with the same bytes is not a
        // reference to it.
        assert_eq!(count_var_refs("puts $xy", "x"), 0);
    }
}
