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
use std::cell::{Cell, RefCell};
use std::rc::Weak;

struct Invocation {
    events: Rc<RefCell<Vec<&'static str>>>,
    record: Rc<RefCell<Weak<ObjCommand>>>,
    drops: Rc<Cell<usize>>,
    recreate: bool,
}
impl Drop for Invocation {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}
struct Replacement;
impl HostCommand for Replacement {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Ok(Value::string("REPLACEMENT").into())
    }
}
impl HostCommand for Invocation {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        unreachable!("actual HostEntry uses the registrar")
    }
    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        _: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        self.events.borrow_mut().push("entered");
        assert!(registrar.remove_command("activeHost")?);
        let record = self
            .record
            .borrow()
            .upgrade()
            .expect("actual dispatch Rust record remains live");
        assert!(
            record.original_delete_role_is_withdrawn(),
            "registration box's once-only delete callback already ran"
        );
        assert_eq!(
            self.drops.get(),
            0,
            "genuine invocation Rust role survives registration deletion"
        );
        self.events.borrow_mut().push("registration-deleted");
        if self.recreate {
            registrar.define_command("activeHost", Rc::new(Replacement))?;
        }
        self.events.borrow_mut().push("returning");
        Ok(Value::string("ACTIVE").into())
    }
}
#[test]
fn original_host_entry_self_deletion_releases_box_before_return_without_borrowing_it() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual Runtime HostEntry/registrar bridge; no Native client-data free parity claim.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for recreate in [false, true] {
            let mut engine = RuntimeEngine::new();
            engine.set_release(profile).unwrap();
            let events = Rc::new(RefCell::new(Vec::new()));
            let record = Rc::new(RefCell::new(Weak::new()));
            let drops = Rc::new(Cell::new(0));
            engine
                .define_command(
                    "activeHost",
                    Rc::new(Invocation {
                        events: Rc::clone(&events),
                        record: Rc::clone(&record),
                        drops: Rc::clone(&drops),
                        recreate,
                    }),
                )
                .unwrap();
            let generation = engine.interp.resolve_cmd_token(b"activeHost").unwrap();
            match engine
                .interp
                .namespaces()
                .command_in(crate::namespace::GLOBAL, b"activeHost")
                .unwrap()
            {
                Command::ObjCmd(actual) => *record.borrow_mut() = Rc::downgrade(&actual),
                _ => unreachable!(),
            }
            assert_eq!(
                engine
                    .eval_in_invocation("activeHost argument")
                    .unwrap()
                    .value
                    .as_str(),
                Some("ACTIVE")
            );
            assert_eq!(
                events.borrow().as_slice(),
                &["entered", "registration-deleted", "returning"]
            );
            assert_eq!(drops.get(), 1);
            assert!(record.borrow().upgrade().is_none());
            assert!(!engine.host_commands.borrow().contains_key(&generation));
            if recreate {
                assert_eq!(
                    engine
                        .eval_in_invocation("activeHost")
                        .unwrap()
                        .value
                        .as_str(),
                    Some("REPLACEMENT")
                );
            } else {
                assert!(engine.interp.resolve_cmd_token(b"activeHost").is_none());
            }
        }
    }
}
struct CountHost(Rc<Cell<usize>>);
impl HostCommand for CountHost {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        self.0.set(self.0.get() + 1);
        Ok(Value::string("HOST").into())
    }
}
struct ImportTrace {
    record: Weak<ObjCommand>,
    generation: u64,
    callbacks: Cell<usize>,
    host_calls: Rc<Cell<usize>>,
}
unsafe extern "C" fn invoke_import_trace(
    data: *mut c_void,
    actual: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    let trace = unsafe { &*data.cast::<ImportTrace>() };
    let actual = unsafe { &mut *actual };
    trace.callbacks.set(trace.callbacks.get() + 1);
    assert_eq!(
        actual.resolve_cmd_token(b"retiringHost"),
        Some(trace.generation)
    );
    let record = trace
        .record
        .upgrade()
        .expect("actual deletion driver record");
    assert!(record.original_host_backing_is_retired());
    assert_eq!(actual.eval_str(b"retiringHost"), Code::Error);
    assert_eq!(trace.host_calls.get(), 0);
    0
}
#[test]
fn original_import_trace_refuses_fresh_host_entry_after_actual_backing_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual private Runtime Box role, not Native client-data-free/token-null parity.
    for profile in ["tcl8.4", "tcl8.5"] {
        let mut engine = RuntimeEngine::new();
        engine.set_release(profile).unwrap();
        let calls = Rc::new(Cell::new(0));
        engine
            .define_command("retiringHost", Rc::new(CountHost(Rc::clone(&calls))))
            .unwrap();
        let generation = engine.interp.resolve_cmd_token(b"retiringHost").unwrap();
        let Command::ObjCmd(record) = engine
            .interp
            .namespaces()
            .command_in(crate::namespace::GLOBAL, b"retiringHost")
            .unwrap()
        else {
            unreachable!()
        };
        let mut trace = Box::new(ImportTrace {
            record: Rc::downgrade(&record),
            generation,
            callbacks: Cell::new(0),
            host_calls: Rc::clone(&calls),
        });
        drop(record);
        engine
            .interp
            .create_obj_command(
                b"importObserver",
                ObjCommand::new(
                    invoke_import_trace,
                    std::ptr::from_mut(trace.as_mut()).cast(),
                    None,
                ),
            )
            .unwrap();
        assert_eq!(engine.interp.eval_str(b"namespace export retiringHost; namespace eval imported {namespace import ::retiringHost}; trace add command ::imported::retiringHost delete importObserver"),Code::Ok);
        assert!(!engine.interp.delete_command_generation(generation));
        assert_eq!(trace.callbacks.get(), 1);
        assert_eq!(calls.get(), 0);
        assert!(trace.record.upgrade().is_none());
        assert!(engine.interp.resolve_cmd_token(b"retiringHost").is_none());
        assert!(engine
            .interp
            .resolve_cmd_token(b"::imported::retiringHost")
            .is_none());
        assert_eq!(
            engine.interp.native_execution_refusal(),
            Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original Runtime host backing retired"
                )
            ))
        );
        drop(engine);
        drop(trace);
    }
}

#[test]
fn original_typed_host_failure_crosses_the_actual_runtime_engine_boundary() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    struct Typed(tcl_runtime_api::NativeExecutionError);
    impl HostCommand for Typed {
        fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
            Err(EngineError::execution_failure(self.0.clone()))
        }
    }
    let mut engine = RuntimeEngine::new();
    engine.set_release("tcl8.6").unwrap();
    let cause = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "actual Runtime Engine typed callback cause",
        ),
    );
    engine
        .define_command("originalTypedFailure", Rc::new(Typed(cause.clone())))
        .unwrap();
    let error = engine
        .eval_in_invocation("set ::prior BEFORE; catch {originalTypedFailure}; set ::after LATER")
        .unwrap_err();
    assert_eq!(error, EngineError::execution_failure(cause.clone()));
    assert_eq!(engine.interp.native_execution_refusal(), Some(cause));
    assert!(crate::test_storage::global_scalar(&engine.interp, b"prior").is_some());
    assert!(crate::test_storage::global_scalar(&engine.interp, b"after").is_none());
    assert_eq!(
        engine
            .eval_in_invocation("return NEXT")
            .unwrap()
            .value
            .as_str(),
        Some("NEXT")
    );
}
