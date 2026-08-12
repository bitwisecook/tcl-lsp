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

//! Shared Tcl index parsing (`Tcl_GetIntForIndex` / `GetEndOffsetFromObj`).
//!
//! One parser for every indexed command — `string index/range`, `lindex`,
//! `lrange`, `linsert`, `lreplace`, … — so the accepted forms and the error
//! message live once. The grammar is a **base** (`end`, or a signed integer)
//! optionally followed by a **connector** (`+`/`-`) and a (possibly signed)
//! integer operand: `5`, `-2`, `end`, `end-2`, `1+1`, `0-1`, `end--1`
//! (= `end - (-1)`).

use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// Resolve an index `spec` against a container length `len` (so `end` is
/// `len - 1`). The result may be negative or `>= len`; callers clamp per their
/// command's rules. Errors with the canonical message on an unparseable spec.
pub fn resolve(spec: &str, len: usize) -> Result<i64, CmdError> {
    parse(spec, len).ok_or_else(|| bad_index(spec.trim()))
}

/// Resolve an index `spec` against `len`, returning `None` (rather than an
/// error) on an unparseable spec — the `lsearch`/`lsort -index` driving needs the
/// raw option to classify "bad index" vs "out of range" itself.
#[must_use]
pub fn resolve_opt(spec: &str, len: usize) -> Option<i64> {
    parse(spec, len)
}

/// Drill into a (nested) list `value` by an index `path` (`lsearch`/`lsort
/// -index`): each spec steps one level. An out-of-range step is an error
/// (`element <spec> missing from sublist "<list>"`); a non-list step stops,
/// returning the current value (C's `TclLindexFlat` fallthrough). Returns the
/// error **message bytes** so each command wraps them in its own error type.
///
/// # Errors
/// `bad index`, the list-parse error, or `element … missing from sublist …`.
pub fn drill<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    path: &[Vec<u8>],
) -> Result<O::Value, Vec<u8>> {
    let mut cur = value.clone();
    for spec in path {
        let len = ops.list_len(&cur).map_err(|e| e.message().into_bytes())?;
        let idx = resolve_opt(&String::from_utf8_lossy(spec), len).ok_or_else(|| {
            let mut m = b"bad index \"".to_vec();
            m.extend_from_slice(spec);
            m.extend_from_slice(b"\": must be integer?[+-]integer? or end?[+-]integer?");
            m
        })?;
        if idx < 0 || usize::try_from(idx).unwrap_or(usize::MAX) >= len {
            let mut m = b"element ".to_vec();
            m.extend_from_slice(spec);
            m.extend_from_slice(b" missing from sublist \"");
            m.extend_from_slice(&ops.as_bytes(&cur));
            m.push(b'"');
            return Err(m);
        }
        match ops.list_index(&cur, usize::try_from(idx).unwrap_or(0)) {
            Ok(Some(e)) => cur = e,
            _ => return Ok(cur),
        }
    }
    Ok(cur)
}

/// Whether an index `spec` is *encodable* the way `TclIndexEncode` requires for
/// `lsearch`/`lsort -index`: `Some(true)` for a normal index (a non-negative
/// integer, or `end`/`end-N`), `Some(false)` for one that can never be in range
/// (a negative integer, or `end+N`), and `None` for a syntactically bad spec.
/// Length-independent: it classifies end-relativity by resolving against two
/// different lengths.
#[must_use]
pub fn encodable(spec: &str) -> Option<bool> {
    const BIG: usize = 1 << 20;
    let r_big = parse(spec, BIG + 1)?; // None ⇒ syntactically bad
    let r_small = parse(spec, 1)?;
    let big = i64::try_from(BIG).unwrap_or(i64::MAX);
    Some(if r_big == r_small {
        // Absolute: encodable iff non-negative.
        r_big >= 0
    } else {
        // End-relative: encodable iff it lands at or before `end` (offset ≤ 0).
        r_big - big <= 0
    })
}

fn parse(spec: &str, len: usize) -> Option<i64> {
    let s = spec.trim();
    if s.is_empty() {
        return None;
    }
    let len = i64::try_from(len).unwrap_or(i64::MAX);

    // Base: `end` or a leading signed integer.
    let (base, rest) = if let Some(r) = s.strip_prefix("end") {
        (len - 1, r)
    } else {
        parse_int_prefix(s)?
    };
    if rest.is_empty() {
        return Some(base);
    }

    // Optional offset: a `+`/`-` connector then a (possibly signed) integer, so
    // `end--1` is `end - (-1)` and `0-1` is `0 - 1` (matches `GetEndOffsetFromObj`).
    let connector = rest.as_bytes()[0];
    if connector != b'+' && connector != b'-' {
        return None;
    }
    let operand = parse_int_whole(rest[1..].trim())?;
    let offset = if connector == b'-' { -operand } else { operand };
    Some(base + offset)
}

/// [`ParseFlags`](tcl_syntax::number::ParseFlags) for an index integer: reject a
/// fractional part / exponent, but otherwise the full Tcl integer syntax
/// (`0x`/`0o`/`0b`/`0d` radices, `_` separators) — so a hex index like `0x2`
/// resolves the way real Tcl's `Tcl_GetIntForIndex` does (verified against the
/// tclsh 8.6 / 9.0 oracle). Leading-zero integers follow Tcl 9 (decimal, not
/// legacy octal), matching the modern default.
const INDEX_INT_FLAGS: tcl_syntax::number::ParseFlags = tcl_syntax::number::ParseFlags {
    integer_only: true,
    no_whitespace: false,
    no_underscore: false,
};

/// Parse a leading Tcl integer, returning its value and the unconsumed tail via
/// the canonical [`tcl_syntax::number`] parser (so every radix the runtime
/// accepts resolves identically). `None` if `s` does not start with an integer,
/// or the value is a non-integer / bignum (`Int` only).
fn parse_int_prefix(s: &str) -> Option<(i64, &str)> {
    let parsed = tcl_syntax::number::parse(s, INDEX_INT_FLAGS)?;
    match parsed.number {
        tcl_syntax::number::Number::Int(v) => Some((v, &s[parsed.end..])),
        _ => None,
    }
}

/// Parse `s` as a whole Tcl integer — the [`parse_int_prefix`] value only when
/// nothing but trailing space follows it.
fn parse_int_whole(s: &str) -> Option<i64> {
    let (val, tail) = parse_int_prefix(s)?;
    tail.trim().is_empty().then_some(val)
}

/// The canonical `bad index "<spec>": …` error.
#[must_use]
pub fn bad_index(spec: &str) -> CmdError {
    CmdError::new(format!(
        "bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_integer_and_end_forms() {
        // Mirrors Tcl's `TclGetIntForIndex` grammar (end / signed integer /
        // base[+-]integer).
        assert_eq!(resolve("5", 10).unwrap(), 5);
        assert_eq!(resolve("0", 10).unwrap(), 0);
        assert_eq!(resolve("-1", 10).unwrap(), -1);
        assert_eq!(resolve("end", 10).unwrap(), 9);
        assert_eq!(resolve("end-2", 10).unwrap(), 7);
        assert_eq!(resolve("end+1", 10).unwrap(), 10);
        // `end--1` parses as `end - (-1)`.
        assert_eq!(resolve("end--1", 10).unwrap(), 10);
        assert_eq!(resolve("1+1", 10).unwrap(), 2);
        assert_eq!(resolve("0-1", 10).unwrap(), -1);
        // `end` of an empty container is -1.
        assert_eq!(resolve("end", 0).unwrap(), -1);
    }

    #[test]
    fn resolve_rejects_garbage_and_resolve_opt_returns_none() {
        assert!(resolve("bad", 10).is_err());
        assert!(resolve("", 10).is_err());
        assert!(resolve("1.5", 10).is_err());
        assert_eq!(resolve_opt("bad", 10), None);
        assert_eq!(resolve_opt("end-1", 10), Some(8));
    }

    #[test]
    fn resolve_matches_tclsh_oracle() {
        // Ground truth captured from real tclsh 8.6 and 9.0 (identical) via
        // `string index`/`lindex` over a length-5 container (end = 4). Each
        // `resolve_opt` returns the raw resolved index (callers clamp for
        // out-of-range); a `None` is a `bad index` error at runtime.
        let ok: &[(&str, i64)] = &[
            // integers, signs, whitespace
            ("0", 0),
            ("1", 1),
            ("4", 4),
            ("5", 5),
            ("-1", -1),
            ("-2", -2),
            ("-0", 0),
            (" 1", 1),
            ("1 ", 1),
            // end and end offsets (`end--1` = end - (-1) = end + 1)
            ("end", 4),
            ("end-1", 3),
            ("end+1", 5),
            ("end-4", 0),
            ("end-5", -1),
            ("end--1", 5),
            ("end+-1", 3),
            ("end+0", 4),
            // arithmetic operands
            ("1+1", 2),
            ("0-1", -1),
            ("2+2", 4),
            ("3-1", 2),
            ("end-2", 2),
            // radix prefixes (hex / octal / binary / 0d) — oracle accepts these
            ("0x2", 2),
            ("0xa", 10),
            ("0xf", 15),
            ("0o7", 7),
            ("0b101", 5),
            ("0d5", 5),
            ("+0x2", 2),
            ("-0x1", -1),
            // radices inside the arithmetic operands
            ("end-0x2", 2),
            ("end-0o3", 1),
            ("0x2+1", 3),
            ("1+0x1", 2),
        ];
        for &(spec, want) in ok {
            assert_eq!(resolve_opt(spec, 5), Some(want), "resolve_opt({spec:?})");
        }
        // Syntactically bad — a `bad index` error at runtime.
        for spec in ["1.0", "1+", "end-", "foo", "", "1 1", "0x", "0xg", "2e0"] {
            assert_eq!(
                resolve_opt(spec, 5),
                None,
                "resolve_opt({spec:?}) should be None"
            );
        }
    }

    #[test]
    fn encodable_classifies_specs() {
        // `TclIndexEncode` scale (used by `lsearch`/`lsort -index` parse-time
        // validation): Some(true) = encodable, Some(false) = out of range,
        // None = syntactically bad.
        assert_eq!(encodable("5"), Some(true)); // absolute, non-negative
        assert_eq!(encodable("0"), Some(true));
        assert_eq!(encodable("-1"), Some(false)); // absolute negative → out of range
        assert_eq!(encodable("end"), Some(true)); // offset 0 ≤ 0
        assert_eq!(encodable("end-2"), Some(true)); // offset -2 ≤ 0
        assert_eq!(encodable("end+1"), Some(false)); // offset +1 > 0 → out of range
        assert_eq!(encodable("bad"), None); // syntactically bad
    }
}
