// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original counted Unicode search and the reached native range constructor.

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
pub fn compiled_find<O: ValueOps>(
    ops: &mut O,
    needle: &O::Value,
    subject: &O::Value,
    version: TclVersion,
    reverse: bool,
) -> Result<i64, CmdError> {
    require_protocol(ops, version)?;
    let haystack = ops.native_unicode_units(subject)?;
    let needle = ops.native_unicode_units(needle)?;
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
}

fn reached_range<O: ValueOps>(
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
    let first = usize::try_from(first.max(0))
        .map_err(|_| ValueError::CommandProtocolUnavailable("compiled range coordinate width"))?;
    let Ok(last) = usize::try_from(last) else {
        return Ok(ops.new_bytes(&[]));
    };
    if first >= units.len() || first > last {
        return Ok(ops.new_bytes(&[]));
    }
    let last = last.min(units.len() - 1);
    ops.native_unicode_string_result(units[first..=last].into(), version)
        .map_err(Into::into)
}

/// Execute the range after the original search has retained Unicode backing.
/// Character length and both index getters precede result construction.
///
/// # Errors
/// Declines a missing original Unicode representation or an invalid index.
// Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
pub fn compiled_range<O: ValueOps>(
    ops: &mut O,
    subject: &O::Value,
    first: &O::Value,
    last: &O::Value,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
    require_protocol(ops, version)?;
    let length = ops.native_char_len(subject)?;
    let first = index::resolve_value(ops, first, length)?;
    let last = index::resolve_value(ops, last, length)?;
    reached_range(ops, subject, first, last, version)
}

/// Execute the selected original Tail program with its registered `::` literal.
/// This portable instruction consumer supplies no compiler-registration proof.
///
/// # Errors
/// Propagates original getter, range and native result-constructor failures.
// Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
pub fn compiled_tail<O: ValueOps>(
    ops: &mut O,
    subject: &O::Value,
    separator: &O::Value,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
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
}

/// Execute the reached original character-index branch. Counted one-byte
/// resident units retain their original byte; other units use the same native
/// UTF encoder and a fresh byte-backed result.
///
/// # Errors
/// Declines unreached Unicode, foreign protocol or an invalid selected index.
// Native proof: naming.namespace.original-counted-qualifiers-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
pub fn compiled_index<O: ValueOps>(
    ops: &mut O,
    subject: &O::Value,
    index: &O::Value,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
    require_protocol(ops, version)?;
    let length = ops.native_char_len(subject)?;
    let index = index::resolve_value(ops, index, length)?;
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
    let Ok(index) = usize::try_from(index) else {
        return Ok(ops.new_bytes(&[]));
    };
    let Some(&unit) = units.get(index) else {
        return Ok(ops.new_bytes(&[]));
    };
    if let Some(bytes) = snapshot.resident.filter(|bytes| bytes.len() == length) {
        return Ok(ops.new_bytes(&bytes[index..=index]));
    }
    let bytes = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
        .encode_units(&[unit])
        .ok_or(ValueError::CommandProtocolUnavailable(
            "compiled index selected unit width",
        ))?;
    Ok(ops.new_bytes(&bytes))
}

/// Execute the selected original Qualifiers prefix program. Its colon-run
/// predicate uses the exact counted backing already reached by the search.
///
/// # Errors
/// Propagates the original search, backing and range result-constructor refusal.
// Native proof: naming.namespace.original-counted-qualifiers-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
pub fn compiled_qualifiers<O: ValueOps>(
    ops: &mut O,
    subject: &O::Value,
    separator: &O::Value,
    version: TclVersion,
) -> Result<O::Value, CmdError> {
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
}
