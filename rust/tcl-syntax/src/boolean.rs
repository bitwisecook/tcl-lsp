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

//! Tcl boolean recognition — the one home for every Tcl boolean acceptor.
//!
//! C Tcl has **two distinct acceptors**, and every consumer in this tree must
//! use the matching one rather than hand-roll a word list:
//!
//! 1. **Strict boolean** ([`parse_boolean_strict`]) — `ParseBoolean` in
//!    `tclObj.c`, the acceptor behind `string is boolean` and
//!    `Tcl_GetBoolean`: the boolean *words* by unique case-insensitive
//!    prefix, plus exactly the digits `0` / `1`. Nothing else — not `00`,
//!    not `0.0`, not `0x0`, no surrounding whitespace (tclsh-verified table
//!    in the tests).
//! 2. **Truthiness** ([`truthiness`]) — `Tcl_GetBooleanFromObj`'s string
//!    path, the acceptor behind every boolean *context* (`if`, `while`,
//!    `expr`'s `?:` / `&&` / `||` / `!`): a boolean word, else **any
//!    number** compared against zero (`2`, `-1`, `0.5`, `0x0`, `0e5`, a
//!    bignum). `NaN` is a domain **error** in a boolean context
//!    (tclsh: "floating point value is Not a Number"), not a truthy value;
//!    `±Inf` is truthy.
//!
//! The word grammar: `true`/`yes`/`on` and `false`/`no`/`off`,
//! case-insensitively **and by unique prefix** — `t`/`tr`/`tru` are `true`,
//! `ye` is `yes`, `of` is `off`, `n` is `no`.  A prefix shared by a true-word
//! and a false-word is ambiguous and rejected: `o` matches both `on` and
//! `off`, so `expr {o}` is an error in real Tcl.

/// The canonical true-words, longest resolution set for `Tcl_GetBoolean`.
const TRUE_WORDS: [&str; 3] = ["true", "yes", "on"];
/// The canonical false-words.
const FALSE_WORDS: [&str; 3] = ["false", "no", "off"];

/// The six boolean words, full spellings, lowercase — for consumers that
/// classify *literal spellings* (would-commit type classification, safe-word
/// checks) rather than accept runtime values. Value acceptance goes through
/// [`parse_boolean_strict`] / [`truthiness`], which also handle prefixes and
/// numerics.
pub const BOOLEAN_WORDS: [&str; 6] = ["true", "yes", "on", "false", "no", "off"];

/// Whether `word` is one of the six boolean words spelled in full
/// (case-insensitive) — the literal-classifier check. `tru` is *not* a full
/// word (use [`parse_boolean_word`] for prefix acceptance).
#[must_use]
pub fn is_boolean_full_word(word: &str) -> bool {
    BOOLEAN_WORDS.iter().any(|w| w.eq_ignore_ascii_case(word))
}

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

/// The **strict** boolean acceptor — C's `ParseBoolean` (`tclObj.c`), the
/// semantics of `string is boolean` and `Tcl_GetBoolean`.
///
/// Accepts a boolean word by unique case-insensitive prefix, plus exactly
/// the digits `"0"` and `"1"`. Everything else — `00`, `01`, `2`, `0.0`,
/// `0x0`, whitespace-padded spellings, the empty string — is rejected
/// (tclsh-verified, see the oracle-table test).
#[must_use]
pub fn parse_boolean_strict(text: &str) -> Option<bool> {
    match text {
        "0" => Some(false),
        "1" => Some(true),
        _ => parse_boolean_word(text),
    }
}

/// The **boolean-context** acceptor — `Tcl_GetBooleanFromObj`'s string path,
/// the semantics of `if` / `while` conditions and `expr`'s `?:` / `&&` /
/// `||` / `!` operands.
///
/// A boolean word resolves to its value; otherwise any *number* compares
/// against zero — `2`, `-1`, `0.5`, `0x0`, `0e5`, and beyond-wide bignums
/// (non-zero by construction) are all accepted. `NaN` is a domain **error**
/// in a boolean context (tclsh: "floating point value is Not a Number"), so
/// it returns `None`, as does any non-boolean non-number.
#[must_use]
pub fn truthiness(text: &str) -> Option<bool> {
    truthiness_with(text, crate::number::runtime_syntax())
}

/// [`truthiness`] reading the numeric fallback under an explicitly named
/// release, for a caller whose target is not this process's own.
///
/// Only the *number* branch is release-dependent; the boolean words are the
/// same in every release.
#[must_use]
pub fn truthiness_with(text: &str, numbers: crate::number::NumberSyntax) -> Option<bool> {
    truthiness_inner(text, numbers)
}

/// [`truthiness`] where every release must agree — the sound reading when no
/// release is in hand and a wrong "constant" answer would matter. `08` is 8
/// (true) from 9.0 and not a number at all before it, so it yields [`None`]
/// here rather than committing to either.
#[must_use]
pub fn truthiness_in_every_release(text: &str) -> Option<bool> {
    crate::number::NumberSyntax::unanimous(|n| truthiness_inner(text, n)).flatten()
}

fn truthiness_inner(text: &str, numbers: crate::number::NumberSyntax) -> Option<bool> {
    if let Some(b) = parse_boolean_word(text) {
        return Some(b);
    }
    match crate::number::parse_whole_with(
        text,
        crate::number::ParseFlags::for_syntax(numbers),
    )? {
        crate::number::Number::Int(i) => Some(i != 0),
        // A parsed `Big` is beyond `i64`, hence never zero.
        crate::number::Number::Big { .. } => Some(true),
        crate::number::Number::Double(d) => Some(d != 0.0),
        crate::number::Number::Nan { .. } => None,
    }
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

    /// The strict acceptor, pinned to the tclsh 8.6.14 oracle table
    /// (`string is boolean -strict $v` — 9.0 agrees):
    ///
    /// ```text
    /// 0 1                  => accepted (their digit values)
    /// 00 01 2 -1 0.0 1.0   => rejected (numbers, but not THE digits)
    /// 0x0 0x1              => rejected
    /// true tru t TRUE      => true      f of off        => false
    /// ye y                 => true      n no            => false
    /// o                    => rejected (ambiguous on/off)
    /// " true" "true " ""   => rejected (no trimming)
    /// ```
    #[test]
    fn strict_acceptor_matches_oracle_table() {
        for (v, want) in [
            ("0", Some(false)),
            ("1", Some(true)),
            ("00", None),
            ("01", None),
            ("2", None),
            ("-1", None),
            ("0.0", None),
            ("1.0", None),
            ("0x0", None),
            ("0x1", None),
            ("true", Some(true)),
            ("tru", Some(true)),
            ("t", Some(true)),
            ("TRUE", Some(true)),
            ("f", Some(false)),
            ("of", Some(false)),
            ("o", None),
            ("on", Some(true)),
            ("off", Some(false)),
            ("ye", Some(true)),
            ("y", Some(true)),
            ("n", Some(false)),
            ("no", Some(false)),
            (" true", None),
            ("true ", None),
            ("", None),
        ] {
            assert_eq!(parse_boolean_strict(v), want, "strict {v:?}");
        }
    }

    /// The boolean-context acceptor, pinned to the tclsh 8.6.14 oracle table
    /// (`expr {$v ? "T" : "F"}` — 9.0 agrees):
    ///
    /// ```text
    /// 0 00 0.0 0x0 0e5 of   => false
    /// 2 -1 0.5 true tru t on 1e0 Inf -Inf => true
    /// o truex "yes no"      => error ("expected boolean value")
    /// NaN nan -NaN          => error ("floating point value is Not a Number")
    /// ```
    #[test]
    fn truthiness_matches_oracle_table() {
        for (v, want) in [
            ("0", Some(false)),
            ("00", Some(false)),
            ("0.0", Some(false)),
            ("0x0", Some(false)),
            ("0e5", Some(false)),
            ("of", Some(false)),
            ("2", Some(true)),
            ("-1", Some(true)),
            ("0.5", Some(true)),
            ("true", Some(true)),
            ("tru", Some(true)),
            ("t", Some(true)),
            ("on", Some(true)),
            ("1e0", Some(true)),
            ("Inf", Some(true)),
            ("-Inf", Some(true)),
            // A beyond-wide bignum is non-zero by construction.
            ("18446744073709551616", Some(true)),
            ("o", None),
            ("truex", None),
            ("yes no", None),
            ("NaN", None),
            ("nan", None),
            ("-NaN", None),
        ] {
            assert_eq!(truthiness(v), want, "truthiness {v:?}");
        }
    }

    #[test]
    fn full_word_classifier_rejects_prefixes() {
        assert!(is_boolean_full_word("true"));
        assert!(is_boolean_full_word("OFF"));
        assert!(!is_boolean_full_word("tru"));
        assert!(!is_boolean_full_word("0"));
    }
}
