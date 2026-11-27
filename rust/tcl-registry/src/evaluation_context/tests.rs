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
use crate::{Arity, CommandRegistry, CommandSpec, InvocationWord, InvocationWords};

#[test]
fn missing_context_is_unknown_and_never_inferred_from_body_role() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "unmodelled",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Body)],
        ..CommandSpec::DEFAULT
    });
    let invocation = registry
        .resolve_invocation("unmodelled", &["body"], None)
        .unwrap();
    let contexts = invocation.argument_evaluation_contexts().unwrap();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].context, None);
    assert_eq!(contexts[0].interpreter, InterpreterScope::Any);
}

#[test]
fn namespace_operand_and_interpreter_selectors_are_shifted_once() {
    let registry = CommandRegistry::build_default();
    let namespace = registry
        .resolve_invocation("namespace", &["eval", "::n", "body"], None)
        .unwrap();
    let contexts = namespace.argument_evaluation_contexts().unwrap();
    let body = contexts
        .iter()
        .find(|context| context.role == ArgRole::Body)
        .unwrap();
    assert_eq!(body.argument, 2);
    assert_eq!(
        body.context.unwrap().frame,
        EvaluationFrame::NamespaceArgument(0)
    );
    assert_eq!(
        body.frame_subject,
        Some(TransitionSubject::LocatedLiteral {
            value: "::n".to_owned(),
            argument_index: 1
        })
    );
    for (path, expected) in [
        ("", InterpreterScope::Current),
        ("child", InterpreterScope::named("child")),
    ] {
        let arguments = ["eval", path, "body"];
        let invocation = registry
            .resolve_invocation("interp", &arguments, None)
            .unwrap();
        let contexts = invocation.argument_evaluation_contexts().unwrap();
        let body = contexts
            .iter()
            .find(|context| context.role == ArgRole::Body)
            .unwrap();
        assert_eq!(body.argument, 2);
        assert_eq!(body.interpreter, expected);
        assert_eq!(body.context.unwrap().frame, EvaluationFrame::Global);
    }
}

#[test]
fn dynamic_interpreter_selector_stays_unknown_and_expansion_refuses_layout() {
    let registry = CommandRegistry::build_default();
    let arguments = [
        InvocationWord::Literal("eval"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("body"),
    ];
    let invocation = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("interp"), &arguments),
            None,
        )
        .resolved()
        .unwrap();
    let contexts = invocation.argument_evaluation_contexts().unwrap();
    assert_eq!(
        contexts
            .iter()
            .find(|context| context.role == ArgRole::Body)
            .unwrap()
            .interpreter,
        InterpreterScope::Any
    );
    let expanded = [InvocationWord::Literal("eval"), InvocationWord::Expanded];
    let invocation = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("interp"), &expanded),
            None,
        )
        .resolved()
        .unwrap();
    assert!(invocation.argument_evaluation_contexts().is_none());
}

#[test]
fn alias_source_and_target_keep_distinct_interpreter_coordinates() {
    let registry = CommandRegistry::build_default();
    let invocation = registry
        .resolve_invocation(
            "interp",
            &["alias", "source", "alias", "target", "handler"],
            None,
        )
        .unwrap();
    let source = invocation.argument_evaluation_context(2).unwrap();
    let target = invocation.argument_evaluation_context(4).unwrap();
    assert_eq!(source.interpreter, BodyInterpreter::Argument(0));
    assert_eq!(target.interpreter, BodyInterpreter::Argument(2));
    assert_eq!(source.timing, ScriptTiming::ReferenceOnly);
    assert_eq!(target.timing, ScriptTiming::Deferred);
    let contexts = invocation.argument_evaluation_contexts().unwrap();
    assert_eq!(
        contexts
            .iter()
            .find(|context| context.role == ArgRole::CommandPrefix)
            .unwrap()
            .interpreter,
        InterpreterScope::named("target")
    );
}

#[test]
fn hidden_invocation_global_takes_precedence_and_unknown_options_stay_unknown() {
    let registry = CommandRegistry::build_default();
    let invocation = registry
        .resolve_invocation(
            "interp",
            &[
                "invokehidden",
                "child",
                "-namespace",
                "::n",
                "-global",
                "handler",
            ],
            None,
        )
        .unwrap();
    assert_eq!(
        invocation.argument_evaluation_context(5).unwrap().frame,
        EvaluationFrame::Global
    );
    let invocation = registry
        .resolve_invocation(
            "interp",
            &["invokehidden", "child", "-namespace", "::n", "handler"],
            None,
        )
        .unwrap();
    assert_eq!(
        invocation.argument_evaluation_context(4).unwrap().frame,
        EvaluationFrame::NamespaceArgument(2)
    );
    let arguments = [
        InvocationWord::Literal("invokehidden"),
        InvocationWord::Literal("child"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("handler"),
    ];
    let invocation = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("interp"), &arguments),
            None,
        )
        .resolved()
        .unwrap();
    assert_eq!(
        invocation.argument_evaluation_context(3).unwrap().frame,
        EvaluationFrame::Unknown
    );
}

#[test]
fn authoritative_context_resolver_never_falls_back_to_caller_defaults() {
    fn unknown_context(arguments: InvocationArguments<'_>, _: usize) -> Option<EvaluationContext> {
        match arguments.literal_at(0) {
            Some("caller") => Some(EvaluationContext::INVOKING),
            None => Some(EvaluationContext {
                frame: EvaluationFrame::Unknown,
                ..EvaluationContext::INVOKING
            }),
            Some(_) => None,
        }
    }
    let descriptor = EvaluationContextSpec {
        resolver: Some(unknown_context),
        resolver_roles: &[ArgRole::Body],
        ..EvaluationContextSpec::CALLER
    };
    assert_eq!(
        descriptor.context_at(InvocationArguments::literals(&["absent"]), 0, ArgRole::Body),
        None
    );
    assert_eq!(
        descriptor
            .context_at(
                InvocationArguments::structured(&[InvocationWord::Dynamic]),
                0,
                ArgRole::Body
            )
            .unwrap()
            .frame,
        EvaluationFrame::Unknown
    );
}

#[test]
fn completion_boundaries_distinguish_registration_from_later_procedure_entry() {
    use crate::completion::{CompletionBoundary, CompletionBoundarySite, CompletionConversion};
    let registry = CommandRegistry::build_default();
    for (command, arguments, expected) in [
        ("catch", vec!["body"], CompletionBoundary::CATCH),
        (
            "try",
            vec!["body", "finally", "cleanup"],
            CompletionBoundary::TRY,
        ),
        ("while", vec!["1", "body"], CompletionBoundary::LOOP),
        ("eval", vec!["body"], CompletionBoundary::FORWARD),
    ] {
        let invocation = registry
            .resolve_invocation(command, &arguments, None)
            .unwrap();
        assert_eq!(
            invocation
                .semantics
                .completion
                .stated()
                .unwrap()
                .body_boundary,
            Some(expected)
        );
    }
    let proc = registry
        .resolve_invocation("proc", &["p", "", "body"], None)
        .unwrap();
    let boundary = proc
        .semantics
        .completion
        .stated()
        .unwrap()
        .body_boundary
        .unwrap();
    assert_eq!(boundary.site, CompletionBoundarySite::DefinedCallable);
    assert_eq!(boundary, CompletionBoundary::PROCEDURE);
    assert_eq!(
        boundary.conversions,
        &[
            CompletionConversion::EscapingLoopControlIsError,
            CompletionConversion::ProcedureReturn,
        ]
    );
}

#[test]
fn native_procedures_reject_direct_loop_codes_before_exposing_return_codes() {
    let script = r"proc direct {code} {$code}
proc returned {code} {return -code $code}
foreach code {break continue} {
    set directStatus [catch {direct $code} directResult]
    set returnedStatus [catch {returned $code} returnedResult]
    puts [list direct $code $directStatus $directResult]
    puts [list returned $code $returnedStatus $returnedResult]
    set visited {}
    foreach item {1 2} {
        lappend visited before$item
        returned $code
        lappend visited after$item
    }
    puts [list caller-loop $code $visited]
}
";
    let expected = "direct break 1 {invoked \"break\" outside of a loop}\n\
returned break 3 {}\n\
caller-loop break before1\n\
direct continue 1 {invoked \"continue\" outside of a loop}\n\
returned continue 4 {}\n\
caller-loop continue {before1 before2}";
    for interpreter in tcl_test_support::available_tclshs() {
        let output = tcl_test_support::run_script(&interpreter.path, script.as_bytes())
            .expect("execute native procedure completion reference")
            .strict_text()
            .expect("native procedure completion reference succeeds");
        assert_eq!(output, expected, "{}", interpreter.patchlevel);
    }
}

#[test]
fn authored_deferral_derives_resolved_compatibility_facts() {
    use crate::body_execution::{BodyExecutionSpec, DeferredExecutionSpec, DeferredTrigger};
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "deferred-probe",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Body)],
        body_execution: Some(BodyExecutionSpec::Deferred(DeferredExecutionSpec::global(
            DeferredTrigger::Channel,
        ))),
        ..CommandSpec::DEFAULT
    });
    let invocation = registry
        .resolve_invocation("deferred-probe", &["body"], None)
        .unwrap();
    assert!(
        invocation
            .semantics
            .traits
            .contains(crate::Traits::DEFERS_BODY)
    );
    assert_eq!(invocation.semantics.body_kind, crate::BodyKind::Structural);
    assert_eq!(
        invocation.authored_source_script_timing_at(0),
        Some(ScriptTiming::Deferred)
    );
    assert_eq!(
        crate::script_body_flow::script_body_flow(&invocation.facts()),
        crate::script_body_flow::ScriptBodyFlow::Deferred
    );
}

#[test]
fn limit_callback_uses_installing_interpreter_instead_of_limited_child() {
    let registry = CommandRegistry::build_default();
    let invocation = registry
        .resolve_invocation(
            "interp",
            &["limit", "child", "commands", "-command", "handler"],
            None,
        )
        .unwrap();
    let contexts = invocation.argument_evaluation_contexts().unwrap();
    let callback = contexts
        .iter()
        .find(|context| context.argument == 4)
        .unwrap();
    assert_eq!(callback.interpreter, InterpreterScope::Current);
    let context = callback.context.unwrap();
    assert_eq!(context.frame, EvaluationFrame::Global);
    assert_eq!(context.timing, ScriptTiming::Deferred);
}

#[test]
fn explicit_body_override_retains_closed_caller_name_defaults() {
    let descriptor = EvaluationContextSpec {
        roles: &[RoleEvaluationContext {
            roles: &[ArgRole::Body],
            context: EvaluationContext {
                frame: EvaluationFrame::Global,
                ..EvaluationContext::INVOKING
            },
        }],
        ..EvaluationContextSpec::CALLER
    };
    let arguments = InvocationArguments::literals(&["name", "body"]);
    assert_eq!(
        descriptor.context_at(arguments, 0, ArgRole::Name),
        Some(EvaluationContext::REFERENCE)
    );
    assert_eq!(
        descriptor
            .context_at(arguments, 1, ArgRole::Body)
            .unwrap()
            .frame,
        EvaluationFrame::Global
    );
    assert_eq!(
        descriptor.context_at(arguments, 0, ArgRole::LambdaLiteral),
        None
    );
}

#[test]
fn definition_namespaces_and_deferred_receiver_bodies_are_distinct() {
    let registry = CommandRegistry::build_default();
    for (command, namespace) in [
        ("oo::define", "::oo::define"),
        ("oo::objdefine", "::oo::objdefine"),
    ] {
        let invocation = registry
            .resolve_invocation(command, &["target", "puts body"], None)
            .unwrap();
        let context = invocation.argument_evaluation_context(1).unwrap();
        assert_eq!(context.frame, EvaluationFrame::NamedNamespace(namespace));
        assert_eq!(context.timing, ScriptTiming::SameInvocation);
        let invocation = registry
            .resolve_invocation(command, &["target", "method", "inspect", "", "body"], None)
            .unwrap();
        let context = invocation.argument_evaluation_context(4).unwrap();
        assert_eq!(context.frame, EvaluationFrame::ObjectReceiver);
        assert_eq!(context.timing, ScriptTiming::Deferred);
    }
    let invocation = registry
        .resolve_invocation("oo::class", &["create", "C", "body"], None)
        .unwrap();
    let context = invocation.argument_evaluation_context(2).unwrap();
    assert_eq!(
        context.frame,
        EvaluationFrame::NamedNamespace("::oo::define")
    );
    let invocation = registry
        .resolve_invocation("oo::define", &["C", "self", "body"], None)
        .unwrap();
    assert!(invocation.argument_evaluation_context(2).is_none());
}

#[test]
fn alias_target_owner_selects_global_and_current_namespace_coordinates() {
    use tcl_dialect::model::{Family, SurfaceQuery};
    let registry = CommandRegistry::build_default();
    for (command, frame) in [
        ("global", EvaluationFrame::Global),
        ("variable", EvaluationFrame::CurrentNamespace),
    ] {
        let invocation = registry
            .resolve_invocation(
                command,
                &["x"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .unwrap();
        let context = invocation.argument_evaluation_context(0).unwrap();
        assert_eq!(context.frame, frame);
        assert_eq!(context.timing, ScriptTiming::ReferenceOnly);
    }
}

#[test]
fn global_variable_option_scope_overrides_a_broader_caller_context() {
    use crate::hover::{OptionSpec, OptionValue};
    const OPTIONS: &[OptionSpec] = &[OptionSpec {
        name: "-variable",
        value: OptionValue::global_var_name(),
        ..OptionSpec::DEFAULT
    }];
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "scoped-option-probe",
        arity: Arity::at_least(0),
        options: OPTIONS,
        evaluation_contexts: Some(EvaluationContextSpec::CALLER),
        ..CommandSpec::DEFAULT
    });
    let invocation = registry
        .resolve_invocation("scoped-option-probe", &["-variable", "x"], None)
        .unwrap();
    let contexts = invocation.argument_evaluation_contexts().unwrap();
    let context = contexts
        .iter()
        .find(|context| context.argument == 1)
        .unwrap()
        .context
        .unwrap();
    assert_eq!(context.frame, EvaluationFrame::Global);
    assert_eq!(context.timing, ScriptTiming::ReferenceOnly);
}

/// Frame and timing of a script operand in each measured wrapper shape
/// (`script-parameters-design.md`, tclsh 8.4.20 to 9.1.0).
#[test]
fn script_argument_context_places_each_measured_wrapper_shape() {
    use crate::hover::ScriptTiming;
    let registry = CommandRegistry::build_default();
    let dialect = Some(tcl_dialect::model::SurfaceQuery::core(
        tcl_dialect::model::Family::Tcl,
        "8.6",
    ));
    let resolve = |name: &'static str, arguments: &[InvocationWord<'static>]| {
        let arguments = arguments.to_vec();
        let words = InvocationWords::structured(InvocationWord::Literal(name), &arguments)
            .with_dialect(tcl_native_model::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V8_6,
            ));
        let invocation = registry
            .resolve_structured_invocation(words, dialect)
            .resolved()
            .unwrap();
        // The script is the last word; a lone computed `uplevel` operand is
        // the script on the successful path, so it is asked by position.
        invocation.script_argument_context(arguments.len() - 1)
    };
    let now = |frame| ScriptArgumentContext {
        frame,
        timing: ScriptTiming::SameInvocation,
    };
    let script = InvocationWord::Dynamic;
    assert_eq!(
        resolve("uplevel", &[InvocationWord::Literal("1"), script]),
        Some(now(ScriptFrame::Relative(1)))
    );
    assert_eq!(
        resolve("uplevel", &[script]),
        Some(now(ScriptFrame::Relative(1)))
    );
    assert_eq!(
        resolve("uplevel", &[InvocationWord::Literal("#0"), script]),
        Some(now(ScriptFrame::Global))
    );
    assert_eq!(
        resolve("uplevel", &[InvocationWord::Literal("2"), script]),
        Some(now(ScriptFrame::Relative(2)))
    );
    assert_eq!(
        resolve("eval", &[script]),
        Some(now(ScriptFrame::Relative(0)))
    );
    assert_eq!(
        resolve("catch", &[script]),
        Some(now(ScriptFrame::Relative(0)))
    );
    assert_eq!(
        resolve("after", &[InvocationWord::Literal("0"), script]),
        Some(ScriptArgumentContext {
            frame: ScriptFrame::Global,
            timing: ScriptTiming::Deferred,
        })
    );
}
