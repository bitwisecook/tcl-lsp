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
use crate::command_binding::SourceAnalysisOptions;
use tcl_registry::native_compilation::{NativeCompilationContext, NativeCompilationFrame};

fn analyse(
    source: &str,
) -> crate::environment_ingress::RetainedNativeFixture<SourceCommandBindings> {
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::default(),
        &tcl_registry::CommandRegistry::build_default(),
        SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(tcl_native_model::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V8_4,
            )),
            native_compilation: NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ProcedureCode,
                loop_depth: 0,
                catch_depth: Some(0),
            },
            ..SourceAnalysisOptions::default()
        },
    );
    entry.retain(bindings)
}

#[test]
fn original_return_preparation_uses_selected_stack_options_before_result() {
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry: crate::environment_ingress::CapturedNativeEntry =
            crate::environment_ingress::captured_native_entry(profile);
        let dialect = tcl_native_model::InvocationDialect::for_version(version);
        let mut sources = vec!["return [set result BODY]", "return"];
        if version >= tcl_dialect::TclVersion::V8_5 {
            sources.extend([
                "return -level 0 -code error [set result BODY]",
                "return -options [set options $opts] [set result BODY]",
            ]);
        }
        if version >= tcl_dialect::TclVersion::V8_6 {
            sources.push("return -code [set code $code] [set result BODY]");
        }
        for source in sources {
            let inventory = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
                SourceAnalysisOptions {
                    native_entry: Some(&entry),
                    invocation_dialect: Some(dialect),
                    native_compilation: NativeCompilationContext {
                        mode: NativeCompilationMode::BytecodeObject,
                        frame: NativeCompilationFrame::ProcedureCode,
                        loop_depth: 0,
                        catch_depth: Some(0),
                    },
                    ..Default::default()
                },
            );
            assert!(
                !inventory.native_compilation_provider_required_at(0),
                "{version:?} {source}"
            );
            assert!(
                inventory.native_compilation_failure_at(0).is_none(),
                "{version:?} {source}"
            );
        }
    }
}

#[test]
fn original_set_preparation_visits_only_native_target_and_value_regions() {
    for source in [
        "set x VALUE",
        "set x",
        "set {a([incr bad extra args])} VALUE",
    ] {
        let inventory = analyse(source);
        assert!(
            !inventory.native_compilation_provider_required_at(0),
            "{source}"
        );
        assert!(
            inventory.native_compilation_failure_at(0).is_none(),
            "{source}"
        );
    }
    for source in [
        "set x [incr bad extra args]",
        "set a([incr bad extra args]) VALUE",
        "set $a([incr bad extra args]) VALUE",
    ] {
        let inventory = analyse(source);
        assert!(
            inventory.native_compilation_failure_at(0).is_some(),
            "{source}"
        );
        assert!(
            !inventory.native_compilation_provider_required_at(0),
            "{source}"
        );
    }
}

#[test]
fn original_each_rejection_precedes_legacy_arity_projection() {
    let source = "set before 1; foreach \"{i\" {A} {set never 1}";
    let inventory = analyse(source);
    let failure = inventory
        .native_compilation_failure_at(0)
        .expect("native varlist compilation rejection");
    assert_eq!(
        failure.failure.message.as_deref(),
        Some("unmatched open brace in list")
    );
    assert!(!inventory.has_site(0));
    assert!(!inventory.native_compilation_provider_required_at(0));
    assert_ne!(
        failure.dependencies,
        [] as [crate::command_binding::compiled_preflight::SourceNativeCompilationDependency; 0]
    );
}

#[test]
fn original_empty_each_declines_without_manufacturing_a_compile_failure() {
    let inventory = analyse("foreach {} {A} {set never 1}");
    assert!(inventory.native_compilation_failure_at(0).is_none());
    assert!(!inventory.native_compilation_provider_required_at(0));
    assert_eq!(
        inventory
            .invocation_at_source("foreach", 0)
            .native_compilation_admission_selection(),
        NativeCompilationSelection::Generic
    );
}

#[test]
fn original_c84_catch_rolls_back_a_rejected_body_without_granting_child_admission() {
    let source = "catch {incr x bad extra} result; set after 1";
    let inventory = analyse(source);
    assert!(inventory.native_compilation_failure_at(0).is_none());
    assert!(!inventory.native_compilation_provider_required_at(0));
    assert_eq!(
        inventory
            .invocation_at_source("catch", 0)
            .native_compilation_admission_selection(),
        NativeCompilationSelection::Generic
    );
    assert!(
        inventory
            .invocation_at_source("incr", 7)
            .admitted_inline_invocation()
            .is_none()
    );
}

#[test]
fn escaped_c84_head_does_not_donate_an_early_set_arity_failure() {
    let plain = analyse("set ::before 1; set");
    assert!(plain.native_compilation_failure_at(0).is_some());
    let escaped = analyse(r"set ::before 1; se\x74");
    assert!(escaped.native_compilation_failure_at(0).is_none());
    assert!(!escaped.native_compilation_provider_required_at(0));
    let binding = escaped.invocation_at_source("set", 16);
    assert_eq!(
        binding.native_compilation_selection(),
        NativeCompilationSelection::Generic
    );
}

#[test]
fn compiler_visits_runtime_dead_expression_substitutions_and_array_indices() {
    for source in [
        "set before 1; expr {0 && [incr bad extra args]}",
        "set before 1; expr {1 ? 1 : [incr bad extra args]}",
        "set before 1; set x $a([incr bad extra args])",
        "set before 1; expr {1 +}",
    ] {
        let analysis = analyse(source);
        assert!(
            analysis.native_compilation_failure_at(0).is_some(),
            "{source}"
        );
        assert!(!analysis.has_site(0), "prefix executed: {source}");
    }
}

#[test]
fn unsupported_expression_compile_success_retains_an_entry_obligation() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let mut entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    entry.math_functions.as_mut().unwrap().closed = false;
    let analysis = SourceCommandBindings::analyse_with_options(
        "set before 1; expr {future_function(1)}",
        tcl_lexer::LexerConfig::default(),
        &tcl_registry::CommandRegistry::build_default(),
        SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(tcl_native_model::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V8_4,
            )),
            native_compilation: NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ProcedureCode,
                loop_depth: 0,
                catch_depth: Some(0),
            },
            ..SourceAnalysisOptions::default()
        },
    );
    assert!(analysis.native_compilation_failure_at(0).is_none());
    assert!(analysis.native_compilation_provider_required_at(0));
}

#[test]
fn definite_compile_error_prevents_earlier_effects_and_dead_code_execution() {
    let source = "rename set saved; return; set x extra bad";
    let analysis = analyse(source);
    let failure = analysis.native_compilation_failure_at(0).unwrap();
    assert_eq!(
        failure.failure.message.as_deref(),
        Some("wrong # args: should be \"set varName ?newValue?\"")
    );
    assert!(!analysis.has_site(0));
    assert!(
        super::super::source_binding(&analysis.final_state, "set", "::")
            .proved_target()
            .is_some_and(|target| target.registry_backed)
    );
}

#[test]
fn compiler_prunes_only_native_literal_boolean_bodies() {
    for source in [
        "set x 1; if {0} {set x extra bad}",
        "set x 1; while {0} {set x extra bad}",
    ] {
        assert!(
            analyse(source).native_compilation_failure_at(0).is_none(),
            "{source}"
        );
    }
    for source in [
        "set x 1; if {0+0} {set x extra bad}",
        "set x 1; foreach i {} {set x extra bad}",
    ] {
        assert!(
            analyse(source).native_compilation_failure_at(0).is_some(),
            "{source}"
        );
    }
}

#[test]
fn decoded_escape_values_do_not_acquire_literal_compiler_tokens() {
    for source in [r#"if "\61""#, r"if \61", r"i\146 {1}"] {
        assert!(
            analyse(source).native_compilation_failure_at(0).is_none(),
            "{source}"
        );
    }
    for source in [r#""if" {1}"#, "{if} {1}"] {
        assert!(
            analyse(source).native_compilation_failure_at(0).is_some(),
            "{source}"
        );
    }
}

#[test]
fn conditional_failure_keeps_authored_operand_quoting() {
    for (source, operand) in [("if {1}", "{1}"), ("if \"1\"", "\"1\""), ("if 1", "1")] {
        let analysis = analyse(source);
        let failure = analysis.native_compilation_failure_at(0).expect(source);
        assert_eq!(
            failure.failure.message.as_deref(),
            Some(format!("wrong # args: no script following \"{operand}\" argument").as_str()),
        );
    }
    for source in ["if", "{if}", "\"if\""] {
        let analysis = analyse(source);
        assert_eq!(
            analysis
                .native_compilation_failure_at(0)
                .unwrap()
                .failure
                .message
                .as_deref(),
            Some("wrong # args: no expression after \"if\" argument"),
        );
    }
}

#[test]
fn caught_child_compiler_failure_falls_back_without_rejecting_the_parent() {
    let source = "set x 1; catch {set x extra bad}; set y 2";
    let analysis = analyse(source);
    assert!(analysis.native_compilation_failure_at(0).is_none());
    let catch_offset = u32::try_from(source.find("catch").unwrap()).unwrap();
    let catch = analysis.invocation_at_source("catch", catch_offset);
    assert_eq!(
        catch.compiled_candidates,
        [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
    );
    assert!(catch.may_use_live_dispatch);
    let last = u32::try_from(source.rfind("set y").unwrap()).unwrap();
    assert!(
        analysis
            .invocation_at_source("set", last)
            .proved_execution_target()
            .is_some()
    );
}

#[test]
fn failure_retains_native_dependencies_and_original_nested_contexts() {
    let source = "set before 1\nif {1} {\n    set x extra bad\n}";
    let analysis = analyse(source);
    let failure = analysis.native_compilation_failure_at(0).unwrap();
    assert_eq!(failure.failure.error_code.as_deref(), Some("NONE"));
    assert_eq!(failure.contexts.len(), 2);
    assert_eq!(
        failure.contexts[0].command.as_deref(),
        Some("set x extra bad")
    );
    assert_eq!(failure.contexts[0].line_in_chunk, Some(3));
    assert_eq!(
        failure.contexts[1].command.as_deref(),
        Some("if {1} {\n    set x extra bad\n}")
    );
    assert_eq!(failure.contexts[1].line_in_chunk, Some(2));
    assert_eq!(failure.dependencies.len(), 2);
    assert!(failure.dependencies.iter().all(|dependency| {
        dependency.target.registry_backed
            && dependency.guard
                == tcl_registry::native_compilation::NativeCompilationGuard::ChunkEntry
    }));
    assert_eq!(failure.dependencies[0].head, "set");
    assert_eq!(failure.dependencies[1].head, "if");
    assert!(!analysis.has_site(0));
}

#[test]
fn parent_compiled_child_retains_admission_when_runtime_fallback_is_opaque() {
    for body in ["set selected YES", "set ::a(new) NEW"] {
        assert_original_child_compilation(body);
    }
}

fn assert_original_child_compilation(body: &str) {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    let source = format!("if {{$flag}} {{{body}}}");
    let compilation = tcl_registry::native_compilation::NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let analysed = SourceCommandBindings::analyse_with_options(
        &source,
        config,
        &registry,
        SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
            native_compilation: compilation,
            ..Default::default()
        },
    );
    let offset = u32::try_from(source.find(body).unwrap()).unwrap();
    let original = analysed
        .invocation_at_source("set", offset)
        .native_compilation_admission_selection();
    assert_ne!(
        original,
        tcl_registry::native_compilation::NativeCompilationSelection::Unknown
    );
    let namespace = tcl_names::namespace::SourceNamespaceKey::from_native_entry(&entry).unwrap();
    let frame = tcl_names::variable::VariableExecutionFrame::Global
        .with_namespace_identity(namespace.clone());
    let context = original_child_context(compilation, config, &registry, offset, &frame);

    assert_authored_child_context_is_unknown(&analysed, body, context);
    for same_source in [true, false] {
        let mut inventory = analysed.clone();
        let mut late = analysed.final_state.as_ref().clone();
        if !same_source {
            late.current_source_origin = Some(Arc::new(
                tcl_names::origin::SourceOriginId::authored(&Arc::from(format!("{source}\n"))),
            ));
            inventory
                .root_origin
                .clone_from(&late.current_source_origin);
        }
        late.mark_opaque_binding_mutation();
        inventory.walk_source(body, offset, &mut late, &context);
        let site = CommandAllocationSite {
            source: Arc::clone(inventory.root_origin.as_ref().unwrap()),
            offset,
        };
        assert!(inventory.compilation_provider_required.contains(&site));
        assert_eq!(
            inventory.native_compilation_provider_required_at(offset),
            !same_source
        );
        assert!(late.has_opaque_domain());
        let binding = inventory.invocation_at_source("set", offset);
        assert_eq!(
            binding.native_compilation_admission_selection(),
            if same_source {
                original
            } else {
                tcl_registry::native_compilation::NativeCompilationSelection::Unknown
            }
        );
        assert!(binding.proved_execution_target().is_none());
        if same_source {
            // An actual new compilation of the enclosing command with a
            // missing parent slot cannot borrow the old child's recipe.
            let mut reentry = analysed.final_state.as_ref().clone();
            reentry.remove(tcl_names::command::SourceCommandKey::slot(
                namespace.clone(),
                "if".into(),
            ));
            inventory.walk_source(
                &source,
                0,
                &mut reentry,
                &SourceExecutionContext {
                    invocation_offset: 0,
                    ..context
                },
            );
            assert_eq!(
                inventory
                    .invocation_at_source("set", offset)
                    .native_compilation_admission_selection(),
                tcl_registry::native_compilation::NativeCompilationSelection::Unknown
            );
        }
    }
}

fn assert_authored_child_context_is_unknown(
    analysed: &SourceCommandBindings,
    body: &str,
    context: SourceExecutionContext<'_>,
) {
    // An authored presentation of the same namespace cannot stand in for
    // the original native entry, even when the source image is unchanged.
    let mut inventory = analysed.clone();
    let mut state = analysed.final_state.as_ref().clone();
    state.mark_opaque_binding_mutation();
    inventory.walk_source(
        body,
        context.invocation_offset,
        &mut state,
        &SourceExecutionContext {
            frame: &tcl_names::variable::VariableExecutionFrame::Global,
            namespace_key: None,
            ..context
        },
    );
    let binding = inventory.invocation_at_source("set", context.invocation_offset);
    assert_eq!(
        binding.native_compilation_admission_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Unknown
    );
    assert!(binding.proved_execution_target().is_none());
}

fn original_child_context<'a>(
    compilation: tcl_registry::native_compilation::NativeCompilationContext,
    config: tcl_lexer::LexerConfig,
    registry: &'a tcl_registry::CommandRegistry,
    offset: u32,
    frame: &'a tcl_names::variable::VariableExecutionFrame,
) -> SourceExecutionContext<'a> {
    SourceExecutionContext {
        original_logical_body: None,
        control: crate::command_binding::source_execution_control::SourceExecutionControl::new(
            compilation,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
            crate::command_binding::SourceBodyEntryPurpose::Selected,
            tcl_dialect::model::InvocationRealm::RuleLoader,
        ),
        compilation_snapshot: None,
        selected_compilation: None,
        original_variable_compilation: None,
        namespace: "::",
        config,
        registry,
        depth: 0,
        frame,
        invocation_offset: offset,
        variable_read_owner: None,
        original_written_projection: None,
        written_arguments: None,
        written_values: None,
        written_name_values: None,
        written_representations: None,
        written_objects: None,
        written_method_prefixes: None,
        written_variable_reads: None,
        namespace_key: frame.namespace_identity(),
        expression_source: None,
    }
}

#[test]
fn compilation_error_observers_mutate_before_catch_continuation() {
    for (subject, mode) in [
        ("::errorCode", "w"),
        ("::errorInfo", "w"),
        ("::unrelated", "w"),
        ("::errorCode", "r"),
        ("::errorInfo", "r"),
    ] {
        let source = format!(
            "proc observer {{args}} {{proc set {{args}} {{return CUSTOM}}}}\n\
                 trace variable {subject} {mode} observer\n\
                 proc p {{}} {{set ::before 1; set x extra bad}}\n\
                 catch {{p}}\nset x 1"
        );
        let analysis = SourceCommandBindings::analyse_with_options(
            &source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_native_model::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_4,
                )),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..SourceAnalysisOptions::default()
            },
        );
        let offset = u32::try_from(source.rfind("set x 1").unwrap()).unwrap();
        let binding = analysis.invocation_at_source("set", offset);
        let states = analysis
            .points
            .iter()
            .filter(|point| point.dispatch)
            .map(|point| {
                (
                    point.head.as_deref(),
                    point.offset,
                    point.state.opaque_domain,
                    point.state.source_variables.dynamic_traces,
                    point.state.source_variables.traced.len(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            binding.proved_execution_target().is_some(),
            subject == "::unrelated" || mode == "r",
            "{subject} {mode}: lookup unknown={}, compile unknown={}, states={states:?}",
            binding.unknown,
            binding.compiled_execution_unknown(),
        );
    }
}
