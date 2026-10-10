// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original native List range header and member-store transactions.

use super::{IntRep, NativeStringProtocol, Value};
use tcl_cmd_core::native_list_storage::{
    NativeListRangeAction, NativeListRangeSelection, NativeListRangeStorage,
};
use tcl_dialect::TclVersion;
use tcl_syntax::{native_compiled_index::NativeCompiledListRange, value::ValueError};

impl Value {
    /// Original `LIST_RANGE_IMM` getter, header sharing, and backing window.
    pub(crate) fn native_list_range(
        &self,
        coordinates: NativeCompiledListRange,
        protocol: NativeStringProtocol,
    ) -> Result<Self, ValueError> {
        self.native_list_selected_range(NativeListRangeSelection::Immediate(coordinates), protocol)
    }

    pub(crate) fn native_list_command_range(
        &self,
        first: i64,
        last: i64,
        protocol: NativeStringProtocol,
    ) -> Result<Self, ValueError> {
        self.native_list_selected_range(NativeListRangeSelection::Command { first, last }, protocol)
    }

    fn native_list_selected_range(
        &self,
        selection: NativeListRangeSelection,
        protocol: NativeStringProtocol,
    ) -> Result<Self, ValueError> {
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native compiled List range release",
            ))?;
        if selection.is_immediate()
            && version >= TclVersion::V9_0
            && self.resident_string_storage_identity()
                == Some(tcl_syntax::native_string::NativeStringStorageIdentity::CanonicalEmpty)
        {
            return Ok(self.clone());
        }
        drop(self.native_object_list_elements(protocol)?);
        let shared = self.native_object_is_shared();
        let mut primary = self.0.intrep.borrow_mut();
        let IntRep::List { items, .. } = &mut *primary else {
            unreachable!("original List getter installed its storage")
        };
        if version >= TclVersion::V9_0 && !shared {
            items.collect_unreferenced()?;
        }
        let action = selection.action(
            version,
            NativeListRangeStorage {
                length: items.len(),
                header_shared: shared,
                store_shared: items.native_is_shared(),
                has_span: items.has_span(),
                used: items.stored_length(),
                allocated: items.capacity(),
                string: tcl_cmd_core::native_list_storage::NativeListStringState::from_empty(
                    self.0
                        .string
                        .borrow()
                        .as_ref()
                        .is_none_or(|bytes| bytes.bytes().is_empty()),
                ),
            },
        )?;
        self.apply_native_list_range(action, primary, shared, version, protocol)
    }

    fn apply_native_list_range(
        &self,
        action: NativeListRangeAction,
        mut primary: std::cell::RefMut<'_, IntRep>,
        shared: bool,
        version: TclVersion,
        protocol: NativeStringProtocol,
    ) -> Result<Self, ValueError> {
        let IntRep::List {
            items, canonical, ..
        } = &mut *primary
        else {
            unreachable!("original List getter installed its storage")
        };
        match action {
            NativeListRangeAction::OriginalEmpty => Ok(self.clone()),
            NativeListRangeAction::EmptyList => {
                let result = crate::NativeListItems::new(Vec::new(), false);
                if shared {
                    Ok(Self::from_parts(
                        None,
                        IntRep::List {
                            canonical: result.canonical_state(),
                            items: result,
                            string_protocol: std::cell::Cell::new(Some(protocol)),
                        },
                    ))
                } else {
                    *canonical = result.canonical_state();
                    *items = result;
                    drop(primary);
                    self.invalidate_native_list_string();
                    Ok(self.clone())
                }
            }
            NativeListRangeAction::FreshEmpty => {
                Ok(Self::native_list_constructor(Vec::new(), protocol))
            }
            NativeListRangeAction::FreshMembers(range) => {
                let result = crate::NativeListItems::new(items.elements()?[range].to_vec(), false);
                if version >= TclVersion::V9_0 && !shared {
                    *canonical = result.canonical_state();
                    *items = result;
                    drop(primary);
                    self.invalidate_native_list_string();
                    Ok(self.clone())
                } else {
                    Ok(Self::from_parts(
                        None,
                        IntRep::List {
                            canonical: result.canonical_state(),
                            items: result,
                            string_protocol: std::cell::Cell::new(Some(protocol)),
                        },
                    ))
                }
            }
            NativeListRangeAction::Whole => {
                if shared {
                    let result = Self::from_retained_native_list_backing(items, protocol)?;
                    Ok(result)
                } else {
                    drop(primary);
                    self.invalidate_native_list_string();
                    Ok(self.clone())
                }
            }
            NativeListRangeAction::Window { range, span } => {
                if shared {
                    let result = items.range_header(range)?;
                    Ok(Self::from_parts(
                        None,
                        IntRep::List {
                            canonical: result.canonical_state(),
                            items: result,
                            string_protocol: std::cell::Cell::new(Some(protocol)),
                        },
                    ))
                } else {
                    let previous = canonical.get();
                    items.select_window(range, span)?;
                    if version < TclVersion::V9_0 {
                        items.canonical_state().set(previous);
                    }
                    *canonical = items.canonical_state();
                    drop(primary);
                    self.invalidate_native_list_string();
                    Ok(self.clone())
                }
            }
        }
    }
    /// Reached range getter when the native VM discards the result.
    pub(crate) fn native_list_range_validate(
        &self,
        protocol: NativeStringProtocol,
    ) -> Result<(), ValueError> {
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native compiled List range release",
            ))?;
        if version >= TclVersion::V9_0
            && self.resident_string_storage_identity()
                == Some(tcl_syntax::native_string::NativeStringStorageIdentity::CanonicalEmpty)
        {
            return Ok(());
        }
        drop(self.native_object_list_elements(protocol)?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::native_compiled_index::NativeCompiledListIndex;

    #[test]
    fn command_range_empty_result_has_actual_list_store_only_from_c9() {
        // naming.list.original-range-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-range-objects-and-instructions.md
        // Inspect reached result birth before string/list result materialisation.
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let original = Value::string("A B C");
            let alias = original.clone();
            let result = original
                .native_list_command_range(12, 2, NativeStringProtocol::C(version))
                .unwrap();
            assert_eq!(
                result.cached_list_representation().is_some(),
                version >= TclVersion::V9_0
            );
            assert_eq!(
                result.resident_string_bytes().is_some(),
                version < TclVersion::V9_0
            );
            if let Some((items, _)) = result.cached_list_representation() {
                assert_eq!(items.len(), 0);
                assert_eq!(items.capacity(), Some(1));
            }
            assert_eq!(&*original.resident_string_bytes().unwrap(), b"A B C");
            drop(alias);
        }
    }

    #[test]
    fn original_list_ranges_preserve_all_five_native_shared_header_windows() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeStringProtocol::C(version);
            for (first, last, length) in [(0, -2, 3), (1, 1, 1)] {
                let original = Value::string("{A} B C");
                let alias = original.clone();
                let result = original
                    .native_list_range(
                        NativeCompiledListRange {
                            first: NativeCompiledListIndex::from_encoded(first),
                            last: NativeCompiledListIndex::from_encoded(last),
                        },
                        protocol,
                    )
                    .unwrap();
                assert_ne!(
                    original.native_object_identity(),
                    result.native_object_identity()
                );
                assert_eq!(result.native_object_reference_count(), 1);
                assert!(result.resident_string_bytes().is_none());
                assert_eq!(&*original.resident_string_bytes().unwrap(), b"{A} B C");
                let source = original.cached_list_representation().unwrap().0;
                let target = result.cached_list_representation().unwrap().0;
                assert_eq!(target.len(), length);
                assert_eq!(
                    source.same_backing(&target),
                    version >= TclVersion::V9_0 && first == 0
                );
                drop(alias);
                assert_eq!(original.native_object_reference_count(), 1);
            }
        }
    }

    #[test]
    fn native_range_rejects_jim_and_foreign_cached_protocols() {
        let original = Value::string("A B C");
        let range = NativeCompiledListRange {
            first: NativeCompiledListIndex::from_encoded(0),
            last: NativeCompiledListIndex::from_encoded(-2),
        };
        assert!(
            original
                .native_list_range(range, NativeStringProtocol::Jim084)
                .is_err()
        );
        drop(
            original
                .native_object_list_elements(NativeStringProtocol::C(TclVersion::V8_5))
                .unwrap(),
        );
        assert!(
            original
                .native_list_range(range, NativeStringProtocol::C(TclVersion::V9_0))
                .is_err()
        );
    }
}
