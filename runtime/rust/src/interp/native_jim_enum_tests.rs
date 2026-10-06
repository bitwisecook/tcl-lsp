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
    let mut backend = Interp::new();
    backend.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
    );
    let table = NativeStaticIndexTable::supported_backend(WORDS);
    let other = NativeStaticIndexTable::supported_backend(OTHER);
    let bogus = NativeStaticIndexTable::supported_backend(BOGUS);
    let original = obj::Owned::fresh(obj::new_string_bytes(b"pro"));
    let mut result = Vec::new();
    let mut compared = 0;
    for row in
        include_str!("../../../../rust/tcl-registry/tests/data/native_interpreter_enum/Jim.tsv")
            .lines()
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
            "fresh-empty" => Some(obj::Owned::fresh(obj::new_string_bytes(b""))),
            "fresh--" => Some(obj::Owned::fresh(obj::new_string_bytes(b"-"))),
            "fresh-p" => Some(obj::Owned::fresh(obj::new_string_bytes(b"p"))),
            "fresh-bogus" | "compared-failed" => {
                Some(obj::Owned::fresh(obj::new_string_bytes(b"bogus")))
            }
            "fresh---" => Some(obj::Owned::fresh(obj::new_string_bytes(b"--"))),
            "counted-nul-exact" => Some(obj::Owned::fresh(obj::new_string_bytes(b"provide\0x"))),
            "compared-exact" => Some(obj::Owned::fresh(obj::new_string_bytes(b"provide"))),
            "numeric-failed" => Some(obj::Owned::fresh(obj::new_wide_int_obj(42))),
            _ => panic!("unrecognised native stage {stage}"),
        };
        let target = target.as_ref().unwrap_or(&original);
        if stage == "cached-absent" {
            obj::invalidate_string(target.as_ptr());
        }
        if stage == "compared-exact" {
            assert!(backend
                .native_jim_compare_immediate(target.as_ptr(), &table, 0)
                .unwrap());
        }
        if stage == "compared-failed" {
            assert!(backend
                .native_jim_compare_immediate(target.as_ptr(), &bogus, 0)
                .unwrap());
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
        let before =
            obj::has_string_rep(target.as_ptr()).then(|| unsafe { (*target.as_ptr()).bytes });
        let outcome = backend
            .native_jim_enum_from_original(target.as_ptr(), selected, flags, None)
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
        let cache = obj::native_jim_enum::cache(target.as_ptr());
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
        let after =
            obj::has_string_rep(target.as_ptr()).then(|| unsafe { (*target.as_ptr()).bytes });
        assert_eq!(
            usize::from(after.is_some()).to_string(),
            fields[6],
            "{stage}"
        );
        assert_eq!(
            usize::from(before == after).to_string(),
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
        include_str!("../../../../rust/tcl-registry/tests/data/native_interpreter_enum/8.4.20.tsv"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_interpreter_enum/8.5.19.tsv"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_interpreter_enum/8.6.18.tsv"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_interpreter_enum/9.0.4.tsv"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_interpreter_enum/9.1.0.tsv"),
    ];
    let mut compared = 0;
    for (version, fixture) in tcl_dialect::TclVersion::ALL.into_iter().zip(FIXTURES) {
        let mut backend = Interp::new();
        backend.set_runtime_version(version);
        for row in fixture.lines().filter(|row| {
            row.starts_with("root-s\t")
                || row.starts_with("root-h\t")
                || row.starts_with("root-bogus\t")
        }) {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 9);
            let original = obj::Owned::fresh(obj::new_string_bytes(
                fields[0].strip_prefix("root-").unwrap().as_bytes(),
            ));
            let before = unsafe { (*original.as_ptr()).bytes };
            let failure = backend
                .native_interpreter_option_from_original(original.as_ptr(), false)
                .expect_err("actual root table miss");
            assert_eq!(
                hex(failure.message_bytes()),
                fields[8],
                "{version:?} {}",
                fields[0]
            );
            let cache = obj::native_index::cache(original.as_ptr());
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
            let after = unsafe { (*original.as_ptr()).bytes };
            assert!(before == after);
            compared += 1;
        }
    }
    assert_eq!(compared, 15);
}
