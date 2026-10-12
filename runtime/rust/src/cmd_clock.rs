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

struct FormatArgumentPool {
    format: crate::obj::Owned,
    locale: crate::obj::Owned,
    timezone: crate::obj::Owned,
    gmt: crate::obj::Owned,
    conflict: crate::obj::Owned,
}

impl FormatArgumentPool {
    fn new() -> Self {
        Self {
            format: crate::obj::Owned::fresh(crate::obj::new_string_bytes(
                b"%a %b %d %H:%M:%S %Z %Y",
            )),
            locale: crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"C")),
            timezone: crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"")),
            gmt: crate::obj::Owned::fresh(crate::obj::new_string_bytes(b":GMT")),
            conflict: crate::obj::Owned::fresh(crate::obj::new_string_bytes(
                b"cannot use -gmt and -timezone in same call",
            )),
        }
    }

    fn originals(&self) -> core_clock::FormatArgumentLiterals<*mut TclObj> {
        core_clock::FormatArgumentLiterals {
            format: self.format.as_ptr(),
            locale: self.locale.as_ptr(),
            timezone: self.timezone.as_ptr(),
            gmt: self.gmt.as_ptr(),
        }
    }
}

unsafe extern "C" fn delete_format_argument_pool(data: *mut std::ffi::c_void) {
    // SAFETY: registration transfers this Box to its once-only delete callback.
    unsafe { drop(Box::from_raw(data.cast::<FormatArgumentPool>())) };
}

unsafe extern "C" fn parse_format_arguments(
    data: *mut std::ffi::c_void,
    interp: *mut Interp,
    count: std::ffi::c_int,
    words: *const *mut TclObj,
) -> std::ffi::c_int {
    // SAFETY: the object-command owner supplies live client data and argv.
    let originals = unsafe { &*data.cast::<FormatArgumentPool>() }.originals();
    let interp = unsafe { &mut *interp };
    let Ok(count) = usize::try_from(count) else {
        interp.report_obj_cmd_error(tcl_cmd_core::CmdError::new("invalid argument count"));
        return 1;
    };
    let words = unsafe { std::slice::from_raw_parts(words, count) };
    match core_clock::parse_format_args(interp, &words[1..], &originals) {
        Ok(result) => {
            interp.set_result(result);
            0
        }
        Err(core_clock::FormatArgumentError::Command(error)) => {
            interp.report_obj_cmd_error(error);
            1
        }
        Err(core_clock::FormatArgumentError::GmtWithTimezone) => {
            // SAFETY: no callback has retired client data during successful conversions.
            let conflict = unsafe { &*data.cast::<FormatArgumentPool>() }
                .conflict
                .as_ptr();
            interp.set_result(conflict);
            interp.set_c_error_code(b"CLOCK gmtWithTimezone");
            1
        }
    }
}

/// Register `clock`.
pub fn install(interp: &mut Interp) {
    if let Some(name) = interp
        .native_invocation_dialect()
        .clock_parse_format_args_registration()
    {
        let pool = Box::into_raw(Box::new(FormatArgumentPool::new()));
        interp.register_stock_obj_command(
            name.as_bytes(),
            crate::interp::ObjCommand::new_owned_runtime_shim(
                parse_format_arguments,
                pool.cast(),
                Some(delete_format_argument_pool),
            ),
        );
    }
    if interp
        .native_invocation_dialect()
        .clock_bootstrap_ensemble()
        .is_some()
    {
        interp.register_stock_ensemble(
            tcl_registry::invocation_words::EnsembleImplementationFamily::Clock,
            b"clock",
            clock_cmd,
            CLOCK_READING_MEMBERS,
            &[b"clicks", b"seconds", b"milliseconds", b"microseconds"],
        );
        return;
    }
    interp.register_stock_builtin(b"clock", clock_cmd);
    // The ensemble's implementation members: the real `clock.tcl` rewrites
    // `clock` into an ensemble whose subcommands dispatch to `::tcl::clock::…`,
    // and library/framework code calls those fully-qualified forms directly.
    // Each prepends its subcommand and reuses the same dispatch.
    interp.register_stock_builtin(b"::tcl::clock::seconds", clock_seconds);
    interp.register_stock_builtin(b"::tcl::clock::milliseconds", clock_milliseconds);
    interp.register_stock_builtin(b"::tcl::clock::microseconds", clock_microseconds);
    interp.register_stock_builtin(b"::tcl::clock::clicks", clock_clicks);
    interp.register_stock_builtin(b"::tcl::clock::format", clock_format);
    interp.register_stock_builtin(b"::tcl::clock::add", clock_add);
    interp.register_stock_builtin(b"::tcl::clock::scan", clock_scan);
}

const CLOCK_READING_MEMBERS: &[(&[u8], crate::interp::BuiltinFn)] = &[
    (b"clicks", clock_clicks),
    (b"seconds", clock_seconds),
    (b"milliseconds", clock_milliseconds),
    (b"microseconds", clock_microseconds),
];

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
        Err(e) => interp.report_cmd_error(e),
    }
}

#[cfg(test)]
mod bootstrap_tests {
    use crate::interp::{Code, Interp};
    use crate::namespace::{NativeCompilerRecipe, GLOBAL};

    fn c86() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    #[test]
    fn parse_format_args_matches_the_eleven_original_c86_cases() {
        let mut interp = c86();
        let input = include_str!(
            "../../../experiments/name-handling/results/clock-parse-format-args/input.tcl"
        );
        let input = format!(
            "set rows {{}}\n{}\njoin $rows \\n",
            input.replace("puts [list", "lappend rows [list")
        );
        assert_eq!(
            interp.eval_str(input.as_bytes()),
            Code::Ok,
            "{:?}",
            interp.result_bytes()
        );
        assert_eq!(
            interp.result_bytes(),
            include_bytes!("../../../experiments/name-handling/results/clock-parse-format-args/captured/c8.6.18.stdout.txt").strip_suffix(b"\n").unwrap(),
        );
    }

    #[test]
    fn parse_format_args_retains_supplied_objects_and_pool_members_after_retirement() {
        use tcl_syntax::value::ValueOps;
        let mut interp = c86();
        let name = b"::tcl::clock::ParseFormatArgs";
        let token = interp.resolve_cmd_token(name).unwrap();
        assert!(matches!(
            interp.namespaces().native_compiler_recipe(token),
            Some(NativeCompilerRecipe::Absent)
        ));
        let before = interp.native_compiler_cache_epochs(GLOBAL).unwrap();
        assert_eq!(
            interp.eval_str(b"::tcl::clock::ParseFormatArgs 0"),
            Code::Ok
        );
        let first = crate::obj::Owned::retain(interp.get_obj_result());
        let members = interp.list_elements(&first.as_ptr()).unwrap();
        assert_eq!(unsafe { (*members[0]).ref_count }, 2);
        assert_eq!(
            interp.eval_str(b"::tcl::clock::ParseFormatArgs 0"),
            Code::Ok
        );
        let second = crate::obj::Owned::retain(interp.get_obj_result());
        assert_eq!(interp.list_elements(&second.as_ptr()).unwrap(), members);
        assert_eq!(unsafe { (*members[0]).ref_count }, 3);
        assert_eq!(interp.eval_str(b"set fmt supplied; set loc custom; set tz :UTC; ::tcl::clock::ParseFormatArgs 0 -format $fmt -locale $loc -timezone $tz"), Code::Ok);
        let supplied = crate::obj::Owned::retain(interp.get_obj_result());
        let originals = interp.list_elements(&supplied.as_ptr()).unwrap();
        for (index, variable) in [b"fmt".as_slice(), b"loc", b"tz"].into_iter().enumerate() {
            assert_eq!(originals[index], interp.var_get(variable).unwrap());
        }
        assert_eq!(
            interp.eval_str(b"rename ::tcl::clock::ParseFormatArgs {}"),
            Code::Ok
        );
        assert_eq!(interp.native_compiler_cache_epochs(GLOBAL), Some(before));
        assert_eq!(unsafe { (*members[0]).ref_count }, 2);
        assert_eq!(crate::obj::bytes_of(members[0]), b"%a %b %d %H:%M:%S %Z %Y");
        assert_eq!(crate::obj::bytes_of(originals[0]), b"supplied");
    }

    #[test]
    fn c86_clock_scalar_births_preserve_epochs_and_reading_replacement_advances_them() {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        for member in ["clicks", "seconds", "milliseconds", "microseconds"] {
            let name = format!("::tcl::clock::{member}");
            let token = interp.resolve_cmd_token(name.as_bytes()).unwrap();
            assert!(matches!(
                interp.namespaces().native_compiler_recipe(token),
                Some(NativeCompilerRecipe::Registered { .. })
            ));
        }
        for member in ["add", "format", "scan", "getenv"] {
            assert!(interp
                .resolve_cmd_token(format!("::tcl::clock::{member}").as_bytes())
                .is_none());
        }
        let before = interp.native_compiler_cache_epochs(GLOBAL).unwrap();
        assert_eq!(
            interp.eval_str(b"proc boot {} {proc ::tcl::clock::add args {return deferred}; proc ::tcl::clock::format args {return deferred}; proc ::tcl::clock::scan args {return deferred}; set marker after}; boot"),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"after");
        assert_eq!(interp.native_compiler_cache_epochs(GLOBAL), Some(before));
        assert_eq!(interp.eval_str(b"clock ad"), Code::Ok);
        assert_eq!(interp.result_bytes(), b"deferred");
        assert_eq!(
            interp.eval_str(b"proc ::tcl::clock::seconds args {}"),
            Code::Ok
        );
        assert_eq!(
            interp.native_compiler_cache_epochs(GLOBAL),
            Some((before.0 + 1, before.1))
        );
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
            // Free-form scan (without -format) is not implemented; it errors.
            assert_eq!(i.eval_str(b"clock scan now"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"free-form clock scan is not yet supported"
            );
        });
    }
}
