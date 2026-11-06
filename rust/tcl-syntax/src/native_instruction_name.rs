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
