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

//! `switch` — multi-way branch (toward running tcltest). C ref `tclCmdMZ.c`
//! (`TclNRSwitchObjCmd`).
//!
//! `switch ?options? string pattern body ?pattern body ...?` or
//! `switch ?options? string {pattern body ...}`. A `default` pattern matches
//! anything; a body of `-` falls through to the next pattern's body. The chosen
//! body is a **script** evaluated in the current scope (transparent — its code,
//! incl. `return`/`break`/`continue`, propagates). Modes: `-exact` (default),
//! `-glob`, `-regexp`, and Tcl 9.1's `-integer`; plus `-nocase`, `--`, and the
//! TIP #75 regexp side-channel options `-matchvar`/`-indexvar`.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

mod native_jim;

use tcl_cmd_core::switch::{self as core_switch, Options, Selection};
use tcl_runtime_api::completion_options::ControlOptionPolicy;

use crate::cmd_regex::AreEngine;
use crate::interp::{drop_fresh, obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register `switch`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"switch", switch_cmd);
}

fn switch_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if let Some(protocol) = interp
        .native_invocation_dialect()
        .native_jim_switch_protocol()
    {
        return native_jim::invoke(interp, &argv[1..], protocol);
    }
    // Option parsing + the `string` index are the shared core (`argv[1..]` strips
    // the command name to the name-stripped slice the core expects).
    let version = interp
        .native_invocation_dialect()
        .tcl_version
        .unwrap_or_else(|| interp.runtime_version());
    let opts = match core_switch::parse_options(interp, &argv[1..], version) {
        Ok(o) => o,
        Err(e) => return interp.report_cmd_error(e),
    };
    let value_idx = 1 + opts.value_index;
    let value = argv[value_idx];
    let rest = &argv[value_idx + 1..];

    // A single trailing argument is the `{pattern body ...}` list form; anything
    // else is inline pattern/body words.
    if rest.len() == 1 {
        switch_list_form(interp, &opts, value, rest[0], version)
    } else {
        switch_inline_form(interp, &opts, value, rest, version)
    }
}

/// Apply the shared core's [`Selection`] for a matched pattern: write any TIP #75
/// `-matchvar`/`-indexvar` values (trace-aware), returning whether they all
/// succeeded. The fresh value objects are adopted by [`write_var`] (or freed on a
/// failed write); the name objects are borrowed argv objects.
fn apply_writes(interp: &mut Interp, writes: Vec<(*mut TclObj, *mut TclObj)>) -> bool {
    for (name, val) in writes {
        if interp.assign_original_named_variable(name, val).is_err() {
            drop_fresh(val);
            return false;
        }
    }
    true
}

/// The inline form: `switch ?opts? str pat body ?pat body ...?`. Each body is a
/// live argument object, so a located literal runs as a `type source` frame via
/// [`Interp::eval_control_body`] (the same path as `if`/`while` bodies).
fn switch_inline_form(
    interp: &mut Interp,
    opts: &Options<*mut TclObj>,
    value: *mut TclObj,
    words: &[*mut TclObj],
    version: tcl_dialect::TclVersion,
) -> Code {
    let objc = words.len();
    if objc % 2 != 0 {
        return interp.report_cmd_error(core_switch::extra_pattern_error(false));
    }
    let npairs = objc / 2;
    // C rejects a trailing `-` body up front, citing the last *pattern*.
    let trailing = match core_switch::body_is_fallthrough(interp, &words[objc - 1]) {
        Ok(trailing) => trailing,
        Err(error) => return interp.report_cmd_error(error),
    };
    if trailing {
        let pat = match tcl_syntax::value::ValueOps::native_string_bytes(interp, &words[objc - 2]) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        return interp.report_cmd_error(core_switch::no_body_error(&pat));
    }
    // The pattern objects are the inline body args at even indices (borrowed argv).
    let patterns: Vec<*mut TclObj> = (0..npairs).map(|p| words[p * 2]).collect();
    interp.begin_control_options(ControlOptionPolicy::FRESH_FORWARDED);
    let matched = match core_switch::select_original_with_jim::<Interp, AreEngine, _, Code>(
        interp,
        opts,
        &value,
        &patterns,
        version,
        crate::cmd_regex::invoke_jim_regexp,
    ) {
        Ok(Selection::Matched { index, writes }) => {
            if !apply_writes(interp, writes) {
                return Code::Error;
            }
            index
        }
        Ok(Selection::NoMatch) => {
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
        Err(tcl_cmd_core::regex::OriginalRegexConsumerError::Command(error)) => {
            return interp.report_cmd_error(error);
        }
        Err(tcl_cmd_core::regex::OriginalRegexConsumerError::Callback(code)) => return code,
    };
    // Resolve a `-` fall-through to the next non-`-` body (guaranteed to exist).
    let mut b = matched;
    loop {
        match core_switch::body_is_fallthrough(interp, &words[b * 2 + 1]) {
            Ok(true) => b += 1,
            Ok(false) => break,
            Err(error) => return interp.report_cmd_error(error),
        }
    }
    let code = interp.eval_control_body(words[b * 2 + 1]);
    if code == Code::Error {
        arm_error_info(interp, &obj_bytes(words[matched * 2]));
    }
    code
}

/// The list form retains actual original List members. Located literal
/// elements use the original list extent for line tracking; pattern and body
/// execution always consumes the original member objects.
fn switch_list_form(
    interp: &mut Interp,
    opts: &Options<*mut TclObj>,
    value: *mut TclObj,
    list_obj: *mut TclObj,
    version: tcl_dialect::TclVersion,
) -> Code {
    use tcl_syntax::value::ValueOps;
    let members = match interp.list_elements(&list_obj) {
        Ok(members) => members,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    if members.is_empty() {
        return interp.wrong_args(core_switch::usage(version, true).as_bytes());
    }
    if members.len() % 2 != 0 {
        let mut has_comment = false;
        for pattern in members.iter().step_by(2) {
            match interp.native_string_bytes(pattern) {
                Ok(bytes) => has_comment |= bytes.first() == Some(&b'#'),
                Err(error) => return interp.report_cmd_error(error.into()),
            }
        }
        return interp.report_cmd_error(core_switch::extra_pattern_error(has_comment));
    }
    let last = members.len() - 1;
    let last_body = match interp.native_string_bytes(&members[last]) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    if tcl_core_types::c_string_extent(&last_body) == b"-" {
        let pattern = match interp.native_string_bytes(&members[last - 1]) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        return interp.report_cmd_error(core_switch::no_body_error(&pattern));
    }
    let loc = interp.arg_location(list_obj);
    let locations = if loc.is_some() {
        let bytes = match interp.native_string_bytes(&list_obj) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        match scan_elements(&bytes) {
            Ok(elems) if elems.len() == members.len() => Some((bytes, elems)),
            Ok(_) => {
                return interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "switch original list source geometry",
                    )
                    .into(),
                );
            }
            Err(error) => return interp.set_error(error),
        }
    } else {
        None
    };
    let pat_objs = members.iter().step_by(2).copied().collect::<Vec<_>>();
    interp.begin_control_options(ControlOptionPolicy::FRESH_FORWARDED);
    let outcome = core_switch::select_original_with_jim::<Interp, AreEngine, _, Code>(
        interp,
        opts,
        &value,
        &pat_objs,
        version,
        crate::cmd_regex::invoke_jim_regexp,
    );
    let matched = match outcome {
        Ok(Selection::Matched { index, writes }) => {
            if !apply_writes(interp, writes) {
                return Code::Error;
            }
            index
        }
        Ok(Selection::NoMatch) => {
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
        Err(tcl_cmd_core::regex::OriginalRegexConsumerError::Command(error)) => {
            return interp.report_cmd_error(error);
        }
        Err(tcl_cmd_core::regex::OriginalRegexConsumerError::Callback(code)) => return code,
    };
    let mut b = matched;
    while match interp.native_string_bytes(&members[b * 2 + 1]) {
        Ok(bytes) => tcl_core_types::c_string_extent(&bytes) == b"-",
        Err(error) => return interp.report_cmd_error(error.into()),
    } {
        b += 1;
    }
    let location = match (loc, locations.as_ref()) {
        (Some((file, line)), Some((bytes, elems))) if elems[b * 2 + 1].literal => Some((
            file,
            line + count_newlines(&bytes[..elems[b * 2 + 1].start()]),
        )),
        _ => None,
    };
    let code = interp.eval_original_control_body_location(members[b * 2 + 1], location);
    if code == Code::Error {
        let pattern = match interp.native_string_bytes(&pat_objs[matched]) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        arm_error_info(interp, &pattern);
    }
    code
}

/// Append the `("PATTERN" arm line N)` errorInfo frame (C's `SwitchPostProc`),
/// then clear the logged flag so the enclosing eval logs the `switch` command's
/// own `invoked from within` frame. `PATTERN` is the matched pattern, truncated
/// to 50 bytes with a trailing `...` (C's `limit`).
fn arm_error_info(interp: &mut Interp, pattern: &[u8]) {
    let overflow = pattern.len() > 50;
    let mut inner = Vec::with_capacity(pattern.len().min(50) + 8);
    inner.push(b'"');
    inner.extend_from_slice(if overflow { &pattern[..50] } else { pattern });
    if overflow {
        inner.extend_from_slice(b"...");
    }
    inner.extend_from_slice(b"\" arm");
    interp.append_frame_line(&inner);
    interp.clear_error_logged();
}

// list-element scanning (located bodies)

/// A located list element: its interior byte range and whether it is `literal`
/// (verbatim, no backslash collapse). `value.start` doubles as the line-tracking
/// anchor — an element's opening brace/quote shares a line with its interior, so
/// counting newlines to the interior start matches C's `element` anchor.
struct Elem {
    value: core::ops::Range<usize>,
    literal: bool,
}

impl Elem {
    fn start(&self) -> usize {
        self.value.start
    }
}

/// Scan `src` into its located list elements (the offset-aware complement to
/// `split_list`, sharing `tcl_syntax`'s element scanner).
fn scan_elements(src: &[u8]) -> Result<Vec<Elem>, &'static [u8]> {
    let mut elems = Vec::new();
    let mut pos = 0;
    loop {
        match tcl_syntax::list::find_element_bytes(src, pos) {
            Ok(Some(e)) => {
                pos = e.next;
                elems.push(Elem {
                    value: e.value,
                    literal: e.literal,
                });
            }
            Ok(None) => break,
            Err(err) => return Err(err.message().as_bytes()),
        }
    }
    Ok(elems)
}

/// Count the newlines in `s` (line delta between two offsets).
fn count_newlines(s: &[u8]) -> u32 {
    s.iter().filter(|&&b| b == b'\n').count() as u32
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    fn run(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        let code = i.eval_str(src);
        assert_eq!(
            code,
            Code::Ok,
            "eval {:?}: {:?}",
            String::from_utf8_lossy(src),
            i.result_bytes()
        );
        i.result_bytes()
    }

    fn err(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Error,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    #[test]
    fn switch_exact_glob_default_fallthrough() {
        leak_free(|i| {
            // list form, exact.
            assert_eq!(
                run(i, b"switch b {a {set r A} b {set r B} c {set r C}}"),
                b"B"
            );
            // default (last) matches anything.
            assert_eq!(
                run(i, b"switch -- z {a {set r 1} default {set r def}}"),
                b"def"
            );
            // `-` falls through to the next body.
            assert_eq!(
                run(i, b"switch x {a {set r 1} x - y {set r both} z {set r 3}}"),
                b"both"
            );
            // glob mode.
            assert_eq!(
                run(
                    i,
                    b"switch -glob foobar {f* {set r glob} default {set r no}}"
                ),
                b"glob"
            );
            // nocase exact.
            assert_eq!(
                run(i, b"switch -nocase ABC {abc {set r m} default {set r no}}"),
                b"m"
            );
            // inline pattern/body form.
            assert_eq!(run(i, b"switch 2 1 {set r one} 2 {set r two}"), b"two");
            // no match → empty.
            assert_eq!(run(i, b"switch q {a {set r 1} b {set r 2}}"), b"");
            i.eval_str(b"unset r");
        });
    }

    #[test]
    fn switch_propagates_body_code() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"switch a {a {break} b {continue}}"),
                Code::Break
            );
        });
    }

    #[test]
    fn switch_option_prefixes_and_double_mode() {
        leak_free(|i| {
            // Unambiguous option prefixes.
            assert_eq!(run(i, b"switch -exa Foo Foo {set result OK}"), b"OK");
            assert_eq!(run(i, b"switch -gl Foo Fo? {set result OK}"), b"OK");
            i.eval_str(b"unset result");
            // Two mode options conflict.
            assert_eq!(
                err(i, b"switch -exact -glob Foo Foo {x}"),
                b"bad option \"-glob\": -exact option already found"
            );
            // Unknown option.
            assert_eq!(
                err(i, b"switch -foo a b c"),
                b"bad option \"-foo\": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --"
            );
        });
    }

    #[test]
    fn switch_arg_errors() {
        leak_free(|i| {
            assert_eq!(
                err(i, b"switch"),
                b"wrong # args: should be \"switch ?-option ...? string ?pattern body ...? ?default body?\""
            );
            assert_eq!(
                err(i, b"switch x {}"),
                b"wrong # args: should be \"switch ?-option ...? string {?pattern body ...? ?default body?}\""
            );
            assert_eq!(err(i, b"switch a b"), b"extra switch pattern with no body");
            // Trailing `-` body cites the last pattern.
            assert_eq!(
                err(i, b"switch a {a - b - c -}"),
                b"no body specified for pattern \"c\""
            );
        });
    }

    #[test]
    fn switch_regexp_and_matchvars() {
        leak_free(|i| {
            assert_eq!(
                run(
                    i,
                    b"switch -regexp aaaab {^a*b$ {subst regexp} aaaab {subst exact} default {subst none}}"
                ),
                b"regexp"
            );
            // -matchvar captures the whole match and submatches.
            assert_eq!(
                run(i, b"switch -regexp -matchvar x -- abc {.(.). {set x}}"),
                b"abc b"
            );
            // -indexvar reports {start end} pairs.
            assert_eq!(
                run(i, b"switch -regexp -indexvar x -- abc {.(.). {set x}}"),
                b"{0 2} {1 1}"
            );
            // A non-participating group is {-1 -1}.
            assert_eq!(
                run(
                    i,
                    b"switch -regexp -indexvar x -- abcdef {^...(x)? {set x}}"
                ),
                b"{0 2} {-1 -1}"
            );
            // -matchvar without -regexp is rejected.
            assert_eq!(
                err(i, b"switch -glob -matchvar x -- abc . {set x}"),
                b"-matchvar option requires -regexp option"
            );
            // A bad pattern reports the compile error.
            assert_eq!(
                err(
                    i,
                    b"switch -regexp aaaab {*b {subst glob} default {subst none}}"
                ),
                b"cannot compile regular expression pattern: invalid quantifier operand"
            );
            i.eval_str(b"unset -nocomplain x");
        });
    }

    /// Tcl 9.0 reworded the compile-error prefix; tclsh 8.4.20 / 8.5.19 /
    /// 8.6.18 say `couldn't`, 9.0.4 / 9.1.0 `cannot`.
    #[test]
    fn regexp_compile_error_prefix_follows_the_release() {
        use tcl_dialect::TclVersion;
        for (version, verb) in [
            (TclVersion::V8_4, "couldn't"),
            (TclVersion::V8_5, "couldn't"),
            (TclVersion::V8_6, "couldn't"),
            (TclVersion::V9_0, "cannot"),
            (TclVersion::V9_1, "cannot"),
        ] {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(
                    String::from_utf8_lossy(&err(i, b"switch -regexp xa {( {set r hit}}")),
                    format!(
                        "{verb} compile regular expression pattern: parentheses () not balanced"
                    ),
                    "{version:?}"
                );
            });
        }
    }

    /// `-integer` (TIP 730) matches wide integers by value on Tcl 9.1, in both
    /// the inline and list forms. tclsh 9.1.0 prints `b` for
    /// `switch -integer 010 {8 {puts a} 10 {puts b}}`.
    #[test]
    fn switch_integer_matches_by_value_on_tcl91() {
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("tcl9.1"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .expect("selected original C9.1 constructor");
            let i = &mut interp;
            assert_eq!(
                run(i, b"switch -integer 010 {8 {subst a} 10 {subst b}}"),
                b"b"
            );
            assert_eq!(
                run(i, b"switch -int 0x10 16 {subst hex} 2 {subst two}"),
                b"hex"
            );
            assert_eq!(run(i, b"switch -integer { 16 } {0x10 {subst ws}}"), b"ws");
            assert_eq!(run(i, b"switch -integer 1_000 {1000 {subst us}}"), b"us");
            assert_eq!(
                run(i, b"switch -integer 2 {1 {subst a} default {subst d}}"),
                b"d"
            );
            assert_eq!(run(i, b"switch -integer 3 {1 {subst a} 2 {subst b}}"), b"");
            assert_eq!(run(i, b"switch -integer 2 {1 - 2 {subst ft}}"), b"ft");
            // Only the patterns reached are coerced.
            assert_eq!(
                run(i, b"switch -integer 1 {1 {subst a} abc {subst b}}"),
                b"a"
            );
        }
        assert_eq!(counters::finalize(), 0);
        assert_eq!(counters::double_free_count(), 0);
    }

    /// tclsh 9.1.0's error texts and codes for `-integer`.
    #[test]
    fn switch_integer_errors_match_tcl91() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V9_1);
            let cases: &[(&[u8], &[u8], &[u8])] = &[
                (
                    b"switch -integer abc {1 {subst a}}",
                    b"expected integer but got \"abc\"",
                    b"TCL VALUE NUMBER",
                ),
                (
                    b"switch -integer 2 1 {subst a} abc {subst b} default {subst d}",
                    b"expected integer but got \"abc\"",
                    b"TCL VALUE NUMBER",
                ),
                (
                    b"switch -integer 2 {default {subst d} 2 {subst two}}",
                    b"expected integer but got \"default\"",
                    b"TCL VALUE NUMBER",
                ),
                (
                    b"switch -integer 1 {99999999999999999999 {subst a}}",
                    b"integer value too large to represent",
                    b"ARITH IOVERFLOW {integer value too large to represent}",
                ),
                (
                    b"switch -nocase -integer 1 {1 {subst a}}",
                    b"-nocase option cannot be used with -integer option",
                    b"TCL OPERATION SWITCH MODERESTRICTION",
                ),
                (
                    b"switch -integer -glob 1 {1 {subst a}}",
                    b"bad option \"-glob\": -integer option already found",
                    b"TCL OPERATION SWITCH DOUBLEOPT",
                ),
                (
                    b"switch -i 1 {1 {subst a}}",
                    b"ambiguous option \"-i\": must be -exact, -glob, -indexvar, -integer, \
                      -matchvar, -nocase, -regexp, or --",
                    b"TCL LOOKUP INDEX option -i",
                ),
            ];
            for &(src, msg, code) in cases {
                assert_eq!(err(i, src), msg, "{}", String::from_utf8_lossy(src));
                assert_eq!(run(i, b"set ::errorCode"), code);
            }
        });
    }

    /// Before Tcl 9.1 `-integer` is an unknown option (tclsh 9.0.4 / 8.6.18).
    #[test]
    fn switch_integer_is_refused_before_tcl91() {
        for version in [tcl_dialect::TclVersion::V8_6, tcl_dialect::TclVersion::V9_0] {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(
                    err(i, b"switch -integer 1 {1 {subst a}}"),
                    b"bad option \"-integer\": must be -exact, -glob, -indexvar, \
                      -matchvar, -nocase, -regexp, or --"
                );
            });
        }
    }
}
