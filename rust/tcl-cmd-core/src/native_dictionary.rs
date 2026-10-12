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

//! Dictionary mutation over original keys and prepared physical owners.

use crate::CmdError;

/// The actual caller's original-object conversion and copy ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDictionaryPreparation {
    /// Set/append and compiled immediate mutators copy a shared root first.
    CopyBeforeConversion,
    /// Update/with and existing nested path children convert the original first.
    ConvertBeforeCopy,
    /// Generic increment converts first and drops the copied root's string.
    IncrementCommand,
}

/// Physical dictionary operations. Preparation selects COW before retaining a
/// working handle; member operations keep that same prepared ownership.
pub trait NativeDictionaryObjects {
    /// Original Tcl object handle.
    type Value: Clone;
    /// Closed working owner, retaining its selected native object protocol.
    type Prepared;

    /// Prepare an original dictionary, or create the native empty dictionary.
    fn prepare(&self, original: Option<&Self::Value>) -> Result<Self::Prepared, CmdError>;
    /// Convert an existing path child before selecting its native copy.
    fn prepare_child(&self, original: Option<&Self::Value>) -> Result<Self::Prepared, CmdError>;
    /// Borrow a member after the prepared root's native copy operation.
    fn with_member<R>(
        &self,
        dictionary: &Self::Prepared,
        key: &Self::Value,
        operation: impl FnOnce(Option<&Self::Value>) -> Result<R, CmdError>,
    ) -> Result<R, CmdError>;
    /// Publish an original key and value into the prepared dictionary.
    fn set_member(
        &self,
        dictionary: &mut Self::Prepared,
        key: &Self::Value,
        value: Self::Value,
    ) -> Result<(), CmdError>;
    /// Remove a final key; an absent final key leaves the dictionary intact.
    fn remove_member(
        &self,
        dictionary: &mut Self::Prepared,
        key: &Self::Value,
    ) -> Result<(), CmdError>;
    /// Report a missing intermediate key through the selected native formatter.
    fn missing_key(&self, key: &Self::Value) -> CmdError;
    /// Transfer the working handle to the variable publication owner.
    fn finish(&self, dictionary: Self::Prepared) -> Self::Value;
}

/// Set a nested path, preserving original objects and avoiding native recursion.
///
/// # Errors
/// Preserves reached conversion, physical-operation and capability failures.
pub fn set_path<O: NativeDictionaryObjects>(
    ops: &O,
    original: Option<&O::Value>,
    keys: &[O::Value],
    value: O::Value,
) -> Result<O::Value, CmdError> {
    set_prepared_path(ops, ops.prepare(original)?, keys, value)
}

/// Set a path in an already prepared receiver without repeating its COW decision.
///
/// # Errors
/// Preserves reached native conversion and mutation failures.
pub fn set_prepared_path<O: NativeDictionaryObjects>(
    ops: &O,
    mut current: O::Prepared,
    keys: &[O::Value],
    value: O::Value,
) -> Result<O::Value, CmdError> {
    let Some((last, parents)) = keys.split_last() else {
        return Err(CmdError::new("dictionary path has no keys"));
    };
    let mut frames = Vec::with_capacity(parents.len());
    for key in parents {
        let child = ops.with_member(&current, key, |member| ops.prepare_child(member))?;
        frames.push((current, key));
        current = child;
    }
    ops.set_member(&mut current, last, value)?;
    let mut result = ops.finish(current);
    for (mut parent, key) in frames.into_iter().rev() {
        ops.set_member(&mut parent, key, result)?;
        result = ops.finish(parent);
    }
    Ok(result)
}

/// Remove a nested path. Missing intermediate keys error; a missing leaf does
/// not. Each level retains its original physical dictionary owner.
///
/// # Errors
/// Preserves native lookup, conversion and physical-operation failures.
pub fn remove_path<O: NativeDictionaryObjects>(
    ops: &O,
    original: Option<&O::Value>,
    keys: &[O::Value],
) -> Result<O::Value, CmdError> {
    remove_prepared_path(ops, ops.prepare(original)?, keys)
}

/// Remove a path in an already prepared receiver.
///
/// # Errors
/// Preserves native lookup, conversion and mutation failures.
pub fn remove_prepared_path<O: NativeDictionaryObjects>(
    ops: &O,
    mut current: O::Prepared,
    keys: &[O::Value],
) -> Result<O::Value, CmdError> {
    let Some((last, parents)) = keys.split_last() else {
        return Err(CmdError::new("dictionary path has no keys"));
    };
    let mut frames = Vec::with_capacity(parents.len());
    for key in parents {
        let child = ops.with_member(&current, key, |member| {
            let member = member.ok_or_else(|| ops.missing_key(key))?;
            ops.prepare_child(Some(member))
        })?;
        frames.push((current, key));
        current = child;
    }
    ops.remove_member(&mut current, last)?;
    let mut result = ops.finish(current);
    for (mut parent, key) in frames.into_iter().rev() {
        ops.set_member(&mut parent, key, result)?;
        result = ops.finish(parent);
    }
    Ok(result)
}

/// Transform one borrowed member and publish once into its prepared root.
/// Variable lookup, callbacks and publication are owned by the caller's
/// explicit command or retained-local-cell operation.
///
/// # Errors
/// Preserves failures from the physical dictionary and original member update.
pub fn update_member<O: NativeDictionaryObjects>(
    ops: &O,
    original: Option<&O::Value>,
    key: &O::Value,
    operation: impl FnOnce(Option<&O::Value>) -> Result<O::Value, CmdError>,
) -> Result<O::Value, CmdError> {
    update_prepared_member(ops, ops.prepare(original)?, key, operation)
}

/// Update a borrowed member in an already prepared root.
///
/// # Errors
/// Preserves failures from original member preparation and dictionary publication.
pub fn update_prepared_member<O: NativeDictionaryObjects>(
    ops: &O,
    dictionary: O::Prepared,
    key: &O::Value,
    operation: impl FnOnce(Option<&O::Value>) -> Result<O::Value, CmdError>,
) -> Result<O::Value, CmdError> {
    update_prepared_member_if(ops, dictionary, key, |member| operation(member).map(Some))
}

/// Transform a borrowed member, optionally keeping the prepared dictionary
/// unchanged while its caller still publishes the variable receiver.
///
/// # Errors
/// Preserves reached member conversion and publication failures.
pub fn update_prepared_member_if<O: NativeDictionaryObjects>(
    ops: &O,
    mut dictionary: O::Prepared,
    key: &O::Value,
    operation: impl FnOnce(Option<&O::Value>) -> Result<Option<O::Value>, CmdError>,
) -> Result<O::Value, CmdError> {
    if let Some(value) = ops.with_member(&dictionary, key, operation)? {
        ops.set_member(&mut dictionary, key, value)?;
    }
    Ok(ops.finish(dictionary))
}

/// Transform an existing subtree for dictionary scope writeback. An absent
/// path skips publication; malformed reached dictionaries remain errors.
///
/// # Errors
/// Preserves native child conversion and the reached scope transformation.
pub fn transform_existing_prepared_path<O: NativeDictionaryObjects>(
    ops: &O,
    mut current: O::Prepared,
    keys: &[O::Value],
    operation: impl FnOnce(O::Prepared) -> Result<O::Value, CmdError>,
) -> Result<Option<O::Value>, CmdError> {
    let mut frames = Vec::with_capacity(keys.len());
    for key in keys {
        let child = ops.with_member(&current, key, |member| {
            member
                .map(|value| ops.prepare_child(Some(value)))
                .transpose()
        })?;
        let Some(child) = child else {
            return Ok(None);
        };
        frames.push((current, key));
        current = child;
    }
    let mut result = operation(current)?;
    for (mut parent, key) in frames.into_iter().rev() {
        ops.set_member(&mut parent, key, result)?;
        result = ops.finish(parent);
    }
    Ok(Some(result))
}
