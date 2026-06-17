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
use crate::index;

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
    let i = index::resolve(&ops.as_str(idx), chars.len())?;
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
    let lo = index::resolve(&ops.as_str(first), len)?.max(0);
    let hi = index::resolve(&ops.as_str(last), len)?;
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

/// `string cat ?arg ...?` — concatenate the string reps of all arguments.
pub fn cat<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> O::Value {
    let mut out = String::new();
    for a in args {
        out.push_str(&ops.as_str(a));
    }
    ops.new_string(out)
}

/// The default `string trim` set — every Unicode space character plus NUL
/// (`tclDefaultTrimSet`, TIP #413).
const DEFAULT_TRIM_SET: &[char] = &[
    '\u{09}', '\u{0a}', '\u{0b}', '\u{0c}', '\u{0d}', ' ', '\u{00}', '\u{85}', '\u{a0}', '\u{1680}',
    '\u{180e}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}',
    '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{200b}', '\u{2028}', '\u{2029}', '\u{202f}',
    '\u{205f}', '\u{2060}', '\u{3000}', '\u{feff}',
];

/// `string trim`/`trimleft`/`trimright` — strip `chars` (default the TIP #413
/// whitespace set) from the requested ends.
pub fn trim<O: ValueOps>(
    ops: &mut O,
    s: &O::Value,
    chars: Option<&O::Value>,
    left: bool,
    right: bool,
) -> O::Value {
    let string = ops.as_str(s).to_string();
    let custom: Option<Vec<char>> = chars.map(|c| ops.as_str(c).chars().collect());
    let pred = |c: char| match &custom {
        Some(set) => set.contains(&c),
        None => DEFAULT_TRIM_SET.contains(&c),
    };
    let trimmed = match (left, right) {
        (true, true) => string.trim_matches(pred),
        (true, false) => string.trim_start_matches(pred),
        (false, true) => string.trim_end_matches(pred),
        (false, false) => string.as_str(),
    };
    ops.new_string(trimmed.to_string())
}

/// Arity-check (`string ?chars?`) wrapper shared by the three `trim` arms.
fn trim_dispatch<O: ValueOps>(
    ops: &mut O,
    rest: &[O::Value],
    usage: &str,
    left: bool,
    right: bool,
) -> Result<O::Value, CmdError> {
    match rest {
        [s] => Ok(trim(ops, s, None, left, right)),
        [s, c] => Ok(trim(ops, s, Some(c), left, right)),
        _ => Err(CmdError::wrong_args(usage)),
    }
}

/// Dispatch a `string` subcommand whose name `sub` is **already canonical** (the
/// runtime has resolved any unique-prefix abbreviation), with `rest` being the
/// arguments after the subcommand.
///
/// Returns `Some(result)` when the subcommand is handled here, or `None` when it
/// is not yet ported — letting a migrating runtime fall back to its legacy
/// implementation. The `Some(Err(..))` case is a genuine command error (arity,
/// bad index, …).
pub fn dispatch_canon<O: ValueOps>(
    ops: &mut O,
    sub: &str,
    rest: &[O::Value],
) -> Option<Result<O::Value, CmdError>> {
    let arity = |n: usize, usage: &str| -> Option<Result<O::Value, CmdError>> {
        if rest.len() == n {
            None
        } else {
            Some(Err(CmdError::wrong_args(usage)))
        }
    };
    match sub {
        "length" => arity(1, "string length string").or_else(|| Some(Ok(length(ops, &rest[0])))),
        "index" => arity(2, "string index string charIndex")
            .or_else(|| Some(index(ops, &rest[0], &rest[1]))),
        "range" => arity(3, "string range string first last")
            .or_else(|| Some(range(ops, &rest[0], &rest[1], &rest[2]))),
        "reverse" => {
            arity(1, "string reverse string").or_else(|| Some(Ok(reverse(ops, &rest[0]))))
        }
        "repeat" => arity(2, "string repeat string count")
            .or_else(|| Some(repeat(ops, &rest[0], &rest[1]))),
        "cat" => Some(Ok(cat(ops, rest))),
        "trim" => Some(trim_dispatch(ops, rest, "string trim string ?chars?", true, true)),
        "trimleft" => Some(trim_dispatch(
            ops,
            rest,
            "string trimleft string ?chars?",
            true,
            false,
        )),
        "trimright" => Some(trim_dispatch(
            ops,
            rest,
            "string trimright string ?chars?",
            false,
            true,
        )),
        // Not yet ported into the core — caller falls back to its legacy path.
        // (`toupper`/`tolower`/`totitle` need the optional `?first? ?last?` range
        // form before they can move here without a regression.)
        _ => None,
    }
}

/// Dispatch a `string` subcommand from the raw argument vector (`args[0]` is the
/// subcommand). Convenience over [`dispatch_canon`] for a runtime that does not
/// pre-resolve abbreviations; exact subcommand names only.
pub fn dispatch<O: ValueOps>(
    ops: &mut O,
    args: &[O::Value],
) -> Option<Result<O::Value, CmdError>> {
    let sub = ops.as_str(args.first()?).to_string();
    dispatch_canon(ops, &sub, &args[1..])
}

