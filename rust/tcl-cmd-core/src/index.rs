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
    let operand: i64 = rest[1..].trim().parse().ok()?;
    let offset = if connector == b'-' { -operand } else { operand };
    Some(base + offset)
}

/// Parse a leading optionally-signed integer, returning its value and the
/// unconsumed tail. `None` if no digits follow the optional sign.
fn parse_int_prefix(s: &str) -> Option<(i64, &str)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let digits_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        return None;
    }
    let val: i64 = s[..i].parse().ok()?;
    Some((val, &s[i..]))
}

/// The canonical `bad index "<spec>": …` error.
#[must_use]
pub fn bad_index(spec: &str) -> CmdError {
    CmdError::new(format!(
        "bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?"
    ))
}
