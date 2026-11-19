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

//! `regsub` over bytes: its option tables and scan, the entry points for each path and substitution-spec expansion.

use super::regexp_command::{ByteOptionArguments, OptionArguments, slice_match};
use super::{
    AnalysisMatch, Common, NO_MATCH, PrecisionDecline, RegMatch, RegexEngine, RegexError,
    RegexFailure, RegexpPrecision, RegsubResult, Run, decode_utf8, resolve_start_checked,
    wrong_args,
};
use crate::index::NativeIndexOps;
use crate::prefix::OptionTable;
use tcl_dialect::{StringCharacterModel, TclVersion};

pub(super) const REGSUB_USAGE: &[u8] = b"regsub ?-option ...? exp string subSpec ?varName?";

// C's `options[]` in `Tcl_RegsubObjCmd` (`tclCmdMZ.c`), matched with
// `TCL_EXACT` like `regexp`'s — and here even tclsh's bytecode compiler
// (`TclCompileRegsubCmd`) only fast-paths a literal `-all`, so `regsub` is
// exact-only in every context.
//
// Unlike `regexp`'s, this table *changed shape* at 9.0: TIP #463 inserted
// `-command` after `-all` and shifted `-nocase` into alphabetical order. The
// error noun moved too — `Tcl_GetIndexFromObj`'s `msg` argument is `"switch"`
// through 8.5 and `"option"` from 8.6 (`tclCmdMZ.c:576` / `:481` / `:521`) —
// and both are visible in the message a pre-9.0 `regsub -command` earns:
//
//   tclsh8.4.20 / 8.5.19: bad switch "-command": must be -all, -nocase,
//       -expanded, -line, -linestop, -lineanchor, -start, or --
//   tclsh8.6.18:          bad option "-command": must be -all, -nocase,
//       -expanded, -line, -linestop, -lineanchor, -start, or --
//   tclsh9.0.4 / 9.1b0:   regsub -command {a} abc {string toupper} -> Abc
static REGSUB_NAMES_8: [&str; 8] = [
    "-all",
    "-nocase",
    "-expanded",
    "-line",
    "-linestop",
    "-lineanchor",
    "-start",
    "--",
];

static REGSUB_NAMES_9: [&str; 9] = [
    "-all",
    "-command",
    "-expanded",
    "-line",
    "-linestop",
    "-lineanchor",
    "-nocase",
    "-start",
    "--",
];

static REGSUB_OPTIONS_8_4: OptionTable<'static> =
    OptionTable::exact_only("switch", &REGSUB_NAMES_8);

static REGSUB_OPTIONS_8_6: OptionTable<'static> =
    OptionTable::exact_only("option", &REGSUB_NAMES_8);

static REGSUB_OPTIONS_9_0: OptionTable<'static> =
    OptionTable::exact_only("option", &REGSUB_NAMES_9);

/// The `regsub` option table for `version`.
fn regsub_options(version: TclVersion) -> &'static OptionTable<'static> {
    if version >= TclVersion::V9_0 {
        &REGSUB_OPTIONS_9_0
    } else if version >= TclVersion::V8_6 {
        &REGSUB_OPTIONS_8_6
    } else {
        &REGSUB_OPTIONS_8_4
    }
}

/// A `regsub` failure once a `-command` prefix can be evaluated: either
/// `regsub`'s own diagnostic, or the callback's error passed through.
///
/// The evaluation error stays the adapter's own type (as [`crate::lsort`]'s
/// `sort_command` keeps its comparator's): a script failure carries a Tcl
/// return code, an `errorCode` and an `errorInfo` trailer — C appends
/// `\n    (-command substitution computation script)` to the latter — and none
/// of that is expressible as this module's ready-to-report message bytes.
pub enum RegsubError<Err> {
    /// An option, argument, pattern or command-prefix error from `regsub`.
    Regex(RegexError),
    /// The `-command` prefix's evaluation failed.
    Eval(Err),
}

impl<Err> From<RegexError> for RegsubError<Err> {
    fn from(e: RegexError) -> Self {
        RegsubError::Regex(e)
    }
}

/// Split a `-command` prefix into its words (C's `TclListObjGetElements` on
/// `objv[2]`), rejecting an empty one.
///
/// tclsh 9.0.4 / 9.1b0:
///   % regsub -command {a} abc {}
///   command prefix must be a list of at least one element
fn command_prefix(subspec: &[u8]) -> Result<Vec<Vec<u8>>, RegexError> {
    let text = core::str::from_utf8(subspec)
        .map_err(|_| RegexError::new(b"command prefix must be a valid list".to_vec()))?;
    let words = tcl_syntax::list::split_list(text)
        .map_err(|e| RegexError::new(e.message().as_bytes().to_vec()))?;
    if words.is_empty() {
        return Err(RegexError::new(
            b"command prefix must be a list of at least one element".to_vec(),
        ));
    }
    Ok(words
        .into_iter()
        .map(|w| w.into_owned().into_bytes())
        .collect())
}

/// Scan `regsub`'s leading options against `options`, returning the shared
/// compile/`-all`/`-start` state, whether `-command` was given, and the index
/// of the first non-option argument.
///
/// # Errors
/// A bad option, in `options`' own noun and enumeration.
pub(super) fn regsub_option_scan(
    args: &mut impl OptionArguments,
    version: TclVersion,
) -> Result<(Common, bool, usize), RegexError> {
    let options = regsub_options(version);
    let mut c = Common::for_release(version);
    let mut command = false;
    let mut i = 0;
    while i < args.len() {
        let name = args.bytes(i)?;
        if name.first() != Some(&b'-') {
            break;
        }
        let idx = args.option(i, options)?;
        i += 1;
        // Matched by name, not by table index: the 8.x and 9.x tables list the
        // same options in different orders (see the tables above), so an index
        // means nothing without knowing which one answered.
        if c.flags.apply_compile_option(options.names()[idx]) {
            continue;
        }
        match options.names()[idx] {
            "-all" => c.all = true,
            "-command" => command = true,
            "-start" => match (i < args.len()).then(|| args.bytes(i)).transpose()? {
                Some(v) => {
                    c.start = Some(v);
                    c.start_at = Some(i);
                    i += 1;
                }
                // A trailing `-start` ends the options (C's `goto
                // endOfForLoop`); the caller's arity check then reports it.
                None => break,
            },
            // `--`: explicit end of options.
            _ => break,
        }
    }
    Ok((c, command, i))
}

/// Resolve a regsub start offset before a runtime command callback borrows
/// the adapter. Native engines preserve their exact selected index policy.
///
/// # Errors
/// Invalid options, arguments, or start index.
pub fn regsub_start<O: NativeIndexOps>(
    ops: &mut O,
    args: &[&[u8]],
    version: TclVersion,
) -> Result<usize, RegexError> {
    crate::index::run_index_output(
        ops,
        |ops| {
            let (common, _, offset) = regsub_option_scan(&mut ByteOptionArguments(args), version)?;
            let rest = &args[offset..];
            if rest.len() < 3 || rest.len() > 4 {
                return Err(wrong_args(REGSUB_USAGE));
            }
            let (characters, _) = decode_utf8(rest[1]);
            common.start.as_ref().map_or(Ok(0), |spec| {
                resolve_start_checked(ops, spec, characters.len())
            })
        },
        |_, _| {},
        RegexError::from,
    )
}

/// C-release compatibility adapter. Native engines use [`regsub_start`] and
/// [`regsub_eval_at`] so Jim never inherits a C-version offset parser.
///
/// # Errors
/// Invalid arguments, regex failures, or callback completion.
pub fn regsub_eval<E: RegexEngine, Err>(
    args: &[&[u8]],
    version: TclVersion,
    eval: impl FnMut(&[Vec<u8>]) -> Result<Vec<u8>, Err>,
) -> Result<RegsubResult, RegsubError<Err>>
where
    E::Regex: 'static,
{
    let (common, _, offset) = regsub_option_scan(&mut ByteOptionArguments(args), version)?;
    let rest = &args[offset..];
    if rest.len() < 3 || rest.len() > 4 {
        return Err(wrong_args(REGSUB_USAGE).into());
    }
    let (characters, _) = decode_utf8(rest[1]);
    let start = regsub_start_offset(common.start.as_deref(), characters.len(), version)?;
    regsub_eval_at::<E, Err>(args, version, start, eval)
}

/// Drive `regsub` over the engine `E`, **without** a script evaluator. `args`
/// is the command's arguments without the command name; the result string +
/// count + optional var name are returned for the adapter to apply.
///
/// The option table is 9.0's (this crate's baseline), so `-command` parses —
/// but serving it means running a Tcl command prefix, which this entry point
/// has no way to do. It therefore refuses, and, like C, only once a
/// substitution is actually due: a `-command` call whose pattern never matches
/// still returns the subject unchanged, and an unusable command prefix is still
/// rejected up front, both as tclsh 9.0.4 does. A caller that *can* evaluate —
/// a runtime rather than the registry's const-folder — calls [`regsub_eval`]
/// instead and gets `-command` for real.
///
/// # Errors
/// Option/arg/compile errors as ready-to-report [`RegexError`] messages.
pub fn regsub<E: RegexEngine>(args: &[&[u8]]) -> Result<RegsubResult, RegexError>
where
    E::Regex: 'static,
{
    regsub_eval::<E, RegexError>(args, TclVersion::V9_0, |_| {
        Err(RegexError::new(
            b"regsub -command is not yet supported".to_vec(),
        ))
    })
    .map_err(|e| match e {
        RegsubError::Regex(e) | RegsubError::Eval(e) => e,
    })
}

fn regsub_start_offset(
    spec: Option<&[u8]>,
    char_len: usize,
    version: TclVersion,
) -> Result<usize, RegexError> {
    spec.map_or(Ok(0), |spec| {
        let text = String::from_utf8_lossy(spec);
        let syntax = tcl_dialect::IndexSyntax::for_version(version);
        if syntax.regex_start_grammar() == tcl_dialect::RegexStartGrammar::Integer {
            let flags = tcl_syntax::number::ParseFlags {
                integer_only: true,
                ..tcl_syntax::number::ParseFlags::for_syntax(syntax.numbers)
            };
            if tcl_syntax::number::parse_whole_with(&text, flags).is_none() {
                return Err(RegexError::new(
                    format!("expected integer but got \"{text}\"").into_bytes(),
                ));
            }
        }
        crate::index::resolve_in(&text, char_len, syntax)
            .map(|value| usize::try_from(value).unwrap_or(0))
            .map_err(RegexError::from)
    })
}

/// `regsub` on the analysis path for `version`: compiled through the
/// thread's bounded pattern cache, each search under `analysis`' limits, and
/// a match that established nothing kept typed. A `-command` substitution
/// runs a script this path cannot, so one that is due declines
/// `FormUnsupported` — never the unsubstituted text, which would claim that
/// nothing matched.
///
/// # Errors
/// [`RegexFailure::Error`] for the error the command raises;
/// [`RegexFailure::Declined`] as for [`regexp_analysis`], and for a due
/// `-command` substitution.
pub fn regsub_analysis<E: RegexEngine>(
    args: &[&[u8]],
    version: TclVersion,
    analysis: &mut AnalysisMatch<'_>,
) -> Result<RegsubResult, RegexFailure>
where
    E::Regex: 'static,
{
    regsub_run::<E, RegexFailure>(
        args,
        version,
        |_| {
            Err(RegexFailure::Declined(PrecisionDecline::FormUnsupported {
                option: "regsub -command",
            }))
        },
        &mut Run::Analysis(analysis),
    )
    .map_err(|failure| match failure {
        RunFailure::Regex(error) => RegexFailure::Error(error),
        RunFailure::Eval(failure) => failure,
        RunFailure::Declined(decline) => RegexFailure::Declined(decline),
    })
}

/// A `regsub` run's failure on either path.
pub(super) enum RunFailure<Err> {
    /// `regsub`'s own diagnostic.
    Regex(RegexError),
    /// The `-command` prefix's evaluation failed.
    Eval(Err),
    /// A match established neither a match nor a no-match.
    Declined(PrecisionDecline),
}

impl<Err> From<RegexError> for RunFailure<Err> {
    fn from(error: RegexError) -> Self {
        Self::Regex(error)
    }
}

impl<Err> From<RegexFailure> for RunFailure<Err> {
    fn from(failure: RegexFailure) -> Self {
        match failure {
            RegexFailure::Error(error) => Self::Regex(error),
            RegexFailure::Declined(decline) => Self::Declined(decline),
        }
    }
}

/// Drive `regsub` over the engine `E` for `version`, evaluating a `-command`
/// prefix through `eval`.
///
/// `eval` receives the whole command word list — the prefix's own words
/// followed by the matched text and each submatch (a non-participating
/// submatch is the empty string) — exactly as C hands it to `Tcl_EvalObjv`,
/// and returns the replacement text. It is called once per substitution, so a
/// `-all` run calls it per match; a pattern that never matches never calls it.
///
/// `version` selects C's option table and the noun its errors use, so
/// `-command` is refused before 9.0 in that release's own words rather than
/// served or refused generically.
///
/// # Errors
/// [`RegsubError::Regex`] for `regsub`'s own diagnostics, [`RegsubError::Eval`]
/// for a failing command prefix.
pub fn regsub_eval_at<E: RegexEngine, Err>(
    args: &[&[u8]],
    version: TclVersion,
    start: usize,
    eval: impl FnMut(&[Vec<u8>]) -> Result<Vec<u8>, Err>,
) -> Result<RegsubResult, RegsubError<Err>>
where
    E::Regex: 'static,
{
    let (c, command, i) = regsub_option_scan(&mut ByteOptionArguments(args), version)?;
    regsub_selected::<E, Err>(
        &args[i..],
        version,
        start,
        &c,
        command,
        eval,
        &mut Run::Runtime,
    )
    .map_err(|failure| match failure {
        RunFailure::Regex(error) => RegsubError::Regex(error),
        RunFailure::Eval(error) => RegsubError::Eval(error),
        RunFailure::Declined(decline) => RegsubError::Regex(decline.into_error()),
    })
}

fn regsub_run<E: RegexEngine, Err>(
    args: &[&[u8]],
    version: TclVersion,
    eval: impl FnMut(&[Vec<u8>]) -> Result<Vec<u8>, Err>,
    run: &mut Run<'_, '_>,
) -> Result<RegsubResult, RunFailure<Err>>
where
    E::Regex: 'static,
{
    let (c, command, i) = regsub_option_scan(&mut ByteOptionArguments(args), version)?;
    let rest = &args[i..];
    if rest.len() < 3 || rest.len() > 4 {
        return Err(wrong_args(REGSUB_USAGE).into());
    }
    let (characters, _) = decode_utf8(rest[1]);
    let start = regsub_start_offset(c.start.as_deref(), characters.len(), version)?;
    regsub_selected::<E, Err>(rest, version, start, &c, command, eval, run)
}

pub(super) fn regsub_selected<E: RegexEngine, Err>(
    rest: &[&[u8]],
    version: TclVersion,
    start: usize,
    c: &Common,
    command: bool,
    mut eval: impl FnMut(&[Vec<u8>]) -> Result<Vec<u8>, Err>,
    run: &mut Run<'_, '_>,
) -> Result<RegsubResult, RunFailure<Err>>
where
    E::Regex: 'static,
{
    if rest.len() < 3 || rest.len() > 4 {
        return Err(wrong_args(REGSUB_USAGE).into());
    }
    let pattern = rest[0];
    let str_bytes = rest[1];
    let subspec = rest[2];
    let var = rest.get(3).map(|&v| v.to_vec());

    // C splits and checks the command prefix *before* the match loop, so an
    // unusable prefix is an error even when the pattern never matches (tclsh
    // 9.0.4: `regsub -command {z} abc {}` is still `command prefix must be a
    // list of at least one element`).
    let prefix = if command {
        Some(command_prefix(subspec)?)
    } else {
        None
    };

    let (cps, byteoff) = decode_utf8(str_bytes);
    let char_len = cps.len();

    let re = run.compile::<E>(pattern, c.flags, version)?;
    let nsubs = E::nsub(&re.borrow());

    let mut offset = start;

    // The **literal empty pattern** is not the general empty-match case. C
    // diverts a metacharacter-free pattern away from the RE engine into a
    // literal string map (`Tcl_RegsubObjCmd`'s "simple one pair string map
    // situation") and spells the empty pattern out as its own branch of it
    // (`if (slen == 0)`): the replacement goes before each character and never
    // at end-of-string, so the count is exactly the subject's character length
    // and an empty subject substitutes nothing at all.
    //
    // That is a `regsub` rule, not an engine rule, which is why it sits in
    // `regsub`'s own loop rather than in the `RegexEngine` the two commands
    // share. An RE that merely *can* match empty keeps the general rule
    // (`regsub -all {(?:)} abc X` is still 4 / `XaXbXcX`), and `regexp -all`
    // never sees this branch at all — it counts 3 for both patterns and still
    // reports one match on an empty subject where `regsub` reports none. The
    // two commands disagree at end-of-string on purpose.
    //
    // The guard is C's, term for term: `-all`, a resolved start of 0, no
    // `-command`, and a subspec free of `&` and `\` — either of those sends C
    // down the general path instead, which is why `regsub -all {} abc &`
    // counts 4. C's remaining term, "the pattern holds none of
    // `*+?{}()[].\|^$`", is implied by the pattern being empty. Measured
    // identical on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0.
    //
    // The characters are the release's (#2170): the count is `string length`
    // under its `StringCharacterModel`, and the replacement goes before each
    // of the model's units. Measured with `regsub -all {} $s X out` over a
    // literal U+1D11E, `LANG=C.UTF-8`, `out` read back with `scan … %c`:
    //
    //   8.4.20, 8.5.19   4   X 240 X 157 X 132 X 158
    //   8.6.18           2   X 55348 X 56606
    //   9.0.4, 9.1.0     1   X 119070
    //
    // 8.4/8.5 never assemble the scalar, so its units are its four UTF-8
    // bytes, each a character of its own, and the output holds them as such.
    // 8.6 puts its second replacement between the surrogate halves, which
    // takes a lone surrogate no Rust string holds; both replacements go
    // before the whole pair instead (`XX<U+1D11E>`), which keeps the count,
    // the length, every replacement ahead of its unit and nothing after the
    // last one (`semantic-aot-optimisation.md` § *Tcl 8
    // supplementary-character boundary*).
    if c.all
        && offset == 0
        && !command
        && pattern.is_empty()
        && !subspec.iter().any(|&b| b == b'&' || b == b'\\')
    {
        let (text, count) = match std::str::from_utf8(str_bytes) {
            Ok(text) => {
                substitute_before_each_unit(subspec, text, version.string_character_model())
            }
            // Bytes no Rust string holds: one replacement per decoded position.
            Err(_) => (
                substitute_before_each_character(subspec, str_bytes, &byteoff),
                char_len,
            ),
        };
        return Ok(RegsubResult {
            text,
            count: i64::try_from(count).unwrap_or(i64::MAX),
            var,
        });
    }

    let mut result: Vec<u8> = Vec::new();
    let mut count: i64 = 0;

    while offset <= char_len {
        let notbol = offset > 0 && cps[offset - 1] != i32::from(b'\n');
        let answer = run.exec::<E>(&re, &cps, offset, notbol);
        if let RegexpPrecision::Declined(decline) = answer {
            // A search cut short is not a no-match: the unsubstituted text
            // would claim nothing matched.
            return Err(RunFailure::Declined(decline));
        }
        let Some(matches) = answer.match_vector() else {
            break;
        };
        if count == 0 && offset > 0 {
            // Copy the skipped prefix when a `-start` offset was given.
            result.extend_from_slice(&str_bytes[..byteoff[offset]]);
        }
        count += 1;

        let m0 = matches[0];
        // Text before this match.
        result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[m0.so]]);
        if let Some(prefix) = prefix.as_ref() {
            // `-command`: the prefix's words, then the whole match and each
            // submatch as further words, evaluated as one command whose result
            // is the replacement (C builds the same word list for
            // `Tcl_EvalObjv`).
            let mut words = prefix.clone();
            words.extend((0..=nsubs).map(|k| match matches.get(k) {
                Some(rm) if rm.so != NO_MATCH => {
                    slice_match(str_bytes, &byteoff, rm.so, rm.eo).to_vec()
                }
                // A submatch that did not participate is an empty word, not a
                // missing one (tclsh 9.0.4: `regsub -command {(a)|(b)} ab cap`
                // passes `a a {}`).
                _ => Vec::new(),
            }));
            result.extend_from_slice(&eval(&words).map_err(RunFailure::Eval)?);
        } else {
            // The substitution spec, with `&`/`\N` expanded.
            apply_subspec(&mut result, subspec, &matches, nsubs, str_bytes, &byteoff);
        }

        // Advance, always consuming at least one char on an empty match.
        if m0.eo == offset {
            if offset < char_len {
                result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[offset + 1]]);
            }
            offset += 1;
        } else {
            offset = m0.eo;
            if m0.so == m0.eo {
                if offset < char_len {
                    result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[offset + 1]]);
                }
                offset += 1;
            }
        }
        if !c.all {
            break;
        }
    }

    let text = if count == 0 {
        str_bytes.to_vec()
    } else {
        if offset < char_len {
            result.extend_from_slice(&str_bytes[byteoff[offset]..]);
        }
        result
    };

    Ok(RegsubResult { text, count, var })
}

/// The literal empty pattern's substitution: `subspec` before each of
/// `model`'s units of `text`, never at end-of-string (see `regsub_run`), and
/// the number of units.
fn substitute_before_each_unit(
    subspec: &[u8],
    text: &str,
    model: StringCharacterModel,
) -> (Vec<u8>, usize) {
    // Saturating, as below. A unit is at most one byte of `text`, and 8.4/8.5
    // spell a byte of a supplementary scalar as a two-byte character.
    let hint = subspec.len().saturating_add(2).saturating_mul(text.len());
    let mut out = Vec::with_capacity(hint);
    let mut count = 0usize;
    let mut held = [0u8; 4];
    for run in model.units(text) {
        for _ in 0..run.units {
            out.extend_from_slice(subspec);
        }
        count += run.units;
        out.extend_from_slice(run.held.encode_utf8(&mut held).as_bytes());
    }
    (out, count)
}

/// The literal empty pattern's substitution over bytes that are not UTF-8:
/// `subspec` before each decoded position of `str_bytes`, never at
/// end-of-string.
fn substitute_before_each_character(
    subspec: &[u8],
    str_bytes: &[u8],
    byteoff: &[usize],
) -> Vec<u8> {
    let char_len = byteoff.len().saturating_sub(1);
    // Saturating: on a 32-bit target (the WASM runtime) a long subject times
    // a long replacement can overflow `usize`, and a capacity hint must never
    // be the thing that aborts. Too small only costs a regrow.
    let hint = subspec
        .len()
        .saturating_mul(char_len)
        .saturating_add(str_bytes.len());
    let mut text = Vec::with_capacity(hint);
    for i in 0..char_len {
        text.extend_from_slice(subspec);
        text.extend_from_slice(&str_bytes[byteoff[i]..byteoff[i + 1]]);
    }
    text
}

/// Expand a `regsub` substitution spec into `out`: `&` / `\0` → whole match,
/// `\N` → capture group N, `\\` → `\`, `\&` → `&`; any other run is copied
/// verbatim. Mirrors the `wsubspec` scan in `Tcl_RegsubObjCmd`.
fn apply_subspec(
    out: &mut Vec<u8>,
    sub: &[u8],
    matches: &[RegMatch],
    nsubs: usize,
    str_bytes: &[u8],
    byteoff: &[usize],
) {
    for (_, part) in super::replacement_parts(sub) {
        match part {
            super::ReplacementPart::Literal(units) => out.extend_from_slice(units),
            super::ReplacementPart::Capture(group) => {
                let index = usize::from(group);
                if index <= nsubs
                    && let Some(rm) = matches.get(index)
                    && rm.so != NO_MATCH
                {
                    out.extend_from_slice(slice_match(str_bytes, byteoff, rm.so, rm.eo));
                }
            }
        }
    }
}
