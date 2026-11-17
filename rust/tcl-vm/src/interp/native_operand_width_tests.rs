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

//! Tcl 9.1 compiles only four-byte operand forms (tclCompile.h
//! `DEPRECATED_OPCODE`), which it names without a width suffix (#2542).
//!
//! Each row is `tcl::unsupported::disassemble proc p` as `pc:mnemonic` on
//! tclsh 9.0.4 and 9.1.0.

use super::*;

const CASES: &[(&str, &str, &str)] = &[
    (
        "set a 1; incr a",
        "0:push1 2:storeScalar1 4:pop 5:incrScalar1Imm 8:done",
        "0:push 5:storeScalar 10:pop 11:incrScalarImm 17:done",
    ),
    (
        "set a(x) 1; incr a(x) 2; append a(x) y; lappend b z",
        "0:push1 2:push1 4:storeArray1 6:pop 7:push1 9:incrArray1Imm 12:pop 13:push1 \
         15:push1 17:appendArray1 19:pop 20:push1 22:lappendScalar1 24:done",
        "0:push 5:push 10:storeArray 15:pop 16:push 21:incrArrayImm 27:pop 28:push 33:push \
         38:appendArray 43:pop 44:push 49:lappendScalar 54:done",
    ),
    (
        "set x 3; if {$x > 1} {set y 1}; set x",
        "0:push1 2:storeScalar1 4:pop 5:loadScalar1 7:push1 9:gt 10:jumpFalse1 12:push1 14:storeScalar1 16:jump1 18:push1 20:pop 21:loadScalar1 23:done",
        "0:push 5:storeScalar 10:pop 11:loadScalar 16:push 21:gt 22:jumpFalse 27:push 32:storeScalar 37:jump 42:push 47:pop 48:loadScalar 53:done",
    ),
    (
        "set s 0; set v 2; incr s $v; set a(k) 1; incr a(k) $v",
        "0:push1 2:storeScalar1 4:pop 5:push1 7:storeScalar1 9:pop 10:loadScalar1 12:incrScalar1 \
         14:pop 15:push1 17:push1 19:storeArray1 21:pop 22:push1 24:loadScalar1 26:incrArray1 \
         28:done",
        "0:push 5:storeScalar 10:pop 11:push 16:storeScalar 21:pop 22:loadScalar 27:incrScalar \
         32:pop 33:push 38:push 43:storeArray 48:pop 49:push 54:loadScalar 59:incrArray 64:done",
    ),
];

fn disassembly(profile: &str, body: &str) -> String {
    let mut vm = crate::native_fixture::interpreter(
        tcl_dialect::DialectProfile::find(profile).expect("release profile"),
    );
    let words = ["p", "", body].map(|word| Value::new_native_string_bytes(word.as_bytes()));
    let head = Value::new_native_string_bytes(b"proc".as_slice());
    assert_eq!(
        vm.invoke_host_original_object_vector(&head, &words).code,
        Code::Ok
    );
    let Some(Command::Proc(command)) = vm.lookup_command("p") else {
        panic!("{profile}: procedure");
    };
    let prepared = vm
        .ensure_proc_traced(command)
        .unwrap_or_else(|error| panic!("{profile}: {error:?}"));
    let wide = vm.runtime_version().emits_wide_instructions_only();
    prepared
        .body
        .asm
        .instructions
        .iter()
        .map(|instruction| {
            let name = if wide {
                instruction.op.wide_mnemonic()
            } else {
                instruction.op.mnemonic()
            };
            format!("{}:{name}", instruction.offset)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn tcl_9_1_compiles_four_byte_operands_where_9_0_compiles_one() {
    let normalise = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut mismatches = Vec::new();
    for (body, tcl90, tcl91) in CASES {
        for (profile, expected) in [("tcl9.0", tcl90), ("tcl9.1", tcl91)] {
            let actual = disassembly(profile, body);
            if actual != normalise(expected) {
                mismatches.push(format!(
                    "{profile} {body:?}:\n  {actual}\n  {}",
                    normalise(expected)
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
