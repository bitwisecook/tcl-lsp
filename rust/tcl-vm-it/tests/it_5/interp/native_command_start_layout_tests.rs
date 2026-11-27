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

//! A procedure body's `startCommand` markers sit where C's
//! `CompileCmdCompileProc` puts them (#2490): one before each compiled
//! command, none for an invoked one, none at the start of the body, and one
//! shared count for commands that start together.
//!
//! Expected skeletons are `tcl::unsupported::disassemble proc` on tclsh
//! 8.5.19, 8.6.18, 9.0.4 and 9.1.0, keeping each `startCommand` with its
//! count (`S<n>`), each generic `invokeStk` (`I`) and `beginCatch` (`B`).
//! `N` is a selection `nop` C has no byte for. `catch_break` is compactible
//! from 8.6, where C's second pass drops every marker.

use crate::prelude::*;
use tcl_bytecode::{Op, Operand};

struct Case {
    name: &'static str,
    params: &'static str,
    body: &'static str,
    skeleton: &'static str,
    /// Where the compact second pass (`atCmdStart == 2`, 8.6 on) differs.
    compact: Option<&'static str>,
}

const CASES: &[Case] = &[
    Case {
        name: "generic_between_compiled",
        params: "",
        body: "set a 1; puts $a; set b 2",
        skeleton: "I S1",
        compact: None,
    },
    Case {
        name: "nested_expressions",
        params: "x",
        body: "puts [expr {$x+1}]; set y [expr {$x*2}]; return $y",
        skeleton: "S1 I S2 S1",
        compact: None,
    },
    Case {
        name: "compiled_after_generic",
        params: "x",
        body: "foo $x; set y [string length $x]; bar; incr y; list $y",
        skeleton: "I S2 I S1 S1",
        compact: None,
    },
    Case {
        name: "branch_bodies",
        params: "x",
        body: "if {$x} {puts a} else {set z 1}; return",
        skeleton: "I S1 S1",
        compact: None,
    },
    Case {
        name: "loop_body",
        params: "x",
        body: "while {$x} {incr x -1; puts $x}; return $x",
        skeleton: "S1 I S1",
        compact: None,
    },
    Case {
        name: "compiled_arguments",
        params: "x",
        body: "puts [list $x [string length $x]]",
        skeleton: "S1 S1 I",
        compact: None,
    },
    Case {
        name: "commands_starting_together",
        params: "x",
        body: "puts $x; set y [expr {[string length $x]+1}]; return $y",
        skeleton: "I S3 S1",
        compact: None,
    },
    Case {
        name: "commands_starting_together_at_the_start",
        params: "x",
        body: "set a [expr {$x+1}]; set b [string length $a]",
        skeleton: "S2",
        compact: Some(""),
    },
    Case {
        name: "folded_error_in_compiled_argument",
        params: "",
        body: "list [expr {1/0}]",
        skeleton: "",
        compact: None,
    },
    Case {
        name: "commands_starting_together_redefined",
        params: "x",
        body: "rename ::expr ::oldexpr; proc ::expr args {return [list hijack $args]}; \
               set y [expr {[string length $x]+1}]; rename ::expr {}; rename ::oldexpr ::expr; \
               return $y",
        skeleton: "I I S3 I I S1",
        compact: None,
    },
    Case {
        name: "catch_break",
        params: "",
        body: "catch {break} code",
        skeleton: "B S1",
        compact: Some("B"),
    },
    Case {
        name: "catch_break_after_generic",
        params: "",
        body: "puts a; catch {break} code",
        skeleton: "I S1 B S1",
        compact: None,
    },
    Case {
        name: "catch_set_after_generic",
        params: "",
        body: "puts a; catch {set x 1} code",
        skeleton: "I S1 B S1",
        compact: None,
    },
    Case {
        name: "catch_generic_body",
        params: "",
        body: "puts a; catch {puts x} code",
        skeleton: "I S1 B I",
        compact: None,
    },
];

fn compiled_body(profile: &str, case: &Case) -> Rc<tcl_bytecode::FunctionAsm> {
    Rc::new(crate::fixture::procedure_body(
        tcl_dialect::DialectProfile::find(profile).expect("release profile"),
        case.name,
        case.params,
        case.body,
    ))
}

fn skeleton(asm: &tcl_bytecode::FunctionAsm) -> String {
    asm.instructions
        .iter()
        .filter_map(|instruction| match instruction.op {
            Op::START_CMD => match instruction.operands.get(1) {
                Some(Operand::Imm(count)) => Some(format!("S{count}")),
                _ => Some("S1".to_owned()),
            },
            Op::INVOKE_STK1 | Op::INVOKE_STK4 => Some("I".to_owned()),
            Op::BEGIN_CATCH4 => Some("B".to_owned()),
            // A selection carrier C has no byte for.
            Op::NOP if instruction.native_compiler_selection.is_some() => Some("N".to_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn procedure_bodies_place_start_command_like_c() {
    let mut mismatches = Vec::new();
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for case in CASES {
            let expected = match case.compact {
                Some(compact) if profile != "tcl8.5" => compact,
                _ => case.skeleton,
            };
            let actual = skeleton(&compiled_body(profile, case));
            if actual != expected {
                mismatches.push(format!(
                    "{profile}/{}: {actual:?} != {expected:?}",
                    case.name
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Tcl 8.4.20 has no `INST_START_CMD` (tclCompile.h): its bytecode checks
/// the compile epoch only on entry, so a compiled command redefined part way
/// through a body keeps running there, as tclsh 8.4.20 shows.
#[test]
fn tcl_8_4_bodies_have_no_start_command() {
    for case in CASES {
        let asm = compiled_body("tcl8.4", case);
        assert!(
            asm.instructions
                .iter()
                .all(|instruction| instruction.op != Op::START_CMD),
            "tcl8.4/{}: {}",
            case.name,
            skeleton(&asm)
        );
    }
    let mut vm = crate::fixture::interpreter(
        tcl_dialect::DialectProfile::find("tcl8.4").expect("release profile"),
    );
    let completion = vm
        .try_eval_source(
            "proc p {} {set a 1; rename ::incr ::oldincr; proc ::incr args {return hijack}; \
             set r [incr a]; rename ::incr {}; rename ::oldincr ::incr; return $r}; p",
        )
        .expect("tcl8.4 evaluation");
    assert_eq!(completion.result.string_bytes().as_ref(), b"2");
}

/// Commands that start on one `startCommand` are checked together; when one
/// is redefined before it runs, C re-evaluates the outermost command from
/// there, once. tclsh 8.5.19 through 9.1.0 answer `{hijack {{[string length
/// $x]+1}}} 42 2`; 8.4.20, which has no `startCommand`, answers `4 4 0`.
#[test]
fn a_redefined_command_starting_with_others_replays_the_outermost() {
    let script = r"
        set n 0
        proc t x {
            rename ::expr ::oldexpr; proc ::expr args {incr ::n; return [list hijack $args]}
            set y [expr {[string length $x]+1}]
            rename ::expr {}; rename ::oldexpr ::expr; return $y
        }
        proc u x {
            rename ::string ::oldstring; proc ::string args {incr ::n; return 41}
            set y [expr {[string length $x]+1}]
            rename ::string {}; rename ::oldstring ::string; return $y
        }
        list [t abc] [u abc] $n
    ";
    let replayed = "{hijack {{[string length $x]+1}}} 42 2";
    for (profile, expected) in [
        ("tcl8.4", "4 4 0"),
        ("tcl8.5", replayed),
        ("tcl8.6", replayed),
        ("tcl9.0", replayed),
        ("tcl9.1", replayed),
    ] {
        let mut vm = crate::fixture::interpreter(
            tcl_dialect::DialectProfile::find(profile).expect("release profile"),
        );
        let completion = vm
            .try_eval_source(script)
            .unwrap_or_else(|error| panic!("{profile}: {error:?}"));
        assert_eq!(
            String::from_utf8_lossy(completion.result.string_bytes().as_ref()),
            expected,
            "{profile}"
        );
    }
}

/// `for {} {$i > 0} {incr i -1} {puts $i}` on tclsh: 8.5.19 keeps the empty
/// start's `push1; pop`, which the optimiser of 8.6.18 and 9.0.4 blanks to
/// three `nop`s ahead of the first jump, and 9.1.0, whose `push` is five
/// bytes, to six. No release has a marker there.
#[test]
fn the_optimiser_blanks_an_empty_for_start_from_8_6() {
    let case = Case {
        name: "count_down",
        params: "i",
        body: "for {} {$i > 0} {incr i -1} {puts $i}",
        skeleton: "",
        compact: None,
    };
    for (profile, nops) in [("tcl8.5", 0), ("tcl8.6", 3), ("tcl9.0", 3), ("tcl9.1", 6)] {
        let asm = compiled_body(profile, &case);
        let start: Vec<_> = asm
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .take_while(|op| !matches!(*op, Op::JUMP1 | Op::JUMP4))
            .collect();
        assert_eq!(
            (
                start.contains(&Op::POP),
                start.iter().filter(|op| **op == Op::NOP).count(),
                start.contains(&Op::START_CMD)
            ),
            (nops == 0, nops, false),
            "{profile}: {start:?}"
        );
    }
}
