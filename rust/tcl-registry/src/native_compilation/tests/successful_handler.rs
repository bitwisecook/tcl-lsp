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

//! Successful-handler metadata, variable output binding and possible handler bodies.

use super::*;

#[test]
fn possible_handler_bodies_share_grammar_without_native_compiler_licence() {
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let dialect =
        tcl_native_model::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
    let words = ["$ready", "set flag 1", "else", "set flag 2"];
    let facts = registry
        .resolve_invocation("if", &words, registry.own_surface_query())
        .unwrap()
        .facts();
    let arguments = crate::InvocationArguments::literals(&words).with_dialect(dialect);
    let entry = facts
        .possible_handler_body_flow(registry, arguments, crate::VariableAliasFrame::Procedure)
        .expect("audited native handler candidate bodies");
    assert_eq!(
        entry.flow,
        crate::script_body_flow::ScriptBodyFlow::Conditional(vec![(Some(0), 1), (None, 3)])
    );
    assert!(
        facts
            .successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
            .is_none()
    );
    assert!(
        facts
            .possible_handler_body_flow(registry, arguments, crate::VariableAliasFrame::Unknown)
            .is_none()
    );
}

#[test]
fn successful_handler_metadata_does_not_depend_on_vendor_compiler_selection() {
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let dialect =
        tcl_native_model::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
    for (head, words) in [
        ("IP::client_addr", vec![]),
        ("append", vec!["::counter", "x"]),
        ("unset", vec!["::counter"]),
        ("array", vec!["set", "::items", "a 1"]),
        ("scan", vec!["value", "%s", "::parsed"]),
        ("catch", vec!["set x 1", "::err"]),
    ] {
        let facts = registry
            .resolve_invocation(head, &words, registry.own_surface_query())
            .unwrap()
            .facts();
        let arguments = crate::InvocationArguments::literals(&words).with_dialect(dialect);
        assert!(
            facts
                .successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
                .is_some(),
            "{head}"
        );
        assert!(facts.successful_handler.is_some(), "{head}");
        if head != "IP::client_addr" {
            assert!(
                facts
                    .successful_handler_effects(arguments, crate::VariableAliasFrame::Unknown)
                    .is_none(),
                "{head}: unknown frame cannot license address transfer"
            );
        }
    }
}

#[test]
fn catch_output_binding_phase_and_legacy_arity_stay_explicit() {
    assert_eq!(
        SuccessfulHandlerSpec::CatchOutputs.variable_binding_phase(),
        VariableOperandBindingPhase::NormalContinuation
    );
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let words = ["set x 1", "::result", "::options"];
    let facts = registry
        .resolve_invocation("catch", &words, registry.own_surface_query())
        .unwrap()
        .facts();
    let args = crate::InvocationArguments::literals(&words).with_dialect(
        tcl_native_model::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules()),
    );
    assert!(
        facts
            .successful_handler_effects(args, crate::VariableAliasFrame::Procedure)
            .is_none()
    );
}

#[test]
fn sequence_normal_effects_require_no_native_expression_callback() {
    let registry = crate::CommandRegistry::build_default();
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let dialect = InvocationDialect::for_version(version);
        for (operands, closed) in [
            (vec!["3"], true),
            (vec!["1", "to", "5", "by", "2"], true),
            (vec!["1+2"], false),
            (vec!["[set ::changed yes]"], false),
            (vec!["1", "count"], false),
        ] {
            let words = InvocationWords::literals("lseq", &operands).with_dialect(dialect);
            let resolution =
                registry.resolve_structured_invocation(words, dialect.authoring_query());
            let facts = resolution.resolved().unwrap().facts();
            assert_eq!(
                facts
                    .successful_value_leaf_world(words.arguments())
                    .is_some(),
                closed,
                "{version:?}: {operands:?}"
            );
            if facts.arity_accepts_frozen_arguments() == Some(true) {
                assert!(
                    facts
                        .normal_list_method_provider(words.arguments())
                        .is_some()
                );
            }
            let spec = facts.native_compilation.unwrap();
            assert_eq!(
                spec.select(
                    words,
                    &vec![NativeCompilationWordShape::Literal; operands.len()],
                    Some(dialect),
                    context(NativeCompilationFrame::ProcedureCode)
                ),
                if version == TclVersion::V9_0 {
                    NativeCompilationSelection::Generic
                } else {
                    NativeCompilationSelection::Unknown
                }
            );
        }
    }
}

#[test]
fn successful_handler_effects_do_not_select_compiler_or_body_protocol() {
    let registry = crate::CommandRegistry::build_default();
    let arguments = crate::InvocationArguments::literals(&["x", "value"]);
    for name in ["set", "incr", "list"] {
        let spec = registry.get(name).unwrap().native_compilation.unwrap();
        let effects = spec
            .successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
            .expect("normative normal handler effects");
        assert_eq!(effects.operation, spec.operation);
        if name != "list" {
            assert!(
                spec.successful_handler_effects(arguments, crate::VariableAliasFrame::Unknown)
                    .is_none()
            );
        }
    }
    for name in ["expr", "if", "while", "return", "proc"] {
        let spec = registry.get(name).unwrap().native_compilation.unwrap();
        assert!(
            spec.successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
                .is_none(),
            "{name}: body/control protocol must remain unproved"
        );
    }
    let spec = registry.get("set").unwrap().native_compilation.unwrap();
    assert!(
        spec.successful_handler_effects(
            crate::InvocationArguments::structured(&[
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Literal("value"),
            ]),
            crate::VariableAliasFrame::Procedure,
        )
        .is_none()
    );
    assert!(
        spec.successful_handler_effects(
            crate::InvocationArguments::literals(&["x", "value", "extra"]),
            crate::VariableAliasFrame::Procedure,
        )
        .is_none()
    );
}
