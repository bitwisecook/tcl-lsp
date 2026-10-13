// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C9 List insertion operation carried between compiler and backend.

/// Actual C9 insertion instruction selected by the native release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeListInsertionInstruction {
    /// C9.0 consumes the original list, index and counted replacement objects.
    ReplaceFour,
    /// C9.1 consumes the original list/index and counted replacement objects.
    Replace,
}

impl NativeListInsertionInstruction {
    /// Exact native disassembly operation; this does not grant execution.
    #[must_use]
    pub const fn opcode_name(self) -> &'static str {
        match self {
            Self::ReplaceFour => "lreplace4",
            Self::Replace => "lreplace",
        }
    }
}
