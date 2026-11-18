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

//! Word effects, calls to unseen code and opaque `switch` and `catch` in the built SSA: the versions they read, define and leave live.

use super::*;

/// The SSA of `source`'s function `name`, built as a unit builds it.
fn ssa_of_function(source: &str, name: &str) -> SsaFunction {
    // A registry naming no release reads an authored `proc` as a
    // declaration only, so its body has no statements; these are Tcl 8.6.
    let reg = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let cu = crate::compilation_unit::CompilationUnit::build_for(source, &reg, false);
    cu.function(name).expect("function analysed").ssa.clone()
}

/// The SSA statements of `ssa` whose statement satisfies `wanted`.
fn statements_where(ssa: &SsaFunction, wanted: impl Fn(&Statement) -> bool) -> Vec<&SsaStatement> {
    ssa.blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter(|statement| wanted(&statement.statement))
        .collect()
}

/// The definition point the CFG builder puts ahead of a statement for
/// what the statement's `[…]` substitutions write.
fn is_effect_call(stmt: &Statement) -> bool {
    stmt.word_effects_host().is_some()
}

/// A statement's word effects are paired with the statement whose words
/// they are by construction: the statement right after the definition
/// point, whatever its span, and no other statement has a host. A control
/// statement's words have no statement of their own, so their definition
/// point has none.
#[test]
fn word_effects_stand_ahead_of_the_statement_whose_words_they_are() {
    let effect_block = |source: &str, name: &str| {
        let ssa = ssa_of_function(source, name);
        let block = ssa
            .blocks
            .values()
            .find(|block| {
                block
                    .statements
                    .iter()
                    .any(|info| is_effect_call(&info.statement))
            })
            .expect("a block with an effect call");
        let call = block
            .statements
            .iter()
            .position(|info| is_effect_call(&info.statement))
            .expect("the effect call");
        (block.statements.clone(), call)
    };
    for (source, name) in [
        (
            "proc p {} {set x 1; set r [expr {$x + [incr x]}]; return $x}\n",
            "::p",
        ),
        ("set x 1\nset r [expr {[incr x] + [foo]}]\n", "::top"),
        ("set x 1\nexpr {$x + [incr x]}\n", "::top"),
    ] {
        let (statements, call) = effect_block(source, name);
        let host = word_effects_host(&statements, call).expect("the call has a host");
        assert_eq!(host, call + 1, "{source}");
        assert!(
            !is_synthetic_statement(&statements[host].statement),
            "{source}"
        );
        assert_eq!(
            statements[host].statement.span(),
            statements[call].statement.span(),
            "{source}"
        );
        for other in (0..statements.len()).filter(|&index| index != call) {
            assert_eq!(word_effects_host(&statements, other), None, "{source}");
        }
    }
    let (mut statements, call) = effect_block(
        "proc p {} {set x 1; set r [expr {$x + [incr x]}]; return $x}\n",
        "::p",
    );
    let host = word_effects_host(&statements, call).expect("the call has a host");
    let Statement::AssignExpr { span, .. } = &mut statements[host].statement else {
        panic!("an assignment of an expression hosts the call");
    };
    *span = tcl_lexer::Span::new(span.start() + 1, span.end());
    assert_eq!(word_effects_host(&statements, call), Some(host));
    let (statements, call) = effect_block(
        "proc p {} {set x 1; return [expr {$x + [incr x]}]}\n",
        "::p",
    );
    assert_eq!(word_effects_host(&statements, call), None);
}

/// The words of a statement whose `[…]` substitutions write a place read it
/// before, between and after the write. The effect call ahead of the
/// statement reads the version before the write, by name, so the store
/// feeding the word that runs first stays live, and the statement's own use
/// names the version after the write by name as well: no word of it is an
/// operand something may forward.
#[test]
fn an_effect_call_reads_the_version_before_the_write_its_statement_reads() {
    for (body, reads_it) in [
        ("set r [expr {$x + [set x 10] + $x}]", true),
        ("expr {$x + [set x 10] + $x}", true),
        ("set r \"$x [set x 10] $x\"", true),
        ("incr r [expr {$x + [set x 10]}]", false),
        ("set r [expr {[string length $x] + [set x 10]}]", false),
        ("set r [expr {[expr {$x + 1}] + [set x 10]}]", false),
    ] {
        let source = format!("proc p {{}} {{\n set x 1\n set r 5\n {body}\n puts $x\n}}\n");
        let ssa = ssa_of_function(&source, "::p");
        let x = ssa.var_symbol("x").expect("x");
        let call = statements_where(&ssa, is_effect_call)[0];
        assert_eq!(call.defs[&x], 2, "{body}");
        assert_eq!(call.uses.get(&x), Some(&1), "{body}");
        assert!(call.name_only_uses.contains(&x), "{body}");
        if reads_it {
            let host = statements_where(&ssa, |stmt| {
                matches!(
                    stmt,
                    Statement::AssignExpr { .. }
                        | Statement::ExprEval { .. }
                        | Statement::AssignValue { .. }
                ) && stmt.span() == call.statement.span()
            })[0];
            assert_eq!(host.uses.get(&x), Some(&2), "{body}");
            assert!(host.name_only_uses.contains(&x), "{body}");
        }
    }
}

/// A place no word of the statement reads is not read by the call that
/// carries its write, so the store the write overwrites stays dead: the
/// call's read is for the words, not for every place it defines.
#[test]
fn an_effect_call_leaves_a_place_its_statement_never_reads_alone() {
    for body in [
        "set r [expr {[set x 10] + 1}]",
        "set r \"[set x 10]\"",
        "set r [expr {[gets $fd x] + 1}]",
    ] {
        let source = format!("proc p {{fd}} {{\n set x 1\n {body}\n puts $x\n}}\n");
        let ssa = ssa_of_function(&source, "::p");
        let x = ssa.var_symbol("x").expect("x");
        let call = statements_where(&ssa, is_effect_call)[0];
        assert_eq!(call.defs[&x], 2, "{body}");
        assert_eq!(call.uses.get(&x), None, "{body}");
    }
}

/// A call that embeds the write holds it among its own definitions, and its
/// words read the place before, between and after it. It reads the version
/// before the write by name, and none of its words is an operand to forward;
/// a place the call names itself, in the position its command reads or
/// writes, stays what it was.
#[test]
fn a_call_reads_a_place_its_embedded_command_writes_by_name() {
    for (command, body) in [
        ("puts", "puts [expr {$x + [set x 10] + $x}]"),
        ("puts", "puts \"$x [incr x] $x\""),
        ("foo", "foo $x [incr x] $x"),
        ("lappend", "lappend l [incr x] $x"),
    ] {
        let source = format!("proc p {{}} {{\n set x 1\n set l {{}}\n {body}\n puts $x\n}}\n");
        let ssa = ssa_of_function(&source, "::p");
        let x = ssa.var_symbol("x").expect("x");
        let host = statements_where(
            &ssa,
            |stmt| matches!(stmt, Statement::Call { command: name, .. } if name == command),
        )[0];
        assert_eq!(host.uses.get(&x), Some(&1), "{body}");
        assert_eq!(host.defs[&x], 2, "{body}");
        assert!(host.name_only_uses.contains(&x), "{body}");
    }
    let ssa = ssa_of_function(
        "proc p {} {\n set l {}\n lappend l $l\n puts $l\n}\n",
        "::p",
    );
    let l = ssa.var_symbol("l").expect("l");
    let host = statements_where(
        &ssa,
        |stmt| matches!(stmt, Statement::Call { command, .. } if command == "lappend"),
    )[0];
    assert!(
        !host.name_only_uses.contains(&l),
        "the command's own target is read before it writes it: its word is an operand"
    );
}

/// A `switch` the flow graph keeps as one statement defines, as may-defs,
/// every name its arms write or bind — the default arm's, a fall-through
/// arm's, a nested command's, an `incr`'s, a `lappend`'s and a `global`
/// binding's — and uses the prior version of each, quoted, so liveness
/// keeps the store that feeds it and no read-before-set is invented.
#[test]
fn an_opaque_switch_defines_every_name_its_arms_write() {
    let ssa = ssa_of_function(
        "proc p {s} {\n set x 1\n set n 0\n\
             switch -glob -- $s {\n  a* - b* { set x 2 }\n  c* { if {[gets stdin line]} { incr n } }\n  default { lappend l 1; global g }\n }\n puts $x\n}\n",
        "::p",
    );
    let statement = ssa
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .find(|statement| matches!(statement.statement, Statement::Switch { .. }))
        .expect("the switch stays one statement");
    let mut written: Vec<&str> = statement
        .may_defs
        .iter()
        .map(|&symbol| ssa.var_name(symbol))
        .collect();
    written.sort_unstable();
    assert_eq!(written, ["g", "l", "line", "n", "x"]);
    for &symbol in &statement.may_defs {
        assert!(statement.defs.contains_key(&symbol));
        assert!(statement.quoted_uses.contains(&symbol));
    }
    // The read after the switch reads the version the switch defines.
    let x = ssa.var_symbol("x").expect("x");
    assert_eq!(statement.defs[&x], 2);
    assert_eq!(statement.uses[&x], 1);
}

/// The version each name holds where a call to a command the module cannot
/// see sits is recorded, in a procedure as at the top level; a version a
/// later definition makes is not.
#[test]
fn an_unseen_call_records_the_versions_live_where_it_sits() {
    let ssa = ssa_of_function("set x 1\nset y 2\nfoo\nset x 3\nputs $x$y\n", "::top");
    let x = ssa.var_symbol("x").expect("x");
    let y = ssa.var_symbol("y").expect("y");
    assert!(ssa.is_observed_by_unseen_call(x, 1));
    assert!(ssa.is_observed_by_unseen_call(y, 1));
    assert!(!ssa.is_observed_by_unseen_call(x, 2));
    assert!(ssa.name_is_observed_by_unseen_call("x", 1));
    assert!(!ssa.name_is_observed_by_unseen_call("nothing", 1));
    let ssa = ssa_of_function("proc p {} {\n set x 1\n foo\n puts $x\n}\n", "::p");
    assert!(ssa.name_is_observed_by_unseen_call("x", 1));
    // A call the module resolves records nothing.
    for (source, name) in [
        ("proc foo {} { puts hi }\nset x 1\nfoo\nputs $x\n", "::top"),
        ("set x 1\nputs $x\n", "::top"),
    ] {
        let ssa = ssa_of_function(source, name);
        assert!(!ssa.name_is_observed_by_unseen_call("x", 1), "{source}");
    }
    // A command a substitution runs, a condition's and an arm's count too.
    for source in [
        "set x 1\nputs [foo]\n",
        "set x 1\nwhile {[foo]} { puts hi }\n",
        "set x 1\nswitch -glob -- $s { q* { foo } }\n",
    ] {
        let ssa = ssa_of_function(source, "::top");
        assert!(ssa.name_is_observed_by_unseen_call("x", 1), "{source}");
    }
}

/// A sourced file runs in the frame of the call, so it may write or read
/// any name there: a top-level name, and a procedure's local too.
#[test]
fn a_sourced_file_records_the_versions_live_where_it_sits_in_a_procedure_too() {
    for (source, name) in [
        ("set x 1\nsource other.tcl\nputs $x\n", "::top"),
        ("set x 1\nputs [source other.tcl]\n", "::top"),
        (
            "proc p {} {\n set x 1\n source other.tcl\n puts $x\n}\n",
            "::p",
        ),
        (
            "proc p {s} {\n set x 1\n switch -glob -- $s { q* { source other.tcl } }\n puts $x\n}\n",
            "::p",
        ),
    ] {
        let ssa = ssa_of_function(source, name);
        assert!(ssa.name_is_observed_by_unseen_call("x", 1), "{source}");
    }
}

/// The statements of `function` in `source`, in block order.
fn ssa_statements(ssa: &SsaFunction) -> Vec<&SsaStatement> {
    ssa.blocks
        .values()
        .flat_map(|block| &block.statements)
        .collect()
}

/// A callee an arm of an opaque `switch` calls writes the caller's name
/// through `upvar`, as it would at a call the graph lowers: the statement
/// that follows the `switch` defines the name as a *may*-definition that
/// uses the version before it, so the value the name held when no arm runs
/// survives into the join.
#[test]
fn an_opaque_switch_arm_that_calls_a_writer_may_define_the_name_it_writes() {
    let ssa = ssa_of_function(
        "proc zero {v} {upvar 1 $v x; set x 0}\nproc p {s} {\n set line 5\n\
             switch -glob -- $s { g* { zero line } }\n puts $line\n}\n",
        "::p",
    );
    let marker = ssa_statements(&ssa)
        .into_iter()
        .find(|statement| is_arm_writes_marker(&statement.statement))
        .expect("the switch is followed by the marker for what its callee writes");
    let line = ssa.var_symbol("line").expect("line");
    assert_eq!(marker.may_defs, HashSet::from([line]));
    assert_eq!(marker.defs[&line], 2);
    assert_eq!(marker.uses[&line], 1);
    assert!(marker.quoted_uses.contains(&line));
    // A callee that writes a global, not this frame's name, adds no
    // may-definition of a local, and an arm that calls nothing adds no
    // marker.
    for source in [
        "proc hit {} {global hits; incr hits}\nproc p {s} {\n set n 5\n\
             switch -glob -- $s { g* { hit } }\n puts $n\n}\n",
        "proc p {s} {\n set n 5\n switch -glob -- $s { g* { set m 1 } }\n puts $n\n}\n",
    ] {
        let ssa = ssa_of_function(source, "::p");
        let n = ssa.var_symbol("n").expect("n");
        assert!(
            ssa_statements(&ssa)
                .into_iter()
                .filter(|statement| is_arm_writes_marker(&statement.statement))
                .all(|statement| !statement.may_defs.contains(&n)),
            "{source}"
        );
    }
}

/// The body of an opaque `catch` stays inside the call, so what it writes is
/// a *may*-definition the marker ahead of the call states: it reads the
/// version before it, the solver joins the two, and the call itself defines
/// only its result variable.
#[test]
fn an_opaque_catch_may_define_the_names_its_body_writes() {
    let ssa = ssa_of_function(
        "set g 5\ncatch { if {[gets stdin] eq {q}} { set g 0 } } msg\nputs $g$msg\n",
        "::top",
    );
    let statements = ssa_statements(&ssa);
    let marker = statements
        .iter()
        .find(|statement| is_arm_writes_marker(&statement.statement))
        .expect("the catch body's writes are stated ahead of the call");
    let g = ssa.var_symbol("g").expect("g");
    let msg = ssa.var_symbol("msg").expect("msg");
    assert_eq!(marker.may_defs, HashSet::from([g]));
    assert_eq!(marker.defs[&g], 2);
    assert_eq!(marker.uses[&g], 1);
    assert!(marker.quoted_uses.contains(&g));
    let call = statements
            .iter()
            .find(|statement| {
                matches!(&statement.statement, Statement::Call { command, .. } if command == "catch")
            })
            .expect("the catch call");
    assert!(call.defs.contains_key(&msg));
    assert!(
        !call.defs.contains_key(&g),
        "the call defines no name its body writes"
    );
}

/// The call reads a name its body writes as the body's own: what the body
/// consumes of the frame is its reads less its writes, as an opaque
/// `switch`'s arms are read, so `catch {set i 0; puts $i}` reads nothing
/// of `i`, while a name the body only reads is still a read of the frame.
#[test]
fn an_opaque_catch_reads_its_own_writes_as_its_own() {
    let ssa = ssa_of_function("catch {set i 0; puts $i; puts $j}\n", "::top");
    let statements = ssa_statements(&ssa);
    let call = statements
            .iter()
            .find(|statement| {
                matches!(&statement.statement, Statement::Call { command, .. } if command == "catch")
            })
            .expect("the catch call");
    let i = ssa.var_symbol("i").expect("i");
    let j = ssa.var_symbol("j").expect("j");
    assert!(!call.uses.contains_key(&i), "{:?}", call.uses);
    assert_eq!(call.uses.get(&j), Some(&0), "{:?}", call.uses);
}

/// A call to a command the module cannot see reads its words before its
/// head runs, so the version each word reads is the one the call holds,
/// which is recorded — the code the call reaches may read it — and the code
/// may rewrite the name, so a read after the call is of the fresh version
/// the marker leaves, which is not, and whose binding is the version the
/// call held. A name only a braced word carries, which the code may
/// evaluate, is recorded as well.
#[test]
fn a_call_the_module_cannot_see_holds_the_versions_its_words_read() {
    for (source, names) in [
        ("set cmd puts\n$cmd hi\nputs $cmd\n", vec!["cmd"]),
        ("set a 1\nset b 2\n$cmd $a $b\nputs $a$b\n", vec!["a", "b"]),
        ("set a 1\nset b 2\nget $a $b\nputs $a$b\n", vec!["a", "b"]),
        ("set f x.tcl\nsource $f\nputs $f\n", vec!["f"]),
    ] {
        let ssa = ssa_of_function(source, "::top");
        for name in names {
            let symbol = ssa.var_symbol(name).expect("the name is an SSA symbol");
            assert!(
                ssa.name_is_observed_by_unseen_call(name, 1),
                "{source}: {name}#1"
            );
            assert!(
                !ssa.name_is_observed_by_unseen_call(name, 2),
                "{source}: {name}#2"
            );
            assert_eq!(ssa.binding_version(symbol, 2), 1, "{source}: {name}#2");
        }
    }
    for source in [
        "set cmd puts\nset x 1\n$cmd hi\nputs $x\n",
        "set x 1\nget {$x}\nputs $x\n",
        "set x 1\nputs [get $x]\nputs $x\n",
    ] {
        let ssa = ssa_of_function(source, "::top");
        assert!(ssa.name_is_observed_by_unseen_call("x", 1), "{source}");
    }
}

/// The marker beside such a call defines no name: each name live past it
/// takes a fresh version where it stands
/// ([`SsaFunction::value_clobbers`]), whose binding is the version before
/// it, and a loop that holds the call places the φ the fresh version
/// needs. A name no read after the marker reaches takes none.
#[test]
fn the_marker_beside_a_call_the_module_cannot_see_gives_live_names_fresh_versions() {
    let fresh = |ssa: &SsaFunction, symbol: Symbol| -> Vec<(Version, Version)> {
        ssa.value_clobbers
            .values()
            .flat_map(HashMap::values)
            .filter_map(|versions| versions.get(&symbol).copied())
            .collect()
    };
    let ssa = ssa_of_function("set a 1\nget $a\nputs $a\n", "::top");
    let marker = ssa_statements(&ssa)
        .into_iter()
        .find(|statement| is_unseen_call_marker(&statement.statement))
        .expect("the call is followed by its marker");
    assert!(marker.defs.is_empty() && marker.may_defs.is_empty());
    let a = ssa.var_symbol("a").expect("a");
    assert_eq!(fresh(&ssa, a), [(1, 2)]);
    assert_eq!(ssa.binding_version(a, 2), 1);

    let ssa = ssa_of_function(
        "set a 1\nwhile {[gets stdin line] >= 0} { get $a }\nputs $a\n",
        "::top",
    );
    assert!(
        ssa.blocks
            .values()
            .any(|block| block.phis.iter().any(|phi| ssa.var_name(phi.name) == "a")),
        "the loop joins the fresh version `a` takes where the call sits"
    );

    let ssa = ssa_of_function("set a 1\nget $a\nset a 2\nputs $a\n", "::top");
    let a = ssa.var_symbol("a").expect("a");
    assert_eq!(fresh(&ssa, a), [] as [(u32, u32); 0]);
}

/// A call to a command the module cannot see inside a `catch` body, or with
/// a computed head, records the versions live where it sits, as a call the
/// graph lowers does, in a procedure as at the top level; a call the module
/// resolves records nothing.
#[test]
fn a_catch_body_and_a_computed_head_record_the_versions_live_where_they_sit() {
    for source in [
        "set x 1\ncatch {foo}\nputs $x\n",
        "set x 1\ncatch {foo} msg\nputs $x\n",
        "set x 1\ncatch { if {1} { foo } }\nputs $x\n",
        "set x 1\ncatch { puts [foo] }\nputs $x\n",
        "set x 1\ncatch { source other.tcl }\nputs $x\n",
        "set x 1\n$cmd\nputs $x\n",
        "set x 1\n$cmd arg\nputs $x\n",
        "set x 1\nputs [$cmd]\n",
        "set x 1\nif {[$cmd]} { puts hi }\n",
        "set x 1\nif {[catch {foo}]} { puts hi }\n",
        "set x 1\nset rc [catch {foo}]\nputs $x\n",
        "set x 1\nputs [catch { puts [foo] }]\n",
        "set x 1\nif {[catch $script]} { puts hi }\n",
    ] {
        let ssa = ssa_of_function(source, "::top");
        assert!(ssa.name_is_observed_by_unseen_call("x", 1), "{source}");
    }
    for (source, name, observed) in [
        (
            "proc p {} {\n set x 1\n catch { if {1} { foo } }\n puts $x\n}\n",
            "::p",
            true,
        ),
        ("proc p {} {\n set x 1\n $cmd\n puts $x\n}\n", "::p", true),
        ("proc p {} {\n set x 1\n puts [$cmd]\n}\n", "::p", true),
        (
            "proc p {} {\n set x 1\n catch { if {1} { source other.tcl } }\n puts $x\n}\n",
            "::p",
            true,
        ),
        (
            "proc foo {} { puts hi }\nset x 1\ncatch { if {1} { foo } }\nputs $x\n",
            "::top",
            false,
        ),
    ] {
        let ssa = ssa_of_function(source, name);
        assert_eq!(
            ssa.name_is_observed_by_unseen_call("x", 1),
            observed,
            "{source}"
        );
    }
}

/// A command an arm runs that may write any name — `namespace eval`,
/// `dict with`, `eval $script`, an `uplevel` — is followed by the barrier
/// the graph puts after such a command it lowers; an arm that holds none
/// has no barrier after it.
#[test]
fn an_opaque_switch_arm_that_may_write_any_name_is_followed_by_a_barrier() {
    let barrier_after_switch = |source: &str, name: &str| {
        let ssa = ssa_of_function(source, name);
        let statements = ssa_statements(&ssa);
        statements.iter().any(|statement| {
            matches!(
                &statement.statement,
                Statement::Barrier { tokens: Some(tokens), .. }
                    if tokens.synthetic == Some(crate::ir::SyntheticMarker::CallerFrameOpaque)
            )
        })
    };
    for (source, name) in [
        (
            "set go 1\nswitch -glob -- [gets stdin] { q* { namespace eval :: {set go 0} } }\n",
            "::top",
        ),
        (
            "proc p {s} {\n set go 1\n switch -glob -- $s { q* { dict with d { set go 0 } } }\n}\n",
            "::p",
        ),
        (
            "proc p {s} {\n set go 1\n switch -glob -- $s { q* { eval $s } }\n}\n",
            "::p",
        ),
    ] {
        assert!(barrier_after_switch(source, name), "{source}");
    }
    assert!(!barrier_after_switch(
        "proc p {s} {\n set go 1\n switch -glob -- $s { q* { set go 0 } }\n}\n",
        "::p"
    ));
}

#[test]
fn supplied_switch_may_writes_keep_actual_context_and_unavailable_residual() {
    let (context, unit) = logical_metadata_fixture("switch -glob $subject {a {set arm ONE}}");
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    let selection = SsaInvocationContext::supplied(registry, Some(metadata));
    let statement = unit
        .ir_module
        .top_level
        .statements
        .iter()
        .find(|statement| matches!(statement, Statement::Switch { .. }))
        .expect("actual retained opaque switch");
    assert!(switch_may_defs_in_context(statement, selection).contains(&"arm".to_owned()));
    let Statement::Switch { arms, .. } = statement else {
        unreachable!()
    };
    let body = arms.iter().find_map(|arm| arm.body.as_ref()).unwrap();
    let mut names = BTreeSet::new();
    assert!(!nested_writes_in_context(body, selection, &mut names));
    assert!(names.contains("arm"));
    assert!(nested_writes_in_context(
        body,
        SsaInvocationContext::supplied(registry, None),
        &mut BTreeSet::new(),
    ));
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    let foreign_profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        foreign_profile,
        input.unit_profile(),
        input.context_registry().clone(),
        input.lexer_config(),
    );
    let foreign_metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, &foreign,
        )
        .expect("same actual Registry and lexer configuration");
    assert_eq!(foreign.lexer_config(), input.lexer_config());
    assert_ne!(foreign, *input);
    assert!(nested_writes_in_context(
        body,
        SsaInvocationContext::supplied(registry, Some(foreign_metadata)),
        &mut BTreeSet::new(),
    ));
}

/// Trust ends where an unknown command runs, not for the whole file: a
/// later `eval $y` withdraws the command world from that point on, so the
/// calls that ran before it keep their resolved targets and carry no
/// unseen-call marker. tclsh 8.4.20, 8.6.18, 9.0.4 and 9.1.0 run
/// `set x 1; puts hi; puts $x` before the eval raises `can't read "y"`.
#[test]
fn a_later_unknown_script_leaves_the_calls_before_it_seen() {
    let source = "set x 1\nputs hi\nputs $x\neval $y\n";
    let eval_at = u32::try_from(source.find("eval").unwrap()).unwrap();
    let ssa = ssa_of_function(source, "::top");
    let markers: Vec<u32> = ssa_statements(&ssa)
        .into_iter()
        .filter(|statement| is_unseen_call_marker(&statement.statement))
        .map(|statement| statement.statement.span().start())
        .collect();
    assert!(
        markers.iter().all(|&start| start >= eval_at),
        "only the eval may be unseen: {markers:?}"
    );
}
