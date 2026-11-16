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

use crate::interprocedural::test_body_identity::fixture_id;

#[test]
fn catalogue_baseline_retains_rootability_and_exact_descriptor_coordinates() {
    let mut registry = CommandRegistry::build_default();
    assert!(registry.get("my").is_some());
    assert!(registry.get("::my").is_none());
    assert_eq!(default_binding("::my", &registry).kind, BindingKind::Opaque);
    for label in ["expr", "::expr"] {
        assert_eq!(default_binding(label, &registry).kind, BindingKind::Builtin);
    }
    for label in [":expr", "::::expr", "::missing", "::"] {
        assert_eq!(default_binding(label, &registry).kind, BindingKind::Opaque);
    }
    let mut rooted = registry.get_exact("expr").unwrap().clone();
    rooted.name = "::my";
    registry.insert(rooted);
    assert_eq!(
        default_binding("::my", &registry).kind,
        BindingKind::Builtin
    );
}

use super::*;
use crate::compilation_unit::CompilationUnit;

fn analyse(src: &str) -> (CompilationUnit, CommandRegistry) {
    let reg = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for(src, &reg, false);
    (cu, reg)
}

fn source_proof(
    source: &str,
    head: &str,
    dialect: tcl_registry::InvocationDialect,
) -> SourceInvocationBinding {
    let registry = CommandRegistry::build_default();
    let binding = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::default(),
        &registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..SourceAnalysisOptions::default()
        },
    );
    binding.invocation_at_source(head, u32::try_from(source.rfind(head).unwrap()).unwrap())
}

fn allocation_source(source: &str) -> SourceCommandBindings {
    SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::default(),
        &CommandRegistry::build_default(),
        SourceAnalysisOptions {
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V8_6,
            )),
            ..SourceAnalysisOptions::default()
        },
    )
}

#[test]
fn logical_command_layout_keeps_unknown_physical_frames_separate() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let image = tcl_lexer::SourceImage::document("proc subject {} {return VALUE}");
    let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
        &image,
        &crate::var_resolve::VariableExecutionFrame::Unknown,
        config,
        context.commands(),
        SourceAnalysisOptions::for_logical_source(&input).unwrap(),
    )
    .unwrap();
    let point = bindings.points.first().expect("original source command");
    assert_eq!(point.state.logical_source_name_advice_input(), Some(&input));
    assert_eq!(
        point.state.variable_frame,
        crate::var_resolve::VariableExecutionFrame::Unknown
    );
    let segment =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .remove(0);
    let tokens = crate::ir::CommandTokens::from_segmented(&image.source_map(), config, &segment);
    let site = CommandAllocationSite {
        source: Arc::clone(point.state.current_source_origin.as_ref().unwrap()),
        offset: segment.span.start(),
    };
    let snapshot = Arc::new(SourceLookupSnapshot::new(point.state.clone()));
    let selected = original_site_operand_layout_advice(
        &site,
        &tokens,
        &snapshot,
        &point.namespace_key,
        config,
    )
    .expect("conditional original Logical layout");
    assert_eq!(
        selected.frame(),
        &crate::var_resolve::VariableExecutionFrame::Unknown
    );
    assert!(selected.targets.iter().all(|target| target.registry_backed));
    assert!(snapshot.state.baseline.native_entry.is_none());
    assert!(snapshot.state.baseline.execution_name_policy.is_none());

    let mut stale = config;
    stale.strict_quoting = !stale.strict_quoting;
    assert!(
            original_site_operand_layout_advice(
                &site,
                &tokens,
                &snapshot,
                &point.namespace_key,
                stale,
            )
            .is_none()
        );
    let mut unowned = snapshot.state.clone();
    Arc::make_mut(&mut unowned.baseline).logical_source_input = None;
    let unowned = Arc::new(SourceLookupSnapshot::new(unowned));
    assert!(
            original_site_operand_layout_advice(
                &site,
                &tokens,
                &unowned,
                &point.namespace_key,
                config,
            )
            .is_none()
        );
    let mut foreign_source = snapshot.state.clone();
    foreign_source.current_source_origin = None;
    let foreign_source = Arc::new(SourceLookupSnapshot::new(foreign_source));
    assert!(
        original_site_operand_layout_advice(
            &site,
            &tokens,
            &foreign_source,
            &point.namespace_key,
            config,
        )
        .is_none()
    );
}

#[test]
fn constructed_builder_argv_preserves_retained_lexer_overlays() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = current.commands();
    let unit = CompilationUnit::build_for_dialect("set x 1", registry, false, "tcl8.6");
    let bindings = ModuleCommandBindings::analyse(&unit.ir_module, registry);
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    let mut config = input.lexer_config();
    config.escapes = tcl_dialect::EscapeSyntax::Tcl84;
    let overlay = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    );
    let spelling = r"[list \U00000041]";
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        registry, &overlay,
    )
    .unwrap();
    assert_eq!(
        constructed_script_words_with_metadata_context(
            spelling,
            registry,
            &bindings,
            "::",
            Some(metadata),
        ),
        Some(vec!["U00000041".to_owned()]),
    );
    assert_eq!(
        constructed_script_words_with_metadata_context(
            spelling,
            registry,
            &bindings,
            "::",
            Some(current.into()),
        ),
        Some(vec!["A".to_owned()]),
    );
    assert_eq!(
        constructed_script_words(spelling, registry, &bindings, "::"),
        Some(vec!["A".to_owned()]),
    );
    assert_eq!(
        constructed_script_words_with_metadata_context(spelling, registry, &bindings, "::", None,),
        None,
    );
}

#[test]
fn literal_procedure_returns_preserve_later_dispatch_points() {
    let source = "proc helper {mode} {return $mode}; helper a; helper b; set final 1";
    let analysis = allocation_source(source);
    for head in ["helper a", "helper b", "set final"] {
        let offset = u32::try_from(source.find(head).unwrap()).unwrap();
        let proof = analysis.invocation_at_source(head.split_whitespace().next().unwrap(), offset);
        assert!(proof.proved_target().is_some(), "{head}: {proof:#?}");
    }
}

#[test]
fn hypothetical_lookup_does_not_inherit_a_different_selected_operation() {
    let source = "set x 1";
    let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let analysis = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::for_profile(owner.commands().profile()),
        owner.commands(),
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V8_6,
            )),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let actual = analysis.invocation_at_source("set", 0);
    assert!(matches!(
        actual.native_compilation_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Inline { .. }
    ));
    for proof in [
        analysis.projection_at_source("list", 0),
        analysis.invocation_unpositioned("list"),
    ] {
        assert!(
            proof
                .proved_target()
                .is_some_and(|target| nqn(&target.command) == "::list")
        );
        assert_eq!(
            proof.native_compilation_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
        assert!(proof.native_handler_envelope.is_none());
        assert_eq!(
            proof.compiled_candidates,
            [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
        );
    }
    let unknown = analysis.projection_at_source("missing", 0);
    assert_eq!(unknown.targets, [] as [SourceCommandTarget; 0]);
    assert_eq!(
        unknown.native_compilation_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Unknown
    );
}

#[test]
fn whole_source_head_cache_preserves_proof_and_invalidates_on_point_changes() {
    let mut bindings = allocation_source("set x 1; set x 2");
    let first = bindings.invocation_unpositioned("set");
    let second = bindings.invocation_unpositioned("set");
    assert_eq!(first, second);
    assert!(Arc::ptr_eq(
        first.lookup_state.as_ref().unwrap(),
        second.lookup_state.as_ref().unwrap()
    ));
    assert_eq!(bindings.unpositioned_projections.0.lock().unwrap().len(), 1);
    let cloned = bindings.clone();
    assert!(cloned.unpositioned_projections.0.lock().unwrap().is_empty());
    assert_eq!(cloned.invocation_unpositioned("set"), first);
    let mut changed = bindings
        .points
        .iter()
        .find(|point| point.dispatch)
        .unwrap()
        .clone();
    changed.state.mark_opaque_binding_mutation();
    bindings.record_point(changed);
    assert!(
        bindings
            .unpositioned_projections
            .0
            .lock()
            .unwrap()
            .is_empty()
    );
    assert!(bindings.invocation_unpositioned("set").unknown);
}

#[test]
fn cached_source_origin_fingerprints_do_not_replace_semantic_identity() {
    let mut left = SourceOriginId::authored(&Arc::from("proc p {} {return LEFT}"));
    let mut right = SourceOriginId::authored(&Arc::from("proc p {} {return RIGHT}"));
    left.fingerprint = 7;
    right.fingerprint = 7;
    assert_ne!(left, right);
    assert_ne!(left.cmp(&right), std::cmp::Ordering::Equal);
    let mut identical = SourceOriginId::authored(&Arc::from("proc p {} {return LEFT}"));
    identical.fingerprint = 7;
    assert_eq!(left, identical);
}

#[test]
fn activation_source_ids_check_full_equality_and_keep_equal_live_clones() {
    let left = Arc::new(SourceOriginId::authored(&Arc::from("left")));
    let right = Arc::new(SourceOriginId::authored(&Arc::from("right")));
    let mut interner = SourceOriginInterner::default();
    let collision_key = interner.intern(&left.kind, 7);
    assert_ne!(collision_key, interner.intern(&right.kind, 7));
    let retained_payload = Arc::clone(&left.kind);
    assert_eq!(collision_key, interner.intern(&retained_payload, 7));
    let first = SourceOriginId::activation_key(&left);
    assert_ne!(first, SourceOriginId::activation_key(&right));
    let clone = Arc::new(left.as_ref().clone());
    assert_eq!(first, SourceOriginId::activation_key(&clone));
    drop(left);
    let equal = Arc::new(clone.as_ref().clone());
    assert_eq!(first, SourceOriginId::activation_key(&equal));
    let derived = Arc::new(SourceOriginId::derived(
        CommandAllocationSite {
            source: Arc::clone(&equal),
            offset: 0,
        },
        vec![1],
        &Arc::from("left"),
        MaterialisedSourceKind::Script,
    ));
    assert_ne!(first, SourceOriginId::activation_key(&derived));
}

#[test]
fn known_concatenated_script_mutates_the_shared_command_table() {
    let source = "eval {rename set} {saved}; set x 1";
    let analysis = allocation_source(source);
    let binding =
        analysis.invocation_at_source("set", u32::try_from(source.rfind("set").unwrap()).unwrap());
    assert!(binding.proved_target().is_none());
    assert!(!analysis.has_site(6));
}

#[test]
fn derived_procedure_body_retains_its_source_and_completion() {
    let source = "eval {proc p {} } {{return; rename set saved}}; p; set x 1";
    let analysis = allocation_source(source);
    let call = analysis.invocation_at_source(
        "p",
        u32::try_from(source.find("; p;").unwrap()).unwrap() + 2,
    );
    let target = call.proved_target().unwrap();
    assert!(matches!(
        target
            .implementation_allocation
            .as_ref()
            .unwrap()
            .site
            .source
            .kind(),
        SourceOriginKind::Derived { .. }
    ));
    assert!(!target.matches_authored_implementation(source, 0));
    let final_set =
        analysis.invocation_at_source("set", u32::try_from(source.rfind("set").unwrap()).unwrap());
    assert!(
        final_set
            .proved_target()
            .is_some_and(|target| target.registry_backed)
    );
}

#[test]
fn derived_procedure_mutation_reaches_the_authored_continuation() {
    let source = "eval {proc p {} } {{rename set saved}}; p; set x 1";
    let analysis = allocation_source(source);
    let final_set =
        analysis.invocation_at_source("set", u32::try_from(source.rfind("set").unwrap()).unwrap());
    assert!(final_set.proved_target().is_none());
}

#[test]
fn repeated_definition_site_allocates_distinct_tokens_after_a_move() {
    let source = "proc make {} {proc ::f {} {}}; make; rename f old; make; list ready";
    let analysis = allocation_source(source);
    let point =
        analysis.invocation_at_source("list", u32::try_from(source.find("list").unwrap()).unwrap());
    let old = point.lookup_command_word("old");
    let new = point.lookup_command_word("f");
    let old = old.proved_target().unwrap().identity.as_ref().unwrap();
    let new = new.proved_target().unwrap().identity.as_ref().unwrap();
    assert_ne!(old, new);
    assert_eq!(
        old.allocation.as_ref().unwrap().incarnation,
        AllocationIncarnation::First
    );
    assert_eq!(
        new.allocation.as_ref().unwrap().incarnation,
        AllocationIncarnation::Second
    );
}

#[test]
fn repeated_fresh_implementation_cannot_become_a_unique_runtime_token() {
    let source = "proc make {} {proc ::f {} {}}; make; make; make; list ready";
    let analysis = allocation_source(source);
    let point =
        analysis.invocation_at_source("list", u32::try_from(source.find("list").unwrap()).unwrap());
    let implementation = point.lookup_command_word("f");
    assert!(implementation.unknown);
    assert!(implementation.proved_target().is_none());
}

#[test]
fn derived_expression_effects_are_resolved_after_native_concatenation() {
    let source = "catch {expr {1 +} {[rename set saved]}}; set x 1";
    let analysis = allocation_source(source);
    let final_set =
        analysis.invocation_at_source("set", u32::try_from(source.rfind("set").unwrap()).unwrap());
    assert!(final_set.proved_target().is_none());
}

#[test]
fn frozen_script_values_execute_in_the_shared_source_world() {
    for source in [
        "set script {rename set saved}; catch $script; set x 1",
        "set script {rename ::set ::saved}; namespace eval n $script; set x 1",
        "set script {rename set saved}; proc p {} $script; p; set x 1",
        "interp alias {} run {} catch {rename set saved}; run; set x 1",
    ] {
        let analysis = allocation_source(source);
        let last = u32::try_from(source.rfind("set x").unwrap()).unwrap();
        assert!(
            analysis
                .invocation_at_source("set", last)
                .proved_target()
                .is_none(),
            "{source}"
        );
    }
}

#[test]
fn explicit_availability_phase_is_part_of_retained_point_identity() {
    use tcl_dialect::model::InvocationRealm;
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let source = "set x 1";
    let analyse = |invocation_realm| {
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_file_grammar(registry.profile().unwrap().grammar),
            registry,
            SourceAnalysisOptions {
                invocation_realm,
                ..SourceAnalysisOptions::default()
            },
        )
    };
    let loader = analyse(InvocationRealm::RuleLoader).invocation_at_source("set", 0);
    let runtime = analyse(InvocationRealm::InterpreterRuntime).invocation_at_source("set", 0);
    assert_eq!(loader.invocation_realm(), Some(InvocationRealm::RuleLoader));
    assert_eq!(
        runtime.invocation_realm(),
        Some(InvocationRealm::InterpreterRuntime)
    );
    assert_ne!(loader.lookup_state, runtime.lookup_state);
    assert_eq!(
        runtime.lookup_command_word("gets").invocation_realm(),
        Some(InvocationRealm::InterpreterRuntime)
    );
    assert!(
        runtime
            .lookup_command_word("gets")
            .proved_target()
            .is_some()
    );
    assert!(
        runtime
            .lookup_command_word("open")
            .proved_target()
            .is_none()
    );
    let mut joined = loader;
    joined.join(&runtime);
    assert_eq!(joined.invocation_realm(), None);
}

#[test]
fn retained_computed_procedure_body_preserves_its_abrupt_completion() {
    let source = "set script {return; rename set saved}; proc p {} $script; p; set x 1";
    let analysis = allocation_source(source);
    let last = u32::try_from(source.rfind("set x").unwrap()).unwrap();
    assert!(
        analysis
            .invocation_at_source("set", last)
            .proved_target()
            .is_some_and(|target| target.registry_backed)
    );
    let original_body = u32::try_from(source.find("return").unwrap()).unwrap();
    assert!(
        !analysis.has_site(original_body),
        "a stored value does not execute at its original literal location"
    );
}

#[test]
fn lambda_invocation_binds_actual_values_in_the_selected_namespace() {
    let source = "namespace eval n {}; apply {{name} {proc $name {} {}} n} helper; list ready";
    let analysis = allocation_source(source);
    let site = u32::try_from(source.find("list ready").unwrap()).unwrap();
    let point = analysis.invocation_at_source("list", site);
    assert!(
        point
            .lookup_command_word("::n::helper")
            .proved_target()
            .is_some_and(|target| target.kind == BindingKind::Proc),
        "{point:#?}"
    );
    assert!(
        point
            .lookup_command_word("::helper")
            .proved_target()
            .is_none()
    );
}

#[test]
fn lambda_uses_its_actual_caller_frame_and_stops_at_return() {
    let source =
        "set n 1; apply {{name} {upvar 1 $name v; incr v; return; rename set saved}} n; list $n";
    let analysis = allocation_source(source);
    let site = u32::try_from(source.find("list $n").unwrap()).unwrap();
    let point = analysis.invocation_at_source("list", site);
    assert_eq!(
        point.evaluated_argument_values,
        vec![Some("2".to_owned())].into()
    );
    assert!(
        point
            .lookup_command_word("set")
            .proved_target()
            .is_some_and(|target| target.registry_backed)
    );
}

#[test]
fn trusted_loader_requires_the_audited_native_engine() {
    use tcl_dialect::model::Family;
    let registry = CommandRegistry::build_default();
    let loader = TrustedPackageLoader {
        required_core_family: Some(Family::Tcl),
        package: "audited".to_owned(),
        version: Some("1.0".to_owned()),
        accepts_any_requested_version: false,
        implementation_id: "audited-implementation".to_owned(),
        command_surface: vec!["::audited::run".to_owned()],
        compiler_hooks: BTreeMap::new(),
        definition_dispatchers: std::collections::BTreeSet::new(),
        optional_command_surface: Vec::new(),
        namespace_exports: BTreeMap::new(),
        optional_namespace_exports: BTreeMap::new(),
        lookup_dependencies: Vec::new(),
        installed_lookup_dependencies: Vec::new(),
        state_dependency_namespaces: Vec::new(),
        modelled_state_variables: Vec::new(),
    };
    for family in [
        Some(Family::Tcl),
        Some(Family::Jim),
        Some(Family::F5Tcl),
        None,
    ] {
        let mut dialect =
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        dialect.native_family = family;
        let mut state = ModuleCommandBindings::initial_with_options(
            &registry,
            SourceAnalysisOptions {
                trusted_package_loaders: std::slice::from_ref(&loader),
                invocation_dialect: Some(dialect),
                ..SourceAnalysisOptions::default()
            },
            None,
        );
        apply_package_transition(
            &mut state,
            &tcl_registry::model::binding::PackageTransition::Require {
                package: TransitionSubject::Literal("audited".to_owned()),
                requirements: Vec::new(),
                exact: false,
            },
        );
        assert_eq!(
            state.loaded_provider("audited").is_some(),
            family == Some(Family::Tcl),
            "engine {family:?}"
        );
    }
}

#[test]
fn conditional_expression_mutation_reaches_the_selected_body() {
    let source = "if {[rename set saved; expr 1]} {set x 1}";
    let proof = source_proof(
        source,
        "set",
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert!(
        proof
            .proved_target()
            .is_none_or(|target| target.command != "::set")
    );
}

#[test]
fn source_pending_return_never_executes_dead_command_restoration() {
    let source = "rename set stockset; proc set {args} {return CUSTOM}; proc p {} {return; rename set {}; rename stockset set}; p; set x 1";
    let binding = source_proof(
        source,
        "set",
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert!(
        binding
            .proved_target()
            .is_none_or(|target| !target.registry_backed)
    );
}

#[test]
fn source_invocation_freezes_values_before_later_argument_effects() {
    let source = "set v OLD; list $v [set v NEW]";
    let binding = source_proof(
        source,
        "list",
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert_eq!(
        binding
            .evaluated_argument_values
            .first()
            .and_then(Option::as_deref),
        Some("OLD")
    );
}

#[test]
fn source_literal_callee_parameters_select_the_actual_caller_alias() {
    let source = "proc bump {name} {upvar 1 $name v; incr v}; set n 1; bump n; list $n";
    let binding = source_proof(
        source,
        "list",
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert_eq!(
        binding
            .evaluated_argument_values
            .first()
            .and_then(Option::as_deref),
        Some("2"),
        "{binding:#?}"
    );
    assert!(
        binding
            .proved_target()
            .is_some_and(|target| target.command == "::list")
    );
}

#[test]
fn source_catch_resumes_after_a_pending_return_without_running_dead_mutations() {
    let source = "catch {return; rename set {}}; set x 1";
    let binding = source_proof(
        source,
        "set",
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert!(
        binding
            .proved_target()
            .is_some_and(|target| target.command == "::set")
    );
}

#[test]
fn source_abrupt_substitution_skips_later_words_and_outer_dispatch() {
    for source in [
        "catch {puts \"[return stop][rename set oldSet]\"}; set final 1",
        "catch {puts $a([return stop][rename set oldSet])}; set final 1",
    ] {
        let binding = source_proof(
            source,
            "set",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert!(
            binding
                .proved_target()
                .is_some_and(|target| target.command == "::set"),
            "{source}"
        );
    }
    let source = "catch {expr {[return stop] + [rename puts oldPuts]}}; puts final";
    let binding = source_proof(
        source,
        "puts",
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert!(
        binding
            .proved_target()
            .is_some_and(|target| target.command == "::puts")
    );
}

#[test]
fn source_math_callee_preserves_return_boundary_and_global_effects() {
    let source = "proc ::tcl::mathfunc::f {x} {global y; set y 1; return 5; rename puts dead}; expr {f(0)}; list $y";
    let binding = source_proof(
        source,
        "list",
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert_eq!(
        binding
            .evaluated_argument_values
            .first()
            .and_then(Option::as_deref),
        Some("1"),
        "{binding:#?}"
    );
    assert!(
        binding
            .lookup_command_word("puts")
            .proved_target()
            .is_some_and(|target| target.command == "::puts")
    );
}

#[test]
fn source_c_import_follows_the_original_command_object() {
    let source = "rename auto_import savedPrelude; namespace eval a {proc f {} {return OLD}; namespace export f}\nnamespace eval b {namespace import ::a::f}\nrename ::a::f ::a::saved\nproc ::a::f {} {return NEW}\n::b::f";
    let binding = source_proof(
        source,
        "::b::f",
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    let target = binding.proved_target().expect("stable imported command");
    assert_eq!(target.command, "::a::saved");
    assert_eq!(target.identity.as_ref().unwrap().origin, "::a::f");
}

#[test]
fn source_body_namespace_statements_with_a_closed_prelude_keep_lookup_closed() {
    // tclsh 8.6 / 9.0: with the import preload renamed away, export,
    // import and forget in namespace bodies rebind no root command.
    let source = "rename auto_import savedPrelude\nnamespace eval src {proc p {} {return P}; namespace export p}\nnamespace eval dst {namespace import ::src::*}\nnamespace eval dst {namespace forget ::src::p}\nset x 0\nincr x\nset after 1";
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    for head in ["namespace export", "namespace import", "namespace forget"] {
        let binding = source_proof(source, head, dialect);
        assert!(!binding.execution_is_unknown(), "{head}: {binding:#?}");
    }
    for head in ["incr x", "set after"] {
        let binding = source_proof(source, head, dialect);
        assert!(
            binding
                .proved_target()
                .is_some_and(|target| target.registry_backed),
            "{head}: {binding:#?}"
        );
    }
}

#[test]
fn source_body_namespace_statements_outside_the_closed_premises_stay_open() {
    // tclsh 8.6 / 9.0 (each source rebinds the root `incr`, and the
    // document completes early): a document `auto_import`, the stock
    // `auto_import` loading an `auto_index` entry or an `auto_path`
    // index, a delete trace fired by a forget or a forced import, a
    // destructor of an object a forced import replaces, an ensemble map
    // changed for `namespace` or `string`, and an ensemble created over
    // `::namespace` or, by its namespace name, over `::string`.
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let hijack = "proc ::hijack {args} {rename ::incr ::saved_incr; proc ::incr {args} {return -code return}}\n";
    let export = "namespace eval ::m {proc h {} {}; namespace export h}\n";
    let tail = "set x 0\nincr x\nset after 1";
    for body in [
            format!("{export}proc auto_import {{pattern}} {{::hijack}}\nnamespace eval ::a {{namespace import ::m::*}}\n"),
            format!("{export}set ::auto_index(::m::zz) {{::hijack}}\nnamespace eval ::a {{namespace import ::m::*}}\n"),
            format!("lappend ::auto_path /autolib\n{export}namespace eval ::a {{namespace import ::m::*}}\n"),
            format!("rename auto_import savedPrelude\n{export}namespace eval ::a {{namespace import ::m::*}}\ntrace add command ::a::h delete ::hijack\nnamespace eval ::a {{namespace forget ::m::h}}\n"),
            format!("rename auto_import savedPrelude\n{export}namespace eval ::a {{proc h {{}} {{}}}}\ntrace add command ::a::h delete ::hijack\nnamespace eval ::a {{namespace import -force ::m::*}}\n"),
            format!("rename auto_import savedPrelude\n{export}oo::class create C {{destructor {{::hijack}}}}\nnamespace eval ::a {{}}\nC create ::a::h\nnamespace eval ::a {{namespace import -force ::m::*}}\n"),
            "namespace eval ::z {namespace ensemble configure ::namespace -map {eval ::tcl::namespace::eval export ::hijack}}\nnamespace eval ::a {proc h {} {}; namespace export h}\n".to_owned(),
            "namespace eval ::z {namespace ensemble configure ::string -map {repeat ::hijack}}\nstring repeat a 2\n".to_owned(),
            "rename auto_import savedPrelude\nnamespace eval ::fake {namespace ensemble create -command ::namespace -map {eval ::tcl::namespace::eval export ::hijack}}\nnamespace eval ::a {proc h {} {}; namespace export h}\n".to_owned(),
            "namespace eval ::string {proc repeat {args} {::hijack}; namespace export repeat; namespace ensemble create}\nnamespace eval ::b {string repeat a 2}\n".to_owned(),
        ] {
            let source = format!("{hijack}{body}{tail}");
            let binding = source_proof(&source, "incr x", dialect);
            assert!(binding.proved_target().is_none(), "{source}: {binding:#?}");
        }
    // An import brings a renamed `namespace` or `set` into `::a`, or a
    // renamed `eval` into the root; those heads no longer reach the stock
    // command there.
    for (source, head) in [
        (
            "namespace eval ::m {namespace export h namespace; proc h {} {return M}; proc helper args {}; rename helper namespace}\nnamespace eval ::a {namespace import ::m::*}\nnamespace eval ::a {namespace forget ::m::h}\nset ::after 1",
            "namespace forget",
        ),
        (
            "namespace eval ::m {proc h {} {return M}; namespace export h}\nnamespace import ::m::h\nnamespace eval ::q {namespace export eval; proc helper args {}; rename helper eval}\nnamespace eval :: {namespace import -force ::q::eval}\neval {namespace forget ::m::h}\nset ::after 1",
            "eval {namespace forget",
        ),
        (
            "namespace eval ::m {proc h {} {return M}; namespace export h}\nnamespace import ::m::h\nnamespace eval ::q {namespace export set; proc helper args {return -code return}; rename helper set}\nnamespace eval ::a {namespace import ::q::*}\nnamespace eval ::a {set x 1}\nnamespace forget ::m::h\nset ::after 1",
            "set x 1",
        ),
    ] {
        let binding = source_proof(source, head, dialect);
        assert!(
            binding
                .proved_target()
                .is_none_or(|target| !target.registry_backed),
            "{head}: {binding:#?}"
        );
    }
}

#[test]
fn source_renamed_procedure_executes_its_retained_implementation_generation() {
    let source =
        "proc p {} {rename set saved_set}\nrename p saved_p\nproc p {} {}\nsaved_p\nset after 1";
    assert!(
        source_proof(
            source,
            "set after",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
        )
        .proved_target()
        .is_none()
    );
}

#[test]
fn source_jim_import_retargets_the_original_source_name() {
    let source = "namespace eval a {proc f {} {return OLD}; namespace export f}\nnamespace eval b {namespace import ::a::f}\nrename ::a::f ::a::saved\nproc ::a::f {} {return NEW}\n::b::f";
    let binding = source_proof(
        source,
        "::b::f",
        tcl_registry::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        )),
    );
    assert_eq!(
        binding
            .proved_target()
            .expect("name-based imported command")
            .command,
        "::a::f"
    );
}

#[test]
fn source_alias_expands_prefixes_after_late_target_lookup() {
    let source = "interp alias {} outer {} inner A\ninterp alias {} inner {} set B\nouter C";
    let binding = source_proof(
        source,
        "outer",
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    let target = binding.proved_target().expect("effective alias target");
    assert_eq!(target.command, "::set");
    assert_eq!(
        target.prepended,
        [
            crate::registry_invocation::EffectiveInvocationWord::Literal("B".to_owned()),
            crate::registry_invocation::EffectiveInvocationWord::Literal("A".to_owned()),
        ]
    );
}

#[test]
fn source_original_empty_and_native_method_classes_have_bounded_construction() {
    for body in ["", "method local {} {return OK}"] {
        let source = format!("oo::class create Dog {{{body}}}\nset dog [Dog new]\nset after 1");
        let binding = source_proof(
            &source,
            "set after",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert_eq!(
            binding
                .proved_target()
                .expect("default construction retains command proof")
                .command,
            "::set"
        );
    }
}

#[test]
fn source_constructor_and_rebound_member_implementations_withdraw_construction_proof() {
    for source in [
        "oo::class create Dog {constructor {} {rename set saved_set}}\nset dog [Dog new]\nset after 1",
        "proc ::oo::define::method {args} {rename set saved_set}\noo::class create Dog {method local {} {}}\nset dog [Dog new]\nset after 1",
        "oo::objdefine ::oo::class {filter hidden}\noo::class create Dog {}\nset dog [Dog new]\nset after 1",
    ] {
        assert!(
            source_proof(
                source,
                "set after",
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
            )
            .proved_target()
            .is_none()
        );
    }
}

#[test]
fn source_repeated_body_joins_future_rebindings_before_specialisation() {
    let source = "foreach x {a b} {if {$x eq {a}} {proc set {args} {return CUSTOM}}; set y1}";
    let binding = source_proof(
        source,
        "set y1",
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert!(binding.proved_target().is_none());
}

/// A `namespace import` that shadows a proc declared in the same
/// namespace must distrust that binding — tclsh answers `EVIL` for the
/// sheet below, so folding `::ns::answer` with the declared body is
/// wrong. A *globbed* import elsewhere must not distrust the source
/// namespace it imports *from*, or an ordinary `namespace import ::lib::*`
/// would stop every fold in `::lib`.
#[test]
fn an_import_shadowing_a_declared_proc_distrusts_only_its_namespace() {
    let reg = CommandRegistry::build_default();
    let shadowed = "namespace eval ::evil { proc answer {} {return EVIL} ; namespace export answer }\n             namespace eval ::ns { proc answer {} { return 42 } ; namespace import -force ::evil::answer }\n";
    let cu = CompilationUnit::build_for(shadowed, &reg, false);
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(
        !m.trusts_proc_binding("::ns::answer"),
        "the import shadows the declared proc"
    );
    assert!(
        m.trusts_proc_binding("::evil::answer"),
        "the namespace imported *from* is untouched"
    );

    // A wildcard import makes only its target namespace opaque.
    let wildcard = "namespace eval ::lib { proc helper {} { return 1 } ; namespace export helper }\n             namespace eval ::app { namespace import ::lib::* }\n";
    let cu = CompilationUnit::build_for(wildcard, &reg, false);
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.trusts_proc_binding("::app::helper"));
    assert!(
        m.trusts_proc_binding("::lib::helper"),
        "the imported-from namespace still folds"
    );
}

/// A `TclOO` method runs in the receiver's namespace, chosen at run time,
/// so a command it names relatively may resolve to an object-local
/// implementation. That uncertainty belongs to the frame: it disqualifies
/// facts derived inside the body, and leaves the names the top level and
/// the procedures resolve untouched.
#[test]
fn an_ordinary_method_body_scopes_its_opacity_to_its_own_frame() {
    let reg = CommandRegistry::build_default();
    for body in [
        "puts hi",
        "my helper",
        "next",
        "set y 1",
        "if {1} { puts hi }",
    ] {
        let src = format!(
            "oo::class create ::A {{\n    method m {{}} {{ {body} }}\n}}\nproc p {{}} {{ return 1 }}\n"
        );
        let cu = CompilationUnit::build_for(&src, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            m.has_runtime_selected_frames(),
            "{body}: the frame itself stays opaque"
        );
        assert!(
            m.trusts("string"),
            "{body}: a builtin the module never touches must stay trusted"
        );
        assert!(
            m.trusts_proc_binding("::p"),
            "{body}: a proc declared outside the class must stay trusted"
        );
    }
}

/// A binding change from such a frame names its subject where this scan
/// cannot look, so it carries past the frame and no name in the module
/// survives it.
#[test]
fn a_binding_change_in_a_runtime_selected_frame_distrusts_the_module() {
    let reg = CommandRegistry::build_default();
    for body in [
        "rename puts myputs",
        "proc helper {} { return 1 }",
        "interp alias {} shout {} puts",
        "namespace import ::lib::*",
    ] {
        let src = format!(
            "oo::class create ::A {{\n    method m {{}} {{ {body} }}\n}}\nproc p {{}} {{ return 1 }}\n"
        );
        let cu = CompilationUnit::build_for(&src, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !m.trusts_proc_binding("::p"),
            "{body}: an unspellable subject must distrust the module"
        );
    }
}

/// A rebinding does not stop carrying past the frame because it sits in a
/// statically lowered `eval` or `uplevel`. Those lower to
/// `Statement::Block` / `Statement::UpFrame`, whose bodies are part of the
/// frame like any other nested script: `m` really does remove `::string`,
/// so a fold of `[string length x]` elsewhere would be a miscompile.
#[test]
fn a_rebinding_inside_a_lowered_eval_or_uplevel_still_distrusts_the_module() {
    let reg = CommandRegistry::build_default();
    for body in [
        "puts hi; eval {rename ::string ::saved}",
        "puts hi; uplevel 1 {rename ::string ::saved}",
        "puts hi; if {1} { eval {rename ::string ::saved} }",
    ] {
        let src = format!(
            "oo::class create ::A {{\n    method m {{}} {{ {body} }}\n}}\nproc p {{}} {{ return 1 }}\n"
        );
        let cu = CompilationUnit::build_for(&src, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !m.trusts("string"),
            "{body}: a rebinding in a lowered body must distrust the module"
        );
    }
}

/// A body whose commands all resolve absolutely is namespace-invariant, so
/// it takes the ordinary scan and leaves no frame opacity behind.
#[test]
fn an_absolute_only_method_body_leaves_no_frame_opacity() {
    let reg = CommandRegistry::build_default();
    let src = "oo::class create ::A {\n    method m {} { ::puts hi }\n}\nproc p {} { return 1 }\n";
    let cu = CompilationUnit::build_for(src, &reg, false);
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.has_runtime_selected_frames());
    assert!(m.trusts_proc_binding("::p"));
    assert!(m.trusts("puts"));
}

/// The snapshot is the per-procedure lattice memo's trust fact, so every
/// field of the summary must survive the round trip and take part in the
/// key: a memo taken under one binding state must never answer for
/// another. The literal names every field, so a field added to the
/// summary fails to compile here until it is carried.
#[test]
fn opaque_namespace_imports_preserve_empty_command_tails() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    let mutations = ModuleCommandMutations {
        opaque_namespaces: ["::ns".to_owned()].into_iter().collect(),
        ..ModuleCommandMutations::default()
    };
    for name in ["ns::", "::ns::", "::ns::::", "::ns::ordinary"] {
        assert!(!mutations.trusts_proc_binding(name), "{name}");
    }
    assert!(mutations.trusts_proc_binding("::ns"));
    assert!(mutations.trusts_proc_binding("::other::"));
}

#[test]
fn the_trust_snapshot_round_trips_every_mutation_field() {
    let every = ModuleCommandMutations {
        names: std::iter::once("llength".to_owned()).collect(),
        rebound: std::iter::once("p".to_owned()).collect(),
        dynamic: true,
        rebinding_subjects: RebindingSubjects::SomeUnnameable,
        runtime_selected_frames: true,
        resolution_changed: true,
        opaque_namespaces: std::iter::once("::ns".to_owned()).collect(),
        precision_written: PrecisionWrites::MayBeWritten,
    };
    assert_eq!(every.snapshot().to_mutations(), every);
    let base = ModuleCommandMutations::default();
    let one_field: [ModuleCommandMutations; 8] = [
        ModuleCommandMutations {
            names: every.names.clone(),
            ..base.clone()
        },
        ModuleCommandMutations {
            rebound: every.rebound.clone(),
            ..base.clone()
        },
        ModuleCommandMutations {
            dynamic: true,
            ..base.clone()
        },
        ModuleCommandMutations {
            rebinding_subjects: RebindingSubjects::SomeUnnameable,
            ..base.clone()
        },
        ModuleCommandMutations {
            runtime_selected_frames: true,
            ..base.clone()
        },
        ModuleCommandMutations {
            resolution_changed: true,
            ..base.clone()
        },
        ModuleCommandMutations {
            opaque_namespaces: every.opaque_namespaces.clone(),
            ..base.clone()
        },
        ModuleCommandMutations {
            precision_written: PrecisionWrites::MayBeWritten,
            ..base.clone()
        },
    ];
    for m in &one_field {
        assert_eq!(&m.snapshot().to_mutations(), m);
        assert_ne!(m.snapshot(), base.snapshot(), "{m:?} is part of the key");
    }

    // A scanned summary answers both trust stances alike from the
    // snapshot: `rename $a {}` names no subject, so no builtin is
    // claimable (#2168), and an unknown head raises only the top.
    let reg = CommandRegistry::build_default();
    for src in [
        "set a llength\nrename $a {}\n",
        "someUnknownLibraryCall x\n",
        "proc llength {l} { return 99 }\n",
    ] {
        let cu = CompilationUnit::build_for(src, &reg, false);
        let scanned = scan_module_command_mutations(&cu.ir_module, &reg);
        let restored = scanned.snapshot().to_mutations();
        assert_eq!(restored, scanned, "{src:?}");
        for name in ["llength", "list", "set"] {
            assert_eq!(
                restored.observed_binding_is_the_builtin(name),
                scanned.observed_binding_is_the_builtin(name),
                "{src:?} {name}"
            );
            assert_eq!(
                restored.trusts(name),
                scanned.trusts(name),
                "{src:?} {name}"
            );
        }
    }
}

/// The frame fact is part of the memo key: a summary taken with every
/// frame statically nameable must not be reused once one is not.
#[test]
fn the_frame_fact_survives_a_snapshot_round_trip() {
    let reg = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = "oo::class create ::A {\n    method m {} { puts hi }\n}\n";
    let cu = CompilationUnit::build_for(src, &reg, false);
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(m.has_runtime_selected_frames());
    assert!(m.snapshot().to_mutations().has_runtime_selected_frames());
    assert_ne!(
        m.snapshot(),
        ModuleCommandMutations::default().snapshot(),
        "the frame fact must distinguish the two summaries"
    );
}

#[test]
fn independently_analysed_summaries_are_semantically_equal() {
    let (cu, reg) = analyse("interp alias {} e {} expr\ne {1 + 1}");
    let first = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    let second = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

    assert!(!Arc::ptr_eq(&first.bindings, &second.bindings));
    assert!(!Arc::ptr_eq(
        &first.root_boundary_bindings,
        &second.root_boundary_bindings
    ));
    assert_eq!(first, second);
    assert!(
        !first.same_state(&second),
        "the analysis-local fast path deliberately requires one shared baseline Arc"
    );
}

#[test]
fn binding_state_forks_copy_procedure_inventory_only_for_a_new_body() {
    let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
    let original = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    let mut fork = original.clone();

    assert!(Arc::ptr_eq(
        &original.procedure_bodies,
        &fork.procedure_bodies
    ));
    assert!(original.same_state(&fork));

    assert!(!fork.extend_procedure_bodies(["::first".to_owned()]));
    assert!(
        Arc::ptr_eq(&original.procedure_bodies, &fork.procedure_bodies),
        "re-observing a retained body must not detach the shared inventory"
    );

    assert!(fork.extend_procedure_bodies(["::recovered".to_owned()]));
    assert!(!Arc::ptr_eq(
        &original.procedure_bodies,
        &fork.procedure_bodies
    ));
    assert!(!original.procedure_bodies.contains("::recovered"));
    assert!(fork.procedure_bodies.contains("::recovered"));
    assert!(!original.same_state(&fork));
}

#[test]
fn binding_state_forks_copy_binding_map_only_for_a_changed_entry() {
    let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
    let original = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    let mut unchanged = original.clone();

    assert!(Arc::ptr_eq(&original.bindings, &unchanged.bindings));
    assert!(Arc::ptr_eq(
        &original.root_boundary_bindings,
        &unchanged.root_boundary_bindings
    ));
    assert!(!unchanged.join(&original));
    assert!(
        Arc::ptr_eq(&original.bindings, &unchanged.bindings),
        "joining an identical fork must not detach the shared map"
    );

    let (existing_name, existing_bindings) = original
        .bindings
        .iter()
        .next()
        .expect("procedure analysis publishes at least one binding");
    unchanged.replace(existing_name.clone(), existing_bindings.clone());
    assert!(
        Arc::ptr_eq(&original.bindings, &unchanged.bindings),
        "replacing an entry with its current value must not detach the shared map"
    );

    let mut changed = original.clone();
    changed.replace(
        "::recovered".to_owned(),
        BTreeSet::from([MayBinding::Unknown]),
    );
    assert!(!Arc::ptr_eq(&original.bindings, &changed.bindings));
    assert!(!original.bindings.contains_key("::recovered"));
    assert_eq!(
        changed.bindings.get("::recovered"),
        Some(&BTreeSet::from([MayBinding::Unknown]))
    );
    assert!(
        Arc::ptr_eq(
            &original.root_boundary_bindings,
            &changed.root_boundary_bindings
        ),
        "a flow-state mutation must not copy the immutable boundary map"
    );
    assert!(Arc::ptr_eq(
        &original.procedure_bodies,
        &changed.procedure_bodies
    ));
    assert!(!original.same_state(&changed));
}

#[test]
fn exact_procedure_delta_preserves_history_and_root_boundary() {
    let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    let history = bindings
        .bindings
        .get("::first")
        .expect("historical procedure slot");
    let procedure_target = history
        .iter()
        .find_map(|binding| match binding {
            MayBinding::Target(target) if target.kind == BindingKind::Proc => Some(target),
            MayBinding::Target(_)
            | MayBinding::Imported(_)
            | MayBinding::Missing
            | MayBinding::Unknown => None,
        })
        .expect("actual installed procedure");
    assert_eq!(procedure_target.command, "::first");
    assert!(!procedure_target.registry_backed);
    assert_eq!(
        procedure_target.prepended,
        [] as [crate::registry_invocation::EffectiveInvocationWord; 0]
    );
    let allocation = procedure_target
        .implementation_allocation
        .as_ref()
        .expect("actual definition allocation");
    assert_eq!(allocation.site.offset, 0);
    assert_eq!(allocation.command, "::first");
    assert_eq!(
        procedure_target
            .token
            .as_ref()
            .and_then(|identity| identity.allocation.as_ref()),
        Some(allocation)
    );
    let procedure_target = MayBinding::Target(procedure_target.clone());

    assert_eq!(
        bindings.bindings.get("::first"),
        Some(&BTreeSet::from([
            MayBinding::Missing,
            procedure_target.clone()
        ])),
        "the historical view includes the state before and after definition"
    );
    assert_eq!(
        bindings.root_boundary_bindings.get("::first"),
        Some(&BTreeSet::from([procedure_target])),
        "only the post-definition binding is replayable at another root"
    );
}

#[test]
fn exact_procedure_delta_retains_non_binding_opacity() {
    let mut reg = CommandRegistry::build_default();
    let mut external_proc = reg.get("proc").expect("core proc spec").clone();
    external_proc.traits |= tcl_registry::Traits::LOADS_EXTERNAL_UNIT;
    reg.insert(external_proc);

    // The leading command ensures the observed state already exists when
    // the exact definition runs. A one-key fast path here would suppress
    // the ordinary observation and lose the external-unit opacity that
    // accompanies the otherwise-exact Define(Procedure).
    let cu = CompilationUnit::build_for("set marker 1\nproc created {} {return ok}", &reg, false);
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

    assert!(bindings.opaque_binding_mutation);
    assert!(bindings.has_opaque_domain());
    assert!(
        bindings.mutation_projection(&reg).has_dynamic_mutation(),
        "the historical mutation projection must retain external-unit opacity"
    );
}

#[test]
fn exact_procedure_delta_retains_immediate_body_binding_mutation() {
    let mut reg = CommandRegistry::build_default();
    let mut immediate_proc = reg.get("proc").expect("core proc spec").clone();
    immediate_proc
        .traits
        .remove(tcl_registry::Traits::DEFERS_BODY);
    immediate_proc.frame_effect = Some(tcl_registry::FrameEffectSpec {
        level_word: tcl_registry::FrameLevelWord::None,
        layout: tcl_registry::FrameArgLayout::ScriptInCurrentFrame,
    });
    reg.insert(immediate_proc);

    // This authored Proc-lowering operation both defines `created` and
    // immediately evaluates its body. The nested definition changes a
    // second binding without necessarily growing any non-binding axis:
    // both procedure bodies can already be present in the module inventory.
    let cu = CompilationUnit::build_for(
        "set marker 1\nproc created {} {proc set {} {return hijacked}}",
        &reg,
        false,
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

    assert!(
        !bindings.mutation_projection(&reg).trusts("set"),
        "the historical view must retain the immediately evaluated body's builtin redefinition"
    );
}

#[test]
fn narrow_proc_binding_projection_excludes_unavailable_body_opacity() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let (cu, reg) = analyse(
        "set ::auto_index(missing_command) {source lib.tcl}\nmissing_command argument\nproc retained {} {return ok}",
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

    assert!(
        bindings.mutation_projection(&reg).has_dynamic_mutation(),
        "an unresolved command may load code that changes builtin trust"
    );
    assert!(
        bindings
            .proc_binding_trust_projection()
            .trusts_proc_binding("::retained"),
        "unavailable code does not itself prove that a retained proc was rebound"
    );
}

#[test]
fn narrow_proc_binding_projection_tracks_dynamic_rebinding_subjects() {
    let (cu, reg) = analyse("set name retained\nrename $name saved\nproc retained {} {return ok}");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    let trust = bindings.proc_binding_trust_projection();

    assert!(trust.has_dynamic_binding_transition());
    assert!(!trust.trusts_proc_binding("::retained"));
}

#[test]
fn narrow_proc_binding_projection_retains_static_namespace_candidates() {
    let (cu, reg) = analyse("namespace eval ::n {rename retained saved}");
    let trust = ModuleCommandBindings::analyse(&cu.ir_module, &reg).proc_binding_trust_projection();

    assert!(!trust.has_dynamic_binding_transition());
    assert!(!trust.trusts_proc_binding("::n::retained"));
    assert!(
        !trust.trusts_proc_binding("::retained"),
        "the legacy source scan also records a global fallback candidate"
    );
}

#[test]
fn consumed_cfg_commands_retain_the_original_native_entry_baseline() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let (owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = SourceAnalysisEntry {
        native_entry: Some(Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = CompilationUnit::build_with_context_registry(
        "set result VALUE",
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        Arc::clone(&context),
    );
    let cfg = &unit.top_level.cfg;
    let statement = &cfg.blocks[&cfg.entry].statements[0];
    assert!(matches!(statement, Statement::AssignConst { .. }));
    assert!(statement.tokens().is_none());
    let original = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &cfg.command_binding_sites,
        statement,
    )
    .unwrap();
    assert!(
        original
            .source_binding
            .as_ref()
            .unwrap()
            .proved_execution_target()
            .is_some()
    );
    let selected = analyse_command_binding(cfg, context.commands(), &[]);
    assert!(selected.is_original_builtin_at(cfg.entry, 0, "set"));
    let mut missing = cfg.clone();
    missing.command_binding_sites.clear();
    assert!(missing.statement_sources.values().any(Option::is_some));
    assert!(
        !analyse_command_binding(&missing, context.commands(), &[]).is_original_builtin_at(
            missing.entry,
            0,
            "set"
        )
    );
    let mut conflicting = cfg.clone();
    let mut other = conflicting.command_binding_sites[0].clone();
    other.source_tokens.as_mut().unwrap().argv_texts[0] = "other".into();
    conflicting.command_binding_sites.push(other);
    assert!(
        !analyse_command_binding(&conflicting, context.commands(), &[]).is_original_builtin_at(
            conflicting.entry,
            0,
            "set"
        )
    );
    let mut foreign = context.commands().project_for_profile(profile);
    let mut setter = foreign.get("set").unwrap().clone();
    setter.lowering_hook = None;
    foreign.insert(setter);
    assert!(
        !analyse_command_binding(cfg, &foreign, &[]).is_original_builtin_at(cfg.entry, 0, "set")
    );
    drop(owner);
}

#[test]
fn unperturbed_builtin_is_builtin_no_rebound() {
    let (cu, reg) = analyse("string toupper a");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    assert!(cb.is_original_builtin_at(fu.cfg.entry, 0, "string"));
    assert!(cb.rebound_names().is_empty());
    assert!(!cb.has_wildcard());
}

#[test]
fn class_destroy_makes_the_class_command_opaque() {
    // `Animal destroy` deletes the class command: the binding is Class
    // before the destroy and Opaque after, so a later `Animal new`
    // draws W128.  Definer creation and the destructive method are
    // both registry data (definition_body / oo::object's `destroy`).
    let (cu, reg) = analyse(
        "oo::class create Animal {}
Animal new
Animal destroy
Animal new",
    );
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    let animal_calls: Vec<_> = fu.cfg.blocks[&entry]
        .statements
        .iter()
        .enumerate()
        .filter_map(|(index, stmt)| {
            matches!(stmt, Statement::Call { command, .. } | Statement::Barrier { command, .. }
                    if command == "Animal")
            .then_some(index)
        })
        .collect();
    assert_eq!(
        cb.binding_at(entry, animal_calls[0], "Animal").kind,
        BindingKind::Class
    );
    assert_eq!(
        cb.binding_at(entry, animal_calls[2], "Animal").kind,
        BindingKind::Opaque,
        "the class command is deleted after `Animal destroy`"
    );
    assert!(cb.rebound_names().contains("::Animal"));
}

#[test]
fn instance_destroy_makes_the_instance_command_opaque() {
    // `Animal create fido` binds the instance command; `fido destroy`
    // deletes it; the class itself stays bound.
    let (cu, reg) = analyse(
        "oo::class create Animal {}
Animal create fido
fido destroy
fido bark
Animal new",
    );
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    let invocation_index = |command: &str, first_arg: &str| {
        fu.cfg.blocks[&entry]
            .statements
            .iter()
            .position(|stmt| {
                matches!(stmt,
                        Statement::Call { command: source, args, .. }
                        | Statement::Barrier { command: source, args, .. }
                        if source == command
                            && args.first().is_some_and(|arg| arg == first_arg))
            })
            .unwrap()
    };
    assert_eq!(
        cb.binding_at(entry, invocation_index("fido", "bark"), "fido")
            .kind,
        BindingKind::Opaque
    );
    assert_eq!(
        cb.binding_at(entry, invocation_index("Animal", "new"), "Animal")
            .kind,
        BindingKind::Class
    );
}

#[test]
fn closed_static_descriptor_does_not_mask_a_live_object_receiver() {
    let mut reg = CommandRegistry::build_default();
    reg.insert(tcl_registry::CommandSpec {
        name: "Animal",
        state_transitions: Some(tcl_registry::StateTransitionDescriptor::EMPTY),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let cu = CompilationUnit::build_for(
        "oo::class create Animal {}\nAnimal destroy\nAnimal new",
        &reg,
        false,
    );
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    assert_eq!(
        cb.binding_at(fu.cfg.entry, 2, "Animal").kind,
        BindingKind::Opaque,
        "the live object binding wins over the shadowed static descriptor"
    );
}

#[test]
fn destroy_as_ordinary_argument_is_not_a_deletion() {
    // A proc named `destroy` taking a class name as an ARGUMENT must
    // not delete anything: the head is the proc, not the class.
    let (cu, reg) = analyse(
        "oo::class create Animal {}
proc destroy {x} { puts $x }
destroy Animal
Animal new",
    );
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    assert_eq!(
        cb.binding_at(entry, 3, "Animal").kind,
        BindingKind::Class,
        "the class survives an unrelated `destroy` call"
    );
}

#[test]
fn snit_type_creation_has_only_its_attested_dispatcher() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let selected =
        crate::provider_fixtures::entry(registry, &[crate::provider_fixtures::Provider::Snit]);
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let source = "package require snit\nsnit::type Dog {}\nDog create d";
    let offset = u32::try_from(source.find("Dog create").unwrap()).unwrap();
    let bindings =
        SourceCommandBindings::analyse_with_options(source, config, registry, selected.options());
    let call = bindings.invocation_at_source("Dog", offset);
    assert_eq!(
        call.proved_execution_target().unwrap().kind,
        BindingKind::Command
    );
    assert_eq!(
        call.nominal_definition_name_result(registry),
        Some(tcl_registry::definer::DefinitionDispatcher::SnitType)
    );
    let unprovided = SourceCommandBindings::analyse(source, config, registry);
    assert!(
        unprovided
            .invocation_at_source("Dog", offset)
            .nominal_definition_name_result(registry)
            .is_none()
    );
    let retired_source = "package require snit\nsnit::type Dog {}\nDog destroy\nDog create d";
    let retired_offset = u32::try_from(retired_source.find("Dog create").unwrap()).unwrap();
    let retired = SourceCommandBindings::analyse_with_options(
        retired_source,
        config,
        registry,
        selected.options(),
    );
    assert!(
        retired
            .invocation_at_source("Dog", retired_offset)
            .nominal_definition_name_result(registry)
            .is_none()
    );
}

#[test]
fn rename_deletion_makes_old_name_opaque_flow_sensitively() {
    // `string` is its builtin before the rename, opaque after.
    let (cu, reg) = analyse("string toupper a\nrename string {}\nstring toupper b");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    assert!(cb.is_original_builtin_at(entry, 0, "string"));
    assert_eq!(
        cb.binding_at(entry, 2, "string").kind,
        BindingKind::Opaque,
        "string is renamed away before stmt 2"
    );
    assert!(cb.rebound_names().contains("::string"));
}

#[test]
fn rename_redirect_moves_binding_to_new_name() {
    let (cu, reg) = analyse("rename string mystr\nmystr toupper b");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    // After the rename: old `string` is opaque, `mystr` inherits the
    // builtin binding `string` denoted.
    assert_eq!(cb.binding_at(entry, 1, "string").kind, BindingKind::Opaque);
    assert_eq!(cb.binding_at(entry, 1, "mystr").kind, BindingKind::Builtin);
}

#[test]
fn proc_redefinition_binds_name_to_proc() {
    let (cu, reg) = analyse("proc string {x} { return $x }\nstring foo");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    let b = cb.binding_at(entry, 1, "string");
    assert_eq!(b.kind, BindingKind::Proc);
    assert_eq!(b.target.as_deref(), Some("::string"));
    assert!(!cb.is_original_builtin_at(entry, 1, "string"));
    assert!(cb.rebound_names().contains("::string"));
}

#[test]
fn dynamic_rename_collapses_to_wildcard() {
    let (cu, reg) = analyse("set x foo\nrename $x bar\nstring toupper a");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    assert!(cb.has_wildcard(), "dynamic rename sets the wildcard");
    // Under the wildcard everything resolves to Unknown (⊤), never a
    // concrete binding — so no spurious W128 can fire.
    let entry = fu.cfg.entry;
    assert_eq!(cb.binding_at(entry, 2, "string").kind, BindingKind::Unknown);
}

#[test]
fn expanded_embedded_rename_widens_module_bindings() {
    for (source, opaque) in [
        ("set ignored [rename {*}{llength saved_llength}]", false),
        ("set argv [read stdin]; set ignored [rename {*}$argv]", true),
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert_eq!(bindings.has_opaque_domain(), opaque, "{source}");
        if !opaque {
            assert!(
                bindings
                    .targets("::saved_llength", "::")
                    .iter()
                    .any(|target| target.command == "::llength"),
                "{source}"
            );
        }
    }
}

#[test]
fn expanded_embedded_command_head_widens_module_bindings() {
    for (source, opaque) in [
        ("set ignored [{*}{rename string saved_string}]", false),
        ("set argv [read stdin]; set ignored [{*}$argv]", true),
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert_eq!(bindings.has_opaque_domain(), opaque, "{source}");
        if !opaque {
            assert!(
                bindings
                    .targets("::saved_string", "::")
                    .iter()
                    .any(|target| target.command == "::string"),
                "{source}"
            );
        }
    }
}

#[test]
fn embedded_non_body_ensemble_leaf_preserves_command_trust() {
    let (cu, reg) = analyse("proc p {} {set x [namespace qualifiers ::a::b]}");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        !bindings.has_opaque_domain(),
        "a resolved non-body namespace leaf must not inherit the root's dynamic-body opacity"
    );
    let mutations = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(mutations.trusts("namespace"));
    assert!(mutations.trusts("self"));
}

#[test]
fn namespace_code_captures_direct_and_embedded_bodies_without_replaying_them() {
    for source in [
        "namespace code {rename set saved_set}",
        "set body {rename set saved_set}\nset callback [namespace code $body]",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "namespace code stores rather than executes its body: {source}"
        );
        assert!(
            !bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "a captured namespace code body must not mutate bindings now: {source}"
        );
    }
}

#[test]
fn evaluated_body_interpreter_realm_preserves_only_proven_parent_bindings() {
    for source in [
        "interp create slave\ninterp eval slave {set x 99}",
        "interp create slave\ninterp eval slave {puts child}",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a harmless readable body in a named child leaves the parent command table unchanged: {source}"
        );
    }

    for source in [
        "interp eval {} {rename set saved_set}",
        "set child slave\ninterp eval $child {set x 99}",
        "interp create slave\ninterp eval slave {rename set saved_set}",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "a current, unknown, or command-mutating child body must fail closed: {source}"
        );
    }
}

#[test]
fn namespace_resolution_projection_preserves_written_runs_and_constructed_colons() {
    for (pattern, expected) in [
        ("::source:::tail", ":::::tail"),
        ("::source::a:b", ":::::a:b"),
        ("::source::", ":::::"),
    ] {
        let transition = NamespaceTransition::Forget {
            namespace: tcl_registry::NamespaceTransitionTarget::Current,
            patterns: vec![TransitionSubject::Literal(pattern.to_owned())],
        };
        let mut rebound = std::collections::HashSet::new();
        let mut changed = false;
        let mut opaque = std::collections::HashSet::new();
        assert!(collect_namespace_resolution(
            &transition,
            Some(":::"),
            &mut rebound,
            &mut changed,
            &mut opaque,
        ));
        assert!(changed);
        assert!(opaque.is_empty());
        assert_eq!(
            rebound,
            std::collections::HashSet::from([expected.to_owned()])
        );
    }
}

#[test]
fn namespace_resolution_transitions_widen_but_creation_and_export_do_not() {
    for source in [
        "namespace eval ::n {}",
        "namespace eval ::n {namespace export *}",
        "set name v\nglobal $name",
        "set name v\nvariable $name",
        "set ns ::n\nnamespace upvar $ns v local",
        "set pattern *\nnamespace export $pattern",
    ] {
        let (cu, reg) = analyse(source);
        assert!(
            !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
            "a namespace or variable-cell transition that preserves lookup must stay closed: {source}"
        );
    }

    for source in [
        "namespace delete ::n",
        "namespace import -force ::m::*",
        "namespace forget ::m::*",
        "namespace path {::m}",
        "namespace unknown handler",
        "set value ::m\nnamespace path $value",
        "set value handler\nnamespace unknown $value",
    ] {
        let (cu, reg) = analyse(source);
        assert!(
            ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
            "a resolution-changing namespace transition must fail closed: {source}"
        );
    }
}

#[test]
fn namespace_resolution_impact_survives_embedding_and_alias_prefixes() {
    for source in [
        "set result [namespace delete ::n]",
        "set result [namespace import -force ::m::*]",
        "set result [namespace forget ::m::*]",
        "set result [namespace path {::m}]",
        "set result [namespace unknown handler]",
        "interp alias {} mutate {} namespace delete\nmutate ::n",
        "interp alias {} mutate {} namespace import -force\nmutate ::m::*",
        "interp alias {} mutate {} namespace forget\nmutate ::m::*",
        "interp alias {} mutate {} namespace path\nmutate {::m}",
        "interp alias {} mutate {} namespace unknown\nmutate handler",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "resolved nested dispatch must retain namespace lookup impact: {source}"
        );
        assert!(
            scan_module_command_mutations(&cu.ir_module, &reg).changes_command_resolution(),
            "resolved nested dispatch must reach the shared mutation projection: {source}"
        );
        assert!(
            !scan_module_command_mutations(&cu.ir_module, &reg).has_dynamic_mutation(),
            "namespace lookup provenance must not become an unrelated dynamic proc rebinding: {source}"
        );
    }

    for source in [
        "set result [namespace export *]",
        "interp alias {} ensure_n {} namespace eval ::n {}\nensure_n",
        "interp alias {} export_any {} namespace export\nexport_any *",
    ] {
        let (cu, reg) = analyse(source);
        assert!(
            !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
            "Ensure and Export preserve existing command resolution: {source}"
        );
    }
}

#[test]
fn external_unit_execution_widens_direct_and_embedded_command_bindings() {
    for source in [
        "source external.tcl",
        "set result [source external.tcl]",
        "load extension.so",
        "set result [load extension.so]",
        "auto_load widget",
        "set result [auto_load widget]",
        "auto_import pkg::*",
        "set result [auto_import pkg::*]",
        "package require Example",
        "set result [package require Example]",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "unavailable external code may mutate any command binding: {source}"
        );
    }
}

/// Run from a script, the stock `unknown` only tries `auto_load` and then
/// raises "invalid command name" (`library/init.tcl`, Tcl 8.4.20 through
/// 9.1.0). With no autoload index the analysis can see, an unresolved
/// dispatch defines nothing and the command table stays known. Writing an
/// index input, or a handler whose body is not the stock one, keeps the
/// table open.
#[test]
fn an_unresolved_dispatch_widens_only_through_a_visible_autoload_index() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let opaque = |source: &str| {
            let reg = CommandRegistry::build_default()
                .project_for_profile(tcl_dialect::DialectProfile::find(dialect).unwrap());
            let cu = CompilationUnit::build_for(source, &reg, false);
            ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain()
        };
        for source in [
            "unknown missing_command",
            "missing_command argument",
            "set result [missing_command argument]",
            "catch {missing_command argument}\nproc local_command {} {return ok}\nlocal_command",
            "rename unknown {}\nmissing_command argument",
            "interp alias {} unknown {} puts\nmissing_command argument",
        ] {
            assert!(!opaque(source), "{dialect}: {source}");
        }
        for source in [
            "set auto_index(missing_command) {source lib/missing.tcl}\nmissing_command",
            "lappend auto_path lib\nmissing_command",
            "proc unknown {command args} {rename $command saved_command}\nmissing_command",
        ] {
            assert!(opaque(source), "{dialect}: {source}");
        }
    }
}

#[test]
fn a_missing_root_binding_dispatches_through_the_live_unknown_handler() {
    let (cu, reg) = analyse(
        "rename unknown old_unknown\n\
             interp alias {} unknown {} rename expr\n\
             proc maybe {} {}\n\
             rename maybe {}\n\
             maybe",
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings.rebound_names().any(|name| name == "::expr"),
        "calling the missing command must execute the replacement unknown prefix and move expr"
    );
    assert!(
        bindings
            .targets("maybe", "::")
            .iter()
            .any(|target| target.command == "::expr"),
        "the unknown prefix must move expr onto the missing spelling"
    );

    let (cu, reg) = analyse("rename unknown {}\nmissing_command");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(bindings.targets("missing_command", "::").is_empty());
    assert!(!bindings.target_may_be_unknown("missing_command", "::"));
}

#[test]
fn indeterminate_ensemble_dispatch_widens_direct_and_embedded_bodies() {
    for source in [
        "set op eval\nnamespace $op ::n {rename set saved_set}",
        "set op eval\nset result [namespace $op ::n {rename set saved_set}]",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "a dynamic subcommand can select a body-bearing leaf: {source}"
        );
    }
}

#[test]
fn static_apply_replays_binding_changes_in_default_and_explicit_namespaces() {
    for (source, moved_name) in [
        ("apply {{} {rename set saved_set}}", "::saved_set"),
        (
            "namespace eval ::n {}\napply {{} {rename set saved_set} ::n}",
            "::n::saved_set",
        ),
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a literal apply lambda is exactly replayable: {source}"
        );
        assert!(
            bindings
                .targets(moved_name, "::")
                .iter()
                .any(|target| target.command == "::set"),
            "apply body did not move set to {moved_name}: {source}"
        );
    }
}

#[test]
fn dynamic_apply_lambda_keeps_the_command_domain_opaque() {
    for (source, opaque) in [
        (
            "set lambda {{} {rename set saved_set}}; apply $lambda",
            false,
        ),
        ("set lambda [read stdin]; apply $lambda", true),
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert_eq!(bindings.has_opaque_domain(), opaque, "{source}");
        if !opaque {
            assert!(
                bindings
                    .targets("::saved_set", "::")
                    .iter()
                    .any(|target| target.command == "::set"),
                "{source}"
            );
        }
    }
}

#[test]
fn alias_chain_prefix_replays_apply_body_after_arity_validation() {
    let source = "interp alias {} apply_saved {} apply {{} {rename set saved_set}}\n\
                      interp alias {} run_saved {} apply_saved\n\
                      run_saved";
    let (cu, reg) = analyse(source);
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(!bindings.has_opaque_domain());
    assert!(
        bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "the alias chain must supply apply's lambda"
    );

    for source in [
        "interp alias {} bad {} apply {{x} {rename set saved_set}}\nbad",
        "interp alias {} bad {} apply {{{x y z}} {rename set saved_set}}\nbad",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(!bindings.has_opaque_domain(), "known apply error: {source}");
        assert!(
            !bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "an arity or formal-list error must precede the body: {source}"
        );
    }
}

#[test]
fn static_namespace_eval_replays_binding_changes_in_target_namespace() {
    let (cu, reg) = analyse("namespace eval n { rename set saved_set }");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        !bindings.has_opaque_domain(),
        "a literal namespace and body are exactly replayable"
    );
    assert!(
        bindings
            .targets("::n::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "namespace eval body must run in ::n"
    );
}

#[test]
fn readable_nested_body_retains_intermediate_binding_history() {
    let (cu, reg) =
        analyse("namespace eval ::n { rename ::set ::saved_set; rename ::saved_set ::set }");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "the transient binding is observable while the evaluated body runs"
    );
    assert!(
        !bindings
            .root_boundary_bindings
            .get("::saved_set")
            .is_some_and(|states| states.iter().any(|state| matches!(
                state,
                MayBinding::Target(target) if target.command == "::set"
            ))),
        "transient history must not become a replayable root boundary"
    );
}

#[test]
fn dynamic_namespace_eval_target_or_body_keeps_the_domain_opaque() {
    for source in [
        "proc change {ns} {namespace eval $ns {rename set saved_set}}",
        "proc change {body} {namespace eval ::n $body}",
        "proc change {old} {namespace eval ::n [list rename $old saved_set]}",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "dynamic namespace eval input must not be replayed as source: {source}"
        );
    }
}

#[test]
fn frozen_namespace_eval_inputs_retain_the_actual_entered_rename() {
    for source in [
        "set ns ::n\nnamespace eval $ns {rename set saved_set}",
        "set body {rename set saved_set}\nnamespace eval ::n $body",
        "set old set\nnamespace eval ::n [list rename $old saved_set]",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::n::saved_set", "::")
                .iter()
                .any(|target| { target.command == "::set" }),
            "{source}"
        );
    }
}

#[test]
fn alias_chain_prefix_replays_namespace_eval_in_normalised_namespace() {
    let source = "interp alias {} in_n {} namespace eval n::\n\
                      interp alias {} run_in_n {} in_n\n\
                      run_in_n {rename set saved_set}";
    let (cu, reg) = analyse(source);
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(!bindings.has_opaque_domain());
    assert!(
        bindings
            .targets("::n::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "the alias chain must supply namespace eval's normalised target"
    );
}

#[test]
fn empty_namespace_eval_prefix_enters_the_global_body() {
    // Pinned C 8.4–9.1 accepts the empty namespace selector as global.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(profile).unit_profile();
        let reg = CommandRegistry::build_default().project_for_profile(profile);
        for (body, moved) in [("rename set saved_set", true), ("\"unterminated", false)] {
            let source =
                format!("interp alias {{}} in_empty {{}} namespace eval {{}}\nin_empty {{{body}}}");
            let cu = CompilationUnit::build_for_profile(&source, &reg, false, profile);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(!bindings.has_opaque_domain(), "{}: {body}", profile.name);
            assert_eq!(
                bindings
                    .targets("::saved_set", "::")
                    .iter()
                    .any(|target| target.command == "::set"),
                moved,
                "{}: {body}",
                profile.name,
            );
        }
    }
}

#[test]
fn static_delete_and_recreate_does_not_widen_unrelated_commands() {
    let (cu, reg) = analyse(
        "proc p {} {}\n\
             rename p {}\n\
             proc p {} { set local 1 }",
    );
    assert!(cu.ir_module.redefined_procedures.contains("::p"));
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        !bindings.has_opaque_domain(),
        "a readable discarded procedure body must not poison the command domain"
    );
    assert!(
        !bindings.target_may_be_unknown("set", "::"),
        "the unrelated set command remains exactly known"
    );
}

#[test]
fn absolute_heads_keep_relative_transition_operands_in_the_current_namespace() {
    let (cu, reg) = analyse("namespace eval ::n {::rename ::set saved_set}");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings
            .targets("::n::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "absolute head lookup must not replace the command's current namespace"
    );
    assert!(
        !bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "the relative destination must not be moved into the global namespace"
    );
}

#[test]
fn absolute_proc_head_keeps_current_namespace_for_discarded_body() {
    let (cu, reg) = analyse(
        "namespace eval ::n {\n\
                 ::proc p {} {::rename ::set saved_set}\n\
                 ::proc p {} {}\n\
             }",
    );
    assert!(cu.ir_module.redefined_procedures.contains("::n::p"));
    let history = discarded_procedure_history(&cu.ir_module, &reg);
    assert!(!history.opaque);
    assert!(
        history
            .modules
            .iter()
            .any(|module| module.top_level_namespace == "::n")
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings
            .targets("::n::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set")
    );
    assert!(
        !bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set")
    );
}

#[test]
fn consumed_typed_head_is_revalidated_in_future_roots() {
    let (cu, reg) = analyse(
        "proc ::uses_set {} { set x 1 }\n\
             rename ::set ::saved_set",
    );
    assert!(
        cu.ir_module
            .procedures
            .get(&fixture_id(&cu.ir_module.procedures, "::uses_set"))
            .expect("authored fixture body")
            .body
            .command_binding_sites
            .iter()
            .next()
            .is_some(),
        "structured lowering must retain the consumed set dependency"
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings.has_opaque_domain(),
        "the callable procedure may execute after set no longer denotes the compiled identity"
    );
}

#[test]
fn uplevel_global_proc_body_is_recovered_in_its_runtime_namespace() {
    for wrapper in [
        "uplevel #0 {proc p {} {rename expr saved_expr}}",
        "uplevel #0 {eval {proc p {} {rename expr saved_expr}}}",
    ] {
        let source = format!(
            "proc p {{}} {{}}\n\
                 namespace eval ::n {{\n\
                     proc installer {{}} {{ {wrapper} }}\n\
                 }}"
        );
        let (cu, reg) = analyse(&source);
        assert!(
            cu.ir_module.redefined_procedures.contains("::p"),
            "the #0 body is lowered in the same global namespace Tcl selects"
        );
        let history = discarded_procedure_history(&cu.ir_module, &reg);
        assert!(!history.opaque, "literal runtime body is exact: {wrapper}");
        assert!(
            history
                .modules
                .iter()
                .any(|module| module.top_level_namespace == "::"),
            "misqualified retained bodies must be re-lowered globally: {wrapper}"
        );
        assert!(history.rebound_names.contains("::p"));

        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "literal bodies should stay closed: {bindings:#?}"
        );
        assert!(
            bindings
                .targets("::saved_expr", "::")
                .iter()
                .any(|target| target.command == "::expr"),
            "the recovered body must mutate global expr, not ::n::expr: {wrapper}"
        );
        assert!(
            !scan_module_command_mutations(&cu.ir_module, &reg).trusts_proc_binding("::p"),
            "both runtime definitions of ::p make its identity unstable"
        );
    }
}

#[test]
fn unreadable_uplevel_global_proc_body_is_opaque() {
    let (cu, reg) = analyse(
        "namespace eval ::n {\n\
                 proc installer {body} { uplevel #0 {proc p {} $body} }\n\
             }",
    );
    let history = discarded_procedure_history(&cu.ir_module, &reg);
    assert!(
        history.opaque,
        "a misqualified retained procedure cannot borrow an unreadable lexical body"
    );
}

#[test]
fn unavailable_discarded_redefinition_body_keeps_the_domain_opaque() {
    let (cu, reg) = analyse(
        "proc install {replacement} {\n\
                 set p_name {p}\n\
                 proc p {} {}\n\
                 rename p {}\n\
                 proc $p_name {} $replacement\n\
             }\n\
             install {rename set saved_set}\n\
             p",
    );
    assert!(cu.ir_module.redefined_procedures.contains("::p"));
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings.has_opaque_domain(),
        "a runtime-supplied discarded body may mutate any command binding"
    );
}

#[test]
fn direct_and_alias_prefixed_proc_bodies_are_recovered_before_binding() {
    for source in [
        "proc p {} {rename expr saved_expr}\np",
        "interp alias {} makep {} proc p; makep {} {rename expr saved_expr}; p",
        "interp alias {} makep_target {} proc p\n\
             interp alias {} makep {} makep_target\n\
             makep {} {rename expr saved_expr}\n\
             p",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a literal typed proc body is exactly recoverable: {source}"
        );
        assert!(
            bindings
                .targets("::saved_expr", "::")
                .iter()
                .any(|target| target.command == "::expr"),
            "the recovered procedure body must contribute its expr rename: {source}"
        );
        let mutations = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !mutations.trusts("expr"),
            "compiler consumers must not trust expr after the recovered body: {source}"
        );
    }
}

#[test]
fn constructed_namespace_eval_defers_procedure_body_effects_to_the_root_fixpoint() {
    let (cu, reg) = analyse("namespace eval ::n [list proc mutate {} {rename ::set ::saved_set}]");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        !bindings.has_opaque_domain(),
        "the constructed script and procedure body are both exact"
    );
    assert!(
        bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "the constructed procedure must become a future executable root"
    );
}

#[test]
fn constructed_procedure_roots_discovered_by_a_root_reach_the_next_round() {
    let (cu, reg) = analyse(
        "namespace eval ::n [list proc installer {} {\
                 proc late {} {rename ::set ::saved_set}\
             }]",
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "a nested definition discovered while replaying installer must schedule late"
    );
}

#[test]
fn constructed_procedure_redefinitions_retain_every_possible_body() {
    let (cu, reg) = analyse(
        "namespace eval ::n [list proc mutate {} {rename ::set ::saved_set}]\n\
             namespace eval ::n [list proc mutate {} {rename ::expr ::saved_expr}]",
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    for (alias, target) in [("::saved_set", "::set"), ("::saved_expr", "::expr")] {
        assert!(
            bindings
                .targets(alias, "::")
                .iter()
                .any(|resolved| resolved.command == target),
            "every exact replacement body remains a possible future root: {alias}"
        );
    }
}

#[test]
fn constructed_namespaced_tcloo_method_uses_runtime_receiver_namespace() {
    let (cu, reg) = analyse(
        "namespace eval ::n { oo::class create C {\
                 method mutate {} {set x 1}\
             } }",
    );
    assert_eq!(
        cu.ir_module
            .methods
            .get(&fixture_id(&cu.ir_module.methods, "::n::C::mutate"))
            .expect("authored fixture body")
            .execution_namespace,
        crate::ir::ExecutionNamespace::RuntimeSelected
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings.has_opaque_domain(),
        "an object-local set may shadow the typed-lowered global builtin"
    );
}

#[test]
fn unretained_tcloo_body_keeps_the_command_domain_opaque() {
    let (cu, reg) = analyse(
        "set ::body {::rename ::set ::saved_set}\n\
             oo::class create C {method mutate {} $::body}",
    );
    assert!(cu.ir_module.oo_evidence.unretained_executable_roots);
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings.has_opaque_domain(),
        "a runtime-callable body absent from the retained root inventory must fail closed"
    );
    assert!(
        scan_module_command_mutations(&cu.ir_module, &reg).has_dynamic_mutation(),
        "the optimiser projection must retain closed-lattice opacity"
    );
}

#[test]
fn constructed_tcloo_method_keeps_absolute_command_heads_exact() {
    let (cu, reg) = analyse(
        "namespace eval ::n [list oo::class create C {\
                 method mutate {} {::rename ::set ::saved_set}\
             }]",
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "an absolute command and target remain namespace-invariant"
    );
}

#[test]
fn unavailable_direct_or_alias_prefixed_proc_body_is_opaque() {
    for source in [
        "set body {rename expr saved_expr}\nproc p {} $body\np",
        "set body {rename expr saved_expr}\n\
             interp alias {} makep {} proc p\n\
             makep {} $body\n\
             p",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "a procedure target without a retained or recovered body is opaque: {source}"
        );
        assert!(
            bindings.target_may_be_unknown("expr", "::"),
            "the unavailable procedure body must make expr untrusted: {source}"
        );
        assert!(
            !scan_module_command_mutations(&cu.ir_module, &reg).trusts("expr"),
            "the unavailable body makes the whole command-trust projection dynamic: {source}"
        );
    }
}

#[test]
fn try_handler_joins_command_mutation_from_exception_edge() {
    // A fall-through-capable try body may fail before or after the rename.
    // The analysis-only exception edges therefore contribute both the
    // pre-try Builtin state and the body-exit Opaque state to the handler.
    let (cu, reg) = analyse(
        "proc ::p {} {\n try {\n  rename string {}\n } on error {} {\n  string length abc\n }\n}",
    );
    let fu = cu.function("::p").unwrap();
    let handler = fu
        .cfg
        .blocks
        .iter()
        .find_map(|(&id, block)| block.name.starts_with("try_handler").then_some(id))
        .expect("try handler block");
    assert!(
        fu.cfg
            .exception_edges
            .iter()
            .any(|&(_, target)| target == handler),
        "the test must exercise an analysis-only exception edge"
    );

    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    assert_eq!(
        cb.binding_at(handler, 0, "string").kind,
        BindingKind::Unknown,
        "the handler must not infer either the original or renamed binding"
    );
    assert!(
        !cb.is_original_builtin_at(handler, 0, "string"),
        "an optimisation must not trust the builtin at handler entry"
    );
}

#[test]
fn seed_marks_module_procs_as_proc() {
    // The W128 seed: a name seeded as PROC resolves to Proc at entry.
    let (cu, reg) = analyse("nonbuiltin a b");
    let fu = cu.function("::top").unwrap();
    let seed = vec![(
        "::myproc".to_owned(),
        Binding {
            kind: BindingKind::Proc,
            target: Some("::myproc".to_owned()),
        },
    )];
    let cb = analyse_command_binding(&fu.cfg, &reg, &seed);
    assert_eq!(
        cb.binding_at(fu.cfg.entry, 0, "myproc").kind,
        BindingKind::Proc
    );
}

#[test]
fn module_mutations_distrust_rebound_builtins_only() {
    let reg = CommandRegistry::build_default();
    // A builtin renamed inside a proc body is distrusted everywhere
    // (over-approximation); a fresh user proc untrusts nothing.
    let cu = CompilationUnit::build_for(
        "proc clobber {} { rename string {} }\nproc myproc {} { return 1 }",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.trusts("string"), "string is rebound in a proc body");
    assert!(m.trusts("lappend"), "an untouched builtin stays trusted");
    assert!(m.trusts("myproc"), "a fresh user proc untrusts nothing");

    // An actual unknown Value formal keeps the mutation subject unknown.
    let cu2 = CompilationUnit::build_for("proc clobber {x} {rename $x bar}", &reg, false);
    let m2 = scan_module_command_mutations(&cu2.ir_module, &reg);
    assert!(!m2.trusts("string") && !m2.trusts("lappend"));

    // A frozen absent subject errors before moving any command. Its
    // written substitution does not turn the subject into a dynamic one.
    let cu3 = CompilationUnit::build_for("set x foo\nrename $x bar", &reg, false);
    let m3 = scan_module_command_mutations(&cu3.ir_module, &reg);
    assert!(m3.trusts("string") && m3.trusts("lappend"));
}

/// `walk_body_calls` recurses once
/// per nested `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`
/// body, with no depth cap of its own. Transitively
/// bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass,
/// so this is defence-in-depth / consistency with every other
/// full-tree walker in this crate, not a currently-reproducible
/// crash. 1000 levels of source nesting is comfortably past this
/// cap; the assertion is that `scan_module_command_mutations` returns
/// at all, not what it returns. Spawns its own big-stack thread since
/// the lexer/CST/segmenter stages upstream of the lowering cap still
/// walk the full un-truncated source nesting before that cap trims
/// it — same rationale as
/// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
#[test]
fn deeply_nested_if_survives_walk_body_calls() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut src = "proc clobber {} {\n".to_owned();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("rename string {}\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    src.push_str("}\n");
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let reg = CommandRegistry::build_default();
            let cu = CompilationUnit::build_for(&src, &reg, false);
            let _ = scan_module_command_mutations(&cu.ir_module, &reg);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn trusts_proc_binding_true_for_untouched_proc() {
    // TP control: a proc never named by any `rename` / `interp alias`
    // is trusted.
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for("proc myproc {} { return 1 }", &reg, false);
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(m.trusts_proc_binding("myproc"));
    assert!(m.trusts_proc_binding("::myproc"));
}

#[test]
fn trusts_proc_binding_false_for_rename_source_and_target() {
    // FP guard: `rename triple double` perturbs BOTH names — `triple`
    // (vacated, no longer denotes what it did) and `double` (now
    // denotes `triple`'s body, not `double`'s own declaration).
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple double\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.trusts_proc_binding("double"), "rename target");
    assert!(!m.trusts_proc_binding("triple"), "rename source");
}

#[test]
fn trusts_proc_binding_false_for_interp_alias_name() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc answer {} { return 42 }\nproc other {} { return 99 }\ninterp alias {} answer {} other\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.trusts_proc_binding("answer"));
    // The alias *target* itself is untouched — still trusted.
    assert!(m.trusts_proc_binding("other"));
}

#[test]
fn trusts_proc_binding_follows_alias_resolved_mutations() {
    let reg = CommandRegistry::build_default();
    let renamed = CompilationUnit::build_for(
        "proc p {} {return P}\nproc q {} {return Q}\n\
             interp alias {} r {} rename\n\
             r p oldp\nr q p\n",
        &reg,
        false,
    );
    let mutations = scan_module_command_mutations(&renamed.ir_module, &reg);
    for name in ["p", "q", "oldp"] {
        assert!(
            !mutations.trusts_proc_binding(name),
            "alias-resolved rename touched {name}"
        );
    }

    let aliased = CompilationUnit::build_for(
        "proc p {} {return P}\nproc q {} {return Q}\n\
             interp alias {} replace {} interp alias {} p {} q\n\
             replace\n",
        &reg,
        false,
    );
    let mutations = scan_module_command_mutations(&aliased.ir_module, &reg);
    assert!(!mutations.trusts_proc_binding("p"));
    assert!(mutations.trusts_proc_binding("q"));
}

#[test]
fn trusts_proc_binding_unaffected_by_unrelated_rename() {
    // TN control: renaming a DIFFERENT proc must not untrust this one —
    // `trusts_proc_binding` is per-name, unlike the whole-module
    // `dynamic` wildcard.
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple somethingElse\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(m.trusts_proc_binding("double"));
}

/// A fully-qualified `proc ::n::expr` shadows the builtin for bodies that
/// resolve in `::n`, so the builtin is no longer foldable (#2159).
///
/// The two spellings used to disagree: `namespace eval ::n { proc expr … }`
/// distrusted `expr`, while `proc ::n::expr` did not, because the recorded
/// name is `::n::expr` and only a name whose *own* default is a builtin was
/// collected. Measured harm before the fix — `tcl explore --show opt`
/// rewrote
///
/// ```text
/// proc ::n::f {r} { return [expr {$r ** 2}] }
/// ```
///
/// into `[expr {$r * $r}]`, and with `proc ::n::expr {s} {return "SHADOW:$s"}`
/// that changes the program's output from `SHADOW:$r ** 2` to
/// `SHADOW:$r * $r` on tclsh 9.0.4.
#[test]
fn qualified_proc_shadowing_a_builtin_distrusts_it() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::n {}\nproc ::n::expr {s} { return $s }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(
        !m.trusts("expr"),
        "a qualified shadow distrusts the builtin"
    );
}

/// A qualified shadow installed through an alias is distrusted too.
///
/// `interp alias {} make {} proc ::n::expr` carries the prepended name,
/// so the source scan never sees the spelling `proc ::n::expr` — but
/// `ModuleCommandBindings` recovers the definition, and the same tail
/// projection applies there. tclsh 9.0.4 prints `SHADOW:$r ** 2` for the
/// program below; without this, O110 still rewrote the body to
/// `[expr {$r * $r}]`.
#[test]
fn an_alias_installed_qualified_shadow_is_distrusted() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::n {}\ninterp alias {} make {} proc ::n::expr\nmake {s} { return \"SHADOW:$s\" }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(
        !m.trusts("expr"),
        "an alias-installed qualified shadow distrusts the builtin",
    );
}

/// And an alias installing something whose tail is not a builtin leaves
/// trust alone.
#[test]
fn an_alias_installing_a_non_builtin_tail_keeps_trust() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::n {}\ninterp alias {} make {} proc ::n::helper\nmake {s} { return $s }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(m.trusts("expr"), "an unrelated alias keeps trust");
}

/// The same, for a command other than `expr` — the scan is keyed on the
/// tail having a builtin default, not on any one name.
#[test]
fn qualified_shadow_distrust_is_not_specific_to_expr() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::n {}\nproc ::n::llength {s} { return 99 }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.trusts("llength"), "any shadowed builtin tail distrusts");
    assert!(m.trusts("expr"), "and only the one that was shadowed");
}

/// A qualified `proc` whose tail is *not* a builtin changes nothing —
/// the guard must not withdraw folding from every namespaced file.
#[test]
fn qualified_proc_with_a_non_builtin_tail_keeps_trust() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::n {}\nproc ::n::helper {s} { return $s }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(m.trusts("expr"), "an unrelated qualified proc keeps trust");
}

#[test]
fn rebound_candidates_include_relative_qualified_names() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    let namespace = crate::ir::ExecutionNamespace::exact("::ns");
    for (written, expected) in [
        ("inner::p", vec!["::inner::p", "::ns::inner::p"]),
        ("p", vec!["::ns::p", "::p"]),
        ("::inner::p", vec!["::inner::p"]),
    ] {
        let mut bindings = ModuleCommandBindings::default();
        bindings.record_proc_rebound_candidates(written, &namespace);
        assert_eq!(
            bindings.proc_rebound_names,
            expected.into_iter().map(String::from).collect()
        );
        assert!(bindings.bindings.is_empty());
    }
}

#[test]
fn trusts_proc_binding_false_for_namespace_relative_rename() {
    // FP guard (reported in code review): a bare `rename` argument
    // inside a namespaced proc resolves relative to THAT proc's own
    // namespace, not the global namespace — `rename triple double`
    // inside `proc ::ns::doit` renames `::ns::triple` onto
    // `::ns::double`. An earlier version always rooted the bare names
    // globally (`::triple`/`::double`), so it never distrusted the
    // actually-affected namespaced names.
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::ns {\n    proc double {n} { expr {$n * 2} }\n    proc triple {n} { expr {$n * 3} }\n}\nproc ::ns::doit {} { rename triple double }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(
        !m.trusts_proc_binding("::ns::double"),
        "namespace-relative rename target"
    );
    assert!(
        !m.trusts_proc_binding("::ns::triple"),
        "namespace-relative rename source"
    );
    // The scan is flow-insensitive (it can't know whether `double`/
    // `triple` already existed as GLOBAL commands at the point
    // `::ns::doit` runs), so it conservatively distrusts the
    // global-rooted candidate too — a deliberate, sound
    // over-approximation (a missed fold, never a wrong one),
    // mirroring `collect_tampered_builtins`'s existing philosophy.
    assert!(
        !m.trusts_proc_binding("::double"),
        "global-rooted candidate"
    );
    assert!(
        !m.trusts_proc_binding("::triple"),
        "global-rooted candidate"
    );
}

// Regression: `proc max {...}` must not be distrusted. `max`/`min` read
// like `tcl::mathop` operator words, but real Tcl never registered them
// there (verified against tclsh 8.6/9.0 — `info commands
// ::tcl::mathop::*` never lists them); they exist only as unrelated
// `expr` math functions. A now-fixed registry bug once carried bare
// `max`/`min` `CommandSpec` entries as if they were `tcl::mathop`
// members, which made `default_binding` treat them as pre-existing
// builtins — so a completely ordinary `proc max {...}` looked like it
// was "renaming a builtin", silently blocking O103 from folding calls to
// it (caught by `tests/optimiser.rs::interprocedural_constant_folding`).
#[test]
fn module_mutations_do_not_distrust_proc_named_like_mathop_word() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc max {a b} {\n    if {$a > $b} { return $a } else { return $b }\n}\nset v [max 3 7]\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(
        m.trusts("max"),
        "a plain proc sharing a name with an (incorrectly bare-registered) \
             tcl::mathop-lookalike must not be distrusted"
    );
}
