// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original namespace Tail compiler and direct-worker counted-input controls.

use super::*;
use tcl_syntax::value::ValueOps;
const INPUTS: [&[u8]; 10] = [
    b"a::tail",
    b"no_separator",
    b"a:::tail",
    b"a::",
    b"a::\xed\xa0\x80",
    b"a::\xed\xa0\x81",
    b"a::\xff",
    b"a\0::tail",
    b"",
    b":",
];
fn unhex(bytes: &str) -> Vec<u8> {
    bytes
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
// Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
fn original_namespace_tail_matches_one_hundred_counted_native_windows() {
    let mut windows = 0;
    for (engine, table) in [
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-registry/tests/data/native_namespace_string_compilation/8.4.20/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_namespace_string_compilation/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_namespace_string_compilation/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_namespace_string_compilation/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_namespace_string_compilation/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        let mut vm =
            crate::native_fixture::interpreter(tcl_dialect::DialectProfile::find(engine).unwrap());
        vm.try_eval_source("proc originalTail {value} {namespace tail $value}")
            .unwrap();
        for row in table.lines().filter(|row| row.starts_with("R|")) {
            let fields: Vec<_> = row.split('|').collect();
            let form = fields[1];
            let case = fields[2].parse::<usize>().unwrap();
            let input = Value::new_native_string_bytes(INPUTS[case]);
            let head = Value::new_native_string_bytes(if form == "1" {
                b"originalTail".as_slice()
            } else {
                b"namespace".as_slice()
            });
            let args = if form == "1" {
                vec![input.clone()]
            } else {
                vec![
                    Value::new_native_string_bytes(b"tail".as_slice()),
                    input.clone(),
                ]
            };
            let completion = vm.invoke_host_original_object_vector(&head, &args);
            assert!(
                vm.execution_refusal.is_none(),
                "{engine}/{form}/{case}: {:?}",
                vm.execution_refusal
            );
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[3],
                "{engine}/{form}/{case}"
            );
            let kind = completion.result.native_object_type_name();
            let kind = if kind == "none" { "NULL" } else { kind };
            assert_eq!(kind, fields[4], "{engine}/{form}/{case}: result primary");
            assert_eq!(
                usize::from(completion.result.is_same_object(&input)).to_string(),
                fields[5],
                "{engine}/{form}/{case}: result identity"
            );
            let bytes = vm.native_string_bytes(&completion.result).unwrap();
            assert_eq!(
                bytes.as_ref(),
                unhex(fields[7]),
                "{engine}/{form}/{case}: counted result"
            );
            let original = table
                .lines()
                .find(|row| row.starts_with(&format!("I|{form}|{case}|")))
                .unwrap()
                .split('|')
                .collect::<Vec<_>>();
            let kind = input.native_object_type_name();
            let kind = if kind == "none" { "NULL" } else { kind };
            assert_eq!(
                kind, original[3],
                "{engine}/{form}/{case}: original primary"
            );
            assert_eq!(
                vm.native_string_bytes(&input).unwrap().as_ref(),
                unhex(original[5]),
                "{engine}/{form}/{case}: original counted bytes"
            );
            windows += 1;
        }
    }
    assert_eq!(windows, 100);
}

#[test]
fn original_compiled_tail_declines_foreign_policy_and_unreached_unicode_range() {
    // naming.namespace.original-counted-tail-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
    let mut vm = crate::native_fixture::core(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let subject = Value::new_native_string_bytes(b"a::tail".as_slice());
    let separator = Value::new_native_string_bytes(b"::".as_slice());
    assert!(
        tcl_cmd_core::string::compiled_tail(
            &mut vm,
            &subject,
            &separator,
            tcl_dialect::TclVersion::V9_0
        )
        .is_err()
    );
    assert_eq!(subject.native_object_type_name(), "none");
    let first = Value::new_native_string_bytes(b"0".as_slice());
    let last = Value::new_native_string_bytes(b"end".as_slice());
    assert!(
        tcl_cmd_core::string::compiled_range(
            &mut vm,
            &subject,
            &first,
            &last,
            tcl_dialect::TclVersion::V8_6
        )
        .is_err()
    );
}

const QUALIFIERS_INPUTS: [&[u8]; 20] = [
    b"a::tail",
    b"no_separator",
    b"a:::tail",
    b"a::",
    b"a::\xed\xa0\x80",
    b"a::\xed\xa0\x81",
    b"a::\xff",
    b"a\0::tail",
    b"",
    b":",
    b"::a",
    b":::a",
    b"::::",
    b"a::::b",
    b"a::b::c",
    b"::a::b",
    b"\xff::tail",
    b"\xed\xa0\x80::tail",
    b"\xed\xa0\x81::tail",
    b"a::b\0::tail",
];

#[test]
// Native proof: naming.namespace.original-counted-qualifiers-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
fn original_namespace_qualifiers_matches_two_hundred_forty_counted_native_windows() {
    // naming.namespace.original-counted-qualifiers-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
    let mut windows = 0;
    for (engine, table) in [
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.4.20/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/jim/stdout.tsv"
            ),
        ),
    ] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
        vm.try_eval_source("proc originalQualifiers {value} {namespace qualifiers $value}")
            .unwrap();
        for row in table.lines().filter(|row| row.starts_with("R|")) {
            let fields: Vec<_> = row.split('|').collect();
            let form = fields[1];
            let case = fields[2].parse::<usize>().unwrap();
            let input = Value::new_native_string_bytes(QUALIFIERS_INPUTS[case]);
            let head = Value::new_native_string_bytes(if form == "1" {
                b"originalQualifiers".as_slice()
            } else {
                b"namespace".as_slice()
            });
            let args = if form == "1" {
                vec![input.clone()]
            } else {
                vec![
                    Value::new_native_string_bytes(b"qualifiers".as_slice()),
                    input.clone(),
                ]
            };
            let completion = vm.invoke_host_original_object_vector(&head, &args);
            assert!(
                vm.execution_refusal.is_none(),
                "{engine}/{form}/{case}: {:?}",
                vm.execution_refusal
            );
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[3],
                "{engine}/{form}/{case}"
            );
            let kind = completion.result.native_object_type_name();
            let kind = if kind == "none" { "NULL" } else { kind };
            assert_eq!(kind, fields[4], "{engine}/{form}/{case}: result primary");
            assert_eq!(
                usize::from(completion.result.is_same_object(&input)).to_string(),
                fields[5],
                "{engine}/{form}/{case}: result identity"
            );
            let bytes = vm.native_string_bytes(&completion.result).unwrap();
            assert_eq!(
                bytes.as_ref(),
                unhex(fields[7]),
                "{engine}/{form}/{case}: counted result"
            );
            let original = table
                .lines()
                .find(|row| row.starts_with(&format!("I|{form}|{case}|")))
                .unwrap()
                .split('|')
                .collect::<Vec<_>>();
            let kind = input.native_object_type_name();
            let kind = if kind == "none" { "NULL" } else { kind };
            assert_eq!(
                kind, original[3],
                "{engine}/{form}/{case}: original primary"
            );
            assert_eq!(
                vm.native_string_bytes(&input).unwrap().as_ref(),
                unhex(original[5]),
                "{engine}/{form}/{case}: original counted bytes"
            );
            windows += 1;
        }
    }
    assert_eq!(windows, 240);
}

#[test]
// Native proof: naming.namespace.original-counted-qualifiers-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
fn original_compiled_index_preserves_counted_single_bytes_and_native_units() {
    // naming.namespace.original-counted-qualifiers-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
    let mut vm = crate::native_fixture::core(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let needle = Value::new_native_string_bytes(b"::".as_slice());
    let index = Value::new_native_string_bytes(b"0".as_slice());
    for bytes in [b"\xff".as_slice(), b"\0", b"\xed\xa0\x80", b"\xed\xa0\x81"] {
        let subject = Value::new_native_string_bytes(bytes);
        assert_eq!(
            tcl_cmd_core::string::compiled_find(
                &mut vm,
                &needle,
                &subject,
                tcl_dialect::TclVersion::V8_6,
                true
            )
            .unwrap(),
            -1
        );
        let result = tcl_cmd_core::string::compiled_index(
            &mut vm,
            &subject,
            &index,
            tcl_dialect::TclVersion::V8_6,
        )
        .unwrap();
        assert_eq!(result.native_object_type_name(), "none");
        assert!(!result.is_same_object(&subject));
        assert_eq!(vm.native_string_bytes(&result).unwrap().as_ref(), bytes);
        assert_eq!(vm.native_string_bytes(&subject).unwrap().as_ref(), bytes);
    }
    let unreached = Value::new_native_string_bytes(b"x".as_slice());
    assert!(
        tcl_cmd_core::string::compiled_index(
            &mut vm,
            &unreached,
            &index,
            tcl_dialect::TclVersion::V8_6
        )
        .is_err()
    );
    assert!(
        tcl_cmd_core::string::compiled_index(
            &mut vm,
            &unreached,
            &index,
            tcl_dialect::TclVersion::V9_0
        )
        .is_err()
    );
}

#[test]
// Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
fn original_empty_counted_append_preserves_the_reached_release_conversion() {
    use tcl_dialect::TclVersion;
    use tcl_syntax::native_object::NativeObjectCacheSnapshot;
    use tcl_syntax::native_object_append::NativeObjectAppendProtocol;
    use tcl_syntax::native_string::NativeStringProtocol;

    for (engine, version, converts) in [
        ("tcl8.4", TclVersion::V8_4, true),
        ("tcl8.5", TclVersion::V8_5, true),
        ("tcl8.6", TclVersion::V8_6, false),
        ("tcl9.0", TclVersion::V9_0, false),
        ("tcl9.1", TclVersion::V9_1, false),
    ] {
        let vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
        let dialect = vm.native_scalar_carrier_dialect();
        let protocol = dialect
            .native_object_append_protocol(None)
            .unwrap()
            .recipe();
        assert_eq!(protocol.string_protocol(), NativeStringProtocol::C(version));
        assert_eq!(
            protocol,
            NativeObjectAppendProtocol::for_string_protocol(NativeStringProtocol::C(version))
        );
        let input = Value::new_native_string_bytes(&b"a::"[..]);
        let before = input.native_object_snapshot();
        let output = tcl_cmd_core::native_append::append_counted_bytes(
            &crate::value::VmAppendObjects,
            protocol,
            &input,
            b"",
        )
        .unwrap();
        let after = output.native_object_snapshot();
        assert_eq!(
            after.resident.as_deref(),
            Some(b"a::".as_slice()),
            "{engine}"
        );
        if converts {
            assert!(
                matches!(after.cache, NativeObjectCacheSnapshot::String {
                protocol: NativeStringProtocol::C(current), num_chars: None, unicode: None
            } if current == version),
                "{engine}: {:?}",
                after.cache
            );
        } else {
            assert_eq!(after, before, "{engine}");
            assert!(output.is_same_object(&input), "{engine}");
        }
    }
}

#[test]
fn original_empty_counted_append_matches_thirty_native_constructor_windows() {
    // Native proof: naming.object.original-empty-counted-append
    // docs/design/analysis/name-resolution-proofs/original-empty-counted-append.md
    use tcl_syntax::native_object::NativeObjectCacheSnapshot;

    fn primary(cache: &NativeObjectCacheSnapshot) -> &'static str {
        match cache {
            NativeObjectCacheSnapshot::None => "NULL",
            NativeObjectCacheSnapshot::String { .. } => "string",
            NativeObjectCacheSnapshot::Numeric(
                tcl_syntax::scalar_getter::NativeScalarCache::Number(
                    tcl_syntax::number::Number::Int(_),
                ),
            ) => "int",
            _ => panic!("fixed native constructor primary: {cache:?}"),
        }
    }

    let mut windows = 0;
    for (engine, fixture) in [
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-registry/tests/data/native_empty_counted_append/8.4.20/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_empty_counted_append/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_empty_counted_append/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_empty_counted_append/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_empty_counted_append/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        let vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
        let dialect = vm.native_scalar_carrier_dialect();
        let protocol = dialect
            .native_object_append_protocol(None)
            .unwrap()
            .recipe();
        let strings = protocol.string_protocol();
        for row in fixture.lines().filter(|row| row.starts_with("A|")) {
            let fields = row.split('|').collect::<Vec<_>>();
            let label = fields[1];
            let input = match label {
                "fresh-empty" => Value::new_native_string_bytes(&b""[..]),
                "fresh-tail" => Value::new_native_string_bytes(&b"a::"[..]),
                "fresh-counted" => Value::new_native_string_bytes(&b"\xff\0::t"[..]),
                "integer" => Value::int(5),
                "unicode" => {
                    Value::from_native_unicode_units(std::rc::Rc::from([0xd800u32]), dialect)
                        .unwrap()
                }
                "prepared-string" => {
                    let value = Value::new_native_string_bytes(&b"existing"[..]);
                    assert_eq!(
                        value
                            .native_character_count_with_protocol(
                                strings,
                                dialect.string_length_representation().unwrap()
                            )
                            .unwrap(),
                        8
                    );
                    value
                }
                _ => panic!("fixed native constructor {label}"),
            };
            let before = input.native_object_snapshot();
            let original = fixture
                .lines()
                .find(|row| row.starts_with(&format!("B|{label}|")))
                .unwrap()
                .split('|')
                .collect::<Vec<_>>();
            assert_eq!(
                primary(&before.cache),
                original[2],
                "{engine}/{label}: before primary"
            );
            assert_eq!(
                usize::from(before.resident.is_some()).to_string(),
                original[3],
                "{engine}/{label}: before resident"
            );
            let output = tcl_cmd_core::native_append::append_counted_bytes(
                &crate::value::VmAppendObjects,
                protocol,
                &input,
                b"",
            )
            .unwrap();
            let after = output.native_object_snapshot();
            assert_eq!(
                primary(&after.cache),
                fields[2],
                "{engine}/{label}: after primary"
            );
            assert_eq!(
                usize::from(after.resident.is_some()).to_string(),
                fields[3],
                "{engine}/{label}: after resident"
            );
            if label == "unicode" || label == "prepared-string" {
                assert_eq!(after, before, "{engine}/{label}: retained reached cache");
            }
            // The byte getter is reached only after both header observations.
            let bytes = output.native_string_bytes(strings).unwrap();
            assert_eq!(
                bytes.len().to_string(),
                fields[4],
                "{engine}/{label}: counted length"
            );
            assert_eq!(
                bytes.as_ref(),
                unhex(fields[5]),
                "{engine}/{label}: counted bytes"
            );
            windows += 1;
        }
    }
    assert_eq!(windows, 30);
}
