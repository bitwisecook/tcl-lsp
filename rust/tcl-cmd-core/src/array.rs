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

//! `array` ensemble cores — the *read-side* + `unset` of the `array` command,
//! shared once over [`VarStore`] (and its [`array_keys`](VarStore::array_keys)
//! enumeration rung) + [`ValueOps`].
//!
//! `array exists`/`size`/`names`/`get`/`unset` are value→value-ish reads over the
//! variable store, so they live here. `array set`'s element store fires a write
//! trace **per element** that must fail the command (C's `Tcl_ArraySetCmd`), and
//! the contract's [`VarStore::set_elem`] is storage-only (it discards the trace
//! outcome) — so, exactly like `incr`/`append`, the `set` store stays in each
//! adapter. `array default` uses [`default_at`] after the adapter locates its target.
//! `array for` (a Family-B iteration with an eval body) stays per-adapter; this core returns `None` for anything
//! it does not handle, so the caller falls back (and owns the unknown-subcommand
//! message, whose option list differs per runtime).
//!
//! `array unset a` with no pattern removes the **whole array** — not
//! iterate-and-unset over each element, which would leave an empty array
//! behind.
//!
//! Semantics follow tclsh 9.0.

use tcl_runtime_api::{
    ArrayElementRead, ArrayInvalidation, ArrayReadMiss, ArrayTarget, Frames, VarStore,
    VarUnsetError,
};
use tcl_syntax::glob::string_match_bytes;
use tcl_syntax::value::{ValueError, ValueOps};

use crate::error::CmdError;

fn confined_store_error(operation: &[u8], name: &[u8]) -> CmdError {
    let mut message = b"can't ".to_vec();
    message.extend_from_slice(operation);
    message.extend_from_slice(b" \"");
    message.extend_from_slice(name);
    message.extend_from_slice(b"\": stores are confined to the activation");
    let code = if operation == b"unset" {
        b"TCL UNSET VARNAME"
    } else {
        b"TCL WRITE VARNAME"
    };
    CmdError::with_error_code_bytes(message, code.to_vec())
}

/// Dispatch an `array` subcommand handled by the shared core. `rest` is the
/// arguments after the subcommand (`rest[0]` is the array name). Returns `None`
/// for `set`/`default`/`for` and any unknown subcommand, letting the adapter
/// handle them. Handled commands return [`ArrayCommandResult`] rather than the
/// bare value so callers cannot silently discard retained read metadata.
///
/// ```compile_fail
/// # use tcl_runtime_api::{Frames, VarStore};
/// # use tcl_syntax::value::ValueOps;
/// fn lossy<O, V>(ops: &mut O, sub: &str, rest: &[V]) -> V
/// where
///     O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
///     V: Clone,
/// {
///     tcl_cmd_core::array::dispatch(ops, sub, rest)
///         .expect("handled")
///         .expect("success")
/// }
/// ```
pub fn dispatch<O, V>(
    ops: &mut O,
    sub: &str,
    rest: &[V],
) -> Option<Result<ArrayCommandResult<V>, CmdError>>
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
    V: Clone,
{
    dispatch_at(ops, sub, rest, None)
}

/// A shared `array` result plus any options retained by swallowed element
/// reads. Most subcommands carry no options; keeping the exceptional metadata
/// here lets both runtime adapters construct their native completion without
/// moving command assembly out of this shared owner.
#[derive(Debug, Clone)]
pub struct ArrayCommandResult<V> {
    /// The ordinary Tcl result value.
    pub value: V,
    /// Metadata from the last failed candidate read, if any.
    pub read_miss: Option<ArrayReadMiss>,
}

impl<V> ArrayCommandResult<V> {
    fn plain(value: V) -> Self {
        Self {
            value,
            read_miss: None,
        }
    }
}

/// [`dispatch`] with an array target located before the operation trace fired.
/// Tcl retains that cell for existence/key enumeration, while value reads and
/// whole-array mutation deliberately continue through the live spelling.
pub fn dispatch_at<O, V>(
    ops: &mut O,
    sub: &str,
    rest: &[V],
    located: Option<&ArrayTarget>,
) -> Option<Result<ArrayCommandResult<V>, CmdError>>
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
    V: Clone,
{
    dispatch_bytes_at(ops, sub.as_bytes(), rest, located)
}

/// Dispatch an already selected subcommand without projecting its native
/// bytes through a Unicode display. The adapter owns selection and admission;
/// this owner applies the shared array operation to the selected spelling.
pub fn dispatch_bytes_at<O, V>(
    ops: &mut O,
    sub: &[u8],
    rest: &[V],
    located: Option<&ArrayTarget>,
) -> Option<Result<ArrayCommandResult<V>, CmdError>>
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
    V: Clone,
{
    match sub {
        b"exists" => Some(match rest {
            [n] => {
                let name = match ops.native_string_bytes(n) {
                    Ok(name) => name,
                    Err(error) => return Some(Err(error.into())),
                };
                let target = match locate(ops, &name, located) {
                    Ok(target) => target,
                    Err(error) => return Some(Err(error.into())),
                };
                match present_keys(ops, &target) {
                    Ok(keys) => ops
                        .array_existence_result(keys.is_some())
                        .map(ArrayCommandResult::plain)
                        .map_err(CmdError::from),
                    Err(error) => Err(error),
                }
            }
            _ => Err(CmdError::wrong_args("array exists arrayName")),
        }),
        b"size" => Some(match rest {
            [n] => {
                let name = match ops.native_string_bytes(n) {
                    Ok(name) => name,
                    Err(error) => return Some(Err(error.into())),
                };
                let target = match locate(ops, &name, located) {
                    Ok(target) => target,
                    Err(error) => return Some(Err(error.into())),
                };
                let count = match present_keys(ops, &target) {
                    Ok(keys) => keys.map_or(0, |keys| keys.len()),
                    Err(error) => return Some(Err(error)),
                };
                Ok(ArrayCommandResult::plain(
                    ops.new_int(i64::try_from(count).unwrap_or(i64::MAX)),
                ))
            }
            _ => Err(CmdError::wrong_args("array size arrayName")),
        }),
        b"names" => Some(match rest {
            [n] => names(ops, n, None, located).map(ArrayCommandResult::plain),
            [n, p] => names(ops, n, Some(p), located).map(ArrayCommandResult::plain),
            _ => Err(CmdError::wrong_args("array names arrayName ?pattern?")),
        }),
        b"get" => Some(match rest {
            [n] => get(ops, n, None, located),
            [n, p] => get(ops, n, Some(p), located),
            _ => Err(CmdError::wrong_args("array get arrayName ?pattern?")),
        }),
        b"unset" => Some(match rest {
            [n] => unset(ops, n, None, located).map(ArrayCommandResult::plain),
            [n, p] => unset(ops, n, Some(p), located).map(ArrayCommandResult::plain),
            _ => Err(CmdError::wrong_args("array unset arrayName ?pattern?")),
        }),
        _ => None,
    }
}

/// Completed array-default operation or a request to render the actual
/// original invocation's argument-count diagnostic.
#[derive(Debug, Clone)]
pub enum ArrayDefaultCommandResult<V> {
    /// Native command result, including the original default object for `get`.
    Value(V),
    /// The selected worker's usage suffix after its command and option words.
    WrongArguments {
        /// Native usage suffix; the adapter supplies the original header.
        suffix: &'static [u8],
    },
}

/// Select the actual Tcl 9 option-table word before locating the array.
/// Input is the original object's checked native string, not binary backing.
pub fn prepare_default_option(
    _protocol: tcl_runtime_api::NativeArrayDefaultProtocol,
    bytes: &[u8],
) -> Result<&'static [u8], CmdError> {
    const OPTIONS: crate::prefix::OptionTable<'static, &[u8]> =
        crate::prefix::OptionTable::abbreviating("option", &[b"get", b"set", b"exists", b"unset"]);
    let index = OPTIONS.index_of_cmd(tcl_core_types::c_string_extent(bytes))?;
    Ok(OPTIONS.names()[index])
}

/// Select the option on the same original object, including a genuine native
/// Index hit before any string getter. The protocol retains the actual worker.
pub fn prepare_default_option_original<O: ValueOps>(
    ops: &mut O,
    _protocol: tcl_runtime_api::NativeArrayDefaultProtocol,
    original: &O::Value,
) -> Result<&'static [u8], CmdError> {
    static NAMES: [&str; 4] = ["get", "set", "exists", "unset"];
    static OPTIONS: crate::prefix::OptionTable<'static> =
        crate::prefix::OptionTable::abbreviating("option", &NAMES);
    let index = OPTIONS.index_of_original(ops, original)?;
    Ok(NAMES[index].as_bytes())
}

/// Apply an already selected option after the array-operation callbacks.
/// `located` retains the original root for get/exists/unset; set performs the
/// native fresh creating lookup. The recipe does not authenticate an engine.
pub fn default_at<O, V>(
    ops: &mut O,
    _protocol: tcl_runtime_api::NativeArrayDefaultProtocol,
    option: &[u8],
    rest: &[V],
    located: Option<&ArrayTarget>,
) -> Result<ArrayDefaultCommandResult<V>, CmdError>
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
    V: Clone,
{
    use tcl_runtime_api::{ArrayDefaultSetFailure, ArrayDefaultState};
    let count = if option == b"set" { 2 } else { 1 };
    if rest.len() != count {
        return Ok(ArrayDefaultCommandResult::WrongArguments {
            suffix: if option == b"set" {
                b"arrayName value"
            } else {
                b"arrayName"
            },
        });
    }
    let name = ops.native_string_bytes(&rest[0])?;
    let here = Frames::current(ops);
    if matches!(option, b"set" | b"unset") && ops.unset_confined_bytes(here, &name)? {
        return Err(confined_store_error(option, &name));
    }
    if option == b"set" {
        let frame = Frames::current(ops);
        match ops.set_array_default_bytes(frame, &name, rest[1].clone())? {
            Ok(()) => return Ok(ArrayDefaultCommandResult::Value(ops.new_bytes(b""))),
            Err(ArrayDefaultSetFailure::Lookup(diagnostic)) => {
                return Err(variable_diagnostic_error(
                    ops,
                    "array default set",
                    diagnostic,
                ));
            }
            Err(failure @ (ArrayDefaultSetFailure::Element | ArrayDefaultSetFailure::Scalar)) => {
                let name = tcl_core_types::c_string_extent(&name);
                let mut message = b"can't array default set \"".to_vec();
                message.extend_from_slice(name);
                message.extend_from_slice(b"\": variable isn't array");
                let code = if matches!(failure, ArrayDefaultSetFailure::Element) {
                    error_code_words(ops, &[b"TCL", b"LOOKUP", b"VARNAME", name])
                } else {
                    b"TCL WRITE ARRAY".to_vec()
                };
                return Err(CmdError::with_byte_error_details(message, code, None, None));
            }
        }
    }
    let target = locate(ops, &name, located)?;
    let state = ops.array_default_state_at(&target)?;
    let not_array = |ops: &mut O| {
        let name = tcl_core_types::c_string_extent(&name);
        let mut message = b"\"".to_vec();
        message.extend_from_slice(name);
        message.extend_from_slice(b"\" isn't an array");
        let code = error_code_words(ops, &[b"TCL", b"LOOKUP", b"ARRAY", name]);
        CmdError::with_byte_error_details(message, code, None, None)
    };
    let value = match (option, state) {
        (
            b"get",
            ArrayDefaultState::Array {
                default: Some(value),
            },
        ) => value,
        (b"get", ArrayDefaultState::Array { default: None }) => {
            return Err(CmdError::with_error_code(
                "array has no default value",
                "TCL READ ARRAY DEFAULT",
            ));
        }
        (b"get", _) | (_, ArrayDefaultState::NonArray) => return Err(not_array(ops)),
        (b"exists", ArrayDefaultState::Undefined) => ops.new_bool(false),
        (b"exists", ArrayDefaultState::Array { default }) => ops.new_bool(default.is_some()),
        (b"unset", ArrayDefaultState::Array { .. }) => {
            ops.unset_array_default_at(&target)?;
            ops.new_bytes(b"")
        }
        (b"unset", ArrayDefaultState::Undefined) => ops.new_bytes(b""),
        _ => {
            return Err(
                ValueError::CommandProtocolUnavailable("array default selected option").into(),
            );
        }
    };
    Ok(ArrayDefaultCommandResult::Value(value))
}

fn error_code_words<O: ValueOps>(ops: &mut O, words: &[&[u8]]) -> Vec<u8> {
    let words = words.iter().map(|word| ops.new_bytes(word)).collect();
    let value = ops.new_list(words);
    ops.pin_value(&value);
    let bytes = ops.as_bytes(&value).to_vec();
    ops.unpin_value(&value);
    bytes
}

fn locate<O: VarStore + Frames>(
    ops: &O,
    name: &[u8],
    located: Option<&ArrayTarget>,
) -> Result<ArrayTarget, tcl_syntax::value::ValueError> {
    located
        .cloned()
        .map_or_else(|| ops.array_target_bytes(Frames::current(ops), name), Ok)
}

/// `array names arrayName ?pattern?` — element names (glob-filtered).
fn names<O, V>(
    ops: &mut O,
    name: &V,
    pattern: Option<&V>,
    located: Option<&ArrayTarget>,
) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
{
    let name = ops.native_string_bytes(name)?;
    let pattern = pattern.map(|p| ops.native_string_bytes(p)).transpose()?;
    let target = locate(ops, &name, located)?;
    let keys = ops.array_key_bytes_checked_at(&target)?.unwrap_or_default();
    let items: Vec<V> = keys
        .iter()
        .filter(|k| pattern.as_deref().is_none_or(|p| string_match_bytes(p, k)))
        .map(|k| ops.new_bytes(k))
        .collect();
    Ok(ops.new_list(items))
}

/// `array get arrayName ?pattern?` — a flat `key value …` list (glob-filtered on
/// the key).
fn get<O, V>(
    ops: &mut O,
    name: &V,
    pattern: Option<&V>,
    located: Option<&ArrayTarget>,
) -> Result<ArrayCommandResult<V>, CmdError>
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
    V: Clone,
{
    use tcl_syntax::naming::{
        NativeVariableDiagnosticOperation::Read, NativeVariableDiagnosticReason as Reason,
        NativeVariableFailureSite::ValueRead, NativeVariableInputForm as Input,
    };
    let name = ops.native_string_bytes(name)?;
    let pattern = pattern.map(|p| ops.native_string_bytes(p)).transpose()?;
    let target = locate(ops, &name, located)?;
    let keys = ops.array_key_bytes_checked_at(&target)?.unwrap_or_default();
    let mut items: Vec<V> = Vec::with_capacity(keys.len() * 2);
    let mut pinned = Vec::with_capacity(keys.len());
    let mut read_miss = None;
    for k in &keys {
        if pattern
            .as_deref()
            .is_some_and(|p| !string_match_bytes(p, k))
        {
            continue;
        }
        let read = match ops.array_read_elem_bytes_at(&target, k) {
            Ok(read) => read,
            Err(error) => {
                for value in &pinned {
                    ops.unpin_value(value);
                }
                return Err(error.into());
            }
        };
        match read {
            ArrayElementRead::Value(value) => {
                // `array_read_elem_at` returns a transiently-owned handle: it
                // has to acquire that hold before releasing its selected cell,
                // closing the raw-pointer handoff gap. `new_list` takes its own
                // ownership before the holds are released below.
                pinned.push(value.clone());
                items.push(ops.new_bytes(k));
                items.push(value);
            }
            ArrayElementRead::Missing(miss) => {
                read_miss = Some(
                    read_miss.map_or(miss.clone(), |prior: ArrayReadMiss| prior.followed_by(miss)),
                );
            }
            ArrayElementRead::TraceError(failure) => {
                for value in &pinned {
                    ops.unpin_value(value);
                }
                let (message, code, info, line) = failure.into_parts();
                return Err(CmdError::with_byte_error_details(message, code, info, line));
            }
            ArrayElementRead::ArrayInvalidated(invalidation) => {
                for value in &pinned {
                    ops.unpin_value(value);
                }
                let reason = match invalidation {
                    ArrayInvalidation::Unset => Reason::NoSuchVariable,
                    ArrayInvalidation::Retyped => Reason::NoSuchElement,
                };
                let diagnostic = ops.variable_diagnostic_at(
                    Read,
                    reason,
                    ValueRead,
                    Input::Separate {
                        root: &name,
                        element: Some(k),
                    },
                )?;
                return Err(variable_diagnostic_error(ops, "read", diagnostic));
            }
        }
    }
    let value = ops.new_list(items);
    for candidate in &pinned {
        ops.unpin_value(candidate);
    }
    Ok(ArrayCommandResult { value, read_miss })
}

/// `array unset arrayName ?pattern?` — remove matching elements, or (no pattern)
/// the whole array.
fn unset<O, V>(
    ops: &mut O,
    name: &V,
    pattern: Option<&V>,
    located: Option<&ArrayTarget>,
) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
{
    let here = Frames::current(ops);
    let name = ops.native_string_bytes(name)?;
    if ops.unset_confined_bytes(here, &name)? {
        return Err(confined_store_error(b"unset", &name));
    }
    let dictionary =
        ops.variable_container_model() == tcl_dialect::VariableContainerModel::DictionaryValue;
    let pattern = pattern.map(|p| ops.native_string_bytes(p)).transpose()?;
    let target = locate(ops, &name, located)?;
    if pattern.is_none() || (dictionary && pattern.as_deref() == Some(b"*")) {
        // Jim's wildcard removes its whole dictionary root, including malformed
        // contents. C's wildcard removes members and retains the empty array.
        if present_keys(ops, &target)?.is_some() || (dictionary && ops.exists_bytes(here, &name)?) {
            ops.unset_command_bytes(here, &name)?
                .map_err(|error| match error {
                    VarUnsetError::IsConstant => {
                        use tcl_syntax::naming::{
                            NativeVariableDiagnosticOperation::Unset,
                            NativeVariableDiagnosticReason::Constant,
                            NativeVariableFailureSite::ValueUnset,
                            NativeVariableInputForm::Combined,
                        };
                        match ops.variable_diagnostic_at(
                            Unset,
                            Constant,
                            ValueUnset,
                            Combined(&name),
                        ) {
                            Ok(diagnostic) => variable_diagnostic_error(ops, "unset", diagnostic),
                            Err(error) => error.into(),
                        }
                    }
                })?;
        }
    } else if let Some(pattern) = pattern {
        for key in present_keys(ops, &target)?.unwrap_or_default() {
            if string_match_bytes(&pattern, &key) {
                ops.unset_elem_bytes_at(&target, &key)?;
            }
        }
    }
    Ok(ops.empty())
}

fn variable_diagnostic_error<O: ValueOps>(
    ops: &mut O,
    verb: &str,
    diagnostic: tcl_syntax::naming::NativeVariableDiagnosticProjection,
) -> CmdError {
    let mut message = format!("can't {verb} \"").into_bytes();
    message.extend_from_slice(&diagnostic.name);
    message.extend_from_slice(b"\": ");
    message.extend_from_slice(diagnostic.reason.message().as_bytes());
    if let Some(words) = diagnostic.error_code {
        let words: Vec<_> = words.iter().map(Vec::as_slice).collect();
        let code = error_code_words(ops, &words);
        CmdError::with_byte_error_details(message, code, None, None)
    } else {
        CmdError::new_bytes(message)
    }
}

/// Native presence/count/unset queries treat an undecodable dictionary as a
/// non-array. Reached host refusal remains outside that guest result.
fn present_keys<O: VarStore>(
    ops: &O,
    target: &ArrayTarget,
) -> Result<Option<Vec<Vec<u8>>>, CmdError> {
    match ops.array_key_bytes_checked_at(target) {
        Ok(keys) => Ok(keys),
        Err(
            ValueError::ListParse { .. }
            | ValueError::DictionaryParse { .. }
            | ValueError::MissingDictionaryValue,
        ) if ops.variable_container_model()
            == tcl_dialect::VariableContainerModel::DictionaryValue =>
        {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}
