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
use std::cell::RefCell;
use tcl_core_types::VarId;
use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;

fn actual(version: &str) -> Vm {
    let profile = tcl_registry::model::ingress::resolve_environment(version).unit_profile();
    let mut vm = Vm::new();
    assert!(vm.set_native_engine_profile(profile));
    vm.set_dialect_profile(profile);
    vm
}

#[test]
fn confined_quiet_unset_preserves_lookup_misses_and_refuses_foreign_receivers() {
    for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(version);
        vm.set_var("::seed", Value::string("old")).unwrap();
        vm.set_array_elem("::arr", "k", Value::string("held"))
            .unwrap();
        vm.push_call_frame(Some("confined".into()), Vec::new());
        vm.set_stores_confined(true);
        for name in ["::missing_ns::x", "::missing_ns::a(k)", "absent"] {
            vm.unset_original_c_variable(&Value::string(name), false)
                .unwrap_or_else(|error| panic!("{version} {name}: {error:?}"));
        }
        vm.unset_original_c_variable_parts(
            &Value::string("::missing_ns::a"),
            &Value::string("k"),
            false,
        )
        .unwrap();
        vm.set_var("local", Value::string("local")).unwrap();
        vm.unset_original_c_variable(&Value::string("local"), false)
            .unwrap();
        vm.set_array_elem("local_array", "k", Value::string("local"))
            .unwrap();
        vm.unset_original_c_variable_parts(
            &Value::string("local_array"),
            &Value::string("absent"),
            false,
        )
        .unwrap();
        vm.unset_original_c_variable_parts(
            &Value::string("local_array"),
            &Value::string("k"),
            false,
        )
        .unwrap();
        for complain in [false, true] {
            let error = vm
                .unset_original_c_variable(&Value::string("::seed"), complain)
                .unwrap_err();
            assert_eq!(
                error.result.string_bytes().as_ref(),
                b"can't unset \"::seed\": stores are confined to the activation",
                "{version}"
            );
            let error = vm
                .unset_original_c_variable_parts(
                    &Value::string("::arr"),
                    &Value::string("k"),
                    complain,
                )
                .unwrap_err();
            assert_eq!(
                error.result.string_bytes().as_ref(),
                b"can't unset \"::arr\": stores are confined to the activation",
                "{version}"
            );
        }
        assert_eq!(vm.get_var("::seed").unwrap().to_str().as_ref(), "old");
        assert_eq!(
            vm.get_array_elem("::arr", "k").unwrap().to_str().as_ref(),
            "held"
        );
        assert!(
            vm.var_binding_from_bytes(b"::missing_ns::x", vm.current_level())
                .is_none()
        );
    }
}

#[test]
fn confined_generic_capture_checks_name_link_before_following_its_local_target() {
    let mut vm = actual("tcl9.0");
    vm.push_call_frame(Some("confined".into()), Vec::new());
    vm.set_var("target", Value::string("old")).unwrap();
    vm.set_array_elem("target_array", "k", Value::string("held"))
        .unwrap();
    let level = vm.current_level();
    for (alias, target, element) in [
        ("alias", "target", None),
        ("array_alias", "target_array", Some(b"k".as_slice())),
    ] {
        let binding = vm.var_binding_from_bytes(alias.as_bytes(), level).unwrap();
        let target_binding = vm.var_binding_from_bytes(target.as_bytes(), level).unwrap();
        vm.bind_new_var(
            &binding,
            super::super::Local::NameLink(crate::vars::NameLink {
                original_target: None,
                binding: target_binding,
                element: None,
            }),
        )
        .unwrap();
        vm.set_stores_confined(true);
        assert!(
            vm.capture_update_cell(alias.as_bytes(), element).is_err(),
            "{alias}"
        );
        assert!(
            vm.capture_update_cell(target.as_bytes(), element).is_ok(),
            "{target}"
        );
    }
    assert_eq!(vm.get_var("target").unwrap().to_str().as_ref(), "old");
    assert_eq!(
        vm.get_array_elem("target_array", "k")
            .unwrap()
            .to_str()
            .as_ref(),
        "held"
    );
}

#[test]
fn confined_array_store_retains_the_first_host_refusal() {
    let mut vm = actual("tcl9.0");
    vm.push_call_frame(Some("confined".into()), Vec::new());
    vm.set_stores_confined(true);
    let prior = vm.refuse_host_command("terminal array write refusal".into());
    let error = vm
        .set_array_elem_bytes_from(vm.current_level(), b"::fresh", b"k", Value::int(1), true)
        .unwrap_err();
    assert_eq!(error.result.string_bytes(), prior.result.string_bytes());
    let error = vm
        .store_elem_result_bytes(b"::fresh", b"k", Value::int(1))
        .unwrap_err();
    assert_eq!(error.result.string_bytes(), prior.result.string_bytes());
    let binding = vm
        .var_binding_from_bytes(b"::fresh", vm.current_level())
        .unwrap();
    assert!(vm.raw_variable_binding(&binding).is_none());
}

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
    let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
    let output = Rc::new(RefCell::new(Vec::new()));
    let mut vm = crate::native_fixture::interpreter_with_output(
        profile,
        Box::new(TraceProbeOutput(output.clone())),
    );
    let result = vm
        .eval_source(std::str::from_utf8(source).unwrap())
        .unwrap();
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
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut vm = crate::native_fixture::interpreter(profile);
    assert_eq!(
        vm.eval_source(prefix).unwrap().code,
        tcl_runtime_api::Code::Ok
    );
    for (name, expected) in [
        (b"probeSetupCode".as_slice(), b"1".as_slice()),
        (b"probeSetupResult".as_slice(), expected_message.as_slice()),
    ] {
        let value = vm.get_var_bytes(name).unwrap();
        assert_eq!(
            vm.native_name_operand_bytes(&value).unwrap().as_ref(),
            expected
        );
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
                "../../../../../runtime/rust/tests/data/native_active_read_unset/8.4.20.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_active_read_unset/8.5.19.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_active_read_unset/8.6.18.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_active_read_unset/9.0.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_active_read_unset/9.1.0.txt"
            )
            .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_active_read_unset/source.tcl"
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
            "../../../../../runtime/rust/tests/data/native_active_read_unset/source.tcl"
        ),
        include_bytes!("../../../../../runtime/rust/tests/data/native_active_read_unset/jim.txt"),
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
                "../../../../../runtime/rust/tests/data/native_recreated_element_trace/8.4.20.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_recreated_element_trace/8.5.19.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_recreated_element_trace/8.6.18.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_recreated_element_trace/9.0.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_recreated_element_trace/9.1.0.txt"
            )
            .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_recreated_element_trace/source.tcl"
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
            "../../../../../runtime/rust/tests/data/native_recreated_element_trace/source.tcl"
        ),
        include_bytes!(
            "../../../../../runtime/rust/tests/data/native_recreated_element_trace/jim.txt"
        ),
    );
}

#[test]
fn original_unset_parts_keep_part1_cache_and_quiet_missing_lookup() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_array_elem_bytes(b"a(k)", b"index", Value::string("ORIGINAL"))
            .unwrap();
        vm.set_array_elem_bytes(b"a", b"k", Value::string("SIBLING"))
            .unwrap();
        let root = Value::new_native_string_bytes(b"a(k)".as_slice());
        let index = Value::new_native_string_bytes(b"index".as_slice());
        vm.unset_original_c_variable_parts(&root, &index, true)
            .unwrap();
        assert!(
            vm.get_array_elem_bytes_from(vm.current_level(), b"a(k)", b"index")
                .is_none(),
            "{engine}"
        );
        assert_eq!(
            vm.get_array_elem_bytes_from(vm.current_level(), b"a", b"k")
                .unwrap()
                .to_str()
                .as_ref(),
            "SIBLING"
        );
        assert!(
            !root
                .with_native_parsed_variable(|cache| cache.array.is_some())
                .unwrap_or(false)
        );
        let missing = Value::new_native_string_bytes(b"::absent::x".as_slice());
        assert!(
            vm.unset_original_c_variable(&missing, false).is_ok(),
            "{engine}"
        );
        assert!(vm.refused_completion().is_none());
        assert!(
            vm.unset_original_c_variable(&missing, true).is_err(),
            "{engine}"
        );
    }
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
        include_str!("../../../../tcl-registry/tests/data/native_unset_compilation/windows.tsv")
            .lines()
            .skip(1)
    {
        let columns: Vec<_> = row.split('\t').collect();
        if columns[0] == "jim0.84" {
            continue;
        }
        let profile = tcl_dialect::DialectProfile::find(&format!("tcl{}", columns[0])).unwrap();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
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
        assert!(
            vm.refused_completion().is_none(),
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
            include_str!("../../../../tcl-registry/tests/data/native_unset_options/8.4.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../tcl-registry/tests/data/native_unset_options/8.5.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../tcl-registry/tests/data/native_unset_options/8.6.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../tcl-registry/tests/data/native_unset_options/9.0.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../tcl-registry/tests/data/native_unset_options/9.1.tsv"),
        ),
        (
            "jim",
            include_str!("../../../../tcl-registry/tests/data/native_unset_options/jim.tsv"),
        ),
    ] {
        for row in rows.lines() {
            let columns: Vec<_> = row.split('\t').collect();
            let source = String::from_utf8(decode(columns[1])).unwrap();
            // Match the native probe's same-interpreter binary observer.
            let observed = format!(
                "set c [catch {{{source}}} m];binary scan $m H* h;set observed \"$c\\t$h\""
            );
            let expected = format!("{}\t{}", columns[2], columns[3]);
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            if engine == "jim" {
                let empty = vm.try_eval_source("unset").unwrap();
                assert_eq!(empty.code, tcl_runtime_api::Code::Ok);
                assert_eq!(empty.result.to_str().as_ref(), "");
                assert!(vm.refused_completion().is_none());
                // jimsh's binary observer is a distribution extension;
                // it is independent of the native core issuer under test.
                crate::cmd_binary::register(&mut vm);
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
            assert!(vm.refused_completion().is_none(), "{engine}/{}", columns[0]);
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
            include_str!("../../../../../runtime/rust/tests/data/native_unset_execution/8.4.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../../runtime/rust/tests/data/native_unset_execution/8.5.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../../runtime/rust/tests/data/native_unset_execution/8.6.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../../runtime/rust/tests/data/native_unset_execution/9.0.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../../runtime/rust/tests/data/native_unset_execution/9.1.tsv"),
        ),
        (
            "jim",
            include_str!("../../../../../runtime/rust/tests/data/native_unset_execution/jim.tsv"),
        ),
    ] {
        for row in rows.lines() {
            let columns: Vec<_> = row.split('\t').collect();
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
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
            assert!(vm.refused_completion().is_none(), "{engine}/{}", columns[0]);
            compared += 1;
        }
    }
    assert_eq!(compared, 36);
}

thread_local! {
    static ALIAS_SIMPLE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    static ALIAS_CANONICAL_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn alias_canonical_row(stage: &str, case: usize, original: &Value) {
    let canonical = match original.native_object_snapshot().cache {
        Cache::List { canonical, .. } => i32::from(canonical),
        _ => -1,
    };
    ALIAS_CANONICAL_ROWS.with(|rows| {
        rows.borrow_mut().push(format!("{stage}{case}|{canonical}"));
    });
}

fn alias_simple_probe(vm: &mut Vm, _argv: &[Value]) -> Completion<Value> {
    for (case, name) in [b"v".as_slice(), b"d", b"n\0(k)"].into_iter().enumerate() {
        let local = if case == 2 {
            Value::new_native_string_bytes(name)
        } else {
            Value::list(vec![Value::new_native_string_bytes(name)])
        };
        let kind = if case == 2 { "NULL" } else { "list" };
        ALIAS_SIMPLE_ROWS.with(|rows| {
            rows.borrow_mut().push(format!(
                "before{case}|{kind}|{}",
                local.native_object_reference_count(),
            ));
        });
        alias_canonical_row("before", case, &local);
        let target = Value::new_native_string_bytes(b"x".as_slice());
        if let Err(error) = vm.link_upvar_original(0, &target, &local) {
            return error;
        }
        alias_canonical_row("after", case, &local);
        ALIAS_SIMPLE_ROWS.with(|rows| {
            rows.borrow_mut().push(format!(
                "after{case}|0|{kind}|{}",
                local.native_object_reference_count(),
            ));
        });
    }
    crate::interp::ok(Value::empty())
}

thread_local! {
    static GLOBAL_CACHE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn global_cache_probe(vm: &mut Vm, argv: &[Value]) -> Completion<Value> {
    let value = match vm.read_original_c_variable(&argv[0]) {
        Ok(value) => value,
        Err(error) => return error,
    };
    let bytes = vm.native_name_operand_bytes(&value).unwrap();
    GLOBAL_CACHE_ROWS.with(|rows| {
        rows.borrow_mut().push(format!(
            "{}|{}",
            argv[0].native_object_type_name(),
            String::from_utf8(bytes.to_vec()).unwrap(),
        ));
    });
    crate::interp::ok(Value::empty())
}
#[test]
fn dynamic_global_uses_original_compiler_token_tail_and_name_cache() {
    // Native proof: naming.variable.original-global-cache-token
    // docs/design/analysis/name-resolution-proofs/variable-original-global-cache-token.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_str!("../../../tests/data/native_global_cache_token/8.4.20.txt"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tests/data/native_global_cache_token/8.5.19.txt"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tests/data/native_global_cache_token/8.6.18.txt"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tests/data/native_global_cache_token/9.0.4.txt"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tests/data/native_global_cache_token/9.1.0.txt"),
        ),
    ] {
        GLOBAL_CACHE_ROWS.with(|rows| rows.borrow_mut().clear());
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm.register("probe", global_cache_probe);
        vm.set_var_bytes(b"name", Value::new_native_string_bytes(b"v".as_slice()))
            .unwrap();
        for (label, source) in [
            (
                "simple",
                "set v GLOBAL;proc p {} {set v LOCAL;probe $::name;unset v;global $::name;probe $::name;return $v};p",
            ),
            (
                "qualified",
                "namespace eval N {variable v QUALIFIED};set name ::N::v;p",
            ),
        ] {
            let completion = vm.try_eval_source(source).unwrap();
            GLOBAL_CACHE_ROWS.with(|rows| {
                rows.borrow_mut().push(format!(
                    "{label}|{}|{}",
                    completion.code.as_int(),
                    String::from_utf8(completion.result.string_bytes().to_vec()).unwrap(),
                ));
            });
            assert!(vm.refused_completion().is_none(), "{engine}");
        }
        let rows = GLOBAL_CACHE_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
        assert_eq!(rows, expected, "{engine}");
    }
}

#[test]
fn alias_local_simple_lookup_matches_all_15_native_primary_and_key_owners() {
    // Native proof: naming.alias.original-local-name-object
    // docs/design/analysis/name-resolution-proofs/alias-original-local-name-object.md
    // Native proof: naming.alias.original-local-name-list-canonical
    // docs/design/analysis/name-resolution-proofs/alias-original-local-name-list-canonical.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_str!("../../../tests/data/native_alias_simple/8.4.20.txt"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tests/data/native_alias_simple/8.5.19.txt"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tests/data/native_alias_simple/8.6.18.txt"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tests/data/native_alias_simple/9.0.4.txt"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tests/data/native_alias_simple/9.1.0.txt"),
        ),
    ] {
        ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow_mut().clear());
        ALIAS_CANONICAL_ROWS.with(|rows| rows.borrow_mut().clear());
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm.register("alias_probe", alias_simple_probe);
        let completion = vm
            .try_eval_source("set x X; proc p {} {alias_probe; return $v}; p")
            .unwrap();
        assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
        assert_eq!(completion.result.string_bytes().as_ref(), b"X");
        ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow_mut().push("completion|0|X".into()));
        let rows = ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
        assert_eq!(rows, expected, "{engine}");
        let canonical = match engine {
            "tcl8.4" => {
                include_str!("../../../tests/data/native_alias_simple/canonical/8.4.20.txt")
            }
            "tcl8.5" => {
                include_str!("../../../tests/data/native_alias_simple/canonical/8.5.19.txt")
            }
            "tcl8.6" => {
                include_str!("../../../tests/data/native_alias_simple/canonical/8.6.18.txt")
            }
            "tcl9.0" => {
                include_str!("../../../tests/data/native_alias_simple/canonical/9.0.4.txt")
            }
            "tcl9.1" => {
                include_str!("../../../tests/data/native_alias_simple/canonical/9.1.0.txt")
            }
            _ => unreachable!(),
        };
        let rows = ALIAS_CANONICAL_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
        assert_eq!(rows, canonical, "{engine}");
        assert!(vm.refused_completion().is_none());
    }
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
    let source = include_str!("../../../tests/data/native_namespace_handler_order/source.tcl");
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../tests/data/native_namespace_handler_order/tcl8.4.txt")
                .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../tests/data/native_namespace_handler_order/tcl8.5.txt")
                .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../tests/data/native_namespace_handler_order/tcl8.6.txt")
                .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../tests/data/native_namespace_handler_order/tcl9.0.txt")
                .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../tests/data/native_namespace_handler_order/tcl9.1.txt")
                .as_slice(),
        ),
    ] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let bytes = Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut vm = Vm::with_native_core(
            Box::new(Output(Rc::clone(&bytes))),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        let completion = vm.try_eval_source(source).unwrap();
        assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
        assert_eq!(bytes.borrow().as_slice(), expected, "{engine}");
        assert!(vm.refused_completion().is_none());
    }
}

fn namespace_cache_probe(vm: &mut Vm, argv: &[Value]) -> Completion<Value> {
    let original = &argv[0];
    assert_eq!(
        vm.read_original_c_variable(original)
            .unwrap()
            .string_bytes()
            .as_ref(),
        b"LOCAL"
    );
    assert!(original.with_native_local_variable(|_| ()).is_some());
    let slot = vm
        .compiled_local_dynamic_slot(vm.current_level(), b"alias")
        .unwrap();
    let before = vm.native_original_local_binding(original).unwrap();
    let before_cell = vm.raw_variable_binding(&before).unwrap();
    let namespace = vm.current_ns_id();
    vm.link_compiled_namespace_original(slot, namespace, original, true)
        .unwrap();
    assert!(original.with_native_local_variable(|_| ()).is_none());
    assert!(
        original
            .with_native_parsed_variable(|cache| cache.array.is_none())
            .unwrap()
    );
    assert_eq!(vm.raw_variable_binding(&before), Some(before_cell));
    assert_eq!(
        vm.get_var_from_bytes(vm.current_level(), b"v")
            .unwrap()
            .string_bytes()
            .as_ref(),
        b"LOCAL"
    );
    assert_eq!(
        vm.read_compiled_variable_result(slot, None)
            .unwrap()
            .string_bytes()
            .as_ref(),
        b"GLOBAL"
    );
    super::super::ok(Value::new_native_string_bytes(b"".as_slice()))
}

#[test]
fn namespace_opcodes_bypass_original_local_cache_and_retain_the_namespace_cell() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(Vec::<u8>::new()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm.register("namespace_cache_probe", namespace_cache_probe);
        let original = Value::new_native_string_bytes(b"v".as_slice());
        vm.set_var("name", original.clone()).unwrap();
        let completion = vm.try_eval_source(
                "set v GLOBAL; proc p {} {set v LOCAL; namespace_cache_probe $::name; return $alias}; p",
            ).unwrap();
        assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
        assert_eq!(completion.result.string_bytes().as_ref(), b"GLOBAL");
        assert!(
            original
                .with_native_parsed_variable(|cache| cache.array.is_none())
                .unwrap()
        );
        assert!(vm.refused_completion().is_none());
    }
}

#[test]
fn namespace_original_links_preserve_array_parts_and_reject_defined_aliases() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(Vec::<u8>::new()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let original = Value::new_native_string_bytes(b"arr(k)".as_slice());
        let value = Value::new_native_string_bytes(b"VALUE".as_slice());
        vm.store_original_named_variable(&original, value.clone())
            .unwrap();
        vm.push_call_frame(Some("p".into()), Vec::new());
        vm.install_compiled_local_slots(&[tcl_core_types::NameBytes::from(b"alias".as_slice())])
            .unwrap();
        vm.link_compiled_namespace_original(0, tcl_core_types::ROOT_NS, &original, false)
            .unwrap();
        assert!(
            vm.read_compiled_variable_result(0, None)
                .unwrap()
                .is_same_object(&value)
        );
        assert!(
            original
                .with_native_parsed_variable(|cache| cache.array.is_some())
                .unwrap()
        );
        vm.pop_call_frame();
        vm.push_call_frame(Some("p".into()), Vec::new());
        vm.install_compiled_local_slots(&[tcl_core_types::NameBytes::from(b"alias".as_slice())])
            .unwrap();
        vm.bind_compiled_formal_slot(0, Value::string("LOCAL"))
            .unwrap();
        let failure = vm
            .link_compiled_namespace_original(0, tcl_core_types::ROOT_NS, &original, true)
            .unwrap_err();
        assert_eq!(failure.code, tcl_runtime_api::Code::Error);
        assert_eq!(
            vm.read_compiled_variable_result(0, None)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"LOCAL"
        );
        assert!(vm.refused_completion().is_none());
        vm.pop_call_frame();
    }
}

thread_local! {
    static CACHE_LOOKUP_CASE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CACHE_LOOKUP_CALLBACKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CACHE_LOOKUP_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}
struct CacheLookupObserver;
impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Vm> for CacheLookupObserver {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _vm: &mut Vm,
        _access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        CACHE_LOOKUP_CALLBACKS.with(|count| count.set(count.get() + 1));
        Ok(())
    }
}
fn cache_lookup_probe(vm: &mut Vm, argv: &[Value]) -> Completion<Value> {
    use tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation as Op;
    let original = &argv[0];
    let case = CACHE_LOOKUP_CASE.with(std::cell::Cell::get);
    let version = vm.native_c_variable_name_protocol().unwrap().version();
    vm.read_original_c_variable(original).unwrap();
    assert!(original.with_native_local_variable(|_| ()).is_some());
    let before = vm.native_original_local_binding(original).unwrap();
    let before_cell = vm.raw_variable_binding(&before).unwrap();
    if matches!(case, 2 | 3) {
        vm.add_native_variable_observer(
            original,
            &[Op::Read, Op::Write],
            Rc::new(CacheLookupObserver),
        )
        .unwrap();
    }
    if case == 4 {
        assert!(vm.unset_var_bytes(b"x"));
    }
    original.invalidate_native_string_for_test();
    let result = if matches!(case, 1 | 3) {
        vm.store_original_c_variable(original, Value::new_native_string_bytes(b"NEXT".as_slice()))
    } else {
        vm.read_original_c_variable(original)
    };
    assert_eq!(vm.raw_variable_binding(&before), Some(before_cell));
    if version >= tcl_dialect::TclVersion::V8_5 && case >= 2 {
        assert!(result.is_err());
        assert!(
            vm.refused_completion().is_some(),
            "native updater abort must remain a host refusal"
        );
        assert!(original.resident_string_bytes().is_none());
        assert_eq!(CACHE_LOOKUP_CALLBACKS.with(std::cell::Cell::get), 0);
    } else {
        assert_eq!(result.is_err(), case == 4);
        assert_eq!(
            original.resident_string_bytes().is_some(),
            version == tcl_dialect::TclVersion::V8_4
        );
        assert_eq!(
            CACHE_LOOKUP_CALLBACKS.with(std::cell::Cell::get),
            usize::from(matches!(case, 2 | 3))
        );
    }
    let version_label = match version {
        tcl_dialect::TclVersion::V8_4 => "8.4.20",
        tcl_dialect::TclVersion::V8_5 => "8.5.19",
        tcl_dialect::TclVersion::V8_6 => "8.6.18",
        tcl_dialect::TclVersion::V9_0 => "9.0.4",
        tcl_dialect::TclVersion::V9_1 => "9.1.0",
    };
    let row = if vm.refused_completion().is_some() {
        format!("{version_label}|{case}|updater-unavailable")
    } else {
        format!(
            "{version_label}|{case}|{}|{}|localVarName|1",
            usize::from(result.is_err()),
            usize::from(original.resident_string_bytes().is_some())
        )
    };
    CACHE_LOOKUP_ROWS.with(|rows| rows.borrow_mut().push(row));
    if case != 4 {
        assert_eq!(
            vm.get_var_from_bytes(vm.current_level(), b"x")
                .unwrap()
                .string_bytes()
                .as_ref(),
            if matches!(case, 1 | 3) {
                b"NEXT".as_slice()
            } else {
                b"VALUE".as_slice()
            }
        );
    }
    result.map_or_else(|error| error, crate::interp::ok)
}

#[test]
fn original_local_cache_getter_order_matches_all_25_native_paths() {
    // Native proof: naming.variable.original-local-cache-before-getter
    // docs/design/analysis/name-resolution-proofs/variable-original-local-cache-before-getter.md
    // Native proof: naming.variable.original-local-cache-unavailable-updater
    // docs/design/analysis/name-resolution-proofs/variable-original-local-cache-unavailable-updater.md
    let expected = include_str!(
        "../../../../tcl-syntax/tests/data/native_variable_name/cache_lookup/paths.txt"
    );
    assert_eq!(expected.lines().count(), 25);
    CACHE_LOOKUP_ROWS.with(|rows| rows.borrow_mut().clear());
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for case in 0..5 {
            CACHE_LOOKUP_CASE.with(|mode| mode.set(case));
            CACHE_LOOKUP_CALLBACKS.with(|count| count.set(0));
            let mut vm = actual(environment);
            vm.set_compiler(Box::new(
                tcl_codegen::compile_service::BytecodeCompileService::for_profile(
                    vm.dialect_profile(),
                ),
            ));
            vm.register("probe", cache_lookup_probe);
            let result = vm.try_eval_source("proc p {} {set x VALUE;probe x};p");
            if environment != "tcl8.4" && case >= 2 {
                assert!(
                    result.is_err(),
                    "{environment}/{case}: typed updater refusal"
                );
            } else {
                let completion = result.unwrap();
                assert_eq!(completion.code.is_ok(), case != 4, "{environment}/{case}");
            }
        }
    }
    CACHE_LOOKUP_ROWS.with(|rows| {
        assert_eq!(
            rows.borrow().as_slice(),
            expected.lines().collect::<Vec<_>>()
        );
    });
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        use std::fmt::Write;
        write!(output, "{byte:02x}").expect("writing into a String");
        output
    })
}
fn observe(version: &str, case: &str, phase: &str, original: &Value) -> String {
    let snapshot = original.native_object_snapshot();
    let kind = match snapshot.cache {
        Cache::None => "none",
        Cache::Numeric(_) => "int",
        Cache::List { .. } => "list",
        Cache::ByteArray { .. } => "bytearray",
        Cache::ParsedVariableName { .. } => "parsedVarName",
        ref other => panic!("unexpected actual original cache: {other:?}"),
    };
    format!(
        "{version}|{case}|{phase}|{kind}|{}|{}|{}|{}",
        original.native_object_reference_count(),
        usize::from(original.native_primary_has_free_hook()),
        usize::from(snapshot.resident.is_some()),
        snapshot.resident.as_deref().map_or_else(String::new, hex)
    )
}

use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
};
struct ElementCallbackWindows {
    version: &'static str,
    key_k: crate::value::NativeObjectLifetimeLease,
    key_j: crate::value::NativeObjectLifetimeLease,
    old_k: VarId,
    old_j: VarId,
    object_table: bool,
    rows: Rc<RefCell<Vec<String>>>,
    sequence: Rc<RefCell<usize>>,
}
impl ElementCallbackWindows {
    fn record(&self, vm: &Vm, phase: &str) {
        let (dead_k, defined_k, refs_k) = vm
            .var_arena
            .observe_native_element(self.old_k, self.object_table);
        let (dead_j, defined_j, refs_j) = vm
            .var_arena
            .observe_native_element(self.old_j, self.object_table);
        let current_k = vm
            .resolve_var_from_bytes(b"arr(k)", 0)
            .and_then(|resolved| resolved.id)
            .is_some();
        let current_j = vm
            .resolve_var_from_bytes(b"arr(j)", 0)
            .and_then(|resolved| resolved.id)
            .is_some();
        let key_k = self.key_k.value().native_object_reference_count();
        let key_j = self.key_j.value().native_object_reference_count();
        self.rows.borrow_mut().push(format!("{}|window|{phase}|{key_k}|{key_j}|{dead_k}|{dead_j}|{defined_k}|{defined_j}|{refs_k}|{refs_j}|{}|{}", self.version, i32::from(current_k), i32::from(current_j)));
    }
}
impl NativeVariableObserver<Vm> for ElementCallbackWindows {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        vm: &mut Vm,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        let phase = if access.name2.is_empty() {
            "root".to_owned()
        } else {
            *self.sequence.borrow_mut() += 1;
            format!(
                "element{}-{}",
                *self.sequence.borrow(),
                std::str::from_utf8(access.name2).unwrap()
            )
        };
        self.record(vm, &phase);
        // Raw C84 flags do not donate a value or an existence result.
        if !access.name2.is_empty() {
            assert!(!vm.exists_var_bytes_from(if access.name2 == b"k" { b"ak" } else { b"aj" }, 0));
        }
        Ok(())
    }
}

#[test]
fn cpp_table_and_var_roles_match_all_25_original_callback_windows() {
    // Native proof: naming.variable.original-element-unset-callback-roles
    // docs/design/analysis/name-resolution-proofs/variable-original-element-unset-callback-roles.md
    use std::cell::RefCell;
    let rows = Rc::new(RefCell::new(Vec::new()));
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut vm = actual(environment);
        let root = Value::new_native_string_bytes(b"arr".as_slice());
        let key_k = Value::new_native_string_bytes(b"k".as_slice());
        let key_j = Value::new_native_string_bytes(b"j".as_slice());
        let object_table = vm
            .native_c_variable_name_protocol()
            .unwrap()
            .element_table_retains_original();
        let (_, _, _, selected) = vm
            .prepare_native_original_variable(&root, NativeVariableNameLookupPurpose::Write)
            .unwrap();
        let array = selected.id.unwrap();
        let mut ids = Vec::new();
        for key in [&key_k, &key_j] {
            let id = vm
                .ensure_target_var_at_binding(&selected.binding, Some(&key.string_bytes()))
                .unwrap();
            vm.var_arena.replace_state(
                id,
                crate::vars::VarState::Scalar(Value::new_native_string_bytes(b"ONE".as_slice())),
            );
            if object_table {
                assert!(vm.var_arena.retain_original_array_key(
                    array,
                    &tcl_core_types::NameBytes::from(key.string_bytes().as_ref()),
                    key.clone()
                ));
            }
            ids.push(id);
        }
        let name_k = Value::new_native_string_bytes(b"arr(k)".as_slice());
        let name_j = Value::new_native_string_bytes(b"arr(j)".as_slice());
        vm.link_upvar_original(
            0,
            &name_k,
            &Value::new_native_string_bytes(b"ak".as_slice()),
        )
        .unwrap();
        vm.link_upvar_original(
            0,
            &name_j,
            &Value::new_native_string_bytes(b"aj".as_slice()),
        )
        .unwrap();
        let windows = Rc::new(ElementCallbackWindows {
            version,
            key_k: key_k.native_lifetime_lease(),
            key_j: key_j.native_lifetime_lease(),
            old_k: ids[0],
            old_j: ids[1],
            object_table,
            rows: Rc::clone(&rows),
            sequence: Rc::new(RefCell::new(0)),
        });
        for name in [&root, &name_k, &name_j] {
            vm.add_native_variable_observer(
                name,
                &[NativeVariableTraceOperation::Unset],
                windows.clone(),
            )
            .unwrap();
        }
        windows.record(&vm, "before");
        assert!(vm.unset_var_bytes(b"arr"));
        windows.record(&vm, "after");
    }
    let expected = include_str!(
        "../../../../tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.txt"
    )
    .lines()
    .filter(|line| line.contains("|window|"))
    .collect::<Vec<_>>();
    assert_eq!(rows.borrow().len(), 25);
    assert_eq!(*rows.borrow(), expected);
}

#[test]
fn original_upvar_and_unset_retain_actual_target_cache_and_entry() {
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(environment);
        let target = Value::new_native_string_bytes(b"target".as_slice());
        let local = Value::new_native_string_bytes(b"alias".as_slice());
        vm.link_upvar_original(0, &target, &local).unwrap();
        assert!(
            target
                .with_native_parsed_variable(|cache| cache.array.is_none())
                .unwrap()
        );
        let binding = vm.var_binding_from_bytes(b"target", 0).unwrap();
        let selected = vm.raw_variable_binding(&binding).unwrap();
        assert!(vm.var_arena.has_link_refs(selected));
        vm.store_original_named_variable(&local, Value::int(7))
            .unwrap();
        vm.unset_original_named_variable(&target, true).unwrap();
        assert_eq!(vm.raw_variable_binding(&binding), Some(selected));
        assert!(
            target
                .with_native_parsed_variable(|cache| cache.array.is_none())
                .unwrap()
        );
        vm.store_original_named_variable(&local, Value::int(8))
            .unwrap();
        assert_eq!(
            vm.read_original_named_variable(&target)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"8"
        );
    }
}

#[test]
fn search_free_slots_match_all_20_actual_c_windows() {
    // Native proof: naming.variable.array-search-free-slot-retirement
    // docs/design/analysis/name-resolution-proofs/variable-array-search-free-slot-retirement.md
    use tcl_cmd_core::native_array_search::NativeArraySearchBackend;
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut vm = actual(environment);
        let protocol = vm.array_search_protocol().unwrap();
        let name = Value::new_native_string_bytes(b"a(k)".as_slice());
        vm.store_original_named_variable(&name, Value::new_native_string_bytes(b"v".as_slice()))
            .unwrap();
        let array = Value::new_native_string_bytes(b"a".as_slice());
        let handle = tcl_cmd_core::native_array_search::dispatch(
            &mut vm,
            "startsearch",
            std::slice::from_ref(&array),
            None,
        )
        .unwrap()
        .unwrap();
        let result = tcl_cmd_core::native_array_search::dispatch(
            &mut vm,
            "anymore",
            &[array, handle.clone()],
            None,
        )
        .unwrap()
        .unwrap();
        drop(result);
        let observe = |phase: &str, original: &Value| {
            format!(
                "{version}|{phase}|{}|{}|{}",
                if original
                    .native_array_search_cache_in(protocol)
                    .unwrap()
                    .is_some()
                {
                    "array search"
                } else if matches!(
                    original.native_object_snapshot().cache,
                    Cache::String { .. }
                ) {
                    "string"
                } else {
                    "none"
                },
                usize::from(original.native_primary_has_free_hook()),
                usize::from(original.resident_string_bytes().is_some())
            )
        };
        rows.push(observe("converted", &handle));
        let copy = handle.duplicate_native_object_in(vm.actual_native_string_protocol().unwrap());
        rows.push(observe("duplicate", &copy));
        assert!(vm.read_original_named_variable(&handle).is_err());
        rows.push(observe("missing", &handle));
        assert!(vm.read_original_named_variable(&copy).is_err());
        rows.push(observe("duplicate-missing", &copy));
    }
    let expected = include_str!(
        "../../../../tcl-syntax/tests/data/native_variable_name/array_search_free_slot.txt"
    )
    .lines()
    .filter(|line| !line.is_empty())
    .collect::<Vec<_>>();
    assert_eq!(rows.len(), 20);
    assert_eq!(rows, expected);
}

#[test]
fn original_parsed_headers_match_all_150_actual_c_windows() {
    // Native proof: naming.variable.original-scalar-parsed-header
    // docs/design/analysis/name-resolution-proofs/variable-original-scalar-parsed-header.md
    // Native proof: naming.variable.original-counted-array-parsed-header
    // docs/design/analysis/name-resolution-proofs/variable-original-counted-array-parsed-header.md
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut vm = actual(environment);
        for case in ["string", "int", "list", "bytearray", "array"] {
            let original = match case {
                "string" => Value::new_native_string_bytes(b"missing".as_slice()),
                "int" => Value::int(42),
                "list" => Value::list(vec![Value::new_native_string_bytes(b"missing".as_slice())]),
                "bytearray" => Value::byte_array(b"missing".to_vec()),
                "array" => Value::new_native_string_bytes(b"arr(k\0z)".as_slice()),
                _ => unreachable!(),
            };
            rows.push(observe(version, case, "before", &original));
            assert!(vm.read_original_named_variable(&original).is_err());
            rows.push(observe(version, case, "missing", &original));
            let value = Value::new_native_string_bytes(b"VALUE".as_slice());
            assert!(
                vm.store_original_named_variable(&original, value.clone())
                    .is_ok()
            );
            rows.push(observe(version, case, "stored", &original));
            assert!(
                vm.read_original_named_variable(&original)
                    .unwrap()
                    .is_same_object(&value)
            );
            rows.push(observe(version, case, "read", &original));
            let copy =
                original.duplicate_native_object_in(vm.actual_native_string_protocol().unwrap());
            rows.push(observe(version, case, "duplicate", &copy));
            drop(copy);
            for name in [b"missing".as_slice(), b"42".as_slice(), b"arr".as_slice()] {
                vm.unset_var_bytes(name);
            }
            assert!(vm.read_original_named_variable(&original).is_err());
            rows.push(observe(version, case, "after-unset-missing", &original));
        }
    }
    let expected =
        include_str!("../../../../tcl-syntax/tests/data/native_variable_name/parsed_headers.txt")
            .lines()
            .collect::<Vec<_>>();
    assert_eq!(rows.len(), 150);
    assert_eq!(rows, expected);
}

#[test]
fn scalar_alias_entries_and_array_parts_match_all_95_native_windows() {
    // Native proof: naming.variable.original-scalar-alias-entry-lifetime
    // docs/design/analysis/name-resolution-proofs/variable-original-scalar-alias-entry-lifetime.md
    // Native proof: naming.variable.original-parsed-array-child-references
    // docs/design/analysis/name-resolution-proofs/variable-original-parsed-array-child-references.md
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut vm = actual(environment);
        for mode in 0..2 {
            let name = Value::new_native_string_bytes(b"k".as_slice());
            let value = Value::new_native_string_bytes(b"ONE".as_slice());
            vm.store_original_named_variable(&name, value.clone())
                .unwrap();
            let binding = vm.var_binding_from_bytes(b"k", 0).unwrap();
            let before = *vm.var_table_mut(binding.owner).unwrap().get(b"k").unwrap();
            let observe = |vm: &mut Vm, phase: &str, fresh: Option<&Value>| {
                let table = vm.var_table_mut(binding.owner).unwrap();
                let found = table.get(b"k").copied();
                assert!(matches!(
                    name.native_object_snapshot().cache,
                    Cache::ParsedVariableName { .. }
                ));
                format!(
                    "{version}|alias|{mode}|{phase}|parsedVarName|{}|{}|{}|{}|{}",
                    name.native_object_reference_count(),
                    fresh.map_or(-1, |fresh| i64::try_from(
                        fresh.native_object_reference_count()
                    )
                    .unwrap()),
                    usize::from(found.is_some()),
                    usize::from(found == Some(before)),
                    usize::from(table.native_key_is(b"k", &name))
                )
            };
            rows.push(observe(&mut vm, "created", None));
            vm.push_call_frame(Some("run".into()), Vec::new());
            vm.link_upvar_bytes(0, b"k", b"a").unwrap();
            rows.push(observe(&mut vm, "linked", None));
            assert!(vm.unset_var_bytes(b"::k"));
            rows.push(observe(&mut vm, "unset-target", None));
            let fresh = Value::new_native_string_bytes(b"k".as_slice());
            let written = Value::new_native_string_bytes(if mode == 0 {
                b"::k".as_slice()
            } else {
                b"a".as_slice()
            });
            vm.store_original_named_variable(&written, value.clone())
                .unwrap();
            rows.push(observe(&mut vm, "recreated", Some(&fresh)));
            assert!(vm.unset_var_bytes(b"::k"));
            rows.push(observe(&mut vm, "unset-recreated-target", Some(&fresh)));
            assert!(!vm.unset_var_bytes(b"a"));
            rows.push(observe(&mut vm, "unset-alias", Some(&fresh)));
            vm.pop_call_frame();
            rows.push(observe(&mut vm, "frame-popped", Some(&fresh)));
            assert!(vm.read_original_named_variable(&fresh).is_err());
            rows.push(observe(&mut vm, "fresh-read", Some(&fresh)));
        }
        let name = Value::new_native_string_bytes(b"arr(k)".as_slice());
        vm.store_original_named_variable(
            &name,
            Value::new_native_string_bytes(b"VALUE".as_slice()),
        )
        .unwrap();
        let observe = |phase: &str| {
            name.with_native_parsed_variable(|cache| {
                let (root, element) = cache.array.as_ref().unwrap();
                let element_refs = match element {
                    NativeParsedVariableElement::Object(element) => {
                        i64::try_from(element.native_object_reference_count()).unwrap()
                    }
                    NativeParsedVariableElement::Bytes(_) => -1,
                };
                format!(
                    "{version}|parts|{phase}|{}|{element_refs}",
                    root.native_object_reference_count()
                )
            })
            .unwrap()
        };
        rows.push(observe("stored"));
        let copy = name.duplicate_native_object_in(vm.actual_native_string_protocol().unwrap());
        rows.push(observe("duplicated"));
        drop(copy);
        assert!(vm.unset_var_bytes(b"arr"));
        rows.push(observe("unset"));
    }
    let expected = include_str!(
        "../../../../tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt"
    )
    .lines()
    .collect::<Vec<_>>();
    assert_eq!(rows.len(), 95);
    assert_eq!(rows, expected);
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
                "../../../../../runtime/rust/tests/data/native_list_assignment_arity/8.4.20.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_list_assignment_arity/8.5.19.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_list_assignment_arity/8.6.18.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_list_assignment_arity/9.0.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_list_assignment_arity/9.1.0.txt"
            )
            .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_list_assignment_arity/source.tcl"
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
            "../../../../../runtime/rust/tests/data/native_list_assignment_arity/source.tcl"
        ),
        include_bytes!(
            "../../../../../runtime/rust/tests/data/native_list_assignment_arity/jim.txt"
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
                "../../../../../runtime/rust/tests/data/native_variable_read_input/8.4.20.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_variable_read_input/8.5.19.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_variable_read_input/8.6.18.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_variable_read_input/9.0.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_variable_read_input/9.1.0.txt"
            )
            .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_variable_read_input/source.tcl"
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
            "../../../../../runtime/rust/tests/data/native_variable_read_input/source.tcl"
        ),
        include_bytes!("../../../../../runtime/rust/tests/data/native_variable_read_input/jim.txt"),
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
                "../../../../../runtime/rust/tests/data/native_variable_array_write/8.4.20.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_variable_array_write/8.5.19.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_variable_array_write/8.6.18.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_variable_array_write/9.0.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_variable_array_write/9.1.0.txt"
            )
            .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!(
                "../../../../../runtime/rust/tests/data/native_variable_array_write/source.tcl"
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
            "../../../../../runtime/rust/tests/data/native_variable_array_write/source.tcl"
        ),
        include_bytes!(
            "../../../../../runtime/rust/tests/data/native_variable_array_write/jim.txt"
        ),
    );
}
