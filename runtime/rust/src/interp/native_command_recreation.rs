// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded original idle object-command recreation retirement.

use super::{Command, ObjCommand};
use std::rc::{Rc, Weak};
use tcl_registry::native_command_creation::{
    NativeCommandCreationProtocol, NativeObjectCommandCreationPurpose,
    NativeRecreatedObjectRetirement,
};

/// This weak observation neither extends an active command's lifetime nor
/// guesses ownership from a reference count. A completed original retirement
/// and an empty exact slot before it identify the delete-procedure stage.
pub(super) struct OriginalIdleObjectRetirement {
    original: Weak<ObjCommand>,
    had_delete_procedure: bool,
}

impl OriginalIdleObjectRetirement {
    pub(super) fn capture(command: Option<&Command>) -> Option<Self> {
        let Command::ObjCmd(command) = command? else {
            return None;
        };
        if command.creation_purpose
            != Some(NativeObjectCommandCreationPurpose::LegacyPublicObjectCommand)
        {
            return None;
        }
        Some(Self {
            original: Rc::downgrade(command),
            had_delete_procedure: command.delete_proc.is_some(),
        })
    }

    pub(super) fn completed_delete(&self, slot_was_empty: bool) -> bool {
        slot_was_empty && self.had_delete_procedure && self.original.upgrade().is_none()
    }
}

/// Consume the exact overwritten record outside its namespace table borrow.
/// `try_unwrap` proves ownership; extra-held/active records decline and keep
/// their ordinary destructor. No procedure, OO or local-Jim grant is inferred.
pub(super) fn discard_idle_recreated_object(
    protocol: NativeCommandCreationProtocol,
    command: Command,
) -> Result<(), Command> {
    let Command::ObjCmd(command) = command else {
        return Err(command);
    };
    if command
        .creation_purpose
        .and_then(|purpose| protocol.idle_recreated_object_retirement(purpose))
        != Some(NativeRecreatedObjectRetirement::DiscardWithoutDeleteProcedure)
    {
        return Err(Command::ObjCmd(command));
    }
    match Rc::try_unwrap(command) {
        Ok(mut command) => {
            // Original TclCreateObjCommandInNs discards cmdPtr without another
            // deleteProc. Original non-local Jim_ReplaceHashEntry overwrites
            // the callback-set value without a second valDestructor call.
            // Only this uniquely owned intermediate record uses that purpose.
            command.delete_proc = None;
            drop(command);
            Ok(())
        }
        Err(command) => Err(Command::ObjCmd(command)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::{default_host, Code, Interp, WeakInterp};
    use crate::namespace::GLOBAL;
    use crate::obj::{self, TclObj};
    use std::cell::{Cell, RefCell};
    use std::ffi::{c_int, c_void};

    const ORIGINALS: [(&str, &str, &str); 6] = [
        ("tcl8.4", "8.4.20", include_str!("../../../../rust/tcl-registry/tests/data/native_command_recreation_original/capture/8.4.20/execute.stdout")),
        ("tcl8.5", "8.5.19", include_str!("../../../../rust/tcl-registry/tests/data/native_command_recreation_original/capture/8.5.19/execute.stdout")),
        ("tcl8.6", "8.6.18", include_str!("../../../../rust/tcl-registry/tests/data/native_command_recreation_original/capture/8.6.18/execute.stdout")),
        ("tcl9.0", "9.0.4", include_str!("../../../../rust/tcl-registry/tests/data/native_command_recreation_original/capture/9.0.4/execute.stdout")),
        ("tcl9.1", "9.1.0", include_str!("../../../../rust/tcl-registry/tests/data/native_command_recreation_original/capture/9.1.0/execute.stdout")),
        ("jim", "jim", include_str!("../../../../rust/tcl-registry/tests/data/native_command_recreation_original/capture/jim/execute.stdout")),
    ];
    const INVOCATION: &[u8] = include_bytes!("../../../../rust/tcl-registry/tests/data/native_command_recreation_original/request/invoke.tcl");

    struct Probe {
        interpreter: WeakInterp,
        deletes: [Cell<u64>; 3],
        calls: [Cell<u64>; 3],
        recreated: Cell<u64>,
        case: usize,
        mode: usize,
        middle: Cell<*mut Record>,
        held: RefCell<Option<Command>>,
    }
    struct Record {
        owner: *mut Probe,
        role: usize,
    }

    unsafe extern "C" fn invoke_record(
        data: *mut c_void,
        interpreter: *mut Interp,
        _argc: c_int,
        _argv: *const *mut TclObj,
    ) -> c_int {
        // SAFETY: the test retains all records through interpreter teardown.
        let record = unsafe { &*data.cast::<Record>() };
        let owner = unsafe { &*record.owner };
        owner.calls[record.role].set(owner.calls[record.role].get() + 1);
        unsafe { &mut *interpreter }
            .set_result_bytes([b"OLD".as_slice(), b"MIDDLE", b"FINAL"][record.role]);
        0
    }

    fn middle_builtin(_interpreter: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        Code::Ok
    }

    unsafe extern "C" fn delete_record(data: *mut c_void) {
        // SAFETY: the test owns the record and client data independently of the
        // command's actual lifetime, including intentionally discarded records.
        let record = unsafe { &*data.cast::<Record>() };
        let owner = unsafe { &*record.owner };
        owner.deletes[record.role].set(owner.deletes[record.role].get() + 1);
        if record.role == 0 && owner.case == 2 {
            let mut interpreter = owner.interpreter.upgrade().unwrap();
            if owner.mode == 6 {
                interpreter.refuse_host_command("original public constructor delete refusal");
            } else if owner.mode == 7 {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed.packages = vec![("constructor-delete".into(), "1.0".into())];
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
        }
        if record.role == 0 && owner.case == 1 {
            let mut interpreter = owner.interpreter.upgrade().unwrap();
            if owner.mode == 2 {
                interpreter.bind_command_replacement(
                    GLOBAL,
                    b"retiring",
                    Command::Builtin(middle_builtin),
                );
            } else if owner.mode == 3 {
                assert_eq!(
                    interpreter.eval_str(b"proc retiring {} {return MIDDLE}"),
                    Code::Ok
                );
            } else {
                let command = if owner.mode == 5 {
                    ObjCommand::new_owned_runtime_shim(
                        invoke_record,
                        owner.middle.get().cast(),
                        Some(delete_record),
                    )
                } else {
                    ObjCommand::new(
                        invoke_record,
                        owner.middle.get().cast(),
                        Some(delete_record),
                    )
                };
                if owner.mode == 4 {
                    interpreter
                        .create_counted_engine_obj_command(b"retiring", command)
                        .unwrap();
                } else {
                    interpreter
                        .create_obj_command(b"retiring", command)
                        .unwrap();
                }
                if owner.mode == 1 {
                    *owner.held.borrow_mut() =
                        interpreter.namespaces().command_in(GLOBAL, b"retiring");
                }
            }
            owner.recreated.set(owner.recreated.get() + 1);
        }
    }

    fn actual_interpreter(environment: &str) -> Interp {
        Interp::with_native_core(
            default_host(),
            tcl_registry::model::ingress::resolve_environment(environment).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    fn records(interpreter: &Interp, case: usize, mode: usize) -> (Box<Probe>, Box<[Record; 3]>) {
        let mut owner = Box::new(Probe {
            interpreter: interpreter.downgrade(),
            deletes: std::array::from_fn(|_| Cell::new(0)),
            calls: std::array::from_fn(|_| Cell::new(0)),
            recreated: Cell::new(0),
            case,
            mode,
            middle: Cell::new(std::ptr::null_mut()),
            held: RefCell::new(None),
        });
        let pointer = std::ptr::from_mut(owner.as_mut());
        let mut records: Box<[Record; 3]> = Box::new(std::array::from_fn(|role| Record {
            owner: pointer,
            role,
        }));
        owner.middle.set(std::ptr::from_mut(&mut records[1]));
        (owner, records)
    }

    fn counts(owner: &Probe) -> [u64; 7] {
        [
            owner.deletes[0].get(),
            owner.deletes[1].get(),
            owner.deletes[2].get(),
            owner.calls[0].get(),
            owner.calls[1].get(),
            owner.calls[2].get(),
            owner.recreated.get(),
        ]
    }

    fn original_counts(rows: &str, case: usize, stage: &str) -> [u64; 7] {
        let prefix = format!("ROW\tcase={case}\tstage={stage}\t");
        let row = rows.lines().find(|row| row.starts_with(&prefix)).unwrap();
        let field = |name: &str| {
            row.split('\t')
                .find_map(|field| field.strip_prefix(name))
                .unwrap()
                .parse::<u64>()
                .unwrap()
        };
        assert_eq!(field("status="), 0);
        [
            field("old_delete="),
            field("middle_delete="),
            field("final_delete="),
            field("old_call="),
            field("middle_call="),
            field("final_call="),
            field("recreated="),
        ]
    }

    #[test]
    fn original_jim_public_constructor_owns_root_cstring_key_and_retains_occupied_key() {
        // naming.embedding.original-command-recreation-discard
        // docs/design/analysis/name-resolution-proofs/embedding-original-command-recreation-discard.md
        // Actual software CString/key/client-data roles, independent of the
        // native recreation probe's finite callback-count comparison.
        let mut interpreter = actual_interpreter("jim");
        let namespace = crate::cmd_namespace::original_argument_tests::jim_fixture_namespace(
            &mut interpreter,
            b"constructor_scope",
        );
        interpreter.set_current_ns(namespace.token);
        let (owner, mut records) = records(&interpreter, 0, 0);
        let original = interpreter
            .create_obj_command(
                b"::rootEntry\0ignored",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[0]).cast(),
                    Some(delete_record),
                ),
            )
            .expect("genuine Jim_CreateCommand root CString producer");
        let key = interpreter
            .namespaces()
            .original_jim_command_key(GLOBAL, b"rootEntry")
            .unwrap();
        assert_eq!(obj::bytes_of(key), b"::rootEntry");
        // SAFETY: the live original table owns this borrowed key header.
        assert_eq!(
            unsafe { (*key).ref_count },
            1,
            "only the actual table owns the key"
        );
        assert_eq!(
            interpreter
                .namespaces()
                .command_table_report_name(GLOBAL, b"rootEntry"),
            b"::rootEntry"
        );
        assert!(interpreter
            .namespaces()
            .command_in(namespace.token, b"rootEntry")
            .is_none());
        assert!(interpreter
            .namespaces()
            .command_in(GLOBAL, b"rootEntry\0ignored")
            .is_none());
        let replacement = interpreter
            .create_obj_command(
                b"rootEntry\0other",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[2]).cast(),
                    Some(delete_record),
                ),
            )
            .unwrap();
        assert_ne!(original, replacement);
        assert_eq!(owner.deletes[0].get(), 1);
        assert_eq!(
            interpreter
                .namespaces()
                .original_jim_command_key(GLOBAL, b"rootEntry"),
            Some(key),
            "replacement preserves the occupied original hash key"
        );
        // SAFETY: replacement retained the same live occupied table key.
        assert_eq!(unsafe { (*key).ref_count }, 1);
        assert_eq!(obj::bytes_of(key), b"::rootEntry");
        assert_eq!(interpreter.eval_str(b"::rootEntry"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"FINAL");
        assert_eq!(owner.calls[0].get(), 0);
        assert_eq!(owner.calls[2].get(), 1);
        assert!(interpreter.delete_command(b"::rootEntry"));
        assert_eq!(owner.deletes[2].get(), 1);
        assert!(interpreter
            .namespaces()
            .original_jim_command_key(GLOBAL, b"rootEntry")
            .is_none());
        assert!(interpreter.native_execution_refusal().is_none());
        interpreter.set_current_ns(GLOBAL);
        drop(namespace);
        drop(interpreter);
        assert_eq!(owner.deletes[0].get(), 1);
        assert_eq!(owner.deletes[2].get(), 1);
        assert!(owner.interpreter.upgrade().is_none());
    }

    #[test]
    fn original_public_constructor_stops_after_delete_host_or_restored_context_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Reached genuine delete callback and first cause are software controls;
        // this does not infer provider callback chronology or active ownership.
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            for mode in [6, 7] {
                let mut interpreter = actual_interpreter(environment);
                let (owner, mut records) = records(&interpreter, 2, mode);
                interpreter
                    .create_obj_command(
                        b"retiring",
                        ObjCommand::new(
                            invoke_record,
                            std::ptr::from_mut(&mut records[0]).cast(),
                            Some(delete_record),
                        ),
                    )
                    .unwrap();
                interpreter.set_result_bytes(b"CONSTRUCTOR PRIOR");
                let context = interpreter.runtime_context();
                assert!(interpreter
                    .create_obj_command(
                        b"retiring",
                        ObjCommand::new(
                            invoke_record,
                            std::ptr::from_mut(&mut records[2]).cast(),
                            Some(delete_record),
                        ),
                    )
                    .is_none());
                assert_eq!(owner.deletes[0].get(), 1);
                assert_eq!(owner.calls[2].get(), 0);
                assert_eq!(
                    owner.deletes[2].get(),
                    1,
                    "unpublished client data retires once"
                );
                assert_eq!(
                    obj::bytes_of(interpreter.get_obj_result()),
                    b"CONSTRUCTOR PRIOR"
                );
                assert_eq!(interpreter.runtime_context(), context);
                assert!(interpreter
                    .namespaces()
                    .command_in(GLOBAL, b"retiring")
                    .is_none());
                if mode == 6 {
                    let Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) =
                        interpreter.native_execution_refusal()
                    else {
                        panic!("original delete Host cause was lost");
                    };
                    assert_eq!(first.reason, "original public constructor delete refusal");
                } else {
                    assert_eq!(
                        interpreter.native_execution_refusal(),
                        Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                                "stale entered native operation"
                            )
                        ))
                    );
                }
                drop(interpreter);
                assert!(owner.interpreter.upgrade().is_none());
                assert_eq!(owner.deletes[0].get(), 1);
                assert_eq!(owner.deletes[2].get(), 1);
            }
        }
    }

    #[test]
    fn original_idle_object_recreation_matches_all_six_public_callback_windows() {
        // naming.embedding.original-command-recreation-discard
        // docs/design/analysis/name-resolution-proofs/embedding-original-command-recreation-discard.md
        // Preserve the original held client records and exact invocation source.
        // Token addresses, native generation reuse, active/Proc/local cases and
        // build/runtime entry authority are independent of these 60 row fields.
        for (environment, provider, rows) in ORIGINALS {
            assert_eq!(
                rows.lines().filter(|row| row.starts_with("ROW\t")).count(),
                10
            );
            for case in 0..2 {
                let mut interpreter = actual_interpreter(environment);
                let (owner, mut records) = records(&interpreter, case, 0);
                let original = interpreter
                    .create_obj_command(
                        b"retiring",
                        ObjCommand::new(
                            invoke_record,
                            std::ptr::from_mut(&mut records[0]).cast(),
                            Some(delete_record),
                        ),
                    )
                    .unwrap();
                assert_eq!(
                    counts(&owner),
                    original_counts(rows, case, "original-created"),
                    "{provider}/{case}"
                );
                let replacement = interpreter
                    .create_obj_command(
                        b"retiring",
                        ObjCommand::new(
                            invoke_record,
                            std::ptr::from_mut(&mut records[2]).cast(),
                            Some(delete_record),
                        ),
                    )
                    .unwrap_or_else(|| {
                        panic!(
                            "{provider}/{case}: replacement refused, first cause {:?}, resident result {:?}",
                            interpreter.native_execution_refusal(),
                            obj::native_object_snapshot(interpreter.get_obj_result())
                                .map(|snapshot| snapshot.resident),
                        )
                    });
                assert_ne!(
                    original, replacement,
                    "actual Runtime identities remain distinct"
                );
                assert_eq!(
                    counts(&owner),
                    original_counts(rows, case, "replacement-returned"),
                    "{provider}/{case}"
                );
                assert_eq!(
                    interpreter.eval_str(INVOCATION),
                    Code::Ok,
                    "{provider}/{case}"
                );
                assert_eq!(interpreter.result_bytes(), b"FINAL");
                assert_eq!(
                    counts(&owner),
                    original_counts(rows, case, "invocation-returned"),
                    "{provider}/{case}"
                );
                assert!(
                    unsafe {
                        crate::capi::Tcl_DeleteCommand(&mut interpreter, c"retiring".as_ptr())
                    } == 0
                );
                assert_eq!(
                    counts(&owner),
                    original_counts(rows, case, "public-delete-returned"),
                    "{provider}/{case}"
                );
                assert!(interpreter.native_execution_refusal().is_none());
                drop(interpreter);
                assert_eq!(
                    counts(&owner),
                    original_counts(rows, case, "interpreter-destroyed"),
                    "{provider}/{case}"
                );
                assert!(owner.interpreter.upgrade().is_none());
            }
        }
    }

    #[test]
    fn original_idle_jim_recreation_keeps_the_occupied_key_until_public_delete() {
        // naming.embedding.original-command-recreation-discard
        // docs/design/analysis/name-resolution-proofs/embedding-original-command-recreation-discard.md
        // Software table-key lifetime, separate from original486 callback rows.
        // Observe the real header without another native reference or name lookup.
        let mut interpreter = actual_interpreter("jim");
        let (owner, mut records) = records(&interpreter, 1, 0);
        interpreter
            .create_obj_command(
                b"retiring",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[0]).cast(),
                    Some(delete_record),
                ),
            )
            .unwrap();
        let original_key = interpreter
            .namespaces()
            .original_jim_command_key(GLOBAL, b"retiring")
            .expect("actual occupied Jim key");
        // Allocation-only lease permits the final retired-header observation;
        // it leaves the actual table-key count and retirement stage unchanged.
        let key_allocation = obj::NativeObjectLifetime::retain(original_key);
        let references = unsafe { (*original_key).ref_count };
        assert!(references > 0);
        let replacement = interpreter.create_obj_command(
            b"retiring",
            ObjCommand::new(
                invoke_record,
                std::ptr::from_mut(&mut records[2]).cast(),
                Some(delete_record),
            ),
        );
        assert!(
            replacement.is_some(),
            "original replacement refused: {:?}",
            interpreter.native_execution_refusal()
        );
        assert_eq!(
            interpreter
                .namespaces()
                .original_jim_command_key(GLOBAL, b"retiring"),
            Some(original_key)
        );
        assert!(obj::allocation_is_live(original_key));
        assert_eq!(unsafe { (*original_key).ref_count }, references);
        assert_eq!(owner.deletes[0].get(), 1);
        assert_eq!(owner.recreated.get(), 1);
        assert_eq!(owner.deletes[1].get(), 0);
        assert_eq!(owner.deletes[2].get(), 0);
        assert_eq!(interpreter.eval_str(INVOCATION), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"FINAL");
        assert_eq!(owner.calls[1].get(), 0);
        assert_eq!(owner.calls[2].get(), 1);
        assert!(interpreter.delete_command(b"retiring"));
        assert!(interpreter
            .namespaces()
            .original_jim_command_key(GLOBAL, b"retiring")
            .is_none());
        assert!(!obj::allocation_is_live(key_allocation.as_ptr()));
        drop(key_allocation);
        assert_eq!(owner.deletes[1].get(), 0);
        assert_eq!(owner.deletes[2].get(), 1);
        assert!(interpreter.native_execution_refusal().is_none());
        drop(interpreter);
        assert!(owner.interpreter.upgrade().is_none());
        assert_eq!(owner.deletes[0].get(), 1);
        assert_eq!(owner.deletes[1].get(), 0);
        assert_eq!(owner.deletes[2].get(), 1);
    }

    #[test]
    fn original_recreation_extra_owner_and_non_object_purposes_cannot_borrow_idle_discard() {
        // naming.embedding.original-command-recreation-discard
        // docs/design/analysis/name-resolution-proofs/embedding-original-command-recreation-discard.md
        // These are software refusal controls beyond the original idle probe.
        for mode in [1, 2, 3, 4, 5] {
            let mut interpreter = actual_interpreter("tcl8.6");
            let (owner, mut records) = records(&interpreter, 1, mode);
            interpreter
                .create_obj_command(
                    b"retiring",
                    ObjCommand::new(
                        invoke_record,
                        std::ptr::from_mut(&mut records[0]).cast(),
                        Some(delete_record),
                    ),
                )
                .unwrap();
            let replacement = interpreter.create_obj_command(
                b"retiring",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[2]).cast(),
                    Some(delete_record),
                ),
            );
            assert!(replacement.is_none());
            assert_eq!(owner.deletes[0].get(), 1);
            assert_eq!(owner.recreated.get(), 1);
            assert_eq!(owner.deletes[1].get(), 0);
            assert_eq!(owner.calls[2].get(), 0);
            assert!(interpreter.native_execution_refusal().is_some());
            if mode == 1 {
                assert!(interpreter
                    .namespaces()
                    .command_in(GLOBAL, b"retiring")
                    .is_none());
                drop(owner.held.borrow_mut().take());
                assert_eq!(
                    owner.deletes[1].get(),
                    1,
                    "extra-held record keeps ordinary retirement"
                );
            } else if mode == 2 {
                assert!(matches!(
                    interpreter.namespaces().command_in(GLOBAL, b"retiring"),
                    Some(Command::Builtin(_))
                ));
            } else if mode == 3 {
                assert!(matches!(
                    interpreter.namespaces().command_in(GLOBAL, b"retiring"),
                    Some(Command::Proc(_))
                ));
            } else {
                let Some(Command::ObjCmd(middle)) =
                    interpreter.namespaces().command_in(GLOBAL, b"retiring")
                else {
                    unreachable!()
                };
                assert_eq!(
                    middle.creation_purpose(),
                    Some(if mode == 4 {
                        NativeObjectCommandCreationPurpose::CountedEngineObjectCommand
                    } else {
                        NativeObjectCommandCreationPurpose::OwnedRuntimeShim
                    })
                );
                drop(middle);
            }
            drop(interpreter);
            if mode == 4 || mode == 5 {
                assert_eq!(
                    owner.deletes[1].get(),
                    1,
                    "unobserved producers keep ordinary owned cleanup"
                );
            }
        }
    }

    #[test]
    fn original_owned_shim_purpose_survives_the_public_constructor_facade() {
        // naming.embedding.original-command-recreation-discard
        // docs/design/analysis/name-resolution-proofs/embedding-original-command-recreation-discard.md
        // Actual software producer control: owning Rust shim cleanup is not the
        // external legacy public C callback retirement purpose.
        let mut interpreter = actual_interpreter("tcl8.6");
        let (owner, mut records) = records(&interpreter, 1, 0);
        interpreter
            .create_obj_command(
                b"retiring",
                ObjCommand::new_owned_runtime_shim(
                    invoke_record,
                    std::ptr::from_mut(&mut records[0]).cast(),
                    Some(delete_record),
                ),
            )
            .unwrap();
        let Some(Command::ObjCmd(original)) =
            interpreter.namespaces().command_in(GLOBAL, b"retiring")
        else {
            unreachable!()
        };
        assert_eq!(
            original.creation_purpose(),
            Some(NativeObjectCommandCreationPurpose::OwnedRuntimeShim)
        );
        assert!(
            OriginalIdleObjectRetirement::capture(Some(&Command::ObjCmd(original.clone())))
                .is_none()
        );
        drop(original);
        assert!(interpreter
            .create_obj_command(
                b"retiring",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[2]).cast(),
                    Some(delete_record),
                )
            )
            .is_none());
        assert_eq!(owner.deletes[0].get(), 1);
        assert_eq!(owner.recreated.get(), 1);
        assert_eq!(owner.deletes[1].get(), 0);
        assert_eq!(owner.calls[2].get(), 0);
        assert!(interpreter.native_execution_refusal().is_some());
        assert!(matches!(
            interpreter.namespaces().command_in(GLOBAL, b"retiring"),
            Some(Command::ObjCmd(_))
        ));
        drop(interpreter);
        assert_eq!(owner.deletes[1].get(), 1);
    }

    #[test]
    fn original_idle_retirement_requires_completed_original_delete_and_exact_creation_slot() {
        // naming.embedding.original-command-recreation-discard
        // docs/design/analysis/name-resolution-proofs/embedding-original-command-recreation-discard.md
        // Exact ownership checks do not infer idleness from an Rc count or a
        // same-spelled command, and do not donate the measured purpose to Proc.
        let selected = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
            .native_command_creation_protocol()
            .unwrap();
        let mut interpreter = actual_interpreter("tcl8.6");
        let (owner, mut records) = records(&interpreter, 0, 0);
        interpreter
            .create_obj_command(
                b"retiring",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[0]).cast(),
                    Some(delete_record),
                ),
            )
            .unwrap();
        let Command::ObjCmd(original) = interpreter
            .namespaces()
            .command_in(GLOBAL, b"retiring")
            .unwrap()
        else {
            unreachable!()
        };
        let view = Command::ObjCmd(original.clone());
        let retained = OriginalIdleObjectRetirement::capture(Some(&view)).unwrap();
        interpreter.namespaces_mut().remove_in(GLOBAL, b"retiring");
        drop(view);
        assert!(
            !retained.completed_delete(true),
            "a real retained owner is still active"
        );
        drop(original);
        assert!(retained.completed_delete(true));
        assert!(
            !retained.completed_delete(false),
            "a preexisting trace-created slot is a separate purpose"
        );
        assert!(
            OriginalIdleObjectRetirement::capture(Some(&Command::Builtin(middle_builtin)))
                .is_none()
        );
        assert!(discard_idle_recreated_object(selected, Command::Builtin(middle_builtin)).is_err());
        interpreter
            .create_obj_command(
                b"retiring",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[1]).cast(),
                    Some(delete_record),
                ),
            )
            .unwrap();
        let generation = interpreter
            .namespaces()
            .command_generation(GLOBAL, b"retiring")
            .unwrap();
        let entry = interpreter
            .namespaces_mut()
            .native_command_creation_entry(GLOBAL, b"retiring");
        assert!(interpreter
            .namespaces_mut()
            .take_recreated_object_for_creation(&entry, b"retiring", generation + 1)
            .is_none());
        interpreter
            .create_obj_command(
                b"different",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[2]).cast(),
                    None,
                ),
            )
            .unwrap();
        let different = interpreter
            .namespaces_mut()
            .native_command_creation_entry(GLOBAL, b"different");
        assert!(interpreter
            .namespaces_mut()
            .take_recreated_object_for_creation(&different, b"retiring", generation)
            .is_none());
        let empty = interpreter
            .namespaces_mut()
            .native_command_creation_entry(GLOBAL, b"absent");
        assert!(interpreter
            .namespaces_mut()
            .take_recreated_object_for_creation(&empty, b"retiring", generation)
            .is_none());
        let actual = interpreter
            .namespaces_mut()
            .take_recreated_object_for_creation(&entry, b"retiring", generation)
            .unwrap();
        assert!(discard_idle_recreated_object(selected, actual.command).is_ok());
        assert_eq!(owner.deletes[1].get(), 0);
        assert!(interpreter
            .namespaces()
            .command_in(GLOBAL, b"retiring")
            .is_none());
        drop(interpreter);
        assert_eq!(owner.deletes[0].get(), 1);
    }
}
