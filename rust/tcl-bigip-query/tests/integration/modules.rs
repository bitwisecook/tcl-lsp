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

//! Per-module projection coverage for `net` / `sys` / `cm` / `apm` and
//! `ltm snat-translation`, evaluated against the committed sample configs.
//!
//! These are the modules the DSL reference and the operator cookbook
//! (`samples/for_f5_query/sysadmin-queries.md`) address, and every query here
//! is one a documented recipe runs — a `.net.self[]` audit, an APM policy
//! walk, a SNAT-translation reverse lookup, a cert-expiry roster. Each asserts
//! the projected field names and the path-ref auto-deref chains, so a kind
//! silently dropping out of `module_kinds()` fails here rather than in a
//! `no entry` at the operator's terminal.

use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip_query::eval::{EvalContext, Root, evaluate};
use tcl_bigip_query::output::render;
use tcl_bigip_query::parser::parse_query;

const LAB_LOCALHOST: &str =
    include_str!("../../../../samples/for_f5_query/sysadmin/lab_localhost.conf");
const LAB_PLATFORM: &str =
    include_str!("../../../../samples/for_f5_query/sysadmin/lab_platform.conf");
const APM: &str = include_str!("../../../../samples/for_f5_query/apm.conf");
const TIER1: &str = include_str!("../../../../samples/for_f5_query/multitier/tier1-ltm-ha.conf");
const TIER2: &str =
    include_str!("../../../../samples/for_f5_query/multitier/tier2-c01-ltm-ha.conf");
const TIER3: &str =
    include_str!("../../../../samples/for_f5_query/multitier/tier3-reaggregator.conf");
const BIGIP_BASE: &str = include_str!("../../../../samples/bigip/bigip_base.conf");

fn run(source: &str, uri: &str, query: &str) -> Result<String, String> {
    let config = parse_bigip_conf(source, "Common");
    let root = Root::bigip(uri, source.to_owned(), config);
    let mut ctx = EvalContext::new(root);
    let prog = parse_query(query).map_err(|e| e.to_string())?;
    let values = evaluate(&prog, &mut ctx).map_err(|e| e.to_string())?;
    render(&values, "raw").map_err(|e| e.to_string())
}

/// Assert a `--raw` render, with the query echoed on failure.
fn raw_eq(source: &str, uri: &str, query: &str, expected: &str) {
    match run(source, uri, query) {
        Ok(got) => assert_eq!(got.trim_end(), expected, "query: {query}"),
        Err(e) => panic!("query: {query}\n  failed: {e}"),
    }
}

// net

#[test]
fn net_self_projects_address_vlan_and_allow_service() {
    // §10.1 of the cookbook: the self-IP allow-service audit.
    raw_eq(
        LAB_LOCALHOST,
        "lab_localhost.conf",
        r#".net.self[] | tsv(.name, .address, .vlan, join(."allow-service", ","))"#,
        "198.51.100.5\t198.51.100.5/24\t/Common/external\tnone\n\
         10.1.0.5\t10.1.0.5/24\t/Common/internal\tall\n\
         10.2.0.5\t10.2.0.5/24\t/Common/internal\tdefault",
    );
}

#[test]
fn net_self_vlan_derefs_into_net_vlan() {
    // `self.vlan` is a PathRef into `net vlan`, so `.vlan.tag` walks the
    // whole chain in one step.
    raw_eq(
        LAB_LOCALHOST,
        "lab_localhost.conf",
        r".net.self[] | tsv(.name, .vlan.tag, count(.vlan.interfaces))",
        "198.51.100.5\t100\t1\n10.1.0.5\t200\t1\n10.2.0.5\t200\t1",
    );
}

#[test]
fn net_vlan_projects_tag_and_interfaces() {
    raw_eq(
        LAB_LOCALHOST,
        "lab_localhost.conf",
        r#".net.vlan[] | tsv(.name, .tag, join([.interfaces[]], ","))"#,
        "external\t100\t1.1\ninternal\t200\t1.2",
    );
}

#[test]
fn net_route_and_route_domain_project() {
    raw_eq(
        TIER2,
        "tier2-c01-ltm-ha.conf",
        r#".net.route[] | tsv(.name, .network, .gw, ."is-default-route")"#,
        "default_to_t3\tdefault\t10.3.21.254\ttrue",
    );
    raw_eq(
        TIER1,
        "tier1-ltm-ha.conf",
        r#"[.net["route-domain"][] | select(.id > 110) | tsv(.name, .id)] | join(., " ")"#,
        "to_t2_c11\t111 to_t2_c12\t112",
    );
    raw_eq(
        BIGIP_BASE,
        "bigip_base.conf",
        r#".net["route-domain"][] | tsv(.name, .id, join([.vlans[]], ","))"#,
        "0\t0\t/Common/internal,/Common/external",
    );
}

#[test]
fn net_vlan_interfaces_and_route_interface_deref() {
    // `vlan.interfaces[]` resolves against `net interface`, and a route's
    // `interface` against the VLAN it names.
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".net.vlan[] | tsv(.name, .tag, join([.interfaces[].mtu], ","))"#,
        "mgmt\t10\t9198",
    );
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r".net.route[] | tsv(.name, .network, .interface, .interface.tag)",
        "mgmt_default\t198.18.0.0/15\t/Common/mgmt\t10",
    );
}

#[test]
fn net_platform_kinds_project() {
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#"[.net.interface[].name, .net.stp[].name, .net.tunnel[].name,
            .net["port-list"][].name, .net["dns-resolver"][].name] | join(., " ")"#,
        "1.1 1.2 cist http-tunnel web-ports lab-resolver",
    );
    // `net stp.interfaces[]` / `net port-list.ports[]` keep their TMSH
    // spelling and project as lists.
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#"join([.net.stp[].interfaces[]], ",") + " | " + join([.net["port-list"][].ports[]], ",")"#,
        "1.1,1.2 | 80,443,8443",
    );
}

// sys

#[test]
fn sys_file_ssl_cert_projects_the_x509_metadata_fields() {
    // The field names `x509_from_config` reads, in their TMSH spelling.
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".sys["file-ssl-cert"][] | tsv(.name, .subject, ."expiration-string", ."key-type")"#,
        "app.example.test.crt\t\"CN=app.example.test,O=Example,C=AU\"\t\
         \"Jun  4 12:00:00 2026 GMT\"\trsa-public\n\
         legacy.example.test.crt\t\"CN=legacy.example.test,O=Example,C=AU\"\t\
         \"Jan  1 00:00:00 2025 GMT\"\trsa-public",
    );
    // `cache-path` is what `ucs_cert` locates the PEM in the archive by.
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".sys["file-ssl-cert"]["app.example.test.crt"]."cache-path""#,
        "/config/filestore/files_d/Common_d/certificate_d/:Common:app.example.test.crt_1",
    );
}

#[test]
fn sys_file_ssl_cert_projects_through_x509_from_config() {
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".sys["file-ssl-cert"][] | x509_from_config(.) as $x
           | tsv(.name, $x.not_after, $x.key_size)"#,
        "app.example.test.crt\t2026-06-04T12:00:00+00:00\t2048\n\
         legacy.example.test.crt\t2025-01-01T00:00:00+00:00\t1024",
    );
}

#[test]
fn sys_file_ssl_key_projects() {
    raw_eq(
        BIGIP_BASE,
        "bigip_base.conf",
        r#".sys["file-ssl-key"][] | tsv(.name, ."source-path")"#,
        "f5_api_com.key\tfile:///config/ssl/ssl.key/f5_api_com.key",
    );
}

#[test]
fn sys_singletons_hold_one_entry_each() {
    // `sys dns` / `ntp` / `snmp` / `global-settings` parse with an empty
    // full-path, so they are read by streaming the container.
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".sys["global-settings"][] | tsv(.hostname, ."gui-setup", ."mgmt-dhcp")"#,
        "lab-a.example.test\tdisabled\tdisabled",
    );
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#"join([.sys.dns[]."name-servers"[]], ",") + " " + join([.sys.dns[].search[]], ",")"#,
        "10.1.0.53,10.1.0.54 example.test",
    );
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#"join([.sys.ntp[].servers[]], ",") + " " + .sys.ntp[].timezone"#,
        "time1.example.test,time2.example.test UTC",
    );
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".sys.snmp[] | tsv(."sys-contact", ."sys-location")"#,
        "netops@example.test\tlab rack 3",
    );
}

#[test]
fn sys_provision_folder_and_management_route_project() {
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r".sys.provision[] | tsv(.name, .level)",
        "ltm\tnominal\napm\tminimum",
    );
    // `folder.device-group` / `.traffic-group` deref into the `cm` kinds.
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".sys.folder[] | tsv(.name, ."device-group".type, ."traffic-group"."unit-id")"#,
        "Common\tsync-failover\t1",
    );
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".sys["management-route"][] | tsv(.name, .network, .gateway)"#,
        "default\tdefault\t192.0.2.1",
    );
}

// cm

#[test]
fn cm_device_and_device_group_project() {
    raw_eq(
        TIER1,
        "tier1-ltm-ha.conf",
        r#".cm.device[] | tsv(.name, .hostname, ."management-ip")"#,
        "t1-a\tt1-a.example.test\t192.0.2.11\nt1-b\tt1-b.example.test\t192.0.2.12",
    );
    // `device-group.devices[]` derefs into `cm device`.
    raw_eq(
        TIER1,
        "tier1-ltm-ha.conf",
        r#".cm["device-group"][] | tsv(.name, .type, join([.devices[].hostname], ","))"#,
        "t1-failover\tsync-failover\tt1-a.example.test,t1-b.example.test",
    );
}

#[test]
fn cm_cert_key_traffic_group_and_trust_domain_project() {
    // `device.cert` / `device.key` deref into `cm cert` / `cm key`.
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".cm.device["lab-a"] | tsv(.name, .cert.subject, .key."key-size")"#,
        "lab-a\t\"CN=lab-a.example.test\"\t2048",
    );
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r".cm.cert[] | x509_from_config(.) | tsv(.subject, .not_after, .key_size)",
        "CN=lab-a.example.test\t2033-08-12T09:31:00+00:00\t2048",
    );
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".cm["traffic-group"][] | tsv(.name, ."default-device".hostname, ."unit-id")"#,
        "traffic-group-1\tlab-a.example.test\t1",
    );
    // `traffic-group.ha-group` derefs into `cm ha-group`.
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".cm["traffic-group"][] | tsv(.name, ."ha-group".name, ."ha-group"."active-bonus")"#,
        "traffic-group-1\tlab-ha\t10",
    );
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".cm["ha-group"][] | tsv(.name, ."enabled-state", join([.pools[]], ","))"#,
        "lab-ha\tenabled\t/Common/web_pool",
    );
    // `trust-domain.ca-cert` / `.ca-devices[]` deref too.
    raw_eq(
        LAB_PLATFORM,
        "lab_platform.conf",
        r#".cm["trust-domain"][] | tsv(.name, ."ca-cert".subject,
                                       join([."ca-devices"[].hostname], ","))"#,
        "Root\t\"CN=lab-a.example.test\"\tlab-a.example.test,lab-b.example.test",
    );
}

// apm

#[test]
fn apm_access_policy_projects_and_derefs_into_policy_items() {
    // §5.1 of the cookbook.
    raw_eq(
        APM,
        "apm.conf",
        r#".apm["access-policy"][]
           | tsv(.name, ."start-item".caption, ."default-ending", ([.items[]] | count))"#,
        "employee_login\tStart\t/Common/employee_login_end_deny\t5",
    );
}

#[test]
fn apm_policy_items_walk_in_order_with_their_agents() {
    // §5.2 of the cookbook: the policy trace.
    raw_eq(
        APM,
        "apm.conf",
        r#".apm["access-policy"]["/Common/employee_login"]
           | .items[] as $item
           | tsv($item.name, $item.caption, join([$item.agents[]], ","))"#,
        "employee_login_ent\tStart\t\n\
         employee_login_logon_page\tLogon Page\t/Common/employee_login_logon_page_ag\n\
         employee_login_localdb_auth\tLocalDB Auth\t/Common/employee_login_localdb_auth_ag\n\
         employee_login_end_allow\tAllow\t\n\
         employee_login_end_deny\tDeny",
    );
}

// ltm snat-translation

#[test]
fn ltm_snat_translation_resolves_snatpool_members() {
    // §6.5 of the cookbook: the SNAT-pool reverse lookup.
    raw_eq(
        TIER3,
        "tier3-reaggregator.conf",
        r#".ltm.snatpool[] as $sp
           | $sp.members[] as $m
           | tsv($sp.name, $m, .ltm["snat-translation"][str($m)].address)"#,
        "internet_snat\t/Common/snat_xlat_1\t198.51.100.101\n\
         internet_snat\t/Common/snat_xlat_2\t198.51.100.102\n\
         internet_snat\t/Common/snat_xlat_3\t198.51.100.103",
    );
}

// An uncovered module

#[test]
fn a_module_without_a_projection_reports_no_entry() {
    // `pem` carries no projection, so the error names the module and the
    // missing kind rather than silently yielding nothing.
    let err = run(LAB_PLATFORM, "lab_platform.conf", ".pem.policy[]").unwrap_err();
    assert_eq!(err, "pem: no entry 'policy'");
}
