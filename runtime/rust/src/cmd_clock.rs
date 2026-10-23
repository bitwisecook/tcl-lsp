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
use tcl_syntax::value::{ValueError, ValueOps};

use std::rc::Rc;

use crate::interp::{Code, Interp};
use crate::obj::TclObj;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "host_wasm.rs"]
mod test_browser_host;

struct ClockLiteralPool {
    format: crate::obj::Owned,
    locale: crate::obj::Owned,
    timezone: crate::obj::Owned,
    gmt: crate::obj::Owned,
    conflict: crate::obj::Owned,
    date_keys: [crate::obj::Owned; 14],
    ce: crate::obj::Owned,
    bce: crate::obj::Owned,
    integer_too_large: crate::obj::Owned,
}

impl ClockLiteralPool {
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
            date_keys: core_clock::C86_DATE_FIELD_KEYS
                .map(|key| crate::obj::Owned::fresh(crate::obj::new_string_bytes(key))),
            ce: crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"CE")),
            bce: crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"BCE")),
            integer_too_large: crate::obj::Owned::fresh(crate::obj::new_string_bytes(
                b"integer value too large to represent",
            )),
        }
    }

    fn date_literals(&self) -> core_clock::DateFieldLiterals<*mut TclObj> {
        core_clock::DateFieldLiterals {
            keys: std::array::from_fn(|index| self.date_keys[index].as_ptr()),
            ce: self.ce.as_ptr(),
            bce: self.bce.as_ptr(),
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

unsafe extern "C" fn delete_clock_pool(data: *mut std::ffi::c_void) {
    // SAFETY: each registration transfers one Rc to its once-only callback.
    unsafe { drop(Rc::from_raw(data.cast::<ClockLiteralPool>())) };
}

unsafe extern "C" fn parse_format_arguments(
    data: *mut std::ffi::c_void,
    interp: *mut Interp,
    count: std::ffi::c_int,
    words: *const *mut TclObj,
) -> std::ffi::c_int {
    // Keep the pool alive when a getter deletes its owning registrations.
    let pointer = data.cast::<ClockLiteralPool>();
    unsafe { Rc::increment_strong_count(pointer) };
    let pool = unsafe { Rc::from_raw(pointer) };
    let originals = pool.originals();
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
            interp.set_result(pool.conflict.as_ptr());
            interp.set_c_error_code(b"CLOCK gmtWithTimezone");
            1
        }
    }
}

unsafe extern "C" fn get_date_fields(
    data: *mut std::ffi::c_void,
    interp: *mut Interp,
    count: std::ffi::c_int,
    words: *const *mut TclObj,
) -> std::ffi::c_int {
    // The real registration owns data; the call lease survives callback deletion.
    let pointer = data.cast::<ClockLiteralPool>();
    unsafe { Rc::increment_strong_count(pointer) };
    let pool = unsafe { Rc::from_raw(pointer) };
    let interp = unsafe { &mut *interp };
    let Ok(count) = usize::try_from(count) else {
        interp.report_obj_cmd_error(tcl_cmd_core::CmdError::new("invalid argument count"));
        return 1;
    };
    let words = unsafe { std::slice::from_raw_parts(words, count) };
    let mut operations =
        match crate::value_ops::RuntimeClockDateOps::new(interp, pool.date_literals()) {
            Ok(operations) => operations,
            Err(error) => {
                interp.report_obj_cmd_error(error.into());
                return 1;
            }
        };
    if words.len() != 4 {
        let code = interp.wrong_args_for_invocation(words, b"seconds tzdata changeover");
        if !interp.host_refusal_pending() {
            interp.note_c_api_error();
        }
        if let Err(error) = operations.check_value() {
            interp.report_obj_cmd_error(error.into());
        }
        return if code == Code::Ok { 0 } else { 1 };
    }
    match core_clock::get_date_fields(&mut operations, &words[1..]) {
        Ok(result) => {
            let result = crate::obj::Owned::fresh(result);
            if let Err(error) = operations.check_value() {
                interp.report_obj_cmd_error(error.into());
                return 1;
            }
            interp.set_result(result.as_ptr());
            if let Err(error) = operations.check_value() {
                interp.report_obj_cmd_error(error.into());
                return 1;
            }
            0
        }
        Err(core_clock::DateFieldError::Command(error)) => {
            interp.report_obj_cmd_error(error);
            1
        }
        Err(core_clock::DateFieldError::BignumSeconds) => {
            if let Err(error) = operations.check_value() {
                interp.report_obj_cmd_error(error.into());
                return 1;
            }
            interp.set_result(pool.integer_too_large.as_ptr());
            interp.set_c_error_code(b"NONE");
            if let Err(error) = operations.check_value() {
                interp.report_obj_cmd_error(error.into());
            }
            1
        }
    }
}

fn getenv_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        let code = interp.wrong_args_for_invocation(argv, b"name");
        if !interp.host_refusal_pending() {
            interp.note_c_api_error();
        }
        return code;
    }
    let name = match interp.native_string_bytes(&argv[1]) {
        Ok(name) => name,
        Err(error) => return interp.report_obj_cmd_error(error.into()),
    };
    let host = interp.host();
    let Some(environment) = host.env().native_environment_bytes() else {
        return interp.report_obj_cmd_error(
            ValueError::CommandProtocolUnavailable("native environment byte getter").into(),
        );
    };
    let value = environment
        .get_bytes(tcl_core_types::c_string_extent(&name))
        .unwrap_or_default();
    if let Err(error) = interp.check_entered_native_operation() {
        return interp.refuse_native_execution(error);
    }
    let result = interp.new_bytes(&value);
    interp.set_result(result);
    Code::Ok
}

unsafe extern "C" fn clock_getenv(
    _data: *mut std::ffi::c_void,
    interp: *mut Interp,
    count: std::ffi::c_int,
    words: *const *mut TclObj,
) -> std::ffi::c_int {
    // SAFETY: the object-command owner supplies a live interpreter and argv.
    let interp = unsafe { &mut *interp };
    let Ok(count) = usize::try_from(count) else {
        interp.report_obj_cmd_error(tcl_cmd_core::CmdError::new("invalid argument count"));
        return 1;
    };
    let words = unsafe { std::slice::from_raw_parts(words, count) };
    if getenv_cmd(interp, words) == Code::Ok {
        0
    } else {
        1
    }
}

/// Register `clock`.
pub fn install(interp: &mut Interp) {
    if let Some(name) = interp
        .native_invocation_dialect()
        .clock_parse_format_args_registration()
    {
        let pool = Rc::new(ClockLiteralPool::new());
        if let Some(worker) = interp
            .native_invocation_dialect()
            .clock_get_date_fields_registration()
        {
            interp.register_stock_obj_command(
                worker.as_bytes(),
                crate::interp::ObjCommand::new_owned_runtime_shim(
                    get_date_fields,
                    Rc::into_raw(Rc::clone(&pool)).cast_mut().cast(),
                    Some(delete_clock_pool),
                ),
            );
        }
        if let Some(getenv) = interp
            .native_invocation_dialect()
            .clock_getenv_registration()
        {
            interp.register_stock_obj_command(
                getenv.as_bytes(),
                crate::interp::ObjCommand::new_owned_runtime_shim(
                    clock_getenv,
                    Rc::into_raw(Rc::clone(&pool)).cast_mut().cast(),
                    Some(delete_clock_pool),
                ),
            );
        }
        interp.register_stock_obj_command(
            name.as_bytes(),
            crate::interp::ObjCommand::new_owned_runtime_shim(
                parse_format_arguments,
                Rc::into_raw(Rc::clone(&pool)).cast_mut().cast(),
                Some(delete_clock_pool),
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
    interp.register_stock_ensemble(
        tcl_registry::invocation_words::EnsembleImplementationFamily::Clock,
        b"clock",
        clock_cmd,
        &[],
        &[],
    );
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
    let release = interp.runtime_version();
    match core_clock::dispatch(interp, &argv[1..], &now, &offset, release) {
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
    fn private_clock_workers_have_actual_selected_c86_shipped_backing_and_null_hooks() {
        for version in tcl_dialect::TclVersion::ALL {
            let profile = crate::environment::profile_for_dialect(version.dialect_profile_name());
            let dialect = tcl_registry::InvocationDialect::of_profile(profile);
            let registry = crate::environment::store_for_profile(profile);
            let interp = Interp::with_native_core(
                crate::interp::default_host(),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let report = interp.backing_report();
            for identity in [
                "tcl::clock::ParseFormatArgs",
                "tcl::clock::getenv",
                "tcl::clock::GetDateFields",
            ] {
                let selected = version == tcl_dialect::TclVersion::V8_6;
                let declared = registry
                    .get_for_surface(identity, dialect.authoring_query())
                    .map(|spec| spec.runtime_backing);
                let actual = report
                    .iter()
                    .find(|(name, _)| name == identity)
                    .map(|(_, backing)| *backing);
                assert_eq!(
                    declared,
                    selected.then_some(tcl_registry::RuntimeBacking::shipped(identity))
                );
                assert_eq!(
                    actual,
                    selected.then_some(tcl_runtime_api::RegisteredBacking::Builtin)
                );
                let token = interp.resolve_cmd_token(format!("::{identity}").as_bytes());
                assert_eq!(token.is_some(), selected);
                if let Some(token) = token {
                    assert!(matches!(
                        interp.namespaces().native_compiler_recipe(token),
                        Some(NativeCompilerRecipe::Absent)
                    ));
                }
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn getenv_preserves_actual_unix_bytes_and_current_host_view() {
        use std::os::unix::ffi::OsStringExt as _;
        use tcl_platform::Host as _;
        const MARKER: &str = "NH_RUNTIME_GETENV_CHILD";
        const KEY: &str = "NH_RUNTIME_GETENV_VALUE";
        const MISSING: &str = "NH_RUNTIME_GETENV_MISSING";
        const RAW_VALUE: &str = "NH_RUNTIME_GETENV_RAW_VALUE";
        let raw_key = b"NH_RUNTIME_GETENV_\xff";
        if std::env::var_os(MARKER).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("cmd_clock::bootstrap_tests::getenv_preserves_actual_unix_bytes_and_current_host_view")
                .arg("--nocapture")
                .env(MARKER, "1")
                .env(KEY, "actual")
                .env_remove(MISSING)
                .env(std::ffi::OsString::from_vec(raw_key.to_vec()), "raw-name")
                .env(RAW_VALUE, std::ffi::OsString::from_vec(vec![0xff]))
                .status().unwrap();
            assert!(
                status.success(),
                "actual Unix getter child failed: {status}"
            );
            return;
        }
        let host = std::rc::Rc::new(tcl_host_native::NativeHost::new());
        let mut interp = Interp::with_native_core(
            host.clone(),
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"set ::errorCode UNSET; set ::readCalls 0; proc readTrap {a b op} {incr ::readCalls; error SHOULD_NOT_READ_ENV_ARRAY}; trace add variable ::env(NH_RUNTIME_GETENV_VALUE) read readTrap; ::tcl::clock::getenv NH_RUNTIME_GETENV_VALUE"), Code::Ok);
        assert_eq!(interp.result_bytes(), b"actual");
        assert_eq!(
            crate::obj::bytes_of(interp.var_get(b"::readCalls").unwrap()),
            b"0"
        );
        assert_eq!(
            interp.eval_str(b"trace remove variable ::env(NH_RUNTIME_GETENV_VALUE) read readTrap"),
            Code::Ok
        );
        host.env().set(KEY, "updated");
        assert_eq!(interp.eval_str(b"list $::env(NH_RUNTIME_GETENV_VALUE) [::tcl::clock::getenv NH_RUNTIME_GETENV_VALUE]"), Code::Ok);
        // Tcl's built-in environment read trace observes the updated process
        // environment too; the earlier readTrap check isolates the clock getter.
        assert_eq!(interp.result_bytes(), b"updated updated");
        for (name, expected) in [
            (
                b"NH_RUNTIME_GETENV_VALUE\0suffix".as_slice(),
                b"updated".as_slice(),
            ),
            (raw_key.as_slice(), b"raw-name".as_slice()),
            (RAW_VALUE.as_bytes(), b"\xff".as_slice()),
            (MISSING.as_bytes(), b"".as_slice()),
        ] {
            let original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(name));
            interp.var_set(b"name", original.as_ptr()).unwrap();
            assert_eq!(interp.eval_str(b"::tcl::clock::getenv $name"), Code::Ok);
            assert_eq!(interp.result_bytes(), expected);
            assert_eq!(
                crate::obj::bytes_of(interp.var_get(b"::errorCode").unwrap()),
                b"UNSET"
            );
        }
        for command in [
            b"::tcl::clock::getenv".as_slice(),
            b"::tcl::clock::getenv a b",
        ] {
            assert_eq!(interp.eval_str(command), Code::Error);
            assert_eq!(
                interp.result_bytes(),
                b"wrong # args: should be \"::tcl::clock::getenv name\""
            );
            assert_eq!(interp.error_code(), b"TCL WRONGARGS");
        }
        assert_eq!(
            interp.eval_str(b"rename ::tcl::clock::getenv ::renamedGetter"),
            Code::Ok
        );
        assert_eq!(interp.eval_str(b"::renamedGetter a b"), Code::Error);
        assert_eq!(
            interp.result_bytes(),
            b"wrong # args: should be \"::renamedGetter name\""
        );
        assert_eq!(interp.error_code(), b"TCL WRONGARGS");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn absent_native_environment_capability_refuses_and_failed_creation_releases_pool_owner() {
        use super::{clock_getenv, delete_clock_pool, ClockLiteralPool};
        let host = std::rc::Rc::new(super::test_browser_host::BrowserHost::new());
        let mut interp = Interp::with_native_core(
            host,
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interp.eval_str(b"catch {::tcl::clock::getenv KEY} result; set after forbidden"),
            Code::Error
        );
        assert!(matches!(
            interp.native_execution_refusal(),
            Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native environment byte getter"
                )
            ))
        ));
        let first = interp.native_execution_refusal();
        let pool = std::rc::Rc::new(ClockLiteralPool::new());
        let installed = interp.register_stock_obj_command(
            b"::tcl::clock::unpublished",
            crate::interp::ObjCommand::new_owned_runtime_shim(
                clock_getenv,
                std::rc::Rc::into_raw(std::rc::Rc::clone(&pool))
                    .cast_mut()
                    .cast(),
                Some(delete_clock_pool),
            ),
        );
        assert!(installed.is_none());
        assert_eq!(std::rc::Rc::strong_count(&pool), 1);
        assert_eq!(interp.native_execution_refusal(), first);
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
        assert_eq!(unsafe { (*members[0]).ref_count }, 3);
        assert_eq!(interp.eval_str(b"rename ::tcl::clock::getenv {}"), Code::Ok);
        assert_eq!(interp.native_compiler_cache_epochs(GLOBAL), Some(before));
        assert_eq!(unsafe { (*members[0]).ref_count }, 3);
        assert_eq!(
            interp.eval_str(b"rename ::tcl::clock::GetDateFields {}"),
            Code::Ok
        );
        assert_eq!(unsafe { (*members[0]).ref_count }, 2);
        assert_eq!(crate::obj::bytes_of(members[0]), b"%a %b %d %H:%M:%S %Z %Y");
        assert_eq!(crate::obj::bytes_of(originals[0]), b"supplied");
    }

    #[test]
    fn deleting_getenv_first_keeps_parse_format_args_pool_until_its_retirement() {
        use tcl_syntax::value::ValueOps;
        let mut interp = c86();
        let before = interp.native_compiler_cache_epochs(GLOBAL).unwrap();
        assert_eq!(
            interp.eval_str(b"::tcl::clock::ParseFormatArgs 0"),
            Code::Ok
        );
        let result = crate::obj::Owned::retain(interp.get_obj_result());
        let members = interp.list_elements(&result.as_ptr()).unwrap();
        assert_eq!(interp.eval_str(b"rename ::tcl::clock::getenv {}"), Code::Ok);
        assert_eq!(unsafe { (*members[0]).ref_count }, 2);
        assert_eq!(
            interp.eval_str(b"::tcl::clock::ParseFormatArgs 0 -g 1"),
            Code::Ok
        );
        let later = crate::obj::Owned::retain(interp.get_obj_result());
        let again = interp.list_elements(&later.as_ptr()).unwrap();
        assert_eq!(members[0], again[0]);
        assert_eq!(unsafe { (*members[0]).ref_count }, 3);
        assert_eq!(
            interp.eval_str(b"rename ::tcl::clock::ParseFormatArgs {}"),
            Code::Ok
        );
        assert_eq!(unsafe { (*members[0]).ref_count }, 3);
        assert_eq!(
            interp.eval_str(b"rename ::tcl::clock::GetDateFields {}"),
            Code::Ok
        );
        assert_eq!(unsafe { (*members[0]).ref_count }, 2);
        assert_eq!(crate::obj::bytes_of(again[2]), b":GMT");
        assert_eq!(interp.native_compiler_cache_epochs(GLOBAL), Some(before));
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
        for member in [
            "add",
            "format",
            "scan",
            "Oldscan",
            "ConvertLocalToUTC",
            "GetJulianDayFromEraYearMonthDay",
            "GetJulianDayFromEraYearWeekDay",
        ] {
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

#[cfg(test)]
mod get_date_fields_tests {
    use super::*;
    use crate::obj;
    use tcl_syntax::value::ValueOps;

    fn original() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    #[test]
    fn get_date_fields_matches_original_c86_rows_except_unavailable_localtime() {
        let mut interp = original();
        let input = include_str!(
            "../../../experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-question/input.tcl"
        );
        let source = format!(
            "set ::rows {{}}\n{}\njoin $::rows \\n",
            input
                .lines()
                .filter(|line| !line.starts_with("emit empty-tzdata "))
                .collect::<Vec<_>>()
                .join("\n")
                .replace("puts [list ROW", "lappend ::rows [list ROW")
        );
        assert_eq!(
            interp.eval_str(source.as_bytes()),
            Code::Ok,
            "{:?}",
            interp.native_execution_refusal()
        );
        let expected = include_str!("../../../experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-question/capture-01/8.6.18/values/stdout.txt")
            .lines().filter(|line| line.starts_with("ROW ") && !line.starts_with("ROW empty-tzdata "))
            .collect::<Vec<_>>().join("\n");
        assert_eq!(interp.result_bytes(), expected.as_bytes());
        assert_eq!(expected.lines().count(), 32);
    }

    #[test]
    fn empty_localtime_refuses_before_guest_catch_continues() {
        let mut interp = original();
        assert_eq!(
            interp.eval_str(
                b"catch {::tcl::clock::GetDateFields 0 {} 2361222} result; set after forbidden"
            ),
            Code::Error
        );
        assert!(matches!(
            interp.native_execution_refusal(),
            Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native C86 localtime conversion"
                )
            ))
        ));
        let first = interp.native_execution_refusal();
        assert!(interp.var_get(b"after").is_none());
        assert_eq!(interp.native_execution_refusal(), first);
    }

    #[test]
    fn original_name_dict_keys_and_three_worker_retirement_preserve_native_roles() {
        crate::counters::reset();
        {
            let mut interp = original();
            let name = obj::Owned::fresh(obj::new_string_bytes(b"N\0\xff"));
            let start = obj::Owned::fresh(obj::new_string_bytes(b"-9223372036854775808"));
            let offset = obj::Owned::fresh(obj::new_string_bytes(b"0"));
            let ignored = obj::Owned::fresh(obj::new_string_bytes(b"ignored"));
            let row = obj::Owned::fresh(interp.new_list(vec![
                start.as_ptr(),
                offset.as_ptr(),
                ignored.as_ptr(),
                name.as_ptr(),
            ]));
            let table = obj::Owned::fresh(interp.new_list(vec![row.as_ptr()]));
            let head = obj::Owned::fresh(obj::new_string_bytes(b"::tcl::clock::GetDateFields"));
            let seconds = obj::Owned::fresh(obj::new_string_bytes(b"0"));
            let change = obj::Owned::fresh(obj::new_string_bytes(b"2361222"));
            let args = [
                head.as_ptr(),
                seconds.as_ptr(),
                table.as_ptr(),
                change.as_ptr(),
            ];
            assert_eq!(
                interp.eval_original_object_vector(&args),
                Code::Ok,
                "{:?}",
                interp.native_execution_refusal()
            );
            let first = obj::Owned::retain(interp.get_obj_result());
            assert!(std::ptr::eq(
                obj::obj_type_ptr(first.as_ptr()),
                &crate::dict::TCL_DICT_TYPE
            ));
            assert!(!obj::has_string_rep(first.as_ptr()));
            assert_eq!(
                crate::dict::dict_get(first.as_ptr(), b"tzName").unwrap(),
                Some(name.as_ptr())
            );
            assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 3);
            let keys = interp
                .dict_pairs(&first.as_ptr())
                .unwrap()
                .into_iter()
                .map(|(key, _)| key)
                .collect::<Vec<_>>();
            assert_eq!(interp.eval_original_object_vector(&args), Code::Ok);
            let second = obj::Owned::retain(interp.get_obj_result());
            assert_ne!(first.as_ptr(), second.as_ptr());
            assert_eq!(
                interp
                    .dict_pairs(&second.as_ptr())
                    .unwrap()
                    .into_iter()
                    .map(|(key, _)| key)
                    .collect::<Vec<_>>(),
                keys
            );
            assert_eq!(unsafe { (*keys[0]).ref_count }, 3);
            for worker in ["GetDateFields", "ParseFormatArgs"] {
                assert_eq!(
                    interp.eval_str(format!("rename ::tcl::clock::{worker} {{}}").as_bytes()),
                    Code::Ok
                );
                assert_eq!(unsafe { (*keys[0]).ref_count }, 3);
            }
            assert_eq!(interp.eval_str(b"rename ::tcl::clock::getenv {}"), Code::Ok);
            assert_eq!(unsafe { (*keys[0]).ref_count }, 2);
            drop(second);
            drop(first);
            drop(table);
            drop(row);
            assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
            assert_eq!(obj::bytes_of(name.as_ptr()), b"N\0\xff");
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }

    #[test]
    fn aliased_worker_arity_uses_original_invocation_spelling() {
        let mut interp = original();
        assert_eq!(
            interp.eval_str(b"rename ::tcl::clock::GetDateFields ::renamedDate; ::renamedDate 0"),
            Code::Error
        );
        assert_eq!(
            interp.result_bytes(),
            b"wrong # args: should be \"::renamedDate seconds tzdata changeover\""
        );
        assert_eq!(interp.error_code(), b"TCL WRONGARGS");
    }
    struct EffectHost {
        actual: Rc<dyn tcl_platform::Host>,
        effect: std::cell::RefCell<Option<Box<dyn Fn() -> bool>>>,
        abi_calls: std::cell::Cell<usize>,
    }
    impl tcl_platform::Host for EffectHost {
        fn capabilities(&self) -> tcl_platform::Capabilities {
            self.actual.capabilities()
        }
        fn clock(&self) -> &dyn tcl_platform::Clock {
            self.actual.clock()
        }
        fn stdio(&self) -> &dyn tcl_platform::StdIo {
            self.actual.stdio()
        }
        fn env(&self) -> &dyn tcl_platform::Env {
            self.actual.env()
        }
        fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
            self.actual.native_integer_formatter()
        }
        fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
            Some(self)
        }
    }
    impl EffectHost {
        fn actual(&self) -> &dyn tcl_platform::NumericEnvironment {
            self.actual
                .numeric_environment()
                .expect("actual native numeric environment")
        }
    }
    impl tcl_platform::NumericEnvironment for EffectHost {
        fn c_integer_abi(
            &self,
        ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable>
        {
            let result = self.actual().c_integer_abi();
            self.abi_calls.set(self.abi_calls.get() + 1);
            let effect = self.effect.borrow_mut().take();
            if let Some(effect) = effect {
                if !effect() {
                    self.effect.borrow_mut().replace(effect);
                }
            }
            result
        }
        fn state(
            &self,
        ) -> Result<tcl_platform::NumericErrorState, tcl_platform::NumericEnvironmentUnavailable>
        {
            self.actual().state()
        }
        fn reset(&self) -> Result<(), tcl_platform::NumericEnvironmentUnavailable> {
            self.actual().reset()
        }
        fn unsigned_c84(
            &self,
            input: &[u8],
            offset: usize,
            long: bool,
        ) -> Result<
            tcl_platform::UnsignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().unsigned_c84(input, offset, long)
        }
        fn unsigned(
            &self,
            input: &[u8],
            offset: usize,
            base: u32,
        ) -> Result<
            tcl_platform::UnsignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().unsigned(input, offset, base)
        }
        fn unsigned_long(
            &self,
            input: &[u8],
            offset: usize,
            base: u32,
        ) -> Result<
            tcl_platform::UnsignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().unsigned_long(input, offset, base)
        }
        fn signed_long(
            &self,
            input: &[u8],
            base: u32,
        ) -> Result<
            tcl_platform::SignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().signed_long(input, base)
        }
        fn double(
            &self,
            input: &[u8],
            reset: bool,
        ) -> Result<
            tcl_platform::DoubleNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().double(input, reset)
        }
    }

    fn delete_clock_workers_at_abi(interp: &mut Interp) -> Rc<EffectHost> {
        let host = Rc::new(EffectHost {
            actual: interp.host(),
            effect: Default::default(),
            abi_calls: Default::default(),
        });
        interp.set_host(host.clone());
        let owner = interp.downgrade();
        host.effect.borrow_mut().replace(Box::new(move || {
            let mut interp = owner.upgrade().unwrap();
            assert_eq!(
                interp.eval_str(b"rename ::tcl::clock::GetDateFields {}; rename ::tcl::clock::ParseFormatArgs {}; rename ::tcl::clock::getenv {}"),
                Code::Ok
            );
            true
        }));
        host
    }

    #[test]
    fn parse_format_conflict_pool_lease_survives_all_three_real_worker_deletions() {
        let mut interp = original();
        let host = delete_clock_workers_at_abi(&mut interp);
        let words = [
            b"::tcl::clock::ParseFormatArgs".as_slice(),
            b"0",
            b"-gmt",
            b"1",
            b"-timezone",
            b":UTC",
        ]
        .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)));
        let args = words.each_ref().map(obj::Owned::as_ptr);
        assert_eq!(interp.eval_original_object_vector(&args), Code::Error);
        assert!(host.effect.borrow().is_none());
        assert!(host.abi_calls.get() > 0);
        assert_eq!(interp.native_execution_refusal(), None);
        assert_eq!(
            interp.result_bytes(),
            b"cannot use -gmt and -timezone in same call"
        );
        assert_eq!(interp.error_code(), b"CLOCK gmtWithTimezone");
        let result = interp.get_obj_result();
        assert!(obj::check_native_liveness(result).is_ok());
        assert_eq!(unsafe { (*result).ref_count }, 1);
        for worker in ["GetDateFields", "ParseFormatArgs", "getenv"] {
            assert!(interp
                .resolve_cmd_token(format!("::tcl::clock::{worker}").as_bytes())
                .is_none());
        }
    }

    #[test]
    fn parse_format_success_pool_lease_survives_all_three_real_worker_deletions() {
        let mut interp = original();
        let host = delete_clock_workers_at_abi(&mut interp);
        let words = [
            b"::tcl::clock::ParseFormatArgs".as_slice(),
            b"0",
            b"-gmt",
            b"1",
        ]
        .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)));
        let args = words.each_ref().map(obj::Owned::as_ptr);
        assert_eq!(interp.eval_original_object_vector(&args), Code::Ok);
        assert!(host.effect.borrow().is_none());
        assert!(host.abi_calls.get() > 0);
        assert_eq!(interp.native_execution_refusal(), None);
        let result = interp.get_obj_result();
        assert!(std::ptr::eq(
            obj::obj_type_ptr(result),
            &crate::list::TCL_LIST_TYPE
        ));
        assert!(!obj::has_string_rep(result));
        let members = interp.list_elements(&result).unwrap();
        assert_eq!(members.len(), 3);
        for (member, expected) in
            members
                .into_iter()
                .zip([b"%a %b %d %H:%M:%S %Z %Y".as_slice(), b"C", b":GMT"])
        {
            assert!(obj::check_native_liveness(member).is_ok());
            assert_eq!(unsafe { (*member).ref_count }, 1);
            assert_eq!(obj::bytes_of(member), expected);
        }
        for worker in ["GetDateFields", "ParseFormatArgs", "getenv"] {
            assert!(interp
                .resolve_cmd_token(format!("::tcl::clock::{worker}").as_bytes())
                .is_none());
        }
    }

    #[test]
    fn reached_real_getter_context_table_or_row_effect_retains_first_host_cause() {
        use core_clock::NativeClockDateOps;
        for effect_kind in 0..3 {
            let mut interp = original();
            let host = Rc::new(EffectHost {
                actual: interp.host(),
                effect: Default::default(),
                abi_calls: Default::default(),
            });
            interp.set_host(host.clone());
            let table = obj::Owned::fresh(obj::new_string_bytes(b"{-9223372036854775808 0 0 GMT}"));
            let lease = obj::NativeObjectLifetime::retain(table.as_ptr());
            let owner = interp.downgrade();
            host.effect.borrow_mut().replace(Box::new(move || {
                if obj::obj_type_ptr(lease.as_ptr()) != &crate::list::TCL_LIST_TYPE {
                    return false;
                }
                if effect_kind != 0 {
                    let replaced = if effect_kind == 1 {
                        lease.as_ptr()
                    } else {
                        let backing = crate::list::native_list_backing(lease.as_ptr()).unwrap();
                        let row = backing.elements().unwrap()[0];
                        if obj::obj_type_ptr(row) != &crate::list::TCL_LIST_TYPE {
                            return false;
                        }
                        row
                    };
                    unsafe {
                        obj::set_string_rep(replaced, b"{}");
                    }
                    obj::discard_native_internal_representation(replaced).unwrap();
                } else {
                    let mut interp = owner.upgrade().unwrap();
                    let profile = interp.dialect_profile();
                    interp.set_dialect_profile(crate::environment::profile_for_dialect("tcl8.5"));
                    interp.set_dialect_profile(profile);
                }
                true
            }));
            let pool = ClockLiteralPool::new();
            let mut operations =
                crate::value_ops::RuntimeClockDateOps::new(&interp, pool.date_literals()).unwrap();
            let seconds = obj::Owned::fresh(obj::new_string_bytes(b"0"));
            let change = obj::Owned::fresh(obj::new_string_bytes(b"2361222"));
            interp.set_result_bytes(b"PRIOR");
            let context = interp.runtime_context();
            assert!(core_clock::get_date_fields(
                &mut operations,
                &[seconds.as_ptr(), table.as_ptr(), change.as_ptr()]
            )
            .is_err());
            assert!(
                host.effect.borrow().is_none(),
                "reached actual primitive effect"
            );
            let first = interp.native_execution_refusal();
            assert!(first.is_some());
            assert_eq!(interp.runtime_context(), context);
            assert_eq!(interp.result_bytes(), b"PRIOR");
            let reached = host.abi_calls.get();
            assert!(operations.date_wide(&seconds.as_ptr()).is_err());
            assert_eq!(host.abi_calls.get(), reached);
            assert_eq!(interp.native_execution_refusal(), first);
        }
    }

    #[test]
    fn active_call_pool_lease_survives_all_three_real_worker_deletions() {
        let mut interp = original();
        let host = Rc::new(EffectHost {
            actual: interp.host(),
            effect: Default::default(),
            abi_calls: Default::default(),
        });
        interp.set_host(host.clone());
        let table = obj::Owned::fresh(obj::new_string_bytes(b"{-9223372036854775808 0 0 GMT}"));
        let lease = obj::NativeObjectLifetime::retain(table.as_ptr());
        let owner = interp.downgrade();
        host.effect.borrow_mut().replace(Box::new(move || {
            if obj::obj_type_ptr(lease.as_ptr()) != &crate::list::TCL_LIST_TYPE { return false; }
            let mut interp = owner.upgrade().unwrap();
            assert_eq!(interp.eval_str(b"rename ::tcl::clock::GetDateFields {}; rename ::tcl::clock::ParseFormatArgs {}; rename ::tcl::clock::getenv {}"),Code::Ok);
            true
        }));
        let head = obj::Owned::fresh(obj::new_string_bytes(b"::tcl::clock::GetDateFields"));
        let seconds = obj::Owned::fresh(obj::new_string_bytes(b"0"));
        let change = obj::Owned::fresh(obj::new_string_bytes(b"2361222"));
        assert_eq!(
            interp.eval_original_object_vector(&[
                head.as_ptr(),
                seconds.as_ptr(),
                table.as_ptr(),
                change.as_ptr()
            ]),
            Code::Ok,
            "{:?}",
            interp.native_execution_refusal()
        );
        assert!(host.effect.borrow().is_none());
        assert!(std::ptr::eq(
            obj::obj_type_ptr(interp.get_obj_result()),
            &crate::dict::TCL_DICT_TYPE
        ));
        for worker in ["GetDateFields", "ParseFormatArgs", "getenv"] {
            assert!(interp
                .resolve_cmd_token(format!("::tcl::clock::{worker}").as_bytes())
                .is_none());
        }
        assert_eq!(
            obj::bytes_of(
                crate::dict::dict_get(interp.get_obj_result(), b"year")
                    .unwrap()
                    .unwrap()
            ),
            b"1970"
        );
    }
}
