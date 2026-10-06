// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reusable same-original native Index lookup and primary installation.

use super::Interp;
use crate::obj::{self, TclObj};
use tcl_registry::native_index_lookup::NativeStaticIndexTable;
use tcl_syntax::value::{ValueError, ValueOps};

impl Interp {
    /// Actual root/child GetIndex purpose, preserving the C9 silent first miss.
    pub(crate) fn native_interpreter_option_from_original(
        &mut self,
        original: *mut TclObj,
        child: bool,
    ) -> Result<&'static str, tcl_cmd_core::CmdError> {
        let recipe = self
            .native_invocation_dialect()
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
        match self.native_index_operand(original, &miss, false, "option") {
            Err(error) => Err(error),
            Ok(_) => {
                // Native NRInterpCmd always returns ERROR after the second lookup.
                // A successful second lookup installed its cache but did not set result/code.
                let mut details = tcl_cmd_core::CmdError::new_bytes(Vec::new()).into_byte_details();
                details.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Unchanged;
                Err(tcl_cmd_core::CmdError::from_byte_details(details))
            }
        }
    }

    pub(crate) fn native_jim_enum_from_original(
        &mut self,
        original: *mut TclObj,
        table: &NativeStaticIndexTable,
        flags: tcl_registry::native_jim_enum::NativeJimEnumFlags,
        noun: Option<&[u8]>,
    ) -> Result<Result<usize, Option<Vec<u8>>>, ValueError> {
        let dialect = self.native_invocation_dialect();
        let protocol = dialect
            .native_jim_enum_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable("Jim Enum lookup"))?;
        if obj::native_index::cache(original).is_some() {
            return Err(ValueError::CommandProtocolUnavailable(
                "foreign native Index origin",
            ));
        }
        if let Some(index) =
            protocol.cached_index(obj::native_jim_enum::cache(original).as_ref(), table, flags)
        {
            return Ok(Ok(index));
        }
        let bytes = self.native_string_bytes(&original)?;
        match protocol.lookup(&bytes, table, flags, noun) {
            Ok(cache) => {
                let index = match &cache {
                    tcl_core_types::NativeJimOptionCache::Enum { entry, .. } => entry.index(),
                    _ => unreachable!("Enum lookup"),
                };
                obj::native_jim_enum::install(original, cache, dialect)?;
                Ok(Ok(index))
            }
            Err(message) => Ok(Err(message)),
        }
    }

    pub(crate) fn native_jim_compare_immediate(
        &mut self,
        original: *mut TclObj,
        table: &NativeStaticIndexTable,
        index: usize,
    ) -> Result<bool, ValueError> {
        let dialect = self.native_invocation_dialect();
        let protocol =
            dialect
                .native_jim_enum_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim immediate comparison",
                ))?;
        if obj::native_index::cache(original).is_some() {
            return Err(ValueError::CommandProtocolUnavailable(
                "foreign native Index origin",
            ));
        }
        if protocol.compared_hit(obj::native_jim_enum::cache(original).as_ref(), table, index) {
            return Ok(true);
        }
        let bytes = self.native_string_bytes(&original)?;
        if let Some(cache) = protocol.compare(&bytes, table, index) {
            obj::native_jim_enum::install(original, cache, dialect)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn native_index_usage_bytes(
        &mut self,
        original: *mut TclObj,
    ) -> Result<(std::rc::Rc<[u8]>, bool), ValueError> {
        let dialect = self.native_invocation_dialect();
        if let Some((cache, origin)) = obj::native_index::cache(original) {
            if dialect.native_index_lookup_protocol().map(|p| p.version()) != Some(origin) {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native Index usage origin",
                ));
            }
            let bytes = cache.word().map_err(|_| {
                ValueError::CommandProtocolUnavailable("native Index canonical word")
            })?;
            return Ok((bytes, true));
        }
        Ok((self.native_string_bytes(&original)?, false))
    }

    pub(crate) fn native_static_option_index(
        &mut self,
        original: *mut TclObj,
        words: &'static [&'static [u8]],
        exact: bool,
        noun: &'static str,
    ) -> Result<usize, tcl_cmd_core::CmdError> {
        let dialect = self.native_invocation_dialect();
        if dialect.native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            if obj::native_index::cache(original).is_some() {
                return Err(
                    ValueError::CommandProtocolUnavailable("foreign native Index origin").into(),
                );
            }
            let bytes = self.native_string_bytes(&original)?;
            let table = if exact {
                tcl_cmd_core::prefix::OptionTable::exact_only(noun, words)
            } else {
                tcl_cmd_core::prefix::OptionTable::abbreviating(noun, words)
            };
            return table.index_of(&bytes).map_err(|message| {
                tcl_cmd_core::CmdError::with_error_code_bytes(message, b"NONE")
            });
        }
        self.native_index_operand(
            original,
            &NativeStaticIndexTable::supported_backend_bytes(words),
            exact,
            noun,
        )
    }

    pub(crate) fn native_static_string_option_index(
        &mut self,
        original: *mut TclObj,
        words: &'static [&'static str],
        exact: bool,
        noun: &'static str,
    ) -> Result<usize, tcl_cmd_core::CmdError> {
        self.native_index_operand(
            original,
            &NativeStaticIndexTable::supported_backend(words),
            exact,
            noun,
        )
    }

    pub(crate) fn native_index_operand(
        &mut self,
        original: *mut TclObj,
        table: &NativeStaticIndexTable,
        exact: bool,
        noun: &'static str,
    ) -> Result<usize, tcl_cmd_core::CmdError> {
        match self.native_index_from_original(original, table, exact, noun)? {
            Ok(index) => Ok(index),
            Err(message) => {
                let bytes = self.native_string_bytes(&original)?;
                let protocol = self
                    .native_invocation_dialect()
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
        &mut self,
        original: *mut TclObj,
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
        &mut self,
        original: *mut TclObj,
        table: &NativeStaticIndexTable,
        flags: tcl_core_types::NativeIndexLookupFlags,
        noun: &'static str,
    ) -> Result<Result<Option<usize>, Vec<u8>>, ValueError> {
        obj::check_native_liveness(original)?;
        let protocol = self
            .native_invocation_dialect()
            .native_index_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native original Index lookup",
            ))?;
        if obj::native_jim_enum::cache(original).is_some() {
            return Err(ValueError::CommandProtocolUnavailable(
                "foreign Jim option origin",
            ));
        }
        if let Some(index) =
            protocol.cached_index_with_flags(obj::native_index::cache(original), table, flags)?
        {
            return Ok(Ok(Some(index)));
        }
        let bytes = self.native_string_bytes(&original)?;
        let (index, cache) = match protocol.lookup_with_flags(Some(&bytes), table, flags, noun)? {
            Ok(outcome) => outcome,
            Err(message) => return Ok(Err(message)),
        };
        if let Some(cache) = cache {
            obj::native_index::install(original, cache, protocol)?;
        }
        Ok(Ok(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;
    const WORDS: &[&str] = &["provide", "present"];
    const FIXTURES: [&str; 5] = [
        include_str!("../../../../rust/tcl-registry/tests/data/native_stock_index/8.4.20.tsv"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_stock_index/8.5.19.tsv"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_stock_index/8.6.18.tsv"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_stock_index/9.0.4.tsv"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_stock_index/9.1.0.tsv"),
    ];
    #[test]
    fn core_original_option_door_preserves_cached_absent_string_for_all_c_versions() {
        let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", WORDS);
        for version in TclVersion::ALL {
            let mut backend = Interp::new();
            backend.set_runtime_version(version);
            let original = obj::Owned::fresh(obj::new_string_bytes(b"pro"));
            assert_eq!(
                table
                    .index_of_original(&mut backend, &original.as_ptr())
                    .unwrap(),
                0
            );
            obj::invalidate_string(original.as_ptr());
            assert_eq!(
                table
                    .index_of_original(&mut backend, &original.as_ptr())
                    .unwrap(),
                0
            );
            assert!(!obj::has_string_rep(original.as_ptr()));
            assert!(obj::native_index::cache(original.as_ptr()).is_some());
        }
    }

    #[test]
    fn original_index_cache_matches_all_native_25_windows() {
        let mut compared = 0;
        for (version, fixture) in TclVersion::ALL.into_iter().zip(FIXTURES) {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            let table = NativeStaticIndexTable::supported_backend(WORDS);
            let original = obj::Owned::fresh(obj::new_string_bytes(b"pro"));
            let bytes = unsafe { (*original.as_ptr()).bytes };
            for row in fixture.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields.len(), 6);
                let stage = fields[0];
                let mut duplicate = None;
                if stage == "cached-absent" {
                    obj::invalidate_string(original.as_ptr());
                } else if stage == "duplicate-absent" {
                    duplicate = Some(obj::Owned::fresh(obj::duplicate(original.as_ptr())));
                } else if stage == "counted-nul" {
                    duplicate = Some(obj::Owned::fresh(obj::new_string_bytes(b"provide\0junk")));
                }
                let target = duplicate.as_ref().unwrap_or(&original).as_ptr();
                let outcome = interp
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
                    usize::from(obj::has_string_rep(target)).to_string(),
                    fields[4]
                );
                if stage == "abbreviated" || stage == "cached-exact" {
                    assert_eq!(unsafe { (*target).bytes }, bytes);
                }
                assert_eq!(fields[5], "1");
                compared += 1;
            }
        }
        assert_eq!(compared, 25);
    }
    #[test]
    fn foreign_index_origin_refuses_before_getter() {
        let mut interp = Interp::new();
        interp.set_runtime_version(TclVersion::V8_4);
        let table = NativeStaticIndexTable::supported_backend(WORDS);
        let original = obj::Owned::fresh(obj::new_string_bytes(b"pro"));
        interp
            .native_index_from_original(original.as_ptr(), &table, false, "option")
            .unwrap()
            .unwrap();
        let bytes = unsafe { (*original.as_ptr()).bytes };
        interp.set_runtime_version(TclVersion::V9_0);
        assert!(interp
            .native_index_from_original(original.as_ptr(), &table, false, "option")
            .is_err());
        assert_eq!(unsafe { (*original.as_ptr()).bytes }, bytes);
    }

    #[test]
    fn original_index_flags_and_offsets_match_all_native_31_windows() {
        use tcl_core_types::NativeIndexLookupFlags as Flags;
        const FIELDS: &[&str] = &["provide", "padding", "present"];
        const FLAGS_WORDS: &[&str] = &["provide", "present", ""];
        const FIXTURES: [&str; 5] = [
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_stock_index_flags/8.4.20.tsv"
            ),
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_stock_index_flags/8.5.19.tsv"
            ),
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_stock_index_flags/8.6.18.tsv"
            ),
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_stock_index_flags/9.0.4.tsv"
            ),
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_stock_index_flags/9.1.0.tsv"
            ),
        ];
        let mut compared = 0;
        for (version, fixture) in TclVersion::ALL.into_iter().zip(FIXTURES) {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            let pointer_table = NativeStaticIndexTable::supported_backend(FIELDS);
            let struct_table = pointer_table
                .clone()
                .with_entry_stride(2 * std::mem::size_of::<*const std::ffi::c_char>())
                .unwrap();
            let ordinary_table = NativeStaticIndexTable::supported_backend(FLAGS_WORDS);
            let mut original = obj::Owned::fresh(obj::new_string_bytes(b"present"));
            for row in fixture.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields.len(), 6);
                let stage = fields[0];
                let mut flags = Flags::default();
                let table = match stage {
                    "pointer-stride" => &pointer_table,
                    "struct-stride" => &struct_table,
                    "changed-stride-absent" => {
                        obj::invalidate_string(original.as_ptr());
                        &pointer_table
                    }
                    "ordinary" => {
                        original = obj::Owned::fresh(obj::new_string_bytes(b"pro"));
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
                        original = obj::Owned::fresh(obj::new_string_bytes(b"pro"));
                        flags.temporary_table = true;
                        &ordinary_table
                    }
                    "null-empty" => {
                        original = obj::Owned::fresh(obj::new_string_bytes(b""));
                        flags.null_ok = true;
                        &ordinary_table
                    }
                    "empty-exact-entry" => &ordinary_table,
                    "cached-null-absent" => {
                        obj::invalidate_string(original.as_ptr());
                        flags.null_ok = true;
                        &ordinary_table
                    }
                    "null-object" => {
                        flags.null_ok = true;
                        let outcome = interp
                            .native_invocation_dialect()
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
                let target = original.as_ptr();
                let before = unsafe { (*target).bytes };
                let outcome = interp
                    .native_index_from_original_with_flags(target, table, flags, "option")
                    .unwrap();
                match outcome {
                    Ok(index) => {
                        assert_eq!(fields[1], "0");
                        assert_eq!(index.map_or(-1, |n| n as i64).to_string(), fields[2]);
                    }
                    Err(_) => {
                        assert_eq!(fields[1], "1");
                        assert_eq!(fields[2], "-1");
                    }
                }
                assert_eq!(
                    if obj::native_index::cache(target).is_some() {
                        "index"
                    } else {
                        "none"
                    },
                    fields[3]
                );
                assert_eq!(
                    usize::from(obj::has_string_rep(target)).to_string(),
                    fields[4]
                );
                let same = before == unsafe { (*target).bytes };
                assert_eq!(usize::from(same).to_string(), fields[5]);
                compared += 1;
            }
        }
        assert_eq!(compared, 31);
    }
}

#[cfg(test)]
#[path = "native_jim_enum_tests.rs"]
mod native_jim_enum_tests;
