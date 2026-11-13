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

use super::*;

fn native() -> Vm {
    crate::native_fixture::interpreter(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
    )
}

#[test]
fn checked_option_mutation_retains_original_opaque_members_and_carrier() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software object/option ownership control; no original-provider pass or
    // native physical-header observation follows from these assertions.
    let mut vm = native();
    let key = Value::new_native_string_bytes(b"-opaque\xff\0TAIL".as_slice());
    let value = Value::new_native_string_bytes(b"VALUE\xfe\0TAIL".as_slice());
    let list = Value::list(vec![
        key.clone(),
        value.clone(),
        Value::string("-level"),
        Value::int(2),
    ]);
    let changed = with_return_level(&mut vm, &list, 1).unwrap();
    let rows = completion_option_rows_checked(&mut vm, &changed).unwrap();
    assert!(rows[0].0.is_same_object(&key));
    assert!(rows[0].1.is_same_object(&value));
    assert!(changed.cached_list_representation().is_some());
    assert_eq!(
        option_integer_checked(&mut vm, &changed, b"-level", 99).unwrap(),
        1
    );

    let dictionary = tcl_syntax::value::ValueOps::new_dict_checked(
        &mut vm,
        vec![
            (key.clone(), value.clone()),
            (Value::string("-level"), Value::int(2)),
        ],
    )
    .unwrap();
    let changed = with_return_level(&mut vm, &dictionary, 0).unwrap();
    assert!(
        changed
            .with_cached_dictionary_representation(|_, _| ())
            .is_some()
    );
    let rows = completion_option_rows_checked(&mut vm, &changed).unwrap();
    let retained = rows
        .iter()
        .find(|(original, _)| original.is_same_object(&key))
        .unwrap();
    assert!(retained.1.is_same_object(&value));
    assert_eq!(
        option_integer_checked(&mut vm, &dictionary, b"-level", 99).unwrap(),
        2
    );
    assert_eq!(
        option_integer_checked(&mut vm, &changed, b"-level", 99).unwrap(),
        0
    );
}

#[test]
fn invalid_carried_control_is_a_failure_and_absence_has_its_separate_default() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = native();
    assert_eq!(
        option_integer_checked(&mut vm, &Value::empty(), b"-level", 1).unwrap(),
        1
    );
    let options = Value::list(vec![
        Value::string("-level"),
        Value::string("NOT_AN_INTEGER"),
    ]);
    let error = option_integer_checked(&mut vm, &options, b"-level", 1).unwrap_err();
    assert!(
        error.is_guest_error(),
        "the actual primitive produced the failure"
    );
    assert!(
        error
            .guest_completion()
            .unwrap()
            .result
            .resident_string_bytes()
            .unwrap()
            .starts_with(b"expected integer")
    );
    let odd = Value::list(vec![Value::string("-level")]);
    assert!(
        completion_option_rows_checked(&mut vm, &odd)
            .unwrap_err()
            .is_guest_error()
    );
}

#[test]
fn option_snapshot_preserves_first_host_cause_and_earlier_guest_effects() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = native();
    vm.set_var("before", Value::string("RETAINED")).unwrap();
    let cause = tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable;
    vm.refuse_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(cause));
    let options = Value::int(42);
    let completion = Completion::new(Code::Error, Value::int(17), options.clone());
    let error = vm.completion_options_snapshot(&completion).unwrap_err();
    assert!(
        matches!(error, TclError::Host(crate::error::TclHostFailure::Execution(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(original))) if original == cause)
    );
    assert!(options.resident_string_bytes().is_none());
    assert!(completion.result.resident_string_bytes().is_none());
    assert_eq!(
        vm.get_var("before")
            .unwrap()
            .resident_string_bytes()
            .unwrap()
            .as_ref(),
        b"RETAINED"
    );
}

// tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1.0 output for each script.
const LIVE_METADATA_CASES: [(&str, [&str; 4]); 10] = [
    (
        "array set A {k 1}; list [catch {lappend A(nk)} m o] $m $o",
        [
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
        ],
    ),
    (
        "list [catch {incr absent} m o] $m $o",
        [
            "0 1 {-code 0 -level 0}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
        ],
    ),
    (
        "list [catch {lappend absent} m o] $m $o",
        [
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
        ],
    ),
    (
        "list [catch {return -level 0 -errorcode X ok} m o] $m $o",
        [
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
        ],
    ),
    (
        "proc p {} {array set A {k 1}; list [catch {lappend A(nk)} m o] $m $o}; p",
        [
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
        ],
    ),
    (
        "proc p {} {list [catch {incr absent} m o] $m $o}; p",
        [
            "0 1 {-code 0 -level 0}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
        ],
    ),
    (
        "proc tr args {error boom}; set t 1; trace add variable t read tr; list [catch {lappend t} m o] $m $o",
        [
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
        ],
    ),
    (
        "proc tr args {error boom}; set t 1; trace add variable t read tr; list [catch {lappend t x} m o] $m $o",
        [
            "0 x {-code 0 -level 0 -errorcode NONE -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
        ],
    ),
    (
        "proc tr args {error boom}; set t 1; trace add variable t read tr; set c lappend; list [catch {$c t x} m o] $m $o",
        [
            "0 x {-code 0 -level 0 -errorcode NONE -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
        ],
    ),
    (
        "proc q {} {list [catch {return -level 0 -errorcode X ok} m o] $m $o}; q",
        [
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
        ],
    ),
];

/// Live error metadata on a successful completion follows `-code` and
/// `-level`, as `Tcl_GetReturnOptions` appends `iPtr->errorCode` and
/// `errorInfo` after them, while a carried `return -errorcode` stays first.
/// A read trace's failure keeps its code through the command's return.
#[test]
fn successful_completions_place_live_error_metadata_after_control_keys() {
    let mut mismatches = Vec::new();
    for (source, expected) in LIVE_METADATA_CASES {
        for (release, expected) in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
            .into_iter()
            .zip(expected)
        {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect(release),
            );
            let completion = vm.try_eval_source(source).unwrap();
            let actual = completion.result.to_str();
            if completion.code != Code::Ok || actual.as_ref() != expected {
                mismatches.push(format!(
                    "{release}: {source}\n  got  {actual}\n  want {expected}"
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// `array get` swallows a failed element read trace; its live metadata
/// follows the control keys (tclsh 8.6.18, 9.0.4 and 9.1.0).
#[test]
fn array_get_trace_failure_places_live_metadata_after_control_keys() {
    let source = "proc tr args {error boom}; array set A {x 1}; trace add variable A read tr; list [catch {array get A} m o] $m $o";
    let expected = "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr A x read\"\n    (read trace on \"A(x)\")} -errorline 1}";
    for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(release));
        let completion = vm.try_eval_source(source).unwrap();
        assert_eq!(completion.code, Code::Ok, "{release}");
        assert_eq!(completion.result.to_str().as_ref(), expected, "{release}");
    }
}
