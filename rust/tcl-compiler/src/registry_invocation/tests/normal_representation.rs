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

//! Normal representation and taint: handler proofs, value contracts and pattern layouts.

use super::*;

#[test]
fn normal_ensemble_leaf_requires_the_actual_release_worker() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        for (command, introduced) in [
            ("string toupper hello", "tcl8.5"),
            ("binary format a* hello", "tcl8.6"),
        ] {
            let worker = if command.starts_with("string") {
                "::tcl::string::toupper"
            } else {
                "::tcl::binary::format"
            };
            for replaced in [false, true] {
                let source = if replaced {
                    format!(
                        "namespace eval ::tcl::string {{}}; namespace eval ::tcl::binary {{}}; proc {worker} args {{return CUSTOM}}; {command}"
                    )
                } else {
                    command.to_owned()
                };
                let bindings = crate::command_binding::SourceCommandBindings::analyse(
                    &source, config, registry,
                );
                let segment =
                    crate::segmenter::segment_commands_with_offset_and_config(&source, 0, config)
                        .pop()
                        .unwrap();
                let tokens = CommandTokens::from_segmented(
                    &tcl_lexer::SourceMap::new(&source),
                    config,
                    &segment,
                )
                .with_source_binding(
                    bindings.invocation_at_source("ignored", segment.span.start()),
                );
                let expects_worker = profile != "jim" && profile >= introduced;
                assert_eq!(
                    normal_representation_invocation(registry, None, &tokens).is_some(),
                    !replaced || !expects_worker,
                    "{profile}: {source}"
                );
            }
        }
    }
}

#[test]
fn f5_value_provider_proves_normal_representation_without_c_workers() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for command in ["string toupper hello", "binary format a* hello"] {
        let tokens = f5_source_tokens(command);
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
        assert!(normal_representation_invocation(registry, None, &tokens).is_some());
        let replaced = f5_source_tokens(&format!(
            "proc {} args {{return CUSTOM}}; {command}",
            command.split_whitespace().next().unwrap()
        ));
        assert!(normal_representation_invocation(registry, None, &replaced).is_none());
    }
    let tokens = f5_source_tokens_in_realm(
        "namespace eval ::tcl::string {}; proc ::tcl::string::toupper args {return CUSTOM}; string toupper hello",
        tcl_dialect::model::InvocationRealm::InterpreterRuntime,
    );
    assert!(normal_representation_invocation(registry, None, &tokens).is_some());
    let malformed =
        f5_source_tokens("proc ::tcl::string::toupper args {return CUSTOM}; string toupper hello");
    assert!(normal_representation_invocation(registry, None, &malformed).is_none());
}

#[test]
fn normal_completion_retains_handler_return_without_licensing_unknown_compilation() {
    let f5 = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("return VALUE");
    assert!(!normal_handler_completion_route(f5, None, &tokens).normal_possible());
    assert!(resolved_tokens_invocation(f5, None, &tokens).is_none());
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .compiled_execution_unknown()
    );
    let replaced = f5_source_tokens("proc return args {set x CUSTOM}; return VALUE");
    assert!(normal_handler_completion_route(f5, None, &replaced).normal_possible());
    let c = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let tokens = last_source_tokens("return VALUE");
    assert!(!normal_handler_completion_route(c, None, &tokens).normal_possible());
    assert!(resolved_tokens_invocation(c, None, &tokens).is_some());
}

#[test]
fn f5_connection_controls_preserve_following_normal_return_identity() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for command in ["drop", "discard", "reject", "DNS::return"] {
        let tokens = f5_source_tokens(&format!("{command}; return"));
        assert!(
            !normal_handler_completion_route(registry, None, &tokens).normal_possible(),
            "{command}"
        );
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .compiled_execution_unknown()
        );
    }
    let replaced = f5_source_tokens("proc drop {} {proc return args {set x CUSTOM}}; drop; return");
    assert!(normal_handler_completion_route(registry, None, &replaced).normal_possible());
}

#[test]
fn representation_contract_preserves_handler_proof_without_opcode_proof() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("llength {a b}");
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
    let normal = normal_representation_invocation(registry, None, &tokens).unwrap();
    assert_eq!(normal.argument_literal(0).as_deref(), Some("a b"));
    assert_eq!(
        normal.argument_type_hint(0).unwrap().expected,
        Some(tcl_registry::TclType::List)
    );
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .compiled_execution_unknown()
    );
}

#[test]
fn unavailable_f5_string_member_cannot_donate_a_value_handler_contract() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("string cat a b");
    assert!(normal_representation_invocation(registry, None, &tokens).is_none());
}

#[test]
fn unavailable_f5_binary_member_cannot_donate_a_value_handler_contract() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("binary encode hex DATA");
    assert!(normal_representation_invocation(registry, None, &tokens).is_none());
}

#[test]
fn normal_assignment_keeps_value_words_without_an_opcode_or_custom_handler_proof() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("set p [TCP::payload]");
    let normal = normal_representation_invocation(registry, None, &tokens).unwrap();
    let assignment = normal.value_assignment().unwrap();
    assert_eq!(assignment.name, "p");
    assert_eq!(assignment.value, tokens.words()[2]);
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
    for source in ["set p", "proc set args {return CUSTOM}; set p value"] {
        let tokens = f5_source_tokens(source);
        assert!(
            normal_representation_invocation(registry, None, &tokens)
                .and_then(|normal| normal.value_assignment())
                .is_none()
        );
    }
}

#[test]
fn representation_contract_declines_replaced_and_unproved_handlers() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for source in [
        "proc llength args {return CUSTOM}; llength {a b}",
        "set selected mystery; $selected {a b}",
    ] {
        let tokens = f5_source_tokens(source);
        assert!(normal_representation_invocation(registry, None, &tokens).is_none());
    }
}

#[test]
fn normal_taint_source_properties_require_the_selected_getter() {
    use tcl_registry::taint::TaintColour;
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let getter = f5_source_tokens("HTTP::uri");
    let source = normal_taint_invocation(registry, None, &getter).unwrap();
    let colour = source.source_colour().unwrap();
    assert!(colour.contains(TaintColour::TAINTED | TaintColour::PATH_PREFIXED));
    assert!(resolved_tokens_invocation(registry, None, &getter).is_none());

    let setter = f5_source_tokens("HTTP::uri /changed");
    let colour = normal_taint_invocation(registry, None, &setter)
        .unwrap()
        .source_colour()
        .unwrap();
    assert!(colour.contains(TaintColour::TAINTED));
    assert!(!colour.contains(TaintColour::PATH_PREFIXED));
}

#[test]
fn compiled_dynamic_regexp_pattern_retains_its_original_operand_slot() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {pattern subject} {regexp $pattern $subject}",
            registry,
            false,
        );
        let statement = &cu
            .ir_module
            .procedures
            .get(&fixture_id(&cu.ir_module.procedures, "::p"))
            .expect("authored fixture body")
            .body
            .statements[0];
        let tokens = statement.tokens().unwrap();
        let normal = normal_representation_invocation(registry, None, tokens).unwrap();
        assert_eq!(
            normal.pattern_source_argument_index(registry),
            (profile != "tcl8.4").then_some(0),
            "{profile}"
        );
    }
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let tokens = last_source_tokens("regexp -about aa");
    assert_eq!(
        normal_representation_invocation(registry, None, &tokens)
            .unwrap()
            .pattern_source_argument_index(registry),
        Some(1)
    );
}

#[test]
fn normal_pattern_layout_uses_closed_original_read_values() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    for (source, expected) in [
        (
            "proc p {outer inner s} {set re seed; if {$outer} {if {$inner} {set re left}}; regsub -command $re $s callback out}",
            Some(1),
        ),
        (
            "proc p {flag s} {set re seed; if {$flag} {set re -command}; regsub $re $s callback out}",
            None,
        ),
        ("proc p {re s} {regsub -command $re $s callback out}", None),
        (
            "proc p {flag s} {set re seed; if {$flag} {set re left}; proc observer args {}; trace add variable re read observer; regsub -command $re $s callback out}",
            None,
        ),
    ] {
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
        let statement = cu
            .ir_module
            .procedures
            .get(&fixture_id(&cu.ir_module.procedures, "::p"))
            .expect("authored fixture body")
            .body
            .statements
            .last()
            .unwrap();
        let normal = normal_representation_invocation(registry, None, statement.tokens().unwrap());
        assert_eq!(
            normal
                .as_ref()
                .and_then(|normal| normal.pattern_source_argument_index(registry)),
            expected,
            "{source}"
        );
        if let Some(normal) = normal
            && matches!(normal.proof, NormalRepresentationProof::PatternLayout(_))
        {
            assert!(normal.argument_type_hint(1).is_none());
            assert_eq!(normal.operand_roles(), []);
            assert_eq!(
                normal.representation_effect(),
                tcl_registry::RepresentationEffect::None
            );
            assert!(normal.value_assignment().is_none());
        }
    }
}

#[test]
fn normal_taint_numeric_results_do_not_require_opcode_proof() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("string length VALUE");
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
    assert!(
        normal_taint_invocation(registry, None, &tokens)
            .unwrap()
            .is_sanitiser()
    );
    let replaced = f5_source_tokens("proc string args {return CUSTOM}; string length VALUE");
    assert!(normal_taint_invocation(registry, None, &replaced).is_none());
}

#[test]
fn normal_taint_transform_requires_frozen_mapping_values() {
    use tcl_registry::taint::TaintColour;
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for source in [
        r#"string map {"\r" "" "\n" ""} VALUE"#,
        r#"set mapping {"\r" "" "\n" ""}; string map -nocase $mapping VALUE"#,
    ] {
        let tokens = f5_source_tokens(source);
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
        assert_eq!(
            normal_taint_invocation(registry, None, &tokens)
                .unwrap()
                .transform_colour(),
            Some(TaintColour::CRLF_FREE),
            "{source}"
        );
    }
    for source in [
        r#"set mapping [HTTP::uri]; string map $mapping {"\r" "" "\n" ""}"#,
        r#"string map {"\r" "" "\n" "x\ny"} VALUE"#,
        r#"string map {"\r" "" "\n"} VALUE"#,
    ] {
        let tokens = f5_source_tokens(source);
        assert!(
            normal_taint_invocation(registry, None, &tokens)
                .and_then(|normal| normal.transform_colour())
                .is_none(),
            "{source}"
        );
    }
}

#[test]
fn normal_taint_source_properties_decline_replaced_or_unproved_handlers() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for source in [
        "proc HTTP::uri args {return CUSTOM}; HTTP::uri",
        "set selected mystery; $selected",
    ] {
        let tokens = f5_source_tokens(source);
        assert!(normal_taint_invocation(registry, None, &tokens).is_none());
    }
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let tokens = last_source_tokens("HTTP::uri");
    assert!(normal_taint_invocation(registry, None, &tokens).is_none());
}
