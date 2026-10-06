// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed original-object observations from actual C Tcl and pinned Jim.

use crate::{interp::Vm, value::Value};
use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
use tcl_syntax::native_string::NativeStringProtocol;

fn native_type(value: &Value) -> &'static str {
    match value.native_object_snapshot().cache {
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

fn make(kind: &str, protocol: NativeStringProtocol) -> Value {
    match kind {
        "empty-string" => Value::new_native_string_bytes(&b""[..]),
        "text-string" => Value::new_native_string_bytes(&b"TEXT"[..]),
        "integer" => Value::int(5),
        "double" => Value::double(1.5),
        "empty-list" => Value::native_list_constructor(Vec::new(), protocol),
        "typed-list" => Value::native_list_constructor(
            vec![
                Value::new_native_string_bytes(&b"alpha"[..]),
                Value::new_native_string_bytes(&b"beta gamma"[..]),
            ],
            protocol,
        ),
        "pure-empty-ba" => Value::byte_array(&b""[..]),
        "pure-ba" => Value::byte_array(&[0xff, 0][..]),
        "resident-ba" => {
            let value = Value::byte_array(&[0xff, 0][..]);
            value.native_string_bytes(protocol).unwrap();
            value
        }
        _ => panic!("fixture constructor {kind}"),
    }
}

fn hex(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "-".into();
    }
    bytes.iter().fold(String::new(), |mut output, byte| {
        use std::fmt::Write as _;
        write!(output, "{byte:02x}").unwrap();
        output
    })
}

const APPEND_FIXTURES: [(&str, &str); 6] = [
    (
        "tcl8.4",
        include_str!("../../tcl-syntax/tests/data/native_object_append/8.4.20.tsv"),
    ),
    (
        "tcl8.5",
        include_str!("../../tcl-syntax/tests/data/native_object_append/8.5.19.tsv"),
    ),
    (
        "tcl8.6",
        include_str!("../../tcl-syntax/tests/data/native_object_append/8.6.18.tsv"),
    ),
    (
        "tcl9.0",
        include_str!("../../tcl-syntax/tests/data/native_object_append/9.0.4.tsv"),
    ),
    (
        "tcl9.1",
        include_str!("../../tcl-syntax/tests/data/native_object_append/9.1.0.tsv"),
    ),
    (
        "jim",
        include_str!("../../tcl-syntax/tests/data/native_object_append/jim.tsv"),
    ),
];

fn append_vm(profile: &str) -> (Vm, NativeStringProtocol) {
    let mut vm = Vm::new();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment(profile).unit_profile(),
    );
    let protocol = vm
        .native_scalar_carrier_dialect()
        .native_string_protocol()
        .unwrap();
    (vm, protocol)
}

#[test]
fn native_append_preserves_every_measured_primary_storage_and_identity_window() {
    let mut observations = 0;
    for (profile, fixture) in APPEND_FIXTURES {
        for row in fixture.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 17);
            let label = format!("{profile} {} {}", fields[0], fields[1]);
            let kinds = fields[0]
                .strip_prefix("dest-")
                .unwrap()
                .split_once("-source-")
                .unwrap();
            let (mut vm, protocol) = append_vm(profile);
            if kinds.0 != "missing" {
                vm.set_var_bytes(b"v", make(kinds.0, protocol)).unwrap();
                if fields[1] == "shared-destination" {
                    let alias = vm.get_var_bytes(b"v").unwrap();
                    vm.set_var_bytes(b"alias", alias).unwrap();
                }
            }
            let source = if fields[1] == "self-alias" {
                vm.get_var_bytes(b"v").unwrap()
            } else {
                make(kinds.1, protocol)
            };
            assert_eq!(native_type(&source), fields[2], "{label} source before");
            assert_eq!(
                usize::from(source.resident_string_bytes().is_some()).to_string(),
                fields[3],
                "{label} source storage before"
            );
            {
                let before = vm.get_var_bytes(b"v");
                assert_eq!(
                    before.as_ref().map_or("missing", native_type),
                    fields[4],
                    "{label} receiver before"
                );
                assert_eq!(
                    usize::from(
                        before.is_some_and(|value| value.resident_string_bytes().is_some())
                    )
                    .to_string(),
                    fields[5],
                    "{label} receiver storage before"
                );
            }
            // One actual argv reference, in addition to the original source owner.
            let argv = [source.clone()];
            let result = vm
                .append_captured_bytes(b"v", None, &argv)
                .unwrap_or_else(|completion| panic!("{label}: {:?}", completion.code));
            let receiver = vm.get_var_bytes(b"v").unwrap();
            let alias = vm.get_var_bytes(b"alias");
            assert_eq!(native_type(&source), fields[6], "{label} source after");
            assert_eq!(
                usize::from(source.resident_string_bytes().is_some()).to_string(),
                fields[7],
                "{label} source storage after"
            );
            assert_eq!(native_type(&receiver), fields[8], "{label} receiver after");
            assert_eq!(
                usize::from(receiver.resident_string_bytes().is_some()).to_string(),
                fields[9],
                "{label} receiver storage after"
            );
            let identity = [
                source.is_same_object(&receiver),
                alias
                    .as_ref()
                    .is_some_and(|value| value.is_same_object(&receiver)),
                alias
                    .as_ref()
                    .is_some_and(|value| value.is_same_object(&source)),
                result.is_same_object(&receiver),
                result.is_same_object(&source),
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
                hex(&result.native_string_bytes(protocol).unwrap()),
                fields[16],
                "{label} reached result projection"
            );
            observations += 1;
        }
    }
    assert_eq!(observations, 1035);
}

#[test]
fn jim_batch_receipt_retains_one_prepared_receiver_across_original_operands() {
    let mut vm = Vm::new();
    vm.set_dialect_profile(tcl_registry::model::ingress::resolve_environment("jim").unit_profile());
    vm.set_var_bytes(b"v", Value::new_native_string_bytes(&b"BASE"[..]))
        .unwrap();
    let original = vm.get_var_bytes(b"v").unwrap().native_object_identity();
    let result = vm
        .append_captured_bytes(b"v", None, &[Value::int(1), Value::int(2), Value::int(3)])
        .unwrap();
    assert_eq!(result.native_object_identity(), original);
    assert_eq!(result.string_bytes().as_ref(), b"BASE123");
    assert!(matches!(
        result.native_object_snapshot().cache,
        Cache::JimString { num_chars: None }
    ));
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
            .map_or(-1, |bytes| i64::try_from(bytes.len()).unwrap())
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
            num_chars
                .map_or(-1, |count| i64::try_from(count).unwrap())
                .to_string(),
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

fn make_cat(kind: i32, dialect: tcl_registry::InvocationDialect) -> Value {
    let protocol = dialect.native_string_protocol().unwrap();
    match kind {
        0 | 7 => Value::native_list_constructor(Vec::new(), protocol),
        1 => Value::new_native_string_bytes(&b"A"[..]),
        2 => Value::new_native_string_bytes(&[0x80, b'B'][..]),
        3 | 4 => {
            let value =
                Value::from_native_byte_array(std::rc::Rc::from(&[0xff, 0][..]), dialect).unwrap();
            if kind == 4 {
                value.native_string_bytes(protocol).unwrap();
            }
            value
        }
        5 => Value::from_native_unicode_units(std::rc::Rc::from(&[233, 128_512][..]), dialect)
            .unwrap(),
        6 => Value::int(7),
        8 => Value::native_list_constructor(vec![Value::int(7)], protocol),
        9 => Value::dict(Vec::new()),
        10 => {
            let value = Value::new_native_string_bytes(&b"A"[..]);
            value.native_unicode_units(protocol).unwrap();
            value
        }
        11 => Value::new_native_string_bytes(&b"A\0Z"[..]),
        12 => Value::new_native_string_bytes(&b"\xff"[..]),
        13..=15 => {
            let units: &[u32] = match kind {
                13 => &[0xd800, 0xdc00],
                14 => &[0xdc00],
                15 => &[0xd800],
                _ => unreachable!(),
            };
            Value::from_native_unicode_units(std::rc::Rc::from(units), dialect).unwrap()
        }
        _ => panic!("fixed native Cat constructor"),
    }
}
fn compare_cat_windows(fixtures: [(&str, &str); 2], kinds: &[[i32; 3]]) -> usize {
    let mut observations = 0;
    for (profile, fixture) in fixtures {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment(profile).unit_profile(),
        );
        let dialect = vm.native_scalar_carrier_dialect();
        let protocol = dialect.native_object_cat_protocol().unwrap().recipe();
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
                .map(|kind| make_cat(*kind, dialect))
                .collect();
            let alias = sharing.then(|| inputs[0].clone());
            let mut result = None;
            while rows.peek().is_some_and(|row| {
                row[0].parse::<usize>().unwrap() == case && (row[1] == "1") == sharing
            }) {
                let row = rows.next().unwrap();
                observations += 1;
                assert_eq!(row.len(), 13);
                if row[2] != "before" && result.is_none() {
                    result = Some(
                        tcl_cmd_core::native_cat::concatenate(
                            &super::VmAppendObjects,
                            protocol,
                            &inputs,
                            true,
                        )
                        .unwrap(),
                    );
                }
                let value = match row[2] {
                    "before" | "after-before-result-string" => {
                        &inputs[row[3].parse::<usize>().unwrap()]
                    }
                    "result-before-string" | "projection" => result.as_ref().unwrap(),
                    _ => panic!("original observation window"),
                };
                if row[2] == "projection" {
                    let identity = inputs
                        .iter()
                        .map(|input| {
                            usize::from(
                                input.native_object_identity() == value.native_object_identity(),
                            )
                            .to_string()
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    assert_eq!(identity, row[11], "{profile} case {case} shared {sharing}");
                    assert_eq!(
                        hex(&value
                            .native_string_bytes(protocol.string_protocol())
                            .unwrap()),
                        row[12]
                    );
                } else {
                    assert_eq!(native_type(value), row[4], "{profile} case {case} {row:?}");
                    assert_cat_snapshot(value.native_object_snapshot(), &row);
                    let receipt_reference = usize::from(result.as_ref().is_some_and(|result| {
                        result.native_object_identity() == value.native_object_identity()
                    }));
                    assert_eq!(
                        std::rc::Rc::strong_count(&value.0),
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
fn native_cache_carriers_preserve_original_aliases_and_owned_table_lifetime() {
    struct Table(std::cell::RefCell<std::rc::Rc<[u8]>>);
    impl tcl_core_types::NativeIndexTable for Table {
        fn identity(&self) -> usize {
            std::ptr::from_ref(self) as usize
        }
        fn word(
            &self,
            index: usize,
            stride: usize,
        ) -> Result<std::rc::Rc<[u8]>, tcl_core_types::NativeIndexUnavailable> {
            if index == 1 && stride == 8 {
                Ok(self.0.borrow().clone())
            } else {
                Err(tcl_core_types::NativeIndexUnavailable)
            }
        }
    }
    let mut vm = Vm::new();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile(),
    );
    let dialect = vm.native_scalar_carrier_dialect();
    let table = std::rc::Rc::new(Table(std::cell::RefCell::new(std::rc::Rc::from(
        &[0, 0xc0, 0x80, 0xff][..],
    ))));
    let cache = tcl_core_types::NativeIndexCache::new(table.clone(), 8, 1);
    let index = Value::from_native_index_cache(cache, tcl_dialect::TclVersion::V9_0, dialect, None)
        .unwrap();
    let original = Value::new_native_string_bytes(&b"old"[..]);
    let alias = original.clone();
    original
        .adopt_native_object_representation(&index, dialect)
        .unwrap();
    assert_eq!(alias.native_index_cache().unwrap().0.index(), 1);
    *table.0.borrow_mut() = std::rc::Rc::from(&[0xff, 0, 0xc0, 0x80][..]);
    drop(table);
    assert_eq!(
        &*alias
            .native_string_bytes(dialect.native_string_protocol().unwrap())
            .unwrap(),
        &[0xff, 0, 0xc0, 0x80]
    );
    assert_eq!(
        original.native_object_identity(),
        alias.native_object_identity()
    );
    vm.set_dialect_profile(tcl_registry::model::ingress::resolve_environment("jim").unit_profile());
    assert!(
        original
            .adopt_native_object_representation(&index, vm.native_scalar_carrier_dialect())
            .is_err()
    );
}

#[test]
fn authenticated_string_mutation_retains_only_the_preserved_allocation() {
    use tcl_core_types::ResidentStringMutation as Mutation;
    let mut vm = Vm::new();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile(),
    );
    let dialect = vm.native_scalar_carrier_dialect();
    let original = Value::new_native_string_bytes(&[b'1', 0, 0xc0, 0x80, 0xff][..]);
    let old_bytes = original.resident_string_bytes().unwrap();
    let donor = Value::int(1).with_resident_string_bytes(old_bytes.as_ref().into());
    // Imported storage is unknown, so use an imported original with the same receipt.
    let original = original.with_resident_string_bytes(old_bytes.clone());
    let alias = original.clone();
    original
        .adopt_native_object_representation_with_string_mutation(
            &donor,
            dialect,
            Mutation::Preserve,
        )
        .unwrap();
    assert!(std::rc::Rc::ptr_eq(
        &old_bytes,
        &alias.resident_string_bytes().unwrap()
    ));
    assert_eq!(alias.native_scalar_cache(), donor.native_scalar_cache());
    let equal_replacement = Value::int(2).with_resident_string_bytes(old_bytes.as_ref().into());
    let replacement_bytes = equal_replacement.resident_string_bytes().unwrap();
    assert!(!std::rc::Rc::ptr_eq(&old_bytes, &replacement_bytes));
    original
        .adopt_native_object_representation_with_string_mutation(
            &equal_replacement,
            dialect,
            Mutation::Replace,
        )
        .unwrap();
    assert!(std::rc::Rc::ptr_eq(
        &replacement_bytes,
        &alias.resident_string_bytes().unwrap()
    ));
    let invalid = Value::new_native_string_bytes(&b"different"[..]);
    let prior_cache = original.native_scalar_cache();
    assert!(
        original
            .adopt_native_object_representation_with_string_mutation(
                &invalid,
                dialect,
                Mutation::Preserve,
            )
            .is_err()
    );
    assert_eq!(original.native_scalar_cache(), prior_cache);
    assert!(std::rc::Rc::ptr_eq(
        &replacement_bytes,
        &alias.resident_string_bytes().unwrap()
    ));
    assert!(
        original
            .adopt_native_object_representation_with_string_mutation(
                &equal_replacement,
                dialect,
                Mutation::Discard,
            )
            .is_err()
    );
    original
        .adopt_native_object_representation_with_string_mutation(
            &Value::int(3),
            dialect,
            Mutation::Discard,
        )
        .unwrap();
    assert!(alias.resident_string_bytes().is_none());
}

#[test]
fn dictionary_conversion_order_preserves_original_alias_cache_and_working_storage() {
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
    let copy_first = Value::new_native_string_bytes(&b"k 1"[..]);
    let alias = copy_first.clone();
    let root = copy_first.prepare_native_dictionary(protocol).unwrap();
    assert!(matches!(alias.native_object_snapshot().cache, Cache::None));
    assert!(!root.is_same_object(&copy_first));
    let convert_first = Value::new_native_string_bytes(&b"k 1"[..]);
    let alias = convert_first.clone();
    let child = convert_first
        .prepare_native_dictionary_after_conversion(protocol)
        .unwrap();
    assert!(matches!(
        alias.native_object_snapshot().cache,
        Cache::Dictionary { .. }
    ));
    assert!(!child.is_same_object(&convert_first));
    assert!(child.into_value().resident_string_bytes().is_some());
    let increment = convert_first
        .prepare_native_dictionary_for_increment(protocol)
        .unwrap();
    assert!(increment.into_value().resident_string_bytes().is_none());
    assert_eq!(alias.resident_string_bytes().unwrap().as_ref(), b"k 1");
}

#[test]
fn native_cat_preserves_all_original_cache_and_identity_windows() {
    let fixtures = [
        (
            "tcl9.0",
            include_str!("../../tcl-syntax/tests/data/native_object_cat/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tcl-syntax/tests/data/native_object_cat/9.1.0.tsv"),
        ),
    ];
    assert_eq!(compare_cat_windows(fixtures, CAT_INPUT_KINDS), 624);
}

#[test]
fn native_cat_preserves_raw_nul_invalid_bytes_and_surrogate_windows() {
    let fixtures = [
        (
            "tcl9.0",
            include_str!("../../tcl-syntax/tests/data/native_object_cat/bytes/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tcl-syntax/tests/data/native_object_cat/bytes/9.1.0.tsv"),
        ),
    ];
    assert_eq!(compare_cat_windows(fixtures, CAT_BYTE_INPUT_KINDS), 296);
}
