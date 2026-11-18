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

use super::*;
use tcl_compiler::analyser::Analyser;

#[test]
fn inline_arguments_use_complete_original_words_and_shared_literal_values() {
    for source in [
        "proc show {value} {puts $value}\nshow a\\ b\n",
        "proc show {value} {puts $value}\nshow \"a\\x20b\"\n",
        "proc show {value} {puts $value}\nshow {a b}\n",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let walk = super::super::FrameWalk::new(source, &analysis).unwrap();
        let commands = walk.segment_document(source);
        let call = commands
            .iter()
            .find(|command| command.name() == "show")
            .unwrap();
        let procedure = &analysis.all_procs["::show"];
        let bindings = bind_arguments(source, call, procedure, &walk).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].value, "a b");
        assert!(!bindings[0].evaluation_is_observable);
        assert!(outcome_for(source, "show ", tcl_dialect::DialectProfile::plain_tcl()).is_err());
    }
    let source = "proc show {value} {puts $value}\nshow prefix$computed\n";
    assert!(
        outcome_for(
            source,
            "show prefix",
            tcl_dialect::DialectProfile::plain_tcl()
        )
        .is_err()
    );
}

/// Run the transform at the first occurrence of `needle` in `src`, with the
/// document analysed under `dialect`.
fn at_dialect(
    src: &str,
    needle: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Option<Refactoring> {
    let registry = super::super::test_registry();
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(src, dialect.name).clone();
    let cursor = u32::try_from(src.find(needle).expect("needle in source")).unwrap() + 1;
    inline_proc(src, cursor, &analysis, &registry)
}

/// Run the transform at the first occurrence of `needle` in `src`.
fn at(src: &str, needle: &str) -> Option<Refactoring> {
    at_dialect(
        src,
        needle,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    )
}

/// The rewritten document, or the refusal reason, under `dialect`.
fn outcome_for(
    src: &str,
    needle: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Result<String, String> {
    let refactoring = at_dialect(src, needle, dialect).expect("a call to inline");
    match &refactoring.disabled {
        Some(reason) => Err(reason.clone()),
        None => Ok(refactoring.apply(src)),
    }
}

/// The rewritten document, or the refusal reason.
fn outcome(src: &str, needle: &str) -> Result<String, String> {
    outcome_for(
        src,
        needle,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    )
}

// -- FP: a body's nested statements are guarded like its top level ----

/// The body is one command, but that command carries a script. A `set`
/// inside it lands in the caller's frame just as a top-level one would.
///
/// Oracle (tclsh 8.6.18 and 9.0.4 alike): `total` is 99 after the call in
/// the original and 0 if the body is inlined, so the extraction is refused.
#[test]
fn fp_refuses_a_write_nested_in_a_body() {
    let src = "proc reset {} {\n    if {1} {\n        set total 0\n    }\n}\nset total 99\nreset\n";
    let reason = outcome(src, "reset\n").unwrap_err();
    assert!(reason.contains("assigns or binds variables"), "{reason}");
}

/// A loop's own binding leaks exactly as an assignment does, and it is a
/// `LoopVarList` word rather than a `VarWrite` one.
///
/// Oracle: `n` keeps its old value after the call and holds 3 if the body
/// is inlined.
#[test]
fn fp_refuses_a_loop_binding_in_the_body() {
    let src = "proc show {} {\n    foreach n {1 2 3} {\n        puts $n\n    }\n}\nshow\n";
    let reason = outcome(src, "show\n").unwrap_err();
    assert!(reason.contains("binds variables"), "{reason}");
}

/// A frame-sensitive command one level down is still frame-bound: a
/// `return` inside an `if` body would return from the *caller* once
/// inlined.
#[test]
fn fp_refuses_a_frame_sensitive_command_nested_in_a_body() {
    let src = "proc grab {} {\n    if {1} {\n        global config\n    }\n}\ngrab\n";
    let reason = outcome(src, "grab\n").unwrap_err();
    assert!(reason.contains("acts on the call frame"), "{reason}");
}

/// An expression operand one level below the body's own command is still
/// an expression operand.
///
/// Oracle: `check abc` prints 1, while `puts [expr {abc eq "abc"}]` fails
/// with `invalid bareword "abc"` on tclsh 8.6.18 and 9.0.4.
#[test]
fn fp_refuses_a_bareword_bound_to_a_nested_expression_operand() {
    let src = "proc check {n} {\n    puts [expr {$n eq \"abc\"}]\n}\ncheck abc\n";
    let reason = outcome(src, "check abc").unwrap_err();
    assert!(reason.contains("expression variable"), "{reason}");
}

// -- TP: binding is performed and the result is correct ---------------

#[test]
fn tp_inlines_a_literal_argument() {
    let src = "proc double {x} {\n    expr {$x * 2}\n}\ndouble 5\n";
    assert_eq!(
        outcome(src, "double 5").unwrap(),
        "proc double {x} {\n    expr {$x * 2}\n}\nexpr {5 * 2}\n"
    );
}

#[test]
fn tp_binds_a_declared_default_when_the_call_omits_the_argument() {
    // C Tcl 9.0.3 prints `hello world` for the original.  The old textual
    // splice emitted `puts $name`, which errors on an unset variable.
    let src = "proc greet {{name world}} {\n    puts $name\n}\ngreet\n";
    assert_eq!(
        outcome(src, "greet\n").unwrap(),
        "proc greet {{name world}} {\n    puts $name\n}\nputs world\n"
    );
}

#[test]
fn tp_a_written_argument_overrides_the_default() {
    let src = "proc greet {{name world}} {\n    puts $name\n}\ngreet there\n";
    assert_eq!(
        outcome(src, "greet there").unwrap(),
        "proc greet {{name world}} {\n    puts $name\n}\nputs there\n"
    );
}

#[test]
fn tp_repeated_parameter_use_is_fine_for_a_literal() {
    let src = "proc sq {n} {\n    expr {$n * $n}\n}\nsq 7\n";
    assert_eq!(
        outcome(src, "sq 7").unwrap(),
        "proc sq {n} {\n    expr {$n * $n}\n}\nexpr {7 * 7}\n"
    );
}

#[test]
fn tp_an_unread_args_tail_does_not_block_inlining() {
    let src = "proc shout {word args} {\n    puts $word\n}\nshout hi extra\n";
    assert_eq!(
        outcome(src, "shout hi").unwrap(),
        "proc shout {word args} {\n    puts $word\n}\nputs hi\n"
    );
}

#[test]
fn tp_a_prefix_sharing_parameter_name_is_not_corrupted() {
    // tclsh 8.6.18 and 9.0.4: a free `$nn` fails in the proc, so inlining
    // refuses; bound, `$nn` keeps its own value beside `$n`.
    let src = "proc f {n} {\n    puts $n$nn\n}\nf 1\n";
    assert!(outcome(src, "f 1\n").is_err());
    let src = "proc f {n nn} {\n    list $n $nn\n}\nf 1 2\n";
    let result = outcome(src, "f 1 2\n").unwrap();
    assert!(result.ends_with("list 1 2\n"), "{result}");
}

// FP: refusals that keep behaviour.

#[test]
fn fp_refuses_a_braced_argument_whose_value_is_not_a_plain_word() {
    // Original prints `hello a b`; the
    // textual splice emitted `puts "hello {a b}"`, printing the braces.
    let src = "proc greet {name} {\n    puts \"hello $name\"\n}\ngreet {a b}\n";
    let reason = outcome(src, "greet {a b}").unwrap_err();
    assert!(reason.contains("inert literal"), "{reason}");
}

#[test]
fn fp_refuses_a_substituted_argument() {
    let src = "proc double {x} {\n    expr {$x * 2}\n}\ndouble $n\n";
    let reason = outcome(src, "double $n").unwrap_err();
    assert!(reason.contains("substituted at run time"), "{reason}");
}

#[test]
fn fp_refuses_a_non_numeric_value_in_an_expression_operand() {
    // `expr {abc * 2}` reads `abc` as a function name, not the string.
    let src = "proc double {x} {\n    expr {$x * 2}\n}\ndouble abc\n";
    let reason = outcome(src, "double abc").unwrap_err();
    assert!(reason.contains("nonnumeric"), "{reason}");
}

#[test]
fn fp_refuses_too_few_arguments() {
    let src = "proc pair {a b} {\n    puts $a$b\n}\npair 1\n";
    let reason = outcome(src, "pair 1\n").unwrap_err();
    assert!(reason.contains("wrong # args"), "{reason}");
}

#[test]
fn fp_refuses_too_many_arguments() {
    let src = "proc one {a} {\n    puts $a\n}\none 1 2\n";
    let reason = outcome(src, "one 1 2").unwrap_err();
    assert!(reason.contains("bind its original formals"), "{reason}");
}

#[test]
fn fp_refuses_a_body_that_reads_args() {
    let src = "proc all {args} {\n    puts $args\n}\nall 1 2\n";
    let reason = outcome(src, "all 1 2").unwrap_err();
    assert!(reason.contains("list"), "{reason}");
}

#[test]
fn fp_refuses_a_body_that_assigns_a_variable() {
    // `total` would survive in the caller and clobber a same-named one.
    let src = "proc reset {} {\n    set total 0\n}\nreset\n";
    let reason = outcome(src, "reset\n").unwrap_err();
    assert!(reason.contains("assigns or binds variables"), "{reason}");
}

#[test]
fn fp_refuses_a_body_using_upvar() {
    let src = "proc bind {name} {\n    upvar 1 $name local\n}\nbind x\n";
    let reason = outcome(src, "bind x\n").unwrap_err();
    assert!(reason.contains("call frame"), "{reason}");
}

#[test]
fn fp_refuses_a_body_using_return() {
    let src = "proc answer {} {\n    return 42\n}\nanswer\n";
    let reason = outcome(src, "answer\n").unwrap_err();
    assert!(reason.contains("call frame"), "{reason}");
}

#[test]
fn fp_refuses_a_body_using_global() {
    let src = "proc grab {} {\n    global config\n}\ngrab\n";
    let reason = outcome(src, "grab\n").unwrap_err();
    assert!(reason.contains("call frame"), "{reason}");
}

#[test]
fn fp_refuses_a_multi_command_body() {
    let src = "proc two {} {\n    puts a\n    puts b\n}\ntwo\n";
    let reason = outcome(src, "two\n").unwrap_err();
    assert!(reason.contains("commands"), "{reason}");
}

#[test]
fn fp_refuses_an_empty_body() {
    let src = "proc nothing {} {}\nnothing\n";
    let reason = outcome(src, "nothing\n").unwrap_err();
    assert!(reason.contains("0 commands"), "{reason}");
}

#[test]
fn fp_refuses_a_computed_parameter_list() {
    let src = "proc makeargs {} {return {a b}}\nproc p [makeargs] {\n    puts $a\n}\np 1 2\n";
    // The formals are a run-time value, so no procedure is offered.
    assert!(at(src, "p 1 2").is_none());
}

// TN: nothing to offer.

#[test]
fn tn_no_action_on_a_builtin_call() {
    assert!(at("puts hello\n", "puts").is_none());
}

#[test]
fn tn_no_action_on_an_unresolvable_head() {
    assert!(at("frobnicate 1\n", "frobnicate").is_none());
}

#[test]
fn tn_no_action_outside_any_command() {
    assert!(at("\n\n", "\n").is_none());
}

// Unit-level predicates.

#[test]
fn plain_word_rejects_every_parser_significant_character() {
    assert!(is_plain_word("abc"));
    assert!(is_plain_word("1.5"));
    assert!(is_plain_word("a-b_c.d/e"));
    assert!(!is_plain_word(""));
    for value in ["a b", "a$b", "a[b", "a{b", "a\"b", "a\\b", "a;b", "a#b"] {
        assert!(!is_plain_word(value), "{value} must not be a plain word");
    }
}

#[test]
fn expr_literal_accepts_what_expr_parses_as_a_number() {
    let n = NumberSyntax::Tcl90;
    for value in ["0", "42", "-7", "+7", "1.5", "1e3", "0xff", "0b1010"] {
        assert!(is_expr_literal(value, n), "{value} is a number");
    }
    for value in ["abc", "", "-", "1a2"] {
        assert!(!is_expr_literal(value, n), "{value} is not a number");
    }
}

/// The bug the shared numeral facility fixes: the old recogniser accepted
/// any *alphanumeric* run after a radix prefix, so `0xZZZ` / `0b999` /
/// `0o8` passed as numbers.  Splicing one of those into an `expr` operand
/// turns a working script into a runtime error — `expr` reads a
/// non-numeric bareword as a function name.
#[test]
fn expr_literal_rejects_radix_invalid_digits() {
    for syntax in [
        NumberSyntax::Tcl84,
        NumberSyntax::Tcl85,
        NumberSyntax::Tcl90,
    ] {
        for value in ["0xZZZ", "0b999", "0o8", "0b2", "0x", "0x_", "--5", "+-5"] {
            assert!(
                !is_expr_literal(value, syntax),
                "{value} is not a number under {syntax:?}"
            );
        }
    }
}

/// A radix prefix only exists from the release that added it: `0o`/`0b`
/// from 8.5, `0d` from 9.0, `_` digit separators from 9.0.  Before that the
/// word is a bareword, which `expr` would read as a function name.
#[test]
fn expr_literal_gates_radix_prefixes_on_the_release() {
    for syntax in [NumberSyntax::Tcl85, NumberSyntax::Tcl90] {
        assert!(is_expr_literal("0o17", syntax), "{syntax:?}");
    }
    assert!(!is_expr_literal("0o17", NumberSyntax::Tcl84));
    assert!(is_expr_literal("0d99", NumberSyntax::Tcl90));
    assert!(is_expr_literal("1_000", NumberSyntax::Tcl90));
    for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
        assert!(!is_expr_literal("0d99", syntax), "{syntax:?}");
        assert!(!is_expr_literal("1_000", syntax), "{syntax:?}");
    }
}

/// End-to-end: the dialect the document was analysed under decides whether
/// an `0o17` argument may be substituted into an `expr` operand.
#[test]
fn fp_refuses_a_radix_prefix_the_target_release_lacks() {
    let src = "proc double {x} {\n    expr {$x * 2}\n}\ndouble 0o17\n";
    assert_eq!(
        outcome_for(
            src,
            "double 0o17",
            tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile()
        )
        .unwrap(),
        "proc double {x} {\n    expr {$x * 2}\n}\nexpr {0o17 * 2}\n"
    );
    let reason = outcome_for(
        src,
        "double 0o17",
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    )
    .unwrap_err();
    assert!(reason.contains("nonnumeric"), "{reason}");
}

/// The bug end-to-end: `0xZZZ` is not a number in any release, so the
/// refactor must refuse rather than emit `expr {0xZZZ * 2}`.
#[test]
fn fp_refuses_radix_invalid_digits_in_an_expr_operand() {
    let src = "proc double {x} {\n    expr {$x * 2}\n}\ndouble 0xZZZ\n";
    let reason = outcome(src, "double 0xZZZ").unwrap_err();
    assert!(reason.contains("nonnumeric"), "{reason}");
}

#[test]
fn inline_original_components_keep_literal_and_escaped_parameter_spelling() {
    for source in [
        "proc show {value} {puts {$value}}\nshow TEXT\n",
        "proc show {value} {puts \"escaped=\\$value $value\"}\nshow TEXT\n",
    ] {
        let result = outcome(source, "show TEXT").unwrap();
        assert!(result.contains("$value"), "{result}");
        assert!(!result.contains("{$TEXT}"), "{result}");
        if source.contains("escaped=") {
            assert!(result.contains("escaped=\\$value TEXT"), "{result}");
        } else {
            assert!(result.contains("puts {$value}"), "{result}");
        }
    }
    let source = "proc same {value} {expr {{${value}} eq \"$value\"}}\nsame 7\n";
    let nonnumeric = source.replace("same 7", "same TEXT");
    assert!(
        outcome(&nonnumeric, "same TEXT")
            .unwrap_err()
            .contains("nonnumeric")
    );
    let result = outcome(source, "same 7").unwrap();
    assert!(result.ends_with("expr {{${value}} eq \"7\"}\n"), "{result}");
    let source = "proc show {value} {puts $array($value)}\nshow TEXT\n";
    assert!(
        outcome(source, "show TEXT")
            .unwrap_err()
            .contains("array reference")
    );
}

#[test]
fn variable_references_finds_whole_names_only() {
    let found = variable_references("puts $n$nn ${n}x", BracedVarStyle::Tcl9Nesting);
    let names: Vec<&str> = found.iter().map(|(name, _, _)| name.as_str()).collect();
    assert_eq!(names, vec!["n", "nn", "n"]);
}

/// Inline-proc **rewrites** each reference's own byte
/// span, so the span must be the one the document's release parses. On a
/// 9.x document `${a{b}c}` is one reference spanning all 8 bytes; on 8.x
/// it ends at the first `}` and the trailing `c}` is word text that must
/// survive the substitution untouched.
///
/// Oracle: `puts ${a{b}c}` prints `NINE` on tclsh 9.0.4 and `EIGHTc}` on
/// 8.6.16.
#[test]
fn variable_reference_spans_follow_the_documents_release() {
    let text = "puts ${a{b}c}";
    let nine = variable_references(text, BracedVarStyle::Tcl9Nesting);
    assert_eq!(nine.len(), 1, "{nine:?}");
    assert_eq!(nine[0].0, "a{b}c");
    // `$` at 5 through the closing `}` at 12 inclusive.
    assert_eq!((nine[0].1, nine[0].2), (5, 13));

    let eight = variable_references(text, BracedVarStyle::FirstClose);
    assert_eq!(eight.len(), 1, "{eight:?}");
    assert_eq!(eight[0].0, "a{b");
    // The reference stops before `c}`, which stays ordinary word text.
    assert_eq!((eight[0].1, eight[0].2), (5, 11));
    assert_eq!(&text[eight[0].2..], "c}");
}
