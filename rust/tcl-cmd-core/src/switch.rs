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
//! table (`-exact`/`-glob`/`-regexp`/`-nocase`/`-indexvar`/`-matchvar`/`--`), the
//! value/pattern selection across the three match modes (incl. `default`), and
//! the TIP #75 `-matchvar`/`-indexvar` value construction. Each runtime keeps the
//! per-target parts: extracting the pattern/body pairs (the inline vs. brace-list
//! forms, with the runtime's `info frame` line tracking), resolving a `-`
//! fall-through body, the trace-aware variable writes, and evaluating the chosen
//! body as a transparent script.
//!
//! Mirrors C's `TclNRSwitchObjCmd` (`tclCmdMZ.c`). The regexp mode drives the
//! shared [`RegexEngine`](crate::regex::RegexEngine) provider, exactly as
//! `regexp`/`regsub` do.

// `ops`/`opts` and the regexp `so`/`eo` offsets are deliberately terse mirrors of
// the C names and recur across the module's functions, so the similar-names allow
// stays module-scoped. (The regexp offset cast is narrowed to `regexp_writes`.)
#![allow(clippy::similar_names)]

use tcl_syntax::glob::string_case_match;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;
use crate::regex::{NO_MATCH, RegMatch, RegexEngine, RegexFlags, decode_utf8};

/// The matching mode (`-exact` is the default).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Exact string equality.
    Exact,
    /// Glob (`string match`) patterns.
    Glob,
    /// Tcl ARE regular expressions.
    Regexp,
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
    /// Index, in the name-stripped args, of the `string` to switch on.
    pub value_index: usize,
}

// The option table — mirrors C's `options[]` so an "already found" error names
// the canonical mode option.
const OPT_EXACT: usize = 0;
const OPT_GLOB: usize = 1;
const OPT_INDEXV: usize = 2;
const OPT_MATCHV: usize = 3;
const OPT_NOCASE: usize = 4;
const OPT_REGEXP: usize = 5;
const OPT_LAST: usize = 6;
const OPT_NAMES: [&str; 7] = [
    "-exact",
    "-glob",
    "-indexvar",
    "-matchvar",
    "-nocase",
    "-regexp",
    "--",
];
const USAGE_INLINE: &str = "switch ?-option ...? string ?pattern body ...? ?default body?";

/// Parse the leading options of `args` (the name-stripped argv: any options, then
/// the `string`, then the pattern/body pairs or the single brace-list). Returns
/// the option state and the index of the `string` argument. Mirrors C's option
/// scan, whose bound leaves the string plus at least one pattern/body word.
///
/// # Errors
/// A bad/ambiguous option, a repeated mode option, a missing `-matchvar`/
/// `-indexvar` argument, `-matchvar`/`-indexvar` without `-regexp`, or too few
/// arguments after the options.
pub fn parse_options<O, V>(ops: &mut O, args: &[V]) -> Result<Options<V>, CmdError>
where
    O: ValueOps<Value = V>,
    V: Clone,
{
    let objc = args.len();
    let mut mode = OPT_EXACT;
    let mut found_mode = false;
    let mut nocase = false;
    let mut match_var: Option<V> = None;
    let mut index_var: Option<V> = None;

    let mut i = 0;
    // Leave the string plus at least one pattern/body word unparsed; `--` ends it.
    while i + 2 < objc {
        let arg = ops.as_str(&args[i]);
        if !arg.starts_with('-') {
            break;
        }
        let idx = match crate::prefix::lookup(&OPT_NAMES, arg.as_bytes(), false) {
            crate::prefix::Lookup::Found(i) => i,
            miss => {
                return Err(crate::prefix::lookup_error(
                    &OPT_NAMES, "option", &arg, miss,
                ));
            }
        };
        match idx {
            OPT_LAST => {
                i += 1;
                break;
            }
            OPT_NOCASE => nocase = true,
            OPT_INDEXV | OPT_MATCHV => {
                i += 1;
                if i + 2 > objc {
                    let name = if idx == OPT_INDEXV {
                        "-indexvar"
                    } else {
                        "-matchvar"
                    };
                    return Err(CmdError::new(format!(
                        "missing variable name argument to {name} option"
                    )));
                }
                if idx == OPT_INDEXV {
                    index_var = Some(args[i].clone());
                } else {
                    match_var = Some(args[i].clone());
                }
            }
            _ => {
                // A mode option (`-exact`/`-glob`/`-regexp`).
                if found_mode {
                    return Err(double_option(&arg, OPT_NAMES[mode]));
                }
                found_mode = true;
                mode = idx;
            }
        }
        i += 1;
    }

    if i + 2 > objc {
        return Err(CmdError::wrong_args(USAGE_INLINE));
    }
    if index_var.is_some() && mode != OPT_REGEXP {
        return Err(mode_restriction("-indexvar"));
    }
    if match_var.is_some() && mode != OPT_REGEXP {
        return Err(mode_restriction("-matchvar"));
    }

    Ok(Options {
        mode: match mode {
            OPT_GLOB => Mode::Glob,
            OPT_REGEXP => Mode::Regexp,
            _ => Mode::Exact,
        },
        nocase,
        match_var,
        index_var,
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
/// # Errors
/// A malformed `-regexp` pattern (the engine's compile error).
pub fn select<O, E, V>(
    ops: &mut O,
    opts: &Options<V>,
    value: &V,
    patterns: &[V],
) -> Result<Selection<V>, CmdError>
where
    O: ValueOps<Value = V>,
    E: RegexEngine,
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
    for (p, pat_val) in patterns.iter().enumerate() {
        let pat = ops.as_str(pat_val);
        // `default` matches anything, but only as the final pattern.
        if p == npairs - 1 && &*pat == "default" {
            let writes = default_writes(ops, opts);
            return Ok(Selection::Matched { index: p, writes });
        }
        match opts.mode {
            Mode::Exact => {
                let hit = if opts.nocase {
                    pat.eq_ignore_ascii_case(&val_str)
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
                    ..RegexFlags::default()
                };
                let mut re = E::compile(pat.as_bytes(), flags).map_err(|d| compile_error(&d))?;
                let value_bytes = ops.as_bytes(value);
                let (cps, byteoff) = decode_utf8(&value_bytes);
                if let Some(m) = E::exec(&mut re, &cps, 0, false) {
                    let writes = regexp_writes(ops, opts, &m, &value_bytes, &byteoff);
                    return Ok(Selection::Matched { index: p, writes });
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

// option-name resolution + error catalogue
//
// The unique-prefix resolution and the `bad option "x": must be …` /
// `ambiguous option …` texts route through the shared matcher
// (`crate::prefix`), so `switch` cannot drift from the other option tables.

fn double_option(arg: &str, found_name: &str) -> CmdError {
    CmdError::new(format!(
        "bad option \"{arg}\": {found_name} option already found"
    ))
}

fn mode_restriction(opt: &str) -> CmdError {
    CmdError::new(format!("{opt} option requires -regexp option"))
}

fn compile_error(detail: &[u8]) -> CmdError {
    CmdError::new(format!(
        "cannot compile regular expression pattern: {}",
        String::from_utf8_lossy(detail)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_table_matches_exact_and_unique_prefix() {
        // Tcl unambiguous-prefix option matching, via the shared matcher.
        use crate::prefix::{Lookup, lookup};
        let l = |arg: &str| lookup(&OPT_NAMES, arg.as_bytes(), false);
        assert_eq!(l("-exact"), Lookup::Found(0));
        assert_eq!(l("-glob"), Lookup::Found(1));
        assert_eq!(l("-e"), Lookup::Found(0)); // unique prefix
        assert_eq!(l("-g"), Lookup::Found(1));
        assert_eq!(l("-i"), Lookup::Found(2));
        assert_eq!(l("-m"), Lookup::Found(3));
        assert_eq!(l("-n"), Lookup::Found(4));
        assert_eq!(l("-"), Lookup::Ambiguous); // prefixes all
        assert_eq!(l("-zzz"), Lookup::None);
        // The option-list text stays byte-identical to the old inline const
        // (tclsh: `switch -zzz a b {}`).
        assert_eq!(
            crate::prefix::lookup_error(&OPT_NAMES, "option", "-zzz", Lookup::None).message(),
            r#"bad option "-zzz": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --"#
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
        ) -> Option<Vec<RegMatch>> {
            unreachable!()
        }
    }

    fn exact_opts() -> Options<String> {
        Options {
            mode: Mode::Exact,
            nocase: false,
            match_var: None,
            index_var: None,
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
        let result = select::<_, NoEngine, _>(&mut ops, &opts, &value, &[]).unwrap();
        assert!(matches!(result, Selection::NoMatch));
    }

    #[test]
    fn select_exact_still_matches_after_guard() {
        // Regression guard: the empty-list early-return must not disturb normal
        // selection. A trailing `default` and a literal hit both still work.
        let mut ops = StrOps;
        let opts = exact_opts();
        let value = String::from("b");
        let pats = vec![
            String::from("a"),
            String::from("b"),
            String::from("default"),
        ];
        match select::<_, NoEngine, _>(&mut ops, &opts, &value, &pats).unwrap() {
            Selection::Matched { index, .. } => assert_eq!(index, 1),
            Selection::NoMatch => panic!("expected a match"),
        }
        // No literal hit ⇒ the final `default` matches.
        let value = String::from("zzz");
        match select::<_, NoEngine, _>(&mut ops, &opts, &value, &pats).unwrap() {
            Selection::Matched { index, .. } => assert_eq!(index, 2),
            Selection::NoMatch => panic!("expected default to match"),
        }
    }
}
