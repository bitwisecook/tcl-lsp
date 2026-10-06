// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object `TclStringCat`, preserving native materialisation order.

use std::rc::Rc;

use crate::native_append::NativeAppendObjects;
use tcl_syntax::native_object::{
    NativeObjectCacheSnapshot as Cache, NativeObjectSnapshot as Snapshot,
    NativeObjectStringEmptiness as Empty, native_c9_string_emptiness,
};
use tcl_syntax::native_object_append::{NativeObjectCatMode as Mode, NativeObjectCatProtocol};
use tcl_syntax::value::ValueError;

/// Native concatenation operations beyond the append object's physical seam.
pub trait NativeCatObjects: NativeAppendObjects {
    /// Observe actual original sharing before retaining the result handle.
    fn is_shared(&self, value: &Self::Value) -> bool;
    /// Install resident string bytes with NULL primary representation.
    fn set_plain_string(&self, value: &Self::Value, bytes: Rc<[u8]>) -> Result<(), ValueError>;
}

fn checked_size(current: usize, extra: usize, unicode: bool) -> Result<usize, ValueError> {
    let limit = if unicode {
        isize::MAX as usize / 4
    } else {
        isize::MAX as usize
    };
    current
        .checked_add(extra)
        .filter(|size| *size <= limit)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native concatenation allocation size",
        ))
}

struct NativeCatLayout {
    first: usize,
    last: usize,
    length: usize,
}

fn select_byte_layout<O: NativeCatObjects>(
    ops: &O,
    inputs: &[O::Value],
    string: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<NativeCatLayout, ValueError> {
    let mut first;
    let mut last;
    let mut length = 0;
    let mut next = 0;
    loop {
        let mut pending = None;
        loop {
            let value = &inputs[next];
            let snapshot = ops.snapshot(value)?;
            let empty = native_c9_string_emptiness(&snapshot).map_err(|_| {
                ValueError::CommandProtocolUnavailable("native concatenation empty storage")
            })?;
            if snapshot.resident.is_none() && empty != Empty::Empty {
                pending = Some(next);
            } else {
                length = ops.string(value, string)?.len();
            }
            next += 1;
            if next == inputs.len() || length != 0 || pending.is_some() {
                break;
            }
        }
        first = next - 1;
        last = first;
        if next < inputs.len() && length == 0 {
            let pending = pending.expect("native pending value before following operands");
            let following_length;
            loop {
                let bytes = ops.string(&inputs[next], string)?;
                next += 1;
                if next == inputs.len()
                    || !bytes.is_empty()
                    || ops.snapshot(&inputs[pending])?.resident.is_some()
                {
                    following_length = bytes.len();
                    break;
                }
            }
            if following_length != 0 {
                last = next - 1;
            }
            if next < inputs.len() || following_length != 0 {
                length = ops.string(&inputs[pending], string)?.len();
            }
            if length == 0 && following_length != 0 {
                first = last;
            }
            length = checked_size(length, following_length, false)?;
        }
        if next == inputs.len() || length != 0 {
            break;
        }
    }
    while next < inputs.len() {
        let bytes = ops.string(&inputs[next], string)?;
        if !bytes.is_empty() {
            last = next;
            length = checked_size(length, bytes.len(), false)?;
        }
        next += 1;
    }
    Ok(NativeCatLayout {
        first,
        last,
        length,
    })
}

fn select_layout<O: NativeCatObjects>(
    ops: &O,
    inputs: &[O::Value],
    snapshots: &[Snapshot],
    mode: Mode,
    string: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<NativeCatLayout, ValueError> {
    let mut first = inputs.len() - 1;
    let mut last = 0;
    let mut length = 0;
    match mode {
        Mode::Binary => {
            for (index, snapshot) in snapshots.iter().enumerate() {
                if let Cache::ByteArray {
                    bytes,
                    proper: true,
                } = &snapshot.cache
                    && !bytes.is_empty()
                {
                    last = index;
                    if length == 0 {
                        first = last;
                    }
                    length = checked_size(length, bytes.len(), false)?;
                }
            }
        }
        Mode::Unicode => {
            for (index, value) in inputs.iter().enumerate() {
                let snapshot = ops.snapshot(value)?;
                if snapshot
                    .resident
                    .as_ref()
                    .is_none_or(|bytes| !bytes.is_empty())
                {
                    let units = ops.unicode(value, string)?;
                    if !units.is_empty() {
                        last = index;
                        if length == 0 {
                            first = last;
                        }
                        length = checked_size(length, units.len(), true)?;
                    }
                }
            }
        }
        Mode::Bytes => return select_byte_layout(ops, inputs, string),
    }
    Ok(NativeCatLayout {
        first,
        last,
        length,
    })
}

/// Concatenate original operands using the actual C9 representation recipe.
/// `in_place` permits mutation of the first nonempty original only when its
/// actual sharing allows it. A single effective operand is returned unchanged.
///
/// # Errors
/// Preserves reached native storage, updater and allocation-capacity refusals.
pub fn concatenate<O: NativeCatObjects>(
    ops: &O,
    protocol: NativeObjectCatProtocol,
    inputs: &[O::Value],
    in_place: bool,
) -> Result<O::Value, ValueError> {
    match inputs {
        [] => return Ok(ops.new_string(Rc::from(&b""[..]))),
        [only] => return Ok(only.clone()),
        _ => {}
    }
    let snapshots = inputs
        .iter()
        .map(|value| ops.snapshot(value))
        .collect::<Result<Vec<_>, _>>()?;
    let mode = protocol.mode(&snapshots);
    let string = protocol.string_protocol();
    let NativeCatLayout {
        first,
        last,
        length,
    } = select_layout(ops, inputs, &snapshots, mode, string)?;
    if last <= first {
        return Ok(inputs[first].clone());
    }
    let reuse = in_place && !ops.is_shared(&inputs[first]);
    let result = if reuse {
        inputs[first].clone()
    } else {
        ops.new_string(Rc::from(&b""[..]))
    };
    match mode {
        Mode::Binary => {
            let mut bytes = Vec::with_capacity(length);
            for value in &inputs[first..=last] {
                if let Cache::ByteArray {
                    bytes: payload,
                    proper: true,
                } = ops.snapshot(value)?.cache
                {
                    bytes.extend_from_slice(&payload);
                }
            }
            ops.set_binary(&result, string, Rc::from(bytes))?;
        }
        Mode::Unicode => {
            let mut units = Vec::with_capacity(length);
            for value in &inputs[first..=last] {
                let snapshot = ops.snapshot(value)?;
                if snapshot
                    .resident
                    .as_ref()
                    .is_none_or(|bytes| !bytes.is_empty())
                {
                    units.extend_from_slice(&ops.unicode(value, string)?);
                }
            }
            ops.set_string(&result, string, None, Some(length), Some(Rc::from(units)))?;
        }
        Mode::Bytes => {
            let mut bytes = Vec::with_capacity(length);
            for value in &inputs[first..=last] {
                let snapshot = ops.snapshot(value)?;
                if snapshot
                    .resident
                    .as_ref()
                    .is_none_or(|bytes| !bytes.is_empty())
                {
                    bytes.extend_from_slice(&ops.string(value, string)?);
                }
            }
            if reuse {
                ops.set_plain_string(&result, Rc::from(bytes))?;
            } else {
                ops.set_string(
                    &result,
                    string,
                    Some((
                        Rc::from(bytes),
                        tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                    )),
                    None,
                    None,
                )?;
            }
        }
    }
    Ok(result)
}
