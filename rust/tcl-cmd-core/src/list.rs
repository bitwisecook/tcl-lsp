//! Portable `list`-family command logic, generic over [`ValueOps`].
//!
//! The pure list operations — those that read/build list values without touching
//! interpreter variables. The variable-mutating members (`lappend`, `lassign`)
//! stay per-runtime over the Family-B var store. Indices go through the shared
//! [`crate::index`] parser, so a bad index errors faithfully (where some legacy
//! paths silently returned empty).
//!
//! [`ValueOps`]: tcl_syntax::value::ValueOps

use tcl_syntax::value::ValueOps;

use crate::error::CmdError;
use crate::index;

fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// `list ?arg ...?` — a list of the arguments verbatim.
pub fn list<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> O::Value {
    ops.new_list(args.to_vec())
}

/// `llength list`.
pub fn llength<O: ValueOps>(ops: &mut O, value: &O::Value) -> Result<O::Value, CmdError> {
    let n = ops.list_len(value)?;
    Ok(ops.new_int(ilen(n)))
}

/// `lindex list ?index ...?` — descend through each index; an out-of-range index
/// yields the empty string, a malformed index spec errors.
///
/// A lone index argument is itself split into an index *path*
/// (`lindex {{a b} c} {0 1}` → `b`); multiple arguments are each a single index.
/// Mirrors `Tcl_LindexObjCmd` (`TclLindexList`/`TclLindexFlat`).
pub fn lindex<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    idxs: &[O::Value],
) -> Result<O::Value, CmdError> {
    let specs: Vec<String> = if idxs.len() == 1 {
        let s = ops.as_str(&idxs[0]);
        tcl_syntax::list::split_list(&s)
            .map_err(|e| CmdError::new(e.message()))?
            .iter()
            .map(|p| p.as_ref().to_string())
            .collect()
    } else {
        idxs.iter().map(|i| ops.as_str(i).to_string()).collect()
    };
    let mut cur = value.clone();
    for spec in &specs {
        let elems = ops.list_elements(&cur)?;
        let i = index::resolve(spec, elems.len())?;
        match usize::try_from(i).ok().filter(|&i| i < elems.len()) {
            Some(i) => cur = elems[i].clone(),
            None => return Ok(ops.empty()),
        }
    }
    Ok(cur)
}

/// `lrange list first last` — the inclusive sublist, clamped.
pub fn lrange<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    first: &O::Value,
    last: &O::Value,
) -> Result<O::Value, CmdError> {
    let elems = ops.list_elements(value)?;
    let len = elems.len();
    let lo = index::resolve(&ops.as_str(first), len)?.max(0);
    let hi = index::resolve(&ops.as_str(last), len)?;
    let Ok(lo) = usize::try_from(lo) else {
        return Ok(ops.new_list(Vec::new()));
    };
    if hi < 0 || lo >= len {
        return Ok(ops.new_list(Vec::new()));
    }
    let hi = usize::try_from(hi).unwrap_or(usize::MAX).min(len - 1);
    if lo > hi {
        return Ok(ops.new_list(Vec::new()));
    }
    Ok(ops.new_list(elems[lo..=hi].to_vec()))
}

/// `lreverse list`.
pub fn lreverse<O: ValueOps>(ops: &mut O, value: &O::Value) -> Result<O::Value, CmdError> {
    let mut elems = ops.list_elements(value)?;
    elems.reverse();
    Ok(ops.new_list(elems))
}

/// `lrepeat count ?value ...?` — `count` copies of the value sequence.
pub fn lrepeat<O: ValueOps>(
    ops: &mut O,
    count: &O::Value,
    values: &[O::Value],
) -> Result<O::Value, CmdError> {
    let n = ops.as_int(count)?;
    if n < 0 {
        return Err(CmdError::new(format!("bad count \"{n}\": must be >= 0")));
    }
    let n = usize::try_from(n).unwrap_or(0);
    let mut out = Vec::with_capacity(n.saturating_mul(values.len()));
    for _ in 0..n {
        out.extend(values.iter().cloned());
    }
    Ok(ops.new_list(out))
}

/// The ASCII whitespace Tcl trims/splits on (space, tab, newline, CR, VT, FF).
const TCL_WS: &[char] = &[' ', '\t', '\n', '\r', '\u{0b}', '\u{0c}'];

/// `concat ?arg ...?` — trim each argument and join with single spaces, dropping
/// the args that are empty after trimming.
pub fn concat<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> O::Value {
    let mut parts: Vec<String> = Vec::new();
    for v in args {
        let s = ops.as_str(v);
        let t = s.trim_matches(TCL_WS);
        if !t.is_empty() {
            parts.push(t.to_string());
        }
    }
    ops.new_string(parts.join(" "))
}

/// `join list ?joinString?` (default separator a single space).
pub fn join<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    sep: Option<&O::Value>,
) -> Result<O::Value, CmdError> {
    let sep = sep.map_or_else(|| " ".to_string(), |s| ops.as_str(s).to_string());
    let elems = ops.list_elements(value)?;
    let mut parts: Vec<String> = Vec::with_capacity(elems.len());
    for e in &elems {
        parts.push(ops.as_str(e).to_string());
    }
    Ok(ops.new_string(parts.join(&sep)))
}

/// `split string ?splitChars?` — split into a list. The default split set is
/// whitespace; an empty split set splits into individual characters.
pub fn split<O: ValueOps>(ops: &mut O, value: &O::Value, chars: Option<&O::Value>) -> O::Value {
    let string = ops.as_str(value).to_string();
    let set = chars.map(|c| ops.as_str(c).to_string());
    let pieces: Vec<String> = if string.is_empty() {
        // `split ""` is the empty list (not a single empty element).
        Vec::new()
    } else if set.as_deref() == Some("") {
        // An empty split set makes each character its own element.
        string.chars().map(|c| c.to_string()).collect()
    } else {
        let set: Vec<char> = set.map_or_else(|| TCL_WS.to_vec(), |c| c.chars().collect());
        string.split(|c| set.contains(&c)).map(str::to_string).collect()
    };
    let mut values = Vec::with_capacity(pieces.len());
    for p in pieces {
        values.push(ops.new_string(p));
    }
    ops.new_list(values)
}
