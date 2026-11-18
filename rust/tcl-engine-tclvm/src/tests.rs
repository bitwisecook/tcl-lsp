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

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use tcl_vm::{Code, Completion};

use super::{Engine, TclVmEngine, recover_dictionary_children, register_host_command};
use tcl_engine_api::{
    Budget, BudgetKind, CommandRegistrar, CompileUnit, EngineError, HostCommand, HostOutcome, Value,
};

#[test]
fn callback_cmdname_retains_private_scope_and_distinct_duplicate_identity() {
    use tcl_engine_api::{OriginalObject, OriginalObjectResult as R};
    use tcl_runtime_api::native_command_name::NativeCommandNameCache;
    use tcl_runtime_api::native_compilation::NativeInterpreterIdentity;
    let vm = tcl_vm::test_support::native_vm();
    let dialect = vm.native_scalar_carrier_dialect();
    let scope = vm.native_object_bridge_identity();
    let original = tcl_vm::Value::new_native_string_bytes(b"head".as_slice());
    let cache = NativeCommandNameCache {
        interpreter: NativeInterpreterIdentity {
            owner: scope.0,
            interpreter: scope.1,
        },
        version: tcl_dialect::TclVersion::V9_0,
        slot: tcl_runtime_api::CommandSlot::new(
            tcl_runtime_api::ByteNamespacePath::root(),
            b"head".as_slice().into(),
        ),
        namespace_token: 0,
        token: 17,
        implementation_generation: 17,
        command_epoch: 0,
        reference: None,
    };
    original
        .install_native_command_name_cache(cache.clone(), dialect)
        .unwrap();
    let capability = Rc::new(super::VmOriginalObject {
        value: super::VmOriginalValue::Header(original.native_lifetime_lease()),
        dialect,
        interpreter: scope,
    });
    let retained = R::Original(capability.clone());
    let same = super::recover_original_result(&retained, dialect, scope).unwrap();
    assert_eq!(
        same.native_object_identity(),
        original.native_object_identity()
    );
    let carrier = capability.command_name_cache().unwrap();
    let duplicate = R::Resident {
        value: Box::new(R::CommandName(carrier.clone())),
        string: Rc::from(b"head".as_slice()),
        storage: tcl_engine_api::NativeStringStorageIdentity::Allocated,
    };
    let restored = super::recover_original_result(&duplicate, dialect, scope).unwrap();
    assert_ne!(
        restored.native_object_identity(),
        original.native_object_identity()
    );
    assert_eq!(restored.native_command_name_cache(), Some(cache));
    capability
        .apply(&duplicate, tcl_engine_api::ResidentStringMutation::Preserve)
        .unwrap();
    assert!(original.native_command_name_cache().is_some());
    assert!(
        duplicate.snapshot().is_err(),
        "opaque native descriptor is not a data snapshot"
    );
    let foreign = tcl_vm::test_support::native_vm().native_object_bridge_identity();
    assert!(super::recover_original_result(&duplicate, dialect, foreign).is_err());
    let forged = R::Resident {
        value: Box::new(R::CommandName(tcl_engine_api::NativeCommandNameCache::new(
            tcl_engine_api::NativeCVersion::V9_0,
            scope,
            Rc::new(()),
        ))),
        string: Rc::from(b"head".as_slice()),
        storage: tcl_engine_api::NativeStringStorageIdentity::Allocated,
    };
    assert!(super::recover_original_result(&forged, dialect, scope).is_err());
}

/// A namespace's name, kept as a value, resolves as its name does when the
/// namespace is later recreated under it: the engine retains no stale scope
/// behind the value. tclsh 8.6.18 and 9.0.4 print `HERE` for
/// `namespace eval ::a {set ::n [namespace current]}; namespace delete ::a;
/// namespace eval ::a {proc here {} {return HERE}}; namespace eval $::n {here}`.
#[test]
fn callback_namespace_primary_retains_private_scope_and_descriptor() {
    let mut engine = TclVmEngine::new();
    let handle = engine
        .compile(unit(
            "namespace eval ::a {set ::n [namespace current]}\n\
             namespace delete ::a\n\
             namespace eval ::a {proc here {} {return HERE}}\n\
             return [namespace eval $::n {here}]",
        ))
        .expect("compiles");
    let result = engine
        .invoke(&handle, &[Value::list([]), Value::list([])])
        .expect("runs");
    assert_eq!(result.as_str(), Some("HERE"));
}

#[test]
fn original_result_requires_its_actual_interpreter_and_native_issuer() {
    let first = tcl_vm::test_support::native_vm();
    let second = tcl_vm::test_support::native_vm();
    let original = tcl_vm::Value::from_native_string_bytes(b"a\0\xff".as_slice());
    let dialect = first.native_scalar_carrier_dialect();
    let scope = first.native_object_bridge_identity();
    let result =
        tcl_engine_api::OriginalObjectResult::Original(std::rc::Rc::new(super::VmOriginalObject {
            value: super::VmOriginalValue::Header(original.native_lifetime_lease()),
            dialect,
            interpreter: scope,
        }));
    let result = tcl_engine_api::OriginalObjectResult::Shared(std::rc::Rc::new(result));
    let returned = super::recover_original_result(&result, dialect, scope).unwrap();
    assert_eq!(
        returned.native_object_identity(),
        original.native_object_identity()
    );
    assert_eq!(
        returned.resident_string_bytes().unwrap().as_ref(),
        b"a\0\xff"
    );
    assert!(matches!(
        super::recover_original_result(&result, dialect, second.native_object_bridge_identity()),
        Err(EngineError::ExecutionRefusal(_))
    ));
    assert!(matches!(
        super::recover_original_result(
            &result,
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
            scope,
        ),
        Err(EngineError::ExecutionRefusal(_))
    ));
}

#[test]
fn native_callback_view_does_not_materialize_original_scalar_storage() {
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
    let original = tcl_vm::Value::from_native_scalar_cache(
        tcl_syntax::scalar_getter::NativeScalarCache::Number(tcl_syntax::number::Number::Double(
            1.5,
        )),
        None,
        dialect,
    )
    .unwrap();
    let carrier = super::from_vm_object_value(&original).unwrap();
    assert!(
        matches!(carrier,tcl_engine_api::Value::NativeScalar(tcl_engine_api::NativeScalarCache::Double(value)) if value.to_bits() == 1.5_f64.to_bits())
    );
    assert!(original.resident_string_bytes().is_none());
    let copied = super::to_vm_value(&carrier, dialect).unwrap();
    assert!(copied.resident_string_bytes().is_none());
    assert!(
        matches!(copied.native_scalar_cache(),Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(tcl_syntax::number::Number::Double(value))) if value.to_bits() == 1.5_f64.to_bits())
    );
}

#[test]
fn compiled_handle_does_not_invoke_a_replacement_binding() {
    let mut engine = TclVmEngine::new();
    let handle = engine
        .compile(CompileUnit {
            name: "binding",
            parameters: &[],
            body: "return ORIGINAL",
        })
        .unwrap();
    assert_eq!(
        engine.invoke(&handle, &[]).unwrap().as_str(),
        Some("ORIGINAL")
    );
    engine
        .vm
        .try_define_procedure(handle.procedure(), &[], "return REPLACED")
        .unwrap();
    assert!(matches!(
        engine.invoke(&handle, &[]),
        Err(EngineError::ExecutionRefusal(_))
    ));
}

struct Collector {
    emitted: RefCell<Vec<Vec<String>>>,
}

impl HostCommand for Collector {
    fn invoke(&self, arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        self.emitted.borrow_mut().push(
            arguments
                .iter()
                .map(|argument| argument.as_str().unwrap_or_default().to_owned())
                .collect(),
        );
        Ok(Value::Empty.into())
    }
}

fn unit(body: &str) -> CompileUnit<'_> {
    CompileUnit {
        name: "test",
        parameters: &["words", "ctx"],
        body,
    }
}

fn registry_with_custom_list_expr_hook() -> tcl_registry::CommandRegistry {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let mut custom = registry.get("list").expect("list spec").clone();
    custom.lowering_hook = Some(tcl_registry::hooks::LoweringHookId::Expr);
    custom.inline_codegen_hook = Some(tcl_registry::hooks::InlineCodegenHookId::Expr);
    registry.insert(custom);
    registry
}

struct RefusingHostCommand;

impl HostCommand for RefusingHostCommand {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::ExecutionRefusal(
            "host provider unavailable".into(),
        ))
    }
}

#[test]
fn reached_host_refusal_preserves_prior_effects_and_cannot_be_caught() {
    let mut engine = TclVmEngine::new();
    engine
        .define_command("host_refuse", std::rc::Rc::new(RefusingHostCommand))
        .unwrap();
    let collector = std::rc::Rc::new(Collector {
        emitted: RefCell::new(Vec::new()),
    });
    engine.define_command("record", collector.clone()).unwrap();
    let handle = engine
        .compile(unit(
            "record BEFORE; catch {host_refuse} result; record AFTER; return $result",
        ))
        .unwrap();
    let error = engine
        .invoke(&handle, &[Value::Empty, Value::Empty])
        .unwrap_err();
    assert!(matches!(error.native_execution_failure(),
        Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(actual))
            if actual.reason == "host provider unavailable"
    ));
    assert_eq!(*collector.emitted.borrow(), [vec!["BEFORE".to_owned()]]);
    // Refusal cleanup permits a separate accepted operation without replaying
    // the first invocation or publishing its internal empty unwind carrier.
    let handle = engine.compile(unit("record NEXT; return OK")).unwrap();
    assert_eq!(
        engine
            .invoke(&handle, &[Value::Empty, Value::Empty])
            .unwrap()
            .as_str(),
        Some("OK")
    );
    assert_eq!(
        *collector.emitted.borrow(),
        [vec!["BEFORE".to_owned()], vec!["NEXT".to_owned()]]
    );
}

#[test]
fn invocation_evaluation_keeps_host_refusal_outside_guest_completion() {
    let mut engine = TclVmEngine::new();
    engine
        .define_command("host_refuse", Rc::new(RefusingHostCommand))
        .unwrap();
    let collector = Rc::new(Collector {
        emitted: RefCell::new(Vec::new()),
    });
    engine.define_command("record", collector.clone()).unwrap();
    let error = engine
        .eval_in_invocation("record BEFORE; catch {host_refuse} result; record AFTER")
        .unwrap_err();
    assert!(matches!(error.native_execution_failure(),
        Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(actual))
            if actual.reason == "host provider unavailable"
    ));
    assert_eq!(*collector.emitted.borrow(), [vec!["BEFORE".to_owned()]]);
    assert!(error.script_message_bytes().is_none());
    assert!(error.script_options_bytes().is_none());
}

#[test]
fn variable_consumers_report_trace_host_refusal_without_guest_projection() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for operation in ["r", "w", "u"] {
        let mut engine = TclVmEngine::new();
        engine
            .define_command("host_refuse", Rc::new(RefusingHostCommand))
            .unwrap();
        let collector = Rc::new(Collector {
            emitted: RefCell::new(Vec::new()),
        });
        engine.define_command("record", collector.clone()).unwrap();
        engine.eval_in_invocation(&format!(
                "set target ORIGINAL; proc trace_refusal args {{record BEFORE; host_refuse; record AFTER}}; trace variable target {operation} trace_refusal",
            )).unwrap();
        let error = match operation {
            "r" => engine.variable("target").map(|_| ()),
            "w" => engine.set_variable("target", Value::string("REPLACEMENT")),
            "u" => engine.unset_variable("target"),
            _ => unreachable!(),
        }
        .expect_err("actual reached host trace refusal");
        assert!(matches!(error.native_execution_failure(),
            Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(actual))
                if actual.reason == "host provider unavailable"
        ));
        assert!(error.script_message_bytes().is_none());
        assert!(error.script_options_bytes().is_none());
        assert_eq!(*collector.emitted.borrow(), [vec!["BEFORE".to_owned()]]);
    }
}

#[test]
fn checked_dictionary_import_materializes_original_keys_with_the_actual_issuer() {
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
    let protocol = dialect.native_string_protocol().unwrap();
    let key = tcl_vm::Value::from_native_byte_array(Rc::from(&b"\0\xff"[..]), dialect).unwrap();
    let identity = key.native_object_identity();
    let mut children = vec![key.clone(), tcl_vm::Value::string("member")];
    let dictionary = recover_dictionary_children(&mut children, 1, Some(4), protocol).unwrap();
    assert!(children.is_empty());
    assert_eq!(
        key.resident_string_bytes().unwrap().as_ref(),
        b"\xc0\x80\xc3\xbf"
    );
    let members = dictionary.cached_dictionary_representation().unwrap();
    assert_eq!(dictionary.cached_dictionary_bucket_count(), Some(4));
    assert_eq!(members[0].0.native_object_identity(), identity);
    assert_eq!(
        members[0].0.byte_array_representation().unwrap().as_ref(),
        b"\0\xff"
    );
}

#[test]
fn native_dictionary_search_fatal_refusal_crosses_the_engine_boundary_outside_guest_handlers() {
    use tcl_syntax::raw_string::NativeFatalCondition;
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
    for add in [false, true] {
        for body in [
            "record BEFORE;catch {advance} result;record AFTER;return $result",
            "record BEFORE;try {advance} on error {r o} {record CAUGHT};record AFTER",
        ] {
            let mut engine = TclVmEngine::new();
            let driver =
                tcl_vm::native_conformance::dictionary_search_mutation_driver(dialect, add)
                    .unwrap();
            engine.vm.register_native_command("advance", driver);
            let collector = Rc::new(Collector {
                emitted: RefCell::new(Vec::new()),
            });
            engine.define_command("record", collector.clone()).unwrap();
            let handle = engine.compile(unit(body)).unwrap();
            let error = engine
                .invoke(&handle, &[Value::Empty, Value::Empty])
                .unwrap_err();
            assert_eq!(
                error,
                EngineError::execution_failure(
                    tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::FatalCondition(
                            NativeFatalCondition::DictionarySearchConcurrentMutation
                        )
                    )
                )
            );
            assert_eq!(*collector.emitted.borrow(), [vec!["BEFORE".to_owned()]]);
            let next = engine.compile(unit("record NEXT;return OK")).unwrap();
            assert_eq!(
                engine
                    .invoke(&next, &[Value::Empty, Value::Empty])
                    .unwrap()
                    .as_str(),
                Some("OK")
            );
            assert_eq!(
                *collector.emitted.borrow(),
                [vec!["BEFORE".to_owned()], vec!["NEXT".to_owned()]]
            );
        }
    }
}

#[test]
fn a_unit_compiles_once_and_invokes_with_structured_values() {
    let mut engine = TclVmEngine::new();
    let handle = engine
        .compile(unit("return [llength $words]"))
        .expect("the body compiles");
    let words = Value::list([Value::string("a"), Value::string("b")]);
    let ctx = Value::dict_of([("nwords", Value::Int(2))]);
    let result = engine
        .invoke(&handle, &[words, ctx])
        .expect("the body runs");
    assert_eq!(result.as_str(), Some("2"));
}

#[test]
fn an_owned_registry_hook_survives_vm_profile_compilation() {
    let mut engine = TclVmEngine::with_registry(registry_with_custom_list_expr_hook());
    let handle = engine
        .compile(unit("list {1 + 2}"))
        .expect("the custom Expr hook compiles");
    let result = engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect("the custom hook's bytecode runs");

    // The normal `list` command returns the literal `1 + 2`; the owned
    // registry deliberately assigns it the Expr hook, which returns 3.
    assert_eq!(result.as_str(), Some("3"));
}

#[test]
fn a_dict_crosses_as_a_dict() {
    let mut engine = TclVmEngine::new();
    let handle = engine
        .compile(unit("return [dict get $ctx command]"))
        .expect("compiles");
    let ctx = Value::dict_of([
        ("command", Value::string("mylib::sort")),
        ("nwords", Value::Int(0)),
    ]);
    let result = engine
        .invoke(&handle, &[Value::list([]), ctx])
        .expect("runs");
    assert_eq!(result.as_str(), Some("mylib::sort"));
}

#[test]
fn a_host_command_receives_what_the_body_emits() {
    let mut engine = TclVmEngine::new();
    let collector = std::rc::Rc::new(Collector {
        emitted: RefCell::new(Vec::new()),
    });
    engine
        .define_command("role", collector.clone())
        .expect("registers");
    let handle = engine
        .compile(unit("role 0 varwrite\nrole 1 body"))
        .expect("compiles");
    engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect("runs");
    assert_eq!(
        *collector.emitted.borrow(),
        vec![
            vec!["0".to_string(), "varwrite".to_string()],
            vec!["1".to_string(), "body".to_string()],
        ],
    );
}

/// A host command that creates another through the door it is given.
struct Factory(std::rc::Rc<Collector>);

impl HostCommand for Factory {
    fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::Unsupported("a factory needs the door"))
    }

    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        _arguments: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        registrar.define_command("made", self.0.clone())?;
        Ok(Value::string("built").into())
    }
}

#[test]
fn a_host_command_registers_on_a_bare_vm_with_the_door_open() {
    let mut engine = TclVmEngine::new();
    let collector = std::rc::Rc::new(Collector {
        emitted: RefCell::new(Vec::new()),
    });
    register_host_command(
        engine.vm_mut(),
        "factory",
        std::rc::Rc::new(Factory(collector.clone())),
    );
    let handle = engine
        .compile(unit("set built [factory]\nmade $built 2"))
        .expect("compiles");
    engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect("a command registered on the VM can register another, and both run");
    assert_eq!(
        *collector.emitted.borrow(),
        vec![vec!["built".to_string(), "2".to_string()]],
    );
}

#[test]
fn a_command_nobody_registered_on_the_vm_is_not_there() {
    let mut engine = TclVmEngine::new();
    let handle = engine.compile(unit("factory")).expect("compiles");
    let error = engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect_err("no such command");
    assert_eq!(
        error.script_message_bytes(),
        Some(b"invalid command name \"factory\"".as_slice()),
        "{error:?}"
    );
}

#[test]
fn an_error_in_the_body_is_reported_not_propagated() {
    let mut engine = TclVmEngine::new();
    let handle = engine.compile(unit("error boom")).expect("compiles");
    let error = engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect_err("the body raises");
    assert!(
        error.script_message_bytes() == Some(b"boom".as_slice()),
        "{error:?}"
    );
}

#[test]
fn the_command_budget_is_enforced_and_distinguishable() {
    let mut engine = TclVmEngine::new();
    engine
        .set_budget(Budget::of_commands(200).with_wall_clock(Duration::from_secs(5)))
        .expect("the VM enforces both");
    // Dispatch-driven: `[$cmd length abc]` resolves a computed command
    // name, so every iteration is a real dispatch and is charged.
    let handle = engine
        .compile(unit(
            "set cmd string\nset i 0\nwhile {$i < 100000} { set x [$cmd length abc]\n incr i }",
        ))
        .expect("compiles");
    let error = engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect_err("the budget stops it");
    assert_eq!(error, EngineError::BudgetExceeded(BudgetKind::Commands));
}

/// The third budget, and the reason it exists: one `string repeat` spends
/// a single command and no measurable time while asking the allocator for
/// as much as it likes. With only the other two armed this reaches OOM —
/// which kills the process rather than the hook, so the host can neither
/// report nor quarantine it. Here it is an ordinary, catchable refusal.
#[test]
fn the_value_size_budget_stops_an_allocation_the_other_two_cannot_see() {
    let mut engine = TclVmEngine::new();
    engine
        .set_budget(
            Budget::of_commands(1_000_000)
                .with_wall_clock(Duration::from_secs(5))
                .with_max_value_bytes(1024),
        )
        .expect("the VM enforces all three");
    let handle = engine
        .compile(unit("string repeat aaaaaaaa 1000000"))
        .expect("compiles");
    let error = engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect_err("the value-size budget stops it");
    assert_eq!(error, EngineError::BudgetExceeded(BudgetKind::ValueSize));
}

/// The same call under the cap still works: the budget bounds a runaway,
/// it does not ban the command.
#[test]
fn a_value_within_the_size_budget_is_built_normally() {
    let mut engine = TclVmEngine::new();
    engine
        .set_budget(Budget::of_commands(1_000).with_max_value_bytes(1024))
        .expect("the VM enforces it");
    let handle = engine
        .compile(unit("string repeat ab 8"))
        .expect("compiles");
    let value = engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect("well within the cap");
    assert_eq!(value.as_str(), Some("abababababababab"));
}

struct Exhausted;

impl HostCommand for Exhausted {
    fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::BudgetExceeded(BudgetKind::Commands))
    }
}

/// A `catch` of a limit error does not turn the breach into an answer: a
/// host command's own exhausted budget, and the VM's value-size refusal,
/// are the invocation's failure whatever the body did with the error. The
/// next invocation starts with the breach forgotten.
#[test]
fn a_caught_budget_failure_is_still_the_invocations_failure() {
    let mut engine = TclVmEngine::new();
    engine
        .set_budget(Budget::of_commands(1_000).with_max_value_bytes(64))
        .expect("the VM enforces both");
    engine
        .define_command("exhausted", std::rc::Rc::new(Exhausted))
        .expect("registers");
    let arguments = [Value::list([]), Value::dict_of::<&str>([])];
    let commands = engine
        .compile(unit("catch {exhausted}; return exact"))
        .expect("compiles");
    assert_eq!(
        engine.invoke(&commands, &arguments).expect_err("exhausted"),
        EngineError::BudgetExceeded(BudgetKind::Commands)
    );
    let value_size = engine
        .compile(unit("catch {string repeat abcdefgh 100}; return exact"))
        .expect("compiles");
    assert_eq!(
        engine
            .invoke(&value_size, &arguments)
            .expect_err("over the cap"),
        EngineError::BudgetExceeded(BudgetKind::ValueSize)
    );
    let fine = engine.compile(unit("return exact")).expect("compiles");
    let value = engine
        .invoke(&fine, &arguments)
        .expect("a fresh invocation");
    assert_eq!(value.as_str(), Some("exact"));
}

/// Every value builder answers to the value-size limit, not `string repeat`
/// alone: a concatenation, a repeated list, an append loop, a word built
/// from substitutions, and a list that holds itself twice — which renders
/// to more than any host has while its elements stay small — are each
/// refused, inline or dispatched, and a `catch` does not hide it.
#[test]
fn every_value_builder_answers_to_the_value_size_budget() {
    let mut engine = TclVmEngine::new();
    engine
        .set_budget(
            Budget::of_commands(1_000_000)
                .with_wall_clock(Duration::from_secs(5))
                .with_max_value_bytes(64),
        )
        .expect("the VM enforces all three");
    let arguments = [Value::list([]), Value::dict_of::<&str>([])];
    for body in [
        "return [string cat abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh \
             abcdefgh abcdefgh abcdefgh]",
        "return [lrepeat 100 abcdefgh]",
        "return [lrepeat 1000000000 abcdefgh]",
        "set c string; return [$c cat abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh \
             abcdefgh abcdefgh abcdefgh abcdefgh]",
        "return [concat abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh \
             abcdefgh]",
        "return [join [lrepeat 8 abcdefgh] --]",
        "return [format %100s x]",
        "return [lseq 100]",
        "return [lseq 10000000 10000020]",
        "return [format %.100d 1]",
        "return [string map {a abcdefgh} aaaaaaaaaa]",
        "set s abcdefgh; while 1 {append s $s}",
        "set s abcdefgh; while 1 {set s $s$s}",
        "set l x; for {set i 0} {$i < 64} {incr i} {set l [list $l $l]}; string length $l",
        "set s abcdefgh; catch {while 1 {append s $s}}; return exact",
    ] {
        let handle = engine.compile(unit(body)).expect("compiles");
        assert_eq!(
            engine.invoke(&handle, &arguments).expect_err(body),
            EngineError::BudgetExceeded(BudgetKind::ValueSize),
            "{body}"
        );
    }
    let within = engine
        .compile(unit(
            "list [string cat ab cd] [lrepeat 2 x] [concat a b] [format %3s x]",
        ))
        .expect("compiles");
    let value = engine
        .invoke(&within, &arguments)
        .expect("well within the cap");
    assert_eq!(value.as_str(), Some("abcd {x x} {a b} {  x}"));
}

/// An error that merely spells a limit's message is the script's own error,
/// not a budget the body outran.
#[test]
fn a_script_error_spelling_a_limit_message_is_a_script_error() {
    let mut engine = TclVmEngine::new();
    engine
        .set_budget(Budget::of_commands(1_000))
        .expect("the VM enforces it");
    let handle = engine
        .compile(unit("error {command count limit exceeded}"))
        .expect("compiles");
    let error = engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect_err("the body raises");
    assert_eq!(
        error.script_message_bytes(),
        Some(b"command count limit exceeded".as_slice()),
        "{error:?}"
    );
}

/// A loop the compiler inlines dispatches nothing. Its commands still count
/// against the command budget at each command's start, as C's
/// `INST_START_CMD` counts them, but a budget sized for real work is far
/// from spent when the wall clock runs out. The wall clock is what contains
/// that case, and the two together are why the host sets both.
#[test]
fn the_wall_clock_budget_stops_a_loop_the_command_budget_cannot_see() {
    let mut engine = TclVmEngine::new();
    engine
        .set_budget(Budget::of_commands(1_000_000).with_wall_clock(Duration::from_millis(50)))
        .expect("the VM enforces both");
    let handle = engine
        .compile(unit("set i 0\nwhile {1} { incr i }"))
        .expect("compiles");
    let started = std::time::Instant::now();
    let error = engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect_err("the wall clock stops it");
    assert_eq!(error, EngineError::BudgetExceeded(BudgetKind::WallClock));
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the cap must fire promptly: {:?}",
        started.elapsed()
    );
}

#[test]
fn a_restricted_engine_has_only_the_whitelist_and_the_host_commands() {
    let mut engine = TclVmEngine::new();
    let collector = std::rc::Rc::new(Collector {
        emitted: RefCell::new(Vec::new()),
    });
    engine.define_command("fold", collector).expect("registers");
    let handle = engine
        .compile(unit("fold [string length [lindex $words 0]]"))
        .expect("compiles");
    engine
        .restrict_commands(&["set", "expr", "if", "return", "string", "lindex", "llength"])
        .expect("restricts");
    engine
        .invoke(
            &handle,
            &[
                Value::list([Value::string("abcde")]),
                Value::dict_of::<&str>([]),
            ],
        )
        .expect("the whitelisted body still runs");

    let opener = engine
        .compile(unit("open /etc/passwd"))
        .expect("a body naming a forbidden command still compiles");
    let error = engine
        .invoke(&opener, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect_err("but cannot run it");
    assert!(
        error
            .script_message_bytes()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .is_some_and(|message| message.contains("invalid command name")),
        "{error:?}"
    );
}

/// The compiler lowers `string trim` — and every other subcommand of an
/// ensemble the VM knows — to a direct call of `::tcl::string::trim`, so a
/// whitelist that names `string` has to keep those, or the same body runs when
/// the call is the argument of a host command and fails when it is anywhere
/// else. An ensemble the whitelist does not name keeps none of its own.
#[test]
fn a_whitelisted_ensembles_subcommands_run_wherever_they_are_called() {
    let arguments = [Value::list([]), Value::dict_of::<&str>([])];
    let mut engine = TclVmEngine::new();
    engine
        .restrict_commands(&["set", "return", "string", "dict"])
        .expect("restricts");
    for (body, expected) in [
        ("set r [string cat a - b]\nreturn $r", "a-b"),
        ("return [string toupper [string trim { x }]]", "X"),
        ("set d [dict create k v]\nreturn [dict get $d k]", "v"),
        ("return [string map {a b} [string cat a c]]", "bc"),
    ] {
        let handle = engine.compile(unit(body)).expect("compiles");
        let answer = engine.invoke(&handle, &arguments);
        assert_eq!(
            answer.as_ref().map(|value| value.as_str()),
            Ok(Some(expected)),
            "{body}"
        );
    }

    // Naming `dict` does not name `string`'s subcommands, spelt out or not.
    let mut dicts_only = TclVmEngine::new();
    dicts_only
        .restrict_commands(&["set", "return", "dict"])
        .expect("restricts");
    for body in [
        "return [::tcl::string::trim { x }]",
        "return [string trim { x }]",
    ] {
        let handle = dicts_only.compile(unit(body)).expect("compiles");
        assert!(dicts_only.invoke(&handle, &arguments).is_err(), "{body}");
    }
}

#[test]
fn return_is_an_ordinary_early_exit() {
    let mut engine = TclVmEngine::new();
    let handle = engine
        .compile(unit("if {[llength $words] == 0} { return }\nreturn folded"))
        .expect("compiles");
    let abstained = engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .expect("an early return is not an error");
    assert_eq!(abstained.as_str(), Some(""));
}

#[test]
fn a_fresh_native_release_selects_its_issuer_before_compilation() {
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let engine = TclVmEngine::for_native_release(name).expect("measured native core");
        let expected = tcl_registry::model::ingress::resolve_environment(name).unit_profile();
        assert_eq!(engine.release(), Some(expected.name));
        assert_eq!(
            engine.vm.native_scalar_carrier_dialect().execution_point(),
            tcl_registry::InvocationDialect::of_profile(expected).execution_point()
        );
    }
    for name in ["no-such-dialect", "", "tcl", "tk", "jim", "f5-irules"] {
        assert!(TclVmEngine::for_native_release(name).is_err(), "{name}");
    }
}

#[test]
fn fresh_native_engines_execute_their_own_units_and_refuse_foreign_handles() {
    let mut engine = TclVmEngine::for_native_release("tcl8.6").expect("native C8.6");
    let handle = engine
        .compile(unit("return [expr {6 * 7}]"))
        .expect("compiles");
    let arguments = [Value::list([]), Value::dict_of::<&str>([])];
    assert_eq!(
        engine.invoke(&handle, &arguments).unwrap().as_str(),
        Some("42")
    );
    let mut foreign = TclVmEngine::for_native_release("tcl9.0").expect("native C9.0");
    assert!(foreign.invoke(&handle, &arguments).is_err());
    let actual = foreign.vm.native_scalar_carrier_dialect().execution_point();
    foreign.set_release("tcl8.6").expect("source profile pins");
    assert_eq!(
        foreign.vm.native_scalar_carrier_dialect().execution_point(),
        actual
    );
}

/// `set_release` pins the VM's release: a leading zero is octal up to
/// 8.6 and decimal from 9.0 (`expr {010 + 0}` is 8 on tclsh 8.4 to 8.6
/// and 10 on 9.0 and 9.1), and every operation leaves the thread's own
/// numeral grammar as it found it.
#[test]
fn set_release_pins_the_numeral_grammar() {
    // An F5 dialect is for F5 only: no Tcl core evaluates for it, so it
    // has no row here.
    for (profile, want) in [("tcl8.6", "8"), ("tcl9.0", "10")] {
        let mut engine = TclVmEngine::new();
        let collector = std::rc::Rc::new(Collector {
            emitted: RefCell::new(Vec::new()),
        });
        engine
            .define_command("fold", collector.clone())
            .expect("registers");
        engine
            .set_release(profile)
            .expect("a catalogue profile pins");
        assert_eq!(engine.release(), Some(profile));
        let before = tcl_syntax::number::runtime_syntax();
        let handle = engine
            .compile(unit("fold [expr {010 + 0}]"))
            .expect("compiles");
        engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("runs");
        assert_eq!(
            *collector.emitted.borrow(),
            vec![vec![want.to_owned()]],
            "{profile}"
        );
        assert_eq!(
            tcl_syntax::number::runtime_syntax(),
            before,
            "{profile}: the thread keeps its own grammar"
        );
        assert_eq!(
            engine.set_release(profile),
            Ok(()),
            "{profile}: pinning the same release again is a no-op"
        );
        let other = if profile == "tcl9.0" {
            "tcl8.6"
        } else {
            "tcl9.0"
        };
        assert_eq!(
            engine.set_release(other),
            Err(EngineError::Unsupported(
                "pinning a release after a unit was compiled"
            )),
            "{profile}: a compiled engine keeps its release"
        );
    }
    for name in ["no-such-dialect", "", "tcl", "tk", "jim"] {
        let mut engine = TclVmEngine::new();
        assert_eq!(
            engine.set_release(name),
            Err(EngineError::Unsupported("pinning a release")),
            "{name:?} names no release the VM can pin"
        );
        assert_eq!(engine.release(), None);
    }
}

fn script_error_message(error: &EngineError) -> Option<&[u8]> {
    match error {
        EngineError::Script { message, .. } => Some(message.as_bytes()),
        EngineError::ScriptBytes { message, .. } => Some(message.as_slice()),
        _ => None,
    }
}

fn is_confined_store_error<T>(answer: &Result<T, EngineError>) -> bool {
    const MESSAGE: &[u8] = b"stores are confined to the activation";
    answer
        .as_ref()
        .err()
        .and_then(script_error_message)
        .is_some_and(|message| {
            message
                .windows(MESSAGE.len())
                .any(|window| window == MESSAGE)
        })
}

/// `confine_stores` keeps every write in the invocation's own frame. Each
/// body below writes somewhere else — a qualified global, an element, a
/// loop, destructuring or capture target, a `dict` update, a namespace
/// variable, a linked local, the caller's frame — and raises `can't set
/// …: stores are confined to the activation` without writing. A caught
/// error publishes neither `::errorInfo` nor `::errorCode`, since a later
/// invocation would read them. The same writes to locals succeed, and
/// reading a global still reads.
#[test]
fn confine_stores_refuses_every_store_outside_the_activation() {
    let arguments = [Value::list([]), Value::dict_of::<&str>([])];
    for (body, written) in [
        ("set ::g 1", "::g"),
        ("incr ::counter", "::counter"),
        ("append ::app x", "::app"),
        ("lappend ::l x", "::l"),
        ("incr ::incr_array(k)", "::incr_array"),
        ("append ::append_array(k) x", "::append_array"),
        ("lappend ::list_array(k) x", "::list_array"),
        ("global linked; incr linked", "::linked"),
        (
            "upvar 0 ::linked_list alias; lappend alias x",
            "::linked_list",
        ),
        ("set ::a(k) 1", "::a"),
        ("foreach ::x {1 2} {}", "::x"),
        ("lassign {1 2} ::p q", "::p"),
        ("regexp {(a)} a ::m", "::m"),
        ("regsub a abc b ::rs", "::rs"),
        ("scan 5 %d ::n", "::n"),
        ("dict set ::d k v", "::d"),
        ("dict unset ::du k", "::du"),
        ("dict lappend ::dl k v", "::dl"),
        ("dict incr ::di k", "::di"),
        ("dict append ::da k v", "::da"),
        ("binary scan A a ::b", "::b"),
        ("namespace eval ::ns {variable v 1}", "::ns::v"),
        ("global g2; set g2 1", "::g2"),
        ("upvar 0 ::g3 alias; set alias 1", "::g3"),
        ("uplevel 1 {set up 1}", "::up"),
    ] {
        let mut engine = TclVmEngine::new();
        engine.confine_stores().expect("the VM confines its stores");
        let handle = engine.compile(unit(body)).expect("compiles");
        let answer = engine.invoke(&handle, &arguments);
        assert!(is_confined_store_error(&answer), "{body}: {answer:?}");
        let probe = engine
            .compile(unit(&format!("return [info exists {written}]")))
            .expect("compiles");
        assert_eq!(
            engine.invoke(&probe, &arguments).expect("reads").as_str(),
            Some("0"),
            "{body}: nothing was written"
        );
    }
    let mut engine = TclVmEngine::new();
    engine.confine_stores().expect("the VM confines its stores");
    let published = "return [list [info exists ::errorInfo] [info exists ::errorCode]]";
    let caught = format!("catch {{lindex {{}} y}}; {published}");
    for body in [caught.as_str(), published] {
        let handle = engine.compile(unit(body)).expect("compiles");
        assert_eq!(
            engine.invoke(&handle, &arguments).expect("runs").as_str(),
            Some("0 0"),
            "{body}"
        );
    }
    let mut engine = TclVmEngine::new();
    engine
        .vm_mut()
        .set_var("::seen", tcl_vm::Value::string("yes"))
        .expect("seeds");
    engine.confine_stores().expect("the VM confines its stores");
    let handle = engine
        .compile(unit(
            "set acc {}; foreach x {a b} {lappend acc $x}; incr n; dict set d k v\n\
                 set arr(k) 1; lassign {1 2} p q; regexp {(a)} a whole m\n\
                 regsub a abc b rs; dict lappend dl k v; dict incr di k\n\
                 return [list $acc $n $d $arr(k) $p $m $rs $dl $di $::seen]",
        ))
        .expect("compiles");
    assert_eq!(
        engine.invoke(&handle, &arguments).expect("runs").as_str(),
        Some("{a b} 1 {k v} 1 1 a bbc {k v} {k 1} yes")
    );
    assert_confined_random_generator();
}

fn authored_random_engine() -> TclVmEngine {
    let mut engine = TclVmEngine::new();
    engine.set_release("f5-irules").expect("logical F5 release");
    let vm = engine.vm_mut();
    assert!(vm.set_logical_eval_object_provider(
        tcl_registry::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation,
    ));
    assert!(vm.set_logical_source_word_provider(
        tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation,
    ));
    assert!(vm.set_logical_expression_parse_provider(
        tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
    ));
    assert!(vm.set_logical_quote_provider(
        tcl_registry::invocation_words::LogicalExpressionQuoteProvider::Tcl84CoreSimulation,
    ));
    assert!(vm.set_logical_numeric_provider(
        tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core,
    ));
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ),)
    );
    assert!(vm.set_logical_compiled_variable_provider(
            tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation,
        ));
    assert!(vm.set_logical_math_function_provider(Some(
        tcl_runtime_api::expression_policy::AuthoredMathFunctionProvider::Tcl84Core,
    )));
    engine
}

#[test]
fn logical_random_release_requires_its_actual_or_authored_expression_issuer() {
    use tcl_runtime_api::{NativeCompilationAdmissionError, NativeExecutionError};
    for release in ["tcl8.4", "f5-irules"] {
        for body in ["expr {srand(7)}", "expr {rand()}"] {
            let mut engine = TclVmEngine::new();
            engine.set_release(release).unwrap();
            engine.confine_stores().unwrap();
            let handle = engine
                .compile(unit(body))
                .expect("declaration compiles lazily");
            let error = engine
                .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
                .unwrap_err();
            assert_eq!(
                error.native_execution_failure(),
                Some(&NativeExecutionError::CompilationAdmission(
                    NativeCompilationAdmissionError::NativePreflightRequired,
                ),)
            );
        }
    }
    let mut engine = authored_random_engine();
    assert!(engine.vm_mut().set_logical_math_function_provider(None));
    let handle = engine.compile(unit("expr {rand()}")).unwrap();
    let error = engine
        .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
        .unwrap_err();
    assert_eq!(
        error.native_execution_failure(),
        Some(&NativeExecutionError::CompilationAdmission(
            NativeCompilationAdmissionError::NativePreflightRequired,
        ),)
    );
}

fn assert_confined_random_generator() {
    let arguments = [Value::list([]), Value::dict_of::<&str>([])];
    // The `rand()` generator's seed is interpreter state every
    // invocation shares: `srand` writes it and `rand` reads what an
    // earlier call left. Both raise while stores are confined, under the
    // VM's default release and under 8.4, whose math functions are
    // `expr` builtins no command restriction removes.
    for release in [None, Some("tcl8.4"), Some("f5-irules")] {
        for body in ["expr {srand(7)}", "expr {rand()}"] {
            let mut engine = match release {
                Some("tcl8.4") => TclVmEngine::for_native_release("tcl8.4")
                    .expect("actual C8.4 fixed-table issuer"),
                Some("f5-irules") => authored_random_engine(),
                None => TclVmEngine::new(),
                Some(_) => unreachable!("fixed release control"),
            };
            engine.confine_stores().expect("the VM confines its stores");
            let handle = engine.compile(unit(body)).expect("compiles");
            let answer = engine.invoke(&handle, &arguments);
            assert!(
                is_confined_store_error(&answer),
                "{release:?} {body}: {answer:?}"
            );
        }
    }
}

#[test]
fn confined_empty_lappend_reads_existing_global_but_cannot_create_one() {
    let mut engine = TclVmEngine::new();
    engine
        .vm_mut()
        .eval_source("set ::existing {a b}")
        .expect("setup");
    engine.confine_stores().expect("confine");
    let args = [Value::list([]), Value::dict_of::<&str>([])];
    let read = engine
        .compile(unit("return [lappend ::existing]"))
        .expect("compiles");
    assert_eq!(
        engine.invoke(&read, &args).expect("reads").as_str(),
        Some("a b")
    );
    let append_read = engine
        .compile(unit("return [append ::existing]"))
        .expect("compiles");
    assert_eq!(
        engine.invoke(&append_read, &args).expect("reads").as_str(),
        Some("a b")
    );
    let missing = engine.compile(unit("lappend ::missing")).expect("compiles");
    let answer = engine.invoke(&missing, &args);
    assert!(
        answer
            .as_ref()
            .err()
            .and_then(script_error_message)
            .is_some()
    );
    assert!(engine.vm_mut().get_var("::missing").is_none());
}

#[test]
fn confined_read_modify_write_does_not_fire_foreign_read_trace() {
    let mut engine = TclVmEngine::new();
    engine.vm_mut().eval_source(
            "set ::reads 0; set ::counter 5; proc observe args {incr ::reads}; trace add variable ::counter read observe",
        ).expect("setup");
    engine.confine_stores().expect("confine");
    let handle = engine.compile(unit("incr ::counter")).expect("compiles");
    let arguments = [Value::list([]), Value::dict_of::<&str>([])];
    let answer = engine.invoke(&handle, &arguments);
    assert!(
        answer
            .as_ref()
            .err()
            .and_then(script_error_message)
            .is_some()
    );
    assert_eq!(
        engine
            .vm_mut()
            .get_var("::reads")
            .unwrap()
            .to_str()
            .as_ref(),
        "0"
    );
    assert_eq!(
        engine
            .vm_mut()
            .get_var("::counter")
            .unwrap()
            .to_str()
            .as_ref(),
        "5"
    );
}

/// `confine_stores` refuses an array's creation and an unset outside the
/// activation as it refuses a store: `array set ::fresh {}` makes no
/// global array, and no form of removal — a whole variable or an
/// element, `-nocomplain`, `array unset` with or without a pattern, a
/// linked local, a `dict update` over a missing key — takes away a
/// global seeded before the confinement. The same forms on locals run.
#[test]
fn confine_stores_refuses_creation_and_unset_outside_the_activation() {
    let arguments = [Value::list([]), Value::dict_of::<&str>([])];
    let run = |engine: &mut TclVmEngine, body: &str| {
        let handle = engine.compile(unit(body)).expect("compiles");
        engine
            .invoke(&handle, &arguments)
            .map(|value| value.as_str().expect("a string").to_owned())
    };
    let left = "return [list [info exists ::fresh] [info exists ::seed] [info exists ::arr(k)]]";
    for body in [
        "array set ::fresh {}",
        "unset ::seed",
        "unset -nocomplain ::seed",
        "unset ::arr(k)",
        "array unset ::arr",
        "array unset ::arr k*",
        "global seed; unset seed",
        "global arr; unset arr(k)",
        "upvar #0 seed alias; unset alias",
        "set d {}; dict update d k ::seed {}",
    ] {
        let mut engine = TclVmEngine::new();
        run(&mut engine, "set ::seed old; set ::arr(k) v").expect("an open VM writes globals");
        engine.confine_stores().expect("the VM confines its stores");
        let answer = run(&mut engine, body);
        assert!(is_confined_store_error(&answer), "{body}: {answer:?}");
        assert_eq!(run(&mut engine, left), Ok("0 1 1".to_owned()), "{body}");
    }
    let mut engine = TclVmEngine::new();
    engine.confine_stores().expect("the VM confines its stores");
    assert_eq!(
        run(
            &mut engine,
            "array set fresh {}; unset fresh; set x 1; unset x; set y 1; unset -nocomplain y z\n\
                 array set a {k 1 j 2}; unset a(k); array unset a j*; array unset a\n\
                 set d {}; dict update d k v {}\n\
                 return [list [info exists fresh] [info exists x] [info exists y] [info exists a]]",
        ),
        Ok("0 0 0 0".to_owned())
    );
}

/// An engine restricted to a whitelist that allows `expr` keeps its
/// math functions, which from 8.5 are commands (`tcl::mathfunc::abs`),
/// so a body answers `expr {abs(-1)}` under every pinned release rather
/// than only under 8.4, where they are builtins; `rand` and `srand` go
/// with every other command the whitelist does not name.
#[test]
fn a_restricted_engine_keeps_the_math_functions_but_the_generator() {
    let arguments = [Value::list([]), Value::dict_of::<&str>([])];
    for release in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "f5-irules",
    ] {
        let mut engine = TclVmEngine::new();
        engine.set_release(release).expect("pins");
        engine
            .restrict_commands(&["expr", "return"])
            .expect("restricts");
        engine.confine_stores().expect("confines");
        let handle = engine
            .compile(unit("return [expr {abs(-1) + int(2.5) + double(1)}]"))
            .expect("compiles");
        assert_eq!(
            engine.invoke(&handle, &arguments).expect("folds").as_str(),
            Some("4.0"),
            "{release}"
        );
        for body in ["return [expr {rand()}]", "return [expr {srand(7)}]"] {
            let handle = engine.compile(unit(body)).expect("compiles");
            assert!(
                engine.invoke(&handle, &arguments).is_err(),
                "{release}: {body}"
            );
        }
    }
}

/// A confined engine reads no host environment (`value-evaluation.md`
/// § *The rest of the route contract*): before `confine_stores` the
/// host has seeded `::env`, `::tcl_platform` and `::tcl_library`, and
/// after it none exists, so reading one raises as a read of an unset
/// variable does and a body cannot fold the analysing machine's user,
/// platform or paths into an answer about a program that runs
/// elsewhere.
#[test]
fn a_confined_engine_reads_no_host_environment() {
    let arguments = [Value::list([]), Value::dict_of::<&str>([])];
    let seeded = "return [list [info exists ::env] [info exists ::tcl_platform] \
                      [info exists ::tcl_library]]";
    let mut open = TclVmEngine::new();
    let handle = open.compile(unit(seeded)).expect("compiles");
    assert_eq!(
        open.invoke(&handle, &arguments).expect("reads").as_str(),
        Some("1 1 1"),
        "the host seeds its globals"
    );
    let mut confined = TclVmEngine::new();
    confined
        .confine_stores()
        .expect("the VM confines its stores");
    let handle = confined.compile(unit(seeded)).expect("compiles");
    assert_eq!(
        confined
            .invoke(&handle, &arguments)
            .expect("reads")
            .as_str(),
        Some("0 0 0"),
        "confining removes them"
    );
    for body in [
        "return $::tcl_platform(byteOrder)",
        "return $::tcl_library",
        "return $::env(PATH)",
    ] {
        let handle = confined.compile(unit(body)).expect("compiles");
        let answer = confined.invoke(&handle, &arguments);
        assert!(
            answer
                .as_ref()
                .err()
                .and_then(script_error_message)
                .is_some(),
            "{body}: {answer:?}"
        );
    }
}

#[test]
fn each_invocation_gets_its_own_locals() {
    let mut engine = TclVmEngine::new();
    let handle = engine
        .compile(unit(
            "if {[info exists leak]} { return leaked }\nset leak 1\nreturn fresh",
        ))
        .expect("compiles");
    for _ in 0..3 {
        let result = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect("runs");
        assert_eq!(result.as_str(), Some("fresh"));
    }
}
#[test]
fn byte_bridge_keeps_resident_spelling_and_binary_purity() {
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
    for bytes in [b"a\0z".as_slice(), b"a\xC0\x80z", b"a\xFFz"] {
        let value = super::to_vm_value(&Value::string_bytes(bytes), dialect).unwrap();
        assert_eq!(value.string_bytes().as_ref(), bytes);
        assert_eq!(
            super::from_vm_value(&value, dialect).unwrap().as_bytes(),
            Some(bytes)
        );
    }
    let pure = super::to_vm_value(&Value::byte_array(&b"\0\xFF"[..]), dialect).unwrap();
    let carrier = super::from_vm_value(&pure, dialect).unwrap();
    assert_eq!(carrier.as_byte_array(), Some(b"\0\xFF".as_slice()));
    assert_eq!(carrier.as_bytes(), None);
    assert!(pure.resident_string_bytes().is_none());
    let resident = Value::byte_array(&b"\0\xFF"[..]).with_resident_string(&b"custom\xFF"[..]);
    let value = super::to_vm_value(&resident, dialect).unwrap();
    assert_eq!(
        value
            .byte_array_representation()
            .expect("binary backing")
            .as_ref(),
        b"\0\xFF"
    );
    assert_eq!(
        value.resident_string_bytes().expect("resident").as_ref(),
        b"custom\xFF"
    );
}

#[test]
fn c84_long_carrier_retains_original_descriptor_and_refuses_foreign_release() {
    use tcl_engine_api::{NativeScalarCache, NativeStringStorageIdentity};
    let original = Value::NativeScalar(NativeScalarCache::Tcl84Long(17))
        .with_resident_string_storage(&b"17\0suffix"[..], NativeStringStorageIdentity::Allocated);
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    let native = super::to_vm_value(&original, dialect).unwrap();
    assert_eq!(
        native.native_scalar_cache(),
        Some(tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(17))
    );
    let exported = super::from_vm_value(&native, dialect).unwrap();
    assert_eq!(exported.as_bytes(), original.as_bytes());
    let Value::Resident { value, storage, .. } = exported else {
        panic!("resident native long")
    };
    assert_eq!(storage, NativeStringStorageIdentity::Allocated);
    assert!(matches!(
        value.as_ref(),
        Value::NativeScalar(NativeScalarCache::Tcl84Long(17))
    ));
    let foreign = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
    assert!(super::to_vm_value(&original, foreign).is_err());
}

#[test]
fn binary_bridge_seals_actual_constructor_and_validates_recorded_resident_storage() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_engine_api::NativeStringStorageIdentity;
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let binary = Value::byte_array(b"\xff\0A".as_slice());
    let original = super::to_vm_value(&binary, dialect).unwrap();
    let recipe = original.byte_array_origin().unwrap();
    assert_eq!(recipe.logical_provider(), None);
    assert_eq!(
        recipe.protocol(),
        tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6)
    );
    assert!(original.resident_string_bytes().is_none());
    assert_eq!(
        original.byte_array_representation().unwrap().as_ref(),
        b"\xff\0A"
    );
    assert_eq!(
        original
            .native_string_bytes(recipe.protocol())
            .unwrap()
            .as_ref(),
        b"\xc3\xbf\xc0\x80A"
    );
    assert_eq!(original.byte_array_origin(), Some(recipe));
    let resident = binary.clone().with_resident_string_storage(
        b"RECORDED\0\xff".as_slice(),
        NativeStringStorageIdentity::Allocated,
    );
    let imported = super::to_vm_value(&resident, dialect).unwrap();
    assert_eq!(
        imported.resident_string_bytes().unwrap().as_ref(),
        b"RECORDED\0\xff"
    );
    assert_eq!(
        imported.byte_array_representation().unwrap().as_ref(),
        b"\xff\0A"
    );
    assert_eq!(imported.byte_array_origin().unwrap(), recipe);
    for bad in [
        NativeStringStorageIdentity::Unknown,
        NativeStringStorageIdentity::CanonicalEmpty,
    ] {
        let invalid = binary
            .clone()
            .with_resident_string_storage(b"NONEMPTY".as_slice(), bad);
        assert!(matches!(
            super::to_vm_value(&invalid, dialect),
            Err(EngineError::ExecutionRefusal(_))
        ));
    }
    let jim = tcl_registry::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment("jim").analyser_profile(),
    );
    assert!(super::to_vm_value(&binary, jim).is_err());
    let authored =
        tcl_registry::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
    assert!(super::to_vm_value(&binary, authored).is_err());
}

#[test]
fn scalar_bridge_retains_full_cache_storage_and_descriptor_origin() {
    use tcl_engine_api::{
        NativeCVersion, NativeIntegerRadix, NativeScalarCache, NativeStringStorageIdentity,
    };
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
    let original = Value::NativeScalar(NativeScalarCache::BigInteger {
        negative: false,
        radix: NativeIntegerRadix::Decimal,
        digits: Rc::from("18446744073709551617"),
    })
    .with_resident_string_storage(
        &b"0x10000000000000001\0tail"[..],
        NativeStringStorageIdentity::Allocated,
    );
    let native = super::to_vm_value(&original, dialect).unwrap();
    let exported = super::from_vm_value(&native, dialect).unwrap();
    assert_eq!(exported.as_bytes(), original.as_bytes());
    let Value::Resident { value, storage, .. } = exported else {
        panic!("resident scalar")
    };
    assert_eq!(storage, NativeStringStorageIdentity::Allocated);
    assert!(
        matches!(value.as_ref(), Value::NativeScalar(NativeScalarCache::BigInteger { digits, .. }) if digits.as_ref()=="18446744073709551617")
    );

    let word = Value::NativeScalar(NativeScalarCache::WordBoolean {
        value: true,
        origin: NativeCVersion::V9_0,
    });
    assert!(super::to_vm_value(&word, dialect).is_err());
    let resident = word.with_resident_string(&b"YES\xFF"[..]);
    let native = super::to_vm_value(&resident, dialect).unwrap();
    assert_eq!(
        native.native_word_boolean_version(),
        Some(tcl_dialect::TclVersion::V9_0)
    );
    assert_eq!(native.resident_string_bytes().unwrap().as_ref(), b"YES\xFF");
    assert!(
        super::to_vm_value(
            &resident,
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
        )
        .is_err()
    );

    let bits = 0xfff8_1234_0000_4321;
    let native = super::to_vm_value(
        &Value::NativeScalar(NativeScalarCache::Double(f64::from_bits(bits))),
        dialect,
    )
    .unwrap();
    let Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
        tcl_syntax::number::Number::Double(value),
    )) = native.native_scalar_cache()
    else {
        panic!("Double cache")
    };
    assert_eq!(value.to_bits(), bits);
}

#[test]
fn checked_completion_projection_keeps_guest_option_conversion_error_guest() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A constructed malformed carrier reaches the actual checked guest
    // conversion owner; this is not an original-provider observation.
    let mut engine = TclVmEngine::new();
    let completion = Completion::new(
        Code::Error,
        tcl_vm::Value::int(17),
        tcl_vm::Value::list(vec![tcl_vm::Value::string("-errorcode")]),
    );
    let error = TclVmEngine::completion_to_result(&mut engine.vm, &completion).unwrap_err();
    assert!(matches!(
        &error,
        EngineError::Script { .. } | EngineError::ScriptBytes { .. }
    ));
    assert_eq!(
        error.script_code_bytes(),
        Some(b"TCL VALUE DICTIONARY".as_slice())
    );
    assert!(error.script_options_bytes().is_some());
    assert!(completion.result.resident_string_bytes().is_none());
    assert!(completion.options.resident_string_bytes().is_none());
}

#[test]
fn checked_completion_projection_retains_host_before_original_result_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // This deliberately retained owner failure proves adapter channel and
    // ordering only; no getter failure or original-provider pass is claimed.
    let mut engine = TclVmEngine::new();
    engine
        .vm
        .set_var("before", tcl_vm::Value::string("RETAINED"))
        .unwrap();
    let key = tcl_vm::Value::int(41);
    let value = tcl_vm::Value::int(42);
    let completion = Completion::new(
        Code::Error,
        tcl_vm::Value::int(17),
        tcl_vm::Value::list(vec![key.clone(), value.clone()]),
    );
    let cause = tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable;
    engine
        .vm
        .refuse_tcl_host_failure(tcl_vm::TclHostFailure::ValueAccess(cause));
    assert!(matches!(
        TclVmEngine::completion_to_result(&mut engine.vm, &completion),
        Err(EngineError::ExecutionFailure(first)) if *first == tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(cause)
    ));
    assert!(key.resident_string_bytes().is_none());
    assert!(value.resident_string_bytes().is_none());
    assert!(completion.result.resident_string_bytes().is_none());
    assert!(completion.options.resident_string_bytes().is_none());
    let error = engine.vm.try_eval_source("set late UNREACHED").unwrap_err();
    assert!(matches!(error,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(original)
            if original == cause
    ));
    assert!(engine.vm.get_var("late").is_none());
    assert_eq!(
        engine
            .vm
            .get_var("before")
            .unwrap()
            .resident_string_bytes()
            .unwrap()
            .as_ref(),
        b"RETAINED"
    );
}

#[test]
fn guest_byte_error_retains_complete_native_options() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let completion = tcl_vm::Completion::new(
        tcl_vm::Code::Error,
        tcl_vm::Value::from_string_bytes(b"prefix\0\xFF".as_slice()),
        tcl_vm::Value::list(vec![
            tcl_vm::Value::string("-errorcode"),
            tcl_vm::Value::from_string_bytes(b"CODE\0\xFF".as_slice()),
            tcl_vm::Value::string("-custom"),
            tcl_vm::Value::string("retained"),
        ]),
    );
    let expected_options = completion.options.string_bytes();
    let mut engine = TclVmEngine::new();
    let error = super::internal_error(
        &mut engine.vm,
        tcl_vm::TclError::from_completion(completion),
    );
    let EngineError::ScriptBytes {
        message,
        code,
        options,
    } = error
    else {
        panic!("byte guest carrier")
    };
    assert_eq!(message, b"prefix\0\xFF");
    assert_eq!(code.as_deref(), Some(b"CODE\0\xFF".as_slice()));
    assert_eq!(options.as_deref(), Some(expected_options.as_ref()));
}

#[test]
fn guest_error_export_preserves_original_stringless_dictionary_and_members() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_dialect::TclVersion;
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let native = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
            .unit_profile();
        let mut vm =
            tcl_vm::test_support::native_vm_for_profile(native, Box::new(std::io::stdout()));
        let key = tcl_vm::Value::new_native_string_bytes(b"-errorcode".as_slice());
        let code = tcl_vm::Value::new_native_string_bytes(b"CODE\0\xff".as_slice());
        let key_identity = key.native_object_identity();
        let code_identity = code.native_object_identity();
        let original = tcl_vm::Value::native_dictionary_constructor(
            vec![(key.clone(), code.clone())],
            Some(4),
            dialect.native_string_protocol().unwrap(),
        )
        .unwrap();
        let identity = original.native_object_identity();
        assert!(original.resident_string_bytes().is_none());
        let completion = tcl_vm::Completion::new(
            tcl_vm::Code::Error,
            tcl_vm::Value::new_native_string_bytes(b"FAILED\0\xff".as_slice()),
            original,
        );
        let EngineError::ScriptBytes {
            message,
            code: exported_code,
            options,
        } = TclVmEngine::completion_to_result(&mut vm, &completion).unwrap_err()
        else {
            panic!("original guest-error export")
        };
        assert_eq!(message, b"FAILED\0\xff");
        assert_eq!(exported_code.as_deref(), Some(b"CODE\0\xff".as_slice()));
        assert_eq!(
            options.as_deref(),
            completion.options.resident_string_bytes().as_deref()
        );
        assert_eq!(completion.options.native_object_identity(), identity);
        assert_eq!(completion.options.cached_dictionary_bucket_count(), Some(4));
        assert!(completion.options.cached_list_representation().is_none());
        drop(key);
        drop(code);
        completion
            .options
            .with_cached_dictionary_representation(|pairs, _| {
                assert_eq!(pairs[0].0.native_object_identity(), key_identity);
                assert_eq!(pairs[0].1.native_object_identity(), code_identity);
                assert!(!pairs[0].0.native_object_is_shared());
                assert!(!pairs[0].1.native_object_is_shared());
            })
            .unwrap();
    }
}

#[test]
fn guest_dictionary_options_refuse_an_unknown_native_issuer_without_shimmer() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
    let options = tcl_vm::Value::native_dictionary_constructor(
        vec![(
            tcl_vm::Value::string("-errorcode"),
            tcl_vm::Value::string("CODE"),
        )],
        Some(4),
        dialect.native_string_protocol().unwrap(),
    )
    .unwrap();
    let identity = options.native_object_identity();
    let completion = tcl_vm::Completion::new(
        tcl_vm::Code::Error,
        tcl_vm::Value::string("FAILED"),
        options,
    );
    let mut vm = tcl_vm::test_support::native_vm();
    let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )));
    vm.set_dialect_profile(unknown);
    assert!(
        vm.native_scalar_carrier_dialect()
            .native_string_protocol()
            .is_none()
    );
    assert!(matches!(
        TclVmEngine::completion_to_result(&mut vm, &completion),
        Err(EngineError::ExecutionFailure(_))
    ));
    assert_eq!(completion.options.native_object_identity(), identity);
    assert!(completion.options.resident_string_bytes().is_none());
    assert_eq!(completion.options.cached_dictionary_bucket_count(), Some(4));
    assert!(completion.options.cached_list_representation().is_none());
}

/// Records the words it is called with.
struct WordRecorder(Rc<RefCell<Vec<String>>>);

impl tcl_vm::NativeCommand for WordRecorder {
    fn invoke(&self, _vm: &mut tcl_vm::Vm, args: &[tcl_vm::Value]) -> Completion<tcl_vm::Value> {
        self.0
            .borrow_mut()
            .extend(args.iter().map(|word| word.to_str().to_string()));
        Completion::new(Code::Ok, tcl_vm::Value::empty(), tcl_vm::Value::empty())
    }
}

/// Evaluates its argument as a script, as a host command running a body does.
struct BodyEvaluator;

impl tcl_vm::NativeCommand for BodyEvaluator {
    fn invoke(&self, vm: &mut tcl_vm::Vm, args: &[tcl_vm::Value]) -> Completion<tcl_vm::Value> {
        let body = args[0].to_str().to_string();
        vm.eval_value_source(&body).expect("the body compiles")
    }
}

/// A script value keeps a bare CR in a braced word, at the top level and when
/// a host command evaluates a body: C translates line endings only where a
/// channel reads a file. tclsh 8.6.18/9.0.4:
/// `eval "set x {a\rb}\nstring first \r \$x"` -> 1.
#[test]
fn a_script_value_keeps_a_bare_carriage_return_in_a_braced_word() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let mut engine = TclVmEngine::new();
    engine
        .vm_mut()
        .register_native_command("record", Rc::new(WordRecorder(Rc::clone(&seen))));
    engine
        .vm_mut()
        .register_native_command("body", Rc::new(BodyEvaluator));
    let handle = engine
        .compile(CompileUnit {
            name: "carriage",
            parameters: &[],
            body: "record {left\rright}; body {record {left\rright}; set a 1}",
        })
        .unwrap();
    engine.invoke(&handle, &[]).unwrap();
    engine
        .vm_mut()
        .eval_value_source("record {left\rright}")
        .unwrap();
    assert_eq!(*seen.borrow(), ["left\rright"; 3]);
}
