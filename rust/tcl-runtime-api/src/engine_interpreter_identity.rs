// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Callback-free diagnostic identity; serialized fields grant no operation.

use crate::native_compilation::NativeInterpreterIdentity;

/// The Engine ABI's diagnostic projection of its already-live interpreter.
/// Never used as a lookup, entry, object or command authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EngineInterpreterIdentity(pub NativeInterpreterIdentity);

impl EngineInterpreterIdentity {
    /// Canonical export whose output is two little-endian u64 fields.
    pub const EXPORT: &'static str = "tcl_engine_interpreter_identity";
    /// The exact counted output length, independent of Rust structure layout.
    pub const BYTE_LENGTH: usize = 16;
    /// Explicit byte encoding; no native sizeof/alignment claim.
    #[must_use]
    pub fn bytes(self) -> [u8; Self::BYTE_LENGTH] {
        let mut bytes = [0; Self::BYTE_LENGTH];
        bytes[..8].copy_from_slice(&self.0.owner.to_le_bytes());
        bytes[8..].copy_from_slice(&self.0.interpreter.to_le_bytes());
        bytes
    }
    /// Decode reporting data. This neither validates lifetime nor grants authority.
    #[must_use]
    pub fn from_bytes(bytes: [u8; Self::BYTE_LENGTH]) -> Self {
        Self(NativeInterpreterIdentity {
            owner: u64::from_le_bytes(bytes[..8].try_into().expect("fixed identity owner field")),
            interpreter: u64::from_le_bytes(
                bytes[8..].try_into().expect("fixed interpreter field"),
            ),
        })
    }
}
