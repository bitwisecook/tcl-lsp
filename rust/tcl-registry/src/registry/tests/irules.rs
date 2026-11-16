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

//! iRules legality by event, top-level declarations, sink commands and XC translatability flags.

use super::*;

#[test]
fn irules_command_legality_matrix() {
    use crate::events::EventRegistry;
    use crate::profiles::ProfileRegistry;
    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    // HTTP::respond is satisfied in HTTP_REQUEST (HTTP profile implied) but
    // not in the L4 CLIENT_ACCEPTED event.
    assert!(reg.is_irules_command_legal_in_event(
        "HTTP::respond",
        "HTTP_REQUEST",
        &events,
        &profiles
    ));
    assert!(!reg.is_irules_command_legal_in_event(
        "HTTP::respond",
        "CLIENT_ACCEPTED",
        &events,
        &profiles
    ));
    // An unknown event is illegal for every command.
    assert!(!reg.is_irules_command_legal_in_event(
        "HTTP::respond",
        "NOT_AN_EVENT",
        &events,
        &profiles
    ));
    // HA::status is explicitly excluded from RULE_INIT.
    assert!(!reg.is_irules_command_legal_in_event("HA::status", "RULE_INIT", &events, &profiles));

    // The inverse list is sorted and reflects the same matrix.
    let evs = reg.irules_events_for_command("HTTP::respond", &events, &profiles);
    assert!(evs.contains(&"HTTP_REQUEST"));
    assert!(!evs.contains(&"CLIENT_ACCEPTED"));
    assert!(
        evs.windows(2).all(|w| w[0] <= w[1]),
        "events must be sorted"
    );
}

#[test]
fn irules_placement_contract_owns_declaration_and_execution_contexts() {
    use crate::events::{IrulesCommandPlacement as Placement, IrulesExecutionContext as Ctx};

    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    for declaration in ["when", "proc", "timing", "priority"] {
        assert_eq!(
            registry.irules_command_placement(declaration, Ctx::TopLevel),
            Placement::Allowed,
            "{declaration} is an iRules top-level declaration"
        );
        assert_eq!(
            registry.irules_command_placement(declaration, Ctx::EventBody),
            Placement::RequiresTopLevel,
            "{declaration} cannot be nested in an event"
        );
        assert_eq!(
            registry.irules_command_placement(declaration, Ctx::ProcedureBody),
            Placement::RequiresTopLevel,
            "{declaration} cannot be nested in a proc"
        );
    }
    for executable in ["set", "HTTP::uri", "call", "user_proc"] {
        assert_eq!(
            registry.irules_command_placement(executable, Ctx::TopLevel),
            Placement::RequiresEventOrProcedure,
            "{executable} cannot execute at iRules top level"
        );
        assert_eq!(
            registry.irules_command_placement(executable, Ctx::EventBody),
            Placement::Allowed
        );
        assert_eq!(
            registry.irules_command_placement(executable, Ctx::ProcedureBody),
            Placement::Allowed
        );
    }
}

#[test]
fn irules_declaration_owner_requires_braced_source_bodies() {
    use crate::events::IrulesTopLevelDeclaration as Declaration;

    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let events = crate::events::EventRegistry::build();
    let declaration = |name: &str, args: &[&str]| {
        let body_index = if name == "proc" {
            2
        } else {
            args.len().saturating_sub(1)
        };
        let tokens = args
            .iter()
            .enumerate()
            .map(|(index, _)| {
                tcl_lexer::Token::new(
                    if index == body_index {
                        tcl_lexer::TokenType::Str
                    } else {
                        tcl_lexer::TokenType::Esc
                    },
                    tcl_lexer::Span::empty(0),
                )
            })
            .collect::<Vec<_>>();
        let single = vec![true; args.len()];
        let closed = tokens
            .iter()
            .map(|token| token.kind == tcl_lexer::TokenType::Str)
            .collect::<Vec<_>>();
        registry.irules_top_level_declaration(
            name,
            crate::events::IrulesDeclarationArguments::new(args, &tokens, &single, &closed)
                .expect("parallel source-word views"),
            &events,
        )
    };
    assert!(matches!(
        declaration("when", &["HTTP_REQUEST", "set x 1"]),
        Some(Declaration::Event { .. })
    ));
    assert!(matches!(
        declaration("::when", &["HTTP_REQUEST", "set x 1"]),
        Some(Declaration::Event { .. })
    ));
    assert!(declaration("when", &["BOGUS_EVENT", "pool x"]).is_none());
    for valid in [
        &["HTTP_REQUEST", "body"][..],
        &["HTTP_REQUEST", "priority", "100", "body"][..],
        &["HTTP_REQUEST", "timing", "on", "body"][..],
        &[
            "HTTP_REQUEST",
            "priority",
            "100",
            "timing",
            "disable",
            "body",
        ][..],
        &["HTTP_REQUEST", "priority", "0", "body"][..],
        &["HTTP_REQUEST", "priority", "1000", "body"][..],
        &["HTTP_REQUEST", "timing", "on", "priority", "100", "body"][..],
    ] {
        assert!(declaration("when", valid).is_some());
    }
    for invalid in [
        &["HTTP_REQUEST"][..],
        &["HTTP_REQUEST", "body", "trailing"][..],
        &["HTTP_REQUEST", "priority", "bad", "body"][..],
        &["HTTP_REQUEST", "priority", "-1", "body"][..],
        &["HTTP_REQUEST", "priority", "1001", "body"][..],
        &["HTTP_REQUEST", "timing", "maybe", "body"][..],
        &["HTTP_REQUEST", "priority", "1", "priority", "2", "body"][..],
        &["HTTP_REQUEST", "timing", "on", "timing", "off", "body"][..],
        &["HTTP_REQUEST", "timing", "on", "priority", "1001", "body"][..],
    ] {
        assert!(declaration("when", invalid).is_none());
    }
    assert!(matches!(
        declaration("proc", &["p", "", "return"]),
        Some(Declaration::Procedure { .. })
    ));
    for value in ["0", "500", "1000"] {
        assert_eq!(
            declaration("priority", &[value]),
            Some(Declaration::Priority {
                value: value.parse().unwrap()
            })
        );
    }
    for invalid in ["-1", "1001", "bad"] {
        assert!(declaration("priority", &[invalid]).is_none());
    }
    assert_eq!(
        declaration("timing", &["enable"]),
        Some(Declaration::Timing { enabled: true })
    );
    for malformed in [&["p", ""][..], &["p", "", "return", "extra"][..]] {
        assert!(declaration("proc", malformed).is_none());
    }
}

#[test]
fn irules_declaration_owner_rejects_bare_and_quoted_source_bodies() {
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    for (name, args, body_index) in [
        ("when", &["HTTP_REQUEST", "set x 1"][..], 1),
        ("proc", &["p", "", "return"][..], 2),
    ] {
        let mut tokens =
            vec![
                tcl_lexer::Token::new(tcl_lexer::TokenType::Esc, tcl_lexer::Span::empty(0),);
                args.len()
            ];
        let single = vec![true; args.len()];
        let closed = vec![false; args.len()];
        for (kind, in_quote) in [
            (tcl_lexer::TokenType::Esc, false),
            (tcl_lexer::TokenType::Esc, true),
            // Lexer recovery uses Str for `{body` at EOF; the separate
            // closed-brace fact must keep it out of declarations too.
            (tcl_lexer::TokenType::Str, false),
        ] {
            tokens[body_index] = tcl_lexer::Token {
                kind,
                in_quote,
                ..tokens[body_index]
            };
            let arguments =
                crate::events::IrulesDeclarationArguments::new(args, &tokens, &single, &closed)
                    .expect("parallel source-word views");
            assert!(
                registry
                    .irules_top_level_declaration_shape(name, arguments)
                    .is_none(),
                "{name} must reject a non-closed-braced body"
            );
        }
    }
}

#[test]
fn irules_sink_commands_carry_structural_options() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();

    let respond = reg.get("HTTP::respond").expect("HTTP::respond loaded");
    let opts: Vec<&str> = respond.options.iter().map(|o| o.name).collect();
    // Option set follows the reference standard:
    // -version/-content/-ifile/
    // -noserver/-reset. (`-status` is the positional status arg.)
    assert!(
        opts.contains(&"-version")
            && opts.contains(&"-content")
            && opts.contains(&"-noserver")
            && opts.contains(&"-reset"),
        "HTTP::respond options {opts:?} should include -version / -content / -noserver / -reset",
    );
    let noserver = respond
        .options
        .iter()
        .find(|o| o.name == "-noserver")
        .unwrap();
    assert!(!noserver.takes_value());
    let version = respond
        .options
        .iter()
        .find(|o| o.name == "-version")
        .unwrap();
    assert!(version.takes_value());

    let header = reg.get("HTTP::header").expect("HTTP::header loaded");
    let header_opts: Vec<&str> = header.options.iter().map(|o| o.name).collect();
    assert!(
        header_opts.contains(&"-noupdate"),
        "HTTP::header options {header_opts:?} should include -noupdate",
    );
}

#[test]
fn xc_translatability_helpers_read_spec_flags() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();

    // `xc_translatable: Some(false)` → never translatable (consumed by the
    // `f5-xc` translator's XC300 branch).
    assert!(reg.is_xc_never_translatable("eval"));
    assert!(!reg.is_xc_translatable_override("eval"));

    // `xc_translatable: Some(true)` → translatable override despite an
    // otherwise-untranslatable namespace prefix (e.g. `IP::`, `ASM::`).
    assert!(reg.is_xc_translatable_override("IP::client_addr"));
    assert!(!reg.is_xc_never_translatable("IP::client_addr"));

    // Commands with no `xc_translatable` flag report neither.
    assert!(!reg.is_xc_never_translatable("set"));
    assert!(!reg.is_xc_translatable_override("set"));
    // An unknown command name is safely neither.
    assert!(!reg.is_xc_never_translatable("no_such_command_xyz"));
    assert!(!reg.is_xc_translatable_override("no_such_command_xyz"));
}
