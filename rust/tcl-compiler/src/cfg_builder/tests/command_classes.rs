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

//! Registry-derived command classes: completion routes, custom registry traits and procedure-exit host commands.

use super::*;

/// The registry-derived commands which leave a procedure when they appear
/// as ordinary calls. Keep test data tied to traits rather than duplicating
/// command names alongside the production lookup.
fn proc_exit_host_commands() -> Vec<String> {
    let classes = CfgCommandClasses::for_dialect(Some("tcl9.0"));
    let mut commands: Vec<_> = classes
        .semantics
        .command_names_with_traits(Traits::TERMINATES_BLOCK)
        .filter(|name| {
            classes
                .semantics
                .command(name)
                .is_some_and(|facts| facts.lowering_hook() != Some(LoweringHookId::Return))
        })
        .chain(
            classes
                .semantics
                .command_names_with_traits(Traits::REPLACES_FRAME),
        )
        .map(str::to_owned)
        .collect();
    commands.sort_unstable();
    commands.dedup();
    commands
}

fn host_tokens(command: &str) -> CommandTokens {
    let registry = native_registry();
    let mut lowerer = crate::lowering::Lowerer::new(&registry);
    let module = lowerer.lower(command);
    module.top_level.statements[0]
        .tokens()
        .expect("real source invocation proof")
        .clone()
}

fn host_call(command: String) -> Statement {
    let tokens = host_tokens(&command);
    Statement::Call {
        span: Span::new(0, u32::try_from(command.len()).unwrap()),
        command,
        canonical_command: None,
        args: Vec::new(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: Some(tokens),
        foreach_groups: None,
    }
}

fn host_barrier(command: String) -> Statement {
    let tokens = host_tokens(&command);
    Statement::Barrier {
        span: Span::new(0, u32::try_from(command.len()).unwrap()),
        reason: "opaque host command".into(),
        command,
        canonical_command: None,
        args: Vec::new(),
        tokens: Some(tokens),
    }
}

fn build_analysis_cfg(name: &str, script: &Script) -> Function {
    CfgBuilder::new(true, tcl_registry::default_registry())
        .with_faithful_exceptions()
        .build_function(name, script)
}

fn proc_exit_barrier_switch(mode: SwitchMode) -> Script {
    let commands = proc_exit_host_commands();
    let arms = commands
        .iter()
        .enumerate()
        .map(|(i, command)| SwitchArm {
            pattern: i.to_string(),
            pattern_braced: true,
            pattern_span: Span::new(0, 1),
            // Exercise absolute spellings inside both flattened and opaque
            // switch paths; classification must normalise the `::` prefix.
            body: Some(Script::from_statements(vec![host_barrier(format!(
                "::{command}"
            ))])),
            body_span: Some(Span::new(0, 1)),
            fallthrough: false,
        })
        .collect();
    Script::from_statements(vec![Statement::Switch {
        subject_braced: false,
        raw_arg_braced: Vec::new(),
        raw_arg_quoted: Vec::new(),
        command: "switch".into(),
        span: Span::new(0, 1),
        subject: "$which".into(),
        subject_span: Span::new(0, 1),
        arms,
        default_body: Some(Script::from_statements(vec![host_barrier(format!(
            "::{}",
            commands[0]
        ))])),
        default_span: Some(Span::new(0, 1)),
        mode,
        nocase: false,
        raw_args: Vec::new(),
        patterns_braced: true,
    }])
}

/// Drift guard: the registry-derived classification sets must equal the
/// name lists pinned here, so a trait stamping change is a conscious
/// CFG-shape decision rather than a silent one.
#[test]
fn registry_derived_cfg_classes_match_previous_hardcodes() {
    fn sorted(classes: &CfgCommandClasses, traits: Traits) -> Vec<&str> {
        let mut v: Vec<&str> = classes
            .semantics
            .command_names_with_traits(traits)
            .collect();
        v.sort_unstable();
        v
    }
    let classes = CfgCommandClasses::for_dialect(Some("tcl9.0"));
    // `return` carries TERMINATES_BLOCK but lowers to
    // `Statement::Return`, so the name-keyed terminator set excludes it.
    let mut terminates = sorted(&classes, Traits::TERMINATES_BLOCK);
    terminates.retain(|name| {
        classes
            .semantics
            .command(name)
            .is_some_and(|facts| facts.lowering_hook() != Some(LoweringHookId::Return))
    });
    assert_eq!(terminates, ["error", "exit", "throw"]);
    assert_eq!(
        sorted(&classes, Traits::CATCHABLE_THROW),
        ["error", "throw"]
    );
    assert_eq!(sorted(&classes, Traits::REPLACES_FRAME), ["tailcall"]);
    assert_eq!(sorted(&classes, Traits::BREAKS_LOOP), ["break"]);
    assert_eq!(sorted(&classes, Traits::CONTINUES_LOOP), ["continue"]);

    let tcl84 = CfgCommandClasses::for_dialect(Some("tcl8.4"));
    assert!(
        !tcl84.is_block_terminating_command("throw")
            && !tcl84
                .semantics
                .command("throw")
                .is_some_and(|facts| facts.has_traits(Traits::CATCHABLE_THROW)),
        "Tcl 8.4 must select its visible spec set, not inherit later `throw` traits"
    );
}

#[test]
fn observed_completion_never_manufactures_a_procedure_return() {
    use tcl_registry::completion::CompletionCode;
    use tcl_registry::completion_route::InvocationCompletionRoute as Route;
    let registry = native_registry().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    for (source, expected) in [
        ("return -level 0 VALUE; set reached 1", None),
        ("return -level $level VALUE; set reached 1", None),
        (
            "return -code error -level 0 BOOM; set reached 1",
            Some(Route::Tcl(CompletionCode::Error)),
        ),
        ("exit 0; set reached 1", Some(Route::ProcessExit)),
    ] {
        let module = crate::lowering::lower_to_ir(source, &registry);
        let cfg = build_cfg_with_registry(&module, false, &registry);
        let entry = &cfg.top_level.blocks[&cfg.top_level.entry];
        if let Some(expected) = expected {
            assert!(
                matches!(entry.terminator, Some(Terminator::Complete {route, ..}) if route == expected),
                "{source}: {entry:?}"
            );
        } else {
            assert!(
                !matches!(
                    entry.terminator,
                    Some(Terminator::Complete { .. } | Terminator::Return { .. })
                ),
                "{source}: {entry:?}"
            );
            assert!(entry.statements.iter().any(|statement| matches!(statement,
                    Statement::AssignConst {name, ..} | Statement::AssignValue {name, ..} if name == "reached")), "{source}: {entry:?}");
        }
    }
}

#[test]
fn whole_module_cfg_uses_the_exact_custom_registry_traits() {
    let mut registry = native_registry();
    let mut custom = registry.get("puts").expect("puts spec").clone();
    custom.traits |= Traits::TERMINATES_BLOCK;
    custom.completion = Some(tcl_registry::CompletionDescriptor::exact(&[
        tcl_registry::CompletionCode::Error,
    ]));
    registry.insert(custom);

    let module = crate::lowering::lower_to_ir("puts before\nset reached 1", &registry);
    let cfg = build_cfg_with_registry(&module, false, &registry);
    let entry = &cfg.top_level.blocks[&cfg.top_level.entry];
    assert!(
        matches!(
            entry.terminator,
            Some(Terminator::Complete {
                route: tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::CompletionCode::Error
                ),
                ..
            })
        ),
        "the custom registry's completion must shape the whole-module CFG: {entry:?}"
    );
    assert!(
        cfg.top_level.blocks.values().any(|block| {
            block.name.starts_with("unreachable")
                && block.statements.iter().any(|statement| {
                    matches!(statement, Statement::AssignConst { name, .. } if name == "reached")
                        || matches!(statement, Statement::Call { command, args, .. }
                                if command == "set" && args == &["reached", "1"])
                })
        }),
        "the statement after the custom terminator must be unreachable"
    );
}

/// The classification helpers keep each site's own name
/// normalisation: the terminator / throw / tailcall checks trim
/// leading `:` runs (so canonical `::error` classifies), while
/// [`CfgBuilder::lower_loop_jump`] matches the raw word (so
/// `::break` stays a plain call there).
#[test]
fn cfg_class_helpers_keep_site_normalisation() {
    let classes = CfgCommandClasses::for_dialect(Some("tcl9.0"));
    assert!(classes.is_block_terminating_command("error"));
    assert!(classes.is_block_terminating_command("::throw"));
    assert!(classes.is_block_terminating_command("exit"));
    assert!(!classes.is_block_terminating_command("return"));
    assert!(!classes.is_block_terminating_command("break"));
    assert!(
        classes
            .semantics
            .command("error")
            .is_some_and(|facts| facts.has_traits(Traits::CATCHABLE_THROW))
    );
    assert!(
        !classes
            .semantics
            .command("exit")
            .is_some_and(|facts| facts.has_traits(Traits::CATCHABLE_THROW))
    );
    assert!(classes.is_tailcall_command("::tailcall"));
    assert!(!classes.is_tailcall_command("error"));
    assert!(classes.is_loop_break_command("break"));
    assert!(!classes.is_loop_break_command("::break"));
    assert!(classes.is_loop_continue_command("continue"));
    assert!(!classes.is_loop_continue_command("break"));
}

#[test]
fn opaque_host_barriers_match_calls_for_every_registry_proc_exit_command() {
    // This is deliberately registry-driven: today the traits yield
    // `error`, `exit`, `throw`, and `tailcall`, but a trait change must
    // extend this test automatically. Test both source spellings the CFG
    // accepts, including the global-qualified form.
    for command in proc_exit_host_commands() {
        for spelling in [command.clone(), format!("::{command}")] {
            for (kind, statement) in [
                ("Call", host_call(spelling.clone())),
                ("Barrier", host_barrier(spelling.clone())),
            ] {
                assert_eq!(
                    flow_facts_stmt(&statement).1,
                    Completion::ProcExit,
                    "{kind} {spelling} must leave the procedure",
                );

                let script = Script::from_statements(vec![
                    statement,
                    Statement::AssignConst {
                        span: Span::new(2, 3),
                        name: "dead".into(),
                        name_braced: false,
                        value: "1".into(),
                        value_span: None,
                    },
                ]);
                let func = build_analysis_cfg("::test", &script);
                let entry = &func.blocks[&func.entry];
                assert_eq!(
                    entry.statements.len(),
                    1,
                    "{kind} {spelling} must keep following statements unreachable",
                );
                assert!(
                    matches!(entry.terminator, Some(Terminator::Complete { .. })),
                    "{kind} {spelling} must terminate its CFG block, got {:?}",
                    entry.terminator,
                );
            }
        }
    }
}

#[test]
fn proc_exit_host_barriers_terminate_direct_and_opaque_switch_forms() {
    // Exact switches lower each arm directly; glob switches remain an
    // opaque Statement::Switch. In both representations, barriers whose
    // host command is a registry-described proc exit must retain the same
    // completion fact as a Call.
    let commands = proc_exit_host_commands();
    let direct = build_analysis_cfg("::direct", &proc_exit_barrier_switch(SwitchMode::Exact));
    assert!(matches!(
        direct.blocks[&direct.entry].terminator,
        Some(Terminator::Branch { .. })
    ));
    let direct_barriers: Vec<_> = direct
        .blocks
        .values()
        .filter(|block| {
            block
                .statements
                .iter()
                .any(|stmt| matches!(stmt, Statement::Barrier { .. }))
        })
        .collect();
    assert_eq!(direct_barriers.len(), commands.len() + 1);
    assert!(
        direct_barriers
            .iter()
            .all(|block| { matches!(block.terminator, Some(Terminator::Complete { .. })) })
    );

    let opaque = build_analysis_cfg("::opaque", &proc_exit_barrier_switch(SwitchMode::Glob));
    let entry = &opaque.blocks[&opaque.entry];
    assert!(matches!(
        entry.statements.as_slice(),
        [Statement::Switch { .. }]
    ));
    assert!(
        matches!(entry.terminator, Some(Terminator::Complete { .. })),
        "an all-proc-exit opaque switch must not fall through, got {:?}",
        entry.terminator,
    );
}
