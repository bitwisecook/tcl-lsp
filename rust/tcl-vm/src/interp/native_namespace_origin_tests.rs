// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original public origin and helper-root reports from retained references.

use std::{cell::RefCell, io::Write, rc::Rc};

struct Output(Rc<RefCell<Vec<u8>>>);
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn compare(engine: &str, oracle: &[u8], input: &str, expected_rows: usize) {
    let profile = crate::environment::profile_for_dialect(engine);
    let output = Rc::new(RefCell::new(Vec::new()));
    let mut vm = crate::native_fixture::interpreter_with_output(
        profile,
        Box::new(Output(Rc::clone(&output))),
    );
    let completion = vm.try_eval_source(input).unwrap_or_else(|error| {
        panic!(
            "{engine}: {error:?}; output before refusal:\n{}",
            String::from_utf8_lossy(&output.borrow())
        )
    });
    assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
    assert!(vm.refused_completion().is_none(), "{engine}");
    let actual = output.borrow();
    let (actual_identity, actual_rows) =
        actual.split_at(actual.iter().position(|byte| *byte == b'\n').unwrap() + 1);
    let (oracle_identity, oracle_rows) =
        oracle.split_at(oracle.iter().position(|byte| *byte == b'\n').unwrap() + 1);
    if engine == "jim" {
        assert_eq!(actual_identity, b"IDENTITY 0.84\n");
        assert_eq!(oracle_identity, b"IDENTITY 0.84-9-g5bac7c9\n");
    } else {
        assert_eq!(
            actual_identity, oracle_identity,
            "{engine}: provider reporting"
        );
    }
    assert_eq!(
        std::str::from_utf8(oracle_rows).unwrap().lines().count(),
        expected_rows
    );
    assert_eq!(
        actual_rows, oracle_rows,
        "{engine}: exact original origin/helper-root output, including alias prefix objects and replacement generations"
    );
    eprintln!(
        "{engine}:{expected_rows} original observation rows; reports never recover command identity"
    );
}

macro_rules! origins {
    ($test:ident, $engine:literal, $provider:literal) => {
        #[test]
        fn $test() {
            compare($engine, include_bytes!(concat!("../../../../experiments/name-handling/namespace-origin-query/", $provider, "/stdout.bin")),
                include_str!("../../../../experiments/name-handling/namespace-origin-query/input.tcl"),37);
            compare($engine, include_bytes!(concat!("../../../../experiments/name-handling/namespace-origin-query/helper-root/", $provider, "/stdout.bin")),
                include_str!("../../../../experiments/name-handling/namespace-origin-query/helper-root-input.tcl"),5);
        }
    };
}
origins!(original_namespace_origin_c84, "tcl8.4", "8.4.20");
origins!(original_namespace_origin_c85, "tcl8.5", "8.5.19");
origins!(original_namespace_origin_c86, "tcl8.6", "8.6.18");
origins!(original_namespace_origin_c90, "tcl9.0", "9.0.4");
origins!(original_namespace_origin_c91, "tcl9.1", "9.1.0");
origins!(original_namespace_origin_jim, "jim", "jim");

#[test]
fn unbounded_original_jim_alias_cycle_refuses_without_a_guest_error_claim() {
    let profile = crate::environment::profile_for_dialect("jim");
    let mut vm = crate::native_fixture::interpreter(profile);
    let setup = vm
        .try_eval_source("alias ::__no_cycle_a ::__no_cycle_b; alias ::__no_cycle_b ::__no_cycle_a")
        .unwrap();
    assert!(setup.code.is_ok());
    let query = vm
        .try_eval_source("namespace origin ::__no_cycle_a")
        .unwrap_err();
    assert_eq!(
        query,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "unbounded Jim namespace origin alias cycle",
            ),
        )
    );
}

#[test]
fn origin_retired_world_refuses_instead_of_reporting_a_missing_command() {
    let profile = crate::environment::profile_for_dialect("jim");
    let mut vm = crate::native_fixture::interpreter(profile);
    let original = super::Value::string("puts");
    vm.name_world.borrow_mut().retire();
    assert!(vm.native_namespace_command_name(&original, true).is_err());
}

#[test]
fn origin_canonical_root_relative_retains_the_actual_original_header() {
    use tcl_registry::native_jim_lookup::NativeJimNamespaceOriginBackend;
    let profile = crate::environment::profile_for_dialect("jim");
    let mut vm = crate::native_fixture::interpreter(profile);
    let original = super::Value::from_native_string_bytes(b"puts".as_slice());
    let entered =
        super::native_original_lookup::OriginalLookup::enter(&mut vm, super::ROOT_NS, &original)
            .unwrap();
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_jim_lookup_protocol()
        .unwrap();
    let mut backend = super::native_command_names::JimNamespaceOriginBackend {
        vm: &mut vm,
        entered: &entered,
        protocol,
    };
    let canonical = backend.canonical(&original).unwrap();
    assert!(canonical.is_same_object(&original));
    let absolute = super::Value::from_native_string_bytes(b"::::puts".as_slice());
    let canonical = backend.canonical(&absolute).unwrap();
    assert!(!canonical.is_same_object(&absolute));
    assert_eq!(canonical.resident_string_bytes().unwrap().as_ref(), b"puts");
}
