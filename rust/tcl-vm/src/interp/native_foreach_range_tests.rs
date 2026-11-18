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

//! A command a procedure's loop body invokes compiles as tclsh compiles it,
//! with no instruction of its own before its first word, and a completion it
//! returns continues and breaks where the loop exception range of
//! `tcl::unsupported::disassemble proc p` says.

use super::*;

/// `(break, continue)` targets.
type LoopTargets = (Option<i32>, Option<i32>);

/// `pc:mnemonic` per instruction, and the `(break, continue)` targets of the
/// invocation inside the loop.
fn compiled(release: tcl_dialect::TclVersion, body: &str) -> (Vec<String>, Option<LoopTargets>) {
    let mut vm = crate::native_fixture::interpreter(
        tcl_dialect::DialectProfile::find(release.dialect_name()).expect("release profile"),
    );
    let words = ["p", "", body].map(|word| Value::new_native_string_bytes(word.as_bytes()));
    let head = Value::new_native_string_bytes(b"proc".as_slice());
    assert_eq!(
        vm.invoke_host_original_object_vector(&head, &words).code,
        Code::Ok
    );
    let Some(Command::Proc(command)) = vm.lookup_command("p") else {
        panic!("{release:?}: procedure");
    };
    let prepared = vm
        .ensure_proc_traced(command)
        .unwrap_or_else(|error| panic!("{release:?}: {error:?}"));
    let asm = &prepared.body.asm;
    let instructions = asm
        .instructions
        .iter()
        .map(|instruction| format!("{}:{}", instruction.offset, instruction.op.mnemonic()))
        .collect();
    let invoke = asm.instructions.iter().position(|instruction| {
        matches!(
            instruction.op,
            tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
        )
    });
    (
        instructions,
        invoke.and_then(|index| asm.loop_targets.get(&index).copied()),
    )
}

/// tclsh's instructions and its loop range's `(break, continue)` targets.
fn oracle(release: tcl_dialect::TclVersion, body: &str) -> Option<(Vec<String>, LoopTargets)> {
    let tclsh = tcl_test_support::witness_tclsh(release)?;
    let output = tcl_test_support::run_script(
        &tclsh.path,
        format!("proc p {{}} {{{body}}}; puts [tcl::unsupported::disassemble proc p]").as_bytes(),
    )
    .expect("the oracle runs");
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let instructions = text
        .lines()
        .filter_map(|line| {
            let (pc, rest) = line.trim_start().strip_prefix('(')?.split_once(") ")?;
            Some(format!("{pc}:{}", rest.split_whitespace().next()?))
        })
        .collect();
    let range = text
        .lines()
        .find(|line| line.contains(", loop, pc "))
        .expect("tclsh reports the loop range");
    let target = |key: &str| {
        range
            .split(", ")
            .find_map(|part| part.strip_prefix(key))
            .and_then(|value| value.trim().parse().ok())
    };
    Some((instructions, (target("break "), target("continue "))))
}

#[test]
fn an_invoked_command_in_a_loop_body_compiles_as_tclsh_compiles_it() {
    for body in [
        "foreach x {1 2} { $completion }",
        "foreach x {1 2} {set a 1; $completion}",
        "while 1 {$completion}",
    ] {
        for release in [tcl_dialect::TclVersion::V8_6, tcl_dialect::TclVersion::V9_0] {
            let Some((instructions, targets)) = oracle(release, body) else {
                continue;
            };
            assert_eq!(
                compiled(release, body),
                (instructions, Some(targets)),
                "{release:?} {body}"
            );
        }
    }
}
