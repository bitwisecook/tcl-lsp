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

//! Field projection of GTM objects: data centres, servers, pools, wide IPs and listeners.

use super::ltm::str_list;
use super::{Fields, path_ref, path_ref_list_strs};
use crate::value::Value;
use indexmap::IndexMap;
use tcl_bigip::model::{
    BigipGtmDatacenter, BigipGtmListener, BigipGtmPool, BigipGtmPoolMember, BigipGtmServer,
    BigipGtmWideip,
};

// GTM projections

pub(super) fn project_gtm_datacenter(o: &BigipGtmDatacenter) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("contact", &o.contact)
        .s("location", &o.location)
        .s("description", &o.description)
        .v("prober-pool", path_ref(&o.prober_pool, "gtm prober-pool"))
        .s("prober-preference", &o.prober_preference)
        .s("prober-fallback", &o.prober_fallback)
        .s("state", &o.state)
        .done()
}

pub(super) fn project_gtm_server(o: &BigipGtmServer) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("datacenter", path_ref(&o.datacenter, "gtm datacenter"))
        .s("monitor", &o.monitor)
        .s("product", &o.product)
        // The device's own (self) IP addresses.
        .v("addresses", str_list(&o.addresses))
        // The destinations of the server's virtual-servers — these are the
        // downstream LTM virtual addresses this GTM balances across, and the
        // signal the report uses to link a GTM tier to its LTM tiers.
        .v("virtual-servers", str_list(&o.virtual_servers))
        .s("description", &o.description)
        .s("state", &o.state)
        .v("prober-pool", path_ref(&o.prober_pool, "gtm prober-pool"))
        .s("virtual-server-discovery", &o.virtual_server_discovery)
        .done()
}

pub(super) fn project_gtm_pool(o: &BigipGtmPool) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("record-type", &o.record_type)
        .s("monitor", &o.monitor)
        .s("load-balancing-mode", &o.load_balancing_mode)
        .s("alternate-mode", &o.alternate_mode)
        .s("fallback-mode", &o.fallback_mode)
        .s("fallback-ip", &o.fallback_ip)
        .s("ttl", &o.ttl)
        .s("max-answers-returned", &o.max_answers_returned)
        .s("verify-member-availability", &o.verify_member_availability)
        .v("members", project_gtm_pool_members(&o.members))
        .s("description", &o.description)
        .s("state", &o.state)
        .done()
}

fn project_gtm_pool_members(members: &[BigipGtmPoolMember]) -> Value {
    Value::List(
        members
            .iter()
            .map(|m| {
                Value::Object(
                    Fields::new()
                        .s("name", &m.name)
                        .s("service-port", &m.service_port)
                        .s("order", &m.order)
                        .s("member-order", &m.member_order)
                        .s("ratio", &m.ratio)
                        .s("monitor", &m.monitor)
                        .s("static-target", &m.static_target)
                        .s("state", &m.state)
                        .s("description", &m.description)
                        .done(),
                )
            })
            .collect(),
    )
}

pub(super) fn project_gtm_wideip(o: &BigipGtmWideip) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("record-type", &o.record_type)
        .v("pools", path_ref_list_strs(&o.pools, "gtm pool"))
        .v("aliases", str_list(&o.aliases))
        .s("pool-lb-mode", &o.pool_lb_mode)
        .v(
            "last-resort-pool",
            path_ref(&o.last_resort_pool, "gtm pool"),
        )
        .s("persistence", &o.persistence)
        .s("description", &o.description)
        .s("state", &o.state)
        .done()
}

pub(super) fn project_gtm_listener(o: &BigipGtmListener) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("address", &o.address)
        .s("port", &o.port)
        .s("ip-protocol", &o.ip_protocol)
        .s("mask", &o.mask)
        .v("pool", path_ref(&o.pool, "gtm pool"))
        .v("profiles", path_ref_list_strs(&o.profiles, "ltm profile"))
        .v("rules", path_ref_list_strs(&o.rules, "gtm rule"))
        .s("source-address-translation", &o.source_address_translation)
        .v("vlans", str_list(&o.vlans))
        .b("vlans-disabled", o.vlans_disabled)
        .b("vlans-enabled", o.vlans_enabled)
        .s("state", &o.state)
        .s("description", &o.description)
        .done()
}
