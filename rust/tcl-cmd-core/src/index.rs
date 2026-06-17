//! Shared Tcl index parsing (`Tcl_GetIntForIndex` / `GetEndOffsetFromObj`).
//!
//! One parser for every indexed command — `string index/range`, `lindex`,
//! `lrange`, `linsert`, `lreplace`, … — so the accepted forms and the error
//! message live once. The grammar is a **base** (`end`, or a signed integer)
//! optionally followed by a **connector** (`+`/`-`) and a (possibly signed)
//! integer operand: `5`, `-2`, `end`, `end-2`, `1+1`, `0-1`, `end--1`
//! (= `end - (-1)`).

use crate::error::CmdError;

/// Resolve an index `spec` against a container length `len` (so `end` is
/// `len - 1`). The result may be negative or `>= len`; callers clamp per their
/// command's rules. Errors with the canonical message on an unparseable spec.
pub fn resolve(spec: &str, len: usize) -> Result<i64, CmdError> {
    parse(spec, len).ok_or_else(|| bad_index(spec.trim()))
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
