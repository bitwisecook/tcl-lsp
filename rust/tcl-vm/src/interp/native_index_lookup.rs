// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reusable same-original native Index getter, independent of option consumers.

use super::Vm;
use crate::Value;
use tcl_registry::native_index_lookup::NativeStaticIndexTable;
use tcl_syntax::value::ValueError;

impl Vm {
    /// Actual root/child `GetIndex` purpose, preserving the C9 silent first miss.
    pub(crate) fn native_interpreter_option_from_original(
        &self,
        original: &Value,
        child: bool,
    ) -> Result<&'static str, tcl_cmd_core::CmdError> {
        let recipe = self
            .actual_native_invocation_dialect()
            .native_interpreter_option_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native interp options",
            ))?;
        let words = if child { recipe.child() } else { recipe.root() };
        let table = NativeStaticIndexTable::supported_backend(words);
        if child || recipe.root_miss().is_none() {
            return self
                .native_index_operand(original, &table, false, "option")
                .map(|index| words[index]);
        }
        if let Ok(index) = self.native_index_from_original(original, &table, false, "option")? {
            return Ok(words[index]);
        }
        let miss =
            NativeStaticIndexTable::supported_backend(recipe.root_miss().expect("C9 miss table"));
        if let Err(error) = self.native_index_operand(original, &miss, false, "option") {
            Err(error)
        } else {
            // Native NRInterpCmd always returns ERROR after the second lookup.
            // A successful second lookup installed its cache but did not set result/code.
            let mut details = tcl_cmd_core::CmdError::new_bytes(Vec::new()).into_byte_details();
            details.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Unchanged;
            Err(tcl_cmd_core::CmdError::from_byte_details(details))
        }
    }

    pub(crate) fn native_jim_enum_from_original(
        &self,
        original: &Value,
        table: &NativeStaticIndexTable,
        flags: tcl_registry::native_jim_enum::NativeJimEnumFlags,
        noun: Option<&[u8]>,
    ) -> Result<Result<usize, Option<Vec<u8>>>, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        let protocol = dialect
            .native_jim_enum_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable("Jim Enum lookup"))?;
        if original.native_index_cache().is_some() {
            return Err(ValueError::CommandProtocolUnavailable(
                "foreign native Index origin",
            ));
        }
        if let Some(index) =
            protocol.cached_index(original.native_jim_option_cache().as_ref(), table, flags)
        {
            return Ok(Ok(index));
        }
        let bytes = original
            .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::Jim084)
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)?;
        match protocol.lookup(&bytes, table, flags, noun) {
            Ok(cache) => {
                let index = match &cache {
                    tcl_core_types::NativeJimOptionCache::Enum { entry, .. } => entry.index(),
                    tcl_core_types::NativeJimOptionCache::ComparedString { .. } => {
                        unreachable!("Enum lookup")
                    }
                };
                original.install_native_jim_option_cache(cache, dialect)?;
                Ok(Ok(index))
            }
            Err(message) => Ok(Err(message)),
        }
    }

    pub(crate) fn native_jim_compare_immediate(
        &self,
        original: &Value,
        table: &NativeStaticIndexTable,
        index: usize,
    ) -> Result<bool, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        let protocol =
            dialect
                .native_jim_enum_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim immediate comparison",
                ))?;
        if original.native_index_cache().is_some() {
            return Err(ValueError::CommandProtocolUnavailable(
                "foreign native Index origin",
            ));
        }
        if protocol.compared_hit(original.native_jim_option_cache().as_ref(), table, index) {
            return Ok(true);
        }
        let bytes = original
            .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::Jim084)
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)?;
        if let Some(cache) = protocol.compare(&bytes, table, index) {
            original.install_native_jim_option_cache(cache, dialect)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn native_index_usage_bytes(
        &self,
        original: &Value,
    ) -> Result<(std::rc::Rc<[u8]>, bool), ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if let Some((cache, origin)) = original.native_index_cache() {
            if dialect
                .native_index_lookup_protocol()
                .map(tcl_registry::native_index_lookup::NativeIndexLookupProtocol::version)
                != Some(origin)
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native Index usage origin",
                ));
            }
            let bytes = cache.word().map_err(|_| {
                ValueError::CommandProtocolUnavailable("native Index canonical word")
            })?;
            return Ok((bytes, true));
        }
        Ok((
            original
                .native_string_bytes(dialect.native_string_protocol().ok_or(
                    ValueError::CommandProtocolUnavailable("usage string getter"),
                )?)
                .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)?,
            false,
        ))
    }

    pub(crate) fn native_static_option_index(
        &self,
        original: &Value,
        words: &'static [&'static str],
        exact: bool,
        noun: &'static str,
    ) -> Result<usize, tcl_cmd_core::CmdError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.native_jim_enum_protocol().is_some() {
            return self
                .native_jim_enum_from_original(
                    original,
                    &NativeStaticIndexTable::supported_backend(words),
                    tcl_registry::native_jim_enum::NativeJimEnumFlags::options(exact),
                    Some(noun.as_bytes()),
                )?
                .map_err(|message| {
                    tcl_cmd_core::CmdError::with_error_code_bytes(
                        message.unwrap_or_default(),
                        b"NONE",
                    )
                });
        }
        self.native_index_operand(
            original,
            &NativeStaticIndexTable::supported_backend(words),
            exact,
            noun,
        )
    }

    pub(crate) fn native_index_operand(
        &self,
        original: &Value,
        table: &NativeStaticIndexTable,
        exact: bool,
        noun: &'static str,
    ) -> Result<usize, tcl_cmd_core::CmdError> {
        match self.native_index_from_original(original, table, exact, noun)? {
            Ok(index) => Ok(index),
            Err(message) => {
                let dialect = self.actual_native_invocation_dialect();
                let bytes = original
                    .native_string_bytes(dialect.native_string_protocol().ok_or(
                        ValueError::CommandProtocolUnavailable("native Index string getter"),
                    )?)
                    .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)?;
                let protocol = self
                    .actual_native_invocation_dialect()
                    .native_index_lookup_protocol()
                    .expect("selected native Index protocol");
                Err(tcl_cmd_core::CmdError::with_error_code_bytes(
                    message,
                    protocol.error_code(noun.as_bytes(), &bytes),
                )
                .with_native_string_result(
                    tcl_syntax::native_string::NativeStringProtocol::C(protocol.version()),
                ))
            }
        }
    }

    pub(crate) fn native_index_from_original(
        &self,
        original: &Value,
        table: &NativeStaticIndexTable,
        exact: bool,
        noun: &'static str,
    ) -> Result<Result<usize, Vec<u8>>, ValueError> {
        self.native_index_from_original_with_flags(
            original,
            table,
            tcl_core_types::NativeIndexLookupFlags {
                exact,
                ..Default::default()
            },
            noun,
        )
        .map(|outcome| outcome.map(|index| index.expect("Index lookup without NULL_OK")))
    }

    pub(crate) fn native_index_from_original_with_flags(
        &self,
        original: &Value,
        table: &NativeStaticIndexTable,
        flags: tcl_core_types::NativeIndexLookupFlags,
        noun: &'static str,
    ) -> Result<Result<Option<usize>, Vec<u8>>, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        let protocol = dialect.native_index_lookup_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("native original Index lookup"),
        )?;
        if original.native_jim_option_cache().is_some() {
            return Err(ValueError::CommandProtocolUnavailable(
                "foreign Jim option origin",
            ));
        }
        if let Some(index) =
            protocol.cached_index_with_flags(original.native_index_cache(), table, flags)?
        {
            return Ok(Ok(Some(index)));
        }
        let bytes = original
            .native_string_bytes(dialect.native_string_protocol().ok_or(
                ValueError::CommandProtocolUnavailable("native Index string getter"),
            )?)
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)?;
        let (index, cache) = match protocol.lookup_with_flags(Some(&bytes), table, flags, noun)? {
            Ok(outcome) => outcome,
            Err(message) => return Ok(Err(message)),
        };
        if let Some(cache) = cache {
            let donor = Value::from_native_index_cache(
                cache,
                protocol.version(),
                dialect,
                original
                    .resident_string_bytes()
                    .zip(original.resident_string_storage_identity()),
            )?;
            original.adopt_native_object_representation_with_string_mutation(
                &donor,
                dialect,
                tcl_core_types::ResidentStringMutation::Preserve,
            )?;
        }
        Ok(Ok(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use tcl_dialect::TclVersion;
    use tcl_syntax::native_string::NativeStringProtocol;
    const WORDS: &[&str] = &["provide", "present"];
    const FIXTURES: [&str; 5] = [
        include_str!("../../../tcl-registry/tests/data/native_stock_index/8.4.20.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_stock_index/8.5.19.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_stock_index/8.6.18.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_stock_index/9.0.4.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_stock_index/9.1.0.tsv"),
    ];
    #[test]
    fn core_index_error_string_uses_actual_engine_independently_of_logical_profile() {
        use tcl_syntax::{
            native_object::NativeObjectCacheSnapshot, native_string::NativeStringProtocol,
        };
        let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", WORDS);
        let mut physical = Vm::new();
        physical.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(
            physical
                .set_native_engine_profile(tcl_dialect::DialectProfile::find("tcl9.0").unwrap())
        );
        let original = Value::new_native_string_bytes(b"bad".as_slice());
        let error = table
            .index_of_original(&mut physical, &original)
            .expect_err("actual Index miss");
        let completion = crate::command::completion_from_cmd_error(&mut physical, error);
        assert!(physical.execution_refusal.is_none());
        assert_eq!(completion.code, tcl_runtime_api::Code::Error);
        assert!(matches!(
            completion.result.native_object_snapshot().cache,
            NativeObjectCacheSnapshot::String {
                protocol: NativeStringProtocol::C(TclVersion::V9_0),
                ..
            }
        ));

        let mut unknown = Vm::new();
        unknown.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        let error = table
            .index_of_original(&mut unknown, &original)
            .expect_err("absent actual issuer");
        crate::command::completion_from_cmd_error(&mut unknown, error);
        assert!(unknown.execution_refusal.is_some());

        let mut legacy = Vm::new();
        legacy.set_runtime_version(TclVersion::V8_4);
        let error = table
            .index_of_original(&mut legacy, &original)
            .expect_err("actual C84 Index miss");
        crate::command::completion_from_cmd_error(&mut physical, error);
        assert!(
            physical.execution_refusal.is_some(),
            "foreign physical producer cannot be adopted"
        );
    }

    #[test]
    fn core_original_option_door_preserves_cached_absent_string_for_all_c_versions() {
        let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", WORDS);
        for version in TclVersion::ALL {
            let mut backend = Vm::new();
            backend.set_runtime_version(version);
            let original = Value::new_native_string_bytes(b"pro".as_slice());
            assert_eq!(table.index_of_original(&mut backend, &original).unwrap(), 0);
            original.invalidate_native_string_for_test();
            assert_eq!(table.index_of_original(&mut backend, &original).unwrap(), 0);
            assert!(original.resident_string_bytes().is_none());
            assert!(original.native_index_cache().is_some());
        }
    }

    #[test]
    fn original_index_cache_matches_all_native_25_windows() {
        let mut compared = 0;
        for (version, fixture) in TclVersion::ALL.into_iter().zip(FIXTURES) {
            let mut vm = Vm::new();
            vm.set_runtime_version(version);
            let dialect = vm.actual_native_invocation_dialect();
            let table = NativeStaticIndexTable::supported_backend(WORDS);
            let original = Value::new_native_string_bytes(b"pro".as_slice());
            let bytes = original.resident_string_bytes().unwrap();
            for row in fixture.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields.len(), 6);
                let stage = fields[0];
                let mut duplicate = None;
                if stage == "cached-absent" {
                    let (cache, origin) = original.native_index_cache().unwrap();
                    let donor =
                        Value::from_native_index_cache(cache, origin, dialect, None).unwrap();
                    original
                        .adopt_native_object_representation_with_string_mutation(
                            &donor,
                            dialect,
                            tcl_core_types::ResidentStringMutation::Discard,
                        )
                        .unwrap();
                } else if stage == "duplicate-absent" {
                    duplicate =
                        Some(original.duplicate_native_object_in(NativeStringProtocol::C(version)));
                } else if stage == "counted-nul" {
                    duplicate = Some(Value::new_native_string_bytes(b"provide\0junk".as_slice()));
                }
                let target = duplicate.as_ref().unwrap_or(&original);
                let outcome = vm
                    .native_index_from_original(
                        target,
                        &table,
                        stage != "abbreviated" && stage != "counted-nul",
                        "option",
                    )
                    .unwrap();
                assert_eq!(outcome.unwrap().to_string(), fields[2]);
                assert_eq!(fields[1], "0");
                assert_eq!(fields[3], "index");
                assert_eq!(
                    usize::from(target.resident_string_bytes().is_some()).to_string(),
                    fields[4]
                );
                if stage == "abbreviated" || stage == "cached-exact" {
                    assert!(Rc::ptr_eq(&bytes, &target.resident_string_bytes().unwrap()));
                }
                assert_eq!(fields[5], "1");
                compared += 1;
            }
        }
        assert_eq!(compared, 25);
    }

    #[test]
    fn foreign_index_origin_refuses_before_getter_and_failed_match_preserves_primary() {
        let mut vm = Vm::new();
        vm.set_runtime_version(TclVersion::V8_4);
        let table = NativeStaticIndexTable::supported_backend(WORDS);
        let original = Value::new_native_string_bytes(b"pro".as_slice());
        vm.native_index_from_original(&original, &table, false, "option")
            .unwrap()
            .unwrap();
        let bytes = original.resident_string_bytes().unwrap();
        vm.set_runtime_version(TclVersion::V9_0);
        assert!(
            vm.native_index_from_original(&original, &table, false, "option")
                .is_err()
        );
        assert!(Rc::ptr_eq(
            &bytes,
            &original.resident_string_bytes().unwrap()
        ));
        let numeric = Value::int(42);
        assert!(
            vm.native_index_from_original(&numeric, &table, false, "option")
                .unwrap()
                .is_err()
        );
        assert!(numeric.native_scalar_cache().is_some());
    }

    fn check_original_index_window(
        vm: &mut Vm,
        target: &Value,
        table: &NativeStaticIndexTable,
        flags: tcl_core_types::NativeIndexLookupFlags,
        fields: &[&str],
    ) {
        let before = target.resident_string_bytes();
        let outcome = vm
            .native_index_from_original_with_flags(target, table, flags, "option")
            .unwrap();
        if let Ok(index) = outcome {
            assert_eq!(fields[1], "0");
            assert_eq!(
                index
                    .map_or(-1, |n| i64::try_from(n).expect("native index fits i64"))
                    .to_string(),
                fields[2]
            );
        } else {
            assert_eq!(fields[1], "1");
            assert_eq!(fields[2], "-1");
        }
        assert_eq!(
            if matches!(
                target.native_object_snapshot().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::Index { .. }
            ) {
                "index"
            } else {
                "none"
            },
            fields[3]
        );
        assert_eq!(
            usize::from(target.resident_string_bytes().is_some()).to_string(),
            fields[4]
        );
        let same = match (before, target.resident_string_bytes()) {
            (Some(before), Some(after)) => std::rc::Rc::ptr_eq(&before, &after),
            (None, None) => true,
            _ => false,
        };
        assert_eq!(usize::from(same).to_string(), fields[5]);
    }

    const INDEX_FLAG_FIXTURES: [&str; 5] = [
        include_str!("../../../tcl-registry/tests/data/native_stock_index_flags/8.4.20.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_stock_index_flags/8.5.19.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_stock_index_flags/8.6.18.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_stock_index_flags/9.0.4.tsv"),
        include_str!("../../../tcl-registry/tests/data/native_stock_index_flags/9.1.0.tsv"),
    ];

    fn original_index_tables(
        fields: &'static [&'static str],
        flags_words: &'static [&'static str],
    ) -> (
        NativeStaticIndexTable,
        NativeStaticIndexTable,
        NativeStaticIndexTable,
    ) {
        let pointer_table = NativeStaticIndexTable::supported_backend(fields);
        let struct_table = pointer_table
            .clone()
            .with_entry_stride(2 * std::mem::size_of::<*const std::ffi::c_char>())
            .unwrap();
        let ordinary_table = NativeStaticIndexTable::supported_backend(flags_words);
        (pointer_table, struct_table, ordinary_table)
    }

    const FIELDS: &[&str] = &["provide", "padding", "present"];
    const FLAGS_WORDS: &[&str] = &["provide", "present", ""];

    #[test]
    fn original_index_flags_and_offsets_match_all_native_31_windows() {
        use tcl_core_types::NativeIndexLookupFlags as Flags;

        let mut compared = 0;
        for (version, fixture) in TclVersion::ALL.into_iter().zip(INDEX_FLAG_FIXTURES) {
            let mut vm = Vm::new();
            vm.set_runtime_version(version);
            let (pointer_table, struct_table, ordinary_table) =
                original_index_tables(FIELDS, FLAGS_WORDS);
            let mut original = Value::new_native_string_bytes(b"present".as_slice());
            for row in fixture.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields.len(), 6);
                let stage = fields[0];
                let mut flags = Flags::default();
                let table = match stage {
                    "pointer-stride" => &pointer_table,
                    "struct-stride" => &struct_table,
                    "changed-stride-absent" => {
                        let (cache, origin) = original.native_index_cache().unwrap();
                        let donor = Value::from_native_index_cache(
                            cache,
                            origin,
                            vm.actual_native_invocation_dialect(),
                            None,
                        )
                        .unwrap();
                        original
                            .adopt_native_object_representation_with_string_mutation(
                                &donor,
                                vm.actual_native_invocation_dialect(),
                                tcl_core_types::ResidentStringMutation::Discard,
                            )
                            .unwrap();
                        &pointer_table
                    }
                    "ordinary" => {
                        original = Value::new_native_string_bytes(b"pro".as_slice());
                        &ordinary_table
                    }
                    "temporary-exact-miss" => {
                        flags.exact = true;
                        flags.temporary_table = true;
                        &ordinary_table
                    }
                    "cache-after-temporary-miss" => {
                        flags.exact = true;
                        &ordinary_table
                    }
                    "temporary-fresh" => {
                        original = Value::new_native_string_bytes(b"pro".as_slice());
                        flags.temporary_table = true;
                        &ordinary_table
                    }
                    "null-empty" => {
                        original = Value::new_native_string_bytes(b"".as_slice());
                        flags.null_ok = true;
                        &ordinary_table
                    }
                    "empty-exact-entry" => &ordinary_table,
                    "cached-null-absent" => {
                        let (cache, origin) = original.native_index_cache().unwrap();
                        let donor = Value::from_native_index_cache(
                            cache,
                            origin,
                            vm.actual_native_invocation_dialect(),
                            None,
                        )
                        .unwrap();
                        original
                            .adopt_native_object_representation_with_string_mutation(
                                &donor,
                                vm.actual_native_invocation_dialect(),
                                tcl_core_types::ResidentStringMutation::Discard,
                            )
                            .unwrap();
                        flags.null_ok = true;
                        &ordinary_table
                    }
                    "null-object" => {
                        flags.null_ok = true;
                        let outcome = vm
                            .actual_native_invocation_dialect()
                            .native_index_lookup_protocol()
                            .unwrap()
                            .lookup_with_flags(None, &ordinary_table, flags, "option")
                            .unwrap()
                            .unwrap();
                        assert!(outcome.0.is_none() && outcome.1.is_none());
                        assert_eq!(&fields[1..], &["0", "-1", "none", "0", "1"]);
                        compared += 1;
                        continue;
                    }
                    _ => panic!("unrecognized native row {stage}"),
                };
                let target = &original;
                check_original_index_window(&mut vm, target, table, flags, &fields);
                compared += 1;
            }
        }
        assert_eq!(compared, 31);
    }
}

#[cfg(test)]
#[path = "native_jim_enum_tests.rs"]
mod native_jim_enum_tests;
