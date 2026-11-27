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

//! `regexp`: its option tables and scan, the byte and original-object entry points, `-about` and the match items.

use super::jim::regexp_jim_original;
use super::original_pattern::{NativeRegexObjects, regexp_native_selected};
use super::{
    AnalysisMatch, Common, NO_MATCH, RegMatch, RegexEngine, RegexError, RegexFailure, RegexFlags,
    RegexpPrecision, RegexpResult, Run, decode_utf8, discard_regexp_result, regexp_notbol_at,
    resolve_start_checked, wrong_args,
};
use crate::error::CmdError;
use crate::index::NativeIndexOps;
use crate::prefix::OptionTable;
use tcl_dialect::TclVersion;
use tcl_syntax::value::ValueOps;

pub(super) const REGEXP_USAGE: &[u8] =
    b"regexp ?-option ...? exp string ?matchVar? ?subMatchVar ...?";

// C's `options[]` in `Tcl_RegexpObjCmd` (`tclCmdMZ.c`), matched with
// `TCL_EXACT`: abbreviations are rejected, so `regexp -no …` is a bad option
// here. tclsh's bytecode compiler (`TclCompileRegexpCmd`) separately accepts
// any two-plus-character prefix of `-nocase` in its no-match-variable fast
// path `regexp ?-nocase? ?--? exp string`, so that one form abbreviates in
// tclsh scripts; every other form (match variables, any other option, an
// `eval`'d word list) reaches the runtime command and is exact-only. We
// implement the runtime semantics everywhere (probed tclsh 8.6.14; 9.0.4
// source).
const RE_ALL: usize = 0;

const RE_ABOUT: usize = 1;

const RE_INDICES: usize = 2;

const RE_INLINE: usize = 3;

const RE_START: usize = 9;

static REGEXP_NAMES: [&str; 11] = [
    "-all",
    "-about",
    "-indices",
    "-inline",
    "-expanded",
    "-line",
    "-linestop",
    "-lineanchor",
    "-nocase",
    "-start",
    "--",
];

static REGEXP_OPTIONS: OptionTable<'static> = OptionTable::exact_only("option", &REGEXP_NAMES);

// `Tcl_GetIndexFromObj`'s noun is "switch" through 8.5, as for `regsub`:
//   tclsh8.4.20 / 8.5.19: bad switch "-bogus": must be -all, -about, ...
static REGEXP_OPTIONS_8_4: OptionTable<'static> = OptionTable::exact_only("switch", &REGEXP_NAMES);

/// Drive `regexp` over the engine `E` and value-ops `O` for `version`. `args`
/// is the command's arguments **without** the command name.
///
/// # Errors
/// Option/arg/compile errors as ready-to-report [`RegexError`] messages.
// Both byte compatibility entry points and native original-object callers
// use this scanner. Physical Index selection is owned by ValueOps, never by
// reconstructed option strings.
pub(super) trait OptionArguments {
    fn len(&self) -> usize;
    fn bytes(&mut self, index: usize) -> Result<Vec<u8>, RegexError>;
    fn option(&mut self, index: usize, table: &OptionTable<'static>) -> Result<usize, RegexError>;
}

pub(super) struct ByteOptionArguments<'a>(pub(super) &'a [&'a [u8]]);

impl OptionArguments for ByteOptionArguments<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn bytes(&mut self, index: usize) -> Result<Vec<u8>, RegexError> {
        Ok(self.0[index].to_vec())
    }
    fn option(&mut self, index: usize, table: &OptionTable<'static>) -> Result<usize, RegexError> {
        table.index_of(self.0[index]).map_err(RegexError::new)
    }
}

pub(super) struct OriginalOptionArguments<'a, O: ValueOps> {
    pub(super) ops: &'a mut O,
    pub(super) args: &'a [O::Value],
}

impl<O: ValueOps> OptionArguments for OriginalOptionArguments<'_, O> {
    fn len(&self) -> usize {
        self.args.len()
    }
    fn bytes(&mut self, index: usize) -> Result<Vec<u8>, RegexError> {
        self.ops
            .native_string_bytes(&self.args[index])
            .map(|bytes| bytes.to_vec())
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))
    }
    fn option(&mut self, index: usize, table: &OptionTable<'static>) -> Result<usize, RegexError> {
        table
            .index_of_original(self.ops, &self.args[index])
            .map_err(RegexError::from)
    }
}

fn regexp_option_scan(
    args: &mut impl OptionArguments,
    version: TclVersion,
) -> Result<(Common, bool, bool, bool, usize), RegexError> {
    let mut c = Common::for_release(version);
    let mut indices = false;
    let mut inline = false;
    let mut about = false;

    let mut i = 0;
    while i < args.len() {
        let name = args.bytes(i)?;
        if name.first() != Some(&b'-') {
            break;
        }
        let options = if version < TclVersion::V8_6 {
            &REGEXP_OPTIONS_8_4
        } else {
            &REGEXP_OPTIONS
        };
        let idx = args.option(i, options)?;
        i += 1;
        if c.flags.apply_compile_option(options.names()[idx]) {
            continue;
        }
        match idx {
            RE_ALL => c.all = true,
            RE_ABOUT => about = true,
            RE_INDICES => indices = true,
            RE_INLINE => inline = true,
            RE_START => match (i < args.len()).then(|| args.bytes(i)).transpose()? {
                Some(v) => {
                    c.start = Some(v);
                    c.start_at = Some(i);
                    i += 1;
                }
                // A trailing `-start` ends the options (C's `goto
                // endOfForLoop`); the arity check below then reports it.
                None => break,
            },
            // `--`: explicit end of options.
            _ => break,
        }
    }

    Ok((c, indices, inline, about, i))
}

/// Regex preparation or an original variable setter completion. Setter failures
/// stop the current match before any later variable or match is evaluated.
#[derive(Debug)]
pub enum RegexpExecutionError<E> {
    Regex(RegexError),
    Assignment(E),
}

pub(super) fn regex_execution_error<E>(error: impl Into<CmdError>) -> RegexpExecutionError<E> {
    RegexpExecutionError::Regex(RegexError::from(error.into()))
}

/// Resolve original options and assign each original target inside the match
/// loop. Unmatched, inline and about paths never materialise target objects.
pub fn regexp_original<O: NativeRegexObjects<E>, E: RegexEngine, Err>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
    mut assign: impl FnMut(&mut O, &O::Value, O::Value) -> Result<(), Err>,
) -> Result<RegexpResult<O::Value>, RegexpExecutionError<Err>>
where
    E::Regex: 'static,
{
    crate::index::run_index_output(
        ops,
        |ops| {
            if ops
                .jim_regex_recipe()
                .map_err(|error| {
                    RegexpExecutionError::Regex(RegexError::from(crate::CmdError::from(error)))
                })?
                .is_some()
            {
                return regexp_jim_original::<O, E, Err>(ops, args, version, assign);
            }
            let (c, indices, inline, about, offset) =
                regexp_option_scan(&mut OriginalOptionArguments { ops, args }, version)
                    .map_err(RegexpExecutionError::Regex)?;
            let remaining = &args[offset..];
            // Validate arity before reaching any non-option getter.
            if remaining.len() + usize::from(about) < 2 {
                return Err(RegexpExecutionError::Regex(wrong_args(REGEXP_USAGE)));
            }
            if inline && remaining.len() != 2 {
                let message = b"regexp match variables not allowed when using -inline".to_vec();
                // C8.6 added the operation code to this refusal.
                return Err(RegexpExecutionError::Regex(
                    if version >= TclVersion::V8_6 {
                        RegexError::coded(message, "TCL OPERATION REGEXP MIX_VAR_INLINE")
                    } else {
                        RegexError::new(message)
                    },
                ));
            }
            let recipe = ops.regex_recipe().map_err(|error| {
                RegexpExecutionError::Regex(RegexError::from(crate::CmdError::from(error)))
            })?;
            if let Some(recipe) = recipe {
                return regexp_native_selected::<O, E, Err>(
                    ops,
                    args,
                    remaining,
                    recipe,
                    RegexpOptions {
                        version,
                        common: c,
                        indices,
                        inline,
                        about,
                    },
                    |ops, index, value| assign(ops, &remaining[index + 2], value),
                );
            }
            let bytes = remaining[..if about { 1 } else { 2 }]
                .iter()
                .map(|value| {
                    ops.native_string_bytes(value)
                        .map(|bytes| bytes.to_vec())
                        .map_err(|error| {
                            RegexpExecutionError::Regex(RegexError::from(crate::CmdError::from(
                                error,
                            )))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut rest = bytes.iter().map(Vec::as_slice).collect::<Vec<_>>();
            rest.resize(remaining.len(), &[]);
            regexp_selected_with_sink::<O, E, Err>(
                ops,
                &rest,
                RegexpOptions {
                    version,
                    common: c,
                    indices,
                    inline,
                    about,
                },
                |ops, index, value| assign(ops, &remaining[index + 2], value),
            )
        },
        discard_regexp_result,
        |first| RegexpExecutionError::Regex(RegexError::from(first)),
    )
}

/// Drive the byte compatibility command under the runtime search limits.
/// Original-object callers use [`regexp_original`].
///
/// # Errors
/// Command errors or an incomplete search, retaining the full error receipt.
pub fn regexp<O: NativeIndexOps, E: RegexEngine>(
    ops: &mut O,
    args: &[&[u8]],
    version: TclVersion,
) -> Result<RegexpResult<O::Value>, RegexError>
where
    E::Regex: 'static,
{
    regexp_run::<O, E>(ops, args, version, &mut Run::Runtime).map_err(RegexFailure::into_error)
}

/// Byte compatibility analysis with the bounded cache and search limits.
///
/// # Errors
/// Command errors remain errors; an incomplete search remains a typed decline.
pub fn regexp_analysis<O: NativeIndexOps, E: RegexEngine>(
    ops: &mut O,
    args: &[&[u8]],
    version: TclVersion,
    analysis: &mut AnalysisMatch<'_>,
) -> Result<RegexpResult<O::Value>, RegexFailure>
where
    E::Regex: 'static,
{
    regexp_run::<O, E>(ops, args, version, &mut Run::Analysis(analysis))
}

fn regexp_run<O: NativeIndexOps, E: RegexEngine>(
    ops: &mut O,
    args: &[&[u8]],
    version: TclVersion,
    run: &mut Run<'_, '_>,
) -> Result<RegexpResult<O::Value>, RegexFailure>
where
    E::Regex: 'static,
{
    crate::index::run_index_output(
        ops,
        |ops| {
            let (common, indices, inline, about, i) =
                regexp_option_scan(&mut ByteOptionArguments(args), version)?;
            let rest = &args[i..];
            // Deferred captures keep their genuine constructor cleanup role
            // through every later match/getter, including replacement by -all.
            let mut pairs: Vec<(Vec<u8>, O::IndexKeyOwner)> = Vec::new();
            let outcome = regexp_selected_run::<O, E, CmdError>(
                ops,
                rest,
                RegexpOptions {
                    version,
                    common,
                    indices,
                    inline,
                    about,
                },
                |ops, index, value| {
                    if index == 0 {
                        pairs.clear();
                    }
                    pairs.push((rest[index + 2].to_vec(), ops.index_created_key(value)?));
                    Ok(())
                },
                run,
            );
            match outcome {
                Ok(RegexpResult::Count { count, .. }) => {
                    let mut assign = Vec::with_capacity(pairs.len());
                    for (name, owner) in pairs {
                        let value = match ops.index_key_result(&owner, true, false) {
                            Ok(crate::index::IndexResult::Created(value)) => value,
                            Ok(_) => unreachable!("deferred capture constructor role"),
                            Err(first) => {
                                for (_, reached) in assign {
                                    ops.discard_owned_index_result(reached);
                                }
                                return Err(RegexFailure::Error(RegexError::from(first)));
                            }
                        };
                        assign.push((name, value));
                    }
                    Ok(RegexpResult::Count {
                        assign: (count != 0).then_some(assign),
                        count,
                    })
                }
                Ok(result) => Ok(result),
                Err(RegexpRunError::Regex(error)) => Err(error),
                Err(RegexpRunError::Assignment(error)) => {
                    Err(RegexFailure::Error(RegexError::from(error)))
                }
            }
        },
        discard_regexp_result,
        |first| RegexFailure::Error(RegexError::from(first)),
    )
}

enum RegexpRunError<Err> {
    Regex(RegexFailure),
    Assignment(Err),
}

fn regexp_selected_with_sink<O: NativeIndexOps, E: RegexEngine, Err>(
    ops: &mut O,
    rest: &[&[u8]],
    options: RegexpOptions,
    assign: impl FnMut(&mut O, usize, O::Value) -> Result<(), Err>,
) -> Result<RegexpResult<O::Value>, RegexpExecutionError<Err>>
where
    E::Regex: 'static,
{
    regexp_selected_run::<O, E, Err>(ops, rest, options, assign, &mut Run::Runtime).map_err(
        |failure| match failure {
            RegexpRunError::Regex(error) => RegexpExecutionError::Regex(error.into_error()),
            RegexpRunError::Assignment(error) => RegexpExecutionError::Assignment(error),
        },
    )
}

pub(super) struct RegexpOptions {
    pub(super) version: TclVersion,
    pub(super) common: Common,
    pub(super) indices: bool,
    pub(super) inline: bool,
    pub(super) about: bool,
}

/// `TclRegAbout` (`tclRegexp.c`): a two-element list of the subexpression
/// count and the engine's `re_info` flag names. The compile flags still apply
/// — `regexp -about -expanded {a b}` is `0 REG_UNONPOSIX` on every release —
/// but nothing else about the command runs: no subject decode, no `-start`, no
/// match loop, and `-indices`/`-all` are simply ignored (all tclsh-verified,
/// 8.4.20 through 9.1b0).
fn regexp_about<O: NativeIndexOps, E: RegexEngine, Err>(
    ops: &mut O,
    pattern: &[u8],
    flags: RegexFlags,
    version: TclVersion,
    run: &mut Run<'_, '_>,
) -> Result<RegexpResult<O::Value>, RegexpRunError<Err>>
where
    E::Regex: 'static,
{
    let re = run
        .compile::<E>(pattern, flags, version)
        .map_err(RegexpRunError::Regex)?;
    // `Inline` is "the command result *is* this value, and no match
    // variable is written", which is exactly `-about`'s contract too — so
    // it carries the answer rather than the result enum gaining a variant
    // every adapter would have to learn.
    Ok(RegexpResult::Inline(about_answer::<O, E>(
        ops,
        &re.borrow(),
    )))
}

/// The `-about` answer: the subexpression count and the engine's flag names.
pub(super) fn about_answer<O: ValueOps, E: RegexEngine>(ops: &mut O, re: &E::Regex) -> O::Value {
    let count = ops.new_int(i64::try_from(E::nsub(re)).unwrap_or(i64::MAX));
    let names = E::info_names(re)
        .into_iter()
        .map(|name| ops.new_str(name))
        .collect();
    let names = ops.new_list(names);
    ops.new_list(vec![count, names])
}

fn regexp_selected_run<O: NativeIndexOps, E: RegexEngine, Err>(
    ops: &mut O,
    rest: &[&[u8]],
    options: RegexpOptions,
    mut assign: impl FnMut(&mut O, usize, O::Value) -> Result<(), Err>,
    run: &mut Run<'_, '_>,
) -> Result<RegexpResult<O::Value>, RegexpRunError<Err>>
where
    E::Regex: 'static,
{
    let RegexpOptions {
        version,
        common: c,
        indices,
        inline,
        about,
    } = options;
    // C: `(objc - i) < (2 - about)`. `-about` never looks at a subject, so the
    // pattern alone is enough — tclsh 8.4.20/8.5.19/8.6.18/9.0.4/9.1b0 all
    // answer `regexp -about {a(b)c}` with `1 {}` and give the same answer for
    // `regexp -about {(a)} extraarg`, while bare `regexp -about` is still a
    // wrong-# args.
    if rest.len() + usize::from(about) < 2 {
        return Err(RegexpRunError::Regex(wrong_args(REGEXP_USAGE).into()));
    }
    // C tests `-inline` against the *exact* remaining count and does it before
    // branching to `-about`, so `regexp -about -inline {(a)}` is the mix error
    // rather than an about answer (tclsh 8.4.20–9.1b0). Without `-about` the
    // arity check above has already forced `>= 2`, so `!= 2` is the old `> 2`.
    if inline && rest.len() != 2 {
        return Err(RegexpRunError::Regex(
            RegexError::coded(
                b"regexp match variables not allowed when using -inline".to_vec(),
                "TCL OPERATION REGEXP MIX_VAR_INLINE",
            )
            .into(),
        ));
    }

    let pattern = rest[0];
    if about {
        return regexp_about::<O, E, Err>(ops, pattern, c.flags, version, run);
    }
    let str_bytes = rest[1];
    let (cps, byteoff) = decode_utf8(str_bytes);
    let char_len = cps.len();
    let match_vars = &rest[2..];

    let re = run
        .compile::<E>(pattern, c.flags, version)
        .map_err(RegexpRunError::Regex)?;
    let nsubs = E::nsub(&re.borrow());

    let mut offset = c
        .start
        .as_ref()
        .map_or(Ok(0), |spec| resolve_start_checked(ops, spec, char_len))
        .map_err(|error| RegexpRunError::Regex(error.into()))?;

    // Tcl's `all` doubles as flag + counter: starts 1 if `-all`, else 0.
    let mut all_count: i64 = i64::from(c.all);
    let mut inline_items: Vec<O::IndexKeyOwner> = Vec::new();

    loop {
        let notbol = regexp_notbol_at(&cps, offset, version);
        let answer = run.exec::<E>(&re, &cps, offset, notbol);
        if let RegexpPrecision::Declined(decline) = answer {
            return Err(RegexpRunError::Regex(RegexFailure::Declined(decline)));
        }
        let Some(matches) = answer.match_vector() else {
            if all_count <= 1 {
                // No match at all (first time through).
                return Ok(if inline {
                    RegexpResult::Inline(ops.new_list(Vec::new()))
                } else {
                    RegexpResult::Count {
                        assign: None,
                        count: 0,
                    }
                });
            }
            break;
        };
        let m0 = matches[0];
        if inline {
            for k in 0..=nsubs {
                let created =
                    build_match_item(ops, &matches, k, nsubs, indices, str_bytes, &byteoff);
                inline_items.push(
                    ops.index_created_key(created)
                        .map_err(|error| RegexpRunError::Regex(RegexError::from(error).into()))?,
                );
            }
        } else {
            for index in 0..match_vars.len() {
                let value =
                    build_match_item(ops, &matches, index, nsubs, indices, str_bytes, &byteoff);
                assign(ops, index, value).map_err(RegexpRunError::Assignment)?;
            }
        }

        if !c.all {
            break;
        }
        offset = m0.eo;
        if m0.eo == m0.so {
            offset += 1; // zero-length match: always advance to avoid looping
        }
        all_count += 1;
        if offset >= char_len {
            break;
        }
    }

    if inline {
        let values = inline_items
            .iter()
            .map(|owner| ops.index_key_argument(owner))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| RegexpRunError::Regex(RegexError::from(error).into()))?;
        return Ok(RegexpResult::Inline(ops.new_list(values)));
    }
    Ok(RegexpResult::Count {
        assign: None,
        count: if all_count > 0 { all_count - 1 } else { 1 },
    })
}

/// Slice the original bytes for the character range `[so, eo)` via the char→byte
/// table, **guarding every index**. `byteoff` has one entry per character plus a
/// final `bytes.len()`, so `so`/`eo` must be `<= char_len`; the FFI ARE engine
/// always honours that, but it is foreign code, so an out-of-range or inverted
/// `[so, eo)` must not index-panic here. Anything
/// off the table or backwards yields an empty slice rather than aborting.
pub(super) fn slice_match<'a>(
    str_bytes: &'a [u8],
    byteoff: &[usize],
    so: usize,
    eo: usize,
) -> &'a [u8] {
    match (byteoff.get(so), byteoff.get(eo)) {
        (Some(&a), Some(&b)) if a <= b && b <= str_bytes.len() => &str_bytes[a..b],
        _ => &[],
    }
}

/// Build the value for match item `k`: an `{start end}` index pair (`-indices`)
/// or the matched substring (default). A non-participating group yields
/// `{-1 -1}` / the empty string, per `Tcl_RegexpObjCmd`.
// char offsets into a real subject are far below `i64::MAX`; the `i64` pair is
// Tcl's `-indices` result format.
#[allow(clippy::cast_possible_wrap)]
fn build_match_item<O: ValueOps>(
    ops: &mut O,
    matches: &[RegMatch],
    k: usize,
    nsubs: usize,
    indices: bool,
    str_bytes: &[u8],
    byteoff: &[usize],
) -> O::Value {
    let m = if k <= nsubs {
        matches.get(k).copied()
    } else {
        None
    };
    if indices {
        let (start, end): (i64, i64) = match m {
            Some(rm) if rm.so != NO_MATCH => (rm.so as i64, rm.eo as i64 - 1),
            _ => (-1, -1),
        };
        let a = ops.new_int(start);
        let b = ops.new_int(end);
        ops.new_list(vec![a, b])
    } else {
        match m {
            Some(rm) if rm.so != NO_MATCH && rm.eo > 0 => {
                ops.new_bytes(slice_match(str_bytes, byteoff, rm.so, rm.eo))
            }
            _ => ops.new_bytes(b""),
        }
    }
}
