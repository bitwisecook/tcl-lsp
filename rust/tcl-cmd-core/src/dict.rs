//! Portable `dict`-family command logic, generic over [`ValueOps`].
//!
//! The pure dict operations — read/build dict values without touching
//! interpreter variables. Keys are compared by string rep ([`ValueOps::as_str`]),
//! and dicts are canonicalised (last value wins, first-occurrence order) by the
//! [`ValueOps::dict_pairs`] seam. The variable-mutating members (`dict set`/
//! `unset`/`incr`/`append`/`lappend`/`for`/`update`/`with`) keep per-runtime
//! adapters that reuse [`upsert`]/[`lookup`] over the same seam.
//!
//! [`ValueOps`]: tcl_syntax::value::ValueOps

use tcl_syntax::glob::string_match;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Find the value for `key` (string-compared) in `pairs`.
pub fn lookup<O: ValueOps>(
    ops: &mut O,
    pairs: &[(O::Value, O::Value)],
    key: &str,
) -> Option<O::Value> {
    for (k, v) in pairs {
        if *ops.as_str(k) == *key {
            return Some(v.clone());
        }
    }
    None
}

/// Insert or update `key` in `pairs` (last value wins, position preserved).
pub fn upsert<O: ValueOps>(
    ops: &mut O,
    pairs: &mut Vec<(O::Value, O::Value)>,
    key: &O::Value,
    value: O::Value,
) {
    let ks = ops.as_str(key).to_string();
    let mut found = None;
    for (i, (k, _)) in pairs.iter().enumerate() {
        if *ops.as_str(k) == *ks {
            found = Some(i);
            break;
        }
    }
    match found {
        Some(i) => pairs[i].1 = value,
        None => pairs.push((key.clone(), value)),
    }
}

/// `dict create ?key value ...?`.
pub fn create<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Result<O::Value, CmdError> {
    if args.len() % 2 != 0 {
        return Err(CmdError::wrong_args("dict create ?key value ...?"));
    }
    let mut pairs: Vec<(O::Value, O::Value)> = Vec::new();
    for chunk in args.chunks_exact(2) {
        upsert(ops, &mut pairs, &chunk[0], chunk[1].clone());
    }
    Ok(ops.new_dict(pairs))
}

/// `dict get dictionary ?key ...?` — descend nested keys; a missing key errors.
pub fn get<O: ValueOps>(
    ops: &mut O,
    dict: &O::Value,
    keys: &[O::Value],
) -> Result<O::Value, CmdError> {
    let mut cur = dict.clone();
    for k in keys {
        let ks = ops.as_str(k).to_string();
        let pairs = ops.dict_pairs(&cur)?;
        match lookup(ops, &pairs, &ks) {
            Some(v) => cur = v,
            None => return Err(CmdError::new(format!("key \"{ks}\" not known in dictionary"))),
        }
    }
    Ok(cur)
}

/// `dict exists dictionary key ?key ...?` — boolean, never errors on a missing key.
pub fn exists<O: ValueOps>(ops: &mut O, dict: &O::Value, keys: &[O::Value]) -> O::Value {
    let mut cur = dict.clone();
    for k in keys {
        let ks = ops.as_str(k).to_string();
        let Ok(pairs) = ops.dict_pairs(&cur) else {
            return ops.new_bool(false);
        };
        match lookup(ops, &pairs, &ks) {
            Some(v) => cur = v,
            None => return ops.new_bool(false),
        }
    }
    ops.new_bool(true)
}

/// `dict keys dictionary ?globPattern?`.
pub fn keys<O: ValueOps>(
    ops: &mut O,
    dict: &O::Value,
    pattern: Option<&O::Value>,
) -> Result<O::Value, CmdError> {
    let pat = pattern.map(|p| ops.as_str(p).to_string());
    let pairs = ops.dict_pairs(dict)?;
    let mut out = Vec::new();
    for (k, _) in &pairs {
        let ks = ops.as_str(k);
        if pat.as_deref().is_none_or(|p| string_match(p, &ks)) {
            out.push(k.clone());
        }
    }
    Ok(ops.new_list(out))
}

/// `dict values dictionary ?globPattern?`.
pub fn values<O: ValueOps>(
    ops: &mut O,
    dict: &O::Value,
    pattern: Option<&O::Value>,
) -> Result<O::Value, CmdError> {
    let pat = pattern.map(|p| ops.as_str(p).to_string());
    let pairs = ops.dict_pairs(dict)?;
    let mut out = Vec::new();
    for (_, v) in &pairs {
        let vs = ops.as_str(v);
        if pat.as_deref().is_none_or(|p| string_match(p, &vs)) {
            out.push(v.clone());
        }
    }
    Ok(ops.new_list(out))
}

/// `dict size dictionary`.
pub fn size<O: ValueOps>(ops: &mut O, dict: &O::Value) -> Result<O::Value, CmdError> {
    let n = ops.dict_pairs(dict)?.len();
    Ok(ops.new_int(ilen(n)))
}

/// `dict merge ?dictionary ...?` — later dicts override earlier keys.
pub fn merge<O: ValueOps>(ops: &mut O, dicts: &[O::Value]) -> Result<O::Value, CmdError> {
    let mut acc: Vec<(O::Value, O::Value)> = Vec::new();
    for d in dicts {
        let pairs = ops.dict_pairs(d)?;
        for (k, v) in pairs {
            upsert(ops, &mut acc, &k, v);
        }
    }
    Ok(ops.new_dict(acc))
}

/// Dispatch a pure `dict` subcommand. `rest` is the args after the subcommand.
/// Returns `None` for a not-yet-ported (variable-mutating) subcommand so the
/// caller falls back to its legacy path.
pub fn dispatch_canon<O: ValueOps>(
    ops: &mut O,
    sub: &str,
    rest: &[O::Value],
) -> Option<Result<O::Value, CmdError>> {
    match sub {
        "create" => Some(create(ops, rest)),
        "get" => match rest.split_first() {
            Some((d, keys)) => Some(get(ops, d, keys)),
            None => Some(Err(CmdError::wrong_args("dict get dictionary ?key ...?"))),
        },
        "exists" => match rest.split_first() {
            Some((d, keys)) if !keys.is_empty() => Some(Ok(exists(ops, d, keys))),
            _ => Some(Err(CmdError::wrong_args(
                "dict exists dictionary key ?key ...?",
            ))),
        },
        "keys" => match rest {
            [d] => Some(keys(ops, d, None)),
            [d, p] => Some(keys(ops, d, Some(p))),
            _ => Some(Err(CmdError::wrong_args("dict keys dictionary ?pattern?"))),
        },
        "values" => match rest {
            [d] => Some(values(ops, d, None)),
            [d, p] => Some(values(ops, d, Some(p))),
            _ => Some(Err(CmdError::wrong_args(
                "dict values dictionary ?pattern?",
            ))),
        },
        "size" => match rest {
            [d] => Some(size(ops, d)),
            _ => Some(Err(CmdError::wrong_args("dict size dictionary"))),
        },
        "merge" => Some(merge(ops, rest)),
        _ => None,
    }
}
