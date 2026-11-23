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
use crate::model::ModelObject;
use crate::parser::parse_bigip_conf;

#[test]
fn policy_values_handle_multibyte_utf8() {
    // Multibyte chars inside a quoted value must advance by the char's full
    // UTF-8 width, not one byte, or the slice lands mid-char and panics.
    assert_eq!(
        parse_policy_values_block("{ \"café.example.com\" }"),
        vec!["café.example.com".to_owned()]
    );
    // A backslash-escaped multibyte char is handled too.
    assert_eq!(
        parse_policy_values_block("{ \"a\\ïb\" }"),
        vec!["aïb".to_owned()]
    );
}

fn config() -> crate::parser::BigipConfig {
    let src = include_str!("../../../../../samples/bigip/bigip.conf");
    parse_bigip_conf(src, "Common")
}

fn find<'a>(cfg: &'a crate::parser::BigipConfig, table: &str, fp: &str) -> &'a ModelObject {
    &cfg.objects
        .iter()
        .find(|p| p.table_name == table && p.full_path == fp)
        .unwrap_or_else(|| panic!("missing {table} {fp}"))
        .object
}

#[test]
fn pool_members() {
    let cfg = config();
    let ModelObject::Pool(p) = find(&cfg, "pools", "/Common/web_pool") else {
        panic!("not a pool");
    };
    assert_eq!(p.monitor, "/Common/my_http_monitor");
    assert_eq!(p.load_balancing_mode, "round-robin");
    assert_eq!(p.members.syntax, ListSyntax::KeyedBlock);
    assert_eq!(p.members.len(), 2);
    let keys: Vec<&str> = p.members.items.iter().map(|i| i.key.as_str()).collect();
    assert_eq!(keys, vec!["/Common/web1:80", "/Common/web2:80"]);

    // Verify the typed member parse via parse_pool_members directly so
    // address / port / field_offsets are exercised (the value layer has
    // no pool-member list variant to carry them on the BigipList).
    let body = "\n    /Common/web1:80 {\n        address 10.0.1.10\n    }\n";
    let members = parse_pool_members(body, 100);
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].name, "/Common/web1:80");
    assert_eq!(
        members[0].address.as_ref().unwrap().to_string(),
        "10.0.1.10"
    );
    assert_eq!(members[0].port, 80);
    assert!(members[0].field_offsets.contains_key("address"));
}

#[test]
fn pool_member_port_reads_the_tmsh_service_from_each_key() {
    let body = r"{
        /Common/folder/v6:443 { address 2001:db8::53%0 }
        /Common/2001:db8::54%0.8443 { address 2001:db8::54%0 }
        /Common/web:http { address 192.0.2.53 }
        /Common/numeric:0 { address 192.0.2.54 port 80 }
    }";
    let members = parse_pool_members(body, 0);
    let ports: Vec<_> = members.iter().map(|member| member.port).collect();
    assert_eq!(ports, [443, 8443, 80, 0]);
}

#[test]
fn empty_pool_has_no_members() {
    let cfg = config();
    let ModelObject::Pool(p) = find(&cfg, "pools", "/Common/empty_pool") else {
        panic!("not a pool");
    };
    assert_eq!(p.members.len(), 0);
}

#[test]
fn pool_member_port_from_name_without_address() {
    // No explicit address -> port parsed from the ``:80`` name tail.
    let body = "{ /Common/n1:8080 { } }";
    let members = parse_pool_members(body, 0);
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].port, 8080);
    assert!(members[0].address.is_none());
}

#[test]
fn snatpool_members() {
    let cfg = config();
    let ModelObject::SnatPool(s) = find(&cfg, "snat_pools", "/Common/my_snatpool") else {
        panic!("not a snatpool");
    };
    assert_eq!(s.members.syntax, ListSyntax::BracedSpaceSeparated);
    assert_eq!(s.members.paths(), vec!["10.0.2.100", "10.0.2.101"]);
}

#[test]
fn node_address() {
    let cfg = config();
    let ModelObject::Node(n) = find(&cfg, "nodes", "/Common/web1") else {
        panic!("not a node");
    };
    assert_eq!(n.address.as_ref().unwrap().to_string(), "10.0.1.10");
    assert!(n.fqdn.is_none());
}

#[test]
fn monitor_fields() {
    let cfg = config();
    let ModelObject::Monitor(m) = find(&cfg, "monitors", "/Common/my_http_monitor") else {
        panic!("not a monitor");
    };
    assert_eq!(m.monitor_type, "http");
    assert_eq!(m.defaults_from, "/Common/http");
    assert_eq!(m.interval, "10");
    assert_eq!(m.timeout, "31");
    // send is _unquote-d.
    assert_eq!(
        m.send,
        "GET /health HTTP/1.1\\r\\nHost: localhost\\r\\n\\r\\n"
    );
    assert_eq!(m.recv, "200 OK");
}

#[test]
fn profile_types() {
    let cfg = config();
    let expect = [
        ("/Common/my_http_profile", ProfileType::Http, "", ""),
        ("/Common/my_tcp_profile", ProfileType::Tcp, "", ""),
        (
            "/Common/my_clientssl",
            ProfileType::ClientSsl,
            "/Common/default.crt",
            "/Common/default.key",
        ),
        ("/Common/my_serverssl", ProfileType::ServerSsl, "", ""),
        ("/Common/my_oneconnect", ProfileType::OneConnect, "", ""),
    ];
    for (fp, ty, cert, key) in expect {
        let ModelObject::Profile(p) = find(&cfg, "profiles", fp) else {
            panic!("not a profile");
        };
        assert_eq!(p.profile_type, ty, "type for {fp}");
        assert_eq!(p.cert, cert, "cert for {fp}");
        assert_eq!(p.key, key, "key for {fp}");
    }
}

#[test]
fn persistence_fields() {
    let cfg = config();
    let ModelObject::Persistence(c) = find(&cfg, "persistence", "/Common/my_cookie_persist") else {
        panic!("not persistence");
    };
    assert_eq!(c.persistence_type, "cookie");
    assert_eq!(c.cookie_name, "SERVERID");
    assert_eq!(c.defaults_from, "/Common/cookie");
    let ModelObject::Persistence(s) = find(&cfg, "persistence", "/Common/my_source_persist") else {
        panic!("not persistence");
    };
    assert_eq!(s.persistence_type, "source-addr");
    assert_eq!(s.timeout, "300");
}

#[test]
fn bigip_21_1_ai_profiles_are_typed() {
    let source = "ltm profile aimcp /Common/ai { }\n\
                      ltm profile json /Common/json_ai { maximum-entries 4096 }\n\
                      ltm profile sse /Common/sse_ai { max-field-name-size 2048 }\n\
                      ltm persistence mcp /Common/mcp_ai { mcp-encryption-passphrase secret }\n";
    let cfg = parse_bigip_conf(source, "Common");

    let ModelObject::Profile(aimcp) = find(&cfg, "profiles", "/Common/ai") else {
        panic!("not an AIMCP profile");
    };
    assert_eq!(aimcp.profile_type, ProfileType::Aimcp);

    let ModelObject::Profile(json) = find(&cfg, "profiles", "/Common/json_ai") else {
        panic!("not a JSON profile");
    };
    assert_eq!(json.profile_type, ProfileType::Json);
    assert_eq!(json.maximum_entries, "4096");

    let ModelObject::Profile(sse) = find(&cfg, "profiles", "/Common/sse_ai") else {
        panic!("not an SSE profile");
    };
    assert_eq!(sse.profile_type, ProfileType::Sse);
    assert_eq!(sse.max_field_name_size, "2048");

    let ModelObject::Persistence(mcp) = find(&cfg, "persistence", "/Common/mcp_ai") else {
        panic!("not MCP persistence");
    };
    assert_eq!(mcp.persistence_type, "mcp");
    assert_eq!(mcp.mcp_encryption_passphrase, "secret");
}

#[test]
fn data_groups() {
    let cfg = config();
    let expect: [(&str, &str, &[&str]); 4] = [
        (
            "/Common/allowed_hosts",
            "string",
            &["www.example.com", "api.example.com", "admin.example.com"],
        ),
        (
            "/Common/blocked_ips",
            "ip",
            &["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"],
        ),
        (
            "/Common/uri_redirect_map",
            "string",
            &["/old-path", "/legacy"],
        ),
        ("/Common/unused_datagroup", "string", &["test_entry"]),
    ];
    for (fp, vt, records) in expect {
        let ModelObject::DataGroup(d) = find(&cfg, "data_groups", fp) else {
            panic!("not a data-group");
        };
        assert_eq!(d.kind, DataGroupType::Internal, "kind for {fp}");
        assert_eq!(d.value_type, vt, "value_type for {fp}");
        assert_eq!(d.records, records, "records for {fp}");
    }
}

#[test]
fn rule_source_is_raw_body() {
    let cfg = config();
    let ModelObject::Rule(r) = find(&cfg, "rules", "/Common/ip_blocklist") else {
        panic!("not a rule");
    };
    assert!(r.source.starts_with("when CLIENT_ACCEPTED"));
    assert!(r.source.contains("blocked_ips"));
    assert_eq!(r.description, "");
}

#[test]
fn virtual_structured_fields() {
    let cfg = config();
    let ModelObject::VirtualServer(v) = find(&cfg, "virtual_servers", "/Common/www_vs") else {
        panic!("not a virtual");
    };
    assert_eq!(
        v.destination.as_ref().unwrap().to_string(),
        "/Common/10.0.0.1:443"
    );
    assert_eq!(v.pool, "/Common/web_pool");

    assert_eq!(v.profiles.syntax, ListSyntax::KeyedBlock);
    assert_eq!(v.profiles.len(), 5);
    let prof: Vec<(String, String)> = v
        .profiles
        .items
        .iter()
        .map(|i| {
            let ListItemValue::Profile(pa) = &i.value else {
                panic!("not a profile attachment");
            };
            (pa.path.clone(), pa.context.clone())
        })
        .collect();
    assert_eq!(
        prof,
        vec![
            ("/Common/my_http_profile".to_owned(), "all".to_owned()),
            ("/Common/my_tcp_profile".to_owned(), "all".to_owned()),
            ("/Common/my_clientssl".to_owned(), "clientside".to_owned()),
            ("/Common/my_serverssl".to_owned(), "serverside".to_owned()),
            ("/Common/my_oneconnect".to_owned(), String::new()),
        ]
    );

    assert_eq!(v.rules.syntax, ListSyntax::BracedSpaceSeparated);
    assert_eq!(
        v.rules.paths(),
        vec![
            "/Common/route_by_host",
            "/Common/ip_blocklist",
            "/Common/uri_rewrite"
        ]
    );

    assert_eq!(v.persist.syntax, ListSyntax::KeyedBlock);
    assert_eq!(v.persist.len(), 1);
    let ListItemValue::Persistence(pa) = &v.persist.items[0].value else {
        panic!("not a persistence attachment");
    };
    assert_eq!(pa.path, "/Common/my_cookie_persist");
    assert!(pa.default);

    // source-address-translation: type snat, pool fills snatpool.
    assert_eq!(v.source_address_translation, "snat");
    assert_eq!(v.snatpool, "/Common/my_snatpool");

    // pool_range captured from the reference parser.
    let pr = v.pool_range.unwrap();
    assert_eq!(
        (pr.start.line, pr.start.character, pr.start.offset),
        (192, 9, 4102)
    );
    assert_eq!(
        (pr.end.line, pr.end.character, pr.end.offset),
        (192, 24, 4117)
    );
}

#[test]
fn virtual_destination_for_api_vs() {
    let cfg = config();
    let ModelObject::VirtualServer(v) = find(&cfg, "virtual_servers", "/Common/api_vs") else {
        panic!("not a virtual");
    };
    assert_eq!(
        v.destination.as_ref().unwrap().to_string(),
        "/Common/10.0.0.2:443"
    );
}

#[test]
fn quoted_description_not_resplit_into_phantom_props() {
    // A quoted multi-word description whose inner words match known
    // property names (`enabled`) must stay intact and must not fabricate
    // sibling props. The real bare `disabled` flag must still register.
    let src = "\
ltm virtual /Common/tls_vs {
    description \"TLS enabled for clients\"
    disabled
    destination /Common/10.0.0.9:443
}
";
    let cfg = parse_bigip_conf(src, "Common");
    let ModelObject::VirtualServer(v) = find(&cfg, "virtual_servers", "/Common/tls_vs") else {
        panic!("not a virtual");
    };
    // Description preserved verbatim (unquoted), not truncated to "TLS".
    assert_eq!(v.description, "TLS enabled for clients");
    // Explicitly-disabled VS is not flipped to enabled by a phantom prop.
    assert_eq!(v.state, "disabled");
}

#[test]
fn split_inline_keys_is_quote_aware() {
    // Direct unit test of the splitter: the `enabled` inside the quoted
    // description is not treated as a property boundary; the trailing bare
    // `disabled` is.
    let known = vec!["enabled".to_owned(), "disabled".to_owned()];
    let out = split_inline_keys("\"TLS enabled for clients\" disabled", &known);
    assert_eq!(
        out.get("__head__").map(String::as_str),
        Some("\"TLS enabled for clients\"")
    );
    assert_eq!(out.get("disabled").map(String::as_str), Some(""));
    assert!(!out.contains_key("enabled"));
}

#[test]
fn gtm_topology_built_from_identifier() {
    let body = "\n    order 1\n    score 100\n";
    let topo = parse_gtm_topology(
        "ldns: subnet 10.0.0.0/8 server: subnet 10.1.0.0/16",
        body,
        Range::zero(),
    );
    assert_eq!(
        topo.name,
        "ldns: subnet 10.0.0.0/8 server: subnet 10.1.0.0/16"
    );
    assert_eq!(topo.full_path, topo.name);
    assert_eq!(topo.order, "1");
    assert_eq!(topo.score, "100");
}

#[test]
fn gtm_pool_members_and_record_type() {
    let body = "\n    members {\n        /Common/srv:vs1 {\n            ratio 2\n        }\n    }\n    load-balancing-mode round-robin\n";
    let pool = parse_gtm_pool("/Common/gp", body, "a", Range::zero());
    assert_eq!(pool.record_type, "a");
    assert_eq!(pool.members.len(), 1);
    assert_eq!(pool.members[0].name, "/Common/srv:vs1");
    assert_eq!(pool.members[0].ratio, "2");
    assert_eq!(pool.load_balancing_mode, "round-robin");
}

#[test]
fn gtm_wideip_strips_last_resort_prefix() {
    let body = "\n    pools {\n        /Common/p1\n    }\n    last-resort-pool a /Common/p2\n";
    let w = parse_gtm_wideip("/Common/w", body, "a", Range::zero());
    assert_eq!(w.record_type, "a");
    assert_eq!(w.pools, vec!["/Common/p1"]);
    assert_eq!(w.last_resort_pool, "/Common/p2");
}

#[test]
fn net_route_default_and_typed() {
    let body = "\n    network default\n    gw 10.0.0.1\n";
    let r = parse_net_route("/Common/default", body, Range::zero());
    assert!(r.is_default_route);
    assert_eq!(r.gw.as_ref().unwrap().to_string(), "10.0.0.1");

    let body2 = "\n    network 10.0.0.0/8\n    gw 192.168.1.1\n";
    let r2 = parse_net_route("/Common/r2", body2, Range::zero());
    assert!(!r2.is_default_route);
    assert!(r2.network.is_some());
    assert_eq!(r2.gw.as_ref().unwrap().to_string(), "192.168.1.1");
}

#[test]
fn net_self_address_typed() {
    let body = "\n    address 10.0.0.1/24\n    vlan /Common/internal\n    allow-service { tcp:443 udp:53 }\n";
    let s = parse_net_self("/Common/self1", body, Range::zero());
    assert!(s.address.is_some());
    assert_eq!(s.vlan, "/Common/internal");
    assert_eq!(s.allow_service, vec!["tcp:443", "udp:53"]);
}

#[test]
fn sys_ntp_restrict_entries() {
    let body = "\n    servers { 0.pool.ntp.org 1.pool.ntp.org }\n    timezone UTC\n    restrict {\n        default {\n            address 0.0.0.0\n            no-query enabled\n        }\n    }\n";
    let ntp = parse_sys_ntp("", body, Range::zero());
    assert_eq!(ntp.servers, vec!["0.pool.ntp.org", "1.pool.ntp.org"]);
    assert_eq!(ntp.timezone, "UTC");
    assert_eq!(ntp.restrict.len(), 1);
    assert_eq!(ntp.restrict[0].name, "default");
    assert_eq!(ntp.restrict[0].address, "0.0.0.0");
    assert_eq!(ntp.restrict[0].flags, vec!["no-query"]);
}

#[test]
fn sys_snmp_users_and_unquote() {
    let body = "\n    sys-contact \"admin@example.com\"\n    communities {\n        comm-public { }\n    }\n    users {\n        user1 {\n            username u1\n            security-level auth-privacy\n        }\n    }\n";
    let snmp = parse_sys_snmp("", body, Range::zero());
    assert_eq!(snmp.sys_contact, "admin@example.com");
    assert_eq!(snmp.communities, vec!["comm-public"]);
    assert_eq!(snmp.users.len(), 1);
    assert_eq!(snmp.users[0].username, "u1");
    assert_eq!(snmp.users[0].security_level, "auth-privacy");
}

#[test]
fn policy_rules_conditions_actions() {
    let body = "\n    strategy /Common/first-match\n    rules {\n        rule1 {\n            ordinal 1\n            conditions {\n                0 {\n                    http-host\n                    host\n                    equals\n                    values { www.example.com }\n                }\n            }\n            actions {\n                0 {\n                    forward\n                    select\n                    pool /Common/web_pool\n                }\n            }\n        }\n    }\n";
    let pol = parse_policy("/Common/pol", body, Range::zero());
    assert_eq!(pol.strategy, "first-match");
    assert_eq!(pol.rules.len(), 1);
    let r = &pol.rules[0];
    assert_eq!(r.name, "rule1");
    assert_eq!(r.ordinal, 1);
    assert_eq!(r.conditions.len(), 1);
    let c = &r.conditions[0];
    assert_eq!(c.operand, "http-host");
    assert_eq!(c.selector, "host");
    assert_eq!(c.operator, "equals");
    assert_eq!(c.values, vec!["www.example.com"]);
    assert_eq!(r.actions.len(), 1);
    let a = &r.actions[0];
    assert_eq!(a.target, "forward");
    assert_eq!(a.verb, "select");
    assert_eq!(a.pool, "/Common/web_pool");
}

#[test]
fn security_firewall_rule_names() {
    let body = "\n    rules {\n        allow_http {\n            action accept\n        }\n        deny_all {\n            action drop\n        }\n    }\n";
    let rl = parse_security_firewall_rule_list("/Common/rl", body, Range::zero());
    assert_eq!(rl.rules, vec!["allow_http", "deny_all"]);
}
