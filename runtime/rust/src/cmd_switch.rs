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
//! `-glob`, `-regexp`; plus `-nocase`, `--`, and the TIP #75 regexp side-channel
//! options `-matchvar`/`-indexvar`.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use tcl_cmd_core::switch::{self as core_switch, Options, Selection};

use crate::cmd_regex::AreEngine;
use crate::frame::split_array_ref;
use crate::interp::{drop_fresh, new_string, obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register `switch`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"switch", switch_cmd);
}

const USAGE_LIST: &[u8] = b"switch ?-option ...? string {?pattern body ...? ?default body?}";

fn switch_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // Option parsing + the `string` index are the shared core (`argv[1..]` strips
    // the command name to the name-stripped slice the core expects).
    let opts = match core_switch::parse_options(interp, &argv[1..]) {
        Ok(o) => o,
        Err(e) => return interp.set_error(e.message().as_bytes()),
    };
    let value_idx = 1 + opts.value_index;
    let value = argv[value_idx];
    let rest = &argv[value_idx + 1..];

    // A single trailing argument is the `{pattern body ...}` list form; anything
    // else is inline pattern/body words.
    if rest.len() == 1 {
        switch_list_form(interp, &opts, value, rest[0])
    } else {
        switch_inline_form(interp, &opts, value, rest)
    }
}

/// Apply the shared core's [`Selection`] for a matched pattern: write any TIP #75
/// `-matchvar`/`-indexvar` values (trace-aware), returning whether they all
/// succeeded. The fresh value objects are adopted by [`write_var`] (or freed on a
/// failed write); the name objects are borrowed argv objects.
fn apply_writes(interp: &mut Interp, writes: Vec<(*mut TclObj, *mut TclObj)>) -> bool {
    for (name, val) in writes {
        if write_var(interp, &obj_bytes(name), val).is_err() {
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
) -> Code {
    let objc = words.len();
    if objc % 2 != 0 {
        return interp.set_error(core_switch::extra_pattern_error(false).message().as_bytes());
    }
    let npairs = objc / 2;
    // C rejects a trailing `-` body up front, citing the last *pattern*.
    if obj_bytes(words[objc - 1]).as_slice() == b"-" {
        let pat = obj_bytes(words[objc - 2]);
        return interp.set_error(
            core_switch::no_body_error(&String::from_utf8_lossy(&pat))
                .message()
                .as_bytes(),
        );
    }
    // The pattern objects are the inline body args at even indices (borrowed argv).
    let patterns: Vec<*mut TclObj> = (0..npairs).map(|p| words[p * 2]).collect();
    let matched = match core_switch::select::<Interp, AreEngine, _>(interp, opts, &value, &patterns)
    {
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
        Err(e) => return interp.set_error(e.message().as_bytes()),
    };
    // Resolve a `-` fall-through to the next non-`-` body (guaranteed to exist).
    let mut b = matched;
    while obj_bytes(words[b * 2 + 1]).as_slice() == b"-" {
        b += 1;
    }
    let code = interp.eval_control_body(words[b * 2 + 1]);
    if code == Code::Error {
        arm_error_info(interp, &obj_bytes(words[matched * 2]));
    }
    code
}

/// The list form: `switch ?opts? str {pat body ...}`. The body is a sub-element
/// of the single list literal `list_obj`, so it has no `Tcl_Obj` of its own; its
/// `info frame` line is the list word's line plus the newlines preceding the
/// element (C's `TclListLines`). A `default` pattern (last) matches anything; a
/// `-` body falls through.
fn switch_list_form(
    interp: &mut Interp,
    opts: &Options<*mut TclObj>,
    value: *mut TclObj,
    list_obj: *mut TclObj,
) -> Code {
    let list_str = obj_bytes(list_obj);
    let elems = match scan_elements(&list_str) {
        Ok(e) => e,
        Err(e) => return interp.set_error(e),
    };
    if elems.is_empty() {
        return interp.wrong_args(USAGE_LIST);
    }
    if elems.len() % 2 != 0 {
        // The infamous "comment in switch" heuristic: a pattern beginning with
        // `#` in a braced body is almost certainly a misplaced comment.
        let has_comment = (0..elems.len())
            .step_by(2)
            .any(|p| list_str.get(elems[p].start()) == Some(&b'#'));
        return interp.set_error(
            core_switch::extra_pattern_error(has_comment)
                .message()
                .as_bytes(),
        );
    }
    let last = elems.len() - 1;
    if element_value(&list_str, &elems[last]).as_slice() == b"-" {
        let pat = element_value(&list_str, &elems[last - 1]);
        return interp.set_error(
            core_switch::no_body_error(&String::from_utf8_lossy(&pat))
                .message()
                .as_bytes(),
        );
    }
    let npairs = elems.len() / 2;
    let pat_bytes: Vec<Vec<u8>> = (0..npairs)
        .map(|p| element_value(&list_str, &elems[p * 2]))
        .collect();
    let loc = interp.arg_location(list_obj);

    // The list-form patterns are sub-strings of the literal (no `Tcl_Obj` of their
    // own), so mint temporary objects for the shared `select`, then free them — it
    // only reads them, and the result never references a pattern.
    let pat_objs: Vec<*mut TclObj> = pat_bytes.iter().map(|b| new_string(b)).collect();
    let outcome = core_switch::select::<Interp, AreEngine, _>(interp, opts, &value, &pat_objs);
    for &o in &pat_objs {
        drop_fresh(o);
    }
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
        Err(e) => return interp.set_error(e.message().as_bytes()),
    };
    let mut b = matched;
    while element_value(&list_str, &elems[b * 2 + 1]).as_slice() == b"-" {
        b += 1;
    }
    let body_elem = &elems[b * 2 + 1];
    let body = element_value(&list_str, body_elem);
    // Source-track only a literal body in a located list (a body with backslash
    // collapse, or a dynamic list, reverts to body-relative — C sets such lines
    // to -1).
    let code = match (&loc, body_elem.literal) {
        (Some((file, bline)), true) => {
            let line = bline + count_newlines(&list_str[..body_elem.start()]);
            interp.eval_located_body(file.clone(), line, &body)
        }
        _ => interp.eval_unlocated_body(&body),
    };
    if code == Code::Error {
        arm_error_info(interp, &pat_bytes[matched]);
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

/// Set the variable named by the (possibly `arr(idx)`) `name` to `obj`,
/// producing the C `can't set "name": ...` message on failure (and freeing the
/// unstored `obj`). Drives the TIP #75 `-matchvar`/`-indexvar` writes the shared
/// `select` produces (the name objects are borrowed; the value objects are fresh).
fn write_var(interp: &mut Interp, name: &[u8], obj: *mut TclObj) -> Result<(), ()> {
    let (base, elem) = split_array_ref(name);
    let stored = match &elem {
        Some(key) => interp.var_set_elem(&base, key, obj),
        None => interp.var_set(&base, obj),
    };
    match stored {
        Ok(()) => Ok(()),
        Err(e) => {
            drop_fresh(obj);
            crate::builtins::var_error(interp, name, e);
            Err(())
        }
    }
}

// -- list-element scanning (located bodies) ---------------------------------

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
    let s = core::str::from_utf8(src).map_err(|_| b"unmatched open brace in list".as_slice())?;
    let mut elems = Vec::new();
    let mut pos = 0;
    loop {
        match tcl_syntax::list::find_element(s, pos) {
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

/// The element's value bytes: verbatim for a literal (`{braced}`) element, else
/// backslash-collapsed (matching `split_list`).
fn element_value(src: &[u8], e: &Elem) -> Vec<u8> {
    if e.literal {
        src[e.value.clone()].to_vec()
    } else {
        tcl_syntax::backslash::decode_bytes(&src[e.value.clone()]).into_owned()
    }
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
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
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
}
