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

//! Tcl boolean-word recognition (`Tcl_GetBoolean`).
//!
//! Tcl accepts the boolean *words* `true`/`yes`/`on` and `false`/`no`/`off`
//! case-insensitively **and by unique prefix** — `t`/`tr`/`tru` are `true`,
//! `ye` is `yes`, `of` is `off`, `n` is `no`.  A prefix shared by a true-word
//! and a false-word is ambiguous and rejected: `o` matches both `on` and
//! `off`, so `expr {o}` is an error in real Tcl.
//!
//! Numeric booleans (`0`/`1`, and any integer under `Tcl_GetBoolean` where a
//! non-zero value is true) are handled by the number grammar, not here.

/// The canonical true-words, longest resolution set for `Tcl_GetBoolean`.
const TRUE_WORDS: [&str; 3] = ["true", "yes", "on"];
/// The canonical false-words.
const FALSE_WORDS: [&str; 3] = ["false", "no", "off"];

/// Resolve a Tcl boolean *word* (or unique prefix) to its value.
///
/// Returns `Some(true)` / `Some(false)` for a case-insensitive unique prefix of
/// a boolean word, and `None` when the input is empty, matches nothing, or is
/// an ambiguous prefix (`"o"`).  Numeric forms (`0`/`1`) are **not** handled
/// here — callers that also accept numbers should try the number grammar.
#[must_use]
pub fn parse_boolean_word(word: &str) -> Option<bool> {
    if word.is_empty() {
        return None;
    }
    let lc = word.to_ascii_lowercase();
    let is_true = TRUE_WORDS.iter().any(|w| w.starts_with(lc.as_str()));
    let is_false = FALSE_WORDS.iter().any(|w| w.starts_with(lc.as_str()));
    match (is_true, is_false) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        // Ambiguous prefix (`o`) or no match.
        _ => None,
    }
}

/// Whether `word` is a Tcl boolean word or unique prefix (`t`, `ye`, `of`, …).
#[must_use]
pub fn is_boolean_word(word: &str) -> bool {
    parse_boolean_word(word).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_words_resolve() {
        for w in ["true", "TRUE", "yes", "on", "True"] {
            assert_eq!(parse_boolean_word(w), Some(true), "{w}");
        }
        for w in ["false", "FALSE", "no", "off", "Off"] {
            assert_eq!(parse_boolean_word(w), Some(false), "{w}");
        }
    }

    #[test]
    fn unique_prefixes_resolve() {
        for w in ["t", "tr", "tru", "y", "ye", "on"] {
            assert_eq!(parse_boolean_word(w), Some(true), "{w}");
        }
        for w in ["f", "fa", "fal", "n", "no", "of", "off"] {
            assert_eq!(parse_boolean_word(w), Some(false), "{w}");
        }
    }

    #[test]
    fn ambiguous_and_unknown_reject() {
        // `o` matches both `on` and `off`.
        assert_eq!(parse_boolean_word("o"), None);
        assert_eq!(parse_boolean_word(""), None);
        assert_eq!(parse_boolean_word("maybe"), None);
        assert_eq!(parse_boolean_word("1"), None); // numeric — caller's job
        assert_eq!(parse_boolean_word("trueish"), None);
    }
}
