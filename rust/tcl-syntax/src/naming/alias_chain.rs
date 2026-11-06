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

//! Alias-chain topology over caller-owned current command identities.

/// Follow actual alias edges until a missing/non-alias target or a cycle.
/// The callback owns lookup, interpreter lifetime and native naming purposes.
/// An error retains unknown topology; it is never treated as a missing target.
///
/// # Errors
/// Returns any lookup or lifetime failure supplied by the caller.
pub fn alias_chain_loops<K: Clone + Eq, E>(
    start: K,
    mut next: impl FnMut(&K) -> Result<Option<K>, E>,
) -> Result<bool, E> {
    let mut seen = vec![start.clone()];
    let mut current = start;
    while let Some(selected) = next(&current)? {
        if seen.contains(&selected) {
            return Ok(true);
        }
        seen.push(selected.clone());
        current = selected;
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_alias_chain_keeps_missing_unknown_and_cycles_separate() {
        // Implementation contract: naming.alias.original-fresh-creation-transfer
        // docs/design/analysis/name-resolution-proofs/original-fresh-alias-creation-transfer.md
        assert_eq!(
            alias_chain_loops(0, |node| Ok::<_, ()>((*node < 2).then(|| *node + 1))),
            Ok(false)
        );
        assert_eq!(
            alias_chain_loops(0, |node| Ok::<_, ()>(Some((*node + 1) % 3))),
            Ok(true)
        );
        assert_eq!(alias_chain_loops(0, |_| Ok::<_, ()>(Some(1))), Ok(true));
        assert_eq!(
            alias_chain_loops(0, |_| Err::<Option<i32>, _>("unknown")),
            Err("unknown")
        );
    }
}
