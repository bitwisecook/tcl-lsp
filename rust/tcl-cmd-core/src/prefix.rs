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
//! One matcher + one error formatter for every option/subcommand table, with
//! [`OptionTable`] as the one API: a const-constructible value carrying a
//! command's names in **C table order**, its error noun (C's `msg`
//! argument), and its abbreviation mode (`TCL_EXACT` inverted). It is
//! generic over `AsRef<[u8]>` entries, so a static `&str` table
//! (`switch`'s option scan), a runtime `String` table (`tcl::prefix
//! match`), and the runtimes' byte tables (OO property declarations) all
//! resolve through the same value. Re-derived from
//! `tmp/tcl8.6.14/generic/tclIndexObj.c`
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

/// Outcome of a raw table scan — `Tcl_GetIndexFromObjStruct`'s three-way
/// result, refined into [`Resolution`] by [`OptionTable::resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lookup {
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
fn lookup<T: AsRef<[u8]>>(table: &[T], key: &[u8], exact: bool) -> Lookup {
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

/// Render `TclOO`'s method-list variant: `a`, `a or b`, or `a, b or c`.
///
/// `TclOO`'s `unknown method` diagnostics deliberately omit the Oxford comma
/// that [`choice_list_bytes`] uses for `Tcl_GetIndexFromObj` tables. Keeping
/// the variant here prevents runtime consumers from reimplementing either
/// separator rule beside their command handlers.
#[must_use]
pub fn tcloo_choice_list_bytes<T: AsRef<[u8]>>(table: &[T]) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, entry) in table.iter().enumerate() {
        if index > 0 {
            out.extend_from_slice(if index == table.len() - 1 {
                b" or "
            } else {
                b", "
            });
        }
        out.extend_from_slice(entry.as_ref());
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

/// [`OptionTable::resolve`] without a table value: C's rule over a bare
/// table, for the composing consumers that build their own message around
/// [`bad_key_message`] (a byte `-message` noun, an `-error {}` early
/// return). The [`Resolution::Exact`] / [`Resolution::UniquePrefix`] split
/// refines the raw scan's hit by an equality check, since some consumers
/// report the two differently.
#[must_use]
pub fn scan<T: AsRef<[u8]>>(table: &[T], word: &[u8], exact: bool) -> Resolution {
    match lookup(table, word, exact) {
        Lookup::Found(i) => {
            if table[i].as_ref() == word {
                Resolution::Exact(i)
            } else {
                Resolution::UniquePrefix(i)
            }
        }
        Lookup::Ambiguous => Resolution::Ambiguous,
        Lookup::None => Resolution::NoMatch,
    }
}

/// The outcome of resolving a word against an [`OptionTable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// The word equals `names[index]` exactly.
    Exact(usize),
    /// The word is a unique, non-empty prefix of `names[index]` — only
    /// produced by an abbreviating table.
    UniquePrefix(usize),
    /// The word is a prefix of more than one entry and equal to none — only
    /// produced by an abbreviating table (an exact-only table reports the
    /// same word as [`Resolution::NoMatch`], which C renders as `bad`, not
    /// `ambiguous`). The empty word abbreviates every entry, so it lands
    /// here whenever the table has two or more entries.
    Ambiguous,
    /// Nothing matched: no entry starts with the word, the table is
    /// exact-only, or the word is empty (C never abbreviation-matches the
    /// empty key).
    NoMatch,
}

/// One command's option table: the option words in C table order (the error
/// message enumerates them in that order), the error noun (C's `msg`
/// argument — `"option"` for command options, `"operation"` for `trace`
/// op-lists), and whether abbreviations are accepted (C's `TCL_EXACT` flag
/// inverted). Generic over the entry type so `&str`, `String`, and byte
/// tables all resolve identically.
pub struct OptionTable<'t, T = &'t str> {
    names: &'t [T],
    what: &'t str,
    exact: bool,
}

impl<'t, T: AsRef<[u8]>> OptionTable<'t, T> {
    /// A table that accepts unique-prefix abbreviations (a C caller passing
    /// flags `0`).
    #[must_use]
    pub const fn abbreviating(what: &'t str, names: &'t [T]) -> Self {
        Self {
            names,
            what,
            exact: false,
        }
    }

    /// A table that demands exact option names (a C caller passing
    /// `TCL_EXACT`).
    #[must_use]
    pub const fn exact_only(what: &'t str, names: &'t [T]) -> Self {
        Self {
            names,
            what,
            exact: true,
        }
    }

    /// The canonical option names, in table order.
    #[must_use]
    pub const fn names(&self) -> &'t [T] {
        self.names
    }

    /// Resolve `word` with C's rule ([`scan`]): exact match first; then, if
    /// the table abbreviates, a unique non-empty prefix; then
    /// ambiguous/no-match.
    #[must_use]
    pub fn resolve(&self, word: &[u8]) -> Resolution {
        scan(self.names, word, self.exact)
    }

    /// Resolve `word` to its table index, or the ready-to-report error bytes
    /// ([`bad_key_message`] — `bad option "-x": must be …` / `ambiguous
    /// option "-x": must be …`). The word is embedded verbatim, so a
    /// non-UTF-8 word round-trips byte-exactly through byte-string errors.
    ///
    /// # Errors
    /// The C-shaped message for an unmatched or ambiguous word.
    pub fn index_of(&self, word: &[u8]) -> Result<usize, Vec<u8>> {
        match self.resolve(word) {
            Resolution::Exact(i) | Resolution::UniquePrefix(i) => Ok(i),
            Resolution::Ambiguous => Err(bad_key_message(
                self.names,
                self.what.as_bytes(),
                word,
                true,
            )),
            Resolution::NoMatch => Err(bad_key_message(
                self.names,
                self.what.as_bytes(),
                word,
                false,
            )),
        }
    }

    /// [`Self::index_of`] for string words and [`CmdError`] consumers.
    ///
    /// # Errors
    /// The C-shaped message for an unmatched or ambiguous word.
    pub fn index_of_str(&self, word: &str) -> Result<usize, CmdError> {
        self.index_of(word.as_bytes())
            .map_err(|m| CmdError::new(String::from_utf8_lossy(&m).into_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [`bad_key_message`] over `&str` parts, as the tests' [`CmdError`]
    /// convenience (production consumers go through [`OptionTable`]).
    fn lookup_error<T: AsRef<str>>(table: &[T], what: &str, key: &str, miss: Lookup) -> CmdError {
        let bytes: Vec<&[u8]> = table.iter().map(|s| s.as_ref().as_bytes()).collect();
        let msg = bad_key_message(
            &bytes,
            what.as_bytes(),
            key.as_bytes(),
            matches!(miss, Lookup::Ambiguous),
        );
        CmdError::new(String::from_utf8(msg).expect("valid UTF-8 message parts"))
    }

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
    fn tcloo_method_choices_omit_the_oxford_comma() {
        assert_eq!(
            tcloo_choice_list_bytes(&["-append", "-clear", "-set"]),
            b"-append, -clear or -set"
        );
        assert_eq!(tcloo_choice_list_bytes(&["a", "b"]), b"a or b");
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

    const TABLE_FRUIT: [&str; 3] = ["apple", "apricot", "banana"];
    const ABBREV: OptionTable<'static> = OptionTable::abbreviating("option", &TABLE_FRUIT);
    const EXACT: OptionTable<'static> = OptionTable::exact_only("option", &TABLE_FRUIT);

    #[test]
    fn abbreviating_matches_exact_then_unique_prefix() {
        // tclsh (Tcl_GetIndexFromObj, flags 0): exact wins, unique prefix
        // abbreviates, shared prefixes are ambiguous.
        assert_eq!(ABBREV.resolve(b"apple"), Resolution::Exact(0));
        assert_eq!(ABBREV.resolve(b"apr"), Resolution::UniquePrefix(1));
        assert_eq!(ABBREV.resolve(b"b"), Resolution::UniquePrefix(2));
        assert_eq!(ABBREV.resolve(b"ap"), Resolution::Ambiguous);
        assert_eq!(ABBREV.resolve(b"z"), Resolution::NoMatch);
    }

    #[test]
    fn exact_beats_a_longer_entry_sharing_the_prefix() {
        // `{-exact -exactly}`-style tables: the exact hit wins even though the
        // word also prefixes another entry.
        let names = ["stop", "stopped"];
        let t = OptionTable::abbreviating("option", &names);
        assert_eq!(t.resolve(b"stop"), Resolution::Exact(0));
        assert_eq!(t.resolve(b"stopp"), Resolution::UniquePrefix(1));
    }

    #[test]
    fn exact_only_reports_every_miss_as_bad() {
        // C's TCL_EXACT: abbreviations — even unique ones — are a *bad*
        // option, never ambiguous (tclsh: `regexp -al a a` → bad option).
        assert_eq!(EXACT.resolve(b"apple"), Resolution::Exact(0));
        assert_eq!(EXACT.resolve(b"apr"), Resolution::NoMatch);
        assert_eq!(EXACT.resolve(b"ap"), Resolution::NoMatch);
        assert_eq!(
            EXACT.index_of(b"ap").unwrap_err(),
            b"bad option \"ap\": must be apple, apricot, or banana".to_vec()
        );
    }

    #[test]
    fn empty_word_never_abbreviates() {
        // C: `key[0] == '\0'` forces the error path; the message is
        // "ambiguous" for 2+ entries (it prefixes them all) and "bad" for a
        // single-entry table. Probed: `tcl::prefix match {apple apricot
        // banana} ""` → ambiguous; `tcl::prefix match {apple} ""` → bad;
        // `lsort "" {a b}` / `trace add "" x` → ambiguous (tclsh 8.6.14).
        assert_eq!(ABBREV.resolve(b""), Resolution::Ambiguous);
        let one = ["apple"];
        let t = OptionTable::abbreviating("option", &one);
        assert_eq!(t.resolve(b""), Resolution::NoMatch);
        assert_eq!(
            t.index_of(b"").unwrap_err(),
            b"bad option \"\": must be apple".to_vec()
        );
    }

    #[test]
    fn error_messages_match_c_shapes() {
        // The join: 1 entry plain, 2 without comma, 3+ with the Oxford comma
        // (tclsh: `bad option "z": must be apple or apricot` for two).
        assert_eq!(
            ABBREV.index_of(b"z").unwrap_err(),
            b"bad option \"z\": must be apple, apricot, or banana".to_vec()
        );
        assert_eq!(
            ABBREV.index_of(b"ap").unwrap_err(),
            b"ambiguous option \"ap\": must be apple, apricot, or banana".to_vec()
        );
        let two = ["apple", "apricot"];
        let t = OptionTable::abbreviating("option", &two);
        assert_eq!(
            t.index_of(b"z").unwrap_err(),
            b"bad option \"z\": must be apple or apricot".to_vec()
        );
        let none: [&str; 0] = [];
        let t = OptionTable::abbreviating("option", &none);
        assert_eq!(
            t.index_of(b"z").unwrap_err(),
            b"bad option \"z\": no valid options".to_vec()
        );
    }

    #[test]
    fn the_error_noun_is_data() {
        // `trace` op-lists use "operation" (C's msg argument).
        let ops = ["array", "read", "unset", "write"];
        let t = OptionTable::exact_only("operation", &ops);
        let Err(e) = t.index_of_str("w") else {
            panic!("`w` must not abbreviate an exact-only table");
        };
        assert_eq!(
            e.message(),
            "bad operation \"w\": must be array, read, unset, or write"
        );
    }

    #[test]
    fn enumerate_names_join_shapes() {
        assert_eq!(choice_list::<&str>(&[]), "");
        assert_eq!(choice_list(&["a"]), "a");
        assert_eq!(choice_list(&["a", "b"]), "a or b");
        assert_eq!(choice_list(&["a", "b", "c"]), "a, b, or c");
    }
}
