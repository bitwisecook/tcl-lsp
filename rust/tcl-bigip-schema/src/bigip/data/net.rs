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

//! BIG-IP object specs for the `net` tmsh module.
//!
//! **Hand-maintained**, with no generator to produce these from — edit
//! them directly, in the same shape the other module files use.  Organised
//! by tmsh module name (this file's own module field, `Some("net")`).
//!
//! `cargo xtask bigip-data-schema --check` enforces the internal
//! invariants a generator would otherwise have guaranteed: every `kind`
//! is globally unique, filed under the module file matching its own
//! `module` field, and every `references` target either names a real
//! kind or is on the documented known-gap list.
//!
//! One file per tmsh component (the word after the module); a large
//! component has one file per following word.

use crate::bigip::BigipObjectSpec;

mod address_list;
mod arp;
mod bwc;
mod cmetrics;
mod cos;
mod dag_globals;
mod dns_resolver;
mod fdb;
mod interface;
mod ipsec;
mod ipv6_subscriber_prefix_length;
mod lacp_globals;
mod lldp_globals;
mod multicast_globals;
mod ndp;
mod packet_filter;
mod packet_filter_trusted;
mod port_list;
mod port_mirror;
mod rate_shaping;
mod route;
mod route_domain;
mod router_advertisement;
mod routing;
mod self_;
mod self_allow;
mod service_policy;
mod sfc;
mod stp;
mod stp_globals;
mod timer_policy;
mod trunk;
mod tunnels;
mod vlan;
mod vlan_group;
mod wccp;

/// The spec tables of `net`, in the order they are filed.
#[must_use]
pub fn tables() -> Vec<&'static [BigipObjectSpec]> {
    [
        vec![
            address_list::SPECS,
            arp::SPECS,
            bwc::SPECS,
            cmetrics::SPECS,
            cos::SPECS,
            dag_globals::SPECS,
            dns_resolver::SPECS,
            fdb::SPECS,
            interface::SPECS,
            ipsec::SPECS,
            ipv6_subscriber_prefix_length::SPECS,
            lacp_globals::SPECS,
            lldp_globals::SPECS,
            multicast_globals::SPECS,
            ndp::SPECS,
            packet_filter::SPECS,
            packet_filter_trusted::SPECS,
            port_list::SPECS,
            port_mirror::SPECS,
            rate_shaping::SPECS,
            route::SPECS,
            route_domain::SPECS,
            router_advertisement::SPECS,
        ],
        routing::tables(),
        vec![
            self_::SPECS,
            self_allow::SPECS,
            service_policy::SPECS,
            sfc::SPECS,
            stp::SPECS,
            stp_globals::SPECS,
            timer_policy::SPECS,
            trunk::SPECS,
            tunnels::SPECS,
            vlan::SPECS,
            vlan_group::SPECS,
            wccp::SPECS,
        ],
    ]
    .concat()
}
