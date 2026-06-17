//! Portable `string` command logic, generic over [`ValueOps`].
//!
//! Each helper takes already-sliced argument values and returns
//! `Result<O::Value, CmdError>`. All indexing is by **character** (the contract
//! in [`tcl_syntax::value`]), so a byte-oriented runtime conforms via its
//! `ValueOps` impl and these bodies stay correct everywhere.
//!
//! This is the Phase-1 proving subset (`length`/`index`/`range`/`reverse`/
//! `repeat`/`toupper`/`tolower`); the remaining subcommands fill in during
//! rollout. [`dispatch`] returns `None` for a not-yet-ported subcommand so a
//! runtime can fall back to its legacy implementation during migration.

use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// `string length str` — the character count.
pub fn length<O: ValueOps>(ops: &mut O, s: &O::Value) -> O::Value {
    let n = ops.char_len(s);
    ops.new_int(i64::try_from(n).unwrap_or(i64::MAX))
}

/// `string index str charIndex` — the one-character string at `idx`, or empty
/// when out of range.
pub fn index<O: ValueOps>(
    ops: &mut O,
    s: &O::Value,
    idx: &O::Value,
) -> Result<O::Value, CmdError> {
    let chars: Vec<char> = ops.as_str(s).chars().collect();
    let i = parse_index(&ops.as_str(idx), chars.len())?;
    if i < 0 {
        return Ok(ops.empty());
    }
    match usize::try_from(i).ok().and_then(|i| chars.get(i)) {
        Some(c) => Ok(ops.new_string(c.to_string())),
        None => Ok(ops.empty()),
    }
}

/// `string range str first last` — the inclusive character slice, clamped.
pub fn range<O: ValueOps>(
    ops: &mut O,
    s: &O::Value,
    first: &O::Value,
    last: &O::Value,
) -> Result<O::Value, CmdError> {
    let chars: Vec<char> = ops.as_str(s).chars().collect();
    let len = chars.len();
    let lo = parse_index(&ops.as_str(first), len)?.max(0);
    let hi = parse_index(&ops.as_str(last), len)?;
    let Ok(lo) = usize::try_from(lo) else {
        return Ok(ops.empty());
    };
    if hi < 0 || lo >= len || lo > usize::try_from(hi).unwrap_or(usize::MAX) {
        return Ok(ops.empty());
    }
    let hi = usize::try_from(hi).unwrap_or(usize::MAX).min(len - 1);
    let out: String = chars[lo..=hi].iter().collect();
    Ok(ops.new_string(out))
}

/// `string reverse str`.
pub fn reverse<O: ValueOps>(ops: &mut O, s: &O::Value) -> O::Value {
    let out: String = ops.as_str(s).chars().rev().collect();
    ops.new_string(out)
}

/// `string repeat str count` — `count` (clamped at 0) copies.
pub fn repeat<O: ValueOps>(
    ops: &mut O,
    s: &O::Value,
    count: &O::Value,
) -> Result<O::Value, CmdError> {
    let n = ops.as_int(count)?;
    if n <= 0 {
        return Ok(ops.empty());
    }
    let src = ops.as_str(s);
    let n = usize::try_from(n).unwrap_or(0);
    Ok(ops.new_string(src.repeat(n)))
}

/// `string toupper str` (whole-string form).
pub fn to_upper<O: ValueOps>(ops: &mut O, s: &O::Value) -> O::Value {
    let out = ops.as_str(s).to_uppercase();
    ops.new_string(out)
}

/// `string tolower str` (whole-string form).
pub fn to_lower<O: ValueOps>(ops: &mut O, s: &O::Value) -> O::Value {
    let out = ops.as_str(s).to_lowercase();
    ops.new_string(out)
}

/// Dispatch a `string` subcommand to a ported helper. `args` is the argument
/// vector **after** the command name (`args[0]` is the subcommand).
///
/// Returns `Some(result)` when the subcommand is handled here, or `None` when it
/// is not yet ported — letting a migrating runtime fall back to its legacy
/// implementation. The `Some(Err(..))` case is a genuine command error (arity,
/// bad index, …).
pub fn dispatch<O: ValueOps>(
    ops: &mut O,
    args: &[O::Value],
) -> Option<Result<O::Value, CmdError>> {
    let sub = ops.as_str(args.first()?).to_string();
    let rest = &args[1..];
    let arity = |n: usize, usage: &str| -> Option<Result<O::Value, CmdError>> {
        if rest.len() == n {
            None
        } else {
            Some(Err(CmdError::wrong_args(usage)))
        }
    };
    match sub.as_str() {
        "length" => match arity(1, "string length string") {
            Some(e) => Some(e),
            None => Some(Ok(length(ops, &rest[0]))),
        },
        "index" => match arity(2, "string index string charIndex") {
            Some(e) => Some(e),
            None => Some(index(ops, &rest[0], &rest[1])),
        },
        "range" => match arity(3, "string range string first last") {
            Some(e) => Some(e),
            None => Some(range(ops, &rest[0], &rest[1], &rest[2])),
        },
        "reverse" => match arity(1, "string reverse string") {
            Some(e) => Some(e),
            None => Some(Ok(reverse(ops, &rest[0]))),
        },
        "repeat" => match arity(2, "string repeat string count") {
            Some(e) => Some(e),
            None => Some(repeat(ops, &rest[0], &rest[1])),
        },
        "toupper" => match arity(1, "string toupper string ?first? ?last?") {
            Some(e) => Some(e),
            None => Some(Ok(to_upper(ops, &rest[0]))),
        },
        "tolower" => match arity(1, "string tolower string ?first? ?last?") {
            Some(e) => Some(e),
            None => Some(Ok(to_lower(ops, &rest[0]))),
        },
        // Not yet ported into the core — caller falls back to its legacy path.
        _ => None,
    }
}

/// Parse a Tcl string index (`Tcl_GetIntForIndex`): `integer`, `integer±integer`,
/// `end`, or `end±integer`, resolved against `char_len` (so `end` is
/// `char_len - 1`). The result may be out of range; callers clamp.
fn parse_index(s: &str, char_len: usize) -> Result<i64, CmdError> {
    let s = s.trim();
    let len = i64::try_from(char_len).unwrap_or(i64::MAX);
    if let Some(rest) = s.strip_prefix("end") {
        if rest.is_empty() {
            return Ok(len - 1);
        }
        if let Some(off) = parse_signed(rest) {
            return Ok(len - 1 + off);
        }
        return Err(bad_index(s));
    }
    parse_arith(s).ok_or_else(|| bad_index(s))
}

/// Parse `±integer` (a leading sign is required).
fn parse_signed(s: &str) -> Option<i64> {
    if s.starts_with(['+', '-']) {
        s.parse::<i64>().ok()
    } else {
        None
    }
}

/// Parse `integer` or `integer±integer`.
fn parse_arith(s: &str) -> Option<i64> {
    if let Ok(n) = s.parse::<i64>() {
        return Some(n);
    }
    // Split on the operator that is not the (optional) leading sign.
    let bytes = s.as_bytes();
    for i in 1..bytes.len() {
        if bytes[i] == b'+' || bytes[i] == b'-' {
            let a = s[..i].parse::<i64>().ok()?;
            let b = s[i..].parse::<i64>().ok()?; // includes the sign
            return Some(a + b);
        }
    }
    None
}

fn bad_index(s: &str) -> CmdError {
    CmdError::new(format!(
        "bad index \"{s}\": must be integer?[+-]integer? or end?[+-]integer?"
    ))
}
