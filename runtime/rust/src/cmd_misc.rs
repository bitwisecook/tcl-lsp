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

/// `encoding`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it.
/// 9.0's table also carries `profiles` and `user`, which need the encoding
/// machinery this runtime does not model. `dirs` arrives in 8.5, so the table
/// is filtered to the emulated release before the scan.
const ENCODING_SUBS: &[&[u8]] = &[b"convertfrom", b"convertto", b"dirs", b"names", b"system"];

fn encoding_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"encoding subcommand ?arg ...?");
    }
    let word = obj_bytes(argv[1]);
    let subs = crate::environment::release_subcommands(
        interp.runtime_version().dialect_profile_name(),
        "encoding",
        ENCODING_SUBS,
    );
    let Some(index) = tcl_cmd_core::ensemble::resolve_subcommand(subs, &word, true) else {
        return interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
            subs,
            &word,
            true,
            b"::tcl::encoding",
        ));
    };
    match subs[index] {
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
        other => interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
            subs,
            other,
            true,
            b"::tcl::encoding",
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::interp::{Code, Interp};

    /// Issue #1607: `encoding` is a `TclMakeEnsemble` command, so its
    /// exact-then-unique-prefix scan and its miss sentence belong to
    /// `tcl_cmd_core::ensemble`; this matched exactly and spelled the sentence
    /// out by hand. The list still names only what this runtime implements
    /// (9.0's table also carries `profiles` and `user`).
    ///
    /// tclsh 8.6.16 / 9.0.4 (the verdicts, not the shortened list):
    ///   encoding s  -> the system encoding
    ///   encoding c  -> unknown or ambiguous subcommand "c": must be …
    ///                  (convertfrom/convertto)
    ///   encoding {} -> unknown or ambiguous subcommand "": must be …
    #[test]
    fn encoding_ensemble_resolves_like_tclsh() {
        const MUST: &str = "must be convertfrom, convertto, dirs, names, or system";
        let mut i = Interp::new();
        assert_eq!(i.eval_str(b"encoding s"), Code::Ok);
        assert_eq!(i.result_bytes(), b"utf-8");
        assert_eq!(i.eval_str(b"encoding c"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("unknown or ambiguous subcommand \"c\": {MUST}").as_bytes()
        );
        assert_eq!(i.eval_str(b"encoding {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("unknown or ambiguous subcommand \"\": {MUST}").as_bytes()
        );
        // A unique prefix resolves.
        assert_eq!(i.eval_str(b"encoding na"), Code::Ok);
        assert_eq!(i.result_bytes(), b"utf-8 unicode ascii iso8859-1");
    }

    /// Issue #1607: `clock` is an ensemble too — its list was spelled out
    /// beside the dispatch and matched exactly, so `clock se` failed.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   clock se -> the seconds count
    ///   clock m  -> unknown or ambiguous subcommand "m": must be add, clicks,
    ///               format, microseconds, milliseconds, scan, or seconds
    ///   clock {} -> unknown or ambiguous subcommand "": must be <same>
    #[test]
    fn clock_ensemble_resolves_like_tclsh() {
        const MUST: &str = "must be add, clicks, format, microseconds, milliseconds, scan, \
                            or seconds";
        let mut i = Interp::new();
        assert_eq!(i.eval_str(b"clock se"), Code::Ok);
        assert_eq!(i.eval_str(b"clock m"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("unknown or ambiguous subcommand \"m\": {MUST}").as_bytes()
        );
        assert_eq!(i.eval_str(b"clock {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("unknown or ambiguous subcommand \"\": {MUST}").as_bytes()
        );
    }
}
