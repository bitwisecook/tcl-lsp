// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual procedure-header adapter and software formatter entry controls.
use super::*;
use std::{cell::Cell, rc::Rc};
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache};
struct FormatterHost {
    actual: Rc<dyn tcl_platform::Host>,
    acquisitions: Cell<usize>,
    formats: Cell<usize>,
}
impl NativeIntegerFormatter for FormatterHost {
    fn build(&self) -> NativeIntegerFormatterBuild {
        NativeIntegerFormatterBuild {
            version: [8, 4, 20, 0],
            sha256: [8; 32],
            long_bits: 64,
        }
    }
    fn format(
        &self,
        _kind: NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        self.formats.set(self.formats.get() + 1);
        Ok(value.to_string().into_bytes())
    }
}
impl tcl_platform::Host for FormatterHost {
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
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        self.acquisitions.set(self.acquisitions.get() + 1);
        Some(self)
    }
}
fn provided() -> (Vm, Rc<FormatterHost>) {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
    let host = Rc::new(FormatterHost {
        actual: vm.host_rc(),
        acquisitions: Cell::new(0),
        formats: Cell::new(0),
    });
    vm.set_host(host.clone());
    (vm, host)
}
fn integer(vm: &Vm, value: i64) -> Value {
    Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(value)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap()
}
#[test]
fn original_procedure_header_string_adapter_uses_the_selected_formatter_and_stops_at_absent_header()
{
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // This software Host checks the real adapter schedule, not a C Tcl procedure compiler.
    let (mut vm, host) = provided();
    let parameters = integer(&vm, 6);
    let body = integer(&vm, 9);
    let count = parameters.native_object_reference_count();
    assert_eq!(
        procedure_header_compilation(&mut vm, &parameters, &body).unwrap(),
        tcl_dialect::NativeProcedureHeaderCompilation::Absent
    );
    assert_eq!(parameters.resident_string_bytes().unwrap().as_ref(), b"6");
    assert!(body.resident_string_bytes().is_none());
    assert_eq!(parameters.native_object_reference_count(), count);
    assert_eq!(host.formats.get(), 1);
    assert_eq!(host.acquisitions.get(), 1);
    assert!(vm.execution_refusal.is_none());
}
#[test]
fn original_procedure_header_string_adapter_preserves_the_entered_refusal_before_formatting() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, host) = provided();
    let parameters = integer(&vm, 6);
    let body = integer(&vm, 9);
    let before = parameters.native_object_snapshot();
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
    let original = vm.host_rc();
    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
    vm.set_host(original);
    let carrier = procedure_header_compilation(&mut vm, &parameters, &body).unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(carrier.code, Code::Error);
    assert_eq!(host.formats.get(), 0);
    assert_eq!(host.acquisitions.get(), 0);
    assert_eq!(parameters.native_object_snapshot(), before);
    assert!(body.resident_string_bytes().is_none());
    assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
}
