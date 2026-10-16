// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
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
