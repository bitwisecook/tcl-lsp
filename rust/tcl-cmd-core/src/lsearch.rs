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

//! `lsearch` — list search, shared over [`ValueOps`](tcl_syntax::value::ValueOps)
//! and the [`RegexEngine`](crate::regex::RegexEngine) provider.
//!
//! Implements the behaviour of C's `Tcl_LsearchObjCmd` (`tclCmdIL.c`): every option
//! (`-exact`/`-glob`/`-regexp`/`-sorted`/`-bisect`, `-all`/`-inline`/`-not`,
//! `-ascii`/`-dictionary`/`-integer`/`-real`/`-nocase`, `-increasing`/
//! `-decreasing`, `-start`, `-stride`, `-index`, `-subindices`), the sorted
//! binary search, and the stride / sub-index result shapes. The comparison
//! primitives are the shared [`crate::sort`] core; `-regexp` runs through the
//! caller's engine (the real ARE engine on the WASM runtime, the `regex` crate on
//! the VM). `lsearch` never writes a variable, so the whole command is a pure
//! value→value function here — the adapter only maps the result/error onto its
//! protocol.
//!
//! Semantics follow tclsh 9.0.

// The sorted binary search and stride/index arithmetic mirror C's `isize`/`usize`
// index math (each cast is range-checked by the surrounding logic — list lengths
// and resolved indices are bounded by the list); the `(lower+upper)/2` truncation
// is deliberate (the exact C midpoint), not `isize::midpoint`'s flooring.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::struct_excessive_bools,
    clippy::many_single_char_names,
    clippy::manual_midpoint
)]

use core::cmp::Ordering;

use crate::index::NativeIndexOps;
use crate::native_option_index::{OptionCommand, Path, PreparedPath};
use tcl_dialect::TclVersion;
#[cfg(test)]
use tcl_syntax::value::ValueOps;

use tcl_syntax::list::split_list;

use crate::index;
use crate::prefix::OptionTable;
use crate::regex::{RegexEngine, RegexFlags, decode_utf8};
use crate::sort::{self, SortMode};

// C's `options[]` in `Tcl_LsearchObjCmd` (`tclCmdIL.c`), resolved with
// abbreviations allowed (flags 0), so `lsearch -exa …`/`lsearch -inl …` work
// like tclsh and a shared prefix (`-in`, `-no`, the empty word) is an
// ambiguous option.
const OPT_ALL: usize = 0;
const OPT_ASCII: usize = 1;
const OPT_BISECT: usize = 2;
const OPT_DECREASING: usize = 3;
const OPT_DICTIONARY: usize = 4;
const OPT_EXACT: usize = 5;
const OPT_GLOB: usize = 6;
const OPT_INCREASING: usize = 7;
const OPT_INDEX: usize = 8;
const OPT_INLINE: usize = 9;
const OPT_INTEGER: usize = 10;
const OPT_NOCASE: usize = 11;
const OPT_NOT: usize = 12;
const OPT_REAL: usize = 13;
const OPT_REGEXP: usize = 14;
const OPT_SORTED: usize = 15;
const OPT_START: usize = 16;
const OPT_STRIDE: usize = 17;
const OPT_SUBINDICES: usize = 18;
static OPT_NAMES: [&str; 19] = [
    "-all",
    "-ascii",
    "-bisect",
    "-decreasing",
    "-dictionary",
    "-exact",
    "-glob",
    "-increasing",
    "-index",
    "-inline",
    "-integer",
    "-nocase",
    "-not",
    "-real",
    "-regexp",
    "-sorted",
    "-start",
    "-stride",
    "-subindices",
];
static OPTIONS: OptionTable<'static> = OptionTable::abbreviating("option", &OPT_NAMES);

/// The match mode (`-exact`/`-glob`/`-regexp`/`-sorted`).
#[derive(Clone, Copy, PartialEq)]
enum SearchMode {
    Exact,
    Glob,
    Regexp,
    Sorted,
}

/// An `lsearch` failure: the message bytes plus an optional Tcl `errorCode` (the
/// WASM runtime sets it; the VM ignores it).
pub struct LsearchError {
    pub message: Vec<u8>,
    /// Complete selected guest error metadata, when supplied by a shared owner.
    pub command_error: Option<crate::CmdError>,
    /// Operational Unicode refusal; adapters must bypass guest completion.
    pub native_access_refusal: Option<tcl_syntax::raw_string::NativeValueAccessRefusal>,
    pub code: Option<&'static [u8]>,
}

impl LsearchError {
    fn from_command(error: crate::CmdError) -> Self {
        let native_access_refusal = error.native_access_refusal();
        let message = error.message_bytes().to_vec();
        Self {
            message,
            command_error: Some(error),
            native_access_refusal,
            code: None,
        }
    }

    fn msg(m: impl Into<Vec<u8>>) -> Self {
        Self {
            message: m.into(),
            command_error: None,
            native_access_refusal: None,
            code: None,
        }
    }
    fn coded(m: impl Into<Vec<u8>>, code: &'static [u8]) -> Self {
        Self {
            message: m.into(),
            command_error: None,
            native_access_refusal: None,
            code: Some(code),
        }
    }
}

fn bad_index(spec: &[u8]) -> LsearchError {
    let mut m = b"bad index \"".to_vec();
    m.extend_from_slice(spec);
    m.extend_from_slice(b"\": must be integer?[+-]integer? or end?[+-]integer?");
    LsearchError::msg(m)
}

fn not_integer(s: &[u8]) -> LsearchError {
    let mut m = b"expected integer but got \"".to_vec();
    m.extend_from_slice(s);
    m.push(b'"');
    LsearchError::msg(m)
}

/// The parsed `lsearch` options.
struct Opts {
    mode: SearchMode,
    dtype: SortMode,
    increasing: bool,
    all: bool,
    inline: bool,
    not: bool,
    nocase: bool,
    bisect: bool,
    subindices: bool,
    group: usize,
    index_path: PreparedPath,
    start_argument: Option<usize>,
}

/// Drive `lsearch` over `args` (the command's arguments **without** the command
/// name: `?option ...? list pattern`).
///
/// # Errors
/// Option/index/coercion errors as a [`LsearchError`] (message + optional code).
pub fn lsearch<O: NativeIndexOps, E: RegexEngine>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
) -> Result<O::Value, LsearchError> {
    lsearch_with(
        ops,
        args,
        version,
        |ops, pattern, flags| {
            let bytes =
                index::checked_operand_bytes(ops, pattern).map_err(LsearchError::from_command)?;
            E::compile(&bytes, flags).map_err(|detail| {
                let mut message = version.regex_compile_error_prefix().as_bytes().to_vec();
                message.extend_from_slice(&detail);
                LsearchError::msg(message)
            })
        },
        |ops, compiled, subject| {
            let bytes =
                index::checked_operand_bytes(ops, subject).map_err(LsearchError::from_command)?;
            let (units, _) = decode_utf8(&bytes);
            crate::regex::completed_matches(E::exec(compiled, &units, 0, false))
                .map(|matches| matches.is_some())
                .map_err(|error| LsearchError::from_command(error.into_cmd_error()))
        },
    )
}

/// Native original-object pattern cache and subject units for regexp search.
pub fn lsearch_original<O: crate::regex::NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
) -> Result<O::Value, LsearchError> {
    lsearch_original_with_jim::<O, E, crate::CmdError>(ops, args, version, |_, _, _, _| {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original Jim regexp command callback",
        )
        .into())
    })
    .map_err(|error| match error {
        crate::regex::OriginalRegexConsumerError::Command(error) => error,
        crate::regex::OriginalRegexConsumerError::Callback(error) => {
            LsearchError::from_command(error)
        }
    })
}

/// Jim delegates every reached comparison to the real original regexp command.
/// Its callback completion is preserved separately from list/options failures.
pub fn lsearch_original_with_jim<O: crate::regex::NativeRegexObjects<E>, E: RegexEngine, Err>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
    mut invoke: impl FnMut(&mut O, &O::Value, &O::Value, bool) -> Result<bool, Err>,
) -> Result<O::Value, crate::regex::OriginalRegexConsumerError<Err, LsearchError>> {
    enum Pattern<R> {
        Prepared(R),
        Jim(bool),
    }
    let mut callback_failure = None;
    let result = lsearch_with(
        ops,
        args,
        version,
        |ops, pattern, flags| {
            if ops
                .jim_regex_recipe()
                .map_err(|error| LsearchError::from_command(error.into()))?
                .is_some()
            {
                return Ok(Pattern::Jim(flags.nocase));
            }
            crate::regex::prepare_search_pattern_original::<O, E>(ops, pattern, flags, version)
                .map(Pattern::Prepared)
                .map_err(|error| LsearchError::from_command(error.into_cmd_error()))
        },
        |ops, compiled, subject| match compiled {
            Pattern::Prepared(compiled) => {
                crate::regex::execute_pattern_original::<O, E>(ops, compiled, subject, 0, false)
                    .map(|matched| matched.is_some())
                    .map_err(|error| LsearchError::from_command(error.into_cmd_error()))
            }
            Pattern::Jim(nocase) => match invoke(
                ops,
                args.last().expect("validated original lsearch pattern"),
                subject,
                *nocase,
            ) {
                Ok(matched) => Ok(matched),
                Err(error) => {
                    callback_failure = Some(error);
                    Err(LsearchError::from_command(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original Jim regexp callback completion",
                        )
                        .into(),
                    ))
                }
            },
        },
    );
    if let Some(error) = callback_failure {
        return Err(crate::regex::OriginalRegexConsumerError::Callback(error));
    }
    result.map_err(crate::regex::OriginalRegexConsumerError::Command)
}

impl Opts {
    fn new() -> Self {
        Self {
            mode: SearchMode::Glob,
            dtype: SortMode::Ascii,
            increasing: true,
            all: false,
            inline: false,
            not: false,
            nocase: false,
            bisect: false,
            subindices: false,
            group: 1,
            index_path: PreparedPath::default(),
            start_argument: None,
        }
    }
}

/// C8.4's `lsearch` option table, mapped onto the shared option ordinals.
const C84_MAPPING: [usize; 14] = [
    OPT_ALL,
    OPT_ASCII,
    OPT_DECREASING,
    OPT_DICTIONARY,
    OPT_EXACT,
    OPT_GLOB,
    OPT_INCREASING,
    OPT_INLINE,
    OPT_INTEGER,
    OPT_NOT,
    OPT_REAL,
    OPT_REGEXP,
    OPT_SORTED,
    OPT_START,
];
static C84_NAMES: [&str; 14] = [
    "-all",
    "-ascii",
    "-decreasing",
    "-dictionary",
    "-exact",
    "-glob",
    "-increasing",
    "-inline",
    "-integer",
    "-not",
    "-real",
    "-regexp",
    "-sorted",
    "-start",
];
static C84: OptionTable<'static> = OptionTable::abbreviating("option", &C84_NAMES);
/// C8.5's `lsearch` option table, mapped onto the shared option ordinals.
const C85_MAPPING: [usize; 17] = [
    OPT_ALL,
    OPT_ASCII,
    OPT_DECREASING,
    OPT_DICTIONARY,
    OPT_EXACT,
    OPT_GLOB,
    OPT_INCREASING,
    OPT_INDEX,
    OPT_INLINE,
    OPT_INTEGER,
    OPT_NOCASE,
    OPT_NOT,
    OPT_REAL,
    OPT_REGEXP,
    OPT_SORTED,
    OPT_START,
    OPT_SUBINDICES,
];
static C85_NAMES: [&str; 17] = [
    "-all",
    "-ascii",
    "-decreasing",
    "-dictionary",
    "-exact",
    "-glob",
    "-increasing",
    "-index",
    "-inline",
    "-integer",
    "-nocase",
    "-not",
    "-real",
    "-regexp",
    "-sorted",
    "-start",
    "-subindices",
];
static C85: OptionTable<'static> = OptionTable::abbreviating("option", &C85_NAMES);
/// C8.6's `lsearch` option table, mapped onto the shared option ordinals.
const C86_MAPPING: [usize; 18] = [
    OPT_ALL,
    OPT_ASCII,
    OPT_BISECT,
    OPT_DECREASING,
    OPT_DICTIONARY,
    OPT_EXACT,
    OPT_GLOB,
    OPT_INCREASING,
    OPT_INDEX,
    OPT_INLINE,
    OPT_INTEGER,
    OPT_NOCASE,
    OPT_NOT,
    OPT_REAL,
    OPT_REGEXP,
    OPT_SORTED,
    OPT_START,
    OPT_SUBINDICES,
];
static C86_NAMES: [&str; 18] = [
    "-all",
    "-ascii",
    "-bisect",
    "-decreasing",
    "-dictionary",
    "-exact",
    "-glob",
    "-increasing",
    "-index",
    "-inline",
    "-integer",
    "-nocase",
    "-not",
    "-real",
    "-regexp",
    "-sorted",
    "-start",
    "-subindices",
];
static C86: OptionTable<'static> = OptionTable::abbreviating("option", &C86_NAMES);

/// Select one option's shared ordinal through the selected release's table.
fn select_option<O: NativeIndexOps>(ops: &mut O, arg: &O::Value) -> Result<usize, LsearchError> {
    let (table, mapping): (&OptionTable<'static>, &[usize]) = match ops.index_c_version() {
        Some(TclVersion::V8_4) => (&C84, &C84_MAPPING),
        Some(TclVersion::V8_5) => (&C85, &C85_MAPPING),
        Some(TclVersion::V8_6) => (&C86, &C86_MAPPING),
        _ => {
            return OPTIONS
                .index_of_original(ops, arg)
                .map_err(LsearchError::from_command);
        }
    };
    let selected = table
        .index_of_original(ops, arg)
        .map_err(LsearchError::from_command)?;
    Ok(mapping[selected])
}

/// The `-stride` group length, through the original primitive when present.
fn parse_stride<O: NativeIndexOps>(ops: &mut O, arg: &O::Value) -> Result<usize, LsearchError> {
    if let Some(integer) = index::original_list_stride(ops, arg, OptionCommand::Search)
        .map_err(LsearchError::from_command)?
    {
        return usize::try_from(integer).map_err(|_| {
            LsearchError::from_command(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original search stride extent",
                )
                .into(),
            )
        });
    }
    // Explicit neutral lexical route grants no native primitive/cache.
    let sb = ops.as_bytes(arg);
    match sort::parse_wide(&sb) {
        Some(w) if w >= 1 => Ok(usize::try_from(w).unwrap_or(usize::MAX)),
        Some(_) => Err(LsearchError::msg("stride length must be at least 1")),
        None => Err(not_integer(&sb)),
    }
}

/// The `-index` path, through the original C option path when present.
fn parse_index_path<O: NativeIndexOps>(
    ops: &mut O,
    arg: &O::Value,
) -> Result<PreparedPath, LsearchError> {
    if let Some(path) = ops
        .original_c_option_path(arg, OptionCommand::Search, true)
        .map_err(LsearchError::from_command)?
    {
        return Ok(PreparedPath::original(path));
    }
    let path = split_index(&ops.as_bytes(arg))?;
    validate_index_path(ops, &path)?;
    Ok(PreparedPath::neutral(path))
}

fn parse_options<O: NativeIndexOps>(ops: &mut O, args: &[O::Value]) -> Result<Opts, LsearchError> {
    let n = args.len();
    let mut o = Opts::new();
    // Options occupy everything before the trailing `list pattern`.
    let last_opt = n - 2; // first non-option index
    let mut i = 0;
    while i < last_opt {
        let option = select_option(ops, &args[i])?;
        match option {
            OPT_ALL => o.all = true,
            OPT_ASCII => o.dtype = SortMode::Ascii,
            OPT_DICTIONARY => o.dtype = SortMode::Dictionary,
            OPT_INTEGER => o.dtype = SortMode::Integer,
            OPT_REAL => o.dtype = SortMode::Real,
            OPT_BISECT => {
                o.mode = SearchMode::Sorted;
                o.bisect = true;
            }
            OPT_DECREASING => o.increasing = false,
            OPT_INCREASING => o.increasing = true,
            OPT_EXACT => o.mode = SearchMode::Exact,
            OPT_GLOB => o.mode = SearchMode::Glob,
            OPT_REGEXP => o.mode = SearchMode::Regexp,
            OPT_SORTED => o.mode = SearchMode::Sorted,
            OPT_INLINE => o.inline = true,
            OPT_NOCASE => o.nocase = true,
            OPT_NOT => o.not = true,
            OPT_SUBINDICES => o.subindices = true,
            OPT_START => {
                if i + 1 >= last_opt {
                    return Err(LsearchError::msg("missing starting index"));
                }
                o.start_argument = Some(i + 1);
                i += 1;
            }
            OPT_STRIDE => {
                if i + 1 >= last_opt {
                    return Err(LsearchError::msg(
                        "\"-stride\" option must be followed by stride length",
                    ));
                }
                o.group = parse_stride(ops, &args[i + 1])?;
                i += 1;
            }
            OPT_INDEX => {
                if i + 1 >= last_opt {
                    return Err(LsearchError::msg(
                        "\"-index\" option must be followed by list index",
                    ));
                }
                o.index_path = parse_index_path(ops, &args[i + 1])?;
                i += 1;
            }
            _ => unreachable!("the option table is closed"),
        }
        i += 1;
    }
    // `-subindices` only makes sense alongside `-index` (C's BAD_OPTION_MIX).
    if o.subindices && o.index_path.path().is_empty() {
        return Err(LsearchError::coded(
            "-subindices cannot be used without -index option",
            b"TCL OPERATION LSEARCH BAD_OPTION_MIX",
        ));
    }

    Ok(o)
}

struct SearchInput<'a, V> {
    elems: &'a [V],
    options: &'a Opts,
    start: usize,
    group_offset: usize,
    key_path: Path<'a>,
    pattern: Option<&'a [u8]>,
    number: Option<crate::native_list_numeric::NumericKey>,
}

/// Split `-stride` + `-index`: the leading index value picks the key element
/// within each group; the rest of the path applies inside it.
fn stride_key_path<'o, O: NativeIndexOps>(
    ops: &mut O,
    o: &'o Opts,
) -> Result<(usize, Path<'o>), LsearchError> {
    let key_path = o.index_path.path();
    if o.group <= 1 || key_path.is_empty() {
        return Ok((0, key_path));
    }
    match key_path.decode(ops, 0, o.group - 1).map(usize::try_from) {
        Ok(Ok(g)) if g < o.group => Ok((g, key_path.tail())),
        Err(error) if error.native_execution_refusal().is_some() => {
            Err(LsearchError::from_command(error))
        }
        _ => Err(LsearchError::msg(
            "when used with \"-stride\", the leading \"-index\" value must be within the group",
        )),
    }
}

/// The original numeric pattern key of an `-exact`/`-sorted` numeric search.
fn search_number<O: NativeIndexOps>(
    ops: &mut O,
    o: &Opts,
    pattern: &O::Value,
) -> Result<Option<crate::native_list_numeric::NumericKey>, LsearchError> {
    let numeric_kind = if matches!(o.mode, SearchMode::Exact | SearchMode::Sorted) {
        match o.dtype {
            SortMode::Integer => Some(index::ListNumericKind::Integer),
            SortMode::Real => Some(index::ListNumericKind::Real),
            _ => None,
        }
    } else {
        None
    };
    numeric_kind
        .map(|kind| crate::native_list_numeric::number(ops, pattern, OptionCommand::Search, kind))
        .transpose()
        .map_err(LsearchError::from_command)
}

fn lsearch_with<O: NativeIndexOps, R>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
    mut prepare: impl FnMut(&mut O, &O::Value, RegexFlags) -> Result<R, LsearchError>,
    mut regex_matches: impl FnMut(&mut O, &mut R, &O::Value) -> Result<bool, LsearchError>,
) -> Result<O::Value, LsearchError> {
    let original = ops
        .enter_index_operation()
        .map_err(LsearchError::from_command)?;
    let reached = (|ops: &mut O| {
        let n = args.len();
        if n < 2 {
            return Err(LsearchError::msg(
                "wrong # args: should be \"lsearch ?-option value ...? list pattern\"",
            ));
        }
        let o = parse_options(ops, args)?;

        // Pre-compile a -regexp pattern once (over the caller's engine).
        let mut re = if o.mode == SearchMode::Regexp {
            let flags = RegexFlags {
                nocase: o.nocase,
                ..RegexFlags::for_release(version)
            };
            Some(prepare(ops, &args[n - 1], flags)?)
        } else {
            None
        };

        let elems = ops
            .index_list_elements(&args[n - 2])
            .map_err(LsearchError::from_command)?;
        let listc = elems.len();
        if o.group > 1 && listc % o.group != 0 {
            return Err(LsearchError::msg(
                "list size must be a multiple of the stride length",
            ));
        }

        let (group_offset, key_path) = stride_key_path(ops, &o)?;

        // Resolve -start (relative to listc, then clamped to a group boundary).
        let mut start = 0usize;
        if let Some(slot) = o.start_argument {
            let s = index::resolve_value(ops, &args[slot], listc)
                .map_err(LsearchError::from_command)?;
            let s = usize::try_from(s.max(0)).unwrap_or(0);
            if s >= listc {
                return Ok(index::IndexResult::Created(empty_result(
                    ops, &original, &o,
                )?));
            }
            start = s - (s % o.group);
        }

        let number = search_number(ops, &o, &args[n - 1])?;
        // Original numeric conversion can shimmer the input container itself.
        // The selected C command restores its actual List view after the pattern.
        let elems = if number.is_some() && ops.index_c_version().is_some() {
            ops.index_list_elements(&args[n - 2])
                .map_err(LsearchError::from_command)?
        } else {
            elems
        };
        let pattern = if o.mode != SearchMode::Regexp && number.is_none() {
            Some(
                index::checked_operand_bytes(ops, &args[n - 1])
                    .map_err(LsearchError::from_command)?
                    .to_vec(),
            )
        } else {
            None
        };
        let input = SearchInput {
            elems: &elems,
            options: &o,
            start,
            group_offset,
            key_path,
            pattern: pattern.as_deref(),
            number,
        };
        if o.mode == SearchMode::Sorted && !o.all && !o.not {
            sorted_search(ops, &original, &input)
        } else {
            linear_search(ops, &original, &input, &mut re, &mut regex_matches)
        }
    })(ops);
    if let Err(first) = ops.check_index_operation(&original) {
        if let Ok(result) = reached {
            discard_search_items(ops, vec![result]);
        }
        let _ = ops.check_index_operation(&original);
        return Err(LsearchError::from_command(first));
    }
    reached.map(|result| match result {
        index::IndexResult::Borrowed(value)
        | index::IndexResult::Created(value)
        | index::IndexResult::Detached(value)
        | index::IndexResult::ReturnedOwnedTransport(value) => value,
    })
}

fn sorted_search<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::IndexOperation,
    input: &SearchInput<'_, O::Value>,
) -> Result<index::IndexResult<O::Value>, LsearchError> {
    let elems = input.elems;
    let o = input.options;
    let start = input.start;
    let group_offset = input.group_offset;
    let key_path = input.key_path;
    let pattern = input.pattern;
    let listc = elems.len();
    let mut index: isize = -1;
    // Sorted binary search.
    let mut lower: isize = start as isize - o.group as isize;
    let mut upper: isize = listc as isize;
    while lower + (o.group as isize) != upper {
        let mut mid = (lower + upper) / 2;
        mid -= mid % o.group as isize;
        let key = select_key(ops, elems, mid as usize, group_offset, key_path)?;
        let ord = ordered_elem_cmp(
            ops,
            o.dtype,
            o.nocase,
            pattern,
            input.number,
            index::key_value::<O>(&key),
        )?;
        match ord {
            Ordering::Equal => {
                index = mid;
                if o.bisect {
                    lower = mid;
                } else {
                    upper = mid;
                }
            }
            Ordering::Less => {
                if o.increasing {
                    upper = mid;
                } else {
                    lower = mid;
                }
            }
            Ordering::Greater => {
                if o.increasing {
                    lower = mid;
                } else {
                    upper = mid;
                }
            }
        }
    }
    if o.bisect && index < 0 {
        index = lower;
    }
    result_one(ops, original, input, index)
}

fn linear_search<O: NativeIndexOps, R>(
    ops: &mut O,
    original: &O::IndexOperation,
    input: &SearchInput<'_, O::Value>,
    re: &mut Option<R>,
    regex_matches: &mut impl FnMut(&mut O, &mut R, &O::Value) -> Result<bool, LsearchError>,
) -> Result<index::IndexResult<O::Value>, LsearchError> {
    let elems = input.elems;
    let o = input.options;
    let start = input.start;
    let group_offset = input.group_offset;
    let key_path = input.key_path;
    let pattern = input.pattern;
    let listc = elems.len();
    let mut index: isize = -1;
    // Linear search.
    let mut matches: Vec<usize> = Vec::new();
    let mut g = start;
    while g < listc {
        let key = select_key(ops, elems, g, group_offset, key_path)?;
        let mut m = match o.mode {
            SearchMode::Glob => {
                let kb = index::checked_operand_bytes(ops, index::key_value::<O>(&key))
                    .map_err(LsearchError::from_command)?;
                match (str_opt(pattern.expect("glob pattern")), str_opt(&kb)) {
                    (Some(p), Some(e)) => tcl_syntax::glob::string_case_match(p, e, o.nocase),
                    _ => false,
                }
            }
            SearchMode::Regexp => regex_matches(
                ops,
                re.as_mut().expect("compiled original pattern"),
                index::key_value::<O>(&key),
            )?,
            SearchMode::Exact | SearchMode::Sorted => ordered_elem_cmp(
                ops,
                o.dtype,
                o.nocase,
                pattern,
                input.number,
                index::key_value::<O>(&key),
            )?
            .is_eq(),
        };
        if o.not {
            m = !m;
        }
        if m {
            if !o.all {
                index = g as isize;
                break;
            }
            matches.push(g);
        }
        g += o.group;
    }
    if o.all {
        return result_all(ops, original, input, &matches);
    }
    result_one(ops, original, input, index)
}

/// The element used as the search key for logical group base `base`: the
/// in-group key element drilled by the remaining `-index` path.
fn select_key<O: NativeIndexOps>(
    ops: &mut O,
    elems: &[O::Value],
    base: usize,
    group_offset: usize,
    key_path: Path<'_>,
) -> Result<index::IndexedKey<O::IndexKeyOwner>, LsearchError> {
    select_by_index(ops, &elems[base + group_offset], key_path)
}

/// Drill into a (nested) list value by an index `path` — the shared
/// [`index::drill`], with the message wrapped in [`LsearchError`].
fn select_by_index<O: NativeIndexOps>(
    ops: &mut O,
    value: &O::Value,
    path: Path<'_>,
) -> Result<index::IndexedKey<O::IndexKeyOwner>, LsearchError> {
    path.drill(ops, value).map_err(LsearchError::from_command)
}

/// Compare the search `pattern` against key `obj` under `dtype` (`-exact`/
/// `-sorted` ordering).
fn ordered_elem_cmp<O: NativeIndexOps>(
    ops: &mut O,
    dtype: SortMode,
    nocase: bool,
    pattern: Option<&[u8]>,
    number: Option<crate::native_list_numeric::NumericKey>,
    obj: &O::Value,
) -> Result<Ordering, LsearchError> {
    if let Some(pattern) = number {
        let kind = if dtype == SortMode::Integer {
            index::ListNumericKind::Integer
        } else {
            index::ListNumericKind::Real
        };
        let key = crate::native_list_numeric::number(ops, obj, OptionCommand::Search, kind)
            .map_err(LsearchError::from_command)?;
        return Ok(crate::native_list_numeric::search_order(pattern, key));
    }
    elem_cmp(
        ops,
        dtype,
        nocase,
        pattern.expect("String comparison pattern"),
        obj,
    )
}

// Original pure String comparison API is also used by the existing Unicode controls.
fn elem_cmp<O: NativeIndexOps>(
    ops: &mut O,
    dtype: SortMode,
    nocase: bool,
    pattern: &[u8],
    obj: &O::Value,
) -> Result<Ordering, LsearchError> {
    let ob = index::checked_operand_bytes(ops, obj).map_err(LsearchError::from_command)?;
    Ok(match dtype {
        SortMode::Dictionary => sort::dictionary_compare(pattern, &ob),
        SortMode::Integer | SortMode::Real => {
            unreachable!("numeric comparisons use their original scalar owner")
        }
        SortMode::Ascii => {
            if nocase {
                // Full-range fold, as C's `TclUtfCasecmp` (#2125): tclsh
                // 8.5.19 onwards answer `lsearch -nocase [list \u00c9] \u00e9`
                // with `0`, in `-exact` and `-sorted` alike.
                crate::string::fold_lower_bytes(pattern).cmp(&crate::string::fold_lower_bytes(&ob))
            } else {
                pattern.cmp(ob.as_ref())
            }
        }
    })
}

/// The empty result for a search that starts past the end.
fn empty_result<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::IndexOperation,
    o: &Opts,
) -> Result<O::Value, LsearchError> {
    if o.all || o.inline {
        Ok(ops.new_list(Vec::new()))
    } else {
        index::integer_result(ops, original, -1).map_err(LsearchError::from_command)
    }
}

/// Build the single-match result (`index < 0` = no match).
fn result_one<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::IndexOperation,
    input: &SearchInput<'_, O::Value>,
    index: isize,
) -> Result<index::IndexResult<O::Value>, LsearchError> {
    let (elems, o, group_offset, key_path) = (
        input.elems,
        input.options,
        input.group_offset,
        input.key_path,
    );
    let listc = elems.len();
    if o.inline {
        if index < 0 {
            return Ok(index::IndexResult::Created(ops.new_list(Vec::new())));
        }
        let base = usize::try_from(index).unwrap_or(0);
        return inline_item(ops, elems, base, o, group_offset, key_path);
    }
    if index < 0 {
        return index::integer_result(ops, original, -1)
            .map(index::IndexResult::Created)
            .map_err(LsearchError::from_command);
    }
    let base = usize::try_from(index).unwrap_or(0);
    if o.subindices {
        Ok(index::IndexResult::Created(subindex_obj(
            ops,
            original,
            base + group_offset,
            key_path,
            listc,
        )?))
    } else {
        index::integer_result(ops, original, index as i64)
            .map(index::IndexResult::Created)
            .map_err(LsearchError::from_command)
    }
}

/// Build the `-all` result (every match).
fn result_all<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::IndexOperation,
    input: &SearchInput<'_, O::Value>,
    matches: &[usize],
) -> Result<index::IndexResult<O::Value>, LsearchError> {
    let (elems, o, group_offset, key_path) = (
        input.elems,
        input.options,
        input.group_offset,
        input.key_path,
    );
    let listc = elems.len();
    let mut out: Vec<index::IndexResult<O::Value>> = Vec::new();
    for &base in matches {
        if o.inline {
            // The inline item is a list (group) or a value; flatten a group.
            if !o.subindices && o.group > 1 {
                for e in &elems[base..base + o.group] {
                    out.push(index::IndexResult::Borrowed(e.clone()));
                }
            } else {
                match inline_leaf(ops, elems, base, o, group_offset, key_path) {
                    Ok(member) => out.push(member),
                    Err(error) => {
                        if let Some(first) = &error.command_error {
                            ops.retain_index_failure(first);
                        }
                        discard_search_items(ops, out);
                        let _ = ops.check_index_operation(original);
                        return Err(error);
                    }
                }
            }
        } else if o.subindices {
            match subindex_obj(ops, original, base + group_offset, key_path, listc) {
                Ok(created) => out.push(index::IndexResult::Created(created)),
                Err(error) => {
                    if let Some(first) = &error.command_error {
                        ops.retain_index_failure(first);
                    }
                    discard_search_items(ops, out);
                    let _ = ops.check_index_operation(original);
                    return Err(error);
                }
            }
        } else {
            match index::integer_result(ops, original, i64::try_from(base).unwrap_or(i64::MAX)) {
                Ok(created) => out.push(index::IndexResult::Created(created)),
                Err(error) => {
                    discard_search_items(ops, out);
                    let _ = ops.check_index_operation(original);
                    return Err(LsearchError::from_command(error));
                }
            }
        }
    }
    let values = out
        .into_iter()
        .map(|item| match item {
            index::IndexResult::Borrowed(value)
            | index::IndexResult::Created(value)
            | index::IndexResult::Detached(value)
            | index::IndexResult::ReturnedOwnedTransport(value) => value,
        })
        .collect();
    Ok(index::IndexResult::Created(ops.new_list(values)))
}

fn discard_search_items<O: NativeIndexOps>(ops: &mut O, items: Vec<index::IndexResult<O::Value>>) {
    for item in items {
        match item {
            index::IndexResult::Created(created)
            | index::IndexResult::Detached(created)
            | index::IndexResult::ReturnedOwnedTransport(created) => {
                ops.discard_owned_index_result(created);
            }
            index::IndexResult::Borrowed(member) => drop(member),
        }
    }
}

/// The `-inline` result item for a single match (a group list, a value, or the
/// drilled `-subindices` leaf).
fn inline_item<O: NativeIndexOps>(
    ops: &mut O,
    elems: &[O::Value],
    base: usize,
    o: &Opts,
    group_offset: usize,
    key_path: Path<'_>,
) -> Result<index::IndexResult<O::Value>, LsearchError> {
    if !o.subindices && o.group > 1 {
        let group: Vec<O::Value> = elems[base..base + o.group].to_vec();
        Ok(index::IndexResult::Created(ops.new_list(group)))
    } else {
        inline_leaf(ops, elems, base, o, group_offset, key_path)
    }
}

/// The single inline leaf value: the matched leaf (drilled by `key_path`), or —
/// when the stride consumed the only `-index` value — the in-group key element.
fn inline_leaf<O: NativeIndexOps>(
    ops: &mut O,
    elems: &[O::Value],
    base: usize,
    o: &Opts,
    group_offset: usize,
    key_path: Path<'_>,
) -> Result<index::IndexResult<O::Value>, LsearchError> {
    if o.subindices && !key_path.is_empty() {
        let key = select_by_index(ops, &elems[base + group_offset], key_path)?;
        index::key_result(ops, key).map_err(LsearchError::from_command)
    } else {
        Ok(index::IndexResult::Borrowed(
            elems[base + group_offset].clone(),
        ))
    }
}

/// Build a `-subindices` result element: the group base followed by each
/// remaining `-index` value, decoded the way C's `lsearch` does (end-relative
/// specs resolve against the top-level list count `listc`).
fn subindex_obj<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::IndexOperation,
    base: usize,
    key_path: Path<'_>,
    listc: usize,
) -> Result<O::Value, LsearchError> {
    let mut out = vec![
        index::integer_result(ops, original, i64::try_from(base).unwrap_or(i64::MAX))
            .map_err(LsearchError::from_command)?,
    ];
    for ordinal in 0..key_path.len() {
        let v = match key_path.decode(ops, ordinal, listc) {
            Ok(value) => value,
            Err(error) if error.native_execution_refusal().is_some() => {
                ops.retain_index_failure(&error);
                for created in out {
                    ops.discard_owned_index_result(created);
                }
                return Err(LsearchError::from_command(error));
            }
            Err(_) => 0,
        };
        match index::integer_result(ops, original, v) {
            Ok(created) => out.push(created),
            Err(error) => {
                for created in out {
                    ops.discard_owned_index_result(created);
                }
                let _ = ops.check_index_operation(original);
                return Err(LsearchError::from_command(error));
            }
        }
    }
    Ok(ops.new_list(out))
}

/// Split an `-index` argument (a Tcl list) into its component specs.
fn split_index(arg: &[u8]) -> Result<Vec<Vec<u8>>, LsearchError> {
    let s = str_opt(arg).ok_or_else(|| bad_index(arg))?;
    split_list(s)
        .map(|elems| {
            elems
                .into_iter()
                .map(|e| e.into_owned().into_bytes())
                .collect()
        })
        .map_err(|e| LsearchError::msg(e.message().to_string()))
}

/// Validate each `-index` value's scale at parse time (`TclIndexEncode`): a
/// syntactically-bad spec is `bad index`, an unencodable one (negative, or
/// `end+N`) is `out of range`.
fn validate_index_path<O: NativeIndexOps>(
    ops: &mut O,
    path: &[Vec<u8>],
) -> Result<(), LsearchError> {
    for spec in path {
        match index::encodable_for_ops(ops, &str_of(spec)) {
            Ok(true) => {}
            Ok(false) => {
                let mut m = b"index \"".to_vec();
                m.extend_from_slice(spec);
                m.extend_from_slice(b"\" out of range");
                return Err(LsearchError::coded(m, b"TCL VALUE INDEX OUTOFRANGE"));
            }
            Err(error) if error.native_execution_refusal().is_some() => {
                return Err(LsearchError::from_command(error));
            }
            Err(_) => return Err(bad_index(spec)),
        }
    }
    Ok(())
}

/// `&[u8]` → owned `String` (lossy-safe: index/option specs are ASCII).
fn str_of(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// `&[u8]` → `&str`, or `None` if not valid UTF-8.
fn str_opt(b: &[u8]) -> Option<&str> {
    core::str::from_utf8(b).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regex::RegMatch;

    /// `split_index` result as a `Vec<Vec<u8>>` (`LsearchError` has no Debug,
    /// so we can't `.unwrap()`).
    fn split_ok(arg: &[u8]) -> Vec<Vec<u8>> {
        let Ok(v) = split_index(arg) else {
            panic!("expected a parseable index list")
        };
        v
    }

    #[test]
    fn split_index_parses_list_specs() {
        // `lsearch -index {…}` splits a Tcl list into component specs.
        assert_eq!(
            split_ok(b"0 1 2"),
            vec![b"0".to_vec(), b"1".to_vec(), b"2".to_vec()]
        );
        assert_eq!(split_ok(b"{0} {1}"), vec![b"0".to_vec(), b"1".to_vec()]);
    }

    #[test]
    fn validate_index_path_classifies_specs() {
        // Parse-time `TclIndexEncode` validation: encodable → Ok, negative /
        // `end+N` → out of range, garbage → bad index.
        assert!(validate_index_path(&mut StrOps, &[b"0".to_vec(), b"1".to_vec()]).is_ok());
        assert!(validate_index_path(&mut StrOps, &[b"end".to_vec()]).is_ok());
        assert!(validate_index_path(&mut StrOps, &[b"-1".to_vec()]).is_err()); // out of range
        assert!(validate_index_path(&mut StrOps, &[b"end+1".to_vec()]).is_err()); // out of range
        assert!(validate_index_path(&mut StrOps, &[b"bad".to_vec()]).is_err()); // bad index
    }

    /// A throwaway string-only `ValueOps`, as `switch`/`string` keep for their
    /// own core tests: `elem_cmp` only ever reads the element's bytes.
    #[derive(Default)]
    struct StrOps;

    impl crate::index::NativeIndexOps for StrOps {
        type IndexOperation = ();
        type IndexBinaryReceipt = ();
        type IndexKeyOwner = Self::Value;
        fn index_borrowed_key(
            &mut self,
            value: &Self::Value,
        ) -> Result<Self::Value, crate::CmdError> {
            Ok(value.clone())
        }
        fn index_created_key(
            &mut self,
            value: Self::Value,
        ) -> Result<Self::Value, crate::CmdError> {
            Ok(value)
        }
        fn index_key_value(owner: &Self::Value) -> &Self::Value {
            owner
        }
        fn index_hold_key_result(
            &mut self,
            owner: &Self::Value,
        ) -> Result<Self::Value, crate::CmdError> {
            Ok(owner.clone())
        }
        fn index_finish_held_key_result(
            &mut self,
            held: Self::Value,
        ) -> Result<crate::index::IndexResult<Self::Value>, crate::CmdError> {
            Ok(crate::index::IndexResult::Detached(held))
        }
        fn index_key_argument(
            &mut self,
            owner: &Self::Value,
        ) -> Result<Self::Value, crate::CmdError> {
            Ok(owner.clone())
        }
        fn index_key_result(
            &mut self,
            owner: &Self::Value,
            created: bool,
            ancestor: bool,
        ) -> Result<crate::index::IndexResult<Self::Value>, crate::CmdError> {
            Ok(if created {
                crate::index::IndexResult::Created(owner.clone())
            } else if ancestor {
                crate::index::IndexResult::Detached(owner.clone())
            } else {
                crate::index::IndexResult::Borrowed(owner.clone())
            })
        }
        fn enter_index_operation(&mut self) -> Result<(), crate::CmdError> {
            Ok(())
        }
        fn check_index_operation(&mut self, _: &()) -> Result<(), crate::CmdError> {
            Ok(())
        }
        fn retain_index_failure(&mut self, _error: &crate::CmdError) {
            // Pure neutral adapter has no interpreter first-cause owner.
        }
        fn discard_owned_index_result(&mut self, created: Self::Value) {
            drop(created);
        }
    }

    impl ValueOps for StrOps {
        fn index_syntax(&self) -> Option<tcl_dialect::IndexSyntax> {
            Some(tcl_dialect::IndexSyntax::for_version(TclVersion::V9_0))
        }
        type Value = String;
        fn new_str(&mut self, s: &str) -> String {
            s.to_owned()
        }
        fn new_int(&mut self, n: i64) -> String {
            n.to_string()
        }
        fn new_double(&mut self, f: f64) -> String {
            tcl_syntax::number::format_double(f)
        }
        fn new_bool(&mut self, b: bool) -> String {
            (if b { "1" } else { "0" }).to_owned()
        }
        fn new_list(&mut self, items: Vec<String>) -> String {
            items.join(" ")
        }
        fn as_bytes(&mut self, v: &String) -> std::rc::Rc<[u8]> {
            std::rc::Rc::from(v.as_bytes())
        }
        fn new_bytes(&mut self, bytes: &[u8]) -> Self::Value {
            self.new_str(std::str::from_utf8(bytes).expect("Unicode-only fixture input"))
        }

        fn as_int(&mut self, v: &String) -> Result<i64, tcl_syntax::value::ValueError> {
            v.parse()
                .map_err(|_| tcl_syntax::value::ValueError::NotInteger(v.clone()))
        }
        fn as_double(&mut self, _v: &String) -> Result<f64, tcl_syntax::value::ValueError> {
            Ok(0.0)
        }
        fn as_bool(&mut self, _v: &String) -> Result<bool, tcl_syntax::value::ValueError> {
            Ok(false)
        }
        fn list_elements(
            &mut self,
            v: &String,
        ) -> Result<Vec<String>, tcl_syntax::value::ValueError> {
            Ok(v.split_whitespace().map(str::to_owned).collect())
        }
    }

    /// An engine whose every pattern fails to compile.
    enum RejectingEngine {}

    impl RegexEngine for RejectingEngine {
        type Regex = ();
        fn compile(_pattern: &[u8], _flags: RegexFlags) -> Result<(), Vec<u8>> {
            Err(b"parentheses () not balanced".to_vec())
        }
        fn nsub(_re: &()) -> usize {
            unreachable!()
        }
        fn exec(
            _re: &mut (),
            _cps: &[i32],
            _offset: usize,
            _notbol: bool,
        ) -> crate::regex::RegexpPrecision<RegMatch> {
            unreachable!()
        }
        const IDENTITY: crate::regex::EngineIdentity = crate::regex::EngineIdentity {
            name: "rejecting",
            revision: 0,
        };
    }

    #[test]
    fn regexp_compile_error_prefix_follows_the_release() {
        // tclsh 8.4.20 / 8.5.19 / 8.6.18 say `couldn't`, 9.0.4 / 9.1.0 `cannot`:
        //   % lsearch -regexp {a b} (
        //   couldn't compile regular expression pattern: parentheses () not balanced
        for (version, verb) in [
            (TclVersion::V8_4, "couldn't"),
            (TclVersion::V8_5, "couldn't"),
            (TclVersion::V8_6, "couldn't"),
            (TclVersion::V9_0, "cannot"),
            (TclVersion::V9_1, "cannot"),
        ] {
            let args = ["-regexp", "a b", "("].map(str::to_owned);
            let Err(e) = lsearch::<StrOps, RejectingEngine>(&mut StrOps, &args, version) else {
                panic!("{version:?}: a bad pattern must not compile")
            };
            assert_eq!(
                String::from_utf8_lossy(&e.message),
                format!("{verb} compile regular expression pattern: parentheses () not balanced"),
                "{version:?}"
            );
        }
    }

    #[test]
    fn nocase_element_compare_folds_the_full_unicode_range() {
        // Regression (#2125): the `-exact`/`-sorted` `-nocase` comparison folded
        // with `to_ascii_lowercase`, so a non-ASCII letter never matched. tclsh
        // 8.5.19 / 8.6.18 / 9.0.4 / 9.1b0 (the releases with `lsearch -nocase`):
        //   % lsearch -nocase [list É] é        ;# 0
        //   % lsearch -exact -nocase [list É] é ;# 0
        //   % lsearch -sorted -nocase [list É] é ;# 0
        //   % lsearch -nocase [list İ] i             ;# 0
        let mut ops = StrOps;
        let cmp = |ops: &mut StrOps, pattern: &str, elem: &str| {
            let Ok(o) = elem_cmp(
                ops,
                SortMode::Ascii,
                true,
                pattern.as_bytes(),
                &elem.to_owned(),
            ) else {
                panic!("ascii compare cannot fail")
            };
            o
        };
        assert_eq!(cmp(&mut ops, "\u{e9}", "\u{c9}"), Ordering::Equal);
        assert_eq!(cmp(&mut ops, "\u{410}", "\u{430}"), Ordering::Equal);
        assert_eq!(cmp(&mut ops, "i", "\u{130}"), Ordering::Equal);
        // Ordering (which `-sorted` bisects on) follows the folded code points.
        assert_eq!(cmp(&mut ops, "\u{e1}", "\u{c2}"), Ordering::Less);
        assert_eq!(cmp(&mut ops, "aBc", "AbC"), Ordering::Equal);
        // Case-sensitive comparison is untouched.
        let Ok(o) = elem_cmp(
            &mut ops,
            SortMode::Ascii,
            false,
            "\u{e9}".as_bytes(),
            &"\u{c9}".to_owned(),
        ) else {
            panic!("ascii compare cannot fail")
        };
        assert_eq!(o, Ordering::Greater);
    }
}
