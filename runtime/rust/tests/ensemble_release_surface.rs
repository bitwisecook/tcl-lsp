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

//! A `TclMakeEnsemble` subcommand table is a **release** fact, and this engine
//! is release-selectable — so resolving against one release's table under
//! every pin is wrong twice over. It dispatches names the pinned release never
//! had (`info cmdtype` under 8.6), and, less visibly, a 9-only name changes
//! the prefix verdict for a word that has nothing to do with it: `dict g` is
//! `get` on 8.6 and ambiguous on 9.0, `string in` is `index` on 8.6 and
//! ambiguous on 9.0, `file te` is `tempfile` on 8.6 and ambiguous on 9.0.
//!
//! Every row below is byte-checked against `tmp/tcl8.6.16/unix/tclsh` and
//! `tmp/tcl9.0.4/unix/tclsh`. Rows the engine deliberately does not implement
//! are marked `UNIMPLEMENTED:` and assert this engine's documented answer, not
//! tclsh's.

use tcl_dialect::TclVersion;
use tcl_runtime::interp::{Code, Interp};

/// An interpreter pinned to `version`.
fn at(version: TclVersion) -> Interp {
    let mut interp = Interp::new();
    interp.set_runtime_version(version);
    interp
}

/// Run `src` and return `(code, result)`.
fn run(interp: &mut Interp, src: &str) -> (Code, String) {
    let code = interp.eval_str(src.as_bytes());
    (
        code,
        String::from_utf8_lossy(&interp.result_bytes()).into_owned(),
    )
}

/// `dict getdef`/`getwithdefault` (TIP 342) are Tcl 9.
///
/// tclsh8.6.16:  dict g {a 1} a          -> 1
///               dict getdef {a 1} z D   -> unknown or ambiguous subcommand
///                                          "getdef": must be append, …
/// tclsh9.0.4:   dict g {a 1} a          -> unknown or ambiguous subcommand
///                                          "g": must be … get, getdef,
///                                          getwithdefault, …
///               dict getdef {a 1} z D   -> D
#[test]
fn dict_get_prefix_is_unique_before_tcl9() {
    let mut old = at(TclVersion::V8_6);
    assert_eq!(run(&mut old, "dict g {a 1} a"), (Code::Ok, "1".to_string()));
    let (code, message) = run(&mut old, "dict getdef {a 1} z D");
    assert_eq!(code, Code::Error);
    assert_eq!(
        message,
        "unknown or ambiguous subcommand \"getdef\": must be append, create, exists, filter, \
         for, get, incr, info, keys, lappend, map, merge, remove, replace, set, size, unset, \
         update, values, or with"
    );

    let mut new = at(TclVersion::V9_0);
    let (code, message) = run(&mut new, "dict g {a 1} a");
    assert_eq!(code, Code::Error);
    assert_eq!(
        message,
        "unknown or ambiguous subcommand \"g\": must be append, create, exists, filter, for, \
         get, getdef, getwithdefault, incr, info, keys, lappend, map, merge, remove, replace, \
         set, size, unset, update, values, or with"
    );
    assert_eq!(
        run(&mut new, "dict getdef {a 1} z D"),
        (Code::Ok, "D".to_string())
    );
}

/// `string insert` is Tcl 9.
///
/// tclsh8.6.16:  string in abc 1        -> b   (`index`)
/// tclsh9.0.4:   string in abc 1        -> unknown or ambiguous subcommand
///                                         "in": must be … index, insert, is, …
///               string insert abc 1 X  -> aXbc
#[test]
fn string_index_prefix_is_unique_before_tcl9() {
    let mut old = at(TclVersion::V8_6);
    assert_eq!(
        run(&mut old, "string in abc 1"),
        (Code::Ok, "b".to_string())
    );
    let (code, message) = run(&mut old, "string insert abc 1 X");
    assert_eq!(code, Code::Error);
    assert!(
        message.starts_with("unknown or ambiguous subcommand \"insert\": must be cat, compare"),
        "{message}"
    );

    let mut new = at(TclVersion::V9_0);
    let (code, message) = run(&mut new, "string in abc 1");
    assert_eq!(code, Code::Error);
    assert!(
        message.starts_with("unknown or ambiguous subcommand \"in\": must be cat, compare"),
        "{message}"
    );
    assert!(message.contains("index, insert, is,"), "{message}");
    assert_eq!(
        run(&mut new, "string insert abc 1 X"),
        (Code::Ok, "aXbc".to_string())
    );
}

/// `info cmdtype`/`constant`/`consts` are Tcl 9. The exact-name row is the one
/// that shows a *resolution* miss must report rather than fall through: the
/// dispatch arms match the canonical name, so `info cmdtype` under an 8.6 pin
/// would otherwise still run.
///
/// tclsh8.6.16:  info cm            -> a count   (`cmdcount`)
///               info cmdtype set   -> unknown or ambiguous subcommand
///                                     "cmdtype": must be args, body, class, …
/// tclsh9.0.4:   info cm            -> unknown or ambiguous subcommand "cm":
///                                     must be … cmdcount, cmdtype, …
///               info cmdtype set   -> native
#[test]
fn info_cmdcount_prefix_is_unique_before_tcl9() {
    let mut old = at(TclVersion::V8_6);
    let (code, count) = run(&mut old, "info cm");
    assert_eq!(code, Code::Ok);
    assert!(
        count.bytes().all(|b| b.is_ascii_digit()) && !count.is_empty(),
        "info cm must be the cmdcount, got {count}"
    );
    let (code, message) = run(&mut old, "info cmdtype set");
    assert_eq!(code, Code::Error);
    assert_eq!(
        message,
        "unknown or ambiguous subcommand \"cmdtype\": must be args, body, class, cmdcount, \
         commands, complete, coroutine, default, errorstack, exists, frame, functions, globals, \
         hostname, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, \
         script, sharedlibextension, tclversion, or vars"
    );

    let mut new = at(TclVersion::V9_0);
    let (code, message) = run(&mut new, "info cm");
    assert_eq!(code, Code::Error);
    assert!(
        message.contains("cmdcount, cmdtype,"),
        "9.0 must advertise cmdtype: {message}"
    );
    assert_eq!(
        run(&mut new, "info cmdtype set"),
        (Code::Ok, "native".to_string())
    );
}

/// `chan isbinary` is Tcl 9; `pipe`/`pop`/`push` are 8.6.
///
/// tclsh8.6.16:  chan isbinary stdin -> unknown or ambiguous subcommand
///                                      "isbinary": must be blocked, close, …
/// UNIMPLEMENTED: under 9.0 this engine resolves the name and then declines it
/// — the release gate is what this test pins, not the missing feature.
#[test]
fn chan_isbinary_is_unknown_before_tcl9() {
    let mut old = at(TclVersion::V8_6);
    let (code, message) = run(&mut old, "chan isbinary stdin");
    assert_eq!(code, Code::Error);
    assert_eq!(
        message,
        "unknown or ambiguous subcommand \"isbinary\": must be blocked, close, configure, copy, \
         create, eof, event, flush, gets, names, pending, pipe, pop, postevent, push, puts, \
         read, seek, tell, or truncate"
    );

    let mut new = at(TclVersion::V9_0);
    let (code, message) = run(&mut new, "chan isbinary stdin");
    assert_eq!(code, Code::Error);
    assert_eq!(
        message,
        "chan isbinary is not supported under the WASM runtime"
    );
}

/// `array default`/`for` are Tcl 9.
///
/// tclsh8.6.16:  array f x -> unknown or ambiguous subcommand "f": must be …
/// tclsh9.0.4:   array f x -> wrong # args: should be "array for {key value}
///                            arrayName script"   (`for` resolves, then its
///                            own arity check speaks)
///
/// UNIMPLEMENTED: this engine's `array` table advertises only the subcommands
/// it dispatches, so its enumeration is shorter than tclsh's on both releases;
/// the release gate is still what decides whether `f` resolves at all.
#[test]
fn array_for_is_unknown_before_tcl9() {
    let mut old = at(TclVersion::V8_6);
    let (code, message) = run(&mut old, "array f x");
    assert_eq!(code, Code::Error);
    assert!(
        message.starts_with("unknown or ambiguous subcommand \"f\""),
        "{message}"
    );
    assert!(
        !message.contains("for"),
        "8.6 must not advertise for: {message}"
    );

    let mut new = at(TclVersion::V9_0);
    let (code, message) = run(&mut new, "array f x");
    assert_eq!(code, Code::Error);
    assert_eq!(
        message,
        "wrong # args: should be \"array for {key value} arrayName script\""
    );
}

/// `file home`/`tempdir`/`tildeexpand` are Tcl 9 — the row the VM's own
/// regression was found on, pinned here for this engine too.
///
/// tclsh8.6.16:  file te x -> a channel   (`tempfile`)
/// tclsh9.0.4:   file te x -> unknown or ambiguous subcommand "te": must be
///                            … tail, tempdir, tempfile, tildeexpand, …
#[test]
fn file_tempfile_prefix_is_unique_before_tcl9() {
    let mut old = at(TclVersion::V8_6);
    let (code, channel) = run(&mut old, "file te x");
    assert_eq!(code, Code::Ok, "{channel}");
    assert!(channel.starts_with("file"), "a channel name: {channel}");

    let mut new = at(TclVersion::V9_0);
    let (code, message) = run(&mut new, "file te x");
    assert_eq!(code, Code::Error);
    assert!(
        message.contains("tempdir, tempfile, tildeexpand,"),
        "{message}"
    );
}
