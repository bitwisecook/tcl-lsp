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

//! `array` ensemble cores — the *read-side* + `unset` of the `array` command,
//! shared once over [`VarStore`] (+ its new [`array_keys`](VarStore::array_keys)
//! enumeration rung) + [`ValueOps`].
//!
//! `array exists`/`size`/`names`/`get`/`unset` are value→value-ish reads over the
//! variable store, so they live here. `array set`'s element store fires a write
//! trace **per element** that must fail the command (C's `Tcl_ArraySetCmd`), and
//! the contract's [`VarStore::set_elem`] is storage-only (it discards the trace
//! outcome) — so, exactly like `incr`/`append`, the `set` store stays in each
//! adapter. `array default` (TIP 508) and `array for` (a Family-B iteration with
//! an eval body) likewise stay per-adapter; this core returns `None` for anything
//! it does not handle, so the caller falls back (and owns the unknown-subcommand
//! message, whose option list differs per runtime).
//!
//! `array unset a` with no pattern removes the **whole array** — not
//! iterate-and-unset over each element, which would leave an empty array
//! behind.
//!
//! Semantics verified against tclsh 9.0.

use tcl_runtime_api::{Frames, VarStore};
use tcl_syntax::glob::string_match;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// Dispatch an `array` subcommand handled by the shared core. `rest` is the
/// arguments after the subcommand (`rest[0]` is the array name). Returns `None`
/// for `set`/`default`/`for` and any unknown subcommand, letting the adapter
/// handle them.
pub fn dispatch<O, V>(ops: &mut O, sub: &str, rest: &[V]) -> Option<Result<V, CmdError>>
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
{
    match sub {
        "exists" => Some(match rest {
            [n] => {
                let here = Frames::current(ops);
                let name = ops.as_str(n);
                Ok(ops.new_bool(ops.array_keys(here, &name).is_some()))
            }
            _ => Err(CmdError::wrong_args("array exists arrayName")),
        }),
        "size" => Some(match rest {
            [n] => {
                let here = Frames::current(ops);
                let name = ops.as_str(n);
                let count = ops.array_keys(here, &name).map_or(0, |k| k.len());
                Ok(ops.new_int(i64::try_from(count).unwrap_or(i64::MAX)))
            }
            _ => Err(CmdError::wrong_args("array size arrayName")),
        }),
        "names" => Some(match rest {
            [n] => Ok(names(ops, n, None)),
            [n, p] => Ok(names(ops, n, Some(p))),
            _ => Err(CmdError::wrong_args("array names arrayName ?pattern?")),
        }),
        "get" => Some(match rest {
            [n] => Ok(get(ops, n, None)),
            [n, p] => Ok(get(ops, n, Some(p))),
            _ => Err(CmdError::wrong_args("array get arrayName ?pattern?")),
        }),
        "unset" => Some(match rest {
            [n] => Ok(unset(ops, n, None)),
            [n, p] => Ok(unset(ops, n, Some(p))),
            _ => Err(CmdError::wrong_args("array unset arrayName ?pattern?")),
        }),
        _ => None,
    }
}

/// `array names arrayName ?pattern?` — element names (glob-filtered).
fn names<O, V>(ops: &mut O, name: &V, pattern: Option<&V>) -> V
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
{
    let here = Frames::current(ops);
    let name = ops.as_str(name);
    let pattern = pattern.map(|p| ops.as_str(p).to_string());
    let keys = ops.array_keys(here, &name).unwrap_or_default();
    let items: Vec<V> = keys
        .iter()
        .filter(|k| pattern.as_deref().is_none_or(|p| string_match(p, k)))
        .map(|k| ops.new_str(k))
        .collect();
    ops.new_list(items)
}

/// `array get arrayName ?pattern?` — a flat `key value …` list (glob-filtered on
/// the key).
fn get<O, V>(ops: &mut O, name: &V, pattern: Option<&V>) -> V
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
{
    let here = Frames::current(ops);
    let name = ops.as_str(name);
    let pattern = pattern.map(|p| ops.as_str(p).to_string());
    let keys = ops.array_keys(here, &name).unwrap_or_default();
    let mut items: Vec<V> = Vec::with_capacity(keys.len() * 2);
    for k in &keys {
        if pattern.as_deref().is_some_and(|p| !string_match(p, k)) {
            continue;
        }
        if let Some(v) = ops.get_elem(here, &name, k) {
            items.push(ops.new_str(k));
            items.push(v);
        }
    }
    ops.new_list(items)
}

/// `array unset arrayName ?pattern?` — remove matching elements, or (no pattern)
/// the whole array.
fn unset<O, V>(ops: &mut O, name: &V, pattern: Option<&V>) -> V
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
{
    let here = Frames::current(ops);
    let name = ops.as_str(name);
    match pattern.map(|p| ops.as_str(p).to_string()) {
        // No pattern: remove the whole array variable (ignore "didn't exist").
        None => {
            ops.unset(here, &name);
        }
        Some(pattern) => {
            for k in ops.array_keys(here, &name).unwrap_or_default() {
                if string_match(&pattern, &k) {
                    ops.unset_elem(here, &name, &k);
                }
            }
        }
    }
    ops.empty()
}
