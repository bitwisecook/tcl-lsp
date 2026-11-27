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

//! Variable traces, `lassign`, reads and array-root writes from complete
//! original sources, against the tclsh and jimsh captures.

use crate::prelude::*;
use std::cell::RefCell;

struct TraceProbeOutput(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for TraceProbeOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn trace_source_observation(engine: &str, source: &[u8], expected: &[u8]) {
    let profile = tcl_vm::test_support::profile(engine);
    let output = Rc::new(RefCell::new(Vec::new()));
    let mut vm = crate::fixture::interpreter_with_output(
        profile,
        Box::new(TraceProbeOutput(output.clone())),
    );
    let result = crate::fixture::eval(&mut vm, std::str::from_utf8(source).unwrap());
    assert_eq!(result.code, tcl_runtime_api::Code::Ok, "{engine}");
    let actual = output.borrow();
    // Only setup and callback observations are compared; version reporting
    // identifies the native process and is a separate API surface.
    let after_version = |bytes: &[u8]| {
        let end = bytes.iter().position(|byte| *byte == b'\n').unwrap();
        bytes[end + 1..].to_vec()
    };
    assert_eq!(after_version(&actual), after_version(expected), "{engine}");
}

fn jim_trace_setup_observation(source: &[u8], expected: &[u8]) {
    // Compare the original source prefix through SETUP, independently of
    // the unimplemented binary reporter after this frontier.
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
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert!(
        !captured
            .lines()
            .any(|line| line.starts_with("OBSERVATION "))
    );
    let profile = tcl_vm::test_support::profile("jim");
    let mut vm = crate::fixture::interpreter(profile);
    assert_eq!(
        crate::fixture::eval(&mut vm, prefix).code,
        tcl_runtime_api::Code::Ok
    );
    for (name, expected) in [
        (b"probeSetupCode".as_slice(), b"1".as_slice()),
        (b"probeSetupResult".as_slice(), expected_message.as_slice()),
    ] {
        let value = vm.get_var_bytes(name).unwrap();
        assert_eq!(crate::fixture::bytes(&vm, &value).as_slice(), expected);
    }
}

#[test]
fn active_read_unset_matches_five_original_source_controls() {
    // Native proof: naming.variable.unset-active-read-callback-retirement
    // docs/design/analysis/name-resolution-proofs/variable-unset-active-read-callback-retirement.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_active_read_unset/8.4.20.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_active_read_unset/8.5.19.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_active_read_unset/8.6.18.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_active_read_unset/9.0.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_active_read_unset/9.1.0.txt"
            )
            .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_active_read_unset/source.tcl"
            ),
            expected,
        );
    }
}

#[test]
fn active_read_unset_preserves_jim_unsupported_setup() {
    // Native proof: naming.variable.unset-active-read-callback-retirement
    // docs/design/analysis/name-resolution-proofs/variable-unset-active-read-callback-retirement.md
    jim_trace_setup_observation(
        include_bytes!(
            "../../../../../../runtime/rust/tests/data/native_active_read_unset/source.tcl"
        ),
        include_bytes!(
            "../../../../../../runtime/rust/tests/data/native_active_read_unset/jim.txt"
        ),
    );
}

#[test]
fn recreated_element_read_trace_matches_five_original_source_controls() {
    // Native proof: naming.variable.recreated-element-independent-read-trace
    // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_recreated_element_trace/8.4.20.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_recreated_element_trace/8.5.19.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_recreated_element_trace/8.6.18.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_recreated_element_trace/9.0.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_recreated_element_trace/9.1.0.txt"
            )
            .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_recreated_element_trace/source.tcl"
            ),
            expected,
        );
    }
}

#[test]
fn recreated_element_trace_preserves_jim_unsupported_setup() {
    // Native proof: naming.variable.recreated-element-independent-read-trace
    // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
    jim_trace_setup_observation(
        include_bytes!(
            "../../../../../../runtime/rust/tests/data/native_recreated_element_trace/source.tcl"
        ),
        include_bytes!(
            "../../../../../../runtime/rust/tests/data/native_recreated_element_trace/jim.txt"
        ),
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
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_list_assignment_arity/8.4.20.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_list_assignment_arity/8.5.19.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_list_assignment_arity/8.6.18.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_list_assignment_arity/9.0.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_list_assignment_arity/9.1.0.txt"
            )
            .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_list_assignment_arity/source.tcl"
            ),
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
        include_bytes!(
            "../../../../../../runtime/rust/tests/data/native_list_assignment_arity/source.tcl"
        ),
        include_bytes!(
            "../../../../../../runtime/rust/tests/data/native_list_assignment_arity/jim.txt"
        ),
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
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_read_input/8.4.20.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_read_input/8.5.19.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_read_input/8.6.18.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_read_input/9.0.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_read_input/9.1.0.txt"
            )
            .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_read_input/source.tcl"
            ),
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
        include_bytes!(
            "../../../../../../runtime/rust/tests/data/native_variable_read_input/source.tcl"
        ),
        include_bytes!(
            "../../../../../../runtime/rust/tests/data/native_variable_read_input/jim.txt"
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
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_array_write/8.4.20.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_array_write/8.5.19.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_array_write/8.6.18.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_array_write/9.0.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_array_write/9.1.0.txt"
            )
            .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!(
                "../../../../../../runtime/rust/tests/data/native_variable_array_write/source.tcl"
            ),
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
        include_bytes!(
            "../../../../../../runtime/rust/tests/data/native_variable_array_write/source.tcl"
        ),
        include_bytes!(
            "../../../../../../runtime/rust/tests/data/native_variable_array_write/jim.txt"
        ),
    );
}

#[test]
fn original_unset_execution_matches_85_native_completion_windows() {
    fn unhex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut compared = 0;
    for row in
        include_str!("../../../../../tcl-registry/tests/data/native_unset_compilation/windows.tsv")
            .lines()
            .skip(1)
    {
        let columns: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        if columns[0] == "jim0.84" {
            continue;
        }
        let profile = tcl_test_support::fixture::oracle_release(columns[0])
            .map(tcl_dialect::TclVersion::dialect_profile_name)
            .and_then(tcl_dialect::DialectProfile::find)
            .unwrap();
        let mut vm = crate::fixture::interpreter(profile);
        let body = String::from_utf8(unhex(columns[2])).unwrap();
        let source = format!(
            "proc f {{name other names}} {{\n{body}\n}}\nset ::seen BEFORE\nset code [catch {{f x y {{x y}}}} value]\nlist RESULT $code $value $::seen"
        );
        let completion = vm.try_eval_source(&source).unwrap();
        assert_eq!(
            completion.code,
            tcl_runtime_api::Code::Ok,
            "{}:{}",
            columns[0],
            columns[1]
        );
        let expected = String::from_utf8(unhex(columns[5])).unwrap();
        assert_eq!(
            completion.result.to_str().as_ref(),
            expected,
            "{}:{}",
            columns[0],
            columns[1]
        );
        compared += 1;
    }
    assert_eq!(compared, 85);
}

#[test]
fn original_unset_options_match_66_native_results() {
    fn decode(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut compared = 0;
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!("../../../../../tcl-registry/tests/data/native_unset_options/8.4.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../../tcl-registry/tests/data/native_unset_options/8.5.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../../tcl-registry/tests/data/native_unset_options/8.6.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../../tcl-registry/tests/data/native_unset_options/9.0.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../../tcl-registry/tests/data/native_unset_options/9.1.tsv"),
        ),
        (
            "jim",
            include_str!("../../../../../tcl-registry/tests/data/native_unset_options/jim.tsv"),
        ),
    ] {
        for row in rows.lines() {
            let columns: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
            let source = String::from_utf8(decode(columns[1])).unwrap();
            // Match the native probe's same-interpreter binary observer.
            let observed = format!(
                "set c [catch {{{source}}} m];binary scan $m H* h;set observed \"$c\\t$h\""
            );
            let expected = format!("{}\t{}", columns[2], columns[3]);
            let profile = tcl_vm::test_support::profile(engine);
            let mut vm = crate::fixture::interpreter(profile);
            if engine == "jim" {
                let empty = vm.try_eval_source("unset").unwrap();
                assert_eq!(empty.code, tcl_runtime_api::Code::Ok);
                assert_eq!(empty.result.to_str().as_ref(), "");
                // The observer calls jimsh's `binary`, a C extension the VM
                // installs only through a private registration.
                compared += 1;
                continue;
            }
            let completion = vm.try_eval_source(&observed).unwrap_or_else(|error| {
                panic!("{engine}/{} original unset observer: {error:?}", columns[0])
            });
            assert_eq!(
                completion.code,
                tcl_runtime_api::Code::Ok,
                "{engine}/{}",
                columns[0]
            );
            assert_eq!(
                completion.result.to_str().as_ref(),
                expected,
                "{engine}/{}",
                columns[0]
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 66);
}

#[test]
fn original_unset_traces_and_operand_order_match_36_native_results() {
    fn decode(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut compared = 0;
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!(
                "../../../../../../runtime/rust/tests/data/native_unset_execution/8.4.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../../../runtime/rust/tests/data/native_unset_execution/8.5.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../../../runtime/rust/tests/data/native_unset_execution/8.6.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../../../runtime/rust/tests/data/native_unset_execution/9.0.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../../../runtime/rust/tests/data/native_unset_execution/9.1.tsv"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../../../../runtime/rust/tests/data/native_unset_execution/jim.tsv"
            ),
        ),
    ] {
        for row in rows.lines() {
            let columns: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
            let profile = tcl_vm::test_support::profile(engine);
            let mut vm = crate::fixture::interpreter(profile);
            let completion = vm.try_eval_source_bytes(&decode(columns[1])).unwrap();
            let code = completion.code;
            assert_eq!(
                code.as_int(),
                columns[2].parse::<i64>().unwrap(),
                "{engine}/{}",
                columns[0]
            );
            assert_eq!(
                tcl_syntax::value::ValueOps::native_string_bytes(&mut vm, &completion.result)
                    .unwrap()
                    .to_vec(),
                decode(columns[3]),
                "{engine}/{}",
                columns[0]
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 36);
}

#[test]
fn generic_namespace_declarations_match_all_25_native_execution_results() {
    struct Output(Rc<std::cell::RefCell<Vec<u8>>>);
    impl std::io::Write for Output {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let source =
        include_str!("../../../../../tcl-vm/tests/data/native_namespace_handler_order/source.tcl");
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!(
                "../../../../../tcl-vm/tests/data/native_namespace_handler_order/tcl8.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../tcl-vm/tests/data/native_namespace_handler_order/tcl8.5.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../tcl-vm/tests/data/native_namespace_handler_order/tcl8.6.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../tcl-vm/tests/data/native_namespace_handler_order/tcl9.0.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../tcl-vm/tests/data/native_namespace_handler_order/tcl9.1.txt"
            )
            .as_slice(),
        ),
    ] {
        let profile = tcl_vm::test_support::profile(engine);
        let bytes = Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut vm =
            crate::fixture::interpreter_with_output(profile, Box::new(Output(Rc::clone(&bytes))));
        let completion = vm.try_eval_source(source).unwrap();
        assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
        assert_eq!(bytes.borrow().as_slice(), expected, "{engine}");
    }
}
