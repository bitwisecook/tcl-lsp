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

//! Calls and substitutions that reach code the module cannot see, and the markers and barriers they leave.

use super::*;

/// A top-level replay starts from the entry's package rows as it starts
/// from the entry's command rows. A package the module's own walk loaded
/// is not loaded before the document's first command, so `package require
/// Tk` installs the supplied Tk 8.6 provider's widget commands instead of
/// counting as satisfied with none of them bound. A package no loader
/// declares stays unseen. Tcl 8.6 (#2324).
#[test]
fn a_top_level_replay_loads_a_supplied_package_from_the_entry_state() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let unit = |source: &str| {
        let context = std::sync::Arc::new(
            crate::environment_ingress::context_for_profile(registry.profile().unwrap())
                .with_command_store(registry.snapshot().shared_registry()),
        );
        crate::compilation_unit::CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: &registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
                dialect: registry.profile(),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&crate::provider_fixtures::entry(
                &registry,
                &[crate::provider_fixtures::Provider::Tk],
            )),
            context,
        )
    };
    let unseen = |unit: &crate::compilation_unit::CompilationUnit| {
        unit.top_level
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .filter(|stmt| {
                stmt.tokens().is_some_and(|tokens| {
                    tokens.synthetic == Some(crate::ir::SyntheticMarker::UnseenCall)
                })
            })
            .count()
    };
    assert_eq!(
        unseen(&unit(
            "package require Tk\nset v 1\nttk::entry .f\nputs $v\n"
        )),
        0,
        "a supplied Tk provider binds package, ttk::entry and puts"
    );
    assert!(
        unseen(&unit("package require Mystery\nset v 1\nputs $v\n")) > 0,
        "a package no loader declares runs code the module cannot see"
    );
}

/// The marker for where code the module cannot see runs, and the barrier
/// for a command that may write any name, stand ahead of the opaque
/// `catch` call whose body runs it: at the top level a plain name is the
/// global the code reaches, in a procedure a local is in the reach of a
/// callee that runs `upvar 1`, and a sourced file runs in the frame of the
/// call.
#[test]
fn an_opaque_catch_marks_the_unseen_code_and_the_barrier_its_body_runs() {
    let has_marker = |func: &Function, marker: crate::ir::SyntheticMarker| {
        func.blocks
            .values()
            .flat_map(|block| &block.statements)
            .any(|stmt| {
                matches!(
                    stmt,
                    Statement::Call { tokens: Some(tokens), .. }
                        | Statement::Barrier { tokens: Some(tokens), .. }
                        if tokens.synthetic == Some(marker)
                )
            })
    };
    let top = |src: &str| build_cfg(&lower_module(src), false).top_level;
    let in_proc = |src: &str| {
        let module = lower_module(&format!("proc p {{}} {{\n{src}\n}}"));
        build_cfg(&module, false)
            .procedures
            .get(&fixture_id(&module.procedures, "::p"))
            .expect("authored fixture body")
            .clone()
    };
    for src in [
        "catch {foo}",
        "catch {foo} msg",
        "catch { if {1} { foo } }",
        "catch { puts [foo] }",
        "catch { source other.tcl }",
        "if {[catch {foo}]} { puts bad }",
        "set rc [catch {foo} msg]",
        "puts [catch { puts [foo] }]",
        "while {[catch {if {1} {foo}}]} { break }",
        "set rc [catch $script]",
    ] {
        let func = top(src);
        assert!(
            has_marker(&func, crate::ir::SyntheticMarker::UnseenCall),
            "{src}"
        );
    }
    // An autoloaded or unknown-handled callee reaches a procedure's local
    // through `upvar 1`; a sourced file runs in its frame.
    for src in [
        "catch { if {1} { foo } }",
        "if {[catch {foo}]} { puts bad }",
        "set rc [catch {foo} msg]",
        "puts [catch { puts [foo] }]",
    ] {
        assert!(
            has_marker(&in_proc(src), crate::ir::SyntheticMarker::UnseenCall),
            "proc: {src}"
        );
    }
    assert!(has_marker(
        &in_proc("if {[catch {source other.tcl}]} { puts bad }"),
        crate::ir::SyntheticMarker::UnseenCall
    ));
    assert!(has_marker(
        &in_proc("catch { if {1} { source other.tcl } }"),
        crate::ir::SyntheticMarker::UnseenCall
    ));
    for (func, src) in [
        (top("catch { namespace eval :: {set go 0} }"), "top"),
        (
            in_proc("catch { if {1} { namespace eval :: {set go 0} } }"),
            "proc",
        ),
        (
            in_proc("catch { if {1} { dict with d { set go 0 } } }"),
            "dict with",
        ),
    ] {
        assert!(
            has_marker(&func, crate::ir::SyntheticMarker::CallerFrameOpaque),
            "{src}"
        );
    }
    // A body that writes another name and runs only commands the module
    // knows states no marker.
    let func = top("catch { if {1} { set other 1 } }");
    assert!(!has_marker(&func, crate::ir::SyntheticMarker::UnseenCall));
    assert!(!has_marker(
        &func,
        crate::ir::SyntheticMarker::CallerFrameOpaque
    ));
}

/// A loop that only writes variables leaves the command table it found:
/// the commands after it are the module's own, with no unseen code
/// (#2523, #2531).
#[test]
fn a_loop_that_writes_only_variables_keeps_the_commands_after_it_named() {
    let markers = |src: &str| -> Vec<Statement> {
        build_cfg(&lower_module(src), false)
            .top_level
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .filter(|stmt| {
                stmt.tokens().is_some_and(|tokens| {
                    matches!(
                        tokens.synthetic,
                        Some(
                            crate::ir::SyntheticMarker::UnseenCall
                                | crate::ir::SyntheticMarker::CallerFrameOpaque
                        )
                    )
                })
            })
            .cloned()
            .collect()
    };
    for src in [
        "for {set i 0} {$i < 10} {incr i} {}\nproc p {} { set i 0; incr i }\n",
        "set i 0\nwhile {$i < 5} {incr i}\nset r [lindex {a b} $i]\n",
        "foreach x {1 2} {set y $x}\nputs $y\n",
    ] {
        let found = markers(src);
        assert!(found.is_empty(), "{src}: {found:?}");
    }
    // A loop that rebinds a command leaves the commands after it unknown.
    assert_ne!(
        markers("foreach n {a b} {proc puts args {}}\nputs hi\n"),
        []
    );
}

/// A body runs once its procedure is invoked, so a call to a procedure the
/// file defines later is the module's own when no statement between the two
/// definitions can invoke the caller (#2407). A call at top level before
/// the callee's definition, or a definition a loop may reach again, keeps
/// the call unseen.
#[test]
fn a_forward_call_in_a_body_nothing_runs_early_is_named() {
    let unseen_in = |src: &str, name: &str| {
        let cfg = build_cfg(&lower_module(src), false);
        calls_in_order(
            cfg.procedures
                .get(&fixture_id(&cfg.procedures, name))
                .expect("authored fixture body"),
        )
        .iter()
        .any(|(command, _)| *command == "<unseen-call>")
    };
    let twice = "proc twice {name} {upvar 1 $name w; bump w; bump w}\n";
    let bump = "proc bump {name} {upvar 1 $name v; incr v}\n";
    for between in ["", "set x 0\n", "variable y 1\n"] {
        let src = format!("{twice}{between}{bump}proc c {{}} {{set x 0; twice x; return $x}}\n");
        assert!(!unseen_in(&src, "::twice"), "{src}");
    }
    for src in [
        format!("{twice}set x 0\ntwice x\n{bump}"),
        format!("{twice}puts hi\n{bump}"),
        format!("foreach i {{1 2}} {{\n{twice}}}\n{bump}"),
    ] {
        assert!(unseen_in(&src, "::twice"), "{src}");
    }
}

/// The marker for a call whose own head runs code the module cannot see
/// follows the call and carries nothing: the names the call's words read,
/// which may be rewritten as it runs, are the SSA's to state.
#[test]
fn the_marker_for_a_call_follows_it_and_states_no_name() {
    for (src, host) in [
        ("set cmd puts\n$cmd hi", "${cmd}"),
        ("set suffix _hi\nfoo$suffix", "foo${suffix}"),
        ("set a 1\nget $a", "get"),
        ("set f x.tcl\nsource $f", "source"),
    ] {
        let cfg = build_cfg(&lower_module(src), false);
        let calls = calls_in_order(&cfg.top_level);
        assert_eq!(
            calls,
            [(host, &[][..]), ("<unseen-call>", &[][..])],
            "{src}"
        );
    }
}

/// A computed head runs whatever its value names, so it is code the module
/// cannot see, as a literal head it does not define is, at the top level
/// and in a procedure body alike.
#[test]
fn a_computed_head_is_an_unseen_call() {
    let unseen = |func: &Function| {
        calls_in_order(func)
            .iter()
            .any(|(command, _)| *command == "<unseen-call>")
    };
    for src in [
        "$cmd",
        "$cmd arg",
        "puts [$cmd]",
        "set x [$cmd]",
        "if {[$cmd]} { puts a }",
    ] {
        assert!(
            unseen(&build_cfg(&lower_module(src), false).top_level),
            "{src}"
        );
    }
    for src in ["$cmd", "puts [$cmd]", "set x [$cmd]"] {
        let module = lower_module(&format!("proc p {{}} {{\n{src}\n}}"));
        let cfg = build_cfg(&module, false);
        assert!(
            unseen(
                cfg.procedures
                    .get(&fixture_id(&cfg.procedures, "::p"))
                    .expect("authored fixture body")
            ),
            "proc: {src}"
        );
    }
}

/// A command the document declares as a plain call that states its frame
/// effect is one the module can name: under `-frame own` and `-frame none`
/// its call, direct or in a substitution, marks nothing, and under
/// `-frame caller` it brings `argparse`'s caller-frame effect to the
/// computed-name walk. A declaration that states no frame effect, or
/// names a word the flow graph would have to follow, stays a call to code
/// the module cannot see; one with a body word lowers to the barrier
/// `time {…}` lowers to, whatever its frame effect.
#[test]
fn a_declared_plain_call_brings_the_frame_effect_it_states() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let build = |stub: &str, body: &str| {
        let src = format!(
            "# tcl-lsp: stubs-begin\n# tcl-lsp: stub {stub}\n# tcl-lsp: stubs-end\n\
                 proc p {{}} {{\n    set g 5\n    {body}\n    puts $g\n}}\n"
        );
        let declared = crate::analyser::utils::document_declared_surface(&src, None, "tcl8.6");
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(registry, config)
                .with_dialect(registry.profile())
                .with_declared_commands(Some(&declared)),
            &src,
        );
        build_cfg_with_registry(&module, false, registry)
            .procedures
            .get(&fixture_id(&module.procedures, "::p"))
            .expect("authored fixture body")
            .clone()
    };
    let unseen = |func: &Function| {
        calls_in_order(func)
            .iter()
            .any(|(command, _)| *command == "<unseen-call>")
    };
    let writes =
        |func: &Function| crate::dynamic_names::dynamic_name_barrier(func, registry, config).writes;
    for stub in [
        "db_query {sql} -frame own",
        "db_query {sql:pattern} -pure -frame none",
    ] {
        for body in ["db_query {select 1}", "set n [db_query {select 1}]"] {
            let func = build(stub, body);
            assert!(!unseen(&func), "{stub}: {body}");
            assert!(!writes(&func), "{stub}: {body}");
            assert_eq!(
                func.declared_frame_effects.get("::db_query"),
                Some(&None),
                "{stub}"
            );
        }
    }
    for body in ["db_query {select 1}", "set n [db_query {select 1}]"] {
        let func = build("db_query {sql} -frame caller", body);
        assert!(!unseen(&func), "caller: {body}");
        assert!(writes(&func), "caller: {body}");
    }
    for stub in [
        "db_query {sql}",
        "db_query {sql} -frame bogus",
        "db_query {row:var} -frame own",
        "db_query {sql:expr} -frame own",
        "db_query {sql} -barrier -frame own",
        "db_query {sql} -extension -frame own",
    ] {
        // #2427: an autoload index entry keeps the undeclared `db_query` unseen.
        let func = build(
            stub,
            "set ::auto_index(db_query) {source lib.tcl}; db_query {select 1}",
        );
        assert!(unseen(&func), "{stub}");
        assert!(func.declared_frame_effects.is_empty(), "{stub}");
    }
    let barrier_reason = |func: &Function, head: &str| {
        func.blocks
            .values()
            .flat_map(|block| &block.statements)
            .find_map(|stmt| match stmt {
                Statement::Barrier {
                    command, reason, ..
                } if command == head => Some(reason.clone()),
                _ => None,
            })
    };
    let registry_body = build("db_query {sql} -frame own", "time {set x 1}");
    assert!(barrier_reason(&registry_body, "time").is_some());
    for stub in [
        "db_eval {sql script:body} -frame own",
        "db_eval {sql script:body} -frame none",
        "db_eval {sql script:body}",
        "db_eval {sql cb:command_prefix} -frame own",
    ] {
        let declared_body = build(stub, "db_eval {select 1} {set x 1}");
        assert_eq!(
            barrier_reason(&declared_body, "db_eval"),
            barrier_reason(&registry_body, "time"),
            "{stub}"
        );
        assert!(declared_body.declared_frame_effects.is_empty(), "{stub}");
    }
}

/// A substitution's command reaches code the module cannot see through every
/// word it runs as code: a lambda's body, a body that runs in another frame
/// (`uplevel`, `namespace eval`), the commands in a `subst` text and in an
/// expression word of a body, as through a body of this frame, in a
/// procedure as at the top level. A body that holds no such code marks
/// nothing.
#[test]
fn a_substitution_marks_the_unseen_code_in_every_word_its_command_runs() {
    let unseen = |func: &Function| {
        calls_in_order(func)
            .iter()
            .any(|(command, _)| *command == "<unseen-call>")
    };
    for src in [
        "set x [apply {{} {foo}}]",
        "set x [apply {{x} {foo $x}} 1]",
        "set x [apply {{} {if {1} {foo}}}]",
        "set x [namespace eval ns {foo}]",
        "set x [namespace eval ns {namespace eval inner {foo}}]",
        "set x [uplevel #0 {foo}]",
        "set x [subst {[foo]}]",
        "set x [subst -nobackslashes {a [foo] b}]",
        "set x [catch {apply {{} {foo}}}]",
        "set x [catch {if {[foo]} {puts a}}]",
        "set x [catch {while {[foo]} {puts a}}]",
        "set x [catch {expr {[foo] + 1}}]",
        "set x [apply {{} {if {[foo]} {puts a}}}]",
        "puts [apply {{} {foo}}]",
        "if {[apply {{} {foo}}]} {puts a}",
        "set x [apply $lambda]",
        "set x [namespace eval ns $script]",
    ] {
        assert!(
            unseen(&build_cfg(&lower_module(src), false).top_level),
            "{src}"
        );
    }
    for src in [
        "set x [apply {{} {set y 1}}]",
        "set x [apply {{x} {set y $x}} 1]",
        "set x [namespace eval ns {set y 1}]",
        "set x [uplevel #0 {set y 1}]",
        "set x [subst {abc}]",
        "set x [subst {[set y 1]}]",
        "set x [subst -nocommands {a [foo] b}]",
        "set x [catch {apply {{} {set y 1}}}]",
        "set x [catch {if {[set y 1]} {puts a}}]",
        "set x [apply {{} {if {[set y 1]} {puts a}}}]",
    ] {
        assert!(
            !unseen(&build_cfg(&lower_module(src), false).top_level),
            "{src}"
        );
    }
    // A callee the code reaches runs `upvar 1` into a procedure's frame too.
    for src in [
        "set x [apply {{} {foo}}]",
        "set x [namespace eval ns {foo}]",
        "set x [subst {[foo]}]",
    ] {
        let module = lower_module(&format!("proc p {{}} {{\n{src}\n}}"));
        let cfg = build_cfg(&module, false);
        assert!(
            unseen(
                cfg.procedures
                    .get(&fixture_id(&cfg.procedures, "::p"))
                    .expect("authored fixture body")
            ),
            "proc: {src}"
        );
    }
}
