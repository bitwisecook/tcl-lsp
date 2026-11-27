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

//! The dispatch shape extracted from a user-defined `unknown` procedure.

use super::*;

fn param(name: &str) -> ParamDef {
    ParamDef {
        name: name.to_string(),
        has_default: false,
        default_value: None,
    }
}

/// Depth coverage: `walk_unknown_stmt` recurses once per nested
/// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`/`Block`/`UpFrame`
/// body, with no depth cap of its own. Transitively bounded to
/// `MAX_LOWER_NEST_DEPTH`
/// (256) by the lowering pass today, so this is defence-in-depth /
/// consistency with every other full-tree walker in this crate, not a
/// currently-reproducible crash. 1000 levels of *source* nesting is
/// comfortably past this new cap; the assertion is that
/// `extract_unknown_proc_info` returns at all, not what it returns.
/// Spawns its own big-stack thread since the lexer/CST/segmenter
/// stages upstream of `walk_unknown_stmt`'s own new cap still walk the
/// full un-truncated source nesting before lowering's cap trims it —
/// same rationale as `structured::tests::deeply_nested_if_survives_structured_walk`.
#[test]
fn extract_unknown_proc_info_survives_deep_nesting() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut body = String::new();
    for _ in 0..DEPTH {
        body.push_str("if {1} {\n");
    }
    body.push_str("exec ls\n");
    for _ in 0..DEPTH {
        body.push_str("}\n");
    }
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let mut a = Analyser::new();
            let _ = a.extract_unknown_proc_info(&body, &[param("cmd"), param("args")]);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn extract_unknown_proc_info_empty_body_marks_empty_stub() {
    let mut a = Analyser::new();
    let info = a.extract_unknown_proc_info("", &[param("cmd"), param("args")]);
    assert!(info.empty_stub);
    assert_eq!(info.dispatch_targets.len(), 0);
}

#[test]
fn extract_unknown_proc_info_whitespace_body_marks_empty_stub() {
    let mut a = Analyser::new();
    let info = a.extract_unknown_proc_info("   \n  \t ", &[param("cmd"), param("args")]);
    assert!(info.empty_stub);
}

fn logical_unknown_input(
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> super::super::super::ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    super::super::super::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    )
}

fn logical_unknown_analyser() -> Analyser {
    let input = logical_unknown_input(tcl_registry::model::ingress::context_for_profile(
        tcl_dialect::DialectProfile::plain_tcl(),
    ));
    assert!(input.has_logical_source_name_context());
    Analyser::new().with_resolved_input(input)
}

#[test]
fn original_unknown_source_context_requires_retained_logical_input_and_current_registry() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    let body = "switch -exact $cmd {foo {return 1}}";
    let mut missing = Analyser::new();
    assert!(
        missing
            .extract_unknown_proc_info(body, &[param("cmd")])
            .dispatch_targets
            .is_empty()
    );
    let input = logical_unknown_analyser().resolved_input.unwrap();
    let options =
        crate::command_binding::SourceAnalysisOptions::for_logical_source(&input).unwrap();
    let context = input.context_registry();
    let mut body_builder =
        crate::lowering::Lowerer::with_config(context.commands(), input.lexer_config());
    body_builder.set_source_analysis_options(options);
    let lowered = body_builder.lower(body);
    let statement = lowered.top_level.statements.first().unwrap();
    let binding = lowered
        .top_level
        .retained_source_tokens_for_statement(statement)
        .unwrap()
        .source_binding
        .as_ref()
        .unwrap();
    assert_eq!(binding.logical_source_name_advice_input(), Some(&input));
    assert!(matches!(
        statement,
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
    // The possible source bodies do not settle the unknown option branch.
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        context.commands(),
        &input,
    )
    .unwrap();
    let case =
        crate::registry_invocation::source_structure::original_logical_source_case_for_tokens(
            lowered
                .top_level
                .retained_source_tokens_for_statement(statement)
                .unwrap(),
            context.commands(),
            metadata,
        )
        .unwrap();
    assert!(case.body_operands().selection_unknown);
    let mut logical = logical_unknown_analyser();
    assert!(
        logical
            .extract_unknown_proc_info(body, &[param("cmd")])
            .dispatch_targets
            .contains("foo")
    );
    let mut logical = logical_unknown_analyser();
    // Retired analyser registry labels cannot replace the retained input.
    logical.registry = Some(
        tcl_registry::model::ingress::static_context_for("f5-irules")
            .commands()
            .snapshot()
            .shared_registry(),
    );
    assert!(
        logical
            .extract_unknown_proc_info(body, &[param("cmd")])
            .dispatch_targets
            .contains("foo")
    );
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jimtcl",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
    ] {
        let environment = tcl_registry::model::ingress::resolve_environment(dialect);
        let profile = environment.unit_profile();
        let input = super::super::super::ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            profile,
            environment.default_context_registry(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        assert!(!input.has_logical_source_name_context(), "{dialect}");
        let mut native = Analyser::new().with_resolved_input(input);
        assert!(
            native
                .extract_unknown_proc_info(body, &[param("cmd")])
                .dispatch_targets
                .is_empty(),
            "{dialect}"
        );
    }
}

#[test]
fn original_unknown_case_advice_retains_residuals_in_final_info() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    // Possible source dispatch keeps its selection/error and no-match
    // paths; it supplies no Native occupancy, chosen effect or body entry.
    let mut analyser = logical_unknown_analyser();
    let result = analyser.analyse(
        "proc unknown {cmd args} {switch -exact $cmd {café {return 1} default {return 0}}}",
        "tcl",
    );
    let info = result
        .unknown_proc_info
        .expect("original unknown declaration");
    assert!(info.dispatch_targets.contains("café"));
    assert!(!info.dispatch_targets.contains("default"));
    assert!(!info.has_pattern_dispatch);
    let [residual] = info.case_body_residuals.as_slice() else {
        panic!("the final AnalysisResult must retain its canonical case residual")
    };
    assert!(residual.selection_unknown);
    assert!(!residual.no_match_possible);
    assert_eq!(residual.bodies.len(), 2);
    assert_eq!(residual.patterns.len(), 2);

    let input = logical_unknown_analyser().resolved_input.unwrap();
    let context = input.context_registry();
    let closed = "switch -exact -- $cmd {foo {return 1} default {return 0}}";
    let mut lowerer =
        crate::lowering::Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
    let script = lowerer.lower(closed).top_level.clone();
    assert!(matches!(
        script.statements.first(),
        Some(Statement::Switch { .. })
    ));
    let mut analyser = Analyser::new().with_resolved_input(input);
    let info = analyser.extract_unknown_proc_info(closed, &[param("cmd")]);
    assert!(info.dispatch_targets.contains("foo"));
    assert_eq!(info.case_body_residuals.len(), 1);
    assert!(!info.case_body_residuals[0].selection_unknown);
    assert!(!info.case_body_residuals[0].no_match_possible);
}

#[test]
fn original_unknown_case_advice_keeps_captured_ordinals_and_literal_parameter_names() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    let source = "interp alias {} ::choisiré {} switch -exact; ::choisiré $cmd {é {return 1} default {return 0}}";
    let mut analyser = logical_unknown_analyser();
    let info = analyser.extract_unknown_proc_info(source, &[param("cmd")]);
    assert!(info.dispatch_targets.contains("é"));
    let [residual] = info.case_body_residuals.as_slice() else {
        panic!("the captured selector retains a case residual")
    };
    assert!(residual.selection_unknown);
    assert_eq!(residual.bodies[0].argument, 2);
    assert_eq!(residual.bodies[0].list_element, Some(1));
    assert_eq!(residual.patterns[0].argument, 2);
    assert_eq!(residual.patterns[0].list_element, Some(0));

    for source in [
        "switch -exact $cmdSuffix {foo {return 1}}",
        "switch -exact {$cmd} {foo {return 1}}",
        "switch -exact $cmd(index) {foo {return 1}}",
    ] {
        let info = analyser.extract_unknown_proc_info(source, &[param("cmd")]);
        assert!(info.dispatch_targets.is_empty(), "{source}");
        assert!(info.case_body_residuals.is_empty(), "{source}");
    }
    let info = analyser.extract_unknown_proc_info(
        "switch -exact ${$cmd} {literal {return 1}}",
        &[param("$cmd")],
    );
    assert!(info.dispatch_targets.contains("literal"));
}

fn original_case_availability_contexts() -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
) {
    use std::sync::Arc;
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut spec = registry.get("switch").unwrap().clone();
    spec.name = "current_case";
    spec.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(spec);
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(current.commands())),
    );
    assert!(Arc::ptr_eq(current.commands(), older.commands()));
    (current, older)
}

#[test]
fn original_unknown_case_advice_refuses_missing_foreign_changed_and_shadowed_owners() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    use crate::registry_invocation::{
        InvocationMetadataContext, source_structure::original_logical_source_case_for_tokens,
    };
    use std::sync::Arc;
    let (current, older) = original_case_availability_contexts();
    let input = logical_unknown_input(Arc::clone(&current));
    let profile = input.unit_profile();
    let config = input.lexer_config();
    let source = "interp alias {} selecté {} current_case -exact; selecté $cmd {foo {return 1}}";
    let mut analyser = Analyser::new().with_resolved_input(input.clone());
    let positive = analyser.extract_unknown_proc_info(source, &[param("cmd")]);
    assert!(positive.dispatch_targets.contains("foo"));
    assert!(positive.case_body_residuals[0].no_match_possible);
    let mut lowerer = crate::lowering::Lowerer::with_config(current.commands(), config)
        .with_resolved_analysis_input(input.clone());
    let script = lowerer.lower(source).top_level.clone();
    let tokens = script
        .retained_source_tokens_for_statement(script.statements.last().unwrap())
        .unwrap();
    let metadata =
        InvocationMetadataContext::for_analysis_input(current.commands(), &input).unwrap();
    assert!(
        original_logical_source_case_for_tokens(tokens, current.commands(), metadata).is_some()
    );
    assert!(
        original_logical_source_case_for_tokens(
            tokens,
            current.commands(),
            current.as_ref().into()
        )
        .is_none()
    );
    let foreign = logical_unknown_input(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
    );
    let mut changed_config = config;
    changed_config.strict_quoting = !changed_config.strict_quoting;
    let changed = super::super::super::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&current),
        changed_config,
    );
    for input in [&foreign, &changed] {
        let metadata = InvocationMetadataContext::for_analysis_input(
            input.borrowed_context_registry().commands(),
            input,
        )
        .unwrap();
        assert!(
            original_logical_source_case_for_tokens(tokens, current.commands(), metadata).is_none()
        );
    }
    let mut missing = tokens.clone();
    missing.source_binding = None;
    assert!(
        original_logical_source_case_for_tokens(&missing, current.commands(), metadata).is_none()
    );
    let mut altered = tokens.clone();
    let tcl_source::words::WordExpr::Variable { spelling, .. } = &mut altered.word_exprs[1] else {
        panic!("original written subject")
    };
    *spelling = "$other".into();
    assert!(
        original_logical_source_case_for_tokens(&altered, current.commands(), metadata).is_none()
    );
    let mut absent = Analyser::new();
    assert!(
        absent
            .extract_unknown_proc_info(source, &[param("cmd")])
            .dispatch_targets
            .is_empty()
    );
    let mut unavailable = Analyser::new().with_resolved_input(logical_unknown_input(older));
    assert!(
        unavailable
            .extract_unknown_proc_info(source, &[param("cmd")])
            .dispatch_targets
            .is_empty()
    );
    for source in [
        "proc current_case args {}; interp alias {} selecté {} current_case -exact; selecté $cmd {foo {return 1}}",
        "interp alias {} selecté {} current_case -exact; proc current_case args {}; selecté $cmd {foo {return 1}}",
        "current_case $option $cmd {foo {return 1}}",
    ] {
        assert!(
            analyser
                .extract_unknown_proc_info(source, &[param("cmd")])
                .dispatch_targets
                .is_empty(),
            "{source}"
        );
    }
}

#[test]
fn original_unknown_case_advice_selects_the_original_case_fold_child() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    let mut analyser = logical_unknown_analyser();
    let source =
        "interp alias {} foldé {} string tolower; switch -exact [foldé $cmd] {foo {return 1}}";
    let info = analyser.extract_unknown_proc_info(source, &[param("cmd")]);
    assert!(info.dispatch_targets.contains("foo"));
    assert!(info.case_insensitive);
    for source in [
        "proc string args {}; switch -exact [string tolower $cmd] {foo {return 1}}",
        "switch -exact [list {string tolower $cmd}] {foo {return 1}}",
        "switch -exact [string tolower $other] {foo {return 1}}",
    ] {
        let info = analyser.extract_unknown_proc_info(source, &[param("cmd")]);
        assert!(info.dispatch_targets.is_empty(), "{source}");
        assert!(!info.case_insensitive, "{source}");
    }
    let input = logical_unknown_analyser().resolved_input.unwrap();
    let context = input.context_registry();
    let mut lowerer =
        crate::lowering::Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
    let script = lowerer.lower(source).top_level.clone();
    let mut tokens = script
        .retained_source_tokens_for_statement(script.statements.last().unwrap())
        .unwrap()
        .clone();
    assert_ne!(
        *tokens.nested_bindings,
        [] as [(u32, crate::command_binding::SourceInvocationBinding); 0]
    );
    tokens.nested_bindings_mut().clear();
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        context.commands(),
        &input,
    )
    .unwrap();
    let case =
        crate::registry_invocation::source_structure::original_logical_source_case_for_tokens(
            &tokens,
            context.commands(),
            metadata,
        )
        .unwrap();
    assert!(!case.references_parameter("cmd"));
    assert!(!case.folds_case());
}

#[test]
fn original_unknown_native_case_advice_requires_typed_switch_and_original_input() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    // This fixture retains actual registration and compiler entry. The
    // readonly case query grants no successful variable read or body entry.
    use crate::registry_invocation::{
        InvocationMetadataContext, InvocationMetadataInput,
        source_structure::original_native_source_case_for_statement,
    };
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let profile = environment.unit_profile();
    let context = environment.default_context_registry();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = super::super::super::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        profile,
        context.clone(),
        config,
    );
    let (_owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let options = crate::command_binding::SourceAnalysisOptions {
        metadata_context: InvocationMetadataInput::SuppliedSource(Some(&input)),
        native_entry: Some(&captured),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
        .with_dialect(Some(profile))
        .with_resolved_analysis_input(input.clone());
    lowerer.set_source_analysis_options(options);
    let script = lowerer
        .lower("switch -exact -- $cmd {foo {return 1} default {return 0}}")
        .top_level
        .clone();
    let statement = script.statements.first().unwrap();
    assert!(matches!(statement, Statement::Switch { .. }));
    let metadata =
        InvocationMetadataContext::for_analysis_input(context.commands(), &input).unwrap();
    let case =
        original_native_source_case_for_statement(&script, statement, context.commands(), metadata)
            .expect("typed original Native case source");
    assert!(case.references_parameter("cmd"));
    assert_eq!(case.patterns(), &["foo"]);
    assert!(!case.body_operands().selection_unknown);
    assert!(!case.body_operands().no_match_possible);
    let mut info = UnknownProcInfo::default();
    walk_unknown_stmt(
        statement,
        &script,
        &context,
        Some(metadata),
        "cmd",
        &mut info,
        0,
    );
    assert!(info.dispatch_targets.contains("foo"));
    assert_eq!(info.case_body_residuals.len(), 1);
    assert!(
        original_native_source_case_for_statement(
            &script,
            statement,
            context.commands(),
            context.as_ref().into(),
        )
        .is_none()
    );
    let mut missing = script.clone();
    missing.command_binding_sites = crate::ir::CommandBindingSites::default();
    assert!(
        original_native_source_case_for_statement(
            &missing,
            &missing.statements[0],
            context.commands(),
            metadata,
        )
        .is_none()
    );
    let mut generic_lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
        .with_resolved_analysis_input(input.clone());
    let generic = generic_lowerer
        .lower("switch -exact -- $cmd {foo {return 1} default {return 0}}")
        .top_level
        .clone();
    let generic_statement = generic.statements.first().unwrap();
    assert!(!matches!(generic_statement, Statement::Switch { .. }));
    assert!(
        original_native_source_case_for_statement(
            &generic,
            generic_statement,
            context.commands(),
            metadata,
        )
        .is_none()
    );
}

#[test]
fn extract_unknown_proc_info_exact_switch_collects_dispatch_targets() {
    let mut a = logical_unknown_analyser();
    let body = r"switch -exact $cmd {
            foo { return 1 }
            bar { return 2 }
            default { return 0 }
        }";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(!info.empty_stub);
    assert!(info.dispatch_targets.contains("foo"));
    assert!(info.dispatch_targets.contains("bar"));
    assert!(!info.dispatch_targets.contains("default"));
    assert!(!info.has_pattern_dispatch);
}

#[test]
fn extract_unknown_proc_info_glob_switch_marks_pattern_dispatch() {
    let mut a = logical_unknown_analyser();
    let body = r"switch -glob $cmd {
            foo* { return 1 }
            *bar { return 2 }
        }";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(info.has_pattern_dispatch);
    assert_eq!(info.dispatch_targets.len(), 0);
}

#[test]
fn extract_unknown_proc_info_chains_original_via_known_target() {
    let mut a = logical_unknown_analyser();
    let body = r"rename unknown _original_unknown
_original_unknown $cmd $args";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(info.chains_original);
}

#[test]
fn unknown_handler_effects_require_the_live_implementation() {
    for (body, flag) in [
        ("proc exec args {}; exec $cmd", "exec"),
        ("proc auto_load args {}; auto_load $cmd", "load"),
        ("_original_unknown $cmd", "chain"),
        ("if {$condition} {rename exec {}}; exec $cmd", "exec"),
    ] {
        let mut analyser = logical_unknown_analyser();
        let info = analyser.extract_unknown_proc_info(body, &[param("cmd")]);
        let claimed = match flag {
            "exec" => info.has_exec,
            "load" => info.has_auto_load,
            _ => info.chains_original,
        };
        assert!(
            !claimed,
            "unproved implementation acquired builtin effects: {body}"
        );
    }
    let mut analyser = Analyser::new();
    let result = analyser.analyse(
        "proc exec args {}; proc unknown {cmd args} {exec $cmd}",
        "tcl8.6",
    );
    assert!(!result.unknown_proc_info.as_ref().unwrap().has_exec);
}

#[test]
fn unknown_handler_effects_follow_resolved_alias_arguments() {
    let mut analyser = logical_unknown_analyser();
    let info = analyser.extract_unknown_proc_info(
        "interp alias {} ensure {} package require; ensure Tcl",
        &[param("cmd")],
    );
    assert!(info.has_auto_load);
}

#[test]
fn original_unknown_operation_advice_keeps_actual_availability_and_captured_sources() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    // Operation descriptions are conditional source advice, without an
    // executed load, Native handler, cell, frame or Normal certificate.
    use std::sync::Arc;
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut loader = registry.get("package").unwrap().clone();
    loader.name = "context_package";
    loader.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    let mut subcommands = loader.subcommands.to_vec();
    subcommands
        .iter_mut()
        .find(|subcommand| subcommand.name == "require")
        .unwrap()
        .surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    loader.subcommands = Box::leak(subcommands.into_boxed_slice());
    registry.insert(loader);
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(current.commands())),
    );
    assert!(Arc::ptr_eq(current.commands(), older.commands()));
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = |context| {
        super::super::super::ResolvedAnalysisInput::new(profile, profile, context, config)
    };
    let source = "interp alias {} ::chargeré {} context_package require; ::chargeré Tcl";
    let mut analyser = Analyser::new().with_resolved_input(input(Arc::clone(&current)));
    assert!(
        analyser
            .extract_unknown_proc_info(source, &[param("cmd")])
            .has_auto_load
    );
    analyser.registry = Some(
        tcl_registry::model::ingress::static_context_for("f5-irules")
            .commands()
            .snapshot()
            .shared_registry(),
    );
    assert!(
        analyser
            .extract_unknown_proc_info(source, &[param("cmd")])
            .has_auto_load
    );
    let mut unavailable = Analyser::new().with_resolved_input(input(older));
    assert!(
        !unavailable
            .extract_unknown_proc_info(source, &[param("cmd")])
            .has_auto_load
    );
    let replaced = "proc context_package args {}; interp alias {} ensure {} context_package require; ensure Tcl";
    assert!(
        !analyser
            .extract_unknown_proc_info(replaced, &[param("cmd")])
            .has_auto_load
    );
}

fn assert_captured_logical_source_target(
    tokens: &crate::ir::CommandTokens,
    context: &tcl_registry::model::ContextRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
) {
    let input = metadata.source_analysis_input().unwrap();
    let original = tokens.source_binding.as_ref().unwrap();
    let target = original
        .proved_execution_target()
        .expect("closed Logical source target");
    assert!(target.registry_backed);
    let dialect = original
        .original_source_word_dialect_for_tokens(tokens)
        .expect("actual Logical source dialect");
    assert!(dialect.native_name_protocol().is_none());
    assert!(
        crate::command_binding::SourceAnalysisOptions::for_logical_source(input)
            .unwrap()
            .native_entry
            .is_none()
    );
    let selected =
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            context.commands(),
            metadata,
            tokens,
        )
        .unwrap();
    assert!(
        selected
            .with_metadata_schema(
                context.commands(),
                metadata,
                original.invocation_realm().unwrap(),
                |schema| {
                    let target_spec = context.context().resolve_spec_in_realm(
                        context.commands(),
                        &target.command,
                        original.invocation_realm().unwrap(),
                    )?;
                    Some(std::ptr::eq(
                        target_spec,
                        schema.authored_source_descriptors().command,
                    ))
                },
            )
            .unwrap()
    );
    assert_eq!(
        selected.evaluated_words[0].literal_bytes(),
        Some(b"require".as_slice())
    );
    assert_eq!(
        selected.effective.origins[1],
        crate::registry_invocation::InvocationWordOrigin::BindingPrefix(0)
    );
    assert_eq!(selected.effective.written_argument(1), Some(0));
}

#[test]
fn original_unknown_operation_advice_refuses_missing_foreign_changed_and_dynamic_source() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    use crate::registry_invocation::InvocationMetadataContext;
    let input = logical_unknown_analyser().resolved_input.unwrap();
    let context = input.context_registry();
    let source = "interp alias {} ensure {} package require; ensure Tcl";
    let mut lowerer =
        crate::lowering::Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
    let script = lowerer.lower(source).top_level.clone();
    let statement = script.statements.last().unwrap();
    let original = statement.tokens().unwrap().source_binding.as_ref().unwrap();
    assert!(original.original_recorded_head_name_input().is_none());
    let metadata = InvocationMetadataContext::for_source_input(
        context.commands(),
        &input,
        input.lexer_config(),
        Some(input.unit_profile()),
    )
    .unwrap();
    assert_captured_logical_source_target(statement.tokens().unwrap(), &context, metadata);
    let inspect = |statement: &Statement, metadata| {
        let mut info = UnknownProcInfo::default();
        walk_unknown_stmt(statement, &script, &context, metadata, "cmd", &mut info, 0);
        info
    };
    assert!(inspect(statement, Some(metadata)).has_auto_load);
    let foreign = super::super::super::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    );
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let changed = super::super::super::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    );
    for metadata in [
        None,
        Some(context.as_ref().into()),
        InvocationMetadataContext::for_analysis_input(
            foreign.borrowed_context_registry().commands(),
            &foreign,
        ),
        InvocationMetadataContext::for_analysis_input(context.commands(), &changed),
    ] {
        assert!(!inspect(statement, metadata).has_auto_load);
    }
    let mut missing = statement.clone();
    missing.tokens_mut().unwrap().source_binding = None;
    assert!(!inspect(&missing, Some(metadata)).has_auto_load);
    let mut analyser = logical_unknown_analyser();
    for source in [
        "interp alias {} ensure {} package $selector; ensure Tcl",
        "interp alias {} ensure {} package require; $unknown Tcl",
        "interp alias {} ensure {} package require; proc package args {}; ensure Tcl",
    ] {
        assert!(
            !analyser
                .extract_unknown_proc_info(source, &[param("cmd")])
                .has_auto_load,
            "{source}"
        );
    }
    assert!(
        !Analyser::new()
            .extract_unknown_proc_info(source, &[param("cmd")])
            .has_auto_load
    );
}

#[test]
fn original_unknown_operation_purpose_requires_the_complete_original_source_input() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    let input = logical_unknown_analyser().resolved_input.unwrap();
    let context = input.context_registry();
    let source = "interp alias {} ensure {} package require; ensure Tcl";
    let mut lowerer =
        crate::lowering::Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
    let script = lowerer.lower(source).top_level.clone();
    let statement = script.statements.last().unwrap();
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        context.commands(),
        &input,
    )
    .unwrap();
    let mut selected = UnknownProcInfo::default();
    walk_unknown_stmt(
        statement,
        &script,
        &context,
        Some(metadata),
        "cmd",
        &mut selected,
        0,
    );
    assert!(selected.has_auto_load);
    let native_profile =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let native_input = super::super::super::ResolvedAnalysisInput::new(
        native_profile,
        native_profile,
        std::sync::Arc::clone(&context),
        input.lexer_config(),
    );
    assert!(!native_input.has_logical_source_name_context());
    for metadata in [
        None,
        Some(context.as_ref().into()),
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            context.commands(),
            &native_input,
        ),
    ] {
        let mut refused = UnknownProcInfo::default();
        walk_unknown_stmt(
            statement,
            &script,
            &context,
            metadata,
            "cmd",
            &mut refused,
            0,
        );
        assert!(!refused.has_auto_load);
        assert!(!refused.has_exec);
        assert!(!refused.chains_original);
        assert!(refused.dispatch_targets.is_empty());
    }
}

#[test]
fn extract_unknown_proc_info_detects_exec_call() {
    let mut a = logical_unknown_analyser();
    let body = r"exec $cmd {*}$args";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(info.has_exec);
}

#[test]
fn extract_unknown_proc_info_detects_auto_load_call() {
    let mut a = logical_unknown_analyser();
    let body = r"auto_load $cmd";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(info.has_auto_load);
}

#[test]
fn extract_unknown_proc_info_case_insensitive_via_string_tolower() {
    let mut a = logical_unknown_analyser();
    let body = r"switch -exact [string tolower $cmd] {
            foo { return 1 }
        }";
    let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
    assert!(info.case_insensitive);
    assert!(info.dispatch_targets.contains("foo"));
}

#[test]
fn extract_unknown_proc_info_no_first_param_defaults_to_cmd() {
    // Empty params list — the helper should fall back to
    // ``"cmd"`` as the dispatch-subject variable name.
    let mut a = logical_unknown_analyser();
    let body = r"switch -exact $cmd { foo { return 1 } }";
    let info = a.extract_unknown_proc_info(body, &[]);
    assert!(info.dispatch_targets.contains("foo"));
}
