// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original `TclGetInnerContext` instruction-name objects and their updater.

use crate::native_string::NativeStringProtocol;
use tcl_dialect::TclVersion;

/// Reached original return instruction retained by an inner context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeReturnInstructionName {
    /// An immediate merged-options return instruction.
    Immediate,
    /// A reached compiler Syntax failure instruction.
    Syntax,
}

/// Actual C instname primary, with NULL free/duplicate hooks and a string updater.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeInstructionName {
    version: TclVersion,
    instruction: NativeReturnInstructionName,
}

impl NativeInstructionName {
    /// Select the descriptor from the actual C string/error-stack recipe.
    #[must_use]
    pub const fn for_return(
        protocol: NativeStringProtocol,
        instruction: NativeReturnInstructionName,
    ) -> Option<Self> {
        match protocol {
            NativeStringProtocol::C(
                version @ (TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1),
            ) => Some(Self {
                version,
                instruction,
            }),
            _ => None,
        }
    }

    /// Exact native descriptor release; this metadata does not grant execution.
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.version
    }

    /// Original native opcode value retained in longValue, independently of portable opcodes.
    #[must_use]
    pub const fn opcode(self) -> u8 {
        match (self.version, self.instruction) {
            (TclVersion::V8_6, NativeReturnInstructionName::Immediate) => 98,
            (TclVersion::V8_6, NativeReturnInstructionName::Syntax) => 125,
            (_, NativeReturnInstructionName::Immediate) => 92,
            (_, NativeReturnInstructionName::Syntax) => 118,
        }
    }

    /// Reached native updater bytes; construction leaves resident bytes absent.
    #[must_use]
    pub const fn string_bytes(self) -> &'static [u8] {
        match self.instruction {
            NativeReturnInstructionName::Immediate => b"returnImm",
            NativeReturnInstructionName::Syntax => b"syntax",
        }
    }
}
