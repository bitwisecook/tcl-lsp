//! Top-level driver — `parse_bigip_conf`, a faithful port of
//! `dialects/f5/bigip/parser/_driver.py`.
//!
//! The container mirrors the Python `BigipConfig`: a `default_partition`,
//! the `generic_objects` index (every stanza lands one row), and the
//! typed objects (each carrying the `BigipConfig` field name it belongs
//! to so the `PyO3` layer can place it). Typed dispatch is layered on in
//! subsequent stages; this stage establishes the block iteration,
//! header resolution, partition-prefixing, and `generic_objects` exactly.

use tcl_lexer::LineIndex;
use tcl_registry::bigip::BigipRegistry;

use crate::model::{BigipGenericObject, ModelObject};
use crate::range::Range;

use super::header::{parse_generic_header, ObjectTypeIndex};
use super::helpers::extract_blocks;

/// One typed object placed in a `BigipConfig` collection. `table_name`
/// is the Python `BigipConfig` attribute it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// The `BigipConfig` attribute name (e.g. `"pools"`).
    pub table_name: &'static str,
    /// Object full path (dict key; `""` for singletons).
    pub full_path: String,
    /// The parsed object.
    pub object: ModelObject,
}

/// Parsed BIG-IP configuration. Mirrors the Python `BigipConfig`
/// contract (a `default_partition`, the `generic_objects` index, and the
/// per-kind typed objects).
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

/// `(module, object_type)` pairs whose identifier is itself a partition
/// / cluster-wide reference and must not be partition-prefixed. Mirrors
/// the Python `_no_partition_prefix`.
const NO_PARTITION_PREFIX: &[(&str, &str)] = &[("auth", "partition")];

/// Parse a BIG-IP configuration file into a [`BigipConfig`]. Mirrors
/// `parse_bigip_conf`.
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
        // full path; the Python driver builds the typed object here before
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
                range: Some(range),
            },
        ));

        // Typed dispatch — mirrors the strict `_parse_header` path.
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

/// Route one stanza to its typed object, mirroring the strict-header
/// dispatch in `_driver.py`. `generic_*` come from the generic header
/// (already partition-prefixed) for the bare-singleton fallback.
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
    use crate::model::gen::dispatch::{
        dispatch_ltm_tables, dispatch_minimal, dispatch_named, dispatch_singleton,
        parse_header_strict,
    };
    use crate::model::gen::parsers::*;
    use crate::model::ModelObject;

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
            return placed(table, "", object);
        }
        if let Some((table, object)) =
            dispatch_minimal(generic_module, generic_type, "", body, range)
        {
            return placed(table, "", object);
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
        return placed(table, fp, object);
    }
    if let Some((table, object)) = dispatch_named(&module, &object_type, fp, body, range) {
        return placed(table, fp, object);
    }
    if full_path.is_empty() {
        if let Some((table, object)) = dispatch_singleton(&module, &object_type, fp, body, range) {
            return placed(table, fp, object);
        }
    }
    if let Some((table, object)) = dispatch_ltm_tables(&module, &object_type, fp, body, range) {
        return placed(table, fp, object);
    }

    // Family parsers with a sub-type argument + the ltm/gtm match block.
    if module == "apm" && object_type.starts_with("policy agent ") {
        return placed(
            "apm_policy_agents",
            fp,
            ModelObject::ApmPolicyAgent(parse_bigip_apm_policy_agent(fp, body, range)),
        );
    }
    if module == "gtm" && object_type.starts_with("pool ") {
        let record_type = object_type.strip_prefix("pool ").unwrap_or("");
        return placed(
            "gtm_pools",
            fp,
            ModelObject::GtmPool(super::bespoke::parse_gtm_pool(fp, body, record_type, range)),
        );
    }
    if module == "gtm" && object_type.starts_with("wideip ") {
        let record_type = object_type.strip_prefix("wideip ").unwrap_or("");
        return placed(
            "gtm_wideips",
            fp,
            ModelObject::GtmWideip(super::bespoke::parse_gtm_wideip(
                fp,
                body,
                record_type,
                range,
            )),
        );
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
        return placed(
            "pem_profiles",
            fp,
            ModelObject::PemProfile(parse_bigip_pem_profile(fp, body, range)),
        );
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
            return placed(
                "data_groups",
                fp,
                ModelObject::DataGroup(super::bespoke::parse_data_group(fp, body, kind, range)),
            );
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
            return placed(
                "ltm_dns_cache_records",
                fp,
                ModelObject::LtmDnsCacheRecord(parse_bigip_ltm_dns_cache_record(fp, body, range)),
            )
        }
        ot if ot.starts_with("profile ") => {
            let profile_type = ot.strip_prefix("profile ").unwrap_or("");
            return placed(
                "profiles",
                fp,
                ModelObject::Profile(super::bespoke::parse_profile(fp, profile_type, body, range)),
            );
        }
        ot if ot.starts_with("persistence ") => {
            let persistence_type = ot.strip_prefix("persistence ").unwrap_or("");
            return placed(
                "persistence",
                fp,
                ModelObject::Persistence(super::bespoke::parse_persistence(
                    fp,
                    persistence_type,
                    body,
                    range,
                )),
            );
        }
        ot if ot.starts_with("monitor ") => {
            let monitor_type = ot.strip_prefix("monitor ").unwrap_or("");
            let table = if module == "gtm" {
                "gtm_monitors"
            } else {
                "monitors"
            };
            return placed(
                table,
                fp,
                ModelObject::Monitor(super::bespoke::parse_monitor(fp, body, monitor_type, range)),
            );
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
    placed(table, fp, object)
}

/// Build a [`Placed`] from a table name + object.
#[allow(clippy::unnecessary_wraps)]
fn placed(table: &'static str, full_path: &str, object: ModelObject) -> Option<Placed> {
    Some(Placed {
        table_name: table,
        full_path: full_path.to_owned(),
        object,
    })
}

/// Intercept the `(module, object_type)` keys whose generated dispatch
/// routes through the scalar named/singleton tables but which carry
/// structured fields handled by [`super::bespoke`]. Returns `None` for any
/// other key so the generated dispatch proceeds unchanged.
#[allow(clippy::too_many_lines)]
fn bespoke_override(
    module: &str,
    object_type: &str,
    full_path: &str,
    body: &str,
    range: Range,
) -> Option<Placed> {
    match (module, object_type) {
        ("sys", "ntp") => placed(
            "sys_ntp",
            full_path,
            ModelObject::SysNtp(super::bespoke::parse_sys_ntp(full_path, body, range)),
        ),
        ("sys", "snmp") => placed(
            "sys_snmp",
            full_path,
            ModelObject::SysSnmp(super::bespoke::parse_sys_snmp(full_path, body, range)),
        ),
        ("net", "route") => placed(
            "net_routes",
            full_path,
            ModelObject::NetRoute(super::bespoke::parse_net_route(full_path, body, range)),
        ),
        ("net", "self") => placed(
            "net_selves",
            full_path,
            ModelObject::NetSelf(super::bespoke::parse_net_self(full_path, body, range)),
        ),
        ("security", "firewall rule-list") => placed(
            "security_firewall_rule_lists",
            full_path,
            ModelObject::SecurityFirewallRuleList(
                super::bespoke::parse_security_firewall_rule_list(full_path, body, range),
            ),
        ),
        ("security", "firewall policy") => placed(
            "security_firewall_policies",
            full_path,
            ModelObject::SecurityFirewallPolicy(super::bespoke::parse_security_firewall_policy(
                full_path, body, range,
            )),
        ),
        ("gtm", "server") => placed(
            "gtm_servers",
            full_path,
            ModelObject::GtmServer(super::bespoke::parse_gtm_server(full_path, body, range)),
        ),
        ("apm", "oauth db-instance") => placed(
            "apm_oauth_db_instances",
            full_path,
            ModelObject::ApmOauthDbInstance(super::bespoke::parse_apm_oauth_db_instance(
                full_path, body, range,
            )),
        ),
        ("apm", "policy policy-item") => placed(
            "apm_policy_items",
            full_path,
            ModelObject::ApmPolicyItem(super::bespoke::parse_apm_policy_item(
                full_path, body, range,
            )),
        ),
        ("auth", "partition") => placed(
            "auth_partitions",
            full_path,
            ModelObject::AuthPartition(super::bespoke::parse_auth_partition(
                full_path, body, range,
            )),
        ),
        ("cm", "device") => placed(
            "cm_devices",
            full_path,
            ModelObject::CmDevice(super::bespoke::parse_cm_device(full_path, body, range)),
        ),
        ("cm", "device-group") => placed(
            "cm_device_groups",
            full_path,
            ModelObject::CmDeviceGroup(super::bespoke::parse_cm_device_group(
                full_path, body, range,
            )),
        ),
        ("cm", "traffic-group") => placed(
            "cm_traffic_groups",
            full_path,
            ModelObject::CmTrafficGroup(super::bespoke::parse_cm_traffic_group(
                full_path, body, range,
            )),
        ),
        ("cm", "trust-domain") => placed(
            "cm_trust_domains",
            full_path,
            ModelObject::CmTrustDomain(super::bespoke::parse_cm_trust_domain(
                full_path, body, range,
            )),
        ),
        ("gtm", "datacenter") => placed(
            "gtm_datacenters",
            full_path,
            ModelObject::GtmDatacenter(super::bespoke::parse_gtm_datacenter(
                full_path, body, range,
            )),
        ),
        ("gtm", "prober-pool") => placed(
            "gtm_prober_pools",
            full_path,
            ModelObject::GtmProberPool(super::bespoke::parse_gtm_prober_pool(
                full_path, body, range,
            )),
        ),
        ("gtm", "rule") => placed(
            "gtm_rules",
            full_path,
            ModelObject::GtmRule(super::bespoke::parse_gtm_rule(full_path, body, range)),
        ),
        ("ltm", "dns cache resolver") => placed(
            "ltm_dns_cache_resolvers",
            full_path,
            ModelObject::LtmDnsCacheResolver(super::bespoke::parse_ltm_dns_cache_resolver(
                full_path, body, range,
            )),
        ),
        ("net", "dns-resolver") => placed(
            "net_dns_resolvers",
            full_path,
            ModelObject::NetDnsResolver(super::bespoke::parse_net_dns_resolver(
                full_path, body, range,
            )),
        ),
        ("net", "interface") => placed(
            "net_interfaces",
            full_path,
            ModelObject::NetInterface(super::bespoke::parse_net_interface(full_path, body, range)),
        ),
        ("net", "port-list") => placed(
            "net_port_lists",
            full_path,
            ModelObject::NetPortList(super::bespoke::parse_net_port_list(full_path, body, range)),
        ),
        ("net", "route-domain") => placed(
            "net_route_domains",
            full_path,
            ModelObject::NetRouteDomain(super::bespoke::parse_net_route_domain(
                full_path, body, range,
            )),
        ),
        ("net", "stp") => placed(
            "net_stps",
            full_path,
            ModelObject::NetStp(super::bespoke::parse_net_stp(full_path, body, range)),
        ),
        ("net", "vlan") => placed(
            "net_vlans",
            full_path,
            ModelObject::NetVlan(super::bespoke::parse_net_vlan(full_path, body, range)),
        ),
        ("pem", "listener") => placed(
            "pem_listeners",
            full_path,
            ModelObject::PemListener(super::bespoke::parse_pem_listener(full_path, body, range)),
        ),
        ("pem", "policy") => placed(
            "pem_policies",
            full_path,
            ModelObject::PemPolicy(super::bespoke::parse_pem_policy(full_path, body, range)),
        ),
        ("pem", "service-chain-endpoint") => placed(
            "pem_service_chain_endpoints",
            full_path,
            ModelObject::PemServiceChainEndpoint(super::bespoke::parse_pem_service_chain_endpoint(
                full_path, body, range,
            )),
        ),
        ("security", "firewall port-list") => placed(
            "security_firewall_port_lists",
            full_path,
            ModelObject::SecurityFirewallPortList(
                super::bespoke::parse_security_firewall_port_list(full_path, body, range),
            ),
        ),
        ("security", "log profile") => placed(
            "security_log_profiles",
            full_path,
            ModelObject::SecurityLogProfile(super::bespoke::parse_security_log_profile(
                full_path, body, range,
            )),
        ),
        ("security", "nat policy") => placed(
            "security_nat_policies",
            full_path,
            ModelObject::SecurityNatPolicy(super::bespoke::parse_security_nat_policy(
                full_path, body, range,
            )),
        ),
        ("security", "packet-filter policy") => placed(
            "security_packet_filter_policies",
            full_path,
            ModelObject::SecurityPacketFilterPolicy(
                super::bespoke::parse_security_packet_filter_policy(full_path, body, range),
            ),
        ),
        ("sys", "file ssl-cert") => placed(
            "sys_file_ssl_certs",
            full_path,
            ModelObject::SysFileSslCert(super::bespoke::parse_sys_file_ssl_cert(
                full_path, body, range,
            )),
        ),
        ("sys", "provision") => placed(
            "sys_provisions",
            full_path,
            ModelObject::SysProvision(super::bespoke::parse_sys_provision(full_path, body, range)),
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_objects_match_python_on_corpus() {
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
        // Range captured from the live Python parser.
        let r = obj.range.unwrap();
        assert_eq!(
            (r.start.line, r.start.character, r.start.offset),
            (3, 46, 162)
        );
        assert_eq!((r.end.line, r.end.character, r.end.offset), (10, 1, 287));
    }

    #[test]
    fn typed_object_inventory_matches_python_on_corpus() {
        let src = include_str!("../../../../samples/bigip/bigip.conf");
        let config = parse_bigip_conf(src, "Common");
        // Per-table counts captured from the live Python parser.
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
