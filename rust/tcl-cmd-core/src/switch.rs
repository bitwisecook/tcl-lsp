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

//! `switch` option parsing + pattern selection, shared across runtimes.
//!
//! `switch` is a stateful command (it evaluates a body script), so — like
//! `lsort -command` — only its **decision** logic is shared here: the option
//! table (`-exact`/`-glob`/`-regexp`/`-nocase`/`-indexvar`/`-matchvar`/`--`, plus
//! Tcl 9.1's `-integer`), the value/pattern selection across the four match modes
//! (incl. `default`), and the TIP #75 `-matchvar`/`-indexvar` value construction.
//! Each runtime keeps the per-target parts: extracting the pattern/body pairs
//! (the inline vs. brace-list forms, with the runtime's `info frame` line
//! tracking), resolving a `-` fall-through body, the trace-aware variable
//! writes, and evaluating the chosen body as a transparent script.
//!
//! Mirrors C's `TclNRSwitchObjCmd` (`tclCmdMZ.c`). The regexp mode drives the
//! shared [`RegexEngine`](crate::regex::RegexEngine) provider, exactly as
//! `regexp`/`regsub` do.

// `ops`/`opts` and the regexp `so`/`eo` offsets are deliberately terse mirrors of
// the C names and recur across the module's functions, so the similar-names allow
// stays module-scoped. (The regexp offset cast is narrowed to `regexp_writes`.)
#![allow(clippy::similar_names)]

use tcl_dialect::TclVersion;
use tcl_syntax::expr::errors::{IOVERFLOW_CODE, IOVERFLOW_MESSAGE};
use tcl_syntax::glob::string_case_match;
use tcl_syntax::number::{Number, ParseFlags, parse_whole_with};
use tcl_syntax::value::{ValueError, ValueOps};

use crate::error::CmdError;
use crate::prefix::OptionTable;
use crate::regex::{
    AnalysisMatch, NO_MATCH, RegMatch, RegexEngine, RegexFailure, RegexFlags, Run, decode_utf8,
};

/// The matching mode (`-exact` is the default).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Exact string equality.
    Exact,
    /// Glob (`string match`) patterns.
    Glob,
    /// Tcl ARE regular expressions.
    Regexp,
    /// Numeric equality of wide integers (TIP 730, Tcl 9.1+).
    Integer,
}

impl Mode {
    /// The canonical option naming this mode (C's `options[mode]`).
    #[must_use]
    pub const fn option_name(self) -> &'static str {
        match self {
            Mode::Exact => "-exact",
            Mode::Glob => "-glob",
            Mode::Regexp => "-regexp",
            Mode::Integer => "-integer",
        }
    }
}

/// The parsed option state plus the index of the `string` argument.
pub struct Options<V> {
    /// The match mode.
    pub mode: Mode,
    /// `-nocase`.
    pub nocase: bool,
    /// TIP #75 `-matchvar` target (the matched-substring list; regexp only).
    pub match_var: Option<V>,
    /// TIP #75 `-indexvar` target (the `{start end}` pair list; regexp only).
    pub index_var: Option<V>,
    /// Index, in the name-stripped args, of the word naming the `-matchvar`
    /// target — the place a caller's write lands on.
    pub match_var_at: Option<usize>,
    /// Index, in the name-stripped args, of the word naming the `-indexvar`
    /// target.
    pub index_var_at: Option<usize>,
    /// Index, in the name-stripped args, of the `string` to switch on.
    pub value_index: usize,
}

// The option tables mirror C's `options[]`, whose order is the "bad option"
// enumeration. Tcl 9.1 inserts `-integer` (TIP 730), which also makes `-i`/`-in`
// ambiguous where 9.0 resolved them to `-indexvar`.
const OPT_NAMES: [&str; 7] = [
    "-exact",
    "-glob",
    "-indexvar",
    "-matchvar",
    "-nocase",
    "-regexp",
    "--",
];
const OPT_NAMES_9_1: [&str; 8] = [
    "-exact",
    "-glob",
    "-indexvar",
    "-integer",
    "-matchvar",
    "-nocase",
    "-regexp",
    "--",
];
// C resolves switch options with abbreviations allowed (flags 0), so `-gl`,
// `-noc`, … work like tclsh.
const OPTIONS: OptionTable<'static> = OptionTable::abbreviating("option", &OPT_NAMES);
const OPTIONS_9_1: OptionTable<'static> = OptionTable::abbreviating("option", &OPT_NAMES_9_1);

/// The `switch` option table for `version`.
fn options(version: TclVersion) -> &'static OptionTable<'static> {
    if version >= TclVersion::V9_1 {
        &OPTIONS_9_1
    } else {
        &OPTIONS
    }
}

const USAGE_INLINE: &str = "switch ?-option ...? string ?pattern body ...? ?default body?";

/// Parse the leading options of `args` (the name-stripped argv: any options, then
/// the `string`, then the pattern/body pairs or the single brace-list). Returns
/// the option state and the index of the `string` argument. Mirrors C's option
/// scan, whose bound leaves the string plus at least one pattern/body word.
///
/// # Errors
/// A bad/ambiguous option, a repeated mode option, a missing `-matchvar`/
/// `-indexvar` argument, `-matchvar`/`-indexvar` without `-regexp`, `-nocase`
/// with `-integer`, or too few arguments after the options.
pub fn parse_options<O, V>(
    ops: &mut O,
    args: &[V],
    version: TclVersion,
) -> Result<Options<V>, CmdError>
where
    O: ValueOps<Value = V>,
    V: Clone,
{
    let table = options(version);
    let objc = args.len();
    let mut mode = Mode::Exact;
    let mut found_mode = false;
    let mut nocase = false;
    let mut match_var: Option<V> = None;
    let mut index_var: Option<V> = None;
    let mut match_var_at: Option<usize> = None;
    let mut index_var_at: Option<usize> = None;

    let mut i = 0;
    // Leave the string plus at least one pattern/body word unparsed; `--` ends it.
    while i + 2 < objc {
        let arg = ops.as_str(&args[i]);
        if !arg.starts_with('-') {
            break;
        }
        let name = table.names()[table.index_of_str(&arg)?];
        let picked = match name {
            "--" => {
                i += 1;
                break;
            }
            "-nocase" => {
                nocase = true;
                None
            }
            "-indexvar" | "-matchvar" => {
                i += 1;
                if i + 2 > objc {
                    return Err(CmdError::with_error_code(
                        format!("missing variable name argument to {name} option"),
                        "TCL OPERATION SWITCH NOVAR",
                    ));
                }
                if name == "-indexvar" {
                    index_var = Some(args[i].clone());
                    index_var_at = Some(i);
                } else {
                    match_var = Some(args[i].clone());
                    match_var_at = Some(i);
                }
                None
            }
            "-glob" => Some(Mode::Glob),
            "-regexp" => Some(Mode::Regexp),
            "-integer" => Some(Mode::Integer),
            _ => Some(Mode::Exact),
        };
        if let Some(m) = picked {
            if found_mode {
                return Err(double_option(&arg, mode.option_name()));
            }
            found_mode = true;
            mode = m;
        }
        i += 1;
    }

    if i + 2 > objc {
        return Err(CmdError::wrong_args(USAGE_INLINE));
    }
    if index_var.is_some() && mode != Mode::Regexp {
        return Err(mode_restriction("-indexvar option requires -regexp option"));
    }
    if match_var.is_some() && mode != Mode::Regexp {
        return Err(mode_restriction("-matchvar option requires -regexp option"));
    }
    if nocase && mode == Mode::Integer {
        return Err(mode_restriction(
            "-nocase option cannot be used with -integer option",
        ));
    }

    Ok(Options {
        mode,
        nocase,
        match_var,
        index_var,
        match_var_at,
        index_var_at,
        value_index: i,
    })
}

/// The outcome of [`select`].
pub enum Selection<V> {
    /// No pattern matched — the command result is the empty string.
    NoMatch,
    /// `patterns[index]` matched. `writes` are the TIP #75 `-matchvar`/`-indexvar`
    /// variable writes (each `(name, value)`, in C's write order: index then
    /// match) the adapter must apply **before** evaluating the body; it is empty
    /// unless regexp mode set those options. `index` is the matched pattern before
    /// any `-` fall-through (which the adapter resolves, since it owns the bodies).
    Matched {
        /// The matched pattern's index.
        index: usize,
        /// The `(var-name, value)` writes to apply before the body runs.
        writes: Vec<(V, V)>,
    },
}

/// Find the pattern that matches `value`, across the option's match mode. A
/// `default` pattern matches anything but only as the final pattern (otherwise it
/// is an ordinary literal pattern). The regexp mode compiles + executes each
/// pattern through the `E` provider and, on a match, builds the `-matchvar`/
/// `-indexvar` values.
///
/// The `-integer` mode coerces the value up front and each pattern as it is
/// reached, so a non-integer pattern after the matching one goes unchecked.
///
/// # Errors
/// A malformed `-regexp` pattern (the engine's compile error), under
/// `-integer` a value or reached pattern that is not a wide integer, and a
/// search that established neither a match nor a no-match, raised as the
/// error it is — never read as a pattern that did not match.
pub fn select<O, E, V>(
    ops: &mut O,
    opts: &Options<V>,
    value: &V,
    patterns: &[V],
    version: TclVersion,
) -> Result<Selection<V>, CmdError>
where
    O: ValueOps<Value = V>,
    E: RegexEngine,
    E::Regex: 'static,
    V: Clone,
{
    select_run::<O, E, V>(ops, opts, value, patterns, version, &mut Run::Runtime).map_err(
        |failure| match failure {
            SelectFailure::Error(error) => error,
            SelectFailure::Regex(failure) => {
                CmdError::new(String::from_utf8_lossy(&failure.into_error().0).into_owned())
            }
        },
    )
}

/// [`select`] on the analysis path: a `-regexp` pattern compiled through the
/// thread's bounded pattern cache, each search run under `analysis`' limits,
/// and a search that established neither a match nor a no-match — or matched
/// with an approximate span — kept typed as [`RegexFailure::Declined`] rather
/// than raised. Only a completed match or a completed no-match selects, so an
/// arm is never chosen, or passed over, on a search that was cut short.
///
/// # Errors
/// [`RegexFailure::Error`] for the error the selection raises;
/// [`RegexFailure::Declined`] for a pattern that does not compile, a refused
/// compile charge, an exhausted or cancelled search, or an approximate span.
pub fn select_analysis<O, E, V>(
    ops: &mut O,
    opts: &Options<V>,
    value: &V,
    patterns: &[V],
    version: TclVersion,
    analysis: &mut AnalysisMatch<'_>,
) -> Result<Selection<V>, RegexFailure>
where
    O: ValueOps<Value = V>,
    E: RegexEngine,
    E::Regex: 'static,
    V: Clone,
{
    select_run::<O, E, V>(
        ops,
        opts,
        value,
        patterns,
        version,
        &mut Run::Analysis(analysis),
    )
    .map_err(|failure| match failure {
        SelectFailure::Error(error) => {
            RegexFailure::Error(crate::regex::RegexError(error.into_message().into_bytes()))
        }
        SelectFailure::Regex(failure) => failure,
    })
}

/// A selection that did not complete: the error `switch` raises with its
/// own error code (an `-integer` operand that is not a wide integer), or a
/// `-regexp` pattern's failure on either path.
enum SelectFailure {
    Error(CmdError),
    Regex(RegexFailure),
}

impl From<CmdError> for SelectFailure {
    fn from(error: CmdError) -> Self {
        Self::Error(error)
    }
}

impl From<RegexFailure> for SelectFailure {
    fn from(failure: RegexFailure) -> Self {
        Self::Regex(failure)
    }
}

/// The one selection algorithm both paths run.
fn select_run<O, E, V>(
    ops: &mut O,
    opts: &Options<V>,
    value: &V,
    patterns: &[V],
    version: TclVersion,
    run: &mut Run<'_, '_>,
) -> Result<Selection<V>, SelectFailure>
where
    O: ValueOps<Value = V>,
    E: RegexEngine,
    E::Regex: 'static,
    V: Clone,
{
    let npairs = patterns.len();
    // No patterns ⇒ nothing can match. Guard here so the `npairs - 1` below
    // (the "is this the final, `default`-eligible pattern?" test) can't underflow
    // on an empty list and panic; an empty `patterns` simply has no match.
    if npairs == 0 {
        return Ok(Selection::NoMatch);
    }
    let val_str = ops.as_str(value);
    let val_int = if opts.mode == Mode::Integer {
        Some(wide_int(&val_str, version)?)
    } else {
        None
    };
    for (p, pat_val) in patterns.iter().enumerate() {
        let pat = ops.as_str(pat_val);
        // `default` matches anything, but only as the final pattern.
        if p == npairs - 1 && &*pat == "default" {
            let writes = default_writes(ops, opts);
            return Ok(Selection::Matched { index: p, writes });
        }
        match opts.mode {
            Mode::Exact => {
                // C's `-exact -nocase` arm is `TclUtfCasecmp` (`tclCmdMZ.c`'s
                // `Tcl_SwitchObjCmd`), a full-range `Tcl_UniCharToLower` fold —
                // not an ASCII one. tclsh 8.5.19/8.6.18/9.0.4/9.1b0 all select
                // the arm for `switch -nocase -- \u00e9 {\u00c9 {…}}`, and for
                // `İ` against `i`; an ASCII fold matches neither (#2125).
                let hit = if opts.nocase {
                    crate::string::fold_lower_bytes(pat.as_bytes())
                        == crate::string::fold_lower_bytes(val_str.as_bytes())
                } else {
                    *pat == *val_str
                };
                if hit {
                    return Ok(Selection::Matched {
                        index: p,
                        writes: Vec::new(),
                    });
                }
            }
            Mode::Glob => {
                if string_case_match(&pat, &val_str, opts.nocase) {
                    return Ok(Selection::Matched {
                        index: p,
                        writes: Vec::new(),
                    });
                }
            }
            Mode::Regexp => {
                let flags = RegexFlags {
                    nocase: opts.nocase,
                    ..RegexFlags::for_release(version)
                };
                let re = run.compile::<E>(pat.as_bytes(), flags, version)?;
                let value_bytes = ops.as_bytes(value);
                let (cps, byteoff) = decode_utf8(&value_bytes);
                let answer = run.exec::<E>(&re, &cps, 0, false);
                if let crate::regex::RegexpPrecision::Declined(decline) = answer {
                    // A search cut short selects no arm: the runtime raises
                    // it and the analysis path declines, never reading it
                    // as a pattern that did not match.
                    return Err(RegexFailure::Declined(decline).into());
                }
                if let Some(m) = answer.match_vector() {
                    let writes = regexp_writes(ops, opts, &m, &value_bytes, &byteoff);
                    return Ok(Selection::Matched { index: p, writes });
                }
            }
            Mode::Integer => {
                if val_int == Some(wide_int(&pat, version)?) {
                    return Ok(Selection::Matched {
                        index: p,
                        writes: Vec::new(),
                    });
                }
            }
        }
    }
    Ok(Selection::NoMatch)
}

/// `extra switch pattern with no body` — an odd number of pattern/body words.
/// `comment_hint` appends the "misplaced comment" note (a braced body whose
/// first word begins with `#`), matching C.
#[must_use]
pub fn extra_pattern_error(comment_hint: bool) -> CmdError {
    let mut m = String::from("extra switch pattern with no body");
    if comment_hint {
        m.push_str(
            ", this may be due to a comment incorrectly placed outside of a \
             switch body - see the \"switch\" documentation",
        );
    }
    CmdError::new(m)
}

/// `no body specified for pattern "P"` — a trailing `-` fall-through body has no
/// real body to fall through to.
#[must_use]
pub fn no_body_error(pattern: &str) -> CmdError {
    CmdError::new(format!("no body specified for pattern \"{pattern}\""))
}

/// The `-matchvar`/`-indexvar` writes for the `default` arm (TIP #75: the targets
/// become empty values), in C's order: index variable first, then match variable.
fn default_writes<O, V>(ops: &mut O, opts: &Options<V>) -> Vec<(V, V)>
where
    O: ValueOps<Value = V>,
    V: Clone,
{
    let mut writes = Vec::new();
    if let Some(iv) = &opts.index_var {
        let v = iv.clone();
        writes.push((v, ops.empty()));
    }
    if let Some(mv) = &opts.match_var {
        let v = mv.clone();
        writes.push((v, ops.empty()));
    }
    writes
}

/// Build the `-indexvar` (`{start end}` pairs) and `-matchvar` (substring list)
/// values for a regexp match. Mirrors C's `matchFoundRegexp`: a non-participating
/// or start-anchored empty group yields `{-1 -1}` / the empty string.
// `so`/`eo` are codepoint offsets bounded by the subject length; the `i64` is the
// `{start end}` pair format, like the `regexp` core's `-indices`.
#[allow(clippy::cast_possible_wrap)]
fn regexp_writes<O, V>(
    ops: &mut O,
    opts: &Options<V>,
    m: &[RegMatch],
    value_bytes: &[u8],
    byteoff: &[usize],
) -> Vec<(V, V)>
where
    O: ValueOps<Value = V>,
    V: Clone,
{
    let mut writes = Vec::new();
    if let Some(iv) = &opts.index_var {
        let name = iv.clone();
        let mut pairs: Vec<V> = Vec::with_capacity(m.len());
        for mm in m {
            let (a, b) = if mm.so != NO_MATCH && mm.eo > 0 {
                (mm.so as i64, mm.eo as i64 - 1)
            } else {
                (-1, -1)
            };
            let lo = ops.new_int(a);
            let hi = ops.new_int(b);
            pairs.push(ops.new_list(vec![lo, hi]));
        }
        let list = ops.new_list(pairs);
        writes.push((name, list));
    }
    if let Some(mv) = &opts.match_var {
        let name = mv.clone();
        let mut subs: Vec<V> = Vec::with_capacity(m.len());
        for mm in m {
            let v = if mm.so != NO_MATCH && mm.eo > 0 {
                ops.new_bytes(&value_bytes[byteoff[mm.so]..byteoff[mm.eo]])
            } else {
                ops.empty()
            };
            subs.push(v);
        }
        let list = ops.new_list(subs);
        writes.push((name, list));
    }
    writes
}

// the switch-specific error catalogue (option resolution is the shared
// `crate::prefix` wrapper over `crate::prefix`, so `switch` cannot
// drift from the other option tables)

fn double_option(arg: &str, found_name: &str) -> CmdError {
    CmdError::with_error_code(
        format!("bad option \"{arg}\": {found_name} option already found"),
        "TCL OPERATION SWITCH DOUBLEOPT",
    )
}

fn mode_restriction(message: &str) -> CmdError {
    CmdError::with_error_code(message, "TCL OPERATION SWITCH MODERESTRICTION")
}

/// `Tcl_GetWideIntFromObj` for the `-integer` mode: the release's integer
/// grammar, with an out-of-range integer reported as an overflow rather than a
/// non-integer (C refuses bignums here; it never compares them).
fn wide_int(text: &str, version: TclVersion) -> Result<i64, CmdError> {
    let flags = ParseFlags {
        integer_only: true,
        ..ParseFlags::for_syntax(version.number_syntax())
    };
    match parse_whole_with(text, flags) {
        Some(Number::Int(n)) => Ok(n),
        Some(Number::Big { .. }) => {
            Err(CmdError::with_error_code(IOVERFLOW_MESSAGE, IOVERFLOW_CODE))
        }
        _ => Err(CmdError::with_error_code(
            ValueError::NotInteger(text.to_owned()).message(),
            "TCL VALUE NUMBER",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_table_matches_exact_and_unique_prefix() {
        // Tcl unambiguous-prefix option matching over the shared matcher.
        // OPT_NAMES = -exact -glob -indexvar -matchvar -nocase -regexp --.
        use crate::prefix::Resolution;
        assert_eq!(OPTIONS.resolve(b"-exact"), Resolution::Exact(0));
        assert_eq!(OPTIONS.resolve(b"-glob"), Resolution::Exact(1));
        assert_eq!(OPTIONS.resolve(b"-e"), Resolution::UniquePrefix(0));
        assert_eq!(OPTIONS.resolve(b"-g"), Resolution::UniquePrefix(1));
        assert_eq!(OPTIONS.resolve(b"-i"), Resolution::UniquePrefix(2));
        assert_eq!(OPTIONS.resolve(b"-m"), Resolution::UniquePrefix(3));
        assert_eq!(OPTIONS.resolve(b"-n"), Resolution::UniquePrefix(4));
        assert_eq!(OPTIONS.resolve(b"-"), Resolution::Ambiguous); // prefixes all
        assert_eq!(OPTIONS.resolve(b"-zzz"), Resolution::NoMatch);
        // The generated errors keep switch's exact tclsh message text.
        let Err(e) = OPTIONS.index_of_str("-badopt") else {
            panic!("-badopt must not resolve");
        };
        assert_eq!(
            e.message(),
            "bad option \"-badopt\": must be -exact, -glob, -indexvar, \
             -matchvar, -nocase, -regexp, or --"
        );
    }

    #[test]
    fn switch_error_message_formats() {
        assert_eq!(
            extra_pattern_error(false).message(),
            "extra switch pattern with no body"
        );
        assert!(
            extra_pattern_error(true)
                .message()
                .contains("comment incorrectly placed")
        );
        assert_eq!(
            no_body_error("foo").message(),
            r#"no body specified for pattern "foo""#
        );
    }

    /// A throwaway string-only `ValueOps` for the selection tests: just enough of
    /// the seam to drive `select` (which here only needs `as_str` on the value and
    /// patterns). Numeric/list methods are present to satisfy the trait but the
    /// `select` paths under test never reach them.
    #[derive(Default)]
    struct StrOps;

    impl ValueOps for StrOps {
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
        fn as_str(&mut self, v: &String) -> std::rc::Rc<str> {
            std::rc::Rc::from(v.as_str())
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

    /// A never-invoked `RegexEngine`: the selection tests below stay in exact/glob
    /// mode (and the empty-patterns case returns before any engine call), so these
    /// methods are unreachable.
    enum NoEngine {}

    impl RegexEngine for NoEngine {
        type Regex = ();
        fn compile(_pattern: &[u8], _flags: RegexFlags) -> Result<(), Vec<u8>> {
            unreachable!("regexp engine not used in these tests")
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
            name: "none",
            revision: 0,
        };
    }

    /// An engine whose every pattern fails to compile.
    enum RejectingEngine {}

    impl RegexEngine for RejectingEngine {
        type Regex = ();
        fn compile(_pattern: &[u8], _flags: RegexFlags) -> Result<(), Vec<u8>> {
            Err(b"invalid escape \\ sequence".to_vec())
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
        //   % switch -regexp xa {{a\q} {}}
        //   couldn't compile regular expression pattern: invalid escape \ sequence
        let opts = Options {
            mode: Mode::Regexp,
            ..exact_opts()
        };
        let value = String::from("xa");
        let pats = vec![String::from(r"a\q")];
        for (version, verb) in [
            (TclVersion::V8_4, "couldn't"),
            (TclVersion::V8_5, "couldn't"),
            (TclVersion::V8_6, "couldn't"),
            (TclVersion::V9_0, "cannot"),
            (TclVersion::V9_1, "cannot"),
        ] {
            let Err(e) =
                select::<_, RejectingEngine, _>(&mut StrOps, &opts, &value, &pats, version)
            else {
                panic!("{version:?}: a bad pattern must not compile")
            };
            assert_eq!(
                e.message(),
                format!(r"{verb} compile regular expression pattern: invalid escape \ sequence"),
                "{version:?}"
            );
        }
    }

    fn exact_opts() -> Options<String> {
        Options {
            mode: Mode::Exact,
            nocase: false,
            match_var: None,
            index_var: None,
            match_var_at: None,
            index_var_at: None,
            value_index: 0,
        }
    }

    #[test]
    fn select_empty_patterns_is_no_match_not_underflow() {
        // An empty `patterns` list must not underflow `npairs - 1` (the
        // final-pattern `default` test) and panic — it simply matches nothing.
        let mut ops = StrOps;
        let opts = exact_opts();
        let value = String::from("anything");
        let result =
            select::<_, NoEngine, _>(&mut ops, &opts, &value, &[], TclVersion::V9_0).unwrap();
        assert!(matches!(result, Selection::NoMatch));
    }

    #[test]
    fn select_exact_still_matches_after_guard() {
        // The empty-list early-return must not disturb normal selection. A
        // trailing `default` and a literal hit both still work.
        let mut ops = StrOps;
        let opts = exact_opts();
        let value = String::from("b");
        let pats = vec![
            String::from("a"),
            String::from("b"),
            String::from("default"),
        ];
        match select::<_, NoEngine, _>(&mut ops, &opts, &value, &pats, TclVersion::V9_0).unwrap() {
            Selection::Matched { index, .. } => assert_eq!(index, 1),
            Selection::NoMatch => panic!("expected a match"),
        }
        // No literal hit ⇒ the final `default` matches.
        let value = String::from("zzz");
        match select::<_, NoEngine, _>(&mut ops, &opts, &value, &pats, TclVersion::V9_0).unwrap() {
            Selection::Matched { index, .. } => assert_eq!(index, 2),
            Selection::NoMatch => panic!("expected default to match"),
        }
    }

    #[test]
    fn switch_nocase_exact_folds_the_full_unicode_range() {
        // Regression (#2125): the `-exact -nocase` arm folded with
        // `eq_ignore_ascii_case`, so any non-ASCII letter failed to match.
        // C folds with `Tcl_UniCharToLower` over the whole range — tclsh
        // 8.5.19, 8.6.18, 9.0.4 and 9.1b0 all agree:
        //   % switch -nocase -- é {É {return arm} default {return none}}
        //   arm
        //   % switch -nocase -- i {İ {return arm} default {return none}}
        //   arm
        let mut ops = StrOps;
        let opts = Options {
            nocase: true,
            ..exact_opts()
        };
        let hit = |ops: &mut StrOps, value: &str, pat: &str| {
            let pats = vec![pat.to_owned()];
            matches!(
                select::<_, NoEngine, _>(ops, &opts, &value.to_owned(), &pats, TclVersion::V9_0)
                    .unwrap(),
                Selection::Matched { .. }
            )
        };
        assert!(hit(&mut ops, "\u{e9}", "\u{c9}"));
        assert!(hit(&mut ops, "\u{c9}", "\u{e9}"));
        assert!(hit(&mut ops, "\u{430}", "\u{410}")); // Cyrillic a / A
        // `İ` (dotted capital I) folds to plain `i` under
        // `Tcl_UniCharToLower`, which is why the fold must be the simple 1:1
        // mapping and not Rust's full one (that expands to `i` + U+0307).
        assert!(hit(&mut ops, "i", "\u{130}"));
        // Still no false positives.
        assert!(!hit(&mut ops, "\u{e9}", "\u{e8}"));
        assert!(!hit(&mut ops, "ab", "abc"));
        // And ASCII keeps working.
        assert!(hit(&mut ops, "AbC", "aBc"));
    }

    fn parse_at(version: TclVersion, args: &[&str]) -> Result<Options<String>, CmdError> {
        let args: Vec<String> = args.iter().map(|&a| a.to_owned()).collect();
        parse_options(&mut StrOps, &args, version)
    }

    fn select_int(value: &str, pats: &[&str]) -> Result<Option<usize>, CmdError> {
        let opts = Options {
            mode: Mode::Integer,
            ..exact_opts()
        };
        let pats: Vec<String> = pats.iter().map(|&p| p.to_owned()).collect();
        let sel = select::<_, NoEngine, _>(
            &mut StrOps,
            &opts,
            &value.to_owned(),
            &pats,
            TclVersion::V9_1,
        )?;
        Ok(match sel {
            Selection::Matched { index, .. } => Some(index),
            Selection::NoMatch => None,
        })
    }

    #[test]
    fn integer_option_exists_only_from_tcl91() {
        // tclsh 9.0.4:
        //   % switch -integer 1 {1 {}}
        //   bad option "-integer": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --
        for v in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
        ] {
            let Err(e) = parse_at(v, &["-integer", "1", "{1 {}}"]) else {
                panic!("-integer must be refused on {v:?}");
            };
            assert_eq!(
                e.message(),
                "bad option \"-integer\": must be -exact, -glob, -indexvar, \
                 -matchvar, -nocase, -regexp, or --"
            );
            // Before 9.1 `-i` is a unique prefix of `-indexvar`.
            assert_eq!(
                e.error_code(),
                Some("TCL LOOKUP INDEX option -integer"),
                "{v:?}"
            );
            assert!(parse_at(v, &["-regexp", "-i", "x", "1", "{1 {}}"]).is_ok());
        }
        let opts = parse_at(TclVersion::V9_1, &["-int", "1", "{1 {}}"]).unwrap();
        assert!(opts.mode == Mode::Integer);
        assert_eq!(opts.value_index, 1);
    }

    #[test]
    fn tcl91_option_table_lists_integer_and_makes_i_ambiguous() {
        // tclsh 9.1.0:
        //   % switch -bad 1 {1 {}}
        //   bad option "-bad": must be -exact, -glob, -indexvar, -integer, -matchvar, -nocase, -regexp, or --
        //   % switch -in 1 {1 {}}
        //   ambiguous option "-in": must be -exact, -glob, -indexvar, -integer, -matchvar, -nocase, -regexp, or --
        let choices = "must be -exact, -glob, -indexvar, -integer, -matchvar, \
                       -nocase, -regexp, or --";
        let Err(e) = parse_at(TclVersion::V9_1, &["-bad", "1", "{1 {}}"]) else {
            panic!("-bad must not resolve");
        };
        assert_eq!(e.message(), format!("bad option \"-bad\": {choices}"));
        for word in ["-i", "-in"] {
            let Err(e) = parse_at(TclVersion::V9_1, &[word, "1", "{1 {}}"]) else {
                panic!("{word} must be ambiguous on 9.1");
            };
            assert_eq!(
                e.message(),
                format!("ambiguous option \"{word}\": {choices}")
            );
        }
    }

    #[test]
    fn integer_mode_option_conflicts_match_c() {
        // tclsh 9.1.0:
        //   % switch -integer -nocase 1 {1 {}}
        //   -nocase option cannot be used with -integer option
        //   % switch -glob -integer 1 {1 {}}
        //   bad option "-integer": -glob option already found
        //   % switch -integer -glob 1 {1 {}}
        //   bad option "-glob": -integer option already found
        //   % switch -integer -matchvar x 1 {1 {}}
        //   -matchvar option requires -regexp option
        let v = TclVersion::V9_1;
        for args in [
            &["-integer", "-nocase", "1", "{1 {}}"][..],
            &["-nocase", "-integer", "1", "{1 {}}"][..],
        ] {
            let Err(e) = parse_at(v, args) else {
                panic!("{args:?} must be refused");
            };
            assert_eq!(
                e.message(),
                "-nocase option cannot be used with -integer option"
            );
            assert_eq!(e.error_code(), Some("TCL OPERATION SWITCH MODERESTRICTION"));
        }
        let Err(e) = parse_at(v, &["-glob", "-integer", "1", "{1 {}}"]) else {
            panic!("double mode must be refused");
        };
        assert_eq!(
            e.message(),
            "bad option \"-integer\": -glob option already found"
        );
        assert_eq!(e.error_code(), Some("TCL OPERATION SWITCH DOUBLEOPT"));
        let Err(e) = parse_at(v, &["-integer", "-glob", "1", "{1 {}}"]) else {
            panic!("double mode must be refused");
        };
        assert_eq!(
            e.message(),
            "bad option \"-glob\": -integer option already found"
        );
        // `-matchvar` is checked before the `-nocase` conflict.
        let Err(e) = parse_at(v, &["-integer", "-nocase", "-matchvar", "x", "1", "{1 {}}"]) else {
            panic!("-matchvar without -regexp must be refused");
        };
        assert_eq!(e.message(), "-matchvar option requires -regexp option");
    }

    #[test]
    fn integer_mode_compares_numerically() {
        // tclsh 9.1.0: `switch -integer 010 {8 {puts a} 10 {puts b}}` prints `b`
        // (Tcl 9 reads `010` as decimal), and every radix/separator/whitespace
        // spelling `Tcl_GetWideIntFromObj` accepts compares by value.
        assert_eq!(select_int("010", &["8", "10"]).unwrap(), Some(1));
        assert_eq!(select_int("0x10", &["16"]).unwrap(), Some(0));
        assert_eq!(select_int(" 16 ", &["0x10"]).unwrap(), Some(0));
        assert_eq!(select_int("1_000", &["1000"]).unwrap(), Some(0));
        assert_eq!(select_int("0b11", &["0o3"]).unwrap(), Some(0));
        assert_eq!(
            select_int("-9223372036854775808", &["-0x8000000000000000"]).unwrap(),
            Some(0)
        );
        assert_eq!(select_int("2", &["1", "3"]).unwrap(), None);
        // A trailing `default` matches without being coerced.
        assert_eq!(select_int("2", &["1", "default"]).unwrap(), Some(1));
        // C coerces only the patterns it reaches: one after the match is unchecked.
        assert_eq!(select_int("1", &["1", "abc"]).unwrap(), Some(0));
    }

    #[test]
    fn integer_mode_coercion_errors_match_c() {
        // tclsh 9.1.0:
        //   % switch -integer abc {1 {}}            ;# also with only `default`
        //   expected integer but got "abc"          (TCL VALUE NUMBER)
        //   % switch -integer 2 {1 {} abc {} default {}}
        //   expected integer but got "abc"
        //   % switch -integer 2 {default {} 2 {}}
        //   expected integer but got "default"
        //   % switch -integer 99999999999999999999 {1 {}}
        //   integer value too large to represent    (ARITH IOVERFLOW ...)
        let not_int = |value: &str, pats: &[&str], bad: &str| {
            let Err(e) = select_int(value, pats) else {
                panic!("{value} {pats:?} must be refused");
            };
            assert_eq!(e.message(), format!("expected integer but got \"{bad}\""));
            assert_eq!(e.error_code(), Some("TCL VALUE NUMBER"));
        };
        not_int("abc", &["1"], "abc");
        not_int("abc", &["default"], "abc");
        not_int("", &["1"], "");
        not_int("1.0", &["1"], "1.0");
        not_int("2", &["1", "abc", "default"], "abc");
        not_int("2", &["default", "2"], "default");
        for (value, pats) in [
            ("99999999999999999999", &["1"][..]),
            ("1", &["99999999999999999999"][..]),
            ("18446744073709551615", &["-1"][..]),
            ("9223372036854775808", &["1"][..]),
        ] {
            let Err(e) = select_int(value, pats) else {
                panic!("{value} {pats:?} must overflow");
            };
            assert_eq!(e.message(), "integer value too large to represent");
            assert_eq!(
                e.error_code(),
                Some("ARITH IOVERFLOW {integer value too large to represent}")
            );
        }
    }
}
