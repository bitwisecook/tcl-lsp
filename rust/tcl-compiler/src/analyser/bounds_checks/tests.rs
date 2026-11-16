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

use super::{
    BoundsMetadataContext, any_command_recursive, infer_list_length_from_recent_set, sole_command,
};
use crate::analyser::Analyser;
use tcl_core_types::DiagCode;

fn config() -> tcl_lexer::LexerConfig {
    tcl_lexer::LexerConfig::default()
}

fn codes(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W240" | "W241"))
        .map(|d| d.code.to_string())
        .collect()
}

/// A `break` in a switch clause action runs in the loop's own frame, so
/// it ends the loop (tclsh: `while 1 {switch a {a {break}}}` returns),
/// whatever the solver decides about the constant header.
#[test]
fn w241_is_silent_when_a_switch_clause_breaks_the_loop() {
    for source in [
        "while 1 { switch -glob $x {a* {break} default {break}} }\n",
        "proc f {x} { while 1 { switch -glob $x {a* {break} default {break}} }; puts $y }\n",
        "while 1 { switch -- $x a {break} default {return} }\n",
    ] {
        assert!(codes(source).is_empty(), "{source}: {:?}", codes(source));
    }
    assert_eq!(codes("while 1 { switch $x {a {puts a}} }\n"), vec!["W241"]);
}

#[test]
fn w240_constant_false_condition() {
    assert_eq!(codes("while 0 {puts hi}\n"), vec!["W240"]);
    assert_eq!(codes("for {set i 0} 0 {incr i} {}\n"), vec!["W240"]);
}

#[test]
fn configured_bounds_scanners_cover_each_segmenter_site() {
    // Mutation proof for the three bounds callers: iRules treats `}{` as
    // a ghost separator, whereas the default Tcl lexer keeps the second
    // braced word attached to the first command.
    let config = tcl_lexer::LexerConfig::for_dialect("f5-irules");
    let one = sole_command("cmd {a}{b}", config).expect("one command");
    assert_eq!(one.args().len(), 2, "iRules words: {:?}", one.texts);
    assert!(any_command_recursive(
        "cmd {a}{b}",
        BoundsMetadataContext::Standalone(None),
        config,
        &mut |cmd| { cmd.args().len() == 2 }
    ));

    let source = "set xs {a b}\nlset xs 9 v\n";
    let before = u32::try_from(source.find("lset").expect("lset")).expect("offset");
    assert_eq!(
        infer_list_length_from_recent_set(
            source,
            "xs",
            before,
            super::BoundsMetadataContext::Standalone(None),
            config,
        ),
        Some(2)
    );
}

/// #2402: a `$body` word holds no script the scan can read, and a `for`
/// init that leaves the procedure never reaches the body; tclsh 8.4.20
/// to 9.1.0 end each of these loops.
#[test]
fn w241_declines_a_variable_body_and_an_exiting_for_init() {
    for source in [
        "proc p {} { set body {return 1}; while 1 $body; return none }\n",
        "set body {break}; while 1 $body; puts after\n",
        "proc p {} { for {return 1} {1} {} {}; return none }\n",
    ] {
        assert!(codes(source).is_empty(), "{source}: {:?}", codes(source));
    }
    assert_eq!(codes("for {set i 0} {1} {} {}\n"), vec!["W241"]);
}

#[test]
fn w241_constant_true_no_exit() {
    assert_eq!(codes("while 1 {puts hi}\n"), vec!["W241"]);
    // A `break` in the body suppresses W241.
    assert_eq!(codes("while 1 {break}\n"), [] as [std::string::String; 0]);
    assert_eq!(codes("while 1 {return}\n"), [] as [std::string::String; 0]);
}

#[test]
fn w241_throw_and_tailcall_leave_the_loop() {
    // `throw` and `tailcall` both terminate the loop after one
    // iteration (verified against tclsh 9.0.4), so a `while 1` body
    // containing either is NOT provably infinite. `throw` resolves via the
    // registry's TERMINATES_BLOCK trait; `tailcall` is named explicitly.
    assert_eq!(
        codes("while 1 {throw MYERR boom}\n").len(),
        0,
        "throw must suppress W241",
    );
    assert_eq!(
        codes("while 1 {tailcall foo}\n").len(),
        0,
        "tailcall must suppress W241",
    );
    // TP controls: the other block-terminators still suppress, and a body
    // with no exit at all still fires.
    assert_eq!(
        codes("while 1 {error boom}\n"),
        [] as [std::string::String; 0]
    );
    assert_eq!(codes("while 1 {exit 1}\n"), [] as [std::string::String; 0]);
    assert_eq!(codes("while 1 {incr n}\n"), vec!["W241"]);
    // `continue` is NOT an exit — it keeps looping — so W241 still fires.
    assert_eq!(codes("while 1 {continue}\n"), vec!["W241"]);
}

/// Every loop-termination verdict (W240, W241, W242) `src` draws, in
/// source order.
fn loop_verdicts(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    let mut found: Vec<(u32, String)> = a
        .analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W240" | "W241" | "W242"))
        .map(|d| (d.span.start(), d.code.to_string()))
        .collect();
    found.sort();
    found.into_iter().map(|(_, code)| code).collect()
}

/// W240 and W241 read the loop header's branch fact instead of the
/// condition's text: a header the solver decides false at entry is W240,
/// one it decides true at every test with no executable exit is W241, and
/// either suppresses W242 — where the text alone can say nothing but the
/// hint a counter nothing modifies draws. A header it does not decide keeps
/// the text's verdict, and an exit anywhere the flow graph or the body's
/// text finds keeps a true header silent.
#[test]
fn w240_and_w241_read_the_branch_fact() {
    // `n` is 0 at every test and `go` is 1: the branch fact decides.
    assert_eq!(
        loop_verdicts("set n 0\nwhile {$n} {puts \"never runs\"}\n"),
        ["W240"]
    );
    assert_eq!(loop_verdicts("set go 1\nwhile {$go} {puts x}\n"), ["W241"]);
    assert_eq!(
        loop_verdicts("proc p {} {\n set n 0\n while {$n} {puts x}\n}\n"),
        ["W240"]
    );
    assert_eq!(
        loop_verdicts("proc p {} {\n set go 1\n while {$go} {puts x}\n}\n"),
        ["W241"]
    );
    // A `for` whose counter never moves is infinite whatever its bound
    // says; one whose first test fails never runs.
    assert_eq!(
        loop_verdicts("for {set i 0} {$i < 10} {} {puts hi}\n"),
        ["W241"]
    );
    assert_eq!(
        loop_verdicts("for {set i 0} {$i < 0} {incr i} {puts hi}\n"),
        ["W240"]
    );
    // The literal forms decide too, with the text's own message.
    assert_eq!(loop_verdicts("while 0 {puts hi}\n"), ["W240"]);
    assert_eq!(loop_verdicts("while 1 {puts hi}\n"), ["W241"]);
    // An exit the flow graph finds keeps a true header silent: `break`,
    // `return`, `error`; and so does one it cannot see, inside a `catch`
    // body the graph does not lower.
    for body in ["break", "return", "error boom", "catch {break}"] {
        assert!(
            loop_verdicts(&format!(
                "proc p {{}} {{\n set go 1\n while {{$go}} {{{body}}}\n}}\n"
            ))
            .is_empty(),
            "{body}"
        );
    }
    // Nothing decides a parameter or an unset variable: the text's
    // verdict stands, W242's hint for a counter nothing modifies.
    assert_eq!(
        loop_verdicts("proc p {n} {\n while {$n} {puts x}\n}\n"),
        ["W242"]
    );
    assert_eq!(loop_verdicts("while {$x < 10} {puts hi}\n"), ["W242"]);
    // A counter the body advances is never proven either way.
    assert_eq!(
        loop_verdicts("proc p {} {\n set i 0\n while {$i < 3} {incr i}\n}\n"),
        [] as [std::string::String; 0]
    );
}

/// A loop's own test decided false names the loop in its I230, beside the
/// W240 of a body that never runs; a decided `if` keeps its own wording.
#[test]
fn a_loop_test_decided_false_names_the_loop() {
    let i230 = |src: &str| -> Vec<String> {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .into_iter()
            .filter(|d| d.code == DiagCode::I230)
            .map(|d| d.message)
            .collect()
    };
    assert_eq!(
        i230("set n 0\nwhile {$n} {puts never}\n"),
        ["Loop condition '$n' is never true; the loop leaves at this test"]
    );
    assert_eq!(
        i230("proc p {} {\n for {set i 0} {$i < 0} {incr i} {puts never}\n}\n"),
        ["Loop condition '$i < 0' is never true; the loop leaves at this test"]
    );
    assert_eq!(
        i230("set n 0\nif {$n} {puts never}\n"),
        ["Condition '$n' is always false; the consequent branch is unreachable"]
    );
}

/// A loop inside a proc body reaches the same verdict on the per-item
/// path, which walks each body in isolation and grafts the candidates
/// back at their real positions.
#[test]
fn loop_verdicts_agree_across_the_whole_file_and_per_item_walks() {
    let src = "proc a {} {\n set n 0\n while {$n} {puts x}\n}\n\
                   proc b {} {\n set go 1\n while {$go} {puts y}\n}\n\
                   proc c {n} {\n while {$n} {puts z}\n}\n\
                   set m 0\nwhile {$m} {puts top}\n";
    let verdicts = |per_item: bool| -> Vec<(u32, String)> {
        let mut a = Analyser::new();
        let result = if per_item {
            a.analyse_per_item(src, "tcl8.6")
        } else {
            a.analyse(src, "tcl8.6")
        };
        let mut found: Vec<(u32, String)> = result
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W240" | "W241" | "W242"))
            .map(|d| (d.span.start(), d.code.to_string()))
            .collect();
        found.sort();
        found
    };
    let whole = verdicts(false);
    let codes: Vec<&str> = whole.iter().map(|(_, code)| code.as_str()).collect();
    assert_eq!(codes, ["W240", "W241", "W242", "W240"], "{whole:?}");
    assert_eq!(verdicts(true), whole);
}

/// A registry whose loop vocabulary is not Tcl's: `spin` loops, `peek`
/// has the identical argument shape and does not.
fn registry_with_a_declared_loop() -> tcl_registry::CommandRegistry {
    use tcl_registry::arg_role::ArgRole;
    use tcl_registry::spec::CommandSpec;

    const ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Expr), (1, ArgRole::Body)];
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "spin",
        traits: tcl_registry::Traits::CONTROL_FLOW
            .union(tcl_registry::Traits::HAS_BOOLEAN_COND)
            .union(tcl_registry::Traits::HAS_LOOP_BODY),
        arity: tcl_registry::arity::Arity::exact(2),
        arg_roles: ROLES,
        ..CommandSpec::DEFAULT
    });
    registry.insert(CommandSpec {
        name: "peek",
        traits: tcl_registry::Traits::HAS_BOOLEAN_COND,
        arity: tcl_registry::arity::Arity::exact(2),
        arg_roles: ROLES,
        ..CommandSpec::DEFAULT
    });
    registry
}

fn loop_codes(src: &str, registry: &tcl_registry::CommandRegistry) -> Vec<DiagCode> {
    let command = sole_command(src, config()).expect("one command");
    let name = command.texts.first().expect("a command word").clone();
    super::loop_termination_diagnostics(
        &name,
        command.args(),
        command.arg_tokens(),
        super::BoundsMetadataContext::Standalone(Some(registry)),
        config(),
        &tcl_dialect::LexerGrammar::default(),
    )
    .iter()
    .map(|d| d.code)
    .collect()
}

#[test]
fn a_declared_loop_is_analysed_under_whatever_name_it_has() {
    // The loop question is the registry's: a command carrying
    // HAS_LOOP_BODY reaches W240 / W241 with no name written here, which
    // is what lets a spec pack ship its own loop construct.
    let registry = registry_with_a_declared_loop();
    assert_eq!(loop_codes("spin 0 {puts hi}", &registry), [DiagCode::W240]);
    assert_eq!(loop_codes("spin 1 {puts hi}", &registry), [DiagCode::W241]);
    assert_eq!(
        loop_codes("spin 1 {break}", &registry),
        [] as [tcl_core_types::DiagCode; 0]
    );
    // The control that makes the trait load-bearing: the same argument
    // shape without it runs its body once and is not a loop.
    assert_eq!(
        loop_codes("peek 1 {puts hi}", &registry),
        [] as [tcl_core_types::DiagCode; 0]
    );
}

/// The W240 / W241 codes for `src`, under the catalogue plus the one
/// inline `stub` declaration a document states.
fn declared_loop_codes(stub: &str, src: &str) -> Vec<DiagCode> {
    let declared = crate::analyser::utils::document_declared_surface(
        &format!("# tcl-lsp: stubs-begin\n# tcl-lsp: stub {stub}\n# tcl-lsp: stubs-end\n"),
        None,
        "tcl8.6",
    );
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, Some(&declared));
    let command = sole_command(src, config()).expect("one command");
    let name = command.texts.first().expect("a command word").clone();
    super::loop_termination_candidate(
        &name,
        command.args(),
        command.arg_tokens(),
        super::BoundsMetadataContext::Standalone(Some(registry)),
        Some(&surface),
        config(),
        &tcl_dialect::LexerGrammar::default(),
    )
    .and_then(|candidate| candidate.lexical_diagnostic())
    .into_iter()
    .map(|diagnostic| diagnostic.code)
    .collect()
}

#[test]
fn a_document_declaration_answers_the_loop_question_for_its_name() {
    // A stub's `-loop` is the `HAS_LOOP_BODY` a pack command states, and
    // its `cond:expr body:body` words are the shape.
    let spin = "spin {cond:expr body:body}";
    assert_eq!(
        declared_loop_codes(&format!("{spin} -loop"), "spin 1 {puts hi}"),
        [DiagCode::W241]
    );
    assert_eq!(
        declared_loop_codes(&format!("{spin} -loop"), "spin 0 {puts hi}"),
        [DiagCode::W240]
    );
    assert_eq!(
        declared_loop_codes(spin, "spin 1 {puts hi}"),
        [] as [tcl_core_types::DiagCode; 0]
    );
    // Nearest wins: a document that redeclares `while` answers for it, so
    // a declaration stating no loop body is not a loop, and one stating
    // it is.
    let redeclared = "while {cond:expr body:body}";
    assert_eq!(
        declared_loop_codes(redeclared, "while 1 {puts hi}"),
        [] as [tcl_core_types::DiagCode; 0]
    );
    assert_eq!(
        declared_loop_codes(&format!("{redeclared} -loop"), "while 1 {puts hi}"),
        [DiagCode::W241]
    );
}

#[test]
fn w241_for_loop_throw_tailcall_suppress() {
    // Same coverage on the `for` provably-infinite counter shape: a body
    // that throws / tailcalls is not an infinite loop.
    assert_eq!(
        codes("for {set i 0} {$i < 10} {incr i 0} {throw E x}\n"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        codes("for {set i 5} {$i > 0} {incr i} {tailcall done}\n"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn dynamic_condition_is_silent() {
    assert_eq!(
        codes("while {$x < 10} {incr x}\n"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn w241_for_step_provably_infinite() {
    // Step 0, wrong-direction step, increment-away, never-equal-skip.
    assert_eq!(
        codes("for {set i 0} {$i < 10} {incr i 0} {}\n"),
        vec!["W241"]
    );
    assert_eq!(
        codes("for {set i 0} {$i < 10} {incr i -1} {}\n"),
        vec!["W241"]
    );
    assert_eq!(codes("for {set i 5} {$i > 0} {incr i} {}\n"), vec!["W241"]);
    assert_eq!(
        codes("for {set i 0} {$i != 10} {incr i 3} {}\n"),
        vec!["W241"]
    );
    // A correct counting loop is silent.
    assert_eq!(
        codes("for {set i 0} {$i < 10} {incr i} {}\n"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn for_cond_parsers() {
    assert_eq!(
        super::parse_simple_for_cond("$i < 10", &tcl_dialect::LexerGrammar::default()),
        Some(("i".into(), "<".into(), 10)),
    );
    assert_eq!(
        super::parse_simple_for_cond("10 > $i", &tcl_dialect::LexerGrammar::default()),
        Some(("i".into(), "<".into(), 10)), // flipped
    );
    assert_eq!(
        super::parse_simple_for_cond("$i < 10 && 0", &tcl_dialect::LexerGrammar::default()),
        None
    );
    // Negative bound: the expression lexer splits `-5` into a unary
    // `-` plus `5`, which the signed-integer folding reassembles.
    assert_eq!(
        super::parse_simple_for_cond("$i > -5", &tcl_dialect::LexerGrammar::default()),
        Some(("i".into(), ">".into(), -5)),
    );
    assert_eq!(
        super::parse_simple_for_cond("-5 < $i", &tcl_dialect::LexerGrammar::default()),
        Some(("i".into(), ">".into(), -5)), // flipped
    );
    // String comparison operators are accepted; braces are stripped.
    assert_eq!(
        super::parse_simple_for_cond("{$i ne 3}", &tcl_dialect::LexerGrammar::default()),
        Some(("i".into(), "ne".into(), 3)),
    );
    // `${i}` braced variable form and `==`.
    assert_eq!(
        super::parse_simple_for_cond("${i} == 7", &tcl_dialect::LexerGrammar::default()),
        Some(("i".into(), "==".into(), 7)),
    );
    // Compound / logical / ternary / non-integer bounds reject.
    assert_eq!(
        super::parse_simple_for_cond("$i < 10 || $j > 0", &tcl_dialect::LexerGrammar::default()),
        None
    );
    assert_eq!(
        super::parse_simple_for_cond("!$done", &tcl_dialect::LexerGrammar::default()),
        None
    );
    assert_eq!(
        super::parse_simple_for_cond("$i ? 1 : 0", &tcl_dialect::LexerGrammar::default()),
        None
    );
    assert_eq!(
        super::parse_simple_for_cond("$i < 10.5", &tcl_dialect::LexerGrammar::default()),
        None
    );
    // The `-?\d+` bound and `\$\{?(\w+)\}?` var shapes mean a leading
    // `+`, an array index, or a namespace qualifier reject.
    assert_eq!(
        super::parse_simple_for_cond("$i < +5", &tcl_dialect::LexerGrammar::default()),
        None
    );
    assert_eq!(
        super::parse_simple_for_cond("$arr(i) < 5", &tcl_dialect::LexerGrammar::default()),
        None
    );
    assert_eq!(
        super::parse_simple_for_cond("$ns::v < 5", &tcl_dialect::LexerGrammar::default()),
        None
    );
}

/// A step is what the registry's integer cell update adds, run by its
/// route over literal words from 0: no command is read by its spelling.
#[test]
fn the_step_runs_its_route() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let step = |text: &str| {
        let cmd = sole_command(text, config()).expect("one command");
        super::command_increment(
            &cmd,
            "i",
            super::BoundsMetadataContext::Standalone(Some(registry)),
            config(),
        )
    };
    assert_eq!(step("incr i"), Some(1));
    assert_eq!(step("incr i -2"), Some(-2));
    assert_eq!(step("::incr i 3"), Some(3));
    assert_eq!(step("set i 3"), None);
    assert_eq!(step("incr j"), None);
    assert_eq!(step("incr i $n"), None);
    assert_eq!(step("append i 1"), None);
}

/// W241 seeds a loop's counter from its iteration plan's bound and step,
/// and from the integer the solver proves it starts at, so a counter
/// started from a variable is checked as one started from a literal is:
/// `set start 5` then `for {set i $start} {$i < 10} {incr i -1} {}`
/// never reaches 10, nor does the same counter in a `while` whose body's
/// one increment moves it away. A start the unit does not prove, a
/// counter that reaches its bound, a second write of the counter, an
/// increment that does not run on every pass, and a body that can leave
/// the loop — here by a `break` written bare, which the body scan reads
/// as the script it is (#2381) — each keep the loop silent; so do a
/// write a `[…]` word makes, a binder, and a call to a command the file
/// does not define, each of which the unit holding the loop knows may
/// write the counter.
/// A bound or step held in a variable is read as the integer the solver
/// proves it holds where the loop starts, when nothing in the loop may
/// write it (#2452).
#[test]
fn w241_reads_a_proven_bound_and_step() {
    for src in [
        "proc q {} { set i 0; set n 10; while {$i < $n} {incr i -1} }\n",
        "proc r {} { set i 0; set s -1; while {$i < 10} {incr i $s} }\n",
        "proc t {} { set i 0; set n 10; while {$n > $i} {incr i -1} }\n",
        "proc u {} {\n set n 10\n set s -2\n for {set i 0} {$i < $n} {incr i $s} {}\n}\n",
    ] {
        assert_verdicts(src, &["W241"]);
    }
    for src in [
        "proc q {} { set i 0; set n 10; while {$i < $n} {incr i} }\n",
        "proc q {n} { set i 0; while {$i < $n} {incr i -1} }\n",
        "proc q {} { set i 0; set n 10; while {$i < $n} {incr i -1; incr n -2} }\n",
        "proc r {s} { set i 0; while {$i < 10} {incr i $s} }\n",
        "proc r {} { set i 0; set s -1; while {$i < 10} {incr i $s; set s 1} }\n",
    ] {
        assert_verdicts(src, &[]);
    }
}

#[test]
fn w240_seeds_from_the_iteration_plan() {
    for src in [
        "set start 5\nfor {set i $start} {$i < 10} {incr i -1} {}\n",
        "set start 5\nset i $start\nwhile {$i < 10} {incr i -1}\n",
        "set i 5\nwhile {$i < 10} {puts $i; incr i -1}\n",
        "proc p {} {\n set n 3\n for {set i $n} {$i != 10} {incr i 2} {}\n}\n",
    ] {
        assert_verdicts(src, &["W241"]);
    }
    for src in [
        "proc p {start} {\n set i $start\n while {$i < 10} {incr i}\n}\n",
        "proc p {start} {\n for {set i $start} {$i < 10} {incr i -1} {}\n}\n",
        "set start 5\nset i $start\nwhile {$i < 10} {incr i}\n",
        "set i 5\nwhile {$i < 10} {incr i -1; if {$i < 0} {set i 20}}\n",
        "set i 5\nwhile {$i < 10} {incr i -1; incr i -1}\n",
        "set i 5\nwhile {$i < 10} {if {[gets stdin] eq {x}} {incr i -1}}\n",
        "set i 5\nwhile {$i < 10} {if {$i < 0} break; incr i -1}\n",
        "set i 5\nwhile {$i < 10} {incr i -1; set j [incr i 20]}\n",
        "set i 5\nwhile {$i < 10} {incr i -1; foreach i {100} {}}\n",
        "set i 5\nwhile {$i < 10} {incr i -1; foo i}\n",
    ] {
        assert_verdicts(src, &[]);
    }
    let mut analyser = Analyser::new();
    let messages: Vec<String> = analyser
        .analyse(
            "set start 5\nset i $start\nwhile {$i < 10} {incr i -1}\n",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "W241")
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(
        messages,
        [
            "while loop is provably infinite: counter $i starts at 5, moves by -1 per step, \
              and compares < 10 (never reached)"
        ]
    );
}

#[test]
fn body_scans_are_command_structural() {
    // A write counts only in command position, not inside a string.
    assert!(super::body_writes_var(
        "incr i",
        "i",
        super::BoundsMetadataContext::Standalone(None),
        config()
    ));
    assert!(super::body_writes_var(
        "if {$c} {set i 9}",
        "i",
        super::BoundsMetadataContext::Standalone(None),
        config()
    )); // nested body
    assert!(!super::body_writes_var(
        "puts \"set i now\"",
        "i",
        super::BoundsMetadataContext::Standalone(None),
        config()
    )); // inside a string
    assert!(!super::body_writes_var(
        "incr index",
        "i",
        super::BoundsMetadataContext::Standalone(None),
        config()
    )); // word boundary
    // `break` / `return` / `throw` / `tailcall` likewise count only as
    // commands. `return`/`throw` resolve via the registry's
    // TERMINATES_BLOCK trait; `break`/`tailcall` are recognised without it.
    let reg = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    assert!(super::body_may_exit(
        "break",
        super::BoundsMetadataContext::Standalone(Some(reg)),
        config()
    ));
    assert!(super::body_may_exit(
        "if {$c} {return}",
        super::BoundsMetadataContext::Standalone(Some(reg)),
        config()
    )); // nested
    assert!(super::body_may_exit(
        "throw MYERR boom",
        super::BoundsMetadataContext::Standalone(Some(reg)),
        config()
    )); // now covered
    assert!(super::body_may_exit(
        "tailcall foo",
        super::BoundsMetadataContext::Standalone(Some(reg)),
        config()
    ));
    assert!(!super::body_may_exit(
        "puts breakfast",
        super::BoundsMetadataContext::Standalone(Some(reg)),
        config()
    )); // not a command
    // `break`/`tailcall` are recognised even without a registry handle.
    assert!(super::body_may_exit(
        "break",
        super::BoundsMetadataContext::Standalone(None),
        config()
    ));
    assert!(super::body_may_exit(
        "tailcall foo",
        super::BoundsMetadataContext::Standalone(None),
        config()
    ));
    // A word the registry names a body is a script however it is
    // written: `if` runs a bare or a quoted body as `{…}`, so the exit
    // inside one counts (#2381). The same word as another command's
    // data is no script.
    assert!(super::body_may_exit(
        "if {$c} break",
        super::BoundsMetadataContext::Standalone(Some(reg)),
        config()
    ));
    assert!(super::body_may_exit(
        "if {$c} \"break\"",
        super::BoundsMetadataContext::Standalone(Some(reg)),
        config()
    ));
    assert!(super::body_may_exit(
        "if {$c} {puts x} else return",
        super::BoundsMetadataContext::Standalone(Some(reg)),
        config()
    ));
    assert!(super::body_may_exit(
        "if {$c} break",
        super::BoundsMetadataContext::Standalone(None),
        config()
    ));
    assert!(!super::body_may_exit(
        "puts break",
        super::BoundsMetadataContext::Standalone(Some(reg)),
        config()
    ));
}

fn idx_codes_for(src: &str, dialect: &str) -> Vec<String> {
    // Authored source number syntax and actual catalogue availability are
    // independent of Native body entry or an executed handler.
    let selected = tcl_dialect::DialectProfile::find(dialect).expect("authored bounds grammar");
    let mut profile = tcl_dialect::DialectProfile::plain_tcl().clone();
    profile.grammar = selected.grammar;
    let profile = profile.intern();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment(dialect).default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    assert!(input.has_logical_source_name_context());
    let mut a = Analyser::new().with_resolved_input(input);
    a.analyse(src, "tcl")
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W230" | "W232"))
        .map(|d| d.code.to_string())
        .collect()
}

fn idx_codes(src: &str) -> Vec<String> {
    idx_codes_for(src, "tcl8.6")
}

#[test]
fn w230_list_index_out_of_range() {
    assert_eq!(idx_codes("lindex {a b c} 5\n"), vec!["W230"]);
    assert_eq!(idx_codes("lindex {a b c} -1\n"), vec!["W230"]);
    assert_eq!(idx_codes("lindex {a b c} end-5\n"), vec!["W230"]);
    // In range and dynamic list → none.
    assert_eq!(
        idx_codes("lindex {a b c} 1\n"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        idx_codes("lindex {a b c} end\n"),
        [] as [std::string::String; 0]
    );
    assert_eq!(idx_codes("lindex $x 5\n"), [] as [std::string::String; 0]);
}

#[test]
fn w230_single_element_nested_list_length() {
    // `{{a b c}}` is a one-element list (the inner `{a b c}` is a
    // single braced element).  The segmenter already strips the outer
    // braces, so its length is 1 — index 2 is out of range.  A
    // double brace-strip would wrongly count three words and miss it.
    assert_eq!(idx_codes("lindex {{a b c}} 2\n"), vec!["W230"]);
    // Index 0 is the lone element → in range.
    assert_eq!(
        idx_codes("lindex {{a b c}} 0\n"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn w232_string_index_out_of_range() {
    assert_eq!(idx_codes("string index abc 10\n"), vec!["W232"]);
    assert_eq!(idx_codes("string index abc -1\n"), vec!["W232"]);
    assert_eq!(
        idx_codes("string index abc 1\n"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        idx_codes("string index abc end\n"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn w232_braced_literal_string_length() {
    // `{{hello}}` is a braced word whose runtime value is the literal
    // 7-char string `{hello}` (the segmenter strips only the outer
    // braces).  Index 6 is the last char — in range.  A double
    // brace-strip would count five chars and flag it spuriously.
    assert_eq!(
        idx_codes("string index {{hello}} 6\n"),
        [] as [std::string::String; 0]
    );
    // One past the end is still out of range.
    assert_eq!(idx_codes("string index {{hello}} 7\n"), vec!["W232"]);
}

#[test]
fn w230_lrange_lreplace_empty_slice() {
    assert_eq!(idx_codes("lrange {a b c} 5 7\n"), vec!["W230"]);
    assert_eq!(idx_codes("lrange {a b c} -3 -1\n"), vec!["W230"]);
    assert_eq!(idx_codes("lrange {a b c} 2 0\n"), vec!["W230"]); // clamped first>last
    assert_eq!(idx_codes("lreplace {a b c} 5 7 X\n"), vec!["W230"]);
    assert_eq!(
        idx_codes("lrange {a b c} 0 1\n"),
        [] as [std::string::String; 0]
    );
    // A first index past the end empties the slice whatever `last` is
    // (#2387); `lreplace` there appends (8.6+) or fails (8.4/8.5).
    assert_eq!(idx_codes("lrange {a b c} 5 end\n"), vec!["W230"]);
    assert_eq!(
        idx_codes("set l {a b c}\nset r [lrange $l 3 end]\n"),
        vec!["W230"]
    );
    assert_eq!(
        idx_codes("lreplace {a b c} 5 end X\n"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        idx_codes("lrange {a b c} 2 end\n"),
        [] as [std::string::String; 0]
    );
}

/// The length W231 checks against is the list's value after every cell
/// update on it the walk can evaluate: `lappend xs a b c` after `set xs
/// {}` leaves three elements (#2054).
#[test]
fn w231_length_follows_the_cell_updates() {
    assert_eq!(
        code_msgs("set xs {}\nlappend xs a b c\nlset xs 2 X\n", "W231"),
        [] as [std::string::String; 0]
    );
    let m = code_msgs("set xs {}\nlappend xs a b c\nlset xs 5 Y\n", "W231");
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(m[0].contains("(list has 3 elements)"), "{}", m[0]);
    assert_eq!(
        code_msgs(
            "set xs {}\nlappend xs a\nlappend xs {b c} d\nlset xs 3 X\n",
            "W231"
        ),
        [] as [std::string::String; 0]
    );
    let m = code_msgs(
        "set xs {}\nlappend xs a\nlappend xs {b c} d\nlset xs 4 X\n",
        "W231",
    );
    assert!(
        m.len() == 1 && m[0].contains("(list has 3 elements)"),
        "{m:?}"
    );
    // A write the walk cannot evaluate forgets the length rather than
    // reporting against a stale one.
    assert_eq!(
        code_msgs("set xs {a b}\nlappend xs $v\nlset xs 9 X\n", "W231"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        code_msgs("set xs {a b}\nappend xs $v\nlset xs 9 X\n", "W231"),
        [] as [std::string::String; 0]
    );
    // A string append changes the list too: `a b` then `append xs c`
    // is `a bc`, two elements.
    let m = code_msgs("set xs {a b}\nappend xs c\nlset xs 5 X\n", "W231");
    assert!(
        m.len() == 1 && m[0].contains("(list has 2 elements)"),
        "{m:?}"
    );
}

/// The walk reads a literal word as Tcl substitutes it: `append xs
/// "\tc"` appends a tab, so `a b<tab>c` has three elements (tclsh 8.6
/// to 9.1 extend it with `lset xs 3 X` and print `a b c X`). Read raw,
/// the list had two elements, and W231 reported a raise at index 3.
#[test]
fn w231_reads_an_appended_escape_as_tcl_substitutes_it() {
    assert_eq!(
        code_msgs("set xs {a b}\nappend xs \"\\tc\"\nlset xs 2 X\n", "W231"),
        [] as [std::string::String; 0]
    );
    let m = code_msgs("set xs {a b}\nappend xs \"\\tc\"\nlset xs 5 X\n", "W231");
    assert!(
        m.len() == 1 && m[0].contains("(list has 3 elements)"),
        "{m:?}"
    );
}

#[test]
fn w232_string_range_replace_empty_slice() {
    assert_eq!(idx_codes("string range abc 5 7\n"), vec!["W232"]);
    assert_eq!(idx_codes("string range abc -3 -1\n"), vec!["W232"]);
    assert_eq!(idx_codes("string replace abc 5 7 X\n"), vec!["W232"]);
    assert_eq!(
        idx_codes("string range abc 0 1\n"),
        [] as [std::string::String; 0]
    );
    assert_eq!(idx_codes("string range abc 5 end\n"), vec!["W232"]);
    assert_eq!(idx_codes("string replace abc 3 end X\n"), vec!["W232"]);
    assert_eq!(
        idx_codes("string range abc 2 end\n"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn pair_slice_empty_logic() {
    assert!(super::pair_slice_empty(5, 7, 3)); // both above
    assert!(super::pair_slice_empty(-3, -1, 3)); // both below
    assert!(super::pair_slice_empty(2, 0, 3)); // clamped first>last
    assert!(super::pair_slice_empty(0, 0, 0)); // empty container
    assert!(!super::pair_slice_empty(0, 1, 3)); // valid
}

#[test]
fn range_slice_empty_logic() {
    assert!(super::range_slice_empty(5, 2, 3)); // `5 end` past the end
    assert!(super::range_slice_empty(3, 2, 3));
    assert!(super::range_slice_empty(-3, -1, 3));
    assert!(super::range_slice_empty(2, 0, 3));
    assert!(super::range_slice_empty(0, 0, 0));
    assert!(!super::range_slice_empty(-5, 0, 3));
    assert!(!super::range_slice_empty(2, 9, 3));
}

#[test]
fn index_helpers() {
    let n = tcl_dialect::NumberSyntax::Tcl85;
    assert_eq!(super::resolve_index("end", 3, n), Some(2));
    assert_eq!(super::resolve_index("end-5", 3, n), Some(-3));
    assert_eq!(super::resolve_index("end+1", 3, n), Some(3));
    assert_eq!(super::resolve_index("2", 3, n), Some(2));
    assert_eq!(super::resolve_index("1+1", 3, n), Some(2));
    assert_eq!(super::resolve_index("0x2", 3, n), Some(2));
    assert_eq!(super::resolve_index("+5", 3, n), Some(5));
    assert_eq!(super::resolve_index("$x", 3, n), None);
    assert!(super::is_literal_index("end-2", n));
    assert!(!super::is_literal_index("end - 2", n));
}

#[test]
fn index_diagnostics_use_the_document_number_syntax() {
    let list = "lindex {a b c d e f g h i} 010\n";
    let string = "string index abcdefghi 010\n";
    // Tcl 8.x reads 010 as octal 8 (in range); Tcl 9.x reads it as
    // decimal 10 (out of range). Mutation: replacing the threaded
    // profile syntax with the ambient parser makes these pairs agree.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        assert_eq!(idx_codes_for(list, dialect).len(), 0, "{dialect}");
        assert_eq!(idx_codes_for(string, dialect).len(), 0, "{dialect}");
    }
    for dialect in ["tcl9.0", "tcl9.1"] {
        assert_eq!(idx_codes_for(list, dialect), vec!["W230"], "{dialect}");
        assert_eq!(idx_codes_for(string, dialect), vec!["W232"], "{dialect}");
    }
}

#[test]
fn condition_constant_classifies() {
    assert_eq!(super::condition_constant("0"), Some(false));
    assert_eq!(super::condition_constant("1"), Some(true));
    assert_eq!(super::condition_constant("{true}"), Some(true));
    assert_eq!(super::condition_constant("$x < 10"), None);
}

// W231 (lset out of range) & W242 (unprovable termination).

fn code_msgs_for(src: &str, dialect: &str, code: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, dialect)
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == code)
        .map(|d| d.message.clone())
        .collect()
}

fn code_msgs(src: &str, code: &str) -> Vec<String> {
    code_msgs_for(src, "tcl8.6", code)
}

#[test]
fn w231_lset_negative_index_always_errors() {
    let m = code_msgs("lset L -1 x\n", "W231");
    assert_eq!(m.len(), 1);
    assert!(m[0].contains("lset index '-1' is negative"), "{m:?}");
    // Negative literal fires even for nested (multi-index) forms.
    assert_eq!(code_msgs("lset L i j -1 v\n", "W231").len(), 1);
}

#[test]
fn w231_lset_index_past_append_slot() {
    // List length recovered from the recent literal `set`.
    let m = code_msgs("set L {a b c}\nlset L 5 x\n", "W231");
    assert_eq!(m.len(), 1);
    assert!(
        m[0].contains("resolves to 5 (list has 3 elements)"),
        "{m:?}"
    );
    // The append slot (index == length) and valid indices are fine.
    assert_eq!(
        code_msgs("set L {a b c}\nlset L 3 x\n", "W231"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        code_msgs("set L {a b c}\nlset L end+1 x\n", "W231"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        code_msgs("set L {a b c}\nlset L 2 x\n", "W231"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn w231_uses_the_document_number_syntax() {
    let src = "set L {a b c d e f g h i}\nlset L 010 x\n";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        assert_eq!(
            code_msgs_for(src, dialect, "W231").len(),
            0,
            "{dialect} reads 010 as in-range octal 8"
        );
    }
    for dialect in ["tcl9.0", "tcl9.1"] {
        assert_eq!(
            code_msgs_for(src, dialect, "W231").len(),
            1,
            "{dialect} reads 010 as out-of-range decimal 10"
        );
    }
}

/// `lset` takes an index equal to the list's length as an append from 8.6;
/// tclsh 8.4.20 and 8.5.19 raise `list index out of range` there, as does
/// `JimTcl` (measured, with the empty list's `0`).
#[test]
fn w231_reads_the_length_as_an_append_only_from_8_6() {
    for src in [
        "set x {a b c}\nlset x 3 Z\n",
        "set x {a b c}\nlset x {3} Z\n",
        "set x {}\nlset x 0 Z\n",
    ] {
        for dialect in ["tcl8.4", "tcl8.5"] {
            let m = code_msgs_for(src, dialect, "W231");
            assert_eq!(m.len(), 1, "{dialect}: {src:?}");
            assert!(m[0].contains("does not append at the list length"), "{m:?}");
        }
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            assert!(
                code_msgs_for(src, dialect, "W231").is_empty(),
                "{dialect} appends: {src:?}"
            );
        }
    }
}

/// `end+1` names the length of a three-element list. 8.5 reads it and
/// raises; 8.4 does not read it as an index at all (`bad index`), so
/// W231's out-of-range claim is not made there.
#[test]
fn w231_end_plus_one_follows_the_release() {
    let src = "set x {a b c}\nlset x end+1 Z\n";
    assert_eq!(code_msgs_for(src, "tcl8.5", "W231").len(), 1);
    assert_eq!(
        code_msgs_for(src, "tcl8.4", "W231"),
        [] as [std::string::String; 0]
    );
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(code_msgs_for(src, dialect, "W231").is_empty(), "{dialect}");
    }
    let past = "set x {a b c}\nlset x end+2 Z\n";
    assert_eq!(code_msgs_for(past, "tcl8.5", "W231").len(), 1);
    assert_eq!(code_msgs_for(past, "tcl8.6", "W231").len(), 1);
}

/// A document that names no release is not told the boundary raises: the
/// releases disagree there. An index past the length still raises under
/// all of them.
#[test]
fn w231_abstains_at_the_length_when_no_release_is_selected() {
    let at = "set x {a b c}\nlset x 3 Z\n";
    let past = "set x {a b c}\nlset x 4 Z\n";
    assert_eq!(
        code_msgs_for(at, "tcl", "W231"),
        [] as [std::string::String; 0]
    );
    assert_eq!(code_msgs_for(past, "tcl", "W231").len(), 1);
}

/// Unmeasured writes invalidate an earlier list length. Tcl 9.0.4 accepts
/// index 2 after appending three elements to an initially empty list.
#[test]
fn w231_abstains_after_a_write_it_cannot_measure() {
    for src in [
        "set xs {}\nlappend xs a b c\nlset xs 2 X\n",
        // Same gap, reached by a non-literal `set` rather than `lappend`.
        "set xs {}\nset y {a b c}\nset xs $y\nlset xs 2 X\n",
        // And by a command substitution.
        "set xs {}\nset xs [list a b c]\nlset xs 2 X\n",
        // `append` is the same class of unmeasurable write.
        "set xs {}\nappend xs \"a b c\"\nlset xs 2 X\n",
    ] {
        assert_eq!(
            code_msgs(src, "W231").len(),
            0,
            "an unmeasurable write must invalidate the earlier length: {src:?}",
        );
    }
}

/// The abstention must not silence a genuine out-of-range index: with no
/// intervening write the literal length still stands.
#[test]
fn w231_still_reports_a_real_overrun() {
    assert!(
        !code_msgs("set xs {a b}\nlset xs 5 X\n", "W231").is_empty(),
        "an untouched literal list keeps its length",
    );
}

#[test]
fn w231_silent_without_recoverable_length() {
    // No prior literal `set` → only negative literals would fire.
    assert_eq!(
        code_msgs("lset L 5 x\n", "W231"),
        [] as [std::string::String; 0]
    );
    // A `set` in a deeper scope must not be trusted.
    assert_eq!(
        code_msgs("proc p {} { set L {a b c} }\nlset L 5 x\n", "W231").len(),
        0,
        "deeper-scope set should not leak its length"
    );
}

#[test]
fn w242_counter_not_modified() {
    let m = code_msgs("while {$x < 10} {puts hi}\n", "W242");
    assert_eq!(m.len(), 1);
    assert!(
        m[0].contains("variable 'x' in the condition is never modified"),
        "{m:?}"
    );
    // `for` with an empty step and a body that ignores the counter, over a
    // bound nothing proves: the condition does not decide.
    assert_eq!(
        code_msgs("for {set i 0} {$i < $n} {} {puts hi}\n", "W242").len(),
        1
    );
}

#[test]
fn w242_silent_when_counter_modified() {
    // Body writes the counter via incr / set / lappend → no W242.
    assert_eq!(
        code_msgs("while {$x < 10} {incr x}\n", "W242"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        code_msgs("while {$x < 10} {set x 5}\n", "W242"),
        [] as [std::string::String; 0]
    );
    // A normal `for` whose step advances the counter is silent.
    assert_eq!(
        code_msgs("for {set i 0} {$i < 10} {incr i} {puts hi}\n", "W242"),
        [] as [std::string::String; 0]
    );
}

// Corpus shape (`tcltest.tcl`'s option-usage word-wrapper): a `[cmd $var]`
// command substitution in the condition hides the loop's real progress
// variable from the shallow scalar scan, which would then blame whichever
// *other* bare variable it finds instead.

#[test]
fn w242_silent_when_the_progress_variable_is_inside_a_command_substitution() {
    // FP: `u` shrinks every iteration via `string range`/`string trim` in
    // the body — real, provable progress — but the condition's only
    // *bare* variable is `rest` (a fixed threshold the loop never
    // touches), which a first-`Variable`-token scan would pick instead.
    // Exact corpus shape.
    assert_eq!(
        code_msgs(
            "while {[string length $u] > $rest} {\
                     set u [string trim [string range $u 1 end]]\
                 }\n",
            "W242"
        )
        .len(),
        0
    );
}

#[test]
fn w242_silent_for_a_command_substitution_condition_even_when_nothing_is_modified() {
    // FP guard, the genuinely-unprovable case: abstaining is still
    // correct here — the analyser cannot see into `[llength $l]`'s
    // argument to know whether `l` (or anything else) changes, so it
    // must not guess at a bare variable elsewhere in the condition
    // (there is none here) or fabricate a counter name.
    assert_eq!(
        code_msgs("while {[llength $l] > 0} {puts hi}\n", "W242"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn w242_severity_is_hint() {
    let mut a = Analyser::new();
    let r = a.analyse("while {$x < 10} {puts hi}\n", "tcl8.6");
    let w242 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W242)
        .unwrap();
    assert_eq!(w242.severity, super::Severity::Hint);
}

fn w231(src: &str) -> usize {
    let mut a = Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W231)
        .count()
}

#[test]
fn w231_list_length_recovered_inside_proc_body() {
    // The `set l {a b c}` shares the proc body's flat scope with the
    // `lset`, so its length (3) is recovered and `99` proves OOR —
    // top-level-only segmentation would miss this.
    assert_eq!(
        w231("proc f {} {\n    set l {a b c}\n    lset l 99 X\n}\n"),
        1
    );
    // `end` on an empty list is out of range.
    assert_eq!(w231("proc f {} {\n    set l {}\n    lset l end X\n}\n"), 1);
    // Still fires at top level.
    assert_eq!(w231("set l {a b c}\nlset l 99 X\n"), 1);
    // The append slot (index == length) is accepted.
    assert_eq!(
        w231("proc f {} {\n    set l {a b c}\n    lset l 3 X\n}\n"),
        0
    );
}

fn has_code(src: &str, code: &str) -> bool {
    let mut a = Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == code)
}

#[test]
fn bounds_checks_recurse_into_command_substitutions() {
    // Original authored nested-source geometry is queried once. This
    // positive Logical fixture grants no Native child body activation.
    assert_eq!(idx_codes("set x [lindex {a b c} 9]\n"), vec!["W230"]);
    assert_eq!(
        idx_codes("proc f {} { return [string index abc 99] }\n"),
        vec!["W232"]
    );
    assert_eq!(
        idx_codes("proc f {} { return [join [string index abc 99]] }\n"),
        vec!["W232"]
    );
}

#[test]
fn security_checks_recurse_into_command_substitutions() {
    // The full per-command dispatch — not just bounds — runs on a
    // substitution command, so `open "|$cmd"` nested in `[…]` still
    // fires the W103 pipeline-injection check.
    assert!(has_code(
        "proc f {cmd} { set fh [open \"|$cmd\" r] }\n",
        "W103"
    ));
}

#[test]
fn w231_length_not_recovered_across_scope_boundary() {
    // A `set` in a *different* proc body must not leak its length to a
    // top-level `lset` — it belongs to a script the `lset` never enters.
    assert_eq!(w231("proc other {} { set l {a b c} }\nlset l 99 X\n"), 0);
    // Descending into a body starts a fresh scope, so an outer `set` is
    // not trusted for an `lset` written inside one.
    assert_eq!(
        w231("set l {a b c}\nnamespace eval ns {\n    lset l 99 X\n}\n"),
        0
    );
    // Nor across an `oo::define` member body.
    assert_eq!(
        w231("set l {a b c}\noo::define C {\n    method m {} { lset l 99 X }\n}\n"),
        0
    );
}

#[test]
fn w231_ignores_a_set_that_is_not_a_command() {
    // The `set` inside the quoted word is text, not an assignment, so it
    // supplies no length and cannot shorten the real one.  A line-anchored
    // byte scan matches it and reports index 4 as out of range for a
    // two-element list.
    let src = "set l {a b c d e f}\nputs \"\nset l {a b}\n\"\nlset l 4 X\n";
    assert_eq!(w231(src), 0);
}

#[test]
fn w231_reports_across_a_sibling_definition() {
    // `proc` here is a *sibling* command, not a scope the `lset` is
    // inside, so the top-level `set`'s length still reaches it.  A
    // `\\b(?:proc|namespace\\s+eval|apply|try)\\b` marker scan sees the
    // word `proc` between the two and goes silent.
    assert_eq!(
        w231("set l {a b c}\nproc helper {} { return 1 }\nlset l 9 X\n"),
        1
    );
}

#[test]
fn w231_recovers_a_length_the_line_anchored_scan_missed() {
    // Not at a line start, and a literal carrying a nested sublist: both
    // are ordinary words to the segmenter, though neither matches
    // `(?:^|\\n)\\s*set\\s+(\\w+)\\s+(\\{[^{}]*\\})`.
    assert_eq!(w231("puts hi; set l {a {b c} d}\nlset l 9 X\n"), 1);
}
#[test]
fn shallow_bounds_metadata_retains_availability_without_default_catalogue_fallback() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current = tcl_registry::model::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
    );
    let old = tcl_registry::model::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.4").unwrap(),
    );
    let old = old.with_command_store(std::sync::Arc::clone(current.commands()));
    assert!(super::is_loop_exit_command(
        "tailcall",
        super::BoundsMetadataContext::Retained(&current)
    ));
    assert!(!super::is_loop_exit_command(
        "tailcall",
        super::BoundsMetadataContext::Retained(&old)
    ));
    assert!(super::is_loop_exit_command(
        "break",
        super::BoundsMetadataContext::Retained(&old)
    ));
    let no_tcl = tcl_registry::model::context_for_profile(
        tcl_dialect::DialectProfile::find("f5-bigip").unwrap(),
    );
    let no_tcl = no_tcl.with_command_store(std::sync::Arc::clone(current.commands()));
    assert!(
        super::loop_shape(
            "while",
            &[],
            super::BoundsMetadataContext::Retained(&no_tcl),
            None
        )
        .is_none()
    );
    assert!(
        super::loop_shape(
            "while",
            &[],
            super::BoundsMetadataContext::Standalone(None),
            None
        )
        .is_some()
    );
}

/// The loop-termination verdicts (W240, W241, W242) `src` draws on the
/// whole-file walk and on the per-item walk, each in source order.
fn verdicts_on_both_paths(src: &str) -> (Vec<String>, Vec<String>) {
    let verdicts = |per_item: bool| -> Vec<String> {
        let mut a = Analyser::new();
        let result = if per_item {
            a.analyse_per_item(src, "tcl8.6")
        } else {
            a.analyse(src, "tcl8.6")
        };
        let mut found: Vec<(u32, String)> = result
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W240" | "W241" | "W242"))
            .map(|d| (d.span.start(), d.code.to_string()))
            .collect();
        found.sort();
        found.into_iter().map(|(_, code)| code).collect()
    };
    (verdicts(false), verdicts(true))
}

/// `src` draws exactly `expected` on both paths.
fn assert_verdicts(src: &str, expected: &[&str]) {
    let (whole, per_item) = verdicts_on_both_paths(src);
    assert_eq!(whole, expected, "whole file: {src}");
    assert_eq!(per_item, expected, "per item: {src}");
}

/// `src` draws neither W240 nor W241 on either path: the header is left
/// undecided, and the text may still draw W242's hint that nothing in the
/// loop modifies its counter.
fn assert_undecided(src: &str) {
    let (whole, per_item) = verdicts_on_both_paths(src);
    for (path, found) in [("whole file", &whole), ("per item", &per_item)] {
        assert!(
            found.iter().all(|code| code == "W242"),
            "{path}: {src}\n{found:?}"
        );
    }
    assert_eq!(per_item, whole, "{src}");
}

/// A `switch` the flow graph keeps as one statement — `-glob`, `-regexp`,
/// `-nocase`, a fall-through arm, `case` — defines every name its arms
/// write, so a loop an arm can end is no infinite loop. tclsh 8.4 to 9.1
/// with input `q`: `set go 1; while {$go} { switch -glob -- [gets stdin]
/// { q* { set go 0 } } }; puts done` prints `done`.
#[test]
fn a_switch_arm_that_ends_the_loop_keeps_w241_silent() {
    for src in [
        "set go 1; while {$go} { switch -glob -- [gets stdin] { q* { set go 0 } } }; puts done\n",
        "set go 1\nwhile {$go} {\n switch -nocase -- [gets stdin] {\n  q { set go 0 }\n }\n}\n",
        "set go 1\nwhile {$go} {\n switch -regexp -- [gets stdin] {\n  {^q} { set go 0 }\n }\n}\n",
        "set go 1\nwhile {$go} {\n switch -glob -- [gets stdin] {\n  x - q* { set go 0 }\n }\n}\n",
        "set go 1\nwhile {$go} {\n switch -glob -- [gets stdin] {\n  a* {}\n  default { set go 0 }\n }\n}\n",
        "set go 1\nwhile {$go} {\n case [gets stdin] in {\n  q* { set go 0 }\n }\n}\n",
        "set go 1\nfor {} {$go} {} {\n switch -glob -- [gets stdin] { q* { set go 0 } }\n}\n",
        "proc p {} {\n set go 1\n while {$go} {\n  switch -glob -- [gets stdin] { q* { set go 0 } }\n }\n}\n",
        // A write buried in a command the arm runs, and each kind of write.
        "set go 1\nwhile {$go} {\n switch -glob -- [gets stdin] { q* { if {[llength {a}]} { set go 0 } } }\n}\n",
        "set n 3\nwhile {$n} {\n switch -glob -- [gets stdin] { q* { incr n -1 } }\n}\n",
        "set l {}\nwhile {[llength $l] < 1} {\n switch -glob -- [gets stdin] { q* { lappend l x } }\n}\n",
        "set go 1\nwhile {[info exists go]} {\n switch -glob -- [gets stdin] { q* { unset go } }\n}\n",
        // A `global` binding an arm makes, then writes through.
        "proc p {} {\n set go 1\n while {$go} {\n  switch -glob -- [gets stdin] { q* { global go; set go 0 } }\n }\n}\n",
    ] {
        assert_verdicts(src, &[]);
    }
}

/// What a command an arm runs does to the frame ends the loop as a write
/// the arm makes itself would: a callee that writes the caller's name
/// through `upvar`, and a command that may write any name (`namespace
/// eval`, `dict with`). A callee that writes only a global leaves a local
/// loop alone.
#[test]
fn a_command_an_arm_runs_that_ends_the_loop_keeps_w241_silent() {
    for src in [
        "proc zero {v} {upvar 1 $v x; set x 0}\nproc p {} {\n set go 1\n while {$go} {\n  switch -glob -- [gets stdin] { q* { zero go } }\n }\n}\n",
        "proc zero {v} {upvar 1 $v x; set x 0}\nset go 1\nwhile {$go} {\n switch -glob -- [gets stdin] { q* { zero go } }\n}\n",
        "set go 1\nwhile {$go} {\n switch -glob -- [gets stdin] { q* { namespace eval :: {set go 0} } }\n}\n",
        "proc p {} {\n set go 1\n while {$go} {\n  switch -glob -- [gets stdin] { q* { dict with d { set go 0 } } }\n }\n}\n",
    ] {
        assert_undecided(src);
    }
    assert_verdicts(
        "proc hit {} {global hits; incr hits}\nproc p {} {\n set go 1\n while {$go} {\n  switch -glob -- [gets stdin] { q* { hit } }\n }\n}\n",
        &["W241"],
    );
}

/// The same statement leaves a loop no arm writes for alone: only a name
/// an arm writes stops being decided.
#[test]
fn a_switch_arm_that_writes_another_name_leaves_the_verdict() {
    assert_verdicts(
        "set go 1\nwhile {$go} {\n switch -glob -- [gets stdin] { q* { set other 0 } }\n}\n",
        &["W241"],
    );
    assert_verdicts(
        "proc p {} {\n set go 1\n while {$go} {\n  switch -glob -- [gets stdin] { q* { set other 0 } }\n }\n}\n",
        &["W241"],
    );
}

/// A loop nothing else sets is dead until an arm sets it: no W240 for
/// `set go 0; switch -glob -- [gets stdin] { q* { set go 1 } }; while
/// {$go} { … }`, which tclsh runs once for input `q`.
#[test]
fn a_switch_arm_that_starts_the_loop_keeps_w240_silent() {
    for src in [
        "set go 0\nswitch -glob -- [gets stdin] { q* { set go 1 } }\nwhile {$go} { puts x; set go 0 }\n",
        "set go 0\nswitch -glob -- [gets stdin] { q* { set go 1 } }\nfor {} {$go} {} { puts x; set go 0 }\n",
        "proc p {} {\n set go 0\n switch -glob -- [gets stdin] { q* { set go 1 } }\n while {$go} { puts x; set go 0 }\n}\n",
    ] {
        assert_verdicts(src, &[]);
    }
    assert_verdicts(
        "set n 0\nswitch -glob -- [gets stdin] { q* { set other 1 } }\nwhile {$n} {puts x}\n",
        &["W240"],
    );
}

/// A callback script the module stores writes the loop's variable
/// without any statement of the loop saying so: `after`, `after idle`, a
/// variable trace, `fileevent`, `chan event`, `bind`, and a callback that
/// names a procedure writing the name through `::`.
#[test]
fn a_callback_that_writes_the_loop_variable_keeps_w241_silent() {
    for src in [
        "set done 0\nafter 100 { set done 1 }\nwhile {!$done} { update }\n",
        "set go 1\nafter idle {set ::go 0}\nwhile {$go} {update}\n",
        "set go 1\ntrace add variable x write { set ::go 0 ;# }\nwhile {$go} { set x 1 }\nputs done\n",
        "set go 1\nfileevent stdin readable { set go 0 }\nwhile {$go} { update }\n",
        "set go 1\nchan event stdin readable { set go 0 }\nwhile {$go} { update }\n",
        "set go 1\nbind . <Key> { set go 0 }\nwhile {$go} { update }\n",
        "proc tick {} { set ::done 1 }\nset done 0\nafter 100 tick\nwhile {!$done} { update }\n",
        // A quoted script, and a command prefix built with `list`.
        "set done 0\nafter 100 \"set done 1\"\nwhile {!$done} { update }\n",
        "set done 0\nafter 100 [list set done 1]\nwhile {!$done} { update }\n",
        "proc tick {n} { set ::done $n }\nset done 0\nafter 100 [list tick 1]\nwhile {!$done} { update }\n",
    ] {
        assert_undecided(src);
    }
}

/// A plain top-level name is the global name, so a call to a command the
/// module cannot see may end the loop as it may one over `::go`, and so may
/// such a call over a procedure's local, which an autoloaded or
/// unknown-handled callee reaches with `upvar 1` (tclsh 8.4 to 9.1 end
/// `while {$go} {stopper}` where `auto_index(stopper)` defines `stopper`
/// as `upvar 1 go go; set go 0`). A procedure the module defines writes no
/// global.
#[test]
fn a_call_the_module_cannot_see_keeps_a_loop_silent() {
    assert_undecided("set go 1\nwhile {$go} { foo }\n");
    assert_undecided("set ::go 1\nwhile {$::go} { foo }\n");
    assert_undecided("proc p {} {\n set go 1\n while {$go} { foo }\n}\n");
    // A sourced file runs in the frame of the call, a procedure's too.
    assert_undecided("set go 1\nwhile {$go} { source other.tcl }\n");
    assert_undecided("proc p {} {\n set go 1\n while {$go} { source other.tcl }\n}\n");
    assert_verdicts(
        "proc foo {} { puts hi }\nset go 1\nwhile {$go} { foo }\n",
        &["W241"],
    );
}

/// A command the module cannot see inside the body of a `catch` the flow
/// graph keeps as one statement runs as one outside it does, and a
/// computed head is a command the module cannot see: either may end the
/// loop as a write to `::go` does, or to a procedure's local through
/// `upvar 1`. A procedure the module defines is seen, and a body that
/// writes another name leaves the verdict.
#[test]
fn a_catch_body_or_a_computed_head_the_module_cannot_see_keeps_a_loop_silent() {
    for src in [
        "set go 1\ncatch {foo}\nwhile {$go} {puts x}\n",
        "set go 1\ncatch {foo} msg\nwhile {$go} {puts x}\n",
        "set go 1\ncatch {if {1} {foo}}\nwhile {$go} {puts x}\n",
        "set go 1\ncatch {puts [foo]}\nwhile {$go} {puts x}\n",
        "set go 1\ncatch {source other.tcl}\nwhile {$go} {puts x}\n",
        "set go 1\nif {[catch {foo}]} {puts bad}\nwhile {$go} {puts x}\n",
        "set go 1\nset rc [catch {foo} msg]\nwhile {$go} {puts x}\n",
        "set go 1\nwhile {$go} { if {[catch {foo}]} {puts bad} }\n",
        "set go 1\ncatch { namespace eval :: {set go 0} }\nwhile {$go} {puts x}\n",
        "set go 1\ncatch { if {[gets stdin] eq {q}} { set go 0 } }\nwhile {$go} {puts x}\n",
        "set go 1\n$cmd\nwhile {$go} {puts x}\n",
        "set go 1\nputs [$cmd]\nwhile {$go} {puts x}\n",
        "set go 1\nwhile {$go} { $cmd }\n",
        "proc p {} {\n set go 1\n catch { if {1} { source other.tcl } }\n while {$go} {puts x}\n}\n",
        "proc p {} {\n set go 1\n catch { if {1} { foo } }\n while {$go} {puts x}\n}\n",
        "proc p {} {\n set go 1\n $cmd\n while {$go} {puts x}\n}\n",
        "proc p {} {\n set go 1\n if {[catch {foo}]} {puts bad}\n while {$go} {puts x}\n}\n",
    ] {
        assert_undecided(src);
    }
    for src in [
        "proc foo {} { puts hi }\nset go 1\ncatch { if {1} { foo } }\nwhile {$go} {puts x}\n",
        "set go 1\ncatch { if {[gets stdin] eq {q}} { set other 0 } }\nwhile {$go} {puts x}\n",
    ] {
        assert_verdicts(src, &["W241"]);
    }
}

/// A callback the scan cannot read may write the loop's variable, so it
/// ends no verdict it could have drawn: a script spelled as several words
/// is read as the one they make, and a word the run time computes, a
/// substitution of any other command, an expansion and a command the module
/// cannot see are not. A callback that is only cancelled, or stores a
/// script that writes another name, leaves the loop alone.
#[test]
fn a_callback_the_scan_cannot_read_keeps_w241_silent() {
    for src in [
        "set done 0\nafter 100 set done 1\nwhile {!$done} { update }\n",
        "set done 0\nafter idle set done 1\nwhile {!$done} { update }\n",
        "set done 0\nafter 100 incr done\nwhile {!$done} { update }\n",
        "set done 0\nafter 100 $script\nwhile {!$done} { update }\n",
        "set done 0\nafter 100 finish\nwhile {!$done} { update }\n",
        "set done 0\ninterp alias {} fin {} set done 1\nafter 100 fin\nwhile {!$done} { update }\n",
        "set done 0\nafter 100 {*}[list set done 1]\nwhile {!$done} { update }\n",
        "set done 0\nafter 100 [build]\nwhile {!$done} { update }\n",
        "set done 0\nafter 100 {finish now}\nwhile {!$done} { update }\n",
        "proc p {} {\n set done 0\n after 100 $script\n while {!$done} { update }\n}\n",
    ] {
        assert_undecided(src);
    }
    for src in [
        "set done 0\nafter cancel {set done 1}\nwhile {!$done} { update }\n",
        "set done 0\nafter cancel $id\nwhile {!$done} { update }\n",
        "set done 0\nafter 100 {puts hi}\nwhile {!$done} { update }\n",
        "set done 0\nafter 100 set other 1\nwhile {!$done} { update }\n",
        "proc tick {} { puts hi }\nset done 0\nafter 100 tick\nwhile {!$done} { update }\n",
    ] {
        assert_verdicts(src, &["W241"]);
    }
}
