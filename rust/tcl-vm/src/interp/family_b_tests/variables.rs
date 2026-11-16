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

//! The variable store by frame, `tcl_precision`, native variable names and variable links.

use super::*;

#[test]
fn varstore_honours_frame_id() {
    let mut vm = Vm::new();
    // Write into the global frame while it is current.
    vm.set(GLOBAL_FRAME, "g", Value::string("global"));
    // Enter a proc-call frame; the global var is not in it.
    vm.push_call_frame(Some("p".to_string()), vec![Value::string("p")]);
    let here = FrameId(vm.current_level());
    assert_ne!(here, GLOBAL_FRAME);
    vm.set(here, "loc", Value::string("local"));
    // FrameId is honoured: each frame sees only its own var.
    assert_eq!(
        vm.get(GLOBAL_FRAME, "g").map(|v| v.to_str().to_string()),
        Some("global".to_string())
    );
    assert!(vm.get(here, "g").is_none());
    assert!(vm.exists(here, "loc"));
    assert!(!vm.exists(GLOBAL_FRAME, "loc"));
    // Reach back into the global frame from the child frame.
    vm.set(GLOBAL_FRAME, "g2", Value::string("two"));
    vm.pop_call_frame();
    assert_eq!(
        vm.get(GLOBAL_FRAME, "g2").map(|v| v.to_str().to_string()),
        Some("two".to_string())
    );
    assert!(vm.unset(GLOBAL_FRAME, "g2"));
    assert!(!vm.exists(GLOBAL_FRAME, "g2"));
}

#[test]
fn varstore_array_elements() {
    let mut vm = Vm::new();
    assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "k"));
    vm.set_elem(GLOBAL_FRAME, "a", "k", Value::string("v"));
    assert!(vm.exists_elem(GLOBAL_FRAME, "a", "k"));
    assert_eq!(
        vm.get_elem(GLOBAL_FRAME, "a", "k")
            .map(|v| v.to_str().to_string()),
        Some("v".to_string())
    );
    assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "nope"));
    assert!(vm.unset_elem(GLOBAL_FRAME, "a", "k"));
    assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "k"));

    // The pair is never recomposed and reparsed: every parenthesis shape is
    // a valid array base when the caller already supplied it separately.
    for base in ["z(b", "z)b", "z(b)c"] {
        vm.set_elem(GLOBAL_FRAME, base, "a", Value::string(base));
        assert_eq!(
            vm.get_elem(GLOBAL_FRAME, base, "a")
                .map(|value| value.to_str().to_string()),
            Some(base.to_owned())
        );
        assert!(vm.exists_elem(GLOBAL_FRAME, base, "a"));
        assert!(vm.unset_elem(GLOBAL_FRAME, base, "a"));
    }
}

#[test]
fn raw_scalar_replacement_retires_array_element_trace_cells() {
    let mut vm = Vm::new();
    let baseline = vm.var_arena.len();
    assert!(vm.set_array_elem("a", "k", Value::string("old")).is_ok());
    vm.add_var_trace(
        "a(k)",
        vec!["unset".to_owned()],
        "callback".to_owned(),
        false,
    );
    assert_eq!(vm.var_arena.len(), baseline + 2);

    vm.write_scalar_raw("a", Value::string("scalar"));

    assert_eq!(vm.get_var("a").unwrap().to_str().as_ref(), "scalar");
    assert!(vm.variable_observers.script_traces.is_empty());
    assert_eq!(vm.var_arena.len(), baseline + 1);
}

#[test]
fn noncurrent_varstore_unset_retires_array_element_trace_cells() {
    let mut vm = Vm::new();
    let baseline = vm.var_arena.len();
    assert!(vm.set_array_elem("a", "k", Value::string("old")).is_ok());
    vm.add_var_trace(
        "a(k)",
        vec!["unset".to_owned()],
        "callback".to_owned(),
        false,
    );
    vm.push_call_frame(Some("p".to_owned()), vec![Value::string("p")]);
    assert!(vm.unset(GLOBAL_FRAME, "a"));

    assert!(vm.variable_observers.script_traces.is_empty());
    assert_eq!(vm.var_arena.len(), baseline);
    vm.pop_call_frame();
}

#[test]
fn noncurrent_varstore_unset_elem_retires_the_reached_trace_cell() {
    let mut vm = Vm::new();
    let baseline = vm.var_arena.len();
    vm.set_elem(GLOBAL_FRAME, "a", "k", Value::string("old"));
    vm.add_var_trace(
        "a(k)",
        vec!["unset".to_owned()],
        "callback".to_owned(),
        false,
    );
    vm.push_call_frame(Some("p".to_owned()), vec![Value::string("p")]);

    assert!(vm.unset_elem(GLOBAL_FRAME, "a", "k"));
    assert!(vm.variable_observers.script_traces.is_empty());
    assert_eq!(vm.var_arena.len(), baseline + 1, "the empty array remains");
    assert_eq!(vm.array_keys(GLOBAL_FRAME, "a"), Some(Vec::new()));
    vm.pop_call_frame();
}

#[test]
fn noncurrent_varstore_unset_reports_trace_only_cells_missing() {
    let mut vm = Vm::new();
    let baseline = vm.var_arena.len();
    vm.add_var_trace("x", vec!["unset".to_owned()], "callback".to_owned(), false);
    assert!(vm.ensure_array("a").is_ok());
    vm.add_var_trace(
        "a(k)",
        vec!["unset".to_owned()],
        "callback".to_owned(),
        false,
    );
    vm.push_call_frame(Some("p".to_owned()), vec![Value::string("p")]);

    assert!(!vm.unset(GLOBAL_FRAME, "x"));
    assert!(!vm.unset_elem(GLOBAL_FRAME, "a", "k"));
    assert!(vm.variable_observers.script_traces.is_empty());
    assert_eq!(vm.var_arena.len(), baseline + 1, "the empty array remains");
    vm.pop_call_frame();
}

#[test]
fn varstore_array_keys_honours_the_requested_frame() {
    let mut vm = Vm::new();
    vm.set_elem(GLOBAL_FRAME, "same", "global-a", Value::string("A"));
    vm.set_elem(GLOBAL_FRAME, "same", "global-b", Value::string("B"));
    vm.push_call_frame(Some("p".to_owned()), vec![Value::string("p")]);
    let local = FrameId(vm.current_level());
    vm.set_elem(local, "same", "local", Value::string("L"));

    assert!(vm.exists(GLOBAL_FRAME, "same"));
    assert!(vm.exists(local, "same"));
    // C's string-hash order: tclsh 8.4-9.1 report `global-b global-a`.
    assert_eq!(
        vm.array_keys(GLOBAL_FRAME, "same"),
        Some(vec!["global-b".to_owned(), "global-a".to_owned()])
    );
    assert_eq!(vm.array_keys(local, "same"), Some(vec!["local".to_owned()]));
    vm.pop_call_frame();
}

#[test]
fn native_double_strings_observe_precision_only_until_first_materialisation() {
    let source = "set tcl_precision 12; set x 1.0; set d [expr {$x/3}]; set tcl_precision 4; set first [list $d]; set tcl_precision 12; set second [list $d]; set e [expr {$x/3}]; expr {$e+0}; set tcl_precision 4; set third [list $e]; list $first $second $third";
    for profile in variable_link_profiles() {
        let mut vm = crate::native_fixture::interpreter(profile);
        let wanted = match profile.double_string_policy().unwrap() {
            tcl_dialect::DoubleStringPolicy::Tcl84Precision
            | tcl_dialect::DoubleStringPolicy::TclPrecision => "0.3333 0.3333 0.3333",
            tcl_dialect::DoubleStringPolicy::Shortest => {
                "0.3333333333333333 0.3333333333333333 0.3333333333333333"
            }
            tcl_dialect::DoubleStringPolicy::JimTwelve => {
                "0.333333333333 0.333333333333 0.333333333333"
            }
        };
        let result = vm.eval_source(source).unwrap();
        assert_eq!(
            result.code,
            Code::Ok,
            "{}: {}",
            profile.name,
            result.result.to_str()
        );
        assert_eq!(result.result.to_str().as_ref(), wanted, "{}", profile.name);
    }
}

#[test]
fn native_scan_and_binary_fields_preserve_lazy_double_representations() {
    let cases = [
        (
            "set tcl_precision 4; set d [expr {1.0/3}]; binary scan [binary format d $d] d copy; set tcl_precision 12; list $d $copy",
            "0.333333333333 0.333333333333",
            "0.3333333333333333 0.3333333333333333",
        ),
        (
            "set tcl_precision 4; set d [expr {1.0/3}]; binary scan [binary format d* [list $d $d]] d* copy; set tcl_precision 12; list $d $copy",
            "0.333333333333 {0.333333333333 0.333333333333}",
            "0.3333333333333333 {0.3333333333333333 0.3333333333333333}",
        ),
        (
            "set tcl_precision 12; scan 0.3333333333333333 %f d; set tcl_precision 4; list $d",
            "0.3333",
            "0.3333333333333333",
        ),
    ];
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        for (source, precision_wanted, shortest_wanted) in cases {
            let mut vm = crate::native_fixture::interpreter(profile);
            let wanted = match profile.double_string_policy().unwrap() {
                tcl_dialect::DoubleStringPolicy::Shortest => shortest_wanted,
                _ => precision_wanted,
            };
            let result = vm.eval_source(source).unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn native_precision_observer_follows_storage_and_validates_writes() {
    let cases = [
        (
            "set tcl_precision 12; proc localPrecision {} {set tcl_precision 4; expr {1.0/3}}; localPrecision",
            "0.333333333333",
        ),
        (
            "set tcl_precision 4; upvar 0 tcl_precision alias; set alias 6; set status [catch {set tcl_precision 18} message]; list $status $message $tcl_precision",
            "1 {can't set \"tcl_precision\": improper value for precision} 6",
        ),
        (
            "set tcl_precision 4; set child [interp create -safe]; set status [$child eval {catch {set tcl_precision 8} message}]; set message [$child eval {set message}]; interp delete $child; list $status $message $tcl_precision",
            "1 {can't set \"tcl_precision\": can't modify precision from a safe interpreter} 4",
        ),
    ];
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        for (source, wanted) in cases {
            let mut vm = crate::native_fixture::interpreter(profile);
            let result = vm.eval_source(source).unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn native_precision_array_trace_runs_before_element_callbacks() {
    let cases = [
        (
            "set tcl_precision 12; unset tcl_precision; set tcl_precision(x) 12; upvar 0 tcl_precision(x) p; set p 4; set d [expr {1.0/3}]; list $d $p $tcl_precision(x)",
            "0.333333333333 4 12",
        ),
        (
            "set tcl_precision 12; unset tcl_precision; set tcl_precision(x) 12; proc change {name key op} {upvar #0 $name a; set a($key) 8}; trace variable tcl_precision(x) w change; set tcl_precision(x) 4; list [expr {1.0/3}] $tcl_precision(x)",
            "0.3333 4",
        ),
    ];
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        for (source, wanted) in cases {
            let mut vm = crate::native_fixture::interpreter(profile);
            let result = vm.eval_source(source).unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn native_precision_is_shared_with_children_and_survives_unset() {
    let source = "set tcl_precision 4; set x 1.0; set d [expr {$x/7}]; set child [interp create]; $child eval {set tcl_precision 12}; set result [list $tcl_precision $d]; interp delete $child; unset tcl_precision; list $result [info exists tcl_precision] $tcl_precision";
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let mut vm = crate::native_fixture::interpreter(profile);
        let result = vm.eval_source(source).unwrap();
        assert_eq!(
            result.code,
            Code::Ok,
            "{}: {}",
            profile.name,
            result.result.to_str()
        );
        assert_eq!(
            result.result.to_str().as_ref(),
            "{12 0.142857142857} 1 12",
            "{}",
            profile.name
        );
    }
}

#[test]
fn native_variable_updates_retain_exact_keys_and_failed_read_options() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(*version);
        vm.set_var_bytes(b"k\0z", Value::int(7)).unwrap();
        let increment = vm
            .increment_captured_bytes(b"k\0z", None, &Value::int(2))
            .unwrap();
        assert_eq!(increment.value.as_int().unwrap(), 9);
        assert_eq!(vm.get_var_bytes(b"k\0z").unwrap().as_int().unwrap(), 9);
        assert!(crate::command::opt_get(&increment.options, "-errorcode").is_none());
        let appended = vm
            .lappend_list_update_bytes(b"missing", None, &[Value::string("x")])
            .unwrap();
        assert_eq!(appended.value.string_bytes().as_ref(), b"x");
        let code = crate::command::opt_get(&appended.options, "-errorcode");
        if *version >= tcl_dialect::TclVersion::V8_6 {
            assert_eq!(code.unwrap().string_bytes().as_ref(), b"TCL READ VARNAME");
        } else {
            assert!(
                code.is_none(),
                "{version:?}: {:?}",
                code.map(|code| code.to_str().to_string())
            );
        }
        vm.set_array_elem_bytes(b"a", b"key\0tail", Value::int(3))
            .unwrap();
        let increment = vm
            .increment_captured_bytes(b"a", Some(b"key\0tail"), &Value::int(4))
            .unwrap();
        assert_eq!(increment.value.as_int().unwrap(), 7);
        assert_eq!(
            vm.get_array_elem_bytes_from(0, b"a", b"key\0tail")
                .unwrap()
                .as_int()
                .unwrap(),
            7
        );
    }
}

#[test]
fn native_read_result_reports_the_actual_failure_site_and_operand_form() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = Vm::new();
        vm.set_runtime_version(*version);
        vm.ensure_array("a").unwrap();
        let error = vm
            .read_variable_result_bytes(b"a(k\0tail)", None)
            .unwrap_err();
        assert_eq!(
            error.result.string_bytes().as_ref(),
            b"can't read \"a(k\": no such element in array"
        );
        if *version >= tcl_dialect::TclVersion::V8_6 {
            assert_eq!(
                crate::command::opt_get(&error.options, "-errorcode")
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                b"TCL READ VARNAME"
            );
        }
        let error = vm
            .read_variable_result_bytes(b"a", Some(b"k\0tail"))
            .unwrap_err();
        assert_eq!(
            error.result.string_bytes().as_ref(),
            b"can't read \"a(k)\": no such element in array"
        );
    }
}

#[test]
fn native_variable_bytes_preserve_full_keys_and_nul_qualification() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(*version);
        vm.set_var_bytes(b"k\0z", Value::string("FULL")).unwrap();
        vm.set_var_bytes(b"N\0z::k", Value::string("LATE")).unwrap();
        assert_eq!(
            vm.get_var_bytes(b"k\0z").unwrap().string_bytes().as_ref(),
            b"FULL"
        );
        assert_eq!(
            vm.get_var_bytes(b"N\0z::k")
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"LATE"
        );
        assert_eq!(
            vm.get_var_bytes(b"k").is_some(),
            *version == tcl_dialect::TclVersion::V8_4
        );
        vm.set_var_bytes(b"\xff", Value::string("INVALID")).unwrap();
        assert_eq!(
            vm.get_var_bytes(b"\xff").unwrap().string_bytes().as_ref(),
            b"INVALID"
        );
        assert!(
            vm.frame_var_name_bytes(true)
                .iter()
                .any(|name| name == b"\xff")
        );
    }
}

#[test]
fn native_combined_and_separate_array_inputs_select_distinct_receivers() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(*version);
        vm.set_var_bytes(b"a\0z(k)", Value::string("COMBINED"))
            .unwrap();
        vm.set_array_elem_bytes_from(0, b"a\0z", b"k", Value::string("SEPARATE"), false)
            .unwrap();
        let combined = vm.get_var_bytes(b"a\0z(k)").unwrap();
        assert_eq!(
            combined.string_bytes().as_ref(),
            if *version >= tcl_dialect::TclVersion::V9_0 {
                b"COMBINED".as_slice()
            } else {
                b"SEPARATE".as_slice()
            }
        );
        assert_eq!(
            vm.get_array_elem_bytes_from(0, b"a\0z", b"k")
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"SEPARATE"
        );
        vm.set_var_bytes(b"b(k\0z)", Value::string("ELEMENT"))
            .unwrap();
        let target = VarStore::array_target_bytes(&vm, FrameId(0), b"b").unwrap();
        let keys = VarStore::array_key_bytes_checked_at(&vm, &target)
            .unwrap()
            .unwrap();
        assert_eq!(
            keys,
            vec![if *version >= tcl_dialect::TclVersion::V9_0 {
                b"k\0z".to_vec()
            } else {
                b"k".to_vec()
            }]
        );
    }
}

#[test]
fn native_variable_links_keep_byte_names_and_retired_cell_identity() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(*version);
        vm.set_var_bytes(b"x", Value::string("OLD")).unwrap();
        vm.push_call_frame(Some("P".into()), Vec::new());
        vm.link_upvar_bytes(0, b"x", b"a\0z")
            .unwrap_or_else(|_| panic!("byte upvar publication succeeds"));
        assert_eq!(
            vm.get_var_bytes(b"a\0z").unwrap().string_bytes().as_ref(),
            b"OLD"
        );
        assert_eq!(
            vm.get_var_bytes(b"a").is_some(),
            *version == tcl_dialect::TclVersion::V8_4
        );
        let target = vm
            .resolve_var_from_bytes(b"a\0z", vm.current_level())
            .unwrap()
            .id
            .unwrap();
        vm.unset_from(0, "x");
        vm.write_scalar_from(0, "x", Value::string("NEW"));
        assert_eq!(
            vm.resolve_var_from_bytes(b"a\0z", vm.current_level())
                .unwrap()
                .id,
            Some(target)
        );
        assert_eq!(
            vm.get_var_bytes(b"a\0z").unwrap().string_bytes().as_ref(),
            b"NEW"
        );
        vm.pop_call_frame();
    }
}

#[test]
fn native_trace_registration_projects_its_own_name_and_keeps_prefix_value() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(*version);
        let prefix = Value::from_string_bytes(b"capture\xff".as_slice());
        vm.ensure_trace_variable_bytes(b"k\0z").unwrap();
        vm.add_var_trace_bytes(b"k\0z", vec!["read".into()], &prefix, false);
        let registered = vm.registered_trace_cell_bytes(b"k\0z").unwrap().id.unwrap();
        assert_eq!(
            vm.registered_trace_cell_bytes(b"k").unwrap().id,
            Some(registered)
        );
        assert_eq!(
            vm.trace_cell_bytes(b"k\0z")
                .and_then(|cell| cell.id)
                .is_some(),
            *version == tcl_dialect::TclVersion::V8_4
        );
        let entries = vm.var_trace_info_bytes(b"k\0z");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1.string_bytes().as_ref(), b"capture\xff");
        vm.remove_var_trace_bytes(b"k\0z", &["read".into()], b"capture\xff");
        assert!(vm.var_trace_info_bytes(b"k").is_empty());
    }
}

#[test]
fn confined_jim_dictionary_lappend_refuses_foreign_write_and_preserves_reads() {
    let mut vm = Vm::new();
    let jim = tcl_registry::model::ingress::resolve_environment("jim").analyser_profile();
    vm.set_dialect_profile(jim);
    assert!(vm.dictionary_variable_containers());
    vm.set_var("::held", Value::string("one")).expect("seed");
    vm.set_stores_confined(true);
    assert!(
        vm.lappend_list_update_bytes(b"::held", None, &[Value::string("two")])
            .is_err()
    );
    assert_eq!(vm.get_var("::held").unwrap().to_str().as_ref(), "one");
    let read = vm
        .lappend_list_update_bytes(b"::held", None, &[])
        .expect("read-only");
    assert_eq!(read.value.to_str().as_ref(), "one");
    assert!(
        vm.lappend_list_update_bytes(b"::missing", None, &[])
            .is_err()
    );
    assert!(vm.get_var("::missing").is_none());
}

/// The selected engine with the scripted distribution its shell loads, so
/// Jim's `namespace` helpers are present as they are under jimsh.
fn variable_link_vm(profile: &'static tcl_dialect::DialectProfile) -> Vm {
    let mut vm = crate::native_fixture::interpreter(profile);
    crate::native_fixture::install_distribution(&mut vm);
    vm
}

fn variable_link_profiles() -> Vec<&'static tcl_dialect::DialectProfile> {
    let mut profiles: Vec<_> = tcl_dialect::TclVersion::ALL
        .iter()
        .map(|version| tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap())
        .collect();
    let jim = tcl_registry::model::ingress::resolve_environment("jim").analyser_profile();
    assert_eq!(
        jim.core_point
            .map(tcl_dialect::model::DialectPoint::release),
        Some(tcl_dialect::model::Release::JIM_0_84),
        "the VM fixture and pinned native oracle must select the same release"
    );
    profiles.push(jim);
    profiles
}

#[test]
fn variable_links_follow_authored_dialect_lifetimes() {
    let cases = [
        (
            "set x OLD; upvar 0 x y; unset x; set y NEW; list $x $y",
            "NEW NEW",
            "NEW NEW",
        ),
        (
            "set x OLD; upvar 0 x y; unset x; set x NEW; list $x $y",
            "NEW NEW",
            "NEW NEW",
        ),
        (
            "set results {}; upvar 0 x y; foreach n {1 2 3} {set x $n; lappend results $y; unset x}; set results",
            "1 2 3",
            "1 2 3",
        ),
        (
            "set a(k) OLD; upvar 0 a(k) y; unset a; set a(k) NEW; list [catch {set y} error] $error $a(k)",
            "1 {can't read \"y\": no such variable} NEW",
            "0 NEW NEW",
        ),
        (
            "set a(k) OLD; upvar 0 a(k) y; unset a(k); set a(k) NEW; list $y $a(k)",
            "NEW NEW",
            "NEW NEW",
        ),
        (
            "namespace eval ::life {variable x OLD}; upvar #0 ::life::x y; namespace delete ::life; namespace eval ::life {variable x NEW}; list [catch {set y} error] $error $::life::x",
            "1 {can't read \"y\": no such variable} NEW",
            "0 NEW NEW",
        ),
        (
            "namespace eval ::life2 {variable x OLD}; upvar #0 ::life2::x y; unset ::life2::x; set y NEW; list $y $::life2::x",
            "NEW NEW",
            "NEW NEW",
        ),
    ];
    for profile in variable_link_profiles() {
        for (source, cell_want, name_want) in cases {
            let mut vm = variable_link_vm(profile);
            let wanted = match profile.variable_link_binding().unwrap() {
                tcl_dialect::VariableLinkBinding::StableCell => cell_want,
                tcl_dialect::VariableLinkBinding::SelectedFrameName => name_want,
            };
            let result = vm
                .eval_source(&format!("proc probe {{}} {{{source}}}; probe"))
                .unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                &*result.result.to_str(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn variable_link_target_rebinding_and_materialisation_follow_the_policy() {
    let cases = [
        (
            "set x OLD; set z NEW; upvar 0 x a; upvar 0 a b; upvar 0 z a; list $a $b",
            "0 {NEW OLD}",
            "0 {NEW NEW}",
        ),
        (
            "upvar 0 a(k) y; list [info exists a] [array exists a] [info exists y]",
            "0 {1 1 0}",
            "0 {0 0 0}",
        ),
        (
            "set a(k) OLD; upvar 0 a(k) y; unset a; set y NEW; list $a(k) $y",
            "1 {can't set \"y\": upvar refers to element in deleted array}",
            "0 {NEW NEW}",
        ),
        (
            "upvar 0 x x",
            "1 {can't upvar from variable to itself}",
            "1 {can't upvar from variable to itself}",
        ),
        (
            "upvar 0 a b; upvar 0 b a",
            "1 {can't upvar from variable to itself}",
            "1 {can't upvar from variable to itself}",
        ),
    ];
    for profile in variable_link_profiles() {
        for (source, cell_want, name_want) in cases {
            let mut vm = variable_link_vm(profile);
            let result = vm
                .eval_source(&format!(
                    "proc probe {{}} {{{source}}}; list [catch probe result] $result"
                ))
                .unwrap();
            let wanted = if profile.variable_link_binding()
                == Some(tcl_dialect::VariableLinkBinding::StableCell)
            {
                cell_want
            } else {
                name_want
            };
            assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
            assert_eq!(
                &*result.result.to_str(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn namespace_var_enumeration_uses_the_exact_retained_token() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert_eq!(
        eval_value(
            &mut vm,
            "namespace eval N {\
                     variable old OLD;\
                     proc p {} {\
                         namespace delete ::N;\
                         namespace eval ::N {variable fresh NEW};\
                         yield paused\
                     }\
                 }",
        ),
        ""
    );
    let old = Namespaces::find_namespace(&vm, ROOT_NS, "::N").unwrap();
    assert_eq!(eval_value(&mut vm, "coroutine c ::N::p"), "paused");
    let fresh = Namespaces::find_namespace(&vm, ROOT_NS, "::N").unwrap();

    assert_ne!(old, fresh);
    assert_eq!(Namespaces::vars_in(&vm, old), vec!["old".to_owned()]);
    assert_eq!(Namespaces::vars_in(&vm, fresh), vec!["fresh".to_owned()]);

    assert_eq!(eval_value(&mut vm, "c"), "");
}
