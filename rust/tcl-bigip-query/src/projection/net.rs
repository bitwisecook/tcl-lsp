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

//! Field projection of the L2/L3 underlay: routes, VLANs, self IPs, route domains, interfaces and tunnels.

use super::ltm::str_list;
use super::{Fields, path_ref, path_ref_list_strs, typed_str};
use crate::value::Value;
use indexmap::IndexMap;
use tcl_bigip::model::{
    BigipNetDnsResolver, BigipNetInterface, BigipNetPortList, BigipNetRoute, BigipNetRouteDomain,
    BigipNetSelf, BigipNetStp, BigipNetTunnel, BigipNetVlan,
};

// Net (L2/L3 underlay) projections

pub(super) fn project_net_route(o: &BigipNetRoute) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("network", typed_str(o.network.as_ref()))
        .b("is-default-route", o.is_default_route)
        .v("gw", typed_str(o.gw.as_ref()))
        .v("pool", path_ref(&o.pool, "ltm pool"))
        // A route's `interface` carries a VLAN or tunnel path, not a physical
        // interface name — the same target set as `ltm virtual`'s
        // `transparent-nexthop`, which resolves against `net vlan` too.
        .v("interface", path_ref(&o.interface, "net vlan"))
        .b("blackhole", o.blackhole)
        .s("mtu", &o.mtu)
        .s("description", &o.description)
        .done()
}

pub(super) fn project_net_vlan(o: &BigipNetVlan) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .i("tag", o.tag)
        .v(
            "interfaces",
            path_ref_list_strs(&o.interfaces, "net interface"),
        )
        .s("description", &o.description)
        .s("mtu", &o.mtu)
        .s("cmp-hash", &o.cmp_hash)
        .s("failsafe", &o.failsafe)
        .s("failsafe-action", &o.failsafe_action)
        .s("failsafe-timeout", &o.failsafe_timeout)
        .s("fwd-mode", &o.fwd_mode)
        .s("hardware-syncookie", &o.hardware_syncookie)
        .s("learning", &o.learning)
        .s("tag-mode", &o.tag_mode)
        .s("virtual-wire", &o.virtual_wire)
        .s("auto-lasthop", &o.auto_lasthop)
        .s("source-check", &o.source_check)
        .s("source-checking", &o.source_checking)
        .s("syn-flood-rate-limit", &o.syn_flood_rate_limit)
        .s("syncache-threshold", &o.syncache_threshold)
        .s("service-policy", &o.service_policy)
        .done()
}

pub(super) fn project_net_self(o: &BigipNetSelf) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("address", typed_str(o.address.as_ref()))
        .v("vlan", path_ref(&o.vlan, "net vlan"))
        .v(
            "traffic-group",
            path_ref(&o.traffic_group, "cm traffic-group"),
        )
        .v("allow-service", str_list(&o.allow_service))
        .s("description", &o.description)
        .s("floating", &o.floating)
        .s("unit", &o.unit)
        .s("service-policy", &o.service_policy)
        .s("fw-enforced-policy", &o.fw_enforced_policy)
        .s("fw-staged-policy", &o.fw_staged_policy)
        .s("inherited-traffic-group", &o.inherited_traffic_group)
        .s("address-source", &o.address_source)
        .done()
}

pub(super) fn project_net_route_domain(o: &BigipNetRouteDomain) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .i("id", o.id)
        .v("vlans", path_ref_list_strs(&o.vlans, "net vlan"))
        .s("description", &o.description)
        .v("parent", path_ref(&o.parent, "net route-domain"))
        .s("strict", &o.strict)
        .s("fw-enforced-policy", &o.fw_enforced_policy)
        .s("fw-staged-policy", &o.fw_staged_policy)
        .s("bwc-policy", &o.bwc_policy)
        .s("connection-limit", &o.connection_limit)
        .s("flow-eviction-policy", &o.flow_eviction_policy)
        .v("routing-protocol", str_list(&o.routing_protocol))
        .s("security-nat-policy", &o.security_nat_policy)
        .s("service-policy", &o.service_policy)
        .done()
}

pub(super) fn project_net_port_list(o: &BigipNetPortList) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("ports", str_list(&o.ports))
        .s("description", &o.description)
        .done()
}

pub(super) fn project_net_interface(o: &BigipNetInterface) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("mac-address", &o.mac_address)
        .b("enabled", o.enabled)
        .b("disabled", o.disabled)
        .s("description", &o.description)
        .s("bundle", &o.bundle)
        .s("bundle-speed", &o.bundle_speed)
        .s("lldp-admin", &o.lldp_admin)
        .s("mtu", &o.mtu)
        .s("flow-control", &o.flow_control)
        .s("media-active", &o.media_active)
        .s("media-fixed", &o.media_fixed)
        .s("media-max", &o.media_max)
        .s("media-sfp", &o.media_sfp)
        .s("port-fwd-mode", &o.port_fwd_mode)
        .s("qinq-ethertype", &o.qinq_ethertype)
        .s("stp", &o.stp)
        .s("stp-edge-port", &o.stp_edge_port)
        .s("stp-link-type", &o.stp_link_type)
        .s("stp-auto-edge-port", &o.stp_auto_edge_port)
        .s("stp-reset", &o.stp_reset)
        .s("sflow-poll-interval", &o.sflow_poll_interval)
        .s("sflow-poll-interval-global", &o.sflow_poll_interval_global)
        .s("vendor", &o.vendor)
        .s("vendor-oui", &o.vendor_oui)
        .s("vendor-partnum", &o.vendor_partnum)
        .s("vendor-revision", &o.vendor_revision)
        .s("virtual-wire", &o.virtual_wire)
        .s("transmitter-technology", &o.transmitter_technology)
        .s("lacp-port-priority", &o.lacp_port_priority)
        .done()
}

pub(super) fn project_net_dns_resolver(o: &BigipNetDnsResolver) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v(
            "route-domain",
            path_ref(&o.route_domain, "net route-domain"),
        )
        .v("nameservers", str_list(&o.nameservers))
        .v("forward-zones", str_list(&o.forward_zones))
        .s("description", &o.description)
        .s("cache-size", &o.cache_size)
        .s("randomize-query-name-case", &o.randomize_query_name_case)
        .s("use-ipv4", &o.use_ipv4)
        .s("use-ipv6", &o.use_ipv6)
        .s("use-tcp", &o.use_tcp)
        .s("use-udp", &o.use_udp)
        .s("answer-default-zones", &o.answer_default_zones)
        .s("prefetch", &o.prefetch)
        .s("nameserver-min-rtt", &o.nameserver_min_rtt)
        .s("nameserver-ttl", &o.nameserver_ttl)
        .s("outbound-msg-retry", &o.outbound_msg_retry)
        .done()
}

pub(super) fn project_net_tunnel(o: &BigipNetTunnel) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("profile", &o.profile)
        .s("local-address", &o.local_address)
        .s("remote-address", &o.remote_address)
        .s("secondary-address", &o.secondary_address)
        .v(
            "traffic-group",
            path_ref(&o.traffic_group, "cm traffic-group"),
        )
        .s("description", &o.description)
        .s("mtu", &o.mtu)
        .s("mode", &o.mode)
        .s("idle-timeout", &o.idle_timeout)
        .s("auto-lasthop", &o.auto_lasthop)
        .s("transparent", &o.transparent)
        .s("key", &o.key)
        .s("use-pmtu", &o.use_pmtu)
        .s("tos", &o.tos)
        .done()
}

pub(super) fn project_net_stp(o: &BigipNetStp) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v(
            "interfaces",
            path_ref_list_strs(&o.interfaces, "net interface"),
        )
        .v("vlans", path_ref_list_strs(&o.vlans, "net vlan"))
        .s("description", &o.description)
        .s("mode", &o.mode)
        .s("priority", &o.priority)
        .s("external-path-cost", &o.external_path_cost)
        .s("internal-path-cost", &o.internal_path_cost)
        .done()
}
