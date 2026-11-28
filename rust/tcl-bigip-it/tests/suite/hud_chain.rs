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

use tcl_bigip::hud_chain::*;
use tcl_bigip::parser::driver::parse_bigip_conf;

const CONF: &str = "\
ltm rule /Common/r1 {
when HTTP_REQUEST {
    set u [HTTP::uri]
}
when CLIENT_ACCEPTED { set c 1 }
}
ltm rule /Common/r2 {
priority 100
when HTTP_REQUEST { set u x }
when HTTP_REQUEST priority 500 { log local0. late }
}
ltm rule /Common/r3 {
when HTTP_REQUEST { log local0. r3 }
when CLIENTSSL_HANDSHAKE { log local0. tls }
}
ltm profile client-ssl /Common/my_clientssl { }
ltm virtual /Common/vs1 {
    destination /Common/10.0.0.1:443
    ip-protocol tcp
    profiles {
        /Common/http { }
        /Common/my_clientssl { context clientside }
        /Common/f5-tcp-progressive { }
    }
    rules {
        /Common/r1
        /Common/r2
        /Common/r3
    }
}
ltm virtual /Common/vs_tcp {
    destination /Common/10.0.0.2:80
    profiles {
        /Common/custom_thing { }
    }
    rules {
        /Common/r1
    }
}
";

fn chain(name: &str) -> HudChain {
    hud_chain(&parse_bigip_conf(CONF, "Common"), name).expect("virtual server")
}

#[test]
fn filters_resolve_config_and_base_profiles_in_layer_order() {
    crate::install();
    let c = chain("vs1");
    let types: Vec<_> = c.filters.iter().map(|f| f.profile_type).collect();
    assert_eq!(types, [Some("TCP"), Some("CLIENTSSL"), Some("HTTP")]);
    let sources: Vec<_> = c.filters.iter().map(|f| f.type_source).collect();
    assert_eq!(
        sources,
        [
            FilterTypeSource::BaseObject,
            FilterTypeSource::Config,
            FilterTypeSource::BaseObject
        ]
    );
    assert_eq!(c.filters[1].context.as_deref(), Some("clientside"));
    assert_eq!(c.implied_filters, Vec::<&str>::new());
    assert_eq!(c.destination.as_deref(), Some("/Common/10.0.0.1:443"));
}

#[test]
fn handlers_merge_by_priority_then_rule_list_position() {
    crate::install();
    let c = chain("/Common/vs1");
    let req = c
        .events
        .iter()
        .find(|e| e.event == "HTTP_REQUEST")
        .expect("HTTP_REQUEST");
    let order: Vec<_> = req
        .handlers
        .iter()
        .map(|h| (h.rule.as_str(), h.priority, h.ordinal))
        .collect();
    assert_eq!(
        order,
        [
            ("/Common/r2", 100, 1),
            ("/Common/r1", 500, 1),
            ("/Common/r2", 500, 2),
            ("/Common/r3", 500, 1),
        ]
    );
    assert_eq!(req.handlers[0].tied_with, Vec::<String>::new());
    assert_eq!(req.handlers[1].tied_with, ["/Common/r2", "/Common/r3"]);
    assert_eq!(req.raised, Some(true));
    assert_eq!(req.multiplicity, "per_request");
}

#[test]
fn events_follow_canonical_order() {
    crate::install();
    let c = chain("vs1");
    let names: Vec<_> = c.events.iter().map(|e| e.event.as_str()).collect();
    assert_eq!(
        names,
        ["CLIENT_ACCEPTED", "CLIENTSSL_HANDSHAKE", "HTTP_REQUEST"]
    );
}

#[test]
fn unresolved_filters_leave_unraised_events_undecided() {
    crate::install();
    let c = chain("vs_tcp");
    assert_eq!(c.filters[0].type_source, FilterTypeSource::Unresolved);
    assert_eq!(c.implied_filters, ["TCP"]);
    let req = c
        .events
        .iter()
        .find(|e| e.event == "HTTP_REQUEST")
        .expect("HTTP_REQUEST");
    assert_eq!(req.raised, None);
    let accepted = c
        .events
        .iter()
        .find(|e| e.event == "CLIENT_ACCEPTED")
        .expect("CLIENT_ACCEPTED");
    assert_eq!(accepted.raised, Some(true));
}

#[test]
fn a_complete_stack_without_the_filter_reports_the_event_unraised() {
    crate::install();
    let conf = CONF.replace("/Common/custom_thing { }", "/Common/tcp { }");
    let c = hud_chain(&parse_bigip_conf(&conf, "Common"), "vs_tcp").expect("vs");
    let req = c
        .events
        .iter()
        .find(|e| e.event == "HTTP_REQUEST")
        .expect("HTTP_REQUEST");
    assert_eq!(req.raised, Some(false));
    assert_eq!(req.requires, ["FASTHTTP", "HTTP"]);
}

#[test]
fn unknown_rules_are_listed_without_handlers() {
    crate::install();
    let conf = CONF.replace("/Common/r3\n", "/Common/missing\n");
    let c = hud_chain(&parse_bigip_conf(&conf, "Common"), "vs1").expect("vs");
    let missing = &c.rules[2];
    assert_eq!(missing.reference, "/Common/missing");
    assert_eq!(missing.full_path, None);
    assert!(!missing.handlers_known);
    assert_eq!(missing.position, 3);
}

fn bigip6038(source: &str) -> bool {
    tcl_bigip::validator::validate_bigip_source(source, "Common")
        .iter()
        .any(|d| d.code == "BIGIP6038")
}

#[test]
fn bigip6038_types_base_configuration_profiles_and_abstains_on_unknown_ones() {
    crate::install();
    let rule = "ltm rule /Common/http_rule {\nwhen HTTP_REQUEST { return }\n}\n";
    let vs = |profiles: &str| {
        format!(
            "{rule}ltm virtual /Common/vs {{\n  rules {{ /Common/http_rule }}\n  \
                 profiles {{ {profiles} }}\n}}\n"
        )
    };
    assert!(!bigip6038(&vs(
        "/Common/f5-tcp-progressive { } /Common/http { }"
    )));
    assert!(!bigip6038(&vs("/Common/defined_elsewhere { }")));
    assert!(bigip6038(&vs("/Common/tcp { }")));
}

#[test]
fn every_virtual_server_gets_a_chain() {
    crate::install();
    let chains = hud_chains(&parse_bigip_conf(CONF, "Common"));
    let names: Vec<_> = chains.iter().map(|c| c.virtual_server.as_str()).collect();
    assert_eq!(names, ["/Common/vs1", "/Common/vs_tcp"]);
    assert!(hud_chain(&parse_bigip_conf(CONF, "Common"), "nope").is_none());
}
