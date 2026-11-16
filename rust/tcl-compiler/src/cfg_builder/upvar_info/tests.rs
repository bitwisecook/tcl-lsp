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
use crate::ir::Script;
use tcl_registry::CommandRegistry;

fn lower(src: &str) -> Script {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("test native core");
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    crate::lowering::lower_proc_body_isolated(
        src,
        "::",
        &registry,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
        Some(profile),
    )
}

fn module_proc_info(src: &str, qname: &str) -> UpvarInfo {
    module_proc_info_in(src, qname, false)
}

/// Under the lenient `tcl` profile: a Logical source with no native naming.
fn logical_proc_info(src: &str, qname: &str) -> UpvarInfo {
    module_proc_info_in(src, qname, true)
}

fn module_proc_info_in(src: &str, qname: &str, logical: bool) -> UpvarInfo {
    let environment = crate::environment_ingress::resolve_environment("tcl9.0");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = if logical {
        tcl_dialect::DialectProfile::plain_tcl()
    } else {
        environment.unit_profile()
    };
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let module = crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(registry, input.lexer_config())
            .with_resolved_analysis_input(input),
        src,
    );
    let bindings = ModuleCommandBindings::analyse(&module, registry);
    let procedure = module
        .procedures
        .get(&crate::interprocedural::test_body_identity::fixture_id(
            &module.procedures,
            qname,
        ))
        .expect("procedure");
    let namespace = procedure
        .body
        .namespace_context
        .as_deref()
        .expect("actual original body namespace");
    collect_upvar_targets_with_bindings(
        &procedure.body,
        &procedure.params,
        registry,
        &bindings,
        namespace,
        &module,
    )
}

#[test]
fn method_frame_summary_keeps_the_issued_header_and_actual_registry() {
    let environment = crate::environment_ingress::resolve_environment("tcl9.0");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        native_entry: Some(std::sync::Arc::new(native)),
        ..Default::default()
    };
    let build = || {
        crate::compilation_unit::CompilationUnit::build_with_context_registry(
            "oo::class create C {method empty {} {}}",
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(&context),
        )
        .ir_module
    };
    let module = build();
    assert_eq!(module.methods.len(), 1);
    let (&id, method) = module.methods.iter().next().unwrap();
    assert!(module.procedure_ids.owns(id));
    assert!(
        module
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .matches_original_method_summary(id, method)
    );
    assert!(!method_reaches_caller_frame(&module, id, registry));
    let mut changed = module.clone();
    changed
        .methods
        .get_mut(&id)
        .unwrap()
        .params
        .push("detached".into());
    assert!(method_reaches_caller_frame(&changed, id, registry));
    let mut changed_body = module.clone();
    changed_body.methods.get_mut(&id).unwrap().body = lower("upvar 1 caller local");
    assert!(method_reaches_caller_frame(&changed_body, id, registry));
    let foreign = build();
    let foreign_id = *foreign.methods.keys().next().unwrap();
    assert_eq!(foreign_id.ordinal(), id.ordinal());
    assert_ne!(foreign_id, id);
    assert!(method_reaches_caller_frame(&module, foreign_id, registry));
    let foreign_registry =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    assert!(method_reaches_caller_frame(
        &module,
        id,
        foreign_registry.commands(),
    ));
}

#[test]
fn native_frame_aliases_distinguish_written_scalar_reads_from_literal_values() {
    let environment = crate::environment_ingress::resolve_environment("tcl9.0");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        native_entry: Some(std::sync::Arc::new(native)),
        ..Default::default()
    };
    for (source, parameter) in [
        ("proc helper {source} {upvar 1 $source local}", true),
        ("proc helper {source} {upvar 1 \"$source\" local}", true),
        ("proc helper {source} {upvar 1 {$source} local}", false),
        (
            "interp alias {} captured {} upvar 1 {$source}; proc helper {source} {captured local}",
            false,
        ),
    ] {
        let module = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(&context),
        )
        .ir_module;
        let id =
            crate::interprocedural::test_body_identity::fixture_id(&module.procedures, "::helper");
        assert!(module.procedure_ids.owns(id));
        let summaries = crate::cfg_builder::detect_upvar_procs_with_registry(&module, registry);
        let summary = &summaries[&id];
        assert_eq!(
            summary.param_targets.get("local").map(String::as_str),
            parameter.then_some("source"),
            "{source}"
        );
        if !parameter {
            assert!(summary.param_targets.is_empty(), "{source}");
            assert!(
                summary.has_unresolvable_caller_target || summary.caller_frame_opaque_writes,
                "{source}"
            );
        }
    }
}

#[test]
fn logical_frame_aliases_keep_real_formals_and_literal_dollar_names() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let formal = logical_proc_info("proc helper {source} {upvar 1 $source local}", "::helper");
    assert_eq!(
        formal.param_targets.get("local"),
        Some(&"source".to_owned())
    );
    assert!(!formal.has_unresolvable_caller_target);
    assert!(!formal.caller_frame_opaque_writes);

    let literal = logical_proc_info(
        "proc helper {} {upvar 1 {$scalar(open} {$alias}; set {$alias} VALUE}",
        "::helper",
    );
    assert_eq!(
        literal.literal_targets.get("$alias"),
        Some(&"$scalar(open".to_owned())
    );
    assert!(!literal.has_unresolvable_caller_target);
}

#[test]
fn empty_body_has_no_upvars() {
    let body = lower("");
    let info = collect_upvar_targets(&body, &[]);
    assert!(info.is_empty());
}

#[test]
fn standalone_alias_pairs_keep_original_reference_grammar_and_literal_data() {
    for (source, parameter) in [
        ("upvar 1 ${odd name} local", "odd name"),
        ("upvar 1 \"$source\" local", "source"),
    ] {
        let body = lower(source);
        let info = collect_upvar_targets(&body, &[parameter.to_owned()]);
        assert_eq!(
            info.param_targets.get("local").map(String::as_str),
            Some(parameter)
        );
    }
    let body = lower("upvar 1 {$source} local");
    let info = collect_upvar_targets(&body, &["source".to_owned()]);
    assert!(info.param_targets.is_empty());
    assert!(info.has_unresolvable_caller_target);
    let mut changed = lower("upvar 1 $source local");
    let (Statement::Call { args, .. } | Statement::Barrier { args, .. }) =
        &mut changed.statements[0]
    else {
        panic!("actual standalone upvar call");
    };
    args[1] = "$different".to_owned();
    let info = collect_upvar_targets(&changed, &["different".to_owned()]);
    assert!(info.param_targets.is_empty());
    assert!(info.caller_frame_opaque_writes);
}

#[test]
fn literal_target_recorded() {
    let body = lower("upvar 1 caller_x x");
    let info = collect_upvar_targets(&body, &[]);
    assert_eq!(info.literal_targets.get("x"), Some(&"caller_x".to_string()),);
    assert_eq!(info.param_targets.len(), 0);
    assert_eq!(info.args_tail_upvar.len(), 0);
}

#[test]
fn multiple_literal_targets_recorded() {
    let body = lower("upvar 1 a la b lb c lc");
    let info = collect_upvar_targets(&body, &[]);
    assert_eq!(info.literal_targets.get("la"), Some(&"a".to_string()));
    assert_eq!(info.literal_targets.get("lb"), Some(&"b".to_string()));
    assert_eq!(info.literal_targets.get("lc"), Some(&"c".to_string()));
}

#[test]
fn param_target_recorded_when_param_exists() {
    let body = lower("upvar 1 $name x");
    let info = collect_upvar_targets(&body, &["name".to_string()]);
    assert_eq!(info.param_targets.get("x"), Some(&"name".to_string()));
    assert_eq!(info.literal_targets.len(), 0);
}

#[test]
fn dollar_var_skipped_when_not_a_param() {
    let body = lower("upvar 1 $foo x");
    let info = collect_upvar_targets(&body, &["bar".to_string()]);
    // $foo is not a param (only `bar` is); not classifiable per-name —
    // and the summary must say so, not silently under-approximate: the
    // callee can write ANY caller variable through the alias.
    assert_eq!(info.param_targets.len(), 0);
    assert_eq!(info.literal_targets.len(), 0);
    assert!(info.has_unresolvable_caller_target);
}

#[test]
fn fully_dynamic_source_widens_summary() {
    // `upvar 1 [pick] x` / an array-element source — the caller-side
    // name is runtime state; the flag must be set so the call site
    // widens to an opaque caller-frame clobber.
    for src in ["upvar 1 [pick] x", "upvar 1 a($i) x"] {
        let body = lower(src);
        let info = collect_upvar_targets(&body, &[]);
        assert!(
            info.has_unresolvable_caller_target,
            "{src} must widen the summary"
        );
    }
    // Resolvable shapes must NOT set the flag.
    let body = lower("upvar 1 caller_x x\nupvar 1 $p y");
    let info = collect_upvar_targets(&body, &["p".to_string()]);
    assert!(!info.has_unresolvable_caller_target);
}

#[test]
fn args_tail_upvar_recorded() {
    let body = lower("upvar 1 $args y");
    let info = collect_upvar_targets(&body, &["args".to_string()]);
    // `args` triggers the trailing-vararg path regardless of
    // its position in the param list.
    assert_eq!(info.args_tail_upvar, vec!["y".to_string()]);
    assert_eq!(info.param_targets.len(), 0);
}

#[test]
fn level_omitted_defaults_to_one() {
    // No leading level word: `upvar src dst`.
    let body = lower("upvar caller_x x");
    let info = collect_upvar_targets(&body, &[]);
    assert_eq!(info.literal_targets.get("x"), Some(&"caller_x".to_string()),);
}

#[test]
fn level_hash_zero_is_not_a_caller_frame_binding() {
    // TN — `upvar #0 g local_g` binds the **global** `g`, not the
    // caller's, however deep the call stack is (tclsh 9.0.4 / 8.6.14:
    // `proc deep {} {upvar #0 gvar g; set g DEEP}` called two frames
    // down sets `::gvar`).  Reporting it as a caller-frame def would
    // mark an unrelated *local* named `g` defined in every caller.
    // `global_write_info` owns the real, global effect.
    let body = lower("upvar #0 g local_g");
    let info = collect_upvar_targets(&body, &[]);
    assert_eq!(info.literal_targets.len(), 0, "got {info:?}");
    assert!(!info.caller_frame_opaque_writes, "got {info:?}");
}

#[test]
fn level_zero_is_the_callees_own_frame_and_contributes_nothing() {
    // TN — `upvar 0 x y` aliases the *callee's own* local `x` (tclsh
    // 9.0.4 / 8.6.14: `upvar 0 target alias` inside a proc whose caller
    // has `target` reads `can't read "alias"`), so no caller effect.
    let body = lower("upvar 0 x y");
    let info = collect_upvar_targets(&body, &[]);
    assert!(info.is_empty(), "got {info:?}");
}

#[test]
fn level_beyond_the_caller_widens_rather_than_binding() {
    // `upvar 2 far f` writes the caller's *caller* and nothing between:
    // on tclsh 8.4.20, 8.6.18 and 9.0.4, `proc gp {} {upvar 2 far f; set
    // f FAR}` through one intermediate frame sets the grandparent's `far`
    // and leaves the intermediate frame's variables alone. So it is an
    // exact target two frames out. A level nothing static places may be
    // any frame, the direct caller included, so it widens that too.
    let body = lower("upvar 2 far f");
    let info = collect_upvar_targets(&body, &["lvl".to_string()]);
    assert_eq!(
        info.beyond_caller,
        BTreeSet::from([FrameTarget::Caller(std::num::NonZeroU32::new(2).unwrap())]),
        "got {info:?}"
    );
    assert!(!info.caller_frame_opaque_writes, "got {info:?}");
    assert_eq!(info.literal_targets.len(), 0, "got {info:?}");
    for src in ["upvar #3 far f", "upvar $lvl a b"] {
        let body = lower(src);
        let info = collect_upvar_targets(&body, &["lvl".to_string()]);
        assert_eq!(
            info.beyond_caller,
            BTreeSet::from([FrameTarget::Unknown]),
            "{src}: {info:?}"
        );
        assert!(
            info.caller_frame_opaque_writes,
            "{src} must widen; got {info:?}"
        );
        assert_eq!(info.literal_targets.len(), 0, "{src}: got {info:?}");
    }
}

#[test]
fn an_uplevel_past_the_caller_is_an_exact_target_there() {
    // On tclsh 8.4.20, 8.6.18 and 9.0.4, `proc deep {} {uplevel 3 {set x
    // 2}}` called through two wrappers sets `x` in the third frame out,
    // and only there.
    let info = collect_upvar_targets(&lower("uplevel 3 {set x 2}"), &[]);
    assert_eq!(
        info.beyond_caller,
        BTreeSet::from([FrameTarget::Caller(std::num::NonZeroU32::new(3).unwrap())]),
        "got {info:?}"
    );
    assert!(!info.caller_frame_opaque_writes, "got {info:?}");
    assert!(info.uplevel_literal_writes.is_empty(), "got {info:?}");
    let unplaced = collect_upvar_targets(&lower("uplevel #2 {set x 2}"), &[]);
    assert_eq!(
        unplaced.beyond_caller,
        BTreeSet::from([FrameTarget::Unknown]),
        "got {unplaced:?}"
    );
    assert!(unplaced.caller_frame_opaque_writes, "got {unplaced:?}");
}

#[test]
fn frame_targets_move_one_level_nearer_per_call_and_saturate() {
    use std::num::NonZeroU32;
    let caller = |distance| FrameTarget::Caller(NonZeroU32::new(distance).unwrap());
    assert_eq!(FrameTarget::past_the_caller(1), None);
    assert_eq!(FrameTarget::past_the_caller(2), Some(caller(2)));
    assert_eq!(
        FrameTarget::past_the_caller(MAX_FRAME_DISTANCE + 1),
        Some(FrameTarget::Unknown)
    );
    assert_eq!(caller(3).one_call_nearer(), Some(caller(2)));
    assert_eq!(caller(2).one_call_nearer(), None);
    assert_eq!(
        FrameTarget::Unknown.one_call_nearer(),
        Some(FrameTarget::Unknown)
    );
}

#[test]
fn level_word_presence_follows_argument_count_parity() {
    // `upvar $lvl a b` — three words, so `$lvl` is the level and
    // `(a, b)` is the pair. A text-sniffing level test saw no level here
    // and paired `($lvl, a)`, losing the binding (tclsh 9.0.4 / 8.6.14:
    // `proc t4 {lvl} {upvar $lvl a b; set b 4}` really does set the
    // caller's `a`).  The level is dynamic, so the summary widens.
    let body = lower("upvar $lvl a b");
    let info = collect_upvar_targets(&body, &["lvl".to_string()]);
    assert!(info.caller_frame_opaque_writes, "got {info:?}");

    // `upvar 1 b` — two words, so there is NO level word and `1` is the
    // caller-side *variable name* (tclsh 9.0.4 / 8.6.14: with `set 1
    // ONE` in the caller, the callee reads `ONE` through the alias).
    let body = lower("upvar 1 b");
    let info = collect_upvar_targets(&body, &[]);
    assert_eq!(info.literal_targets.get("b"), Some(&"1".to_string()));

    // `upvar 1 a b c` — four words, so no level: two pairs, `(1, a)` and
    // `(b, c)` (tclsh 9.0.4 / 8.6.14 accept it without error).
    let body = lower("upvar 1 a b c");
    let info = collect_upvar_targets(&body, &[]);
    assert_eq!(info.literal_targets.get("a"), Some(&"1".to_string()));
    assert_eq!(info.literal_targets.get("c"), Some(&"b".to_string()));
}

#[test]
fn original_protected_substitution_keeps_caller_alias_pairs() {
    // experiments/name-handling/results/upvar-protected-child-20261014/oracles/observations.json
    for command in [
        "catch {upvar 1 target local}",
        "catch {catch {upvar 1 target local}}",
    ] {
        let info = module_proc_info(
            &format!("proc p {{}} {{set result [{command}]; set local VALUE}}"),
            "::p",
        );
        assert_eq!(
            info.literal_targets.get("local"),
            Some(&"target".to_owned()),
            "{command}"
        );
    }
    let dynamic = module_proc_info("proc p {script} {set result [catch $script]}", "::p");
    assert!(dynamic.caller_frame_opaque_writes);
    assert!(dynamic.caller_frame_opaque_reads);
}

#[test]
fn original_caller_argument_values_preserve_literal_scalar_names() {
    // experiments/name-handling/results/upvar-selected-value-names-20261014/oracles/observations.json
    let mut info = module_proc_info("proc helper {source} {upvar 1 $source local}", "::helper");
    let params = vec!["source".to_owned()];
    for name in ["", "$name", "a b", "{literal}"] {
        let words = [InvocationWord::Literal(name)];
        let explicit = info.caller_side_effects(InvocationArguments::Structured(&words), &params);
        assert_eq!(explicit.defs, vec![name.to_owned()]);
        assert!(!explicit.opaque);
        info.param_defaults = name_param_defaults(
            &info,
            &format!("{{source {{{name}}}}}"),
            tcl_syntax::word_rules::WordValueRules::from_config(&tcl_lexer::LexerConfig::default()),
        );
        let omitted = info.caller_side_effects(InvocationArguments::Structured(&[]), &params);
        assert_eq!(omitted.defs, vec![name.to_owned()]);
        assert!(!omitted.opaque);
    }
    let dynamic = [InvocationWord::Dynamic];
    let effects = info.caller_side_effects(InvocationArguments::Structured(&dynamic), &params);
    assert!(effects.opaque);
}

#[test]
fn uplevel_literal_body_records_caller_frame_writes() {
    // `uplevel 1 {set litVar hello}` — the lowering inlines the literal
    // body as `Statement::UpFrame`; its writes land in the caller. The
    // caller can shadow unqualified `set`, so the summary also widens.
    let info = logical_proc_info(
        "proc ::setter {} {uplevel 1 {set litVar hello}}; ::setter",
        "::setter",
    );
    assert!(
        info.uplevel_literal_writes.contains("litVar"),
        "got {info:?}"
    );
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
}

#[test]
fn uplevel_literal_body_resolves_late_alias_prefix_write() {
    // `setter` is lowered before `put` is installed, and the alias
    // prepends `x`: Tcl 9.0.4 evaluates the literal body as `set x 99`
    // in the caller's frame.  The module-wide binding projection must
    // recover that caller-frame write even though lower-time defs cannot.
    let info = module_proc_info(
        "proc ::setter {} { uplevel 1 { put 99 } }\n\
             interp alias {} put {} set x",
        "::setter",
    );
    assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
}

#[test]
fn nested_relative_zero_preserves_the_caller_frame() {
    let info = module_proc_info(
        "proc ::setter {} { uplevel 1 { uplevel 0 { set x 1 } } }; ::setter",
        "::setter",
    );
    assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
}

#[test]
fn late_alias_to_eval_preserves_an_enclosing_caller_frame() {
    let info = module_proc_info(
        "proc ::setter {} { uplevel 1 { ::inner {::set x 1} } }\n\
             interp alias {} ::inner {} eval",
        "::setter",
    );
    assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
}

#[test]
fn late_alias_to_relative_zero_uplevel_preserves_an_enclosing_caller_frame() {
    let info = module_proc_info(
        "proc ::setter {} { uplevel 1 { ::inner {::set x 1} } }\n\
             interp alias {} ::inner {} uplevel 0",
        "::setter",
    );
    assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
}

#[test]
fn late_alias_to_relative_one_uplevel_reaches_past_an_enclosing_caller_frame() {
    let info = module_proc_info(
        "proc ::setter {} { uplevel 1 { ::inner {::set x 1} } }\n\
             interp alias {} ::inner {} uplevel 1",
        "::setter",
    );
    // Exactly two frames out, and only there: on tclsh 8.4.20, 8.6.18
    // and 9.0.4, with `proc caller {} {set x 0; setter; return $x}`
    // called from a frame holding `x`, `caller` returns 0 and the
    // calling frame's `x` becomes 1.
    assert_eq!(
        info.beyond_caller,
        BTreeSet::from([FrameTarget::Caller(std::num::NonZeroU32::new(2).unwrap())]),
        "got {info:?}"
    );
    assert!(!info.caller_frame_opaque_writes, "got {info:?}");
}

#[test]
fn current_frame_alias_pairs_compose_inside_the_selected_caller_frame() {
    let info = module_proc_info(
        "proc ::setter {} { uplevel 1 { ::upvar 0 x local; ::set local 1 } }",
        "::setter",
    );
    assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
}

#[test]
fn alias_prefixed_alias_pairs_keep_their_effective_operands() {
    let info = module_proc_info(
        "proc ::setter {} { uplevel 1 { ::bind local; ::set local 1 } }\n\
             interp alias {} ::bind {} upvar 0 x",
        "::setter",
    );
    assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
}

#[test]
fn dynamic_current_frame_alias_target_widens_the_selected_caller_frame() {
    let info = module_proc_info(
        "proc ::setter {} { uplevel 1 { ::upvar 0 $name local; ::set local 1 } }",
        "::setter",
    );
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
}

#[test]
fn caller_alias_inside_the_selected_caller_frame_reaches_past_it() {
    let info = module_proc_info(
        "proc ::setter {} { uplevel 1 { ::upvar 1 x local; ::set local 1 } }",
        "::setter",
    );
    // Exactly two frames out, and only there: on tclsh 8.4.20, 8.6.18
    // and 9.0.4, with `proc caller {} {set x 0; setter; return $x}`
    // called from a frame holding `x`, `caller` returns 0 and the
    // calling frame's `x` becomes 1.
    assert_eq!(
        info.beyond_caller,
        BTreeSet::from([FrameTarget::Caller(std::num::NonZeroU32::new(2).unwrap())]),
        "got {info:?}"
    );
    assert!(!info.caller_frame_opaque_writes, "got {info:?}");
}

#[test]
fn current_frame_body_retains_nested_caller_frame_effects() {
    let info = module_proc_info(
        "proc ::setter {} { uplevel 0 { uplevel 1 { set x 1 } } }",
        "::setter",
    );
    assert!(info.uplevel_literal_writes.contains("x"), "got {info:?}");
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
}

#[test]
fn nested_relative_one_reaches_past_the_caller() {
    let info = module_proc_info(
        "proc ::setter {} { uplevel 1 { uplevel 1 { set x 1 } } }; \
             proc ::caller {} {::setter}; ::caller",
        "::setter",
    );
    // Two frames out for a caller in `::`: on tclsh 8.4.20, 8.6.18 and
    // 9.0.4, with `proc caller {} {set x 0; setter; return $x}` called
    // from a frame holding `x`, `caller` returns 0 and the calling
    // frame's `x` becomes 1.
    assert_eq!(
        info.beyond_caller,
        BTreeSet::from([FrameTarget::Caller(std::num::NonZeroU32::new(2).unwrap())]),
        "got {info:?}"
    );
    // The inner `uplevel` is unqualified and resolves in the caller
    // frame's namespace, so the summary cannot say "only there" for a
    // caller it has not seen: the same releases, with the caller in
    // `namespace eval ns { proc uplevel args {} }`, run `::ns::uplevel`
    // and write `x` in neither frame.
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
}

#[test]
fn relative_upframe_does_not_borrow_the_callee_namespace_alias() {
    let info = module_proc_info(
        "interp alias {} ::callee::put {} puts\n\
             interp alias {} ::caller::put {} set x\n\
             proc ::callee::setter {} { uplevel 1 { put 99 } }",
        "::callee::setter",
    );
    // The literal `put` resolves in the frame selected by `uplevel 1`,
    // not in ::callee. A caller-local alias may write arbitrary names.
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
    assert!(info.caller_frame_opaque_reads, "got {info:?}");
}

#[test]
fn uplevel_constructed_list_resolves_the_written_name() {
    // A substituted constructor operand is not a literal result element.
    // Even when it names a formal parameter, retaining its source spelling
    // would mistake `$varName` for the value supplied at runtime.
    let info = module_proc_info(
        "proc ::dynamic {varName} { uplevel 1 [list set $varName 1] }",
        "::dynamic",
    );
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
    assert!(info.caller_frame_opaque_reads, "got {info:?}");
    assert_eq!(info.uplevel_param_writes.len(), 0, "got {info:?}");

    // Even literal result words retain an unqualified command head, which
    // resolves in the caller's namespace rather than ::literal's.
    let info = module_proc_info(
        "proc ::literal {} { uplevel 1 [list set fixed 1] }",
        "::literal",
    );
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
    assert!(info.caller_frame_opaque_reads, "got {info:?}");
    assert_eq!(info.uplevel_literal_writes.len(), 0, "got {info:?}");

    // A target that is neither literal nor a parameter (a `foreach`
    // variable, say) is unknowable — widen.
    let info = module_proc_info(
        "proc ::loop {vs} { foreach v $vs { uplevel 1 [list set $v 1] } }",
        "::loop",
    );
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
}

#[test]
fn absolute_constructed_set_is_exact_while_relative_set_is_opaque() {
    let exact = module_proc_info(
        "proc ::exact {} { uplevel 1 [list ::set fixed 1] }",
        "::exact",
    );
    assert_eq!(
        exact.uplevel_literal_writes,
        BTreeSet::from(["fixed".to_owned()]),
        "got {exact:?}"
    );
    assert!(!exact.caller_frame_opaque_writes, "got {exact:?}");
    assert!(!exact.caller_frame_opaque_reads, "got {exact:?}");

    let relative = module_proc_info(
        "proc ::relative {} { uplevel 1 [list set fixed 1] }",
        "::relative",
    );
    assert_eq!(relative.uplevel_literal_writes.len(), 0, "got {relative:?}");
    assert!(relative.caller_frame_opaque_writes, "got {relative:?}");
    assert!(relative.caller_frame_opaque_reads, "got {relative:?}");
}

#[test]
fn relative_constructed_bodies_do_not_borrow_callee_namespace() {
    let setter = module_proc_info(
        "interp alias {} ::caller::set {} set shadow\n\
             namespace eval ::callee {}\n\
             proc ::callee::setter {} { uplevel 1 [list set fixed 1] }",
        "::callee::setter",
    );
    assert!(setter.caller_frame_opaque_writes, "got {setter:?}");
    assert!(setter.caller_frame_opaque_reads, "got {setter:?}");
    assert_eq!(setter.uplevel_literal_writes.len(), 0, "got {setter:?}");

    let forwarded = module_proc_info(
        "proc ::worker {name} { upvar 1 $name local; set local 1 }\n\
             interp alias {} ::caller::worker {} set shadow\n\
             namespace eval ::callee {}\n\
             proc ::callee::forward {} { uplevel 1 [list worker target] }",
        "::callee::forward",
    );
    assert!(forwarded.caller_frame_opaque_writes, "got {forwarded:?}");
    assert!(forwarded.caller_frame_opaque_reads, "got {forwarded:?}");
    assert_eq!(
        forwarded.uplevel_forwarded_calls.len(),
        0,
        "got {forwarded:?}"
    );
}

#[test]
fn rebound_list_constructor_widens_caller_frame_effects() {
    let info = module_proc_info(
        "proc list args { return {set hidden 1} }\n\
             proc ::p {} { uplevel 1 [list set claimed 1] }",
        "::p",
    );
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
    assert!(info.caller_frame_opaque_reads, "got {info:?}");
    assert_eq!(info.uplevel_literal_writes.len(), 0, "got {info:?}");
}

#[test]
fn uplevel_dynamic_body_is_an_opaque_caller_frame_barrier() {
    let body = lower("uplevel 1 $script");
    let info = collect_upvar_targets(&body, &["script".to_string()]);
    assert!(info.caller_frame_opaque_writes, "got {info:?}");
    assert!(info.caller_frame_opaque_reads, "got {info:?}");
    let barrier = info.caller_frame_barrier();
    assert!(barrier.writes && barrier.reads && barrier.destroys);
}

#[test]
fn uplevel_at_the_current_or_global_frame_is_not_a_caller_effect() {
    // TN — `uplevel 0 $s` re-enters the callee's own frame and
    // `uplevel #0 $s` the global one; neither touches the caller.
    // `crate::dynamic_names` raises the *callee's* own barrier for both.
    for src in ["uplevel 0 $s", "uplevel #0 $s"] {
        let body = lower(src);
        let info = collect_upvar_targets(&body, &["s".to_string()]);
        assert!(info.is_empty(), "{src}: got {info:?}");
    }
}

#[test]
fn uplevel_of_a_side_effect_free_command_has_no_caller_effect() {
    // TN — an absolute `::puts` head is namespace-invariant and writes no
    // variable anywhere, so
    // the summary must stay clear rather than widening on the mere
    // presence of an `uplevel`.
    let info = module_proc_info("proc ::shout {} { uplevel 1 [list ::puts hi] }", "::shout");
    assert!(info.is_empty(), "got {info:?}");
}

#[test]
fn dynamic_destination_with_literal_source_still_names_the_caller_var() {
    // `upvar 1 x $dst` aliases exactly the caller's `x`;
    // only the local key is dynamic.  The summary must not read as
    // empty (tclsh 9.0.4 / 8.6.16: `proc p {dst} {upvar 1 x $dst; set
    // $dst 2}; proc c {} {set x 1; p x; puts $x}` prints 2, so a call
    // site that kept `x = 1` foldable would miscompile).
    let body = lower("upvar 1 caller_x $local");
    let info = collect_upvar_targets(&body, &[]);
    assert!(!info.is_empty(), "got {info:?}");
    assert!(
        info.uplevel_literal_writes.contains("caller_x"),
        "got {info:?}"
    );
    assert!(!info.has_unresolvable_caller_target, "got {info:?}");
    // Structural caller-frame reach survives an unnameable local.
    assert!(info.unnameable_local_aliases.contains("caller_x"));
    // And the call-site resolver surfaces the caller-side name.
    assert_eq!(info.caller_side_defs(&[], &[]), vec!["caller_x"]);
}

#[test]
fn dynamic_destination_with_param_source_resolves_at_the_call_site() {
    // `upvar 1 $name $dst`: the caller-side name is the
    // value passed for `name`, exactly like `param_targets`.
    let body = lower("upvar 1 $name $local");
    let info = collect_upvar_targets(&body, &["name".to_string()]);
    assert!(info.uplevel_param_writes.contains("name"), "got {info:?}");
    assert!(!info.has_unresolvable_caller_target, "got {info:?}");
    let defs = info.caller_side_defs(&["target".to_string()], &["name".to_string()]);
    assert_eq!(defs, vec!["target"]);
}

#[test]
fn dynamic_destination_with_dynamic_source_widens() {
    // Both sides dynamic: the callee can clobber any
    // caller variable, so the summary must widen, not stay empty.
    for src in [
        "upvar 1 [pick] $local",
        "upvar 1 $notparam $local",
        "upvar 1 $args $local",
    ] {
        let body = lower(src);
        let info = collect_upvar_targets(&body, &["args".to_string()]);
        assert!(
            info.has_unresolvable_caller_target,
            "{src} must widen; got {info:?}"
        );
    }
}

#[test]
fn upvar_inside_if_body_collected() {
    let body = lower("if {1} { upvar 1 caller_x x }");
    let info = collect_upvar_targets(&body, &[]);
    assert_eq!(info.literal_targets.get("x"), Some(&"caller_x".to_string()),);
}

#[test]
fn upvar_inside_while_body_collected() {
    let body = lower("while {1} { upvar 1 caller_y y; break }");
    let info = collect_upvar_targets(&body, &[]);
    assert_eq!(info.literal_targets.get("y"), Some(&"caller_y".to_string()),);
}

#[test]
fn resolve_literal_or_param_literal() {
    let mut info = UpvarInfo::default();
    info.literal_targets
        .insert("x".to_string(), "caller_x".to_string());
    let args = BTreeMap::new();
    assert_eq!(
        info.resolve_literal_or_param("x", &args),
        Some("caller_x".to_string()),
    );
}

#[test]
fn resolve_literal_or_param_via_param() {
    let mut info = UpvarInfo::default();
    info.param_targets
        .insert("x".to_string(), "name".to_string());
    let mut args = BTreeMap::new();
    args.insert("name".to_string(), Some("caller_real".to_string()));
    assert_eq!(
        info.resolve_literal_or_param("x", &args),
        Some("caller_real".to_string()),
    );
}

#[test]
fn resolve_literal_or_param_dynamic_param_returns_none() {
    let mut info = UpvarInfo::default();
    info.param_targets
        .insert("x".to_string(), "name".to_string());
    let mut args = BTreeMap::new();
    args.insert("name".to_string(), None); // dynamic call-site arg
    assert!(info.resolve_literal_or_param("x", &args).is_none());
}

#[test]
fn resolve_literal_or_param_unknown_local_returns_none() {
    let info = UpvarInfo::default();
    let args = BTreeMap::new();
    assert!(info.resolve_literal_or_param("missing", &args).is_none());
}

#[test]
fn is_empty_helper() {
    let mut info = UpvarInfo::default();
    assert!(info.is_empty());
    info.literal_targets.insert("x".into(), "y".into());
    assert!(!info.is_empty());
}

#[test]
fn caller_side_defs_literal_only() {
    let mut info = UpvarInfo::default();
    info.literal_targets.insert("x".into(), "caller_x".into());
    info.literal_targets.insert("y".into(), "caller_y".into());
    let defs = info.caller_side_defs(&[], &[]);
    let mut sorted = defs.clone();
    sorted.sort();
    assert_eq!(sorted, vec!["caller_x".to_string(), "caller_y".to_string()]);
}

#[test]
fn caller_side_defs_param_resolution() {
    let mut info = UpvarInfo::default();
    info.param_targets.insert("x".into(), "name".into());
    // proc foo {name body} { upvar 1 $name x ... }
    // Call: foo my_var { ... }
    let params = vec!["name".to_string(), "body".to_string()];
    let call_args = vec!["my_var".to_string(), "{set x 1}".to_string()];
    let defs = info.caller_side_defs(&call_args, &params);
    assert_eq!(defs, vec!["my_var".to_string()]);
}

#[test]
fn caller_side_defs_param_dollar_normalisation() {
    let mut info = UpvarInfo::default();
    info.param_targets.insert("x".into(), "name".into());
    let params = vec!["name".to_string()];
    // call site passes `$user_var` — should normalise to `user_var`.
    let call_args = vec!["$user_var".to_string()];
    let defs = info.caller_side_defs(&call_args, &params);
    assert_eq!(defs, vec!["user_var".to_string()]);
}

#[test]
fn caller_side_defs_literal_plus_param() {
    let mut info = UpvarInfo::default();
    info.literal_targets.insert("a".into(), "caller_a".into());
    info.param_targets.insert("b".into(), "n".into());
    let params = vec!["n".to_string()];
    let call_args = vec!["caller_b".to_string()];
    let defs = info.caller_side_defs(&call_args, &params);
    let mut sorted = defs.clone();
    sorted.sort();
    assert_eq!(sorted, vec!["caller_a".to_string(), "caller_b".to_string()]);
}

#[test]
fn caller_side_defs_dedup() {
    let mut info = UpvarInfo::default();
    info.literal_targets.insert("a".into(), "same".into());
    info.literal_targets.insert("b".into(), "same".into());
    let defs = info.caller_side_defs(&[], &[]);
    assert_eq!(defs, vec!["same".to_string()]);
}

#[test]
fn caller_side_defs_args_tail_upvar_positional() {
    // `args_tail_upvar` records the locals (in declaration order)
    // for every `upvar 1 $args local` form in the proc body.  The
    // caller-side resolver pairs each entry positionally with a
    // call-site tail argument starting at `params.len() - 1`.
    let mut info = UpvarInfo::default();
    info.args_tail_upvar.push("x".into()); // first tail arg → x
    info.args_tail_upvar.push("y".into()); // second tail arg → y
    let params = vec!["a".to_string(), "args".to_string()];
    // Call: foo first_pos tail_one tail_two tail_three
    // tail_start = params.len() - 1 = 1.
    let call_args = vec![
        "first_pos".to_string(),
        "tail_one".to_string(),
        "tail_two".to_string(),
        "tail_three".to_string(),
    ];
    let defs = info.caller_side_defs(&call_args, &params);
    // Two declarations pick the first two tail args.
    assert_eq!(defs, vec!["tail_one".to_string(), "tail_two".to_string()]);
}

#[test]
fn caller_side_defs_missing_param_skipped() {
    let mut info = UpvarInfo::default();
    info.param_targets
        .insert("x".into(), "missing_param".into());
    let params = vec!["other".to_string()];
    let call_args = vec!["v".to_string()];
    let defs = info.caller_side_defs(&call_args, &params);
    // `missing_param` isn't in params — skip silently.
    assert_eq!(defs.len(), 0);
}

#[test]
fn caller_side_defs_dynamic_arg_normalises_to_empty_skipped() {
    let mut info = UpvarInfo::default();
    info.param_targets.insert("x".into(), "n".into());
    let params = vec!["n".to_string()];
    // `$` alone normalises to empty — should be skipped.
    let call_args = vec!["$".to_string()];
    let defs = info.caller_side_defs(&call_args, &params);
    assert_eq!(defs.len(), 0);
}
#[test]
fn caller_frame_metadata_keeps_actual_availability_and_missing_owner_hazards() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    // A source MAY footprint does not require or create a Native entered frame.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&current),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let mut module = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        "proc helper {} {uplevel 1 {::lassign {VALUE} target}}",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    )
    .ir_module;
    let body_id =
        crate::interprocedural::test_body_identity::fixture_id(&module.procedures, "::helper");
    let selected = crate::cfg_builder::detect_upvar_procs_with_registry(&module, registry);
    assert!(selected[&body_id].uplevel_literal_writes.contains("target"));
    let original = module.source_metadata_input.as_ref().unwrap().clone();
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(registry));
    for context in [
        std::sync::Arc::new(older),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
    ] {
        module.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            original.analyser_profile(),
            original.unit_profile(),
            context,
            original.lexer_config(),
        ));
        let unavailable = crate::cfg_builder::detect_upvar_procs_with_registry(&module, registry);
        assert!(unavailable[&body_id].caller_frame_opaque_writes);
    }
    module.source_metadata_input = None;
    let missing = crate::cfg_builder::detect_upvar_procs_with_registry(&module, registry);
    assert!(missing[&body_id].caller_frame_opaque_writes);
    assert!(missing[&body_id].caller_frame_opaque_reads);
    assert!(!missing[&body_id].is_empty());
}
