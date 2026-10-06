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
//! The pure dict operations — read/build/describe dict values without touching
//! interpreter variables. Keys are compared by string rep ([`ValueOps::as_str`]),
//! and dicts are canonicalised (last value wins, first-occurrence order) by the
//! [`ValueOps::dict_pairs`] seam. `dict info` also routes through the shared Tcl
//! hash-table owner. The variable-mutating members (`dict set`/
//! `unset`/`incr`/`append`/`lappend`/`for`/`update`/`with`) keep per-runtime
//! adapters that reuse [`upsert`]/[`lookup`] over the same seam.
//!
//! [`ValueOps`]: tcl_syntax::value::ValueOps

use tcl_syntax::glob::string_match_bytes;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;
use crate::namespace::TclStringHashOrder;

/// Render the selected lookup stage's error without changing dictionary key
/// equality or rebuilding an original key object.
pub fn missing_key_error<O: ValueOps>(
    ops: &mut O,
    operation: tcl_syntax::naming::NativeDictionaryMissingKeyOperation,
    original: &[u8],
) -> CmdError {
    let Some(policy) = ops.name_policy_protocol() else {
        return tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native dictionary missing-key diagnostic",
        )
        .into();
    };
    match tcl_syntax::naming::report_native_dictionary_missing_key(
        policy.recipe(),
        operation,
        original,
    ) {
        Ok(report) => match report.error_code {
            Some(code) => CmdError::with_error_code_bytes(report.message, code),
            None => CmdError::new_bytes(report.message),
        },
        Err(_) => tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native dictionary missing-key diagnostic",
        )
        .into(),
    }
}

/// Re-word a list-codec parse failure as the dict failure C reports.
///
/// `SetDictFromAny` uses the same element grammar with the type strings
/// `dict`/`DICTIONARY`; the shared list codec therefore supplies the structure
/// and this owner supplies the public noun. Already dict-specific messages pass
/// through unchanged.
#[must_use]
pub fn worded_parse_error(message: &str) -> String {
    if let Some(rest) = message.strip_prefix("list element in ") {
        format!("dict element in {rest}")
    } else if message == "unmatched open brace in list" {
        "unmatched open brace in dict".to_owned()
    } else if message == "unmatched open quote in list" {
        "unmatched open quote in dict".to_owned()
    } else {
        message.to_owned()
    }
}

fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Find the value for `key` (string-compared) in `pairs`.
pub fn lookup<O: ValueOps>(
    ops: &mut O,
    pairs: &[(O::Value, O::Value)],
    key: impl AsRef<[u8]>,
) -> Result<Option<O::Value>, CmdError> {
    let key = key.as_ref();
    for (k, v) in pairs {
        if *ops.native_string_bytes(k)? == *key {
            return Ok(Some(v.clone()));
        }
    }
    Ok(None)
}

/// Insert or update `key` in `pairs` (last value wins, position preserved).
pub fn upsert<O: ValueOps>(
    ops: &mut O,
    pairs: &mut Vec<(O::Value, O::Value)>,
    key: &O::Value,
    value: O::Value,
) -> Result<(), CmdError> {
    let ks = ops.native_string_bytes(key)?.to_vec();
    let mut found = None;
    for (i, (k, _)) in pairs.iter().enumerate() {
        if *ops.native_string_bytes(k)? == *ks {
            found = Some(i);
            break;
        }
    }
    match found {
        Some(i) => pairs[i].1 = value,
        None => pairs.push((key.clone(), value)),
    }
    Ok(())
}

/// Key → position index over `pairs`, so the dict build commands can upsert
/// in O(1) instead of [`upsert`]'s O(N) linear scan.
fn index_of_pairs<O: ValueOps>(
    ops: &mut O,
    pairs: &[(O::Value, O::Value)],
) -> Result<std::collections::HashMap<Vec<u8>, usize>, CmdError> {
    let mut index = std::collections::HashMap::with_capacity(pairs.len());
    for (i, (k, _)) in pairs.iter().enumerate() {
        index.insert(ops.native_string_bytes(k)?.to_vec(), i);
    }
    Ok(index)
}

/// Bucket-array size produced by Tcl's native dict-copy operation.
///
/// `DupDictInternalRep` starts from four buckets and reinserts every live key;
/// it does not inherit deleted-entry history from the source object. Commands
/// that copy before transforming use this size as their new table's baseline.
fn copied_hash_bucket_count<O: ValueOps>(
    ops: &mut O,
    pairs: &[(O::Value, O::Value)],
) -> Result<usize, CmdError> {
    let mut table = TclStringHashOrder::default();
    for (key, _) in pairs {
        table.insert(&ops.native_string_bytes(key)?);
    }
    Ok(table.bucket_count())
}

/// [`upsert`] against a maintained key→position `index` (last value wins,
/// position preserved). Keeps the batch dict builders O(N) overall rather
/// than O(N²).
fn upsert_indexed<O: ValueOps>(
    ops: &mut O,
    pairs: &mut Vec<(O::Value, O::Value)>,
    index: &mut std::collections::HashMap<Vec<u8>, usize>,
    key: &O::Value,
    value: O::Value,
) -> Result<(), CmdError> {
    let ks = ops.native_string_bytes(key)?.to_vec();
    if let Some(&i) = index.get(&ks) {
        pairs[i].1 = value;
    } else {
        index.insert(ks, pairs.len());
        pairs.push((key.clone(), value));
    }
    Ok(())
}

/// `dict create ?key value ...?`.
pub fn create<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Result<O::Value, CmdError> {
    if args.len() % 2 != 0 {
        return Err(CmdError::wrong_args("dict create ?key value ...?"));
    }
    let mut pairs: Vec<(O::Value, O::Value)> = Vec::new();
    let mut index = std::collections::HashMap::new();
    for chunk in args.as_chunks::<2>().0 {
        upsert_indexed(ops, &mut pairs, &mut index, &chunk[0], chunk[1].clone())?;
    }
    Ok(ops.new_dict_checked(pairs)?)
}

/// `dict get dictionary ?key ...?` — descend nested keys; a missing key errors.
pub fn get<O: ValueOps>(
    ops: &mut O,
    dict: &O::Value,
    keys: &[O::Value],
) -> Result<O::Value, CmdError> {
    let mut cur = dict.clone();
    for k in keys {
        let ks = ops.native_string_bytes(k)?.to_vec();
        let pairs = ops.dict_pairs(&cur)?;
        if let Some(value) = lookup(ops, &pairs, &ks)? {
            cur = value;
        } else {
            return Err(missing_key_error(
                ops,
                tcl_syntax::naming::NativeDictionaryMissingKeyOperation::Get,
                &ks,
            ));
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
pub fn exists<O: ValueOps>(
    ops: &mut O,
    dict: &O::Value,
    keys: &[O::Value],
) -> Result<O::Value, CmdError> {
    let mut cur = dict.clone();
    for k in keys {
        let ks = ops.native_string_bytes(k)?.to_vec();
        let pairs = match ops.dict_pairs(&cur) {
            Ok(pairs) => pairs,
            Err(error) => {
                let error = CmdError::from(error);
                if error.native_access_refusal().is_some() {
                    return Err(error);
                }
                return Ok(ops.new_bool(false));
            }
        };
        match lookup(ops, &pairs, &ks)? {
            Some(v) => cur = v,
            None => return Ok(ops.new_bool(false)),
        }
    }
    Ok(ops.new_bool(true))
}

/// `dict keys dictionary ?globPattern?`.
pub fn keys<O: ValueOps>(
    ops: &mut O,
    dict: &O::Value,
    pattern: Option<&O::Value>,
) -> Result<O::Value, CmdError> {
    let pat = pattern.map(|p| ops.as_bytes(p));
    let pairs = ops.dict_pairs(dict)?;
    let mut out = Vec::new();
    for (k, _) in &pairs {
        let ks = ops.as_bytes(k);
        if pat.as_deref().is_none_or(|p| string_match_bytes(p, &ks)) {
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
    let pat = pattern.map(|p| ops.as_bytes(p));
    let pairs = ops.dict_pairs(dict)?;
    let mut out = Vec::new();
    for (_, v) in &pairs {
        let vs = ops.as_bytes(v);
        if pat.as_deref().is_none_or(|p| string_match_bytes(p, &vs)) {
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

/// `dict info dictionary` — the implementation-defined, human-readable hash
/// table statistics Tcl produces.
///
/// The canonical pair seam first validates the value and collapses duplicate
/// keys. The shared Tcl hash-table owner then supplies both layout and
/// formatting, so every runtime reports one result for the same dictionary.
pub fn info<O: ValueOps>(ops: &mut O, dict: &O::Value) -> Result<O::Value, CmdError> {
    let pairs = ops.dict_pairs(dict)?;
    let mut table = TclStringHashOrder::default();
    if let Some(bucket_count) = ops.dict_hash_bucket_count(dict)? {
        table.retain_bucket_count(bucket_count);
    }
    for (key, _) in &pairs {
        table.insert(&ops.native_string_bytes(key)?);
    }
    Ok(ops.new_string(table.statistics()))
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
    let pats: Vec<_> = patterns.iter().map(|p| ops.as_bytes(p)).collect();
    let pairs = ops.dict_pairs(dict)?;
    let mut out: Vec<O::Value> = Vec::new();
    for (k, v) in pairs {
        let target = ops.as_bytes(if by_key { &k } else { &v });
        if pats.iter().any(|p| string_match_bytes(p, &target)) {
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
            upsert_indexed(ops, &mut acc, &mut index, &k, v)?;
        }
    }
    Ok(ops.new_dict_checked(acc)?)
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
    let bucket_count = copied_hash_bucket_count(ops, &pairs)?;
    let mut index = index_of_pairs(ops, &pairs)?;
    for chunk in kv.as_chunks::<2>().0 {
        upsert_indexed(ops, &mut pairs, &mut index, &chunk[0], chunk[1].clone())?;
    }
    Ok(ops.new_dict_with_hash_bucket_count_checked(pairs, bucket_count)?)
}

/// `dict remove dictionary ?key ...?` — the dict without the given keys (a
/// missing key is not an error). The result is canonicalised.
pub fn remove<O: ValueOps>(
    ops: &mut O,
    dict: &O::Value,
    keys: &[O::Value],
) -> Result<O::Value, CmdError> {
    let pairs = ops.dict_pairs(dict)?;
    let bucket_count = copied_hash_bucket_count(ops, &pairs)?;
    let drop = keys
        .iter()
        .map(|key| ops.native_string_bytes(key).map(|bytes| bytes.to_vec()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut kept: Vec<(O::Value, O::Value)> = Vec::with_capacity(pairs.len());
    for (k, v) in pairs {
        if !drop.contains(&ops.native_string_bytes(&k)?.to_vec()) {
            kept.push((k, v));
        }
    }
    Ok(ops.new_dict_with_hash_bucket_count_checked(kept, bucket_count)?)
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
        let ks = ops.native_string_bytes(k)?.to_vec();
        let pairs = ops.dict_pairs(&cur)?;
        match lookup(ops, &pairs, &ks)? {
            Some(v) => cur = v,
            None => return Ok(default.clone()),
        }
    }
    Ok(cur)
}

/// `dict filter`'s type word, in C table order (`filters[]`, `tclDictObj.c`):
/// `Tcl_GetIndexFromObj(…, "filterType", 0)`, so `k`/`s`/`v` abbreviate and
/// the empty word — a prefix of all three — is `ambiguous filterType ""`.
static FILTER_TYPE_NAMES: [&str; 3] = ["key", "script", "value"];
static FILTER_TYPES: crate::prefix::OptionTable<'static> =
    crate::prefix::OptionTable::abbreviating("filterType", &FILTER_TYPE_NAMES);

/// Dispatch a pure `dict` subcommand. `rest` is the args after the subcommand;
/// `invoked` is the actual command prefix used by `info`'s arity diagnostic
/// (either `dict info` or a separately invoked/renamed implementation command).
///
/// Returns `None` for the variable-mutating subcommands, which the caller's
/// adapter owns.
pub fn dispatch_canon<O: ValueOps>(
    ops: &mut O,
    invoked: &[u8],
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
            Some((d, keys)) if !keys.is_empty() => Some(exists(ops, d, keys)),
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
        "info" => {
            if let [d] = rest {
                Some(info(ops, d))
            } else {
                let mut usage = invoked.to_vec();
                usage.extend_from_slice(b" dictionary");
                Some(Err(CmdError::wrong_args_bytes(&usage)))
            }
        }
        "merge" => Some(merge(ops, rest)),
        // `key`/`value` are pure (glob); `script` is Family-B → `None` so the
        // caller's adapter handles it. The filterType is validated *before* the
        // dict is parsed (tclsh: `dict filter {a b c} bogus` is a bad-filterType
        // error, not a bad-dict one).
        "filter" => match (rest.first(), rest.get(1)) {
            (Some(dict), Some(ft)) => match FILTER_TYPES.index_of_original(ops, ft) {
                Ok(0) => Some(filter(ops, dict, true, &rest[2..])),
                Ok(2) => Some(filter(ops, dict, false, &rest[2..])),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
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
