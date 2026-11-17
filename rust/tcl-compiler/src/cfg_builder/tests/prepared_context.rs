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

//! The prepared module context: source metadata, `detect_upvar_procs` propagation and the compatibility entry point.

use super::*;

#[test]
fn cfg_and_binding_analysis_retain_the_same_registry_generation() {
    let registry = native_registry();
    let module = crate::lowering::lower_to_ir("set value 1", &registry);
    let bindings = ModuleCommandBindings::analyse(&module, &registry);
    let classes = CfgCommandClasses::from_registry(&registry);

    assert!(Arc::ptr_eq(
        &registry.effective_semantics(),
        &module.resolved_registry().effective_semantics(),
    ));
    // The actual entry roster is narrower than catalogue assistance. Both
    // projections query the same frozen registry generation.
    assert!(bindings.effective_semantics().command("set").is_some());
    assert!(classes.semantics.command("set").is_some());
}

#[test]
fn prepared_cfg_source_metadata_preserves_availability_and_refuses_missing_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let context = Arc::new(generation.with_command_store(Arc::clone(generation.commands())));
    let registry = generation.commands();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
        .with_dialect(registry.profile())
        .with_context_registry(Arc::clone(&context));
    let module = lowerer.lower("throw ERROR payload").clone();
    let statement = &module.top_level.statements[0];
    let tokens = statement.tokens().expect("original command words");
    let prepared = prepare_cfg_context_bundle(&module, registry);
    assert!(
        prepared
            .command_classes
            .logical_structured_invocation(tokens)
            .is_some()
    );
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(registry)),
    );
    let original = module
        .source_metadata_input
        .as_ref()
        .expect("retained metadata");
    let input = crate::analyser::ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        older,
        original.lexer_config(),
    );
    let unavailable =
        PreparedCfgContext::from_source_input(prepared.context.clone(), registry, Some(&input));
    assert!(
        unavailable
            .command_classes
            .logical_structured_invocation(tokens)
            .is_none()
    );
    assert!(
        unavailable
            .command_classes
            .completion_route(statement)
            .is_none()
    );
    let foreign = CfgCommandClasses::from_metadata(
        registry,
        Some(tcl_registry::model::ingress::context_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        )),
    );
    assert!(foreign.logical_structured_invocation(tokens).is_none());
    assert!(foreign.resolved_tokens(tokens).is_none());
    let absent = PreparedCfgContext::from_source_input(prepared.context.clone(), registry, None);
    assert!(
        absent
            .command_classes
            .logical_structured_invocation(tokens)
            .is_none()
    );
    assert!(absent.command_classes.resolved_tokens(tokens).is_none());
    assert!(
        absent
            .command_classes
            .exact_completion(statement, &|_| None)
            .is_none()
    );
    let mut missing = module.clone();
    missing.source_metadata_input = None;
    assert!(
        prepare_cfg_context_bundle(&missing, registry)
            .command_classes
            .logical_structured_invocation(tokens)
            .is_none()
    );
    assert!(
        CfgCommandClasses::from_registry(registry)
            .logical_structured_invocation(tokens)
            .is_some(),
        "explicit standalone control"
    );
    let mut incomplete = tokens.clone();
    incomplete.source_binding = None;
    assert!(
        prepared
            .command_classes
            .logical_structured_invocation(&incomplete)
            .is_none()
    );
}

#[test]
fn prepared_cfg_refuses_changed_module_profile_and_body_configuration() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let registry = context.commands();
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        "if {[set output VALUE]} {puts $output}",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let module = &unit.ir_module;
    assert!(
        prepare_cfg_context_bundle(module, registry)
            .command_classes
            .metadata_context()
            .is_some()
    );
    let mut wrong_profile = module.clone();
    wrong_profile.dialect_profile = Some(tcl_dialect::DialectProfile::find("tcl9.0").unwrap());
    assert!(
        prepare_cfg_context_bundle(&wrong_profile, registry)
            .command_classes
            .metadata_context()
            .is_none()
    );
    let prepared = prepare_cfg_context_bundle(module, registry);
    let bad_config = tcl_lexer::LexerConfig {
        expand_syntax: !config.expand_syntax,
        ..config
    };
    let builder = CfgBuilder::new_with_upvars_and_classes(
        true,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        prepared.context.3.clone(),
        registry,
        prepared.command_classes.clone(),
    )
    .with_lexer_config(bad_config);
    assert!(builder.command_classes.metadata_context().is_none());
    // Complete source metadata is still independently retained by its owner.
    assert_eq!(wrong_profile.source_metadata_input.as_ref(), Some(&input));
}

#[test]
fn original_cfg_metadata_preserves_explicit_standalone_and_missing_purposes() {
    let context = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let registry = context.commands();
    let mut lowerer = crate::lowering::Lowerer::with_config(
        registry,
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    )
    .with_dialect(registry.profile());
    let module = lowerer.lower("set value 1; puts $value").clone();
    assert!(module.source_entry.metadata_context.is_standalone());
    assert!(module.source_metadata_input.is_none());
    let original = prepare_cfg_context_bundle(&module, registry);
    assert!(matches!(
        original.command_classes.metadata,
        CfgMetadataContext::Standalone
    ));
    let explicit = PreparedCfgContext::from_metadata_input(
        original.context.clone(),
        registry,
        crate::registry_invocation::InvocationMetadataInput::Retained(
            &module.source_entry.metadata_context,
        ),
    );
    assert!(matches!(
        explicit.command_classes.metadata,
        CfgMetadataContext::Standalone
    ));
    let missing = PreparedCfgContext::from_source_input(original.context.clone(), registry, None);
    assert!(matches!(
        missing.command_classes.metadata,
        CfgMetadataContext::Unavailable
    ));
    let mut changed = module.clone();
    changed.source_entry.metadata_context = CfgMetadataContext::Unavailable;
    assert!(matches!(
        prepare_cfg_context_bundle(&changed, registry)
            .command_classes
            .metadata,
        CfgMetadataContext::Unavailable
    ));
    let foreign = CommandRegistry::build_default();
    assert!(matches!(
        prepare_cfg_context_bundle(&module, &foreign)
            .command_classes
            .metadata,
        CfgMetadataContext::Unavailable
    ));
}

#[test]
fn prepared_cfg_source_metadata_is_shared_by_module_and_body_builders() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let registry = context.commands();
    let mut lowerer = crate::lowering::Lowerer::with_config(
        registry,
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    )
    .with_dialect(registry.profile())
    .with_context_registry(Arc::clone(&context));
    let module = lowerer.lower("throw ERROR payload; set reached 1").clone();
    let prepared = prepare_cfg_context_bundle(&module, registry);
    let shared = PreparedCfgContext::from_source_input(
        prepared.context.clone(),
        registry,
        module.source_metadata_input.as_ref(),
    );
    let CfgMetadataContext::SuppliedSource(retained) = &shared.command_classes.metadata else {
        panic!("the actual body request retains its original context");
    };
    assert!(Arc::ptr_eq(&retained.context_registry(), &context));
    let cfg = build_cfg_with_registry(&module, false, registry);
    let body = build_cfg_function_with_prepared_context(
        "::top",
        &module.top_level,
        true,
        registry,
        module.plain_command_dispatch,
        &shared,
        module.lexer_config,
    );
    let expected = Some(Terminator::Complete {
        route: tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
            tcl_registry::CompletionCode::Error,
        ),
        span: Some(module.top_level.statements[0].span()),
    });
    assert_eq!(
        cfg.top_level.blocks[&cfg.top_level.entry].terminator,
        expected
    );
    assert_eq!(body.blocks[&body.entry].terminator, expected);
}

#[test]
fn prepared_context_cfg_matches_compatibility_entry_point() {
    let registry = native_registry();
    let module = crate::lowering::lower_to_ir(
        "proc writer {} { global answer; set answer 42 }\n\
             interp alias {} write-answer {} writer\n\
             write-answer\n",
        &registry,
    );
    let context = prepare_cfg_context_bundle(&module, &registry);

    assert_eq!(
        build_cfg_with_registry_and_context(
            &module,
            false,
            &registry,
            &context,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
        ),
        build_cfg_with_registry(&module, false, &registry),
    );
}

#[test]
fn detect_upvar_procs_retains_issued_body_effects() {
    let module = lower_module(
        "namespace eval ::ns {}\nproc ::ns::p {} { upvar 1 caller_x x }\nproc ::ns::p2 {} {}",
    );
    let upvar_procs = detect_upvar_procs(&module);
    assert!(!upvar_procs[&summary_id(&module, "::ns::p")].is_empty());
    assert!(!upvar_procs[&summary_id(&module, "p")].is_empty());
    // The empty body has an independently retained empty effect row.
    assert!(upvar_procs[&summary_id(&module, "::ns::p2")].is_empty());
    assert!(upvar_procs[&summary_id(&module, "p2")].is_empty());
}

/// #1797. tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 all print `2 0`
/// for `list [caller] [caller_old]`, with
/// `proc caller {} {p; return $x}` and
/// `proc caller_old {} {old; info exists x}`: each call reaches its own
/// generation, and only the redefined one writes the caller's `x`.
#[test]
fn each_call_joins_only_its_own_procedure_generation() {
    let module = lower_module(
        "proc p {} {}\n\
             rename p old\n\
             proc p {} {upvar 1 x y; set y 2}\n\
             proc caller {} {p}\n\
             proc caller_old {} {old}",
    );
    let summaries = detect_upvar_procs(&module);
    let generation = |needle: &str| {
        let start = u32::try_from(module.source.try_text().unwrap().find(needle).unwrap())
            .expect("fixture source offset fits u32");
        let (id, _) = module
            .procedures
            .iter()
            .find(|(_, procedure)| procedure.span.start() == start)
            .expect("authored generation");
        *id
    };
    let original = generation("proc p {} {}");
    let redefined = generation("proc p {} {upvar");
    assert!(summaries[&original].is_empty());
    assert_eq!(
        summaries[&redefined]
            .literal_targets
            .get("y")
            .map(String::as_str),
        Some("x")
    );
    assert!(!summaries[&redefined].caller_frame_barrier().writes);

    let cfg = build_cfg(&module, false);
    let function = |name: &str| {
        cfg.procedures
            .get(&fixture_id(&cfg.procedures, name))
            .expect("authored fixture body")
    };
    let caller = function("::caller");
    assert!(!caller.caller_frame_barrier.writes);
    assert!(caller.alias_observed_vars.contains("x"));
    let caller_old = function("::caller_old");
    assert!(!caller_old.caller_frame_barrier.writes);
    assert!(!caller_old.alias_observed_vars.contains("x"));
}

fn lower_for_release(release: &str, src: &str) -> Module {
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment(release).analyser_profile(),
    );
    crate::lowering::lower_to_ir(src, &registry)
}

/// #1797. tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 all return `2`
/// from `caller` (`proc caller {} {::n::writer; return $x}`): the
/// list-built script defines `::n::writer`, whose `upvar` writes the
/// caller's `x`. The binding owner recovers that body under its own ID,
/// and the call joins its precise summary.
#[test]
fn a_recovered_procedure_body_has_a_precise_summary() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let module = lower_for_release(
            release,
            "proc caller {} {::n::writer}\n\
             namespace eval ::n [list proc writer {} {upvar 1 x y; ::set y 2}]",
        );
        let recovered: Vec<_> = module
            .recovered_procedure_bodies()
            .map(|(id, _)| *id)
            .collect();
        let [writer] = recovered.as_slice() else {
            panic!("{release}: one recovered body, got {recovered:?}");
        };
        let summaries = detect_upvar_procs(&module);
        assert_eq!(
            summaries[writer]
                .literal_targets
                .get("y")
                .map(String::as_str),
            Some("x"),
            "{release}"
        );
        assert!(
            !summaries[writer].caller_frame_barrier().writes,
            "{release}"
        );
        let cfg = build_cfg(&module, false);
        let caller = cfg
            .procedures
            .get(&fixture_id(&cfg.procedures, "::caller"))
            .expect("authored fixture body");
        assert!(!caller.caller_frame_barrier.writes, "{release}");
        assert!(caller.alias_observed_vars.contains("x"), "{release}");
    }
}

/// tclsh 8.4.20 through 9.1.0: inside `::m`, `proc` names `::m::proc`, so
/// `namespace eval ::m [list proc writer ...]` calls it and creates no
/// `::m::writer` (`info commands ::m::writer` is empty). The recovered
/// script's `proc` is resolved in the command world current when it runs.
#[test]
fn a_recovered_script_publishes_no_body_through_a_shadowed_proc() {
    let module = lower_module(
        "namespace eval ::m {proc proc args {}}\n\
         namespace eval ::m [list proc writer {} {upvar 1 x y; ::set y 2}]",
    );
    assert_eq!(module.recovered_procedure_bodies().count(), 0);
}

/// A literal-list script keeps later `proc`s published exactly when the
/// body walk shows it cannot rebind `proc`. tclsh 8.4.20 through 9.1.0
/// return `ok` from `after_k` after `namespace eval ::k [list proc proc
/// args {}]`: that defines `::k::proc` and leaves the global `proc` alone.
#[test]
fn literal_list_scripts_keep_proc_publication_unless_they_rebind_it() {
    for (prefix, published) in [
        ("namespace eval ::n [list set a 1]", true),
        ("eval [list set a 1]", true),
        ("namespace eval ::k [list proc proc args {}]", true),
        ("namespace eval ::n [list rename proc other]", false),
        ("namespace eval ::n $script", false),
        ("set s [list set a 1]\nnamespace eval ::n $s", false),
    ] {
        let module = lower_module(&format!("{prefix}\nproc after_k {{}} {{return ok}}"));
        assert_eq!(
            fixture_id_if_present(&module.procedures, "::after_k").is_some(),
            published,
            "{prefix}"
        );
    }
}

// A wrapper proc
// that reaches an already-known upvar proc through a *plain* call
// (`real_worker $fvar $nvar $script`, not `uplevel`) does NOT itself
// become an upvar-write target for its own caller — tclsh9.0/8.6-
// verified (`can't read "myf": no such variable"` when the caller reads
// the variable in a statement separate from the call, i.e. genuinely
// outside any `uplevel`'d script argument). A plain call only shares
// *values*, not stack frames: `real_worker`'s own `upvar 1` reaches the
// wrapper's frame, not the wrapper's caller's frame; treating such a
// pass-through as transitive is wrong, as reading the variable outside
// any uplevel'd script shows. The tcllib idiom
// (`page::util::flow`) reaches its own worker via `uplevel 1 [list
// ... ]`, not a plain call — that genuinely propagates one frame further,
// confirmed against tclsh9.0 — but modelling it soundly (composing the
// wrapper's own uplevel level with the callee's upvar level) is not done
// here.

#[test]
fn detect_upvar_procs_does_not_propagate_through_a_plain_call_wrapper() {
    // TN — `wrapper` has no `upvar` of its own, and its call to the
    // known upvar proc `real_worker` is a plain call (not `uplevel`),
    // so it must NOT be registered as a transitive upvar proc.
    let module = lower_module(
        "proc real_worker {fvar nvar script} {\n\
             upvar 1 $fvar f\n\
             upvar 1 $nvar n\n\
             set f 1\n\
             set n 2\n\
             uplevel 1 $script\n\
             }\n\
             proc wrapper {fvar nvar script} {\n\
             real_worker $fvar $nvar $script\n\
             }",
    );
    let upvar_procs = detect_upvar_procs(&module);
    assert!(
        upvar_procs[&summary_id(&module, "wrapper")].is_empty(),
        "a plain-call wrapper must not be treated as a transitive upvar proc",
    );
}

#[test]
fn detect_upvar_procs_does_not_propagate_when_args_are_not_passed_through() {
    // TN — `unrelated_wrapper` calls the known upvar proc `real_worker`,
    // but with literal args ("x"/"y"), not its own parameters passed
    // through unchanged — real tclsh9.0/8.6 confirms this genuinely
    // errors ("can't read \"myf\": no such variable"), so it must NOT
    // be registered.
    let module = lower_module(
        "proc real_worker {fvar nvar script} {\n\
             upvar 1 $fvar f\n\
             upvar 1 $nvar n\n\
             set f 1\n\
             set n 2\n\
             uplevel 1 $script\n\
             }\n\
             proc unrelated_wrapper {a b c} {\n\
             real_worker x y $c\n\
             }",
    );
    let upvar_procs = detect_upvar_procs(&module);
    assert!(
        upvar_procs[&summary_id(&module, "unrelated_wrapper")].is_empty(),
        "unrelated_wrapper passes literal args, not its own params — must not propagate",
    );
}

/// The *other* half of the rule
/// the two tests above pin. A level-**1** effect stops at the wrapper's
/// own frame, but a level that lands past the callee's caller reaches
/// this proc's caller through an ordinary call, so the wrapper really
/// does become a caller-frame writer.
///
/// Byte-identical on tclsh 9.0.4 and 8.6.16:
///
/// ```tcl
/// proc setUp2 {var} { uplevel 2 [list set $var 99] }
/// proc middle {}    { setUp2 answer }
/// proc outer  {}    { middle ; return $answer }
/// puts [outer]        ;# -> 99
/// ```
#[test]
fn detect_upvar_procs_propagates_a_beyond_caller_effect_one_plain_hop() {
    let module = lower_module(
        "proc setUp2 {var} {\n             uplevel 2 [list set $var 99]\n             }\n             proc middle {} {\n             setUp2 answer\n             }",
    );
    let upvar_procs = detect_upvar_procs(&module);
    let middle = upvar_procs
        .get(&summary_id(&module, "middle"))
        .expect("a plain call to a beyond-caller writer registers the wrapper");
    assert!(
        middle.caller_frame_opaque_writes,
        "setUp2's `uplevel 2` lands in middle's caller: {middle:?}"
    );
    assert!(
        middle.caller_frame_opaque_reads,
        "the same script may read there too: {middle:?}"
    );
}

/// On tclsh 9.0.4:
///
/// ```tcl
/// proc far {} {uplevel 2 {set x 2}}
/// interp alias {} via {} far
/// proc middle {} {via}
/// proc outer {} {set x 1; middle; return $x}
/// puts [outer] ;# -> 2
/// ```
#[test]
fn alias_terminal_proc_propagates_a_beyond_caller_effect() {
    let module = lower_module(
        "proc far {} {uplevel 2 {set x 2}}\n\
             interp alias {} via {} far\n\
             proc middle {} {via}",
    );
    let upvar_procs = detect_upvar_procs(&module);
    let middle = upvar_procs
        .get(&summary_id(&module, "::middle"))
        .expect("the alias resolves to far's beyond-caller effect");
    assert!(middle.caller_frame_opaque_writes, "{middle:?}");
    assert!(middle.caller_frame_opaque_reads, "{middle:?}");
}

/// TN — `uplevel #0` (global) and `uplevel 0` (the callee's own frame)
/// reach no caller at all, so neither is a beyond-caller effect and
/// neither makes a plain-call wrapper one.
#[test]
fn detect_upvar_procs_does_not_propagate_a_global_or_own_frame_level() {
    for level in ["#0", "0"] {
        let module = lower_module(&format!(
            "proc setSomewhere {{var}} {{\n                 uplevel {level} [list set $var 99]\n                 }}\n                 proc wrapper {{}} {{\n                 setSomewhere answer\n                 }}"
        ));
        let upvar_procs = detect_upvar_procs(&module);
        assert!(
            upvar_procs[&summary_id(&module, "wrapper")].is_empty(),
            "`uplevel {level}` never touches a caller frame: {upvar_procs:?}"
        );
    }
}

/// The published summaries carry no composition bookkeeping — they are
/// folded into every procedure's `function_lattice` memo key, so only
/// what a call site actually reads may travel in them.
#[test]
fn detect_upvar_procs_publishes_no_plain_call_bookkeeping() {
    let module = lower_module(
        "proc setUp2 {var} {\n             uplevel 2 [list set $var 99]\n             }\n             proc middle {} {\n             setUp2 answer\n             }",
    );
    for (name, info) in detect_upvar_procs(&module) {
        assert!(
            info.plain_calls.is_empty(),
            "{name:?} published composition input: {info:?}"
        );
    }
}

#[test]
fn prepare_cfg_context_registers_params_for_all_procs() {
    let module = lower_module("proc ::ns::p {a b} { upvar 1 $a x }\nproc q {c} {}");
    let (_upvar_procs, proc_params, _global_write_procs, _command_bindings, _owners) =
        prepare_cfg_context(&module);
    assert_eq!(
        proc_params.get(&summary_id(&module, "::ns::p")),
        Some(&vec!["a".to_string(), "b".to_string()]),
    );
    assert_eq!(
        proc_params.get(&summary_id(&module, "::ns::p")),
        Some(&vec!["a".to_string(), "b".to_string()]),
    );
    // q has no upvar but its params should still be in proc_params.
    assert_eq!(
        proc_params.get(&summary_id(&module, "::q")),
        Some(&vec!["c".to_string()])
    );
}
