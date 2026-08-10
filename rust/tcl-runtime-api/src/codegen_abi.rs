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

//! Target-neutral vocabulary for the compiler/runtime WASM code-generation ABI.
//!
//! Concrete runtimes export these imports over their shared linear memory, and
//! target emitters lower the descriptors to their native IR. This crate owns
//! the spelling, wasm32 layout, and signatures so neither side grows a parallel
//! copy of the transport contract.

/// A scalar ABI value type used by the current WASM import subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodegenAbiValueType {
    /// A wasm32 integer, pointer, length, status, or Tcl completion code.
    I32,
}

/// One compiler/runtime import descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodegenAbiImport {
    /// The runtime import module.
    pub module: &'static str,
    /// The stable import field name.
    pub name: &'static str,
    /// Parameter types in call order.
    pub parameters: &'static [CodegenAbiValueType],
    /// Result types in return order.
    pub results: &'static [CodegenAbiValueType],
}

const I32: &[CodegenAbiValueType] = &[CodegenAbiValueType::I32];
const I32_I32: &[CodegenAbiValueType] = &[CodegenAbiValueType::I32; 2];
const I32_I32_I32: &[CodegenAbiValueType] = &[CodegenAbiValueType::I32; 3];
const NONE: &[CodegenAbiValueType] = &[];

/// Compiler/runtime code-generation imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodegenAbiImportId {
    /// Allocate a re-entrant transient shared-memory call frame.
    CallFrameAlloc,
    /// Free a transient shared-memory call frame using its runtime-recorded layout.
    CallFrameFree,
    /// Copy bytes to a Tcl object with one caller-owned reference.
    NewOwnedString,
    /// Dispatch one fully evaluated argv vector.
    InvokeArgv,
    /// Release the owned result/options pair stored in a completion triple.
    CompletionRelease,
    /// Duplicate a Tcl object owned reference for completion forwarding.
    ObjectRetain,
    /// Release one Tcl object owned reference.
    ObjectRelease,
}

impl CodegenAbiImportId {
    /// Return this import's shared ABI descriptor.
    #[must_use]
    pub const fn descriptor(self) -> CodegenAbiImport {
        match self {
            Self::CallFrameAlloc => CodegenAbiImport {
                module: "tcl",
                name: "tcl_codegen_call_frame_alloc",
                parameters: I32_I32,
                results: I32,
            },
            Self::CallFrameFree => CodegenAbiImport {
                module: "tcl",
                name: "tcl_codegen_call_frame_free",
                parameters: I32,
                results: I32,
            },
            Self::NewOwnedString => CodegenAbiImport {
                module: "tcl",
                name: "tcl_obj_new_string_owned",
                parameters: I32_I32,
                results: I32,
            },
            Self::InvokeArgv => CodegenAbiImport {
                module: "tcl",
                name: "tcl_invoke_argv",
                parameters: I32_I32_I32,
                results: I32,
            },
            Self::CompletionRelease => CodegenAbiImport {
                module: "tcl",
                name: "tcl_completion_release",
                parameters: I32,
                results: NONE,
            },
            Self::ObjectRetain => CodegenAbiImport {
                module: "tcl",
                name: "tcl_obj_retain",
                parameters: I32,
                results: I32,
            },
            Self::ObjectRelease => CodegenAbiImport {
                module: "tcl",
                name: "tcl_obj_release",
                parameters: I32,
                results: NONE,
            },
        }
    }
}

/// wasm32 linear-memory pointer width.
pub const WASM32_POINTER_BYTES: i32 = 4;
/// First byte of the immutable data window reserved for generated wasm32 code.
///
/// The runtime's downward-growing shadow stack occupies the preceding MiB.
pub const WASM32_CODEGEN_DATA_START: i64 = 0x10_0000;
/// Exclusive end of the immutable data window reserved for generated wasm32 code.
///
/// Linked runtime data starts here (`wasm-ld --global-base=0x20_0000`), so the
/// complete generated constant pool must stay below this address.
pub const WASM32_CODEGEN_DATA_END: i64 = 0x20_0000;
/// Offset of `TclCompletionAbi.code`.
pub const WASM32_COMPLETION_CODE_OFFSET: i32 = 0;
/// Offset of `TclCompletionAbi.result`.
pub const WASM32_COMPLETION_RESULT_OFFSET: i32 = 4;
/// Offset of `TclCompletionAbi.options`.
pub const WASM32_COMPLETION_OPTIONS_OFFSET: i32 = 8;
/// Size of the wasm32 `TclCompletionAbi` transport layout.
pub const WASM32_COMPLETION_SIZE: i32 = 12;
/// Alignment of the wasm32 `TclCompletionAbi` transport layout.
pub const WASM32_COMPLETION_ALIGN: i32 = 4;
