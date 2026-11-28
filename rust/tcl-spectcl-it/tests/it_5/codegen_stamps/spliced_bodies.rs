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

//! What a spliced body answers: early `return`, resolution in the definition's namespace, the variables it reads, braced literals and the claims a function makes.

use super::*;

/// A body the splice renames nothing in, spliced by the VM's own compile of
/// its caller under the live native entry: the spliced command keeps the native
/// selection the runtime replays, so the function runs as compiled where the
/// pack's facts are held, and is refused and recompiled plain where they are not.
#[test]
fn a_body_with_nothing_to_rename_is_spliced_by_the_live_compile() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-live-verbatim",
        &[("vthree", 0, "proc vthree {} {string length abc}")],
    );
    let caller = "proc caller {} {vthree}";
    assert_eq!(
        pack.inlined(&[caller], "caller"),
        Outcome {
            code: Code::Ok,
            answer: "3".to_owned(),
            plain: 0,
            spliced: live_splices(1)
        }
    );
    let mut vm = vm_for_overlay();
    let (plain, spliced) = PlainCounting::installed_counting(&mut vm, service_for(&pack.set), None);
    assert_eq!(
        pack.drive(&mut vm, &plain, &spliced, &[caller], "caller"),
        Outcome {
            code: Code::Ok,
            answer: "3".to_owned(),
            plain: live_splices(1),
            spliced: live_splices(1)
        },
        "no facts held: refused, recompiled plain"
    );
}

/// A body with a `return` before its end, spliced where its value is the
/// procedure's own, answers the value it returned.
#[test]
fn a_body_with_a_return_before_its_end_answers_through_the_splice() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-early-return",
        &[(
            "vabs",
            1,
            "proc vabs {x} {if {$x < 0} {return [expr {-$x}]}; return $x}",
        )],
    );
    let caller = "proc caller {n} {vabs $n}";
    let branch = "proc caller3 {n} {if {$n != 0} {vabs $n} else {return zero}}";
    assert_eq!(pack.bindings(caller, "::caller"), 1, "inlined");
    assert_eq!(pack.bindings(branch, "::caller3"), 1, "inlined");
    for (script, answer) in [
        ("caller -5", "5"),
        ("caller 7", "7"),
        ("caller3 -5", "5"),
        ("caller3 0", "zero"),
    ] {
        assert_eq!(
            pack.inlined(&[caller, branch], script),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0,
                spliced: 0
            },
            "{script}"
        );
    }
}

/// A body that ends in a `return`, spliced into a static `eval` that is the
/// procedure's last command, still hands its value out: the block's value is its
/// last command's, and the procedure's is the block's.
#[test]
fn an_inlined_body_inside_a_static_eval_keeps_its_value() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-eval",
        &[("vdouble", 1, "proc vdouble {x} {return [expr {$x * 2}]}")],
    );
    let caller = "proc caller {n} {eval {vdouble $n}}";
    let after = "proc after {n} {eval {vdouble $n}; return tail}";
    assert_eq!(pack.bindings(caller, "::caller"), 1, "inlined");
    for (script, answer) in [("caller 21", "42"), ("after 21", "tail")] {
        assert_eq!(
            pack.inlined(&[caller, after], script),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0,
                spliced: 0
            },
            "{script}"
        );
    }
}

/// An unqualified command in an inlined body is the one the definition's own
/// namespace resolves it to, not whatever the caller's namespace makes of the
/// name: a caller in `app` that defines its own `string` does not turn the pack's
/// `string length` into that.
#[test]
fn an_inlined_body_resolves_the_commands_it_calls_as_its_definition_did() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-namespace",
        &[("vlen", 1, "proc vlen {s} {string length $s}")],
    );
    let setup = [
        "namespace eval app {}",
        "proc app::string {args} {return shadowed}",
        "proc app::caller {n} {vlen $n}",
    ];
    assert_eq!(
        pack.inlined(&setup, "app::caller abc"),
        Outcome {
            code: Code::Ok,
            answer: "3".to_owned(),
            plain: 0,
            spliced: 0
        }
    );
    // The shadow is made after the caller was compiled, which no compile can see.
    let late = [
        "namespace eval app {}",
        "proc app::caller {n} {vlen $n}",
        "app::caller abc",
        "proc app::string {args} {return shadowed}",
    ];
    let outcome = pack.inlined(&late, "app::caller abc");
    assert_eq!(outcome.answer, "3", "{outcome:?}");
}

/// A command a word substitutes is text to the compiler, which cannot spell it
/// from the global namespace as it does a call: a body that runs one stays a call
/// in another namespace and answers what its definition does, and is spliced
/// where nothing needs spelling.
#[test]
fn a_body_that_substitutes_a_command_is_not_given_the_callers_namespace_to_resolve_it_in() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-namespace-substitution",
        &[
            ("vjoin", 1, "proc vjoin {l} {return [join $l ,]}"),
            (
                "vquoted",
                1,
                "proc vquoted {s} {set n \"[string length $s]x\"; return $n}",
            ),
            (
                "vformat",
                1,
                "proc vformat {x} {return [format %d-%s $x $x]}",
            ),
            ("vdouble", 1, "proc vdouble {x} {return [expr {$x * 2}]}"),
        ],
    );
    for (name, shadow, argument, answer) in [
        ("vjoin", "join", "{a b}", "a,b"),
        ("vquoted", "string", "abc", "3x"),
        ("vformat", "format", "5", "5-5"),
        // The `expr` it substitutes is the statement's own, bound to the live command.
        ("vdouble", "string", "4", "8"),
    ] {
        let setup = [
            "namespace eval app {}".to_owned(),
            format!("proc app::{shadow} {{args}} {{return shadowed}}"),
            format!("proc app::caller {{n}} {{{name} $n}}"),
        ];
        let setup: Vec<&str> = setup.iter().map(String::as_str).collect();
        assert_eq!(
            pack.inlined(&setup, &format!("app::caller {argument}")),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0,
                spliced: 0
            },
            "{name}"
        );
        let global = format!("proc caller {{n}} {{{name} $n}}");
        let namespaced = format!("namespace eval app {{}}\nproc app::caller {{n}} {{{name} $n}}");
        assert_eq!(pack.bindings(&global, "::caller"), 1, "{name}: global");
        assert_eq!(
            pack.bindings(&namespaced, "::app::caller"),
            usize::from(name == "vdouble"),
            "{name}: another namespace"
        );
    }
}

/// A body reads what its definition binds, and nothing of the caller's: a name it
/// never sets is unset in the definition's frame, and a name it sets only on one
/// path is unset on the other, whatever the caller holds of the same spelling.
#[test]
fn an_inlined_body_reads_only_what_its_definition_binds() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-frame",
        &[
            ("vgreet", 0, "proc vgreet {} {return \"hello $name\"}"),
            (
                "vpick",
                1,
                "proc vpick {flag} {if {$flag} {set v 1}; return $v}",
            ),
        ],
    );
    let greet = "proc greet {n} {set name bob; vgreet}";
    let choose = "proc choose {n} {vpick $n}";
    for (script, code, answer) in [
        (
            "greet 1",
            Code::Error,
            "can't read \"name\": no such variable",
        ),
        (
            "choose 0",
            Code::Error,
            "can't read \"v\": no such variable",
        ),
        ("choose 1", Code::Ok, "1"),
    ] {
        assert_eq!(
            pack.inlined(&[greet, choose], script),
            Outcome {
                code,
                answer: answer.to_owned(),
                plain: 0,
                spliced: 0
            },
            "{script}"
        );
    }
}

/// A braced word is literal: `$x` in one is the two characters, and the splice
/// leaves it as the definition has it, in a call's word, a return's value and the
/// text of a command a word substitutes.
#[test]
fn a_braced_word_in_an_inlined_body_stays_literal() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-braced",
        &[
            ("vlit", 1, "proc vlit {x} {string length {$x}}"),
            ("vbrace", 1, "proc vbrace {x} {return {$x}}"),
            (
                "vnested",
                1,
                "proc vnested {x} {return [string length {$x}]}",
            ),
        ],
    );
    for (name, answer, inlined, live) in [
        ("vlit", "2", 1, live_splices(1)),
        ("vbrace", "$x", 1, live_splices(1)),
        ("vnested", "2", 0, 0),
    ] {
        let caller = format!("proc caller {{n}} {{{name} $n}}");
        assert_eq!(pack.bindings(&caller, "::caller"), inlined, "{name}");
        assert_eq!(
            pack.inlined(&[&caller], "caller abc"),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0,
                spliced: live
            },
            "{name}"
        );
    }
}

/// A body that reads a parameter through an operand the compiler keeps as text — a
/// command an expression substitutes, a quoted operand, a loop condition that is a
/// command — is not rewritten for the caller's frame, so it stays a call and
/// answers what its definition does.
#[test]
fn a_body_that_reads_through_an_operand_kept_as_text_stays_a_call() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-text-operands",
        &[
            (
                "vcommand",
                1,
                "proc vcommand {x} {expr {[string length $x] + 1}}",
            ),
            ("vquote", 1, "proc vquote {x} {expr {\"$x\" eq \"abc\"}}"),
            (
                "vwhile",
                1,
                "proc vwhile {x} {set i 0; while {[expr {$i < $x}]} {incr i}; return $i}",
            ),
        ],
    );
    for (name, argument, answer) in [
        ("vcommand", "abc", "4"),
        ("vquote", "abc", "1"),
        ("vwhile", "4", "4"),
    ] {
        let caller = format!("proc caller {{n}} {{{name} $n}}");
        assert_eq!(pack.bindings(&caller, "::caller"), 0, "{name}");
        assert_eq!(
            pack.inlined(&[&caller], &format!("caller {argument}")),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0,
                spliced: 0
            },
            "{name}"
        );
    }
}

/// A function claims the procedure it inlined and no other: where codegen emits a
/// body from the source text of the command that carries it — a `catch`, a `try`,
/// an `uplevel` — a splice made in the lowered copy is never emitted, so the
/// function keeps its call and records no binding for it.
#[test]
fn a_function_claims_a_splice_only_where_it_emits_it() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-claims",
        &[
            ("vdouble", 1, "proc vdouble {x} {expr {$x * 2}}"),
            (
                "vearly",
                1,
                "proc vearly {x} {if {$x < 0} {return neg}; expr {$x * 2}}",
            ),
            (
                "vlocal",
                1,
                "proc vlocal {x} {set y [expr {$x + 1}]; return $y}",
            ),
        ],
    );
    let mut emitted = 0;
    for call in ["vdouble $n", "vearly $n", "vlocal $n"] {
        for (name, template) in [
            ("plain", "proc caller {n} {@CALL@}"),
            ("catch", "proc caller {n} {catch {@CALL@} r; return $r}"),
            (
                "catch twice",
                "proc caller {n} {catch {@CALL@; @CALL@} r; return $r}",
            ),
            (
                "try",
                "proc caller {n} {try {@CALL@} on error {e} {return err}}",
            ),
            (
                "try handler",
                "proc caller {n} {try {error x} on error {e} {@CALL@}}",
            ),
            (
                "try finally",
                "proc caller {n} {try {set k 1} finally {@CALL@}}",
            ),
            ("uplevel", "proc caller {n} {uplevel 0 {@CALL@}}"),
            (
                "catch in a loop",
                "proc caller {n} {foreach i {1 2} {catch {@CALL@}}; return done}",
            ),
        ] {
            let source = template.replace("@CALL@", call);
            let module = service_for(&pack.set)
                .compile_for_profile(&source, tcl9())
                .expect("compiles");
            let caller = fixture_procedure(&module, "::caller");
            assert_eq!(
                !caller.procedure_bindings.is_empty(),
                !calls_a_command(caller),
                "{call} in {name}: bindings {:?}, instructions {:#?}",
                caller.procedure_bindings,
                caller.instructions
            );
            assert_eq!(
                caller.site_claims.is_empty(),
                caller.procedure_bindings.is_empty(),
                "{call} in {name}: a binding is claimed with the pack's facts, and only then"
            );
            emitted += caller.procedure_bindings.len();
        }
    }
    assert!(emitted > 0, "nothing was spliced, so nothing was compared");
}

/// A body whose value somebody reads is not given a wrap: each iteration of an
/// `lmap` gives its body's value to the list it builds, so a body with a `return`
/// stays a call there, a body that is a branch is not one the collector gathers
/// from, and one with neither is spliced as the last command's value.
#[test]
fn an_inlined_body_in_an_lmap_body_gives_the_list_each_iterations_value() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-lmap",
        &[
            ("vdouble", 1, "proc vdouble {x} {return [expr {$x * 2}]}"),
            ("vtail", 1, "proc vtail {x} {expr {$x * 2}}"),
            (
                "vif",
                1,
                "proc vif {x} {if {$x < 0} {set r neg} else {set r pos}}",
            ),
            ("vnoop", 0, "proc vnoop {} {}"),
        ],
    );
    for (label, source, answer, bindings) in [
        (
            "a body with a return",
            "proc caller {n} {lmap i {1 2 3} {vdouble $n}}",
            "14 14 14",
            0,
        ),
        (
            "a body with none",
            "proc caller {n} {lmap i {1 2 3} {vtail $n}}",
            "14 14 14",
            1,
        ),
        (
            "a branch",
            "proc caller {n} {lmap i {1 2} {vif $n}}",
            "pos pos",
            0,
        ),
        (
            "an empty body alone",
            "proc caller {n} {lmap i {1 2} {vnoop}}",
            "{} {}",
            1,
        ),
        (
            "an empty body after a command",
            "proc caller {n} {lmap i {1 2} {set a 5; vnoop}}",
            "{} {}",
            0,
        ),
        (
            "a dict map",
            "proc caller {n} {dict map {k v} {a 1 b 2} {vdouble $n}}",
            "a 14 b 14",
            0,
        ),
    ] {
        assert_eq!(pack.bindings(source, "::caller"), bindings, "{label}");
        assert_eq!(
            pack.inlined(&[source], "caller 7"),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0,
                spliced: 0
            },
            "{label}"
        );
    }
}

/// A braced word is literal where the wrap stores a value as well: `{a\tb}` and
/// `{[string length $x]}` are the characters they are, whether the value is the
/// last command's or an early `return`'s. The answers are Tcl's, written down: the
/// VM's own compile of `return {[…]}` evaluates the brackets, so the matrix, which
/// holds a splice to the definition run as a procedure, has no reference for it.
#[test]
pub(super) fn a_braced_literal_stays_literal_through_the_wrap() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-braced-wrap",
        &[
            (
                "vsetbs",
                1,
                "proc vsetbs {x} {if {$x < 0} {return neg}; set r {a\\tb}}",
            ),
            (
                "vretbs",
                1,
                "proc vretbs {x} {if {$x > 0} {return {a\\tb}}; return neg}",
            ),
            (
                "vsetbr",
                1,
                "proc vsetbr {x} {if {$x < 0} {return neg}; set r {[string length $x]}}",
            ),
            (
                "vretbr",
                1,
                "proc vretbr {x} {if {$x > 0} {return {[string length $x]}}; return neg}",
            ),
        ],
    );
    for (name, answer) in [
        ("vsetbs", r"a\tb"),
        ("vretbs", r"a\tb"),
        ("vsetbr", "[string length $x]"),
        ("vretbr", "[string length $x]"),
    ] {
        let caller = format!("proc caller {{n}} {{{name} $n}}");
        assert_eq!(pack.bindings(&caller, "::caller"), 1, "{name}");
        assert_eq!(
            pack.inlined(&[&caller], "caller 7"),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0,
                spliced: 0
            },
            "{name}"
        );
    }
}

/// A definition in a namespace of its own resolves each command it names there
/// first and in the global namespace after it, so a `vendor::string` the package
/// defines answers for `vendor::vlen`'s `string`, made before the caller was
/// compiled or after it ran. No spelling says that to a caller elsewhere, so the
/// body is spliced only where the caller looks names up as the definition did.
#[test]
fn a_definition_in_a_namespace_resolves_its_commands_as_it_did() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-definition-namespace",
        &[
            (
                "vendor::vlen",
                1,
                "proc vendor::vlen {s} {string length $s}",
            ),
            (
                "vendor::vinc",
                1,
                "proc vendor::vinc {x} {return [expr {$x + 1}]}",
            ),
        ],
    )
    .in_namespace("vendor");
    let shadow = "proc vendor::string {args} {return shadowed}";
    for (source, name, call) in [
        (
            "proc caller {n} {vendor::vlen $n}",
            "::caller",
            "caller abc",
        ),
        (
            "namespace eval app {}\nproc app::caller {n} {vendor::vlen $n}",
            "::app::caller",
            "app::caller abc",
        ),
    ] {
        assert_eq!(pack.bindings(source, name), 0, "{call}");
        for (setup, answer) in [
            (vec![source], "3"),
            (vec![shadow, source], "shadowed"),
            (vec![source, call, shadow], "shadowed"),
        ] {
            assert_eq!(
                pack.inlined(&setup, call),
                Outcome {
                    code: Code::Ok,
                    answer: answer.to_owned(),
                    plain: 0,
                    spliced: 0
                },
                "{call} with {setup:?}"
            );
        }
    }
    // A body that names no command by a word the compiler keeps is spliced anywhere,
    // and what lowering consumed of it is held to the live command in the
    // definition's namespace: a `vendor::expr` made later is the one that answers.
    let increment = "proc caller {n} {vendor::vinc $n}";
    assert_eq!(pack.bindings(increment, "::caller"), 1, "names no command");
    let expr_shadow = "proc vendor::expr {args} {return shadowed}";
    for (setup, answer) in [
        (vec![increment], "8"),
        (vec![expr_shadow, increment], "shadowed"),
        (vec![increment, "caller 7", expr_shadow], "shadowed"),
    ] {
        let outcome = pack.inlined(&setup, "caller 7");
        assert_eq!(
            (outcome.code, outcome.answer.as_str()),
            (Code::Ok, answer),
            "{setup:?}"
        );
    }
    let own = "proc vendor::caller {n} {vlen $n}";
    assert_eq!(
        pack.bindings(own, "::vendor::caller"),
        1,
        "its own namespace"
    );
    assert_eq!(
        pack.inlined(&[own], "vendor::caller abc"),
        Outcome {
            code: Code::Ok,
            answer: "3".to_owned(),
            plain: 0,
            spliced: live_splices(1)
        }
    );
}

/// A variable a command a word substitutes is handed by name is the definition's
/// own, and the splice has no `$` there to rewrite: `[set y]` would read the
/// caller's `y`, and `[incr y]` would write it.
#[test]
fn a_variable_a_substituted_command_names_is_the_definitions_own() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-named-variables",
        &[
            ("vsetcmd", 1, "proc vsetcmd {x} {set y $x; return [set y]}"),
            (
                "vincrcmd",
                1,
                "proc vincrcmd {x} {set y $x; return [incr y]}",
            ),
            (
                "vfmtcmd",
                1,
                "proc vfmtcmd {x} {return [format %s [set x]]}",
            ),
            ("vexprcmd", 1, "proc vexprcmd {x} {expr {[set x] + 1}}"),
        ],
    );
    for (name, answer) in [
        ("vsetcmd", "1"),
        ("vincrcmd", "2"),
        ("vfmtcmd", "1"),
        ("vexprcmd", "2"),
    ] {
        let source = format!("proc caller {{n}} {{set x 42; set y 43; {name} $n}}");
        assert_eq!(pack.bindings(&source, "::caller"), 0, "{name}");
        assert_eq!(
            pack.inlined(&[&source], "caller 1"),
            Outcome {
                code: Code::Ok,
                answer: answer.to_owned(),
                plain: 0,
                spliced: 0
            },
            "{name}"
        );
    }
}

/// The code generator answers the empty string for the value of an `if` that ends
/// an arm the procedure answers, so a call whose body is one is spliced where its
/// value is the procedure's own and stays a call in an arm.
#[test]
fn a_call_whose_body_is_a_branch_is_not_given_the_value_of_an_arm() {
    crate::install_surfaces();
    let pack = Spliced::new(
        "splice-arm",
        &[(
            "vif",
            1,
            "proc vif {x} {if {$x < 0} {set r neg} else {set r pos}}",
        )],
    );
    for (label, source, bindings) in [
        ("terminal", "proc caller {n} {vif $n}", 1),
        (
            "then arm",
            "proc caller {n} {if {$n != 0} {vif $n} else {return zero}}",
            0,
        ),
        (
            "else arm",
            "proc caller {n} {if {$n == 0} {return zero} else {vif $n}}",
            0,
        ),
        (
            "switch arm",
            "proc caller {n} {switch -- $n {0 {return zero} default {vif $n}}}",
            0,
        ),
    ] {
        assert_eq!(pack.bindings(source, "::caller"), bindings, "{label}");
        assert_eq!(
            pack.inlined(&[source], "caller 7"),
            Outcome {
                code: Code::Ok,
                answer: "pos".to_owned(),
                plain: 0,
                spliced: 0
            },
            "{label}"
        );
    }
}
