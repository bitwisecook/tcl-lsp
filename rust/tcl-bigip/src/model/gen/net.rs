// @generated — do not edit.
//! Generated BIG-IP `net` model structs.

// Generated BIG-IP config records are flat data structs of
// independent, orthogonal boolean attributes (mirroring the tmsh
// object schema) — not state machines, so struct_excessive_bools is
// a false positive here and is allowed deliberately.
#![allow(clippy::struct_excessive_bools)]
#![allow(unused_imports)]

use super::*;

/// `BigipNetDnsResolver`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipNetDnsResolver {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `route_domain`
    pub route_domain: String,
    /// `forward_zones`
    pub forward_zones: Vec<String>,
    /// `description`
    pub description: String,
    /// `cache_size`
    pub cache_size: String,
    /// `randomize_query_name_case`
    pub randomize_query_name_case: String,
    /// `use_ipv4`
    pub use_ipv4: String,
    /// `use_ipv6`
    pub use_ipv6: String,
    /// `use_tcp`
    pub use_tcp: String,
    /// `use_udp`
    pub use_udp: String,
    /// `nameservers`
    pub nameservers: Vec<String>,
    /// `answer_default_zones`
    pub answer_default_zones: String,
    /// `prefetch`
    pub prefetch: String,
    /// `nameserver_min_rtt`
    pub nameserver_min_rtt: String,
    /// `nameserver_ttl`
    pub nameserver_ttl: String,
    /// `outbound_msg_retry`
    pub outbound_msg_retry: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipNetInterface`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipNetInterface {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `media_fixed`
    pub media_fixed: String,
    /// `description`
    pub description: String,
    /// `enabled`
    pub enabled: bool,
    /// `disabled`
    pub disabled: bool,
    /// `bundle`
    pub bundle: String,
    /// `bundle_speed`
    pub bundle_speed: String,
    /// `lldp_admin`
    pub lldp_admin: String,
    /// `mtu`
    pub mtu: String,
    /// `flow_control`
    pub flow_control: String,
    /// `mac_address`
    pub mac_address: String,
    /// `media_active`
    pub media_active: String,
    /// `media_max`
    pub media_max: String,
    /// `media_sfp`
    pub media_sfp: String,
    /// `port_fwd_mode`
    pub port_fwd_mode: String,
    /// `qinq_ethertype`
    pub qinq_ethertype: String,
    /// `stp`
    pub stp: String,
    /// `stp_edge_port`
    pub stp_edge_port: String,
    /// `stp_link_type`
    pub stp_link_type: String,
    /// `stp_auto_edge_port`
    pub stp_auto_edge_port: String,
    /// `stp_reset`
    pub stp_reset: String,
    /// `sflow_poll_interval`
    pub sflow_poll_interval: String,
    /// `sflow_poll_interval_global`
    pub sflow_poll_interval_global: String,
    /// `vendor`
    pub vendor: String,
    /// `vendor_oui`
    pub vendor_oui: String,
    /// `vendor_partnum`
    pub vendor_partnum: String,
    /// `vendor_revision`
    pub vendor_revision: String,
    /// `virtual_wire`
    pub virtual_wire: String,
    /// `transmitter_technology`
    pub transmitter_technology: String,
    /// `lacp_port_priority`
    pub lacp_port_priority: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipNetPortList`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipNetPortList {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `ports`
    pub ports: Vec<String>,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipNetRoute`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipNetRoute {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `network`
    pub network: Option<crate::value::Network>,
    /// `is_default_route`
    pub is_default_route: bool,
    /// `gw`
    pub gw: Option<crate::value::IPAddress>,
    /// `pool`
    pub pool: String,
    /// `description`
    pub description: String,
    /// `mtu`
    pub mtu: String,
    /// `blackhole`
    pub blackhole: bool,
    /// `interface`
    pub interface: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipNetRouteDomain`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipNetRouteDomain {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `id`
    pub id: i64,
    /// `vlans`
    pub vlans: Vec<String>,
    /// `description`
    pub description: String,
    /// `parent`
    pub parent: String,
    /// `strict`
    pub strict: String,
    /// `fw_enforced_policy`
    pub fw_enforced_policy: String,
    /// `fw_staged_policy`
    pub fw_staged_policy: String,
    /// `bwc_policy`
    pub bwc_policy: String,
    /// `connection_limit`
    pub connection_limit: String,
    /// `flow_eviction_policy`
    pub flow_eviction_policy: String,
    /// `routing_protocol`
    pub routing_protocol: Vec<String>,
    /// `security_nat_policy`
    pub security_nat_policy: String,
    /// `service_policy`
    pub service_policy: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipNetSelf`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipNetSelf {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `address`
    pub address: Option<crate::value::Network>,
    /// `vlan`
    pub vlan: String,
    /// `traffic_group`
    pub traffic_group: String,
    /// `allow_service`
    pub allow_service: Vec<String>,
    /// `description`
    pub description: String,
    /// `floating`
    pub floating: String,
    /// `unit`
    pub unit: String,
    /// `service_policy`
    pub service_policy: String,
    /// `fw_enforced_policy`
    pub fw_enforced_policy: String,
    /// `fw_staged_policy`
    pub fw_staged_policy: String,
    /// `inherited_traffic_group`
    pub inherited_traffic_group: String,
    /// `address_source`
    pub address_source: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipNetStp`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipNetStp {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `interfaces`
    pub interfaces: Vec<String>,
    /// `description`
    pub description: String,
    /// `mode`
    pub mode: String,
    /// `priority`
    pub priority: String,
    /// `external_path_cost`
    pub external_path_cost: String,
    /// `internal_path_cost`
    pub internal_path_cost: String,
    /// `vlans`
    pub vlans: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipNetTunnel`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipNetTunnel {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `profile`
    pub profile: String,
    /// `local_address`
    pub local_address: String,
    /// `remote_address`
    pub remote_address: String,
    /// `description`
    pub description: String,
    /// `mtu`
    pub mtu: String,
    /// `mode`
    pub mode: String,
    /// `idle_timeout`
    pub idle_timeout: String,
    /// `auto_lasthop`
    pub auto_lasthop: String,
    /// `secondary_address`
    pub secondary_address: String,
    /// `traffic_group`
    pub traffic_group: String,
    /// `transparent`
    pub transparent: String,
    /// `key`
    pub key: String,
    /// `use_pmtu`
    pub use_pmtu: String,
    /// `tos`
    pub tos: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipNetVlan`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipNetVlan {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `tag`
    pub tag: i64,
    /// `interfaces`
    pub interfaces: Vec<String>,
    /// `description`
    pub description: String,
    /// `mtu`
    pub mtu: String,
    /// `cmp_hash`
    pub cmp_hash: String,
    /// `failsafe`
    pub failsafe: String,
    /// `failsafe_action`
    pub failsafe_action: String,
    /// `failsafe_timeout`
    pub failsafe_timeout: String,
    /// `fwd_mode`
    pub fwd_mode: String,
    /// `hardware_syncookie`
    pub hardware_syncookie: String,
    /// `learning`
    pub learning: String,
    /// `tag_mode`
    pub tag_mode: String,
    /// `virtual_wire`
    pub virtual_wire: String,
    /// `auto_lasthop`
    pub auto_lasthop: String,
    /// `source_check`
    pub source_check: String,
    /// `source_checking`
    pub source_checking: String,
    /// `syn_flood_rate_limit`
    pub syn_flood_rate_limit: String,
    /// `syncache_threshold`
    pub syncache_threshold: String,
    /// `service_policy`
    pub service_policy: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}
