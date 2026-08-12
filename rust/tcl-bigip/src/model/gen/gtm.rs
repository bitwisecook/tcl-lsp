// @generated — do not edit.
//! Generated BIG-IP `gtm` model structs.

// Generated BIG-IP config records are flat data structs of
// independent, orthogonal boolean attributes (mirroring the tmsh
// object schema) — not state machines, so struct_excessive_bools is
// a false positive here and is allowed deliberately.
#![allow(clippy::struct_excessive_bools)]
#![allow(unused_imports)]

use super::*;

/// `BigipGtmDatacenter`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmDatacenter {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `contact`
    pub contact: String,
    /// `location`
    pub location: String,
    /// `description`
    pub description: String,
    /// `prober_pool`
    pub prober_pool: String,
    /// `prober_preference`
    pub prober_preference: String,
    /// `prober_fallback`
    pub prober_fallback: String,
    /// `state`
    pub state: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmDistributedApp`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmDistributedApp {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `wide_ips`
    pub wide_ips: Vec<String>,
    /// `persist_cidr`
    pub persist_cidr: String,
    /// `dependency_level`
    pub dependency_level: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmGlobalSettingsGeneral`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmGlobalSettingsGeneral {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `auto_discovery`
    pub auto_discovery: String,
    /// `synchronization`
    pub synchronization: String,
    /// `synchronization_group_name`
    pub synchronization_group_name: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmGlobalSettingsLoadBalancing`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmGlobalSettingsLoadBalancing {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `topology_longest_match`
    pub topology_longest_match: String,
    /// `ignore_path_ttl`
    pub ignore_path_ttl: String,
    /// `respect_dependent_objects`
    pub respect_dependent_objects: String,
    /// `verify_vs_availability`
    pub verify_vs_availability: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmGlobalSettingsMetrics`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmGlobalSettingsMetrics {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `metrics_collection_protocols`
    pub metrics_collection_protocols: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmGlobalSettingsMetricsExclusions`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmGlobalSettingsMetricsExclusions {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `addresses`
    pub addresses: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmLink`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmLink {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `datacenter`
    pub datacenter: String,
    /// `monitor`
    pub monitor: String,
    /// `prober_pool`
    pub prober_pool: String,
    /// `state`
    pub state: String,
    /// `weight`
    pub weight: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmListener`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmListener {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `address`
    pub address: String,
    /// `port`
    pub port: String,
    /// `ip_protocol`
    pub ip_protocol: String,
    /// `mask`
    pub mask: String,
    /// `pool`
    pub pool: String,
    /// `profiles`
    pub profiles: Vec<String>,
    /// `rules`
    pub rules: Vec<String>,
    /// `source_address_translation`
    pub source_address_translation: String,
    /// `state`
    pub state: String,
    /// `vlans`
    pub vlans: Vec<String>,
    /// `vlans_disabled`
    pub vlans_disabled: bool,
    /// `vlans_enabled`
    pub vlans_enabled: bool,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmListenerDohProxy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmListenerDohProxy {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `address`
    pub address: String,
    /// `port`
    pub port: String,
    /// `pool`
    pub pool: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmListenerDohServer`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmListenerDohServer {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `address`
    pub address: String,
    /// `port`
    pub port: String,
    /// `pool`
    pub pool: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmPool`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmPool {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `record_type`
    pub record_type: String,
    /// `members`
    pub members: Vec<BigipGtmPoolMember>,
    /// `monitor`
    pub monitor: String,
    /// `alternate_mode`
    pub alternate_mode: String,
    /// `fallback_mode`
    pub fallback_mode: String,
    /// `load_balancing_mode`
    pub load_balancing_mode: String,
    /// `ttl`
    pub ttl: String,
    /// `description`
    pub description: String,
    /// `state`
    pub state: String,
    /// `verify_member_availability`
    pub verify_member_availability: String,
    /// `fallback_ip`
    pub fallback_ip: String,
    /// `max_answers_returned`
    pub max_answers_returned: String,
    /// `qos_hit_ratio`
    pub qos_hit_ratio: String,
    /// `qos_hops`
    pub qos_hops: String,
    /// `qos_kbps`
    pub qos_kbps: String,
    /// `qos_lcs`
    pub qos_lcs: String,
    /// `qos_packet_rate`
    pub qos_packet_rate: String,
    /// `qos_rtt`
    pub qos_rtt: String,
    /// `qos_topology`
    pub qos_topology: String,
    /// `qos_vs_capacity`
    pub qos_vs_capacity: String,
    /// `qos_vs_score`
    pub qos_vs_score: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmPoolMember`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmPoolMember {
    /// `name`
    pub name: String,
    /// `description`
    pub description: String,
    /// `state`
    pub state: String,
    /// `member_order`
    pub member_order: String,
    /// `order`
    pub order: String,
    /// `service_port`
    pub service_port: String,
    /// `ratio`
    pub ratio: String,
    /// `monitor`
    pub monitor: String,
    /// `depends_on`
    pub depends_on: String,
    /// `limit_max_bps`
    pub limit_max_bps: String,
    /// `limit_max_connections`
    pub limit_max_connections: String,
    /// `limit_max_pps`
    pub limit_max_pps: String,
    /// `static_target`
    pub static_target: String,
}

/// `BigipGtmProberPool`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmProberPool {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `load_balancing_mode`
    pub load_balancing_mode: String,
    /// `members`
    pub members: Vec<String>,
    /// `state`
    pub state: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmRegion`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmRegion {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `region_members`
    pub region_members: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmRule`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmRule {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `source`
    pub source: String,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmServer`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmServer {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `datacenter`
    pub datacenter: String,
    /// `monitor`
    pub monitor: String,
    /// `product`
    pub product: String,
    /// `addresses`
    pub addresses: Vec<String>,
    /// `virtual_servers`
    pub virtual_servers: Vec<String>,
    /// `description`
    pub description: String,
    /// `state`
    pub state: String,
    /// `prober_pool`
    pub prober_pool: String,
    /// `prober_preference`
    pub prober_preference: String,
    /// `prober_fallback`
    pub prober_fallback: String,
    /// `virtual_server_discovery`
    pub virtual_server_discovery: String,
    /// `expose_route_domains`
    pub expose_route_domains: String,
    /// `iq_allow_path`
    pub iq_allow_path: String,
    /// `iq_allow_service_check`
    pub iq_allow_service_check: String,
    /// `iq_allow_snmp`
    pub iq_allow_snmp: String,
    /// `limit_max_bps`
    pub limit_max_bps: String,
    /// `limit_max_connections`
    pub limit_max_connections: String,
    /// `limit_max_pps`
    pub limit_max_pps: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmTopology`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmTopology {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `order`
    pub order: String,
    /// `score`
    pub score: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipGtmWideip`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipGtmWideip {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `record_type`
    pub record_type: String,
    /// `pools`
    pub pools: Vec<String>,
    /// `aliases`
    pub aliases: Vec<String>,
    /// `pool_lb_mode`
    pub pool_lb_mode: String,
    /// `last_resort_pool`
    pub last_resort_pool: String,
    /// `description`
    pub description: String,
    /// `state`
    pub state: String,
    /// `failure_rcode`
    pub failure_rcode: String,
    /// `failure_rcode_response`
    pub failure_rcode_response: String,
    /// `failure_rcode_ttl`
    pub failure_rcode_ttl: String,
    /// `minimal_response`
    pub minimal_response: String,
    /// `persistence`
    pub persistence: String,
    /// `persist_cidr_ipv4`
    pub persist_cidr_ipv4: String,
    /// `persist_cidr_ipv6`
    pub persist_cidr_ipv6: String,
    /// `topology_prefer_edns0_client_subnet`
    pub topology_prefer_edns0_client_subnet: String,
    /// `ttl_persistence`
    pub ttl_persistence: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}
