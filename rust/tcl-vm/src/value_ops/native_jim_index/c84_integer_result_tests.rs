// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Index result construction borrows the already entered original operation.
use super::*;
use std::cell::Cell;
use tcl_syntax::{scalar_getter::NativeScalarCache, value::ValueOps};
struct CountingHost {
    actual: Rc<dyn tcl_platform::Host>,
    queries: Rc<Cell<usize>>,
}
impl tcl_platform::Host for CountingHost {
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
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.queries.set(self.queries.get() + 1);
        self.actual.numeric_environment()
    }
}
fn actual() -> Vm {
    crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"))
}
#[test]
fn original_c84_search_long_results_preserve_wide_operands_and_true_list_member_roles() {
    // Native descriptor facts and software receiver counts are independent observations.
    // A genuine Wide42 operand fits C84 search Int; the recorded Wide4294967296
    // IntWidthOverflow remains its independent unchanged primitive outcome.
    let mut vm = actual();
    let ingress = vm.capture_original_scalar_ingress().unwrap();
    let operand = vm
        .import_original_native_scalar(
            &ingress,
            &tcl_core_types::NativeScalarCache::Tcl84Wide(42),
            None,
        )
        .unwrap();
    vm.finish_original_scalar_ingress(ingress).unwrap();
    let list = ValueOps::new_list(&mut vm, vec![operand.clone()]);
    let exact = Value::new_native_string_bytes(b"-exact".as_slice());
    let integer = Value::new_native_string_bytes(b"-integer".as_slice());
    let args = [&exact, &integer, &list, &operand].map(|v| v.native_lifetime_lease().into_value());
    let searched = tcl_cmd_core::lsearch::lsearch::<Vm, crate::cmd_regexp::CrateEngine>(
        &mut vm,
        &args,
        tcl_dialect::TclVersion::V8_4,
    )
    .unwrap_or_else(|error| panic!("C84 search failed: {:?}", error.message));
    assert_eq!(searched.native_object_type_name(), "int");
    assert_eq!(
        searched.native_scalar_cache(),
        Some(NativeScalarCache::Tcl84Long(0))
    );
    assert!(searched.resident_string_bytes().is_none());
    assert_eq!(searched.native_object_reference_count(), 1);
    assert_eq!(operand.native_object_type_name(), "wideInt");
    assert!(operand.resident_string_bytes().is_none());
    assert_eq!(operand.native_object_reference_count(), 2);
    let all = Value::new_native_string_bytes(b"-all".as_slice());
    let args =
        [&all, &exact, &integer, &list, &operand].map(|v| v.native_lifetime_lease().into_value());
    let searched = tcl_cmd_core::lsearch::lsearch::<Vm, crate::cmd_regexp::CrateEngine>(
        &mut vm,
        &args,
        tcl_dialect::TclVersion::V8_4,
    )
    .unwrap_or_else(|error| panic!("C84 search failed: {:?}", error.message));
    let members = searched
        .native_object_list_elements(
            vm.actual_native_invocation_dialect()
                .native_string_protocol()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(
        members[0].native_scalar_cache(),
        Some(NativeScalarCache::Tcl84Long(0))
    );
    assert!(members[0].resident_string_bytes().is_none());
    assert_eq!(members[0].native_object_reference_count(), 1);
}
#[test]
fn original_c84_index_result_keeps_first_cause_without_querying_a_refreshed_target() {
    for mode in [0, 1, 2, 3] {
        let mut vm = actual();
        let queries = Rc::new(Cell::new(0));
        vm.set_host(Rc::new(CountingHost {
            actual: vm.host_rc(),
            queries: Rc::clone(&queries),
        }));
        let operation = vm.enter_index_operation().unwrap();
        let mut other = actual();
        if mode == 1 {
            let _ = vm.refuse_host_command("original Index result first Host".to_owned());
        } else if mode == 2 {
            let before = vm.runtime_context().clone();
            let mut changed = before.clone();
            changed
                .packages
                .push(("original-index-result".to_owned(), "1".to_owned()));
            vm.pin_context(&changed).unwrap();
            vm.pin_context(&before).unwrap();
        }
        let reached = if mode == 3 {
            tcl_cmd_core::index::integer_result(&mut other, &operation, 0)
        } else {
            tcl_cmd_core::index::integer_result(
                &mut vm,
                &operation,
                if mode == 0 {
                    i64::from(i32::MAX) + 1
                } else {
                    0
                },
            )
        };
        assert!(reached.is_err());
        assert_eq!(queries.get(), 0);
        if mode != 3 {
            assert_eq!(
                reached.unwrap_err().native_execution_refusal(),
                vm.execution_refusal.as_ref()
            );
        }
    }
}
