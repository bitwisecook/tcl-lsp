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

#[test]
fn source_flow_actions_keep_exact_original_event_control_operands() {
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let context = crate::model::ingress::static_context_for("f5-irules");
    let registry = context.commands();
    let descriptor = registry.get("event").unwrap();
    for (arguments, expected) in [
        (
            vec![Literal("disable"), Literal("all")],
            Some(IrulesSourceFlowAction::DisablesAllEvents),
        ),
        (vec![Literal("enable"), Literal("all")], None),
        (vec![Literal("disable"), Literal("HTTP_REQUEST")], None),
        (vec![Literal("disable"), Dynamic], None),
        (vec![Dynamic, Literal("all")], None),
        (vec![Expanded, Literal("all")], None),
        (
            vec![Literal("disable"), Literal("all"), Literal("extra")],
            None,
        ),
    ] {
        let words = crate::InvocationWords::structured(Literal("event"), &arguments);
        let selected = crate::registry::resolve_source_descriptor_invocation(
            descriptor,
            words,
            registry.source_descriptor_availability(
                descriptor,
                Some(tcl_dialect::model::SurfaceQuery::any_release(
                    tcl_dialect::model::Family::F5Irules,
                )),
            ),
            None,
        );
        assert_eq!(selected.authored_source_irules_flow_action(), expected);
        assert_eq!(selected.facts().irules_source_flow_action, expected);
    }
    let mut renamed = registry.get("drop").unwrap().clone();
    renamed.name = "probe::renamed_drop";
    let words = crate::InvocationWords::structured(Literal("probe::renamed_drop"), &[]);
    let selected = crate::registry::resolve_source_descriptor_invocation(
        &renamed,
        words,
        registry.source_descriptor_availability(&renamed, None),
        None,
    );
    assert_eq!(
        selected.authored_source_irules_flow_action(),
        Some(IrulesSourceFlowAction::DropsConnection)
    );
}

#[test]
fn build_event_registry() {
    let reg = EventRegistry::build();
    assert!(reg.len() > 10);
    assert!(reg.is_known("HTTP_REQUEST"));
    assert!(!reg.is_known("NONEXISTENT_EVENT"));
}

#[test]
fn event_props_lookup() {
    let reg = EventRegistry::build();
    let props = reg.get_props("HTTP_REQUEST").unwrap();
    assert!(props.client_side);
    assert!(props.hot);
    assert!(props.implied_profiles.contains(&"HTTP"));
}

#[test]
fn ssl_handshake_attachment_profiles_keep_event_specific_persistence() {
    let registry = EventRegistry::build();
    for (event, ssl, persist) in [
        ("CLIENTSSL_CLIENTHELLO", "CLIENTSSL", true),
        ("CLIENTSSL_HANDSHAKE", "CLIENTSSL", false),
        ("SERVERSSL_CLIENTHELLO_SEND", "SERVERSSL", false),
        ("SERVERSSL_SERVERHELLO", "SERVERSSL", true),
        ("SERVERSSL_HANDSHAKE", "SERVERSSL", true),
    ] {
        let profiles = registry.get_props(event).unwrap().implied_profiles;
        assert!(profiles.contains(&ssl), "{event}");
        assert_eq!(profiles.contains(&"PERSIST"), persist, "{event}");
        assert_eq!(profiles.contains(&"SSL_PERSISTENCE"), persist, "{event}");
    }
}

#[test]
fn command_side_distinguishes_ambiguous_response_events() {
    let reg = EventRegistry::build();
    assert_eq!(
        reg.get_props("HTTP_REQUEST")
            .and_then(EventProps::command_side),
        Some(ConnectionSide::Client)
    );
    assert_eq!(
        reg.get_props("HTTP_RESPONSE")
            .and_then(EventProps::command_side),
        Some(ConnectionSide::Server)
    );
    assert_eq!(
        reg.get_props("HTTP_PROXY_RESPONSE")
            .and_then(EventProps::command_side),
        None,
        "an ambiguous event without an explicit command side must abstain"
    );
}

#[test]
fn implicit_release_requires_the_matching_setup_event_and_side() {
    let reg = EventRegistry::build();
    assert!(reg.data_event_releases_collection(
        "HTTP_RESPONSE_DATA",
        "HTTP_RESPONSE",
        "HTTP",
        ConnectionSide::Server,
    ));
    assert!(!reg.data_event_releases_collection(
        "HTTP_RESPONSE_DATA",
        "HTTP_REQUEST",
        "HTTP",
        ConnectionSide::Server,
    ));
    assert!(!reg.data_event_releases_collection(
        "HTTP_RESPONSE_DATA",
        "HTTP_RESPONSE",
        "HTTP",
        ConnectionSide::Client,
    ));
}

#[test]
fn once_per_connection_check() {
    let reg = EventRegistry::build();
    assert!(reg.is_once_per_connection("CLIENT_ACCEPTED"));
    assert!(!reg.is_once_per_connection("HTTP_REQUEST"));
}

#[test]
fn per_request_check() {
    let reg = EventRegistry::build();
    assert!(reg.is_per_request("HTTP_REQUEST"));
    assert!(!reg.is_per_request("CLIENT_ACCEPTED"));
}

#[test]
fn master_order_starts_with_rule_init() {
    let reg = EventRegistry::build();
    assert_eq!(reg.master_order()[0].event, "RULE_INIT");
}

#[test]
fn flow_chains_exist() {
    let reg = EventRegistry::build();
    assert_ne!(reg.flow_chains().len(), 0);
    assert_eq!(reg.flow_chains()[0].chain_id, "plain_tcp");
}

#[test]
fn event_scan_uses_top_level_rooted_normalised_owner() {
    assert_eq!(
        scan_when_events("::when http_request { if {1} { :::when client_data {} } }"),
        ["HTTP_REQUEST"]
    );
}

#[test]
fn event_scan_ignores_nested_script_surfaces() {
    let source = r#"
            set payload {when CLIENT_DATA {}}
            set quoted "when SERVER_DATA {}"
            # when RULE_INIT {}
            ::when http_request { if {1} { :::when client_data {} } }
        "#;
    assert_eq!(scan_when_events(source), ["HTTP_REQUEST"]);
}

#[test]
fn event_scan_ignores_case_and_unavailable_apply_when_words() {
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let source = r"
            switch -- $x { a { when CLIENT_DATA {} } default { when SERVER_DATA {} } }
            apply {{} { when HTTP_REQUEST {} }}
            set inert {when RULE_INIT {}}
        ";
    let events: Vec<_> = top_level_when_handlers_with_registry_and_head_resolver(
        source,
        registry,
        &WrittenHeadResolver,
    )
    .into_iter()
    .map(|handler| handler.event)
    .collect();
    assert_eq!(events.len(), 0);
}

#[test]
fn non_irules_definition_bodies_are_not_event_handlers() {
    let tcl = crate::model::ingress::static_context_for("tcl9.0").commands();
    let source = r"
            oo::class create C {
                method m {} { when HTTP_REQUEST {} }
                self { method n {} { when CLIENT_DATA {} } }
                property p -get { when SERVER_DATA {} }
            }
        ";
    let events: Vec<_> =
        top_level_when_handlers_with_registry_and_head_resolver(source, tcl, &WrittenHeadResolver)
            .into_iter()
            .map(|handler| handler.event)
            .collect();
    assert!(
        events.is_empty(),
        "Tcl's user-level `when` is not an iRules handler"
    );

    let source = r"
            snit::type S { method m {} { when RULE_INIT {} } }
            itcl::class I { public method m {} { when CLIENT_ACCEPTED {} } }
        ";
    let events: Vec<_> = top_level_when_handlers_with_registry_and_head_resolver(
        source,
        crate::model::ingress::static_context_for("tcl8.6").commands(),
        &WrittenHeadResolver,
    )
    .into_iter()
    .map(|handler| handler.event)
    .collect();
    assert!(
        events.is_empty(),
        "non-iRules dialects have no event-handler trait"
    );
}

#[test]
fn a_user_proc_named_when_is_not_an_event_handler() {
    let source = r"
            proc when {event body} { puts $event }
            when HTTP_REQUEST {}
        ";
    let events: Vec<_> = top_level_when_handlers_with_registry_and_head_resolver(
        source,
        crate::model::ingress::static_context_for("tcl9.0").commands(),
        &WrittenHeadResolver,
    )
    .into_iter()
    .map(|handler| handler.event)
    .collect();
    assert_eq!(events.len(), 0);
}

#[test]
fn declaration_source_facts_distinguish_closed_and_unterminated_braces() {
    let closed = "when HTTP_REQUEST { pool live }";
    let unterminated = "when HTTP_REQUEST { pool inert";
    for (source, expected) in [(closed, true), (unterminated, false)] {
        let command = tcl_syntax::event_handler::script_commands(
            source,
            LexerConfig::for_file_dialect("f5-irules"),
        )
        .into_iter()
        .next()
        .expect("one recovered command");
        let tokens: Vec<Token> = command.words.iter().skip(1).map(|word| word[0]).collect();
        let single: Vec<bool> = command
            .words
            .iter()
            .skip(1)
            .map(|word| word.len() == 1)
            .collect();
        let facts = closed_braced_argument_words(source, &tokens, &single)
            .expect("parallel argument facts");
        assert_eq!(facts.last().copied(), Some(expected), "{source:?}");
    }
}
#[test]
fn http_request_availability_keeps_its_documented_prebaseline_introduction() {
    // experiments/irules/event-availability/http-request-20261015/README.md
    let registry = EventRegistry::build();
    assert!(!registry.event_available_at("HTTP_REQUEST", "8.4.0"));
    for version in ["9.0.0", "9.4.0", "16.1.0"] {
        assert!(
            registry.event_available_at("HTTP_REQUEST", version),
            "{version}"
        );
    }
    assert!(registry.event_available_at("XML_BEGIN_DOCUMENT", "9.4.0"));
    for version in ["10.0.0", "16.1.0"] {
        assert!(
            !registry.event_available_at("XML_BEGIN_DOCUMENT", version),
            "{version}"
        );
    }
    assert!(!registry.event_available_at("BOGUS_EVENT", "16.1.0"));
}

#[test]
fn variable_frames_and_lifecycle_relations_do_not_invent_event_dominance() {
    let registry = EventRegistry::build();
    assert_eq!(
        registry.variable_frame("RULE_INIT"),
        EventVariableFrame::InitialisationNamespace
    );
    assert_eq!(
        registry.variable_frame("HTTP_REQUEST"),
        EventVariableFrame::Connection
    );
    assert_eq!(
        registry.variable_frame("NOT_AN_EVENT"),
        EventVariableFrame::Unknown
    );
    assert_eq!(
        registry.lifecycle_relation("RULE_INIT", "HTTP_REQUEST"),
        EventLifecycleRelation::InitialisationBeforeTraffic
    );
    assert_eq!(
        registry.lifecycle_relation("CLIENT_ACCEPTED", "HTTP_REQUEST"),
        EventLifecycleRelation::MayPrecede
    );
    assert_eq!(
        registry.lifecycle_relation("HTTP_REQUEST", "CLIENT_ACCEPTED"),
        EventLifecycleRelation::ReaderPrecedesWriter
    );
    assert_eq!(
        registry.lifecycle_relation("NOT_AN_EVENT", "HTTP_REQUEST"),
        EventLifecycleRelation::Unknown
    );
}
#[test]
fn event_invocation_envelopes_keep_worker_and_connection_lifetimes_distinct() {
    let registry = EventRegistry::build();
    assert_eq!(
        registry.execution_multiplicity("RULE_INIT"),
        EventExecutionMultiplicity::OncePerWorkerEpoch,
    );
    assert_eq!(
        registry.execution_multiplicity("CLIENT_ACCEPTED"),
        EventExecutionMultiplicity::OncePerConnection,
    );
    assert_eq!(
        registry.execution_multiplicity("HTTP_REQUEST"),
        EventExecutionMultiplicity::MayRepeat,
    );
    assert_eq!(
        registry.execution_multiplicity("NOT_AN_EVENT"),
        EventExecutionMultiplicity::Unknown,
    );
    assert_eq!(
        registry.lifecycle_relation("RULE_INIT", "RULE_INIT"),
        EventLifecycleRelation::SameEvent,
    );
    assert_eq!(
        registry.lifecycle_relation("RULE_INIT", "NOT_AN_EVENT"),
        EventLifecycleRelation::Unknown,
    );
}
