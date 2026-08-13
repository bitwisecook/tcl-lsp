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

//! Portable `string is` classification — `string is class ?-strict? str`.
//!
//! Pure character-class and value-class membership tests, returning
//! `(is_member, fail_index)` where `fail_index` is the **character** offset of
//! the first failure (or -1 for the "valid list but wrong shape" cases). No
//! [`ValueOps`](tcl_syntax::value::ValueOps) is needed — this operates on the
//! already-extracted string — so each runtime's `string is` handler parses the
//! options (`-strict`, `-failindex var`), calls [`class_check`], writes the fail
//! variable, and returns the boolean. (tclCmdMZ.c).

use crate::error::CmdError;
use crate::prefix::OptionTable;
use tcl_syntax::number::{self, Number, NumberSyntax, ParseFlags};

/// Canonical class names, in the order Tcl lists them in error messages.
pub const CLASSES: &[&str] = &[
    "alnum",
    "alpha",
    "ascii",
    "control",
    "boolean",
    "dict",
    "digit",
    "double",
    "entier",
    "false",
    "graph",
    "integer",
    "list",
    "lower",
    "print",
    "punct",
    "space",
    "true",
    "upper",
    "wideinteger",
    "wordchar",
    "xdigit",
];

/// Cast a character index to `i64` for a fail-index result.
fn ci(i: usize) -> i64 {
    i64::try_from(i).unwrap_or(i64::MAX)
}

/// Resolve a (possibly abbreviated) class name via the shared
/// [`OptionTable`] rule — C's `StringIsCmd` matches its class table with
/// abbreviations allowed (flags 0), noun `class`. `Err` carries the
/// bad/ambiguous `CmdError`.
///
/// # Errors
/// `bad class "X"` / `ambiguous class "X"`, enumerating [`CLASSES`].
pub fn resolve_class(input: &str) -> Result<&'static str, CmdError> {
    const TABLE: OptionTable<'static> = OptionTable::abbreviating("class", CLASSES);
    TABLE.index_of_str(input).map(|i| CLASSES[i])
}

/// Classify `s` against `class` (canonical), returning `(is_member, fail_index)`.
///
/// `numbers` is the numeral grammar of the release being emulated: the numeric
/// classes inherit every version difference in it, so `string is integer 08` is
/// false up to 8.6 (invalid octal) and true from 9.0 (plain decimal).
#[must_use]
pub fn class_check(class: &str, s: &str, strict: bool, numbers: NumberSyntax) -> (bool, i64) {
    let chars: Vec<char> = s.chars().collect();
    class_check_chars(class, &chars, strict, numbers)
}

/// Returns `(is_member, fail_index)`. `fail_index` is a character offset (or -1
/// for the "valid list but wrong shape" cases, e.g. an odd-length dict).
fn class_check_chars(
    class: &str,
    chars: &[char],
    strict: bool,
    numbers: NumberSyntax,
) -> (bool, i64) {
    if chars.is_empty() {
        // The empty string is the valid empty list/dict even under `-strict`;
        // for the other classes `-strict` rejects it.
        if matches!(class, "list" | "dict") {
            return (true, -1);
        }
        return (!strict, 0);
    }
    match class {
        // Which classes are bounded to a wide is release-dependent, and it is
        // not a numeral-grammar question — pinned on tclsh 8.6.16 / 9.0.4 with
        // `string is CLASS 99999999999999999999`:
        //
        //   class         8.6   9.0
        //   wideinteger   0     0     always bounded
        //   integer       0     1     unbounded from 9.0
        //   entier        1     1     always unbounded
        "wideinteger" => scan_integer(chars, Width::Wide, numbers),
        "integer" => scan_integer(chars, integer_class_width(numbers), numbers),
        "entier" => scan_integer(chars, Width::Unbounded, numbers),
        "double" => scan_double(chars, numbers),
        "boolean" | "true" | "false" => (check_boolean(chars, class), 0),
        "list" => scan_list(chars).0,
        "dict" => scan_dict(chars),
        _ => char_class(class, chars),
    }
}

/// Per-character classes: returns the index of the first failing character.
fn char_class(class: &str, chars: &[char]) -> (bool, i64) {
    let pred: fn(char) -> bool = match class {
        "alnum" => |c| c.is_alphanumeric(),
        "alpha" => char::is_alphabetic,
        "ascii" => |c| (c as u32) < 0x80,
        "control" => char::is_control,
        "digit" => |c| c.is_ascii_digit(),
        "graph" => |c| !c.is_control() && !c.is_whitespace() && c != '\u{0}',
        "lower" => char::is_lowercase,
        "print" => |c| !c.is_control(),
        "punct" => {
            |c| !c.is_alphanumeric() && !c.is_whitespace() && !c.is_control() && !c.is_ascii_digit()
        }
        "space" => char::is_whitespace,
        "upper" => char::is_uppercase,
        "wordchar" => |c| c.is_alphanumeric() || c == '_',
        "xdigit" => |c| c.is_ascii_hexdigit(),
        _ => return (false, 0),
    };
    match chars.iter().position(|&c| !pred(c)) {
        Some(idx) => (false, ci(idx)),
        None => (true, -1),
    }
}

fn check_boolean(chars: &[char], class: &str) -> bool {
    // The canonical strict acceptor (`ParseBoolean`): `0`/`1` plus any
    // *unique* case-insensitive prefix of the boolean words (`f`→false,
    // `tru`→true; `o` is ambiguous on/off and rejected).
    let s: String = chars.iter().collect();
    match tcl_syntax::boolean::parse_boolean_strict(&s) {
        Some(value) => match class {
            "true" => value,
            "false" => !value,
            _ => true,
        },
        None => false,
    }
}

/// How wide a value the class admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Width {
    /// Must fit a signed 64-bit integer (`wideinteger`, and `integer` before 9.0).
    Wide,
    /// Any magnitude — the bignum rung is a member (`entier`, `integer` from 9.0).
    Unbounded,
}

/// Whether the `integer` class is bounded, which changed in 9.0 (see the table
/// in [`class_check_chars`]). Keyed on the grammar enum because `Tcl90` *is* the
/// "9.0 and later" variant; the boundedness itself is a class-semantics change,
/// not a numeral-grammar one.
fn integer_class_width(numbers: NumberSyntax) -> Width {
    if numbers.is_tcl9_or_later() {
        Width::Unbounded
    } else {
        Width::Wide
    }
}

/// The character index of byte offset `off` in `s` — `-failindex` is reported in
/// characters, while the numeral parser works in bytes.
fn char_index_of(s: &str, off: usize) -> i64 {
    ci(s.get(..off).unwrap_or(s).chars().count())
}

/// Byte offset past any trailing ASCII whitespace starting at `off`.
///
/// C tolerates space after the numeral (`"12 "` is a valid integer) and reports
/// the failure at the first *non*-whitespace byte beyond it, so `"12 x"` fails
/// at the `x` (index 3), not the interior space.
fn skip_trailing_ws(s: &str, off: usize) -> usize {
    let b = s.as_bytes();
    let mut i = off;
    while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    i
}

/// Scan a Tcl integer, returning `(valid, fail_index)`, reading the numeral
/// under `numbers` — the emulated release's grammar.
///
/// Delegates to the toolchain's one numeral parser, which makes this correct
/// across releases for free: `08` is invalid octal up to 8.6 and decimal 8 from
/// 9.0, `1_0` and `0d1` exist only from 9.0, `0o`/`0b` only from 8.5.
///
/// **`Parsed.end` is C's `-failindex`.** Verified against tclsh for every
/// failure class: the parser stops exactly where C stops, so the reported index
/// is simply where the parse ran out, after skipping trailing whitespace.
///
/// | input | stops at | `-failindex` |
/// |---|---|---|
/// | `08` (≤8.6) | 1 — the octal scan halts on the `8` | 1 |
/// | `0x` / `0b2` / `0o8` | 1 — the prefix never commits | 1 |
/// | `1.5` (integer-only) | 1 — the fraction is trailing junk | 1 |
/// | `12x` | 2 | 2 |
/// | `12 x` | 2, then the space is skipped | 3 |
fn scan_integer(chars: &[char], width: Width, numbers: NumberSyntax) -> (bool, i64) {
    let s: String = chars.iter().collect();
    let flags = ParseFlags {
        integer_only: true,
        ..ParseFlags::for_syntax(numbers)
    };
    let Some(parsed) = number::parse(&s, flags) else {
        return (false, 0);
    };
    match parsed.number {
        // A magnitude past a wide belongs only to the unbounded classes. C
        // reports -1 rather than an index: the numeral parsed fine, it just
        // does not fit.
        Number::Big { .. } if width == Width::Wide => return (false, -1),
        Number::Int(_) | Number::Big { .. } => {}
        // `Inf`/`NaN` are doubles, never integers, and C reports index 0 — no
        // integer began here at all, rather than one that stopped early.
        Number::Double(_) | Number::Nan { .. } => return (false, 0),
    }
    let end = skip_trailing_ws(&s, parsed.end);
    if end == s.len() {
        (true, -1)
    } else {
        (false, char_index_of(&s, end))
    }
}

/// Scan a Tcl double (which also accepts every integer spelling), returning
/// `(valid, fail_index)` under `numbers`.
///
/// The hand-rolled scanner this replaced accepted only a decimal
/// mantissa/exponent, so `string is double 0x1f` and `0o17` answered false —
/// wrong on *every* release, since C reads a radix integer as a perfectly good
/// double. Going through the facility fixes that along with the
/// release-dependent spellings (`08`, `1_0`).
fn scan_double(chars: &[char], numbers: NumberSyntax) -> (bool, i64) {
    let s: String = chars.iter().collect();
    let Some(parsed) = number::parse(&s, ParseFlags::for_syntax(numbers)) else {
        return (false, 0);
    };
    let end = skip_trailing_ws(&s, parsed.end);
    if end == s.len() {
        (true, -1)
    } else {
        (false, char_index_of(&s, end))
    }
}

fn is_list_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0b}' | '\u{0c}')
}

/// Validate a Tcl list, returning `(valid, fail_index)` and the element count.
fn scan_list_counted(chars: &[char]) -> ((bool, i64), usize) {
    let n = chars.len();
    let mut i = 0;
    let mut count = 0;
    loop {
        while i < n && is_list_space(chars[i]) {
            i += 1;
        }
        if i >= n {
            return ((true, -1), count);
        }
        let start = i;
        count += 1;
        if chars[i] == '{' {
            let mut depth = 1;
            i += 1;
            while i < n && depth > 0 {
                match chars[i] {
                    '\\' => i = (i + 2).min(n),
                    '{' => {
                        depth += 1;
                        i += 1;
                    }
                    '}' => {
                        depth -= 1;
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            if depth != 0 || (i < n && !is_list_space(chars[i])) {
                return ((false, ci(start)), count);
            }
        } else if chars[i] == '"' {
            i += 1;
            while i < n && chars[i] != '"' {
                i = if chars[i] == '\\' {
                    (i + 2).min(n)
                } else {
                    i + 1
                };
            }
            if i >= n || (i + 1 < n && !is_list_space(chars[i + 1])) {
                return ((false, ci(start)), count);
            }
            i += 1;
        } else {
            while i < n && !is_list_space(chars[i]) {
                i = if chars[i] == '\\' {
                    (i + 2).min(n)
                } else {
                    i + 1
                };
            }
        }
    }
}

fn scan_list(chars: &[char]) -> ((bool, i64), usize) {
    scan_list_counted(chars)
}

fn scan_dict(chars: &[char]) -> (bool, i64) {
    let ((valid, fail), count) = scan_list_counted(chars);
    if !valid {
        return (false, fail);
    }
    if count % 2 == 0 {
        (true, -1)
    } else {
        // A valid list with an odd number of elements is not a dict; Tcl reports
        // a fail index of -1 for this shape error.
        (false, -1)
    }
}

#[cfg(test)]
mod tests {
    use super::{class_check, Width, integer_class_width, scan_integer};
    use tcl_syntax::number::NumberSyntax;

    /// The existing expectations were written for 9.x semantics; keep them
    /// reading that way now the release is explicit.
    fn check(class: &str, s: &str, strict: bool) -> (bool, i64) {
        class_check(class, s, strict, NumberSyntax::Tcl90)
    }


    #[test]
    fn char_classes_failindex() {
        assert_eq!(check("alpha", "abc5def", false), (false, 3));
        assert_eq!(check("alnum", "abc1.23", false), (false, 4));
        assert_eq!(check("alpha", "abc", false), (true, -1));
        assert_eq!(check("space", "a b", false), (false, 0));
        assert_eq!(check("lower", "abCd", false), (false, 2));
    }

    #[test]
    fn empty_and_strict() {
        assert_eq!(check("alpha", "", false), (true, 0));
        assert_eq!(check("alpha", "", true), (false, 0));
    }

    #[test]
    fn integer_failindex() {
        assert_eq!(check("integer", "123abc", false), (false, 3));
        assert_eq!(check("integer", "1.0", false), (false, 1));
        assert_eq!(check("integer", "0o36963", false), (false, 4));
        assert_eq!(check("integer", "123", false), (true, -1));
        assert_eq!(check("integer", "0x1f", false), (true, -1));
    }

    #[test]
    fn list_and_dict() {
        assert_eq!(check("list", "a b c", false), (true, -1));
        assert_eq!(check("dict", "a 1 b 2", false), (true, -1));
        assert_eq!(check("dict", "a 1 b", false), (false, -1));
    }

    /// The numeric classes inherit every release difference in the numeral
    /// grammar. Pinned against tclsh 8.6.16 and 9.0.4.
    #[test]
    fn numeric_classes_follow_the_emulated_release() {
        // (value, class, 8.6 member, 9.0 member)
        let matrix = [
            // Invalid octal up to 8.6; plain decimal from 9.0.
            ("08", "integer", false, true),
            ("08", "double", false, true),
            // `_` separators are 9.0+.
            ("1_0", "integer", false, true),
            ("1_0", "double", false, true),
            // The `0d` prefix is 9.0+.
            ("0d1", "integer", false, true),
            // `0o`/`0b` exist from 8.5, so both these releases read them.
            ("0o17", "integer", true, true),
            ("0b101", "integer", true, true),
            // Leading zero is octal up to 8.6 and decimal from 9.0 — a member
            // either way, only the *value* differs.
            ("010", "integer", true, true),
        ];
        for (v, class, on_86, on_90) in matrix {
            assert_eq!(
                class_check(class, v, false, NumberSyntax::Tcl85).0,
                on_86,
                "string is {class} {v} on 8.6"
            );
            assert_eq!(
                class_check(class, v, false, NumberSyntax::Tcl90).0,
                on_90,
                "string is {class} {v} on 9.0"
            );
        }
    }

    /// A radix integer is a valid `double` on every release. The hand-rolled
    /// scanner this replaced only understood a decimal mantissa, so it answered
    /// false here — wrong on 8.6 *and* 9.0, not merely release-blind.
    #[test]
    fn a_radix_integer_is_a_double_on_every_release() {
        // Spelled out per grammar rather than derived: `0x` is universal while
        // `0o`/`0b` arrive in 8.5, and listing them is clearer than a predicate
        // — which would also be a hand-rolled prefix test of exactly the kind
        // `cargo xtask number-drift` exists to reject.
        for (numbers, values) in [
            (NumberSyntax::Tcl84, &["0x1f"][..]),
            (NumberSyntax::Tcl85, &["0x1f", "0b101", "0o17"][..]),
            (NumberSyntax::Tcl90, &["0x1f", "0b101", "0o17"][..]),
        ] {
            for v in values {
                assert_eq!(
                    class_check("double", v, false, numbers),
                    (true, -1),
                    "string is double {v} under {numbers:?}"
                );
            }
        }
    }

    /// Which classes are bounded to a wide is a class-semantics change, not a
    /// grammar one: `integer` became unbounded in 9.0, `entier` always was, and
    /// `wideinteger` never is. `string is CLASS 99999999999999999999` on tclsh.
    #[test]
    fn integer_class_boundedness_follows_the_release() {
        const BIG: &str = "99999999999999999999";
        assert!(!class_check("integer", BIG, false, NumberSyntax::Tcl85).0);
        assert!(class_check("integer", BIG, false, NumberSyntax::Tcl90).0);
        for numbers in NumberSyntax::ALL.iter().copied() {
            assert!(
                class_check("entier", BIG, false, numbers).0,
                "entier is unbounded under {numbers:?}"
            );
            assert!(
                !class_check("wideinteger", BIG, false, numbers).0,
                "wideinteger is bounded under {numbers:?}"
            );
        }
        assert_eq!(integer_class_width(NumberSyntax::Tcl85), Width::Wide);
        assert_eq!(integer_class_width(NumberSyntax::Tcl90), Width::Unbounded);
    }

    /// `-failindex` is the byte offset the numeral parser stopped at, converted
    /// to characters. Every row pinned against tclsh.
    #[test]
    fn failindex_is_where_the_numeral_parse_stopped() {
        let chars: Vec<char> = "12 x".chars().collect();
        assert_eq!(
            scan_integer(&chars, Width::Unbounded, NumberSyntax::Tcl90),
            (false, 3),
            "trailing whitespace is skipped before reporting"
        );
        for (v, want) in [
            ("12x", 2),
            ("1.5", 1),
            ("0x", 1),
            ("0b2", 1),
            ("0o8", 1),
        ] {
            let c: Vec<char> = v.chars().collect();
            assert_eq!(
                scan_integer(&c, Width::Unbounded, NumberSyntax::Tcl90).1,
                want,
                "failindex for {v}"
            );
        }
        // Invalid octal before 9.0 stops just past the leading zero.
        let c: Vec<char> = "08".chars().collect();
        assert_eq!(scan_integer(&c, Width::Unbounded, NumberSyntax::Tcl85).1, 1);
        // `Inf`/`NaN` are doubles: no integer began at all, so index 0.
        for v in ["Inf", "NaN"] {
            let c: Vec<char> = v.chars().collect();
            assert_eq!(
                scan_integer(&c, Width::Unbounded, NumberSyntax::Tcl90),
                (false, 0),
                "{v} as an integer"
            );
        }
        // A past-wide magnitude parses but does not fit: -1, not an index.
        let c: Vec<char> = "99999999999999999999".chars().collect();
        assert_eq!(scan_integer(&c, Width::Wide, NumberSyntax::Tcl90), (false, -1));
    }

    /// Non-ASCII text must not panic the byte→char index conversion.
    #[test]
    fn failindex_counts_characters_not_bytes() {
        let c: Vec<char> = "1é".chars().collect();
        // The `é` is two bytes but one character: the failure is at index 1.
        assert_eq!(scan_integer(&c, Width::Unbounded, NumberSyntax::Tcl90), (false, 1));
    }
}
