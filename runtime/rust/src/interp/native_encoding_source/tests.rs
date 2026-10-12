// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use tcl_test_support::encoding_names::{EncodingHost, EncodingStages};

fn supplied(script: &[u8]) -> (Interp, Rc<EncodingStages>) {
    let stages = Rc::new(EncodingStages::default());
    let host = EncodingHost::new(
        super::super::default_host(),
        Rc::clone(&stages),
        SystemEncoding::Identity,
        script.to_vec(),
    );
    let interpreter = Interp::with_native_core(
        Rc::new(host),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    stages.events.borrow_mut().clear();
    (interpreter, stages)
}

fn invoke(interpreter: &mut Interp, words: &[&[u8]]) -> super::super::Code {
    let argv = words
        .iter()
        .map(|bytes| super::super::new_string(bytes))
        .collect::<Vec<_>>();
    for &original in &argv {
        unsafe { crate::capi::Tcl_IncrRefCount(original) };
    }
    let code = interpreter.dispatch(&argv);
    for original in argv {
        unsafe { crate::capi::Tcl_DecrRefCount(original) };
    }
    code
}

#[test]
fn original_source_argument_encoding_is_consumed_and_missing_file_precedes_lookup() {
    // naming.encoding.original-byte-name-lookup-and-source-order
    // docs/design/analysis/name-resolution-proofs/encoding-original-byte-name-lookup-and-source-order.md
    let (mut interpreter, stages) = supplied(b"set ::encoded \xe9; return YES");
    assert_eq!(
        invoke(
            &mut interpreter,
            &[b"source", b"-encoding", b"iso8859-1\0TAIL", b"/source.tcl"]
        ),
        super::super::Code::Ok
    );
    assert_eq!(interpreter.result_bytes(), b"YES");
    assert_eq!(interpreter.eval_str(b"set ::encoded"), super::super::Code::Ok);
    assert_eq!(interpreter.result_bytes(), "é".as_bytes());
    assert_eq!(
        stages.events.borrow().as_slice(),
        ["filesystem", "open", "read", "close"]
    );
    stages.events.borrow_mut().clear();
    assert_eq!(
        invoke(
            &mut interpreter,
            &[b"source", b"-encoding", b"\xff", b"/absent.tcl"]
        ),
        super::super::Code::Error
    );
    assert_eq!(stages.events.borrow().as_slice(), ["filesystem", "open"]);
    assert_eq!(interpreter.eval_str(b"set ::errorCode"), super::super::Code::Ok);
    assert_eq!(
        interpreter.result_bytes(),
        b"POSIX ENOENT {no such file or directory}"
    );
}

#[test]
fn original_system_reset_and_catalogue_callback_keep_original_currency() {
    // naming.encoding.original-byte-name-lookup-and-source-order
    // docs/design/analysis/name-resolution-proofs/encoding-original-byte-name-lookup-and-source-order.md
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut interpreter, stages) = supplied(b"return YES");
    assert_eq!(
        invoke(&mut interpreter, &[b"encoding", b"system", b"ascii"]),
        super::super::Code::Ok
    );
    assert_eq!(interpreter.system_encoding(), SystemEncoding::Ascii);
    assert_eq!(
        invoke(&mut interpreter, &[b"encoding", b"system", b"\0TAIL"]),
        super::super::Code::Ok
    );
    assert_eq!(interpreter.system_encoding(), SystemEncoding::Identity);
    assert!(stages.events.borrow().is_empty());
    let scope =
        super::super::native_operation_currency::NativeOperationScope::enter(&interpreter).unwrap();
    let mut callback = interpreter.clone();
    *stages.after_lookup.borrow_mut() = Some(Box::new(move || {
        let original = callback.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("encoding-catalogue-callback".to_owned(), "1.0".to_owned())];
        callback.pin_context(&changed).unwrap();
        callback.pin_context(&original).unwrap();
    }));
    assert_eq!(
        invoke(
            &mut interpreter,
            &[b"encoding", b"system", b"external-codec"]
        ),
        super::super::Code::Error
    );
    let first = interpreter
        .native_execution_refusal()
        .expect("same original currency withdrawal");
    assert_eq!(
        scope.currency().ensure_current_or_refuse(),
        Err(first.clone())
    );
    assert_eq!(interpreter.system_encoding(), SystemEncoding::Identity);
    assert_eq!(stages.events.borrow().as_slice(), ["catalogue", "lookup"]);
}

#[test]
fn original_unknown_io_refusal_is_retained_before_mandatory_close_callback() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual supplied read/close callbacks, without external Native errno or close chronology.
    let (mut interpreter, stages) = supplied(b"set ::SHOULD_NOT_RUN YES");
    *stages.read_error.borrow_mut() = Some(HostError::Io("unrecorded errno".to_owned()));
    let scope =
        super::super::native_operation_currency::NativeOperationScope::enter(&interpreter).unwrap();
    let mut callback = interpreter.clone();
    *stages.after_close.borrow_mut() = Some(Box::new(move || {
        let original = callback.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("encoding-close-callback".to_owned(), "1.0".to_owned())];
        callback.pin_context(&changed).unwrap();
        callback.pin_context(&original).unwrap();
    }));
    assert_eq!(
        invoke(
            &mut interpreter,
            &[b"source", b"-encoding", b"utf-8", b"/source.tcl"]
        ),
        super::super::Code::Error
    );
    let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "source filesystem errno unavailable",
        ),
    );
    assert_eq!(interpreter.native_execution_refusal(), Some(first.clone()));
    assert_eq!(scope.currency().ensure_current_or_refuse(), Err(first));
    assert_eq!(
        stages.events.borrow().as_slice(),
        ["filesystem", "open", "read", "close"]
    );
}

#[test]
fn original_filename_external_zero_refuses_before_open_and_keeps_report_bytes() {
    // naming.source.original-counted-filename-filesystem-extent
    // docs/design/analysis/name-resolution-proofs/source-original-counted-filename-filesystem-extent.md
    for suffix in [b"\0TAIL".as_slice(), b"\xc0\x80TAIL".as_slice()] {
        let (mut interpreter, stages) = supplied(b"set ::SHOULD_NOT_RUN YES");
        assert_eq!(
            invoke(&mut interpreter, &[b"encoding", b"system", b"utf-8"]),
            super::super::Code::Ok
        );
        let original = [b"/source.tcl".as_slice(), suffix].concat();
        assert_eq!(
            invoke(&mut interpreter, &[b"source", &original]),
            super::super::Code::Error
        );
        assert_eq!(
            interpreter.result_bytes(),
            [
                b"couldn't read file \"".as_slice(),
                tcl_core_types::c_string_extent(&original),
                b"\": invalid argument"
            ]
            .concat()
        );
        assert_eq!(
            interpreter.eval_str(b"set ::errorCode"),
            super::super::Code::Ok
        );
        assert_eq!(
            interpreter.result_bytes(),
            b"POSIX EINVAL {invalid argument}"
        );
        assert!(stages.events.borrow().is_empty());
        assert!(interpreter.native_execution_refusal().is_none());
    }
}

#[test]
fn original_filename_supplied_codec_checks_currency_before_open() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut interpreter, stages) = supplied(b"set ::SHOULD_NOT_RUN YES");
    assert_eq!(
        invoke(&mut interpreter, &[b"encoding", b"system", b"ascii"]),
        super::super::Code::Ok
    );
    *stages.filename_conversion.borrow_mut() = Some(Ok(b"/source.tcl".to_vec()));
    let scope =
        super::super::native_operation_currency::NativeOperationScope::enter(&interpreter).unwrap();
    let mut callback = interpreter.clone();
    *stages.after_filename.borrow_mut() = Some(Box::new(move || {
        let original = callback.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("source-filename-callback".to_owned(), "1.0".to_owned())];
        callback.pin_context(&changed).unwrap();
        callback.pin_context(&original).unwrap();
    }));
    assert_eq!(
        invoke(&mut interpreter, &[b"source", b"/source.tcl"]),
        super::super::Code::Error
    );
    let first = interpreter
        .native_execution_refusal()
        .expect("actual original converter currency refusal");
    assert_eq!(scope.currency().ensure_current_or_refuse(), Err(first));
    assert_eq!(
        stages.events.borrow().as_slice(),
        ["filesystem", "filename-conversion"]
    );
}

#[test]
fn original_filename_iso88591_selected_system_keeps_plain_source_working() {
    // naming.source.original-counted-filename-filesystem-extent
    // docs/design/analysis/name-resolution-proofs/source-original-counted-filename-filesystem-extent.md
    let (mut interpreter, stages) = supplied(b"set ::filename_effect YES; return DONE");
    assert_eq!(
        invoke(&mut interpreter, &[b"encoding", b"system", b"iso8859-1"]),
        super::super::Code::Ok
    );
    assert_eq!(interpreter.system_encoding(), SystemEncoding::Iso88591);
    assert_eq!(
        invoke(&mut interpreter, &[b"source", b"/source.tcl"]),
        super::super::Code::Ok
    );
    assert_eq!(
        interpreter.eval_str(b"set ::filename_effect"),
        super::super::Code::Ok
    );
    assert_eq!(interpreter.result_bytes(), b"YES");
    assert_eq!(
        stages.events.borrow().as_slice(),
        ["filesystem", "open", "read", "close"]
    );
    assert!(interpreter.native_execution_refusal().is_none());
}
