// SPDX-License-Identifier: AGPL-3.0-or-later
//! `TclMergeReturnOptions` retains one original dictionary through expansion and removal.

use crate::{
    CmdError,
    return_options::{self, ReturnOptionsOps, ReturnOptionsProtocol},
};

/// Original key/value handles obtained from one native dictionary traversal.
pub type NativeReturnOptionPairs<V> = Vec<(V, V)>;

/// Physical object doors supplied by an independently selected C backend.
pub trait NativeReturnMergeObjects: ReturnOptionsOps {
    /// Working original Dictionary header, with its real insertion history.
    type Dictionary;
    /// Start with the native new empty object and convert that same header.
    fn fresh_dictionary(&mut self) -> Result<Self::Dictionary, CmdError>;
    /// Store originals on that same unshared header.
    fn put(
        &mut self,
        root: &mut Self::Dictionary,
        key: &Self::Value,
        value: &Self::Value,
    ) -> Result<(), CmdError>;
    /// Reach an original member without a string reconstruction.
    fn get(&mut self, root: &Self::Dictionary, key: &[u8])
    -> Result<Option<Self::Value>, CmdError>;
    /// Remove from the same backing, preserving its bucket/history state.
    fn remove(&mut self, root: &mut Self::Dictionary, key: &[u8]) -> Result<(), CmdError>;
    /// C8.5's original Dictionary conversion and iteration order.
    fn dictionary_pairs(
        &mut self,
        value: &Self::Value,
    ) -> Result<NativeReturnOptionPairs<Self::Value>, CmdError>;
    /// Actual remaining Dictionary cardinality.
    fn size(&mut self, root: &Self::Dictionary) -> Result<usize, CmdError>;
    /// Transfer the manufactured original header.
    fn finish(&mut self, root: Self::Dictionary) -> Self::Value;
}

/// The original merged header and the independent native out parameters.
pub struct MergedNativeReturnOptions<V> {
    /// Same manufactured options header, never serialized/reparsed.
    pub options: V,
    /// Native signed completion-code out parameter.
    pub code: i32,
    /// Native signed return-level out parameter.
    pub level: i32,
    /// Actual remaining Dictionary cardinality.
    pub size: usize,
}

fn invalid_options<O: NativeReturnMergeObjects>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    original: &O::Value,
) -> Result<CmdError, CmdError> {
    let bytes = ops.bytes(original)?;
    let bytes = &bytes[..bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len())];
    let mut message = b"bad -options value: expected dictionary but got \"".to_vec();
    message.extend_from_slice(bytes);
    message.push(b'"');
    Ok(protocol.error(message, b"TCL RESULT ILLEGAL_OPTIONS"))
}

/// Expand native original pairs, validate control/metadata and remove controls
/// from the same physical header. No final-options serialization is performed.
///
/// # Errors
/// Preserves actual getter refusals and native option-conversion failures.
pub fn merge<O: NativeReturnMergeObjects>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    arguments: &[O::Value],
) -> Result<MergedNativeReturnOptions<O::Value>, CmdError> {
    if !matches!(
        protocol,
        ReturnOptionsProtocol::Tcl85 | ReturnOptionsProtocol::Tcl86Plus
    ) || !arguments.len().is_multiple_of(2)
    {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native return options merge geometry",
        )
        .into());
    }
    let mut root = ops.fresh_dictionary()?;
    let mut pending = arguments
        .as_chunks::<2>()
        .0
        .iter()
        .rev()
        .map(|pair| (pair[0].clone(), pair[1].clone()))
        .collect::<Vec<_>>();
    while let Some((key, value)) = pending.pop() {
        if ops.bytes(&key)? != b"-options" {
            ops.put(&mut root, &key, &value)?;
            continue;
        }
        if protocol == ReturnOptionsProtocol::Tcl85 {
            let mut nested = value;
            loop {
                let pairs = match ops.dictionary_pairs(&nested) {
                    Ok(pairs) => pairs,
                    Err(error) if error.native_access_refusal().is_some() => return Err(error),
                    Err(_) => return Err(invalid_options(ops, protocol, &nested)?),
                };
                for (key, value) in pairs {
                    ops.put(&mut root, &key, &value)?;
                }
                let Some(next) = ops.get(&root, b"-options")? else {
                    break;
                };
                ops.remove(&mut root, b"-options")?;
                nested = next;
            }
        } else {
            let elements = match ops.list(&value) {
                Ok(elements) if elements.len().is_multiple_of(2) => elements,
                Err(error) if error.native_access_refusal().is_some() => return Err(error),
                _ => return Err(invalid_options(ops, protocol, &value)?),
            };
            pending.extend(
                elements
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .rev()
                    .map(|pair| (pair[0].clone(), pair[1].clone())),
            );
        }
    }
    let mut code = 0;
    let mut level = 1;
    if let Some(value) = ops.get(&root, b"-code")? {
        code = return_options::parse_completion_code(ops, protocol, &value)?;
        ops.remove(&mut root, b"-code")?;
    }
    if let Some(value) = ops.get(&root, b"-level")? {
        level = tcl_syntax::number::native_int32_low_bits(return_options::parse_level(
            ops, protocol, &value,
        )?);
        ops.remove(&mut root, b"-level")?;
    }
    if let Some(value) = ops.get(&root, b"-errorcode")? {
        return_options::validate_list(ops, protocol, &value, false)?;
    }
    if protocol == ReturnOptionsProtocol::Tcl86Plus
        && let Some(value) = ops.get(&root, b"-errorstack")?
    {
        return_options::validate_list(ops, protocol, &value, true)?;
    }
    if code == 2 {
        code = 0;
        level = level.wrapping_add(1);
    }
    let size = ops.size(&root)?;
    Ok(MergedNativeReturnOptions {
        options: ops.finish(root),
        code,
        level,
        size,
    })
}

/// `Tcl_SetReturnOptions` validates the original outer even List before merging.
/// # Errors
/// Preserves original cache effects and the instruction's invalid-dictionary diagnostic.
pub fn merge_stack<O: NativeReturnMergeObjects>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    original: &O::Value,
) -> Result<MergedNativeReturnOptions<O::Value>, CmdError> {
    let elements = match ops.list(original) {
        Ok(elements) if elements.len().is_multiple_of(2) => elements,
        Err(error) if error.native_access_refusal().is_some() => return Err(error),
        _ => {
            let bytes = ops.bytes(original)?;
            let bytes = &bytes[..bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len())];
            let mut message = b"expected dict but got \"".to_vec();
            message.extend_from_slice(bytes);
            message.push(b'"');
            return Err(protocol.error(message, b"TCL RESULT ILLEGAL_OPTIONS"));
        }
    };
    merge(ops, protocol, &elements)
}
