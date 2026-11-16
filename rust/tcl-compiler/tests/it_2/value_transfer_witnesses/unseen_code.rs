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

//! Writes made by code the module cannot see: opaque arms, callbacks, sourced and autoloaded commands, stubs and aliases.

use super::*;

/// A write an arm of a `switch` the flow graph keeps as one statement makes
/// is never folded away: tclsh 8.4 to 9.1 print `b` and `0` for each of these
/// programs, before and after the optimiser, where taking the earlier `go` as
/// the value after the switch printed `a` and `1`. `-nocase` is from 8.5.
#[test]
fn a_write_an_opaque_switch_arm_makes_is_never_folded_away() {
    for (arm, first) in [
        ("-glob -- $s { q* { set go 0 } }", "8.4"),
        ("-nocase -- $s { Q1 { set go 0 } }", "8.5"),
        ("-regexp -- $s { {^q} { set go 0 } }", "8.4"),
        ("-glob -- $s { x - q* { set go 0 } }", "8.4"),
        (
            "-glob -- $s { z* { set other 1 } default { set go 0 } }",
            "8.4",
        ),
    ] {
        let source = format!(
            "set go 1\nset s [string tolower Q1]\nswitch {arm}\nif {{$go}} {{puts a}} else {{puts b}}\nputs $go\n"
        );
        prints_under_releases_from(&source, "b\n0\n", first);
        let in_proc = format!(
            "proc p {{}} {{\n set go 1\n set s [string tolower Q1]\n switch {arm}\n if {{$go}} {{puts a}} else {{puts b}}\n puts $go\n}}\np\n"
        );
        prints_under_releases_from(&in_proc, "b\n0\n", first);
    }
}

/// What a command an arm of such a `switch` runs does to the frame is never
/// folded away either: a callee that writes the caller's `go` through `upvar`,
/// `namespace eval` at the global level and `dict with` (8.5 on) each leave
/// `b` and `0` at the top level, where the earlier `go` printed `a` and `1`;
/// in a procedure `namespace eval ::` writes the global, and the local keeps
/// its value before and after the optimiser.
#[test]
fn a_write_a_command_an_opaque_switch_arm_runs_is_never_folded_away() {
    for (arm, first, in_proc) in [
        ("zero go", "8.4", "b\n0\n"),
        ("namespace eval :: {set go 0}", "8.4", "a\n1\n"),
        ("set d {}; dict with d {set go 0}", "8.5", "b\n0\n"),
    ] {
        let source = format!(
            "proc zero {{v}} {{upvar 1 $v x; set x 0}}\nset go 1\nset s [string tolower Q1]\n\
             switch -glob -- $s {{ q* {{ {arm} }} }}\nif {{$go}} {{puts a}} else {{puts b}}\nputs $go\n"
        );
        prints_under_releases_from(&source, "b\n0\n", first);
        let in_proc_source = format!(
            "proc zero {{v}} {{upvar 1 $v x; set x 0}}\nproc p {{}} {{\n set go 1\n set s [string tolower Q1]\n \
             switch -glob -- $s {{ q* {{ {arm} }} }}\n if {{$go}} {{puts a}} else {{puts b}}\n puts $go\n}}\np\n"
        );
        prints_under_releases_from(&in_proc_source, in_proc, first);
    }
}

/// A write a callback script makes — an `after` handler, a variable trace's
/// callback, a procedure named as a callback — is never folded away either:
/// each program prints the callback's value, `1` and `b`, under every
/// release, before and after the optimiser.
#[test]
fn a_write_a_callback_script_makes_is_never_folded_away() {
    for source in [
        "set done 0\nafter 10 { set done 1 }\nafter 50\nupdate\nputs $done\n",
        "proc tick {} { set ::done 1 }\nset done 0\nafter 10 tick\nafter 50\nupdate\nputs $done\n",
        "set done 0\nafter 10 { set ::done 1 }\nafter 50\nupdate\nif {$done} {puts 1} else {puts 0}\n",
        "set done 0\nafter 10 \"set ::done 1\"\nafter 50\nupdate\nputs $done\n",
        "set done 0\nafter 10 [list set ::done 1]\nafter 50\nupdate\nputs $done\n",
        "proc tick {n} { set ::done $n }\nset done 0\nafter 10 [list tick 1]\nafter 50\nupdate\nputs $done\n",
    ] {
        prints_under_every_release(source, "1\n");
    }
    prints_under_every_release(
        "set go 1\ntrace add variable x write { set ::go 0 ;# }\nset x 1\nif {$go} {puts a} else {puts b}\n",
        "b\n",
    );
}

/// A command the module cannot see may write a plain top-level name as it
/// writes `::g`, so the name is never folded across the call: `foo` here is
/// defined at run time, from a file the program writes and sources, and sets
/// the global — tclsh 8.4 to 9.1 print `six` and `6`, where taking `5` across
/// the call printed `other` and `5`.
#[test]
fn a_write_a_command_the_module_cannot_see_makes_is_never_folded_away() {
    let define = "set f [file join [file dirname [info script]] vt-unseen-[pid].tcl]\n\
                  set fh [open $f w]\nputs $fh {proc foo {} {set ::g 6}}\nclose $fh\n\
                  source $f\nfile delete $f\n";
    for tail in [
        "set g 5\nfoo\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set ::g 5\nfoo\nif {$::g == 6} {puts six} else {puts other}\nputs $::g\n",
        "set g 5\nwhile {$g != 6} { foo }\nputs six\nputs $g\n",
    ] {
        prints_under_every_release(&format!("{define}{tail}"), "six\n6\n");
    }
}

/// A sourced file runs in the frame of the call, so it writes a procedure's
/// local as well as a global: the file here sets `g` to 6, and tclsh 8.4 to 9.1
/// print `six` and `6` at the top level and in a procedure, where taking `5`
/// across the `source` printed `other` and `5`.
#[test]
fn a_write_a_sourced_file_makes_is_never_folded_away() {
    let write_file = "set f [file join [file dirname [info script]] vt-sourced-[pid].tcl]\n\
                      set fh [open $f w]\nputs $fh {set g 6}\nclose $fh\n";
    let top = format!(
        "{write_file}set g 5\nsource $f\nfile delete $f\nif {{$g == 6}} {{puts six}} else {{puts other}}\nputs $g\n"
    );
    prints_under_every_release(&top, "six\n6\n");
    let in_proc = format!(
        "proc p {{f}} {{\n set g 5\n source $f\n if {{$g == 6}} {{puts six}} else {{puts other}}\n puts $g\n}}\n{write_file}p $f\nfile delete $f\n"
    );
    prints_under_every_release(&in_proc, "six\n6\n");
}

/// A command the module cannot see may write a procedure's local as well as a
/// global: a callee an autoloader or the unresolved-command handler brings in
/// runs `upvar 1` into the frame that called it. `missing` here is defined by
/// `auto_index` when Tcl's `unknown` first looks for it, and sets the
/// caller's `g` to 6 — tclsh 8.4 to 9.1 print `six` and `6` from each
/// procedure, where taking `5` across the call printed `other` and `5`, and the
/// loop the call ends never stopped.
#[test]
fn a_write_an_autoloaded_command_makes_to_a_procedure_local_is_never_folded_away() {
    let define = "set auto_index(missing) {proc missing {} {upvar 1 g g; set g 6}}\n";
    for body in [
        " set g 5\n missing\n if {$g == 6} {puts six} else {puts other}\n puts $g\n",
        " set g 5\n set r [missing]\n if {$g == 6} {puts six} else {puts other}\n puts $g\n",
        " set g 5\n while {$g != 6} { missing }\n puts six\n puts $g\n",
    ] {
        prints_under_every_release(
            &format!("{define}proc p {{}} {{\n{body}}}\np\n"),
            "six\n6\n",
        );
    }
}

/// A command the module cannot see — here `foo`, defined at run time from a
/// file the program writes and sources — writes a plain top-level name as it
/// writes `::g` when it runs inside the body of a `catch` the flow graph
/// keeps as one statement, or through a computed head: tclsh 8.4 to 9.1 print
/// `six` and `6` for each program, where taking `5` across the call printed
/// `other` and `5` and a rewrite followed.
#[test]
fn a_write_a_catch_body_or_a_computed_head_runs_is_never_folded_away() {
    let define = "set f [file join [file dirname [info script]] vt-unseen-[pid].tcl]\n\
                  set fh [open $f w]\nputs $fh {proc foo {} {set ::g 6}}\nclose $fh\n\
                  source $f\nfile delete $f\n";
    for tail in [
        "set g 5\ncatch {foo}\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set g 5\ncatch {foo} msg\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set g 5\ncatch {if {1} {foo}}\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set ::g 5\ncatch {foo}\nif {$::g == 6} {puts six} else {puts other}\nputs $::g\n",
        "set g 5\nif {[catch {foo}]} {puts bad}\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set cmd foo\nset g 5\n$cmd\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set cmd foo\nset g 5\nwhile {[$cmd] != 7} {break}\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
    ] {
        prints_under_every_release(&format!("{define}{tail}"), "six\n6\n");
    }
    for tail in [
        "set g 5\ncatch {puts [foo]}\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set cmd foo\nset g 5\nputs [$cmd]\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
    ] {
        prints_under_every_release(&format!("{define}{tail}"), "6\nsix\n6\n");
    }
    // A command in the body that may write any name.
    for program in [
        "set go 1\ncatch { namespace eval :: {set go 0} }\nif {$go} {puts a} else {puts b}\n",
        "set go 1\nset script {set go 0}\ncatch { eval $script }\nif {$go} {puts a} else {puts b}\n",
    ] {
        prints_under_every_release(program, "b\n");
    }
}

/// A command the module cannot see — `foo` again, defined at run time — runs
/// inside a body that is no body of the frame the substitution is written in:
/// a lambda `apply` runs, a `namespace eval` or `uplevel` body, the text a
/// `subst` substitutes, an expression word inside a body. It writes `::g`
/// there as it does anywhere, so tclsh 8.4 to 9.1 print `six` and `6` for each
/// program, where the value before the call was taken across it and the
/// condition decided. A lambda body that writes a name of its own, and a
/// `subst` of text that runs nothing unseen, leave the condition decided.
#[test]
fn a_substitution_runs_the_unseen_code_in_every_body_it_holds() {
    let define = "set f [file join [file dirname [info script]] vt-unseen-[pid].tcl]\n\
                  set fh [open $f w]\nputs $fh {proc foo {} {set ::g 6}}\nclose $fh\n\
                  source $f\nfile delete $f\n";
    let check = "if {$g == 6} {puts six} else {puts other}\nputs $g\n";
    for form in [
        "set x [namespace eval ns {foo}]",
        "set x [uplevel #0 {foo}]",
        "set x [subst {[foo]}]",
        "set x [catch {expr {[foo] + 1}}]",
        "set x [catch {if {[foo]} {set y 1}}]",
    ] {
        prints_under_every_release(&format!("{define}set g 5\n{form}\n{check}"), "six\n6\n");
    }
    for form in [
        "set x [apply {{} {foo}}]",
        "set x [catch {apply {{} {foo}}}]",
        "set x [apply {{} {if {[foo]} {set y 1}}}]",
        "set x [apply {{n} {foo}} 1]",
    ] {
        prints_under_releases_from(
            &format!("{define}set g 5\n{form}\n{check}"),
            "six\n6\n",
            "8.5",
        );
    }
    // A body with nothing unseen in it leaves the condition decided: the
    // optimiser folds it, and the program prints the same.
    let decided = "set g 5\nset x [namespace eval ns {set y 1}]\n\
                   if {$g == 5} {puts five} else {puts other}\nputs $g\n";
    let lambda = "set g 5\nset x [apply {{} {set y 1}}]\n\
                  if {$g == 5} {puts five} else {puts other}\nputs $g\n";
    let text = "set g 5\nset x [subst {[set y 1]}]\n\
                if {$g == 5} {puts five} else {puts other}\nputs $g\n";
    for (source, first) in [(decided, "8.4"), (lambda, "8.5"), (text, "8.4")] {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(!optimised(source, dialect).0.contains("other"), "{source}");
        }
        prints_under_releases_from(source, "five\n5\n", first);
    }
}

/// A call to a command the module cannot see reads its words before its head
/// runs and may read or write any global afterwards: `foo` here is defined at
/// run time, reads `g` and sets `g` and `m`, and tclsh 8.4 to 9.1 print what each
/// program says. The operand keeps the value it read; the store the callee reads
/// stays, whether a later read follows it or a later store overwrites it —
/// deleting it made the callee raise `can't read "g"`; and a read after the
/// call is never taken for the value before it, of the name the call's words
/// read or of one they do not.
#[test]
fn a_call_the_module_cannot_see_reads_its_words_first_and_may_read_and_write_after() {
    let define = "set f [file join [file dirname [info script]] vt-unseen-[pid].tcl]\n\
                  set fh [open $f w]\nputs $fh {proc foo {n} {global g; puts \"n=$n g=$g\"; set ::g 6; set ::m 4}}\n\
                  close $fh\nsource $f\nfile delete $f\n";
    for (tail, expected) in [
        ("set g 5\nfoo $g\nputs $g\n", "n=5 g=5\n6\n"),
        ("set g 5\nset cmd foo\n$cmd $g\nputs $g\n", "n=5 g=5\n6\n"),
        (
            "set cmd foo\nset g 5\n$cmd $g\nputs [expr {$g + 0}]\n",
            "n=5 g=5\n6\n",
        ),
        (
            "set g 5\nset m 3\nfoo $g\nputs $g\nputs $m\n",
            "n=5 g=5\n6\n4\n",
        ),
        ("set g 5\nfoo 1\nputs $g\n", "n=1 g=5\n6\n"),
        ("set cmd foo\nset g 5\n$cmd 1\nputs $g\n", "n=1 g=5\n6\n"),
        ("set g 5\nfoo 1\nset g 7\nputs $g\n", "n=1 g=5\n7\n"),
        (
            "set cmd foo\nset g 5\n$cmd 1\nset g 7\nputs $g\n",
            "n=1 g=5\n7\n",
        ),
    ] {
        prints_under_every_release(&format!("{define}{tail}"), expected);
    }
}

/// A callback spelled as several words, which `after` joins as `concat` does,
/// or one the scan cannot read — a computed script, a command the module never
/// defines, an alias, an expansion — writes the variable a loop waits on, or
/// the one a later read folds: tclsh prints `1` for each program, where
/// taking `0` across the `update` printed `0`. A callback that only cancels,
/// or stores a script that writes another name, leaves the value alone.
#[test]
fn a_write_a_callback_the_scan_could_not_read_makes_is_never_folded_away() {
    let define = "set f [file join [file dirname [info script]] vt-callback-[pid].tcl]\n\
                  set fh [open $f w]\nputs $fh {proc finish {} {set ::done 1}}\nclose $fh\n\
                  source $f\nfile delete $f\n";
    // Two rounds, so a callback that stores another still runs.
    let wait = "after 30\nupdate\nafter 30\nupdate\nputs $done\n";
    for head in [
        "after 10 set done 1",
        "after idle set done 1",
        "after 10 incr done",
        "after 10 {set done} 1",
        "set script {set done 1}\nafter 10 $script",
        "after 10 finish",
        "interp alias {} fin {} set done 1\nafter 10 fin",
        "proc build {} {return {set done 1}}\nafter 10 [build]",
        "after 10 {after 10 set done 1}",
    ] {
        prints_under_every_release(&format!("{define}set done 0\n{head}\n{wait}"), "1\n");
    }
    prints_under_releases_from(
        &format!("set done 0\nafter 10 {{*}}[list set done 1]\n{wait}"),
        "1\n",
        "8.5",
    );
    for head in [
        "after 10 {set done 1}\nafter cancel {set done 1}",
        "set id [after 10 {set done 1}]\nafter cancel $id",
        "after 10 {set other 1}",
        "after 10 set other 1",
    ] {
        prints_under_every_release(&format!("set done 0\n{head}\n{wait}"), "0\n");
    }
}

/// An alias's target word and the words after it are the one command a call
/// to the alias runs, so an alias to `string length` stores no callback that
/// writes a variable, however `string` is spelled: the top-level branch on `x`
/// is decided and folded, and tclsh 8.4 to 9.1 print `kept` before and after
/// the optimiser. Read alone, the target word is `string` with no subcommand,
/// a script that may write any variable, and the branch was left undecided.
#[test]
fn an_alias_to_a_command_that_writes_nothing_leaves_its_branch_decided() {
    for target in ["string length", "::string length"] {
        let source = format!(
            "interp alias {{}} safe {{}} {target}\nset x 5\nsafe {{set x 6}}\n\
             if {{$x == 5}} {{puts kept}} else {{puts changed}}\n"
        );
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
            let unit = unit_of(&source, dialect);
            assert!(
                !unit.top_level.sccp.constant_branches.is_empty(),
                "{dialect}: {source}"
            );
            let (rewritten, rewrites) = optimised(&source, dialect);
            assert!(
                !rewritten.contains("changed"),
                "{dialect}: {source}\n{rewritten}\n{rewrites:#?}"
            );
        }
        prints_under_every_release(&source, "kept\n");
    }
}

/// The e5 program with its stub block in place: `db_query` and `db_eval`
/// declared under each `-frame` word, called in a procedure that holds `g`.
fn stubbed_e5(frame: &str) -> String {
    format!(
        "# tcl-lsp: stubs-begin\n# tcl-lsp: stub db_query {{sql}}{frame}\n\
         # tcl-lsp: stub db_eval {{sql script:body}}{frame}\n# tcl-lsp: stubs-end\n\
         proc q {{}} {{\n    set g 5\n    db_query {{select 1}}\n    \
         if {{$g == 5}} {{puts five}} else {{puts other}}\n}}\n\
         proc p {{}} {{\n    set g 5\n    db_eval {{select 1}} {{set g 6}}\n    \
         if {{$g == 5}} {{puts five}} else {{puts other}}\n}}\nq\np\n"
    )
}

/// A stub that states its frame effect is a command the module can name.
/// `-frame own` and `-frame none` state that the call crosses no
/// frame, so the locals `q` holds at `db_query {select 1}` keep their values
/// and `$g == 5` is decided; `-frame caller` states `argparse`'s effect, and a
/// stub that states none may reach the frame that calls it as code the module
/// cannot see may, so the condition stays open under both. A stub with a body
/// role answers as the registry's rule answers for one: `db_eval`'s call is
/// the barrier `time {…}` lowers to, and the condition after it stays open
/// whatever the frame effect. Each declaration runs under tclsh with a
/// definition it describes — a procedure that returns, or one that sets its
/// caller's `g` through `upvar 1` — and prints what the claims allow.
#[test]
fn a_stub_that_states_its_frame_effect_is_a_named_head() {
    let own = "proc db_query {sql} {return 1}\nproc db_eval {sql script} {uplevel 1 $script}\n";
    let caller = "proc db_query {sql} {upvar 1 g g; set g 6}\nproc db_eval {sql script} {uplevel 1 $script}\n";
    for (frame, definitions, claims, printed) in [
        (" -frame own", own, &[true][..], "five\nother\n"),
        (" -frame none", own, &[true][..], "five\nother\n"),
        (" -frame caller", caller, &[][..], "other\nother\n"),
        ("", caller, &[][..], "other\nother\n"),
    ] {
        let source = stubbed_e5(frame);
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert_eq!(
                condition_claims(&source, dialect),
                claims,
                "{dialect}{frame}:\n{source}"
            );
        }
        for (series, tclsh) in releases_on_path() {
            assert_eq!(
                run_script(&tclsh, &format!("{definitions}{source}")),
                Some((true, printed.to_owned())),
                "tclsh{series}{frame}"
            );
        }
    }
    let registry = static_context_for("tcl8.6").commands();
    let barrier_of = |source: &str, head: &str| {
        let declared =
            tcl_compiler::analyser::utils::document_declared_surface(source, None, "tcl8.6");
        let module = tcl_compiler::lowering::lower_to_ir_with(
            tcl_compiler::lowering::Lowerer::with_config(
                registry,
                tcl_lexer::LexerConfig::for_profile(registry.profile()),
            )
            .with_declared_commands(Some(&declared)),
            source,
        );
        let mut reasons = Vec::new();
        tcl_compiler::ir::for_each_statement(
            &module
                .procedures
                .get(&fixture_id(&module.procedures, "::p"))
                .expect("authored fixture body")
                .body,
            &mut |statement| {
                if let Statement::Barrier {
                    command, reason, ..
                } = statement
                    && command == head
                {
                    reasons.push(reason.clone());
                }
            },
        );
        reasons
    };
    let timed = barrier_of(
        "proc p {} {\n    set g 5\n    time {set g 6}\n    puts $g\n}\n",
        "time",
    );
    assert_eq!(timed, ["unsupported body command"]);
    for frame in ["", " -frame own", " -frame none", " -frame caller"] {
        assert_eq!(barrier_of(&stubbed_e5(frame), "db_eval"), timed, "{frame}");
    }
    // Run as a callback, a command that reaches the frame it runs in writes
    // the global frame: `$done` stays open after `update` under `-frame
    // caller`, and with no `-frame`, where tclsh prints 1.
    let callback = |frame: &str| {
        format!(
            "# tcl-lsp: stubs-begin\n# tcl-lsp: stub db_bind {{spec}}{frame}\n\
             # tcl-lsp: stubs-end\nset done 0\nafter 10 {{db_bind {{done}}}}\nafter 50\n\
             update\nif {{$done}} {{puts 1}} else {{puts 0}}\n"
        )
    };
    let reaches = "proc db_bind {spec} {upvar 1 done d; set d 1}\n";
    for (frame, definition, claims, printed) in [
        (" -frame caller", reaches, &[][..], "1\n"),
        ("", reaches, &[][..], "1\n"),
        (
            " -frame own",
            "proc db_bind {spec} {return}\n",
            &[false][..],
            "0\n",
        ),
    ] {
        let source = callback(frame);
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert_eq!(
                condition_claims(&source, dialect),
                claims,
                "{dialect}{frame}:\n{source}"
            );
        }
        for (series, tclsh) in releases_on_path() {
            assert_eq!(
                run_script(&tclsh, &format!("{definition}{source}")),
                Some((true, printed.to_owned())),
                "tclsh{series}{frame}"
            );
        }
    }
}

/// A lambda an `after` callback applies runs its body in a frame of its own,
/// so what it writes in the global frame is the callback's write: the scan of
/// callback scripts reads the lambda as a procedure's summary reads a body,
/// and `$done` after `update` decides nothing. Each program prints `changed`
/// and `1` under tclsh 8.5 to 9.1, before and after `tcl opt`, where the
/// rewrite had printed `zero` and `0`.
#[test]
fn a_lambda_a_callback_applies_writes_the_globals_its_body_writes() {
    for (lambda, words) in [
        ("{} {global done; set done 1}", ""),
        ("{} {set ::done 1}", ""),
        ("{} {upvar #0 done d; set d 1}", ""),
        ("{} {incr ::done}", ""),
        ("{} {global done; foreach done {1} {}}", ""),
        ("{} {global done; catch {set done 1}}", ""),
        ("{} {uplevel #0 {set done 1}}", ""),
        ("{} {apply {{} {global done; set done 1}}}", ""),
        ("{} {after 0 {set done 1}; update}", ""),
        ("{x} {global done; set done $x}", " 1"),
    ] {
        for callback in [
            format!("after 10 {{apply {{{lambda}}}{words}}}"),
            format!("after 10 [list apply {{{lambda}}}{words}]"),
        ] {
            let source = format!(
                "set done 0\n{callback}\nafter 50\nupdate\n\
                 if {{$done eq 0}} {{puts zero}} else {{puts changed}}\nputs $done\n"
            );
            prints_under_releases_from(&source, "changed\n1\n", "8.5");
        }
    }
}
