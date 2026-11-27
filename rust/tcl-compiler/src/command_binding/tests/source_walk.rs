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

//! Source interpretation: origins, derived scripts, lambdas, abrupt completions and imports.

use super::*;
use tcl_names::origin::SourceOriginKind;

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
            invocation_dialect: Some(tcl_native_model::InvocationDialect::for_version(
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
            tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
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
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
            tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
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
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
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
            tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
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
        tcl_native_model::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
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
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
            tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
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
                tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
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
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert!(binding.proved_target().is_none());
}
