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

//! Literal byte storage is independent of checked native execution admission.

use tcl_bytecode::{FunctionAsm, Instruction, LiteralTable, Op, Operand};
use tcl_runtime_api::{NativeCompilationPreflight, NativeExecutionError};

fn literal_function(bytes: &[u8], verbatim: bool) -> FunctionAsm {
    let mut literals = LiteralTable::new();
    literals.intern_bytes(bytes);
    let mut push = Instruction::new(Op::PUSH1, vec![Operand::Imm(0)]);
    push.push_verbatim = verbatim;
    FunctionAsm {
        literals,
        instructions: vec![push, Instruction::new(Op::DONE, vec![])],
        native_compilation_preflight: NativeCompilationPreflight::NotRequired,
        ..FunctionAsm::default()
    }
}

#[test]
fn unrepresented_native_byte_literal_is_a_host_refusal() {
    for verbatim in [false, true] {
        let mut vm = tcl_vm::test_support::native_vm();
        let artifact = literal_function(&[0xff], verbatim);
        assert!(matches!(
            vm.try_run_function(&artifact),
            Err(NativeExecutionError::HostCommandRefusal(_))
        ));
        assert_eq!(artifact.literals.entries()[0].bytes(), &[0xff]);
    }
}

#[test]
fn checked_unicode_native_literal_keeps_its_exact_bytes() {
    let mut vm = tcl_vm::test_support::native_vm();
    let result = vm
        .try_run_function(&literal_function("ÿ".as_bytes(), true))
        .unwrap();
    assert!(result.code.is_ok());
    assert_eq!(result.result.string_bytes().as_ref(), "ÿ".as_bytes());
}
