// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed full Number/Bignum and actual increment observations from C Tcl.

use crate::value::Value;
use crate::value_ops::VmIncrementObjects;
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
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn shape(index: usize, dialect: tcl_registry::InvocationDialect) -> Value {
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
        return Value::new_native_string_bytes(texts[index]);
    }
    match index {
        10 => Value::double(1.5),
        11 => {
            let value = Value::new_native_string_bytes(b"NaN".as_slice());
            let _ = value.native_scalar_getter(dialect, Scalar::Double);
            value
        }
        12 => Value::int(7),
        13 => {
            let value = Value::new_native_string_bytes(b"true".as_slice());
            value
                .native_scalar_getter(dialect, Scalar::Boolean)
                .unwrap();
            value
        }
        14 | 15 => {
            let value = Value::new_native_string_bytes(texts[4]);
            value
                .native_number_probe(dialect, Kind::Bignum)
                .unwrap()
                .unwrap();
            if index == 15 {
                value.with_resident_string_bytes(b"".as_slice().into())
            } else {
                value
            }
        }
        _ => unreachable!("fixed native shape"),
    }
}
fn primary(value: &Value) -> &'static str {
    match value.native_object_snapshot().cache {
        Cache::None => "none",
        Cache::Numeric(tcl_syntax::scalar_getter::NativeScalarCache::Number(Number::Int(_))) => {
            "int"
        }
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
fn storage(value: &Value) -> (bool, i64) {
    value.resident_string_bytes().map_or((false, -1), |bytes| {
        (true, i64::try_from(bytes.len()).unwrap())
    })
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
fn original_number_bignum_and_increment_match_all_192_native_windows() {
    let fixtures = [
        (
            TclVersion::V8_5,
            include_str!("../../tcl-syntax/tests/data/native_scalar_getters/number/8.5.19.tsv"),
        ),
        (
            TclVersion::V8_6,
            include_str!("../../tcl-syntax/tests/data/native_scalar_getters/number/8.6.18.tsv"),
        ),
        (
            TclVersion::V9_0,
            include_str!("../../tcl-syntax/tests/data/native_scalar_getters/number/9.0.4.tsv"),
        ),
        (
            TclVersion::V9_1,
            include_str!("../../tcl-syntax/tests/data/native_scalar_getters/number/9.1.0.tsv"),
        ),
    ];
    let mut count = 0;
    for (version, fixture) in fixtures {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let protocol = NativeStringProtocol::C(version);
        let ops = VmIncrementObjects::selected(dialect).unwrap();
        for row in fixture.lines() {
            let cells: Vec<_> = row.split('\t').collect();
            let index: usize = cells[0].parse().unwrap();
            let stage: usize = cells[1].parse().unwrap();
            let value = shape(index, dialect);
            assert_eq!(primary(&value), cells[2], "before {version:?} {row}");
            assert_eq!(
                storage(&value),
                (cells[3] == "1", cells[4].parse().unwrap())
            );
            let mut number_type = -1;
            let outcome: Result<(), CmdError> = match stage {
                0 => match value.native_number_probe(dialect, Kind::Number).unwrap() {
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
                1 => match value.native_number_probe(dialect, Kind::Bignum).unwrap() {
                    Ok(_) => Ok(()),
                    Err(_) => Err(value
                        .native_scalar_getter(dialect, Scalar::Wide)
                        .unwrap_err()
                        .into()),
                },
                2 => tcl_cmd_core::native_increment::increment(&ops, Some(&value), &Value::int(1))
                    .map(|_| ()),
                _ => unreachable!("fixed native primitive"),
            };
            assert_eq!(
                i32::from(outcome.is_err()),
                cells[5].parse::<i32>().unwrap(),
                "code {version:?} {row}"
            );
            assert_eq!(number_type, cells[6].parse::<i32>().unwrap());
            assert_eq!(primary(&value), cells[7], "after {version:?} {row}");
            assert_eq!(
                storage(&value),
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
                value.native_string_bytes(protocol).unwrap().as_ref(),
                bytes(cells[12]),
                "object {version:?} {row}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 192);
}
