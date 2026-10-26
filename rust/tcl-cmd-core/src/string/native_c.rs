// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C String storage, index units and range result constructors.
//!

//! The original operation and operand headers supply authority. Public native
//! rows bound cache-name/resident/result-byte predicates; source and software
//! callback controls independently bound checked storage retirement.
use crate::{
    CmdError,
    index::{self, NativeIndexOps},
};
use std::rc::Rc;
use tcl_dialect::TclVersion;
use tcl_syntax::{
    native_tcl_utf::{NativeCPlainChar, NativeTclUtf},
    value::ValueError,
};

/// Already reached original SetStringFromAny/count storage. Its resident bytes
/// and optional Unicode belong to the same selected original object. This
/// projection itself grants no getter, cache adoption or constructor purpose.
pub struct NativeCStringStorage {
    pub count: usize,
    pub bytes: Option<Rc<[u8]>>,
    pub unicode: Option<Rc<[u32]>>,
}
impl NativeCStringStorage {
    fn fast_index_unit(
        &self,
        index: usize,
        version: TclVersion,
        promotion: NativeCPlainChar,
    ) -> Option<u32> {
        if let Some(unicode) = &self.unicode {
            return unicode.get(index).copied();
        }
        let bytes = self
            .bytes
            .as_ref()
            .filter(|bytes| bytes.len() == self.count)?;
        let byte = *bytes.get(index)?;
        Some(
            if version <= TclVersion::V8_5 && promotion == NativeCPlainChar::Signed {
                u32::from(u16::from_ne_bytes(
                    i16::from(i8::from_ne_bytes([byte])).to_ne_bytes(),
                ))
            } else {
                u32::from(byte)
            },
        )
    }
    fn raw_range(&self, version: TclVersion) -> Option<&[u8]> {
        // C84/85 GetRange selects the resident count==length branch even when
        // Unicode is present. C86+ selects it only before FillUnicodeRep.
        self.bytes.as_deref().filter(|bytes| {
            bytes.len() == self.count && (version <= TclVersion::V8_5 || self.unicode.is_none())
        })
    }
}

/// Genuine C command index producer. C84/85's binary command door precedes
/// index parsing; C86+ independently count first and reach the unit getter.
pub(super) fn original_index<O: NativeIndexOps>(
    ops: &mut O,
    subject: &O::Value,
    operand: &O::Value,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
    if version <= TclVersion::V8_5
        && let Some((receipt, bytes)) = ops.index_c_select_binary(subject, version)?
    {
        let index = index::resolve_value(ops, operand, bytes.len())?;
        let bytes = if version == TclVersion::V8_4 {
            if usize::try_from(index).is_ok_and(|index| index < bytes.len()) {
                ops.index_c_check_binary_storage(subject, &receipt)?;
            }
            bytes
        } else {
            ops.index_c_binary_bytes(subject)?
        };
        return match usize::try_from(index)
            .ok()
            .and_then(|index| bytes.get(index))
        {
            Some(byte) => ops.index_c_binary_result(Rc::from([*byte])),
            None => Ok(ops.empty()),
        };
    }
    let length = ops.index_character_count(subject)?;
    let index = index::resolve_value(ops, operand, length)?;
    let Ok(index) = usize::try_from(index) else {
        return Ok(ops.empty());
    };
    if index >= length {
        return Ok(ops.empty());
    }
    // Modern GetUniChar independently selects the current binary backing.
    if version >= TclVersion::V8_6
        && let Some((_, bytes)) = ops.index_c_select_binary(subject, version)?
    {
        return match bytes.get(index) {
            Some(byte) => ops.index_c_binary_result(Rc::from([*byte])),
            None => Ok(ops.empty()),
        };
    }
    let storage = ops.index_c_string_storage(subject)?;
    let unit =
        if let Some(unit) = storage.fast_index_unit(index, version, ops.index_c_plain_char()?) {
            unit
        } else {
            let units = ops.index_c_unicode_units(subject)?;
            let Some(&unit) = units.get(index) else {
                return Ok(ops.empty());
            };
            unit
        };
    let bytes = NativeTclUtf::for_version(version)
        .encode_units(&[unit])
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original C index unit width",
        ))?;
    Ok(ops.new_bytes(&bytes))
}

/// Same selected `GetRange` producer for command and compiled parsed coordinates.
/// It preserves raw one-byte-per-unit slices and native Unicode constructors;
/// neither route projects through Rust Unicode or aliases the entire C subject.
pub(crate) fn parsed_range<O: NativeIndexOps>(
    ops: &mut O,
    subject: &O::Value,
    first: i64,
    last: i64,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
    let first = usize::try_from(first.max(0))
        .map_err(|_| ValueError::CommandProtocolUnavailable("original C range first width"))?;
    let Ok(last) = usize::try_from(last) else {
        return Ok(ops.empty());
    };
    if version >= TclVersion::V8_6
        && let Some((_, bytes)) = ops.index_c_select_binary(subject, version)?
    {
        return match clamp_range(first, last, bytes.len()) {
            Some((first, last)) => ops.index_c_binary_result(Rc::from(&bytes[first..=last])),
            None => Ok(ops.empty()),
        };
    }
    let storage = ops.index_c_string_storage(subject)?;
    let Some((first, last)) = clamp_range(first, last, storage.count) else {
        return Ok(ops.empty());
    };
    if let Some(bytes) = storage.raw_range(version) {
        return ops.index_c_counted_string_result(Rc::from(&bytes[first..=last]), last - first + 1);
    }
    let units = match storage.unicode {
        Some(units) => units,
        None => ops.index_c_unicode_units(subject)?,
    };
    let slice = units
        .get(first..=last)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original C range Unicode extent",
        ))?;
    ops.index_c_unicode_result(Rc::from(slice), version)
}
fn clamp_range(first: usize, last: usize, length: usize) -> Option<(usize, usize)> {
    if first >= length || first > last {
        return None;
    }
    Some((first, last.min(length - 1)))
}

pub(super) fn original_range<O: NativeIndexOps>(
    ops: &mut O,
    subject: &O::Value,
    first: &O::Value,
    last: &O::Value,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
    let binary = if version <= TclVersion::V8_5 {
        ops.index_c_select_binary(subject, version)?
    } else {
        None
    };
    let length = match &binary {
        Some((_, bytes)) => bytes.len(),
        None => ops.index_character_count(subject)?,
    };
    let first = index::resolve_value(ops, first, length)?.max(0);
    let mut last = index::resolve_value(ops, last, length)?;
    if version < TclVersion::V9_0 {
        last = last.min(
            i64::try_from(length).map_err(|_| {
                ValueError::CommandProtocolUnavailable("original C range length width")
            })? - 1,
        );
        if last < first {
            return Ok(ops.empty());
        }
    } else if last < 0 {
        return Ok(ops.empty());
    }
    if let Some((receipt, bytes)) = binary {
        let bytes = if version == TclVersion::V8_4 {
            ops.index_c_check_binary_storage(subject, &receipt)?;
            bytes
        } else {
            ops.index_c_binary_bytes(subject)?
        };
        let first = usize::try_from(first).map_err(|_| {
            ValueError::CommandProtocolUnavailable("original binary range first width")
        })?;
        let last = usize::try_from(last).map_err(|_| {
            ValueError::CommandProtocolUnavailable("original binary range last width")
        })?;
        let slice = bytes
            .get(first..=last)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "retired original binary range extent",
            ))?;
        return ops.index_c_binary_result(Rc::from(slice));
    }
    parsed_range(ops, subject, first, last, version)
}
