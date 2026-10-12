// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed selected-frame physical target controls; no external frame chronology.

use super::*;
use crate::command::NativeCommand;
use crate::interp::InterpId;
use crate::value::Value;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tcl_runtime_api::{Code, Completion, Frames, NativeExecutionError};
use tcl_syntax::raw_string::NativeValueAccessRefusal;

struct LeaveChild {
    child: InterpId,
    explicit: bool,
    hits: Rc<Cell<usize>>,
    first: Rc<RefCell<Option<NativeExecutionError>>>,
}
impl NativeCommand for LeaveChild {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.hits.set(self.hits.get() + 1);
        assert_eq!(vm.frames.len(), 1);
        if self.explicit {
            let _ = vm.refuse_host_command("selected package frame first Host".into());
            *self.first.borrow_mut() = vm.execution_refusal.clone();
        }
        crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(vm, self.child);
        super::super::ok(Value::string("LATER_GUEST"))
    }
}
struct Probe {
    child: InterpId,
    checked: Rc<Cell<usize>>,
}
impl NativeCommand for Probe {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        let parent = vm.cur_interp();
        assert!(vm.frames.len() > 1);
        let frames = vm.frames.iter().map(|frame| Rc::clone(&frame.activation)).collect::<Vec<_>>();
        let namespaces = vm.resolution_stacks.ns_id_stack.clone();
        let scripts = vm.ns_script_frames.clone();
        let recursion = vm.recursion_depth;
        let child_frames = vm.in_interp(self.child, |vm| {
            vm.frames.iter().map(|frame| Rc::clone(&frame.activation)).collect::<Vec<_>>()
        });
        let child_namespaces = vm.in_interp(self.child, |vm| vm.resolution_stacks.ns_id_stack.clone());
        let completion = vm.invoke_command("package", &[
            Value::string("require"), Value::string("original-package"), Value::string("1.0"),
        ]);
        assert_eq!(completion.code, Code::Error);
        assert!(vm.interp_is_current(self.child));
        assert_eq!(vm.frames.len(), child_frames.len());
        assert!(vm.frames.iter().zip(&child_frames).all(|(actual, expected)| Rc::ptr_eq(&actual.activation, expected)));
        assert_eq!(vm.resolution_stacks.ns_id_stack, child_namespaces);
        vm.in_interp(parent, |vm| {
            assert_eq!(vm.frames.len(), frames.len());
            assert!(vm.frames.iter().zip(&frames).all(|(actual, expected)| Rc::ptr_eq(&actual.activation, expected)));
            assert_eq!(vm.resolution_stacks.ns_id_stack, namespaces);
            assert_eq!(vm.ns_script_frames, scripts);
            assert_eq!(vm.recursion_depth, recursion);
            assert!(vm.package_loading_version_bytes(b"original-package").is_none());
            assert!(vm.take_package_file_scope().is_empty());
        });
        assert!(vm.interp_is_current(self.child));
        self.checked.set(self.checked.get() + 1);
        // Isolate this selected-frame boundary from the independently owned
        // ordinary-procedure abort/dispatch target restoration.
        crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(vm, parent);
        completion
    }
}
#[test]
fn original_package_global_selection_restores_hidden_parent_and_preserves_callback_child() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual procedure, NativeCommand, physical switch and saved frame/tail controls only.
    // The probe restores its caller before ordinary-proc abort; that separate owner is not closed by this control.
    for profile in ["tcl8.6", "tcl9.1"] {
        for loading in [false, true] {
            for explicit in [false, true] {
                let mut vm = crate::native_fixture::interpreter(crate::environment::profile_for_dialect(profile));
                let parent = vm.cur_interp();
                let global = Rc::clone(&vm.frames[0].activation);
                let child_name = vm.create_child(Some("selected_package_child".into()), false);
                let child = vm.child_id(&child_name).unwrap();
                vm.in_interp(child, |vm| { Frames::push(vm, tcl_runtime_api::ROOT_NS); });
                let hits = Rc::new(Cell::new(0));
                let checked = Rc::new(Cell::new(0));
                let first = Rc::new(RefCell::new(None));
                vm.try_register_native_command("leavePackageChild", Rc::new(LeaveChild { child, explicit, hits: Rc::clone(&hits), first: Rc::clone(&first) })).unwrap();
                vm.try_register_native_command("probePackageFrame", Rc::new(Probe { child, checked: Rc::clone(&checked) })).unwrap();
                let setup = if loading {
                    "package ifneeded original-package 1.0 leavePackageChild"
                } else {
                    "package unknown leavePackageChild"
                };
                assert_eq!(vm.eval_source(setup).unwrap().code, Code::Ok);
                let reached = vm.eval_source("proc originalPackageCaller {} {set local PARENT; probePackageFrame}; originalPackageCaller");
                assert!(reached.is_err());
                assert_eq!(hits.get(), 1);
                assert_eq!(checked.get(), 1);
                assert!(vm.interp_is_current(parent));
                assert_eq!(vm.frames.len(), 1);
                assert!(Rc::ptr_eq(&vm.frames[0].activation, &global));
                vm.in_interp(child, |vm| { assert_eq!(vm.frames.len(), 2); });
                let retained = vm.execution_refusal.clone().unwrap();
                if explicit {
                    assert_eq!(first.borrow().as_ref(), Some(&retained));
                } else {
                    assert_eq!(retained, NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable("entered VM original execution context")));
                }
            }
        }
    }
}
