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

//! Error messages translated back from minified names and line numbers.

use super::command_rendering::utf8_len;
use super::symbol_map::SymbolMap;

/// Translate a Tcl / iRule error message from minified names back
/// to the originals using `symbol_map`.  Replaces `$short` and
/// `"short"` occurrences; the source-correlated line remapping is
/// handled separately by [`remap_line_references`].
#[must_use]
pub fn unminify_error(error_message: &str, symbol_map: &SymbolMap) -> String {
    let rev = symbol_map.bare_reverse();
    if rev.is_empty() {
        return error_message.to_owned();
    }
    let bytes = error_message.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        let c = bytes[i];
        // `$short` variable reference.
        if c == b'$' {
            let start = i + 1;
            let mut j = start;
            while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j > start {
                let ident = &error_message[start..j];
                if let Some(orig) = rev.get(ident) {
                    out.push('$');
                    out.push_str(orig);
                    i = j;
                    continue;
                }
            }
            out.push('$');
            i += 1;
            continue;
        }
        // `"short"` quoted identifier.
        if c == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j < n && bytes[j] == b'"' && j > start {
                let ident = &error_message[start..j];
                if let Some(orig) = rev.get(ident) {
                    out.push('"');
                    out.push_str(orig);
                    out.push('"');
                    i = j + 1;
                    continue;
                }
            }
            out.push('"');
            i += 1;
            continue;
        }
        let ch_len = utf8_len(c);
        out.push_str(&error_message[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Remap `line N` / `(procedure "X" line N)` references in `message`
/// from minified positions to approximate original lines, using the
/// proportional heuristic from `_remap_line_references`.  Single
/// pass (no double-application).
#[must_use]
pub fn remap_line_references(
    message: &str,
    minified_source: &str,
    original_source: &str,
) -> String {
    let min_commands = minified_source.matches(';').count() + 1;
    let orig_non_empty: Vec<usize> = original_source
        .lines()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .map(|(i, _)| i + 1)
        .collect();
    if orig_non_empty.is_empty() {
        return message.to_owned();
    }
    let map_line = |line_no: usize| -> Option<usize> {
        // Reject `line_no == 0` as well as out-of-range high values: the
        // proportional map below computes `line_no - 1`, which underflows
        // at zero (`(procedure "f" line 0)` is untrusted error text) — a
        // debug panic / silent garbage line in release (F2b).
        if line_no == 0 || line_no > min_commands {
            return None;
        }
        // Integer proportional map: (line_no-1)/(min_commands-1) of the
        // non-empty original lines.
        let denom = min_commands.saturating_sub(1).max(1);
        let last = orig_non_empty.len() - 1;
        let idx = ((line_no - 1) * last / denom).min(last);
        Some(orig_non_empty[idx])
    };

    let bytes = message.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        // `(procedure "NAME" line N)` form.
        if message[i..].starts_with("(procedure \"")
            && let Some(rewritten) = try_remap_procline(&message[i..], &map_line)
        {
            out.push_str(&rewritten.0);
            i += rewritten.1;
            continue;
        }
        // Standalone `line N` (word-bounded).
        if message[i..].starts_with("line ")
            && (i == 0 || !is_word_byte(Some(bytes[i - 1])))
            && let Some((rewritten, consumed)) = try_remap_line(&message[i..], &map_line)
        {
            out.push_str(&rewritten);
            i += consumed;
            continue;
        }
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&message[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Parse a leading `line N` (digits, then a word boundary) and remap
/// it.  Returns `(replacement, bytes_consumed)`.
fn try_remap_line(s: &str, map_line: &impl Fn(usize) -> Option<usize>) -> Option<(String, usize)> {
    let rest = s.strip_prefix("line ")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let after = rest.as_bytes().get(digits.len()).copied();
    if is_word_byte(after) {
        return None;
    }
    let line_no: usize = digits.parse().ok()?;
    if line_no == 1 {
        // Line 1 of minified code = whole script; not useful.
        return None;
    }
    let orig = map_line(line_no)?;
    let consumed = "line ".len() + digits.len();
    Some((format!("line {orig} (minified line {line_no})"), consumed))
}

/// Parse a leading `(procedure "NAME" line N)` and remap the line.
fn try_remap_procline(
    s: &str,
    map_line: &impl Fn(usize) -> Option<usize>,
) -> Option<(String, usize)> {
    let rest = s.strip_prefix("(procedure \"")?;
    let name_end = rest.find('"')?;
    let name = &rest[..name_end];
    let tail = &rest[name_end + 1..];
    let tail = tail.strip_prefix(" line ")?;
    let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let rest_after = &tail[digits.len()..];
    let close = rest_after.strip_prefix(')')?;
    let _ = close;
    let line_no: usize = digits.parse().ok()?;
    let orig = map_line(line_no)?;
    let consumed = s.len() - rest_after.len() + 1; // up to and including ')'
    Some((
        format!("(procedure \"{name}\" line {orig}, minified line {line_no})"),
        consumed,
    ))
}

/// Whether `b` is `[A-Za-z0-9_]`.
fn is_word_byte(b: Option<u8>) -> bool {
    matches!(b, Some(c) if c.is_ascii_alphanumeric() || c == b'_')
}
