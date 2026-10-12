// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed full Number/Bignum and actual increment observations from C Tcl.

#[cfg(have_tommath)]
mod measured {

    use crate::obj::{self, TclObj};
    use crate::value_ops::{RuntimeAppendValue, RuntimeIncrementObjects};
    use tcl_cmd_core::{CmdError, CmdErrorCodeUpdate};
    use tcl_dialect::TclVersion;
    use tcl_syntax::{
        native_object::NativeObjectCacheSnapshot as Cache,
        native_string::NativeStringProtocol,
        number::Number,
        scalar_getter::{NativeNumberGetterKind as Kind, NativeScalarGetterKind as Scalar},
    };

    fn bytes(hex: &str) -> Vec<u8> {
        if hex == "-" {
            return Vec::new();
        }
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    fn shape(index: usize, dialect: tcl_registry::InvocationDialect) -> obj::Owned {
        let texts: [&[u8]; 10] = [
            b"1",
            b"1.5",
            b"NaN",
            b"08",
            b"184467440737095516160000",
            b"true",
            b"",
            b" 2 ",
            b"1\0x",
            b"1\xc0\x80x",
        ];
        if index < texts.len() {
            return obj::Owned::fresh(obj::new_string_bytes(texts[index]));
        }
        match index {
            10 => obj::Owned::fresh(obj::new_double_obj(1.5)),
            11 => {
                let value = obj::Owned::fresh(obj::new_string_bytes(b"NaN"));
                let _ = crate::typed_value::native_scalar_getter(
                    value.as_ptr(),
                    dialect,
                    Scalar::Double,
                );
                value
            }
            12 => obj::Owned::fresh(obj::new_wide_int_obj(7)),
            13 => {
                let value = obj::Owned::fresh(obj::new_string_bytes(b"true"));
                crate::typed_value::native_scalar_getter(value.as_ptr(), dialect, Scalar::Boolean)
                    .unwrap();
                value
            }
            14 | 15 => {
                let value = obj::Owned::fresh(obj::new_string_bytes(texts[4]));
                crate::typed_value::native_number_probe(value.as_ptr(), dialect, Kind::Bignum)
                    .unwrap()
                    .unwrap();
                if index == 15 {
                    obj::invalidate_string(value.as_ptr());
                    // Explicit native host header construction, independently of empty canonical storage.
                    unsafe { obj::set_native_updater_string_rep(value.as_ptr(), b"", false) };
                }
                value
            }
            _ => unreachable!("fixed native shape"),
        }
    }
    fn primary(value: *mut TclObj) -> &'static str {
        match obj::native_object_snapshot(value).unwrap().cache {
            Cache::None => "none",
            Cache::Numeric(tcl_syntax::scalar_getter::NativeScalarCache::Number(Number::Int(
                _,
            ))) => "int",
            Cache::Numeric(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                Number::Double(_) | Number::Nan { .. },
            )) => "double",
            Cache::Numeric(tcl_syntax::scalar_getter::NativeScalarCache::Number(Number::Big {
                ..
            })) => "bignum",
            Cache::WordBoolean { version, .. } => {
                if version >= TclVersion::V9_0 {
                    "boolean"
                } else {
                    "booleanString"
                }
            }
            _ => panic!("native scalar fixture primary"),
        }
    }
    fn storage(value: *mut TclObj) -> (bool, i64) {
        if obj::has_string_rep(value) {
            (true, obj::bytes_of(value).len() as i64)
        } else {
            (false, -1)
        }
    }
    fn failure(error: CmdError) -> (Vec<u8>, Vec<u8>) {
        assert!(error.native_access_refusal().is_none());
        let details = error.into_byte_details();
        let code = match details.error_code {
            CmdErrorCodeUpdate::Unchanged => b"SENTINEL".to_vec(),
            CmdErrorCodeUpdate::Set(bytes) => bytes,
            other => panic!("native primitive error-code action {other:?}"),
        };
        (details.message, code)
    }
    fn expected_code(options: &str, protocol: NativeStringProtocol) -> Vec<u8> {
        let data = bytes(options);
        let words = tcl_syntax::list::split_native_list_bytes(&data, protocol).unwrap();
        let index = words
            .iter()
            .position(|word| word.as_ref() == b"-errorcode")
            .unwrap();
        words[index + 1].to_vec()
    }

    #[test]
    #[cfg(have_tommath)]
    fn original_number_bignum_and_increment_match_all_192_native_windows() {
        // Native proof: naming.numeric.get-number-null
        // docs/design/analysis/name-resolution-proofs/numeric-get-number-null.md
        // Native proof: naming.numeric.get-bignum-interpreter
        // docs/design/analysis/name-resolution-proofs/numeric-get-bignum-interpreter.md
        // Native proof: naming.numeric.increment-original-object
        // docs/design/analysis/name-resolution-proofs/numeric-increment-original-object.md

        let fixtures = [
            (
                TclVersion::V8_5,
                include_str!(
                    "../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/8.5.19.tsv"
                ),
            ),
            (
                TclVersion::V8_6,
                include_str!(
                    "../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/8.6.18.tsv"
                ),
            ),
            (
                TclVersion::V9_0,
                include_str!(
                    "../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/9.0.4.tsv"
                ),
            ),
            (
                TclVersion::V9_1,
                include_str!(
                    "../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/9.1.0.tsv"
                ),
            ),
        ];
        let mut count = 0;
        for (version, fixture) in fixtures {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let protocol = NativeStringProtocol::C(version);
            let mut interpreter = crate::interp::Interp::new();
            interpreter.set_runtime_version(version);
            let ops = RuntimeIncrementObjects::selected(&interpreter).unwrap();
            for row in fixture.lines() {
                let cells: Vec<_> = row.split('\t').collect();
                let index: usize = cells[0].parse().unwrap();
                let stage: usize = cells[1].parse().unwrap();
                let value = shape(index, dialect);
                assert_eq!(
                    primary(value.as_ptr()),
                    cells[2],
                    "before {version:?} {row}"
                );
                assert_eq!(
                    storage(value.as_ptr()),
                    (cells[3] == "1", cells[4].parse().unwrap())
                );
                let mut number_type = -1;
                let outcome: Result<(), CmdError> = match stage {
                    0 => match crate::typed_value::native_number_probe(
                        value.as_ptr(),
                        dialect,
                        Kind::Number,
                    )
                    .unwrap()
                    {
                        Ok(number) => {
                            number_type = match number {
                                Number::Int(_) => {
                                    if version >= TclVersion::V9_0 {
                                        2
                                    } else {
                                        1
                                    }
                                }
                                Number::Big { .. } => 3,
                                Number::Double(value) if value.is_nan() => 5,
                                Number::Double(_) => 4,
                                Number::Nan { .. } => 5,
                            };
                            Ok(())
                        }
                        Err(_) => Err(CmdError::new("unpublished Number probe")),
                    },
                    1 => match crate::typed_value::native_number_probe(
                        value.as_ptr(),
                        dialect,
                        Kind::Bignum,
                    )
                    .unwrap()
                    {
                        Ok(_) => Ok(()),
                        Err(_) => Err(crate::typed_value::native_scalar_getter(
                            value.as_ptr(),
                            dialect,
                            Scalar::Wide,
                        )
                        .unwrap_err()
                        .into()),
                    },
                    2 => {
                        let amount = obj::Owned::fresh(obj::new_wide_int_obj(1));
                        tcl_cmd_core::native_increment::increment(
                            &ops,
                            Some(&RuntimeAppendValue::borrowed(value.as_ptr())),
                            &RuntimeAppendValue::borrowed(amount.as_ptr()),
                        )
                    }
                    .map(|_| ()),
                    _ => unreachable!("fixed native primitive"),
                };
                assert_eq!(
                    i32::from(outcome.is_err()),
                    cells[5].parse::<i32>().unwrap(),
                    "code {version:?} {row}"
                );
                assert_eq!(number_type, cells[6].parse::<i32>().unwrap());
                assert_eq!(primary(value.as_ptr()), cells[7], "after {version:?} {row}");
                assert_eq!(
                    storage(value.as_ptr()),
                    (cells[8] == "1", cells[9].parse().unwrap()),
                    "storage {version:?} {row}"
                );
                let actual = if stage == 0 {
                    (b"SEED".to_vec(), b"SENTINEL".to_vec())
                } else {
                    outcome.map_or_else(failure, |()| (b"SEED".to_vec(), b"SENTINEL".to_vec()))
                };
                assert_eq!(actual.0, bytes(cells[10]), "result {version:?} {row}");
                assert_eq!(
                    actual.1,
                    expected_code(cells[11], protocol),
                    "error code {version:?} {row}"
                );
                assert_eq!(
                    obj::bytes_of(value.as_ptr()).as_slice(),
                    bytes(cells[12]),
                    "object {version:?} {row}"
                );
                count += 1;
            }
        }
        assert_eq!(count, 192);
    }
}

#[test]
#[cfg(not(have_tommath))]
fn missing_bignum_backend_refuses_before_lossy_numeric_mutation() {
    use crate::obj;
    use tcl_syntax::scalar_getter::NativeNumberGetterKind as Kind;
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let original = obj::Owned::fresh(obj::new_string_bytes(b"184467440737095516160000"));
        for kind in [Kind::Number, Kind::Bignum] {
            assert_eq!(
                crate::typed_value::native_number_probe(original.as_ptr(), dialect, kind),
                Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)
            );
            assert!(obj::obj_type_ptr(original.as_ptr()).is_null());
            assert_eq!(
                obj::bytes_of(original.as_ptr()),
                b"184467440737095516160000"
            );
        }
        let mut interpreter = crate::interp::Interp::new();
        interpreter.set_runtime_version(version);
        let ops = crate::value_ops::RuntimeIncrementObjects::selected(&interpreter).unwrap();
        let current = obj::Owned::fresh(obj::new_wide_int_obj(i64::MAX));
        let amount = obj::Owned::fresh(obj::new_wide_int_obj(1));
        let result = tcl_cmd_core::native_increment::increment(
            &ops,
            Some(&crate::value_ops::RuntimeAppendValue::borrowed(
                current.as_ptr(),
            )),
            &crate::value_ops::RuntimeAppendValue::borrowed(amount.as_ptr()),
        );
        assert!(result.err().unwrap().native_access_refusal().is_some());
        assert_eq!(obj::wide_of(current.as_ptr()), i64::MAX);
        assert!(!obj::has_string_rep(current.as_ptr()));
        assert!(!obj::has_string_rep(amount.as_ptr()));
    }
}

#[cfg(test)]
mod currency {
    use crate::{
        interp::Interp,
        obj,
        value_ops::{
            RuntimeAppendValue, RuntimeIncrementObjects, RuntimeLegacyIncrementAmountOps,
            RuntimeLegacyIncrementObjects,
        },
    };
    use tcl_syntax::{
        number::Number,
        scalar_getter::{NativeLegacyIncrementRecipe, NativeScalarCache},
    };

    thread_local! {
        static UPDATER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    extern "C" fn changing_updater(value: *mut obj::TclObj) {
        UPDATER_CALLS.with(|calls| calls.set(calls.get() + 1));
        // SAFETY: the test's actual entered caller retains this interpreter
        // throughout the original updater; the clone borrows no engine state.
        let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        let original = interpreter.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("increment-updater".into(), "1.0".into())];
        interpreter.pin_context(&changed).unwrap();
        interpreter.pin_context(&original).unwrap();
        // SAFETY: the reached descriptor owns this live original header.
        unsafe { obj::set_native_updater_string_rep(value, b"17", false) };
    }

    static CHANGING_TYPE: obj::TclObjType = obj::TclObjType {
        name: c"originalIncrementCurrency".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(changing_updater),
        set_from_any_proc: None,
    };

    struct NoNumericHost(std::rc::Rc<dyn tcl_platform::Host>);
    impl tcl_platform::Host for NoNumericHost {
        fn capabilities(&self) -> tcl_platform::Capabilities {
            self.0.capabilities()
        }
        fn clock(&self) -> &dyn tcl_platform::Clock {
            self.0.clock()
        }
        fn stdio(&self) -> &dyn tcl_platform::StdIo {
            self.0.stdio()
        }
        fn env(&self) -> &dyn tcl_platform::Env {
            self.0.env()
        }
    }

    fn native(version: tcl_dialect::TclVersion) -> Interp {
        let mut interpreter = Interp::new();
        interpreter.set_runtime_version(version);
        interpreter
    }

    fn stale() -> tcl_runtime_api::NativeExecutionError {
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "stale entered native operation",
            ),
        )
    }

    fn leak_free(body: impl FnOnce()) {
        crate::counters::reset();
        body();
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }

    fn assert_reached_only(value: *mut obj::TclObj, references: obj::TclSize) {
        UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
        assert_eq!(obj::bytes_of(value), b"17");
        assert_eq!(obj::obj_type_ptr(value), &CHANGING_TYPE as *const _);
        assert!(obj::native_scalar_cache(value).unwrap().is_none());
        // SAFETY: the owning original remains held through these observations.
        assert_eq!(unsafe { (*value).ref_count }, references);
    }

    #[test]
    fn modern_increment_retains_reached_updater_and_stops_before_numeric_cache_or_store() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual software updater and entered currency, not native measurement.
        leak_free(|| {
            for amount_stage in [false, true] {
                let mut interpreter = native(tcl_dialect::TclVersion::V8_6);
                let original = obj::Owned::fresh(obj::alloc_typed(&CHANGING_TYPE, 0));
                let ordinary = obj::Owned::fresh(obj::new_wide_int_obj(1));
                let (current, amount) = if amount_stage {
                    (ordinary.as_ptr(), original.as_ptr())
                } else {
                    (original.as_ptr(), ordinary.as_ptr())
                };
                let context = interpreter.runtime_context();
                let references = unsafe { (*original.as_ptr()).ref_count };
                interpreter.set_result_bytes(b"PRIOR");
                let prior = interpreter.get_obj_result();
                let ops = RuntimeIncrementObjects::selected(&interpreter).unwrap();
                UPDATER_CALLS.with(|calls| calls.set(0));
                crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
                let error = tcl_cmd_core::native_increment::increment(
                    &ops,
                    Some(&RuntimeAppendValue::borrowed(current)),
                    &RuntimeAppendValue::borrowed(amount),
                )
                .err()
                .unwrap();
                assert_eq!(error.native_execution_refusal(), Some(&stale()));
                assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
                assert_eq!(interpreter.runtime_context(), context);
                assert_reached_only(original.as_ptr(), references);
                assert_eq!(
                    obj::native_scalar_cache(ordinary.as_ptr()).unwrap(),
                    Some(NativeScalarCache::Number(Number::Int(1)))
                );
                assert_eq!(unsafe { (*ordinary.as_ptr()).ref_count }, 1);
                assert_eq!(interpreter.get_obj_result(), prior);
                assert_eq!(interpreter.result_bytes(), b"PRIOR");
                crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
            }
        });
    }

    #[test]
    fn legacy_increment_amount_and_receiver_use_the_original_guarded_scalar_stages() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        leak_free(|| {
            for amount_stage in [false, true] {
                let mut interpreter = native(tcl_dialect::TclVersion::V8_4);
                let original = obj::Owned::fresh(obj::alloc_typed(&CHANGING_TYPE, 0));
                let references = unsafe { (*original.as_ptr()).ref_count };
                let context = interpreter.runtime_context();
                UPDATER_CALLS.with(|calls| calls.set(0));
                crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
                let error = if amount_stage {
                    tcl_cmd_core::native_increment::prepare_legacy_amount(
                        &mut RuntimeLegacyIncrementAmountOps(&mut interpreter),
                        NativeLegacyIncrementRecipe::Tcl84,
                        &original.as_ptr(),
                    )
                    .err()
                    .unwrap()
                } else {
                    let ops = RuntimeLegacyIncrementObjects::selected(&interpreter).unwrap();
                    tcl_cmd_core::native_increment::increment_legacy(
                        &ops,
                        Some(&RuntimeAppendValue::borrowed(original.as_ptr())),
                        tcl_cmd_core::native_increment::default_legacy_amount(
                            NativeLegacyIncrementRecipe::Tcl84,
                        ),
                    )
                    .err()
                    .unwrap()
                };
                assert_eq!(error.native_execution_refusal(), Some(&stale()));
                assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
                assert_eq!(interpreter.runtime_context(), context);
                assert_reached_only(original.as_ptr(), references);
                crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
            }
        });
    }

    #[test]
    fn legacy_increment_keeps_supported_cache_without_borrowing_missing_fresh_environment() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        leak_free(|| {
            let mut interpreter = native(tcl_dialect::TclVersion::V8_4);
            interpreter.set_host(std::rc::Rc::new(NoNumericHost(interpreter.host())));
            assert!(interpreter.host().numeric_environment().is_none());
            let cached = obj::Owned::fresh(obj::new_wide_int_obj(17));
            let fresh = obj::Owned::fresh(obj::new_string_bytes(b"17"));
            let shared = cached.clone();
            let cached_before = obj::native_object_snapshot(cached.as_ptr()).unwrap();
            let fresh_before = obj::native_object_snapshot(fresh.as_ptr()).unwrap();
            let prepared = tcl_cmd_core::native_increment::prepare_legacy_amount(
                &mut RuntimeLegacyIncrementAmountOps(&mut interpreter),
                NativeLegacyIncrementRecipe::Tcl84,
                &cached.as_ptr(),
            )
            .unwrap();
            let ops = RuntimeLegacyIncrementObjects::selected(&interpreter).unwrap();
            let result = tcl_cmd_core::native_increment::increment_legacy(
                &ops,
                Some(&RuntimeAppendValue::borrowed(cached.as_ptr())),
                prepared,
            )
            .unwrap();
            assert_eq!(
                obj::native_scalar_cache(result.as_ptr()).unwrap(),
                Some(NativeScalarCache::Number(Number::Int(34)))
            );
            drop(result);
            assert_eq!(
                obj::native_object_snapshot(cached.as_ptr()).unwrap(),
                cached_before
            );
            let error = tcl_cmd_core::native_increment::prepare_legacy_amount(
                &mut RuntimeLegacyIncrementAmountOps(&mut interpreter),
                NativeLegacyIncrementRecipe::Tcl84,
                &fresh.as_ptr(),
            )
            .err()
            .unwrap();
            assert!(error.native_access_refusal().is_some());
            assert_eq!(
                obj::native_object_snapshot(fresh.as_ptr()).unwrap(),
                fresh_before
            );
            assert!(interpreter.native_execution_refusal().is_none());
            drop(shared);
        });
    }

    #[test]
    fn original_increment_keeps_real_copy_on_write_and_pending_first_host() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        leak_free(|| {
            for shared in [false, true] {
                let mut interpreter = native(tcl_dialect::TclVersion::V8_6);
                let original = obj::Owned::fresh(obj::new_string_bytes(b"17"));
                let other = shared.then(|| original.clone());
                let amount = obj::Owned::fresh(obj::new_wide_int_obj(1));
                let ops = RuntimeIncrementObjects::selected(&interpreter).unwrap();
                let result = tcl_cmd_core::native_increment::increment(
                    &ops,
                    Some(&RuntimeAppendValue::borrowed(original.as_ptr())),
                    &RuntimeAppendValue::borrowed(amount.as_ptr()),
                )
                .unwrap();
                assert_eq!(result.as_ptr() == original.as_ptr(), !shared);
                assert_eq!(
                    obj::native_scalar_cache(result.as_ptr()).unwrap(),
                    Some(NativeScalarCache::Number(Number::Int(18)))
                );
                assert!(!obj::has_string_rep(result.as_ptr()));
                if shared {
                    assert_eq!(obj::bytes_of(original.as_ptr()), b"17");
                    assert!(obj::native_scalar_cache(original.as_ptr())
                        .unwrap()
                        .is_none());
                }
                drop(result);
                assert_eq!(
                    unsafe { (*original.as_ptr()).ref_count },
                    if shared { 2 } else { 1 }
                );
                interpreter.refuse_host_command("original increment first refusal");
                let first = interpreter.native_execution_refusal().unwrap();
                let before = obj::native_object_snapshot(original.as_ptr()).unwrap();
                let error = tcl_cmd_core::native_increment::increment(
                    &ops,
                    Some(&RuntimeAppendValue::borrowed(original.as_ptr())),
                    &RuntimeAppendValue::borrowed(amount.as_ptr()),
                )
                .err()
                .unwrap();
                assert_eq!(error.native_execution_refusal(), Some(&first));
                assert_eq!(interpreter.native_execution_refusal(), Some(first));
                assert_eq!(
                    obj::native_object_snapshot(original.as_ptr()).unwrap(),
                    before
                );
                drop(other);
            }
        });
    }
}
