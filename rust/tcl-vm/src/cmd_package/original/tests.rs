// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed software owner controls; no external package callback chronology claim.

use super::*;
use crate::command::NativeCommand;
use crate::interp::ok;
use std::cell::{Cell, RefCell};
use tcl_platform::{NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable, NativeIntegerKind};
use tcl_runtime_api::{Code, NativeExecutionError};
use tcl_syntax::native_string::NativeStringUnavailable;
use tcl_syntax::raw_string::{NativeStringAccessError, NativeValueAccessRefusal};
use tcl_syntax::scalar_getter::NativeScalarCache;

fn native(profile: &str) -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect(profile))
}
fn call(vm: &mut Vm, words: &[&str]) -> Completion<Value> {
    let operands = words.iter().map(|word| Value::string(*word)).collect::<Vec<_>>();
    super::super::cmd_package(vm, &operands)
}
struct Formatter {
    failed: bool,
    calls: Cell<usize>,
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        NativeIntegerFormatterBuild { version: [8, 4, 20, 0], sha256: [7; 32], long_bits: 64 }
    }
    fn format(&self, kind: NativeIntegerKind, value: i64) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        assert_eq!(kind, NativeIntegerKind::Long);
        assert_eq!(value, i64::MIN);
        self.calls.set(self.calls.get() + 1);
        if self.failed { Err(NativeIntegerFormatterUnavailable::Operation) } else { Ok(value.to_string().into_bytes()) }
    }
}
struct FormatterHost {
    actual: Rc<dyn tcl_platform::Host>,
    formatter: Rc<Formatter>,
    acquisitions: Rc<Cell<usize>>,
}
impl tcl_platform::Host for FormatterHost {
    fn capabilities(&self) -> tcl_platform::Capabilities { self.actual.capabilities() }
    fn clock(&self) -> &dyn tcl_platform::Clock { self.actual.clock() }
    fn stdio(&self) -> &dyn tcl_platform::StdIo { self.actual.stdio() }
    fn env(&self) -> &dyn tcl_platform::Env { self.actual.env() }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        self.acquisitions.set(self.acquisitions.get() + 1);
        Some(&*self.formatter)
    }
}
#[test]
fn original_package_version_string_uses_actual_formatter_and_full_first_failure() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Supplied software formatter forwarding and first cause only; no C84 minimum-integer formatter equivalence.
    for failed in [false, true] {
        let mut vm = native("tcl8.4");
        let formatter = Rc::new(Formatter { failed, calls: Cell::new(0) });
        let acquisitions = Rc::new(Cell::new(0));
        vm.set_host(Rc::new(FormatterHost { actual: vm.host_rc(), formatter: Rc::clone(&formatter), acquisitions: Rc::clone(&acquisitions) }));
        let first = Value::from_native_scalar_cache(NativeScalarCache::Tcl84Long(i64::MIN), None, vm.actual_native_invocation_dialect()).unwrap();
        let later = Value::from_native_scalar_cache(NativeScalarCache::Tcl84Long(2), None, vm.actual_native_invocation_dialect()).unwrap();
        let before = later.native_object_snapshot();
        let references = [first.native_object_reference_count(), later.native_object_reference_count()];
        let operands = [Value::string("vcompare"), first, later];
        let completion = super::super::cmd_package(&mut vm, &operands);
        assert_eq!(completion.code, Code::Error);
        assert_eq!(formatter.calls.get(), 1);
        assert_eq!(acquisitions.get(), 1);
        assert_eq!(operands[2].native_object_snapshot(), before);
        assert_eq!([operands[1].native_object_reference_count(), operands[2].native_object_reference_count()], references);
        if failed {
            assert_eq!(vm.execution_refusal, Some(NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::StringAccess(NativeStringAccessError::Unavailable(NativeStringUnavailable::StringUpdater)))));
            assert!(operands[1].resident_string_bytes().is_none());
        } else {
            assert!(vm.execution_refusal.is_none());
            assert_eq!(completion.result.resident_string_bytes().unwrap().as_ref(), b"expected version number but got \"-9223372036854775808\"");
        }
    }
}
struct Withdraw {
    loading: bool,
    explicit: bool,
    hits: Rc<Cell<usize>>,
    first: Rc<RefCell<Option<NativeExecutionError>>>,
}
impl NativeCommand for Withdraw {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.hits.set(self.hits.get() + 1);
        assert_eq!(vm.package_loading_version_bytes(b"original-package").is_some(), self.loading);
        vm.provide_package(b"retained-effect", b"2.0");
        if self.explicit {
            let _ = vm.refuse_host_command("original package callback first Host".into());
            *self.first.borrow_mut() = vm.execution_refusal.clone();
        }
        let actual = vm.host_rc();
        vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
        vm.set_host(actual);
        ok(Value::string("LATER_GUEST"))
    }
}
#[test]
fn original_package_unknown_and_loader_restore_before_original_currency_settlement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual VM callbacks, saved table/file/global-frame cleanup and first cause only.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for loading in [false, true] {
            for explicit in [false, true] {
                let mut vm = native(profile);
                let hits = Rc::new(Cell::new(0));
                let captured = Rc::new(RefCell::new(None));
                vm.try_register_native_command("withdrawPackage", Rc::new(Withdraw { loading, explicit, hits: Rc::clone(&hits), first: Rc::clone(&captured) })).unwrap();
                let setup = if loading {
                    call(&mut vm, &["ifneeded", "original-package", "1.0", "withdrawPackage"])
                } else {
                    call(&mut vm, &["unknown", "withdrawPackage"])
                };
                assert_eq!(setup.code, Code::Ok);
                let selected = vm.cur_interp();
                let script = vm.current_script().to_owned();
                let completion = call(&mut vm, &["require", "original-package", "1.0"]);
                assert_eq!(completion.code, Code::Error);
                assert_eq!(hits.get(), 1);
                assert!(vm.interp_is_current(selected));
                assert!(!vm.package_table_is_authored());
                assert_eq!(vm.current_script(), script);
                assert!(vm.package_loading_version_bytes(b"original-package").is_none());
                assert!(vm.take_package_file_scope().is_empty());
                assert_eq!(vm.package_version_bytes(b"retained-effect").unwrap().as_bytes(), b"2.0");
                let first = vm.execution_refusal.clone().unwrap();
                if explicit {
                    assert_eq!(captured.borrow().as_ref(), Some(&first));
                } else {
                    assert_eq!(first, NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable("entered VM original execution context")));
                }
                let retry = call(&mut vm, &["provide", "LATER", "1.0"]);
                assert_eq!(retry.code, Code::Error);
                assert_eq!(vm.execution_refusal, Some(first));
                assert!(vm.package_version_bytes(b"LATER").is_none());
            }
        }
    }
}
#[test]
fn original_package_pending_host_precedes_operand_materialisation_and_table_effects() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Existing first software Host channel only; no additional native argument chronology.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = native(profile);
        let original = Value::byte_array(b"provide".as_slice());
        let before = original.native_object_snapshot();
        let operands = [original, Value::string("LATER"), Value::string("1.0")];
        let _ = vm.refuse_host_command("package pending first Host".into());
        let first = vm.execution_refusal.clone().unwrap();
        let completion = super::super::cmd_package(&mut vm, &operands);
        assert_eq!(completion.code, Code::Error);
        assert_eq!(vm.execution_refusal, Some(first));
        assert_eq!(operands[0].native_object_snapshot(), before);
        assert!(vm.package_version_bytes(b"LATER").is_none());
    }
}

#[test]
fn original_embedding_package_provide_uses_actual_database_protocol() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Selected software package protocol only; the suffix input is a fixed source control, not an additional native observation.
    for (actual, logical, expected) in [
        ("tcl8.6", tcl_dialect::TclVersion::V9_1, Code::Error),
        ("tcl9.0", tcl_dialect::TclVersion::V8_4, Code::Ok),
    ] {
        let mut vm = native(actual);
        vm.set_runtime_version(logical);
        let operands = [Value::string("original-embedding"), Value::from_native_string_bytes(b"1+\xff".as_slice())];
        let completion = super::super::pkg_provide(&mut vm, &operands, logical);
        assert_eq!(completion.code, expected);
        assert_eq!(vm.package_version_bytes(b"original-embedding").is_some(), expected == Code::Ok);
        assert!(vm.execution_refusal.is_none());
    }
}
