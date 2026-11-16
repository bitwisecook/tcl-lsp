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

//! The edit list between two encoded token arrays.

use super::emission::TOKEN_STRIDE;

/// One minimal edit transforming a previous packed token stream
/// into a new one: starting at integer offset `start`, delete
/// `delete_count` integers and splice in `data`.
///
/// All three fields are token-aligned (multiples of
/// [`TOKEN_STRIDE`]) so the edit splits cleanly into whole
/// `SemanticToken`s, which is what the LSP `semanticTokens/full/
/// delta` wire shape requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenEdit {
    /// Integer offset into the previous `data` where the edit
    /// begins.
    pub start: u32,
    /// Number of integers to remove from the previous `data`.
    pub delete_count: u32,
    /// Replacement integers (the changed run of the new stream).
    pub data: Vec<u32>,
}

/// Compute the single minimal edit that turns `old` into `new`
/// by trimming the common leading and trailing tokens.
///
/// Operates at whole-token granularity: a token counts as common
/// only when its entire 5-integer group is identical, so the
/// returned offsets stay token-aligned.  Because the packed
/// encoding is *relative* (each token's delta is measured from
/// its predecessor), any change that shifts a token's position
/// perturbs its 5-tuple and pulls it into the replacement run —
/// so a prefix/suffix diff on the encoded array is correct
/// without re-deltifying the boundary.
///
/// Returns `None` when the streams are identical.
#[must_use]
pub fn diff(old: &[u32], new: &[u32]) -> Option<TokenEdit> {
    if old == new {
        return None;
    }
    let old_tokens = old.len() / TOKEN_STRIDE;
    let new_tokens = new.len() / TOKEN_STRIDE;
    let token = |buf: &[u32], i: usize| -> [u32; TOKEN_STRIDE] {
        let base = i * TOKEN_STRIDE;
        [
            buf[base],
            buf[base + 1],
            buf[base + 2],
            buf[base + 3],
            buf[base + 4],
        ]
    };
    let max_common = old_tokens.min(new_tokens);
    let mut prefix = 0;
    while prefix < max_common && token(old, prefix) == token(new, prefix) {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < max_common - prefix
        && token(old, old_tokens - 1 - suffix) == token(new, new_tokens - 1 - suffix)
    {
        suffix += 1;
    }
    let start = prefix * TOKEN_STRIDE;
    let delete_count = (old_tokens - prefix - suffix) * TOKEN_STRIDE;
    let data = new[start..(new_tokens - suffix) * TOKEN_STRIDE].to_vec();
    // Token streams are bounded well below `u32::MAX`; on the
    // theoretical overflow, return `None` so the caller falls back to
    // a full token set rather than emitting an invalid edit.
    let (Ok(start), Ok(delete_count)) = (u32::try_from(start), u32::try_from(delete_count)) else {
        return None;
    };
    Some(TokenEdit {
        start,
        delete_count,
        data,
    })
}
