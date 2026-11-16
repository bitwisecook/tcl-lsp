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

fn assistance(
    source: &str,
    dialect: &str,
) -> (
    crate::provider_fixtures::OriginalNativeModuleFixture,
    Box<crate::registry_invocation::InvocationBodyAssistance>,
) {
    let fixture = crate::provider_fixtures::original_native_module(source, dialect);
    let (context, module) = &*fixture;
    let tokens = module
        .top_level
        .statements
        .iter()
        .find_map(|statement| {
            module
                .top_level
                .retained_source_tokens_for_statement(statement)
        })
        .expect("actual original source statement");
    let view = crate::registry_invocation::invocation_body_assistance(
        context.commands(),
        context.as_ref(),
        &DocumentCommandSurface::new(context.commands(), None),
        tokens,
    );
    (fixture, view)
}

#[test]
fn caller_template_literal_data_keeps_name_bytes_and_held_variable_policy() {
    for (dialect, qualified_local) in [("tcl8.4", false), ("tcl9.1", false), ("jimtcl", true)] {
        let (_fixture, view) = assistance("set {$literal} VALUE", dialect);
        let selected = view
            .caller_name_template_invocation()
            .expect("actual original selected set invocation");
        assert_eq!(
            view.caller_template_literal_data(selected, 0).as_deref(),
            Some("$literal")
        );
        for name in [
            "",
            " with space ",
            "$literal",
            "[literal]",
            "literal(open",
            "1abc",
            "café",
        ] {
            assert!(is_local_scalar_name(name, selected), "{dialect}/{name}");
        }
        assert_eq!(
            is_local_scalar_name("N::qualified", selected),
            qualified_local,
            "{dialect}"
        );
        assert!(!is_local_scalar_name("::rooted", selected));
        assert!(!is_local_scalar_name("array(key)", selected));
    }
}

#[test]
fn caller_template_literal_data_refuses_foreign_changed_and_dynamic_carriers() {
    let (_fixture, mut view) = assistance("set {$literal} VALUE", "tcl9.1");
    let selected = view
        .caller_name_template_invocation()
        .expect("original selected set");
    assert_eq!(
        view.caller_template_literal_data(selected, 0).as_deref(),
        Some("$literal")
    );
    let detached = selected.clone();
    assert!(view.caller_template_literal_data(&detached, 0).is_none());
    view.definite_invocation.as_mut().unwrap().arguments[0] = Some("changed".into());
    assert!(
        view.caller_template_literal_data(view.caller_name_template_invocation().unwrap(), 0)
            .is_none()
    );
    let (_fixture, view) = assistance("set $computed VALUE", "tcl9.1");
    assert!(
        view.caller_name_template_invocation()
            .and_then(|selected| view.caller_template_literal_data(selected, 0))
            .is_none()
    );
}

#[test]
fn original_logical_lambda_traits_use_selected_source_fields() {
    let source = "proc reader {outer} {apply {{inner} {eval $inner}} $outer}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let realm = analysis.retained_command_realm().unwrap();
    let body = realm
        .original_logical_procedure_body_cards(input)
        .unwrap()
        .into_iter()
        .find(|body| body.name() == Some("reader"))
        .unwrap();
    let body_span = body.scope().scope_body().content_span();
    let text = &source[body_span.as_range()];
    let env = TraitScanEnv {
        surface: DocumentCommandSurface::new(analysis.resolved_registry().unwrap(), None),
        config: input.lexer_config(),
        identities: realm,
        executed_source: None,
        original_logical_body: Some(&body),
    };
    let traits = infer_param_traits_deep(&["outer"], text, env);
    assert!(traits["outer"].contains(&ProcArgTrait::Eval));

    let changed = text.replace("inner", "other");
    assert!(infer_param_traits_deep(&["outer"], &changed, env).is_empty());
}

#[test]
fn original_logical_lambda_nested_body_roles_keep_selected_source_owner() {
    let source = "proc reader {outer} {apply {{inner} {if 1 {eval $inner}}} $outer}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let realm = analysis.retained_command_realm().unwrap();
    let body = realm
        .original_logical_procedure_body_cards(input)
        .unwrap()
        .into_iter()
        .find(|body| body.name() == Some("reader"))
        .unwrap();
    let text = &source[body.scope().scope_body().content_span().as_range()];
    let env = TraitScanEnv {
        surface: DocumentCommandSurface::new(analysis.resolved_registry().unwrap(), None),
        config: input.lexer_config(),
        identities: realm,
        executed_source: None,
        original_logical_body: Some(&body),
    };
    let traits = infer_param_traits_deep(&["outer"], text, env);
    assert!(traits["outer"].contains(&ProcArgTrait::Eval));

    let changed = text.replace("eval $inner", "eval $other");
    assert!(infer_param_traits_deep(&["outer"], &changed, env).is_empty());
}
