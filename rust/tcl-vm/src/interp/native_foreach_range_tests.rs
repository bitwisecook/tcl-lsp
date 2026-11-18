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
    compiled_with(release, "", body)
}

/// As [`compiled`], for a procedure with formal parameters `params`.
fn compiled_with(
    release: tcl_dialect::TclVersion,
    params: &str,
    body: &str,
) -> (Vec<String>, Option<LoopTargets>) {
    let mut vm = crate::native_fixture::interpreter(
        tcl_dialect::DialectProfile::find(release.dialect_name()).expect("release profile"),
    );
    let words = ["p", params, body].map(|word| Value::new_native_string_bytes(word.as_bytes()));
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

/// tclsh's instructions and, where it has one, its loop range's
/// `(break, continue)` targets.
fn oracle(
    release: tcl_dialect::TclVersion,
    body: &str,
) -> Option<(Vec<String>, Option<LoopTargets>)> {
    oracle_of(
        release,
        &format!("proc p {{}} {{{body}}}; puts [tcl::unsupported::disassemble proc p]"),
    )
}

/// tclsh's disassembly printed by `script`, as `pc:mnemonic` and loop targets.
fn oracle_of(
    release: tcl_dialect::TclVersion,
    script: &str,
) -> Option<(Vec<String>, Option<LoopTargets>)> {
    let tclsh = tcl_test_support::witness_tclsh(release)?;
    let output =
        tcl_test_support::run_script(&tclsh.path, script.as_bytes()).expect("the oracle runs");
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let instructions = text
        .lines()
        .filter_map(|line| {
            let (pc, rest) = line.trim_start().strip_prefix('(')?.split_once(") ")?;
            Some(format!("{pc}:{}", rest.split_whitespace().next()?))
        })
        .collect();
    let targets = text
        .lines()
        .find(|line| line.contains(", loop, pc "))
        .map(|range| {
            let target = |key: &str| {
                range
                    .split(", ")
                    .find_map(|part| part.strip_prefix(key))
                    .and_then(|value| value.trim().parse().ok())
            };
            (target("break "), target("continue "))
        });
    Some((instructions, targets))
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
            assert!(targets.is_some(), "tclsh reports the loop range: {body}");
            assert_eq!(
                compiled(release, body),
                (instructions, targets),
                "{release:?} {body}"
            );
        }
    }
}

/// tclsh converts a control condition that is a bare primary (variable,
/// command substitution) before its jump: `tryCvtToNumeric` in 8.5, a `nop`
/// in its place from 8.6.
#[test]
fn a_bare_primary_condition_converts_before_its_jump_as_tclsh_does() {
    for body in [
        "if {$a} {$c}",
        "if {$a} {set b 1}",
        "if {$a} {set b 1} elseif {$b} {set c 1}",
        "if {[f]} {set b 1}",
        "while {$a} {set a 0}",
        "for {} {$a} {} {set a 0}",
    ] {
        for release in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
        ] {
            let Some((instructions, _)) = oracle(release, body) else {
                continue;
            };
            assert_eq!(
                compiled(release, body).0,
                instructions,
                "{release:?} {body}"
            );
        }
    }
}

/// A top-level script's instructions as this VM compiles it for evaluation.
fn script_compiled(release: tcl_dialect::TclVersion, script: &str) -> Vec<String> {
    let mut vm = crate::native_fixture::interpreter(
        tcl_dialect::DialectProfile::find(release.dialect_name()).expect("release profile"),
    );
    let unit = vm
        .compile_script_cached(script)
        .unwrap_or_else(|error| panic!("{release:?}: {error:?}"));
    unit.asm
        .instructions
        .iter()
        .map(|instruction| format!("{}:{}", instruction.offset, instruction.op.mnemonic()))
        .collect()
}

/// The same source compiles to tclsh's stack-variable forms as a top-level
/// script and to its local-variable forms as a procedure body.
#[test]
fn a_script_and_a_procedure_body_compile_as_tclsh_compiles_each() {
    for source in [
        "set x 1;incr x;lappend l $x;unset x",
        "set t 0;incr t 2;lappend acc $t;return $t",
    ] {
        for release in [tcl_dialect::TclVersion::V8_6, tcl_dialect::TclVersion::V9_0] {
            let Some((script, _)) = oracle_of(
                release,
                &format!("puts [tcl::unsupported::disassemble script {{{source}}}]"),
            ) else {
                continue;
            };
            assert_eq!(
                script_compiled(release, source),
                script,
                "{release:?} script {source}"
            );
            let (body, _) = oracle(release, source).expect("the same oracle");
            assert_eq!(
                compiled(release, source).0,
                body,
                "{release:?} proc {source}"
            );
        }
    }
}

/// A quoted word interpolating parameters compiles as tclsh compiles it.
#[test]
fn an_interpolated_value_in_a_procedure_compiles_as_tclsh_compiles_it() {
    let body = "set msg \"Hello, $name! You are $age years old.\"; return $msg";
    for release in [tcl_dialect::TclVersion::V8_6, tcl_dialect::TclVersion::V9_0] {
        let Some((instructions, _)) = oracle_of(
            release,
            &format!("proc p {{name age}} {{{body}}}; puts [tcl::unsupported::disassemble proc p]"),
        ) else {
            continue;
        };
        assert_eq!(
            compiled_with(release, "name age", body).0,
            instructions,
            "{release:?}"
        );
    }
}

/// Every body ends in the `done` C appends: after a script's `returnImm`, and
/// after a procedure's tail `return` (folded to `done`), which 8.6's optimizer
/// then trims and 8.5 keeps.
#[test]
fn a_body_ends_with_the_done_tclsh_appends() {
    for source in ["return $x", "set y 1; return $x", "return 1"] {
        for release in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
        ] {
            let Some((body, _)) = oracle_of(
                release,
                &format!("proc p {{x}} {{{source}}}; puts [tcl::unsupported::disassemble proc p]"),
            ) else {
                continue;
            };
            assert_eq!(
                compiled_with(release, "x", source).0,
                body,
                "{release:?} proc {source}"
            );
            // 8.5 scripts read variables through its own stack forms, a
            // separate witness.
            if release >= tcl_dialect::TclVersion::V8_6 {
                let (script, _) = oracle_of(
                    release,
                    &format!("puts [tcl::unsupported::disassemble script {{{source}}}]"),
                )
                .expect("the same oracle");
                assert_eq!(
                    script_compiled(release, source),
                    script,
                    "{release:?} script {source}"
                );
            }
        }
    }
}
