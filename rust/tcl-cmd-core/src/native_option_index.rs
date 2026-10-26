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

//! Original search/sort option-path conversion and pure prepared coordinates.

use crate::{
    CmdError,
    index::{self, NativeIndexOps},
    native_list_index::{self, NativeIndexMembers, NativeListIndexOps},
};
use tcl_dialect::TclVersion;
use tcl_syntax::{
    native_compiled_index::NativeCompiledListIndex, native_end_offset::NativeEndOffset,
    number::Number, value::ValueError,
};

/// Descriptive selected local C9 layout, independent of native object authority.
/// A foreign library or BIG-IP layout requires its own actual issuer.
#[derive(Clone, Copy, Debug)]
pub struct NativeOptionIndexLayout {
    /// Local native `Tcl_Size` maximum.
    pub size_max: i64,
    /// Pinned C9 `ListStore` allocation bound under that local C ABI layout.
    pub list_max: i64,
}
impl NativeOptionIndexLayout {
    /// Derive the actual local target's standard C9 field/alignment geometry.
    /// This pure description cannot admit an original object/getter operation.
    ///
    /// # Errors
    /// The local int/size/pointer layout is outside this selected producer.
    pub fn local() -> Result<Self, ValueError> {
        #[repr(C)]
        struct ListStore {
            first_used: isize,
            num_used: isize,
            num_allocated: isize,
            ref_count: usize,
            flags: std::ffi::c_int,
            slots: [*mut std::ffi::c_void; 0],
        }
        if core::mem::size_of::<std::ffi::c_int>() != 4 {
            return Err(ValueError::CommandProtocolUnavailable(
                "C9 option index Int32 layout",
            ));
        }
        let size_max = i64::try_from(isize::MAX)
            .map_err(|_| ValueError::CommandProtocolUnavailable("C9 option Tcl_Size layout"))?;
        let slots_offset = core::mem::offset_of!(ListStore, slots);
        let list_max = (usize::try_from(isize::MAX).expect("positive local size") - slots_offset)
            / core::mem::size_of::<*mut std::ffi::c_void>();
        Ok(Self {
            size_max,
            list_max: i64::try_from(list_max).expect("local ListStore bound"),
        })
    }
}

/// The independently selected native command option consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionCommand {
    Search,
    Sort,
}

/// Coordinates prepared on actual original headers, or explicit neutral text.
#[derive(Default)]
pub struct PreparedPath {
    neutral: Vec<Vec<u8>>,
    original: Option<Vec<NativeCompiledListIndex>>,
}
impl PreparedPath {
    /// Pure neutral coordinates; no native cache or getter claim.
    #[must_use]
    pub fn neutral(neutral: Vec<Vec<u8>>) -> Self {
        Self {
            neutral,
            original: None,
        }
    }
    /// Coordinates issued by the checked original C option owner.
    #[must_use]
    pub fn original(original: Vec<NativeCompiledListIndex>) -> Self {
        Self {
            neutral: Vec::new(),
            original: Some(original),
        }
    }
    /// Borrow prepared geometry without re-entering an index getter.
    #[must_use]
    pub fn path(&self) -> Path<'_> {
        self.original
            .as_deref()
            .map_or(Path::Neutral(&self.neutral), Path::Original)
    }
}

/// Borrowed pure path geometry; this value grants no native object authority.
#[derive(Clone, Copy)]
pub enum Path<'a> {
    Neutral(&'a [Vec<u8>]),
    Original(&'a [NativeCompiledListIndex]),
}
impl Path<'_> {
    /// Number of prepared nested selections.
    #[must_use]
    pub fn len(self) -> usize {
        match self {
            Self::Neutral(v) => v.len(),
            Self::Original(v) => v.len(),
        }
    }
    /// Whether the key is the entire original value.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    /// Remove the already reached group coordinate.
    #[must_use]
    pub fn tail(self) -> Self {
        match self {
            Self::Neutral(v) => Self::Neutral(&v[1..]),
            Self::Original(v) => Self::Original(&v[1..]),
        }
    }
    /// Decode one prepared coordinate against the consumer's actual end.
    ///
    /// # Errors
    /// A neutral lexical coordinate rejects its grammar.
    pub fn decode<O: NativeIndexOps>(
        self,
        ops: &mut O,
        ordinal: usize,
        end: usize,
    ) -> Result<i64, CmdError> {
        match self {
            Self::Neutral(v) => {
                let text = std::str::from_utf8(&v[ordinal]).map_err(|error| {
                    tcl_syntax::raw_string::UnicodeAccessError {
                        valid_up_to: error.valid_up_to(),
                        error_len: error.error_len(),
                    }
                })?;
                index::resolve_for_ops(ops, text, end.saturating_add(1))
            }
            Self::Original(v) => {
                i64::try_from(v[ordinal].decode(i128::try_from(end).map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original option path end width")
                })?))
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original option path decoded width")
                        .into()
                })
            }
        }
    }
    /// Select using the same actual Length/member/returned-role owners.
    ///
    /// # Errors
    /// Reached original List/member/diagnostic or full first Host refusal.
    pub fn drill<O: NativeIndexOps>(
        self,
        ops: &mut O,
        original: &O::Value,
    ) -> Result<index::IndexedKey<O::IndexKeyOwner>, CmdError> {
        match self {
            Self::Neutral(path) => index::drill(ops, original, path),
            Self::Original(path) => index::drill_prepared_c(ops, original, path),
        }
    }
}

/// Validate the original selected option path, without holding its members.
/// Modern lsort's later quiet pass calls this same worker on the last original.
///
/// # Errors
/// Original List/index conversion, native encoding bounds or full first Host refusal.
pub fn prepare_original<O: NativeListIndexOps>(
    ops: &mut O,
    original: &O::Value,
    command: OptionCommand,
    report: bool,
) -> Result<Vec<NativeCompiledListIndex>, CmdError> {
    index::run_index_metadata(ops, |ops| {
        let version = ops.index_version()?;
        if version == TclVersion::V8_4 {
            if command == OptionCommand::Search {
                return Err(ValueError::CommandProtocolUnavailable(
                    "C8.4 lsearch has no index option",
                )
                .into());
            }
            let coordinate = native_list_index::original_index(ops, original, -2, true)?
                .expect("reported C8.4 single option index");
            let coordinate = tcl_syntax::number::native_int32_low_bits(coordinate);
            return Ok(if coordinate == -1 {
                Vec::new()
            } else {
                vec![NativeCompiledListIndex::from_encoded(coordinate)]
            });
        }

        // Borrow original path members. No recreated String or protective +1.
        let members = ops.index_option_members_checked(original)?;
        let count = match members.index_elements() {
            Ok(members) => members.len(),
            Err(error) => {
                let error = CmdError::from(error);
                ops.retain_index_failure(&error);
                return Err(error);
            }
        };
        let mut coordinates = Vec::with_capacity(count);
        for ordinal in 0..count {
            let member = match members.index_elements() {
                Ok(members) => &members[ordinal],
                Err(error) => {
                    let error = CmdError::from(error);
                    ops.retain_index_failure(&error);
                    return Err(error);
                }
            };
            let outcome = encode_original(ops, member, version, report);
            // A callback cannot donate a withdrawn original path array to the
            // next coordinate, even when its old children remain independently live.
            if let Err(error) = &outcome {
                ops.retain_index_failure(error);
            }
            let current = members.index_elements().map_err(CmdError::from);
            if let Err(error) = &current {
                ops.retain_index_failure(error);
            }
            current?;
            let encoded = match outcome {
                Ok(encoded) => encoded,
                Err(error) if error.native_execution_refusal().is_some() => return Err(error),
                Err(error) => return Err(option_item_error(error, ordinal)),
            };
            coordinates.push(NativeCompiledListIndex::from_encoded(encoded));
        }
        Ok(coordinates)
    })
}

fn option_item_error(error: CmdError, ordinal: usize) -> CmdError {
    let mut details = error.into_byte_details();
    let info = details
        .error_info
        .get_or_insert_with(|| details.message.clone());
    info.extend_from_slice(format!("\n    (-index option item number {ordinal})").as_bytes());
    CmdError::from_byte_details(details)
}
fn out_of_range<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::Value,
    version: TclVersion,
) -> Result<CmdError, CmdError> {
    let bytes = ops.index_operand_bytes(original)?;
    let mut message = b"index \"".to_vec();
    message.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
    message.extend_from_slice(if version == TclVersion::V8_6 {
        b"\" cannot select an element from any list"
    } else {
        b"\" out of range"
    });
    let error = CmdError::with_error_code_bytes(
        message,
        if version == TclVersion::V8_6 {
            b"TCL VALUE INDEXOUTOFRANGE".as_slice()
        } else {
            b"TCL VALUE INDEX OUTOFRANGE".as_slice()
        },
    );
    Ok(
        error
            .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::C(version)),
    )
}

fn encode_original<O: NativeListIndexOps>(
    ops: &mut O,
    original: &O::Value,
    version: TclVersion,
    report: bool,
) -> Result<i32, CmdError> {
    if version == TclVersion::V8_5 {
        let index = native_list_index::original_index(ops, original, -2, true)?
            .expect("reported C8.5 option index");
        return Ok(tcl_syntax::number::native_int32_low_bits(index));
    }
    if version == TclVersion::V8_6 {
        let integer = match ops.index_number_checked(original)? {
            Some(Number::Int(integer)) => Some(tcl_syntax::number::native_int32_low_bits(integer)),
            _ => None,
        };
        let encoded = if let Some(integer) = integer {
            absolute86(integer)
        } else if let Some(offset) = original_end_offset86(ops, original)? {
            if offset > 0 {
                i32::MAX
            } else if offset < i32::MIN + 2 {
                -1
            } else {
                offset - 2
            }
        } else {
            let integer = native_list_index::original_index(ops, original, 0, true)?
                .expect("reported C8.6 option sum");
            absolute86(tcl_syntax::number::native_int32_low_bits(integer))
        };
        if encoded == -1 || encoded == i32::MAX {
            return if report {
                Err(out_of_range(ops, original, version)?)
            } else {
                Ok(0)
            };
        }
        return Ok(encoded);
    }
    let end = 2 * i64::from(i32::MAX);
    let wide = native_list_index::original_index(ops, original, end, true)?
        .expect("reported C9 option index");
    let layout = ops.index_option_layout()?;
    let relative = O::index_offset(original).is_some_and(|offset| offset.encoded() < 0);
    match c9_encoding(wide, relative, layout) {
        C9Encoding::Coordinate(encoded) => Ok(encoded),
        C9Encoding::Sentinel if !report => Ok(-1),
        C9Encoding::Sentinel | C9Encoding::RangeError => Err(out_of_range(ops, original, version)?),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum C9Encoding {
    Coordinate(i32),
    Sentinel,
    RangeError,
}

// Descriptive arithmetic after the actual original GetWideForIndex operation.
// Neither a parsed integer nor this layout value grants native object authority.
fn c9_encoding(wide: i64, relative: bool, layout: NativeOptionIndexLayout) -> C9Encoding {
    let int_max = i64::from(i32::MAX);
    let end = 2 * int_max;
    if relative {
        if layout.size_max > int_max && wide > end - layout.list_max && wide <= int_max {
            C9Encoding::RangeError
        } else if wide > end || wide <= int_max {
            C9Encoding::Sentinel
        } else {
            C9Encoding::Coordinate(tcl_syntax::number::native_int32_low_bits(wide))
        }
    } else if layout.size_max > int_max && wide > int_max && wide < i64::MAX - 1 {
        C9Encoding::RangeError
    } else if wide < 0 || wide > int_max {
        C9Encoding::Sentinel
    } else {
        C9Encoding::Coordinate(i32::try_from(wide).expect("encodable C9 numeric option index"))
    }
}

fn absolute86(integer: i32) -> i32 {
    if integer < 0 { -1 } else { integer }
}

fn original_end_offset86<O: NativeListIndexOps>(
    ops: &mut O,
    original: &O::Value,
) -> Result<Option<i32>, CmdError> {
    if let Some(offset) = O::index_offset(original) {
        if offset.version() != TclVersion::V8_6 {
            return Err(ValueError::CommandProtocolUnavailable(
                "original C8.6 option offset version",
            )
            .into());
        }
        return Ok(Some(tcl_syntax::number::native_int32_low_bits(
            offset.encoded(),
        )));
    }
    let bytes = ops.index_operand_bytes(original)?;
    if bytes.first() != Some(&b'e') || !b"end".starts_with(&bytes[..bytes.len().min(3)]) {
        return Ok(None);
    }
    let offset = if bytes.len() <= 3 {
        0
    } else {
        if bytes.len() <= 4 || !matches!(bytes[3], b'+' | b'-') || bytes[4].is_ascii_whitespace() {
            return Ok(None);
        }
        let input = tcl_core_types::c_string_extent(&bytes[4..]);
        let Some(integer) = ops.index_int_c_string(input)? else {
            return Ok(None);
        };
        if bytes[3] == b'-' {
            integer.wrapping_neg()
        } else {
            integer
        }
    };
    ops.index_install_offset_checked(
        original,
        NativeEndOffset::new(TclVersion::V8_6, i64::from(offset)),
    )?;
    Ok(Some(offset))
}

#[cfg(test)]
mod local_layout_source_controls {
    use super::{C9Encoding, NativeOptionIndexLayout, c9_encoding};

    #[test]
    fn local_list_store_bound_and_c9_encoding_edges_follow_pinned_source_arithmetic() {

        // Source controls: source-windows.json /anchors/19/snippet TclIndexEncode;
        // retained pinned-input-map.json + pinned-input-bytes.tar.gz contain
        // tcl9.1.0/generic/tclInt.h physical-LF 2591..2611 (ListStore/LIST_MAX).
        // The 636 command rows do not measure these large bounds, target layout,
        // foreign library ABI or allocation/callback behaviour.
        let int_max = i64::from(i32::MAX);
        let end = 2 * int_max;
        let narrow = NativeOptionIndexLayout {
            size_max: int_max,
            list_max: 536_870_906,
        };
        let wide = NativeOptionIndexLayout {
            size_max: i64::MAX,
            list_max: 1_152_921_504_606_846_970,
        };
        // Fixed standard layouts: (SIZE_MAX - offsetof(slots)) / sizeof(pointer).
        assert_eq!(narrow.list_max, (int_max - 20) / 4);
        assert_eq!(wide.list_max, (i64::MAX - 40) / 8);
        let local = NativeOptionIndexLayout::local().unwrap();
        if core::mem::size_of::<isize>() == 8 {
            assert_eq!(local.size_max, wide.size_max);
            assert_eq!(local.list_max, wide.list_max);
        } else {
            assert_eq!(core::mem::size_of::<isize>(), 4);
            assert_eq!(local.size_max, narrow.size_max);
            assert_eq!(local.list_max, narrow.list_max);
        }
        assert_eq!(
            c9_encoding(int_max, false, wide),
            C9Encoding::Coordinate(i32::MAX)
        );
        assert_eq!(
            c9_encoding(int_max + 1, false, wide),
            C9Encoding::RangeError
        );
        assert_eq!(
            c9_encoding(i64::MAX - 2, false, wide),
            C9Encoding::RangeError
        );
        assert_eq!(c9_encoding(i64::MAX - 1, false, wide), C9Encoding::Sentinel);
        assert_eq!(
            c9_encoding(int_max + 1, false, narrow),
            C9Encoding::Sentinel
        );
        assert_eq!(c9_encoding(-1, false, wide), C9Encoding::Sentinel);
        let lower = end - wide.list_max;
        assert_eq!(c9_encoding(lower - 1, true, wide), C9Encoding::Sentinel);
        assert_eq!(c9_encoding(lower, true, wide), C9Encoding::Sentinel);
        assert_eq!(c9_encoding(lower + 1, true, wide), C9Encoding::RangeError);
        assert_eq!(c9_encoding(int_max, true, wide), C9Encoding::RangeError);
        assert_eq!(c9_encoding(int_max, true, narrow), C9Encoding::Sentinel);
        assert_eq!(
            c9_encoding(int_max + 1, true, wide),
            C9Encoding::Coordinate(i32::MIN)
        );
        assert_eq!(c9_encoding(end, true, wide), C9Encoding::Coordinate(-2));
        assert_eq!(c9_encoding(end + 1, true, wide), C9Encoding::Sentinel);
    }
}
