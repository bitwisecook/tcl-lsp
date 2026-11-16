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

use super::regexp_command::slice_match;
use super::*;
use tcl_syntax::value::ValueOps;

#[test]
fn regex_error_wrapper_retains_arity_and_primitive_state_actions() {
    let usage = wrong_args(b"regexp native\0\xff").into_cmd_error();
    assert_eq!(
        usage.error_code_update(),
        &crate::CmdErrorCodeUpdate::WrongArguments
    );
    let details = crate::CmdErrorDetails {
        string_result: None,
        message: b"RAW\0\xc0\x80\xff".to_vec(),
        error_code: crate::CmdErrorCodeUpdate::Unchanged,
        error_info: Some(b"INFO\0\xff".to_vec()),
        error_line: Some(17),
        primitive_getter: None,
    };
    assert_eq!(
        RegexError::from(crate::CmdError::from_byte_details(details.clone()))
            .into_cmd_error()
            .into_byte_details(),
        details
    );
}

#[test]
fn original_match_targets_are_borrowed_and_assigned_before_the_next_match() {
    let args = ["-all", "a", "aba", "first", "second"].map(str::to_owned);
    let mut writes = Vec::new();
    let result = regexp_original::<ListOps, LiteralEngine, &'static str>(
        &mut ListOps,
        &args,
        TclVersion::V9_0,
        |_, name, value| {
            let original = if name == "first" { &args[3] } else { &args[4] };
            assert!(std::ptr::eq(name, original));
            writes.push((name.clone(), value));
            if writes.len() == 3 {
                Err("setter failed")
            } else {
                Ok(())
            }
        },
    );
    assert!(matches!(
        result,
        Err(RegexpExecutionError::Assignment("setter failed"))
    ));
    assert_eq!(
        writes,
        [
            ("first".into(), "a".into()),
            ("second".into(), String::new()),
            ("first".into(), "a".into())
        ]
    );
}

#[test]
fn resolve_start_handles_integer_and_end_forms() {
    // regexp `-start` index: integer / end / end±N against the char
    // length, clamped to 0.
    assert_eq!(resolve_start_checked(&mut ListOps, b"5", 10).unwrap(), 5);
    assert_eq!(resolve_start_checked(&mut ListOps, b"0", 10).unwrap(), 0);
    assert_eq!(resolve_start_checked(&mut ListOps, b"end", 10).unwrap(), 9);
    assert_eq!(
        resolve_start_checked(&mut ListOps, b"end-2", 10).unwrap(),
        7
    );
    assert_eq!(
        resolve_start_checked(&mut ListOps, b"end+1", 10).unwrap(),
        10
    );
    assert_eq!(resolve_start_checked(&mut ListOps, b"1+1", 10).unwrap(), 2);
    assert_eq!(resolve_start_checked(&mut ListOps, b"0x2", 10).unwrap(), 2);
    assert_eq!(resolve_start_checked(&mut ListOps, b"+5", 10).unwrap(), 5);
    assert_eq!(resolve_start_checked(&mut ListOps, b"-3", 10).unwrap(), 0); // Tcl clamps negatives.
    assert!(resolve_start_checked(&mut ListOps, b"bad", 10).is_err());
    assert!(resolve_start_checked(&mut ListOps, b"end - 2", 10).is_err());
    assert_eq!(resolve_start_checked(&mut ListOps, b"end", 0).unwrap(), 0);
}

#[test]
fn resolve_start_end_offset_saturates_without_overflow() {
    // A giant `end±N` must not overflow the isize add/sub. An `N` near
    // `isize::MAX` still parses, so `(len-1) ± N` is where the wrap would
    // happen — `saturating_*` pins it instead: `end+N` to "past the end" (the
    // match loop then finds nothing), `end-N` back to the start.
    let big = b"end+9223372036854775800"; // close to isize::MAX, parses fine
    assert_eq!(
        resolve_start_checked(&mut ListOps, big, 10).unwrap(),
        usize::try_from(i64::MAX).unwrap_or(usize::MAX)
    );
    assert_eq!(
        resolve_start_checked(&mut ListOps, b"end-9223372036854775800", 10).unwrap(),
        0
    );
    // A bignum operand is a bad index, not a silent reset to zero.
    assert!(resolve_start_checked(&mut ListOps, b"end+99999999999999999999999", 10).is_err());
}

#[test]
fn slice_match_guards_out_of_range_engine_offsets() {
    // The byte-offset slice must never index-panic on a (foreign)
    // engine offset past the char→byte table or with `eo < so`.
    let bytes = b"hello";
    let byteoff = [0usize, 1, 2, 3, 4, 5]; // 5 chars + final len
    assert_eq!(slice_match(bytes, &byteoff, 1, 4), b"ell");
    // `eo` past the table → empty, not a panic.
    assert_eq!(slice_match(bytes, &byteoff, 0, 99), b"");
    // `so` past the table → empty.
    assert_eq!(slice_match(bytes, &byteoff, 99, 100), b"");
    // Inverted range → empty.
    assert_eq!(slice_match(bytes, &byteoff, 4, 1), b"");
    // The `NO_MATCH` sentinel as an index → empty (never indexes).
    assert_eq!(slice_match(bytes, &byteoff, NO_MATCH, NO_MATCH), b"");
}

/// A throwaway `ValueOps` whose `new_list` renders real Tcl list syntax, so
/// an assertion reads exactly as tclsh prints the answer.
#[derive(Default)]
struct ListOps;

impl crate::index::NativeIndexOps for ListOps {
    type IndexOperation = ();
    type IndexBinaryReceipt = ();
    type IndexKeyOwner = Self::Value;
    fn index_borrowed_key(&mut self, value: &Self::Value) -> Result<Self::Value, crate::CmdError> {
        Ok(value.clone())
    }
    fn index_created_key(&mut self, value: Self::Value) -> Result<Self::Value, crate::CmdError> {
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
    fn index_key_argument(&mut self, owner: &Self::Value) -> Result<Self::Value, crate::CmdError> {
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
    fn check_index_operation(&mut self, (): &()) -> Result<(), crate::CmdError> {
        Ok(())
    }
    fn retain_index_failure(&mut self, _error: &crate::CmdError) {
        // Pure neutral adapter has no interpreter first-cause owner.
    }
    fn discard_owned_index_result(&mut self, created: Self::Value) {
        drop(created);
    }
}

impl ValueOps for ListOps {
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
        tcl_syntax::list::join_list(items)
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
    fn list_elements(&mut self, v: &String) -> Result<Vec<String>, tcl_syntax::value::ValueError> {
        Ok(v.split_whitespace().map(str::to_owned).collect())
    }
}

impl NativeRegexSource for ListOps {
    fn regex_recipe(
        &self,
    ) -> Result<Option<tcl_syntax::native_regex::NativeRegexpRecipe>, tcl_syntax::value::ValueError>
    {
        Ok(None)
    }
}
impl NativeRegexObjects<LiteralEngine> for ListOps {
    fn regex_cached_pattern(
        &self,
        _original: &String,
        _recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        _flags: u32,
    ) -> Result<Option<std::rc::Rc<std::cell::RefCell<LiteralRe>>>, tcl_syntax::value::ValueError>
    {
        unreachable!("compatibility fixture")
    }
    fn regex_cached_glob(
        &self,
        _original: &String,
        _recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        _flags: u32,
    ) -> Result<Option<std::rc::Rc<[u8]>>, tcl_syntax::value::ValueError> {
        Ok(None)
    }
    fn regex_install_pattern(
        &mut self,
        _original: &String,
        _recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        _flags: u32,
        _compiled: std::rc::Rc<std::cell::RefCell<LiteralRe>>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        unreachable!("compatibility fixture")
    }
    fn regex_range_value(
        &mut self,
        _range: tcl_syntax::native_regex::NativeRegexpRange,
    ) -> Result<String, tcl_syntax::value::ValueError> {
        unreachable!("compatibility fixture")
    }
}

/// A stand-in engine for the *plumbing* tests. `regexp -about` and `regsub
/// -command` are pure plumbing over `nsub` / `info_names` / `exec`, so a
/// deliberately trivial provider exercises them without dragging the real
/// ARE engine (a different crate) into this one's unit tests.
///
/// Its pattern language: the text matches **literally** once `(` and `)`
/// are dropped, `nsub` is the number of `(`, and every participating
/// subexpression reports the whole match. A pattern of `!bad` fails to
/// compile, so the compile-error path is reachable.
struct LiteralEngine;

struct LiteralRe {
    text: Vec<i32>,
    nsub: usize,
}

impl RegexEngine for LiteralEngine {
    type Regex = LiteralRe;

    fn compile(pattern: &[u8], _flags: RegexFlags) -> Result<LiteralRe, Vec<u8>> {
        if pattern == b"!bad" {
            return Err(b"brackets [] not balanced".to_vec());
        }
        let (cps, _) = decode_utf8(pattern);
        Ok(LiteralRe {
            nsub: cps.iter().filter(|&&c| c == i32::from(b'(')).count(),
            text: cps
                .into_iter()
                .filter(|&c| c != i32::from(b'(') && c != i32::from(b')'))
                .collect(),
        })
    }

    fn nsub(re: &LiteralRe) -> usize {
        re.nsub
    }

    fn exec(
        re: &mut LiteralRe,
        cps: &[i32],
        offset: usize,
        _notbol: bool,
    ) -> RegexpPrecision<RegMatch> {
        let n = re.text.len();
        let at = cps
            .len()
            .checked_sub(n)
            .and_then(|last| (offset..=last).find(|&i| cps[i..i + n] == re.text[..]));
        let Some(at) = at else {
            return RegexpPrecision::NoMatch;
        };
        let whole = RegMatch { so: at, eo: at + n };
        RegexpPrecision::exact(&core::iter::repeat_n(whole, re.nsub + 1).collect::<Vec<_>>())
    }

    const IDENTITY: EngineIdentity = EngineIdentity {
        name: "literal",
        revision: 1,
    };
}

/// The same engine with `re_info` flags, to prove `-about` renders the
/// engine's list in the engine's order.
struct FlaggyEngine;

impl RegexEngine for FlaggyEngine {
    type Regex = LiteralRe;
    fn compile(pattern: &[u8], flags: RegexFlags) -> Result<LiteralRe, Vec<u8>> {
        LiteralEngine::compile(pattern, flags)
    }
    fn nsub(re: &LiteralRe) -> usize {
        re.nsub
    }
    fn info_names(_re: &LiteralRe) -> Vec<&'static str> {
        vec!["REG_UNONPOSIX", "REG_ULOCALE"]
    }
    fn exec(
        re: &mut LiteralRe,
        cps: &[i32],
        offset: usize,
        notbol: bool,
    ) -> RegexpPrecision<RegMatch> {
        LiteralEngine::exec(re, cps, offset, notbol)
    }

    const IDENTITY: EngineIdentity = EngineIdentity {
        name: "flaggy",
        revision: 1,
    };
}

/// An engine whose every search stops: what a search that ran out of
/// budget looks like to the plumbing.
struct StoppingEngine;

impl RegexEngine for StoppingEngine {
    type Regex = LiteralRe;
    fn compile(pattern: &[u8], flags: RegexFlags) -> Result<LiteralRe, Vec<u8>> {
        LiteralEngine::compile(pattern, flags)
    }
    fn nsub(re: &LiteralRe) -> usize {
        re.nsub
    }
    fn exec(
        _re: &mut LiteralRe,
        _cps: &[i32],
        _offset: usize,
        _notbol: bool,
    ) -> RegexpPrecision<RegMatch> {
        RegexpPrecision::Declined(PrecisionDecline::FuelExhausted { spent: 7 })
    }
    const IDENTITY: EngineIdentity = EngineIdentity {
        name: "stopping",
        revision: 1,
    };
}

/// A search that established neither a match nor a no-match is raised on
/// the runtime path — never a count of 0, never the unsubstituted text —
/// and kept typed on the analysis path.
#[test]
fn a_stopped_search_is_raised_at_run_time_and_typed_in_analysis() {
    let raised = b"error while matching regular expression: the search exceeded its work budget";
    let mut ops = ListOps;
    assert!(matches!(
        regexp::<ListOps, StoppingEngine>(&mut ops, &[b"abc", b"xabcx"], TclVersion::V9_0),
        Err(error) if error.message_bytes() == raised
    ));
    assert!(matches!(
        regsub::<StoppingEngine>(&[b"abc", b"xabcx", b"-"]),
        Err(error) if error.message_bytes() == raised
    ));
    let mut charge = |_: u64| true;
    let mut analysis = AnalysisMatch {
        target: (None, None),
        charge: &mut charge,
        limits: MatchLimits::default(),
    };
    assert!(matches!(
        regexp_analysis::<ListOps, StoppingEngine>(
            &mut ops,
            &[b"abc", b"xabcx"],
            TclVersion::V9_0,
            &mut analysis
        ),
        Err(RegexFailure::Declined(PrecisionDecline::FuelExhausted {
            spent: 7
        }))
    ));
    assert!(matches!(
        regsub_analysis::<StoppingEngine>(
            &[b"abc", b"xabcx", b"-"],
            TclVersion::V9_0,
            &mut analysis
        ),
        Err(RegexFailure::Declined(PrecisionDecline::FuelExhausted {
            spent: 7
        }))
    ));
}

/// The analysis path compiles through the thread's cache: a pattern is
/// charged its length squared once, then served from the cache; a
/// different target is a different key; a refused charge is `Cancelled`
/// and caches nothing; and a `-command` substitution that is due
/// declines rather than answer with the unsubstituted text.
#[test]
fn the_pattern_cache_charges_a_compile_once_per_key() {
    let mut ops = ListOps;
    let mut ledger: Vec<u64> = Vec::new();
    let mut charge = |units: u64| {
        ledger.push(units);
        true
    };
    {
        let mut analysis = AnalysisMatch {
            target: (None, None),
            charge: &mut charge,
            limits: MatchLimits::default(),
        };
        for _ in 0..2 {
            let answer = regexp_analysis::<ListOps, LiteralEngine>(
                &mut ops,
                &[b"cached-abc", b"xcached-abcx"],
                TclVersion::V9_0,
                &mut analysis,
            );
            assert!(matches!(answer, Ok(RegexpResult::Count { count: 1, .. })));
        }
        // Another target is another key.
        analysis.target = (Some(StringCharacterModel::Utf16CodeUnits), None);
        let answer = regexp_analysis::<ListOps, LiteralEngine>(
            &mut ops,
            &[b"cached-abc", b"xcached-abcx"],
            TclVersion::V9_0,
            &mut analysis,
        );
        assert!(answer.is_ok());
        // A `-command` substitution that is due declines.
        assert!(matches!(
            regsub_analysis::<LiteralEngine>(
                &[b"-command", b"b", b"abc", b"string toupper"],
                TclVersion::V9_0,
                &mut analysis
            ),
            Err(RegexFailure::Declined(
                PrecisionDecline::FormUnsupported { .. }
            ))
        ));
    }
    assert_eq!(
        ledger,
        [100, 100, 1],
        "one compile per key, charged its length squared"
    );
    let mut refuse = |_: u64| false;
    let mut refused = AnalysisMatch {
        target: (None, None),
        charge: &mut refuse,
        limits: MatchLimits::default(),
    };
    assert!(matches!(
        regexp_analysis::<ListOps, LiteralEngine>(
            &mut ops,
            &[b"uncached", b"x"],
            TclVersion::V9_0,
            &mut refused
        ),
        Err(RegexFailure::Declined(PrecisionDecline::Cancelled))
    ));
}

/// The cache's bound is retained bytes: filling it past
/// [`PATTERN_CACHE_BYTES`] evicts the coldest entries, and it never
/// holds more.
#[test]
fn the_pattern_cache_stays_within_its_byte_bound() {
    let mut ops = ListOps;
    let mut charge = |_: u64| true;
    let mut analysis = AnalysisMatch {
        target: (None, None),
        charge: &mut charge,
        limits: MatchLimits::default(),
    };
    for n in 0..6u8 {
        let mut pattern = vec![b'q'; 1024 * 1024];
        pattern[0] = b'a' + n;
        let answer = regexp_analysis::<ListOps, LiteralEngine>(
            &mut ops,
            &[&pattern, b"z"],
            TclVersion::V9_0,
            &mut analysis,
        );
        assert!(matches!(answer, Ok(RegexpResult::Count { count: 0, .. })));
        assert!(pattern_cache_retained_bytes() <= PATTERN_CACHE_BYTES);
    }
    assert!(
        pattern_cache_retained_bytes() >= 3 * 1024 * 1024,
        "the cache keeps what fits"
    );
}

fn about(args: &[&[u8]]) -> Result<String, String> {
    let mut ops = ListOps;
    match regexp::<ListOps, LiteralEngine>(&mut ops, args, TclVersion::V9_0) {
        Ok(RegexpResult::Inline(v)) => Ok(v),
        Ok(RegexpResult::Count { count, .. }) => Ok(count.to_string()),
        Err(error) => Err(String::from_utf8_lossy(error.message_bytes()).into_owned()),
    }
}

#[test]
fn regexp_about_reports_the_subexpression_count_and_info_list() {
    // Regression (#2124): `-about` was refused outright with `regexp -about
    // is not yet supported`, on every release. tclsh 8.4.20, 8.5.19,
    // 8.6.18, 9.0.4 and 9.1b0 all answer:
    //   % regexp -about {a(b)c}        ;# 1 {}
    //   % regexp -about abc            ;# 0 {}
    //   % regexp -about {(a)(b)}       ;# 2 {}
    //   % regexp -about {(a)} extraarg ;# 1 {}   (the subject is ignored)
    assert_eq!(about(&[b"-about", b"a(b)c"]).unwrap(), "1 {}");
    assert_eq!(about(&[b"-about", b"abc"]).unwrap(), "0 {}");
    assert_eq!(about(&[b"-about", b"(a)(b)"]).unwrap(), "2 {}");
    assert_eq!(about(&[b"-about", b"(a)", b"extraarg"]).unwrap(), "1 {}");
    assert_eq!(about(&[b"-about", b"--", b"(a)"]).unwrap(), "1 {}");
    // `-all`/`-indices`/`-start` are ignored in about mode, and `-nocase`
    // only reaches the compile.
    assert_eq!(about(&[b"-all", b"-about", b"(a)"]).unwrap(), "1 {}");
    assert_eq!(about(&[b"-about", b"-indices", b"(a)"]).unwrap(), "1 {}");
    assert_eq!(
        about(&[b"-about", b"-start", b"3", b"(a)"]).unwrap(),
        "1 {}"
    );
    // The engine's `re_info` names become the second element, in order.
    let mut ops = ListOps;
    let Ok(RegexpResult::Inline(v)) =
        regexp::<ListOps, FlaggyEngine>(&mut ops, &[b"-about", b"(a)"], TclVersion::V9_0)
    else {
        panic!("-about must answer")
    };
    assert_eq!(v, "1 {REG_UNONPOSIX REG_ULOCALE}");
}

#[test]
fn regexp_about_keeps_cs_arity_and_inline_rules() {
    // C's arity bound is `(objc - i) < (2 - about)`, so `-about` needs the
    // pattern and nothing more, but a bare `regexp -about` is still a
    // wrong-# args (tclsh 8.6.18/9.0.4/9.1b0 word it with `?-option ...?`).
    assert_eq!(
        about(&[b"-about"]).unwrap_err(),
        "wrong # args: should be \"regexp ?-option ...? exp string \
             ?matchVar? ?subMatchVar ...?\""
    );
    assert_eq!(
        about(&[b"-about", b"--"]).unwrap_err(),
        "wrong # args: should be \"regexp ?-option ...? exp string \
             ?matchVar? ?subMatchVar ...?\""
    );
    // C checks `-inline` before branching to `-about`, so the mix error
    // wins — tclsh 8.4.20 through 9.1b0:
    //   % regexp -about -inline {(a)}
    //   regexp match variables not allowed when using -inline
    assert_eq!(
        about(&[b"-about", b"-inline", b"(a)"]).unwrap_err(),
        "regexp match variables not allowed when using -inline"
    );
    // A bad pattern is still a compile error, not an about answer.
    assert_eq!(
        about(&[b"-about", b"!bad"]).unwrap_err(),
        "cannot compile regular expression pattern: brackets [] not balanced"
    );
    // Without `-about`, the ordinary two-argument minimum still applies.
    assert_eq!(
        about(&[b"(a)"]).unwrap_err(),
        "wrong # args: should be \"regexp ?-option ...? exp string \
             ?matchVar? ?subMatchVar ...?\""
    );
}

#[test]
fn compile_error_prefix_follows_the_release() {
    // tclsh 8.4.20 / 8.5.19 / 8.6.18 vs 9.0.4 / 9.1.0:
    //   % regexp {[a} b
    //   couldn't compile regular expression pattern: brackets [] not balanced
    //   cannot compile regular expression pattern: brackets [] not balanced
    for (version, verb) in [
        (TclVersion::V8_4, "couldn't"),
        (TclVersion::V8_5, "couldn't"),
        (TclVersion::V8_6, "couldn't"),
        (TclVersion::V9_0, "cannot"),
        (TclVersion::V9_1, "cannot"),
    ] {
        let want = format!("{verb} compile regular expression pattern: brackets [] not balanced");
        for args in [&[b"!bad".as_slice(), b"x"][..], &[b"-about", b"!bad"]] {
            let mut ops = ListOps;
            let Err(error) = regexp::<ListOps, LiteralEngine>(&mut ops, args, version) else {
                panic!("{version:?}: a bad pattern must not compile")
            };
            assert_eq!(
                String::from_utf8_lossy(error.message_bytes()),
                want,
                "{version:?}"
            );
        }
        assert_eq!(
            regsub_at(version, &[b"!bad", b"x", b"y"]).unwrap_err(),
            want,
            "{version:?}"
        );
    }
}

fn regsub_at(version: TclVersion, args: &[&[u8]]) -> Result<(String, i64), String> {
    let joined = |argv: &[Vec<u8>]| {
        let words: Vec<String> = argv
            .iter()
            .map(|w| String::from_utf8_lossy(w).into_owned())
            .collect();
        Ok::<Vec<u8>, String>(format!("<{}>", words.join("|")).into_bytes())
    };
    match regsub_eval::<LiteralEngine, String>(args, version, joined) {
        Ok(r) => Ok((String::from_utf8_lossy(&r.text).into_owned(), r.count)),
        Err(RegsubError::Regex(error)) => {
            Err(String::from_utf8_lossy(error.message_bytes()).into_owned())
        }
        Err(RegsubError::Eval(e)) => Err(e),
    }
}

#[test]
fn regsub_command_is_refused_before_9_0_in_cs_own_words() {
    // Regression (#2124): `-command` was refused with `regsub -command is
    // not yet supported` on every release, where C has three answers.
    //
    // tclsh8.4.20 and tclsh8.5.19 (`Tcl_GetIndexFromObj`'s noun is
    // "switch" until 8.6, and the table has no `-command`):
    //   % regsub -command {a} abc {string toupper}
    //   bad switch "-command": must be -all, -nocase, -expanded, -line,
    //   -linestop, -lineanchor, -start, or --
    for v in [TclVersion::V8_4, TclVersion::V8_5] {
        assert_eq!(
            regsub_at(v, &[b"-command", b"a", b"abc", b"up"]).unwrap_err(),
            "bad switch \"-command\": must be -all, -nocase, -expanded, \
                 -line, -linestop, -lineanchor, -start, or --",
            "{v:?}"
        );
    }
    // tclsh8.6.18 — same table, the noun becomes "option":
    //   % regsub -command {a} abc {string toupper}
    //   bad option "-command": must be -all, -nocase, -expanded, -line,
    //   -linestop, -lineanchor, -start, or --
    assert_eq!(
        regsub_at(TclVersion::V8_6, &[b"-command", b"a", b"abc", b"up"]).unwrap_err(),
        "bad option \"-command\": must be -all, -nocase, -expanded, -line, \
             -linestop, -lineanchor, -start, or --"
    );
    // The rest of the 8.x enumeration is the 8.x table too, not 9.0's —
    // tclsh8.6.18 `regsub -bogus a b c` prints exactly the same list.
    assert_eq!(
        regsub_at(TclVersion::V8_6, &[b"-bogus", b"a", b"abc", b"x"]).unwrap_err(),
        "bad option \"-bogus\": must be -all, -nocase, -expanded, -line, \
             -linestop, -lineanchor, -start, or --"
    );
    // …while 9.0's own enumeration carries `-command` and puts `-nocase`
    // last but one (tclsh9.0.4 / 9.1b0 `regsub -bogus a b c`).
    assert_eq!(
        regsub_at(TclVersion::V9_0, &[b"-bogus", b"a", b"abc", b"x"]).unwrap_err(),
        "bad option \"-bogus\": must be -all, -command, -expanded, -line, \
             -linestop, -lineanchor, -nocase, -start, or --"
    );
}

#[test]
fn regsub_command_evaluates_the_prefix_from_9_0() {
    // tclsh9.0.4 / 9.1b0 serve `-command` by appending the match and its
    // submatches to the prefix and substituting the result:
    //   % proc cap args {return "<[join $args |]>"}
    //   % regsub -command {(a)(b)} xabcy cap   ;# x<ab|a|b>cy
    //   % regsub -all -command {b} abc {list X} ;# aX bc
    for v in [TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            regsub_at(v, &[b"-command", b"(a)(b)", b"xabcy", b"cap"]).unwrap(),
            ("x<cap|ab|ab|ab>cy".to_owned(), 1),
            "{v:?}"
        );
        // The prefix is a *list*, so its words arrive as separate words.
        assert_eq!(
            regsub_at(v, &[b"-command", b"b", b"abc", b"list X"]).unwrap(),
            ("a<list|X|b>c".to_owned(), 1),
            "{v:?}"
        );
        // `-all` evaluates once per match.
        assert_eq!(
            regsub_at(v, &[b"-all", b"-command", b"b", b"abcb", b"f"]).unwrap(),
            ("a<f|b>c<f|b>".to_owned(), 2),
            "{v:?}"
        );
        // A pattern that never matches returns the subject and never
        // evaluates (tclsh9.0.4: `regsub -command {z} abc {string toupper}`
        // → `abc`).
        assert_eq!(
            regsub_at(v, &[b"-command", b"z", b"abc", b"f"]).unwrap(),
            ("abc".to_owned(), 0),
            "{v:?}"
        );
        // …but an unusable prefix is still rejected, match or no match
        // (tclsh9.0.4: `regsub -command {z} abc {}`).
        assert_eq!(
            regsub_at(v, &[b"-command", b"z", b"abc", b""]).unwrap_err(),
            "command prefix must be a list of at least one element",
            "{v:?}"
        );
        // Without `-command` the subspec is still expanded, not evaluated.
        assert_eq!(
            regsub_at(v, &[b"b", b"abc", b"[&]"]).unwrap(),
            ("a[b]c".to_owned(), 1),
            "{v:?}"
        );
    }
}

#[test]
fn regsub_without_an_evaluator_still_refuses_command() {
    // The evaluator-free `regsub` keeps 9.0's option table (the VM's own
    // tests pin that enumeration) and refuses only when a substitution is
    // actually due — so a non-matching `-command` call still answers.
    let refused = regsub::<LiteralEngine>(&[b"-command", b"a", b"abc", b"f"])
        .err()
        .map(|error| String::from_utf8_lossy(error.message_bytes()).into_owned());
    assert_eq!(
        refused.as_deref(),
        Some("regsub -command is not yet supported")
    );
    let Ok(r) = regsub::<LiteralEngine>(&[b"-command", b"z", b"abc", b"f"]) else {
        panic!("a non-matching -command needs no evaluator")
    };
    assert_eq!(
        (String::from_utf8_lossy(&r.text).into_owned(), r.count),
        ("abc".to_owned(), 0)
    );
}

#[test]
fn regsub_all_with_the_literal_empty_pattern_substitutes_once_per_character() {
    // Regression (#2147): `regsub -all {} abc X` counted 4 and produced
    // `XaXbXcX` — the general empty-match rule, one substitution before
    // each character *and* one at end-of-string. C never runs the engine
    // here: a metacharacter-free pattern goes down `Tcl_RegsubObjCmd`'s
    // literal string-map path, whose empty-pattern branch walks the
    // subject's characters and stops at the last one.
    //
    //   % regsub -all {} abc X r ;# 3, r is XaXbXc
    //
    // (tclsh 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1b0 all agree.)
    for v in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(
            regsub_at(v, &[b"-all", b"", b"abc", b"X"]).unwrap(),
            ("XaXbXc".to_owned(), 3)
        );
        // One character, and a multi-character replacement, so a fix that
        // merely subtracted one from the old count is not enough: the
        // *text* has to lose its trailing replacement too.
        assert_eq!(
            regsub_at(v, &[b"-all", b"", b"a", b"YZ"]).unwrap(),
            ("YZa".to_owned(), 1)
        );
        // An empty subject has no characters, so nothing is substituted —
        // where the general rule (and `regexp -all`) still reports one
        // empty match at offset 0.
        assert_eq!(
            regsub_at(v, &[b"-all", b"", b"", b"X"]).unwrap(),
            (String::new(), 0)
        );
    }
}

#[test]
fn regsub_empty_pattern_special_case_keeps_cs_guard_terms() {
    // Each row here is a way C *declines* its literal string-map path and
    // falls back to the engine's general empty-match rule, which counts
    // one more (a substitution at end-of-string). They are the guard's
    // reason for existing: drop a term and its row swings to the
    // once-per-character answer.
    let v = TclVersion::V9_0;
    // `&` in the substitution spec (C: `strpbrk(subSpec, "&\\")`).
    assert_eq!(
        regsub_at(v, &[b"-all", b"", b"abc", b"&"]).unwrap(),
        ("abc".to_owned(), 4)
    );
    // A backslash in the substitution spec, same C term.
    assert_eq!(
        regsub_at(v, &[b"-all", b"", b"abc", br"\0"]).unwrap(),
        ("abc".to_owned(), 4)
    );
    // A non-zero resolved `-start` (C: `offset == 0`).
    assert_eq!(
        regsub_at(v, &[b"-all", b"-start", b"1", b"", b"abc", b"X"]).unwrap(),
        ("aXbXcX".to_owned(), 3)
    );
    // `-start 0` resolves to 0, so the special case *does* apply.
    assert_eq!(
        regsub_at(v, &[b"-all", b"-start", b"0", b"", b"abc", b"X"]).unwrap(),
        ("XaXbXc".to_owned(), 3)
    );
    // `-command` (C: `command == 0`, 9.0's added term).
    assert_eq!(
        regsub_at(v, &[b"-all", b"-command", b"", b"abc", b"f"]).unwrap(),
        ("<f|>a<f|>b<f|>c<f|>".to_owned(), 4)
    );
    // No `-all` at all: a single substitution at offset 0, untouched.
    assert_eq!(
        regsub_at(v, &[b"", b"abc", b"X"]).unwrap(),
        ("Xabc".to_owned(), 1)
    );
}

#[test]
fn regsub_all_literal_empty_pattern_walks_the_releases_characters() {
    // Regression (#2170): a supplementary subject declined the fast path
    // and the general loop answered one replacement per scalar plus one
    // at end-of-string (2 for U+1D11E on every release). Measured with
    // `regsub -all {} $s X out`, `$s` built as
    // `[encoding convertfrom utf-8 "\xf0\x9d\x84\x9e"]`, `out` read back
    // with `scan … %c` over `split $out {}`, under LANG=C.UTF-8 and
    // unset alike:
    //
    //   subject   8.4.20, 8.5.19                  8.6.18                    9.0.4, 9.1.0
    //   G         4: X 240 X 157 X 132 X 158      2: X 55348 X 56606        1: X 119070
    //   aGb       6: X a X 240 … X 158 X b        4: X a X 55348 X 56606 X b 3: X a X G X b
    //
    // With an empty replacement 8.4/8.5 still answer the four characters
    // 240 157 132 158. 8.6's second replacement sits between the
    // surrogate halves, which a Rust string cannot hold apart, so both go
    // before the pair; the count and the length are tclsh's.
    let g = "\u{1D11E}".as_bytes();
    let a_g_b = "a\u{1D11E}b".as_bytes();
    let latin1 = "\u{F0}\u{9D}\u{84}\u{9E}";
    for v in [TclVersion::V8_4, TclVersion::V8_5] {
        assert_eq!(
            regsub_at(v, &[b"-all", b"", g, b"X"]).unwrap(),
            ("X\u{F0}X\u{9D}X\u{84}X\u{9E}".to_owned(), 4),
            "{v:?}"
        );
        assert_eq!(
            regsub_at(v, &[b"-all", b"", a_g_b, b"X"]).unwrap(),
            ("XaX\u{F0}X\u{9D}X\u{84}X\u{9E}Xb".to_owned(), 6),
            "{v:?}"
        );
        assert_eq!(
            regsub_at(v, &[b"-all", b"", g, b""]).unwrap(),
            (latin1.to_owned(), 4),
            "{v:?}"
        );
    }
    assert_eq!(
        regsub_at(TclVersion::V8_6, &[b"-all", b"", g, b"X"]).unwrap(),
        ("XX\u{1D11E}".to_owned(), 2)
    );
    assert_eq!(
        regsub_at(TclVersion::V8_6, &[b"-all", b"", a_g_b, b"X"]).unwrap(),
        ("XaXX\u{1D11E}Xb".to_owned(), 4)
    );
    for v in [TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            regsub_at(v, &[b"-all", b"", g, b"X"]).unwrap(),
            ("X\u{1D11E}".to_owned(), 1),
            "{v:?}"
        );
        assert_eq!(
            regsub_at(v, &[b"-all", b"", a_g_b, b"X"]).unwrap(),
            ("XaX\u{1D11E}Xb".to_owned(), 3),
            "{v:?}"
        );
    }
    // `-start 0` resolves to 0 and takes the same path (8.4.20: 4).
    assert_eq!(
        regsub_at(TclVersion::V8_4, &[b"-all", b"-start", b"0", b"", g, b"<>"])
            .unwrap()
            .1,
        4
    );
}
