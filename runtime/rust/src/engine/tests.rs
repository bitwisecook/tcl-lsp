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
use tcl_engine_api::{NativeCVersion, NativeScalarCache, NativeStringStorageIdentity};

struct Constant;
impl HostCommand for Constant {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Ok(Value::string("host").into())
    }
}

fn engine(profile: &str) -> RuntimeEngine {
    let mut engine = RuntimeEngine::new();
    engine.set_release(profile).unwrap();
    engine
}

#[test]
fn scalar_importer_preserves_first_host_cause_and_guest_state() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    let mut engine = engine("tcl8.6");
    engine.interp.set_result_bytes(b"SEEDED RESULT");
    engine.interp.set_c_error_code(b"SEEDED CODE");
    let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "original scalar import refusal",
        ),
    );
    engine.interp.refuse_native_execution(first.clone());
    let error = to_obj(
        &mut engine.interp,
        &Value::NativeScalar(NativeScalarCache::Integer(17)),
    )
    .err()
    .expect("first actual scalar import refusal");
    assert!(matches!(error, EngineError::ExecutionFailure(cause) if *cause == first));
    assert_eq!(engine.interp.native_execution_refusal(), Some(first));
    assert_eq!(engine.interp.result_bytes(), b"SEEDED RESULT");
    assert_eq!(engine.interp.snapshot_error().code_bytes(), b"SEEDED CODE");
}

#[test]
fn completion_metadata_getter_failure_is_host_refusal_at_engine_boundary() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = engine("tcl9.0");
    let error = engine.eval_in_invocation("set ::prior BEFORE; catch {return -level 0 -code error -errorcode [lseq 100000001] BODY} captured options; set ::after YES").unwrap_err();
    assert!(matches!(error, EngineError::ExecutionFailure(_)));
    assert_eq!(
        engine.interp.native_execution_refusal(),
        Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::Materialization(
                tcl_syntax::raw_string::NativeMaterializationLimitError::new(
                    100_000_001,
                    100_000_000
                ),
            ),
        ),)
    );
    assert_eq!(
        crate::interp::obj_bytes(
            crate::test_storage::global_scalar(&engine.interp, b"prior").unwrap()
        ),
        b"BEFORE"
    );
    for name in [b"captured".as_slice(), b"options", b"::after"] {
        assert!(crate::test_storage::global_scalar(&engine.interp, name).is_none());
    }
}

// Software embedding controls: the audited name recipes live in the shared
// naming owner. These assertions exercise this adapter's actual tables,
// receipt authority and callbacks, rather than claiming a fresh C run.
#[test]
fn publication_receipts_keep_actual_slots_and_refuse_forged_stale_or_foreign_authority() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut engine = engine(profile);
        let service = engine.command_publication_service().unwrap();
        let publication = service
            .prepare(
                b"::scope::command",
                CommandPublicationPurpose::CreateCCommand,
            )
            .unwrap();
        assert!(!service.observed_presence(&publication).unwrap());
        let mut forged = publication.clone();
        forged.key.simple = Rc::from(&b"other"[..]);
        assert!(matches!(
            engine.define_prepared_command(forged, Rc::new(Constant)),
            Err(EngineError::ExecutionRefusal(_))
        ));
        engine
            .define_prepared_command(publication.clone(), Rc::new(Constant))
            .unwrap();
        assert!(matches!(
            engine.define_prepared_command(publication, Rc::new(Constant)),
            Err(EngineError::ExecutionRefusal(_))
        ));
        let deletion = service
            .prepare(
                b"::scope::command",
                CommandPublicationPurpose::DeleteCCommand,
            )
            .unwrap();
        assert!(service.observed_presence(&deletion).unwrap());
        engine
            .eval_in_invocation("rename ::scope::command ::scope::moved")
            .unwrap();
        assert!(matches!(
            engine.remove_prepared_command(deletion),
            Err(EngineError::ExecutionRefusal(_))
        ));
        assert_eq!(
            engine
                .eval_in_invocation("::scope::moved")
                .unwrap()
                .value
                .as_str(),
            Some("host")
        );
        let publication = service
            .prepare(
                b"::scope::another",
                CommandPublicationPurpose::CreateCCommand,
            )
            .unwrap();
        let mut foreign = self::engine(profile);
        assert!(matches!(
            foreign.define_prepared_command(publication, Rc::new(Constant)),
            Err(EngineError::ExecutionRefusal(_))
        ));
        let publication = service
            .prepare(
                b"::scope::replacement",
                CommandPublicationPurpose::CreateCCommand,
            )
            .unwrap();
        engine
            .eval_in_invocation("namespace delete ::scope; namespace eval ::scope {}")
            .unwrap();
        assert!(matches!(
            engine.define_prepared_command(publication, Rc::new(Constant)),
            Err(EngineError::ExecutionRefusal(_))
        ));
        let missing = service
            .prepare(b"absent", CommandPublicationPurpose::DeleteCCommand)
            .unwrap();
        assert!(!engine.remove_prepared_command(missing).unwrap());
        drop(engine);
        assert!(matches!(
            service.prepare(b"expired", CommandPublicationPurpose::CreateCCommand),
            Err(EngineError::ExecutionRefusal(_))
        ));
    }
}

struct CaptureScope(Rc<RefCell<Option<Rc<dyn CommandPublicationService>>>>);
impl HostCommand for CaptureScope {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Ok(Value::Empty.into())
    }
    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        _: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        *self.0.borrow_mut() = Some(registrar.command_publication_service()?);
        Ok(Value::Empty.into())
    }
}

#[test]
fn callback_scope_is_owned_and_c_creation_keeps_unqualified_global_rule() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = engine("tcl8.6");
    let captured = Rc::new(RefCell::new(None));
    engine
        .define_command("capture", Rc::new(CaptureScope(Rc::clone(&captured))))
        .unwrap();
    engine
        .eval_in_invocation("namespace eval scope {capture}")
        .unwrap();
    let service = captured.borrow_mut().take().unwrap();
    let global = service
        .prepare(b"global_host", CommandPublicationPurpose::CreateCCommand)
        .unwrap();
    let nested = service
        .prepare(b"child::nested", CommandPublicationPurpose::CreateCCommand)
        .unwrap();
    assert_eq!(global.key.namespace, 0);
    assert_ne!(nested.key.namespace, 0);
    engine
        .define_prepared_command(global, Rc::new(Constant))
        .unwrap();
    engine
        .define_prepared_command(nested, Rc::new(Constant))
        .unwrap();
    assert_eq!(
        engine
            .eval_in_invocation("list [global_host] [::scope::child::nested]")
            .unwrap()
            .value
            .as_str(),
        Some("host host")
    );
    assert!(matches!(
        service.note_publication(
            &service
                .prepare(b"pending", CommandPublicationPurpose::CreateCCommand)
                .unwrap(),
            true
        ),
        Err(EngineError::ExecutionRefusal(_))
    ));
}

#[test]
fn whitelist_keeps_renamed_opaque_host_token_and_exact_unit_tokens() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = engine("tcl8.6");
    engine
        .define_command_bytes(b"opaque\xff", Rc::new(Constant))
        .unwrap();
    engine.define_command("host", Rc::new(Constant)).unwrap();
    let handle = engine
        .compile(CompileUnit {
            name: "whitelist renamed host",
            parameters: &[],
            body: "moved",
        })
        .unwrap();
    engine.eval_in_invocation("rename host moved").unwrap();
    engine.restrict_commands(&["list"]).unwrap();
    assert_eq!(engine.invoke(&handle, &[]).unwrap().as_str(), Some("host"));
    let opaque = obj::Owned::fresh(new_string(b"opaque\xff"));
    assert_eq!(engine.interp.dispatch(&[opaque.as_ptr()]), Code::Ok);
    assert_eq!(engine.interp.result_bytes(), b"host");
    assert!(engine.interp.resolve_cmd_token(b"host").is_none());
}

struct Observe {
    view: HostArgumentView,
    arguments: Rc<RefCell<Vec<Value>>>,
}
impl HostCommand for Observe {
    fn argument_view(&self) -> HostArgumentView {
        self.view
    }
    fn invoke(&self, arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        *self.arguments.borrow_mut() = arguments.to_vec();
        Ok(arguments.first().cloned().unwrap_or(Value::Empty).into())
    }
}

#[test]
fn callbacks_and_results_keep_exact_counted_bytes_and_scalar_storage() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = engine("tcl8.6");
    let observed = Rc::new(RefCell::new(Vec::new()));
    engine
        .define_command(
            "observe",
            Rc::new(Observe {
                view: HostArgumentView::MaterializedStrings,
                arguments: Rc::clone(&observed),
            }),
        )
        .unwrap();
    let handle = engine
        .compile(CompileUnit {
            name: "counted callback argument",
            parameters: &["argument"],
            body: "observe $argument",
        })
        .unwrap();
    let bytes = &b"A\0\xffB"[..];
    let returned = engine
        .invoke(&handle, &[Value::string_bytes(bytes)])
        .unwrap();
    assert!(matches!(returned, Value::StringBytes(ref actual) if actual.as_ref() == bytes));
    assert!(
        matches!(&observed.borrow()[0], Value::StringBytes(actual) if actual.as_ref() == bytes)
    );
    engine
        .define_command(
            "observe",
            Rc::new(Observe {
                view: HostArgumentView::NativeObjectSnapshots,
                arguments: Rc::clone(&observed),
            }),
        )
        .unwrap();
    let input = Value::NativeScalar(NativeScalarCache::Integer(16)).with_resident_string_storage(
        Rc::<[u8]>::from(&b"0x10"[..]),
        NativeStringStorageIdentity::Allocated,
    );
    engine.invoke(&handle, &[input]).unwrap();
    assert!(
        matches!(&observed.borrow()[0], Value::Resident { value, string, storage: NativeStringStorageIdentity::Allocated }
            if matches!(value.as_ref(), Value::NativeScalar(NativeScalarCache::Integer(16))) && string.as_ref() == b"0x10")
    );
    let byte_array = Value::byte_array(Rc::<[u8]>::from(&b"\0\xff"[..]));
    engine.invoke(&handle, &[byte_array]).unwrap();
    assert!(
        matches!(&observed.borrow()[0], Value::ByteArray(bytes) if bytes.as_ref() == b"\0\xff")
    );
    let foreign = Value::NativeScalar(NativeScalarCache::WordBoolean {
        value: true,
        origin: NativeCVersion::V9_1,
    });
    assert!(matches!(
        engine.invoke(&handle, &[foreign]),
        Err(EngineError::ExecutionRefusal(_))
    ));
    let unknown = Value::Int(16).with_resident_string(Rc::<[u8]>::from(&b"0x10"[..]));
    assert!(matches!(
        engine.invoke(&handle, &[unknown]),
        Err(EngineError::ExecutionRefusal(_))
    ));
}

struct Refuse;
impl HostCommand for Refuse {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::ExecutionRefusal(
            "callback capability unavailable".into(),
        ))
    }
}

#[test]
fn host_refusal_is_uncatchable_and_retains_prior_effects_and_completion_state() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Existing child host-failure source/API contract:
    // naming.interpreter.original-child-host-refusal-transport
    // docs/design/analysis/name-resolution-proofs/interpreter-original-child-host-refusal-transport.md
    // This callback control adds a host reason without claiming C supplied it.
    let mut engine = engine("tcl8.6");
    engine.define_command("refuse", Rc::new(Refuse)).unwrap();
    assert!(matches!(
        engine.eval_in_invocation("set before 1; catch {refuse}; set after 1"),
        Err(EngineError::ExecutionFailure(_))
    ));
    assert!(crate::test_storage::global_scalar(&engine.interp, b"before").is_some());
    assert!(crate::test_storage::global_scalar(&engine.interp, b"after").is_none());
    assert_eq!(
        engine
            .eval_in_invocation("set before")
            .unwrap()
            .value
            .as_str(),
        Some("1")
    );
    let mut origin = Interp::new();
    origin.set_result_bytes(b"retained\0\xff");
    origin.set_return_state(3, Code::Other(7));
    origin.set_return_options(vec![(b"-custom".to_vec(), b"owned".to_vec())]);
    assert_eq!(
        fail(
            &mut origin,
            EngineError::ExecutionRefusal("first cause".into())
        ),
        Code::Error
    );
    assert_eq!(origin.result_bytes(), b"retained\0\xff");
    assert_eq!(origin.pending_return_level(), 3);
    assert_eq!(origin.pending_return_code(), Code::Other(7));
    assert_eq!(origin.pending_return_option_objects().len(), 1);
    let mut parent = Interp::new();
    parent.set_result_bytes(b"parent");
    parent.transport_host_refusal_from(&origin);
    assert_eq!(
        parent.host_command_refusal().as_deref(),
        Some("first cause")
    );
    assert_eq!(parent.result_bytes(), b"parent");
}

struct GuestError;
impl HostCommand for GuestError {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::ScriptBytes {
            message: b"message".to_vec(),
            code: None,
            options: Some(b"-code 1 -level 0 -errorcode {CUSTOM DETAIL} -tag retained".to_vec()),
        })
    }
}
#[test]
fn guest_failure_options_and_nonstandard_completions_use_shared_original_owner() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = engine("tcl8.6");
    engine.define_command("guest", Rc::new(GuestError)).unwrap();
    let result = engine.eval_in_invocation("catch {guest} message options; list $message [dict get $options -tag] [dict get $options -errorcode]").unwrap();
    assert_eq!(
        result.value.as_str(),
        Some("message retained {CUSTOM DETAIL}")
    );
    let completion = engine
        .eval_in_invocation("return -level 0 -code 7 -tag retained value")
        .unwrap();
    assert_eq!(completion.code, CompletionCode::Other(7));
    assert_eq!(completion.value.as_str(), Some("value"));
    assert!(value_bytes(&mut engine.interp, &completion.options)
        .unwrap()
        .windows(b"-tag retained".len())
        .any(|bytes| bytes == b"-tag retained"));
    let error = engine.eval_in_invocation("guest").unwrap_err();
    assert_eq!(error.script_message_bytes(), Some(&b"message"[..]));
    assert!(error
        .script_options_bytes()
        .unwrap()
        .windows(b"-tag retained".len())
        .any(|bytes| bytes == b"-tag retained"));
}

#[test]
fn compiled_handles_retain_installed_generation_and_original_interpreter() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut original = engine(profile);
        let unit = CompileUnit {
            name: "handle control",
            parameters: &[],
            body: "set ::entered ORIGINAL; return ORIGINAL",
        };
        let handle = original.compile(unit).unwrap();
        assert_eq!(
            original.invoke(&handle, &[]).unwrap().as_str(),
            Some("ORIGINAL")
        );
        let mut foreign = engine(profile);
        let foreign_handle = foreign.compile(unit).unwrap();
        assert_eq!(foreign_handle.procedure(), handle.procedure());
        assert!(matches!(
            foreign.invoke(&handle, &[]),
            Err(EngineError::ExecutionRefusal(_))
        ));
        assert!(foreign.interp.var_get(b"entered").is_none());
        original.eval_in_invocation("unset ::entered").unwrap();
        original
            .eval_in_invocation(&format!(
                "rename {} {{}}; proc {} {{}} {{set ::entered REPLACEMENT; return REPLACEMENT}}",
                handle.procedure(),
                handle.procedure(),
            ))
            .unwrap();
        assert!(matches!(
            original.invoke(&handle, &[]),
            Err(EngineError::ExecutionRefusal(_))
        ));
        assert!(original.interp.var_get(b"entered").is_none());
        assert_eq!(
            original
                .eval_in_invocation(handle.procedure())
                .unwrap()
                .value
                .as_str(),
            Some("REPLACEMENT")
        );
    }
}

#[cfg(debug_assertions)]
struct RetireChangingContext {
    interpreter: crate::interp::WeakInterp,
    calls: Rc<std::cell::Cell<usize>>,
}
#[cfg(debug_assertions)]
impl HostCommand for RetireChangingContext {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Ok(Value::Empty.into())
    }
    fn retire_with_registrar(&self, _: &mut dyn CommandRegistrar) -> Result<(), EngineError> {
        self.calls.set(self.calls.get() + 1);
        let mut interpreter = self.interpreter.upgrade().unwrap();
        let original = interpreter.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("engine-retirement".into(), "1.0".into())];
        interpreter.pin_context(&changed).unwrap();
        interpreter.pin_context(&original).unwrap();
        Err(EngineError::Script {
            message: "later guest error".into(),
            code: None,
        })
    }
}

// A changed and restored context is re-proved only in debug builds.
#[cfg(debug_assertions)]
#[test]
fn host_retirement_context_change_stops_later_definition_and_preserves_original_generation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = engine("tcl8.6");
    let calls = Rc::new(std::cell::Cell::new(0));
    let interpreter = engine.interp.downgrade();
    engine
        .define_command(
            "retiring",
            Rc::new(RetireChangingContext {
                interpreter,
                calls: calls.clone(),
            }),
        )
        .unwrap();
    let original = engine.interp.resolve_cmd_token(b"retiring");
    engine.interp.set_result_bytes(b"SEEDED RESULT");
    let result = engine.interp.get_obj_result();
    let stale = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "stale entered native operation",
        ),
    );
    assert!(matches!(
        engine.define_command("retiring", Rc::new(Constant)),
        Err(EngineError::ExecutionFailure(cause)) if *cause == stale
    ));
    assert_eq!(calls.get(), 1);
    assert_eq!(engine.interp.resolve_cmd_token(b"retiring"), original);
    assert_eq!(engine.interp.get_obj_result(), result);
    assert_eq!(engine.interp.native_execution_refusal(), Some(stale));
}

#[test]
fn package_and_host_publication_keep_pending_first_cause_without_mutation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = engine("tcl8.6");
    engine.define_command("kept", Rc::new(Constant)).unwrap();
    let original = engine.interp.resolve_cmd_token(b"kept");
    engine.interp.set_result_bytes(b"SEEDED RESULT");
    let result = engine.interp.get_obj_result();
    engine
        .interp
        .refuse_host_command("original Engine publication refusal");
    let first = engine.interp.native_execution_refusal();
    assert!(matches!(
        engine.provide_package("blocked", "1.0"),
        Err(EngineError::ExecutionFailure(_))
    ));
    assert!(matches!(
        engine.define_command("untouched::command", Rc::new(Constant)),
        Err(EngineError::ExecutionFailure(_))
    ));
    assert!(matches!(
        engine.remove_command("kept"),
        Err(EngineError::ExecutionFailure(_))
    ));
    assert_eq!(engine.interp.resolve_cmd_token(b"kept"), original);
    assert!(engine
        .interp
        .resolve_cmd_token(b"untouched::command")
        .is_none());
    assert_eq!(engine.interp.get_obj_result(), result);
    assert_eq!(engine.interp.native_execution_refusal(), first);
    // An explicit new evaluation clears the refused operation; version2
    // succeeds only if the refused publication did not install version1.
    engine.interp.reset_native_compilation_admission();
    assert!(engine.provide_package("blocked", "2.0").is_ok());
    assert!(matches!(
        engine.provide_package("blocked", "1.0"),
        Err(EngineError::ScriptBytes { .. })
    ));
}

struct RetireReplacing(Rc<RefCell<usize>>);
impl HostCommand for RetireReplacing {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Ok(Value::Empty.into())
    }
    fn retire_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
    ) -> Result<(), EngineError> {
        *self.0.borrow_mut() += 1;
        registrar.define_command("retiring", Rc::new(Constant))
    }
}
#[test]
fn retirement_callback_reentry_protects_new_generation_and_guest_traces_refuse_before_effects() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = engine("tcl8.6");
    let calls = Rc::new(RefCell::new(0));
    engine
        .define_command("retiring", Rc::new(RetireReplacing(Rc::clone(&calls))))
        .unwrap();
    assert!(!engine.remove_command("retiring").unwrap());
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(
        engine
            .eval_in_invocation("retiring")
            .unwrap()
            .value
            .as_str(),
        Some("host")
    );
    engine.eval_in_invocation("proc traced {} {}; proc observer args {set ::fired 1}; trace add command traced delete observer").unwrap();
    let service = engine.command_publication_service().unwrap();
    assert!(matches!(
        service.prepare(b"traced", CommandPublicationPurpose::DeleteCCommand),
        Err(EngineError::ExecutionRefusal(_))
    ));
    assert!(engine.interp.var_get(b"::fired").is_none());
    assert!(engine.interp.resolve_cmd_token(b"traced").is_some());
}

#[test]
fn original_object_view_and_logical_native_receipts_refuse_explicitly() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut logical = engine("irules");
    assert!(logical.interp.set_logical_name_provider(
        tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
    ));
    assert!(matches!(
        logical.command_publication_service(),
        Err(EngineError::ExecutionRefusal(_))
    ));
    logical
        .define_command("compatibility", Rc::new(Constant))
        .unwrap();
    let mut engine = engine("tcl8.6");
    engine
        .define_command(
            "original",
            Rc::new(Observe {
                view: HostArgumentView::OriginalObjects,
                arguments: Rc::new(RefCell::new(Vec::new())),
            }),
        )
        .unwrap();
    assert!(matches!(
        engine.eval_in_invocation("catch {original anything}; set after 1"),
        Err(EngineError::ExecutionFailure(_))
    ));
    assert!(crate::test_storage::global_scalar(&engine.interp, b"after").is_none());
}
#[test]
fn original_engine_completion_entry_distinguishes_independent_retry_and_nested_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut engine = engine("tcl8.6");
    let calls = std::cell::Cell::new(0);
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter(&engine.interp)
            .unwrap();
    engine.interp.set_result_bytes(b"PRIOR\0\xff");
    let prior = engine.interp.result_obj();
    engine
        .interp
        .refuse_host_command("original nested Engine completion cause");
    let first = engine.interp.native_execution_refusal().unwrap();
    let error = evaluate(&mut engine.interp, |interpreter| {
        calls.set(calls.get() + 1);
        interpreter.set_result_bytes(b"UNREACHED");
        Code::Ok
    })
    .unwrap_err();
    assert!(matches!(error, EngineError::ExecutionFailure(cause) if *cause == first));
    assert_eq!(calls.get(), 0);
    assert_eq!(engine.interp.result_obj(), prior);
    assert_eq!(engine.interp.native_execution_refusal(), Some(first));
    drop(scope);
    let completion = evaluate(&mut engine.interp, |interpreter| {
        calls.set(calls.get() + 1);
        interpreter.set_result_bytes(b"GUEST\0\xff");
        Code::Other(7)
    })
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(completion.code, CompletionCode::Other(7));
    assert_eq!(
        value_bytes(&mut engine.interp, &completion.value).unwrap(),
        b"GUEST\0\xff"
    );
    assert!(value_bytes(&mut engine.interp, &completion.options)
        .unwrap()
        .windows(b"-code 7".len())
        .any(|bytes| bytes == b"-code 7"));
    assert!(engine.interp.native_execution_refusal().is_none());
    assert_eq!(engine.interp.entered_native_operation_count(), 0);
}
