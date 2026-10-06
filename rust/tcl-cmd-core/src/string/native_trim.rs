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

//! Original trim getters and independently selected result ownership.

use crate::CmdError;
use tcl_dialect::TclVersion;
use tcl_syntax::value::{ValueError, ValueOps};

fn range(
    bytes: &[u8],
    characters: &[u8],
    version: TclVersion,
    left: bool,
    right: bool,
) -> Result<std::ops::Range<usize>, CmdError> {
    tcl_syntax::native_string_trim::trim_range(bytes, characters, version, left, right)
        .ok_or(ValueError::CommandProtocolUnavailable("native trim byte boundary").into())
}

/// Execute the actual trim instruction: characters are converted before subject;
/// a no-cut result retains the same subject, and an effective cut creates a
/// fresh string-only original object. The caller owns both stack operands.
///
/// # Errors
/// Returns the original getter refusal or an unavailable native byte boundary.
pub fn compiled_trim<O: ValueOps>(
    ops: &mut O,
    subject: &O::Value,
    characters: &O::Value,
    version: TclVersion,
    left: bool,
    right: bool,
) -> Result<O::Value, CmdError> {
    let characters = ops.native_concat_string_bytes(characters)?;
    let bytes = ops.native_concat_string_bytes(subject)?;
    let cut = range(&bytes, &characters, version, left, right)?;
    if cut.start == 0 && cut.end == bytes.len() {
        Ok(subject.clone())
    } else {
        Ok(ops.new_bytes(&bytes[cut]))
    }
}

pub(super) fn command_trim<O: ValueOps>(
    ops: &mut O,
    subject: &O::Value,
    characters: Option<&O::Value>,
    version: TclVersion,
    left: bool,
    right: bool,
) -> Result<O::Value, CmdError> {
    let characters = characters
        .map(|value| ops.native_string_bytes(value))
        .transpose()?;
    let characters = characters
        .as_deref()
        .unwrap_or(tcl_syntax::native_string_trim::default_trim_set(version));
    let bytes = ops.native_string_bytes(subject)?;
    let cut = range(&bytes, characters, version, left, right)?;
    Ok(ops.new_bytes(&bytes[cut]))
}
