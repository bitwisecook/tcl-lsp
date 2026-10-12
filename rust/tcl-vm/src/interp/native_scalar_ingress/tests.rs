// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual ingress and descriptive transport facts are separate software roles.
use super::*;
use tcl_platform::{
    DoubleNumericConversion, NumericEnvironmentUnavailable, NumericErrorState,
    UnsignedNumericConversion,
};
use tcl_syntax::{
    number::Number,
    scalar_getter::{NativeScalarGetterFailure as Failure, NativeScalarGetterKind as Kind},
};
struct Numeric {
    abi: Cell<NativeCIntegerAbi>,
    queries: Cell<usize>,
}
impl NumericEnvironment for Numeric {
    fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
        self.queries.set(self.queries.get() + 1);
        Ok(self.abi.get())
    }
    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }
    fn unsigned(
        &self,
        _: &[u8],
        _: usize,
        _: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }
    fn double(
        &self,
        _: &[u8],
        _: bool,
    ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }
}
struct Host {
    actual: Rc<dyn tcl_platform::Host>,
    numeric: Rc<Numeric>,
}
impl tcl_platform::Host for Host {
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
    fn numeric_environment(&self) -> Option<&dyn NumericEnvironment> {
        Some(&*self.numeric)
    }
}
fn supplied() -> (Vm, Rc<Numeric>) {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
    let numeric = Rc::new(Numeric {
        abi: Cell::new(NativeCIntegerAbi {
            char_bits: 8,
            int_bytes: 4,
            long_bytes: 8,
        }),
        queries: Cell::new(0),
    });
    vm.set_host(Rc::new(Host {
        actual: vm.host_rc(),
        numeric: Rc::clone(&numeric),
    }));
    (vm, numeric)
}
#[test]
fn original_c84_wide_ingress_preserves_descriptor_without_changing_semantic_getter_equations() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The supplied ABI is a software provider; it supplies no recorded Native observation.
    let (mut vm, numeric) = supplied();
    let original = vm.capture_original_scalar_ingress().unwrap();
    let wide = vm
        .import_original_native_scalar(&original, &Carrier::Tcl84Wide(4_294_967_296), None)
        .unwrap();
    assert_eq!(numeric.queries.get(), 1);
    assert_eq!(wide.native_object_type_name(), "wideInt");
    assert!(!wide.native_primary_has_free_hook());
    assert_eq!(
        wide.native_scalar_cache(),
        Some(NativeScalarCache::Number(Number::Int(4_294_967_296)))
    );
    assert_eq!(
        wide.native_scalar_descriptor_origin(),
        Some(tcl_dialect::TclVersion::V8_4)
    );
    assert!(wide.resident_string_bytes().is_none());
    let dialect = vm.native_scalar_carrier_dialect();
    let probe = wide
        .native_scalar_probe_with_environment(dialect, Kind::Int, Some(&*numeric))
        .unwrap();
    assert_eq!(probe, Err(Failure::IntWidthOverflow));
    assert_eq!(wide.native_object_type_name(), "wideInt");
    assert!(wide.resident_string_bytes().is_none());
    let string = dialect.native_string_protocol().unwrap();
    let copy = wide.duplicate_native_object_in(string);
    assert_eq!(copy.native_object_type_name(), "wideInt");
    assert!(copy.resident_string_bytes().is_none());
    assert_eq!(
        wide.native_string_bytes(string).unwrap().as_ref(),
        b"4294967296"
    );
    assert_eq!(wide.native_object_type_name(), "wideInt");
    assert!(copy.resident_string_bytes().is_none());
    let long = vm
        .import_original_native_scalar(&original, &Carrier::Tcl84Long(17), None)
        .unwrap();
    assert_eq!(long.native_object_type_name(), "int");
    assert_eq!(
        long.native_scalar_cache(),
        Some(NativeScalarCache::Tcl84Long(17))
    );
    let neutral = Value::int(4_294_967_296);
    assert_eq!(neutral.native_object_type_name(), "int");
    assert_eq!(neutral.native_scalar_descriptor_origin(), None);
}
#[test]
fn original_c84_wide_ingress_requires_actual_receipt_and_unchanged_queried_abi() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, numeric) = supplied();
    let dialect = vm.native_scalar_carrier_dialect();
    let (cache, origin) = carrier::import_scalar(&Carrier::Tcl84Wide(17));
    assert!(Value::from_native_scalar_cache(cache, origin, dialect).is_err());
    assert_eq!(numeric.queries.get(), 0);
    let original = vm.capture_original_scalar_ingress().unwrap();
    let first = vm
        .import_original_native_scalar(&original, &Carrier::Tcl84Wide(17), None)
        .unwrap();
    let before = first.native_object_snapshot();
    numeric.abi.set(NativeCIntegerAbi {
        char_bits: 8,
        int_bytes: 4,
        long_bytes: 4,
    });
    let refused = vm
        .import_original_native_scalar(&original, &Carrier::Tcl84Wide(17), None)
        .err()
        .unwrap();
    assert_eq!(numeric.queries.get(), 2);
    assert_eq!(vm.execution_refusal.as_ref(), Some(&refused));
    assert_eq!(first.native_object_snapshot(), before);
    assert!(first.resident_string_bytes().is_none());
}
#[test]
fn original_c84_wide_ingress_stops_before_abi_after_actual_host_context_or_descriptor_withdrawal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [0, 1, 2, 3] {
        let (mut vm, numeric) = supplied();
        let original = vm.capture_original_scalar_ingress().unwrap();
        let facts = if mode == 0 {
            Carrier::Integer(17)
        } else {
            Carrier::Tcl84Wide(17)
        };
        match mode {
            1 => {
                vm.refuse_host_command("original scalar Host refusal".into());
            }
            2 => {
                let old = vm.host_rc();
                vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
                vm.set_host(old);
            }
            3 => {
                vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl8.6"));
                vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl8.4"));
            }
            _ => {}
        }
        let before = vm.execution_refusal.clone();
        let refused = vm
            .import_original_native_scalar(&original, &facts, None)
            .err()
            .unwrap();
        assert_eq!(numeric.queries.get(), 0);
        assert_eq!(vm.execution_refusal.as_ref(), Some(&refused));
        if let Some(first) = before {
            assert_eq!(refused, first);
        }
    }
}
