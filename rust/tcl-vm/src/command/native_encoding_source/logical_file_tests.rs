// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use std::cell::RefCell;
use tcl_registry::source_file::LogicalSourceFileProvider;
use tcl_test_support::encoding_names::{EncodingHost, EncodingStages};

fn supplied() -> (Vm, Rc<EncodingStages>) {
    let host = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let mut vm = crate::native_fixture::interpreter(host);
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    assert!(vm.set_logical_eval_object_provider(
        tcl_registry::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation
    ));
    assert!(vm.set_logical_source_word_provider(
        tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation
    ));
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(vm.set_logical_compiled_variable_provider(tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation));
    let stages = Rc::new(EncodingStages::default());
    let host = EncodingHost::new(
        vm.host_rc(),
        Rc::clone(&stages),
        SystemEncoding::Utf8,
        b"return YES".to_vec(),
    );
    vm.set_host(Rc::new(host));
    stages.events.borrow_mut().clear();
    (vm, stages)
}

fn install(vm: &mut Vm) {
    assert!(
        vm.set_logical_source_file_provider(Some(LogicalSourceFileProvider::Tcl84CoreSimulation))
    );
}

#[test]
fn logical_file_provider_is_explicit_and_does_not_select_native_grammar() {
    let (mut vm, stages) = supplied();
    let args = [Value::new_native_string_bytes(b"/source.tcl".as_slice())];
    let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    assert!(
        vm.logical_source_file_entry(&names, &args)
            .unwrap()
            .is_none()
    );
    assert!(
        vm.native_invocation_dialect()
            .source_file_grammar()
            .is_none()
    );
    assert!(stages.events.borrow().is_empty());
    names.finish(&mut vm).unwrap();
    install(&mut vm);
    let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let entry = vm
        .logical_source_file_entry(&names, &args)
        .unwrap()
        .unwrap();
    assert_eq!(
        entry.grammar(),
        tcl_registry::source_file::SourceFileGrammar::Filename
    );
    assert_eq!(
        entry.protocol(),
        tcl_cmd_core::encoding::NativeSourceFileProtocol::CSystem(tcl_dialect::TclVersion::V8_4)
    );
    assert!(matches!(
        entry.grammar().select_original(3, Some(b"-encoding")),
        tcl_registry::source_file::SourceFileSelection::Invalid
    ));
    names.finish(&mut vm).unwrap();
}

#[test]
fn logical_file_ascii_kernel_reads_once_and_closes_the_actual_handle() {
    let (mut vm, stages) = supplied();
    install(&mut vm);
    let args = [Value::new_native_string_bytes(b"/source.tcl".as_slice())];
    let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let entry = vm
        .logical_source_file_entry(&names, &args)
        .unwrap()
        .unwrap();
    let mut ops = SourceOps {
        vm: &mut vm,
        names: &names,
        args: &args,
        encoding_at: None,
        reached: Vec::new(),
        logical: Some(&entry),
    };
    let prepared =
        tcl_cmd_core::encoding::prepare_source_file(&mut ops, entry.protocol(), 0, None).unwrap();
    assert_eq!(ops.reached, [0]);
    assert_eq!(prepared.path, b"/source.tcl");
    assert_eq!(prepared.script, b"return YES");
    assert_eq!(
        stages.events.borrow().as_slice(),
        ["filesystem", "open", "read", "close"]
    );
    drop(ops);
    names.finish(&mut vm).unwrap();
}

#[test]
fn logical_file_high_bytes_refuse_before_conversion_or_open() {
    for bytes in [
        b"/\xf0\x9f\x98\x80".as_slice(),
        b"/source.tcl\0\xff",
        b"\xc3\xa9",
    ] {
        let (mut vm, stages) = supplied();
        install(&mut vm);
        let args = [Value::new_native_string_bytes(bytes)];
        let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let entry = vm
            .logical_source_file_entry(&names, &args)
            .unwrap()
            .unwrap();
        let mut ops = SourceOps {
            vm: &mut vm,
            names: &names,
            args: &args,
            encoding_at: None,
            reached: Vec::new(),
            logical: Some(&entry),
        };
        let error =
            tcl_cmd_core::encoding::prepare_source_file(&mut ops, entry.protocol(), 0, None)
                .err()
                .expect("unsupported logical filename");
        assert_eq!(ops.reached, [0]);
        assert!(format!("{error:?}").contains("logical source filename byte correspondence"));
        assert!(stages.events.borrow().is_empty());
        drop(ops);
        let _ = names.finish(&mut vm);
    }
}

#[test]
fn logical_file_ascii_empty_dash_and_nul_keep_original_filename_kernel() {
    for (bytes, opens) in [
        (b"".as_slice(), true),
        (b"-encoding".as_slice(), true),
        (b"/source.tcl\0tail".as_slice(), true),
    ] {
        let (mut vm, stages) = supplied();
        install(&mut vm);
        let args = [Value::new_native_string_bytes(bytes)];
        let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let entry = vm
            .logical_source_file_entry(&names, &args)
            .unwrap()
            .unwrap();
        let mut ops = SourceOps {
            vm: &mut vm,
            names: &names,
            args: &args,
            encoding_at: None,
            reached: Vec::new(),
            logical: Some(&entry),
        };
        let prepared =
            tcl_cmd_core::encoding::prepare_source_file(&mut ops, entry.protocol(), 0, None);
        assert_eq!(ops.reached, [0]);
        assert_eq!(stages.events.borrow().contains(&"open"), opens);
        assert_eq!(prepared.is_ok(), bytes.starts_with(b"/source.tcl\0"));
        drop(ops);
        names.finish(&mut vm).unwrap();
    }
}

#[test]
fn logical_file_provider_withdrawal_and_restore_cannot_revive_entry() {
    let (mut vm, stages) = supplied();
    install(&mut vm);
    let args = [Value::new_native_string_bytes(b"/source.tcl".as_slice())];
    let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let entry = vm
        .logical_source_file_entry(&names, &args)
        .unwrap()
        .unwrap();
    assert!(vm.set_logical_source_file_provider(None));
    install(&mut vm);
    assert!(entry.check(&mut vm).is_err());
    assert!(stages.events.borrow().is_empty());
    assert!(names.finish(&mut vm).is_err());
}

#[test]
fn logical_file_read_header_withdrawal_keeps_first_error_and_closes_once() {
    let (mut vm, stages) = supplied();
    install(&mut vm);
    let owner = Rc::new(RefCell::new(Some(Value::new_native_string_bytes(
        b"/source.tcl".as_slice(),
    ))));
    let lease = owner.borrow().as_ref().unwrap().native_lifetime_lease();
    let released = Rc::clone(&owner);
    *stages.after_read.borrow_mut() = Some(Box::new(move || drop(released.borrow_mut().take())));
    let args = std::slice::from_ref(lease.value());
    let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let entry = vm.logical_source_file_entry(&names, args).unwrap().unwrap();
    let mut ops = SourceOps {
        vm: &mut vm,
        names: &names,
        args,
        encoding_at: None,
        reached: Vec::new(),
        logical: Some(&entry),
    };
    assert!(
        tcl_cmd_core::encoding::prepare_source_file(&mut ops, entry.protocol(), 0, None).is_err()
    );
    assert_eq!(ops.reached, [0]);
    assert_eq!(
        stages.events.borrow().as_slice(),
        ["filesystem", "open", "read", "close"]
    );
    drop(ops);
    let first = vm
        .execution_refusal
        .clone()
        .expect("original header refusal");
    assert_eq!(names.operation_scope().check_native(&mut vm), Err(first));
    assert!(names.finish(&mut vm).is_err());
}

#[test]
fn logical_file_nonresident_integer_keeps_the_single_original_string_stage() {
    let (mut vm, stages) = supplied();
    install(&mut vm);
    let args = [Value::int(17)];
    assert!(args[0].resident_string_bytes().is_none());
    let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let entry = vm
        .logical_source_file_entry(&names, &args)
        .unwrap()
        .unwrap();
    let mut ops = SourceOps {
        vm: &mut vm,
        names: &names,
        args: &args,
        encoding_at: None,
        reached: Vec::new(),
        logical: Some(&entry),
    };
    let bytes = ops.original_bytes(0).unwrap();
    assert_eq!(bytes.as_ref(), b"17");
    assert_eq!(ops.reached, [0]);
    assert_eq!(
        stages
            .events
            .borrow()
            .iter()
            .filter(|event| **event == "formatter")
            .count(),
        1
    );
    drop(ops);
    names.finish(&mut vm).unwrap();
}
