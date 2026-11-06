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

//! Opaque editor transport allocation discriminators, independent of source
//! ownership, names, semantic identity and current host publication receipts.

/// One original random editor-wire allocation discriminator. Its wire bytes
/// carry equality only; decoding cannot issue a retained registry or ledger
/// allocation. Source/currentness and purpose-specific token checks remain
/// separate obligations of each actual host adopter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct EditorWireSession([u8; 16]);

impl EditorWireSession {
    /// Ask the actual OS/WASI or browser Web Crypto provider for a fresh
    /// allocation discriminator. Unavailable randomness stays unavailable;
    /// there is no source/hash/time/reset-counter fallback or semantic grant.
    #[must_use]
    pub fn allocate() -> Option<Self> {
        let mut bytes = [0; 16];
        getrandom::getrandom(&mut bytes).ok()?;
        Some(Self(bytes))
    }
}
