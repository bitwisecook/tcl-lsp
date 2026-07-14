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

//! `lsort` — list sort, shared over [`ValueOps`](tcl_syntax::value::ValueOps)
//! and a `-command` comparator callback.
//!
//! Implements the behaviour of C's `Tcl_LsortObjCmd` (`tclCmdIL.c`): the option set
//! (`-ascii`/`-dictionary`/`-integer`/`-real`/`-nocase`, `-increasing`/
//! `-decreasing`, `-unique`, `-indices`, `-index` *path*, `-stride`, `-command`),
//! the up-front numeric-key validation, mode-aware `-unique`, and the stride /
//! `-indices` result shapes. The non-command comparison is the shared
//! [`crate::sort`] core.
//!
//! `-command` evaluates a user Tcl proc per comparison (Family-B), so — like
//! `lseq`'s expression edge — it is split out: [`prepare`] does everything that
//! needs `ValueOps` (option parse, key extraction, and, for the non-command
//! modes, the whole sort+build), returning either the finished list or a
//! [`CommandJob`]; [`sort_command`] then runs the stable merge sort over the
//! adapter's comparator **without** `ValueOps` (so the interp the comparator
//! evaluates against isn't double-borrowed); [`build_command`] builds the result
//! with `ValueOps` afterwards. Three sequential calls, no borrow conflict.
//!
//! Semantics verified against tclsh 9.0.

use tcl_syntax::list::split_list;
use tcl_syntax::value::ValueOps;

use crate::index;
use crate::option_table::OptionTable;
use crate::sort::{self, SortMode};

// C's `switches[]` in `Tcl_LsortObjCmd` (`tclCmdIL.c`), resolved with
// abbreviations allowed (flags 0), so `lsort -uniq …`/`lsort -decr …` work
// like tclsh and a shared prefix (`-d`, `-in`, the empty word) is an
// ambiguous option.
const OPT_ASCII: usize = 0;
const OPT_COMMAND: usize = 1;
const OPT_DECREASING: usize = 2;
const OPT_DICTIONARY: usize = 3;
const OPT_INCREASING: usize = 4;
const OPT_INDEX: usize = 5;
const OPT_INDICES: usize = 6;
const OPT_INTEGER: usize = 7;
const OPT_NOCASE: usize = 8;
const OPT_REAL: usize = 9;
const OPT_STRIDE: usize = 10;
const OPT_UNIQUE: usize = 11;
const OPT_NAMES: [&str; 12] = [
    "-ascii",
    "-command",
    "-decreasing",
    "-dictionary",
    "-increasing",
    "-index",
    "-indices",
    "-integer",
    "-nocase",
    "-real",
    "-stride",
    "-unique",
];
const OPTIONS: OptionTable<'static> = OptionTable::abbreviating("option", &OPT_NAMES);

/// An `lsort` failure (message-only — `lsort` sets no `errorCode`).
pub struct LsortError {
    pub message: Vec<u8>,
}

impl LsortError {
    fn msg(m: impl Into<Vec<u8>>) -> Self {
        Self { message: m.into() }
    }
}

fn not_integer(s: &[u8]) -> LsortError {
    let mut m = b"expected integer but got \"".to_vec();
    m.extend_from_slice(s);
    m.push(b'"');
    LsortError::msg(m)
}

fn not_real(s: &[u8]) -> LsortError {
    let mut m = b"expected floating-point number but got \"".to_vec();
    m.extend_from_slice(s);
    m.push(b'"');
    LsortError::msg(m)
}

/// The outcome of [`prepare`].
pub enum Lsort<V> {
    /// A non-`-command` sort: the finished result list.
    Done(V),
    /// A `-command` sort: hand to [`sort_command`] then [`build_command`].
    Command(CommandJob<V>),
}

/// A prepared `-command` sort: the decorated items (group base + the key element
/// passed to the comparator) plus everything [`build_command`] needs.
pub struct CommandJob<V> {
    /// `(group base, key element)` per logical element, in input order.
    pub items: Vec<(usize, V)>,
    /// The flat source list elements.
    elems: Vec<V>,
    /// `-increasing` (vs `-decreasing`).
    pub increasing: bool,
    unique: bool,
    group: usize,
    indices_out: bool,
    /// The `-command` prefix value (the adapter's comparator evaluates it).
    pub cmd_prefix: V,
}

/// Parse + key-extract (and, for the non-command modes, fully sort+build).
///
/// # Errors
/// Option/index/coercion errors as a [`LsortError`].
// `too_many_lines`: option scan + key extraction + sort/build in one pass, mirrors
// Tcl_LsortObjCmd. The `-stride`/`-index` values are validated `>= 2` / `>= 0`
// before the `as usize` casts, so the truncation/sign-loss allows are safe here.
#[allow(clippy::too_many_lines)]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn prepare<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Result<Lsort<O::Value>, LsortError> {
    if args.is_empty() {
        return Err(LsortError::msg(
            "wrong # args: should be \"lsort ?-option value ...? list\"",
        ));
    }
    let mut mode = SortMode::Ascii;
    let mut command = false;
    let (mut nocase, mut increasing, mut unique, mut indices_out) = (false, true, false, false);
    let mut group = 1usize;
    let mut index_path: Vec<Vec<u8>> = Vec::new();
    let mut cmd_prefix: Option<O::Value> = None;

    let last = args.len() - 1; // the list
    let mut i = 0;
    while i < last {
        let opt = ops.as_bytes(&args[i]);
        match OPTIONS.index_of(&opt).map_err(LsortError::msg)? {
            OPT_ASCII => mode = SortMode::Ascii,
            OPT_DICTIONARY => mode = SortMode::Dictionary,
            OPT_INTEGER => mode = SortMode::Integer,
            OPT_REAL => mode = SortMode::Real,
            OPT_NOCASE => nocase = true,
            OPT_INCREASING => increasing = true,
            OPT_DECREASING => increasing = false,
            OPT_INDICES => indices_out = true,
            OPT_INDEX => {
                if i + 1 >= last {
                    return Err(needs_value(b"-index", b"list index"));
                }
                index_path = split_index(&ops.as_bytes(&args[i + 1]))?;
                i += 1;
            }
            OPT_STRIDE => {
                if i + 1 >= last {
                    return Err(needs_value(b"-stride", b"stride length"));
                }
                let sb = ops.as_bytes(&args[i + 1]);
                match sort::parse_wide(&sb) {
                    Some(w) if w >= 2 => group = w as usize,
                    Some(_) => return Err(LsortError::msg("stride length must be at least 2")),
                    None => return Err(not_integer(&sb)),
                }
                i += 1;
            }
            OPT_COMMAND => {
                if i + 1 >= last {
                    return Err(needs_value(b"-command", b"comparison command"));
                }
                command = true;
                cmd_prefix = Some(args[i + 1].clone());
                i += 1;
            }
            OPT_UNIQUE => unique = true,
            _ => unreachable!("the option table is closed"),
        }
        i += 1;
    }

    let elems = ops
        .list_elements(&args[last])
        .map_err(|e| LsortError::msg(e.message()))?;
    let len = elems.len();
    if len == 0 {
        return Ok(Lsort::Done(ops.new_list(Vec::new())));
    }

    // -stride groups the flat list; the leading -index value (default 0) picks
    // the key element within each group, the rest of the path applies within it.
    let mut group_offset = 0usize;
    let mut key_path: &[Vec<u8>] = &index_path;
    if group > 1 {
        if len % group != 0 {
            return Err(LsortError::msg(
                "list size must be a multiple of the stride length",
            ));
        }
        if !index_path.is_empty() {
            match index::resolve_opt(&String::from_utf8_lossy(&index_path[0]), group) {
                Some(g) if g >= 0 && (g as usize) < group => group_offset = g as usize,
                _ => {
                    return Err(LsortError::msg(
                        "when used with \"-stride\", the leading \"-index\" value must be within the group",
                    ));
                }
            }
            key_path = &index_path[1..];
        }
    }
    let logical = len / group;

    if command {
        // Extract the key *values* (the comparator receives them); the merge sort
        // happens in `sort_command` over the adapter's callback.
        let mut items: Vec<(usize, O::Value)> = Vec::with_capacity(logical);
        for l in 0..logical {
            let base = l * group;
            let key = index::drill(ops, &elems[base + group_offset], key_path)
                .map_err(LsortError::msg)?;
            items.push((base, key));
        }
        return Ok(Lsort::Command(CommandJob {
            items,
            elems,
            increasing,
            unique,
            group,
            indices_out,
            cmd_prefix: cmd_prefix.expect("set with -command"),
        }));
    }

    // Non-command: extract the key *bytes*, validate, sort, unique, build.
    let mut items: Vec<(usize, Vec<u8>)> = Vec::with_capacity(logical);
    for l in 0..logical {
        let base = l * group;
        let key =
            index::drill(ops, &elems[base + group_offset], key_path).map_err(LsortError::msg)?;
        items.push((base, ops.as_bytes(&key).to_vec()));
    }
    // Validate numeric keys up front (Tcl errors on the first non-number).
    if mode == SortMode::Integer
        && let Some((_, k)) = items.iter().find(|(_, k)| sort::parse_wide(k).is_none())
    {
        return Err(not_integer(k));
    } else if mode == SortMode::Real
        && let Some((_, k)) = items.iter().find(|(_, k)| sort::parse_real(k).is_none())
    {
        return Err(not_real(k));
    }
    items.sort_by(|a, b| {
        let ord = sort::key_compare(mode, nocase, &a.1, &b.1);
        if increasing { ord } else { ord.reverse() }
    });
    if unique {
        // Keep the *last* of each equal run (C's `MergeLists` keeps the right
        // element), so two inputs that compare equal but render differently
        // yield the later one — `lsort -nocase -unique {A a}` → `a`,
        // `lsort -integer -unique {01 1}` → `1`. `Vec::dedup_by` keeps the
        // first, which is the wrong end.
        let mut deduped: Vec<(usize, Vec<u8>)> = Vec::with_capacity(items.len());
        for it in items.drain(..) {
            if let Some(prev) = deduped.last()
                && sort::key_compare(mode, nocase, &prev.1, &it.1).is_eq()
            {
                deduped.pop();
            }
            deduped.push(it);
        }
        items = deduped;
    }
    let bases: Vec<usize> = items.iter().map(|(b, _)| *b).collect();
    Ok(Lsort::Done(build_result(
        ops,
        &bases,
        &elems,
        group,
        indices_out,
    )))
}

/// Run the `-command` stable merge sort over `cmp` (which returns the comparison
/// sign `<0`/`0`/`>0`), then mode-aware `-unique`. Pure + callback — no
/// `ValueOps`, so the interp the comparator evaluates against isn't borrowed
/// here.
///
/// # Errors
/// Propagates the comparator's error `E` (e.g. a non-integer result, or the
/// script itself failing).
pub fn sort_command<V: Clone, E>(
    job: &mut CommandJob<V>,
    mut cmp: impl FnMut(&V, &V) -> Result<i32, E>,
) -> Result<(), E> {
    let increasing = job.increasing;
    merge_sort(&mut job.items, increasing, &mut cmp)?;
    if job.unique {
        // `-unique` equivalence is the comparator's (not bytewise); the *last* of
        // an equal run is kept (C's `MergeLists` keeps the right element).
        let mut deduped: Vec<(usize, V)> = Vec::with_capacity(job.items.len());
        for it in job.items.drain(..) {
            if let Some(prev) = deduped.last()
                && cmp(&prev.1, &it.1)? == 0
            {
                deduped.pop();
            }
            deduped.push(it);
        }
        job.items = deduped;
    }
    Ok(())
}

/// Stable bottom-up-free recursive merge sort driven by `cmp` (reentrant — the
/// comparator runs arbitrary Tcl, so a plain `sort_by` won't do).
fn merge_sort<V: Clone, E>(
    a: &mut [(usize, V)],
    increasing: bool,
    cmp: &mut impl FnMut(&V, &V) -> Result<i32, E>,
) -> Result<(), E> {
    let n = a.len();
    if n <= 1 {
        return Ok(());
    }
    let mid = n / 2;
    let mut left = a[..mid].to_vec();
    let mut right = a[mid..].to_vec();
    merge_sort(&mut left, increasing, cmp)?;
    merge_sort(&mut right, increasing, cmp)?;
    let (mut li, mut ri, mut k) = (0usize, 0usize, 0usize);
    while li < left.len() && ri < right.len() {
        // Stable: take from left unless right strictly precedes it.
        let ord = cmp(&left[li].1, &right[ri].1)?;
        let take_left = if increasing { ord <= 0 } else { ord >= 0 };
        if take_left {
            a[k] = left[li].clone();
            li += 1;
        } else {
            a[k] = right[ri].clone();
            ri += 1;
        }
        k += 1;
    }
    while li < left.len() {
        a[k] = left[li].clone();
        li += 1;
        k += 1;
    }
    while ri < right.len() {
        a[k] = right[ri].clone();
        ri += 1;
        k += 1;
    }
    Ok(())
}

/// Build the result list after a `-command` sort.
#[must_use]
pub fn build_command<O: ValueOps>(ops: &mut O, job: &CommandJob<O::Value>) -> O::Value {
    let bases: Vec<usize> = job.items.iter().map(|(b, _)| *b).collect();
    build_result(ops, &bases, &job.elems, job.group, job.indices_out)
}

/// Build the output list from the sorted group `bases`: `-indices` emits each
/// element's position, `-stride` emits whole groups, otherwise the elements.
fn build_result<O: ValueOps>(
    ops: &mut O,
    bases: &[usize],
    elems: &[O::Value],
    group: usize,
    indices_out: bool,
) -> O::Value {
    let mut out: Vec<O::Value> = Vec::with_capacity(bases.len() * group);
    for &base in bases {
        for j in 0..group {
            if indices_out {
                out.push(ops.new_int(i64::try_from(base + j).unwrap_or(i64::MAX)));
            } else {
                out.push(elems[base + j].clone());
            }
        }
    }
    ops.new_list(out)
}

// option helpers

fn split_index(arg: &[u8]) -> Result<Vec<Vec<u8>>, LsortError> {
    let Ok(s) = core::str::from_utf8(arg) else {
        return Err(LsortError::msg("invalid index"));
    };
    split_list(s)
        .map(|elems| {
            elems
                .into_iter()
                .map(|e| e.into_owned().into_bytes())
                .collect()
        })
        .map_err(|e| LsortError::msg(e.message().to_string()))
}

/// `"-index" option must be followed by list index` — C names the canonical
/// option even when the user abbreviated it.
fn needs_value(opt: &[u8], what: &[u8]) -> LsortError {
    let mut m = b"\"".to_vec();
    m.extend_from_slice(opt);
    m.extend_from_slice(b"\" option must be followed by ");
    m.extend_from_slice(what);
    LsortError::msg(m)
}
