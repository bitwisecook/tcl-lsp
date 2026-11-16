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

//! iRules: command legality by event, event info, side switches, dialect-specific side effects and payload lifecycles.

use super::*;

// Commands-for-event / command legality (iRules event ⇄ command cross-product)

/// `HTTP::header` is valid/legal in `HTTP_REQUEST`.
///
/// f5-dialect: events and the HTTP profile model are F5 concepts, not tclsh.
#[test]
fn http_header_legal_in_http_request() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let valid = reg.valid_irules_commands_for_event("HTTP_REQUEST", &events, &profiles, None);
    assert!(valid.contains(&"HTTP::header"));
    assert!(reg.is_irules_command_legal_in_event(
        "HTTP::header",
        "HTTP_REQUEST",
        &events,
        &profiles
    ));
}

/// `HTTP::header` requires an HTTP profile, which `RULE_INIT` lacks.
///
/// f5-dialect.
#[test]
fn http_header_not_legal_in_rule_init() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    assert!(!reg.is_irules_command_legal_in_event("HTTP::header", "RULE_INIT", &events, &profiles));
    let valid = reg.valid_irules_commands_for_event("RULE_INIT", &events, &profiles, None);
    assert!(!valid.contains(&"HTTP::header"));
}

/// An unknown event is illegal for every command and yields an empty valid
/// set.
///
/// f5-dialect.
#[test]
fn unknown_event_is_illegal_for_all() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    assert!(!reg.is_irules_command_legal_in_event(
        "HTTP::header",
        "TOTALLY_FAKE_EVENT",
        &events,
        &profiles
    ));
    assert_eq!(
        reg.valid_irules_commands_for_event("TOTALLY_FAKE_EVENT", &events, &profiles, None)
            .len(),
        0
    );
}

/// The HTTP2 family is legal in `HTTP_REQUEST` and in the message-routing
/// events.
///
/// f5-dialect.
#[test]
fn http2_commands_legal_in_http_and_mr_events() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    for command in [
        "HTTP2::active",
        "HTTP2::concurrency",
        "HTTP2::stream",
        "HTTP2::version",
    ] {
        assert!(
            reg.is_irules_command_legal_in_event(command, "HTTP_REQUEST", &events, &profiles),
            "{command} should be legal in HTTP_REQUEST"
        );
    }
    assert!(reg.is_irules_command_legal_in_event(
        "HTTP2::active",
        "MR_INGRESS",
        &events,
        &profiles
    ));
    assert!(reg.is_irules_command_legal_in_event("HTTP2::active", "MR_EGRESS", &events, &profiles));
}

/// The O(1) legality check agrees with the bulk valid-command listing for
/// every known event.
///
/// f5-dialect.
#[test]
fn legality_matches_valid_command_listing() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    for event in events.all_event_names() {
        let listed = reg.valid_irules_commands_for_event(event, &events, &profiles, None);
        // Spot-check a handful per event rather than the full cross-product
        // (the full product is large; the invariant is per-command).
        for cmd in listed.iter().take(20) {
            assert!(
                reg.is_irules_command_legal_in_event(cmd, event, &events, &profiles),
                "{cmd} listed valid in {event} but is_legal disagrees"
            );
        }
    }
}

/// A command's `excluded_events` make it illegal in those events even when the
/// profile requirements would otherwise pass. `irules_events_for_command` is
/// the inverse listing and excludes them.
///
/// f5-dialect.
#[test]
fn excluded_events_are_respected() {
    let (reg, ds) = reg_and_set("f5-irules");
    let spec = reg
        .get_for_surface("TCP::rcv_scale", ds)
        .expect("TCP::rcv_scale registered");
    assert!(
        !spec.excluded_events.is_empty(),
        "TCP::rcv_scale should declare excluded events"
    );
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let excluded = spec.excluded_events[0];
    assert!(
        !reg.is_irules_command_legal_in_event("TCP::rcv_scale", excluded, &events, &profiles),
        "TCP::rcv_scale must be illegal in its excluded event {excluded}"
    );
    let legal_events = reg.irules_events_for_command("TCP::rcv_scale", &events, &profiles);
    assert!(!legal_events.contains(&excluded));
}

// Registry info helpers (event_info)

/// `event_info` resolves a known event with a side label, ≥1 valid command,
/// and serialises a dual transport as `tcp/udp`.
///
/// f5-dialect.
#[test]
fn event_info_for_known_events() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();

    let http = reg.event_info("http_request", &events, &profiles, None);
    assert_eq!(http.event, "HTTP_REQUEST", "name is upper-cased");
    assert!(http.known);
    assert!(http.valid_command_count() >= 1);
    assert!(
        [
            "client-side",
            "server-side",
            "client-side and server-side",
            "global"
        ]
        .contains(&http.side),
        "side={}",
        http.side
    );

    let ca = reg.event_info("client_accepted", &events, &profiles, None);
    assert_eq!(ca.transport.as_deref(), Some("tcp/udp"));
}

/// An unknown event is `known=false`, empty valid commands, side `"unknown"`.
///
/// f5-dialect.
#[test]
fn event_info_for_unknown_event() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let info = reg.event_info("totally_fake_event", &events, &profiles, None);
    assert_eq!(info.event, "TOTALLY_FAKE_EVENT");
    assert!(!info.known);
    assert_eq!(info.valid_commands.len(), 0);
    assert_eq!(info.side, "unknown");
    assert!(info.transport.is_none());
}

// iRules nesting script bodies (side switches)

/// `clientside`, `serverside`, `peer` are side-switches.
///
/// f5-dialect: side switches are an iRules concept.
#[test]
fn irules_side_switches_are_flagged() {
    let (reg, _) = reg_and_set("f5-irules");
    for name in ["clientside", "serverside", "peer"] {
        assert!(reg.is_side_switch(name), "{name} should be a side switch");
    }
    // A plain command is not a side switch.
    assert!(!reg.is_side_switch("set"));
}

/// Collect/release/payload and side-switch behaviour is explicit registry
/// data. In particular, `UDP::payload` is available per datagram and must not
/// lead a generic consumer to invent a nonexistent `UDP::collect` command.
///
/// f5-dialect + registry-metadata.
#[test]
fn irules_collection_and_side_switch_facts_are_registry_driven() {
    let (reg, _) = reg_and_set("f5-irules");
    let http_payload = reg
        .data_collection_operation("HTTP::payload")
        .expect("HTTP payload lifecycle fact");
    assert_eq!(http_payload.action, DataCollectionAction::Payload);
    assert_eq!(http_payload.protocol.name, "HTTP");
    assert_eq!(
        http_payload.protocol.payload_requirement,
        PayloadCollectionRequirement::ExplicitCollect
    );
    assert_eq!(
        reg.data_collection_collect_command("HTTP")
            .expect("HTTP collect command")
            .name,
        "HTTP::collect"
    );

    let udp = reg
        .data_collection_operation("UDP::payload")
        .expect("UDP payload lifecycle fact");
    assert_eq!(
        udp.protocol.payload_requirement,
        PayloadCollectionRequirement::NotRequired
    );
    assert!(
        reg.data_collection_collect_command("UDP").is_none(),
        "UDP has no explicit collect command"
    );

    assert_eq!(
        reg.side_switch_target("clientside"),
        Some(SideSwitchTarget::Client)
    );
    assert_eq!(
        reg.side_switch_target("serverside"),
        Some(SideSwitchTarget::Server)
    );
    assert_eq!(reg.side_switch_target("peer"), Some(SideSwitchTarget::Peer));
    assert_eq!(SideSwitchTarget::Client.execution_side("server"), "client");
    assert_eq!(SideSwitchTarget::Server.execution_side("client"), "server");
    assert_eq!(SideSwitchTarget::Peer.execution_side("client"), "server");
    assert_eq!(SideSwitchTarget::Peer.execution_side("server"), "client");

    let priority = reg
        .event_handler_priority("when")
        .expect("when priority policy");
    let handler = reg.event_handler_spec().expect("event handler spec");
    assert_eq!(handler.name, "when");
    assert_eq!(handler.event_handler_priority, Some(priority));
    assert_eq!(priority.keyword, "priority");
    assert_eq!(priority.default_priority, 500);
    assert!(
        !priority.warn_when_implicit,
        "BIG-IP accepts an omitted priority"
    );

    assert!(
        reg.get("HTTP::header")
            .is_some_and(|spec| spec.traits.contains(Traits::REQUIRES_HTTP_CONTEXT)),
        "ordinary HTTP transaction commands require the live context"
    );
    assert!(
        reg.get("HTTP::has_responded")
            .is_some_and(|spec| !spec.traits.contains(Traits::REQUIRES_HTTP_CONTEXT)),
        "the response-state query remains valid after a response is committed"
    );
}

/// `clientside` / `serverside` take zero or one argument; `peer` requires its
/// script — arity bounds.
///
/// f5-dialect: arity is registry metadata.
#[test]
fn side_switch_arities() {
    let (reg, ds) = reg_and_set("f5-irules");
    for name in ["clientside", "serverside"] {
        let spec = reg
            .get_for_surface(name, ds)
            .unwrap_or_else(|| panic!("{name}"));
        assert_eq!(spec.arity.min, 0, "{name} min");
        assert_eq!(spec.arity.max, 1, "{name} max");
    }
    let peer = reg.get_for_surface("peer", ds).expect("peer registered");
    assert_eq!(peer.arity.min, 1);
    assert_eq!(peer.arity.max, 1);
}

// Dialect-differentiated side effects

/// `close` is a file-I/O op in plain Tcl and a connection-control op in iRules
/// (two specs under one name).
///
/// tclsh: `close` closes a channel. f5-dialect: the iRules `close` overlay
/// (connection control) is F5-specific. The dialect-specific *classification*
/// is registry metadata.
#[test]
fn close_side_effects_differ_by_dialect() {
    use tcl_registry::side_effects::SideEffectTarget;
    let (tcl_reg, tcl_ds) = reg_and_set("tcl8.6");
    let tcl_close = tcl_reg
        .get_for_surface("close", tcl_ds)
        .expect("close in tcl");
    assert!(
        tcl_close
            .side_effects
            .lower_bound()
            .iter()
            .any(|e| e.target == SideEffectTarget::FileIo),
        "tcl close is file I/O"
    );

    let (ir_reg, ir_ds) = reg_and_set("f5-irules");
    let ir_close = ir_reg
        .get_for_surface("close", ir_ds)
        .expect("close in irules");
    assert!(
        ir_close
            .side_effects
            .lower_bound()
            .iter()
            .any(|e| e.target == SideEffectTarget::ConnectionControl),
        "irules close is connection control"
    );
}

#[test]
fn irules_payload_lifecycle_inventory_is_registry_complete() {
    use tcl_registry::DataCollectionAction::{Collect, Payload, Release};

    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let mut actual: Vec<_> = reg
        .command_names()
        .filter_map(|name| {
            reg.data_collection_operation(name)
                .map(|op| (name.to_string(), op.protocol.name, op.action))
        })
        .collect();
    actual.sort_unstable_by(|left, right| left.0.cmp(&right.0));

    let mut expected = vec![
        ("ASM::payload".to_string(), "ASM", Payload),
        ("CACHE::payload".to_string(), "CACHE", Payload),
        ("DIAMETER::payload".to_string(), "DIAMETER", Payload),
        ("GTP::payload".to_string(), "GTP", Payload),
        ("HTTP::collect".to_string(), "HTTP", Collect),
        ("HTTP::payload".to_string(), "HTTP", Payload),
        ("HTTP::release".to_string(), "HTTP", Release),
        ("MQTT::collect".to_string(), "MQTT", Collect),
        ("MQTT::payload".to_string(), "MQTT", Payload),
        ("MQTT::release".to_string(), "MQTT", Release),
        ("MR::collect".to_string(), "MR", Collect),
        ("MR::payload".to_string(), "MR", Payload),
        ("MR::release".to_string(), "MR", Release),
        ("REWRITE::payload".to_string(), "REWRITE", Payload),
        ("RTSP::collect".to_string(), "RTSP", Collect),
        ("RTSP::payload".to_string(), "RTSP", Payload),
        ("RTSP::release".to_string(), "RTSP", Release),
        ("SCTP::collect".to_string(), "SCTP", Collect),
        ("SCTP::payload".to_string(), "SCTP", Payload),
        ("SCTP::release".to_string(), "SCTP", Release),
        ("SIP::payload".to_string(), "SIP", Payload),
        ("SSL::collect".to_string(), "SSL", Collect),
        ("SSL::payload".to_string(), "SSL", Payload),
        ("SSL::release".to_string(), "SSL", Release),
        ("TCP::collect".to_string(), "TCP", Collect),
        ("TCP::payload".to_string(), "TCP", Payload),
        ("TCP::release".to_string(), "TCP", Release),
        ("UDP::payload".to_string(), "UDP", Payload),
        ("WS::collect".to_string(), "WS", Collect),
        ("WS::payload".to_string(), "WS", Payload),
        ("WS::release".to_string(), "WS", Release),
        ("XML::payload".to_string(), "XML", Payload),
    ];
    expected.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(actual, expected);

    // UDP::release ends UDP::hold rather than a payload-collection lifecycle.
    // XML::collect/release are retired legacy XML parser controls; the current
    // XML::payload is event-owned. Neither pair belongs in this inventory.
    assert!(reg.data_collection_operation("UDP::release").is_none());
    assert!(reg.data_collection_operation("XML::collect").is_none());
    assert!(reg.data_collection_operation("XML::release").is_none());
}

#[test]
fn mqtt_payload_forms_declare_event_and_collection_contracts() {
    use tcl_registry::PayloadCollectionRequirement::{ExplicitCollect, NotRequired};

    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let operation = reg
        .data_collection_operation("MQTT::payload")
        .expect("MQTT payload lifecycle descriptor");
    assert_eq!(
        operation.payload_requirement_for_args(&[]),
        Some(ExplicitCollect)
    );
    assert_eq!(
        operation.payload_requirement_for_args(&["append", "bytes"]),
        Some(ExplicitCollect)
    );
    for form in ["length", "replace", "prepend"] {
        assert_eq!(
            operation.payload_requirement_for_args(&[form]),
            Some(NotRequired),
            "{form} is available on the current MQTT PUBLISH message"
        );
    }
    assert_eq!(operation.payload_requirement_for_args(&["$form"]), None);

    let mqtt = reg.get("MQTT::payload").expect("MQTT payload spec");
    let ingress = mqtt.event_requirements_for_args(&["length"]);
    assert!(ingress.matched_form);
    assert!(ingress.only_in.contains(&"MQTT_CLIENT_INGRESS"));
    let collected = mqtt.event_requirements_for_args(&[]);
    assert!(collected.matched_form);
    assert_eq!(collected.only_in, &["MQTT_CLIENT_DATA", "MQTT_SERVER_DATA"]);
    let dynamic = mqtt.event_requirements_for_args(&["$form"]);
    assert!(!dynamic.matched_form);
    assert_eq!(dynamic.only_in.len(), 0);

    let diameter = reg
        .data_collection_operation("DIAMETER::payload")
        .expect("DIAMETER payload availability descriptor");
    assert_eq!(
        diameter.payload_requirement_for_args(&[]),
        Some(NotRequired)
    );
    assert!(reg.get("DIAMETER::collect").is_none());
}
