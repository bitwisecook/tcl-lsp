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

//! A command mutated while its own arguments are substituted keeps its entered token and deoptimises at the next command: the per-command table and the codegen-hook case.

use super::*;

#[test]
fn non_expr_codegen_hook_redispatches_after_replacement() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("proc p {} {mutate; llength {a b c}}")
        .expect("module compiles");
    assert!(
        sole_compiled_procedure(&compiled)
            .command_bindings
            .iter()
            .any(|binding| binding.name == "llength" && binding.identity == "llength")
    );
    eval_ok(
        &mut vm,
        "proc mutate {} {rename llength real_llength; proc llength args {return LIST_OVERRIDE}}; \
         proc p {} {mutate; llength {a b c}}",
    );

    // `llength` is registry-hooked independently of AssignExpr; the same
    // binding-generation contract protects it.
    assert_eq!(eval_ok(&mut vm, "p"), "LIST_OVERRIDE");
}

#[test]
#[allow(clippy::too_many_lines)] // One table keeps the Tcl 9.0.4 differential cases uniform.
fn argument_substitution_mutation_keeps_entered_token_then_deopts_at_next_command() {
    struct Case {
        name: &'static str,
        source: &'static str,
        want: &'static str,
    }

    let cases = [
        Case {
            name: "direct replacement",
            source: concat!(
                "set ::replacement_called 0\n",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {",
                "set ::replacement_called 1; return REPLACED}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "llength [mutate]; ",
                "list $::replacement_called [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "0 REPLACED\n",
        },
        Case {
            name: "direct rename away",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "return {a b c}}\n",
                "proc p {} {",
                "llength [mutate]; ",
                "set code [catch {llength {d e}} result]; ",
                "list $code $result}\n",
                "puts [p]\n",
            ),
            want: "1 {invalid command name \"llength\"}\n",
        },
        Case {
            name: "direct forced import",
            source: concat!(
                "set ::import_called 0\n",
                "namespace eval alt {",
                "proc llength args {",
                "set ::import_called 1; return IMPORTED}; ",
                "namespace export llength}\n",
                "namespace eval N {",
                "namespace import ::llength; ",
                "proc mutate {} {",
                "namespace import -force ::alt::llength; ",
                "return {a b c}}; ",
                "proc p {} {",
                "llength [mutate]; ",
                "list $::import_called [llength {d e}]}}\n",
                "puts [N::p]\n",
            ),
            want: "0 IMPORTED\n",
        },
        Case {
            name: "nested replacement",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return REPLACED}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "3 REPLACED\n",
        },
        Case {
            name: "nested rename away",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "set code [catch {llength {d e}} result]; ",
                "list $first $code $result}\n",
                "puts [p]\n",
            ),
            want: "3 1 {invalid command name \"llength\"}\n",
        },
        Case {
            name: "nested forced import",
            source: concat!(
                "namespace eval alt {",
                "proc llength args {return IMPORTED}; ",
                "namespace export llength}\n",
                "namespace eval N {",
                "namespace import ::llength; ",
                "proc mutate {} {",
                "namespace import -force ::alt::llength; ",
                "return {a b c}}; ",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {d e}]}}\n",
                "puts [N::p]\n",
            ),
            want: "3 IMPORTED\n",
        },
        Case {
            name: "ordinary generic command resolves after substitution",
            source: concat!(
                "proc foo x {return OLD:$x}\n",
                "proc mutate {} {",
                "rename foo saved_foo; ",
                "proc foo x {return NEW:$x}; ",
                "return X}\n",
                "proc p {} {foo [mutate]}\n",
                "puts [p]\n",
            ),
            want: "NEW:X\n",
        },
        Case {
            name: "wrong-arity builtin remains generic",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return NEW}; ",
                "return X}\n",
                "proc p {} {llength [mutate] extra}\n",
                "puts [p]\n",
            ),
            want: "NEW\n",
        },
        Case {
            name: "outer generic command does not replace inner marker",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return NEW}; ",
                "return {a b c}}\n",
                "proc p {} {puts [llength [mutate]]}\n",
                "p\n",
            ),
            want: "3\n",
        },
        Case {
            name: "dynamic head remains late resolved",
            source: concat!(
                "set ::replacement_called 0\n",
                "set ::payload {a b c}\n",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {",
                "set ::replacement_called 1; return REPLACED}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set command llength; ",
                "$command [mutate]; ",
                "list $::replacement_called [$command {d e}]}\n",
                "puts [p]\n",
            ),
            want: "1 REPLACED\n",
        },
        Case {
            name: "direct inner command revalidates while outer token stays entered",
            source: concat!(
                "set ::replacement_called 0\n",
                "set ::payload {a b c}\n",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {",
                "set ::replacement_called 1; return REPLACED}; ",
                "return ignored}\n",
                "proc p {} {",
                "llength [mutate; llength {d e}; set ::payload]}\n",
                "puts [list [p] $::replacement_called]\n",
            ),
            want: "3 1\n",
        },
        Case {
            name: "nested inner command revalidates while outer token stays entered",
            source: concat!(
                "set ::replacement_called 0\n",
                "set ::payload {a b c}\n",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {",
                "set ::replacement_called 1; return REPLACED}; ",
                "return ignored}\n",
                "proc p {} {",
                "set outer [llength [mutate; llength {d e}; set ::payload]]; ",
                "list $outer $::replacement_called}\n",
                "puts [p]\n",
            ),
            want: "3 1\n",
        },
        Case {
            name: "expanded nested replacement resolves after expansion",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return REPLACED}; ",
                "return abc}\n",
                "proc p {} {",
                "set first [llength {*}[mutate]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "REPLACED REPLACED\n",
        },
        Case {
            name: "execution trace plain-dispatch resolves after substitution",
            source: concat!(
                "proc callback args {}\n",
                "trace add execution llength enter callback\n",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return REPLACED}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "REPLACED REPLACED\n",
        },
        Case {
            name: "pre-existing execution trace deoptimises specialised opcode",
            source: concat!(
                "set ::traced 0\n",
                "proc callback {command op} {",
                "if {$command eq {llength {a b}}} {incr ::traced}}\n",
                "trace add execution llength enter callback\n",
                "proc p {} {llength {a b}}\n",
                "puts [list [p] $::traced]\n",
            ),
            want: "2 1\n",
        },
        Case {
            name: "first argument substitution cannot pre-empt command entry",
            source: concat!(
                "set ::x A\n",
                "proc noop {} {return B}\n",
                "proc callback args {",
                "trace remove variable ::x read callback; ",
                "rename llength saved_llength; ",
                "proc llength args {return REPLACED}}\n",
                "trace add variable ::x read callback\n",
                "proc p {} {",
                "set first [llength $::x[noop]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "1 REPLACED\n",
        },
        Case {
            name: "lassign typed hook retains its entered command",
            source: concat!(
                "proc mutate {} {",
                "rename lassign saved_lassign; ",
                "proc lassign args {return NEW}; ",
                "return {a b}}\n",
                "proc p {} {",
                "set a 0; set b 0; ",
                "set first [lassign [mutate] a b]; ",
                "list $a $b $first [lassign {x} a]}\n",
                "puts [p]\n",
            ),
            want: "a b {} NEW\n",
        },
        Case {
            name: "append typed hook retains its entered command",
            source: concat!(
                "proc mutate {} {",
                "rename append saved_append; ",
                "proc append args {return NEW}; ",
                "return X}\n",
                "proc p {} {",
                "set x A; ",
                "set first [append x [mutate]]; ",
                "list $first $x [append x B]}\n",
                "puts [p]\n",
            ),
            want: "AX AX NEW\n",
        },
        Case {
            name: "concat typed hook retains its entered command",
            source: concat!(
                "proc mutate {} {",
                "rename concat saved_concat; ",
                "proc concat args {return NEW}; ",
                "return {a b}}\n",
                "proc p {} {",
                "set first [concat [mutate] c]; ",
                "list $first [concat d e]}\n",
                "puts [p]\n",
            ),
            want: "{a b c} NEW\n",
        },
        Case {
            name: "lrange typed hook survives inline-hook distrust",
            source: concat!(
                "proc mutate {} {",
                "rename lrange saved_lrange; ",
                "proc lrange args {return NEW}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [lrange [mutate] 0 1]; ",
                "list $first [lrange {d e} 0 0]}\n",
                "puts [p]\n",
            ),
            want: "{a b} NEW\n",
        },
        Case {
            name: "linsert typed hook survives inline-hook distrust",
            source: concat!(
                "proc mutate {} {",
                "rename linsert saved_linsert; ",
                "proc linsert args {return NEW}; ",
                "return {a b}}\n",
                "proc p {} {",
                "set first [linsert [mutate] 1 X]; ",
                "list $first [linsert {d e} 1 Y]}\n",
                "puts [p]\n",
            ),
            want: "{a X b} NEW\n",
        },
        Case {
            name: "namespace-local proc never acquires builtin entry timing",
            source: concat!(
                "namespace eval N {",
                "proc llength args {return OLD}; ",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return NEW}; ",
                "return {a b}}; ",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {c d}]}}\n",
                "puts [N::p]\n",
            ),
            want: "NEW NEW\n",
        },
        Case {
            name: "trace added during namespace-local proc argument applies immediately",
            source: concat!(
                "namespace eval N {",
                "set traced 0; ",
                "proc llength args {return OLD}; ",
                "proc callback {command op} {incr ::N::traced}; ",
                "proc mutate {} {",
                "trace add execution llength enter callback; ",
                "return {a b}}; ",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first $::N::traced}}\n",
                "puts [N::p]\n",
            ),
            want: "OLD 1\n",
        },
        Case {
            name: "coroutine initial command uses traced plain admission",
            source: concat!(
                "set ::traced 0\n",
                "proc callback {command op} {",
                "if {$command eq {expr 1+2}} {incr ::traced}}\n",
                "trace add execution expr enter callback\n",
                "set result [coroutine c expr {1+2}]\n",
                "puts [list $result $::traced]\n",
            ),
            want: "3 1\n",
        },
        Case {
            name: "cached proc tracks first trace add and last removal",
            source: concat!(
                "set ::traced 0\n",
                "proc callback {command op} {",
                "if {$command eq {llength {a b}}} {incr ::traced}}\n",
                "proc p {} {llength {a b}}\n",
                "set before [p]\n",
                "trace add execution llength enter callback\n",
                "set during [p]\n",
                "trace remove execution llength enter callback\n",
                "set after [p]\n",
                "puts [list $before $during $after $::traced]\n",
            ),
            want: "2 2 2 1\n",
        },
        Case {
            name: "execution trace on alias source deoptimises fused target",
            source: concat!(
                "interp alias {} e {} expr\n",
                "set ::traced 0\n",
                "proc callback {command op} {",
                "if {$command eq {e 1+2}} {incr ::traced}}\n",
                "trace add execution e enter callback\n",
                "proc p {} {e {1+2}}\n",
                "puts [list [p] $::traced]\n",
            ),
            want: "3 1\n",
        },
        Case {
            name: "execution trace on alias target deoptimises fused source",
            source: concat!(
                "interp alias {} e {} expr\n",
                "set ::traced 0\n",
                "proc callback {command op} {",
                "if {$command eq {expr 1+2}} {incr ::traced}}\n",
                "trace add execution expr enter callback\n",
                "proc p {} {e {1+2}}\n",
                "puts [list [p] $::traced]\n",
            ),
            want: "3 1\n",
        },
        Case {
            name: "trace added after entry does not change entered token",
            source: concat!(
                "proc callback args {}\n",
                "proc mutate {} {",
                "trace add execution llength enter callback; ",
                "rename llength saved_llength; ",
                "proc llength args {return NEWA}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "3 NEWA\n",
        },
        Case {
            name: "trace added after entry applies only to later calls",
            source: concat!(
                "set ::traced 0\n",
                "proc callback args {incr ::traced}\n",
                "proc mutate {} {",
                "trace add execution llength enter callback; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "set before $::traced; ",
                "set second [llength {d e}]; ",
                "list $first $before $second $::traced}\n",
                "puts [p]\n",
            ),
            want: "3 0 2 1\n",
        },
        Case {
            name: "trace removed after entry keeps late resolution",
            source: concat!(
                "proc callback args {}\n",
                "trace add execution llength enter callback\n",
                "proc mutate {} {",
                "trace remove execution llength enter callback; ",
                "rename llength saved_llength; ",
                "proc llength args {return NEWB}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "NEWB NEWB\n",
        },
        Case {
            name: "catch body retains eligible specialised entry",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return NEW}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set rc [catch {llength [mutate]} result]; ",
                "list $rc $result}\n",
                "puts [p]\n",
            ),
            want: "0 3\n",
        },
        Case {
            name: "try handler retains eligible specialised entry",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return NEW}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "try {error boom} on error {m} {llength [mutate]}}\n",
                "puts [p]\n",
            ),
            want: "3\n",
        },
        Case {
            name: "suspended command retains entered token across mutation",
            source: concat!(
                "proc p {} {llength [yield PAUSE]}\n",
                "puts [coroutine c p]\n",
                "rename llength saved_llength\n",
                "proc llength args {return NEW}\n",
                "puts [c {a b c}]\n",
            ),
            want: "PAUSE\n3\n",
        },
        Case {
            name: "failed substitution prunes entered token before catch continuation",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return REPLACED}; ",
                "error BOOM}\n",
                "proc p {} {",
                "set code [catch {set first [llength [mutate]]} result]; ",
                "list $code $result [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "1 BOOM REPLACED\n",
        },
        Case {
            name: "invoke replace keeps specialised entered command",
            source: concat!(
                "proc mutate {} {",
                "rename string saved_string; ",
                "proc string args {return STRING_REPLACED}; ",
                "return x}\n",
                "proc p {} {",
                "set first [string equal -nocase [mutate] X]; ",
                "list $first [string equal x x]}\n",
                "puts [p]\n",
            ),
            want: "1 STRING_REPLACED\n",
        },
        Case {
            name: "invoke replace resolves internal implementation after substitution",
            source: concat!(
                "namespace eval ::tcl::string {}\n",
                "proc mutate {} {",
                "rename ::tcl::string::equal saved_equal; ",
                "proc ::tcl::string::equal args {return INTERNAL_REPLACED}; ",
                "return x}\n",
                "proc p {} {",
                "string equal -nocase [mutate] X}\n",
                "puts [p]\n",
            ),
            // A later ordinary `string equal` belongs to the
            // registry/ensemble deduplication; this vector
            // isolates INVOKE_REPLACE's same-invocation lookup timing.
            want: "INTERNAL_REPLACED\n",
        },
        // The `string` cases hold for tclsh 8.6.18, 9.0.4 and 9.1.0; 8.5.19
        // does not compile the ensemble and prints `MAP_REPLACED` twice here.
        Case {
            name: "invoke replace keeps its rewrite when the ensemble map changes",
            source: concat!(
                "proc ::myeq args {return MAP_REPLACED}\n",
                "proc mutate {} {",
                "namespace ensemble configure ::string -map ",
                "[dict replace [namespace ensemble configure ::string -map] equal ::myeq]; ",
                "return x}\n",
                "proc p {} {",
                "set first [string equal -nocase [mutate] X]; ",
                "list $first [string equal -nocase x x]}\n",
                "puts [p]\n",
            ),
            want: "1 MAP_REPLACED\n",
        },
    ];

    // Tcl 9.0.4 resolves the specialised command token before evaluating its
    // remaining words. The token therefore survives mutations performed by an
    // argument substitution, while the following source command is the replay
    // boundary and resolves the replacement, missing name, or forced import.
    // Revalidating the outer operation itself would disagree with C Tcl and
    // would also risk evaluating a substitution twice.
    let oracle = tcl_test_support::locate_tclsh(TclVersion::V9_0)
        .expect("the Tcl 9.0 oracle override is valid");
    let marker_module = BytecodeCompileService::default()
        .compile("proc p {} {set first [llength [mutate]]; return $first}")
        .expect("literal nested command compiles");
    assert!(
        sole_compiled_procedure(&marker_module)
            .instructions
            .iter()
            .any(|instruction| {
                matches!(
                    instruction.op,
                    tcl_bytecode::Op::PUSH1 | tcl_bytecode::Op::PUSH4
                ) && instruction.entered_command.as_ref().is_some_and(|entered| {
                    entered.binding.name == "llength"
                        && entered.binding.identity == "llength"
                        && !entered.end.is_empty()
                })
            }),
        "compiler lost the typed literal entered-command marker"
    );
    assert!(
        !sole_compiled_procedure(&marker_module)
            .instructions
            .iter()
            .any(|instruction| {
                instruction.op == tcl_bytecode::Op::START_CMD
                    && instruction.entered_command.is_some()
            }),
        "entered-command metadata belongs to the literal head, not a nested substitution"
    );
    let expanded_marker_module = BytecodeCompileService::default()
        .compile("proc p {} {set first [llength {*}[mutate]]; return $first}")
        .expect("expanded literal nested command compiles");
    assert!(
        !sole_compiled_procedure(&expanded_marker_module)
            .instructions
            .iter()
            .any(
                |instruction| instruction.entered_command.as_ref().is_some_and(|entered| {
                    entered.binding.name == "llength" && !entered.end.is_empty()
                })
            ),
        "expanded commands resolve after expansion and must not retain a token"
    );
    for case in cases {
        let (mut vm, output) = vm();
        eval_ok(&mut vm, case.source);
        let actual = String::from_utf8(output.0.borrow().clone()).expect("UTF-8 VM output");
        assert_eq!(actual, case.want, "{}: TclVM", case.name);

        if let Some(tclsh) = &oracle {
            assert_eq!(
                tclsh.patchlevel,
                tcl_test_support::reference_patchlevel(TclVersion::V9_0),
                "exact Tcl oracle pin"
            );
            let outcome = tcl_test_support::run_script(&tclsh.path, case.source.as_bytes())
                .expect("Tcl 9.0.4 oracle runs");
            let oracle_output = outcome.strict_text().expect("Tcl 9.0.4 succeeds");
            assert_eq!(
                format!("{oracle_output}\n"),
                case.want,
                "{}: Tcl 9.0.4 oracle",
                case.name
            );
        }
    }
}
