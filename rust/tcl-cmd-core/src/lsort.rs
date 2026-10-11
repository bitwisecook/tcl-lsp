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
//! the selected original numeric-key stages, mode-aware `-unique`, and the stride /
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
//! Primitive and reached schedules select the actual C release or Jim producer.

use crate::index::NativeIndexOps;
use crate::native_option_index::{OptionCommand, PreparedPath};
use tcl_syntax::list::split_list;
use tcl_syntax::value::ValueOps;

use crate::index;
use crate::prefix::OptionTable;
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
static OPT_NAMES: [&str; 12] = [
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
static OPTIONS: OptionTable<'static> = OptionTable::abbreviating("option", &OPT_NAMES);

/// An `lsort` failure retaining selected native guest error metadata.
pub struct LsortError {
    pub message: Vec<u8>,
    /// Complete selected guest error metadata, when supplied by a shared owner.
    pub command_error: Option<crate::CmdError>,
    /// Operational Unicode refusal; adapters must bypass guest completion.
    pub native_access_refusal: Option<tcl_syntax::raw_string::NativeValueAccessRefusal>,
}

impl LsortError {
    fn from_command(error: crate::CmdError) -> Self {
        let native_access_refusal = error.native_access_refusal();
        let message = error.message_bytes().to_vec();
        Self {
            message,
            command_error: Some(error),
            native_access_refusal,
        }
    }

    fn msg(m: impl Into<Vec<u8>>) -> Self {
        Self {
            message: m.into(),
            command_error: None,
            native_access_refusal: None,
        }
    }
}

fn not_integer(s: &[u8]) -> LsortError {
    let mut m = b"expected integer but got \"".to_vec();
    m.extend_from_slice(s);
    m.push(b'"');
    LsortError::msg(m)
}

/// The outcome of [`prepare`].
pub enum Lsort<V, K> {
    /// A non-`-command` sort: the finished result list.
    Done(V),
    /// Jim's actual zero/one-element input result, with its explicit borrowed role.
    Original(index::IndexResult<V>),
    /// A `-command` sort: hand to [`sort_command`] then [`build_command`].
    Command(CommandJob<V, K>),
}

/// A prepared `-command` sort: the decorated items (group base + the key element
/// passed to the comparator) plus everything [`build_command`] needs.
pub struct CommandJob<V, K> {
    /// `(group base, key element)` per logical element, in input order.
    pub items: Vec<(usize, index::IndexedKey<K>)>,
    /// The flat source list elements.
    elems: Vec<V>,
    /// `-increasing` (vs `-decreasing`).
    pub increasing: bool,
    unique: bool,
    group: usize,
    indices_out: bool,
    /// The `-command` prefix value (the adapter's comparator evaluates it).
    pub cmd_prefix: V,
    /// Exact C84 SortElement member references; key metadata clones add no holds.
    _temporary_members: Vec<K>,
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
pub fn prepare<O: NativeIndexOps>(
    ops: &mut O,
    args: &[O::Value],
) -> Result<Lsort<O::Value, O::IndexKeyOwner>, LsortError> {
    index::run_index_output(
        ops,
        |ops| {
            if args.is_empty() {
                return Err(LsortError::msg(
                    "wrong # args: should be \"lsort ?-option value ...? list\"",
                ));
            }
            let mut mode = SortMode::Ascii;
            let mut command = false;
            let (mut nocase, mut increasing, mut unique, mut indices_out) =
                (false, true, false, false);
            let mut group = 1usize;
            let mut index_path = PreparedPath::default();
            let mut last_index_argument = None;
            let mut jim_path: Option<Vec<O::Value>> = None;
            let mut cmd_prefix: Option<O::Value> = None;

            let last = args.len() - 1; // the list
            let mut i = 0;
            while i < last {
                let option = if ops.index_c_version() == Some(tcl_dialect::TclVersion::V8_4) {
                    const MAPPING: [usize; 9] = [
                        OPT_ASCII,
                        OPT_COMMAND,
                        OPT_DECREASING,
                        OPT_DICTIONARY,
                        OPT_INCREASING,
                        OPT_INDEX,
                        OPT_INTEGER,
                        OPT_REAL,
                        OPT_UNIQUE,
                    ];
                    static NAMES: [&str; 9] = [
                        "-ascii",
                        "-command",
                        "-decreasing",
                        "-dictionary",
                        "-increasing",
                        "-index",
                        "-integer",
                        "-real",
                        "-unique",
                    ];
                    static SELECTED: OptionTable<'static> =
                        OptionTable::abbreviating("option", &NAMES);
                    MAPPING[SELECTED
                        .index_of_original(ops, &args[i])
                        .map_err(LsortError::from_command)?]
                } else if ops.index_c_version() == Some(tcl_dialect::TclVersion::V8_5) {
                    const MAPPING: [usize; 11] = [
                        OPT_ASCII,
                        OPT_COMMAND,
                        OPT_DECREASING,
                        OPT_DICTIONARY,
                        OPT_INCREASING,
                        OPT_INDEX,
                        OPT_INDICES,
                        OPT_INTEGER,
                        OPT_NOCASE,
                        OPT_REAL,
                        OPT_UNIQUE,
                    ];
                    static NAMES: [&str; 11] = [
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
                        "-unique",
                    ];
                    static SELECTED: OptionTable<'static> =
                        OptionTable::abbreviating("option", &NAMES);
                    MAPPING[SELECTED
                        .index_of_original(ops, &args[i])
                        .map_err(LsortError::from_command)?]
                } else if ops
                    .index_syntax()
                    .is_some_and(|syntax| syntax.grammar == tcl_dialect::IndexGrammar::Jim)
                {
                    const MAPPING: [usize; 11] = [
                        OPT_ASCII,
                        OPT_NOCASE,
                        OPT_INCREASING,
                        OPT_DECREASING,
                        OPT_COMMAND,
                        OPT_INTEGER,
                        OPT_REAL,
                        OPT_INDEX,
                        OPT_UNIQUE,
                        OPT_STRIDE,
                        OPT_DICTIONARY,
                    ];
                    static NAMES: [&str; 11] = [
                        "-ascii",
                        "-nocase",
                        "-increasing",
                        "-decreasing",
                        "-command",
                        "-integer",
                        "-real",
                        "-index",
                        "-unique",
                        "-stride",
                        "-dictionary",
                    ];
                    static SELECTED: OptionTable<'static> =
                        OptionTable::abbreviating("option", &NAMES);
                    MAPPING[SELECTED
                        .index_of_original(ops, &args[i])
                        .map_err(LsortError::from_command)?]
                } else {
                    OPTIONS
                        .index_of_original(ops, &args[i])
                        .map_err(LsortError::from_command)?
                };
                match option {
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
                        if ops
                            .index_syntax()
                            .is_some_and(|syntax| syntax.grammar == tcl_dialect::IndexGrammar::Jim)
                        {
                            jim_path = ops
                                .original_jim_index_path(&args[i + 1])
                                .map_err(LsortError::from_command)?;
                            if jim_path.as_ref().is_some_and(Vec::is_empty) {
                                return Err(needs_value(b"-index", b"list index"));
                            }
                        }
                        if jim_path.is_none() {
                            index_path = match ops
                                .original_c_option_path(&args[i + 1], OptionCommand::Sort, true)
                                .map_err(LsortError::from_command)?
                            {
                                Some(path) => PreparedPath::original(path),
                                None => {
                                    PreparedPath::neutral(split_index(&ops.as_bytes(&args[i + 1]))?)
                                }
                            };
                        }
                        last_index_argument = Some(i + 1);
                        i += 1;
                    }
                    OPT_STRIDE => {
                        if i + 1 >= last {
                            return Err(needs_value(b"-stride", b"stride length"));
                        }
                        if let Some(integer) =
                            index::original_list_stride(ops, &args[i + 1], OptionCommand::Sort)
                                .map_err(LsortError::from_command)?
                        {
                            group = usize::try_from(integer).map_err(|_| {
                                LsortError::from_command(
                                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                        "original sort stride extent",
                                    )
                                    .into(),
                                )
                            })?;
                            i += 2;
                            continue;
                        }
                        // Explicit neutral lexical route has no original scalar/cache authority.
                        let sb = ops.as_bytes(&args[i + 1]);
                        match sort::parse_wide(&sb) {
                            Some(w) if w >= 2 => group = w as usize,
                            Some(_) => {
                                return Err(LsortError::msg("stride length must be at least 2"));
                            }
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

            // C86/C9 prescreen every option, then quietly prepare only the last
            // original path before input conversion. The same worker owns both.
            if ops
                .index_c_version()
                .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_6)
                && let Some(slot) = last_index_argument
            {
                let path = ops
                    .original_c_option_path(&args[slot], OptionCommand::Sort, false)
                    .map_err(LsortError::from_command)?
                    .ok_or_else(|| {
                        LsortError::from_command(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "original C sort option path issuer",
                            )
                            .into(),
                        )
                    })?;
                index_path = PreparedPath::original(path);
            }
            let elems = ops
                .index_list_elements(&args[last])
                .map_err(|e| LsortError::from_command(e.into()))?;
            let len = elems.len();
            let jim = ops.index_jim_sort_selected();
            if jim && len <= 1 {
                let borrowed = ops
                    .index_borrowed_key(&args[last])
                    .map_err(LsortError::from_command)?;
                return ops
                    .index_key_result(&borrowed, false, false)
                    .map(Lsort::Original)
                    .map_err(LsortError::from_command);
            }
            if len == 0 {
                return Ok(Lsort::Done(ops.new_list(Vec::new())));
            }

            // -stride groups the flat list; the leading -index value (default 0) picks
            // the key element within each group, the rest of the path applies within it.
            let mut group_offset = 0usize;
            let mut key_path = index_path.path();
            if group > 1 {
                if len % group != 0 {
                    return Err(LsortError::msg(
                        "list size must be a multiple of the stride length",
                    ));
                }
                if !key_path.is_empty() {
                    match key_path.decode(ops, 0, group - 1) {
                        Ok(g) if g >= 0 && (g as usize) < group => group_offset = g as usize,
                        Err(error) if error.native_execution_refusal().is_some() => {
                            return Err(LsortError::from_command(error));
                        }
                        _ => {
                            return Err(LsortError::msg(
                                "when used with \"-stride\", the leading \"-index\" value must be within the group",
                            ));
                        }
                    }
                    key_path = key_path.tail();
                }
            }
            let logical = len / group;

            let temporary_members = if ops.index_c_version() == Some(tcl_dialect::TclVersion::V8_4)
            {
                elems
                    .iter()
                    .map(|member| ops.index_c84_sort_member_hold(member))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(LsortError::from_command)?
            } else {
                Vec::new()
            };
            if command {
                // Extract the key *values* (the comparator receives them); the merge sort
                // happens in `sort_command` over the adapter's callback.
                let mut items: Vec<(usize, index::IndexedKey<O::IndexKeyOwner>)> =
                    Vec::with_capacity(logical);
                for l in 0..logical {
                    let base = l * group;
                    let key = match &jim_path {
                        Some(path) => {
                            index::drill_original_jim(ops, &elems[base + group_offset], path)
                        }
                        None => key_path.drill(ops, &elems[base + group_offset]),
                    }
                    .map_err(LsortError::from_command)?;
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
                    _temporary_members: temporary_members,
                }));
            }

            // C84 and Jim convert keys only inside the selected comparator.
            // Jim also resolves -index there; the local C qsort supplies actual
            // reached callback order, independently of Rust's sorting algorithms.
            let lazy = jim || ops.index_c_version() == Some(tcl_dialect::TclVersion::V8_4);
            let bases = if lazy {
                let mut order: Vec<usize> = (0..logical).collect();
                let mut compare = |ops: &mut O, left: usize, right: usize| {
                    let left = sort_key(
                        ops,
                        &elems[left * group + group_offset],
                        key_path,
                        jim_path.as_deref(),
                    )?;
                    let right = sort_key(
                        ops,
                        &elems[right * group + group_offset],
                        key_path,
                        jim_path.as_deref(),
                    )?;
                    let reached = compare_original_keys(
                        ops,
                        index::key_value::<O>(&left),
                        index::key_value::<O>(&right),
                        mode,
                        nocase,
                        jim,
                    );
                    if let Err(first) = &reached {
                        ops.retain_index_failure(first);
                    }
                    reached.map(|order| if increasing { order } else { order.reverse() })
                };
                if jim {
                    ops.index_native_sort(&mut order, &mut compare).map_err(
                        |error| match error {
                            index::NativeSortError::Operation(error)
                            | index::NativeSortError::Comparison(error) => {
                                LsortError::from_command(error)
                            }
                        },
                    )?;
                    if unique {
                        let mut kept = Vec::with_capacity(order.len());
                        for position in order {
                            if let Some(&prior) = kept.last() {
                                if compare(ops, prior, position)
                                    .map_err(LsortError::from_command)?
                                    .is_eq()
                                {
                                    kept.pop();
                                }
                            }
                            kept.push(position);
                        }
                        order = kept;
                    }
                } else {
                    // Native C84 MergeSort merges binary buckets and marks equal
                    // left nodes during that same comparator pass, without a later
                    // uniqueness getter pass. Metadata copies grant no native refs.
                    order = c84_merge_order(order.len(), unique, |left, right| {
                        compare(ops, left, right)
                    })
                    .map_err(LsortError::from_command)?;
                }
                order
                    .into_iter()
                    .map(|position| position * group)
                    .collect::<Vec<_>>()
            } else {
                // C85+ prepares each original numeric key in input order; a typed
                // key reaches its primitive without a preceding String projection.
                let mut items = Vec::with_capacity(logical);
                for position in 0..logical {
                    let base = position * group;
                    let key = sort_key(
                        ops,
                        &elems[base + group_offset],
                        key_path,
                        jim_path.as_deref(),
                    )
                    .map_err(LsortError::from_command)?;
                    let prepared = prepare_key(ops, index::key_value::<O>(&key), mode)
                        .map_err(LsortError::from_command)?;
                    items.push((base, prepared));
                }
                items.sort_by(|left, right| {
                    let order = prepared_compare(&left.1, &right.1, mode, nocase);
                    if increasing { order } else { order.reverse() }
                });
                if unique {
                    let mut kept: Vec<(usize, PreparedKey)> = Vec::with_capacity(items.len());
                    for item in items {
                        if let Some(prior) = kept.last() {
                            if prepared_compare(&prior.1, &item.1, mode, nocase).is_eq() {
                                kept.pop();
                            }
                        }
                        kept.push(item);
                    }
                    items = kept;
                }
                items.into_iter().map(|(base, _)| base).collect::<Vec<_>>()
            };
            Ok(Lsort::Done(build_result(
                ops,
                &bases,
                &elems,
                group,
                indices_out,
            )))
        },
        |ops, output| match output {
            Lsort::Done(created) => ops.discard_owned_index_result(created),
            Lsort::Original(result) => match result {
                index::IndexResult::Borrowed(value) => drop(value),
                index::IndexResult::Created(value)
                | index::IndexResult::Detached(value)
                | index::IndexResult::ReturnedOwnedTransport(value) => {
                    ops.discard_owned_index_result(value)
                }
            },
            // Key guards retain constructor cleanup independently of borrowed
            // members; merge copies add no native object references.
            Lsort::Command(job) => drop(job),
        },
        LsortError::from_command,
    )
}

enum PreparedKey {
    Bytes(Vec<u8>),
    Number(crate::native_list_numeric::NumericKey),
}
fn prepare_key<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::Value,
    mode: SortMode,
) -> Result<PreparedKey, crate::CmdError> {
    let kind = match mode {
        SortMode::Integer => Some(index::ListNumericKind::Integer),
        SortMode::Real => Some(index::ListNumericKind::Real),
        _ => None,
    };
    if let Some(kind) = kind {
        return crate::native_list_numeric::number(ops, original, OptionCommand::Sort, kind)
            .map(PreparedKey::Number);
    }
    index::checked_operand_bytes(ops, original).map(|bytes| PreparedKey::Bytes(bytes.to_vec()))
}
fn prepared_compare(
    left: &PreparedKey,
    right: &PreparedKey,
    mode: SortMode,
    nocase: bool,
) -> core::cmp::Ordering {
    match (left, right) {
        (PreparedKey::Bytes(left), PreparedKey::Bytes(right)) => {
            sort::key_compare(mode, nocase, left, right)
        }
        (PreparedKey::Number(left), PreparedKey::Number(right)) => {
            crate::native_list_numeric::sort_order(*left, *right, false)
                .expect("C/neutral comparator uses no signed subtraction")
        }
        _ => unreachable!("one selected sort mode"),
    }
}
fn sort_key<O: NativeIndexOps>(
    ops: &mut O,
    value: &O::Value,
    path: crate::native_option_index::Path<'_>,
    jim_path: Option<&[O::Value]>,
) -> Result<index::IndexedKey<O::IndexKeyOwner>, crate::CmdError> {
    let reached = match jim_path {
        Some(path) => index::drill_original_jim(ops, value, path),
        None => path.drill(ops, value),
    };
    if let Err(first) = &reached {
        ops.retain_index_failure(first);
    }
    reached
}
fn compare_original_keys<O: NativeIndexOps>(
    ops: &mut O,
    left: &O::Value,
    right: &O::Value,
    mode: SortMode,
    nocase: bool,
    jim: bool,
) -> Result<core::cmp::Ordering, crate::CmdError> {
    let left = prepare_key(ops, left, mode)?;
    let right = prepare_key(ops, right, mode)?;
    match (left, right) {
        (PreparedKey::Number(left), PreparedKey::Number(right)) => {
            crate::native_list_numeric::sort_order(left, right, jim)
        }
        (left, right) => Ok(prepared_compare(&left, &right, mode, nocase)),
    }
}

// C84 NUM_LISTS=30 and source MergeLists left<=right ordering. This is a
// software schedule joined to the actual pinned source, not a measured native
// callback chronology. Equal-left marks belong to that comparison, not a rerun.
fn c84_merge_order<E>(
    length: usize,
    unique: bool,
    mut compare: impl FnMut(usize, usize) -> Result<core::cmp::Ordering, E>,
) -> Result<Vec<usize>, E> {
    fn merge<E>(
        left: Vec<usize>,
        right: Vec<usize>,
        duplicate: &mut [bool],
        compare: &mut impl FnMut(usize, usize) -> Result<core::cmp::Ordering, E>,
    ) -> Result<Vec<usize>, E> {
        let mut out = Vec::with_capacity(left.len() + right.len());
        let (mut l, mut r) = (0, 0);
        while l < left.len() && r < right.len() {
            let order = compare(left[l], right[r])?;
            if order.is_gt() {
                out.push(right[r]);
                r += 1;
            } else {
                if order.is_eq() {
                    duplicate[left[l]] = true;
                }
                out.push(left[l]);
                l += 1;
            }
        }
        out.extend_from_slice(&left[l..]);
        out.extend_from_slice(&right[r..]);
        Ok(out)
    }
    let mut buckets: [Option<Vec<usize>>; 30] = core::array::from_fn(|_| None);
    let mut duplicate = vec![false; length];
    for position in 0..length {
        let mut node = vec![position];
        let mut level = 0;
        while level < 30 {
            let Some(left) = buckets[level].take() else {
                break;
            };
            node = merge(left, node, &mut duplicate, &mut compare)?;
            level += 1;
        }
        buckets[level.min(29)] = Some(node);
    }
    let mut ordered = Vec::new();
    for bucket in buckets {
        if let Some(left) = bucket {
            ordered = merge(left, ordered, &mut duplicate, &mut compare)?;
        }
    }
    if unique {
        ordered.retain(|&position| !duplicate[position]);
    }
    Ok(ordered)
}

/// Run the `-command` stable merge sort over `cmp` (which returns the comparison
/// sign `<0`/`0`/`>0`), then mode-aware `-unique`. Pure + callback — no
/// `ValueOps`, so the interp the comparator evaluates against isn't borrowed
/// here.
///
/// # Errors
/// Propagates the comparator's error `E` (e.g. a non-integer result, or the
/// script itself failing).
pub fn sort_command<V, K: Clone, E>(
    job: &mut CommandJob<V, K>,
    mut cmp: impl FnMut(&index::IndexedKey<K>, &index::IndexedKey<K>) -> Result<i32, E>,
) -> Result<(), E> {
    let increasing = job.increasing;
    merge_sort(&mut job.items, increasing, &mut cmp)?;
    if job.unique {
        // `-unique` equivalence is the comparator's (not bytewise); the *last* of
        // an equal run is kept (C's `MergeLists` keeps the right element).
        let mut deduped: Vec<(usize, index::IndexedKey<K>)> = Vec::with_capacity(job.items.len());
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

/// Stable recursive merge sort driven by `cmp`, which is reentrant: the
/// comparator runs arbitrary Tcl, so a plain `sort_by` won't do.
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
pub fn build_command<O: NativeIndexOps>(
    ops: &mut O,
    job: &CommandJob<O::Value, O::IndexKeyOwner>,
) -> O::Value {
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
