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

//! Shared compiler and runtime tables, independent of bytecode instructions.
//!
//! Literal pools retain native bytes, allocation recipes and ordered compiler
//! actions; local tables retain physical slots and their source names. Error
//! regions describe source containment and Tcl error frames. None of these
//! types depends on opcode selection, instruction layout or encoding.

mod error_region;
mod literals;
mod local_vars;

pub use error_region::{ErrorRegion, ErrorRegionFrame};
pub use literals::{
    LiteralTable, NativeExpressionNumberLiteral, NativeLiteralAction, NativeLiteralAllocation,
    NativeStringLiteral,
};
pub use local_vars::LocalVarTable;
