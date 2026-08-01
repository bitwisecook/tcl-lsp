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

//! Direct eBPF bytecode backend: BPF-IR → eBPF instructions, encoded by our own
//! 64-bit instruction model (no assembler, no LLVM in the compile path).

pub mod disasm;
pub mod elf;
pub mod emit;
pub mod insn;

pub use disasm::disasm;
pub use elf::{ElfError, write_object};
pub use emit::{EbpfObject, TargetAbi, emit_program, emit_program_for_target};
pub use insn::Insn;
