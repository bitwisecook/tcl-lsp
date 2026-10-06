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
            let ops = RuntimeIncrementObjects::selected(dialect).unwrap();
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
        let ops = crate::value_ops::RuntimeIncrementObjects::selected(dialect).unwrap();
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
