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

//! Literal parsing and Tcl-source rendering for the optimiser.

use crate::analyses::ConstValue;
// The static variable-name grammar lives with the other word-shape
// predicates in `value_shapes`; re-exported here so the optimiser's
// existing `helpers::literals::is_static_var_word` call sites keep one
// import path and there is still only one definition.
use crate::tcl_expr_eval::{TclValue, format_tcl_value};
pub use crate::value_shapes::is_static_var_word;

/// Grammar of a word that can be emitted into Tcl source without
/// needing brace quoting:
/// `[A-Za-z0-9_./:+-]+`.
#[must_use]
pub fn is_safe_word(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'/' | b':' | b'+' | b'-')
        })
}

/// Return `true` when `text` contains no substitutions — no `$`,
/// no `[`, no `{`, no backslash.
#[must_use]
pub fn is_plain_literal(text: &str) -> bool {
    !text.contains(['$', '[', '{', '\\'])
}

/// Parse `value` into a `Literal` (int / bool / safe string), or
/// `None` when the text does not fit those grammars.
///
/// Parses as:
///
/// - decimal integer literal → `Literal::Int`,
/// - case-insensitive `"true"` / `"false"` → `Literal::Bool`,
/// - otherwise, any `is_safe_word` text → `Literal::Str`,
/// - everything else → `None`.
#[must_use]
pub fn literal_from_constant_str(value: &str) -> Option<Literal> {
    if let Some(i) = parse_decimal_int(value) {
        return Some(Literal::Int(i));
    }
    let lowered = value.trim().to_ascii_lowercase();
    if lowered == "true" {
        return Some(Literal::Bool(true));
    }
    if lowered == "false" {
        return Some(Literal::Bool(false));
    }
    if is_safe_word(value) {
        return Some(Literal::Str(value.to_owned()));
    }
    None
}

/// Tagged value used by [`literal_from_constant_str`] and
/// [`render_folded_literal`].
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Integer literal.
    Int(i64),
    /// Floating-point literal.
    Float(f64),
    /// Boolean literal.
    Bool(bool),
    /// String literal (safe word).
    Str(String),
}

/// Render a folded value back to Tcl source text. Renders as:
///
/// - bool → `"1"` / `"0"`.
/// - int → decimal string.
/// - float whose value is an integer → decimal string, rendered
///   via [`format_tcl_value`] so it round-trips; other floats
///   return `None` (Tcl's default formatting is lossy).
/// - string → echoed verbatim when [`is_safe_word`]; otherwise
///   `None`.
#[must_use]
pub fn render_folded_literal(value: &Literal) -> Option<String> {
    match value {
        Literal::Bool(b) => Some(if *b { "1".into() } else { "0".into() }),
        Literal::Int(i) => Some(i.to_string()),
        Literal::Float(f) => {
            if f.fract() == 0.0 && f.is_finite() && f.abs() < 1e16 {
                // Integral float within +/-1e16 is exactly representable and
                // fits `i64`; render its exact integer decimal (no lossy
                // `as` cast — `{:.0}` of an integral float is exact here).
                // `+ 0.0` normalises `-0.0` to `0.0` so it renders as "0",
                // matching the previous `as i64` cast.
                Some(format!("{:.0}", f + 0.0))
            } else {
                None
            }
        }
        Literal::Str(s) => is_safe_word(s).then(|| s.clone()),
    }
}

/// Quote a string value into a Tcl word suitable for emission.
/// Renders as:
///
/// - empty → `"{}"`.
/// - [`is_safe_word`] → unwrapped.
/// - contains `{`, `}`, backslash, or CR/LF → `None` (needs
///   escaping the caller must handle).
/// - otherwise → `{value}`.
#[must_use]
pub fn render_static_string_word(value: &str) -> Option<String> {
    if value.is_empty() {
        return Some("{}".into());
    }
    if is_safe_word(value) {
        return Some(value.to_owned());
    }
    if value.contains(['{', '}', '\\', '\n', '\r']) {
        return None;
    }
    Some(format!("{{{value}}}"))
}

/// Render a [`ConstValue`] (from the SCCP lattice) as Tcl source
/// text. The float case delegates to [`format_tcl_value`] so the
/// rendered value round-trips.
#[must_use]
pub fn format_constant(value: &ConstValue) -> Option<String> {
    match value {
        ConstValue::Bool(b) => Some(if *b { "1".into() } else { "0".into() }),
        ConstValue::Int(i) => Some(i.to_string()),
        ConstValue::Float(f) => Some(format_tcl_value(&TclValue::Float(*f))),
        ConstValue::String(s) => Some(s.clone()),
    }
}

// Decimal int parsing (kept private so the rest of the crate is
// not tempted to use it as a general-purpose integer parser).

fn parse_decimal_int(text: &str) -> Option<i64> {
    let s = text.trim();
    if s.is_empty() {
        return None;
    }
    // Tcl accepts an optional leading `+` or `-`.
    let (sign, rest) = match s.as_bytes()[0] {
        b'+' => (1_i64, &s[1..]),
        b'-' => (-1_i64, &s[1..]),
        _ => (1_i64, s),
    };
    if rest.is_empty() {
        return None;
    }
    if !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Leading zero only allowed for the single digit "0" (to avoid
    // octal-shaped values being reparsed by downstream code).
    if rest.len() > 1 && rest.starts_with('0') {
        return None;
    }
    let mag: i64 = rest.parse().ok()?;
    sign.checked_mul(mag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_word_grammar() {
        assert!(is_safe_word("foo"));
        assert!(is_safe_word("foo-bar"));
        assert!(is_safe_word("ns::proc"));
        assert!(is_safe_word("/path/to/file.tcl"));
        assert!(is_safe_word("+7"));
        assert!(!is_safe_word(""));
        assert!(!is_safe_word("has space"));
        assert!(!is_safe_word("has$dollar"));
        assert!(!is_safe_word("has{brace"));
    }

    #[test]
    fn static_var_word_grammar() {
        assert!(is_static_var_word("foo"));
        assert!(is_static_var_word("_foo"));
        assert!(is_static_var_word("ns::var"));
        assert!(!is_static_var_word(""));
        assert!(!is_static_var_word("1abc"));
        assert!(!is_static_var_word("ab-cd"));
        assert!(!is_static_var_word("has space"));
    }

    #[test]
    fn plain_literal_rejects_substitutions() {
        assert!(is_plain_literal("abc"));
        assert!(is_plain_literal(""));
        assert!(!is_plain_literal("$x"));
        assert!(!is_plain_literal("[cmd]"));
        assert!(!is_plain_literal("{body}"));
        assert!(!is_plain_literal("a\\nb"));
    }

    #[test]
    fn literal_from_constant_str_parses_int_bool_word() {
        assert_eq!(literal_from_constant_str("42"), Some(Literal::Int(42)));
        assert_eq!(literal_from_constant_str("-7"), Some(Literal::Int(-7)));
        assert_eq!(literal_from_constant_str("true"), Some(Literal::Bool(true)),);
        assert_eq!(
            literal_from_constant_str("False"),
            Some(Literal::Bool(false)),
        );
        assert_eq!(
            literal_from_constant_str("foo"),
            Some(Literal::Str("foo".into())),
        );
        // Leading zero disqualifies decimal parse — falls through
        // to safe-word, which matches "0" → Int(0), but "01" stays
        // a safe word.
        assert_eq!(literal_from_constant_str("0"), Some(Literal::Int(0)));
        assert_eq!(
            literal_from_constant_str("01"),
            Some(Literal::Str("01".into())),
        );
        // Not a word at all.
        assert!(literal_from_constant_str("hello world").is_none());
        assert!(literal_from_constant_str("a\\b").is_none());
    }

    #[test]
    fn render_folded_literal_roundtrips() {
        assert_eq!(
            render_folded_literal(&Literal::Bool(true)).as_deref(),
            Some("1"),
        );
        assert_eq!(
            render_folded_literal(&Literal::Bool(false)).as_deref(),
            Some("0"),
        );
        assert_eq!(
            render_folded_literal(&Literal::Int(42)).as_deref(),
            Some("42"),
        );
        // Integer-valued float renders as integer.
        assert_eq!(
            render_folded_literal(&Literal::Float(3.0)).as_deref(),
            Some("3"),
        );
        // Negative integer-valued float renders as a signed integer.
        assert_eq!(
            render_folded_literal(&Literal::Float(-7.0)).as_deref(),
            Some("-7"),
        );
        // `-0.0` normalises to "0" (not "-0"): tclsh `int(-0.0)` == 0.
        assert_eq!(
            render_folded_literal(&Literal::Float(-0.0)).as_deref(),
            Some("0"),
        );
        // Fractional float refused.
        assert!(render_folded_literal(&Literal::Float(3.5)).is_none());
        // Safe-word string.
        assert_eq!(
            render_folded_literal(&Literal::Str("hello".into())).as_deref(),
            Some("hello"),
        );
        // Unsafe string refused.
        assert!(render_folded_literal(&Literal::Str("has space".into())).is_none());
    }

    #[test]
    fn render_static_string_word_rules() {
        assert_eq!(render_static_string_word("").as_deref(), Some("{}"));
        assert_eq!(render_static_string_word("foo").as_deref(), Some("foo"),);
        assert_eq!(
            render_static_string_word("hello world").as_deref(),
            Some("{hello world}"),
        );
        assert!(render_static_string_word("has{brace").is_none());
        assert!(render_static_string_word("has\nnewline").is_none());
        assert!(render_static_string_word("back\\slash").is_none());
    }

    #[test]
    fn format_constant_rendering() {
        assert_eq!(
            format_constant(&ConstValue::Bool(true)).as_deref(),
            Some("1"),
        );
        assert_eq!(format_constant(&ConstValue::Int(42)).as_deref(), Some("42"),);
        assert_eq!(
            format_constant(&ConstValue::String("hello".into())).as_deref(),
            Some("hello"),
        );
        // Float renders via format_tcl_value.
        let rendered = format_constant(&ConstValue::Float(1.0)).unwrap();
        assert_eq!(rendered, "1.0");
        let rendered = format_constant(&ConstValue::Float(3.5)).unwrap();
        assert_eq!(rendered, "3.5");
    }

    #[test]
    fn decimal_int_parser_rejects_octal_and_nondigits() {
        assert_eq!(parse_decimal_int("42"), Some(42));
        assert_eq!(parse_decimal_int("+7"), Some(7));
        assert_eq!(parse_decimal_int("-9"), Some(-9));
        assert_eq!(parse_decimal_int("0"), Some(0));
        // Leading-zero multi-digit is octal-shaped — refused.
        assert!(parse_decimal_int("01").is_none());
        assert!(parse_decimal_int("0x1a").is_none());
        assert!(parse_decimal_int("abc").is_none());
        assert!(parse_decimal_int("").is_none());
        assert!(parse_decimal_int("   ").is_none());
    }
}
