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

//! iRules `when` declarations: priorities, body shapes, the file surface and dispatch with and without the dialect loaded.

use super::*;

#[test]
fn irules_priority_uses_proved_native_identity_and_frozen_argument_value() {
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    for (source, expected) in [
        (
            "proc priority {value} {return}; priority 700; when HTTP_REQUEST {}",
            500,
        ),
        (
            "set selected 700; priority $selected; when HTTP_REQUEST {}",
            700,
        ),
    ] {
        let module = lower_to_ir_with_config(
            source,
            registry,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );
        assert_eq!(
            module
                .procedures
                .get(&fixture_id(&module.procedures, "::when::HTTP_REQUEST"))
                .expect("authored fixture body")
                .base_priority,
            expected
        );
    }
}

#[test]
fn irules_event_priorities_follow_file_state_and_keep_repeated_handlers() {
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let module = lower_to_ir_with_config(
        "priority 700\n\
             when HTTP_REQUEST {}\n\
             priority 400\n\
             when HTTP_REQUEST {}\n\
             when HTTP_REQUEST priority 200 {}\n\
             when HTTP_REQUEST {}",
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );

    let priorities: Vec<_> = [
        "::when::HTTP_REQUEST",
        "::when::HTTP_REQUEST#1",
        "::when::HTTP_REQUEST#2",
        "::when::HTTP_REQUEST#3",
    ]
    .into_iter()
    .map(|name| {
        module
            .procedures
            .get(&fixture_id(&module.procedures, name))
            .expect("authored fixture body")
            .base_priority
    })
    .collect();
    assert_eq!(priorities, [700, 400, 200, 400]);
}

#[test]
fn irules_declaration_body_shape_gates_lowered_regions() {
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let module = lower_to_ir_with_config(
        "when HTTP_REQUEST priority 1001 {}\n\
             when HTTP_REQUEST bare_body\n\
             when CLIENT_DATA \"quoted body\"\n\
             proc bare_proc {} bare_body\n\
             proc quoted_proc {} \"return\"\n\
             proc valid_proc {} { return }\n\
             when NOT_A_REAL_EVENT {}\n\
             when HTTP_REQUEST priority 200 {}\n\
             when HTTP_REQUEST {}",
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );

    assert!(
        fixture_id_if_present(&module.procedures, "::when::NOT_A_REAL_EVENT").is_some(),
        "a syntactically valid unknown event is lowered so the XC translator can report it"
    );
    for name in ["::when::CLIENT_DATA", "::bare_proc", "::quoted_proc"] {
        assert!(
            fixture_id_if_present(&module.procedures, name).is_none(),
            "a non-braced declaration body must not open {name}"
        );
    }
    assert!(fixture_id_if_present(&module.procedures, "::valid_proc").is_some());
    let request = [
        module
            .procedures
            .get(&fixture_id(&module.procedures, "::when::HTTP_REQUEST"))
            .expect("authored fixture body")
            .base_priority,
        module
            .procedures
            .get(&fixture_id(&module.procedures, "::when::HTTP_REQUEST#1"))
            .expect("authored fixture body")
            .base_priority,
    ];
    assert_eq!(request, [200, 500], "valid repeated handlers keep priority");
}

#[test]
fn irules_declarations_only_lower_at_the_file_surface() {
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let module = lower_to_ir_with_config(
        "if {1} {\n\
                 when CLIENT_DATA { pool top_level_nested_when }\n\
                 proc top_level_nested_proc {} { pool top_level_nested_proc }\n\
             }\n\
             when HTTP_REQUEST {\n\
                 if {1} {\n\
                     when SERVER_DATA { pool event_nested_when }\n\
                     proc event_nested_proc {} { pool event_nested_proc }\n\
                 }\n\
                 pool live_event\n\
             }\n\
             proc helper {} {\n\
                 if {1} {\n\
                     when HTTP_RESPONSE { pool proc_nested_when }\n\
                     proc proc_nested_proc {} { pool proc_nested_proc }\n\
                 }\n\
                 pool live_proc\n\
             }\n",
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );

    for absent in [
        "::when::CLIENT_DATA",
        "::when::SERVER_DATA",
        "::when::HTTP_RESPONSE",
        "::top_level_nested_proc",
        "::event_nested_proc",
        "::proc_nested_proc",
    ] {
        assert!(
            fixture_id_if_present(&module.procedures, absent).is_none(),
            "nested declaration must remain inert: {absent}"
        );
    }
    assert!(fixture_id_if_present(&module.procedures, "::when::HTTP_REQUEST").is_some());
    assert!(fixture_id_if_present(&module.procedures, "::helper").is_some());
}

// `when` and `foreachLine` dispatch through registry-driven
// hook-ID.  `when` is dialect-gated (iRules), so its test
// registry loads the dialect explicitly.  `foreachLine` is
// a Tcl 9.0+ command always registered in `build_default()`.

#[test]
fn registry_dispatch_when_lowers_via_lower_when_with_irules_loaded() {
    // The iRules `when` spec carries `LoweringHookId::When`
    // and only enters the registry after
    // `load_dialect(IRULES)`.  Production callers (the LSP
    // server) always pair `build_default()`
    // with the active dialect; this test mirrors that.
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let m = lower_to_ir_with_config(
        "when HTTP_REQUEST { set q 1 }",
        registry,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "when",
            "expected `when` call via registry hook, got command={command}",
        ),
        other => panic!("expected Call via registry hook, got {other:?}"),
    }
    assert!(
        fixture_id_if_present(&m.procedures, "::when::HTTP_REQUEST").is_some(),
        "expected ::when::HTTP_REQUEST procedure to be registered, got keys={:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn registry_dispatch_when_without_irules_loaded_falls_through() {
    // Without `load_irules()`, the registry has no `when`
    // spec, so `resolve_call` returns `None` and
    // `try_dispatch_structured_hook` falls through to
    // `lower_default`.  This is a misconfiguration on the
    // caller's side (the source uses iRules but the
    // registry doesn't know about it); the lowerer emits a
    // generic `Call` rather than silently treating it as
    // an event handler.  Pinning this behaviour catches
    // future drift if `build_default()` ever folds in
    // iRules.
    let m = lower_to_ir("when HTTP_REQUEST { set q 1 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "when",
            "expected generic Call via lower_default, got command={command}",
        ),
        other => panic!("expected Call via lower_default, got {other:?}"),
    }
    assert!(
        !m.procedures
            .values()
            .any(|body| body.qualified_name.starts_with("::when::")),
        "expected NO ::when::* procedure registered without iRules loaded",
    );
}
