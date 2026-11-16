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

//! Recorded native source controls: trace registrations, list assignment, read diagnostics, retired namespaces, detached elements and array root writes.

use super::*;

fn jim_trace_setup_observation(source: &[u8], expected: &[u8]) {
    // Retain the original probe, but stop at its recorded SETUP frontier
    // before the unrelated binary reporter, which this backend lacks.
    let source = std::str::from_utf8(source).unwrap();
    let (prefix, _) = source
        .split_once("binary scan $probeSetupResult H* probeSetupHex\n")
        .unwrap();
    let captured = std::str::from_utf8(expected).unwrap();
    let setup = captured
        .lines()
        .find(|line| line.starts_with("SETUP "))
        .unwrap();
    let hex = setup.strip_prefix("SETUP 1 ").unwrap();
    let expected_message: Vec<_> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert!(!captured
        .lines()
        .any(|line| line.starts_with("OBSERVATION ")));
    crate::counters::reset();
    {
        let mut interp = Interp::with_native_core(
            super::super::super::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(prefix.as_bytes()), Code::Ok);
        assert!(!interp.host_refusal_pending());
        for (name, expected) in [
            (b"probeSetupCode".as_slice(), b"1".as_slice()),
            (b"probeSetupResult".as_slice(), expected_message.as_slice()),
        ] {
            let value = interp.var_get(name).unwrap();
            assert_eq!(
                interp.native_string_bytes(&value).unwrap().as_ref(),
                expected
            );
        }
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}

#[test]
fn active_read_unset_retires_old_registrations_matches_five_native_source_controls() {
    // Native proof: naming.variable.unset-active-read-callback-retirement
    // docs/design/analysis/name-resolution-proofs/variable-unset-active-read-callback-retirement.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../../tests/data/native_active_read_unset/8.4.20.txt").as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../../tests/data/native_active_read_unset/8.5.19.txt").as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../../tests/data/native_active_read_unset/8.6.18.txt").as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../../tests/data/native_active_read_unset/9.0.4.txt").as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../../tests/data/native_active_read_unset/9.1.0.txt").as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!("../../../../tests/data/native_active_read_unset/source.tcl"),
            expected,
        );
    }
}

#[test]
fn active_read_unset_probe_preserves_actual_jim_unsupported_setup() {
    // Native proof: naming.variable.unset-active-read-callback-retirement
    // docs/design/analysis/name-resolution-proofs/variable-unset-active-read-callback-retirement.md
    // Jim's recorded setup failure supplies no read/unset callback answer.
    jim_trace_setup_observation(
        include_bytes!("../../../../tests/data/native_active_read_unset/source.tcl"),
        include_bytes!("../../../../tests/data/native_active_read_unset/jim.txt"),
    );
}

#[test]
fn recreated_element_read_trace_matches_five_original_source_controls() {
    // Native proof: naming.variable.recreated-element-independent-read-trace
    // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../../tests/data/native_recreated_element_trace/8.4.20.txt")
                .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../../tests/data/native_recreated_element_trace/8.5.19.txt")
                .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../../tests/data/native_recreated_element_trace/8.6.18.txt")
                .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../../tests/data/native_recreated_element_trace/9.0.4.txt")
                .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../../tests/data/native_recreated_element_trace/9.1.0.txt")
                .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!("../../../../tests/data/native_recreated_element_trace/source.tcl"),
            expected,
        );
    }
}

#[test]
fn recreated_element_trace_probe_preserves_actual_jim_unsupported_setup() {
    // Native proof: naming.variable.recreated-element-independent-read-trace
    // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
    // The captured Jim setup failure supplies no independent callback answer.
    jim_trace_setup_observation(
        include_bytes!("../../../../tests/data/native_recreated_element_trace/source.tcl"),
        include_bytes!("../../../../tests/data/native_recreated_element_trace/jim.txt"),
    );
}

#[test]
fn list_assignment_arity_matches_five_original_source_controls() {
    // Native proof: naming.list.original-lassign-target-arity
    // docs/design/analysis/name-resolution-proofs/list-original-lassign-target-arity.md
    // Literal, dynamic and proc source entry are observations, not admission proofs.
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../../tests/data/native_list_assignment_arity/8.4.20.txt")
                .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../../tests/data/native_list_assignment_arity/8.5.19.txt")
                .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../../tests/data/native_list_assignment_arity/8.6.18.txt")
                .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../../tests/data/native_list_assignment_arity/9.0.4.txt")
                .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../../tests/data/native_list_assignment_arity/9.1.0.txt")
                .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!("../../../../tests/data/native_list_assignment_arity/source.tcl"),
            expected,
        );
    }
}

#[test]
fn list_assignment_arity_matches_current_jim_original_source_controls() {
    // Native proof: naming.list.original-lassign-target-arity
    // docs/design/analysis/name-resolution-proofs/list-original-lassign-target-arity.md
    trace_source_observation(
        "jim",
        include_bytes!("../../../../tests/data/native_list_assignment_arity/source.tcl"),
        include_bytes!("../../../../tests/data/native_list_assignment_arity/jim.txt"),
    );
}

#[test]
fn variable_read_diagnostics_match_five_original_source_controls() {
    // Native proof: naming.variable.original-read-diagnostic-input
    // docs/design/analysis/name-resolution-proofs/variable-original-read-diagnostic-input.md
    // These source-entry outcomes grant no private layout or compiler admission.
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../../tests/data/native_variable_read_input/8.4.20.txt")
                .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../../tests/data/native_variable_read_input/8.5.19.txt")
                .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../../tests/data/native_variable_read_input/8.6.18.txt")
                .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../../tests/data/native_variable_read_input/9.0.4.txt")
                .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../../tests/data/native_variable_read_input/9.1.0.txt")
                .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!("../../../../tests/data/native_variable_read_input/source.tcl"),
            expected,
        );
    }
}

#[test]
fn variable_read_input_preserves_current_jim_dictionary_results() {
    // Native proof: naming.variable.original-read-diagnostic-input
    // docs/design/analysis/name-resolution-proofs/variable-original-read-diagnostic-input.md
    // Jim's array-valued root reads succeed; its scalar element reads fail.
    trace_source_observation(
        "jim",
        include_bytes!("../../../../tests/data/native_variable_read_input/source.tcl"),
        include_bytes!("../../../../tests/data/native_variable_read_input/jim.txt"),
    );
}

#[test]
fn retired_namespace_c86_source_entries_preserve_scalar_and_element_sites() {
    trace_source_observation(
            "tcl8.6",
            include_bytes!(
                "../../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/input.tcl"
            ),
            include_bytes!(
                "../../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/8.6.18/stdout.txt"
            ),
        );
}

#[test]
fn retired_namespace_c86_variable_heads_preserve_original_root_tuple() {
    trace_source_observation(
            "tcl8.6",
            include_bytes!(
                "../../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/variable-head-input.tcl"
            ),
            include_bytes!(
                "../../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.6.18/stdout.txt"
            ),
        );
}

#[test]
fn retired_namespace_original_object_vector_keeps_root_despite_compiled_cell() {
    fn original_vector(interp: &mut Interp, _args: &[*mut TclObj]) -> Code {
        let head = Owned::fresh(obj::new_string_bytes(b"set"));
        let name = Owned::fresh(obj::new_string_bytes(b"foo(3)"));
        let value = Owned::fresh(obj::new_string_bytes(b"3"));
        let code =
            interp.eval_original_object_vector(&[head.as_ptr(), name.as_ptr(), value.as_ptr()]);
        let root = obj::native_variable_name::with_parsed(name.as_ptr(), |cache| {
            cache.array.as_ref().map(|(root, _)| root.as_ptr())
        })
        .flatten()
        .expect("original parsed root retained after failure");
        assert!(interp.original_c_local_index(root).is_some());
        code
    }
    let mut interp = Interp::with_native_core(
        super::super::super::default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    interp.register_builtin(b"original_vector_write", original_vector);
    assert_eq!(
        interp.eval_str(
            br#"
            namespace eval test_ns_var {variable foo 2}
            proc p {} {
                variable ::test_ns_var::foo
                set live [set foo]
                namespace delete ::test_ns_var
                set code [catch {original_vector_write} result]
                list $code $result $::errorCode
            }
            p
        "#
        ),
        Code::Ok,
        "{:?}",
        interp.native_execution_refusal()
    );
    let protocol =
        tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
    let capture = include_str!(
            "../../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.6.18/stdout.txt"
        );
    let line = capture
        .lines()
        .find(|line| line.starts_with("ROW variable-head-element "))
        .unwrap();
    let row = tcl_syntax::list::split_native_list_bytes(line.as_bytes(), protocol).unwrap();
    let expected = tcl_syntax::list::split_native_list_bytes(&row[3], protocol).unwrap();
    let result = interp.result_bytes();
    let actual = tcl_syntax::list::split_native_list_bytes(&result, protocol).unwrap();
    assert_eq!(actual.len(), 3);
    for (actual, expected) in actual.iter().zip(&expected[2..]) {
        assert_eq!(actual.as_ref(), expected.as_ref());
    }
    assert!(!interp.host_refusal_pending());
}

#[test]
fn detached_element_c86_original_var17_write_preserves_late_diagnostic() {
    trace_source_observation(
        "tcl8.6",
        include_bytes!(
            "../../../../tests/data/native_detached_element_write_sites/original-var17.tcl"
        ),
        include_bytes!(
            "../../../../tests/data/native_detached_element_write_sites/original-var17.txt"
        ),
    );
}

#[test]
fn detached_element_c86_recreated_root_keeps_old_link_write_detached() {
    trace_source_observation(
        "tcl8.6",
        include_bytes!(
            "../../../../tests/data/native_detached_element_write_sites/root-recreated-old-link.tcl"
        ),
        include_bytes!(
            "../../../../tests/data/native_detached_element_write_sites/root-recreated-old-link.txt"
        ),
    );
}

#[test]
fn detached_element_c86_recreated_root_admits_new_link_write() {
    trace_source_observation(
            "tcl8.6",
            include_bytes!(
                "../../../../tests/data/native_detached_element_write_sites/root-recreated-current-link.tcl"
            ),
            include_bytes!(
                "../../../../tests/data/native_detached_element_write_sites/root-recreated-current-link.txt"
            ),
        );
}

#[test]
fn array_root_write_diagnostics_match_five_original_source_controls() {
    // Native proof: naming.variable.original-array-root-write-diagnostic
    // docs/design/analysis/name-resolution-proofs/variable-original-array-root-write-diagnostic.md
    // Full source output is compared; no private receiver or compiler admission is inferred.
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../../tests/data/native_variable_array_write/8.4.20.txt")
                .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../../tests/data/native_variable_array_write/8.5.19.txt")
                .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../../tests/data/native_variable_array_write/8.6.18.txt")
                .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../../tests/data/native_variable_array_write/9.0.4.txt")
                .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../../tests/data/native_variable_array_write/9.1.0.txt")
                .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!("../../../../tests/data/native_variable_array_write/source.tcl"),
            expected,
        );
    }
}

#[test]
fn array_root_write_preserves_current_jim_dictionary_replacement() {
    // Native proof: naming.variable.original-array-root-write-diagnostic
    // docs/design/analysis/name-resolution-proofs/variable-original-array-root-write-diagnostic.md
    // Jim replaces its dictionary-valued root with the scalar result.
    trace_source_observation(
        "jim",
        include_bytes!("../../../../tests/data/native_variable_array_write/source.tcl"),
        include_bytes!("../../../../tests/data/native_variable_array_write/jim.txt"),
    );
}
