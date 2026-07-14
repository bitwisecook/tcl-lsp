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

//! The `clock` command — wall-clock readout + civil-date math, over the shared
//! [`tcl_cmd_core::clock`] core. The runtime reads the current time from its
//! host's [`Clock`](tcl_platform::Clock) and passes it (plus a local-offset
//! callback) into the host-free core; the fresh result object is retained by
//! `set_result`.

use tcl_cmd_core::clock as core_clock;

use crate::interp::{Code, Interp};
use crate::obj::TclObj;

/// Register `clock`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"clock", clock_cmd);
    // The ensemble's implementation members: the real `clock.tcl` rewrites
    // `clock` into an ensemble whose subcommands dispatch to `::tcl::clock::…`,
    // and library/framework code calls those fully-qualified forms directly.
    // Each prepends its subcommand and reuses the same dispatch.
    interp.register_builtin(b"::tcl::clock::seconds", clock_seconds);
    interp.register_builtin(b"::tcl::clock::milliseconds", clock_milliseconds);
    interp.register_builtin(b"::tcl::clock::microseconds", clock_microseconds);
    interp.register_builtin(b"::tcl::clock::clicks", clock_clicks);
    interp.register_builtin(b"::tcl::clock::format", clock_format);
    interp.register_builtin(b"::tcl::clock::add", clock_add);
    interp.register_builtin(b"::tcl::clock::scan", clock_scan);
}

/// Dispatch an `::tcl::clock::<sub>` ensemble member by prepending `sub` to the
/// argument list and reusing [`clock_cmd`].
fn clock_member(interp: &mut Interp, argv: &[*mut TclObj], sub: &[u8]) -> Code {
    let sub_obj = crate::obj::new_string_bytes(sub);
    let mut v: Vec<*mut TclObj> = Vec::with_capacity(argv.len() + 1);
    v.push(argv[0]);
    v.push(sub_obj);
    v.extend_from_slice(&argv[1..]);
    let code = clock_cmd(interp, &v);
    crate::interp::drop_fresh(sub_obj);
    code
}

fn clock_seconds(i: &mut Interp, a: &[*mut TclObj]) -> Code {
    clock_member(i, a, b"seconds")
}
fn clock_milliseconds(i: &mut Interp, a: &[*mut TclObj]) -> Code {
    clock_member(i, a, b"milliseconds")
}
fn clock_microseconds(i: &mut Interp, a: &[*mut TclObj]) -> Code {
    clock_member(i, a, b"microseconds")
}
fn clock_clicks(i: &mut Interp, a: &[*mut TclObj]) -> Code {
    clock_member(i, a, b"clicks")
}
fn clock_format(i: &mut Interp, a: &[*mut TclObj]) -> Code {
    clock_member(i, a, b"format")
}
fn clock_add(i: &mut Interp, a: &[*mut TclObj]) -> Code {
    clock_member(i, a, b"add")
}
fn clock_scan(i: &mut Interp, a: &[*mut TclObj]) -> Code {
    clock_member(i, a, b"scan")
}

fn clock_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let host = interp.host();
    let now = core_clock::Now {
        secs: host.clock().now_secs(),
        millis: i64::try_from(host.clock().now_millis()).unwrap_or(i64::MAX),
        micros: i64::try_from(host.clock().now_micros()).unwrap_or(i64::MAX),
    };
    let offset = move |ts: i64| host.clock().local_offset_secs(ts);
    match core_clock::dispatch(interp, &argv[1..], &now, &offset) {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(e) => interp.set_error(e.message().as_bytes()),
    }
}

// Module-gated on the tower: the sole test (and so the `leak_free`/`run`
// helpers) needs `expr` for its numeric asserts.
#[cfg(all(test, have_tommath))]
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

    #[test]
    fn clock_format_add_and_timing() {
        leak_free(|i| {
            // Civil-date formatting (UTC), pinned vs tclsh 9.0.
            assert_eq!(
                run(i, b"clock format 1700000000 -gmt 1"),
                b"Tue Nov 14 22:13:20 GMT 2023"
            );
            assert_eq!(
                run(
                    i,
                    b"clock format 1700000000 -format {%Y-%m-%d %H:%M:%S %j %u} -gmt 1"
                ),
                b"2023-11-14 22:13:20 318 2"
            );
            // Unknown specifier passes through (no %F in Tcl).
            assert_eq!(
                run(i, b"clock format 0 -format {%F %T} -gmt 1"),
                b"%F 00:00:00"
            );
            // clock add: fixed + calendar units.
            assert_eq!(
                run(i, b"clock format [clock add 1700000000 3 months -gmt 1] -format {%Y-%m-%d} -gmt 1"),
                b"2024-02-14"
            );
            // Timing.
            assert_eq!(run(i, b"expr {[clock seconds] > 1700000000}"), b"1");
            assert_eq!(
                run(i, b"expr {[clock microseconds] > 1700000000000000}"),
                b"1"
            );
            // `clock scan -format` round-trips `clock format`.
            assert_eq!(
                run(
                    i,
                    b"clock scan {2023-11-14 22:13:20} -format {%Y-%m-%d %H:%M:%S} -gmt 1"
                ),
                b"1700000000"
            );
            assert_eq!(
                run(i, b"clock scan {Nov 14 2023} -format {%b %d %Y} -gmt 1"),
                b"1699920000"
            );
            assert_eq!(
                i.eval_str(b"clock scan {2023-13-01} -format {%Y-%m-%d} -gmt 1"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"unable to convert input string: invalid month"
            );
            // Free-form scan (no -format) is the remaining piece.
            assert_eq!(i.eval_str(b"clock scan now"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"free-form clock scan is not yet supported"
            );
        });
    }
}
