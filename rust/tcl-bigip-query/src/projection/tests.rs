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

use super::{KINDS, MODULE_NAMES, placed_kind};
use std::collections::BTreeSet;
use tcl_bigip::parser::parse_bigip_conf;

/// The root's module containers and the configuration lexer's stanza
/// modules are one set: a module this projection navigates is one whose
/// header the editor layers paint as a header.
#[test]
fn root_modules_are_the_configuration_modules() {
    let root: BTreeSet<&str> = MODULE_NAMES.iter().copied().collect();
    let lexer: BTreeSet<&str> = tcl_bigip::conf_tokens::MODULES.iter().copied().collect();
    assert_eq!(root, lexer);
}

/// The committed configs the coverage gate reads. Between them they carry
/// every module the parser types, so a kind the model gains without a
/// projection shows up here.
const FIXTURES: &[(&str, &str)] = &[
    (
        "bigip.conf",
        include_str!("../../../../samples/bigip/bigip.conf"),
    ),
    (
        "bigip_base.conf",
        include_str!("../../../../samples/bigip/bigip_base.conf"),
    ),
    (
        "ltm.conf",
        include_str!("../../../../samples/for_f5_query/ltm.conf"),
    ),
    (
        "gtm.conf",
        include_str!("../../../../samples/for_f5_query/gtm.conf"),
    ),
    (
        "apm.conf",
        include_str!("../../../../samples/for_f5_query/apm.conf"),
    ),
    (
        "lab_localhost.conf",
        include_str!("../../../../samples/for_f5_query/sysadmin/lab_localhost.conf"),
    ),
    (
        "lab_platform.conf",
        include_str!("../../../../samples/for_f5_query/sysadmin/lab_platform.conf"),
    ),
    (
        "tier1-ltm-ha.conf",
        include_str!("../../../../samples/for_f5_query/multitier/tier1-ltm-ha.conf"),
    ),
    (
        "tier3-reaggregator.conf",
        include_str!("../../../../samples/for_f5_query/multitier/tier3-reaggregator.conf"),
    ),
    (
        "device-01.bigip.conf",
        include_str!("../../../../rust/bigip-report-gen/python/tests/data/device-01.bigip.conf"),
    ),
    (
        "tier2-c05-ltm-ha.conf",
        include_str!("../../../../samples/for_f5_query/multitier/tier2-c05-ltm-ha.conf"),
    ),
    (
        "graph_pilot.conf",
        include_str!("../../../../rust/tcl-bigip/tests/fixtures/graph_pilot.conf"),
    ),
];

/// `BigipConfig` tables the parser fills with a typed struct that the DSL
/// deliberately does not project, as the committed fixtures exercise them.
/// Each entry is a kind an operator cannot navigate to, so adding one is a
/// decision; the gate below fails when a fixture grows a typed kind that is
/// neither projected nor listed here.
///
/// Most are singletons or long-tail settings with little query value.
/// `apm_policy_customization_group` is the notable exception — APM policy
/// agents name one in their `customization-group`, which the projection
/// keeps as a path string precisely because this kind is not navigable.
const UNPROJECTED_TABLES: &[&str] = &[
    "analytics_global_settings",
    "apm_aaa_localdb",
    "apm_epsec_epsec_package",
    "apm_policy_customization_group",
    "apm_profile_access",
    "apm_profile_connectivity",
    "apm_resource_leasepool",
    "apm_resource_network_access",
    "apm_resource_webtop",
    "asm_policies",
    "auth_apm_auths",
    "auth_partitions",
    "auth_password_policy",
    "auth_radius",
    "auth_radius_servers",
    "auth_remote_user",
    "auth_source",
    "auth_users",
    "ilx_global_settings",
    "ltm_auth_radius_servers",
    "ltm_classification_auto_update_settings",
    "ltm_default_node_monitor",
    "net_ipsec_ike_daemon",
    "net_self_allow",
    "pem_gs_analytics",
    "pem_gs_gx",
    "pem_gs_policy",
    "security_bot_defense_profiles",
    "security_dos_ipv6_ext_hdr",
    "security_dos_profiles",
    "security_dos_udp_portlists",
    "security_firewall_config_change_log",
    "security_pi_compliance_maps",
    "security_pi_compliance_objects",
    "security_scrubber_profiles",
    "sys_compatibility_level",
    "sys_diags_ihealth",
    "sys_ecm_cloud_provider",
    "sys_management_ovsdb",
    "sys_software_update",
    "wom_endpoint_discovery",
];

/// Kinds in `KINDS` that no committed fixture carries, so the gate below
/// cannot confirm their `placed_kind` arm exists. Each is a hole in fixture
/// coverage rather than a decision; shrinking this list is how the gate
/// gets stronger.
const FIXTURE_UNCOVERED_KINDS: &[&str] = &[
    "apm ephemeral-auth ssh-security-config",
    "apm oauth db-instance",
    "gtm listener",
    "security nat destination-translation",
    "security nat policy",
    "security nat source-translation",
];

/// Every kind the projection claims to cover must be reachable: a kind in
/// `KINDS` that `placed_kind` never returns is dead weight, and a kind
/// `placed_kind` returns that is missing from `KINDS` is an object the
/// parser types but no container exposes — the failure mode that kept
/// `cm ha-group` out of the DSL while the model parsed it.
#[test]
fn every_parsed_kind_is_projected_or_listed() {
    let mut projected: BTreeSet<&str> = BTreeSet::new();
    let mut unprojected: BTreeSet<&str> = BTreeSet::new();
    for (name, source) in FIXTURES {
        let config = parse_bigip_conf(source, "Common");
        for placed in &config.objects {
            match placed_kind(placed) {
                Some(kind) => {
                    assert!(
                        KINDS.iter().any(|(k, _)| *k == kind),
                        "{name}: parser produced {kind:?}, which no module container exposes"
                    );
                    projected.insert(kind);
                }
                None => {
                    unprojected.insert(placed.table_name);
                }
            }
        }
    }
    let listed: BTreeSet<&str> = UNPROJECTED_TABLES.iter().copied().collect();
    let unlisted: Vec<&&str> = unprojected.difference(&listed).collect();
    assert!(
        unlisted.is_empty(),
        "the parser types these kinds but the DSL projects none of them; \
             add each to KINDS with a label, or record it in UNPROJECTED_TABLES: {unlisted:?}"
    );
    let stale: Vec<&&str> = listed.difference(&unprojected).collect();
    assert!(
        stale.is_empty(),
        "UNPROJECTED_TABLES lists tables no fixture produces any more: {stale:?}"
    );
    // The reverse direction: a row added to `KINDS` whose `placed_kind`
    // arm is missing exposes a module container that can never select an
    // object. Nothing else catches that, so require every row to be either
    // reached from a fixture or recorded as one the fixtures do not carry.
    let uncovered: BTreeSet<&str> = FIXTURE_UNCOVERED_KINDS.iter().copied().collect();
    let unreachable: Vec<&str> = KINDS
        .iter()
        .map(|(kind, _)| *kind)
        .filter(|kind| !projected.contains(kind) && !uncovered.contains(kind))
        .collect();
    assert!(
        unreachable.is_empty(),
        "these kinds are in KINDS but no fixture ever projected one, so the \
             module container exposes a label that can never select an object; \
             add the `placed_kind` arm, or add fixture coverage, or record it in \
             FIXTURE_UNCOVERED_KINDS: {unreachable:?}"
    );
    let now_covered: Vec<&&str> = FIXTURE_UNCOVERED_KINDS
        .iter()
        .filter(|kind| projected.contains(**kind))
        .collect();
    assert!(
        now_covered.is_empty(),
        "FIXTURE_UNCOVERED_KINDS lists kinds the fixtures now do project; \
             drop them: {now_covered:?}"
    );
}

/// Labels are the DSL's identifiers, so they must be unique within a
/// module; kinds are `PathRef` targets, so they must be unique outright.
#[test]
fn kinds_and_labels_are_unambiguous() {
    let mut kinds = BTreeSet::new();
    for (kind, _) in KINDS {
        assert!(kinds.insert(*kind), "duplicate kind {kind:?} in KINDS");
    }
    let mut module_labels = BTreeSet::new();
    for (kind, label) in KINDS {
        let module = kind.split(' ').next().unwrap_or_default();
        assert!(
            module_labels.insert((module, *label)),
            "duplicate label {label:?} in module {module:?}"
        );
    }
}
