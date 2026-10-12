// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native append physical windows shared with the VM adapter.
use crate::{
    cmd_string,
    interp::Interp,
    obj::{self, Owned, TclObj},
};
use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;

fn native_type(value: *mut TclObj) -> &'static str {
    match obj::native_object_snapshot(value).unwrap().cache {
        Cache::None => "none",
        Cache::String { .. } | Cache::JimString { .. } => "string",
        Cache::ByteArray { .. } => "bytearray",
        Cache::Numeric(tcl_syntax::scalar_getter::NativeScalarCache::Number(
            tcl_syntax::number::Number::Int(_),
        )) => "int",
        Cache::Numeric(tcl_syntax::scalar_getter::NativeScalarCache::Number(
            tcl_syntax::number::Number::Double(_),
        )) => "double",
        Cache::List { .. } => "list",
        Cache::Dictionary { .. } => "dict",
        _ => panic!("fixture primary type"),
    }
}
fn make(interp: &mut Interp, kind: &str) -> Owned {
    use tcl_syntax::value::ValueOps;
    Owned::fresh(match kind {
        "empty-string" => obj::new_string_bytes(b""),
        "text-string" => obj::new_string_bytes(b"TEXT"),
        "integer" => obj::new_wide_int_obj(5),
        "double" => obj::new_double_obj(1.5),
        "empty-list" => interp.new_list(Vec::new()),
        "typed-list" => {
            let a = Owned::fresh(obj::new_string_bytes(b"alpha"));
            let b = Owned::fresh(obj::new_string_bytes(b"beta gamma"));
            interp.new_list(vec![a.as_ptr(), b.as_ptr()])
        }
        "pure-empty-ba" => interp.new_native_byte_array(b"").unwrap(),
        "pure-ba" => interp.new_native_byte_array(&[0xff, 0]).unwrap(),
        "resident-ba" => {
            let value = interp.new_native_byte_array(&[0xff, 0]).unwrap();
            obj::bytes_of(value);
            value
        }
        _ => panic!("fixture constructor {kind}"),
    })
}
fn hex(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "-".into();
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn native_append_preserves_every_measured_primary_storage_and_identity_window() {
    // Native proof: naming.append.original-storage-and-identity
    // docs/design/analysis/name-resolution-proofs/append-original-storage-and-identity.md
    let fixtures = [
        (
            "tcl8.4",
            include_str!("../../../rust/tcl-syntax/tests/data/native_object_append/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../rust/tcl-syntax/tests/data/native_object_append/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../rust/tcl-syntax/tests/data/native_object_append/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../rust/tcl-syntax/tests/data/native_object_append/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../rust/tcl-syntax/tests/data/native_object_append/9.1.0.tsv"),
        ),
        (
            "jim",
            include_str!("../../../rust/tcl-syntax/tests/data/native_object_append/jim.tsv"),
        ),
    ];
    let mut observations = 0;
    for (profile, fixture) in fixtures {
        for row in fixture.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 17);
            let label = format!("{profile} {} {}", fields[0], fields[1]);
            let kinds = fields[0]
                .strip_prefix("dest-")
                .unwrap()
                .split_once("-source-")
                .unwrap();
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect(profile));
            if kinds.0 != "missing" {
                let destination = make(&mut interp, kinds.0);
                interp.var_set(b"v", destination.as_ptr()).unwrap();
                if fields[1] == "shared-destination" {
                    interp.var_set(b"alias", destination.as_ptr()).unwrap();
                }
            }
            let source = if fields[1] == "self-alias" {
                Owned::retain(interp.var_get(b"v").unwrap())
            } else {
                make(&mut interp, kinds.1)
            };
            assert_eq!(
                native_type(source.as_ptr()),
                fields[2],
                "{label} source before"
            );
            assert_eq!(
                usize::from(obj::has_string_rep(source.as_ptr())).to_string(),
                fields[3],
                "{label} source storage before"
            );
            let before = interp.var_get(b"v");
            assert_eq!(
                before.map_or("missing", native_type),
                fields[4],
                "{label} receiver before"
            );
            assert_eq!(
                usize::from(before.is_some_and(obj::has_string_rep)).to_string(),
                fields[5],
                "{label} receiver storage before"
            );
            let command = Owned::fresh(obj::new_string_bytes(b"append"));
            let name = Owned::fresh(obj::new_string_bytes(b"v"));
            let argument = source.clone();
            let code = cmd_string::append(
                &mut interp,
                &[command.as_ptr(), name.as_ptr(), argument.as_ptr()],
            );
            assert_eq!(
                code,
                crate::interp::Code::Ok,
                "{label}: {:?}",
                interp.result_bytes()
            );
            let receiver = interp.var_get(b"v").unwrap();
            let alias = interp.var_get(b"alias");
            let result = interp.get_obj_result();
            assert_eq!(
                native_type(source.as_ptr()),
                fields[6],
                "{label} source after"
            );
            assert_eq!(
                usize::from(obj::has_string_rep(source.as_ptr())).to_string(),
                fields[7],
                "{label} source storage after"
            );
            assert_eq!(native_type(receiver), fields[8], "{label} receiver after");
            assert_eq!(
                usize::from(obj::has_string_rep(receiver)).to_string(),
                fields[9],
                "{label} receiver storage after"
            );
            let identity = [
                source.as_ptr() == receiver,
                alias == Some(receiver),
                alias == Some(source.as_ptr()),
                result == receiver,
                result == source.as_ptr(),
            ];
            for (actual, expected) in identity.into_iter().zip(&fields[10..15]) {
                assert_eq!(
                    usize::from(actual).to_string(),
                    *expected,
                    "{label} original identity"
                );
            }
            assert_eq!(fields[15], "0", "{label} native append completion");
            assert_eq!(
                hex(&obj::bytes_of(result)),
                fields[16],
                "{label} reached result projection"
            );
            observations += 1;
        }
    }
    assert_eq!(observations, 1035);
}
const CAT_INPUT_KINDS: &[[i32; 3]] = &[
    [1, 1, -1],
    [3, 3, -1],
    [3, 1, -1],
    [4, 3, -1],
    [5, 1, -1],
    [1, 5, -1],
    [1, 2, -1],
    [6, 2, -1],
    [5, 3, -1],
    [7, 1, -1],
    [9, 1, -1],
    [8, 1, -1],
    [0, 5, 0],
    [5, 0, 0],
    [0, 0, 6],
    [0, 0, 0],
    [10, 1, -1],
    [1, 10, -1],
    [6, 0, 0],
    [8, 0, 0],
    [9, 0, 0],
    [0, 1, 0],
    [1, 0, 1],
];
fn assert_cat_snapshot(snapshot: tcl_syntax::native_object::NativeObjectSnapshot, row: &[&str]) {
    assert_eq!(usize::from(snapshot.resident.is_some()).to_string(), row[5]);
    assert_eq!(
        snapshot
            .resident
            .as_ref()
            .map_or(-1, |bytes| bytes.len() as i64)
            .to_string(),
        row[7]
    );
    if row[4] == "string" {
        let Cache::String {
            num_chars, unicode, ..
        } = snapshot.cache
        else {
            panic!("native String cache");
        };
        assert_eq!(
            num_chars.map_or(-1, |count| count as i64).to_string(),
            row[8]
        );
        assert_eq!(usize::from(unicode.is_some()).to_string(), row[9]);
        let units = unicode.map_or_else(
            || "-".into(),
            |units| {
                units
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            },
        );
        assert_eq!(units, row[10]);
    }
}

const CAT_BYTE_INPUT_KINDS: &[[i32; 3]] = &[
    [11, 1, -1],
    [1, 11, -1],
    [12, 1, -1],
    [1, 12, -1],
    [12, 2, -1],
    [11, 2, -1],
    [13, 1, -1],
    [15, 14, -1],
    [14, 15, -1],
    [13, 2, -1],
    [15, 2, -1],
    [0, 11, 0],
];

fn make_cat(interp: &mut Interp, kind: i32) -> Owned {
    use tcl_syntax::value::ValueOps;
    Owned::fresh(match kind {
        0 | 7 => interp.new_list(Vec::new()),
        1 => obj::new_string_bytes(b"A"),
        2 => obj::new_string_bytes(&[0x80, b'B']),
        3 | 4 => {
            let value = interp.new_native_byte_array(&[0xff, 0]).unwrap();
            if kind == 4 {
                obj::bytes_of(value);
            }
            value
        }
        5 => obj::new_native_unicode_obj(
            std::rc::Rc::from(&[233, 128512][..]),
            interp.native_invocation_dialect(),
        )
        .unwrap(),
        6 => obj::new_wide_int_obj(7),
        8 => {
            let child = Owned::fresh(obj::new_wide_int_obj(7));
            interp.new_list(vec![child.as_ptr()])
        }
        9 => crate::dict::new_dict_obj(&[]),
        10 => {
            let value = obj::new_string_bytes(b"A");
            obj::native_unicode_units(
                value,
                interp
                    .native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap(),
            )
            .unwrap();
            value
        }
        11 => obj::new_string_bytes(b"A\0Z"),
        12 => obj::new_string_bytes(b"\xff"),
        13..=15 => {
            let units: &[u32] = match kind {
                13 => &[0xd800, 0xdc00],
                14 => &[0xdc00],
                15 => &[0xd800],
                _ => unreachable!(),
            };
            obj::new_native_unicode_obj(
                std::rc::Rc::from(units),
                interp.native_invocation_dialect(),
            )
            .unwrap()
        }
        _ => panic!("fixed native Cat constructor"),
    })
}
fn compare_cat_windows(fixtures: [(&str, &str); 2], kinds: &[[i32; 3]]) -> usize {
    use crate::value_ops::{RuntimeAppendObjects, RuntimeAppendValue};
    let mut observations = 0;
    for (profile, fixture) in fixtures {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(profile));
        let dialect = interp.native_invocation_dialect();
        let protocol = dialect.native_object_cat_protocol().unwrap().recipe();
        let objects = RuntimeAppendObjects {
            dialect,
            binary_recipe: dialect.byte_array_string_recipe(None),
        };
        let mut rows = fixture
            .lines()
            .map(|line| line.split('\t').collect::<Vec<_>>())
            .peekable();
        while let Some(row) = rows.peek() {
            let case = row[0].parse::<usize>().unwrap();
            let sharing = row[1] == "1";
            let inputs: Vec<_> = kinds[case]
                .iter()
                .filter(|kind| **kind >= 0)
                .map(|kind| make_cat(&mut interp, *kind))
                .collect();
            let alias = sharing.then(|| inputs[0].clone());
            let borrowed: Vec<_> = inputs
                .iter()
                .map(|value| RuntimeAppendValue::borrowed(value.as_ptr()))
                .collect();
            let mut result = None;
            while rows.peek().is_some_and(|row| {
                row[0].parse::<usize>().unwrap() == case && (row[1] == "1") == sharing
            }) {
                let row = rows.next().unwrap();
                observations += 1;
                assert_eq!(row.len(), 13);
                if row[2] != "before" && result.is_none() {
                    result = Some(
                        tcl_cmd_core::native_cat::concatenate(&objects, protocol, &borrowed, true)
                            .unwrap(),
                    );
                }
                let value = match row[2] {
                    "before" | "after-before-result-string" => {
                        inputs[row[3].parse::<usize>().unwrap()].as_ptr()
                    }
                    "result-before-string" | "projection" => result.as_ref().unwrap().as_ptr(),
                    _ => panic!("original observation window"),
                };
                if row[2] == "projection" {
                    let identity = inputs
                        .iter()
                        .map(|input| usize::from(input.as_ptr() == value).to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    assert_eq!(identity, row[11], "{profile} case {case} shared {sharing}");
                    assert_eq!(hex(&obj::bytes_of(value)), row[12]);
                } else {
                    assert_eq!(native_type(value), row[4], "{profile} case {case} {row:?}");
                    assert_cat_snapshot(obj::native_object_snapshot(value).unwrap(), &row);
                    let receipt_reference = usize::from(
                        result
                            .as_ref()
                            .is_some_and(|result| result.as_ptr() == value),
                    );
                    assert_eq!(
                        unsafe { (*value).ref_count } as usize,
                        row[6].parse::<usize>().unwrap() + receipt_reference,
                        "known returned-owner reference"
                    );
                }
            }
            drop(alias);
        }
    }
    observations
}

#[test]
fn native_cat_preserves_all_original_cache_and_identity_windows() {
    // Native proof: naming.cat.original-cache-and-sharing
    // docs/design/analysis/name-resolution-proofs/cat-original-cache-and-sharing.md
    let fixtures = [
        (
            "tcl9.0",
            include_str!("../../../rust/tcl-syntax/tests/data/native_object_cat/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../rust/tcl-syntax/tests/data/native_object_cat/9.1.0.tsv"),
        ),
    ];
    assert_eq!(compare_cat_windows(fixtures, CAT_INPUT_KINDS), 624);
}

#[test]
fn native_cat_preserves_raw_nul_invalid_bytes_and_surrogate_windows() {
    // Native proof: naming.cat.original-counted-bytes-and-surrogates
    // docs/design/analysis/name-resolution-proofs/cat-original-counted-bytes-and-surrogates.md
    let fixtures = [
        (
            "tcl9.0",
            include_str!("../../../rust/tcl-syntax/tests/data/native_object_cat/bytes/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../rust/tcl-syntax/tests/data/native_object_cat/bytes/9.1.0.tsv"),
        ),
    ];
    assert_eq!(compare_cat_windows(fixtures, CAT_BYTE_INPUT_KINDS), 296);
}
