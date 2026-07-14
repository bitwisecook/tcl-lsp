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

//! Unique-prefix table matching — the shared `Tcl_GetIndexFromObjStruct` port.
//!
//! One matcher + one error formatter for every option/subcommand table:
//! `switch`'s option scan, `tcl::prefix match`, the runtimes'
//! `Tcl_GetIndexFromObj`-style lookups, and ad-hoc option tables (OO property
//! declarations). Re-derived from `tmp/tcl8.6.14/generic/tclIndexObj.c`
//! (unchanged in 9.0.1 but for the TIP-defined `TCL_NULL_OK`, which no
//! consumer here uses), so the fiddly corners live once:
//!
//! - an **exact** entry always wins, even under `-exact` or when it is also a
//!   prefix of another entry;
//! - otherwise a **unique** prefix matches (unless `exact` disallows
//!   abbreviations);
//! - an **empty key** never abbreviates — but it still *counts* the entries it
//!   trivially prefixes, so C words its miss `ambiguous` for a table of two or
//!   more (tclsh: `tcl::prefix match {apple banana} ""` → `ambiguous option
//!   ""…`, but `… {apple} ""` → `bad option ""…`);
//! - the `must be` enumeration omits the comma for two entries
//!   (`a or b`), uses the Oxford comma for three+ (`a, b, or c`), skips empty
//!   entries except a last-position one (`a, b, or `), and renders an empty
//!   table as `no valid options` (hardcoded — even under a custom `-message`,
//!   tclsh-verified).
//!
//! Byte-based (`AsRef<[u8]>` tables) so both runtimes share it; the `&str`
//! conveniences serve the VM/compiler sides. The LSP-side matchers
//! (semantic-tokens / minify) and `tcl_syntax::boolean` keep their own rules
//! on purpose — different contracts (candidate ranking, locale-free boolean
//! prefixes), not this one.

use crate::error::CmdError;

/// Outcome of a [`lookup`] — `Tcl_GetIndexFromObjStruct`'s three-way result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lookup {
    /// Resolved to `table[index]`: an exact match, or the unique abbreviation.
    Found(usize),
    /// No usable match, worded `ambiguous …` — the key abbreviated more than
    /// one entry (which an empty key does whenever the table has ≥ 2 entries).
    Ambiguous,
    /// No usable match, worded `bad …`.
    None,
}

/// Resolve `key` against `table` with Tcl's unique-prefix rule (flags `0`, or
/// `TCL_EXACT` when `exact`): an exact match always wins; otherwise a unique,
/// non-empty prefix matches unless `exact` disallows abbreviations. The miss
/// distinguishes [`Lookup::Ambiguous`] from [`Lookup::None`] exactly as C
/// words its error (see the module docs for the empty-key subtlety).
pub fn lookup<T: AsRef<[u8]>>(table: &[T], key: &[u8], exact: bool) -> Lookup {
    let mut abbrev: Option<usize> = None;
    let mut num_abbrev = 0usize;
    for (i, entry) in table.iter().enumerate() {
        let entry = entry.as_ref();
        if entry == key {
            return Lookup::Found(i);
        }
        if entry.starts_with(key) {
            num_abbrev += 1;
            abbrev = Some(i);
        }
    }
    if !exact
        && !key.is_empty()
        && num_abbrev == 1
        && let Some(i) = abbrev
    {
        return Lookup::Found(i);
    }
    // C: `ambiguous` wording only without TCL_EXACT (an exact-mode miss is
    // always `bad`, however many entries the key prefixes).
    if num_abbrev > 1 && !exact {
        Lookup::Ambiguous
    } else {
        Lookup::None
    }
}

/// Render `table` as Tcl's `must be` enumeration: `a`, `a or b`,
/// `a, b, or c` — C's exact join, including its empty-entry rule (skipped
/// everywhere but the last position; tclsh: `{a {} b}` → `a or b`, `{a b {}}`
/// → `a, b, or `).
#[must_use]
pub fn choice_list_bytes<T: AsRef<[u8]>>(table: &[T]) -> Vec<u8> {
    let mut out = Vec::new();
    // C skips leading empty entries before the first append…
    let mut iter = table.iter().map(AsRef::as_ref).skip_while(|e| e.is_empty());
    let Some(first) = iter.next() else {
        return out;
    };
    out.extend_from_slice(first);
    let rest: Vec<&[u8]> = iter.collect();
    let mut count = 0usize;
    for (i, entry) in rest.iter().enumerate() {
        if i == rest.len() - 1 {
            // …appends the final entry as `(,) or X` even when empty…
            if count > 0 {
                out.push(b',');
            }
            out.extend_from_slice(b" or ");
            out.extend_from_slice(entry);
        } else if !entry.is_empty() {
            // …and skips interior empty entries.
            out.extend_from_slice(b", ");
            out.extend_from_slice(entry);
            count += 1;
        }
    }
    out
}

/// [`choice_list_bytes`] over `&str` entries, as a `String`.
#[must_use]
pub fn choice_list<T: AsRef<str>>(table: &[T]) -> String {
    let bytes: Vec<&[u8]> = table.iter().map(|s| s.as_ref().as_bytes()).collect();
    String::from_utf8(choice_list_bytes(&bytes)).expect("joined valid UTF-8 entries")
}

/// The full `Tcl_GetIndexFromObjStruct` miss message:
/// `{bad|ambiguous} <what> "<key>": must be <enumeration>`, or
/// `bad <what> "<key>": no valid options` for an empty table. `ambiguous`
/// selects the wording (pass [`Lookup::Ambiguous`]'s truth from [`lookup`]).
#[must_use]
pub fn bad_key_message<T: AsRef<[u8]>>(
    table: &[T],
    what: &[u8],
    key: &[u8],
    ambiguous: bool,
) -> Vec<u8> {
    let mut m: Vec<u8> = if ambiguous {
        b"ambiguous ".to_vec()
    } else {
        b"bad ".to_vec()
    };
    m.extend_from_slice(what);
    m.extend_from_slice(b" \"");
    m.extend_from_slice(key);
    if table.iter().all(|e| e.as_ref().is_empty()) {
        // C renders a (semantically) empty table as the literal
        // `no valid options`, whatever the `what` noun is.
        m.extend_from_slice(b"\": no valid options");
    } else {
        m.extend_from_slice(b"\": must be ");
        m.extend_from_slice(&choice_list_bytes(table));
    }
    m
}

/// [`bad_key_message`] as the canonical [`CmdError`] (the string-side
/// counterpart of [`CmdError::bad_choice`], with the enumeration and the
/// `bad`/`ambiguous` fork built here).
#[must_use]
pub fn lookup_error<T: AsRef<str>>(table: &[T], what: &str, key: &str, miss: Lookup) -> CmdError {
    let bytes: Vec<&[u8]> = table.iter().map(|s| s.as_ref().as_bytes()).collect();
    let msg = bad_key_message(
        &bytes,
        what.as_bytes(),
        key.as_bytes(),
        matches!(miss, Lookup::Ambiguous),
    );
    CmdError::new(String::from_utf8(msg).expect("valid UTF-8 message parts"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRUIT: [&str; 3] = ["apple", "apricot", "banana"];

    #[test]
    fn exact_match_always_wins() {
        // `apple` is also a prefix of `applet`, but the exact entry wins —
        // and it wins under `exact` mode too.
        let t = ["apple", "applet"];
        assert_eq!(lookup(&t, b"apple", false), Lookup::Found(0));
        assert_eq!(lookup(&t, b"apple", true), Lookup::Found(0));
    }

    #[test]
    fn unique_prefix_matches_unless_exact() {
        assert_eq!(lookup(&FRUIT, b"b", false), Lookup::Found(2));
        assert_eq!(lookup(&FRUIT, b"b", true), Lookup::None);
        // An exact-mode miss is worded `bad` even when ambiguous.
        assert_eq!(lookup(&FRUIT, b"ap", true), Lookup::None);
    }

    #[test]
    fn ambiguous_and_missing_keys() {
        assert_eq!(lookup(&FRUIT, b"ap", false), Lookup::Ambiguous);
        assert_eq!(lookup(&FRUIT, b"z", false), Lookup::None);
    }

    #[test]
    fn empty_key_never_abbreviates() {
        // tclsh8.6: `tcl::prefix match {apple banana} ""` → ambiguous;
        // `tcl::prefix match {apple} ""` → bad. An empty table entry is still
        // an exact match for the empty key.
        assert_eq!(lookup(&FRUIT, b"", false), Lookup::Ambiguous);
        assert_eq!(lookup(&["apple"], b"", false), Lookup::None);
        assert_eq!(lookup(&["apple", ""], b"", false), Lookup::Found(1));
    }

    #[test]
    fn choice_list_matches_c_join() {
        assert_eq!(choice_list(&["a"]), "a");
        assert_eq!(choice_list(&["a", "b"]), "a or b");
        assert_eq!(choice_list(&["a", "b", "c"]), "a, b, or c");
        // C's empty-entry rule (tclsh8.6-verified, see module docs).
        assert_eq!(choice_list(&["a", "", "b"]), "a or b");
        assert_eq!(choice_list(&["", "a", "b"]), "a or b");
        assert_eq!(choice_list(&["a", "b", ""]), "a, b, or ");
        assert_eq!(choice_list(&["a", ""]), "a or ");
        assert_eq!(choice_list::<&str>(&[]), "");
    }

    #[test]
    fn miss_messages_match_tclsh() {
        assert_eq!(
            lookup_error(&FRUIT, "option", "ap", Lookup::Ambiguous).message(),
            r#"ambiguous option "ap": must be apple, apricot, or banana"#
        );
        assert_eq!(
            lookup_error(&FRUIT, "option", "z", Lookup::None).message(),
            r#"bad option "z": must be apple, apricot, or banana"#
        );
        // Empty table: hardcoded `no valid options` even for a custom noun
        // (tclsh8.6: `tcl::prefix match -message thing {} foo`).
        assert_eq!(
            lookup_error::<&str>(&[], "thing", "foo", Lookup::None).message(),
            r#"bad thing "foo": no valid options"#
        );
    }
}
