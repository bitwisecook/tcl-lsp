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

//! `-index` paths of `lsearch` and `lsort`: checking each element and drilling into nested lists.

use super::native_ops::{IndexResult, NativeIndexOps, run_index_output};
use super::{
    bad_index_parts, prepare_jim_indices, resolve_for_ops, resolve_jim_encoding, resolve_opt,
    resolve_value,
};
use crate::error::CmdError;
use tcl_dialect::TclVersion;
use tcl_syntax::value::{ValueError, ValueOps};

/// A failed `lsearch`/`lsort -index` path step: C's message and, where C sets
/// one, its `-errorcode`.
pub struct IndexPathError {
    pub message: Vec<u8>,
    pub code: Option<&'static [u8]>,
}

/// Why `TclIndexEncode` refuses an `-index` path element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refusal {
    /// Not index syntax at all.
    Bad,
    /// An index it cannot encode.
    OutOfRange,
}

/// An `-index` path element as `TclIndexEncode` encodes it: an absolute index,
/// or one `end` plus a non-positive offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PathEncoding {
    At(i64),
    FromEnd(i64),
}

impl PathEncoding {
    /// Encode `spec`, or refuse it: out of range is an absolute index outside
    /// `0..=INT_MAX`, `end+N`, or `end-N` with `N` at or past `INT_MAX`
    /// (measured on 64-bit tclsh 9.0.4). End-relativity is told apart by
    /// resolving against two different lengths.
    pub(super) fn of(spec: &str) -> Result<Self, Refusal> {
        const BIG: usize = 1 << 20;
        let r_big = resolve_opt(spec, BIG + 1).ok_or(Refusal::Bad)?;
        let r_small = resolve_opt(spec, 1).ok_or(Refusal::Bad)?;
        let int_max = i64::from(i32::MAX);
        if r_big == r_small {
            return if (0..=int_max).contains(&r_big) {
                Ok(Self::At(r_big))
            } else {
                Err(Refusal::OutOfRange)
            };
        }
        let offset = r_big - i64::try_from(BIG).unwrap_or(i64::MAX);
        if (1 - int_max..=0).contains(&offset) {
            Ok(Self::FromEnd(offset))
        } else {
            Err(Refusal::OutOfRange)
        }
    }

    /// The index into a level of `len` elements.
    pub(super) fn decode(self, len: usize) -> i64 {
        match self {
            Self::At(index) => index,
            Self::FromEnd(offset) => i64::try_from(len).unwrap_or(i64::MAX) - 1 + offset,
        }
    }
}

/// Check every `-index` path element encodes (`TclIndexEncode` at option
/// parse): a syntactically bad spec is `bad index`, an unencodable one `index
/// "…" out of range`.
///
/// # Errors
/// The first element that does not encode.
pub fn check_index_path(path: &[Vec<u8>]) -> Result<(), IndexPathError> {
    for spec in path {
        PathEncoding::of(&String::from_utf8_lossy(spec))
            .map_err(|refusal| path_error(spec, refusal))?;
    }
    Ok(())
}

fn path_error(spec: &[u8], refusal: Refusal) -> IndexPathError {
    match refusal {
        Refusal::Bad => {
            let (message, code) = bad_index_parts(spec);
            IndexPathError { message, code }
        }
        Refusal::OutOfRange => {
            let mut message = b"index \"".to_vec();
            message.extend_from_slice(spec);
            message.extend_from_slice(b"\" out of range");
            IndexPathError {
                message,
                code: Some(b"TCL VALUE INDEX OUTOFRANGE"),
            }
        }
    }
}

/// Drill into a (nested) list `value` by an index `path` (`lsearch`/`lsort
/// -index`) over plain [`ValueOps`]: each spec steps one level. A step past
/// its level is `element <n> missing from sublist "<list>"`, naming the index
/// as `release`'s `SelectObjFromSublists` does ([`failed_step_name`]); a
/// non-list step stops, returning the current value (C's `TclLindexFlat`
/// fallthrough). Upstream's release-aware `drill`; work's native
/// [`drill`]/[`drill_prepared_c`] keep the selected original getters.
///
/// # Errors
/// `bad index` / `out of range`, the list-parse error, or `element … missing
/// from sublist …`.
pub fn drill_in_release<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    path: &[Vec<u8>],
    release: TclVersion,
) -> Result<O::Value, IndexPathError> {
    let mut cur = value.clone();
    for spec in path {
        let len = ops.list_len(&cur).map_err(|e| {
            let error = CmdError::from(e).into_byte_details();
            IndexPathError {
                message: error.message,
                code: None,
            }
        })?;
        let encoded = PathEncoding::of(&String::from_utf8_lossy(spec))
            .map_err(|refusal| path_error(spec, refusal))?;
        let idx = encoded.decode(len);
        if idx < 0 || usize::try_from(idx).unwrap_or(usize::MAX) >= len {
            let named = failed_step_name(encoded, idx, release);
            let mut message = format!("element {named} missing from sublist \"").into_bytes();
            message.extend_from_slice(&ops.as_bytes(&cur));
            message.push(b'"');
            return Err(IndexPathError {
                message,
                code: (release >= TclVersion::V8_6).then_some(b"TCL OPERATION LSORT INDEXFAILED"),
            });
        }
        match ops.list_index(&cur, usize::try_from(idx).unwrap_or(0)) {
            Ok(Some(e)) => cur = e,
            _ => return Ok(cur),
        }
    }
    Ok(cur)
}

/// A selected key and backing constructors needed to keep its actual header live.
/// Guards clone allocation transport only. No key is reconstructed from text.
#[derive(Clone)]
pub struct IndexedKey<K> {
    selected: K,
    created: bool,
    ancestors: Vec<(K, bool)>,
}

/// Borrow the selected value without an extra native reference.
pub fn key_value<O: NativeIndexOps>(key: &IndexedKey<O::IndexKeyOwner>) -> &O::Value {
    O::index_key_value(&key.selected)
}

/// Transport the same selected original into a reached container constructor.
///
/// # Errors
/// The original header or entered operation refuses the selected argument role.
pub fn key_argument<O: NativeIndexOps>(
    ops: &mut O,
    key: &IndexedKey<O::IndexKeyOwner>,
) -> Result<O::Value, CmdError> {
    ops.index_key_argument(&key.selected)
}

/// Transfer the exact selected result before mandatory backing cleanup.
pub fn key_result<O: NativeIndexOps>(
    ops: &mut O,
    key: IndexedKey<O::IndexKeyOwner>,
) -> Result<IndexResult<O::Value>, CmdError> {
    let created_ancestor = key.ancestors.iter().any(|(_, created)| *created);
    if created_ancestor && !key.created {
        let held = ops.index_hold_key_result(&key.selected)?;
        drop(key); // mandatory backing cleanup while the real selected child holds
        return ops.index_finish_held_key_result(held);
    }
    ops.index_key_result(&key.selected, key.created, false)
}

fn step_key<O: NativeIndexOps>(
    ops: &mut O,
    mut current: IndexedKey<O::IndexKeyOwner>,
    selected: IndexResult<O::Value>,
) -> Result<IndexedKey<O::IndexKeyOwner>, CmdError> {
    let (next, created) = match selected {
        IndexResult::Borrowed(member) => {
            let selected = ops.index_borrowed_key(&member);
            if let Err(error) = &selected {
                ops.retain_index_failure(error);
            }
            (selected?, false)
        }
        IndexResult::Created(created)
        | IndexResult::Detached(created)
        | IndexResult::ReturnedOwnedTransport(created) => (ops.index_created_key(created)?, true),
    };
    current.ancestors.push((current.selected, current.created));
    Ok(IndexedKey {
        selected: next,
        created,
        ancestors: current.ancestors,
    })
}

/// Drill a search/sort path while retaining explicit selected constructor roles.
/// Parent backing remains live through the last key use; the key guard supplies
/// no additional native reference while original getters execute.
///
/// # Errors
/// Original conversion/index failures or the complete first Host refusal.
pub fn drill<O: NativeIndexOps>(
    ops: &mut O,
    value: &O::Value,
    path: &[Vec<u8>],
) -> Result<IndexedKey<O::IndexKeyOwner>, CmdError> {
    run_index_output(
        ops,
        |ops| {
            let mut current = IndexedKey {
                selected: ops.index_borrowed_key(value)?,
                created: false,
                ancestors: Vec::new(),
            };
            for spec in path {
                let value = O::index_key_value(&current.selected);
                let len = ops.index_list_length(value)?;
                let spec_text = std::str::from_utf8(spec).map_err(|error| {
                    tcl_syntax::raw_string::UnicodeAccessError {
                        valid_up_to: error.valid_up_to(),
                        error_len: error.error_len(),
                    }
                })?;
                let idx = resolve_for_ops(ops, spec_text, len)?;
                if idx < 0 || usize::try_from(idx).unwrap_or(usize::MAX) >= len {
                    let mut message = b"element ".to_vec();
                    message.extend_from_slice(spec);
                    message.extend_from_slice(b" missing from sublist \"");
                    message.extend_from_slice(&ops.as_bytes(value));
                    message.push(b'"');
                    return Err(CmdError::new_bytes(message));
                }
                match ops.index_list_member(value, usize::try_from(idx).unwrap_or(0)) {
                    Ok(Some(member)) => current = step_key(ops, current, member)?,
                    Err(error) if error.native_execution_refusal().is_some() => return Err(error),
                    _ => return Ok(current),
                }
            }
            Ok(current)
        },
        |_, key| drop(key),
        |error| error,
    )
}

/// Select prepared original C coordinates, without reparsing an index object.
/// Original Length precedes the independently selected member getter.
///
/// # Errors
/// Reached original List/member/diagnostic or complete first Host refusal.
pub fn drill_prepared_c<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::Value,
    path: &[tcl_syntax::native_compiled_index::NativeCompiledListIndex],
) -> Result<IndexedKey<O::IndexKeyOwner>, CmdError> {
    run_index_output(
        ops,
        |ops| {
            let version = ops
                .index_c_version()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "prepared original C option path issuer",
                ))?;
            let mut current = IndexedKey {
                selected: ops.index_borrowed_key(original)?,
                created: false,
                ancestors: Vec::new(),
            };
            for coordinate in path {
                let value = O::index_key_value(&current.selected);
                let reached_length = ops.index_list_length(value);
                if let Err(error) = &reached_length {
                    ops.retain_index_failure(error);
                }
                let length = reached_length?;
                let end = i128::try_from(length).map_err(|_| {
                    let error = CmdError::from(ValueError::CommandProtocolUnavailable(
                        "original option List length",
                    ));
                    ops.retain_index_failure(&error);
                    error
                })? - 1;
                let decoded = coordinate.decode(end);
                let reached_member = match usize::try_from(decoded) {
                    Ok(index) => ops.index_list_member(value, index),
                    Err(_) => Ok(None),
                };
                if let Err(error) = &reached_member {
                    ops.retain_index_failure(error);
                }
                let member = reached_member?;
                if let Some(member) = member {
                    current = step_key(ops, current, member)?;
                    continue;
                }
                let mut message = b"element ".to_vec();
                if version >= TclVersion::V9_0 && decoded < 0 {
                    message.extend_from_slice(
                        format!("end-{}", -2_i64 - i64::from(coordinate.encoded())).as_bytes(),
                    );
                } else if version == TclVersion::V8_4 && decoded < 0 {
                    // 8.4.20 names a step before the first element by its
                    // encoding, `end` being -2 (`lsort -index end-5 {{a b}
                    // {c d}}` names `-7`); 8.5 and 8.6 the decoded index.
                    message.extend_from_slice(coordinate.encoded().to_string().as_bytes());
                } else {
                    message.extend_from_slice(decoded.to_string().as_bytes());
                }
                message.extend_from_slice(b" missing from sublist \"");
                let reached_bytes = ops.index_operand_bytes(value);
                if let Err(error) = &reached_bytes {
                    ops.retain_index_failure(error);
                }
                message.extend_from_slice(tcl_core_types::c_string_extent(&reached_bytes?));
                message.push(b'"');
                return Err(if version >= TclVersion::V8_6 {
                    CmdError::with_error_code_bytes(
                        message,
                        b"TCL OPERATION LSORT INDEXFAILED".to_vec(),
                    )
                } else {
                    CmdError::new_bytes(message)
                });
            }
            Ok(current)
        },
        |_, key| drop(key),
        std::convert::identity,
    )
}

/// Drill Jim's original member path with its independently selected native
/// Index objects. Every original coordinate precedes every container getter.
/// The reached missing-member message is Jim's `CString` diagnostic, without a
/// C errorCode or a native String-result claim.
///
/// # Errors
/// Original conversion, Jim missing-member failure or complete Host refusal.
pub fn drill_original_jim<O: NativeIndexOps>(
    ops: &mut O,
    value: &O::Value,
    indices: &[O::Value],
) -> Result<IndexedKey<O::IndexKeyOwner>, CmdError> {
    run_index_output(
        ops,
        |ops| {
            let encoded = prepare_jim_indices(ops, &indices.iter().collect::<Vec<_>>())?;
            let mut current = IndexedKey {
                selected: ops.index_borrowed_key(value)?,
                created: false,
                ancestors: Vec::new(),
            };
            for (ordinal, original) in indices.iter().enumerate() {
                let members = ops.index_list_elements(key_value::<O>(&current))?;
                let at = match &encoded {
                    Some(encoded) => resolve_jim_encoding(encoded[ordinal], members.len()),
                    None => resolve_value(ops, original, members.len())?,
                };
                if let Some(member) = usize::try_from(at).ok().and_then(|at| members.get(at)) {
                    current = step_key(ops, current, IndexResult::Borrowed(member.clone()))?;
                    continue;
                }
                let encoded = encoded
                    .as_ref()
                    .map_or(at, |indices| i64::from(indices[ordinal]));
                let out_of_range = encoded < 0
                    || usize::try_from(encoded).is_ok_and(|index| index > members.len());
                let spelling = ops.index_operand_bytes(original)?;
                let mut message = if out_of_range {
                    b"index \"".to_vec()
                } else {
                    b"element ".to_vec()
                };
                message.extend_from_slice(tcl_core_types::c_string_extent(&spelling));
                if out_of_range {
                    message.extend_from_slice(b"\" out of range");
                } else {
                    message.extend_from_slice(b" missing from sublist \"");
                    let container = ops.index_operand_bytes(key_value::<O>(&current))?;
                    message.extend_from_slice(tcl_core_types::c_string_extent(&container));
                    message.push(b'"');
                }
                return Err(CmdError::new_bytes(message));
            }
            Ok(current)
        },
        |_, key| drop(key),
        |error| error,
    )
}

/// How `release` names an `-index` step that falls outside its sublist.
/// Measured (`lsort -index end-5 {{a b} {c d}}`, the step decoding to -4):
/// tclsh 9.0.4 and 9.1.0 name the word (`end-5`) when an `end` step falls
/// before the first element; 8.5.19 and 8.6.18 name the decoded index (`-4`);
/// 8.4.20 names its encoding, `end` being -2 (`-7`). An index past the end is
/// the decoded number everywhere.
pub(super) fn failed_step_name(step: PathEncoding, decoded: i64, release: TclVersion) -> String {
    match step {
        PathEncoding::FromEnd(offset) if decoded < 0 => match release {
            TclVersion::V8_4 => (offset - 2).to_string(),
            TclVersion::V8_5 | TclVersion::V8_6 => decoded.to_string(),
            _ => format!("end-{}", -offset),
        },
        _ => decoded.to_string(),
    }
}

/// Whether an index `spec` is *encodable* the way `TclIndexEncode` requires for
/// `lsearch`/`lsort -index`: `Some(true)` for a normal index (an integer in
/// `0..=INT_MAX`, or `end`/`end-N`), `Some(false)` for one it refuses as out of
/// range, and `None` for a syntactically bad spec.
#[must_use]
pub fn encodable(spec: &str) -> Option<bool> {
    match PathEncoding::of(spec) {
        Ok(_) => Some(true),
        Err(Refusal::OutOfRange) => Some(false),
        Err(Refusal::Bad) => None,
    }
}
