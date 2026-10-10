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
    /// Reach the canonical empty original's binary getter, retaining its string.
    fn empty_binary(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ValueError>;
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

/// The selected Jim `string cat` worker's original-object transfer and append.
/// A sole original is returned without reaching its string getter. Each
/// multiple-operand append reaches that source getter before replacing the
/// fresh receiver with Jim String backing and an unknown character count.
///
/// The caller supplies its checked original getter and validates a sole
/// original's live header. This recipe establishes no engine or call admission.
/// Jim's `OPT_CAT`, `Jim_AppendObj`, and `SetStringFromAny` own this order.
///
/// # Errors
/// Preserves the actual selected getter/updater cause and allocation refusal.
pub fn concatenate_jim<O: NativeAppendObjects>(
    ops: &O,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    inputs: &[O::Value],
    mut original_string: impl FnMut(&O::Value) -> Result<Rc<[u8]>, ValueError>,
) -> Result<O::Value, ValueError> {
    if !protocol.is_jim084() {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim string cat worker",
        ));
    }
    if let [only] = inputs {
        return Ok(only.clone());
    }
    let result = ops.new_string(Rc::from(&b""[..]));
    let mut bytes = Vec::new();
    for input in inputs {
        let original = original_string(input)?;
        checked_size(bytes.len(), original.len(), false)?;
        bytes.extend_from_slice(&original);
        ops.set_string(
            &result,
            protocol,
            Some((
                Rc::from(bytes.as_slice()),
                tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
            )),
            None,
            None,
        )?;
    }
    Ok(result)
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

/// Execute the original C string-concatenation instruction over original objects.
/// Tail access precedes the first getter; an empty tail retains the first header.
///
/// # Errors
/// Preserves reached native storage, updater and capacity failures.
pub fn concatenate_compiled<O: NativeCatObjects>(
    ops: &O,
    string: tcl_syntax::native_string::NativeStringProtocol,
    inputs: &[O::Value],
) -> Result<O::Value, ValueError> {
    use tcl_dialect::TclVersion;
    let version = string
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "C compiled string concatenation",
        ))?;
    if let Some(protocol) = NativeObjectCatProtocol::for_string_protocol(string) {
        return concatenate(ops, protocol, inputs, true);
    }
    let Some((first, tail)) = inputs.split_first() else {
        return Ok(ops.new_string(Rc::from(&b""[..])));
    };
    if tail.is_empty() {
        return Ok(first.clone());
    }
    let snapshots = inputs
        .iter()
        .map(|value| ops.snapshot(value))
        .collect::<Result<Vec<_>, _>>()?;
    let binary = version >= TclVersion::V8_6
        && snapshots.iter().all(|snapshot| {
            if matches!(snapshot.cache, Cache::ByteArray { proper: true, .. }) {
                snapshot.resident.is_none()
            } else {
                snapshot.storage
                    == Some(tcl_syntax::native_string::NativeStringStorageIdentity::CanonicalEmpty)
            }
        });
    let mut appended = Vec::new();
    for (value, snapshot) in tail.iter().zip(&snapshots[1..]) {
        if binary {
            if let Cache::ByteArray {
                bytes,
                proper: true,
            } = &snapshot.cache
            {
                checked_size(appended.len(), bytes.len(), false)?;
                appended.extend_from_slice(bytes);
            }
        } else {
            let bytes = ops.string(value, string)?;
            checked_size(appended.len(), bytes.len(), false)?;
            appended.extend_from_slice(&bytes);
        }
    }
    if appended.is_empty() {
        return Ok(first.clone());
    }
    let reuse = !ops.is_shared(first);
    let mut bytes = if binary {
        match &snapshots[0].cache {
            Cache::ByteArray {
                bytes,
                proper: true,
            } => bytes.to_vec(),
            _ => ops.empty_binary(first, string)?.to_vec(),
        }
    } else {
        ops.string(first, string)?.to_vec()
    };
    checked_size(bytes.len(), appended.len(), false)?;
    bytes.extend_from_slice(&appended);
    let result = if reuse {
        first.clone()
    } else {
        ops.new_string(Rc::from(&b""[..]))
    };
    if binary {
        ops.set_binary(&result, string, Rc::from(bytes))?;
    } else {
        ops.set_plain_string(&result, Rc::from(bytes))?;
    }
    Ok(result)
}
