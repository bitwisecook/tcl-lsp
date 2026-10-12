// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software callback conversion controls using actual VM dispatch and owners.
use super::*;
use crate::interp::native_name_consumers::OriginalNameConsumerScope;
use crate::{Completion, NativeCommand};

struct BatchCommand(Rc<RefCell<Vec<String>>>);
impl NativeCommand for BatchCommand {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let counts: Vec<_> = args
            .iter()
            .map(Value::native_object_reference_count)
            .collect();
        let words = match vm.original_callback_unicode_words(args, "callback requires Unicode") {
            Ok(words) => words,
            Err(refusal) => return refusal,
        };
        assert_eq!(
            counts,
            args.iter()
                .map(Value::native_object_reference_count)
                .collect::<Vec<_>>()
        );
        *self.0.borrow_mut() = words;
        Completion::new(Code::Ok, Value::string(""), Value::string(""))
    }
}
#[test]
fn original_callback_batch_uses_actual_dispatch_and_counted_unicode_without_new_roles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software dispatch counts; no external callback/reference chronology claim.
    let (mut vm, formatter, queries) = provided();
    let seen = Rc::new(RefCell::new(Vec::new()));
    vm.register_native_command("callbackWords", Rc::new(BatchCommand(Rc::clone(&seen))));
    let args = [
        Value::new_native_string_bytes(b"A\xc0\x80B".as_slice()),
        Value::new_native_string_bytes(b"\xed\xa0\xbd\xed\xb9\x82".as_slice()),
    ];
    assert_eq!(
        vm.try_invoke_command("callbackWords", &args).unwrap().code,
        Code::Ok
    );
    assert_eq!(*seen.borrow(), ["A\0B", "🙂"]);
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
}
#[test]
fn original_callback_batch_requires_entered_operation_and_genuine_string_provider() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    assert!(!vm.native_operation_context.has_entered_operation());
    assert!(
        vm.original_callback_unicode_words(&[Value::string("resident")], "text")
            .is_err()
    );
    assert!(!vm.native_operation_context.has_entered_operation());
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    let mut independent = Vm::new();
    let scope = NativeOperationScope::enter(&mut independent).unwrap();
    assert!(
        independent
            .original_callback_unicode_words(&[Value::string("resident")], "text")
            .is_err()
    );
    assert!(independent.execution_refusal.is_some());
    drop(scope);
}
#[test]
fn original_callback_batch_refuses_non_text_before_reaching_later_getters() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    let args = [
        Value::new_native_string_bytes(b"\xed\xa0\x80".as_slice()),
        value(&vm, i64::MIN),
    ];
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    assert!(
        vm.original_callback_unicode_words(&args, "first non-text argument")
            .is_err()
    );
    assert!(
        matches!(vm.execution_refusal.as_ref(), Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(refusal)) if refusal.reason == "first non-text argument")
    );
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(args[1].resident_string_bytes().is_none());
    drop(scope);
}
#[test]
fn original_callback_batch_retains_formatter_failure_before_changed_name_owner() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The supplied formatter fails and retires the actual world at its format stage.
    let (mut vm, formatter, _) = provided();
    formatter.fail.set(true);
    *formatter.retire_name_world.borrow_mut() = Some(Rc::clone(&vm.name_world));
    let args = [value(&vm, i64::MIN), value(&vm, i64::MIN + 1)];
    let expected: tcl_cmd_core::CmdError = tcl_syntax::value::ValueError::from(
        tcl_syntax::raw_string::NativeStringAccessError::Unavailable(
            tcl_syntax::native_string::NativeStringUnavailable::StringUpdater,
        ),
    )
    .into();
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    assert!(vm.original_callback_unicode_words(&args, "text").is_err());
    assert_eq!(
        vm.execution_refusal.as_ref(),
        expected.native_execution_refusal()
    );
    assert_eq!(
        formatter
            .calls
            .borrow()
            .iter()
            .filter(|call| call.0 == "format")
            .count(),
        1
    );
    assert!(
        args.iter()
            .all(|value| value.resident_string_bytes().is_none())
    );
    drop(scope);
}
#[test]
fn supplied_callback_name_scope_keeps_original_frame_before_string_stages() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    let operation = NativeOperationScope::enter(&mut vm).unwrap();
    let names = OriginalNameConsumerScope::in_original_scope(&mut vm, &operation).unwrap();
    vm.declare_namespace("nested");
    let namespace = vm
        .namespace_object_lookup(&Value::string("::nested"))
        .unwrap()
        .unwrap();
    vm.push_ns_eval_token_frame(namespace, vec![Value::string("namespace")]);
    assert!(names.check(&mut vm).is_err());
    assert!(vm.execution_refusal.is_some());
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    vm.pop_ns();
    vm.pop_call_frame();
    assert!(names.finish(&mut vm).is_err());
}
