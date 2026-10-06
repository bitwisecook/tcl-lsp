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

use tcl_dialect::TclVersion;
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

fn not_real(s: &[u8]) -> LsearchError {
    let mut m = b"expected floating-point number but got \"".to_vec();
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
    index_path: Vec<Vec<u8>>,
    start_spec: Option<Vec<u8>>,
}

/// Drive `lsearch` over `args` (the command's arguments **without** the command
/// name: `?option ...? list pattern`).
///
/// # Errors
/// Option/index/coercion errors as a [`LsearchError`] (message + optional code).
pub fn lsearch<O: ValueOps, E: RegexEngine>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
) -> Result<O::Value, LsearchError> {
    lsearch_with(
        ops,
        args,
        version,
        |ops, pattern, flags| {
            E::compile(&ops.as_bytes(pattern), flags).map_err(|detail| {
                let mut message = version.regex_compile_error_prefix().as_bytes().to_vec();
                message.extend_from_slice(&detail);
                LsearchError::msg(message)
            })
        },
        |ops, compiled, subject| {
            let bytes = ops.as_bytes(subject);
            let (units, _) = decode_utf8(&bytes);
            Ok(E::exec(compiled, &units, 0, false).is_some())
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
            index_path: Vec::new(),
            start_spec: None,
        }
    }
}

fn parse_options<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Result<Opts, LsearchError> {
    let n = args.len();
    let mut o = Opts::new();
    // Options occupy everything before the trailing `list pattern`.
    let last_opt = n - 2; // first non-option index
    let mut i = 0;
    while i < last_opt {
        match OPTIONS
            .index_of_original(ops, &args[i])
            .map_err(LsearchError::from_command)?
        {
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
                o.start_spec = Some(ops.as_bytes(&args[i + 1]).to_vec());
                i += 1;
            }
            OPT_STRIDE => {
                if i + 1 >= last_opt {
                    return Err(LsearchError::msg(
                        "\"-stride\" option must be followed by stride length",
                    ));
                }
                let sb = ops.as_bytes(&args[i + 1]);
                match sort::parse_wide(&sb) {
                    Some(w) if w >= 1 => o.group = usize::try_from(w).unwrap_or(usize::MAX),
                    Some(_) => return Err(LsearchError::msg("stride length must be at least 1")),
                    None => return Err(not_integer(&sb)),
                }
                i += 1;
            }
            OPT_INDEX => {
                if i + 1 >= last_opt {
                    return Err(LsearchError::msg(
                        "\"-index\" option must be followed by list index",
                    ));
                }
                o.index_path = split_index(&ops.as_bytes(&args[i + 1]))?;
                validate_index_path(ops, &o.index_path)?;
                i += 1;
            }
            _ => unreachable!("the option table is closed"),
        }
        i += 1;
    }
    // `-subindices` only makes sense alongside `-index` (C's BAD_OPTION_MIX).
    if o.subindices && o.index_path.is_empty() {
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
    key_path: &'a [Vec<u8>],
    pattern: Option<&'a [u8]>,
}

fn lsearch_with<O: ValueOps, R>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
    mut prepare: impl FnMut(&mut O, &O::Value, RegexFlags) -> Result<R, LsearchError>,
    mut regex_matches: impl FnMut(&mut O, &mut R, &O::Value) -> Result<bool, LsearchError>,
) -> Result<O::Value, LsearchError> {
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
        .list_elements(&args[n - 2])
        .map_err(|e| LsearchError::from_command(e.into()))?;
    let listc = elems.len();
    if o.group > 1 && listc % o.group != 0 {
        return Err(LsearchError::msg(
            "list size must be a multiple of the stride length",
        ));
    }

    // -stride + -index: the leading index value picks the key element within
    // each group; the rest of the path applies inside it.
    let mut group_offset = 0usize;
    let mut key_path: &[Vec<u8>] = &o.index_path;
    if o.group > 1 && !o.index_path.is_empty() {
        match index::resolve_for_ops(ops, &str_of(&o.index_path[0]), o.group).ok() {
            Some(g) if g >= 0 && (g as usize) < o.group => {
                group_offset = usize::try_from(g).unwrap_or(0);
            }
            _ => {
                return Err(LsearchError::msg(
                    "when used with \"-stride\", the leading \"-index\" value must be within the group",
                ));
            }
        }
        key_path = &o.index_path[1..];
    }

    // Resolve -start (relative to listc, then clamped to a group boundary).
    let mut start = 0usize;
    if let Some(spec) = &o.start_spec {
        let s = index::resolve_for_ops(ops, &str_of(spec), listc)
            .ok()
            .ok_or_else(|| bad_index(spec))?;
        let s = usize::try_from(s.max(0)).unwrap_or(0);
        if s >= listc {
            return Ok(empty_result(ops, &o));
        }
        start = s - (s % o.group);
    }

    let pattern = (o.mode != SearchMode::Regexp).then(|| ops.as_bytes(&args[n - 1]).to_vec());
    // For numeric exact/sorted search, the pattern must parse as that type.
    if matches!(o.mode, SearchMode::Exact | SearchMode::Sorted) {
        if o.dtype == SortMode::Integer
            && sort::parse_wide(pattern.as_deref().expect("numeric pattern")).is_none()
        {
            return Err(not_integer(pattern.as_deref().expect("numeric pattern")));
        }
        if o.dtype == SortMode::Real
            && sort::parse_real(pattern.as_deref().expect("numeric pattern")).is_none()
        {
            return Err(not_real(pattern.as_deref().expect("numeric pattern")));
        }
    }
    let input = SearchInput {
        elems: &elems,
        options: &o,
        start,
        group_offset,
        key_path,
        pattern: pattern.as_deref(),
    };
    if o.mode == SearchMode::Sorted && !o.all && !o.not {
        sorted_search(ops, &input)
    } else {
        linear_search(ops, &input, &mut re, &mut regex_matches)
    }
}

fn sorted_search<O: ValueOps>(
    ops: &mut O,
    input: &SearchInput<'_, O::Value>,
) -> Result<O::Value, LsearchError> {
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
        let ord = elem_cmp(
            ops,
            o.dtype,
            o.nocase,
            pattern.expect("ordered pattern"),
            &key,
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
    result_one(ops, elems, index, o, group_offset, key_path, listc)
}

fn linear_search<O: ValueOps, R>(
    ops: &mut O,
    input: &SearchInput<'_, O::Value>,
    re: &mut Option<R>,
    regex_matches: &mut impl FnMut(&mut O, &mut R, &O::Value) -> Result<bool, LsearchError>,
) -> Result<O::Value, LsearchError> {
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
                let kb = ops.as_bytes(&key);
                match (str_opt(pattern.expect("glob pattern")), str_opt(&kb)) {
                    (Some(p), Some(e)) => tcl_syntax::glob::string_case_match(p, e, o.nocase),
                    _ => false,
                }
            }
            SearchMode::Regexp => {
                regex_matches(ops, re.as_mut().expect("compiled original pattern"), &key)?
            }
            SearchMode::Exact | SearchMode::Sorted => elem_cmp(
                ops,
                o.dtype,
                o.nocase,
                pattern.expect("ordered pattern"),
                &key,
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
        return result_all(ops, elems, &matches, o, group_offset, key_path, listc);
    }
    result_one(ops, elems, index, o, group_offset, key_path, listc)
}

/// The element used as the search key for logical group base `base`: the
/// in-group key element drilled by the remaining `-index` path.
fn select_key<O: ValueOps>(
    ops: &mut O,
    elems: &[O::Value],
    base: usize,
    group_offset: usize,
    key_path: &[Vec<u8>],
) -> Result<O::Value, LsearchError> {
    select_by_index(ops, &elems[base + group_offset], key_path)
}

/// Drill into a (nested) list value by an index `path` — the shared
/// [`index::drill`], with the message wrapped in [`LsearchError`].
fn select_by_index<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    path: &[Vec<u8>],
) -> Result<O::Value, LsearchError> {
    index::drill(ops, value, path).map_err(LsearchError::from_command)
}

/// Compare the search `pattern` against key `obj` under `dtype` (`-exact`/
/// `-sorted` ordering).
fn elem_cmp<O: ValueOps>(
    ops: &mut O,
    dtype: SortMode,
    nocase: bool,
    pattern: &[u8],
    obj: &O::Value,
) -> Result<Ordering, LsearchError> {
    let ob = ops.as_bytes(obj);
    Ok(match dtype {
        SortMode::Dictionary => sort::dictionary_compare(pattern, &ob),
        SortMode::Integer => {
            let o = sort::parse_wide(&ob).ok_or_else(|| not_integer(&ob))?;
            sort::parse_wide(pattern).unwrap_or(0).cmp(&o)
        }
        SortMode::Real => {
            let o = sort::parse_real(&ob).ok_or_else(|| not_real(&ob))?;
            sort::parse_real(pattern)
                .unwrap_or(0.0)
                .partial_cmp(&o)
                .unwrap_or(Ordering::Equal)
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
fn empty_result<O: ValueOps>(ops: &mut O, o: &Opts) -> O::Value {
    if o.all || o.inline {
        ops.new_list(Vec::new())
    } else {
        ops.new_int(-1)
    }
}

/// Build the single-match result (`index < 0` = no match).
fn result_one<O: ValueOps>(
    ops: &mut O,
    elems: &[O::Value],
    index: isize,
    o: &Opts,
    group_offset: usize,
    key_path: &[Vec<u8>],
    listc: usize,
) -> Result<O::Value, LsearchError> {
    if o.inline {
        if index < 0 {
            return Ok(ops.new_list(Vec::new()));
        }
        let base = usize::try_from(index).unwrap_or(0);
        return inline_item(ops, elems, base, o, group_offset, key_path);
    }
    if index < 0 {
        return Ok(ops.new_int(-1));
    }
    let base = usize::try_from(index).unwrap_or(0);
    if o.subindices {
        Ok(subindex_obj(ops, base + group_offset, key_path, listc))
    } else {
        Ok(ops.new_int(index as i64))
    }
}

/// Build the `-all` result (every match).
fn result_all<O: ValueOps>(
    ops: &mut O,
    elems: &[O::Value],
    matches: &[usize],
    o: &Opts,
    group_offset: usize,
    key_path: &[Vec<u8>],
    listc: usize,
) -> Result<O::Value, LsearchError> {
    let mut out: Vec<O::Value> = Vec::new();
    for &base in matches {
        if o.inline {
            // The inline item is a list (group) or a value; flatten a group.
            if !o.subindices && o.group > 1 {
                for e in &elems[base..base + o.group] {
                    out.push(e.clone());
                }
            } else {
                out.push(inline_leaf(ops, elems, base, o, group_offset, key_path)?);
            }
        } else if o.subindices {
            out.push(subindex_obj(ops, base + group_offset, key_path, listc));
        } else {
            out.push(ops.new_int(i64::try_from(base).unwrap_or(i64::MAX)));
        }
    }
    Ok(ops.new_list(out))
}

/// The `-inline` result item for a single match (a group list, a value, or the
/// drilled `-subindices` leaf).
fn inline_item<O: ValueOps>(
    ops: &mut O,
    elems: &[O::Value],
    base: usize,
    o: &Opts,
    group_offset: usize,
    key_path: &[Vec<u8>],
) -> Result<O::Value, LsearchError> {
    if !o.subindices && o.group > 1 {
        let group: Vec<O::Value> = elems[base..base + o.group].to_vec();
        Ok(ops.new_list(group))
    } else {
        inline_leaf(ops, elems, base, o, group_offset, key_path)
    }
}

/// The single inline leaf value: the matched leaf (drilled by `key_path`), or —
/// when the stride consumed the only `-index` value — the in-group key element.
fn inline_leaf<O: ValueOps>(
    ops: &mut O,
    elems: &[O::Value],
    base: usize,
    o: &Opts,
    group_offset: usize,
    key_path: &[Vec<u8>],
) -> Result<O::Value, LsearchError> {
    if o.subindices && !key_path.is_empty() {
        select_by_index(ops, &elems[base + group_offset], key_path)
    } else {
        Ok(elems[base + group_offset].clone())
    }
}

/// Build a `-subindices` result element: the group base followed by each
/// remaining `-index` value, decoded the way C's `lsearch` does (end-relative
/// specs resolve against the top-level list count `listc`).
fn subindex_obj<O: ValueOps>(
    ops: &mut O,
    base: usize,
    key_path: &[Vec<u8>],
    listc: usize,
) -> O::Value {
    let mut out = vec![ops.new_int(i64::try_from(base).unwrap_or(i64::MAX))];
    for spec in key_path {
        let v = index::resolve_for_ops(ops, &str_of(spec), listc + 1)
            .ok()
            .unwrap_or(0);
        out.push(ops.new_int(v));
    }
    ops.new_list(out)
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
fn validate_index_path<O: ValueOps>(ops: &mut O, path: &[Vec<u8>]) -> Result<(), LsearchError> {
    for spec in path {
        match index::encodable_for_ops(ops, &str_of(spec)) {
            Some(true) => {}
            Some(false) => {
                let mut m = b"index \"".to_vec();
                m.extend_from_slice(spec);
                m.extend_from_slice(b"\" out of range");
                return Err(LsearchError::coded(m, b"TCL VALUE INDEX OUTOFRANGE"));
            }
            None => return Err(bad_index(spec)),
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
        ) -> Option<Vec<RegMatch>> {
            unreachable!()
        }
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
