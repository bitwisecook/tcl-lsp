// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Jim GetEnum windows from the pinned native producer.
use super::*;
use tcl_core_types::NativeJimOptionCache;
use tcl_registry::native_jim_enum::NativeJimEnumFlags;
const WORDS: &[&str] = &["provide", "present", "--"];
const OTHER: &[&str] = &["present", "provide"];
const BOGUS: &[&str] = &["bogus"];
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn original_jim_enum_matches_all_native_14_windows() {
    let backend = crate::native_fixture::core(
        tcl_registry::model::ingress::static_context_for("jim")
            .commands()
            .profile()
            .expect("resolved Jim profile"),
    );
    let table = NativeStaticIndexTable::supported_backend(WORDS);
    let other = NativeStaticIndexTable::supported_backend(OTHER);
    let bogus = NativeStaticIndexTable::supported_backend(BOGUS);
    let original = Value::new_native_string_bytes(b"pro".as_slice());
    let mut result = Vec::new();
    let mut compared = 0;
    for row in
        include_str!("../../../tcl-registry/tests/data/native_interpreter_enum/Jim.tsv").lines()
    {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 9);
        let stage = fields[0];
        let target = match stage {
            "abbrev-errmsg"
            | "exact-miss-different-flags"
            | "abbrev-noerrmsg-different-flags"
            | "changed-table"
            | "cached-absent" => None,
            "fresh-empty" => Some(Value::new_native_string_bytes(b"".as_slice())),
            "fresh--" => Some(Value::new_native_string_bytes(b"-".as_slice())),
            "fresh-p" => Some(Value::new_native_string_bytes(b"p".as_slice())),
            "fresh-bogus" | "compared-failed" => {
                Some(Value::new_native_string_bytes(b"bogus".as_slice()))
            }
            "fresh---" => Some(Value::new_native_string_bytes(b"--".as_slice())),
            "counted-nul-exact" => Some(Value::new_native_string_bytes(b"provide\0x".as_slice())),
            "compared-exact" => Some(Value::new_native_string_bytes(b"provide".as_slice())),
            "numeric-failed" => Some(Value::int(42)),
            _ => panic!("unrecognised native stage {stage}"),
        };
        let target = target.as_ref().unwrap_or(&original);
        if stage == "cached-absent" {
            target.invalidate_native_string_for_test();
        }
        if stage == "compared-exact" {
            assert!(
                backend
                    .native_jim_compare_immediate(target, &table, 0)
                    .unwrap()
            );
        }
        if stage == "compared-failed" {
            assert!(
                backend
                    .native_jim_compare_immediate(target, &bogus, 0)
                    .unwrap()
            );
        }
        let selected = if matches!(stage, "changed-table" | "cached-absent") {
            &other
        } else {
            &table
        };
        let flags = NativeJimEnumFlags(match stage {
            "exact-miss-different-flags"
            | "counted-nul-exact"
            | "compared-exact"
            | "compared-failed" => 1,
            "abbrev-noerrmsg-different-flags" | "changed-table" | "cached-absent" => 2,
            _ => 3,
        });
        let before = target.resident_string_bytes();
        let outcome = backend
            .native_jim_enum_from_original(target, selected, flags, None)
            .unwrap();
        match outcome {
            Ok(index) => {
                assert_eq!(fields[1], "0", "{stage}");
                assert_eq!(index.to_string(), fields[2], "{stage}");
            }
            Err(message) => {
                assert_eq!(fields[1], "1", "{stage}");
                assert_eq!(fields[2], "-1");
                if let Some(message) = message {
                    result = message;
                }
            }
        }
        let cache = target.native_jim_option_cache();
        let (kind, cached_flags, same_table) = match cache.as_ref() {
            Some(NativeJimOptionCache::Enum { entry, flags }) => (
                "get-enum",
                *flags,
                entry.table_identity() == selected.declaration_identity(),
            ),
            Some(NativeJimOptionCache::ComparedString { .. }) => ("compared-string", -1, false),
            None if stage == "numeric-failed" => ("int", -1, false),
            None => ("none", -1, false),
        };
        assert_eq!(kind, fields[3], "{stage}");
        assert_eq!(cached_flags.to_string(), fields[4], "{stage}");
        assert_eq!(usize::from(same_table).to_string(), fields[5], "{stage}");
        let after = target.resident_string_bytes();
        assert_eq!(
            usize::from(after.is_some()).to_string(),
            fields[6],
            "{stage}"
        );
        assert_eq!(
            usize::from(match (&before, &after) {
                (Some(a), Some(b)) => std::rc::Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            })
            .to_string(),
            fields[7],
            "{stage}"
        );
        assert_eq!(hex(&result), fields[8], "{stage}");
        compared += 1;
    }
    assert_eq!(compared, 14);
}

#[test]
fn original_interpreter_two_table_errors_match_all_native_15_windows() {
    const FIXTURES: [&str; 5] = [
        include_str!("../../../tcl-registry/tests/data/native_interpreter_enum/8.4.20.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_interpreter_enum/8.5.19.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_interpreter_enum/8.6.18.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_interpreter_enum/9.0.4.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_interpreter_enum/9.1.0.tsv"),
    ];
    let mut compared = 0;
    for (version, fixture) in tcl_dialect::TclVersion::ALL.into_iter().zip(FIXTURES) {
        let backend = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap(),
        );
        for row in fixture.lines().filter(|row| {
            row.starts_with("root-s\t")
                || row.starts_with("root-h\t")
                || row.starts_with("root-bogus\t")
        }) {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 9);
            let original =
                Value::new_native_string_bytes(fields[0].strip_prefix("root-").unwrap().as_bytes());
            let before = original.resident_string_bytes().unwrap();
            let failure = backend
                .native_interpreter_option_from_original(&original, false)
                .expect_err("actual root table miss");
            assert_eq!(
                hex(failure.message_bytes()),
                fields[8],
                "{version:?} {}",
                fields[0]
            );
            let cache = original.native_index_cache();
            assert_eq!(
                cache
                    .as_ref()
                    .map_or(-1, |entry| entry.0.index() as i64)
                    .to_string(),
                fields[3]
            );
            if fields[2] == "index" {
                assert_eq!(version, tcl_dialect::TclVersion::V9_0);
                assert_eq!(cache.as_ref().unwrap().0.word().unwrap().as_ref(), b"share");
                assert!(matches!(
                    failure.into_byte_details().error_code,
                    tcl_cmd_core::CmdErrorCodeUpdate::Unchanged
                ));
            } else {
                assert!(cache.is_none());
            }
            let after = original.resident_string_bytes().unwrap();
            assert!(std::rc::Rc::ptr_eq(&before, &after));
            compared += 1;
        }
    }
    assert_eq!(compared, 15);
}
