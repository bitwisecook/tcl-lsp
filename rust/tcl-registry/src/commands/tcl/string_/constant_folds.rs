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

//! Compile-time folds of the pure `string` subcommands.

use super::TclVersion;
use crate::const_fold::{NativeIndexSelection, parse_element_index, parse_index, parse_range};

/// Compile-time folds for pure `string`
/// subcommands, consumed by the optimiser's O129 general-builtin
/// constant-fold path through the registry `const_fold` callbacks.
///
/// Each receives the arguments *after* the subcommand word (so
/// `[string toupper foo]` calls [`fold_toupper`] with `["foo"]`) and
/// is restricted to ASCII input: Rust's `to_ascii_uppercase` /
/// `to_ascii_lowercase` and char-reversal agree with Tcl's
/// `Tcl_UtfToUpper` / `Tcl_UtfToLower` / `string reverse` exactly on
/// ASCII, while non-ASCII case mapping diverges (Rust's full Unicode
/// `char::to_uppercase` can expand one char to several, e.g. ß → SS,
/// whereas Tcl maps 1:1). Bailing on non-ASCII is conservative —
/// never a wrong fold.
pub(crate) fn fold_toupper(args: &[&str]) -> Option<String> {
    match args {
        [s] if s.is_ascii() => Some(s.to_ascii_uppercase()),
        _ => None,
    }
}

pub(crate) fn fold_tolower(args: &[&str]) -> Option<String> {
    match args {
        [s] if s.is_ascii() => Some(s.to_ascii_lowercase()),
        _ => None,
    }
}

pub(crate) fn fold_reverse(args: &[&str]) -> Option<String> {
    match args {
        [s] if s.is_ascii() => Some(s.chars().rev().collect()),
        _ => None,
    }
}

pub(crate) fn fold_length(args: &[&str]) -> Option<String> {
    // ASCII-only: for ASCII the byte length equals the character count,
    // matching Tcl's `string length` (number of characters).  Non-ASCII
    // bails — the char count diverges across Tcl 8.x (UTF-16 units) and
    // Tcl 9 / Rust (Unicode scalars) for astral characters.
    match args {
        [s] if s.is_ascii() => Some(s.len().to_string()),
        _ => None,
    }
}

/// `string cat ?string ...?` — pure concatenation (no transformation, so
/// sound for any input; the O129 path quotes the result as one word).
///
/// The registry stores const-folds as `ConstFoldFn`
/// (`fn(&[&str]) -> Option<String>`), so this signature is fixed by the
/// dispatch table even though concatenation never fails. `unnecessary_wraps`
/// is a false positive here: the `Option` is the shared callback contract,
/// not redundant wrapping we control.
#[allow(clippy::unnecessary_wraps)] // signature fixed by ConstFoldFn dispatch contract
pub(crate) fn fold_cat(args: &[&str]) -> Option<String> {
    Some(args.concat())
}

/// Cap on a single constant-fold's materialised output (1 MiB). `s` may
/// already be large from a nested fold, so bounding only the repeat count
/// leaves an allocation bomb; bound the product and bail above the cap.
const MAX_FOLD_OUTPUT_BYTES: usize = 1 << 20;

/// `string repeat string count` — repeat (bounded by a 10000 count cap and a
/// 1 MiB output cap).  No char transformation → sound for any input.
/// A negative count fails the `usize` parse → bails.
pub(crate) fn fold_repeat(args: &[&str]) -> Option<String> {
    let [s, count] = args else {
        return None;
    };
    let n: usize = count.trim().parse().ok()?;
    if n > 10_000 {
        return None;
    }
    if s.len()
        .checked_mul(n)
        .is_none_or(|bytes| bytes > MAX_FOLD_OUTPUT_BYTES)
    {
        return None;
    }
    Some(s.repeat(n))
}

/// The Tcl default trim set (`string trim` with no explicit `chars`):
/// space, tab, newline, CR, vertical tab, form feed. This matches the Tcl
/// 8.4/8.5 documented default verbatim ("spaces, tabs, newlines, and
/// carriage returns", plus the long-standing \v/\f inclusion the doc text
/// itself never spelled out). From Tcl 8.6 the documented default widens
/// to "any character for which `string is space` returns 1, and \0"
/// (NUL) — a strict superset of this set for ASCII input.
///
/// This fold is registered dialect-invariantly (`const_fold`, not
/// `const_fold_versioned`). A default-set trim containing NUL therefore
/// abstains: Tcl 8.4/8.5 preserve NUL while Tcl 8.6+/9.x trim it, so no one
/// invariant answer is sound. An explicit `chars` word remains exact because
/// it replaces the release-dependent default set.
pub(super) const TRIM_WS: &[char] = &[' ', '\t', '\n', '\r', '\u{0b}', '\u{0c}'];

/// `string trim` / `trimleft` / `trimright`.  ASCII-restricted so the
/// default whitespace set and the explicit chars set match Tcl exactly.
fn fold_trim_impl(args: &[&str], left: bool, right: bool) -> Option<String> {
    let (s, chars): (&str, Vec<char>) = match args {
        [s] if s.contains('\0') => return None,
        [s] => (s, TRIM_WS.to_vec()),
        [s, chars] => (s, chars.chars().collect()),
        _ => return None,
    };
    if !s.is_ascii() || !chars.iter().all(char::is_ascii) {
        return None;
    }
    let pred = |c: char| chars.contains(&c);
    let out = match (left, right) {
        (true, true) => s.trim_matches(pred),
        (true, false) => s.trim_start_matches(pred),
        (false, true) => s.trim_end_matches(pred),
        (false, false) => s,
    };
    Some(out.to_owned())
}

pub(crate) fn fold_trim(args: &[&str]) -> Option<String> {
    fold_trim_impl(args, true, true)
}

pub(crate) fn fold_trimleft(args: &[&str]) -> Option<String> {
    fold_trim_impl(args, true, false)
}

pub(crate) fn fold_trimright(args: &[&str]) -> Option<String> {
    fold_trim_impl(args, false, true)
}

/// `string totitle string` — the no-index form only (first char upper,
/// rest lower).  ASCII-restricted.
pub(crate) fn fold_totitle(args: &[&str]) -> Option<String> {
    let [s] = args else {
        return None;
    };
    if !s.is_ascii() {
        return None;
    }
    if s.is_empty() {
        return Some(String::new());
    }
    let (first, rest) = s.split_at(1); // ASCII → first char is one byte
    Some(format!(
        "{}{}",
        first.to_ascii_uppercase(),
        rest.to_ascii_lowercase()
    ))
}

/// A literal match with no selected release requires agreement on the
/// character model and source encoding.
pub(crate) fn fold_match(args: &[&str]) -> Option<String> {
    fold_match_versioned(args, None)
}

pub(crate) fn fold_match_versioned(args: &[&str], version: Option<TclVersion>) -> Option<String> {
    crate::value_transfer::evaluate_literal(
        &crate::value_transfer::builtins::STRING_MATCH,
        "string",
        Some("match"),
        args,
        version,
    )
}

/// `string index string charIndex`.  ASCII-restricted (byte index ==
/// char index for ASCII).
pub(crate) fn fold_index(args: &[&str]) -> Option<String> {
    let [s, idx_str] = args else {
        return None;
    };
    let (s, idx_str) = (*s, *idx_str);
    if !s.is_ascii() {
        return None;
    }
    Some(match parse_element_index(idx_str, s.len())? {
        NativeIndexSelection::Selected(i) => s[i..=i].to_owned(),
        NativeIndexSelection::Empty => String::new(), // negative or out of range → ""
    })
}

/// `string range` over literal words. An explicit C release uses the shared
/// value route; an unversioned fold requires C/Jim container and origin
/// agreement and is restricted to ASCII character geometry.
pub(crate) fn fold_range(args: &[&str], version: Option<TclVersion>) -> Option<String> {
    if let Some(version) = version
        && let Some(value) = crate::value_transfer::evaluate_literal(
            &crate::value_transfer::builtins::STRING_RANGE,
            "string",
            Some("range"),
            args,
            Some(version),
        )
    {
        return Some(value);
    }
    // An ASCII subject over indices every release reads alike selects the
    // same range in each, so the release-wide answer is this release's too.
    // The detached evaluator above declines such calls (tclsh 8.4–9.1:
    // `string range abcdef 1 3` is `bcd`).
    let [s, first_s, last_s] = args else {
        return None;
    };
    let s = *s;
    if !s.is_ascii() {
        return None;
    }
    match parse_range(first_s, last_s, s.len())? {
        NativeIndexSelection::Selected((lo, hi)) => Some(s[lo..=hi].to_owned()),
        NativeIndexSelection::Empty => Some(String::new()),
    }
}

/// Unversioned container agreement, with no native dispatch or object claim.
pub(crate) fn fold_range_unanimous(args: &[&str]) -> Option<String> {
    fold_range(args, None)
}

/// `string replace string first last ?newString?`.  ASCII-restricted.
pub(crate) fn fold_replace(args: &[&str]) -> Option<String> {
    let (s, first_s, last_s, repl) = match args {
        [s, f, l] => (*s, *f, *l, ""),
        [s, f, l, r] => (*s, *f, *l, *r),
        _ => return None,
    };
    if !s.is_ascii() {
        return None;
    }
    match parse_range(first_s, last_s, s.len())? {
        NativeIndexSelection::Selected((lo, hi)) => {
            Some(format!("{}{}{}", &s[..lo], repl, &s[hi + 1..]))
        }
        NativeIndexSelection::Empty => Some(s.to_owned()),
    }
}

/// `string first needleString haystackString ?startIndex?`.
/// ASCII-restricted.  Returns the byte/char index, or `-1`.
pub(crate) fn fold_first(args: &[&str]) -> Option<String> {
    let (needle, haystack, start) = match args {
        [n, h] => (*n, *h, 0usize),
        [n, h, st] => {
            if !h.is_ascii() {
                return None;
            }
            // A negative start clamps to 0.
            (
                *n,
                *h,
                usize::try_from(parse_index(st, h.len())?).unwrap_or(0),
            )
        }
        _ => return None,
    };
    if !needle.is_ascii() || !haystack.is_ascii() {
        return None;
    }
    if needle.is_empty() {
        return Some("-1".to_owned());
    }
    let pos = haystack
        .get(start..)
        .and_then(|tail| tail.find(needle))
        .map(|i| i + start);
    Some(pos.map_or_else(|| "-1".to_owned(), |i| i.to_string()))
}

/// `string last needleString haystackString ?lastIndex?`.
/// ASCII-restricted.  Searches `haystack[0..end)` from the right.
pub(crate) fn fold_last(args: &[&str]) -> Option<String> {
    let (needle, haystack, end_idx) = match args {
        [n, h] => (*n, *h, None),
        [n, h, last] => {
            if !h.is_ascii() {
                return None;
            }
            (*n, *h, Some(parse_index(last, h.len())? + 1))
        }
        _ => return None,
    };
    if !needle.is_ascii() || !haystack.is_ascii() {
        return None;
    }
    if needle.is_empty() {
        return Some("-1".to_owned());
    }
    let end = match end_idx {
        None => haystack.len(),
        // Clamp the (lastIndex + 1) exclusive end into `[0, len]`.
        Some(e) => usize::try_from(e).unwrap_or(0).min(haystack.len()),
    };
    let pos = haystack.get(..end).and_then(|head| head.rfind(needle));
    Some(pos.map_or_else(|| "-1".to_owned(), |i| i.to_string()))
}

/// `string compare ?-nocase? ?-length N? string1 string2`.
/// ASCII-restricted.  Returns `-1` / `0` / `1`.
pub(crate) fn fold_compare(args: &[&str]) -> Option<String> {
    let mut nocase = false;
    let mut length: Option<usize> = None;
    let mut i = 0;
    while i < args.len() && args[i].starts_with('-') {
        match args[i] {
            "-nocase" => {
                nocase = true;
                i += 1;
            }
            "-length" if i + 1 < args.len() => {
                length = Some(args[i + 1].parse().ok()?);
                i += 2;
            }
            "--" => {
                i += 1;
                break;
            }
            _ => return None,
        }
    }
    if args.len() - i != 2 {
        return None;
    }
    let (mut s1, mut s2) = (args[i].to_owned(), args[i + 1].to_owned());
    if !s1.is_ascii() || !s2.is_ascii() {
        return None;
    }
    if nocase {
        s1 = s1.to_ascii_lowercase();
        s2 = s2.to_ascii_lowercase();
    }
    if let Some(n) = length {
        s1.truncate(n.min(s1.len()));
        s2.truncate(n.min(s2.len()));
    }
    Some(
        match s1.cmp(&s2) {
            std::cmp::Ordering::Less => "-1",
            std::cmp::Ordering::Equal => "0",
            std::cmp::Ordering::Greater => "1",
        }
        .to_owned(),
    )
}

/// `string equal ?-nocase? ?-length N? string1 string2`.
pub(crate) fn fold_equal(args: &[&str]) -> Option<String> {
    match fold_compare(args)?.as_str() {
        "0" => Some("1".to_owned()),
        _ => Some("0".to_owned()),
    }
}

/// `string map ?-nocase? mapping string`.  ASCII-restricted (byte-exact
/// greedy left-to-right replacement matching Tcl's `string map`; the
/// `mapping` is a list of old/new pairs, first matching pair wins).
pub(crate) fn fold_string_map(args: &[&str]) -> Option<String> {
    let (mapping_str, s) = match args {
        [m, s] => (*m, *s),
        ["-nocase", m, s] => {
            // -nocase handled below via case-insensitive byte compare.
            return fold_string_map_impl(m, s, true);
        }
        _ => return None,
    };
    fold_string_map_impl(mapping_str, s, false)
}

fn fold_string_map_impl(mapping_str: &str, s: &str, nocase: bool) -> Option<String> {
    if !mapping_str.is_ascii() || !s.is_ascii() {
        return None;
    }
    let pairs = crate::const_fold::split_list(mapping_str)?;
    if pairs.len() % 2 != 0 {
        return None;
    }
    let reps: Vec<(&str, &str)> = pairs
        .as_chunks::<2>()
        .0
        .iter()
        .map(|kv| (kv[0].as_str(), kv[1].as_str()))
        .collect();
    let sb = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut pos = 0;
    while pos < sb.len() {
        let mut matched = false;
        for (old, new) in &reps {
            let ob = old.as_bytes();
            if ob.is_empty() || pos + ob.len() > sb.len() {
                continue;
            }
            let window = &sb[pos..pos + ob.len()];
            let hit = if nocase {
                window.eq_ignore_ascii_case(ob)
            } else {
                window == ob
            };
            if hit {
                out.push_str(new);
                pos += ob.len();
                matched = true;
                break;
            }
        }
        if !matched {
            out.push(sb[pos] as char); // ASCII byte → char
            pos += 1;
        }
    }
    Some(out)
}
