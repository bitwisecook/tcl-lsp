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

/// Key → position index over `pairs`, so the dict build commands can upsert
/// in O(1) instead of [`upsert`]'s O(N) linear scan (D3).
fn index_of_pairs<O: ValueOps>(
    ops: &mut O,
    pairs: &[(O::Value, O::Value)],
) -> std::collections::HashMap<String, usize> {
    let mut index = std::collections::HashMap::with_capacity(pairs.len());
    for (i, (k, _)) in pairs.iter().enumerate() {
        index.insert(ops.as_str(k).to_string(), i);
    }
    index
}

/// [`upsert`] against a maintained key→position `index` (last value wins,
/// position preserved). Keeps the batch dict builders O(N) overall rather
/// than O(N²).
fn upsert_indexed<O: ValueOps>(
    ops: &mut O,
    pairs: &mut Vec<(O::Value, O::Value)>,
    index: &mut std::collections::HashMap<String, usize>,
    key: &O::Value,
    value: O::Value,
) {
    let ks = ops.as_str(key).to_string();
    if let Some(&i) = index.get(&ks) {
        pairs[i].1 = value;
    } else {
        index.insert(ks, pairs.len());
        pairs.push((key.clone(), value));
    }
}

/// `dict create ?key value ...?`.
pub fn create<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Result<O::Value, CmdError> {
    if args.len() % 2 != 0 {
        return Err(CmdError::wrong_args("dict create ?key value ...?"));
    }
    let mut pairs: Vec<(O::Value, O::Value)> = Vec::new();
    let mut index = std::collections::HashMap::new();
    for chunk in args.as_chunks::<2>().0 {
        upsert_indexed(ops, &mut pairs, &mut index, &chunk[0], chunk[1].clone());
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
            None => {
                return Err(CmdError::new(format!(
                    "key \"{ks}\" not known in dictionary"
                )));
            }
        }
    }
    // `dict get` parses its dictionary argument even with no keys, so a
    // malformed (odd-length) value errors (`dict get {a 1 b}` → "missing value
    // to go with key"). With keys present, each intermediate level was already
    // parsed in the loop, and `cur` is now a *leaf* value that must not itself
    // be dict-parsed — so validate only in the no-key case.
    if keys.is_empty() {
        ops.dict_pairs(&cur)?;
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

/// `dict filter dictionary key|value ?globPattern ...?` — keep entries whose key
/// (or value) matches **any** of the glob patterns (with no patterns, nothing
/// matches, so the result is empty). The `script` filter type is Family-B (it
/// evaluates a body per pair) and stays per-adapter — this returns `None` for it
/// and for an unhandled arg shape so the caller falls back.
///
/// `by_key` selects the `key` form vs `value`.
pub fn filter<O: ValueOps>(
    ops: &mut O,
    dict: &O::Value,
    by_key: bool,
    patterns: &[O::Value],
) -> Result<O::Value, CmdError> {
    let pats: Vec<String> = patterns.iter().map(|p| ops.as_str(p).to_string()).collect();
    let pairs = ops.dict_pairs(dict)?;
    let mut out: Vec<O::Value> = Vec::new();
    for (k, v) in pairs {
        let target = ops.as_str(if by_key { &k } else { &v });
        if pats.iter().any(|p| string_match(p, &target)) {
            out.push(k);
            out.push(v);
        }
    }
    Ok(ops.new_list(out))
}

/// `dict merge ?dictionary ...?` — later dicts override earlier keys.
pub fn merge<O: ValueOps>(ops: &mut O, dicts: &[O::Value]) -> Result<O::Value, CmdError> {
    let mut acc: Vec<(O::Value, O::Value)> = Vec::new();
    let mut index = std::collections::HashMap::new();
    for d in dicts {
        let pairs = ops.dict_pairs(d)?;
        for (k, v) in pairs {
            upsert_indexed(ops, &mut acc, &mut index, &k, v);
        }
    }
    Ok(ops.new_dict(acc))
}

/// `dict replace dictionary ?key value ...?` — the dict with the pairs upserted
/// (last value wins, position preserved). An odd number of `kv` args errors.
pub fn replace<O: ValueOps>(
    ops: &mut O,
    dict: &O::Value,
    kv: &[O::Value],
) -> Result<O::Value, CmdError> {
    if kv.len() % 2 != 0 {
        return Err(CmdError::wrong_args(
            "dict replace dictionary ?key value ...?",
        ));
    }
    let mut pairs = ops.dict_pairs(dict)?;
    let mut index = index_of_pairs(ops, &pairs);
    for chunk in kv.as_chunks::<2>().0 {
        upsert_indexed(ops, &mut pairs, &mut index, &chunk[0], chunk[1].clone());
    }
    Ok(ops.new_dict(pairs))
}

/// `dict remove dictionary ?key ...?` — the dict without the given keys (a
/// missing key is not an error). The result is canonicalised.
pub fn remove<O: ValueOps>(
    ops: &mut O,
    dict: &O::Value,
    keys: &[O::Value],
) -> Result<O::Value, CmdError> {
    let pairs = ops.dict_pairs(dict)?;
    let drop: Vec<String> = keys.iter().map(|k| ops.as_str(k).to_string()).collect();
    let mut kept: Vec<(O::Value, O::Value)> = Vec::with_capacity(pairs.len());
    for (k, v) in pairs {
        if !drop.contains(&ops.as_str(&k).to_string()) {
            kept.push((k, v));
        }
    }
    Ok(ops.new_dict(kept))
}

/// `dict getdef`/`getwithdefault dictionary ?key ...? key default` — like
/// [`get`] over a key path, but returns `default` when any key is absent (a
/// malformed dict or a non-dict intermediate value still errors).
pub fn getdef<O: ValueOps>(
    ops: &mut O,
    dict: &O::Value,
    keys: &[O::Value],
    default: &O::Value,
) -> Result<O::Value, CmdError> {
    let mut cur = dict.clone();
    for k in keys {
        let ks = ops.as_str(k).to_string();
        let pairs = ops.dict_pairs(&cur)?;
        match lookup(ops, &pairs, &ks) {
            Some(v) => cur = v,
            None => return Ok(default.clone()),
        }
    }
    Ok(cur)
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
        // `key`/`value` are pure (glob); `script` is Family-B → `None` so the
        // caller's adapter handles it. The filterType is validated *before* the
        // dict is parsed (tclsh: `dict filter {a b c} bogus` is a bad-filterType
        // error, not a bad-dict one).
        "filter" => match (rest.first(), rest.get(1)) {
            (Some(dict), Some(ft)) => match ops.as_str(ft).as_ref() {
                "key" => Some(filter(ops, dict, true, &rest[2..])),
                "value" => Some(filter(ops, dict, false, &rest[2..])),
                "script" => None,
                other => Some(Err(CmdError::new(format!(
                    "bad filterType \"{other}\": must be key, script, or value"
                )))),
            },
            _ => Some(Err(CmdError::wrong_args(
                "dict filter dictionary filterType ?arg ...?",
            ))),
        },
        "replace" => match rest.split_first() {
            Some((d, kv)) => Some(replace(ops, d, kv)),
            None => Some(Err(CmdError::wrong_args(
                "dict replace dictionary ?key value ...?",
            ))),
        },
        "remove" => match rest.split_first() {
            Some((d, keys)) => Some(remove(ops, d, keys)),
            None => Some(Err(CmdError::wrong_args(
                "dict remove dictionary ?key ...?",
            ))),
        },
        // `dict getdef`/`getwithdefault dictionary ?key ...? key default` — needs
        // the dict, at least one key, and a default (≥ 3 args). The usage echoes
        // the invoked sub-name.
        "getdef" | "getwithdefault" => {
            if rest.len() < 3 {
                Some(Err(CmdError::wrong_args(&format!(
                    "dict {sub} dictionary ?key ...? key default"
                ))))
            } else {
                let dict = &rest[0];
                let default = &rest[rest.len() - 1];
                let keys = &rest[1..rest.len() - 1];
                Some(getdef(ops, dict, keys, default))
            }
        }
        _ => None,
    }
}
