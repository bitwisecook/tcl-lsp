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

//! Top-level driver — `parse_bigip_conf`.
//!
//! The container holds a `default_partition`,
//! the `generic_objects` index (every stanza lands one row), and the
//! typed objects (each carrying the `BigipConfig` field name it belongs
//! to so the `PyO3` layer can place it). The driver establishes the block
//! iteration, header resolution, partition-prefixing, and `generic_objects`
//! index.

use tcl_lexer::LineIndex;
use tcl_registry::bigip::BigipRegistry;

use crate::model::{BigipGenericObject, ModelObject};
use crate::range::Range;

use super::header::{ObjectTypeIndex, parse_generic_header};
use super::helpers::extract_blocks;

/// One typed object placed in a `BigipConfig` collection. `table_name`
/// is the `BigipConfig` attribute it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// The `BigipConfig` attribute name (e.g. `"pools"`).
    pub table_name: &'static str,
    /// Object full path (dict key; `""` for singletons).
    pub full_path: String,
    /// The parsed object.
    pub object: ModelObject,
}

/// Parsed BIG-IP configuration: a `default_partition`, the
/// `generic_objects` index, and the per-kind typed objects.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipConfig {
    /// The partition short-name this source belongs to.
    pub default_partition: String,
    /// `generic_objects`: `(key, object)` for every stanza, in source
    /// order. The key is `"<module>::<object_type>::<identifier or
    /// '<singleton>'>"`.
    pub generic_objects: Vec<(String, BigipGenericObject)>,
    /// Typed objects, each tagged with its destination `BigipConfig`
    /// field. In source order.
    pub objects: Vec<Placed>,
}

/// `(module, object_type)` pairs whose identifier is inherently
/// *unpartitioned* — a hardware, system-wide, or cluster-wide name that lives
/// outside any partition — so a `/Common/` prefix would be bogus and break
/// name lookups (issue 189).  `net interface 1.1` is the port `1.1`, not
/// `/Common/1.1`; `sys provision ltm` provisions the module `ltm`, not
/// `/Common/ltm`; `cm device`/`device-group`/`traffic-group` are cluster
/// objects keyed by (unpartitioned) device/group names.
const NO_PARTITION_PREFIX: &[(&str, &str)] = &[
    ("auth", "partition"),
    ("net", "interface"),
    ("net", "trunk"),
    ("sys", "provision"),
    ("cm", "device"),
    ("cm", "device-group"),
    ("cm", "traffic-group"),
];

/// Parse a BIG-IP configuration file into a [`BigipConfig`].
#[must_use]
pub fn parse_bigip_conf(source: &str, default_partition: &str) -> BigipConfig {
    let part = if default_partition.is_empty() {
        "Common"
    } else {
        default_partition
    };
    let partition_prefix = format!("/{}/", part.trim_matches('/'));

    let registry = BigipRegistry::build();
    let index = ObjectTypeIndex::build(&registry);
    let line_index = LineIndex::new(source);

    let mut config = BigipConfig {
        default_partition: default_partition.to_owned(),
        ..BigipConfig::default()
    };

    for block in extract_blocks(source) {
        // ``gtm topology`` carries a multi-token condition rather than a
        // full path; the driver builds the typed object here before
        // generic_objects and `continue`s, so it never lands a
        // generic_objects row.
        if let Some(topo_id) = block.header.strip_prefix("gtm topology ") {
            let range =
                Range::from_offsets(source, &line_index, block.start_offset, block.end_offset);
            let topo = super::bespoke::parse_gtm_topology(topo_id, &block.body, range);
            config.objects.push(Placed {
                table_name: "gtm_topologies",
                full_path: topo_id.to_owned(),
                object: ModelObject::GtmTopology(topo),
            });
            continue;
        }

        let Some((module, object_type, mut identifier)) =
            parse_generic_header(&block.header, &index)
        else {
            continue;
        };

        if !identifier.is_empty()
            && !identifier.starts_with('/')
            && !NO_PARTITION_PREFIX.contains(&(module.as_str(), object_type.as_str()))
        {
            identifier = format!("{partition_prefix}{identifier}");
        }

        let id_part = if identifier.is_empty() {
            "<singleton>"
        } else {
            identifier.as_str()
        };
        let key = format!("{module}::{object_type}::{id_part}");
        let range = Range::from_offsets(source, &line_index, block.start_offset, block.end_offset);
        config.generic_objects.push((
            key,
            BigipGenericObject {
                module: module.clone(),
                object_type: object_type.clone(),
                identifier: identifier.clone(),
                header: block.header.clone(),
                body: block.body.clone(),
                range: Some(range),
            },
        ));

        // Typed dispatch via the strict-header path.
        let ctx = super::bespoke::BespokeCtx {
            source,
            line_index: &line_index,
            block_start: block.start_offset,
        };
        if let Some(placed) = dispatch_block(
            &block.header,
            &module,
            &object_type,
            &identifier,
            &block.body,
            range,
            &partition_prefix,
            ctx,
        ) {
            config.objects.push(placed);
        }
    }

    config
}

/// Route one stanza to its typed object via the strict-header dispatch.
/// `generic_*` come from the generic header (already partition-prefixed)
/// for the bare-singleton fallback.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::wildcard_imports
)]
fn dispatch_block(
    header: &str,
    generic_module: &str,
    generic_type: &str,
    generic_identifier: &str,
    body: &str,
    range: Range,
    partition_prefix: &str,
    ctx: super::bespoke::BespokeCtx,
) -> Option<Placed> {
    use crate::model::ModelObject;
    use crate::model::r#gen::dispatch::{
        dispatch_ltm_tables, dispatch_minimal, dispatch_named, dispatch_singleton,
        parse_header_strict,
    };
    use crate::model::r#gen::parsers::*;

    let parsed = parse_header_strict(header).map(|(m, o, fp)| {
        let fp = if !fp.is_empty()
            && !fp.starts_with('/')
            && !NO_PARTITION_PREFIX.contains(&(m.as_str(), o.as_str()))
        {
            format!("{partition_prefix}{fp}")
        } else {
            fp
        };
        (m, o, fp)
    });

    let Some((module, object_type, full_path)) = parsed else {
        // Bare singleton: route the empty-identifier generic via the
        // singleton / minimal tables.
        if !generic_identifier.is_empty() {
            return None;
        }
        // Bespoke singletons (sys ntp / sys snmp) intercept before the
        // generated singleton dispatch.
        if let Some(p) = bespoke_override(generic_module, generic_type, "", body, range) {
            return Some(p);
        }
        if let Some((table, object)) =
            dispatch_singleton(generic_module, generic_type, "", body, range)
        {
            return Some(placed(table, "", object));
        }
        if let Some((table, object)) =
            dispatch_minimal(generic_module, generic_type, "", body, range)
        {
            return Some(placed(table, "", object));
        }
        return None;
    };

    let fp = full_path.as_str();

    // Bespoke named/minimal kinds (net route / net self / security firewall
    // rule-list) intercept before the generated dispatch tables.
    if let Some(p) = bespoke_override(&module, &object_type, fp, body, range) {
        return Some(p);
    }

    // Minimal pre-pass + named + singleton + rich ltm tables (generated).
    if let Some((table, object)) = dispatch_minimal(&module, &object_type, fp, body, range) {
        return Some(placed(table, fp, object));
    }
    if let Some((table, object)) = dispatch_named(&module, &object_type, fp, body, range) {
        return Some(placed(table, fp, object));
    }
    if full_path.is_empty()
        && let Some((table, object)) = dispatch_singleton(&module, &object_type, fp, body, range)
    {
        return Some(placed(table, fp, object));
    }
    if let Some((table, object)) = dispatch_ltm_tables(&module, &object_type, fp, body, range) {
        return Some(placed(table, fp, object));
    }

    // Family parsers with a sub-type argument + the ltm/gtm match block.
    if module == "apm" && object_type.starts_with("policy agent ") {
        return Some(placed(
            "apm_policy_agents",
            fp,
            ModelObject::ApmPolicyAgent(parse_bigip_apm_policy_agent(fp, body, range)),
        ));
    }
    if module == "gtm" && object_type.starts_with("pool ") {
        let record_type = object_type.strip_prefix("pool ").unwrap_or("");
        return Some(placed(
            "gtm_pools",
            fp,
            ModelObject::GtmPool(super::bespoke::parse_gtm_pool(fp, body, record_type, range)),
        ));
    }
    if module == "gtm" && object_type.starts_with("wideip ") {
        let record_type = object_type.strip_prefix("wideip ").unwrap_or("");
        return Some(placed(
            "gtm_wideips",
            fp,
            ModelObject::GtmWideip(super::bespoke::parse_gtm_wideip(
                fp,
                body,
                record_type,
                range,
            )),
        ));
    }
    if module == "pem"
        && matches!(
            object_type.as_str(),
            "profile diameter-endpoint"
                | "profile radius-aaa"
                | "profile spm"
                | "profile subscriber-mgmt"
        )
    {
        return Some(placed(
            "pem_profiles",
            fp,
            ModelObject::PemProfile(parse_bigip_pem_profile(fp, body, range)),
        ));
    }
    if module != "ltm" && module != "gtm" {
        return None;
    }

    let object = match object_type.as_str() {
        "data-group internal" | "data-group external" => {
            let kind = if object_type == "data-group external" {
                crate::model::DataGroupType::External
            } else {
                crate::model::DataGroupType::Internal
            };
            return Some(placed(
                "data_groups",
                fp,
                ModelObject::DataGroup(super::bespoke::parse_data_group(fp, body, kind, range)),
            ));
        }
        "pool" if module == "ltm" => {
            ModelObject::Pool(super::bespoke::parse_pool(fp, body, range, ctx))
        }
        "virtual" => {
            ModelObject::VirtualServer(super::bespoke::parse_virtual(fp, body, range, ctx))
        }
        "virtual-address" if module == "ltm" => {
            ModelObject::VirtualAddress(super::bespoke::parse_virtual_address(fp, body, range))
        }
        "node" => ModelObject::Node(super::bespoke::parse_node(fp, body, range)),
        "snatpool" => ModelObject::SnatPool(super::bespoke::parse_snatpool(fp, body, range)),
        "rule" => ModelObject::Rule(super::bespoke::parse_rule(fp, body, range)),
        "policy" if module == "ltm" => {
            ModelObject::Policy(super::bespoke::parse_policy(fp, body, range))
        }
        ot if ot.starts_with("dns cache records ") && module == "ltm" => {
            return Some(placed(
                "ltm_dns_cache_records",
                fp,
                ModelObject::LtmDnsCacheRecord(parse_bigip_ltm_dns_cache_record(fp, body, range)),
            ));
        }
        ot if ot.starts_with("profile ") => {
            let profile_type = ot.strip_prefix("profile ").unwrap_or("");
            return Some(placed(
                "profiles",
                fp,
                ModelObject::Profile(super::bespoke::parse_profile(fp, profile_type, body, range)),
            ));
        }
        ot if ot.starts_with("persistence ") => {
            let persistence_type = ot.strip_prefix("persistence ").unwrap_or("");
            return Some(placed(
                "persistence",
                fp,
                ModelObject::Persistence(super::bespoke::parse_persistence(
                    fp,
                    persistence_type,
                    body,
                    range,
                )),
            ));
        }
        ot if ot.starts_with("monitor ") => {
            let monitor_type = ot.strip_prefix("monitor ").unwrap_or("");
            let table = if module == "gtm" {
                "gtm_monitors"
            } else {
                "monitors"
            };
            return Some(placed(
                table,
                fp,
                ModelObject::Monitor(super::bespoke::parse_monitor(fp, body, monitor_type, range)),
            ));
        }
        _ => return None,
    };
    let table = match object_type.as_str() {
        "pool" => "pools",
        "virtual" => "virtual_servers",
        "virtual-address" => "virtual_addresses",
        "node" => "nodes",
        "snatpool" => "snat_pools",
        "rule" => "rules",
        "policy" => "policies",
        _ => return None,
    };
    Some(placed(table, fp, object))
}

/// Build a [`Placed`] from a table name + object.
fn placed(table: &'static str, full_path: &str, object: ModelObject) -> Placed {
    Placed {
        table_name: table,
        full_path: full_path.to_owned(),
        object,
    }
}

/// Intercept the `(module, object_type)` keys whose generated dispatch
/// routes through the scalar named/singleton tables but which carry
/// structured fields handled by [`super::bespoke`]. Returns `None` for any
/// other key so the generated dispatch proceeds unchanged.
fn bespoke_override(
    module: &str,
    object_type: &str,
    full_path: &str,
    body: &str,
    range: Range,
) -> Option<Placed> {
    match module {
        "sys" => bespoke_sys(object_type, full_path, body, range),
        "net" => bespoke_net(object_type, full_path, body, range),
        "gtm" => bespoke_gtm(object_type, full_path, body, range),
        "cm" => bespoke_cm(object_type, full_path, body, range),
        "security" => bespoke_security(object_type, full_path, body, range),
        "pem" => bespoke_pem(object_type, full_path, body, range),
        "apm" => bespoke_apm(object_type, full_path, body, range),
        "auth" => bespoke_auth(object_type, full_path, body, range),
        "ltm" => bespoke_ltm(object_type, full_path, body, range),
        _ => None,
    }
}

fn bespoke_sys(object_type: &str, full_path: &str, body: &str, range: Range) -> Option<Placed> {
    match object_type {
        "ntp" => Some(placed(
            "sys_ntp",
            full_path,
            ModelObject::SysNtp(super::bespoke::parse_sys_ntp(full_path, body, range)),
        )),
        "snmp" => Some(placed(
            "sys_snmp",
            full_path,
            ModelObject::SysSnmp(super::bespoke::parse_sys_snmp(full_path, body, range)),
        )),
        "file ssl-cert" => Some(placed(
            "sys_file_ssl_certs",
            full_path,
            ModelObject::SysFileSslCert(super::bespoke::parse_sys_file_ssl_cert(
                full_path, body, range,
            )),
        )),
        "provision" => Some(placed(
            "sys_provisions",
            full_path,
            ModelObject::SysProvision(super::bespoke::parse_sys_provision(full_path, body, range)),
        )),
        _ => None,
    }
}

fn bespoke_net(object_type: &str, full_path: &str, body: &str, range: Range) -> Option<Placed> {
    match object_type {
        "route" => Some(placed(
            "net_routes",
            full_path,
            ModelObject::NetRoute(super::bespoke::parse_net_route(full_path, body, range)),
        )),
        "self" => Some(placed(
            "net_selves",
            full_path,
            ModelObject::NetSelf(super::bespoke::parse_net_self(full_path, body, range)),
        )),
        "dns-resolver" => Some(placed(
            "net_dns_resolvers",
            full_path,
            ModelObject::NetDnsResolver(super::bespoke::parse_net_dns_resolver(
                full_path, body, range,
            )),
        )),
        "interface" => Some(placed(
            "net_interfaces",
            full_path,
            ModelObject::NetInterface(super::bespoke::parse_net_interface(full_path, body, range)),
        )),
        "port-list" => Some(placed(
            "net_port_lists",
            full_path,
            ModelObject::NetPortList(super::bespoke::parse_net_port_list(full_path, body, range)),
        )),
        "route-domain" => Some(placed(
            "net_route_domains",
            full_path,
            ModelObject::NetRouteDomain(super::bespoke::parse_net_route_domain(
                full_path, body, range,
            )),
        )),
        "stp" => Some(placed(
            "net_stps",
            full_path,
            ModelObject::NetStp(super::bespoke::parse_net_stp(full_path, body, range)),
        )),
        "vlan" => Some(placed(
            "net_vlans",
            full_path,
            ModelObject::NetVlan(super::bespoke::parse_net_vlan(full_path, body, range)),
        )),
        _ => None,
    }
}

fn bespoke_gtm(object_type: &str, full_path: &str, body: &str, range: Range) -> Option<Placed> {
    match object_type {
        "server" => Some(placed(
            "gtm_servers",
            full_path,
            ModelObject::GtmServer(super::bespoke::parse_gtm_server(full_path, body, range)),
        )),
        "datacenter" => Some(placed(
            "gtm_datacenters",
            full_path,
            ModelObject::GtmDatacenter(super::bespoke::parse_gtm_datacenter(
                full_path, body, range,
            )),
        )),
        "prober-pool" => Some(placed(
            "gtm_prober_pools",
            full_path,
            ModelObject::GtmProberPool(super::bespoke::parse_gtm_prober_pool(
                full_path, body, range,
            )),
        )),
        "rule" => Some(placed(
            "gtm_rules",
            full_path,
            ModelObject::GtmRule(super::bespoke::parse_gtm_rule(full_path, body, range)),
        )),
        _ => None,
    }
}

fn bespoke_cm(object_type: &str, full_path: &str, body: &str, range: Range) -> Option<Placed> {
    match object_type {
        "device" => Some(placed(
            "cm_devices",
            full_path,
            ModelObject::CmDevice(super::bespoke::parse_cm_device(full_path, body, range)),
        )),
        "device-group" => Some(placed(
            "cm_device_groups",
            full_path,
            ModelObject::CmDeviceGroup(super::bespoke::parse_cm_device_group(
                full_path, body, range,
            )),
        )),
        "traffic-group" => Some(placed(
            "cm_traffic_groups",
            full_path,
            ModelObject::CmTrafficGroup(super::bespoke::parse_cm_traffic_group(
                full_path, body, range,
            )),
        )),
        "trust-domain" => Some(placed(
            "cm_trust_domains",
            full_path,
            ModelObject::CmTrustDomain(super::bespoke::parse_cm_trust_domain(
                full_path, body, range,
            )),
        )),
        _ => None,
    }
}

fn bespoke_security(
    object_type: &str,
    full_path: &str,
    body: &str,
    range: Range,
) -> Option<Placed> {
    match object_type {
        "firewall rule-list" => Some(placed(
            "security_firewall_rule_lists",
            full_path,
            ModelObject::SecurityFirewallRuleList(
                super::bespoke::parse_security_firewall_rule_list(full_path, body, range),
            ),
        )),
        "firewall policy" => Some(placed(
            "security_firewall_policies",
            full_path,
            ModelObject::SecurityFirewallPolicy(super::bespoke::parse_security_firewall_policy(
                full_path, body, range,
            )),
        )),
        "firewall port-list" => Some(placed(
            "security_firewall_port_lists",
            full_path,
            ModelObject::SecurityFirewallPortList(
                super::bespoke::parse_security_firewall_port_list(full_path, body, range),
            ),
        )),
        "log profile" => Some(placed(
            "security_log_profiles",
            full_path,
            ModelObject::SecurityLogProfile(super::bespoke::parse_security_log_profile(
                full_path, body, range,
            )),
        )),
        "nat policy" => Some(placed(
            "security_nat_policies",
            full_path,
            ModelObject::SecurityNatPolicy(super::bespoke::parse_security_nat_policy(
                full_path, body, range,
            )),
        )),
        "packet-filter policy" => Some(placed(
            "security_packet_filter_policies",
            full_path,
            ModelObject::SecurityPacketFilterPolicy(
                super::bespoke::parse_security_packet_filter_policy(full_path, body, range),
            ),
        )),
        _ => None,
    }
}

fn bespoke_pem(object_type: &str, full_path: &str, body: &str, range: Range) -> Option<Placed> {
    match object_type {
        "listener" => Some(placed(
            "pem_listeners",
            full_path,
            ModelObject::PemListener(super::bespoke::parse_pem_listener(full_path, body, range)),
        )),
        "policy" => Some(placed(
            "pem_policies",
            full_path,
            ModelObject::PemPolicy(super::bespoke::parse_pem_policy(full_path, body, range)),
        )),
        "service-chain-endpoint" => Some(placed(
            "pem_service_chain_endpoints",
            full_path,
            ModelObject::PemServiceChainEndpoint(super::bespoke::parse_pem_service_chain_endpoint(
                full_path, body, range,
            )),
        )),
        _ => None,
    }
}

fn bespoke_apm(object_type: &str, full_path: &str, body: &str, range: Range) -> Option<Placed> {
    match object_type {
        "oauth db-instance" => Some(placed(
            "apm_oauth_db_instances",
            full_path,
            ModelObject::ApmOauthDbInstance(super::bespoke::parse_apm_oauth_db_instance(
                full_path, body, range,
            )),
        )),
        "policy policy-item" => Some(placed(
            "apm_policy_items",
            full_path,
            ModelObject::ApmPolicyItem(super::bespoke::parse_apm_policy_item(
                full_path, body, range,
            )),
        )),
        _ => None,
    }
}

fn bespoke_auth(object_type: &str, full_path: &str, body: &str, range: Range) -> Option<Placed> {
    match object_type {
        "partition" => Some(placed(
            "auth_partitions",
            full_path,
            ModelObject::AuthPartition(super::bespoke::parse_auth_partition(
                full_path, body, range,
            )),
        )),
        _ => None,
    }
}

fn bespoke_ltm(object_type: &str, full_path: &str, body: &str, range: Range) -> Option<Placed> {
    match object_type {
        "dns cache resolver" => Some(placed(
            "ltm_dns_cache_resolvers",
            full_path,
            ModelObject::LtmDnsCacheResolver(super::bespoke::parse_ltm_dns_cache_resolver(
                full_path, body, range,
            )),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_objects_on_corpus() {
        let src = include_str!("../../../../samples/bigip/bigip.conf");
        let config = parse_bigip_conf(src, "Common");
        assert_eq!(config.default_partition, "Common");
        assert_eq!(config.generic_objects.len(), 28);

        let (key, obj) = &config.generic_objects[0];
        assert_eq!(key, "ltm::data-group internal::/Common/allowed_hosts");
        assert_eq!(obj.module, "ltm");
        assert_eq!(obj.object_type, "data-group internal");
        assert_eq!(obj.identifier, "/Common/allowed_hosts");
        assert_eq!(obj.header, "ltm data-group internal /Common/allowed_hosts");
        // Range captured from the reference parser.
        let r = obj.range.unwrap();
        assert_eq!(
            (r.start.line, r.start.character, r.start.offset),
            (3, 46, 162)
        );
        assert_eq!((r.end.line, r.end.character, r.end.offset), (10, 1, 287));
    }

    #[test]
    fn unpartitioned_kinds_are_not_partition_prefixed() {
        // Hardware / system / cluster kinds keep their bare identifier — no
        // bogus `/Common/` prefix that would break lookups (issue 189).
        let src = "net interface 1.1 { }\n\
                   sys provision ltm { level nominal }\n\
                   cm device bigip1.local { }\n\
                   ltm pool p1 { }\n";
        let config = parse_bigip_conf(src, "Common");
        let id_for = |needle: &str| -> String {
            let (_, obj) = config
                .generic_objects
                .iter()
                .find(|(k, _)| k.contains(needle))
                .unwrap_or_else(|| {
                    panic!("no object matching {needle}: {:?}", config.generic_objects)
                });
            obj.identifier.clone()
        };
        assert_eq!(id_for("net::interface"), "1.1");
        assert_eq!(id_for("sys::provision"), "ltm");
        assert_eq!(id_for("cm::device"), "bigip1.local");
        // A genuinely partitioned kind still gets the prefix.
        assert_eq!(id_for("ltm::pool"), "/Common/p1");
    }

    #[test]
    fn typed_object_inventory_on_corpus() {
        let src = include_str!("../../../../samples/bigip/bigip.conf");
        let config = parse_bigip_conf(src, "Common");
        // Per-table counts captured from the reference parser.
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for p in &config.objects {
            *counts.entry(p.table_name).or_default() += 1;
        }
        let expected = [
            ("data_groups", 4),
            ("pools", 3),
            ("virtual_servers", 4),
            ("nodes", 3),
            ("profiles", 5),
            ("monitors", 1),
            ("snat_pools", 1),
            ("persistence", 2),
            ("rules", 5),
        ];
        for (table, n) in expected {
            assert_eq!(counts.get(table).copied().unwrap_or(0), n, "table {table}");
        }
        assert_eq!(config.objects.len(), 28, "total typed objects");
    }
}
