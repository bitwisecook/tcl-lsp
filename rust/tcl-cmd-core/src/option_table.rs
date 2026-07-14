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

//! [`OptionTable`] — the typed table wrapper over [`crate::prefix`], the
//! shared `Tcl_GetIndexFromObj` matcher.
//!
//! The matching rule and the error-message shape live once, in
//! [`crate::prefix`] (`lookup` / `bad_key_message`); this module packages a
//! command's table into a const-constructible value carrying its names in
//! **C table order**, its error noun (C's `msg` argument), and its
//! abbreviation mode (`TCL_EXACT` inverted), so a per-command module states
//! its whole contract in one declaration and every resolution goes through
//! the same core.
//!
//! New command modules MUST resolve option words through [`OptionTable`] (or
//! [`crate::prefix`] directly where a byte-generic runtime table needs it)
//! instead of hand-rolling the scan — see
//! `docs/design/contracts/shared-utility-contracts-rust.md`. The one deliberate
//! exception is `string match`/`string map`'s `-nocase`, whose C
//! implementation hand-rolls a `length > 1` prefix test that differs from
//! this rule on a lone `-` (see [`crate::string`]).

use crate::error::CmdError;
use crate::prefix::{self, Lookup};

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
/// argument — `"option"` for the command options here, `"operation"` for
/// `trace` op-lists), and whether abbreviations are accepted (C's `TCL_EXACT`
/// flag inverted).
pub struct OptionTable<'t> {
    names: &'t [&'t str],
    what: &'t str,
    exact: bool,
}

impl<'t> OptionTable<'t> {
    /// A table that accepts unique-prefix abbreviations (a C caller passing
    /// flags `0`).
    #[must_use]
    pub const fn abbreviating(what: &'t str, names: &'t [&'t str]) -> Self {
        Self {
            names,
            what,
            exact: false,
        }
    }

    /// A table that demands exact option names (a C caller passing
    /// `TCL_EXACT`).
    #[must_use]
    pub const fn exact_only(what: &'t str, names: &'t [&'t str]) -> Self {
        Self {
            names,
            what,
            exact: true,
        }
    }

    /// The canonical option names, in table order.
    #[must_use]
    pub const fn names(&self) -> &'t [&'t str] {
        self.names
    }

    /// Resolve `word` with C's rule ([`prefix::lookup`]): exact match first;
    /// then, if the table abbreviates, a unique non-empty prefix; then
    /// ambiguous/no-match. The [`Resolution::Exact`] /
    /// [`Resolution::UniquePrefix`] split refines [`Lookup::Found`] by an
    /// equality check, since some consumers report the two differently.
    #[must_use]
    pub fn resolve(&self, word: &[u8]) -> Resolution {
        match prefix::lookup(self.names, word, self.exact) {
            Lookup::Found(i) => {
                if self.names[i].as_bytes() == word {
                    Resolution::Exact(i)
                } else {
                    Resolution::UniquePrefix(i)
                }
            }
            Lookup::Ambiguous => Resolution::Ambiguous,
            Lookup::None => Resolution::NoMatch,
        }
    }

    /// Resolve `word` to its table index, or the ready-to-report error bytes
    /// ([`prefix::bad_key_message`] — `bad option "-x": must be …` /
    /// `ambiguous option "-x": must be …`). The word is embedded verbatim, so
    /// a non-UTF-8 word round-trips byte-exactly through byte-string errors.
    ///
    /// # Errors
    /// The C-shaped message for an unmatched or ambiguous word.
    pub fn index_of(&self, word: &[u8]) -> Result<usize, Vec<u8>> {
        match self.resolve(word) {
            Resolution::Exact(i) | Resolution::UniquePrefix(i) => Ok(i),
            Resolution::Ambiguous => Err(prefix::bad_key_message(
                self.names,
                self.what.as_bytes(),
                word,
                true,
            )),
            Resolution::NoMatch => Err(prefix::bad_key_message(
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

/// The table enumeration C's error builder produces: `A`, `A or B`, or
/// `A, B, or C` (Oxford comma from three entries; none for two). Shared with
/// errors that embed the same list in another sentence shape, such as
/// `trace`'s `bad operation list "": must be one or more of …`.
#[must_use]
pub fn enumerate_names(names: &[&str]) -> String {
    prefix::choice_list(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRUIT: [&str; 3] = ["apple", "apricot", "banana"];
    const ABBREV: OptionTable<'static> = OptionTable::abbreviating("option", &FRUIT);
    const EXACT: OptionTable<'static> = OptionTable::exact_only("option", &FRUIT);

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
        assert_eq!(enumerate_names(&[]), "");
        assert_eq!(enumerate_names(&["a"]), "a");
        assert_eq!(enumerate_names(&["a", "b"]), "a or b");
        assert_eq!(enumerate_names(&["a", "b", "c"]), "a, b, or c");
    }
}
