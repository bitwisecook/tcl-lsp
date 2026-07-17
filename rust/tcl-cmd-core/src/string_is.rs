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
#[must_use]
pub fn class_check(class: &str, s: &str, strict: bool) -> (bool, i64) {
    let chars: Vec<char> = s.chars().collect();
    class_check_chars(class, &chars, strict)
}

/// Returns `(is_member, fail_index)`. `fail_index` is a character offset (or -1
/// for the "valid list but wrong shape" cases, e.g. an odd-length dict).
fn class_check_chars(class: &str, chars: &[char], strict: bool) -> (bool, i64) {
    if chars.is_empty() {
        // The empty string is the valid empty list/dict even under `-strict`;
        // for the other classes `-strict` rejects it.
        if matches!(class, "list" | "dict") {
            return (true, -1);
        }
        return (!strict, 0);
    }
    match class {
        // Only `wideinteger` is bounded to 64 bits; `integer`/`entier` accept
        // arbitrary-precision values in Tcl 9.
        "wideinteger" => scan_integer(chars, true),
        "integer" | "entier" => scan_integer(chars, false),
        "double" => scan_double(chars),
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

/// Scan a Tcl integer, returning `(valid, fail_index)`. When `bounded`, a
/// syntactically valid value that overflows a signed 64-bit integer is rejected
/// (index -1).
fn scan_integer(chars: &[char], bounded: bool) -> (bool, i64) {
    let n = chars.len();
    let mut i = 0;
    while i < n && chars[i].is_whitespace() {
        i += 1;
    }
    let neg = i < n && chars[i] == '-';
    if i < n && (chars[i] == '+' || chars[i] == '-') {
        i += 1;
    }
    let mut radix = 10;
    if i + 1 < n && chars[i] == '0' {
        match chars[i + 1].to_ascii_lowercase() {
            'x' => radix = 16,
            'o' => radix = 8,
            'b' => radix = 2,
            'd' => radix = 10,
            _ => radix = 0, // plain leading 0 — still decimal, but keep the 0
        }
        if radix != 0 {
            i += 2;
        } else {
            radix = 10;
        }
    }
    let digits_start = i;
    while i < n && is_radix_digit(chars[i], radix) {
        i += 1;
    }
    let digits_end = i;
    while i < n && chars[i].is_whitespace() {
        i += 1;
    }
    let has_digits = digits_end > digits_start;
    if !has_digits || i != n {
        // No number at all ⇒ the failure is at the start; otherwise it is the
        // first *non-whitespace* character past the digits. Trailing whitespace
        // is allowed (`"12 "` is a valid integer), so the failure index is `i`
        // — advanced past that whitespace — not `digits_end`: `"12 x"` fails at
        // the `x` (index 3), not the interior space (index 2).
        return (false, if has_digits { ci(i.min(n)) } else { 0 });
    }
    if bounded {
        let digit_str: String = chars[digits_start..digits_end].iter().collect();
        let fits = match u64::from_str_radix(&digit_str, radix) {
            Ok(m) if neg => m <= 1u64 << 63,
            Ok(m) => m < 1u64 << 63,
            Err(_) => false,
        };
        if !fits {
            return (false, -1);
        }
    }
    (true, -1)
}

fn is_radix_digit(c: char, radix: u32) -> bool {
    match radix {
        16 => c.is_ascii_hexdigit(),
        8 => ('0'..='7').contains(&c),
        2 => c == '0' || c == '1',
        _ => c.is_ascii_digit(),
    }
}

/// Scan a Tcl double (also accepts integers). Only ASCII whitespace surrounds a
/// number; the fail index is the end of the longest valid floating-point prefix,
/// or 0 when there is no number.
fn scan_double(chars: &[char]) -> (bool, i64) {
    let ascii_ws = |c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0b}' | '\u{0c}');
    let n = chars.len();
    let mut i = 0;
    while i < n && ascii_ws(chars[i]) {
        i += 1;
    }
    let core_start = i;
    if i < n && (chars[i] == '+' || chars[i] == '-') {
        i += 1;
    }
    let mut has_digits = false;
    while i < n && chars[i].is_ascii_digit() {
        i += 1;
        has_digits = true;
    }
    if i < n && chars[i] == '.' {
        i += 1;
        while i < n && chars[i].is_ascii_digit() {
            i += 1;
            has_digits = true;
        }
    }
    // A valid exponent extends the prefix; an `e` with no following digits ends
    // it (so `1.0e4e4` stops at the second `e`).
    if has_digits && i < n && (chars[i] == 'e' || chars[i] == 'E') {
        let mut j = i + 1;
        if j < n && (chars[j] == '+' || chars[j] == '-') {
            j += 1;
        }
        if j < n && chars[j].is_ascii_digit() {
            i = j;
            while i < n && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
    }
    let core_end = i;
    while i < n && ascii_ws(chars[i]) {
        i += 1;
    }
    if has_digits && i == n {
        let core: String = chars[core_start..core_end].iter().collect();
        if core.parse::<f64>().is_ok() {
            return (true, -1);
        }
    }
    (false, if has_digits { ci(core_end) } else { 0 })
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
    use super::class_check;

    #[test]
    fn char_classes_failindex() {
        assert_eq!(class_check("alpha", "abc5def", false), (false, 3));
        assert_eq!(class_check("alnum", "abc1.23", false), (false, 4));
        assert_eq!(class_check("alpha", "abc", false), (true, -1));
        assert_eq!(class_check("space", "a b", false), (false, 0));
        assert_eq!(class_check("lower", "abCd", false), (false, 2));
    }

    #[test]
    fn empty_and_strict() {
        assert_eq!(class_check("alpha", "", false), (true, 0));
        assert_eq!(class_check("alpha", "", true), (false, 0));
    }

    #[test]
    fn integer_failindex() {
        assert_eq!(class_check("integer", "123abc", false), (false, 3));
        assert_eq!(class_check("integer", "1.0", false), (false, 1));
        assert_eq!(class_check("integer", "0o36963", false), (false, 4));
        assert_eq!(class_check("integer", "123", false), (true, -1));
        assert_eq!(class_check("integer", "0x1f", false), (true, -1));
    }

    #[test]
    fn list_and_dict() {
        assert_eq!(class_check("list", "a b c", false), (true, -1));
        assert_eq!(class_check("dict", "a 1 b 2", false), (true, -1));
        assert_eq!(class_check("dict", "a 1 b", false), (false, -1));
    }
}
