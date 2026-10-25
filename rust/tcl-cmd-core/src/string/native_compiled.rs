// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original counted Unicode search and the reached native range constructor.

use crate::index::NativeIndexOps;
use crate::{CmdError, index};
use tcl_dialect::TclVersion;
use tcl_syntax::native_object::NativeObjectCacheSnapshot;
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::value::{ValueError, ValueOps};

fn require_protocol<O: ValueOps>(ops: &O, version: TclVersion) -> Result<(), CmdError> {
    if version < TclVersion::V8_6
        || ops
            .name_policy_protocol()
            .is_none_or(|policy| policy.string_protocol() != NativeStringProtocol::C(version))
    {
        return Err(
            ValueError::CommandProtocolUnavailable("compiled counted string protocol").into(),
        );
    }
    Ok(())
}

/// Execute the original counted string search; haystack Unicode is reached
/// before needle Unicode. Empty needles remain misses.
///
/// # Errors
/// Returns actual original getter or selected native-protocol failures.
// Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
pub fn compiled_find<O: NativeIndexOps>(
    ops: &mut O,
    needle: &O::Value,
    subject: &O::Value,
    version: TclVersion,
    reverse: bool,
) -> Result<i64, CmdError> {
    index::run_index_metadata(ops, |ops| {
        require_protocol(ops, version)?;
        let haystack = ops.index_c_unicode_units(subject)?;
        let needle = ops.index_c_unicode_units(needle)?;
        if needle.is_empty() || needle.len() > haystack.len() {
            return Ok(-1);
        }
        let mut windows = haystack.windows(needle.len());
        let found = if reverse {
            windows.rposition(|window| window == needle.as_ref())
        } else {
            windows.position(|window| window == needle.as_ref())
        };
        found.map_or(Ok(-1), |index| {
            i64::try_from(index).map_err(|_| {
                ValueError::CommandProtocolUnavailable("compiled string coordinate width").into()
            })
        })
    })
}

fn reached_range<O: NativeIndexOps>(
    ops: &mut O,
    subject: &O::Value,
    first: i64,
    last: i64,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
    require_protocol(ops, version)?;
    let NativeObjectCacheSnapshot::String {
        protocol,
        unicode: Some(units),
        ..
    } = ops.native_object_snapshot(subject)?.cache
    else {
        return Err(ValueError::CommandProtocolUnavailable(
            "compiled range reached Unicode backing",
        )
        .into());
    };
    if protocol != NativeStringProtocol::C(version) {
        return Err(ValueError::CommandProtocolUnavailable(
            "compiled range original Unicode protocol",
        )
        .into());
    }
    // The already reached original Unicode receipt remains required; the
    // same C GetRange constructor/storage owner performs the actual slicing.
    let _ = units;
    super::native_c::parsed_range(ops, subject, first, last, version)
}

/// Execute the range after the original search has retained Unicode backing.
/// Character length and both index getters precede result construction.
///
/// # Errors
/// Declines a missing original Unicode representation or an invalid index.
// Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
pub fn compiled_range<O: NativeIndexOps>(
    ops: &mut O,
    subject: &O::Value,
    first: &O::Value,
    last: &O::Value,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
    index::run_created_index_result(ops, |ops| {
        require_protocol(ops, version)?;
        let length = ops.index_character_count(subject)?;
        let first = index::resolve_value(ops, first, length)?;
        let last = index::resolve_value(ops, last, length)?;
        reached_range(ops, subject, first, last, version)
    })
}

/// Execute the selected original Tail program with its registered `::` literal.
/// This portable instruction consumer supplies no compiler-registration proof.
///
/// # Errors
/// Propagates original getter, range and native result-constructor failures.
// Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
pub fn compiled_tail<O: NativeIndexOps>(
    ops: &mut O,
    subject: &O::Value,
    separator: &O::Value,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
    index::run_created_index_result(ops, |ops| {
        let found = compiled_find(ops, separator, subject, version, true)?;
        let first = if found < 0 {
            found
        } else {
            found
                .checked_add(2)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "compiled Tail coordinate width",
                ))?
        };
        let length = ops.native_char_len(subject)?;
        let last = i64::try_from(length)
            .map_err(|_| ValueError::CommandProtocolUnavailable("compiled Tail character width"))?
            - 1;
        reached_range(ops, subject, first, last, version)
    })
}

/// Execute original `INST_STR_INDEX` from its own checked operand ingress.
/// The namespace instruction retains its independent reached-Unicode consumer.
/// This execution helper grants no compiler registration or erasure authority.
///
/// # Errors
/// Refuses a foreign producer or unavailable original storage, getter or result.
pub fn registered_index_instruction<O: NativeIndexOps>(
    ops: &mut O,
    subject: &O::Value,
    operand: &O::Value,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
    index::run_created_index_result(ops, |ops| {
        if ops.index_c_version() != Some(version)
            || ops
                .name_policy_protocol()
                .is_none_or(|policy| policy.string_protocol() != NativeStringProtocol::C(version))
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "registered String index instruction protocol",
            )
            .into());
        }
        // C84/85 save the original binary pointer before the index getter.
        let captured = if version <= TclVersion::V8_5 {
            ops.index_c_instruction_binary(subject, version)?
        } else {
            None
        };
        let length = match &captured {
            Some((_, bytes)) => bytes.len(),
            None => ops.index_character_count(subject)?,
        };
        let index = index::resolve_value(ops, operand, length)?;
        let Ok(index) = usize::try_from(index) else {
            return Ok(ops.empty());
        };
        if index >= length {
            return Ok(ops.empty());
        }
        if let Some((_, current)) = ops.index_c_select_binary(subject, version)? {
            let bytes = if version <= TclVersion::V8_5 {
                let Some((receipt, original)) = &captured else {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "compiled index original captured binary storage",
                    )
                    .into());
                };
                ops.index_c_check_binary_storage(subject, receipt)?;
                original
            } else {
                &current
            };
            let byte = bytes
                .get(index)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "compiled index selected binary extent",
                ))?;
            return ops.index_c_binary_result(std::rc::Rc::from([*byte]));
        }
        // The VM's one-byte branch reads the current resident extent without
        // forcing a new String primary after the index getter has shimmered it.
        if let Some(bytes) = ops.native_object_snapshot(subject)?.resident
            && bytes.len() == length
        {
            return Ok(ops.new_bytes(&bytes[index..=index]));
        }
        let units = ops.index_c_unicode_units(subject)?;
        let Some(&unit) = units.get(index) else {
            return Ok(ops.empty());
        };
        let selected = if version == TclVersion::V8_6 {
            // TclGetUCS4 suppresses the trailing half and selects both units
            // of a reached pair; TclUCS4ToUtf writes their native C8 encoding.
            if (0xdc00..=0xdfff).contains(&unit)
                && index > 0
                && (0xd800..=0xdbff).contains(&units[index - 1])
            {
                return Ok(ops.empty());
            }
            if (0xd800..=0xdbff).contains(&unit)
                && units
                    .get(index + 1)
                    .is_some_and(|next| (0xdc00..=0xdfff).contains(next))
            {
                &units[index..index + 2]
            } else {
                &units[index..=index]
            }
        } else {
            if version >= TclVersion::V9_0 && unit == u32::MAX {
                return Ok(ops.empty());
            }
            &units[index..=index]
        };
        let bytes = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
            .encode_units(selected)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "compiled index native unit width",
            ))?;
        Ok(ops.new_bytes(&bytes))
    })
}

/// Execute the reached original character-index branch. Counted one-byte
/// resident units retain their original byte; other units use the same native
/// UTF encoder and a fresh byte-backed result.
///
/// # Errors
/// Declines unreached Unicode, foreign protocol or an invalid selected index.
// Native proof: naming.namespace.original-counted-qualifiers-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
pub fn compiled_index<O: NativeIndexOps>(
    ops: &mut O,
    subject: &O::Value,
    index: &O::Value,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
    index::run_created_index_result(ops, |ops| {
        require_protocol(ops, version)?;
        let snapshot = ops.native_object_snapshot(subject)?;
        let NativeObjectCacheSnapshot::String {
            protocol,
            unicode: Some(units),
            ..
        } = snapshot.cache
        else {
            return Err(ValueError::CommandProtocolUnavailable(
                "compiled index reached Unicode backing",
            )
            .into());
        };
        if protocol != NativeStringProtocol::C(version) {
            return Err(ValueError::CommandProtocolUnavailable(
                "compiled index original Unicode protocol",
            )
            .into());
        }
        let _ = (units, snapshot.resident);
        super::native_c::original_index(ops, subject, index, version)
    })
}

/// Execute the selected original Qualifiers prefix program. Its colon-run
/// predicate uses the exact counted backing already reached by the search.
///
/// # Errors
/// Propagates the original search, backing and range result-constructor refusal.
// Native proof: naming.namespace.original-counted-qualifiers-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
pub fn compiled_qualifiers<O: NativeIndexOps>(
    ops: &mut O,
    subject: &O::Value,
    separator: &O::Value,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
    index::run_created_index_result(ops, |ops| {
        let found = compiled_find(ops, separator, subject, version, true)?;
        let NativeObjectCacheSnapshot::String {
            protocol,
            unicode: Some(units),
            ..
        } = ops.native_object_snapshot(subject)?.cache
        else {
            return Err(ValueError::CommandProtocolUnavailable(
                "compiled Qualifiers reached Unicode backing",
            )
            .into());
        };
        if protocol != NativeStringProtocol::C(version) {
            return Err(ValueError::CommandProtocolUnavailable(
                "compiled Qualifiers original Unicode protocol",
            )
            .into());
        }
        let mut last = found
            .checked_sub(1)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "compiled Qualifiers coordinate width",
            ))?;
        while usize::try_from(last)
            .ok()
            .and_then(|index| units.get(index))
            .is_some_and(|unit| *unit == u32::from(b':'))
        {
            last -= 1;
        }
        reached_range(ops, subject, 0, last, version)
    })
}
