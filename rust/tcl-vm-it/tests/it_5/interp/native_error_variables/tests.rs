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

//! `::errorInfo` and `::errorCode` across repeated errors and 8.4's traced publication, against the tclsh captures.

use crate::prelude::*;

fn lower_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::new();
    for byte in bytes {
        write!(&mut text, "{byte:02x}").expect("writing to a String cannot fail");
    }
    text
}

fn native_vm(profile: &str) -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile(profile))
}

fn error_info_has_first(vm: &mut Vm) -> bool {
    crate::fixture::eval(vm, "set ::errorInfo")
        .result
        .string_bytes()
        .windows(b"FIRST_ERROR_LONG".len())
        .any(|bytes| bytes == b"FIRST_ERROR_LONG")
}

#[test]
fn public_error_resets_preserve_global_headers_and_start_fresh_episodes() {
    let observations = include_str!(
        "../../../../../tcl-registry/tests/data/native_error_variables/reset-episodes.tsv"
    );
    for (version, profile) in [
        ("8.4.20", "tcl8.4"),
        ("8.5.19", "tcl8.5"),
        ("8.6.18", "tcl8.6"),
        ("9.0.4", "tcl9.0"),
        ("9.1.0", "tcl9.1"),
    ] {
        let row = observations
            .lines()
            .find(|row| row.starts_with(version))
            .unwrap();
        let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        let code = |index: usize| Code::from_int(fields[index].parse().unwrap());
        let mut vm = native_vm(profile);
        let defined = crate::fixture::eval(&mut vm, "proc p {} {error FIRST_ERROR_LONG}");
        assert_eq!(defined.code, code(1), "{profile}");
        assert_eq!(
            crate::fixture::call(&mut vm, "p", &[]).code,
            code(2),
            "{profile}"
        );
        let redefined = crate::fixture::eval(&mut vm, "proc p {} {error X}");
        assert_eq!(redefined.code, code(5), "{profile}");
        let completion = crate::fixture::call(&mut vm, "p", &[]);
        assert_eq!(completion.code, code(8), "{profile}");
        assert_eq!(
            completion.result.string_bytes().as_ref() == b"X",
            fields[10] == "1",
            "{profile}"
        );
        assert_eq!(
            !error_info_has_first(&mut vm),
            fields[11] == "1",
            "{profile}"
        );
        assert_eq!(
            crate::fixture::call(&mut vm, "p", &[]).code,
            code(12),
            "{profile}"
        );
        assert_eq!(
            !error_info_has_first(&mut vm),
            fields[13] == "1",
            "{profile}"
        );
    }
}

thread_local! {
    static C84_CODE_EVENTS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A variable trace recording each access to `::errorInfo` and
/// `::errorCode` with the value it sees.
fn observe_c84_code(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let name = crate::fixture::bytes(vm, &args[0]);
    let operation = crate::fixture::bytes(vm, &args[2]);
    let rooted = if name.starts_with(b"::") {
        name.clone()
    } else {
        [b"::".as_slice(), name.as_slice()].concat()
    };
    let value = vm
        .get_var_bytes(&rooted)
        .expect("actual traced global cell");
    let hex = lower_hex(&crate::fixture::bytes(vm, &value));
    C84_CODE_EVENTS.with(|events| {
        events.borrow_mut().push(format!(
            "{}|{}|{hex}",
            String::from_utf8(name).unwrap(),
            String::from_utf8(operation).unwrap(),
        ));
    });
    Completion::new(Code::Ok, Value::empty(), Value::empty())
}

fn c84_code_vm() -> Vm {
    let mut vm = native_vm("tcl8.4");
    vm.register("observe", observe_c84_code);
    assert_eq!(
        crate::fixture::eval(
            &mut vm,
            "trace variable ::errorInfo rw observe;trace variable ::errorCode rw observe"
        )
        .code,
        Code::Ok
    );
    C84_CODE_EVENTS.with(|events| events.borrow_mut().clear());
    vm
}

fn check_c84_code_controls(controls: &str, sources: &[(&str, &str)]) {
    for row in controls.lines() {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        let source = sources
            .iter()
            .find(|(name, _)| *name == fields[0])
            .unwrap()
            .1;
        let mut vm = c84_code_vm();
        let completion = crate::fixture::eval(&mut vm, source);
        assert_eq!(completion.code.as_int().to_string(), fields[1], "{row}");
        assert_eq!(
            lower_hex(&crate::fixture::bytes(&vm, &completion.result)),
            fields[2],
            "{row}"
        );
        C84_CODE_EVENTS.with(|events| assert_eq!(events.borrow().join(";"), fields[3], "{row}"));
    }
}

#[test]
fn c84_global_error_code_publication_matches_five_original_callback_controls() {
    check_c84_code_controls(
        include_str!(
            "../../../../../../runtime/rust/tests/data/native_c84_error_code/controls.tsv"
        ),
        &[
            (
                "conflict",
                "package provide probe 1;catch {package provide probe 2} m;list $m $::errorCode",
            ),
            ("no-code", "catch {error FAIL} m;list $m $::errorCode"),
            ("NONE", "catch {error FAIL {} NONE} m;list $m $::errorCode"),
            (
                "structured",
                "catch {error FAIL {} {CUSTOM DETAIL}} m;list $m $::errorCode",
            ),
            (
                "structured-getter",
                "catch {lindex {} BAD} m;list $m $::errorCode",
            ),
        ],
    );
}

#[test]
fn c84_nonempty_error_info_publication_matches_original_callback_controls() {
    check_c84_code_controls(
        include_str!(
            "../../../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/controls.tsv"
        ),
        &[
            (
                "info-no-code",
                "catch {error FAIL INFO} m;list $m $::errorInfo $::errorCode",
            ),
            (
                "info-NONE",
                "catch {error FAIL INFO NONE} m;list $m $::errorInfo $::errorCode",
            ),
            (
                "info-structured",
                "catch {error FAIL INFO {CUSTOM DETAIL}} m;list $m $::errorInfo $::errorCode",
            ),
        ],
    );
}
