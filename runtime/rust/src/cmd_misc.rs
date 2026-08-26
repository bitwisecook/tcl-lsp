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

//! Small host/misc commands needed to bootstrap the real library (M2).
//!
//! `encoding` is near-trivial because UTF-8 is the internal string rep (the
//! cross-cutting contract): `convertto`/`convertfrom` pass through, `system` is
//! `utf-8`, and `dirs` is a no-op store (we don't load encoding files). C ref
//! `tclEncoding.c`. Non-UTF-8 codecs are a deferred edge translation.

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register the misc bootstrap commands.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"encoding", encoding_cmd);
    // The `clock` C subsystem is L3; init.tcl's startup calls this configure
    // hook unconditionally — accept it as a no-op until `clock` lands.
    interp.register_builtin(b"::tcl::unsupported::clock::configure", noop);
    // `tcl::build-info` — build metadata; the tcltest helper (`tcltests.tcl`)
    // queries it for the `debug`/`purify`/`memdebug`/`deprecated` constraints.
    interp.register_builtin(b"::tcl::build-info", build_info_cmd);
    // `pid ?channelId?` — the process id (a channel argument would list the pids
    // of the pipeline behind it, which the WASM runtime has none of).
    interp.register_builtin(b"pid", pid_cmd);
    // Commands with a registry spec but no portable WASM backing: report an
    // explicit "not supported under the WASM runtime" error rather than the
    // generic `invalid command name` an unregistered command yields.
    // Each needs an OS process, sockets, native loading, or the
    // event loop, none of which the single-threaded WASM tier provides.
    for name in [
        b"exec".as_slice(),
        b"socket",
        b"load",
        b"fileevent",
        b"fcopy",
    ] {
        interp.register_builtin(name, unsupported_cmd);
    }
}

/// `pid ?channelId?` — the current process id, or the empty list for a channel
/// argument (the WASM tier runs no external pipelines).
fn pid_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.len() {
        1 => {
            interp.set_result_bytes(std::process::id().to_string().as_bytes());
            Code::Ok
        }
        2 => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        _ => interp.wrong_args(b"pid ?channelId?"),
    }
}

/// A command that is genuinely not portable to the single-threaded WASM runtime
/// (external process, socket, native load, or event loop). A clear error keeps
/// it distinct from an unimplemented or mistyped command.
fn unsupported_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let name = obj_bytes(argv[0]);
    let mut m = b"\"".to_vec();
    m.extend_from_slice(&name);
    m.extend_from_slice(b"\" is not supported under the WASM runtime");
    interp.set_error(&m)
}

/// `tcl::build-info ?option?` — mirrors C's `BuildInfoObjCmd` (`tclBasic.c`):
/// no arg → the full string; `patchlevel` → up to `+`; `version` → up to the
/// second `.`; `commit` → the `+`..`.` segment; any other identifier → its
/// `name-value` suffix value, or boolean 1/0 for its presence.
///
/// The string is composed from the *pinned* release rather than a constant,
/// and both the composition and the field split live in `tcl_dialect`, so
/// this cannot answer differently from `tcl-vm` for the same pin (ledger
/// row B4).
fn build_info_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 2 {
        return interp.wrong_args(b"tcl::build-info ?option?");
    }
    let data = crate::version::build_info(interp.runtime_version());
    if argv.len() < 2 {
        interp.set_result_bytes(data.as_bytes());
        return Code::Ok;
    }
    // A non-UTF-8 option word cannot name a build field; the ordinary
    // "no such suffix word" answer covers it, exactly as a mistyped one.
    let option = obj_bytes(argv[1]);
    let result = match core::str::from_utf8(&option) {
        Ok(option) => tcl_dialect::build_info::query(&data, option),
        Err(_) => "0",
    };
    interp.set_result_bytes(result.as_bytes());
    Code::Ok
}

/// A no-op command (returns the empty string) — a placeholder for a C subsystem
/// hook the bootstrap invokes but doesn't depend on the result of.
fn noop(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(b"");
    Code::Ok
}

fn encoding_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"encoding subcommand ?arg ...?");
    }
    match obj_bytes(argv[1]).as_slice() {
        // `encoding dirs ?list?` — we don't search encoding files; accept + ignore.
        b"dirs" => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        b"system" => {
            interp.set_result_bytes(b"utf-8");
            Code::Ok
        }
        b"names" => {
            interp.set_result_bytes(b"utf-8 unicode ascii iso8859-1");
            Code::Ok
        }
        // `convertto`/`convertfrom ?encoding? data` — pass through (UTF-8 internal).
        b"convertto" | b"convertfrom" => {
            let Some(&data) = argv.last() else {
                return interp.wrong_args(b"encoding convertto ?encoding? data");
            };
            interp.set_result(data);
            Code::Ok
        }
        other => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be convertfrom, convertto, dirs, names, or system");
            interp.set_error(&m)
        }
    }
}
