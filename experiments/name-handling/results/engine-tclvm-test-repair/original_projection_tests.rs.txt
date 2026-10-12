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
use std::cell::Cell;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

struct Counted(Rc<Cell<usize>>);
impl HostCommand for Counted {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        self.0.set(self.0.get() + 1);
        Ok(Value::Empty.into())
    }
}

#[test]
fn original_engine_argument_worker_refuses_real_retired_first_header_before_later_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual production adapter worker under its genuine engine ingress; no
    // external Native execution/refcount chronology is claimed.
    let mut engine = TclVmEngine::new();
    let called = Rc::new(Cell::new(0));
    let callback = HostCommandShim {
        command: Rc::new(Counted(Rc::clone(&called))),
        host_commands: Rc::clone(&engine.host_commands),
    };
    let first = tcl_vm::Value::int(17);
    let lease = first.native_lifetime_lease();
    let later = tcl_vm::Value::int(29);
    let scalar = engine.vm.capture_original_scalar_ingress().unwrap();
    let original = engine.vm.capture_original_operand_projection().unwrap();
    drop(first);
    assert!(!lease.value().native_object_is_live());
    let sources = [
        lease.into_value(),
        later.native_lifetime_lease().into_value(),
    ];
    let error = callback
        .invoke_projected(&mut engine.vm, &original, &scalar, &sources)
        .unwrap_err();
    let expected = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable("retired native object header"),
    );
    assert_eq!(error, EngineError::execution_failure(expected));
    assert_eq!(called.get(), 0);
    assert!(later.resident_string_bytes().is_none());
    assert!(later.native_object_is_live());
    drop(sources);
    assert!(
        engine
            .vm
            .finish_original_operand_projection(original)
            .is_err()
    );
    assert!(engine.vm.finish_original_scalar_ingress(scalar).is_err());
}

struct Typed(NativeExecutionError);
impl HostCommand for Typed {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::execution_failure(self.0.clone()))
    }
}

#[test]
fn original_engine_typed_callback_failure_stops_guest_continuation_and_allows_next_entry() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = TclVmEngine::new();
    let first = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable("actual Engine callback cause"),
    );
    engine
        .define_command("originalTyped", Rc::new(Typed(first.clone())))
        .unwrap();
    let handle = engine
        .compile(CompileUnit {
            name: "typed",
            parameters: &[],
            body: "set ::before BEFORE; catch {originalTyped}; set ::after LATER",
        })
        .unwrap();
    assert_eq!(
        engine.invoke(&handle, &[]).unwrap_err(),
        EngineError::execution_failure(first)
    );
    assert!(engine.vm.get_var("after").is_none());
    assert_eq!(engine.variable("before").unwrap().as_str(), Some("BEFORE"));
    assert_eq!(
        engine
            .eval_in_invocation("return NEXT")
            .unwrap()
            .value
            .as_str(),
        Some("NEXT")
    );
}

#[test]
fn original_engine_returned_option_header_refusal_precedes_original_result_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = TclVmEngine::new();
    let options = tcl_vm::Value::list(vec![
        tcl_vm::Value::string("-errorcode"),
        tcl_vm::Value::int(41),
    ]);
    let lease = options.native_lifetime_lease();
    drop(options);
    assert!(!lease.value().native_object_is_live());
    let result = tcl_vm::Value::int(17);
    let completion = Completion::new(
        Code::Error,
        result.native_lifetime_lease().into_value(),
        lease.into_value(),
    );
    let expected = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable("retired native object header"),
    );
    assert_eq!(
        TclVmEngine::completion_to_result(&mut engine.vm, &completion).unwrap_err(),
        EngineError::execution_failure(expected)
    );
    assert!(result.resident_string_bytes().is_none());
    assert!(result.native_object_is_live());
    assert!(!completion.options.native_object_is_live());
}

struct Nested {
    first: NativeExecutionError,
    later: NativeExecutionError,
}
impl HostCommand for Nested {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::Unsupported(
            "nested invocation needs its actual registrar",
        ))
    }
    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        _: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        let error = registrar
            .eval_in_invocation("originalInnerTyped")
            .unwrap_err();
        assert_eq!(error, EngineError::execution_failure(self.first.clone()));
        Err(EngineError::execution_failure(self.later.clone()))
    }
}

#[test]
fn original_engine_registrar_retains_earlier_nested_failure_before_later_callback_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = TclVmEngine::new();
    let first = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable("actual nested original cause"),
    );
    let later = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable("later outer callback cause"),
    );
    engine
        .define_command("originalInnerTyped", Rc::new(Typed(first.clone())))
        .unwrap();
    engine
        .define_command(
            "originalNested",
            Rc::new(Nested {
                first: first.clone(),
                later,
            }),
        )
        .unwrap();
    assert_eq!(
        engine
            .eval_in_invocation("catch {originalNested}; set ::later LATER")
            .unwrap_err(),
        EngineError::execution_failure(first)
    );
    assert!(engine.vm.get_var("later").is_none());
    assert_eq!(
        engine
            .eval_in_invocation("return RESTORED")
            .unwrap()
            .value
            .as_str(),
        Some("RESTORED")
    );
}

#[test]
fn original_engine_retirement_errors_remain_host_failures_before_guest_continuation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual NativeCommand retirement adapter policy, separate from invocation
    // Guest conversion. Software-only source control; no Native delete chronology.
    struct Retiring(EngineError);
    impl HostCommand for Retiring {
        fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
            Ok(Value::Empty.into())
        }
        fn retire_with_registrar(&self, _: &mut dyn CommandRegistrar) -> Result<(), EngineError> {
            Err(self.0.clone())
        }
    }
    for failure in [
        EngineError::Script {
            message: "retiring Guest-shaped error".into(),
            code: Some("RETIRED".into()),
        },
        EngineError::ScriptBytes {
            message: b"retiring\0\xff".to_vec(),
            code: Some(b"RETIRED".to_vec()),
            options: None,
        },
        EngineError::BudgetExceeded(BudgetKind::Commands),
    ] {
        let mut engine = TclVmEngine::new();
        let expected = failure.to_string();
        let callback = HostCommandShim {
            command: Rc::new(Retiring(failure)),
            host_commands: Rc::clone(&engine.host_commands),
        };
        let completion = callback.retire(&mut engine.vm);
        assert_eq!(completion.code, Code::Error);
        drop(completion);
        let first = engine
            .vm
            .try_eval_source("catch {set ::after_retirement LATER}")
            .unwrap_err();
        assert!(
            matches!(first, NativeExecutionError::HostCommandRefusal(actual) if actual.reason == expected)
        );
        assert!(engine.vm.get_var("after_retirement").is_none());
    }
}
